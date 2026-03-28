import fs from 'node:fs'
import path from 'node:path'

function usage() {
  console.error('usage: node scripts/render-benchmark-comment.mjs <combined-json-path>')
  process.exit(1)
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function formatMs(value) {
  return `${value.toFixed(2)} ms`
}

function formatRatio(value) {
  return `${value.toFixed(2)}x`
}

function table(headers, rows) {
  const headerLine = `| ${headers.join(' | ')} |`
  const separatorLine = `| ${headers.map(() => '---').join(' | ')} |`
  const rowLines = rows.map((row) => `| ${row.join(' | ')} |`)
  return [headerLine, separatorLine, ...rowLines].join('\n')
}

function compareSection(track) {
  const rows = track.results.map((result) => [
    `\`${result.case}\``,
    formatMs(result.participants.antfu.mean),
    formatMs(result.participants.hni.mean),
    formatRatio(result.relative_to_first_mean.hni),
  ])

  return [
    '### Compare',
    '',
    `Overall: \`hni\` vs \`antfu\` geometric mean \`${formatRatio(
      track.summary.geometric_mean_relative_to_first.hni,
    )}\`.`,
    '',
    table(['Case', 'antfu', 'hni', 'Relative'], rows),
  ].join('\n')
}

function fastSection(track) {
  const results = [...track.results]
  const ranked = results
    .map((result) => ({
      ...result,
      relative: result.relative_to_first_mean.fast,
    }))
    .sort((left, right) => right.relative - left.relative)

  const highlights = ranked.slice(0, 5).map((result) => [
    `\`${result.case}\``,
    formatMs(result.participants.pm.mean),
    formatMs(result.participants.fast.mean),
    formatRatio(result.relative),
  ])

  const regressions = ranked
    .filter((result) => result.relative < 1)
    .sort((left, right) => left.relative - right.relative)
    .map((result) => [
      `\`${result.case}\``,
      formatMs(result.participants.pm.mean),
      formatMs(result.participants.fast.mean),
      formatRatio(result.relative),
    ])

  const lines = [
    '### Fast',
    '',
    `Overall: fast vs pm geometric mean \`${formatRatio(
      track.summary.geometric_mean_relative_to_first.fast,
    )}\`.`,
    '',
    'Top wins:',
    '',
    table(['Case', 'PM', 'Fast', 'Relative'], highlights),
  ]

  if (regressions.length > 0) {
    lines.push(
      '',
      'Cases where fast was slower:',
      '',
      table(['Case', 'PM', 'Fast', 'Relative'], regressions),
    )
  }

  return lines.join('\n')
}

function runtimeSection(track) {
  const rows = track.results.map((result) => [
    `\`${result.case}\``,
    formatMs(result.participants.hni.mean),
    formatMs(result.participants.bun.mean),
    formatMs(result.participants.deno.mean),
  ])

  return [
    '### Runtime',
    '',
    `Relative to \`hni\`: \`bun\` \`${formatRatio(
      track.summary.geometric_mean_relative_to_first.bun,
    )}\`, \`deno\` \`${formatRatio(track.summary.geometric_mean_relative_to_first.deno)}\`.`,
    '',
    table(['Case', 'hni', 'bun', 'deno'], rows),
  ].join('\n')
}

const combinedJsonPath = process.argv[2]

if (!combinedJsonPath) {
  usage()
}

const combined = readJson(combinedJsonPath)
const tracks = combined.tracks ?? {}
const firstTrack = tracks.compare ?? tracks.fast ?? tracks.runtime

if (!firstTrack) {
  console.error(`no benchmark tracks found in ${combinedJsonPath}`)
  process.exit(1)
}

const lines = [
  '<!-- hni-benchmark-report -->',
  '## Benchmark Results',
  '',
  `Generated: \`${combined.timestamp}\``,
  `Environment: \`${firstTrack.platform}/${firstTrack.arch}\` · \`${firstTrack.runs}\` measured runs · \`${firstTrack.warmups}\` warmups`,
  '',
]

if (tracks.compare) {
  lines.push(compareSection(tracks.compare), '')
}

if (tracks.fast) {
  lines.push(fastSection(tracks.fast), '')
}

if (tracks.runtime) {
  lines.push(runtimeSection(tracks.runtime), '')
}

lines.push(
  `Artifacts were uploaded from \`${path.basename(combinedJsonPath)}\` in this workflow run.`,
)

process.stdout.write(`${lines.join('\n')}\n`)
