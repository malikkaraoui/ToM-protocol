#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   AIR_HOST=malik-air.local AIR_USER=malik ./scripts/install-mac-air-relay.sh
#
# Optional overrides:
#   AIR_REPO_PATH=/Users/<user>/tom-protocol
#   AIR_RELAY_PORT=3341
#   AIR_METRICS_PORT=9091
#   AIR_COPY_STRESS=1

: "${AIR_HOST:?AIR_HOST is required (hostname/IP of MacBook Air)}"
: "${AIR_USER:?AIR_USER is required (user on MacBook Air)}"

AIR_REPO_PATH="${AIR_REPO_PATH:-/Users/${AIR_USER}/tom-protocol}"
AIR_RELAY_PORT="${AIR_RELAY_PORT:-3341}"
AIR_METRICS_PORT="${AIR_METRICS_PORT:-9091}"
AIR_COPY_STRESS="${AIR_COPY_STRESS:-0}"

for cmd in cargo ssh scp sed mktemp curl; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing required command: ${cmd}" >&2
    exit 1
  fi
done

echo "[0/6] SSH preflight on MacBook Air (${AIR_HOST})"
ssh -o BatchMode=yes "${AIR_USER}@${AIR_HOST}" "echo 'ssh-ok' >/dev/null"

echo "[1/6] Building tom-relay locally"
cargo build --release -p tom-relay

if [[ "${AIR_COPY_STRESS}" == "1" ]]; then
  echo "[1b/6] Building tom-stress locally (optional)"
  cargo build --release -p tom-stress
fi

echo "[2/6] Copying binary to MacBook Air (${AIR_HOST})"
ssh "${AIR_USER}@${AIR_HOST}" "mkdir -p '${AIR_REPO_PATH}/target/release'"
scp "target/release/tom-relay" "${AIR_USER}@${AIR_HOST}:${AIR_REPO_PATH}/target/release/tom-relay"
ssh "${AIR_USER}@${AIR_HOST}" "chmod +x '${AIR_REPO_PATH}/target/release/tom-relay'"

if [[ "${AIR_COPY_STRESS}" == "1" ]]; then
  scp "target/release/tom-stress" "${AIR_USER}@${AIR_HOST}:${AIR_REPO_PATH}/target/release/tom-stress"
  ssh "${AIR_USER}@${AIR_HOST}" "chmod +x '${AIR_REPO_PATH}/target/release/tom-stress'"
fi

echo "[3/6] Installing launchd plist on MacBook Air"
TMP_PLIST="$(mktemp)"
sed \
  -e "s|REPLACE_USER|${AIR_USER}|g" \
  -e "s|3341|${AIR_RELAY_PORT}|g" \
  -e "s|9091|${AIR_METRICS_PORT}|g" \
  "deploy/macos/com.tom.relay.air.plist" > "${TMP_PLIST}"

scp "${TMP_PLIST}" "${AIR_USER}@${AIR_HOST}:/tmp/com.tom.relay.air.plist"
ssh "${AIR_USER}@${AIR_HOST}" "mkdir -p ~/Library/LaunchAgents && mv /tmp/com.tom.relay.air.plist ~/Library/LaunchAgents/com.tom.relay.air.plist"

rm -f "${TMP_PLIST}"

echo "[4/6] Reloading launchd service"
ssh "${AIR_USER}@${AIR_HOST}" "launchctl unload ~/Library/LaunchAgents/com.tom.relay.air.plist >/dev/null 2>&1 || true"
ssh "${AIR_USER}@${AIR_HOST}" "launchctl load ~/Library/LaunchAgents/com.tom.relay.air.plist"

echo "[5/6] Health checks on MacBook Air"
ssh "${AIR_USER}@${AIR_HOST}" "curl -fsS http://127.0.0.1:${AIR_RELAY_PORT}/health"
ssh "${AIR_USER}@${AIR_HOST}" "curl -fsS http://127.0.0.1:${AIR_METRICS_PORT}/metrics >/dev/null"

echo "[6/6] Remote service status"
ssh "${AIR_USER}@${AIR_HOST}" "launchctl list | grep -E 'com\\.tom\\.relay\\.air' || true"

echo "OK: MacBook Air relay is running on :${AIR_RELAY_PORT} (metrics :${AIR_METRICS_PORT})"
echo "Try from your MacBook Pro: curl -fsS http://${AIR_HOST}:${AIR_RELAY_PORT}/health"
