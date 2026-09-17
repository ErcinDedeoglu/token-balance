#!/usr/bin/env bash
# Install token-balance + tb from the latest GitHub release.
# Usage: curl -fsSL https://raw.githubusercontent.com/ErcinDedeoglu/token-balance/main/tooling/install/install.sh | sh
set -euo pipefail

REPO="${TOKEN_BALANCE_REPO:-ErcinDedeoglu/token-balance}"

die() { echo "token-balance install: $*" >&2; exit 1; }

map_target() {
  local os=$1 arch=$2
  case "$os/$arch" in
    Darwin/arm64) echo aarch64-apple-darwin ;;
    Darwin/x86_64) echo x86_64-apple-darwin ;;
    Linux/x86_64) echo x86_64-unknown-linux-gnu ;;
    Linux/aarch64|Linux/arm64) echo aarch64-unknown-linux-gnu ;;
    Windows/x86_64|Windows/X64) echo x86_64-pc-windows-msvc ;;
    Windows/ARM64|Windows/Arm64)
      echo "token-balance install: Windows ARM64 is not in GitHub releases yet. Use: cargo install --git https://github.com/${REPO} --locked" >&2
      return 1
      ;;
    *) die "unsupported platform $os $arch" ;;
  esac
}

if [[ "${1:-}" == --print-target ]]; then
  map_target "${2:?os}" "${3:?arch}"
  exit $?
fi

BIN_DIR="${TOKEN_BALANCE_BIN_DIR:-${PREFIX:+$PREFIX/bin}}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"

need() { command -v "$1" >/dev/null 2>&1 || die "need $1"; }

os=$(uname -s)
arch=$(uname -m)
case "$os" in
  MINGW*|MSYS*|CYGWIN*) die "on Windows use tooling/install/install.ps1" ;;
esac
target=$(map_target "$os" "$arch")

need tar
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -q "$1" -O "$2"; }
else
  die "need curl or wget"
fi

url="https://github.com/${REPO}/releases/latest/download/token-balance-${target}.tar.gz"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
echo "downloading $url"
fetch "$url" "$tmp/tb.tgz"
tar -tzf "$tmp/tb.tgz" | grep -qx token-balance || die "archive missing token-balance"
tar -tzf "$tmp/tb.tgz" | grep -qx tb || die "archive missing tb"
mkdir -p "$BIN_DIR"
tar -C "$BIN_DIR" -xzf "$tmp/tb.tgz"
chmod +x "$BIN_DIR/token-balance" "$BIN_DIR/tb"
echo "installed $BIN_DIR/token-balance and $BIN_DIR/tb"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "add $BIN_DIR to PATH" ;;
esac
"$BIN_DIR/tb" --version
