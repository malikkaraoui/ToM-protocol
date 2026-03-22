#!/usr/bin/env bash
# S7 — 20 runs automated validation
# Criteria: published / discovered / transport_injected / message_passed
# Timings: publication->discovery, discovery->transport, send->receive
set -euo pipefail

RUNS=${1:-20}
TIMEOUT=40  # seconds per run
RESULTS_FILE="/tmp/s7-results.csv"

echo "run,published,discovered,transport_injected,message_passed,t_discovery_s,t_first_msg_s" > "$RESULTS_FILE"

echo "=== S7: $RUNS runs ==="
echo ""

pass=0
fail=0

for i in $(seq 1 "$RUNS"); do
    echo -n "Run $i/$RUNS... "

    LOG="/tmp/s7-run-$i.log"

    # Start relay
    target/debug/tom-relay --dev 2>/dev/null &
    RELAY_PID=$!
    sleep 2

    # Start publisher
    TOM_RELAY_URL=http://127.0.0.1:3340 RUST_LOG=warn \
      target/debug/tom-chat \
        --username publisher --bot --bot-ping 5 \
        --embedded-relay --embedded-relay-publish \
        --relay-ttl 60 --relay-publish-interval 25 \
        2>/dev/null > "/tmp/s7-t1-$i.log" &
    T1_PID=$!

    sleep 2

    # Start observer
    TOM_RELAY_URL=http://127.0.0.1:3340 RUST_LOG=warn \
      target/debug/tom-chat \
        --username obs1 --bot \
        --relay-discovery --relay-ttl 60 \
        2>/dev/null > "/tmp/s7-t2-$i.log" &
    T2_PID=$!

    # Wait for events
    sleep "$TIMEOUT"

    # Kill all
    kill $T1_PID $T2_PID $RELAY_PID 2>/dev/null
    wait $T1_PID $T2_PID $RELAY_PID 2>/dev/null || true

    # Combine logs
    cat "/tmp/s7-t1-$i.log" "/tmp/s7-t2-$i.log" > "$LOG"

    # Check criteria (binary)
    # grep -c returns exit 1 when count=0, so use `|| true` to avoid triggering `set -e`
    published=$(grep -c 'Embedded relay started' "$LOG" 2>/dev/null || true)
    discovered=$(grep -c 'Relay discovered' "$LOG" 2>/dev/null || true)
    transport=$(grep -c 'Transport relay added' "$LOG" 2>/dev/null || true)
    msg_passed=$(grep -c 'recu 5/5' "$LOG" 2>/dev/null || true)

    c_pub=$( [ "$published" -ge 1 ] && echo 1 || echo 0 )
    c_disc=$( [ "$discovered" -ge 1 ] && echo 1 || echo 0 )
    c_trans=$( [ "$transport" -ge 1 ] && echo 1 || echo 0 )
    c_msg=$( [ "$msg_passed" -ge 1 ] && echo 1 || echo 0 )

    # Compute timing: time between first "Embedded relay started" and first "Relay discovered"
    # We can't easily get sub-second timestamps from bot output, so we mark presence only
    t_disc="n/a"
    t_msg="n/a"

    success=$(( c_pub & c_disc & c_trans & c_msg ))

    if [ "$success" -eq 1 ]; then
        echo "OK (pub=$c_pub disc=$c_disc trans=$c_trans msg=$c_msg)"
        pass=$((pass + 1))
    else
        echo "FAIL (pub=$c_pub disc=$c_disc trans=$c_trans msg=$c_msg)"
        fail=$((fail + 1))
    fi

    echo "$i,$c_pub,$c_disc,$c_trans,$c_msg,$t_disc,$t_msg" >> "$RESULTS_FILE"

    # Cleanup temp
    rm -f "/tmp/s7-t1-$i.log" "/tmp/s7-t2-$i.log" "$LOG"

    # Brief pause between runs
    sleep 2
done

echo ""
echo "=== RESULTS ==="
echo "Pass: $pass / $RUNS"
echo "Fail: $fail / $RUNS"
rate=$(echo "scale=1; $pass * 100 / $RUNS" | bc)
echo "Success rate: ${rate}%"
echo ""
echo "Target: >= 90%"
if [ "$pass" -ge $(echo "$RUNS * 9 / 10" | bc) ]; then
    echo "M3: PASS"
else
    echo "M3: FAIL"
fi
echo ""
echo "Details: $RESULTS_FILE"
cat "$RESULTS_FILE"
