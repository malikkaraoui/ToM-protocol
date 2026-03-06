#!/bin/bash
# Stop ToM Relay

PID_FILE="$HOME/Library/Application Support/TomRelay/relay.pid"

if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        echo "Stopping ToM Relay (PID: $PID)..."
        kill "$PID"
        rm "$PID_FILE"
        echo "Relay stopped."
    else
        echo "Relay not running (stale PID file)"
        rm "$PID_FILE"
    fi
else
    echo "Relay not running (no PID file)"
    # Try to kill by process name anyway
    if pkill -f "tom-relay --dev"; then
        echo "Killed relay process by name"
    fi
fi
