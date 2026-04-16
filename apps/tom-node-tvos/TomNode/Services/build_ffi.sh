#!/bin/bash

# Xcode build phase — rebuilds TomProtocolFFI.xcframework (device only: tvOS + iOS).
# Skips if already built. Run `make ffi-xcframework` to force a full rebuild.

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Find workspace root
SEARCH_DIRS=(
    "$SCRIPT_DIR/../.."
    "$SCRIPT_DIR/../../.."
    "$SCRIPT_DIR/../../../.."
    "/Users/malik/Documents/tom-protocol"
    "${SRCROOT:-}/../.."
    "${PROJECT_DIR:-}/../.."
)

WORKSPACE_ROOT=""
for dir in "${SEARCH_DIRS[@]}"; do
    if [ -d "$dir" ] && [ -f "$dir/Cargo.toml" ]; then
        if grep -q "tom-protocol-ffi" "$dir/Cargo.toml" 2>/dev/null; then
            WORKSPACE_ROOT="$(cd "$dir" && pwd)"
            break
        fi
    fi
done

if [ -z "$WORKSPACE_ROOT" ]; then
    echo "❌ Workspace Cargo introuvable"
    exit 1
fi

XCFW="$WORKSPACE_ROOT/apps/tom-node-tvos/build/TomProtocolFFI.xcframework"

# Skip if already built
if [ -d "$XCFW" ] && [ -f "$XCFW/Info.plist" ]; then
    echo "✅ TomProtocolFFI.xcframework déjà présent — skip"
    exit 0
fi

echo "================================================"
echo "🔨 Build TomProtocolFFI.xcframework (device only)"
echo "================================================"

XCFW_SCRIPT="$WORKSPACE_ROOT/scripts/build-tom-protocol-ffi-xcframework.sh"
if [ ! -f "$XCFW_SCRIPT" ]; then
    echo "❌ Script xcframework introuvable: $XCFW_SCRIPT"
    exit 1
fi

cd "$WORKSPACE_ROOT"
DEVICE_ONLY=1 bash "$XCFW_SCRIPT"
