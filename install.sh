#!/usr/bin/env sh
# memex installer — downloads the latest release binary for the current
# platform and places it in ~/.local/bin (or $MEMEX_INSTALL_DIR).
#
# Usage:
#   curl -LsSf https://raw.githubusercontent.com/exfoundry/memex/main/install.sh | sh
#
# Env overrides:
#   MEMEX_INSTALL_DIR — target directory (default: $HOME/.local/bin)
#   MEMEX_VERSION     — tag to install (default: latest)

set -eu

REPO="exfoundry/memex"
BIN_NAME="memex"
INSTALL_DIR="${MEMEX_INSTALL_DIR:-$HOME/.local/bin}"

err() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

# ---------- detect platform ----------

uname_s=$(uname -s)
uname_m=$(uname -m)

case "$uname_s" in
    Darwin) os=apple-darwin ;;
    Linux)  os=unknown-linux-gnu ;;
    *) err "unsupported OS: $uname_s (supported: macOS, Linux)" ;;
esac

case "$uname_m" in
    arm64|aarch64) arch=aarch64 ;;
    x86_64|amd64)  arch=x86_64 ;;
    *) err "unsupported architecture: $uname_m (supported: arm64, x86_64)" ;;
esac

target="${arch}-${os}"

# Only these combinations have prebuilt binaries today.
case "$target" in
    aarch64-apple-darwin|x86_64-unknown-linux-gnu) : ;;
    *) err "no prebuilt binary for $target. Build from source via 'cargo install --path .'" ;;
esac

# ---------- resolve version ----------

if [ "${MEMEX_VERSION:-}" = "" ] || [ "${MEMEX_VERSION:-}" = "latest" ]; then
    tag_url="https://api.github.com/repos/$REPO/releases/latest"
    tag=$(curl -fsSL "$tag_url" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
    [ -n "$tag" ] || err "could not resolve latest release tag"
else
    tag="$MEMEX_VERSION"
fi

asset="${BIN_NAME}-${target}.tar.gz"
url="https://github.com/$REPO/releases/download/$tag/$asset"

# ---------- download, verify, install ----------

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

printf 'Installing %s %s (%s) → %s/%s\n' "$BIN_NAME" "$tag" "$target" "$INSTALL_DIR" "$BIN_NAME"
curl -fsSL "$url" -o "$tmp/$asset" || err "download failed: $url"

# Optional checksum verification.
if curl -fsSL "${url}.sha256" -o "$tmp/$asset.sha256" 2>/dev/null; then
    expected=$(awk '{print $1}' "$tmp/$asset.sha256")
    if command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$tmp/$asset" | awk '{print $1}')
    elif command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$tmp/$asset" | awk '{print $1}')
    else
        actual=""
    fi
    if [ -n "$actual" ] && [ "$actual" != "$expected" ]; then
        err "checksum mismatch: expected $expected, got $actual"
    fi
fi

tar -xzf "$tmp/$asset" -C "$tmp"
[ -f "$tmp/$BIN_NAME" ] || err "archive did not contain '$BIN_NAME' binary"

mkdir -p "$INSTALL_DIR"
mv "$tmp/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
chmod +x "$INSTALL_DIR/$BIN_NAME"

printf 'Installed: '
"$INSTALL_DIR/$BIN_NAME" --version 2>/dev/null || printf '%s\n' "$INSTALL_DIR/$BIN_NAME"

# ---------- PATH hint ----------

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        printf '\nNote: %s is not in your PATH.\n' "$INSTALL_DIR"
        printf 'Add this to your shell profile:\n  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
        ;;
esac
