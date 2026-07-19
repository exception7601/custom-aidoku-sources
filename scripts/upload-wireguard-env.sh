#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
env_file="${1:-$repo_root/.env}"
remote_url="$(git -C "$repo_root" remote get-url origin)"
repo_slug="${GITHUB_REPO:-}"

if [[ ! -f "$env_file" ]]; then
  echo "Missing env file: $env_file" >&2
  exit 1
fi

set -a
source "$env_file"
set +a

if [[ -z "$repo_slug" ]]; then
  case "$remote_url" in
    git@github.com:*)
      repo_slug="${remote_url#git@github.com:}"
      ;;
    https://github.com/*)
      repo_slug="${remote_url#https://github.com/}"
      ;;
    *)
      echo "Could not derive GitHub repo from origin: $remote_url" >&2
      exit 1
      ;;
  esac
  repo_slug="${repo_slug%.git}"
fi

required_vars=(
  WIREGUARD_PRIVATE_KEY
  WIREGUARD_PUBLIC_KEY
  WIREGUARD_ADDRESS
  WIREGUARD_PEER_ADDRESS
  WIREGUARD_LISTEN_PORT
  WIREGUARD_ALLOWED_IPS
  WIREGUARD_ENDPOINT
)

for var_name in "${required_vars[@]}"; do
  if [[ -z "${!var_name:-}" ]]; then
    echo "Missing required variable: $var_name" >&2
    exit 1
  fi
done

gh secret set WIREGUARD_PRIVATE_KEY --repo "$repo_slug" --body "$WIREGUARD_PRIVATE_KEY"
gh secret set WIREGUARD_PUBLIC_KEY --repo "$repo_slug" --body "$WIREGUARD_PUBLIC_KEY"
gh variable set WIREGUARD_ADDRESS --repo "$repo_slug" --body "$WIREGUARD_ADDRESS"
gh variable set WIREGUARD_PEER_ADDRESS --repo "$repo_slug" --body "$WIREGUARD_PEER_ADDRESS"
gh variable set WIREGUARD_LISTEN_PORT --repo "$repo_slug" --body "$WIREGUARD_LISTEN_PORT"
gh variable set WIREGUARD_ALLOWED_IPS --repo "$repo_slug" --body "$WIREGUARD_ALLOWED_IPS"
gh variable set WIREGUARD_ENDPOINT --repo "$repo_slug" --body "$WIREGUARD_ENDPOINT"

echo "WireGuard secrets and variables uploaded to $repo_slug."
