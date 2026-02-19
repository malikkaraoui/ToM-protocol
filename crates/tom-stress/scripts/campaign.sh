#!/usr/bin/env bash
# tom-stress field campaign script.
# Usage: ./campaign.sh <NAS_NODE_ID> <scenario_name>
#
# Examples:
#   ./campaign.sh abc123...def wifi-lan
#   ./campaign.sh abc123...def 4g-cgnat
#   ./campaign.sh abc123...def car-highway
#   ./campaign.sh abc123...def border-ch-fr
set -euo pipefail

if [ $# -lt 2 ]; then
    echo "Usage: $0 <NAS_NODE_ID> <scenario_name>"
    echo ""
    echo "Scenarios: wifi-lan, 4g-cgnat, car-highway, border-ch-fr, ..."
    echo "NAS_NODE_ID: from the listener's started event"
    exit 1
fi

TARGET="$1"
SCENARIO="$2"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
OUTDIR="results/${SCENARIO}_${TIMESTAMP}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BINARY="$PROJECT_ROOT/target/debug/tom-stress"

# Use release binary if available
if [ -x "$PROJECT_ROOT/target/release/tom-stress" ]; then
    BINARY="$PROJECT_ROOT/target/release/tom-stress"
fi

mkdir -p "$OUTDIR"

echo "=== tom-stress field campaign ==="
echo "  Scenario:  $SCENARIO"
echo "  Target:    ${TARGET:0:16}..."
echo "  Output:    $OUTDIR/"
echo "  Binary:    $BINARY"
echo "  Timestamp: $TIMESTAMP"
echo ""

NAME="MacBook-${SCENARIO}"

# --- Phase 1: Ping (10 pings, 1s delay) ---
echo "Phase 1/4: Ping (10 pings, 1s interval)"
"$BINARY" --name "$NAME" --output-dir "$OUTDIR" ping \
    --connect "$TARGET" \
    --count 10 \
    --delay 1000 || true

PING_OK=$(grep -c '"event":"ping"' "$OUTDIR"/${NAME}_ping_*.jsonl 2>/dev/null || echo 0)
echo "  → $PING_OK pings received"
echo ""

# --- Phase 2: Burst (10 envelopes, 1KB payload) ---
echo "Phase 2/4: Burst (10 envelopes, 1KB payload)"
"$BINARY" --name "$NAME" --output-dir "$OUTDIR" burst \
    --connect "$TARGET" \
    --count 10 \
    --payload-size 1024 || true

BURST_ACKED=$(python3 -c "
import json, glob
for f in sorted(glob.glob('$OUTDIR/${NAME}_burst_*.jsonl')):
    for line in open(f):
        d = json.loads(line.strip())
        if d.get('event') == 'burst_result':
            print(d.get('messages_acked', 0))
            break
    break
" 2>/dev/null || echo "?")
echo "  → $BURST_ACKED/10 acked"
echo ""

# --- Phase 3: Ladder (1KB → 64KB) ---
echo "Phase 3/4: Ladder (1KB → 64KB, 2 reps each)"
"$BINARY" --name "$NAME" --output-dir "$OUTDIR" ladder \
    --connect "$TARGET" \
    --sizes 1024,4096,16384,65536 \
    --reps 2 \
    --delay 500 || true

LADDER_STEPS=$(grep -c '"event":"ladder_result"' "$OUTDIR"/${NAME}_ladder_*.jsonl 2>/dev/null || echo 0)
echo "  → $LADDER_STEPS ladder steps completed"
echo ""

# --- Phase 4: Continuous ping (60s) ---
echo "Phase 4/4: Continuous ping (60s, 500ms interval)"
"$BINARY" --name "$NAME" --output-dir "$OUTDIR" ping \
    --connect "$TARGET" \
    --continuous \
    --delay 500 \
    --summary-interval 20 &
CONT_PID=$!

sleep 60
kill $CONT_PID 2>/dev/null || true
wait $CONT_PID 2>/dev/null || true

CONT_PINGS=$(grep -c '"event":"ping"' "$OUTDIR"/${NAME}_ping_*.jsonl 2>/dev/null | tail -1 || echo 0)
echo "  → $CONT_PINGS pings in 60s"
echo ""

# --- Summary ---
echo "=== Campaign complete: $SCENARIO ==="
echo "  Results in: $OUTDIR/"
ls -lh "$OUTDIR/"
echo ""
echo "Analyze with: python3 crates/tom-stress/scripts/analyze-stress.py $OUTDIR/*.jsonl"
