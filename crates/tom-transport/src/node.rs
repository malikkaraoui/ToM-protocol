use crate::config::TomNodeConfig;
use crate::connection::ConnectionPool;
use crate::envelope::MessageEnvelope;
use crate::path::{PathEvent, PathKind};
use crate::protocol::{self, HandlerState, TomProtocolHandler};
use crate::{NodeId, TomTransportError};

use iroh::protocol::Router;
use iroh::Endpoint;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

/// A ToM transport node — bind, send, receive, monitor paths.
///
/// This is the main entry point for consumers. It wraps iroh's `Endpoint`
/// and `Router` behind a stable API.
pub struct TomNode {
    id: NodeId,
    pool: Arc<ConnectionPool>,
    incoming_rx: mpsc::Receiver<(NodeId, MessageEnvelope)>,
    incoming_raw_rx: mpsc::Receiver<(NodeId, Vec<u8>)>,
    path_event_tx: broadcast::Sender<PathEvent>,
    _router: Router,
    endpoint: Endpoint,
    max_message_size: usize,
}

impl TomNode {
    /// Create and bind a new ToM transport node.
    ///
    /// Generates a fresh Ed25519 identity and starts listening for
    /// incoming connections.
    pub async fn bind(config: TomNodeConfig) -> Result<Self, TomTransportError> {
        let endpoint = Endpoint::bind()
            .await
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

        let router = Router::builder(endpoint.clone())
            .accept(config.alpn.clone(), Arc::new(handler))
            .spawn();

        let pool = Arc::new(ConnectionPool::new(endpoint.clone(), config.alpn));

        Ok(Self {
            id,
            pool,
            incoming_rx,
            incoming_raw_rx,
            path_event_tx,
            _router: router,
            endpoint,
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
    pub fn addr(&self) -> iroh::EndpointAddr {
        self.endpoint.addr()
    }

    /// Add a known peer address (for bootstrap or manual discovery).
    pub async fn add_peer_addr(&self, addr: iroh::EndpointAddr) {
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
