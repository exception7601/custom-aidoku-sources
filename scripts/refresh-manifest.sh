#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./scripts/refresh-manifest.sh [--force] [--no-commit] [--site <url>]

Refreshes the ToonLivre manifest locally.
By default, the script probes the live site and only regenerates the manifest
when the bundle changed.
Use `--force` to skip the probe and regenerate immediately.
Use `--no-commit` to skip the local manifest commit.
EOF
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
DEFAULT_SITE_URL="https://toonlivre.net/"
SITE_URL="$DEFAULT_SITE_URL"
MANIFEST_RELATIVE_PATH="manifest/manifest.json"
SOURCE_FALLBACK_RELATIVE_PATH="sources/pt_BR.toonlivre/res/manifest.json"
MANIFEST_PATH="$REPO_ROOT/$MANIFEST_RELATIVE_PATH"
SOURCE_FALLBACK_PATH="$REPO_ROOT/$SOURCE_FALLBACK_RELATIVE_PATH"
NODE_BIN="${NODE_BIN:-$(dirname "$(command -v npm)")/node}"
TSX_CLI_PATH="$REPO_ROOT/extrator/node_modules/tsx/dist/cli.mjs"
FORCE_REFRESH=0
AUTO_COMMIT=1
TEMP_PROBE_PATH="$(mktemp)"

trap 'rm -f "$TEMP_PROBE_PATH"' EXIT

commit_manifest_changes() {
  local MANIFEST_CHANGES=""

  MANIFEST_CHANGES="$(git -C "$REPO_ROOT" status --short -- manifest "$SOURCE_FALLBACK_RELATIVE_PATH")"
  if [[ -z "$MANIFEST_CHANGES" ]]; then
    echo "[manifest] no manifest changes to commit"
    return 0
  fi

  printf '%s\n' "$MANIFEST_CHANGES"
  git -C "$REPO_ROOT" add manifest "$SOURCE_FALLBACK_RELATIVE_PATH"
  git -C "$REPO_ROOT" commit -m "Refresh ToonLivre manifest"
  echo "[manifest] local commit created"
}

while (($#)); do
  case "$1" in
    --force)
      FORCE_REFRESH=1
      shift
      ;;
    --no-commit)
      AUTO_COMMIT=0
      shift
      ;;
    --site)
      if (($# < 2)); then
        echo "[manifest] missing value for --site" >&2
        exit 1
      fi
      SITE_URL="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[manifest] unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "$TSX_CLI_PATH" ]]; then
  echo "[manifest] installing extractor dependencies"
  env -C "$REPO_ROOT/extrator" npm ci
fi

if [[ ! -f "$TSX_CLI_PATH" ]]; then
  echo "[manifest] unable to locate tsx CLI after dependency install" >&2
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
    echo "[manifest] tip: rerun with --force to regenerate anyway"
    exit 0
  fi
fi

"$REPO_ROOT/scripts/extract-manifest.sh" "$SITE_URL"

env -C "$REPO_ROOT/extrator" \
  "$NODE_BIN" node_modules/tsx/dist/cli.mjs src/cli.ts validate \
  --manifest "$MANIFEST_PATH" >/dev/null

echo "[manifest] refresh + validation completed"
echo "[manifest] updated: $MANIFEST_PATH"
echo "[manifest] updated: $SOURCE_FALLBACK_PATH"

if [[ "$AUTO_COMMIT" == "1" ]]; then
  commit_manifest_changes
else
  echo "[manifest] auto-commit disabled"
fi
