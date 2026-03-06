//! FFI wrapper for tom-relay — exposes C API for Swift/tvOS
//!
//! Architecture:
//! - Swift calls tom_relay_start() → returns opaque pointer to TomRelayHandle
//! - Swift stores this handle and uses it for stop/status calls
//! - tom_relay_stop() shuts down the server and frees resources
//!
//! All async operations are managed by an internal tokio runtime.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

/// Opaque handle to the relay server (passed to/from Swift as void*)
pub struct TomRelayHandle {
    runtime: Runtime,
    server: Arc<Mutex<Option<tom_relay::server::Server>>>,
    status: Arc<Mutex<String>>,
}

/// Initialize tracing (logs) for the relay
fn init_tracing() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env())
        .try_init();
}

/// Start the tom-relay server in dev mode (HTTP only, no TLS)
///
/// # Arguments
/// * `bind_addr` - HTTP bind address (e.g., "0.0.0.0:3343")
/// * `metrics_addr` - Metrics bind address (e.g., "0.0.0.0:9093")
///
/// # Returns
/// * Opaque pointer to TomRelayHandle on success
/// * NULL on failure (check logs for details)
///
/// # Safety
/// * Caller must call `tom_relay_stop()` to free resources
/// * `bind_addr` and `metrics_addr` must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_relay_start(
    bind_addr: *const c_char,
    metrics_addr: *const c_char,
) -> *mut TomRelayHandle {
    init_tracing();

    // Parse C strings
    let bind_addr_str = match unsafe { CStr::from_ptr(bind_addr) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Invalid bind_addr UTF-8: {}", e);
            return std::ptr::null_mut();
        }
    };

    let metrics_addr_str = match unsafe { CStr::from_ptr(metrics_addr) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Invalid metrics_addr UTF-8: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Parse socket addresses
    let bind_socket: std::net::SocketAddr = match bind_addr_str.parse() {
        Ok(addr) => addr,
        Err(e) => {
            tracing::error!("Invalid bind_addr socket: {}", e);
            return std::ptr::null_mut();
        }
    };

    let metrics_socket: std::net::SocketAddr = match metrics_addr_str.parse() {
        Ok(addr) => addr,
        Err(e) => {
            tracing::error!("Invalid metrics_addr socket: {}", e);
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

    // Build server config (dev mode: HTTP only, no TLS)
    // Use std::io::Error for EC/EA type parameters (required for TLS configs, but unused here)
    let server_config = tom_relay::server::ServerConfig::<std::io::Error> {
        relay: Some(tom_relay::server::RelayConfig::<std::io::Error> {
            http_bind_addr: bind_socket,
            limits: tom_relay::server::Limits {
                accept_conn_limit: None,
                accept_conn_burst: None,
                client_rx: None,
            },
            tls: None,
            key_cache_capacity: None,
            access: tom_relay::server::AccessConfig::Everyone,
        }),
        quic: None,
        #[cfg(feature = "metrics")]
        metrics_addr: Some(metrics_socket),
    };

    let server_arc = Arc::new(Mutex::new(None));
    let status_arc = Arc::new(Mutex::new("Starting...".to_string()));

    let server_clone = server_arc.clone();
    let status_clone = status_arc.clone();

    // Spawn server on the runtime
    runtime.spawn(async move {
        *status_clone.lock().await = "Initializing server...".to_string();

        match tom_relay::server::Server::spawn(server_config).await {
            Ok(server) => {
                tracing::info!("tom-relay server started successfully");
                *status_clone.lock().await = "Running".to_string();
                *server_clone.lock().await = Some(server);
            }
            Err(e) => {
                tracing::error!("Failed to spawn relay server: {}", e);
                *status_clone.lock().await = format!("Error: {}", e);
            }
        }
    });

    // Return opaque handle
    Box::into_raw(Box::new(TomRelayHandle {
        runtime,
        server: server_arc,
        status: status_arc,
    }))
}

/// Stop the relay server and free all resources
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_relay_start()`
/// * After calling this, `handle` is invalid and must not be used
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_relay_stop(handle: *mut TomRelayHandle) {
    if handle.is_null() {
        tracing::warn!("tom_relay_stop called with NULL handle");
        return;
    }

    let handle_box = unsafe { Box::from_raw(handle) };

    handle_box.runtime.block_on(async {
        let mut server_guard = handle_box.server.lock().await;
        if let Some(server) = server_guard.take() {
            tracing::info!("Shutting down tom-relay server...");
            if let Err(e) = server.shutdown().await {
                tracing::error!("Error during shutdown: {}", e);
            } else {
                tracing::info!("tom-relay server stopped successfully");
            }
        }
        *handle_box.status.lock().await = "Stopped".to_string();
    });

    drop(handle_box);
}

/// Get the current status of the relay server
///
/// # Returns
/// * JSON string with status (caller must free with `tom_relay_free_string()`)
/// * NULL if handle is invalid
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_relay_start()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_relay_status(handle: *const TomRelayHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    let status = handle_ref.runtime.block_on(async {
        handle_ref.status.lock().await.clone()
    });

    // Convert to JSON
    let json = format!(r#"{{"status":"{}"}}"#, status);

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string returned by `tom_relay_status()`
///
/// # Safety
/// * `s` must be a valid pointer returned by `tom_relay_status()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_relay_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = unsafe { CString::from_raw(s) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_lifecycle() {
        use std::ffi::CString;

        let bind_addr = CString::new("127.0.0.1:13343").unwrap();
        let metrics_addr = CString::new("127.0.0.1:19093").unwrap();

        unsafe {
            let handle = tom_relay_start(bind_addr.as_ptr(), metrics_addr.as_ptr());
            assert!(!handle.is_null(), "tom_relay_start should return valid handle");

            // Give server time to start
            std::thread::sleep(std::time::Duration::from_millis(500));

            let status_ptr = tom_relay_status(handle);
            assert!(!status_ptr.is_null(), "tom_relay_status should return valid pointer");

            let status_cstr = CStr::from_ptr(status_ptr);
            let status_str = status_cstr.to_str().unwrap();
            println!("Status: {}", status_str);

            tom_relay_free_string(status_ptr);
            tom_relay_stop(handle);
        }
    }
}
