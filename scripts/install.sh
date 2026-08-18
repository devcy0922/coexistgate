#!/usr/bin/env bash
# Download the latest CoexistGate CLI release. Read this file before piping to bash.
set -euo pipefail
REPO="${COEXISTGATE_REPO:-devcy0922/coexistgate}"
PREFIX="${PREFIX:-/usr/local/bin}"

os=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)
case "$arch" in
  x86_64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) echo "unsupported arch: $arch" >&2; exit 1 ;;
esac
case "$os" in
  darwin) target="${arch}-apple-darwin" ;;
  linux) target="${arch}-unknown-linux-musl" ;;
  *) echo "unsupported os: $os" >&2; exit 1 ;;
esac

asset="coexistgate-${target}"
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
api="https://api.github.com/repos/${REPO}/releases/latest"
url=$(curl -fsSL "$api" | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='''${asset}'''))")
sums_url=$(curl -fsSL "$api" | python3 -c "import sys,json; r=json.load(sys.stdin); print(next((a['browser_download_url'] for a in r['assets'] if a['name']=='SHA256SUMS'), ''))")
curl -fsSL "$url" -o "$tmpdir/$asset"
if [[ -n "$sums_url" ]]; then
  curl -fsSL "$sums_url" -o "$tmpdir/SHA256SUMS"
  (cd "$tmpdir" && grep " $asset\$" SHA256SUMS | shasum -a 256 -c -)
fi
install -m 0755 "$tmpdir/$asset" "$PREFIX/coexistgate"
echo "installed $PREFIX/coexistgate"
