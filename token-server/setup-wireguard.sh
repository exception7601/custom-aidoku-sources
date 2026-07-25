#!/usr/bin/env bash

set -euo pipefail

readonly CONFIG="/etc/wireguard/wg0.conf"

if [ ! -f "$CONFIG" ]; then
    echo "ERROR: Wireguard config not found at $CONFIG"
    echo "Mount ./wireguard to /etc/wireguard in docker-compose.yml"
    exit 1
fi

if [ "${WG_AUTO_UP:-true}" != "true" ]; then
    echo "WG_AUTO_UP is disabled, skipping wireguard setup."
    exit 0
fi

chmod 600 "$CONFIG"
wg-quick up wg0
wg show wg0
