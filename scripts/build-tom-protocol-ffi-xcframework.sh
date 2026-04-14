#!/usr/bin/env bash
set -euo pipefail

# Builds TomProtocolFFI.xcframework bundling all platform slices (tvOS + iOS).
#
# Slices:
#   aarch64-apple-tvos       → tvOS device
#   aarch64-apple-tvos-sim   → tvOS simulator (Apple Silicon)
#   aarch64-apple-ios        → iOS device
#   aarch64-apple-ios-sim    → iOS simulator (Apple Silicon)
#
# Output: apps/tom-node-tvos/build/TomProtocolFFI.xcframework
#
# Usage:
#   ./scripts/build-tom-protocol-ffi-xcframework.sh           # release (default)
#   PROFILE=debug ./scripts/build-tom-protocol-ffi-xcframework.sh

TOOLCHAIN="${TOOLCHAIN:-nightly-aarch64-apple-darwin}"
PROFILE="${PROFILE:-release}"
TV_DEPLOYMENT="${TVOS_DEPLOYMENT_TARGET:-16.3}"
IOS_DEPLOYMENT="${IPHONEOS_DEPLOYMENT_TARGET:-16.0}"

MANIFEST="crates/tom-protocol-ffi/Cargo.toml"
CRATE_TARGET_DIR="crates/tom-protocol-ffi/target"
HEADER_DIR="crates/tom-protocol-ffi/include"
HEADER_SOURCE="${HEADER_DIR}/tom_protocol_ffi.h"
OUT_DIR="apps/tom-node-tvos/build"
XCFW="${OUT_DIR}/TomProtocolFFI.xcframework"

mkdir -p "${OUT_DIR}"

# ── Ensure Rust targets ────────────────────────────────────────────────────

TARGETS=(
    "aarch64-apple-tvos"
    "aarch64-apple-tvos-sim"
    "aarch64-apple-ios"
    "aarch64-apple-ios-sim"
)

echo "[1/6] Ensuring Rust toolchain ${TOOLCHAIN} + targets"
rustup toolchain install "${TOOLCHAIN}" >/dev/null 2>&1 || true
for t in "${TARGETS[@]}"; do
    rustup target add "${t}" --toolchain "${TOOLCHAIN}" >/dev/null 2>&1 || true
done

# ── Build function ─────────────────────────────────────────────────────────

build_slice() {
    local target="$1"
    local env_var="$2"
    local env_val="$3"

    if [[ "${PROFILE}" == "release" ]]; then
        env "${env_var}=${env_val}" \
            cargo +"${TOOLCHAIN}" build \
            --manifest-path "${MANIFEST}" \
            --target "${target}" \
            --release
        echo "${CRATE_TARGET_DIR}/${target}/release/libtom_protocol_ffi.a"
    else
        env "${env_var}=${env_val}" \
            cargo +"${TOOLCHAIN}" build \
            --manifest-path "${MANIFEST}" \
            --target "${target}"
        echo "${CRATE_TARGET_DIR}/${target}/debug/libtom_protocol_ffi.a"
    fi
}

# ── Build all slices ───────────────────────────────────────────────────────

echo "[2/6] Building tvOS device   (aarch64-apple-tvos)"
TV_A=$(build_slice aarch64-apple-tvos TVOS_DEPLOYMENT_TARGET "${TV_DEPLOYMENT}")

echo "[3/6] Building tvOS simulator (aarch64-apple-tvos-sim)"
TVSIM_A=$(build_slice aarch64-apple-tvos-sim TVOS_DEPLOYMENT_TARGET "${TV_DEPLOYMENT}")

echo "[4/6] Building iOS device    (aarch64-apple-ios)"
IOS_A=$(build_slice aarch64-apple-ios IPHONEOS_DEPLOYMENT_TARGET "${IOS_DEPLOYMENT}")

echo "[5/6] Building iOS simulator  (aarch64-apple-ios-sim)"
IOSSIM_A=$(build_slice aarch64-apple-ios-sim IPHONEOS_DEPLOYMENT_TARGET "${IOS_DEPLOYMENT}")

# ── Assemble XCFramework ───────────────────────────────────────────────────

echo "[6/6] Assembling XCFramework → ${XCFW}"

rm -rf "${XCFW}"
xcodebuild -create-xcframework \
    -library "${TV_A}"     -headers "${HEADER_DIR}" \
    -library "${TVSIM_A}"  -headers "${HEADER_DIR}" \
    -library "${IOS_A}"    -headers "${HEADER_DIR}" \
    -library "${IOSSIM_A}" -headers "${HEADER_DIR}" \
    -output "${XCFW}"

# Stage header for HEADER_SEARCH_PATHS
cp -f "${HEADER_SOURCE}" "${OUT_DIR}/tom_protocol_ffi.h"

echo ""
echo "✅ XCFramework ready: ${XCFW}"
ls -lh "${XCFW}"
echo ""
echo "Slices:"
ls "${XCFW}/"
