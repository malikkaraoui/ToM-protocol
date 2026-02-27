#!/usr/bin/env bash
#
# ToM Protocol - iroh PoC: Automated localhost NAT test
#
# Runs nat-test listener + connector on the same machine to verify:
# - Binary compiles and runs correctly
# - JSON events are emitted (started, ping, path_change, summary)
# - Ping/pong works end-to-end
#
# Usage:
#   ./scripts/test-localhost.sh           # default: 5 pings
#   ./scripts/test-localhost.sh --pings 10
#
# Exit codes: 0 = pass, 1 = fail

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PINGS="${1:-5}"
DELAY=1000

# Cross-platform timeout: use gtimeout on macOS if available
if command -v timeout &>/dev/null; then
    TIMEOUT_CMD="timeout 60"
elif command -v gtimeout &>/dev/null; then
    TIMEOUT_CMD="gtimeout 60"
else
    TIMEOUT_CMD=""
fi

# Temp files
LISTENER_LOG=$(mktemp)
LISTENER_ERR=$(mktemp)
CONNECTOR_LOG=$(mktemp)
CONNECTOR_ERR=$(mktemp)

cleanup() {
    [[ -n "${LISTENER_PID:-}" ]] && kill "$LISTENER_PID" 2>/dev/null || true
    wait "$LISTENER_PID" 2>/dev/null || true
    rm -f "$LISTENER_LOG" "$LISTENER_ERR" "$CONNECTOR_LOG" "$CONNECTOR_ERR"
}
trap cleanup EXIT

PASS=0
FAIL=0

check() {
    local desc="$1"
    local result="$2"
    if [[ "$result" == "true" ]]; then
        echo "  [PASS] $desc"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] $desc"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== ToM PoC: Localhost NAT Test ==="
echo "  Dir:   $POC_DIR"
echo "  Pings: $PINGS"
echo ""

# --- Step 1: Build ---
echo "[1/4] Building nat-test..."
if cargo build --bin nat-test --manifest-path "$POC_DIR/Cargo.toml" 2>/dev/null; then
    echo "  [PASS] Build succeeded"
    PASS=$((PASS + 1))
else
    echo "  [FAIL] Build failed"
    exit 1
fi

# Resolve binary path robustly (workspace, CARGO_TARGET_DIR, CI cache layouts)
TARGET_DIR=$(cargo metadata --format-version 1 --no-deps --manifest-path "$POC_DIR/Cargo.toml" 2>/dev/null | \
    python3 -c "import sys, json; print(json.load(sys.stdin)['target_directory'])" 2>/dev/null || true)

if [[ -n "$TARGET_DIR" ]]; then
    NAT_TEST="$TARGET_DIR/debug/nat-test"
else
    # Fallback: keep legacy heuristics
    WORKSPACE_ROOT="$(cd "$POC_DIR/../.." && pwd)"
    if [[ -f "$WORKSPACE_ROOT/Cargo.toml" ]] && grep -q '\[workspace\]' "$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null; then
        NAT_TEST="$WORKSPACE_ROOT/target/debug/nat-test"
    else
        NAT_TEST="$POC_DIR/target/debug/nat-test"
    fi
fi

if [[ ! -x "$NAT_TEST" ]]; then
    # Last-resort search to survive unusual target-dir setups in CI
    NAT_TEST=$(find "${CARGO_TARGET_DIR:-$POC_DIR/target}" "$POC_DIR/target" "$POC_DIR/../../target" \
        -type f -name nat-test -perm -111 2>/dev/null | head -1 || true)
fi

if [[ -z "$NAT_TEST" ]] || [[ ! -x "$NAT_TEST" ]]; then
    echo "  [FAIL] nat-test binary not found after build"
    echo "  [INFO] target_directory from metadata: ${TARGET_DIR:-<none>}"
    exit 1
fi

# --- Step 2: Start listener ---
echo "[2/4] Starting listener..."
"$NAT_TEST" --listen --name Listener > "$LISTENER_LOG" 2>"$LISTENER_ERR" &
LISTENER_PID=$!

# Wait for listener to emit started event
for i in $(seq 1 15); do
    if grep -q '"event":"started"' "$LISTENER_LOG" 2>/dev/null; then
        break
    fi
    sleep 1
done

LISTENER_ID=$(grep '"event":"started"' "$LISTENER_LOG" 2>/dev/null | head -1 | \
    python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")

if [[ -z "$LISTENER_ID" ]]; then
    echo "  [FAIL] Listener failed to start"
    echo "  --- stderr ---"
    cat "$LISTENER_ERR"
    exit 1
fi
echo "  [PASS] Listener started: ${LISTENER_ID:0:12}..."
PASS=$((PASS + 1))

# --- Step 3: Run connector ---
echo "[3/4] Running connector ($PINGS pings, ${DELAY}ms delay)..."
$TIMEOUT_CMD "$NAT_TEST" \
    --connect "$LISTENER_ID" --name Connector \
    --pings "$PINGS" --delay "$DELAY" \
    > "$CONNECTOR_LOG" 2>"$CONNECTOR_ERR" || true

# --- Step 4: Validate JSON output ---
echo "[4/4] Validating results..."

# Check started event
HAS_STARTED=$(grep -q '"event":"started"' "$CONNECTOR_LOG" 2>/dev/null && echo "true" || echo "false")
check "Connector emits 'started' event" "$HAS_STARTED"

# Check ping events
PING_COUNT=$(grep -c '"event":"ping"' "$CONNECTOR_LOG" 2>/dev/null || echo "0")
HAS_PINGS=$([[ "$PING_COUNT" -gt 0 ]] && echo "true" || echo "false")
check "Ping events emitted ($PING_COUNT pings)" "$HAS_PINGS"

# Check summary event
HAS_SUMMARY=$(grep -q '"event":"summary"' "$CONNECTOR_LOG" 2>/dev/null && echo "true" || echo "false")
check "Summary event emitted" "$HAS_SUMMARY"

# Parse summary for details
if [[ "$HAS_SUMMARY" == "true" ]]; then
    SUMMARY_LINE=$(grep '"event":"summary"' "$CONNECTOR_LOG")
    SUCCESSFUL=$(echo "$SUMMARY_LINE" | python3 -c "import sys,json; print(json.load(sys.stdin)['successful_pings'])" 2>/dev/null || echo "0")
    DIRECT_PCT=$(echo "$SUMMARY_LINE" | python3 -c "import sys,json; print(f\"{json.load(sys.stdin)['direct_pct']:.0f}\")" 2>/dev/null || echo "?")
    RECONNECTIONS=$(echo "$SUMMARY_LINE" | python3 -c "import sys,json; print(json.load(sys.stdin).get('reconnections', 0))" 2>/dev/null || echo "0")

    HAS_SUCCESSFUL=$([[ "$SUCCESSFUL" -gt 0 ]] && echo "true" || echo "false")
    check "At least 1 successful ping ($SUCCESSFUL/$PINGS)" "$HAS_SUCCESSFUL"
    echo "  [INFO] Direct: ${DIRECT_PCT}%, Reconnections: $RECONNECTIONS"
fi

# Check path_change events (may or may not happen on localhost)
PATH_CHANGES=$(grep -c '"event":"path_change"' "$CONNECTOR_LOG" 2>/dev/null || echo "0")
echo "  [INFO] Path change events: $PATH_CHANGES"

# Check all JSON lines are valid
TOTAL_LINES=$(wc -l < "$CONNECTOR_LOG" | tr -d ' ')
VALID_JSON=0
while IFS= read -r line; do
    if echo "$line" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        VALID_JSON=$((VALID_JSON + 1))
    fi
done < "$CONNECTOR_LOG"
ALL_VALID=$([[ "$VALID_JSON" -eq "$TOTAL_LINES" && "$TOTAL_LINES" -gt 0 ]] && echo "true" || echo "false")
check "All output lines are valid JSON ($VALID_JSON/$TOTAL_LINES)" "$ALL_VALID"

# --- Results ---
echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
if [[ "$FAIL" -gt 0 ]]; then
    echo ""
    echo "--- Connector stdout ---"
    cat "$CONNECTOR_LOG"
    echo "--- Connector stderr ---"
    cat "$CONNECTOR_ERR"
    exit 1
fi

exit 0
