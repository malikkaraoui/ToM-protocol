use std::net::SocketAddr;
use std::path::PathBuf;

/// Fallback relay list (public relays) used when discovery fails
/// and no static relay is configured.
pub const DEFAULT_RELAY_URLS: &[&str] = &[
    "https://relay-eu.tom-protocol.org",
    "https://relay-us.tom-protocol.org",
    "https://relay-asia.tom-protocol.org",
];

/// Relais fallback privé injecté à la COMPILATION, prioritaire sur les
/// relais publics. Réservé aux déploiements privés (flotte perso, tests) :
/// le défaut committé reste les relais publics — aucun nœud privilégié
/// dans le code source (doctrine ADR-010).
///
/// ```sh
/// TOM_EXTRA_FALLBACK_RELAY=http://192.0.2.1:3340 cargo build -p tom-transport
/// ```
///
/// En local, `.cargo/config.toml` (hors git) peut le fixer via `[env]`.
pub const EXTRA_FALLBACK_RELAY: Option<&str> = option_env!("TOM_EXTRA_FALLBACK_RELAY");

/// DNS TXT fallback domain queried when discovery HTTP is unavailable.
///
/// TXT records should contain one or more relay URLs.
pub const DEFAULT_DNS_FALLBACK_DOMAIN: &str = "_relay._tcp.tom-protocol.org";

pub(crate) fn fallback_relay_urls() -> Vec<tom_connect::RelayUrl> {
    EXTRA_FALLBACK_RELAY
        .iter()
        .chain(DEFAULT_RELAY_URLS.iter())
        .filter_map(|url| url.parse::<tom_connect::RelayUrl>().ok())
        .collect()
}

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
    /// time and periodically refresh discovered relay URLs into the local
    /// relay priority list. Failed fetches are non-fatal and fallback to
    /// static relay configuration.
    pub(crate) relay_discovery_url: Option<String>,
    /// Optional DNS TXT fallback domain for relay discovery.
    ///
    /// If set (or if default is used), the transport can query TXT records
    /// to obtain relay URLs when HTTP discovery is unavailable.
    pub(crate) relay_dns_fallback_domain: Option<String>,
    /// Enable n0-computer address discovery (Pkarr/DNS).
    ///
    /// When `true` (default), the node publishes and resolves addresses via
    /// n0's Pkarr relay and DNS infrastructure. Set to `false` when running
    /// your own relay and using gossip-based peer discovery instead.
    pub(crate) n0_discovery: bool,
    /// Enable local LAN discovery via mDNS bootstrap hints.
    ///
    /// Enabled by default so the protocol runtime can consume LAN bootstrap
    /// hints and fold them into the same bootstrap flow as PeerPresent and DHT.
    pub(crate) local_discovery: bool,
    /// Disable direct IP transports and keep only relay-assisted connectivity.
    ///
    /// Useful for deterministic relay-only tests and benchmarks where direct
    /// path opening would otherwise introduce noise.
    pub(crate) relay_only: bool,
    /// Path to persistent identity file (32-byte Ed25519 secret key).
    ///
    /// If set, the node loads its identity from this file (creating it on first run).
    /// If unset, a fresh ephemeral identity is generated on each bind.
    pub(crate) identity_path: Option<PathBuf>,
    /// Fixed local UDP socket address for the QUIC endpoint.
    ///
    /// If set, the endpoint binds its IP socket to this address instead of an
    /// OS-assigned ephemeral port. A stable port is required for durable
    /// inbound firewall rules (e.g. opening IPv6 on a router). When unset
    /// (default), the OS assigns an ephemeral port on each bind.
    pub(crate) bind_addr: Option<SocketAddr>,
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
        let relay_dns_fallback_domain = std::env::var("TOM_RELAY_DNS_FALLBACK_DOMAIN").ok();

        let bind_addr = std::env::var("TOM_BIND_ADDR")
            .ok()
            .and_then(|s| s.parse().ok());

        Self {
            alpn: crate::TOM_ALPN.to_vec(),
            max_message_size: 1024 * 1024, // 1 MB
            recv_buffer: 256,
            relay_url,
            relay_urls,
            relay_discovery_url,
            relay_dns_fallback_domain,
            n0_discovery: true,
            local_discovery: true,
            relay_only: false,
            identity_path,
            bind_addr,
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
    /// TomNode will query `<url>/relays` at bind time and periodically refresh.
    pub fn relay_discovery_url(mut self, url: impl Into<String>) -> Self {
        self.relay_discovery_url = Some(url.into());
        self
    }

    /// Configure DNS TXT fallback domain for relay discovery.
    pub fn relay_dns_fallback_domain(mut self, domain: impl Into<String>) -> Self {
        self.relay_dns_fallback_domain = Some(domain.into());
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

    /// Enable or disable local LAN discovery via mDNS.
    pub fn local_discovery(mut self, enabled: bool) -> Self {
        self.local_discovery = enabled;
        self
    }

    /// Enable or disable relay-only transport mode.
    ///
    /// When enabled, the node removes all direct IP transports and publishes
    /// relay addresses only. This is intended for measurement and deterministic
    /// relay-path validation.
    pub fn relay_only(mut self, enabled: bool) -> Self {
        self.relay_only = enabled;
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

    /// Set a fixed local UDP socket address for the QUIC endpoint.
    ///
    /// Binds the endpoint to a stable port instead of an OS-assigned ephemeral
    /// one. Required for durable inbound firewall rules. Use `[::]:PORT` to bind
    /// IPv6 (and IPv4 via dual-stack) or `0.0.0.0:PORT` for IPv4 only.
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = Some(addr);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_RELAY_URLS, TomNodeConfig, fallback_relay_urls};

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

    #[test]
    fn relay_dns_fallback_domain_sets_value() {
        let cfg = TomNodeConfig::new().relay_dns_fallback_domain("_relay._tcp.example.org");
        assert_eq!(
            cfg.relay_dns_fallback_domain.as_deref(),
            Some("_relay._tcp.example.org")
        );
    }

    #[test]
    fn fallback_relay_urls_contains_default_public_relays() {
        let parsed = fallback_relay_urls();
        // Robuste à TOM_EXTRA_FALLBACK_RELAY (compile-time) : les relais
        // publics sont toujours là, l'extra s'ajoute devant s'il est défini.
        let extra = usize::from(super::EXTRA_FALLBACK_RELAY.is_some());
        assert_eq!(parsed.len(), DEFAULT_RELAY_URLS.len() + extra);
        for url in DEFAULT_RELAY_URLS {
            let parsed_url: tom_connect::RelayUrl = url.parse().unwrap();
            assert!(parsed.contains(&parsed_url), "relais public manquant: {url}");
        }
        if let Some(extra_url) = super::EXTRA_FALLBACK_RELAY {
            assert_eq!(parsed[0], extra_url.parse::<tom_connect::RelayUrl>().unwrap());
        }
    }

    #[test]
    fn local_discovery_is_enabled_by_default() {
        let cfg = TomNodeConfig::new();
        assert!(cfg.local_discovery);
    }

    #[test]
    fn local_discovery_sets_value() {
        let cfg = TomNodeConfig::new().local_discovery(true);
        assert!(cfg.local_discovery);
    }

    #[test]
    fn relay_only_is_disabled_by_default() {
        let cfg = TomNodeConfig::new();
        assert!(!cfg.relay_only);
    }

    #[test]
    fn relay_only_sets_value() {
        let cfg = TomNodeConfig::new().relay_only(true);
        assert!(cfg.relay_only);
    }
}
