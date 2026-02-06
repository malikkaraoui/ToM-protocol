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
NC='\033[0m' # No Color

# Check if signaling server is already running
if lsof -i:3001 > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Signaling server already running on port 3001"
else
    echo -e "${BLUE}Starting signaling server...${NC}"
    pnpm --filter tom-signaling-server start &
    SIGNALING_PID=$!
    sleep 2
    echo -e "${GREEN}✓${NC} Signaling server started (PID: $SIGNALING_PID)"
fi

# Check if demo is already running
if lsof -i:5173 > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Demo already running on port 5173"
else
    echo -e "${BLUE}Starting demo app...${NC}"
    pnpm --filter tom-demo dev &
    DEMO_PID=$!
    sleep 3
    echo -e "${GREEN}✓${NC} Demo started (PID: $DEMO_PID)"
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

# Wait for Ctrl+C
wait
