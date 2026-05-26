# hni

![hni og banner](.github/og-image.png)

[![CI](https://github.com/happytoolin/hni/actions/workflows/ci.yml/badge.svg)](https://github.com/happytoolin/hni/actions/workflows/ci.yml)
[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-4F46E5.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![crates.io](https://img.shields.io/crates/v/hni?logo=rust&logoColor=white)](https://crates.io/crates/hni)
[![npm](https://img.shields.io/npm/v/%40happytoolin%2Fhni?logo=npm&logoColor=white)](https://www.npmjs.com/package/@happytoolin/hni)
![npm](https://img.shields.io/badge/npm-supported-CB3837?logo=npm&logoColor=white)
![yarn](https://img.shields.io/badge/yarn-supported-2C8EBB?logo=yarn&logoColor=white)
![pnpm](https://img.shields.io/badge/pnpm-supported-F69220?logo=pnpm&logoColor=white)
![bun](https://img.shields.io/badge/bun-supported-111111?logo=bun&logoColor=white)
![deno](https://img.shields.io/badge/deno-supported-000000?logo=deno&logoColor=white)

Fast package manager routing for `npm`, `yarn`, `pnpm`, `bun`, and `deno`.

`hni` is inspired by Antfu's [`ni`](https://github.com/antfu-collective/ni#readme), but packaged as a single multicall binary with extra shell setup for a `node` shim.

`hni` is still beta software and may have bugs.

One install gives you:

- `hni`
- `ni`, `nr`, `nlx`, `nru`, `nun`, `nci`, `na`, `np`, `ns`
- `node` shim via `hni init <shell>` (shell plugin only)

## Install

### npm (global)

```bash
npm install -g @happytoolin/hni
hni --version
```

This installs `hni` and the `ni`-family aliases (`ni`, `nr`, `nlx`, `nru`, `nun`, `nci`, `na`, `np`, `ns`) onto your global npm bin path.
The `node` shim is only enabled through `hni init <shell>`.
Under the hood, npm resolves a platform-specific optional dependency package that contains the native `hni` binary.

### Homebrew

```bash
brew tap happytoolin/happytap
brew install hni
hni --version
```

### Script install (macOS / Linux)

TODO: `https://happytoolin.com/hni/install.sh` is not live yet. Use the raw GitHub script for now:

```bash
curl -fsSL https://raw.githubusercontent.com/happytoolin/hni/main/install.sh | bash
```

Optional environment variables:

- `HNI_VERSION` - install a specific version, for example `v0.0.2`
- `HNI_INSTALL_DIR` - install somewhere other than `~/.local/bin`
- `HNI_NODE=off` - disable the `node` shim for the current environment

### Script install (PowerShell)

TODO: `https://happytoolin.com/hni/install.ps1` is not live yet. Use the raw GitHub script for now:

```powershell
irm https://raw.githubusercontent.com/happytoolin/hni/main/install.ps1 | iex
```

Optional parameters:

- `-Version latest`
- `-InstallDir "$env:LOCALAPPDATA\hni\bin"`

### Deno / JSR

Install `hni`:

```bash
deno install -gA -n hni jsr:@happytoolin/hni/hni
hni --version
```

Install alias commands (example):

```bash
deno install -gA -n ni jsr:@happytoolin/hni/ni
deno install -gA -n nr jsr:@happytoolin/hni/nr
```

## Commands

### `ni`

Install dependencies or add new ones.

```bash
ni
ni vite
ni -D vitest
ni -g eslint
ni --frozen
ni --frozen-if-present
ni --interactive
```

### `nr`

Run package scripts.

```bash
nr
nr dev
nr build
nr test -- --watch
nr --if-present lint
```

### `nlx`

Execute binaries without adding them permanently to your project.

```bash
nlx vitest
nlx eslint .
nlx create-vite@latest
```

### `nru`

Upgrade dependencies.
Named `nru` to avoid shadowing Nushell's `nu` binary.

```bash
nru
nru react react-dom
nru --interactive
```

### `nun`

Remove dependencies.

```bash
nun lodash
nun react react-dom
nun --multi-select
nun -g typescript
```

### `nci`

Run a clean install. If a lockfile exists, `hni` uses the package-manager-specific frozen install command.

```bash
nci
```

### `na`

Print or forward directly to the detected package manager.

```bash
na --version
na config get registry
```

### `np` / `ns`

Run shell commands in parallel or sequentially.

```bash
np "pnpm dev" "pnpm test"
ns "pnpm lint" "pnpm test"
```

### `node`

`hni` can also act as a package-manager-aware `node` shim.
Enable it by adding `hni init <shell>` to your shell config first.

```bash
node install vite
node run dev
node exec vitest
node ci
node p "echo one" "echo two"
```

Regular Node.js usage still passes through:

```bash
node script.js
node -v
node -- --trace-warnings
```

### Utilities

```bash
hni help ni
hni completion zsh
hni init bash
hni doctor
```

## Shell Setup

If you want node-shim behavior, add the init line at the end of your shell config file, after anything that manages Node or rewrites `PATH`, such as `nvm`, `mise`, `asdf`, `fnm`, or `volta`.

Do not append the `hni` directory to the end of `PATH`. Put the init line at the end of the shell config file and let it prepend the correct path for you.

### zsh

Add to `~/.zshrc`:

```bash
eval "$(hni init zsh)"
```

### bash

Add to `~/.bashrc`:

```bash
eval "$(hni init bash)"
```

### fish

Add to `~/.config/fish/config.fish`:

```fish
hni init fish | source
```

### PowerShell

Add to `$PROFILE`:

```powershell
Invoke-Expression (& hni init powershell)
```

### Nushell

Generate a stable init file, then source it from the end of `~/.config/nushell/config.nu`:

```nu
hni init nushell | save --force ~/.config/nushell/hni.nu
source ~/.config/nushell/hni.nu
```

## Global Flags

These work across `hni` and the multicall aliases:

```bash
? --dry-run --print-command
--explain
-C <dir>
-v --version
-h --help
```

Use `--` to forward flags to the underlying package manager or script:

```bash
hni ni -- --help
nr test -- --watch
```

## Configuration

Config file:

- `~/.hnirc`

Supported keys:

```ini
defaultPackageManager=pnpm
globalPackageManager=npm
fastMode=true
```

Environment overrides:

- `HNI_CONFIG_FILE`
- `HNI_DEFAULT_PACKAGE_MANAGER`
- `HNI_GLOBAL_PACKAGE_MANAGER`
- `HNI_FAST`

## How It Works

`hni` detects the package manager from:

1. `packageManager` in `package.json`
2. lockfiles such as `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `yarn.lock`, `package-lock.json`, `bun.lockb`, or `deno.lock`
3. `devEngines.packageManager` in `package.json`
4. install metadata such as `.pnp.cjs`, `node_modules/.pnpm`, or `node_modules/.package-lock.json`
5. config defaults if detection is unavailable

Then it maps the command family to the right underlying command:

- `ni` -> install or add
- `nr` -> run or task
- `nlx` -> `npx` / `pnpm dlx` / `yarn dlx` / `bun x`
- `nru` -> update / upgrade
- `nci` -> frozen install when lockfiles exist

## Troubleshooting

### PowerShell `ni` alias conflict

PowerShell ships with a built-in `ni` alias for `New-Item`.

If that conflicts with `hni`, remove or override it in your profile before loading `hni`:

```powershell
Remove-Item Alias:ni -ErrorAction SilentlyContinue
Invoke-Expression (& hni init powershell)
```

### Check what `hni` resolved

```bash
ni vite --debug-resolved
nr dev --explain
hni doctor
```

## Benchmarking

The active benchmark suite lives in [`benchmark/`](benchmark/).

If you use [`just`](https://github.com/casey/just), the common local commands are wrapped in [`justfile`](justfile):

```bash
just build-release
just test
just test-fast
just ci
just bench
```

Run the default local benchmark with:

```bash
./benchmark/run.sh
just bench
```

Pass options through either entrypoint:

```bash
./benchmark/run.sh --track=compare
./benchmark/run.sh --track=fast
./benchmark/run.sh --track=runtime
./benchmark/run.sh --track=direct
just bench --track=direct --runs=3 --warmups=1 --no-build
```

Run the full release-style matrix with:

```bash
./benchmark/run.sh --track=all --runs=500 --warmups=50
```

Generate flamegraphs with:

```bash
./benchmark/profile.sh
```

Tracked benchmark docs:

- current snapshot: [`benchmark/LATEST.md`](benchmark/LATEST.md)
- lightweight history: [`benchmark/HISTORY.md`](benchmark/HISTORY.md)
- fast-mode compatibility: [`docs/fast-compat.md`](docs/fast-compat.md)

### Representative Results

All numbers below were measured on macOS (Apple Silicon) with the release binary, using `hyperfine` with 10 warmups and 100 measured runs per case. See [`benchmark/LATEST.md`](benchmark/LATEST.md) for the raw tracked snapshot.

**Headline:** `hni --fast` is **7.4x faster** than running package managers directly, and **4.6x faster** than `hni` in its own PM fallback mode.

#### 1. Fast mode vs PM mode (inside hni)

Fast mode bypasses the package manager CLI entirely and runs scripts / local bins natively.

| Case | PM mode | Fast mode | Speedup |
| --- | ---: | ---: | ---: |
| `nr noop (npm)` | 246 ms | 37 ms | **6.6x** |
| `nr noop (pnpm)` | 799 ms | 49 ms | **16.4x** |
| `nr noop (yarn)` | 348 ms | 38 ms | **9.3x** |
| `node run noop (pnpm)` | 956 ms | 34 ms | **28.4x** |
| `nlx hello --flag (npm)` | 288 ms | 17 ms | **17.0x** |
| `nr noop (bun)` | 70 ms | 37 ms | **1.9x** |
| `nr noop (deno)` | 80 ms | 35 ms | **2.2x** |

*Geometric mean across all package managers: **4.6x**.*

pnpm and yarn see the biggest wins because their CLIs carry the most startup overhead. Bun and Deno are already fast, so the margin is smaller (but still consistently ahead).

#### 2. hni fast vs direct package-manager usage

This is the real-world comparison: what users actually type today versus using `hni`.

| Case | Direct PM | hni --fast | Speedup |
| --- | ---: | ---: | ---: |
| `npm run noop` | 320 ms | 53 ms | **6.1x** |
| `pnpm run noop` | 749 ms | 41 ms | **18.2x** |
| `yarn run noop` | 443 ms | 34 ms | **13.0x** |
| `npx hello --flag` | 300 ms | 4.8 ms | **62.0x** |
| `pnpm exec hello --flag` | 733 ms | 8.9 ms | **82.8x** |
| `bun run noop` | 79 ms | 34 ms | **2.4x** |
| `deno task noop` | 50 ms | 34 ms | **1.5x** |

*Geometric mean: **7.4x**.*

Local bin execution is the standout feature: `npx` and `pnpm exec` spend hundreds of milliseconds resolving, validating, and bootstrapping before they even start your binary. `hni` resolves the bin once and runs it directly.

#### 3. hni vs Antfu's `ni`

For the same command-routing workload, `hni` is consistently faster:

| Case | antfu/ni | hni | Speedup |
| --- | ---: | ---: | ---: |
| `ni --version` | 149 ms | 92 ms | **1.6x** |
| `ni vite ?` | 6.0 ms | 3.6 ms | **1.7x** |
| `nr build ?` | 5.0 ms | 3.7 ms | **1.3x** |
| `nlx vitest ?` | 4.6 ms | 3.0 ms | **1.5x** |

*Geometric mean: **1.5x**.*

#### 4. Runtime comparison vs Bun and Deno

Even against native runtime task execution, `hni` holds its own:

| Case | hni | bun | deno |
| --- | ---: | ---: | ---: |
| `task noop` | 33 ms | 78 ms | 49 ms |
| `task hooks` | 90 ms | 210 ms | 77 ms |

`hni` is **2.3x faster than bun** for task execution and slightly faster than Deno for simple scripts.

### Methodology

The benchmark suite lives in [`benchmark/`](benchmark/) and uses `hyperfine` to time the release binary. It covers five angles:

- **`direct`** — normal package-manager commands (`npm run`, `pnpm exec`, etc.) vs `hni --fast`
- **`fast`** — `hni` PM mode vs `hni` fast mode (isolates the native-execution win)
- **`compare`** — `hni` vs `@antfu/ni` on CLI routing overhead
- **`runtime`** — `hni` vs `bun` vs `deno` on actual task execution time
- **`fixtures`** — real project fixtures from `tests/fixtures/` across all detection categories

Run the full matrix locally:

```bash
./benchmark/run.sh --track=all --runs=100 --warmups=10
```

Or generate flamegraphs:

```bash
./benchmark/profile.sh
```

Tracked snapshots are kept in [`benchmark/LATEST.md`](benchmark/LATEST.md).
