#!/usr/bin/env sh
set -eu

repo="${TELLCI_REPO:-Rasalas/tellci}"
version="${TELLCI_VERSION:-latest}"
install_dir="${TELLCI_INSTALL_DIR:-${INSTALL_DIR:-}}"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "tellci install: missing required command: $1" >&2
    exit 1
  fi
}

need curl
need chmod
need mkdir
need mv
need uname

if [ -z "$install_dir" ]; then
  if [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then
    install_dir="/usr/local/bin"
  else
    install_dir="$HOME/.local/bin"
  fi
fi

os="$(uname -s)"
arch="$(uname -m)"
binary_name="tellci"

case "$os:$arch" in
  Linux:x86_64 | Linux:amd64)
    artifact="tellci-linux-x86_64"
    ;;
  Darwin:arm64 | Darwin:aarch64)
    artifact="tellci-macos-aarch64"
    ;;
  MINGW*:x86_64 | MSYS*:x86_64 | CYGWIN*:x86_64)
    artifact="tellci-windows-x86_64.exe"
    binary_name="tellci.exe"
    ;;
  *)
    echo "tellci install: unsupported platform: $os $arch" >&2
    echo "Supported release binaries: linux x86_64, macos aarch64, windows x86_64." >&2
    exit 1
    ;;
esac

if [ "$version" = "latest" ]; then
  url="https://github.com/$repo/releases/latest/download/$artifact"
else
  url="https://github.com/$repo/releases/download/$version/$artifact"
fi

mkdir -p "$install_dir"
target="$install_dir/$binary_name"

tmp="${TMPDIR:-/tmp}/tellci-install-$$"
trap 'rm -f "$tmp"' EXIT INT TERM

echo "Installing tellci from $url"
curl -fsSL "$url" -o "$tmp"
chmod +x "$tmp"
mv "$tmp" "$target"

echo "Installed tellci to $target"
if ! command -v tellci >/dev/null 2>&1; then
  echo "Add this directory to PATH if needed:"
  echo "  export PATH=\"$install_dir:\$PATH\""
fi
