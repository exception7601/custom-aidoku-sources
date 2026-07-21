#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./scripts/refresh-manifest.sh [--force] [--no-commit] [--site <url>] [--sync-source-fallback]

Refreshes the ToonLivre manifest locally.
By default, the script probes the live site and only regenerates the manifest
when the bundle changed.
Use `--force` to skip the probe and regenerate immediately.
Use `--no-commit` to skip the local manifest commit.
This is the script that persists the matching live bundle snapshot into
`extrator/bundles/` and then reuses that saved file to refresh
`extrator/manifest/manifest.json`.
EOF
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
DEFAULT_SITE_URL="https://toonlivre.net/"
SITE_URL="$DEFAULT_SITE_URL"
MANIFEST_RELATIVE_PATH="extrator/manifest/manifest.json"
MANIFEST_DIRECTORY_RELATIVE_PATH="extrator/manifest"
SOURCE_FALLBACK_RELATIVE_PATH="sources/pt_BR.toonlivre/res/manifest.json"
MANIFEST_PATH="$REPO_ROOT/$MANIFEST_RELATIVE_PATH"
SOURCE_FALLBACK_PATH="$REPO_ROOT/$SOURCE_FALLBACK_RELATIVE_PATH"
EXTRATOR_DIR="$REPO_ROOT/extrator"
BUNDLES_DIR="$EXTRATOR_DIR/bundles"
FORCE_REFRESH=0
AUTO_COMMIT=1
SYNC_SOURCE_FALLBACK=0
TEMP_PROBE_PATH="$(mktemp)"
TEMP_DOWNLOAD_PATH="$(mktemp)"
BUNDLE_INPUT_URL="$SITE_URL"

trap 'rm -f "$TEMP_PROBE_PATH" "$TEMP_DOWNLOAD_PATH"' EXIT

commit_manifest_changes() {
  local manifest_changes=""

  manifest_changes="$(git -C "$REPO_ROOT" status --short -- "$MANIFEST_DIRECTORY_RELATIVE_PATH" "$SOURCE_FALLBACK_RELATIVE_PATH")"
  if [[ -z "$manifest_changes" ]]; then
    echo "[manifest] no manifest changes to commit"
    return 0
  fi

  printf '%s\n' "$manifest_changes"
  git -C "$REPO_ROOT" add "$MANIFEST_DIRECTORY_RELATIVE_PATH" "$SOURCE_FALLBACK_RELATIVE_PATH"
  git -C "$REPO_ROOT" commit -m "Refresh ToonLivre manifest"
  echo "manifest_changed=true" >> "$GITHUB_OUTPUT"
  echo "[manifest] manifest commit created"
}

commit_bundle_snapshot_changes() {
  local bundle_changes=""

  bundle_changes="$(git -C "$REPO_ROOT" status --short -- extrator/bundles)"
  if [[ -z "$bundle_changes" ]]; then
    echo "[manifest] no bundle snapshot changes to commit"
    return 0
  fi

  printf '%s\n' "$bundle_changes"
  git -C "$REPO_ROOT" add extrator/bundles
  git -C "$REPO_ROOT" commit -m "Save ToonLivre bundle snapshot"
  echo "[manifest] bundle snapshot commit created"
}

run_extrator() {
  env -C "$EXTRATOR_DIR" node dist/cli.js "$@"
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
    --sync-source-fallback)
      SYNC_SOURCE_FALLBACK=1
      shift
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

env -C "$EXTRATOR_DIR" npm run build >/dev/null

if [[ ! -f "$MANIFEST_PATH" ]]; then
  echo "[manifest] no saved manifest found; generating a new one"
elif [[ "$FORCE_REFRESH" == "1" ]]; then
  echo "[manifest] force refresh enabled"
else
  run_extrator probe \
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

  if [[ -n "$MATCHED_BUNDLE_URL" ]]; then
    BUNDLE_INPUT_URL="$MATCHED_BUNDLE_URL"
  fi
fi

echo "[manifest] downloading bundle snapshot from: $BUNDLE_INPUT_URL"
run_extrator download-bundle \
  --bundle-url "$BUNDLE_INPUT_URL" \
  --out-dir "$BUNDLES_DIR" > "$TEMP_DOWNLOAD_PATH"

SAVED_BUNDLE_FILE="$(jq -r '.bundleFile' "$TEMP_DOWNLOAD_PATH")"
SAVED_BUNDLE_URL="$(jq -r '.bundleUrl' "$TEMP_DOWNLOAD_PATH")"
REUSED_EXISTING_BUNDLE="$(jq -r '.reusedExisting // false' "$TEMP_DOWNLOAD_PATH")"

echo "[manifest] saved bundle file: $SAVED_BUNDLE_FILE"
echo "[manifest] saved bundle url: $SAVED_BUNDLE_URL"
echo "[manifest] reused existing snapshot: $REUSED_EXISTING_BUNDLE"

if [[ "$AUTO_COMMIT" == "1" ]]; then
  commit_bundle_snapshot_changes
fi

if [[ "$SYNC_SOURCE_FALLBACK" == "1" ]]; then
  "$REPO_ROOT/scripts/extract-manifest.sh" \
    --sync-source-fallback \
    --bundle-file "$SAVED_BUNDLE_FILE" \
    --bundle-url-hint "$SAVED_BUNDLE_URL"
else
  "$REPO_ROOT/scripts/extract-manifest.sh" \
    --bundle-file "$SAVED_BUNDLE_FILE" \
    --bundle-url-hint "$SAVED_BUNDLE_URL"
fi

run_extrator validate \
  --manifest "$MANIFEST_PATH" >/dev/null

echo "[manifest] refresh + validation completed"
echo "[manifest] updated: $MANIFEST_PATH"
if [[ "$SYNC_SOURCE_FALLBACK" == "1" ]]; then
  echo "[manifest] updated: $SOURCE_FALLBACK_PATH"
else
  echo "[manifest] source fallback sync skipped"
fi

if [[ "$AUTO_COMMIT" == "1" ]]; then
  commit_manifest_changes
else
  echo "[manifest] auto-commit disabled"
fi
