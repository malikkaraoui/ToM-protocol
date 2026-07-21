//! Gate handler for filtering incoming connections by peer whitelist.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use tom_base::EndpointId;
use tom_connect::protocol::{AcceptError, ProtocolHandler};
use tom_connect::endpoint::Connection;
use tracing::warn;

/// Gate handler that wraps another protocol handler and filters connections
/// by peer whitelist.
///
/// If a whitelist is set (Some(set)), only connections from peers in the set
/// are accepted. If the whitelist is None (default), all connections are accepted.
/// This supports dynamic updates via the shared Arc.
///
/// Scope: INCOMING connections only — outgoing dials are never filtered here.
/// Connections accepted before the whitelist was set are not closed
/// retroactively (callers must set the list before any traffic; see
/// `TomNode::set_allowed_peers`).
///
/// Lock is released before any await to prevent deadlock.
#[derive(Debug)]
pub struct GatedHandler<H: ProtocolHandler> {
    inner: H,
    /// Arc-wrapped list of allowed peers. None = accept all (default).
    /// Some(set) = whitelist mode, only these peers allowed.
    allowed: Arc<RwLock<Option<HashSet<EndpointId>>>>,
}

impl<H: ProtocolHandler> GatedHandler<H> {
    /// Create a new gated handler wrapping the given handler with a shared allowed peers list.
    pub fn new(inner: H, allowed: Arc<RwLock<Option<HashSet<EndpointId>>>>) -> Self {
        Self { inner, allowed }
    }
}

impl<H: ProtocolHandler> ProtocolHandler for GatedHandler<H> {
    // on_accepting() uses the default trait implementation (no override).
    // The gate lives in accept() where remote_id is available post-handshake.
    // NOTE: if a future ProtocolHandler override on_accepting/0-RTT, this wrapper
    // must be revisited to gate at that earlier point.

    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        // Get the remote peer ID. After handshake completion, this should be available.
        let peer_id = connection.remote_id();

        // Check allowlist (lock released before await)
        let allowed = {
            let guard = self.allowed.read().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref() {
                None => true,
                Some(set) => set.contains(&peer_id),
            }
        };

        if !allowed {
            warn!(
                peer_id = %peer_id,
                "connection rejected by gate: peer not in whitelist"
            );
            connection.close(1u32.into(), b"not allowed");
            return Ok(());
        }

        // Delegate to inner handler
        self.inner.accept(connection).await
    }

    async fn shutdown(&self) {
        self.inner.shutdown().await
    }
}
