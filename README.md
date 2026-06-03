# hni

![hni og banner](.github/og-image.png)

[![CI](https://github.com/happytoolin/hni/actions/workflows/ci.yml/badge.svg)](https://github.com/happytoolin/hni/actions/workflows/ci.yml)
[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-4F46E5.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![npm](https://img.shields.io/npm/v/%40happytoolin%2Fhni?logo=npm&logoColor=white)](https://www.npmjs.com/package/@happytoolin/hni)
![npm](https://img.shields.io/badge/npm-supported-CB3837?logo=npm&logoColor=white)
![yarn](https://img.shields.io/badge/yarn-supported-2C8EBB?logo=yarn&logoColor=white)
![pnpm](https://img.shields.io/badge/pnpm-supported-F69220?logo=pnpm&logoColor=white)
![bun](https://img.shields.io/badge/bun-supported-111111?logo=bun&logoColor=white)
![deno](https://img.shields.io/badge/deno-supported-000000?logo=deno&logoColor=white)

Fast package manager routing for `npm`, `yarn`, `pnpm`, `bun`, and `deno`.

`hni` is inspired by Antfu's [`ni`](https://github.com/antfu-collective/ni#readme), but packaged as a single multicall binary with extra shell setup for a `node` shim.

`hni` is still beta software and may have bugs.
The supported interface is the CLI; the Rust crate modules are internal and do not carry a stable API guarantee.

One install gives you:

- `hni`
- `ni`, `nr`, `nlx`, `nun`, `nci`, `np`, `ns`
- `node` shim via `hni init <shell>` (managed launcher)

## Install

### npm (global)

```bash
npm install -g @happytoolin/hni
hni --version
```

This installs `hni` and the `ni`-family aliases (`ni`, `nr`, `nlx`, `nun`, `nci`, `np`, `ns`) onto your global npm bin path.
The `node` shim is only enabled through `hni init <shell>`.
Under the hood, the npm postinstall downloads the matching native `hni` binary from the GitHub release.

### Homebrew

```bash
brew tap happytoolin/happytap
brew install hni
hni --version
```

### Script install (macOS / Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/happytoolin/hni/releases/latest/download/hni-installer.sh | sh
```

To pin a specific version:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/happytoolin/hni/releases/download/v0.0.3/hni-installer.sh | sh
```

### Script install (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/happytoolin/hni/releases/latest/download/hni-installer.ps1 | iex"
```

### CI / automation

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/happytoolin/hni/releases/download/v0.0.3/hni-installer.sh | sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$GITHUB_ENV"
```

Use the versioned release URL to pin. Use `releases/latest/download` to track the latest release.

## Enable the `node` shim

Once `hni` is installed, run `hni init` for your shell to enable the `node` shim.
This creates a managed `node` launcher (a symlink on Unix, copied executable on Windows) and outputs a PATH setup line for your shell config.

Add the output to the **end** of your shell rc file (after nvm / mise / asdf / fnm / volta init):

**zsh** (`~/.zshrc`):

```bash
eval "$(hni init zsh)"
```

**bash** (`~/.bashrc`):

```bash
eval "$(hni init bash)"
```

**fish** (`~/.config/fish/config.fish`):

```fish
hni init fish | source
```

**PowerShell** (`$PROFILE`):

```powershell
Invoke-Expression (& hni init powershell)
```

**Nushell** (`~/.config/nushell/config.nu`):

```nu
hni init nushell | save --force ~/.config/nushell/hni.nu
source ~/.config/nushell/hni.nu
```

Once added, restart your shell. `node` will route known npm verbs through hni
(`node install vite` → `ni vite`) and pass everything else through to the real Node.js.

## Commands

### Canonical `hni` commands

```bash
hni install vite
hni uninstall lodash
hni run dev
hni exec vitest
hni ci
hni parallel "pnpm dev" "pnpm test"
hni sequential "pnpm lint" "pnpm test"
```

### `ni`

Install dependencies or add new ones.

```bash
ni
ni vite
ni -D vitest
ni -g eslint
ni --frozen
ni --frozen-if-present
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

### `nun`

Uninstall dependencies.

```bash
nun lodash
nun react react-dom
nun -g typescript
```

### `nci`

Run a clean install. If a lockfile exists, `hni` uses the package-manager-specific frozen install command.

```bash
nci
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
node uninstall lodash
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

## Global Flags

These work across `hni` and the multicall aliases:

```bash
--print-command
--explain
-C <dir>
-v --version
-h --help
```

Use `--` to forward flags to the underlying package manager or script:

```bash
hni install -- --help
nr test -- --watch
```

## Configuration

Config file:

- `$XDG_CONFIG_HOME/hni/config.toml`
- macOS default: `~/Library/Application Support/hni/config.toml`

Supported keys:

```toml
default_package_manager = "pnpm"
global_package_manager = "npm"
fast_mode = true
```

Environment overrides:

- `HNI_CONFIG_FILE`
- `HNI_DEFAULT_PACKAGE_MANAGER`
- `HNI_GLOBAL_PACKAGE_MANAGER`
- `HNI_FAST_MODE`

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
- `nun` -> uninstall or remove
- `nci` -> frozen install when lockfiles exist
- `np` / `ns` -> parallel or sequential shell commands

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
ni vite --print-command
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
npm ci
npm run bench
just bench
```

Pass options through either entrypoint:

```bash
npm run bench -- --track=compare
npm run bench -- --track=fast
npm run bench -- --track=runtime
npm run bench -- --track=direct
just bench --track=direct --runs=3 --warmups=1 --no-build
```

Run the full release-style matrix with:

```bash
npm run bench -- --track=all --runs=500 --warmups=50
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

| Case                     | PM mode | Fast mode |   Speedup |
| ------------------------ | ------: | --------: | --------: |
| `nr noop (npm)`          |  246 ms |     37 ms |  **6.6x** |
| `nr noop (pnpm)`         |  799 ms |     49 ms | **16.4x** |
| `nr noop (yarn)`         |  348 ms |     38 ms |  **9.3x** |
| `node run noop (pnpm)`   |  956 ms |     34 ms | **28.4x** |
| `nlx hello --flag (npm)` |  288 ms |     17 ms | **17.0x** |
| `nr noop (bun)`          |   70 ms |     37 ms |  **1.9x** |
| `nr noop (deno)`         |   80 ms |     35 ms |  **2.2x** |

_Geometric mean across all package managers: **4.6x**._

pnpm and yarn see the biggest wins because their CLIs carry the most startup overhead. Bun and Deno are already fast, so the margin is smaller (but still consistently ahead).

#### 2. hni fast vs direct package-manager usage

This is the real-world comparison: what users actually type today versus using `hni`.

| Case                     | Direct PM | hni --fast |   Speedup |
| ------------------------ | --------: | ---------: | --------: |
| `npm run noop`           |    320 ms |      53 ms |  **6.1x** |
| `pnpm run noop`          |    749 ms |      41 ms | **18.2x** |
| `yarn run noop`          |    443 ms |      34 ms | **13.0x** |
| `npx hello --flag`       |    300 ms |     4.8 ms | **62.0x** |
| `pnpm exec hello --flag` |    733 ms |     8.9 ms | **82.8x** |
| `bun run noop`           |     79 ms |      34 ms |  **2.4x** |
| `deno task noop`         |     50 ms |      34 ms |  **1.5x** |

_Geometric mean: **7.4x**._

Local bin execution is the standout feature: `npx` and `pnpm exec` spend hundreds of milliseconds resolving, validating, and bootstrapping before they even start your binary. `hni` resolves the bin once and runs it directly.

#### 3. hni vs Antfu's `ni`

For startup/version checks, `hni` is faster:

| Case           | antfu/ni |   hni |  Speedup |
| -------------- | -------: | ----: | -------: |
| `ni --version` |   149 ms | 92 ms | **1.6x** |

_Current compare track keeps only version startup because `hni` no longer carries legacy `?` command-printing compatibility._

#### 4. Runtime comparison vs Bun and Deno

Even against native runtime task execution, `hni` holds its own:

| Case         |   hni |    bun |  deno |
| ------------ | ----: | -----: | ----: |
| `task noop`  | 33 ms |  78 ms | 49 ms |
| `task hooks` | 90 ms | 210 ms | 77 ms |

`hni` is **2.3x faster than bun** for task execution and slightly faster than Deno for simple scripts.

### Methodology

The benchmark suite lives in [`benchmark/`](benchmark/) and uses `hyperfine` to time the release binary. It covers five angles:

- **`direct`** — normal package-manager commands (`npm run`, `pnpm exec`, etc.) vs `hni --fast`
- **`fast`** — `hni` PM mode vs `hni` fast mode (isolates the native-execution win)
- **`compare`** — `hni` vs `@antfu/ni` on startup/version overhead
- **`runtime`** — `hni` vs `bun` vs `deno` on actual task execution time
- **`fixtures`** — real project fixtures from `tests/fixtures/` across all detection categories

Run the full matrix locally:

```bash
npm run bench -- --track=all --runs=100 --warmups=10
```

Or generate flamegraphs:

```bash
./benchmark/profile.sh
```

Tracked snapshots are kept in [`benchmark/LATEST.md`](benchmark/LATEST.md).
