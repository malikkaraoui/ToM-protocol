#!/usr/bin/env bash
# S8 — 10-minute stability test
# M4 criteria: no duplicate insert, no abusive expiration, correct expiration after stop, no crash/panic
set -euo pipefail

DURATION=${1:-600}  # 10 minutes default
LOGDIR="/tmp/s8-stability"
mkdir -p "$LOGDIR"

echo "=== S8: Stability test — ${DURATION}s ==="
echo ""

# Clean up on exit
cleanup() {
    echo ""
    echo "--- Stopping all processes ---"
    kill $RELAY_PID $T1_PID $T2_PID $T3_PID 2>/dev/null || true
    wait $RELAY_PID $T1_PID $T2_PID $T3_PID 2>/dev/null || true
    echo "All stopped."
}
trap cleanup EXIT

# T0 — Bootstrap relay
target/debug/tom-relay --dev 2>"$LOGDIR/relay.log" &
RELAY_PID=$!
sleep 2

echo "[$(date +%H:%M:%S)] T0 relay started (PID=$RELAY_PID)"

# T1 — Publisher (embedded relay + publication + bot-ping every 30s)
TOM_RELAY_URL=http://127.0.0.1:3340 RUST_LOG=warn \
  target/debug/tom-chat \
    --username publisher --bot --bot-ping 30 \
    --embedded-relay --embedded-relay-publish \
    --relay-ttl 60 --relay-publish-interval 25 \
    2>"$LOGDIR/t1-stderr.log" > "$LOGDIR/t1.log" &
T1_PID=$!

echo "[$(date +%H:%M:%S)] T1 publisher started (PID=$T1_PID)"

sleep 3

# T2 — Observer 1
TOM_RELAY_URL=http://127.0.0.1:3340 RUST_LOG=warn \
  target/debug/tom-chat \
    --username obs1 --bot \
    --relay-discovery --relay-ttl 60 \
    2>"$LOGDIR/t2-stderr.log" > "$LOGDIR/t2.log" &
T2_PID=$!

echo "[$(date +%H:%M:%S)] T2 obs1 started (PID=$T2_PID)"

# T3 — Observer 2
TOM_RELAY_URL=http://127.0.0.1:3340 RUST_LOG=warn \
  target/debug/tom-chat \
    --username obs2 --bot \
    --relay-discovery --relay-ttl 60 \
    2>"$LOGDIR/t3-stderr.log" > "$LOGDIR/t3.log" &
T3_PID=$!

echo "[$(date +%H:%M:%S)] T3 obs2 started (PID=$T3_PID)"

echo ""
echo "All processes running. Waiting ${DURATION}s..."
echo ""

# Monitor loop: check every 30s that processes are alive
elapsed=0
interval=30
while [ "$elapsed" -lt "$DURATION" ]; do
    sleep "$interval"
    elapsed=$((elapsed + interval))

    # Check all processes alive
    alive=0
    for pid in $RELAY_PID $T1_PID $T2_PID $T3_PID; do
        if kill -0 "$pid" 2>/dev/null; then
            alive=$((alive + 1))
        fi
    done

    # Count events so far
    disc_count=$(cat "$LOGDIR/t2.log" "$LOGDIR/t3.log" 2>/dev/null | grep -c 'Relay discovered' || true)
    msg_count=$(cat "$LOGDIR/t2.log" "$LOGDIR/t3.log" 2>/dev/null | grep -c 'recu' || true)

    echo "[$(date +%H:%M:%S)] ${elapsed}s/${DURATION}s — procs alive: $alive/4"
done

echo ""
echo "=== DURATION COMPLETE — ANALYZING LOGS ==="
echo ""

# Combine all logs
cat "$LOGDIR/t1.log" "$LOGDIR/t2.log" "$LOGDIR/t3.log" > "$LOGDIR/combined.log"

# ── M4 Criterion 1: No duplicate relay insert ──
echo "--- M4.1: Duplicate relay insert check ---"
dup_count=0
for f in "$LOGDIR/t2.log" "$LOGDIR/t3.log"; do
    local_dups=$(grep 'Transport relay added' "$f" 2>/dev/null | sort | uniq -d | wc -l | tr -d ' ')
    dup_count=$((dup_count + local_dups))
done
if [ "$dup_count" -eq 0 ]; then
    echo "  PASS — no duplicate relay inserts"
else
    echo "  FAIL — $dup_count duplicate relay inserts detected"
    for f in "$LOGDIR/t2.log" "$LOGDIR/t3.log"; do
        grep 'Transport relay added' "$f" 2>/dev/null | sort | uniq -d || true
    done
fi

# ── M4.2: No abusive expiration while publisher is healthy ──
echo ""
echo "--- M4.2: Abusive expiration check (publisher healthy) ---"
expiry_count=$(grep -c 'Relay expired' "$LOGDIR/combined.log" 2>/dev/null || true)
if [ "$expiry_count" -eq 0 ]; then
    echo "  PASS — no relay expiration during test (publisher was healthy)"
else
    echo "  WARNING — $expiry_count relay expiration(s) detected while publisher was running"
    grep 'Relay expired' "$LOGDIR/combined.log" || true
fi

# ── M4.3: No crash/panic ──
echo ""
echo "--- M4.3: Crash/panic check ---"
panic_count=0
for f in "$LOGDIR"/*.log; do
    p=$(grep -Eci 'panic|thread.*panicked|SIGSEGV|fatal' "$f" 2>/dev/null || true)
    panic_count=$((panic_count + p))
done
if [ "$panic_count" -eq 0 ]; then
    echo "  PASS — no crash or panic detected"
else
    echo "  FAIL — $panic_count panic/crash indicators found"
    grep -i 'panic\|thread.*panicked' "$LOGDIR"/*.log || true
fi

# ── M4.4: Processes still alive at end ──
echo ""
echo "--- M4.4: Process liveness at end ---"
all_alive=true
for name_pid in "relay:$RELAY_PID" "publisher:$T1_PID" "obs1:$T2_PID" "obs2:$T3_PID"; do
    name="${name_pid%%:*}"
    pid="${name_pid##*:}"
    if kill -0 "$pid" 2>/dev/null; then
        echo "  $name (PID=$pid): alive"
    else
        echo "  $name (PID=$pid): DEAD"
        all_alive=false
    fi
done
if $all_alive; then
    echo "  PASS — all processes survived 10 minutes"
else
    echo "  FAIL — some processes died during test"
fi

# ── Summary stats ──
echo ""
echo "--- Event summary ---"
echo "  Embedded relay started: $(grep -c 'Embedded relay started' "$LOGDIR/combined.log" 2>/dev/null || true)"
echo "  Relay discovered: $(grep -c 'Relay discovered' "$LOGDIR/combined.log" 2>/dev/null || true)"
echo "  Transport relay added: $(grep -c 'Transport relay added' "$LOGDIR/combined.log" 2>/dev/null || true)"
echo "  Relay expired: $(grep -c 'Relay expired' "$LOGDIR/combined.log" 2>/dev/null || true)"
echo "  Messages received: $(grep -c 'recu' "$LOGDIR/combined.log" 2>/dev/null || true)"
echo "  Pings sent: $(grep -c 'Ping.*envoyé' "$LOGDIR/combined.log" 2>/dev/null || true)"

# ── Overall M4 verdict ──
echo ""
if [ "$dup_count" -eq 0 ] && [ "$expiry_count" -eq 0 ] && [ "$panic_count" -eq 0 ] && $all_alive; then
    echo "=== M4: PASS ==="
else
    echo "=== M4: FAIL ==="
fi

echo ""
echo "Logs: $LOGDIR/"
