#!/usr/bin/env bash
set -euo pipefail

# Builds TomProtocolFFI.xcframework bundling all platform slices.
#
# Platforms (full build):
#   tvOS device     aarch64-apple-tvos
#   tvOS sim        aarch64-apple-tvos-sim
#   iOS device      aarch64-apple-ios
#   iOS sim         aarch64-apple-ios-sim
#   macOS arm64     aarch64-apple-darwin   (M1/M2/M3)
#   macOS x86_64    x86_64-apple-darwin    (Intel)
#   macOS universal lipo(arm64 + x86_64)   → single slice in XCFramework
#
# Output: apps/tom-node-tvos/build/TomProtocolFFI.xcframework
#         (shared by tvOS, iOS, macOS Xcode projects)
#
# Usage:
#   ./scripts/build-tom-protocol-ffi-xcframework.sh           # all platforms, release
#   PROFILE=debug ./scripts/build-tom-protocol-ffi-xcframework.sh
#   DEVICE_ONLY=1 ./scripts/build-tom-protocol-ffi-xcframework.sh  # skip sims
#   MACOS=0 ./scripts/build-tom-protocol-ffi-xcframework.sh        # skip macOS

TOOLCHAIN="${TOOLCHAIN:-nightly-aarch64-apple-darwin}"
PROFILE="${PROFILE:-release}"
TV_DEPLOYMENT="${TVOS_DEPLOYMENT_TARGET:-16.3}"
IOS_DEPLOYMENT="${IPHONEOS_DEPLOYMENT_TARGET:-16.0}"
MACOS_DEPLOYMENT="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
DEVICE_ONLY="${DEVICE_ONLY:-0}"
BUILD_MACOS="${MACOS:-1}"

MANIFEST="crates/tom-protocol-ffi/Cargo.toml"
CRATE_TARGET_DIR="crates/tom-protocol-ffi/target"
HEADER_DIR="crates/tom-protocol-ffi/include"
HEADER_SOURCE="${HEADER_DIR}/tom_protocol_ffi.h"
OUT_DIR="apps/tom-node-tvos/build"
XCFW="${OUT_DIR}/TomProtocolFFI.xcframework"
MACOS_UNIVERSAL="${CRATE_TARGET_DIR}/macos-universal"

mkdir -p "${OUT_DIR}"

# ── Collect required Rust targets ──────────────────────────────────────────

TARGETS=("aarch64-apple-tvos" "aarch64-apple-ios")

if [[ "${DEVICE_ONLY}" != "1" ]]; then
    TARGETS+=("aarch64-apple-tvos-sim" "aarch64-apple-ios-sim")
fi

if [[ "${BUILD_MACOS}" == "1" ]]; then
    TARGETS+=("aarch64-apple-darwin" "x86_64-apple-darwin")
fi

echo "[1/N] Ensuring Rust toolchain ${TOOLCHAIN} + targets"
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

# ── Step counter ───────────────────────────────────────────────────────────
STEP=1
next_step() { echo "[${STEP}/N] $*"; STEP=$((STEP+1)); }

# ── Build all slices ───────────────────────────────────────────────────────

next_step "Building tvOS device   (aarch64-apple-tvos)"
TV_A=$(build_slice aarch64-apple-tvos TVOS_DEPLOYMENT_TARGET "${TV_DEPLOYMENT}")

next_step "Building iOS device    (aarch64-apple-ios)"
IOS_A=$(build_slice aarch64-apple-ios IPHONEOS_DEPLOYMENT_TARGET "${IOS_DEPLOYMENT}")

XCF_ARGS=(
    -library "${TV_A}"  -headers "${HEADER_DIR}"
    -library "${IOS_A}" -headers "${HEADER_DIR}"
)

if [[ "${DEVICE_ONLY}" != "1" ]]; then
    next_step "Building tvOS simulator (aarch64-apple-tvos-sim)"
    TVSIM_A=$(build_slice aarch64-apple-tvos-sim TVOS_DEPLOYMENT_TARGET "${TV_DEPLOYMENT}")

    next_step "Building iOS simulator  (aarch64-apple-ios-sim)"
    IOSSIM_A=$(build_slice aarch64-apple-ios-sim IPHONEOS_DEPLOYMENT_TARGET "${IOS_DEPLOYMENT}")

    XCF_ARGS+=(
        -library "${TVSIM_A}"  -headers "${HEADER_DIR}"
        -library "${IOSSIM_A}" -headers "${HEADER_DIR}"
    )
fi

if [[ "${BUILD_MACOS}" == "1" ]]; then
    next_step "Building macOS arm64    (aarch64-apple-darwin)"
    MAC_ARM_A=$(build_slice aarch64-apple-darwin MACOSX_DEPLOYMENT_TARGET "${MACOS_DEPLOYMENT}")

    next_step "Building macOS x86_64   (x86_64-apple-darwin)"
    MAC_X86_A=$(build_slice x86_64-apple-darwin MACOSX_DEPLOYMENT_TARGET "${MACOS_DEPLOYMENT}")

    next_step "Creating macOS universal binary (lipo)"
    mkdir -p "${MACOS_UNIVERSAL}"
    MACOS_UNIVERSAL_A="${MACOS_UNIVERSAL}/libtom_protocol_ffi.a"
    lipo -create "${MAC_ARM_A}" "${MAC_X86_A}" -output "${MACOS_UNIVERSAL_A}"
    echo "  → ${MACOS_UNIVERSAL_A} ($(du -sh "${MACOS_UNIVERSAL_A}" | cut -f1))"

    XCF_ARGS+=(
        -library "${MACOS_UNIVERSAL_A}" -headers "${HEADER_DIR}"
    )
fi

# ── Assemble XCFramework ───────────────────────────────────────────────────

next_step "Assembling XCFramework → ${XCFW}"
rm -rf "${XCFW}"
xcodebuild -create-xcframework "${XCF_ARGS[@]}" -output "${XCFW}"

# Stage header for HEADER_SEARCH_PATHS (shared by all app projects)
cp -f "${HEADER_SOURCE}" "${OUT_DIR}/tom_protocol_ffi.h"

echo ""
echo "✅ XCFramework ready: ${XCFW}"
ls -lh "${XCFW}"
echo ""
echo "Slices:"
ls "${XCFW}/"
echo ""
echo "Usage in Xcode: add TomProtocolFFI.xcframework to Frameworks, Libraries and Embedded Content"
echo "Header search:  \$(PROJECT_DIR)/../tom-node-tvos/build"
