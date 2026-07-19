#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
INPUT_URL="${1:-https://toonlivre.net/}"
MANIFEST_DIR="$REPO_ROOT/manifest"
SOURCE_FALLBACK_PATH="$REPO_ROOT/sources/pt_BR.toonlivre/res/manifest.json"
EPOCH_SECONDS="$(date +%s)"
TEMP_MANIFEST_PATH="$(mktemp)"
NODE_BIN="${NODE_BIN:-$(dirname "$(command -v npm)")/node}"
TSX_CLI_PATH="$REPO_ROOT/extrator/node_modules/tsx/dist/cli.mjs"

mkdir -p "$MANIFEST_DIR"
trap 'rm -f "$TEMP_MANIFEST_PATH"' EXIT

if [[ ! -f "$TSX_CLI_PATH" ]]; then
  echo "[manifest] run: env -C \"$REPO_ROOT/extrator\" npm install"
  exit 1
fi

echo "[manifest] epoch-seconds: $EPOCH_SECONDS"

env -C "$REPO_ROOT/extrator" \
  "$NODE_BIN" node_modules/tsx/dist/cli.mjs src/cli.ts extract \
  --bundle-url "$INPUT_URL" \
  --out "$TEMP_MANIFEST_PATH" >/dev/null

jq -c . "$TEMP_MANIFEST_PATH" > "$MANIFEST_DIR/manifest.json"
cp "$MANIFEST_DIR/manifest.json" "$MANIFEST_DIR/manifest_v${EPOCH_SECONDS}.json"
cp "$MANIFEST_DIR/manifest.json" "$SOURCE_FALLBACK_PATH"
