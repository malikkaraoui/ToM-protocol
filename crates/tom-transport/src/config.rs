use std::path::PathBuf;

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
    /// Custom relay URL list (priority order).
    ///
    /// When non-empty, this list is preferred over `relay_url` for endpoint setup.
    /// The first relay is used as fallback hint when n0 discovery is disabled.
    pub(crate) relay_urls: Vec<tom_connect::RelayUrl>,
    /// Relay discovery endpoint URL.
    ///
    /// When set, TomNode will fetch `GET <relay_discovery_url>/relays` at bind
    /// time and merge discovered relay URLs into the local relay priority list.
    /// Failed fetches are non-fatal and fallback to static relay configuration.
    pub(crate) relay_discovery_url: Option<String>,
    /// Enable n0-computer address discovery (Pkarr/DNS).
    ///
    /// When `true` (default), the node publishes and resolves addresses via
    /// n0's Pkarr relay and DNS infrastructure. Set to `false` when running
    /// your own relay and using gossip-based peer discovery instead.
    pub(crate) n0_discovery: bool,
    /// Path to persistent identity file (32-byte Ed25519 secret key).
    ///
    /// If set, the node loads its identity from this file (creating it on first run).
    /// If unset, a fresh ephemeral identity is generated on each bind.
    pub(crate) identity_path: Option<PathBuf>,
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
    /// as the relay server. This can be overridden with [`.relay_url()`] or
    /// [`.relay_urls()`].
    pub fn new() -> Self {
        let relay_url = std::env::var("TOM_RELAY_URL")
            .ok()
            .and_then(|s| s.parse().ok());

        let mut relay_urls: Vec<tom_connect::RelayUrl> = std::env::var("TOM_RELAY_URLS")
            .ok()
            .map(|s| {
                s.split(',')
                    .filter_map(|part| {
                        let trimmed = part.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            trimmed.parse().ok()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Backward-compat: TOM_RELAY_URL also seeds relay_urls when list is empty.
        if relay_urls.is_empty() {
            if let Some(url) = relay_url.clone() {
                relay_urls.push(url);
            }
        }

        let identity_path = std::env::var("TOM_IDENTITY_PATH")
            .ok()
            .map(PathBuf::from);

        let relay_discovery_url = std::env::var("TOM_RELAY_DISCOVERY_URL").ok();

        Self {
            alpn: crate::TOM_ALPN.to_vec(),
            max_message_size: 1024 * 1024, // 1 MB
            recv_buffer: 256,
            relay_url,
            relay_urls,
            relay_discovery_url,
            n0_discovery: true,
            identity_path,
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
        self.relay_url = Some(url.clone());
        self.relay_urls = vec![url];
        self
    }

    /// Use a custom relay list (priority order) instead of default n0 relays.
    ///
    /// The first relay in the list is used as fallback hint when n0 discovery
    /// is disabled.
    pub fn relay_urls(mut self, urls: Vec<tom_connect::RelayUrl>) -> Self {
        self.relay_url = urls.first().cloned();
        self.relay_urls = urls;
        self
    }

    /// Append a relay URL to the custom relay list.
    ///
    /// If no relay is currently configured, this relay also becomes `relay_url`.
    pub fn add_relay_url(mut self, url: tom_connect::RelayUrl) -> Self {
        if self.relay_url.is_none() {
            self.relay_url = Some(url.clone());
        }
        if !self.relay_urls.contains(&url) {
            self.relay_urls.push(url);
        }
        self
    }

    /// Configure relay discovery service URL.
    ///
    /// TomNode will query `<url>/relays` at bind time.
    pub fn relay_discovery_url(mut self, url: impl Into<String>) -> Self {
        self.relay_discovery_url = Some(url.into());
        self
    }

    /// Disable n0-computer address discovery (Pkarr/DNS).
    ///
    /// When disabled, the node does not publish or resolve addresses via
    /// n0's infrastructure. Peers must be discovered through gossip or
    /// added manually via [`TomNode::add_peer_addr`](crate::TomNode::add_peer_addr).
    ///
    /// Requires a custom relay URL to be set.
    ///
    /// ```rust
    /// use tom_transport::TomNodeConfig;
    ///
    /// let config = TomNodeConfig::new()
    ///     .relay_url("http://192.168.0.21:3340".parse().unwrap())
    ///     .n0_discovery(false);
    /// ```
    pub fn n0_discovery(mut self, enabled: bool) -> Self {
        self.n0_discovery = enabled;
        self
    }

    /// Use a persistent identity stored at the given path.
    ///
    /// The file contains a raw 32-byte Ed25519 secret key seed.
    /// If the file doesn't exist, a new identity is generated and saved.
    /// If unset, checks the `TOM_IDENTITY_PATH` environment variable.
    ///
    /// ```rust
    /// use tom_transport::TomNodeConfig;
    ///
    /// let config = TomNodeConfig::new()
    ///     .identity_path("/home/user/.tom/identity.key".into());
    /// ```
    pub fn identity_path(mut self, path: PathBuf) -> Self {
        self.identity_path = Some(path);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::TomNodeConfig;

    #[test]
    fn relay_url_sets_single_priority_list() {
        let url: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let cfg = TomNodeConfig::new().relay_url(url.clone());

        assert_eq!(cfg.relay_url, Some(url.clone()));
        assert_eq!(cfg.relay_urls, vec![url]);
    }

    #[test]
    fn relay_urls_sets_first_as_primary() {
        let r1: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let r2: tom_connect::RelayUrl = "http://127.0.0.1:3341".parse().unwrap();

        let cfg = TomNodeConfig::new().relay_urls(vec![r1.clone(), r2.clone()]);
        assert_eq!(cfg.relay_url, Some(r1.clone()));
        assert_eq!(cfg.relay_urls, vec![r1, r2]);
    }

    #[test]
    fn add_relay_url_deduplicates_and_preserves_primary() {
        let r1: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let r2: tom_connect::RelayUrl = "http://127.0.0.1:3341".parse().unwrap();

        let cfg = TomNodeConfig::new()
            .add_relay_url(r1.clone())
            .add_relay_url(r2.clone())
            .add_relay_url(r1.clone());

        assert_eq!(cfg.relay_url, Some(r1.clone()));
        assert_eq!(cfg.relay_urls, vec![r1, r2]);
    }

    #[test]
    fn relay_discovery_url_sets_value() {
        let cfg = TomNodeConfig::new().relay_discovery_url("http://127.0.0.1:8080");
        assert_eq!(
            cfg.relay_discovery_url.as_deref(),
            Some("http://127.0.0.1:8080")
        );
    }
}
