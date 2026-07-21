#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./scripts/extract-manifest.sh [--sync-source-fallback] [bundle-url-or-entry-url]
EOF
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
INPUT_URL="https://toonlivre.net/"
MANIFEST_DIR="$REPO_ROOT/extrator/manifest"
MANIFEST_HISTORY_DIR="$MANIFEST_DIR/history"
MANIFEST_BUNDLES_DIR="$MANIFEST_DIR/bundles"
SOURCE_FALLBACK_PATH="$REPO_ROOT/sources/pt_BR.toonlivre/res/manifest.json"
EPOCH_SECONDS="$(date +%s)"
TEMP_MANIFEST_PATH="$(mktemp)"
EXTRATOR_DIR="$REPO_ROOT/extrator"
SYNC_SOURCE_FALLBACK=0

while (($#)); do
  case "$1" in
    --sync-source-fallback)
      SYNC_SOURCE_FALLBACK=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      INPUT_URL="$1"
      shift
      ;;
  esac
done

mkdir -p "$MANIFEST_DIR" "$MANIFEST_HISTORY_DIR" "$MANIFEST_BUNDLES_DIR"
trap 'rm -f "$TEMP_MANIFEST_PATH"' EXIT

echo "[manifest] epoch-seconds: $EPOCH_SECONDS"
env -C "$EXTRATOR_DIR" npm run build >/dev/null
env -C "$EXTRATOR_DIR" \
  node dist/cli.js extract \
  --bundle-url "$INPUT_URL" \
  --out "$TEMP_MANIFEST_PATH" >/dev/null

jq -c . "$TEMP_MANIFEST_PATH" > "$MANIFEST_DIR/manifest.json"
cp "$MANIFEST_DIR/manifest.json" "$MANIFEST_HISTORY_DIR/manifest_v${EPOCH_SECONDS}.json"
BUNDLE_FILE_NAME="$(jq -r '.bundle.url // empty | split("/") | last' "$MANIFEST_DIR/manifest.json")"
if [[ -n "$BUNDLE_FILE_NAME" ]]; then
  BUNDLE_MANIFEST_NAME="${BUNDLE_FILE_NAME%.js}.json"
  cp "$MANIFEST_DIR/manifest.json" "$MANIFEST_BUNDLES_DIR/$BUNDLE_MANIFEST_NAME"
fi

if [[ "$SYNC_SOURCE_FALLBACK" == "1" ]]; then
  cp "$MANIFEST_DIR/manifest.json" "$SOURCE_FALLBACK_PATH"
  echo "[manifest] synced source fallback: $SOURCE_FALLBACK_PATH"
fi
