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

  changed="$(jq -r '.changed' "$TEMP_PROBE_PATH")"
  reason="$(jq -r '.reason' "$TEMP_PROBE_PATH")"
  matched_bundle_url="$(jq -r '.matchedBundleUrl // empty' "$TEMP_PROBE_PATH")"

  echo "[manifest] probe changed=$changed reason=$reason"
  if [[ -n "$matched_bundle_url" ]]; then
    echo "[manifest] matched-bundle-url: $matched_bundle_url"
  fi

  if [[ "$changed" != "true" ]]; then
    echo "[manifest] skip: live bundle still matches the saved manifest"
    exit 0
  fi
fi

"$REPO_ROOT/scripts/extract-manifest.sh" "$SITE_URL"

env -C "$REPO_ROOT/extrator" \
  "$NODE_BIN" node_modules/tsx/dist/cli.mjs src/cli.ts validate \
  --manifest "$MANIFEST_PATH" >/dev/null

echo "[manifest] refresh + validation completed"
