#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  cat <<'USAGE' >&2
Usage: scripts/update_homebrew_formula.sh <version-without-v> <tap-repo-dir>

Required environment variables:
  MACOS_ARM64_SHA256
  MACOS_X64_SHA256
  LINUX_ARM64_SHA256
  LINUX_X64_SHA256
USAGE
  exit 1
fi

VERSION="$1"
TAP_REPO_DIR="$2"

: "${MACOS_ARM64_SHA256:?missing MACOS_ARM64_SHA256}"
: "${MACOS_X64_SHA256:?missing MACOS_X64_SHA256}"
: "${LINUX_ARM64_SHA256:?missing LINUX_ARM64_SHA256}"
: "${LINUX_X64_SHA256:?missing LINUX_X64_SHA256}"

FORMULA_DIR="${TAP_REPO_DIR}/Formula"
FORMULA_PATH="${FORMULA_DIR}/hni.rb"

mkdir -p "$FORMULA_DIR"

cat >"$FORMULA_PATH" <<EOF
class Hni < Formula
  desc "ni-compatible package manager command router with node shim"
  homepage "https://github.com/happytoolin/hni"
  version "${VERSION}"
  license "GPL-3.0"

  on_macos do
    on_arm do
      url "https://github.com/happytoolin/hni/releases/download/v#{version}/hni-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "${MACOS_ARM64_SHA256}"
    end
    on_intel do
      url "https://github.com/happytoolin/hni/releases/download/v#{version}/hni-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "${MACOS_X64_SHA256}"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/happytoolin/hni/releases/download/v#{version}/hni-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "${LINUX_ARM64_SHA256}"
    end
    on_intel do
      url "https://github.com/happytoolin/hni/releases/download/v#{version}/hni-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "${LINUX_X64_SHA256}"
    end
  end

  def install
    bin.install "hni"
    %w[ni nr nlx nru nun nci na np ns].each do |name|
      bin.install_symlink bin/"hni" => name
    end
    generate_completions_from_executable(bin/"hni", "completion", shells: [:bash, :zsh, :fish])
  end

  def caveats
    <<~EOS
      Add the hni init line at the end of your shell config, after nvm/mise/asdf/fnm/volta init:

        zsh:  eval "\$(hni init zsh)"
        bash: eval "\$(hni init bash)"
        fish: hni init fish | source
        pwsh: Invoke-Expression (& hni init powershell)

      Nushell:

        hni init nushell | save --force ~/.config/nushell/hni.nu
        source ~/.config/nushell/hni.nu
    EOS
  end

  test do
    assert_match "hni", shell_output("#{bin}/hni --version")
    assert_match "usage", shell_output("#{bin}/hni --help").downcase
  end
end
EOF

echo "Updated ${FORMULA_PATH}"
