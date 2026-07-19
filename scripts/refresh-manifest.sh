#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
SITE_URL="${1:-https://toonlivre.net/}"
MANIFEST_PATH="$REPO_ROOT/manifest/manifest.json"
NODE_BIN="${NODE_BIN:-$(dirname "$(command -v npm)")/node}"
TSX_CLI_PATH="$REPO_ROOT/extrator/node_modules/tsx/dist/cli.mjs"
FORCE_REFRESH="${FORCE:-0}"
TEMP_PROBE_PATH="$(mktemp)"

trap 'rm -f "$TEMP_PROBE_PATH"' EXIT

if [[ ! -f "$TSX_CLI_PATH" ]]; then
  echo "[manifest] run: env -C \"$REPO_ROOT/extrator\" npm install"
  exit 1
fi

if [[ ! -f "$MANIFEST_PATH" ]]; then
  echo "[manifest] no saved manifest found; generating a new one"
elif [[ "$FORCE_REFRESH" == "1" ]]; then
  echo "[manifest] force refresh enabled"
else
  env -C "$REPO_ROOT/extrator" \
    "$NODE_BIN" node_modules/tsx/dist/cli.mjs src/cli.ts probe \
    --manifest "$MANIFEST_PATH" \
    --site-url "$SITE_URL" > "$TEMP_PROBE_PATH"

  CHANGED="$(jq -r '.changed' "$TEMP_PROBE_PATH")"
  REASON="$(jq -r '.reason' "$TEMP_PROBE_PATH")"
  ENTRY_STATUS="$(jq -r '.entryStatus' "$TEMP_PROBE_PATH")"
  MATCHED_BUNDLE_URL="$(jq -r '.matchedBundleUrl // empty' "$TEMP_PROBE_PATH")"
  CHECKED_BUNDLE_LOGS="$(jq -r '.checkedBundles[]? | "[manifest] checked-bundle status=\(.status) url=\(.url)"' "$TEMP_PROBE_PATH")"

  echo "[manifest] probe entry-status=$ENTRY_STATUS changed=$CHANGED reason=$REASON"
  if [[ -n "$MATCHED_BUNDLE_URL" ]]; then
    echo "[manifest] matched-bundle-url: $MATCHED_BUNDLE_URL"
  fi
  if [[ -n "$CHECKED_BUNDLE_LOGS" ]]; then
    printf '%s\n' "$CHECKED_BUNDLE_LOGS"
  fi

  if [[ "$CHANGED" != "true" ]]; then
    echo "[manifest] skip: live bundle still matches the saved manifest"
    exit 0
  fi
fi

"$REPO_ROOT/scripts/extract-manifest.sh" "$SITE_URL"

env -C "$REPO_ROOT/extrator" \
  "$NODE_BIN" node_modules/tsx/dist/cli.mjs src/cli.ts validate \
  --manifest "$MANIFEST_PATH" >/dev/null

echo "[manifest] refresh + validation completed"
