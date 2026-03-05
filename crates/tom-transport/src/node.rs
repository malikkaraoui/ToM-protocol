use crate::config::TomNodeConfig;
use crate::connection::ConnectionPool;
use crate::envelope::MessageEnvelope;
use crate::path::{PathEvent, PathKind};
use crate::protocol::{self, HandlerState, TomProtocolHandler};
use crate::{NodeId, TomTransportError};

use tom_base::SecretKey;
use tom_connect::protocol::Router;
use tom_connect::{Endpoint, RelayMode};
use tom_gossip::Gossip;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;

#[derive(Debug, Deserialize)]
struct DiscoveryRelay {
    url: String,
}

#[derive(Debug, Deserialize)]
struct DiscoveryResponse {
    relays: Vec<DiscoveryRelay>,
    #[serde(default)]
    ttl_seconds: Option<u64>,
}

#[derive(Debug)]
struct DiscoverySnapshot {
    relays: Vec<tom_connect::RelayUrl>,
    ttl_seconds: Option<u64>,
}

fn normalize_discovery_relays(relays: Vec<DiscoveryRelay>) -> Vec<tom_connect::RelayUrl> {
    relays
        .into_iter()
        .filter_map(|relay| relay.url.parse::<tom_connect::RelayUrl>().ok())
        .collect()
}

fn merge_relay_lists(
    mut configured: Vec<tom_connect::RelayUrl>,
    discovered: Vec<tom_connect::RelayUrl>,
) -> Vec<tom_connect::RelayUrl> {
    for relay in discovered {
        if !configured.contains(&relay) {
            configured.push(relay);
        }
    }
    configured
}

fn parse_dns_txt_relays(records: &[String]) -> Vec<tom_connect::RelayUrl> {
    records
        .iter()
        .flat_map(|line| line.split(|c: char| c == ',' || c.is_whitespace()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<tom_connect::RelayUrl>().ok())
        .collect()
}

async fn fetch_dns_fallback_relays(
    domain: &str,
) -> Result<Vec<tom_connect::RelayUrl>, TomTransportError> {
    let resolver = hickory_resolver::TokioAsyncResolver::tokio_from_system_conf()
        .map_err(|e| TomTransportError::Config(format!("dns resolver init failed: {e}")))?;

    let lookup = resolver
        .txt_lookup(domain)
        .await
        .map_err(|e| TomTransportError::Config(format!("dns txt lookup failed: {e}")))?;

    let lines: Vec<String> = lookup
        .iter()
        .flat_map(|txt| txt.txt_data().iter())
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .map(ToOwned::to_owned)
        .collect();

    Ok(parse_dns_txt_relays(&lines))
}

async fn fetch_discovery_relays(
    discovery_base_url: &str,
) -> Result<DiscoverySnapshot, TomTransportError> {
    let base = discovery_base_url.trim_end_matches('/');
    let url = format!("{base}/relays");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| TomTransportError::Config(format!("invalid discovery client: {e}")))?;

    let payload = client
        .get(url)
        .send()
        .await
        .map_err(|e| TomTransportError::Config(format!("relay discovery request failed: {e}")))?
        .error_for_status()
        .map_err(|e| TomTransportError::Config(format!("relay discovery bad response: {e}")))?
        .json::<DiscoveryResponse>()
        .await
        .map_err(|e| TomTransportError::Config(format!("relay discovery invalid json: {e}")))?;

    Ok(DiscoverySnapshot {
        relays: normalize_discovery_relays(payload.relays),
        ttl_seconds: payload.ttl_seconds,
    })
}

fn next_discovery_refresh_delay(ttl_seconds: Option<u64>) -> Duration {
    // Clamp TTL to avoid over-polling and avoid stale lists for too long.
    let seconds = ttl_seconds.unwrap_or(30).clamp(5, 300);
    Duration::from_secs(seconds)
}

/// A ToM transport node — bind, send, receive, monitor paths.
///
/// This is the main entry point for consumers. It wraps tom-connect's `Endpoint`
/// and `Router` behind a stable API.
pub struct TomNode {
    id: NodeId,
    pool: Arc<ConnectionPool>,
    incoming_rx: mpsc::Receiver<(NodeId, MessageEnvelope)>,
    incoming_raw_rx: mpsc::Receiver<(NodeId, Vec<u8>)>,
    path_event_tx: broadcast::Sender<PathEvent>,
    _router: Router,
    endpoint: Endpoint,
    gossip: Gossip,
    max_message_size: usize,
    discovery_refresh_stop_tx: Option<oneshot::Sender<()>>,
    discovery_refresh_task: Option<JoinHandle<()>>,
}

impl TomNode {
    /// Create and bind a new ToM transport node.
    ///
    /// If `identity_path` is configured, loads or creates a persistent identity.
    /// Otherwise, generates a fresh ephemeral Ed25519 identity.
    pub async fn bind(config: TomNodeConfig) -> Result<Self, TomTransportError> {
        // Load or generate identity
        let secret_key = match &config.identity_path {
            Some(path) => Some(load_or_create_identity(path)?),
            None => None,
        };

        let mut configured_relays = if !config.relay_urls.is_empty() {
            config.relay_urls.clone()
        } else {
            config.relay_url.clone().into_iter().collect()
        };

        let mut discovery_refresh_delay = next_discovery_refresh_delay(None);
        if let Some(discovery_url) = config.relay_discovery_url.as_deref() {
            match fetch_discovery_relays(discovery_url).await {
                Ok(snapshot) => {
                    tracing::info!(
                        discovery_url = %discovery_url,
                        relays = snapshot.relays.len(),
                        "relay discovery fetched relays"
                    );
                    discovery_refresh_delay = next_discovery_refresh_delay(snapshot.ttl_seconds);
                    configured_relays = merge_relay_lists(configured_relays, snapshot.relays);
                }
                Err(err) => {
                    tracing::warn!(
                        discovery_url = %discovery_url,
                        error = %err,
                        "relay discovery failed"
                    );

                    if configured_relays.is_empty() {
                        configured_relays = crate::config::fallback_relay_urls();
                        tracing::info!(
                            relays = configured_relays.len(),
                            "using fallback relay list after discovery failure"
                        );
                    }
                }
            }
        }

        if configured_relays.is_empty() {
            let dns_domain = config
                .relay_dns_fallback_domain
                .as_deref()
                .unwrap_or(crate::config::DEFAULT_DNS_FALLBACK_DOMAIN);

            match fetch_dns_fallback_relays(dns_domain).await {
                Ok(relays) if !relays.is_empty() => {
                    tracing::info!(
                        dns_domain = %dns_domain,
                        relays = relays.len(),
                        "using DNS TXT relay fallback"
                    );
                    configured_relays = merge_relay_lists(configured_relays, relays);
                }
                Ok(_) => {
                    tracing::warn!(
                        dns_domain = %dns_domain,
                        "dns relay fallback returned no relay URLs"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        dns_domain = %dns_domain,
                        error = %err,
                        "dns relay fallback failed"
                    );
                }
            }
        }

        if configured_relays.is_empty() {
            configured_relays = crate::config::fallback_relay_urls();
            tracing::info!(
                relays = configured_relays.len(),
                "no relay configured, using fallback relay list"
            );
        }

        let mut builder = match (configured_relays.is_empty(), config.n0_discovery) {
            (false, false) => {
                // Own relay, no n0 discovery — fully independent
                Endpoint::empty_builder(RelayMode::custom(configured_relays.clone()))
            }
            (false, true) => {
                // Own relay + n0 discovery (transition mode)
                Endpoint::builder()
                    .relay_mode(RelayMode::custom(configured_relays.clone()))
            }
            (true, false) => {
                // No relay, no discovery — local-only mode (tests, scenarios)
                Endpoint::empty_builder(RelayMode::Disabled)
            }
            (true, true) => {
                // Default: n0 presets (Pkarr/DNS + default relays)
                Endpoint::builder()
            }
        };

        if let Some(key) = secret_key {
            builder = builder.secret_key(key);
        }

        let endpoint = builder.bind().await
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

        let gossip = Gossip::builder().spawn(endpoint.clone());

        let router = Router::builder(endpoint.clone())
            .accept(config.alpn.clone(), Arc::new(handler))
            .accept(tom_gossip::ALPN, gossip.clone())
            .spawn();

        // When n0 discovery is off, pass relay URLs to the pool so it can
        // attempt fallback connections across relays (ordered by priority)
        // when no peer address is known.
        let default_relays = if !config.n0_discovery {
            configured_relays.clone()
        } else {
            Vec::new()
        };
        let pool = Arc::new(ConnectionPool::new(endpoint.clone(), config.alpn, default_relays));

        let (discovery_refresh_stop_tx, discovery_refresh_task) =
            if let Some(discovery_url) = config.relay_discovery_url.clone() {
                let pool = Arc::clone(&pool);
                let n0_discovery_enabled = config.n0_discovery;
                let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
                let task = tokio::spawn(async move {
                    let mut delay = discovery_refresh_delay;
                    loop {
                        tokio::select! {
                            _ = &mut stop_rx => {
                                break;
                            }
                            _ = tokio::time::sleep(delay) => {
                                match fetch_discovery_relays(&discovery_url).await {
                                    Ok(snapshot) => {
                                        delay = next_discovery_refresh_delay(snapshot.ttl_seconds);
                                        if !n0_discovery_enabled {
                                            let current = pool.default_relay_urls().await;
                                            let merged = merge_relay_lists(current, snapshot.relays);
                                            pool.set_default_relay_urls(merged).await;
                                        }
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            discovery_url = %discovery_url,
                                            error = %err,
                                            "periodic relay discovery refresh failed"
                                        );
                                        delay = next_discovery_refresh_delay(None);
                                    }
                                }
                            }
                        }
                    }
                });
                (Some(stop_tx), Some(task))
            } else {
                (None, None)
            };

        Ok(Self {
            id,
            pool,
            incoming_rx,
            incoming_raw_rx,
            path_event_tx,
            _router: router,
            endpoint,
            gossip,
            max_message_size: config.max_message_size,
            discovery_refresh_stop_tx,
            discovery_refresh_task,
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
    pub fn addr(&self) -> tom_connect::EndpointAddr {
        self.endpoint.addr()
    }

    /// Access the gossip handle.
    ///
    /// Use this to subscribe to gossip topics for peer discovery.
    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    /// Add a known peer address (for bootstrap or manual discovery).
    pub async fn add_peer_addr(&self, addr: tom_connect::EndpointAddr) {
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
    pub async fn shutdown(mut self) -> Result<(), TomTransportError> {
        if let Some(stop_tx) = self.discovery_refresh_stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(task) = self.discovery_refresh_task.take() {
            let _ = task.await;
        }
        self.endpoint.close().await;
        Ok(())
    }
}

/// Load an identity from a file, or create a new one if the file doesn't exist.
///
/// The file contains a raw 32-byte Ed25519 secret key seed.
/// On Unix, the file is created with permissions 0600 (owner read/write only).
fn load_or_create_identity(path: &Path) -> Result<SecretKey, TomTransportError> {
    if path.exists() {
        let bytes = std::fs::read(path).map_err(|e| {
            TomTransportError::Identity(format!("failed to read {}: {e}", path.display()))
        })?;
        let key_bytes: [u8; 32] = bytes.try_into().map_err(|v: Vec<u8>| {
            TomTransportError::Identity(format!(
                "invalid identity file {}: expected 32 bytes, got {}",
                path.display(),
                v.len()
            ))
        })?;
        Ok(SecretKey::from_bytes(&key_bytes))
    } else {
        let key = SecretKey::generate(&mut rand::rng());
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TomTransportError::Identity(format!(
                    "failed to create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        std::fs::write(path, key.to_bytes()).map_err(|e| {
            TomTransportError::Identity(format!("failed to write {}: {e}", path.display()))
        })?;
        // Set file permissions to 0600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms).map_err(|e| {
                TomTransportError::Identity(format!(
                    "failed to set permissions on {}: {e}",
                    path.display()
                ))
            })?;
        }
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    async fn spawn_discovery_test_server(
        body: Arc<Mutex<String>>,
    ) -> (String, oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    accept = listener.accept() => {
                        let (mut socket, _) = match accept {
                            Ok(value) => value,
                            Err(_) => continue,
                        };

                        let payload = body.lock().unwrap().clone();
                        tokio::spawn(async move {
                            let mut buf = [0u8; 2048];
                            let _ = tokio::time::timeout(Duration::from_millis(500), socket.read(&mut buf)).await;

                            let response = format!(
                                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                                payload.len(),
                                payload
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                            let _ = socket.shutdown().await;
                        });
                    }
                }
            }
        });

        (format!("http://{}", addr), shutdown_tx)
    }

    #[test]
    fn identity_create_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        // First call: creates the file
        let key1 = load_or_create_identity(&path).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap().len(), 32);

        // Second call: loads the same identity
        let key2 = load_or_create_identity(&path).unwrap();
        assert_eq!(key1.to_bytes(), key2.to_bytes());
    }

    #[test]
    fn identity_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep").join("nested").join("identity.key");

        let key = load_or_create_identity(&path).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap().len(), 32);

        let reloaded = load_or_create_identity(&path).unwrap();
        assert_eq!(key.to_bytes(), reloaded.to_bytes());
    }

    #[test]
    fn identity_rejects_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.key");
        std::fs::write(&path, b"too short").unwrap();

        let result = load_or_create_identity(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected 32 bytes"));
    }

    #[cfg(unix)]
    #[test]
    fn identity_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        load_or_create_identity(&path).unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }

    #[tokio::test]
    async fn bind_with_persistent_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.key");

        // Bind twice with the same identity path — should get the same NodeId
        let config1 = TomNodeConfig::new()
            .n0_discovery(false)
            .identity_path(path.clone());
        let node1 = TomNode::bind(config1).await.unwrap();
        let id1 = node1.id();
        node1.shutdown().await.unwrap();

        let config2 = TomNodeConfig::new()
            .n0_discovery(false)
            .identity_path(path);
        let node2 = TomNode::bind(config2).await.unwrap();
        let id2 = node2.id();
        node2.shutdown().await.unwrap();

        assert_eq!(id1, id2, "Same identity file should produce same NodeId");
    }

    #[tokio::test]
    async fn bind_without_identity_path_is_ephemeral() {
        let config1 = TomNodeConfig::new().n0_discovery(false);
        let node1 = TomNode::bind(config1).await.unwrap();
        let id1 = node1.id();
        node1.shutdown().await.unwrap();

        let config2 = TomNodeConfig::new().n0_discovery(false);
        let node2 = TomNode::bind(config2).await.unwrap();
        let id2 = node2.id();
        node2.shutdown().await.unwrap();

        assert_ne!(id1, id2, "No identity path should produce different NodeIds");
    }

    #[test]
    fn normalize_discovery_relays_filters_invalid_urls() {
        let relays = vec![
            DiscoveryRelay {
                url: "http://127.0.0.1:3340".to_string(),
            },
            DiscoveryRelay {
                url: "not-a-url".to_string(),
            },
            DiscoveryRelay {
                url: "https://relay.example.org".to_string(),
            },
        ];

        let normalized = normalize_discovery_relays(relays);
        assert_eq!(normalized.len(), 2);
    }

    #[test]
    fn merge_relay_lists_preserves_order_and_deduplicates() {
        let a: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let b: tom_connect::RelayUrl = "http://127.0.0.1:3341".parse().unwrap();
        let c: tom_connect::RelayUrl = "http://127.0.0.1:3342".parse().unwrap();

        let merged = merge_relay_lists(vec![a.clone(), b.clone()], vec![b.clone(), c.clone()]);
        assert_eq!(merged, vec![a, b, c]);
    }

    #[test]
    fn parse_dns_txt_relays_accepts_commas_and_spaces() {
        let records = vec![
            "https://relay-eu.tom-protocol.org,https://relay-us.tom-protocol.org".to_string(),
            "https://relay-asia.tom-protocol.org".to_string(),
            "invalid-url".to_string(),
        ];

        let relays = parse_dns_txt_relays(&records);
        assert_eq!(relays.len(), 3);
    }

    #[test]
    fn next_discovery_refresh_delay_clamps_bounds() {
        assert_eq!(next_discovery_refresh_delay(None), Duration::from_secs(30));
        assert_eq!(next_discovery_refresh_delay(Some(1)), Duration::from_secs(5));
        assert_eq!(next_discovery_refresh_delay(Some(20)), Duration::from_secs(20));
        assert_eq!(next_discovery_refresh_delay(Some(999)), Duration::from_secs(300));
    }

    #[tokio::test]
    async fn fetch_discovery_relays_reads_relays_and_ttl() {
        let body = Arc::new(Mutex::new(
            r#"{"relays":[{"url":"http://127.0.0.1:3340"}],"ttl_seconds":7}"#
                .to_string(),
        ));
        let (base_url, shutdown_tx) = spawn_discovery_test_server(body).await;

        let snapshot = fetch_discovery_relays(&base_url).await.unwrap();
        assert_eq!(snapshot.relays.len(), 1);
        assert_eq!(snapshot.ttl_seconds, Some(7));

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn periodic_discovery_refresh_updates_pool_relays() {
        let body = Arc::new(Mutex::new(
            r#"{"relays":[{"url":"http://127.0.0.1:3340"}],"ttl_seconds":5}"#
                .to_string(),
        ));
        let (base_url, shutdown_tx) = spawn_discovery_test_server(Arc::clone(&body)).await;

        let config = TomNodeConfig::new()
            .n0_discovery(false)
            .relay_discovery_url(base_url);
        let node = TomNode::bind(config).await.unwrap();

        let relay_3340: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let relay_3341: tom_connect::RelayUrl = "http://127.0.0.1:3341".parse().unwrap();

        let initial = node.pool.default_relay_urls().await;
        assert!(initial.contains(&relay_3340));

        *body.lock().unwrap() =
            r#"{"relays":[{"url":"http://127.0.0.1:3341"}],"ttl_seconds":5}"#.to_string();

        let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
        loop {
            let relays = node.pool.default_relay_urls().await;
            if relays.contains(&relay_3341) {
                break;
            }

            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for periodic discovery refresh"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        node.shutdown().await.unwrap();
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn fallback_relays_used_when_discovery_fails() {
        let config = TomNodeConfig::new()
            .n0_discovery(false)
            .relay_discovery_url("http://127.0.0.1:9");

        let node = TomNode::bind(config).await.unwrap();
        let relays = node.pool.default_relay_urls().await;

        let eu: tom_connect::RelayUrl = "https://relay-eu.tom-protocol.org".parse().unwrap();
        let us: tom_connect::RelayUrl = "https://relay-us.tom-protocol.org".parse().unwrap();
        let asia: tom_connect::RelayUrl = "https://relay-asia.tom-protocol.org".parse().unwrap();

        assert_eq!(relays.len(), 3);
        assert!(relays.contains(&eu));
        assert!(relays.contains(&us));
        assert!(relays.contains(&asia));

        node.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn fallback_not_used_when_discovery_succeeds() {
        let body = Arc::new(Mutex::new(
            r#"{"relays":[{"url":"http://127.0.0.1:3340"}],"ttl_seconds":30}"#
                .to_string(),
        ));
        let (base_url, shutdown_tx) = spawn_discovery_test_server(body).await;

        let config = TomNodeConfig::new()
            .n0_discovery(false)
            .relay_discovery_url(base_url);
        let node = TomNode::bind(config).await.unwrap();
        let relays = node.pool.default_relay_urls().await;

        let discovered: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let fallback_eu: tom_connect::RelayUrl =
            "https://relay-eu.tom-protocol.org".parse().unwrap();

        assert!(relays.contains(&discovered));
        assert!(!relays.contains(&fallback_eu));

        node.shutdown().await.unwrap();
        let _ = shutdown_tx.send(());
    }
}
