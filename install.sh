#!/usr/bin/env bash
set -euo pipefail

REPO="exfoundry/memex"
BIN_NAME="memex"
INSTALL_DIR="${MEMEX_INSTALL_DIR:-$HOME/.local/bin}"

os=$(uname -s)
arch=$(uname -m)
if [[ "$os" != "Darwin" || "$arch" != "arm64" ]]; then
  echo "memex currently only ships prebuilt binaries for Apple Silicon (Darwin/arm64)."
  echo "Detected: $os/$arch"
  echo "To build from source: git clone https://github.com/$REPO && cd memex && cargo build --release"
  exit 1
fi

target="aarch64-apple-darwin"
asset="${BIN_NAME}-${target}.tar.gz"

if [[ "${MEMEX_VERSION:-}" == "" ]]; then
  latest_url="https://api.github.com/repos/$REPO/releases/latest"
  tag=$(curl -fsSL "$latest_url" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
  if [[ -z "$tag" ]]; then
    echo "Could not determine latest release tag from $latest_url"
    exit 1
  fi
else
  tag="$MEMEX_VERSION"
fi

download="https://github.com/$REPO/releases/download/$tag/$asset"
echo "Installing memex $tag → $INSTALL_DIR/$BIN_NAME"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$download" -o "$tmp/$asset"
curl -fsSL "$download.sha256" -o "$tmp/$asset.sha256" 2>/dev/null || true

if [[ -f "$tmp/$asset.sha256" ]]; then
  (cd "$tmp" && shasum -a 256 -c "$asset.sha256" >/dev/null)
fi

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$INSTALL_DIR"
install -m 0755 "$tmp/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"

echo "Done. Make sure $INSTALL_DIR is on your PATH."
"$INSTALL_DIR/$BIN_NAME" --version 2>/dev/null || true
