#!/usr/bin/env node

import { spawnSync } from 'node:child_process'

const mode = process.argv[2] ?? 'all'

const modes =
  mode === 'all'
    ? ['pm', 'fast']
    : mode === 'pm' || mode === 'fast'
      ? [mode]
      : null

if (!modes) {
  console.error(`Unknown mode '${mode}'. Use pm, fast, or all.`)
  process.exit(1)
}

for (const currentMode of modes) {
  const env = {
    ...process.env,
    HNI_FAST: currentMode === 'fast' ? 'true' : 'false',
  }

  console.log(`\n[hni] cargo test (${currentMode} mode)\n`)
  const result = spawnSync(
    'cargo',
    ['test', '--all-targets', '--all-features'],
    {
      stdio: 'inherit',
      env,
    },
  )

  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }
}
