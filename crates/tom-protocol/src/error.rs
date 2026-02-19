/// Protocol-level errors for ToM.
///
/// Wraps transport errors and adds protocol-specific variants
/// (crypto, routing, serialization).
#[derive(Debug, thiserror::Error)]
pub enum TomProtocolError {
    #[error("transport error: {0}")]
    Transport(#[from] tom_transport::TomTransportError),

    #[error("invalid envelope: {reason}")]
    InvalidEnvelope { reason: String },

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("peer unreachable: {node_id}")]
    PeerUnreachable { node_id: String },

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("deserialization error: {0}")]
    Deserialization(String),

    #[error("signature verification failed")]
    InvalidSignature,

    #[error("relay rejected message: {reason}")]
    RelayRejected { reason: String },
}

impl From<rmp_serde::encode::Error> for TomProtocolError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        TomProtocolError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for TomProtocolError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        TomProtocolError::Deserialization(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_invalid_envelope() {
        let err = TomProtocolError::InvalidEnvelope {
            reason: "missing signature".into(),
        };
        assert_eq!(err.to_string(), "invalid envelope: missing signature");
    }

    #[test]
    fn test_display_crypto() {
        let err = TomProtocolError::Crypto("decryption failed".into());
        assert_eq!(err.to_string(), "crypto error: decryption failed");
    }

    #[test]
    fn test_display_peer_unreachable() {
        let err = TomProtocolError::PeerUnreachable {
            node_id: "abc123".into(),
        };
        assert_eq!(err.to_string(), "peer unreachable: abc123");
    }

    #[test]
    fn test_display_invalid_signature() {
        let err = TomProtocolError::InvalidSignature;
        assert_eq!(err.to_string(), "signature verification failed");
    }

    #[test]
    fn test_display_relay_rejected() {
        let err = TomProtocolError::RelayRejected {
            reason: "ttl exceeded".into(),
        };
        assert_eq!(err.to_string(), "relay rejected message: ttl exceeded");
    }
}
