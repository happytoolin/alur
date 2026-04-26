#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
RESULT_DIR="$REPO_ROOT/benchmark/profiles"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required" >&2
  exit 1
fi

if ! cargo flamegraph --help >/dev/null 2>&1; then
  echo "cargo flamegraph is required. Install it with: cargo install flamegraph" >&2
  exit 1
fi

mkdir -p "$RESULT_DIR"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/hni-benchmark-profile-XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

FIXTURE="$TMP_ROOT/pnpm"
mkdir -p "$FIXTURE/node_modules/.bin"

cat > "$FIXTURE/package.json" <<'JSON'
{
  "name": "benchmark-profile-pnpm",
  "version": "1.0.0",
  "packageManager": "pnpm@9.0.0",
  "scripts": {
    "noop": "node -e \"\"",
    "hooks": "node -e \"\"",
    "prehooks": "node -e \"\"",
    "posthooks": "node -e \"\""
  }
}
JSON

printf 'lock\n' > "$FIXTURE/pnpm-lock.yaml"

timestamp() {
  date -u +"%Y-%m-%dT%H-%M-%SZ"
}

profile_case() {
  local name="$1"
  shift
  local output="$RESULT_DIR/$(timestamp)-$name.svg"
  echo "[benchmark] flamegraph: $name"
  cargo flamegraph --bin hni --output "$output" -- "$@"
  echo "[benchmark] wrote $output"
}

export HNI_SKIP_PM_CHECK=true

ITERATIONS="${HNI_PROFILE_ITERATIONS:-4000}"

profile_case pm-pnpm-resolve internal profile-loop --iterations "$ITERATIONS" nr noop -C "$FIXTURE" --pm
profile_case fast-pnpm-resolve internal profile-loop --iterations "$ITERATIONS" nr noop -C "$FIXTURE" --fast
profile_case fast-pnpm-hooks-resolve internal profile-loop --iterations "$ITERATIONS" nr hooks -C "$FIXTURE" --fast
