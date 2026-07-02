#!/bin/bash
# executor-ctx-monitor.sh — Surveille le contexte et alerter MasterClaude
CTX_FILE="/tmp/masterclaude-ctx-pct"
[ ! -f "$CTX_FILE" ] && exit 0
CTX=$(cat "$CTX_FILE" 2>/dev/null)
[[ ! "$CTX" =~ ^[0-9]+$ ]] && exit 0
PROJECT_ID="claude-atelier"
BUS="http://localhost:4001/v1/bus/messages"
if [ "$CTX" -ge 35 ]; then
  curl -sf -X POST "$BUS" -H 'Content-Type: application/json'     -d "{\"from\":\"$PROJECT_ID\",\"to\":\"masterclaude\",\"type\":\"compact_req\",\"payload\":{\"ctx_pct\":$CTX}}" &>/dev/null || true
fi
exit 0
