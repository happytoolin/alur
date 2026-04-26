#!/usr/bin/env bash
set -euo pipefail

REPO="happytoolin/hni"
DOWNLOAD_ROOT="https://happytoolin.com"
FALLBACK_DOWNLOAD_ROOT="https://github.com/${REPO}"
INSTALL_DIR="${HNI_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${HNI_VERSION:-latest}"

ALIASES=(ni nr nlx nru nun nci na np ns)

log() {
  printf '[hni] %s\n' "$*"
}

fail() {
  printf '[hni] error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

detect_shell_name() {
  local shell_name
  shell_name="${SHELL:-}"
  shell_name="${shell_name##*/}"
  printf '%s' "${shell_name,,}"
}

print_shell_setup() {
  local shell_name="$1"
  log "shell setup:"
  log "  add the hni init line at the end of your shell config, after nvm/mise/asdf/fnm/volta init"

  case "$shell_name" in
  zsh)
    log "  ~/.zshrc: eval \"\$(hni init zsh)\""
    ;;
  bash)
    log "  ~/.bashrc: eval \"\$(hni init bash)\""
    ;;
  fish)
    log "  ~/.config/fish/config.fish: hni init fish | source"
    ;;
  *)
    log "  bash/zsh: eval \"\$(hni init bash)\" or eval \"\$(hni init zsh)\""
    log "  fish: hni init fish | source"
    ;;
  esac

  log "  powershell (\$PROFILE): Invoke-Expression (& hni init powershell)"
  log "  nushell (~/.config/nushell/config.nu):"
  log "    hni init nushell > ~/.config/nushell/hni.nu"
  log "    source ~/.config/nushell/hni.nu"
}

normalize_tag() {
  local value="$1"
  if [[ "$value" == latest ]]; then
    local tag
    tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | sed -nE 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/p' | head -n1)"
    [[ -n "$tag" ]] || fail "unable to resolve latest release tag"
    printf '%s' "$tag"
    return
  fi

  if [[ "$value" == v* ]]; then
    printf '%s' "$value"
  else
    printf 'v%s' "$value"
  fi
}

resolve_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$arch" in
  x86_64 | amd64) arch="x86_64" ;;
  arm64 | aarch64) arch="aarch64" ;;
  *) fail "unsupported architecture: $arch" ;;
  esac

  case "$os" in
  Darwin) printf '%s-apple-darwin' "$arch" ;;
  Linux)
    if [[ "$arch" == "aarch64" || "$arch" == "x86_64" ]]; then
      printf '%s-unknown-linux-musl' "$arch"
    else
      fail "unsupported Linux architecture: $arch"
    fi
    ;;
  *)
    fail "unsupported operating system: $os"
    ;;
  esac
}

download_asset() {
  local tag="$1"
  local target="$2"
  local out_file="$3"
  local asset
  asset="hni-${tag}-${target}.tar.gz"

  local primary_url fallback_url
  primary_url="${DOWNLOAD_ROOT%/}/hni/releases/download/${tag}/${asset}"
  fallback_url="${FALLBACK_DOWNLOAD_ROOT}/releases/download/${tag}/${asset}"

  if curl -fsSL "$primary_url" -o "$out_file"; then
    log "downloaded ${asset} from ${DOWNLOAD_ROOT}"
    return
  fi

  log "primary URL failed, falling back to GitHub releases"
  curl -fsSL "$fallback_url" -o "$out_file" || fail "failed to download ${asset}"
}

main() {
  need_cmd curl
  need_cmd tar
  need_cmd install
  need_cmd mktemp
  need_cmd uname

  local tag target tmp_dir archive
  tag="$(normalize_tag "$VERSION")"
  target="$(resolve_target)"

  log "installing ${REPO} ${tag} for ${target}"
  mkdir -p "$INSTALL_DIR"

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT
  archive="${tmp_dir}/hni.tar.gz"

  download_asset "$tag" "$target" "$archive"
  tar -xzf "$archive" -C "$tmp_dir"
  [[ -f "${tmp_dir}/hni" ]] || fail "archive does not contain hni binary"

  install -m 0755 "${tmp_dir}/hni" "${INSTALL_DIR}/hni"

  for alias in "${ALIASES[@]}"; do
    ln -sf hni "${INSTALL_DIR}/${alias}"
  done

  log "installed to ${INSTALL_DIR}"
  if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    log "add this directory to PATH:"
    log "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  fi

  print_shell_setup "$(detect_shell_name)"
}

main "$@"
