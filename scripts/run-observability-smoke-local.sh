#!/usr/bin/env bash
# Lance un smoke observabilité local relay+discovery (usage hook pre-push)

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

RELAY_URL="${RELAY_URL:-http://127.0.0.1:3340}"
DISCOVERY_URL="${DISCOVERY_URL:-http://127.0.0.1:8080}"
WAIT_SECONDS="${WAIT_SECONDS:-30}"
RELAY_START_TIMEOUT="${RELAY_START_TIMEOUT:-240}"
DISCOVERY_START_TIMEOUT="${DISCOVERY_START_TIMEOUT:-30}"

RELAY_PID=""
DISCOVERY_PID=""
RELAYS_FILE=""

wait_for_http() {
  local url="$1"
  local timeout_secs="$2"
  local deadline=$((SECONDS + timeout_secs))
  while [ "$SECONDS" -lt "$deadline" ]; do
    if curl -fsS --max-time 2 "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

cleanup() {
  if [ -n "$DISCOVERY_PID" ] && kill -0 "$DISCOVERY_PID" 2>/dev/null; then
    kill "$DISCOVERY_PID" 2>/dev/null || true
  fi
  if [ -n "$RELAY_PID" ] && kill -0 "$RELAY_PID" 2>/dev/null; then
    kill "$RELAY_PID" 2>/dev/null || true
  fi
  if [ -n "$RELAYS_FILE" ] && [ -f "$RELAYS_FILE" ]; then
    rm -f "$RELAYS_FILE"
  fi
}
trap cleanup EXIT

# Start local relay if needed
if ! lsof -iTCP:3340 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "[pre-push] starting local relay on :3340"
  cargo run -p tom-relay --features server -- --dev >/tmp/tom-relay-prepush.log 2>&1 &
  RELAY_PID=$!
fi

if ! wait_for_http "${RELAY_URL%/}/health" "$RELAY_START_TIMEOUT"; then
  echo "[pre-push] relay health check failed (${RELAY_URL%/}/health)"
  if [ -n "$RELAY_PID" ] && ! kill -0 "$RELAY_PID" 2>/dev/null; then
    echo "[pre-push] relay process exited before becoming healthy"
  fi
  tail -n 60 /tmp/tom-relay-prepush.log 2>/dev/null || true
  exit 1
fi

# Start local discovery if needed (forced catalog with local relay)
if ! lsof -iTCP:8080 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "[pre-push] starting local relay-discovery on :8080"
  RELAYS_FILE="$(mktemp -t tom-relays.XXXXXX.json)"
  cat >"$RELAYS_FILE" <<JSON
{
  "relays": [
    { "url": "${RELAY_URL}", "region": "local", "load": 0.0, "latency_hint_ms": 1 }
  ]
}
JSON

  RELAY_DISCOVERY_PORT=8080 \
  RELAY_DISCOVERY_RELAYS_FILE="$RELAYS_FILE" \
  node tools/relay-discovery/server.mjs >/tmp/tom-discovery-prepush.log 2>&1 &
  DISCOVERY_PID=$!
fi

if ! wait_for_http "${DISCOVERY_URL%/}/health" "$DISCOVERY_START_TIMEOUT"; then
  echo "[pre-push] discovery health check failed (${DISCOVERY_URL%/}/health)"
  tail -n 60 /tmp/tom-discovery-prepush.log 2>/dev/null || true
  exit 1
fi

echo "[pre-push] running observability smoke"
RELAY_URL="$RELAY_URL" DISCOVERY_URL="$DISCOVERY_URL" ./scripts/smoke-observability.sh --wait "$WAIT_SECONDS"
