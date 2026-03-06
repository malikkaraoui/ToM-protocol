#ifndef TOM_PROTOCOL_FFI_H
#define TOM_PROTOCOL_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Opaque handle to TOM protocol node
typedef void* TomNodeHandle;

/// Create a TOM protocol node (config as JSON string)
///
/// Example config JSON:
/// {
///   "relay_url": "http://127.0.0.1:3343",
///   "n0_discovery": false,
///   "identity_path": "/path/to/identity.key"
/// }
///
/// Returns: opaque handle or NULL on failure
/// Caller must call tom_node_free() to free resources
TomNodeHandle tom_node_create(const char* config_json);

/// Start the protocol runtime (runtime config as JSON string)
///
/// Example runtime config JSON:
/// {
///   "username": "alice",
///   "encryption": true,
///   "enable_dht": true,
///   "relay_url": "http://127.0.0.1:3343",
///   "identity_path": "/path/to/identity.key",
///   "n0_discovery": false,
///   "data_dir": "/path/to/state"
/// }
///
/// Returns: 0 on success, -1 on failure
int32_t tom_node_start(TomNodeHandle handle, const char* runtime_config_json);

/// Stop the node and free all resources
void tom_node_stop(TomNodeHandle handle);

/// Free the node handle (without stopping, if already stopped)
void tom_node_free(TomNodeHandle handle);

/// Send a 1-1 message to a peer
///
/// Args:
///   handle: opaque node handle
///   target_id: recipient NodeId (hex string, null-terminated)
///   payload: raw bytes
///   payload_len: length of payload
///
/// Returns: 0 on success, -1 on failure
int32_t tom_node_send_message(
    const TomNodeHandle handle,
    const char* target_id,
    const uint8_t* payload,
    size_t payload_len
);

/// Create a new group (config as JSON string)
///
/// Example group config JSON:
/// {
///   "name": "My Group",
///   "hub_relay_id": "<hex_node_id>",
///   "initial_members": ["<hex_node_id_1>", "<hex_node_id_2>"],
///   "invite_only": true
/// }
///
/// Returns: 0 on success (command sent to runtime), -1 on failure
/// Note: The group_id will be available via the `GroupCreated` event (poll events)
int32_t tom_node_create_group(const TomNodeHandle handle, const char* group_config_json);

/// Send a message to a group
///
/// Args:
///   handle: opaque node handle
///   group_id: group ID (hex string, null-terminated)
///   text: message text (null-terminated)
///
/// Returns: 0 on success, -1 on failure
int32_t tom_node_send_group_message(
    const TomNodeHandle handle,
    const char* group_id,
    const char* text
);

/// Receive messages (polled by application)
///
/// Returns: JSON array of messages (empty array if no messages)
/// Example: [{"from":"<hex_id>","payload":[65,66,67],"envelope_id":"...","timestamp":1234567890,"signature_valid":true,"was_encrypted":true}]
///
/// Caller must free returned string with tom_node_free_string()
char* tom_node_receive_messages(const TomNodeHandle handle);

/// Get node status
///
/// Returns: JSON string with node_id, status, peers_count, groups_count
/// Example: {"node_id":"<hex_id>","status":"Running","peers_count":5,"groups_count":2}
///
/// Caller must free returned string with tom_node_free_string()
char* tom_node_status(const TomNodeHandle handle);

/// Free a string returned by FFI functions
void tom_node_free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif // TOM_PROTOCOL_FFI_H
