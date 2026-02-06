#!/bin/bash
# ToM Protocol Quick Demo Launcher
#
# Starts the signaling server and opens the demo in your browser.
# Usage: ./scripts/start-demo.sh

set -e

echo "🚀 ToM Protocol Demo"
echo "===================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# PIDs for cleanup
SIGNALING_PID=""
DEMO_PID=""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${BLUE}Shutting down...${NC}"
    if [ -n "$DEMO_PID" ] && kill -0 "$DEMO_PID" 2>/dev/null; then
        kill "$DEMO_PID" 2>/dev/null || true
        echo -e "${GREEN}✓${NC} Demo stopped"
    fi
    if [ -n "$SIGNALING_PID" ] && kill -0 "$SIGNALING_PID" 2>/dev/null; then
        kill "$SIGNALING_PID" 2>/dev/null || true
        echo -e "${GREEN}✓${NC} Signaling server stopped"
    fi
    exit 0
}

# Set up trap for cleanup on Ctrl+C or script exit
trap cleanup INT TERM EXIT

# Wait for port to be available (with timeout)
wait_for_port() {
    local port=$1
    local timeout=$2
    local elapsed=0

    while ! lsof -i:"$port" > /dev/null 2>&1; do
        if [ $elapsed -ge $timeout ]; then
            echo -e "${RED}✗${NC} Timeout waiting for port $port"
            return 1
        fi
        sleep 0.5
        elapsed=$((elapsed + 1))
    done
    return 0
}

# Check if signaling server is already running
if lsof -i:3001 > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Signaling server already running on port 3001"
else
    echo -e "${BLUE}Starting signaling server...${NC}"
    pnpm --filter tom-signaling-server start &
    SIGNALING_PID=$!

    if wait_for_port 3001 20; then
        echo -e "${GREEN}✓${NC} Signaling server started (PID: $SIGNALING_PID)"
    else
        echo -e "${RED}✗${NC} Failed to start signaling server"
        exit 1
    fi
fi

# Check if demo is already running
if lsof -i:5173 > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Demo already running on port 5173"
else
    echo -e "${BLUE}Starting demo app...${NC}"
    pnpm --filter tom-demo dev &
    DEMO_PID=$!

    if wait_for_port 5173 30; then
        echo -e "${GREEN}✓${NC} Demo started (PID: $DEMO_PID)"
    else
        echo -e "${RED}✗${NC} Failed to start demo"
        exit 1
    fi
fi

echo ""
echo "================================"
echo -e "${GREEN}Demo ready!${NC}"
echo ""
echo "Open in browser:"
echo -e "  ${BLUE}http://localhost:5173${NC}"
echo ""
echo "To test P2P:"
echo "  1. Open multiple browser tabs"
echo "  2. Use different usernames"
echo "  3. Send messages between tabs"
echo ""
echo "Press Ctrl+C to stop"
echo "================================"

# Wait for background processes
wait
