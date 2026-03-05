#!/usr/bin/env bash
set -euo pipefail

# Script à lancer SUR le MacBook Air, dans le dossier copié depuis la clé USB.
#
# Structure attendue:
#   .
#   ├─ bin/tom-relay
#   ├─ com.tom.relay.air.plist
#   └─ install-on-air.sh

BUNDLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
USER_HOME="${HOME}"
TARGET_DIR="${USER_HOME}/tom-protocol/target/release"
LAUNCHD_DIR="${USER_HOME}/Library/LaunchAgents"
PLIST_PATH="${LAUNCHD_DIR}/com.tom.relay.air.plist"
CURRENT_USER="$(whoami)"

mkdir -p "${TARGET_DIR}" "${LAUNCHD_DIR}"

cp -f "${BUNDLE_DIR}/bin/tom-relay" "${TARGET_DIR}/tom-relay"
chmod +x "${TARGET_DIR}/tom-relay"

if [[ -f "${BUNDLE_DIR}/bin/tom-stress" ]]; then
  cp -f "${BUNDLE_DIR}/bin/tom-stress" "${TARGET_DIR}/tom-stress"
  chmod +x "${TARGET_DIR}/tom-stress"
fi

TMP_PLIST="$(mktemp)"
sed -e "s|REPLACE_USER|${CURRENT_USER}|g" "${BUNDLE_DIR}/com.tom.relay.air.plist" > "${TMP_PLIST}"
cp -f "${TMP_PLIST}" "${PLIST_PATH}"
rm -f "${TMP_PLIST}"

launchctl unload "${PLIST_PATH}" >/dev/null 2>&1 || true
launchctl load "${PLIST_PATH}"

echo "Checking relay health..."
curl -fsS "http://127.0.0.1:3341/health"
echo

echo "Checking relay metrics..."
curl -fsS "http://127.0.0.1:9091/metrics" >/dev/null

echo "OK: tom-relay installed and running on MacBook Air"
echo "Logs: /tmp/tom-relay-air.log and /tmp/tom-relay-air-error.log"
