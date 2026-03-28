#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const DEFAULT_RUNS = 500
const DEFAULT_WARMUPS = 50
const TRACKS = ['compare', 'fast', 'runtime', 'direct', 'fixtures']
const SUMMARY_ONLY_TRACKS = new Set(['fixtures'])

const PMS = [
  {
    id: 'npm',
    label: 'npm',
    fixtureKey: 'npm',
    packageManager: 'npm@10.0.0',
    lockfile: 'package-lock.json',
    requiredBins: ['npm', 'npx'],
  },
  {
    id: 'pnpm',
    label: 'pnpm',
    fixtureKey: 'pnpm',
    packageManager: 'pnpm@9.0.0',
    lockfile: 'pnpm-lock.yaml',
    requiredBins: ['pnpm'],
  },
  {
    id: 'yarn',
    label: 'yarn',
    fixtureKey: 'yarn',
    packageManager: 'yarn@1.22.0',
    lockfile: 'yarn.lock',
    requiredBins: ['yarn'],
  },
  {
    id: 'bun',
    label: 'bun',
    fixtureKey: 'bun',
    packageManager: 'bun@1.3.5',
    lockfile: 'bun.lockb',
    requiredBins: ['bun'],
  },
  {
    id: 'deno',
    label: 'deno',
    fixtureKey: 'deno',
    requiredBins: ['deno'],
  },
]

function parseArgs(argv) {
  const args = {
    runs: DEFAULT_RUNS,
    warmups: DEFAULT_WARMUPS,
    build: true,
    track: 'all',
    format: 'table',
  }

  for (const raw of argv) {
    if (raw === '--no-build') {
      args.build = false
      continue
    }
    if (raw.startsWith('--runs=')) {
      args.runs = Number(raw.split('=')[1])
      continue
    }
    if (raw.startsWith('--warmups=')) {
      args.warmups = Number(raw.split('=')[1])
      continue
    }
    if (raw.startsWith('--track=')) {
      args.track = raw.split('=')[1]
      continue
    }
    if (raw.startsWith('--format=')) {
      args.format = raw.split('=')[1]
      continue
    }
  }

  if (!Number.isInteger(args.runs) || args.runs <= 0) {
    throw new Error('--runs must be a positive integer')
  }

  if (!Number.isInteger(args.warmups) || args.warmups < 0) {
    throw new Error('--warmups must be a non-negative integer')
  }

  if (args.track !== 'all' && !TRACKS.includes(args.track)) {
    throw new Error(`unsupported track: ${args.track}`)
  }

  if (!['table', 'markdown', 'json'].includes(args.format)) {
    throw new Error(`unsupported format: ${args.format}`)
  }

  return args
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true })
}

function run(cmd, argv, options = {}) {
  execFileSync(cmd, argv, {
    stdio: 'inherit',
    ...options,
  })
}

function ensureBinary(name, installHint = '') {
  const result = spawnSync('sh', ['-c', `command -v ${name}`], {
    encoding: 'utf8',
  })
  const value = result.stdout.trim()
  if (value) {
    return value
  }
  const suffix = installHint ? ` (${installHint})` : ''
  throw new Error(`required binary not found: ${name}${suffix}`)
}

function shellQuote(value) {
  if (value.length === 0) {
    return "''"
  }
  return `'${value.replace(/'/g, `'\"'\"'`)}'`
}

function buildCommand(envMap, executable, args) {
  const parts = ['env']
  for (const [key, value] of Object.entries(envMap)) {
    parts.push(`${key}=${value}`)
  }
  parts.push(executable)
  parts.push(...args)
  return parts.map((part) => shellQuote(part)).join(' ')
}

function buildShellCommand(envMap, shellCommand) {
  const parts = ['env']
  for (const [key, value] of Object.entries(envMap)) {
    parts.push(`${key}=${value}`)
  }
  parts.push('sh')
  parts.push('-lc')
  parts.push(shellCommand)
  return parts.map((part) => shellQuote(part)).join(' ')
}

function percentile(sortedValues, p) {
  if (sortedValues.length === 0) return null
  const index = Math.max(0, Math.ceil(sortedValues.length * p) - 1)
  return sortedValues[index]
}

function fromHyperfineResult(rawResult) {
  const times = Array.isArray(rawResult.times)
    ? [...rawResult.times].sort((a, b) => a - b)
    : []

  return {
    mean: rawResult.mean * 1000,
    median: rawResult.median * 1000,
    p95: (percentile(times, 0.95) ?? rawResult.max) * 1000,
    min: rawResult.min * 1000,
    max: rawResult.max * 1000,
    stddev: rawResult.stddev * 1000,
    samples: times.length,
  }
}

function geometricMean(values) {
  if (values.length === 0 || values.some((value) => value <= 0)) {
    return null
  }
  const sum = values.reduce((acc, value) => acc + Math.log(value), 0)
  return Math.exp(sum / values.length)
}

function groupBy(rows, key) {
  const out = {}
  for (const row of rows) {
    const group = row[key]
    if (!out[group]) out[group] = []
    out[group].push(row)
  }
  return out
}

function writeNodeFixture(dir, pm) {
  ensureDir(dir)
  ensureDir(path.join(dir, 'node_modules', '.bin'))
  fs.writeFileSync(
    path.join(dir, 'package.json'),
    JSON.stringify(
      {
        name: `benchmark-${pm.id}`,
        version: '1.0.0',
        packageManager: pm.packageManager,
        scripts: {
          noop: 'node -e ""',
          build: 'node -e ""',
          dev: 'node -e ""',
          args: 'node -e "" --',
          prehooks: 'node -e ""',
          hooks: 'node -e ""',
          posthooks: 'node -e ""',
        },
      },
      null,
      2,
    ),
    'utf8',
  )
  fs.writeFileSync(path.join(dir, pm.lockfile), 'lock\n', 'utf8')

  const bins = {
    vitest: '#!/bin/sh\nexit 0\n',
    hello: '#!/bin/sh\nexit 0\n',
  }

  for (const [name, contents] of Object.entries(bins)) {
    const binPath = path.join(dir, 'node_modules', '.bin', name)
    fs.writeFileSync(binPath, contents, 'utf8')
    fs.chmodSync(binPath, 0o755)
  }
}

function writeDenoFixture(dir) {
  ensureDir(dir)
  fs.writeFileSync(
    path.join(dir, 'deno.json'),
    JSON.stringify(
      {
        tasks: {
          noop: 'deno eval ""',
          hooks: 'deno eval ""',
        },
      },
      null,
      2,
    ),
    'utf8',
  )
}

function aliasBinPath(dir, name) {
  return process.platform === 'win32' ? path.join(dir, `${name}.exe`) : path.join(dir, name)
}

function createAlias(target, destination) {
  if (process.platform === 'win32') {
    fs.copyFileSync(target, destination)
    return
  }
  fs.symlinkSync(target, destination)
}

function availableBinaries() {
  const out = {}
  for (const name of ['npm', 'npx', 'pnpm', 'yarn', 'bun', 'deno']) {
    out[name] = Boolean(
      spawnSync('sh', ['-c', `command -v ${name}`], { encoding: 'utf8' }).stdout.trim(),
    )
  }
  return out
}

function installAntfuNi(repoRoot, cacheDir) {
  ensureDir(cacheDir)
  process.stdout.write('Installing/updating @antfu/ni in benchmark cache...\n')
  run('npm', ['i', '-g', '@antfu/ni', '--prefix', cacheDir], { cwd: repoRoot })
}

function interpolateArgs(args, fixturePaths) {
  return args.map((arg) => {
    if (arg === '<npmFixture>') return fixturePaths.npm
    if (arg === '<pnpmFixture>') return fixturePaths.pnpm
    if (arg === '<yarnFixture>') return fixturePaths.yarn
    if (arg === '<bunFixture>') return fixturePaths.bun
    if (arg === '<denoFixture>') return fixturePaths.deno
    return arg
  })
}

function compareCases() {
  return [
    {
      id: 'compare_startup_version',
      group: 'startup',
      case: 'ni --version',
      commands: [
        { name: 'antfu', bin: 'ni', args: ['--version'] },
        { name: 'hni', bin: 'ni', args: ['--version'] },
      ],
      requiredBins: [],
    },
    {
      id: 'compare_ni_vite',
      group: 'ni',
      case: 'ni vite ? (npm)',
      commands: [
        { name: 'antfu', bin: 'ni', args: ['-C', '<npmFixture>', 'vite', '?'] },
        { name: 'hni', bin: 'ni', args: ['-C', '<npmFixture>', 'vite', '?'] },
      ],
      requiredBins: ['npm', 'npx'],
    },
    {
      id: 'compare_nr_build',
      group: 'nr',
      case: 'nr build ? (pnpm)',
      commands: [
        { name: 'antfu', bin: 'nr', args: ['-C', '<pnpmFixture>', 'build', '?'] },
        { name: 'hni', bin: 'nr', args: ['-C', '<pnpmFixture>', 'build', '?'] },
      ],
      requiredBins: ['pnpm'],
    },
    {
      id: 'compare_nlx_vitest',
      group: 'nlx',
      case: 'nlx vitest ? (npm)',
      commands: [
        { name: 'antfu', bin: 'nlx', args: ['-C', '<npmFixture>', 'vitest', '?'] },
        { name: 'hni', bin: 'nlx', args: ['-C', '<npmFixture>', 'vitest', '?'] },
      ],
      requiredBins: ['npm', 'npx'],
    },
  ]
}

function fastCases(fixturePaths) {
  const cases = []

  for (const pm of PMS) {
    if (pm.id === 'deno') {
      cases.push(
        {
          id: `${pm.id}_nr_noop`,
          group: 'nr',
          case: `nr noop (${pm.label})`,
          commands: [
            { name: 'pm', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'noop'], env: { HNI_FAST: 'false' } },
            { name: 'fast', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'noop'], env: { HNI_FAST: 'true' } },
          ],
          requiredBins: pm.requiredBins,
        },
        {
          id: `${pm.id}_nr_hooks`,
          group: 'nr',
          case: `nr hooks (${pm.label})`,
          commands: [
            { name: 'pm', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'hooks'], env: { HNI_FAST: 'false' } },
            { name: 'fast', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'hooks'], env: { HNI_FAST: 'true' } },
          ],
          requiredBins: pm.requiredBins,
        },
        {
          id: `${pm.id}_node_run_noop`,
          group: 'node-run',
          case: `node run noop (${pm.label})`,
          commands: [
            { name: 'pm', bin: 'node', args: ['-C', fixturePaths[pm.fixtureKey], 'run', 'noop'], env: { HNI_FAST: 'false' } },
            { name: 'fast', bin: 'node', args: ['-C', fixturePaths[pm.fixtureKey], 'run', 'noop'], env: { HNI_FAST: 'true' } },
          ],
          requiredBins: pm.requiredBins,
        },
      )
      continue
    }

    cases.push(
      {
        id: `${pm.id}_nr_noop`,
        group: 'nr',
        case: `nr noop (${pm.label})`,
        commands: [
          { name: 'pm', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'noop'], env: { HNI_FAST: 'false' } },
          { name: 'fast', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'noop'], env: { HNI_FAST: 'true' } },
        ],
        requiredBins: pm.requiredBins,
      },
      {
        id: `${pm.id}_nr_hooks`,
        group: 'nr',
        case: `nr hooks (${pm.label})`,
        commands: [
          { name: 'pm', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'hooks'], env: { HNI_FAST: 'false' } },
          { name: 'fast', bin: 'nr', args: ['-C', fixturePaths[pm.fixtureKey], 'hooks'], env: { HNI_FAST: 'true' } },
        ],
        requiredBins: pm.requiredBins,
      },
      {
        id: `${pm.id}_node_run_noop`,
        group: 'node-run',
        case: `node run noop (${pm.label})`,
        commands: [
          { name: 'pm', bin: 'node', args: ['-C', fixturePaths[pm.fixtureKey], 'run', 'noop'], env: { HNI_FAST: 'false' } },
          { name: 'fast', bin: 'node', args: ['-C', fixturePaths[pm.fixtureKey], 'run', 'noop'], env: { HNI_FAST: 'true' } },
        ],
        requiredBins: pm.requiredBins,
      },
    )
  }

  cases.push({
    id: 'npm_nlx_hello',
    group: 'nlx',
    case: 'nlx hello --flag (npm local bin)',
    commands: [
      { name: 'pm', bin: 'nlx', args: ['-C', fixturePaths.npm, 'hello', '--flag'], env: { HNI_FAST: 'false' } },
      { name: 'fast', bin: 'nlx', args: ['-C', fixturePaths.npm, 'hello', '--flag'], env: { HNI_FAST: 'true' } },
    ],
    requiredBins: ['npm'],
  })

  return cases
}

function runtimeCases(fixturePaths) {
  return [
    {
      id: 'runtime_task_noop',
      group: 'runtime',
      case: 'task noop',
      commands: [
        {
          name: 'hni',
          kind: 'exec',
          bin: 'nr',
          args: ['-C', fixturePaths.pnpm, 'noop'],
          env: { HNI_FAST: 'true' },
        },
        {
          name: 'bun',
          kind: 'shell',
          command: `cd ${shellQuote(fixturePaths.bun)} && bun run --silent noop`,
        },
        {
          name: 'deno',
          kind: 'shell',
          command: `deno task --cwd ${shellQuote(fixturePaths.deno)} --quiet noop`,
        },
      ],
      requiredBins: ['pnpm', 'bun', 'deno'],
    },
    {
      id: 'runtime_task_hooks',
      group: 'runtime',
      case: 'task hooks',
      commands: [
        {
          name: 'hni',
          kind: 'exec',
          bin: 'nr',
          args: ['-C', fixturePaths.pnpm, 'hooks'],
          env: { HNI_FAST: 'true' },
        },
        {
          name: 'bun',
          kind: 'shell',
          command: `cd ${shellQuote(fixturePaths.bun)} && bun run --silent hooks`,
        },
        {
          name: 'deno',
          kind: 'shell',
          command: `deno task --cwd ${shellQuote(fixturePaths.deno)} --quiet hooks`,
        },
      ],
      requiredBins: ['pnpm', 'bun', 'deno'],
    },
  ]
}

function directCases(fixturePaths) {
  return PMS.flatMap((pm) => {
    const fixture = fixturePaths[pm.fixtureKey]
    const cases = [
      {
        id: `${pm.id}_direct_noop`,
        group: 'noop',
        case: `task noop (${pm.label})`,
        commands: [
          directRunCommand(pm, fixture, 'noop'),
          {
            name: 'hni',
            kind: 'exec',
            bin: 'nr',
            args: ['-C', fixture, 'noop'],
            env: { HNI_FAST: 'true' },
          },
        ],
        requiredBins: pm.requiredBins,
      },
      {
        id: `${pm.id}_direct_hooks`,
        group: 'hooks',
        case: `task hooks (${pm.label})`,
        commands: [
          directRunCommand(pm, fixture, 'hooks'),
          {
            name: 'hni',
            kind: 'exec',
            bin: 'nr',
            args: ['-C', fixture, 'hooks'],
            env: { HNI_FAST: 'true' },
          },
        ],
        requiredBins: pm.requiredBins,
      },
    ]

    const localExec = directLocalExecCommand(pm, fixture, 'hello', ['--flag'])
    if (localExec) {
      cases.push({
        id: `${pm.id}_direct_exec_hello`,
        group: 'exec',
        case: `exec hello --flag (${pm.label})`,
        commands: [
          localExec,
          {
            name: 'hni',
            kind: 'exec',
            bin: 'nlx',
            args: ['-C', fixture, 'hello', '--flag'],
            env: { HNI_FAST: 'true' },
          },
        ],
        requiredBins: pm.requiredBins,
      })
    }

    return cases
  })
}

function directRunCommand(pm, fixturePath, scriptName) {
  if (pm.id === 'deno') {
    return {
      name: 'direct',
      kind: 'shell',
      command: `deno task --cwd ${shellQuote(fixturePath)} --quiet ${shellQuote(scriptName)}`,
    }
  }

  if (pm.id === 'bun') {
    return {
      name: 'direct',
      kind: 'shell',
      command: `cd ${shellQuote(fixturePath)} && bun run --silent ${shellQuote(scriptName)}`,
    }
  }

  if (pm.id === 'yarn') {
    return {
      name: 'direct',
      kind: 'shell',
      command: `cd ${shellQuote(fixturePath)} && yarn run --silent ${shellQuote(scriptName)}`,
    }
  }

  return {
    name: 'direct',
    kind: 'shell',
    command: `cd ${shellQuote(fixturePath)} && ${pm.id} run --silent ${shellQuote(scriptName)}`,
  }
}

function directLocalExecCommand(pm, fixturePath, binName, args) {
  const renderedArgs = args.map((arg) => shellQuote(arg)).join(' ')
  const suffix = renderedArgs.length > 0 ? ` ${renderedArgs}` : ''

  if (pm.id === 'npm') {
    return {
      name: 'direct',
      kind: 'shell',
      command: `cd ${shellQuote(fixturePath)} && npx ${shellQuote(binName)}${suffix}`,
    }
  }

  if (pm.id === 'pnpm') {
    return {
      name: 'direct',
      kind: 'shell',
      command: `cd ${shellQuote(fixturePath)} && pnpm exec ${shellQuote(binName)}${suffix}`,
    }
  }

  if (pm.id === 'yarn') {
    return {
      name: 'direct',
      kind: 'shell',
      command: `cd ${shellQuote(fixturePath)} && yarn --silent ${shellQuote(binName)}${suffix}`,
    }
  }

  if (pm.id === 'bun') {
    return {
      name: 'direct',
      kind: 'shell',
      command: `cd ${shellQuote(fixturePath)} && bun x ${shellQuote(binName)}${suffix}`,
    }
  }

  return null
}

function runHyperfineCase({ repoRoot, caseDef, runs, warmups, rawOutputPath, commands }) {
  const cmdArgs = ['--runs', String(runs), '--warmup', String(warmups), '--style', 'none']

  for (const command of commands) {
    cmdArgs.push('--command-name', command.name)
  }

  cmdArgs.push('--export-json', rawOutputPath)
  cmdArgs.push(...commands.map((command) => command.command))

  const result = spawnSync('hyperfine', cmdArgs, {
    cwd: repoRoot,
    encoding: 'utf8',
  })

  if (result.status !== 0) {
    throw new Error(
      `hyperfine failed for case ${caseDef.id}\nstdout:\n${result.stdout || ''}\nstderr:\n${result.stderr || ''}`,
    )
  }

  const raw = JSON.parse(fs.readFileSync(rawOutputPath, 'utf8'))
  if (!Array.isArray(raw.results) || raw.results.length !== commands.length) {
    throw new Error(`unexpected hyperfine result format for case ${caseDef.id}`)
  }

  const participants = {}
  for (const [index, command] of commands.entries()) {
    participants[command.name] = fromHyperfineResult(raw.results[index])
  }

  const baseline = commands[0].name
  const relativeToFirstMean = {}
  const relativeToFirstMedian = {}
  for (const command of commands.slice(1)) {
    relativeToFirstMean[command.name] =
      participants[baseline].mean / participants[command.name].mean
    relativeToFirstMedian[command.name] =
      participants[baseline].median / participants[command.name].median
  }

  return {
    id: caseDef.id,
    group: caseDef.group,
    case: caseDef.case,
    raw_json: rawOutputPath,
    participants,
    baseline,
    relative_to_first_mean: relativeToFirstMean,
    relative_to_first_median: relativeToFirstMedian,
  }
}

function validateBenchmarkCommands(repoRoot, commands) {
  for (const command of commands) {
    const result = spawnSync('sh', ['-lc', command.command], {
      cwd: repoRoot,
      encoding: 'utf8',
    })
    if (result.status === 0) {
      continue
    }

    const detail = [result.stderr, result.stdout]
      .map((value) => value.trim())
      .find((value) => value.length > 0)
      ?.split('\n')[0]

    return `preflight failed for ${command.name}${detail ? `: ${detail}` : ''}`
  }

  return null
}

function summarizeTrack(track, results, skipped) {
  const grouped = groupBy(results, 'group')
  const perGroup = {}

  for (const [group, rows] of Object.entries(grouped)) {
    const relative = groupRelativeMeans(rows)
    if (Object.keys(relative).length > 0) {
      perGroup[group] = relative
    }
  }

  return {
    total_cases: results.length + skipped.length,
    executed_cases: results.length,
    skipped_cases: skipped.length,
    geometric_mean_relative_to_first: overallRelativeMeans(results),
    per_group_geometric_mean_relative_to_first: perGroup,
    track,
  }
}

function groupRelativeMeans(rows) {
  const merged = {}
  for (const row of rows) {
    for (const [name, value] of Object.entries(row.relative_to_first_mean)) {
      if (!Number.isFinite(value) || value <= 0) continue
      if (!merged[name]) merged[name] = []
      merged[name].push(value)
    }
  }

  return Object.fromEntries(
    Object.entries(merged)
      .map(([name, values]) => [name, geometricMean(values)])
      .filter(([, value]) => value !== null),
  )
}

function overallRelativeMeans(rows) {
  return groupRelativeMeans(rows)
}

function printTrackSummary(payload, format) {
  if (format === 'json') {
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`)
    return
  }

  const lines = []
  lines.push('')
  lines.push(`Track: ${payload.track}`)

  if (payload.track === 'runtime') {
    lines.push(
      'case'.padEnd(28) +
        'hni (ms)'.padStart(12) +
        'bun (ms)'.padStart(12) +
        'deno (ms)'.padStart(12),
    )
    lines.push('-'.repeat(64))
    for (const row of payload.results) {
      lines.push(
        row.case.padEnd(28) +
          row.participants.hni.mean.toFixed(2).padStart(12) +
          row.participants.bun.mean.toFixed(2).padStart(12) +
          row.participants.deno.mean.toFixed(2).padStart(12),
      )
    }
  } else {
    const baselineLabel = payload.results[0]?.baseline ?? 'baseline'
    const participantNames = payload.results[0]
      ? Object.keys(payload.results[0].participants)
      : [baselineLabel]
    const competitors = participantNames.filter((name) => name !== baselineLabel)

    if (competitors.length <= 1) {
      const competitor = competitors[0] ?? 'other'
      lines.push(
        'case'.padEnd(34) +
          `${baselineLabel} (ms)`.padStart(16) +
          `${competitor} (ms)`.padStart(16) +
          'relative'.padStart(12),
      )
      lines.push('-'.repeat(78))
      for (const row of payload.results) {
        lines.push(
          row.case.padEnd(34) +
            row.participants[baselineLabel].mean.toFixed(2).padStart(16) +
            row.participants[competitor].mean.toFixed(2).padStart(16) +
            `${row.relative_to_first_mean[competitor].toFixed(2)}x`.padStart(12),
        )
      }
    } else {
      const widths = {
        case: 32,
        metric: 14,
        relative: 12,
      }
      const separator = '  '
      const summaryWidth =
        widths.case +
        participantNames.length * widths.metric +
        competitors.length * widths.relative +
        (participantNames.length + competitors.length) * separator.length
      lines.push(
        'case'.padEnd(widths.case) +
          participantNames
            .map((name) => `${name} (ms)`.padStart(widths.metric))
            .join(separator) +
          separator +
          competitors
            .map((name) => `${name} rel`.padStart(widths.relative))
            .join(separator),
      )
      lines.push('-'.repeat(summaryWidth))
      for (const row of payload.results) {
        lines.push(
          row.case.padEnd(widths.case) +
            participantNames
              .map((name) => row.participants[name].mean.toFixed(2).padStart(widths.metric))
              .join(separator) +
            separator +
            competitors
              .map((name) => `${row.relative_to_first_mean[name].toFixed(2)}x`.padStart(widths.relative))
              .join(separator),
        )
      }
    }
  }

  const participantCount = payload.results[0]
    ? Object.keys(payload.results[0].participants).length
    : 0
  lines.push(
    '-'.repeat(
      payload.track === 'runtime'
        ? 64
        : participantCount > 2
          ? 32 + participantCount * 14 + (participantCount - 1) * 12 + participantCount * 2
          : 78,
    ),
  )
  for (const [name, value] of Object.entries(payload.summary.geometric_mean_relative_to_first)) {
    lines.push(`geometric mean relative to ${payload.results[0]?.baseline ?? 'baseline'} (${name}): ${value.toFixed(2)}x`)
  }
  lines.push(`executed cases: ${payload.summary.executed_cases}, skipped cases: ${payload.summary.skipped_cases}`)
  lines.push('')

  if (format === 'markdown') {
    process.stdout.write(lines.map((line) => `> ${line}`.trimEnd()).join('\n') + '\n')
    return
  }

  process.stdout.write(lines.join('\n'))
}

function formatMs(value) {
  return `${value.toFixed(2)} ms`
}

function formatRatio(value) {
  return `${value.toFixed(2)}x`
}

function markdownLink(label, target) {
  return `[${label}](${target})`
}

function relativePath(fromDir, toPath) {
  const rel = path.relative(fromDir, toPath)
  return rel.length > 0 ? rel : '.'
}

function trackOverviewLine(payload) {
  const baseline = payload.results[0]?.baseline ?? 'baseline'
  const entries = Object.entries(payload.summary.geometric_mean_relative_to_first)
  if (entries.length === 0) {
    return `No relative benchmark summary was produced for \`${baseline}\`.`
  }

  return entries
    .map(([name, value]) => `Relative to \`${baseline}\`, \`${name}\` averaged \`${formatRatio(value)}\`.`)
    .join(' ')
}

function trackTable(payload) {
  if (payload.track === 'runtime') {
    const lines = [
      '| Case | hni | bun | deno |',
      '| --- | ---: | ---: | ---: |',
    ]

    for (const row of payload.results) {
      lines.push(
        `| ${row.case} | ${formatMs(row.participants.hni.mean)} | ${formatMs(row.participants.bun.mean)} | ${formatMs(row.participants.deno.mean)} |`,
      )
    }

    return lines.join('\n')
  }

  const baseline = payload.results[0]?.baseline ?? 'baseline'
  const participantNames = payload.results[0]
    ? Object.keys(payload.results[0].participants)
    : [baseline]
  const competitors = participantNames.filter((name) => name !== baseline)

  if (competitors.length <= 1) {
    const competitor = competitors[0] ?? 'other'
    const lines = [
      `| Case | ${baseline} | ${competitor} | Relative |`,
      '| --- | ---: | ---: | ---: |',
    ]

    for (const row of payload.results) {
      lines.push(
        `| ${row.case} | ${formatMs(row.participants[baseline].mean)} | ${formatMs(
          row.participants[competitor].mean,
        )} | ${formatRatio(row.relative_to_first_mean[competitor])} |`,
      )
    }

    return lines.join('\n')
  }

  const headers = [
    '| Case |',
    ...participantNames.map((name) => ` ${name} |`),
    ...competitors.map((name) => ` ${name} vs ${baseline} |`),
  ]
  const lines = [
    headers.join(''),
    ['| --- |', ...participantNames.map(() => ' ---: |'), ...competitors.map(() => ' ---: |')].join(''),
  ]

  for (const row of payload.results) {
    lines.push(
      [
        `| ${row.case} |`,
        ...participantNames.map((name) => ` ${formatMs(row.participants[name].mean)} |`),
        ...competitors.map((name) => ` ${formatRatio(row.relative_to_first_mean[name])} |`),
      ].join(''),
    )
  }

  return lines.join('\n')
}

function trackSkippedTable(payload) {
  if (payload.skipped.length === 0) {
    return 'None.'
  }

  const lines = ['| Case | Reason |', '| --- | --- |']
  for (const row of payload.skipped) {
    lines.push(`| ${row.case} | ${row.reason} |`)
  }
  return lines.join('\n')
}

function trackMarkdown(payload, artifactPaths) {
  return [
    `# ${payload.track[0].toUpperCase()}${payload.track.slice(1)} Benchmark`,
    '',
    `Generated: ${payload.timestamp}`,
    '',
    `JSON: ${markdownLink(path.basename(artifactPaths.jsonPath), path.basename(artifactPaths.jsonPath))}`,
    '',
    trackOverviewLine(payload),
    '',
    trackTable(payload),
    '',
    `Executed cases: ${payload.summary.executed_cases}. Skipped cases: ${payload.summary.skipped_cases}.`,
    '',
    '## Skipped',
    '',
    trackSkippedTable(payload),
    '',
  ].join('\n')
}

function combinedMarkdown(combined, combinedArtifacts, fromDir) {
  const lines = [
    '# Benchmark Run',
    '',
    `Generated: ${combined.timestamp}`,
    '',
    `Combined JSON: ${markdownLink(
      path.basename(combinedArtifacts.jsonPath),
      relativePath(fromDir, combinedArtifacts.jsonPath),
    )}`,
    '',
    '## Tracks',
    '',
  ]

  for (const [track, payload] of Object.entries(combined.tracks)) {
    const artifacts = combinedArtifacts.trackArtifacts[track]
    lines.push(`### ${track[0].toUpperCase()}${track.slice(1)}`)
    lines.push('')
    lines.push(trackOverviewLine(payload))
    lines.push('')
    lines.push(
      `Artifacts: ${markdownLink(
        path.basename(artifacts.jsonPath),
        relativePath(fromDir, artifacts.jsonPath),
      )} · ${markdownLink(path.basename(artifacts.markdownPath), relativePath(fromDir, artifacts.markdownPath))}`,
    )
    if (SUMMARY_ONLY_TRACKS.has(track)) {
      lines.push('')
      lines.push('Detailed per-case results are kept in the track artifact.')
    }
    lines.push('')
  }

  return `${lines.join('\n')}\n`
}

function latestMarkdown(combined, combinedArtifacts, benchmarkDir) {
  const lines = [
    '# Latest Benchmark Snapshot',
    '',
    `Updated: ${combined.timestamp}`,
    '',
    'This file is the small release-friendly benchmark snapshot. Raw JSON stays in `benchmark/results/`.',
    '',
    `Combined report: ${markdownLink(
      path.basename(combinedArtifacts.markdownPath),
      relativePath(benchmarkDir, combinedArtifacts.markdownPath),
    )}`,
    '',
  ]

  for (const [track, payload] of Object.entries(combined.tracks)) {
    const artifacts = combinedArtifacts.trackArtifacts[track]
    lines.push(`## ${track[0].toUpperCase()}${track.slice(1)}`)
    lines.push('')
    lines.push(trackOverviewLine(payload))
    lines.push('')
    lines.push(
      `Artifacts: ${markdownLink(
        path.basename(artifacts.markdownPath),
        relativePath(benchmarkDir, artifacts.markdownPath),
      )} · ${markdownLink(path.basename(artifacts.jsonPath), relativePath(benchmarkDir, artifacts.jsonPath))}`,
    )
    lines.push('')
    if (SUMMARY_ONLY_TRACKS.has(track)) {
      lines.push('Detailed per-case results are kept in the track artifact.')
      lines.push('')
    } else {
      lines.push(trackTable(payload))
      lines.push('')
    }
  }

  return `${lines.join('\n')}\n`
}

function historyMarkdown(resultsDir, benchmarkDir) {
  const files = fs
    .readdirSync(resultsDir)
    .filter((name) => name.startsWith('benchmark-') && name.endsWith('.md'))
    .sort()
    .reverse()
    .slice(0, 1)

  const lines = [
    '# Benchmark History',
    '',
    'The repo intentionally keeps only the latest tracked combined benchmark report. Use `benchmark/LATEST.md` for the release-facing snapshot.',
    '',
    '| Run | Report | JSON |',
    '| --- | --- | --- |',
  ]

  for (const file of files) {
    const jsonFile = file.replace(/\.md$/, '.json')
    lines.push(
      `| ${file.replace(/^benchmark-/, '').replace(/\.md$/, '')} | ${markdownLink(
        file,
        relativePath(benchmarkDir, path.join(resultsDir, file)),
      )} | ${markdownLink(jsonFile, relativePath(benchmarkDir, path.join(resultsDir, jsonFile)))} |`,
    )
  }

  return `${lines.join('\n')}\n`
}

function pruneTrackedBenchmarkArtifacts(resultsDir, keepPaths) {
  const keep = new Set(
    [...keepPaths, path.join(resultsDir, '.gitkeep')].map((entry) => path.resolve(entry)),
  )

  for (const entry of fs.readdirSync(resultsDir, { withFileTypes: true })) {
    if (!entry.isFile()) continue
    const absolutePath = path.resolve(resultsDir, entry.name)
    if (!keep.has(absolutePath)) {
      fs.rmSync(absolutePath, { force: true })
    }
  }
}

function payloadForTrack({ track, args, repoRoot, fixtures, binaries, skipped, results }) {
  return {
    timestamp: new Date().toISOString(),
    benchmark_tool: 'hyperfine',
    track,
    platform: process.platform,
    arch: process.arch,
    runs: args.runs,
    warmups: args.warmups,
    binaries,
    fixtures,
    summary: summarizeTrack(track, results, skipped),
    skipped,
    results,
  }
}

function prepareFixtures(tempRoot) {
  const fixturesRoot = path.join(tempRoot, 'fixtures')
  const fixturePaths = {
    npm: path.join(fixturesRoot, 'npm'),
    pnpm: path.join(fixturesRoot, 'pnpm'),
    yarn: path.join(fixturesRoot, 'yarn'),
    bun: path.join(fixturesRoot, 'bun'),
    deno: path.join(fixturesRoot, 'deno'),
  }

  writeNodeFixture(fixturePaths.npm, PMS[0])
  writeNodeFixture(fixturePaths.pnpm, PMS[1])
  writeNodeFixture(fixturePaths.yarn, PMS[2])
  writeNodeFixture(fixturePaths.bun, PMS[3])
  writeDenoFixture(fixturePaths.deno)

  return fixturePaths
}

function prepareFixtureBenchmarkDirs(tempRoot, repoRoot) {
  const sourceRoot = path.join(repoRoot, 'tests', 'fixtures')
  const copiedRoot = path.join(tempRoot, 'fixtures-benchmark')
  ensureDir(copiedRoot)

  for (const category of fs.readdirSync(sourceRoot).sort()) {
    const sourceCategory = path.join(sourceRoot, category)
    if (!fs.statSync(sourceCategory).isDirectory()) continue

    for (const name of fs.readdirSync(sourceCategory).sort()) {
      const sourceFixture = path.join(sourceCategory, name)
      if (!fs.statSync(sourceFixture).isDirectory()) continue

      const targetFixture = path.join(copiedRoot, category, name)
      ensureDir(path.dirname(targetFixture))
      fs.cpSync(sourceFixture, targetFixture, { recursive: true })
    }
  }

  return copiedRoot
}

function inferFixturePmId(name) {
  if (name === 'unknown') return null
  if (name.startsWith('npm')) return 'npm'
  if (name.startsWith('pnpm')) return 'pnpm'
  if (name.startsWith('yarn')) return 'yarn'
  if (name === 'bun') return 'bun'
  if (name === 'deno') return 'deno'
  return null
}

function requiredBinsForPmId(pmId) {
  return PMS.find((pm) => pm.id === pmId)?.requiredBins ?? []
}

function fixtureDirectCommand(pmId, fixturePath) {
  if (pmId === 'deno') {
    return `deno task --cwd ${shellQuote(fixturePath)} --quiet dev`
  }

  if (pmId === 'bun') {
    return `cd ${shellQuote(fixturePath)} && bun run --silent dev`
  }

  if (pmId === 'yarn') {
    return `cd ${shellQuote(fixturePath)} && yarn run --silent dev`
  }

  return `cd ${shellQuote(fixturePath)} && ${pmId} run --silent dev`
}

function fixtureDirectEnv(pmId) {
  if (pmId === 'npm') {
    return { npm_config_yes: 'true' }
  }

  return {}
}

function fixtureHniEnv(pmId, fastEnabled) {
  return {
    HNI_FAST: fastEnabled ? 'true' : 'false',
    ...(pmId === 'npm' ? { npm_config_yes: 'true' } : {}),
  }
}

function fixtureCases(fixturesRoot) {
  const cases = []

  for (const category of fs.readdirSync(fixturesRoot).sort()) {
    const categoryRoot = path.join(fixturesRoot, category)
    if (!fs.statSync(categoryRoot).isDirectory()) continue

    for (const name of fs.readdirSync(categoryRoot).sort()) {
      const fixturePath = path.join(categoryRoot, name)
      if (!fs.statSync(fixturePath).isDirectory()) continue

      const pmId = inferFixturePmId(name)
      if (!pmId) {
        cases.push({
          id: `fixtures_${category}_${name}`.replaceAll(/[^a-zA-Z0-9_]+/g, '_'),
          group: category,
          case: `${category}/${name}`,
          commands: [],
          requiredBins: [],
          skipReason: 'unknown fixture has no benchmark package-manager baseline',
        })
        continue
      }

      cases.push({
        id: `fixtures_${category}_${name}`.replaceAll(/[^a-zA-Z0-9_]+/g, '_'),
        group: category,
        case: `${category}/${name}`,
        commands: [
          {
            name: 'direct',
            kind: 'shell',
            command: fixtureDirectCommand(pmId, fixturePath),
            env: fixtureDirectEnv(pmId),
          },
          {
            name: 'pm',
            bin: 'nr',
            args: ['-C', fixturePath, 'dev'],
            env: fixtureHniEnv(pmId, false),
          },
          {
            name: 'fast',
            bin: 'nr',
            args: ['-C', fixturePath, 'dev'],
            env: fixtureHniEnv(pmId, true),
          },
        ],
        requiredBins: requiredBinsForPmId(pmId),
      })
    }
  }

  return cases
}

function prepareAliasDir(tempRoot, ourBin) {
  const aliasDir = path.join(tempRoot, 'bin')
  ensureDir(aliasDir)
  for (const name of ['hni', 'ni', 'nr', 'nlx', 'node']) {
    createAlias(ourBin, aliasBinPath(aliasDir, name))
  }
  return aliasDir
}

function resolveTrackCases(track, fixturePaths, fixtureBenchmarkRoot) {
  if (track === 'compare') return compareCases()
  if (track === 'fast') return fastCases(fixturePaths)
  if (track === 'runtime') return runtimeCases(fixturePaths)
  if (track === 'direct') return directCases(fixturePaths)
  if (track === 'fixtures') return fixtureCases(fixtureBenchmarkRoot)
  throw new Error(`unsupported track: ${track}`)
}

function materializeCommands({ track, caseDef, baseEnv, aliasDir, antfuBinDir, fixturePaths }) {
  return caseDef.commands.map((command) => {
    const envMap = { ...baseEnv, ...(command.env ?? {}) }

    if (command.kind === 'shell') {
      return {
        name: command.name,
        command: buildShellCommand(envMap, command.command),
      }
    }

    const args = interpolateArgs(command.args, fixturePaths)
    let executable
    if (command.name === 'antfu') {
      executable = aliasBinPath(antfuBinDir, command.bin)
    } else {
      executable = aliasBinPath(aliasDir, command.bin)
    }

    return {
      name: command.name,
      command: buildCommand(envMap, executable, args),
    }
  })
}

function filterRunnableCases(cases, availableBins, antfuBinDir) {
  const skipped = []
  const runnable = []

  for (const caseDef of cases) {
    if (caseDef.skipReason) {
      skipped.push({
        id: caseDef.id,
        case: caseDef.case,
        reason: caseDef.skipReason,
      })
      continue
    }

    const missing = caseDef.requiredBins.filter((bin) => !availableBins[bin])
    if (missing.length > 0) {
      skipped.push({
        id: caseDef.id,
        case: caseDef.case,
        reason: `missing required binaries: ${missing.join(', ')}`,
      })
      continue
    }

    const needsAntfu = caseDef.commands.some((command) => command.name === 'antfu')
    if (needsAntfu) {
      let missingAntfu = false
      for (const command of caseDef.commands) {
        if (command.name !== 'antfu') continue
        const antfuPath = aliasBinPath(antfuBinDir, command.bin)
        if (!fs.existsSync(antfuPath)) {
          skipped.push({
            id: caseDef.id,
            case: caseDef.case,
            reason: `missing antfu binary: ${command.bin}`,
          })
          missingAntfu = true
          break
        }
      }
      if (missingAntfu) continue
    }

    runnable.push(caseDef)
  }

  return { runnable, skipped }
}

function writeTrackJson(resultsDir, track, payload) {
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  const output = path.join(resultsDir, `${track}-${stamp}.json`)
  fs.writeFileSync(output, `${JSON.stringify(payload, null, 2)}\n`, 'utf8')
  return output
}

function writeTrackMarkdown(resultsDir, track, payload, artifactPaths) {
  const stamp = path.basename(artifactPaths.jsonPath).replace(`${track}-`, '').replace(/\.json$/, '')
  const output = path.join(resultsDir, `${track}-${stamp}.md`)
  fs.writeFileSync(output, trackMarkdown(payload, artifactPaths), 'utf8')
  return output
}

function writeCombinedJson(resultsDir, payload) {
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  const output = path.join(resultsDir, `benchmark-${stamp}.json`)
  fs.writeFileSync(output, `${JSON.stringify(payload, null, 2)}\n`, 'utf8')
  return output
}

function writeCombinedMarkdown(resultsDir, combined, combinedArtifacts, benchmarkDir) {
  const stamp = path
    .basename(combinedArtifacts.jsonPath)
    .replace(/^benchmark-/, '')
    .replace(/\.json$/, '')
  const output = path.join(resultsDir, `benchmark-${stamp}.md`)
  fs.writeFileSync(
    output,
    combinedMarkdown(combined, combinedArtifacts, path.dirname(output)),
    'utf8',
  )
  return output
}

function writeLatestSnapshot(benchmarkDir, combined, combinedArtifacts) {
  const output = path.join(benchmarkDir, 'LATEST.md')
  fs.writeFileSync(output, latestMarkdown(combined, combinedArtifacts, benchmarkDir), 'utf8')
  return output
}

function writeHistorySnapshot(resultsDir, benchmarkDir) {
  const output = path.join(benchmarkDir, 'HISTORY.md')
  fs.writeFileSync(output, historyMarkdown(resultsDir, benchmarkDir), 'utf8')
  return output
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  const scriptDir = path.dirname(fileURLToPath(import.meta.url))
  const repoRoot = path.resolve(scriptDir, '..')
  const benchmarkDir = path.join(repoRoot, 'benchmark')
  const resultsDir = path.join(repoRoot, 'benchmark', 'results')
  const rawDir = path.join(resultsDir, 'raw')
  const cacheDir = path.join(repoRoot, 'benchmark', '.cache')
  const antfuPrefix = path.join(cacheDir, 'antfu-ni')
  const antfuBinDir = path.join(antfuPrefix, 'bin')
  const ourBin = path.join(repoRoot, 'target', 'release', 'hni')

  ensureDir(resultsDir)
  ensureDir(rawDir)

  ensureBinary('hyperfine', 'install via `brew install hyperfine` or your package manager')

  const selectedTracks = args.track === 'all' ? TRACKS : [args.track]
  const needsCompare = selectedTracks.includes('compare')

  if (args.build) {
    ensureBinary('cargo')
    process.stdout.write('Building release binary...\n')
    run('cargo', ['build', '--release'], { cwd: repoRoot })
  }

  if (!fs.existsSync(ourBin)) {
    throw new Error(`missing binary: ${ourBin}`)
  }

  if (needsCompare) {
    ensureBinary('npm', 'required to cache @antfu/ni')
    installAntfuNi(repoRoot, antfuPrefix)
  }

  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'hni-benchmark-'))
  try {
    const fixturePaths = prepareFixtures(tempRoot)
    const fixtureBenchmarkRoot = prepareFixtureBenchmarkDirs(tempRoot, repoRoot)
    const aliasDir = prepareAliasDir(tempRoot, ourBin)
    const availableBins = availableBinaries()
    const baseEnv = {
      PATH: [aliasDir, antfuBinDir, process.env.PATH].filter(Boolean).join(path.delimiter),
      HNI_SKIP_PM_CHECK: '1',
    }

    const trackPayloads = {}
    const trackArtifacts = {}

    for (const track of selectedTracks) {
      const trackRawDir = path.join(rawDir, track)
      ensureDir(trackRawDir)

      const cases = resolveTrackCases(track, fixturePaths, fixtureBenchmarkRoot)
      const { runnable, skipped: initiallySkipped } = filterRunnableCases(cases, availableBins, antfuBinDir)
      const skipped = [...initiallySkipped]
      const results = []
      const stamp = new Date().toISOString().replace(/[:.]/g, '-')

      process.stdout.write(
        `Running ${track} benchmark (${args.warmups} warmups + ${args.runs} measured runs per case)...\n`,
      )
      process.stdout.write(`Total cases: ${cases.length}, runnable: ${runnable.length}\n`)

      for (const [index, caseDef] of runnable.entries()) {
        process.stdout.write(`[${index + 1}/${runnable.length}] ${caseDef.case}\n`)
        const commands = materializeCommands({
          track,
          caseDef,
          baseEnv,
          aliasDir,
          antfuBinDir,
          fixturePaths,
        })

        if (track === 'fixtures') {
          const preflightFailure = validateBenchmarkCommands(repoRoot, commands)
          if (preflightFailure) {
            skipped.push({
              id: caseDef.id,
              case: caseDef.case,
              reason: preflightFailure,
            })
            continue
          }
        }

        const rawOutputPath = path.join(trackRawDir, `${stamp}-${caseDef.id}.json`)
        results.push(
          runHyperfineCase({
            repoRoot,
            caseDef,
            runs: args.runs,
            warmups: args.warmups,
            rawOutputPath,
            commands,
          }),
        )
      }

      const payload = payloadForTrack({
        track,
        args,
        repoRoot,
        fixtures: track === 'fixtures' ? { root: fixtureBenchmarkRoot } : fixturePaths,
        binaries: {
          hni: ourBin,
          antfu_prefix: needsCompare ? antfuPrefix : null,
          hyperfine: ensureBinary('hyperfine'),
        },
        skipped,
        results,
      })

      trackPayloads[track] = payload
      printTrackSummary(payload, args.format)
      const trackJson = writeTrackJson(resultsDir, track, payload)
      const trackArtifact = { jsonPath: trackJson }
      const trackMarkdownPath = writeTrackMarkdown(resultsDir, track, payload, trackArtifact)
      trackArtifact.markdownPath = trackMarkdownPath
      trackArtifacts[track] = trackArtifact
      process.stdout.write(`JSON written to ${trackJson}\n`)
      process.stdout.write(`Markdown written to ${trackMarkdownPath}\n`)
    }

    const combined = {
      timestamp: new Date().toISOString(),
      benchmark_tool: 'hyperfine',
      tracks: trackPayloads,
    }
    const combinedPath = writeCombinedJson(resultsDir, combined)
    const combinedArtifacts = {
      jsonPath: combinedPath,
      trackArtifacts,
    }
    const combinedMarkdownPath = writeCombinedMarkdown(
      resultsDir,
      combined,
      combinedArtifacts,
      benchmarkDir,
    )
    combinedArtifacts.markdownPath = combinedMarkdownPath
    const latestPath = writeLatestSnapshot(benchmarkDir, combined, combinedArtifacts)
    const historyPath = writeHistorySnapshot(resultsDir, benchmarkDir)
    if (args.track === 'all') {
      pruneTrackedBenchmarkArtifacts(resultsDir, [
        combinedPath,
        combinedMarkdownPath,
        ...Object.values(trackArtifacts).flatMap((artifact) => [
          artifact.jsonPath,
          artifact.markdownPath,
        ]),
      ])
    }
    process.stdout.write(`Combined JSON written to ${combinedPath}\n`)
    process.stdout.write(`Combined markdown written to ${combinedMarkdownPath}\n`)
    process.stdout.write(`Latest snapshot written to ${latestPath}\n`)
    process.stdout.write(`History written to ${historyPath}\n`)
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true })
  }
}

try {
  main()
} catch (error) {
  const message = error instanceof Error ? error.message : String(error)
  process.stderr.write(`${message}\n`)
  process.exitCode = 1
}
