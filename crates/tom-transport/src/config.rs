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
    /// Custom relay URL. If set, only this relay is used instead of the n0 defaults.
    pub(crate) relay_url: Option<tom_connect::RelayUrl>,
}

impl Default for TomNodeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TomNodeConfig {
    /// Create a new config with defaults.
    ///
    /// If the `TOM_RELAY_URL` environment variable is set, it will be used
    /// as the relay server. This can be overridden with [`.relay_url()`].
    pub fn new() -> Self {
        let relay_url = std::env::var("TOM_RELAY_URL")
            .ok()
            .and_then(|s| s.parse().ok());

        Self {
            alpn: crate::TOM_ALPN.to_vec(),
            max_message_size: 1024 * 1024, // 1 MB
            recv_buffer: 256,
            relay_url,
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

    /// Use a custom relay server instead of the default n0 relays.
    ///
    /// ```rust
    /// use tom_transport::TomNodeConfig;
    ///
    /// let config = TomNodeConfig::new()
    ///     .relay_url("http://192.168.0.21:3340".parse().unwrap());
    /// ```
    pub fn relay_url(mut self, url: tom_connect::RelayUrl) -> Self {
        self.relay_url = Some(url);
        self
    }
}
