#!/usr/bin/env bash
# Génère le header C de tom-protocol-ffi via cbindgen (chantier S2.1).
# Usage :
#   scripts/generate-ffi-header.sh           # régénère include/tom_protocol_ffi.h
#   scripts/generate-ffi-header.sh --check   # échoue si le header commité a dérivé (CI)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/crates/tom-protocol-ffi"
HEADER="$CRATE_DIR/include/tom_protocol_ffi.h"

command -v cbindgen >/dev/null || {
  echo "cbindgen manquant : cargo install cbindgen (ou brew install cbindgen)" >&2
  exit 1
}

if [ "${1:-}" = "--check" ]; then
  TMP="$(mktemp)"
  trap 'rm -f "$TMP"' EXIT
  (cd "$CRATE_DIR" && cbindgen --config cbindgen.toml --crate tom-protocol-ffi --output "$TMP")
  if ! diff -u "$HEADER" "$TMP"; then
    echo "" >&2
    echo "DRIFT : include/tom_protocol_ffi.h ne correspond plus au code Rust." >&2
    echo "Regénérer : scripts/generate-ffi-header.sh puis committer." >&2
    exit 1
  fi
  echo "header à jour ✓"
else
  (cd "$CRATE_DIR" && cbindgen --config cbindgen.toml --crate tom-protocol-ffi --output "$HEADER")
  echo "header régénéré : $HEADER"
fi
