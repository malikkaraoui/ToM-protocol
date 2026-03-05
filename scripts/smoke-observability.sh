#!/usr/bin/env bash
# Smoke test observabilité ToM (relay + discovery)
#
# Usage:
#   RELAY_URL=http://127.0.0.1:3340 DISCOVERY_URL=http://127.0.0.1:8080 ./scripts/smoke-observability.sh
#   RELAY_URL=http://127.0.0.1:3340 DISCOVERY_URL=http://127.0.0.1:8080 ./scripts/smoke-observability.sh --wait 30
#
# Exit codes:
#   0 = tout est healthy
#   1 = au moins un check a échoué

set -euo pipefail

RELAY_URL="${RELAY_URL:-http://127.0.0.1:3340}"
DISCOVERY_URL="${DISCOVERY_URL:-http://127.0.0.1:8080}"
CURL_TIMEOUT="${CURL_TIMEOUT:-5}"
WAIT_SECONDS=0

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

FAILURES=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --wait)
      WAIT_SECONDS="${2:-0}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

ok() {
  echo -e "${GREEN}✓${NC} $1"
}

ko() {
  echo -e "${RED}✗${NC} $1"
  FAILURES=$((FAILURES + 1))
}

fetch_json() {
  local url="$1"
  curl -fsS --max-time "$CURL_TIMEOUT" "$url"
}

assert_json_expr() {
  local payload="$1"
  local expr="$2"
  printf '%s' "$payload" | node -e '
const fs = require("node:fs");
const input = fs.readFileSync(0, "utf8");
const expr = process.argv[1];
const j = JSON.parse(input);
const ok = Function("j", `return (${expr});`)(j);
if (!ok) process.exit(2);
' "$expr"
}

check_endpoint() {
  local name="$1"
  local url="$2"
  local expr="$3"
  local deadline=$((SECONDS + WAIT_SECONDS))

  while true; do
    local payload
    if payload="$(fetch_json "$url" 2>/dev/null)"; then
      if assert_json_expr "$payload" "$expr"; then
        ok "$name ok"
        return
      fi
    fi

    if [ "$SECONDS" -ge "$deadline" ]; then
      ko "$name failed ($url)"
      return
    fi

    sleep 1
  done
}

echo -e "${BLUE}ToM observability smoke${NC}"
echo "relay     = $RELAY_URL"
echo "discovery = $DISCOVERY_URL"
echo "wait      = ${WAIT_SECONDS}s"
echo

# Relay endpoints
check_endpoint "relay /ready" "${RELAY_URL%/}/ready" "j.status === 'ok'"
check_endpoint "relay /health" "${RELAY_URL%/}/health" "j.status === 'ok'"
check_endpoint "relay /healthz" "${RELAY_URL%/}/healthz" "j.status === 'ok'"

# Discovery endpoints
check_endpoint "discovery /health" "${DISCOVERY_URL%/}/health" "j.status === 'ok'"
check_endpoint "discovery /relays" "${DISCOVERY_URL%/}/relays" "Array.isArray(j.relays) && Number.isInteger(j.ttl_seconds)"
check_endpoint "discovery /metrics" "${DISCOVERY_URL%/}/metrics" "j.status === 'ok' && j.counters && Number.isInteger(j.counters.requests_total)"
check_endpoint "discovery /status" "${DISCOVERY_URL%/}/status" "j.status === 'ok' && Array.isArray(j.relays) && Number.isInteger(j.relay_count)"

echo
if [ "$FAILURES" -eq 0 ]; then
  echo -e "${GREEN}All observability checks passed.${NC}"
  exit 0
fi

echo -e "${RED}${FAILURES} check(s) failed.${NC}"
exit 1
