#!/usr/bin/env bash
# Car test script — run campaign against NAS responder over cellular network.
#
# Two modes:
#   MODE=direct  → relay accessible at http://82.67.95.8:3340 (port forwarded on Freebox)
#   MODE=tunnel  → SSH tunnel to NAS, relay on localhost:3340 (fallback)
#
# Usage:
#   ./scripts/car-test.sh [direct|tunnel] [duration_seconds]
#
# Prerequisites:
#   - Responder running on NAS: TOM_RELAY_URL=http://127.0.0.1:3340 /root/tom-stress --no-n0-discovery responder
#   - For tunnel mode: sshpass + autossh installed, NAS SSH on 82.67.95.8:22

set -euo pipefail

MODE="${1:-tunnel}"
DURATION="${2:-3600}"
NAS_EXT_IP="82.67.95.8"
NAS_SSH_PASS="123audia4"
RELAY_PORT=3340

# Get responder node ID from NAS
echo "Fetching responder node ID from NAS..."
RESPONDER_ID=$(sshpass -p "$NAS_SSH_PASS" ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 root@"$NAS_EXT_IP" \
  "grep 'Responder Node ID:' /root/responder.log | tail -1 | awk '{print \$NF}'" 2>/dev/null)

if [ -z "$RESPONDER_ID" ]; then
  echo "ERROR: Could not get responder ID. Is the responder running on NAS?"
  echo "  Start it: TOM_RELAY_URL=http://127.0.0.1:3340 nohup /root/tom-stress --no-n0-discovery responder > /root/responder.log 2>&1 &"
  exit 1
fi

echo "Responder ID: $RESPONDER_ID"
echo "Mode: $MODE"
echo "Duration: ${DURATION}s"
echo ""

if [ "$MODE" = "direct" ]; then
  # Direct mode: relay is port-forwarded on Freebox
  RELAY_URL="http://$NAS_EXT_IP:$RELAY_PORT"
  echo "Using direct relay: $RELAY_URL"

  # Quick connectivity check
  if ! curl -s --max-time 5 -o /dev/null "$RELAY_URL/" 2>/dev/null; then
    echo "WARNING: Relay not reachable at $RELAY_URL"
    echo "  Did you forward port $RELAY_PORT on your Freebox?"
    echo "  Falling back to tunnel mode..."
    MODE="tunnel"
  fi
fi

TUNNEL_PID=""
cleanup() {
  if [ -n "$TUNNEL_PID" ]; then
    echo "Stopping SSH tunnel (PID $TUNNEL_PID)..."
    kill "$TUNNEL_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if [ "$MODE" = "tunnel" ]; then
  RELAY_URL="http://127.0.0.1:$RELAY_PORT"
  echo "Starting autossh tunnel to NAS..."

  # autossh with monitoring port, auto-reconnect
  AUTOSSH_GATETIME=0 \
  AUTOSSH_POLL=10 \
  sshpass -p "$NAS_SSH_PASS" autossh -M 0 \
    -o StrictHostKeyChecking=no \
    -o ServerAliveInterval=10 \
    -o ServerAliveCountMax=3 \
    -o ExitOnForwardFailure=yes \
    -N -L "$RELAY_PORT:localhost:$RELAY_PORT" \
    root@"$NAS_EXT_IP" &
  TUNNEL_PID=$!

  echo "Tunnel PID: $TUNNEL_PID"
  sleep 3

  if ! kill -0 "$TUNNEL_PID" 2>/dev/null; then
    echo "ERROR: SSH tunnel failed to start"
    exit 1
  fi

  # Verify tunnel works
  if curl -s --max-time 5 -o /dev/null "http://127.0.0.1:$RELAY_PORT/" 2>/dev/null; then
    echo "Tunnel active, relay reachable"
  else
    echo "WARNING: Tunnel active but relay not responding (it may use a non-HTTP handshake)"
  fi
fi

echo ""
echo "=== Starting campaign (${DURATION}s) ==="
echo "  Relay: $RELAY_URL"
echo "  Target: $RESPONDER_ID"
echo ""

# Build release if needed
cargo build -p tom-stress --release 2>&1 | tail -3

# Run campaign
TOM_RELAY_URL="$RELAY_URL" \
  cargo run -p tom-stress --release -- \
  --no-n0-discovery \
  campaign \
  --connect "$RESPONDER_ID" \
  --duration "$DURATION"
