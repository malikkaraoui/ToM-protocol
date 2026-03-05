#!/usr/bin/env bash
set -euo pipefail

# Builds tom-relay for tvOS target and stages artifacts for Xcode integration.

TARGET="${TVOS_TARGET:-aarch64-apple-tvos}"
TOOLCHAIN="${TVOS_TOOLCHAIN:-nightly-aarch64-apple-darwin}"
PROFILE="${TVOS_PROFILE:-dev}"
OUT_DIR="apps/relay-tvos/build"

mkdir -p "${OUT_DIR}"

echo "[1/3] Ensuring Rust target ${TARGET} on ${TOOLCHAIN}"
rustup toolchain install "${TOOLCHAIN}" >/dev/null
rustup target add "${TARGET}" --toolchain "${TOOLCHAIN}" >/dev/null

echo "[2/3] Building tom-relay for tvOS (${PROFILE})"
# NOTE: This builds the Rust crate for tvOS target. Depending on dependency support,
# this step may require additional adaptation in tom-relay for full tvOS compatibility.
if [[ "${PROFILE}" == "release" ]]; then
	cargo +"${TOOLCHAIN}" build -p tom-relay --target "${TARGET}" --features server --bin tom-relay --release
	BUILD_DIR="release"
else
	cargo +"${TOOLCHAIN}" build -p tom-relay --target "${TARGET}" --features server --bin tom-relay
	BUILD_DIR="debug"
fi

echo "[3/3] Staging artifacts"
ARTIFACT_PATH="${OUT_DIR}/tom-relay-${TARGET}"
if cp -f "target/${TARGET}/${BUILD_DIR}/tom-relay" "${ARTIFACT_PATH}"; then
	echo "Done. Artifact ready: ${ARTIFACT_PATH}"
else
	cat <<'EOF'
Done, but no executable artifact was staged.
The build may have produced only library artifacts or hit tvOS binary constraints.
Next step in Xcode:
- Open/create tvOS app project
- Add files from apps/relay-tvos/TomRelay/
- Link Rust artifact from apps/relay-tvos/build/ when available
EOF
fi
