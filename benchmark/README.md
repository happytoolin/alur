# Benchmark Suite

This is the single active benchmark suite for `hni`.

It replaces the older benchmark trees, which are preserved under [`benchmark/old/`](old/).

## Tracks

- `compare`: `@antfu/ni` vs `hni` for a very small CLI/startup-oriented set
- `fast`: `hni` pm mode vs `hni` fast mode
- `runtime`: `hni` vs `bun` vs `deno` for a few comparable task-running cases
- `direct`: package-manager-native commands (`npm run`, `pnpm run`, `yarn run`, `bun run`, `deno task`) plus local-bin exec flows (`npx`, `pnpm exec`, `yarn <bin>`, `bun x`) vs `hni` fast mode
- `fixtures`: direct package-manager invocation vs `hni` pm mode vs `hni` fast mode across the runnable fixture corpus in `tests/fixtures`

All timing uses `hyperfine`.

Defaults:

- `500` measured runs per case
- `50` warmups per case

## Requirements

- `cargo`
- `hyperfine`
- `npm`
- `pnpm` for `pnpm` fixture cases
- `yarn` for `yarn` fixture cases
- `bun` for `bun` fixture and runtime cases
- `deno` for `deno` fixture and runtime cases

Install `hyperfine` on macOS:

```bash
brew install hyperfine
```

Install flamegraph support:

```bash
cargo install flamegraph
```

## Run

Build release binary and run all tracks:

```bash
./benchmark/run.sh
```

Run one track:

```bash
./benchmark/run.sh --track=compare
./benchmark/run.sh --track=fast
./benchmark/run.sh --track=runtime
./benchmark/run.sh --track=direct
./benchmark/run.sh --track=fixtures
```

Smaller local run:

```bash
./benchmark/run.sh --runs=3 --warmups=1 --no-build
```

Formats:

```bash
./benchmark/run.sh --format=table
./benchmark/run.sh --format=markdown
./benchmark/run.sh --format=json
```

## Profiling

Generate flamegraphs for the default `pnpm` fast/pm cases:

```bash
./benchmark/profile.sh
```

This writes SVGs into [`benchmark/profiles/`](profiles/).

## Output

- aggregated JSON results in [`benchmark/results/`](results/)
- per-run Markdown reports in [`benchmark/results/`](results/)
- raw per-case `hyperfine` JSON in `benchmark/results/raw/<track>/`
- latest tracked snapshot in [`benchmark/LATEST.md`](LATEST.md)
- lightweight run index in [`benchmark/HISTORY.md`](HISTORY.md)
- flamegraph SVGs in [`benchmark/profiles/`](profiles/)

## Notes

- `compare` is intentionally tiny and presentational.
- `fast` is the engineering benchmark for fast-mode wins and regressions.
- `runtime` keeps `bun` and `deno` separate from the Antfu comparison so the story stays fair.
- `direct` measures the end-user value prop directly: whether `hni --fast` beats invoking the package manager the way most users normally would.
- `fixtures` uses the checked-in runnable detector fixtures and keeps detailed per-fixture output in the track artifact while the top-level snapshot stays summary-only.
- Full all-track runs prune older generated top-level result artifacts so the repo only keeps the current tracked snapshot instead of every intermediate run.
