use crate::{NodeId, TomTransportError};

use iroh::endpoint::Connection;
use iroh::{Endpoint, EndpointAddr};
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Caches QUIC connections per peer. First `send()` triggers connect,
/// subsequent sends reuse the cached connection.
pub(crate) struct ConnectionPool {
    endpoint: Endpoint,
    connections: Mutex<HashMap<NodeId, Connection>>,
    addresses: Mutex<HashMap<NodeId, EndpointAddr>>,
    alpn: Vec<u8>,
}

impl ConnectionPool {
    pub fn new(endpoint: Endpoint, alpn: Vec<u8>) -> Self {
        Self {
            endpoint,
            connections: Mutex::new(HashMap::new()),
            addresses: Mutex::new(HashMap::new()),
            alpn,
        }
    }

    /// Store a known address for a peer.
    pub async fn add_addr(&self, id: NodeId, addr: EndpointAddr) {
        self.addresses.lock().await.insert(id, addr);
    }

    /// Get an existing connection or create a new one.
    pub async fn get_or_connect(
        &self,
        target: NodeId,
    ) -> Result<Connection, TomTransportError> {
        let mut conns = self.connections.lock().await;

        // Check if we have a cached connection that's still alive
        if let Some(conn) = conns.get(&target) {
            // connection.close_reason() returns Some if closed
            if conn.close_reason().is_none() {
                return Ok(conn.clone());
            }
            // Connection is dead, remove it
            conns.remove(&target);
        }

        // Create new connection — use stored address or fall back to discovery
        let addr = {
            let addrs = self.addresses.lock().await;
            addrs
                .get(&target)
                .cloned()
                .unwrap_or_else(|| EndpointAddr::new(*target.as_endpoint_id()))
        };
        let conn = self
            .endpoint
            .connect(addr, &self.alpn)
            .await
            .map_err(|e| TomTransportError::Connect {
                node_id: target,
                source: e.into(),
            })?;

        conns.insert(target, conn.clone());
        Ok(conn)
    }

    /// Remove a connection from the cache (e.g., after send failure).
    pub async fn remove(&self, target: &NodeId) {
        self.connections.lock().await.remove(target);
    }

    /// List all cached (connected) peers.
    pub async fn connected_peers(&self) -> Vec<NodeId> {
        let conns = self.connections.lock().await;
        conns
            .iter()
            .filter(|(_, conn)| conn.close_reason().is_none())
            .map(|(id, _)| *id)
            .collect()
    }
}
