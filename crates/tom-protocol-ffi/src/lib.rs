//! FFI wrapper for tom-protocol — exposes C API for Swift/tvOS
//!
//! Architecture:
//! - Swift calls tom_node_create() with JSON config → returns opaque TomNodeHandle
//! - Swift calls tom_node_start() → spawns ProtocolRuntime
//! - Swift polls tom_node_receive_messages() → batch JSON of incoming messages
//! - Swift calls tom_node_send_message() / tom_node_create_group() → commands
//! - Swift calls tom_node_stop() → shutdown + cleanup
//!
//! All async operations are managed by an internal tokio runtime.

use std::collections::VecDeque;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use tom_protocol::{DeliveredMessage, ProtocolEvent, ProtocolRuntime, RuntimeChannels, RuntimeConfig, RuntimeHandle};
use tom_transport::TomNodeConfig;

mod types;
use types::{DeliveredMessageFFI, GroupConfigFFI, NodeConfigFFI, RuntimeConfigFFI};

/// Opaque handle to the TOM protocol node (passed to/from Swift as void*)
pub struct TomNodeHandle {
    runtime: Runtime,
    handle: Arc<Mutex<Option<RuntimeHandle>>>,
    /// Buffered messages from runtime (polled by Swift)
    message_queue: Arc<Mutex<VecDeque<DeliveredMessage>>>,
    /// Buffered events from runtime (for status/debug)
    event_queue: Arc<Mutex<VecDeque<ProtocolEvent>>>,
    /// Node ID (cached after bind)
    node_id: Arc<Mutex<Option<String>>>,
}

/// Initialize tracing (logs) for the node
fn init_tracing() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env())
        .try_init();
}

/// Create a TOM protocol node (but don't start it yet)
///
/// # Arguments
/// * `config_json` - JSON string with NodeConfig fields (username, relay_url, etc.)
///
/// # Returns
/// * Opaque pointer to TomNodeHandle on success
/// * NULL on failure (check logs for details)
///
/// # Safety
/// * Caller must call `tom_node_free()` to free resources
/// * `config_json` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_create(config_json: *const c_char) -> *mut TomNodeHandle {
    init_tracing();

    // Parse JSON config
    let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Invalid config_json UTF-8: {}", e);
            return std::ptr::null_mut();
        }
    };

    let _config: NodeConfigFFI = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Invalid JSON config: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Create tokio runtime
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to create tokio runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    tracing::info!("Created TomNodeHandle (not started yet)");

    // Store config in handle for later start
    Box::into_raw(Box::new(TomNodeHandle {
        runtime,
        handle: Arc::new(Mutex::new(None)),
        message_queue: Arc::new(Mutex::new(VecDeque::new())),
        event_queue: Arc::new(Mutex::new(VecDeque::new())),
        node_id: Arc::new(Mutex::new(None)),
    }))
}

/// Start the protocol runtime
///
/// # Arguments
/// * `handle` - Opaque handle returned by `tom_node_create()`
/// * `runtime_config_json` - JSON string with RuntimeConfig fields (encryption, username, etc.)
///
/// # Returns
/// * 0 on success
/// * -1 on failure
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
/// * `runtime_config_json` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_start(
    handle: *mut TomNodeHandle,
    runtime_config_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        tracing::error!("tom_node_start: NULL handle");
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    // Parse runtime config
    let config_str = match unsafe { CStr::from_ptr(runtime_config_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Invalid runtime_config_json UTF-8: {}", e);
            return -1;
        }
    };

    let runtime_config: RuntimeConfigFFI = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Invalid JSON runtime config: {}", e);
            return -1;
        }
    };

    // Build transport config
    let mut transport_config = TomNodeConfig::new();

    if let Some(relay_url) = &runtime_config.relay_url {
        if let Ok(url) = relay_url.parse() {
            transport_config = transport_config.relay_url(url);
        }
    }

    if let Some(identity_path) = &runtime_config.identity_path {
        transport_config = transport_config.identity_path(identity_path.into());
    }

    if let Some(n0_discovery) = runtime_config.n0_discovery {
        transport_config = transport_config.n0_discovery(n0_discovery);
    }

    // Build protocol config
    let protocol_config = RuntimeConfig {
        username: runtime_config.username.clone(),
        encryption: runtime_config.encryption.unwrap_or(true),
        enable_dht: runtime_config.enable_dht.unwrap_or(true),
        data_dir: runtime_config.data_dir.map(|p| p.into()),
        ..Default::default()
    };

    let handle_clone = handle_ref.handle.clone();
    let msg_queue = handle_ref.message_queue.clone();
    let event_queue = handle_ref.event_queue.clone();
    let node_id_arc = handle_ref.node_id.clone();

    // Start node in background
    handle_ref.runtime.spawn(async move {
        tracing::info!("Binding TomNode...");

        // Bind transport
        let node = match tom_transport::TomNode::bind(transport_config).await {
            Ok(n) => {
                let id = n.id().to_string();
                tracing::info!("TomNode bound successfully: {}", id);
                *node_id_arc.lock().await = Some(id);
                n
            }
            Err(e) => {
                tracing::error!("Failed to bind TomNode: {}", e);
                return;
            }
        };

        // Spawn protocol runtime
        let channels: RuntimeChannels = ProtocolRuntime::spawn(node, protocol_config);

        tracing::info!("ProtocolRuntime spawned successfully");
        *handle_clone.lock().await = Some(channels.handle.clone());

        // Background task: drain messages + events into queues
        let msg_queue_clone = msg_queue.clone();
        let event_queue_clone = event_queue.clone();
        let mut messages_rx = channels.messages;
        let mut events_rx = channels.events;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = messages_rx.recv() => {
                        msg_queue_clone.lock().await.push_back(msg);
                    }
                    Some(event) = events_rx.recv() => {
                        event_queue_clone.lock().await.push_back(event);
                    }
                    else => break,
                }
            }
            tracing::warn!("Message/event pump stopped");
        });
    });

    0
}

/// Stop the node and free all resources
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
/// * After calling this, `handle` is invalid and must not be used
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_stop(handle: *mut TomNodeHandle) {
    if handle.is_null() {
        tracing::warn!("tom_node_stop: NULL handle");
        return;
    }

    let handle_box = unsafe { Box::from_raw(handle) };

    handle_box.runtime.block_on(async {
        if let Some(runtime_handle) = handle_box.handle.lock().await.take() {
            tracing::info!("Shutting down TOM protocol node...");
            let _ = runtime_handle.shutdown().await;
            tracing::info!("Node stopped successfully");
        }
    });

    drop(handle_box);
}

/// Free a TomNodeHandle without stopping (if already stopped separately)
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_free(handle: *mut TomNodeHandle) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle) };
    }
}

/// Send a 1-1 message to a peer
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `target_id` - Recipient NodeId (hex string)
/// * `payload` - Raw bytes
/// * `payload_len` - Length of payload
///
/// # Returns
/// * 0 on success
/// * -1 on failure
///
/// # Safety
/// * `handle` must be valid
/// * `target_id` must be a valid null-terminated C string
/// * `payload` must be a valid pointer of length `payload_len`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_send_message(
    handle: *const TomNodeHandle,
    target_id: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    let target_str = match unsafe { CStr::from_ptr(target_id) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Invalid target_id UTF-8: {}", e);
            return -1;
        }
    };

    let target_node_id = match target_str.parse() {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Invalid NodeId: {}", e);
            return -1;
        }
    };

    let payload_vec = unsafe { std::slice::from_raw_parts(payload, payload_len) }.to_vec();

    handle_ref.runtime.block_on(async {
        if let Some(runtime_handle) = handle_ref.handle.lock().await.as_ref() {
            match runtime_handle.send_message(target_node_id, payload_vec).await {
                Ok(_) => {
                    tracing::debug!("Message sent to {}", target_str);
                    0
                }
                Err(e) => {
                    tracing::error!("Failed to send message: {}", e);
                    -1
                }
            }
        } else {
            tracing::error!("Node not started");
            -1
        }
    })
}

/// Create a new group
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `group_config_json` - JSON with name, hub_relay_id, initial_members, invite_only
///
/// # Returns
/// * 0 on success (command sent to runtime)
/// * -1 on failure
///
/// # Note
/// * The group_id will be available via the `GroupCreated` event (poll events)
///
/// # Safety
/// * All pointers must be valid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_create_group(
    handle: *const TomNodeHandle,
    group_config_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    let config_str = match unsafe { CStr::from_ptr(group_config_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let group_config: GroupConfigFFI = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Invalid group config JSON: {}", e);
            return -1;
        }
    };

    let result = handle_ref.runtime.block_on(async {
        if let Some(runtime_handle) = handle_ref.handle.lock().await.as_ref() {
            if group_config.invite_only {
                runtime_handle
                    .create_group_invite_only(
                        group_config.name,
                        group_config.hub_relay_id,
                        group_config.initial_members,
                    )
                    .await
            } else {
                runtime_handle
                    .create_group(
                        group_config.name,
                        group_config.hub_relay_id,
                        group_config.initial_members,
                    )
                    .await
            }
        } else {
            Err(tom_protocol::TomProtocolError::InvalidEnvelope {
                reason: "Node not started".into(),
            })
        }
    });

    match result {
        Ok(_) => {
            tracing::debug!("Group creation command sent");
            0
        }
        Err(e) => {
            tracing::error!("Failed to create group: {}", e);
            -1
        }
    }
}

/// Send a message to a group
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `group_id` - Group ID (hex string)
/// * `text` - Message text
///
/// # Returns
/// * 0 on success
/// * -1 on failure
///
/// # Safety
/// * All pointers must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_send_group_message(
    handle: *const TomNodeHandle,
    group_id: *const c_char,
    text: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    let group_id_str = match unsafe { CStr::from_ptr(group_id) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // GroupId is a newtype wrapper around String
    let group_id_parsed = tom_protocol::group::GroupId(group_id_str.to_string());

    handle_ref.runtime.block_on(async {
        if let Some(runtime_handle) = handle_ref.handle.lock().await.as_ref() {
            match runtime_handle
                .send_group_message(group_id_parsed, text_str.to_string())
                .await
            {
                Ok(_) => 0,
                Err(e) => {
                    tracing::error!("Failed to send group message: {}", e);
                    -1
                }
            }
        } else {
            -1
        }
    })
}

/// Receive messages (polled by Swift every ~500ms)
///
/// # Returns
/// * JSON array of messages: `[{"from": "...", "payload": "...", ...}, ...]`
/// * Empty array `[]` if no messages
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_receive_messages(
    handle: *const TomNodeHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    let messages_json = handle_ref.runtime.block_on(async {
        let mut queue = handle_ref.message_queue.lock().await;
        let batch: Vec<DeliveredMessage> = queue.drain(..).collect();

        // Convert to FFI-serializable type
        let ffi_batch: Vec<DeliveredMessageFFI> = batch.into_iter().map(Into::into).collect();

        // Serialize to JSON
        match serde_json::to_string(&ffi_batch) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to serialize messages: {}", e);
                "[]".to_string()
            }
        }
    });

    match CString::new(messages_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get node status
///
/// # Returns
/// * JSON string with node_id, status, peers_count, groups_count
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_status(handle: *const TomNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    let status_json = handle_ref.runtime.block_on(async {
        let node_id = handle_ref.node_id.lock().await.clone();
        let is_running = handle_ref.handle.lock().await.is_some();

        let status = if is_running { "Running" } else { "Stopped" };

        // TODO: Query actual metrics from runtime
        let json = format!(
            r#"{{"node_id":"{}","status":"{}","peers_count":0,"groups_count":0}}"#,
            node_id.unwrap_or_else(|| "unknown".to_string()),
            status
        );

        json
    });

    match CString::new(status_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string returned by FFI functions
///
/// # Safety
/// * `s` must be a valid pointer returned by `tom_node_*` functions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = unsafe { CString::from_raw(s) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_lifecycle() {
        use std::ffi::CString;

        // Minimal config
        let node_config = NodeConfigFFI {
            relay_url: Some("http://127.0.0.1:3343".to_string()),
            n0_discovery: Some(false),
            identity_path: None,
        };
        let node_config_json = serde_json::to_string(&node_config).unwrap();
        let node_config_cstr = CString::new(node_config_json).unwrap();

        let runtime_config = RuntimeConfigFFI {
            username: "test_node".to_string(),
            encryption: Some(false),
            enable_dht: Some(false),
            relay_url: Some("http://127.0.0.1:3343".to_string()),
            identity_path: None,
            n0_discovery: Some(false),
            data_dir: None,
        };
        let runtime_config_json = serde_json::to_string(&runtime_config).unwrap();
        let runtime_config_cstr = CString::new(runtime_config_json).unwrap();

        unsafe {
            let handle = tom_node_create(node_config_cstr.as_ptr());
            assert!(!handle.is_null(), "tom_node_create should return valid handle");

            let start_result = tom_node_start(handle, runtime_config_cstr.as_ptr());
            assert_eq!(start_result, 0, "tom_node_start should succeed");

            // Give node time to bind
            std::thread::sleep(std::time::Duration::from_secs(1));

            let status_ptr = tom_node_status(handle);
            assert!(!status_ptr.is_null(), "tom_node_status should return valid pointer");

            let status_cstr = CStr::from_ptr(status_ptr);
            let status_str = status_cstr.to_str().unwrap();
            println!("Status: {}", status_str);
            assert!(status_str.contains("Running"));

            tom_node_free_string(status_ptr);
            tom_node_stop(handle);
        }
    }
}
