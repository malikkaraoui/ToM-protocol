#!/usr/bin/env bash
# Localhost smoke test for tom-stress.
# Starts a listener, runs ping + burst + ladder against it, validates output.
set -euo pipefail

PASS=0
FAIL=0
LISTENER_PID=""

# Cross-platform timeout support
run_with_timeout() {
    if command -v timeout >/dev/null 2>&1; then
        timeout 90 "$@"
    elif command -v gtimeout >/dev/null 2>&1; then
        gtimeout 90 "$@"
    else
        "$@"
    fi
}

# Resolve project root (script lives in crates/tom-stress/scripts/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
CRATE_MANIFEST="$PROJECT_ROOT/crates/tom-stress/Cargo.toml"

# Resolve binary path from cargo metadata (works with workspace/CI custom target dirs)
TARGET_DIR=$(cargo metadata --format-version 1 --no-deps --manifest-path "$CRATE_MANIFEST" 2>/dev/null | \
    python3 -c "import sys, json; print(json.load(sys.stdin)['target_directory'])" 2>/dev/null || true)

if [ -n "${TARGET_DIR:-}" ]; then
    BINARY="$TARGET_DIR/debug/tom-stress"
else
    BINARY="$PROJECT_ROOT/target/debug/tom-stress"
fi

cleanup() {
    if [ -n "$LISTENER_PID" ]; then
        kill "$LISTENER_PID" 2>/dev/null || true
        wait "$LISTENER_PID" 2>/dev/null || true
    fi
    rm -f /tmp/tom-stress-listener.jsonl /tmp/tom-stress-ping.jsonl \
          /tmp/tom-stress-burst.jsonl /tmp/tom-stress-ladder.jsonl
}
trap cleanup EXIT

check() {
    local desc="$1"
    local result="$2"
    if [ "$result" = "0" ]; then
        echo "  ✓ $desc"
        PASS=$((PASS + 1))
    else
        echo "  ✗ $desc"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== tom-stress localhost smoke test ==="
echo ""

# Build first
echo "Building tom-stress..."
cargo build -p tom-stress 2>&1 | tail -1
if [ ! -x "$BINARY" ]; then
    echo "  ✗ Binary not found at $BINARY"
    echo "  [INFO] target_directory=${TARGET_DIR:-<none>}"
    exit 1
fi
echo ""

# Start listener in background
echo "Starting listener..."
"$BINARY" --name Listener listen > /tmp/tom-stress-listener.jsonl 2>/dev/null &
LISTENER_PID=$!

# Wait for listener to emit "started" event (iroh relay connect can take a few seconds)
for i in $(seq 1 60); do
    if grep -q '"event":"started"' /tmp/tom-stress-listener.jsonl 2>/dev/null; then
        break
    fi
    sleep 0.5
done

# Extract listener's NodeId from the started event
LISTENER_ID=$(head -1 /tmp/tom-stress-listener.jsonl | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(data['id'])
" 2>/dev/null || echo "")

if [ -z "$LISTENER_ID" ]; then
    echo "  ✗ Failed to get listener NodeId"
    echo "  --- listener log ---"
    cat /tmp/tom-stress-listener.jsonl 2>/dev/null || true
    exit 1
fi
echo "  Listener ID: ${LISTENER_ID:0:16}..."
echo ""

# --- Test 1: Ping ---
echo "Test 1: Ping (3 pings, 200ms delay)"
run_with_timeout "$BINARY" --name Pinger ping --connect "$LISTENER_ID" --count 3 --delay 200 \
    > /tmp/tom-stress-ping.jsonl 2>/dev/null || true

PING_STARTED=$(grep -c '"event":"started"' /tmp/tom-stress-ping.jsonl || echo 0)
PING_EVENTS=$(grep -c '"event":"ping"' /tmp/tom-stress-ping.jsonl 2>/dev/null || true)
PING_EVENTS=${PING_EVENTS:-0}
PING_SUMMARY=$(grep -c '"event":"summary"' /tmp/tom-stress-ping.jsonl || echo 0)

check "ping: started event emitted" "$([ "$PING_STARTED" -ge 1 ] && echo 0 || echo 1)"
check "ping: summary emitted" "$([ "$PING_SUMMARY" -ge 1 ] && echo 0 || echo 1)"
echo "  [INFO] ping events observed: $PING_EVENTS"

# Validate JSON
PING_VALID=$(python3 -c "
import json, sys
lines = open('/tmp/tom-stress-ping.jsonl').readlines()
valid = sum(1 for l in lines if l.strip() and json.loads(l))
print(valid)
" 2>/dev/null || echo 0)
check "ping: all lines valid JSON ($PING_VALID lines)" "$([ "$PING_VALID" -ge 1 ] && echo 0 || echo 1)"
echo ""

# --- Test 2: Burst ---
echo "Test 2: Burst (5 envelopes, 512B payload)"
run_with_timeout "$BINARY" --name Burster burst --connect "$LISTENER_ID" --count 5 --payload-size 512 \
    > /tmp/tom-stress-burst.jsonl 2>/dev/null || true

BURST_RESULT=$(grep -c '"event":"burst_result"' /tmp/tom-stress-burst.jsonl || echo 0)
check "burst: burst_result emitted" "$([ "$BURST_RESULT" -ge 1 ] && echo 0 || echo 1)"

# Check some messages were acked
BURST_ACKED=$(python3 -c "
import json
for line in open('/tmp/tom-stress-burst.jsonl'):
    d = json.loads(line.strip())
    if d.get('event') == 'burst_result':
        print(d.get('messages_acked', 0))
        break
" 2>/dev/null || echo 0)
echo "  [INFO] burst messages_acked: $BURST_ACKED"
echo ""

# --- Test 3: Ladder ---
echo "Test 3: Ladder (2 sizes, 2 reps)"
run_with_timeout "$BINARY" --name Ladder ladder --connect "$LISTENER_ID" --sizes 1024,4096 --reps 2 --delay 200 \
    > /tmp/tom-stress-ladder.jsonl 2>/dev/null || true

LADDER_RESULTS=$(grep -c '"event":"ladder_result"' /tmp/tom-stress-ladder.jsonl || echo 0)
check "ladder: ladder_result events ($LADDER_RESULTS)" "$([ "$LADDER_RESULTS" -ge 2 ] && echo 0 || echo 1)"
echo ""

# --- Summary ---
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
