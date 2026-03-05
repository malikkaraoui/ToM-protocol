#!/usr/bin/env bash
set -euo pipefail

# Builds tom-relay for tvOS target and stages artifacts for Xcode integration.

TARGET="aarch64-apple-tvos"
OUT_DIR="apps/relay-tvos/build"

mkdir -p "${OUT_DIR}"

echo "[1/3] Ensuring Rust target ${TARGET}"
rustup target add "${TARGET}" >/dev/null

echo "[2/3] Building tom-relay for tvOS"
# NOTE: This builds the Rust crate for tvOS target. Depending on dependency support,
# this step may require additional adaptation in tom-relay for full tvOS compatibility.
cargo build -p tom-relay --target "${TARGET}"

echo "[3/3] Staging artifacts"
cp -f "target/${TARGET}/debug/tom-relay" "${OUT_DIR}/tom-relay-${TARGET}" || true

cat <<'EOF'
Done.

If no binary was copied, the build likely produced only library artifacts or failed for tvOS constraints.
Next step in Xcode:
- Open/create tvOS app project
- Add files from apps/relay-tvos/TomRelay/
- Link Rust artifact from apps/relay-tvos/build/ when available
EOF
