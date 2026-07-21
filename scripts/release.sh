#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(date +%s)"
shopt -s nullglob

enable_debug_logs=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug-logs)
      enable_debug_logs=1
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Usage: scripts/release.sh [--debug-logs]

Options:
  --debug-logs  Build packages with `debug_assertions` enabled.
EOF
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

package_cmd=(aidoku package)
if [[ $enable_debug_logs -eq 1 ]]; then
  echo "Building with debug logs enabled via release debug assertions."
  package_cmd=(env CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true aidoku package)
fi

for source_json in "$repo_root"/sources/*/res/source.json; do
  source_dir="$(dirname "$(dirname "$source_json")")"
  updated_source_json="$(jq --argjson version "$version" --tab '.info.version = $version' "$source_json")"
  echo "$updated_source_json" > "$source_json"

  env -C "$source_dir" cargo fmt
  env -C "$source_dir" "${package_cmd[@]}"
  aidoku verify "$source_dir/package.aix"
done

rm -rf "$repo_root/public"

(
  cd "$repo_root"
  aidoku build sources/*/package.aix --name "Aidoku Custom Sources"
)

cp "$repo_root/sources/pt_BR.toonlivre/res/manifest.json" "$repo_root/public/manifest.json"
