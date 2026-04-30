#!/usr/bin/env bash
set -euo pipefail

REPO="${PACK_REPO:-Blu3Ph4ntom/pack}"
VERSION="${PACK_VERSION:-}"
INSTALL_DIR="${PACK_INSTALL_DIR:-$HOME/.local/bin}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command not found: $1" >&2
    exit 1
  }
}

need_cmd curl
need_cmd uname
need_cmd mktemp
need_cmd install

AUTH_HEADER=()
if [[ -n "${GITHUB_TOKEN:-}" ]]; then
  AUTH_HEADER=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
fi

curl_get() {
  curl -fsSL "${AUTH_HEADER[@]}" "$@"
}

if [[ -z "$VERSION" ]]; then
  VERSION="$(curl_get "https://api.github.com/repos/$REPO/releases/latest" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\([^"]*\)".*/\1/p' | head -n1)"
fi

if [[ -z "$VERSION" ]]; then
  echo "error: could not resolve latest version; set PACK_VERSION=x.y.z" >&2
  exit 1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-unknown-linux-gnu" ;;
      *) echo "error: unsupported Linux architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
      *) echo "error: unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "error: unsupported OS: $OS (use install.ps1 on Windows)" >&2
    exit 1
    ;;
esac

ASSET="pack-$TARGET"
BASE_URL="https://github.com/$REPO/releases/download/v$VERSION"
ASSET_URL="$BASE_URL/$ASSET"
SUM_URL="$BASE_URL/SHA256SUMS"

TMP_BIN="$(mktemp)"
TMP_SUM="$(mktemp)"
cleanup() {
  rm -f "$TMP_BIN" "$TMP_SUM"
}
trap cleanup EXIT

echo "Installing Pack v$VERSION ($TARGET)..."
curl_get -o "$TMP_BIN" "$ASSET_URL"
curl_get -o "$TMP_SUM" "$SUM_URL"

EXPECTED="$(grep "  $ASSET$" "$TMP_SUM" | awk '{print $1}')"
if [[ -z "$EXPECTED" ]]; then
  echo "error: checksum entry for $ASSET not found" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "$TMP_BIN" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL="$(shasum -a 256 "$TMP_BIN" | awk '{print $1}')"
else
  echo "error: need sha256sum or shasum for checksum verification" >&2
  exit 1
fi

if [[ "$ACTUAL" != "$EXPECTED" ]]; then
  echo "error: checksum verification failed" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
install -m 0755 "$TMP_BIN" "$INSTALL_DIR/pack"

echo "Pack installed to $INSTALL_DIR/pack"
if ! command -v pack >/dev/null 2>&1; then
  echo
  echo "Add this to your shell profile:"
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi
echo
"$INSTALL_DIR/pack" --version || true
