# alur Twitter launch thread draft v7

Status: draft only. Do not publish.

## Publish Direction

Public hook: alur gives you one package-manager vocabulary across projects
without replacing the package manager each project already uses.

Hold back for now: benchmark-heavy videos and exact smoke benchmark numbers.
They are useful as proof, but they make the launch feel internal.

## Tweet 1/4

Introducing alur — one command vocabulary for JavaScript projects.

Use `ni`, `nr`, `nex`, `nci` across npm, pnpm, yarn, bun, and deno.

And if you like Bun-style ergonomics, turn on the optional Node shim:

`node install vite`

Normal Node still stays normal.

Video: `marketing/alur-twitter-thread-videos/out/v7/01-node-commands.mp4`

## Tweet 2/4

alur keeps the project's package manager, but lets your commands stay the same.

`ni vite`
`nr dev`
`nex vitest`
`nci`
`nrm lodash`
`npar "lint" "test"`
`nseq "clean" "build"`

npm, yarn, pnpm, bun, deno — picked automatically from the project you're in.

Video: `marketing/alur-twitter-thread-videos/out/v7/02-detects-pm.mp4`

## Tweet 3/4

Introducing fast mode.

When alur can safely speed up a local script or tool, it skips the extra package-manager startup.

When compatibility matters, it falls back automatically.

Fast when safe. Correct when needed.

Video: `marketing/alur-twitter-thread-videos/out/v7/03-fast-delegates.mp4`

## Tweet 4/4

Introducing the optional Node shim.

Normal Node stays normal:

`node -v`
`node script.js`
`node --watch server.js`

Package commands become available through Node:

`node install`
`node run`
`node exec`

Docs: https://alur.happytoolin.com

Video: `marketing/alur-twitter-thread-videos/out/v7/04-normal-node.mp4`

## Source Notes

- Product description and command list: `README.md`
- Fallback and compatibility rules: `docs/fast-compat.md`
- Benchmark proof held for follow-up: `marketing/alur-direct-benchmark-smoke.md`
- Video size: 1280x720 landscape, 30 fps
