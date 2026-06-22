#!/usr/bin/env bash
# Validation locale du FFI — rejoue EXACTEMENT les étapes du job CI "Rust FFI",
# que `cargo test --workspace` et `cargo clippy --workspace` ne couvrent PAS
# (tom-protocol-ffi est exclu du workspace). À lancer avant tout push touchant
# tom-protocol, tom-dht, tom-connect, tom-relay ou tom-protocol-ffi.
#
# Usage: bash scripts/check-ffi.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "[1/5] Build tom-relay-ffi (membre workspace)"
cargo build -p tom-relay-ffi

echo "[2/5] Clippy tom-relay-ffi -D warnings"
cargo clippy -p tom-relay-ffi -- -D warnings

echo "[3/5] Build tom-protocol-ffi --locked (crate exclu)"
( cd crates/tom-protocol-ffi && cargo build --locked )

echo "[4/5] Clippy tom-protocol-ffi --locked -D warnings"
( cd crates/tom-protocol-ffi && cargo clippy --locked -- -D warnings )

echo "[5/5] Dérive du header C (cbindgen)"
if command -v cbindgen >/dev/null 2>&1; then
    ./scripts/generate-ffi-header.sh --check
else
    echo "  cbindgen absent — étape sautée (installe: cargo install cbindgen)"
fi

echo
echo "✅ FFI OK — les jobs CI 'Rust FFI' et 'Rust macOS (FFI build)' devraient passer."
