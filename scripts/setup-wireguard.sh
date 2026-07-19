#!/usr/bin/env bash

set -euo pipefail

readonly CONFIG="/etc/wireguard/wg0.conf"

sudo apt-get update
sudo apt-get install -y --no-install-recommends wireguard-tools
sudo install -d -m 700 /etc/wireguard

{
    echo "[Interface]"
    echo "PrivateKey = ${WIREGUARD_PRIVATE_KEY}"
    echo "Address = ${WIREGUARD_ADDRESS}"
    echo "DNS = 1.1.1.1, 1.0.0.1, 2606:4700:4700::1111, 2606:4700:4700::1001"
    echo "MTU = 1280"

    echo
    echo "[Peer]"
    echo "PublicKey = ${WIREGUARD_PUBLIC_KEY}"
    echo "AllowedIPs = ${WIREGUARD_ALLOWED_IPS}"
    echo "Endpoint = ${WIREGUARD_ENDPOINT}"
    echo "PersistentKeepalive = 25"

} | sudo tee "$CONFIG" > /dev/null

sudo chmod 600 "$CONFIG"

sudo wg-quick up wg0
sudo wg show wg0
