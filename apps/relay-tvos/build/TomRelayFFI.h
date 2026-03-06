//
//  TomRelayFFI.h
//  ToM Relay FFI Bridge
//
//  C header for tom-relay Rust FFI
//

#ifndef TomRelayFFI_h
#define TomRelayFFI_h

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Opaque handle to the relay server (void* in C, points to TomRelayHandle in Rust)
typedef void* TomRelayHandle;

/// Start the tom-relay server in dev mode (HTTP only, no TLS)
///
/// @param bind_addr HTTP bind address (e.g., "0.0.0.0:3343")
/// @param metrics_addr Metrics bind address (e.g., "0.0.0.0:9093")
/// @return Opaque handle on success, NULL on failure
/// @note Caller must call tom_relay_stop() to free resources
TomRelayHandle tom_relay_start(const char* bind_addr, const char* metrics_addr);

/// Stop the relay server and free all resources
///
/// @param handle Handle returned by tom_relay_start()
/// @note After calling this, handle is invalid and must not be used
void tom_relay_stop(TomRelayHandle handle);

/// Get the current status of the relay server
///
/// @param handle Handle returned by tom_relay_start()
/// @return JSON string with status (caller must free with tom_relay_free_string())
/// @note Returns NULL if handle is invalid
char* tom_relay_status(TomRelayHandle handle);

/// Free a string returned by tom_relay_status()
///
/// @param s String pointer returned by tom_relay_status()
void tom_relay_free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif /* TomRelayFFI_h */
