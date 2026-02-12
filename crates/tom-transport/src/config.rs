/// Configuration for a [`TomNode`](crate::TomNode).
///
/// All fields have sensible defaults. Use the builder pattern:
///
/// ```rust
/// use tom_transport::TomNodeConfig;
///
/// let config = TomNodeConfig::new()
///     .max_message_size(2 * 1024 * 1024)
///     .recv_buffer(512);
/// ```
#[derive(Debug, Clone)]
pub struct TomNodeConfig {
    /// ALPN protocol identifier.
    pub(crate) alpn: Vec<u8>,
    /// Maximum incoming message size in bytes.
    pub(crate) max_message_size: usize,
    /// Channel buffer size for incoming messages.
    pub(crate) recv_buffer: usize,
}

impl Default for TomNodeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TomNodeConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self {
            alpn: crate::TOM_ALPN.to_vec(),
            max_message_size: 1024 * 1024, // 1 MB
            recv_buffer: 256,
        }
    }

    /// Set the ALPN protocol identifier.
    pub fn alpn(mut self, alpn: &[u8]) -> Self {
        self.alpn = alpn.to_vec();
        self
    }

    /// Set maximum incoming message size (default: 1 MB).
    pub fn max_message_size(mut self, bytes: usize) -> Self {
        self.max_message_size = bytes;
        self
    }

    /// Set the channel buffer size for incoming messages (default: 256).
    pub fn recv_buffer(mut self, capacity: usize) -> Self {
        self.recv_buffer = capacity;
        self
    }
}
