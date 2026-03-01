use crate::NodeId;

/// Errors returned by the ToM transport layer.
#[derive(Debug, thiserror::Error)]
pub enum TomTransportError {
    #[error("failed to bind endpoint: {0}")]
    Bind(#[source] anyhow::Error),

    #[error("connection to {node_id} failed: {source}")]
    Connect {
        node_id: NodeId,
        #[source]
        source: anyhow::Error,
    },

    #[error("send to {node_id} failed: {source}")]
    Send {
        node_id: NodeId,
        #[source]
        source: anyhow::Error,
    },

    #[error("receive failed: {0}")]
    Receive(#[source] anyhow::Error),

    #[error("envelope serialization failed: {0}")]
    Serialization(#[source] serde_json::Error),

    #[error("envelope deserialization failed: {0}")]
    Deserialization(#[source] serde_json::Error),

    #[error("message too large: {size} bytes (max {max})")]
    MessageTooLarge { size: usize, max: usize },

    #[error("node is shut down")]
    Shutdown,

    #[error("invalid node id: {0}")]
    InvalidNodeId(String),

    #[error("invalid configuration: {0}")]
    Config(String),
}

// Allow anyhow -> TomTransportError for convenience in protocol handler
impl From<anyhow::Error> for TomTransportError {
    fn from(e: anyhow::Error) -> Self {
        TomTransportError::Receive(e)
    }
}
