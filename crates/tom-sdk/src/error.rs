//! SDK error type.

/// Errors returned by the ToM SDK.
///
/// Marked `#[non_exhaustive]`: new variants may be added in minor releases,
/// always keep a wildcard arm when matching.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TomSdkError {
    /// The underlying transport failed (bind, QUIC, relay connection).
    #[error("transport error: {0}")]
    Transport(#[from] tom_transport::TomTransportError),

    /// The protocol layer rejected the operation.
    #[error("protocol error: {0}")]
    Protocol(#[from] tom_protocol::TomProtocolError),

    /// A peer ticket could not be parsed (see [`crate::TomClient::ticket`]).
    #[error("invalid peer ticket: {0}")]
    InvalidTicket(String),

    /// The relay URL passed to the builder is not a valid URL.
    #[error("invalid relay url: {0}")]
    InvalidRelayUrl(String),

    /// The client is shut down — the runtime no longer accepts commands.
    #[error("client closed")]
    ClientClosed,
}
