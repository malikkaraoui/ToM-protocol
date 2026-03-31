/// Embedded relay service — starts/stops a real `tom-relay` server inside the node process.
///
/// Phase R16: local relay lifecycle + publication-ready state.
/// The service manages the server. RuntimeState tracks logical state and publication.
use std::net::SocketAddr;

use tom_connect::RelayUrl;
use tom_relay::server::{AccessConfig, Limits, RelayConfig, Server, ServerConfig};

/// Status of the embedded relay — lifecycle-local only.
///
/// `Healthy` means the server bound successfully and is listening.
/// It does NOT mean "publicly reachable" or "publishable to the network".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddedRelayStatus {
    /// Relay is not running.
    Stopped,
    /// Relay is in the process of starting.
    Starting,
    /// Relay bound successfully and is listening.
    Healthy,
    /// Relay failed to start or crashed.
    Failed(String),
}

/// Logical state tracked by RuntimeState for the local embedded relay.
///
/// Separation from `EmbeddedRelayStatus` (which lives in the async service):
/// this lives in pure state, never touches I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalEmbeddedRelayState {
    /// No embedded relay running.
    Stopped,
    /// Relay is starting (effect sent, awaiting callback).
    Starting,
    /// Relay is healthy and listening.
    Healthy { bound_relay_url: RelayUrl },
    /// Relay failed to start.
    Failed { error: String, last_failure_at: u64 },
}

/// Publication state of the embedded relay.
///
/// Separate from health: a relay can be healthy but not published,
/// or published but no longer healthy (stale publication).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddedRelayPublicationState {
    /// Not published to the network.
    NotPublished,
    /// Published via RelayReadyAnnounce.
    Published { url: RelayUrl, published_at: u64 },
}

/// Configuration for the embedded relay.
///
/// MVP: bind_addr + optional advertise IP for cross-machine reachability.
#[derive(Debug, Clone)]
pub struct EmbeddedRelayConfig {
    /// Socket address to bind the HTTP relay on (e.g. `[::]:0` for dual-stack, random port).
    pub bind_addr: SocketAddr,
    /// Advertised IP for the relay URL published to the network.
    /// When `None`, auto-detects the machine's outbound IP.
    pub advertise_addr: Option<std::net::IpAddr>,
}

/// Manages the lifecycle of an embedded `tom-relay` server.
///
/// Lives outside `RuntimeState` — owned by the async runtime loop.
/// RuntimeState communicates via effects/commands, never touches this directly.
pub struct EmbeddedRelayService {
    status: EmbeddedRelayStatus,
    server: Option<Server>,
    config: Option<EmbeddedRelayConfig>,
    bound_relay_url: Option<RelayUrl>,
}

impl Default for EmbeddedRelayService {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedRelayService {
    /// Create a new service in Stopped state.
    pub fn new() -> Self {
        Self {
            status: EmbeddedRelayStatus::Stopped,
            server: None,
            config: None,
            bound_relay_url: None,
        }
    }

    /// Start the embedded relay server.
    ///
    /// Returns the bound URL on success.
    /// On failure, sets status to `Failed` and returns the error.
    pub async fn start(&mut self, config: EmbeddedRelayConfig) -> Result<RelayUrl, String> {
        if self.status == EmbeddedRelayStatus::Healthy {
            if let Some(ref url) = self.bound_relay_url {
                return Ok(url.clone());
            }
        }

        self.status = EmbeddedRelayStatus::Starting;

        let relay_config = ServerConfig::<(), ()> {
            relay: Some(RelayConfig {
                http_bind_addr: config.bind_addr,
                tls: None,
                limits: Limits::default(),
                key_cache_capacity: None,
                peer_present_k: None,
                access: AccessConfig::Everyone,
            }),
            quic: None,
            metrics_addr: None,
        };

        match Server::spawn(relay_config).await {
            Ok(server) => {
                let http_addr = server.http_addr().ok_or_else(|| {
                    "server started but no HTTP address available".to_string()
                })?;

                // Determine the advertised address:
                // 1. Explicit advertise_addr from config
                // 2. Auto-detect outbound IP if bind was unspecified
                // 3. Fall back to bound address as-is
                let advertise_ip = config.advertise_addr.or_else(|| {
                    let bound_ip = http_addr.ip();
                    if bound_ip.is_unspecified() || bound_ip.is_loopback() {
                        detect_outbound_ip()
                    } else {
                        Some(bound_ip)
                    }
                });

                let url_str = match advertise_ip {
                    Some(ip) => format!("http://{}:{}", format_ip_for_url(ip), http_addr.port()),
                    None => format!("http://{http_addr}"),
                };
                let url: RelayUrl = url_str.parse().map_err(|e| {
                    format!("invalid relay URL '{url_str}': {e}")
                })?;

                tracing::info!(%url, "embedded relay started");

                self.bound_relay_url = Some(url.clone());
                self.server = Some(server);
                self.config = Some(config);
                self.status = EmbeddedRelayStatus::Healthy;

                Ok(url)
            }
            Err(e) => {
                let err = format!("embedded relay failed to start: {e}");
                tracing::error!("{err}");
                self.status = EmbeddedRelayStatus::Failed(err.clone());
                Err(err)
            }
        }
    }

    /// Stop the embedded relay server gracefully.
    pub async fn stop(&mut self) {
        if let Some(server) = self.server.take() {
            tracing::info!("stopping embedded relay");
            if let Err(e) = server.shutdown().await {
                tracing::warn!("embedded relay shutdown error: {e}");
            }
        }
        self.status = EmbeddedRelayStatus::Stopped;
        self.bound_relay_url = None;
        self.config = None;
    }

    /// Current status of the embedded relay.
    pub fn status(&self) -> &EmbeddedRelayStatus {
        &self.status
    }

    /// The URL the relay is listening on, if healthy.
    pub fn bound_relay_url(&self) -> Option<&RelayUrl> {
        self.bound_relay_url.as_ref()
    }
}

/// Detect the machine's outbound IP by connecting a UDP socket to a public address.
/// No traffic is actually sent. Returns `None` if detection fails.
fn detect_outbound_ip() -> Option<std::net::IpAddr> {
    // Try IPv4 first (most common), then IPv6
    if let Ok(ip) = detect_outbound_ipv4() {
        return Some(std::net::IpAddr::V4(ip));
    }
    if let Ok(ip) = detect_outbound_ipv6() {
        return Some(std::net::IpAddr::V6(ip));
    }
    tracing::warn!("could not auto-detect outbound IP for embedded relay");
    None
}

fn detect_outbound_ipv4() -> Result<std::net::Ipv4Addr, std::io::Error> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    match socket.local_addr()?.ip() {
        std::net::IpAddr::V4(ip) => Ok(ip),
        _ => Err(std::io::Error::other("not ipv4")),
    }
}

fn detect_outbound_ipv6() -> Result<std::net::Ipv6Addr, std::io::Error> {
    let socket = std::net::UdpSocket::bind("[::]:0")?;
    socket.connect("[2001:4860:4860::8888]:80")?;
    match socket.local_addr()?.ip() {
        std::net::IpAddr::V6(ip) => Ok(ip),
        _ => Err(std::io::Error::other("not ipv6")),
    }
}

/// Format an IP address for use in a URL (IPv6 needs brackets).
fn format_ip_for_url(ip: std::net::IpAddr) -> String {
    match ip {
        std::net::IpAddr::V4(v4) => v4.to_string(),
        std::net::IpAddr::V6(v6) => format!("[{v6}]"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_stop() {
        let mut service = EmbeddedRelayService::new();
        assert_eq!(*service.status(), EmbeddedRelayStatus::Stopped);
        assert!(service.bound_relay_url().is_none());

        // Start on random port
        let config = EmbeddedRelayConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            advertise_addr: None,
        };
        let url = service.start(config).await.expect("should start");
        assert_eq!(*service.status(), EmbeddedRelayStatus::Healthy);
        assert!(service.bound_relay_url().is_some());
        // URL may be 127.0.0.1 or auto-detected outbound IP
        assert!(url.to_string().starts_with("http://"));

        // Stop
        service.stop().await;
        assert_eq!(*service.status(), EmbeddedRelayStatus::Stopped);
        assert!(service.bound_relay_url().is_none());
    }

    #[tokio::test]
    async fn test_bind_failure() {
        // Bind to a port first, then try to bind again on the same port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let occupied_addr = listener.local_addr().unwrap();

        let mut service = EmbeddedRelayService::new();
        let config = EmbeddedRelayConfig {
            bind_addr: occupied_addr,
            advertise_addr: None,
        };
        let result = service.start(config).await;
        assert!(result.is_err());
        assert!(matches!(service.status(), EmbeddedRelayStatus::Failed(_)));
    }

    #[tokio::test]
    async fn test_double_start_idempotent() {
        let mut service = EmbeddedRelayService::new();

        let config = EmbeddedRelayConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            advertise_addr: None,
        };
        let url1 = service.start(config.clone()).await.expect("first start");

        // Second start with same service returns same URL (idempotent)
        let url2 = service.start(config).await.expect("second start");
        assert_eq!(url1, url2);

        service.stop().await;
    }

    #[tokio::test]
    async fn test_no_auto_publication() {
        // Verify that starting an embedded relay does NOT produce any gossip
        // or PeerAnnounce side effect. The service is pure lifecycle.
        let mut service = EmbeddedRelayService::new();
        let config = EmbeddedRelayConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            advertise_addr: None,
        };
        let _url = service.start(config).await.expect("should start");

        // The service has no gossip sender, no topology, no announcement mechanism.
        // It only manages Server lifecycle. This test documents that contract.
        assert_eq!(*service.status(), EmbeddedRelayStatus::Healthy);

        service.stop().await;
    }
}
