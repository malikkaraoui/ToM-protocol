#!/usr/bin/env bash
set -euo pipefail

# Builds tom-relay-ffi static library for tvOS and stages artifacts for Xcode integration.

TARGET="${TVOS_TARGET:-aarch64-apple-tvos}"
TOOLCHAIN="${TVOS_TOOLCHAIN:-nightly-aarch64-apple-darwin}"
PROFILE="${TVOS_PROFILE:-release}"
DEPLOYMENT_TARGET="${TVOS_DEPLOYMENT_TARGET:-16.3}"
OUT_DIR="apps/relay-tvos/build"

mkdir -p "${OUT_DIR}"

echo "[1/4] Ensuring Rust target ${TARGET} on ${TOOLCHAIN}"
rustup toolchain install "${TOOLCHAIN}" >/dev/null 2>&1 || true
rustup target add "${TARGET}" --toolchain "${TOOLCHAIN}" >/dev/null 2>&1 || true

echo "[2/4] Building tom-relay-ffi for tvOS (${PROFILE})"
echo "Using TVOS_DEPLOYMENT_TARGET=${DEPLOYMENT_TARGET}"

if [[ "${PROFILE}" == "release" ]]; then
	TVOS_DEPLOYMENT_TARGET="${DEPLOYMENT_TARGET}" cargo +"${TOOLCHAIN}" build \
		-p tom-relay-ffi \
		--target "${TARGET}" \
		--release
	BUILD_DIR="release"
else
	TVOS_DEPLOYMENT_TARGET="${DEPLOYMENT_TARGET}" cargo +"${TOOLCHAIN}" build \
		-p tom-relay-ffi \
		--target "${TARGET}"
	BUILD_DIR="debug"
fi

echo "[3/4] Staging static library artifact"
LIB_SOURCE="target/${TARGET}/${BUILD_DIR}/libtom_relay_ffi.a"
LIB_TARGET="${OUT_DIR}/libtom_relay_ffi.a"

if [[ -f "${LIB_SOURCE}" ]]; then
	cp -f "${LIB_SOURCE}" "${LIB_TARGET}"
	echo "Static library staged: ${LIB_TARGET}"
	ls -lh "${LIB_TARGET}"
else
	echo "ERROR: Static library not found at ${LIB_SOURCE}"
	echo "Build may have failed or produced unexpected output."
	exit 1
fi

echo "[4/4] Staging C header"
HEADER_SOURCE="apps/relay-tvos/TomRelay/TomRelay/TomRelayFFI.h"
HEADER_TARGET="${OUT_DIR}/TomRelayFFI.h"

if [[ -f "${HEADER_SOURCE}" ]]; then
	cp -f "${HEADER_SOURCE}" "${HEADER_TARGET}"
	echo "Header staged: ${HEADER_TARGET}"
else
	echo "WARNING: Header not found at ${HEADER_SOURCE}"
fi

cat <<EOF

✅ Build successful!

Next steps for Xcode integration:
1. Open TomRelay.xcodeproj in Xcode
2. Add libtom_relay_ffi.a to "Link Binary With Libraries" (Build Phases)
3. Add TomRelayFFI.h to bridging header (or import in module map)
4. Update RelayManager.swift to call FFI functions
5. Build & run on tvOS simulator or device

Artifacts:
- Library: ${LIB_TARGET}
- Header: ${HEADER_TARGET}
EOF
