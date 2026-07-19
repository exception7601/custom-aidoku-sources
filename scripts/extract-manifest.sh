#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
INPUT_URL="${1:-https://toonlivre.net/}"
MANIFEST_DIR="$REPO_ROOT/manifest"
SOURCE_FALLBACK_PATH="$REPO_ROOT/sources/pt_BR.toonlivre/res/manifest.json"
EPOCH_SECONDS="$(date +%s)"
TEMP_MANIFEST_PATH="$(mktemp)"
EXTRATOR_DIR="$REPO_ROOT/extrator"

mkdir -p "$MANIFEST_DIR"
trap 'rm -f "$TEMP_MANIFEST_PATH"' EXIT

echo "[manifest] epoch-seconds: $EPOCH_SECONDS"

env -C "$EXTRATOR_DIR" \
  npm exec -- tsx src/cli.ts extract \
  --bundle-url "$INPUT_URL" \
  --out "$TEMP_MANIFEST_PATH" >/dev/null

jq -c . "$TEMP_MANIFEST_PATH" > "$MANIFEST_DIR/manifest.json"
cp "$MANIFEST_DIR/manifest.json" "$MANIFEST_DIR/manifest_v${EPOCH_SECONDS}.json"
cp "$MANIFEST_DIR/manifest.json" "$SOURCE_FALLBACK_PATH"
