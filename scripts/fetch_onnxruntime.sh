#!/usr/bin/env bash
# Downloads the official Microsoft ONNX Runtime (CPU) release, verifies its SHA-256
# and copies the shared library into DEST_DIR (ADR-130 / D15).
#
# Used by the release workflow (library shipped inside the actx package), by CI
# (ORT_DYLIB_PATH for tests) and by developers running the neural tests locally.
#
# Usage: scripts/fetch_onnxruntime.sh <dest_dir> [windows|linux]
set -euo pipefail

ORT_VERSION="1.31.0"
WIN_ASSET="onnxruntime-win-x64-${ORT_VERSION}.zip"
WIN_SHA256="3958d8a44984160692c1b4ffd0de3cb48f8c39f9d34bb31e433db06e30b82195"
LINUX_ASSET="onnxruntime-linux-x64-${ORT_VERSION}.tgz"
LINUX_SHA256="cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57"
BASE_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}"

DEST_DIR="${1:?usage: fetch_onnxruntime.sh <dest_dir> [windows|linux]}"
PLATFORM="${2:-}"
if [ -z "$PLATFORM" ]; then
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
    *) PLATFORM="linux" ;;
  esac
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
mkdir -p "$DEST_DIR"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

if [ "$PLATFORM" = "windows" ]; then
  ASSET="$WIN_ASSET"; EXPECTED="$WIN_SHA256"
else
  ASSET="$LINUX_ASSET"; EXPECTED="$LINUX_SHA256"
fi

echo "Downloading ${ASSET}..."
curl -fsSL --retry 3 -o "$WORK_DIR/$ASSET" "$BASE_URL/$ASSET"
ACTUAL="$(sha256_of "$WORK_DIR/$ASSET")"
if [ "$ACTUAL" != "$EXPECTED" ]; then
  echo "SHA-256 mismatch for ${ASSET}: expected ${EXPECTED}, got ${ACTUAL}" >&2
  exit 1
fi

if [ "$PLATFORM" = "windows" ]; then
  if command -v unzip >/dev/null 2>&1; then
    (cd "$WORK_DIR" && unzip -q "$ASSET")
  elif command -v 7z >/dev/null 2>&1; then
    (cd "$WORK_DIR" && 7z x -y "$ASSET" >/dev/null)
  else
    WIN_WORK="$(cygpath -w "$WORK_DIR" 2>/dev/null || echo "$WORK_DIR")"
    powershell -NoProfile -Command "Expand-Archive -Path '$WIN_WORK\\$ASSET' -DestinationPath '$WIN_WORK' -Force"
  fi
  cp "$WORK_DIR/onnxruntime-win-x64-${ORT_VERSION}/lib/onnxruntime.dll" "$DEST_DIR/onnxruntime.dll"
  echo "$DEST_DIR/onnxruntime.dll"
else
  tar -xzf "$WORK_DIR/$ASSET" -C "$WORK_DIR"
  # Ship the fully versioned library under the canonical name loaded by `ort`.
  cp -L "$WORK_DIR/onnxruntime-linux-x64-${ORT_VERSION}/lib/libonnxruntime.so.${ORT_VERSION}" "$DEST_DIR/libonnxruntime.so"
  echo "$DEST_DIR/libonnxruntime.so"
fi
