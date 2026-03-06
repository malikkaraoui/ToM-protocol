#!/usr/bin/env bash
set -euo pipefail

# Builds tom-protocol-ffi static library for tvOS and stages artifacts for Xcode integration.

TARGET="${TVOS_TARGET:-aarch64-apple-tvos}"
TOOLCHAIN="${TVOS_TOOLCHAIN:-nightly-aarch64-apple-darwin}"
PROFILE="${TVOS_PROFILE:-release}"
DEPLOYMENT_TARGET="${TVOS_DEPLOYMENT_TARGET:-16.3}"
OUT_DIR="apps/tom-node-tvos/build"

mkdir -p "${OUT_DIR}"

echo "[1/5] Ensuring Rust target ${TARGET} on ${TOOLCHAIN}"
rustup toolchain install "${TOOLCHAIN}" >/dev/null 2>&1 || true
rustup target add "${TARGET}" --toolchain "${TOOLCHAIN}" >/dev/null 2>&1 || true

echo "[2/5] Building tom-protocol-ffi for tvOS (${PROFILE})"
echo "Using TVOS_DEPLOYMENT_TARGET=${DEPLOYMENT_TARGET}"

MANIFEST="crates/tom-protocol-ffi/Cargo.toml"
# tom-protocol-ffi is excluded from workspace; use --manifest-path
# Output goes to crate-local target dir
CRATE_TARGET_DIR="crates/tom-protocol-ffi/target"

if [[ "${PROFILE}" == "release" ]]; then
	TVOS_DEPLOYMENT_TARGET="${DEPLOYMENT_TARGET}" cargo +"${TOOLCHAIN}" build \
		--manifest-path "${MANIFEST}" \
		--target "${TARGET}" \
		--release
	BUILD_DIR="release"
else
	TVOS_DEPLOYMENT_TARGET="${DEPLOYMENT_TARGET}" cargo +"${TOOLCHAIN}" build \
		--manifest-path "${MANIFEST}" \
		--target "${TARGET}"
	BUILD_DIR="debug"
fi

echo "[3/5] Staging static library artifact"
LIB_SOURCE="${CRATE_TARGET_DIR}/${TARGET}/${BUILD_DIR}/libtom_protocol_ffi.a"
LIB_TARGET="${OUT_DIR}/libtom_protocol_ffi.a"

if [[ -f "${LIB_SOURCE}" ]]; then
	cp -f "${LIB_SOURCE}" "${LIB_TARGET}"
	echo "Static library staged: ${LIB_TARGET}"
	ls -lh "${LIB_TARGET}"
else
	echo "ERROR: Static library not found at ${LIB_SOURCE}"
	echo "Build may have failed or produced unexpected output."
	exit 1
fi

echo "[4/5] Staging C header"
HEADER_SOURCE="crates/tom-protocol-ffi/include/tom_protocol_ffi.h"
HEADER_TARGET="${OUT_DIR}/tom_protocol_ffi.h"

if [[ -f "${HEADER_SOURCE}" ]]; then
	cp -f "${HEADER_SOURCE}" "${HEADER_TARGET}"
	echo "Header staged: ${HEADER_TARGET}"
else
	echo "WARNING: Header not found at ${HEADER_SOURCE}"
	echo "Creating placeholder header..."
	mkdir -p "$(dirname ${HEADER_SOURCE})"
	cat > "${HEADER_SOURCE}" <<'HEADER_EOF'
#ifndef TOM_PROTOCOL_FFI_H
#define TOM_PROTOCOL_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to TOM protocol node
typedef void* TomNodeHandle;

// Create a TOM protocol node (config as JSON string)
TomNodeHandle tom_node_create(const char* config_json);

// Start the protocol runtime (runtime config as JSON string)
int32_t tom_node_start(TomNodeHandle handle, const char* runtime_config_json);

// Stop the node
void tom_node_stop(TomNodeHandle handle);

// Free the node handle
void tom_node_free(TomNodeHandle handle);

// Send a 1-1 message to a peer
int32_t tom_node_send_message(const TomNodeHandle handle, const char* target_id, const uint8_t* payload, size_t payload_len);

// Create a new group (config as JSON string)
// Returns: JSON string with group_id (caller must free)
char* tom_node_create_group(const TomNodeHandle handle, const char* group_config_json);

// Send a message to a group
int32_t tom_node_send_group_message(const TomNodeHandle handle, const char* group_id, const char* text);

// Receive messages (returns JSON array, caller must free)
char* tom_node_receive_messages(const TomNodeHandle handle);

// Get node status (returns JSON, caller must free)
char* tom_node_status(const TomNodeHandle handle);

// Free a string returned by FFI functions
void tom_node_free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif // TOM_PROTOCOL_FFI_H
HEADER_EOF
	cp -f "${HEADER_SOURCE}" "${HEADER_TARGET}"
	echo "Header created and staged: ${HEADER_TARGET}"
fi

echo "[5/5] Build summary"

cat <<EOF

✅ Build successful!

Next steps for Xcode integration:
1. Create TomNode.xcodeproj in apps/tom-node-tvos/
2. Add libtom_protocol_ffi.a to "Link Binary With Libraries" (Build Phases)
3. Add tom_protocol_ffi.h to bridging header
4. Create Swift bindings (TomNode actor)
5. Build & run on tvOS simulator or device

Artifacts:
- Library: ${LIB_TARGET}
- Header: ${HEADER_TARGET}

To build for simulator, run:
  TVOS_TARGET=aarch64-apple-tvos-sim ./scripts/build-tom-protocol-ffi-tvos.sh

EOF
