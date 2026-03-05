use crate::{NodeId, TomTransportError};

use tom_connect::endpoint::Connection;
use tom_connect::{Endpoint, EndpointAddr};
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Caches QUIC connections per peer. First `send()` triggers connect,
/// subsequent sends reuse the cached connection.
pub(crate) struct ConnectionPool {
    endpoint: Endpoint,
    connections: Mutex<HashMap<NodeId, Connection>>,
    addresses: Mutex<HashMap<NodeId, EndpointAddr>>,
    alpn: Vec<u8>,
    /// Default relay URLs to include when no address is stored for a peer.
    /// Used when n0 discovery is disabled — the pool will try each relay in
    /// order before failing the connection attempt.
    default_relay_urls: Mutex<Vec<tom_connect::RelayUrl>>,
}

impl ConnectionPool {
    pub fn new(
        endpoint: Endpoint,
        alpn: Vec<u8>,
        default_relay_urls: Vec<tom_connect::RelayUrl>,
    ) -> Self {
        Self {
            endpoint,
            connections: Mutex::new(HashMap::new()),
            addresses: Mutex::new(HashMap::new()),
            alpn,
            default_relay_urls: Mutex::new(default_relay_urls),
        }
    }

    /// Replace default relay URL candidates used when no peer address is known.
    pub async fn set_default_relay_urls(&self, relays: Vec<tom_connect::RelayUrl>) {
        *self.default_relay_urls.lock().await = relays;
    }

    /// Return current default relay URL candidates.
    pub async fn default_relay_urls(&self) -> Vec<tom_connect::RelayUrl> {
        self.default_relay_urls.lock().await.clone()
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

        // Create new connection candidates — use stored address first, or
        // fallback to configured relay list (when n0 discovery is disabled).
        let stored_addr = {
            let addrs = self.addresses.lock().await;
            addrs.get(&target).cloned()
        };

        let default_relay_urls = self.default_relay_urls.lock().await.clone();

        let candidates: Vec<EndpointAddr> = if let Some(addr) = stored_addr {
            vec![addr]
        } else if !default_relay_urls.is_empty() {
            default_relay_urls
                .iter()
                .cloned()
                .map(|relay| EndpointAddr::new(*target.as_endpoint_id()).with_relay_url(relay))
                .collect()
        } else {
            vec![EndpointAddr::new(*target.as_endpoint_id())]
        };

        let mut last_err = None;
        let mut established = None;
        for addr in candidates {
            match self.endpoint.connect(addr, &self.alpn).await {
                Ok(conn) => {
                    established = Some(conn);
                    break;
                }
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }

        let conn = if let Some(conn) = established {
            conn
        } else {
            return Err(TomTransportError::Connect {
                node_id: target,
                source: last_err
                    .expect("at least one connect attempt should have been made")
                    .into(),
            });
        };

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
