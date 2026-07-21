#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./scripts/extract-manifest.sh [--sync-source-fallback] [--bundle-file <path>] [--bundle-url-hint <url>] [bundle-url-or-entry-url]
EOF
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
INPUT_URL="https://toonlivre.net/"
BUNDLE_FILE=""
BUNDLE_URL_HINT=""
MANIFEST_DIR="$REPO_ROOT/extrator/manifest"
MANIFEST_BASELINES_DIR="$MANIFEST_DIR/baselines"
SOURCE_FALLBACK_PATH="$REPO_ROOT/sources/pt_BR.toonlivre/res/manifest.json"
TEMP_MANIFEST_PATH="$(mktemp)"
EXTRATOR_DIR="$REPO_ROOT/extrator"
EXTRATOR_CLI="$EXTRATOR_DIR/dist/cli.js"
SYNC_SOURCE_FALLBACK=0

require_extrator_cli() {
  if [[ -f "$EXTRATOR_CLI" ]]; then
    return 0
  fi

  echo "[manifest] missing extrator build: $EXTRATOR_CLI" >&2
  echo "[manifest] run manually: env -C \"$EXTRATOR_DIR\" npm run build" >&2
  exit 1
}

run_extrator() {
  node "$EXTRATOR_CLI" "$@"
}

while (($#)); do
  case "$1" in
    --sync-source-fallback)
      SYNC_SOURCE_FALLBACK=1
      shift
      ;;
    --bundle-file)
      if (($# < 2)); then
        echo "[manifest] missing value for --bundle-file" >&2
        exit 1
      fi
      BUNDLE_FILE="$2"
      shift 2
      ;;
    --bundle-url-hint)
      if (($# < 2)); then
        echo "[manifest] missing value for --bundle-url-hint" >&2
        exit 1
      fi
      BUNDLE_URL_HINT="$2"
      shift 2
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

mkdir -p "$MANIFEST_DIR" "$MANIFEST_BASELINES_DIR"
trap 'rm -f "$TEMP_MANIFEST_PATH"' EXIT

require_extrator_cli

if [[ -n "$BUNDLE_FILE" ]]; then
  run_extrator extract \
    --bundle-file "$BUNDLE_FILE" \
    --out "$TEMP_MANIFEST_PATH" >/dev/null
else
  run_extrator extract \
    --bundle-url "$INPUT_URL" \
    --out "$TEMP_MANIFEST_PATH" >/dev/null
fi

if [[ -n "$BUNDLE_URL_HINT" ]]; then
  jq -c --arg bundle_url "$BUNDLE_URL_HINT" '.bundle.url = $bundle_url' "$TEMP_MANIFEST_PATH" > "$MANIFEST_DIR/manifest.json"
else
  jq -c . "$TEMP_MANIFEST_PATH" > "$MANIFEST_DIR/manifest.json"
fi

BUNDLE_FILE_NAME="$(jq -r '.bundle.url // empty | split("/") | last' "$MANIFEST_DIR/manifest.json")"
if [[ -z "$BUNDLE_FILE_NAME" && -n "$BUNDLE_FILE" ]]; then
  BUNDLE_FILE_NAME="$(basename "$BUNDLE_FILE")"
fi
BUNDLE_STEM="${BUNDLE_FILE_NAME%.js}"
if [[ -z "$BUNDLE_STEM" ]]; then
  BUNDLE_STEM="bundle-$(jq -r '.bundle.hash[0:8]' "$MANIFEST_DIR/manifest.json")"
fi
BUNDLE_MANIFEST_NAME="${BUNDLE_STEM}.json"
cp "$MANIFEST_DIR/manifest.json" "$MANIFEST_BASELINES_DIR/$BUNDLE_MANIFEST_NAME"

echo "[manifest] baseline manifest: $MANIFEST_BASELINES_DIR/$BUNDLE_MANIFEST_NAME"

if [[ "$SYNC_SOURCE_FALLBACK" == "1" ]]; then
  cp "$MANIFEST_DIR/manifest.json" "$SOURCE_FALLBACK_PATH"
  echo "[manifest] synced source fallback: $SOURCE_FALLBACK_PATH"
fi
