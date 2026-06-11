#!/usr/bin/env bash
# Copie le XCFramework buildé vers le Swift Package (chantier S2.2).
# À lancer après scripts/build-tom-protocol-ffi-xcframework.sh.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$REPO_ROOT/apps/tom-node-tvos/build/TomProtocolFFI.xcframework"
DST_DIR="$REPO_ROOT/sdk/swift/TomProtocolKit/Artifacts"

[ -d "$SRC" ] || {
  echo "XCFramework introuvable : $SRC" >&2
  echo "Builder d'abord : bash scripts/build-tom-protocol-ffi-xcframework.sh" >&2
  exit 1
}

mkdir -p "$DST_DIR"
rm -rf "$DST_DIR/TomProtocolFFI.xcframework"
cp -R "$SRC" "$DST_DIR/"
echo "✅ $DST_DIR/TomProtocolFFI.xcframework"
