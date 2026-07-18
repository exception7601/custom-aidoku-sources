#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(date +%s)"
shopt -s nullglob

for source_json in "$repo_root"/sources/*/res/source.json; do
  source_dir="$(dirname "$(dirname "$source_json")")"
  updated_source_json="$(jq --argjson version "$version" --tab '.info.version = $version' "$source_json")"
  echo "$updated_source_json" > "$source_json"

  env -C "$source_dir" cargo fmt
  env -C "$source_dir" aidoku package
  aidoku verify "$source_dir/package.aix"
done

rm -rf "$repo_root/public"

(
  cd "$repo_root"
  aidoku build sources/*/package.aix --name "Aidoku Custom Sources"
)
