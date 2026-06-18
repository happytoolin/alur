# alur direct benchmark smoke snapshot

Generated locally for the Twitter draft on 2026-06-18.

Command:

```bash
npm run bench -- --track=direct --runs=10 --warmups=1 --format=markdown
```

This is a marketing-draft smoke snapshot, not the repo's tracked release
benchmark. The `direct` track compares native package-manager commands
(`npm run`, `pnpm run`, `yarn run`, `bun run`, `deno task`, and local-bin exec
flows) against `alur --fast`.

Relative to `direct`, `alur` averaged `4.71x`.

| Case | direct | alur | Relative |
| --- | ---: | ---: | ---: |
| task noop (npm) | 166.27 ms | 49.29 ms | 3.37x |
| task hooks (npm) | 259.10 ms | 135.90 ms | 1.91x |
| exec hello --flag (npm) | 184.83 ms | 5.38 ms | 34.34x |
| task noop (pnpm) | 564.10 ms | 52.94 ms | 10.66x |
| task hooks (pnpm) | 653.37 ms | 138.72 ms | 4.71x |
| exec hello --flag (pnpm) | 625.21 ms | 9.36 ms | 66.78x |
| task noop (yarn) | 321.04 ms | 48.53 ms | 6.61x |
| task hooks (yarn) | 425.64 ms | 142.59 ms | 2.99x |
| exec hello --flag (yarn) | 146.10 ms | 5.74 ms | 25.45x |
| task noop (bun) | 60.91 ms | 47.80 ms | 1.27x |
| task hooks (bun) | 156.96 ms | 141.18 ms | 1.11x |
| exec hello --flag (bun) | 13.11 ms | 5.73 ms | 2.29x |
| task noop (deno) | 39.79 ms | 27.22 ms | 1.46x |
| task hooks (deno) | 39.96 ms | 26.30 ms | 1.52x |

Executed cases: 14. Skipped cases: 0.
