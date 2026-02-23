use crate::types::NodeId;

/// Abstraction reseau pour le runtime.
///
/// En production : impl par TomNode (iroh QUIC).
/// En test : impl par MockTransport (enregistre les envois).
#[async_trait::async_trait]
pub trait Transport: Send {
    /// Envoyer des bytes bruts a un noeud cible.
    async fn send_raw(&self, target: NodeId, data: &[u8]) -> Result<(), String>;

    /// Lister les peers actuellement connectes.
    async fn connected_peers(&self) -> Vec<NodeId>;
}

// ── Impl pour TomNode (production) ──────────────────────────────────

#[async_trait::async_trait]
impl Transport for tom_transport::TomNode {
    async fn send_raw(&self, target: NodeId, data: &[u8]) -> Result<(), String> {
        tom_transport::TomNode::send_raw(self, target, data)
            .await
            .map_err(|e| e.to_string())
    }

    async fn connected_peers(&self) -> Vec<NodeId> {
        tom_transport::TomNode::connected_peers(self).await
    }
}

// ── MockTransport (tests) ───────────────────────────────────────────

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Faux transport qui enregistre les envois pour verification.
    #[derive(Clone)]
    pub struct MockTransport {
        sent: Arc<Mutex<Vec<(NodeId, Vec<u8>)>>>,
        peers: Arc<Mutex<Vec<NodeId>>>,
        fail_sends: Arc<Mutex<bool>>,
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self {
                sent: Arc::new(Mutex::new(Vec::new())),
                peers: Arc::new(Mutex::new(Vec::new())),
                fail_sends: Arc::new(Mutex::new(false)),
            }
        }

        pub fn sent(&self) -> Vec<(NodeId, Vec<u8>)> {
            self.sent.lock().unwrap().clone()
        }

        pub fn set_peers(&self, peers: Vec<NodeId>) {
            *self.peers.lock().unwrap() = peers;
        }

        pub fn set_fail_sends(&self, fail: bool) {
            *self.fail_sends.lock().unwrap() = fail;
        }

        pub fn clear_sent(&self) {
            self.sent.lock().unwrap().clear();
        }
    }

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn send_raw(&self, target: NodeId, data: &[u8]) -> Result<(), String> {
            if *self.fail_sends.lock().unwrap() {
                return Err("mock: send failed".to_string());
            }
            self.sent.lock().unwrap().push((target, data.to_vec()));
            Ok(())
        }

        async fn connected_peers(&self) -> Vec<NodeId> {
            self.peers.lock().unwrap().clone()
        }
    }
}
