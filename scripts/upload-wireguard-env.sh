#!/usr/bin/env bash
set -euo pipefail

env_file=".env"
repo_slug="exception7601/custom-aidoku-sources"

set -a
source "$env_file"
set +a

gh secret set WIREGUARD_PRIVATE_KEY --repo "$repo_slug" --body "$WIREGUARD_PRIVATE_KEY"
gh secret set WIREGUARD_PUBLIC_KEY --repo "$repo_slug" --body "$WIREGUARD_PUBLIC_KEY"
gh variable set WIREGUARD_ADDRESS --repo "$repo_slug" --body "$WIREGUARD_ADDRESS"
# gh variable set WIREGUARD_PEER_ADDRESS --repo "$repo_slug" --body "$WIREGUARD_PEER_ADDRESS"
gh variable set WIREGUARD_LISTEN_PORT --repo "$repo_slug" --body "$WIREGUARD_LISTEN_PORT"
gh variable set WIREGUARD_ALLOWED_IPS --repo "$repo_slug" --body "$WIREGUARD_ALLOWED_IPS"
gh variable set WIREGUARD_ENDPOINT --repo "$repo_slug" --body "$WIREGUARD_ENDPOINT"

echo "WireGuard secrets and variables uploaded to $repo_slug."
