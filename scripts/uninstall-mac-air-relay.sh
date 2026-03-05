#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   AIR_HOST=malik-air.local AIR_USER=malik ./scripts/uninstall-mac-air-relay.sh

: "${AIR_HOST:?AIR_HOST is required (hostname/IP of MacBook Air)}"
: "${AIR_USER:?AIR_USER is required (user on MacBook Air)}"

echo "Stopping and removing com.tom.relay.air on ${AIR_HOST}"
ssh "${AIR_USER}@${AIR_HOST}" "launchctl unload ~/Library/LaunchAgents/com.tom.relay.air.plist >/dev/null 2>&1 || true"
ssh "${AIR_USER}@${AIR_HOST}" "rm -f ~/Library/LaunchAgents/com.tom.relay.air.plist"

echo "Done. Logs are left in /tmp/tom-relay-air*.log on MacBook Air."
