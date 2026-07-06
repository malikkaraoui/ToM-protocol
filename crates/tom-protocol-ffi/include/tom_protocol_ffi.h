/* tom-protocol-ffi — header généré par cbindgen, ne pas éditer. */

#ifndef TOM_PROTOCOL_FFI_H
#define TOM_PROTOCOL_FFI_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

// Opaque handle to the TOM protocol node (passed to/from Swift as void*)
typedef struct TomNodeHandle TomNodeHandle;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

// Create a TOM protocol node (but don't start it yet)
//
// # Arguments
// * `config_json` - JSON string with NodeConfig fields (username, relay_url, etc.)
//
// # Returns
// * Opaque pointer to TomNodeHandle on success
// * NULL on failure (check logs for details)
//
// # Safety
// * Caller must call `tom_node_free()` to free resources
// * `config_json` must be a valid null-terminated C string
struct TomNodeHandle *tom_node_create(const char *config_json);

// Start the protocol runtime
//
// # Arguments
// * `handle` - Opaque handle returned by `tom_node_create()`
// * `runtime_config_json` - JSON string with RuntimeConfig fields (encryption, username, etc.)
//
// # Returns
// * 0 on success
// * -1 on failure
//
// # Safety
// * `handle` must be a valid pointer returned by `tom_node_create()`
// * `runtime_config_json` must be a valid null-terminated C string
int32_t tom_node_start(struct TomNodeHandle *handle, const char *runtime_config_json);

// Stop the node and free all resources
//
// # Safety
// * `handle` must be a valid pointer returned by `tom_node_create()`
// * After calling this, `handle` is invalid and must not be used
void tom_node_stop(struct TomNodeHandle *handle);

// Free a TomNodeHandle without graceful shutdown (e.g. forceReset after OS suspend).
//
// # Safety
// * `handle` must be a valid pointer returned by `tom_node_create()` and not
//   already freed/stopped (see `detached_teardown` ownership contract).
void tom_node_free(struct TomNodeHandle *handle);

// Send a 1-1 message to a peer
//
// # Arguments
// * `handle` - Opaque handle
// * `target_id` - Recipient NodeId (hex string)
// * `payload` - Raw bytes
// * `payload_len` - Length of payload
//
// # Returns
// * 0 on success
// * -1 on failure
//
// # Safety
// * `handle` must be valid
// * `target_id` must be a valid null-terminated C string
// * `payload` must be a valid pointer of length `payload_len`
int32_t tom_node_send_message(const struct TomNodeHandle *handle,
                              const char *target_id,
                              const uint8_t *payload,
                              uintptr_t payload_len);

// Create a new group
//
// # Arguments
// * `handle` - Opaque handle
// * `group_config_json` - JSON with name, hub_relay_id, initial_members, invite_only
//
// # Returns
// * 0 on success (command sent to runtime)
// * -1 on failure
//
// # Note
// * The group_id will be available via the `GroupCreated` event (poll events)
//
// # Safety
// * All pointers must be valid
int32_t tom_node_create_group(const struct TomNodeHandle *handle, const char *group_config_json);

// Send a message to a group
//
// # Arguments
// * `handle` - Opaque handle
// * `group_id` - Group ID (hex string)
// * `text` - Message text
//
// # Returns
// * 0 on success
// * -1 on failure
//
// # Safety
// * All pointers must be valid null-terminated C strings
int32_t tom_node_send_group_message(const struct TomNodeHandle *handle,
                                    const char *group_id,
                                    const char *text);

// Receive messages (polled by Swift every ~500ms)
//
// # Returns
// * JSON array of messages: `[{"from": "...", "payload": "...", ...}, ...]`
// * Empty array `[]` if no messages
// * NULL on error
//
// # Safety
// * Caller must free returned string with `tom_node_free_string()`
char *tom_node_receive_messages(const struct TomNodeHandle *handle);

// Get node status
//
// # Returns
// * JSON string with node_id, status, peers_count, groups_count
// * NULL on error
//
// # Safety
// * Caller must free returned string with `tom_node_free_string()`
char *tom_node_status(const struct TomNodeHandle *handle);

// Issue a presence challenge toward a peer (L1-001).
//
// The result arrives asynchronously: on acceptance the node updates its
// presence stats (poll `tom_node_presence_stats()`) and logs the event.
// No result at all means the peer is absent, lying, or below the
// anti-Sybil gate — silent by design (no oracle).
//
// # Returns
// * 0 on success (command queued)
// * -1 on failure (null/invalid handle or target, node not started)
//
// # Safety
// * `handle` must be a valid pointer returned by `tom_node_create()`
// * `target_id` must be a valid NUL-terminated C string
int32_t tom_node_check_presence(const struct TomNodeHandle *handle, const char *target_id);

// Get L1-001 presence stats as JSON (see `PresenceStatsFFI` for the schema).
//
// # Returns
// * JSON C string (caller must free with `tom_node_free_string()`)
// * NULL on null handle or serialization failure
//
// # Safety
// * `handle` must be a valid pointer returned by `tom_node_create()`
char *tom_node_presence_stats(const struct TomNodeHandle *handle);

// Get the last error message (after a function returned -1)
//
// # Returns
// * Error message as C string (caller must free with `tom_node_free_string()`)
// * NULL if no error
//
// # Safety
// * `handle` must be a valid pointer returned by `tom_node_create()`
char *tom_node_last_error(const struct TomNodeHandle *handle);

// Add a peer address (so this node can connect to it)
//
// # Arguments
// * `handle` - Opaque handle
// * `peer_addr_json` - JSON with node_id, relay_url, direct_addrs
//   Example: {"node_id":"<hex>","relay_url":"http://1.2.3.4:3340","direct_addrs":["192.168.0.83:3340"]}
//   Only node_id is required. relay_url and direct_addrs are optional.
//
// # Returns
// * 0 on success, -1 on failure
//
// # Safety
// * All pointers must be valid
int32_t tom_node_add_peer_addr(const struct TomNodeHandle *handle,
                               const char *peer_addr_json);

// Get connected peers as JSON array of Node IDs
//
// # Returns
// * JSON array: ["<hex_id_1>", "<hex_id_2>", ...]
// * Empty array "[]" if no peers
//
// # Safety
// * Caller must free returned string with `tom_node_free_string()`
char *tom_node_connected_peers(const struct TomNodeHandle *handle);

// Get peers discovered via gossip/DHT as JSON array
//
// # Returns
// * JSON array: [{"node_id":"...","username":"...","source":"...","discovered_at":123}, ...]
// * Empty array "[]" if no peers discovered yet
//
// # Safety
// * Caller must free returned string with `tom_node_free_string()`
char *tom_node_discovered_peers(const struct TomNodeHandle *handle);

// Get the best available relay URL for this node.
//
// Priority: (1) configured relay URL, (2) most recently discovered relay via gossip.
// Returns NULL when no relay is known yet — the runtime will still work via
// N0 public relays if n0_discovery is enabled.
//
// # Returns
// * Relay URL as null-terminated C string (caller must free with `tom_node_free_string()`)
// * NULL if no relay is known
//
// # Safety
// * `handle` must be a valid pointer returned by `tom_node_create()`
char *tom_get_discovered_relay(const struct TomNodeHandle *handle);

// Free a string returned by FFI functions
//
// # Safety
// * `s` must be a valid pointer returned by `tom_node_*` functions
void tom_node_free_string(char *s);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* TOM_PROTOCOL_FFI_H */
