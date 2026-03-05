use crate::config::TomNodeConfig;
use crate::connection::ConnectionPool;
use crate::envelope::MessageEnvelope;
use crate::path::{PathEvent, PathKind};
use crate::protocol::{self, HandlerState, TomProtocolHandler};
use crate::{NodeId, TomTransportError};

use tom_base::SecretKey;
use tom_connect::protocol::Router;
use tom_connect::{Endpoint, RelayMode};
use tom_gossip::Gossip;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

/// A ToM transport node — bind, send, receive, monitor paths.
///
/// This is the main entry point for consumers. It wraps tom-connect's `Endpoint`
/// and `Router` behind a stable API.
pub struct TomNode {
    id: NodeId,
    pool: Arc<ConnectionPool>,
    incoming_rx: mpsc::Receiver<(NodeId, MessageEnvelope)>,
    incoming_raw_rx: mpsc::Receiver<(NodeId, Vec<u8>)>,
    path_event_tx: broadcast::Sender<PathEvent>,
    _router: Router,
    endpoint: Endpoint,
    gossip: Gossip,
    max_message_size: usize,
}

impl TomNode {
    /// Create and bind a new ToM transport node.
    ///
    /// If `identity_path` is configured, loads or creates a persistent identity.
    /// Otherwise, generates a fresh ephemeral Ed25519 identity.
    pub async fn bind(config: TomNodeConfig) -> Result<Self, TomTransportError> {
        // Load or generate identity
        let secret_key = match &config.identity_path {
            Some(path) => Some(load_or_create_identity(path)?),
            None => None,
        };

        let configured_relays = if !config.relay_urls.is_empty() {
            config.relay_urls.clone()
        } else {
            config.relay_url.clone().into_iter().collect()
        };

        let mut builder = match (configured_relays.is_empty(), config.n0_discovery) {
            (false, false) => {
                // Own relay, no n0 discovery — fully independent
                Endpoint::empty_builder(RelayMode::custom(configured_relays.clone()))
            }
            (false, true) => {
                // Own relay + n0 discovery (transition mode)
                Endpoint::builder()
                    .relay_mode(RelayMode::custom(configured_relays.clone()))
            }
            (true, false) => {
                // No relay, no discovery — local-only mode (tests, scenarios)
                Endpoint::empty_builder(RelayMode::Disabled)
            }
            (true, true) => {
                // Default: n0 presets (Pkarr/DNS + default relays)
                Endpoint::builder()
            }
        };

        if let Some(key) = secret_key {
            builder = builder.secret_key(key);
        }

        let endpoint = builder.bind().await
            .map_err(|e| TomTransportError::Bind(e.into()))?;

        let id = NodeId::from_endpoint_id(endpoint.id());

        let (incoming_tx, incoming_rx) = mpsc::channel(config.recv_buffer);
        let (incoming_raw_tx, incoming_raw_rx) = mpsc::channel(config.recv_buffer);
        let (path_event_tx, _) = broadcast::channel(64);

        let handler_state = Arc::new(HandlerState {
            incoming_tx,
            incoming_raw_tx,
            path_event_tx: path_event_tx.clone(),
            max_message_size: config.max_message_size,
        });

        let handler = TomProtocolHandler {
            state: handler_state,
        };

        let gossip = Gossip::builder().spawn(endpoint.clone());

        let router = Router::builder(endpoint.clone())
            .accept(config.alpn.clone(), Arc::new(handler))
            .accept(tom_gossip::ALPN, gossip.clone())
            .spawn();

        // When n0 discovery is off, pass relay URLs to the pool so it can
        // attempt fallback connections across relays (ordered by priority)
        // when no peer address is known.
        let default_relays = if !config.n0_discovery {
            configured_relays.clone()
        } else {
            Vec::new()
        };
        let pool = Arc::new(ConnectionPool::new(endpoint.clone(), config.alpn, default_relays));

        Ok(Self {
            id,
            pool,
            incoming_rx,
            incoming_raw_rx,
            path_event_tx,
            _router: router,
            endpoint,
            gossip,
            max_message_size: config.max_message_size,
        })
    }

    /// This node's identity (Ed25519 public key).
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// The 32-byte Ed25519 secret key seed.
    ///
    /// Needed by the protocol layer to sign envelopes and derive
    /// X25519 keys for encryption.
    pub fn secret_key_seed(&self) -> [u8; 32] {
        self.endpoint.secret_key().to_bytes()
    }

    /// This node's full address (identity + relay URL + direct addrs).
    ///
    /// Share this with other nodes so they can connect to you.
    pub fn addr(&self) -> tom_connect::EndpointAddr {
        self.endpoint.addr()
    }

    /// Access the gossip handle.
    ///
    /// Use this to subscribe to gossip topics for peer discovery.
    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    /// Add a known peer address (for bootstrap or manual discovery).
    pub async fn add_peer_addr(&self, addr: tom_connect::EndpointAddr) {
        let id = NodeId::from_endpoint_id(addr.id);
        self.pool.add_addr(id, addr).await;
    }

    /// Send an envelope to a peer.
    ///
    /// The connection is established on first use and cached for subsequent sends.
    pub async fn send(
        &self,
        to: NodeId,
        envelope: &MessageEnvelope,
    ) -> Result<(), TomTransportError> {
        let data = envelope
            .to_bytes()
            .map_err(TomTransportError::Serialization)?;
        self.send_raw(to, &data).await
    }

    /// Send raw bytes to a peer.
    pub async fn send_raw(
        &self,
        to: NodeId,
        data: &[u8],
    ) -> Result<(), TomTransportError> {
        if data.len() > self.max_message_size {
            return Err(TomTransportError::MessageTooLarge {
                size: data.len(),
                max: self.max_message_size,
            });
        }

        let conn = self.pool.get_or_connect(to).await?;

        let (mut send, mut recv) = match conn.open_bi().await {
            Ok(pair) => pair,
            Err(e) => {
                // Connection is dead (e.g. NAT rebinding) — evict from pool
                // so next attempt triggers a fresh connect + discovery.
                self.pool.remove(&to).await;
                return Err(TomTransportError::Send {
                    node_id: to,
                    source: e.into(),
                });
            }
        };

        if let Err(e) = protocol::write_framed(&mut send, data).await {
            // Connection may be dead, remove from pool
            self.pool.remove(&to).await;
            return Err(TomTransportError::Send {
                node_id: to,
                source: e,
            });
        }

        // Wait for the receiver to acknowledge (they close their send stream)
        let _ = recv.read_to_end(0).await;

        Ok(())
    }

    /// Receive the next incoming envelope. Blocks until one arrives.
    pub async fn recv(&mut self) -> Result<(NodeId, MessageEnvelope), TomTransportError> {
        self.incoming_rx
            .recv()
            .await
            .ok_or(TomTransportError::Shutdown)
    }

    /// Receive the next incoming raw message. Blocks until one arrives.
    pub async fn recv_raw(&mut self) -> Result<(NodeId, Vec<u8>), TomTransportError> {
        self.incoming_raw_rx
            .recv()
            .await
            .ok_or(TomTransportError::Shutdown)
    }

    /// Subscribe to path change events.
    pub fn path_events(&self) -> broadcast::Receiver<PathEvent> {
        self.path_event_tx.subscribe()
    }

    /// Get the current path kind for a connected peer.
    pub fn path_kind(&self, _peer: NodeId) -> Option<PathKind> {
        // TODO: Track per-peer path state from path watcher events
        None
    }

    /// Force-evict a peer connection from the pool.
    /// Next send() will trigger fresh connect + discovery.
    pub async fn disconnect(&self, peer: NodeId) {
        self.pool.remove(&peer).await;
    }

    /// List all currently connected peers.
    pub async fn connected_peers(&self) -> Vec<NodeId> {
        self.pool.connected_peers().await
    }

    /// Graceful shutdown.
    pub async fn shutdown(self) -> Result<(), TomTransportError> {
        self.endpoint.close().await;
        Ok(())
    }
}

/// Load an identity from a file, or create a new one if the file doesn't exist.
///
/// The file contains a raw 32-byte Ed25519 secret key seed.
/// On Unix, the file is created with permissions 0600 (owner read/write only).
fn load_or_create_identity(path: &Path) -> Result<SecretKey, TomTransportError> {
    if path.exists() {
        let bytes = std::fs::read(path).map_err(|e| {
            TomTransportError::Identity(format!("failed to read {}: {e}", path.display()))
        })?;
        let key_bytes: [u8; 32] = bytes.try_into().map_err(|v: Vec<u8>| {
            TomTransportError::Identity(format!(
                "invalid identity file {}: expected 32 bytes, got {}",
                path.display(),
                v.len()
            ))
        })?;
        Ok(SecretKey::from_bytes(&key_bytes))
    } else {
        let key = SecretKey::generate(&mut rand::rng());
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TomTransportError::Identity(format!(
                    "failed to create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        std::fs::write(path, key.to_bytes()).map_err(|e| {
            TomTransportError::Identity(format!("failed to write {}: {e}", path.display()))
        })?;
        // Set file permissions to 0600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms).map_err(|e| {
                TomTransportError::Identity(format!(
                    "failed to set permissions on {}: {e}",
                    path.display()
                ))
            })?;
        }
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_create_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        // First call: creates the file
        let key1 = load_or_create_identity(&path).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap().len(), 32);

        // Second call: loads the same identity
        let key2 = load_or_create_identity(&path).unwrap();
        assert_eq!(key1.to_bytes(), key2.to_bytes());
    }

    #[test]
    fn identity_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep").join("nested").join("identity.key");

        let key = load_or_create_identity(&path).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap().len(), 32);

        let reloaded = load_or_create_identity(&path).unwrap();
        assert_eq!(key.to_bytes(), reloaded.to_bytes());
    }

    #[test]
    fn identity_rejects_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.key");
        std::fs::write(&path, b"too short").unwrap();

        let result = load_or_create_identity(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected 32 bytes"));
    }

    #[cfg(unix)]
    #[test]
    fn identity_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        load_or_create_identity(&path).unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }

    #[tokio::test]
    async fn bind_with_persistent_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        // Bind twice with the same identity path — should get the same NodeId
        let config1 = TomNodeConfig::new()
            .n0_discovery(false)
            .identity_path(path.clone());
        let node1 = TomNode::bind(config1).await.unwrap();
        let id1 = node1.id();
        node1.shutdown().await.unwrap();

        let config2 = TomNodeConfig::new()
            .n0_discovery(false)
            .identity_path(path);
        let node2 = TomNode::bind(config2).await.unwrap();
        let id2 = node2.id();
        node2.shutdown().await.unwrap();

        assert_eq!(id1, id2, "Same identity file should produce same NodeId");
    }

    #[tokio::test]
    async fn bind_without_identity_path_is_ephemeral() {
        let config1 = TomNodeConfig::new().n0_discovery(false);
        let node1 = TomNode::bind(config1).await.unwrap();
        let id1 = node1.id();
        node1.shutdown().await.unwrap();

        let config2 = TomNodeConfig::new().n0_discovery(false);
        let node2 = TomNode::bind(config2).await.unwrap();
        let id2 = node2.id();
        node2.shutdown().await.unwrap();

        assert_ne!(id1, id2, "No identity path should produce different NodeIds");
    }
}
