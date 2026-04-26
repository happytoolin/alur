# Fast-Mode Compatibility

`hni` fast mode is an optimization for common local script and local bin execution.

It is not intended to perfectly reimplement every package manager's runtime behavior.
When a project layout or shell environment is likely to be package-manager-specific, `hni` should fall back to package-manager mode instead of guessing.

## Summary

| Package manager | `nr` fast | `nlx` fast | Notes |
| --- | --- | --- | --- |
| npm | Yes | Yes | Common local `scripts` and `node_modules/.bin` flows are supported. |
| pnpm | Yes | Yes | Includes local `.bin` lookup and pnpm hoisted `.bin` paths when present. |
| yarn classic | Yes | Yes | Works with standard `node_modules/.bin` layouts. |
| yarn berry (node-modules linker) | Yes | Yes | Supported when the project exposes real filesystem bins. |
| yarn berry (PnP) | No | No | Falls back because PnP does not provide normal `node_modules/.bin` semantics. |
| bun | Yes | Yes | Supported for local script and local bin execution. |
| deno | Yes, task/local cases only | Yes, local bins only | The fast path supports nearest `deno.json{,c}` tasks, mixed `package.json` fallback, and local-bin exec. Workspaces and remote `npm:` exec fall back to package-manager mode. |

## Command Support

| Command | Fast-mode support | Notes |
| --- | --- | --- |
| `nr` | Yes, when eligible | Supports package.json lifecycle hooks where applicable, plus fast Deno task execution in Deno projects. |
| `node run` | Yes, same as `nr` | Inherits the same fast-mode checks and fallbacks. |
| `nlx` | Yes, local bins only | Uses fast execution only when a local executable can be resolved confidently. |
| `node exec` / `node x` / `node dlx` | Yes, local bins only | Inherits the same local-bin behavior as `nlx`. |
| `ni`, `nci`, `nru`, `nun`, `na` | No | These remain in package-manager mode. |
| `np`, `ns` | Already direct | These are not controlled by `fastMode`. |

## Fast-Mode Resolution Rules

For fast `nr`:

| Behavior | Status |
| --- | --- |
| nearest `package.json` script lookup | Supported |
| nearest `deno.json` / `deno.jsonc` task lookup | Supported |
| Deno config wins over same-name `package.json` script in the same project | Supported |
| Deno task object forms with `command`, `description`, `dependencies` | Supported |
| Deno wildcard task selection | Supported |
| Deno task dependencies with deduped dependency execution | Supported |
| Deno task cwd = config directory, `INIT_CWD` = invocation directory | Supported |
| forwarded args | Supported |
| `pre<script>` / `post<script>` hooks | Supported |
| `INIT_CWD`, `npm_lifecycle_event`, `npm_lifecycle_script`, `npm_execpath`, `npm_node_execpath`, `npm_package_json` | Supported |
| package-manager-specific env expansion such as `npm_package_*` / `npm_config_*` inside script bodies | Not supported |
| package-manager-specific loader environments such as Yarn PnP | Falls back |
| Deno workspace/member recursion | Not supported, falls back |
| unsupported or cyclic Deno task graphs | Falls back |

For fast `nlx`:

| Resolution source | Status |
| --- | --- |
| nearest ancestor `node_modules/.bin` | Supported |
| pnpm hoisted `.bin` under `node_modules/.pnpm/node_modules/.bin` | Supported |
| nearest package `package.json` `bin` entries | Supported |
| Deno local-bin execution in a Deno project | Supported |
| remote package fetch/exec | Not supported, falls back |

## Windows

Windows fast-mode support is intentionally conservative.

| Case | Status |
| --- | --- |
| ordinary fast `nr` script execution | Supported |
| direct executable local bins | Supported |
| `.cmd` / `.bat` local bins | Supported via `cmd /C` |
| `.ps1` local bins | Supported via PowerShell |
| `.js` / `.cjs` / `.mjs` local bins | Supported via the resolved real Node binary |
| package-manager-specific shim behavior beyond these cases | Falls back or should be run with `--pm` |

## When To Skip Fast Mode

Prefer `--pm` for:

- Yarn Berry Plug'n'Play projects
- Deno workspace projects
- projects that depend on package-manager-specific shell or env expansion
- cases where you want exact package-manager behavior for debugging
- Windows projects that rely on complex wrapper scripts or custom shell setup

Examples:

```bash
nr --pm build
nlx --pm create-vite@latest
node run --pm dev
```

## Design Principle

If fast execution is not clearly correct, `hni` should use package-manager mode instead of approximating package-manager behavior.
