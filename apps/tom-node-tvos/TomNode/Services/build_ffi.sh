#!/bin/bash

# Xcode build phase — delegates to scripts/build-tom-protocol-ffi-xcframework.sh
# Builds TomProtocolFFI.xcframework (tvOS + iOS, all variants).
# Skipped by Xcode when output already exists (no alwaysOutOfDate).

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

echo "================================================"
echo "🔨 Build TomProtocolFFI.xcframework"
echo "================================================"

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
            echo "  ✅ Workspace: $WORKSPACE_ROOT"
            break
        fi
    fi
done

if [ -z "$WORKSPACE_ROOT" ]; then
    echo "❌ ERREUR: Workspace Cargo introuvable"
    exit 1
fi

XCFW_SCRIPT="$WORKSPACE_ROOT/scripts/build-tom-protocol-ffi-xcframework.sh"
if [ ! -f "$XCFW_SCRIPT" ]; then
    echo "❌ ERREUR: Script xcframework introuvable: $XCFW_SCRIPT"
    exit 1
fi

cd "$WORKSPACE_ROOT"
bash "$XCFW_SCRIPT"
