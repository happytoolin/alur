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

The tracked snapshot in [`benchmark/LATEST.md`](benchmark/LATEST.md) was generated from the `direct` track with `2` warmups and `10` measured runs per case.

If you only want the headline, it is this: `hni --fast` averaged `5.40x` faster than direct package-manager commands in the latest direct run.

The `fast` track compares pm mode versus fast mode inside `hni` across npm, pnpm, yarn, bun, deno, and local-bin execution. In the latest local fast run, `hni --fast` averaged `4.53x` faster than pm mode.

A few representative wins:

| Case | pm | fast | Relative |
| --- | ---: | ---: | ---: |
| `nr noop (npm)` | 217.71 ms | 29.43 ms | 7.40x |
| `nr noop (pnpm)` | 437.23 ms | 29.61 ms | 14.77x |
| `nr noop (yarn)` | 283.58 ms | 28.78 ms | 9.85x |
| `node run noop (pnpm)` | 436.81 ms | 29.99 ms | 14.56x |
| `node run noop (bun)` | 34.59 ms | 29.53 ms | 1.17x |
| `nlx hello --flag (npm local bin)` | 279.14 ms | 5.25 ms | 53.20x |

The direct track also compares normal package-manager usage (`npm run`, `pnpm exec`, `yarn`, `bun x`, `deno task`) with `hni --fast`. In the latest local direct run, `hni` averaged `5.40x` faster, including local-bin wins for `pnpm exec` (`53.20x`) and `yarn` (`16.48x`).

### Methodology

All timing uses `hyperfine` against the release binary. The suite can look at four angles:

- `direct`: normal package-manager usage versus `hni --fast`
- `fast`: pm mode versus fast mode inside `hni`
- `compare`: a small CLI-focused comparison against Antfu's `ni`
- `runtime`: a side-by-side look at `hni`, bun, and deno on a couple of task-style cases

The repo keeps the curated snapshot files rather than every intermediate result. For the current tracked result, use [`benchmark/LATEST.md`](benchmark/LATEST.md).
