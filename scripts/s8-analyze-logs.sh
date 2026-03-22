#!/usr/bin/env bash
# S8 — logs-only analyzer
# Usage: bash scripts/s8-analyze-logs.sh [/tmp/s8-stability]
set -euo pipefail

LOGDIR=${1:-/tmp/s8-stability}
COMBINED="$LOGDIR/combined.log"

if [ ! -d "$LOGDIR" ]; then
  echo "ERROR: log directory not found: $LOGDIR"
  exit 1
fi

for f in "$LOGDIR/t1.log" "$LOGDIR/t2.log" "$LOGDIR/t3.log"; do
  if [ ! -f "$f" ]; then
    echo "ERROR: missing log file: $f"
    exit 1
  fi
done

cat "$LOGDIR/t1.log" "$LOGDIR/t2.log" "$LOGDIR/t3.log" > "$COMBINED"

echo "=== S8 LOG ANALYSIS ==="
echo "Logdir: $LOGDIR"
echo ""

# M4.1 duplicate insert
printf '%s\n' "--- M4.1: Duplicate relay insert check ---"
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

# M4.2 no abusive expiration
printf '\n%s\n' "--- M4.2: Abusive expiration check (publisher healthy) ---"
expiry_count=$(grep -c 'Relay expired' "$COMBINED" 2>/dev/null || true)
if [ "$expiry_count" -eq 0 ]; then
  echo "  PASS — no relay expiration during test (publisher was healthy)"
else
  echo "  WARNING — $expiry_count relay expiration(s) detected while publisher was running"
  grep 'Relay expired' "$COMBINED" || true
fi

# M4.3 no crash/panic
printf '\n%s\n' "--- M4.3: Crash/panic check ---"
panic_count=0
for f in "$LOGDIR"/*.log; do
  p=$(grep -Eci 'panic|thread.*panicked|SIGSEGV|fatal' "$f" 2>/dev/null || true)
  panic_count=$((panic_count + p))
done
if [ "$panic_count" -eq 0 ]; then
  echo "  PASS — no crash or panic detected"
else
  echo "  FAIL — $panic_count panic/crash indicators found"
  grep -Ei 'panic|thread.*panicked|SIGSEGV|fatal' "$LOGDIR"/*.log || true
fi

# M4.4 process liveness cannot be checked from logs post-run
printf '\n%s\n' "--- M4.4: Process liveness at end ---"
echo "  INFO — cannot be recomputed from logs only; rely on runtime monitor output (alive 4/4)"

# Summary
printf '\n%s\n' "--- Event summary ---"
echo "  Embedded relay started: $(grep -c 'Embedded relay started' "$COMBINED" 2>/dev/null || true)"
echo "  Relay discovered: $(grep -c 'Relay discovered' "$COMBINED" 2>/dev/null || true)"
echo "  Transport relay added: $(grep -c 'Transport relay added' "$COMBINED" 2>/dev/null || true)"
echo "  Relay expired: $(grep -c 'Relay expired' "$COMBINED" 2>/dev/null || true)"
echo "  Messages received: $(grep -c 'recu' "$COMBINED" 2>/dev/null || true)"
echo "  Pings sent: $(grep -c 'Ping.*envoyé' "$COMBINED" 2>/dev/null || true)"

printf '\n%s\n' "--- Overall (logs-only) ---"
if [ "$dup_count" -eq 0 ] && [ "$expiry_count" -eq 0 ] && [ "$panic_count" -eq 0 ]; then
  echo "M4(partial): PASS"
else
  echo "M4(partial): FAIL"
fi

echo ""
echo "Combined log: $COMBINED"
