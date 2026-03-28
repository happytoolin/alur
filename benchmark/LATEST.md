# Latest Benchmark Snapshot

Updated: 2026-03-28T11:05:20.478Z

This file is the small release-friendly benchmark snapshot. Raw JSON stays in `benchmark/results/`.

Combined report: [benchmark-2026-03-28T11-05-20-478Z.md](results/benchmark-2026-03-28T11-05-20-478Z.md)

## Compare

Relative to `antfu`, `hni` averaged `9.30x`.

Artifacts: [compare-2026-03-28T11-04-28-182Z.md](results/compare-2026-03-28T11-04-28-182Z.md) · [compare-2026-03-28T11-04-28-182Z.json](results/compare-2026-03-28T11-04-28-182Z.json)

| Case | antfu | hni | Relative |
| --- | ---: | ---: | ---: |
| ni --version | 1172.48 ms | 112.02 ms | 10.47x |
| ni vite ? (npm) | 4.55 ms | 3.62 ms | 1.26x |
| nr build ? (pnpm) | 178.25 ms | 7.20 ms | 24.74x |
| nlx vitest ? (npm) | 164.59 ms | 7.16 ms | 22.98x |

## Native

Relative to `delegated`, `native` averaged `3.65x`.

Artifacts: [native-2026-03-28T11-04-37-421Z.md](results/native-2026-03-28T11-04-37-421Z.md) · [native-2026-03-28T11-04-37-421Z.json](results/native-2026-03-28T11-04-37-421Z.json)

| Case | delegated | native | Relative |
| --- | ---: | ---: | ---: |
| nr noop (npm) | 276.05 ms | 29.50 ms | 9.36x |
| nr hooks (npm) | 597.14 ms | 81.85 ms | 7.30x |
| node run noop (npm) | 272.51 ms | 94.07 ms | 2.90x |
| nr noop (pnpm) | 441.92 ms | 32.66 ms | 13.53x |
| nr hooks (pnpm) | 662.85 ms | 78.93 ms | 8.40x |
| node run noop (pnpm) | 442.94 ms | 92.27 ms | 4.80x |
| nr noop (yarn) | 422.93 ms | 38.47 ms | 10.99x |
| nr hooks (yarn) | 553.58 ms | 94.64 ms | 5.85x |
| node run noop (yarn) | 346.80 ms | 92.87 ms | 3.73x |
| nr noop (bun) | 36.61 ms | 30.20 ms | 1.21x |
| nr hooks (bun) | 98.96 ms | 83.90 ms | 1.18x |
| node run noop (bun) | 38.77 ms | 99.45 ms | 0.39x |
| nr noop (deno) | 31.80 ms | 23.87 ms | 1.33x |
| nr hooks (deno) | 32.02 ms | 26.38 ms | 1.21x |
| node run noop (deno) | 32.35 ms | 24.87 ms | 1.30x |
| nlx hello --flag (npm local bin) | 441.60 ms | 13.38 ms | 33.01x |

## Runtime

Relative to `hni`, `bun` averaged `0.73x`. Relative to `hni`, `deno` averaged `1.45x`.

Artifacts: [runtime-2026-03-28T11-04-38-306Z.md](results/runtime-2026-03-28T11-04-38-306Z.md) · [runtime-2026-03-28T11-04-38-306Z.json](results/runtime-2026-03-28T11-04-38-306Z.json)

| Case | hni | bun | deno |
| --- | ---: | ---: | ---: |
| task noop | 28.44 ms | 40.72 ms | 32.71 ms |
| task hooks | 80.67 ms | 106.03 ms | 33.32 ms |

## Direct

Relative to `direct`, `hni` averaged `6.12x`.

Artifacts: [direct-2026-03-28T11-04-45-526Z.md](results/direct-2026-03-28T11-04-45-526Z.md) · [direct-2026-03-28T11-04-45-526Z.json](results/direct-2026-03-28T11-04-45-526Z.json)

| Case | direct | hni | Relative |
| --- | ---: | ---: | ---: |
| task noop (npm) | 212.69 ms | 29.74 ms | 7.15x |
| task hooks (npm) | 379.76 ms | 80.41 ms | 4.72x |
| exec hello --flag (npm) | 240.28 ms | 9.55 ms | 25.16x |
| task noop (pnpm) | 382.28 ms | 31.52 ms | 12.13x |
| task hooks (pnpm) | 521.23 ms | 81.71 ms | 6.38x |
| exec hello --flag (pnpm) | 474.75 ms | 11.56 ms | 41.07x |
| task noop (yarn) | 304.64 ms | 31.82 ms | 9.57x |
| task hooks (yarn) | 367.09 ms | 84.49 ms | 4.34x |
| exec hello --flag (yarn) | 271.19 ms | 12.79 ms | 21.20x |
| task noop (bun) | 38.10 ms | 29.54 ms | 1.29x |
| task hooks (bun) | 104.07 ms | 83.54 ms | 1.25x |
| exec hello --flag (bun) | 175.36 ms | 11.38 ms | 15.41x |
| task noop (deno) | 32.20 ms | 23.62 ms | 1.36x |
| task hooks (deno) | 32.43 ms | 25.05 ms | 1.29x |

## Fixtures

Relative to `direct`, `delegated` averaged `0.93x`. Relative to `direct`, `native` averaged `4.40x`.

Artifacts: [fixtures-2026-03-28T11-05-20-477Z.md](results/fixtures-2026-03-28T11-05-20-477Z.md) · [fixtures-2026-03-28T11-05-20-477Z.json](results/fixtures-2026-03-28T11-05-20-477Z.json)

Detailed per-case results are kept in the track artifact.

