//! Client builder — the single entry point of the SDK.

use std::path::PathBuf;

use tokio::sync::mpsc;
use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::client::TomClient;
use crate::error::TomSdkError;
use crate::event::{map_message, map_protocol_event, map_status};

/// Buffer size of the unified event channel handed to the application.
///
/// Backpressure: when the application stops draining events, the merger task
/// awaits on `send` (no drop on the SDK side) and the upstream runtime
/// channels fill in turn. 4096 absorbs bursts (gossip waves, group fan-out)
/// for a consumer that polls at least every few seconds. Making this
/// configurable is tracked in the S0→S3 review backlog.
const EVENT_BUFFER: usize = 4096;

/// Builds and connects a [`TomClient`].
///
/// # Examples
///
/// ```no_run
/// # async fn demo() -> Result<(), tom_sdk::TomSdkError> {
/// let mut client = tom_sdk::TomClientBuilder::new()
///     .username("alice")
///     .connect()
///     .await?;
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct TomClientBuilder {
    username: String,
    encryption: bool,
    relay_url: Option<String>,
    n0_discovery: bool,
    enable_dht: bool,
    identity_path: Option<PathBuf>,
    data_dir: Option<PathBuf>,
}

impl Default for TomClientBuilder {
    fn default() -> Self {
        Self {
            username: "anonymous".to_string(),
            encryption: true,
            relay_url: None,
            n0_discovery: true,
            enable_dht: true,
            identity_path: None,
            data_dir: None,
        }
    }
}

impl TomClientBuilder {
    /// New builder with defaults: encryption on, public discovery on,
    /// ephemeral identity and state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Username announced to peers (group membership, discovery).
    pub fn username(mut self, name: impl Into<String>) -> Self {
        self.username = name.into();
        self
    }

    /// Enable or disable end-to-end encryption of outbound messages.
    /// Enabled by default — disable only for debugging.
    pub fn encryption(mut self, enabled: bool) -> Self {
        self.encryption = enabled;
        self
    }

    /// Use a specific relay (e.g. `http://192.168.0.21:3340` for a
    /// self-hosted `tom-relay --dev`). When unset, the `TOM_RELAY_URL`
    /// environment variable then the default relays apply.
    pub fn relay_url(mut self, url: impl Into<String>) -> Self {
        self.relay_url = Some(url.into());
        self
    }

    /// Enable or disable public Pkarr/DNS discovery (n0 network).
    /// Disable to run fully self-contained (own relay only).
    pub fn n0_discovery(mut self, enabled: bool) -> Self {
        self.n0_discovery = enabled;
        self
    }

    /// Enable or disable DHT peer discovery (Mainline BEP-0044).
    pub fn dht(mut self, enabled: bool) -> Self {
        self.enable_dht = enabled;
        self
    }

    /// Persist the node identity (Ed25519 key) at this path.
    /// Without it a fresh identity is generated at every start.
    pub fn identity_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.identity_path = Some(path.into());
        self
    }

    /// Directory for persistent protocol state (SQLite).
    /// Without it the state is ephemeral.
    pub fn data_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.data_dir = Some(dir.into());
        self
    }

    /// Bind the node, start the protocol runtime and return a connected
    /// client.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::InvalidRelayUrl`] if the configured relay URL does not
    /// parse, [`TomSdkError::Transport`] if the node cannot bind.
    pub async fn connect(self) -> Result<TomClient, TomSdkError> {
        let mut node_config = TomNodeConfig::new().n0_discovery(self.n0_discovery);
        if let Some(url) = &self.relay_url {
            let parsed = url
                .parse()
                .map_err(|_| TomSdkError::InvalidRelayUrl(url.clone()))?;
            node_config = node_config.relay_url(parsed);
        }
        if let Some(path) = self.identity_path {
            node_config = node_config.identity_path(path);
        }

        let node = TomNode::bind(node_config).await?;
        let local_addr = node.addr();

        let runtime_config = RuntimeConfig {
            username: self.username,
            encryption: self.encryption,
            enable_dht: self.enable_dht,
            data_dir: self.data_dir,
            ..Default::default()
        };
        let channels = ProtocolRuntime::spawn(node, runtime_config);

        let (event_tx, event_rx) = mpsc::channel(EVENT_BUFFER);
        spawn_event_merger(channels.messages, channels.status_changes, channels.events, event_tx);

        Ok(TomClient::new(channels.handle, local_addr, event_rx))
    }
}

/// Merges the three runtime channels into the single SDK event stream.
/// Stops when the application drops the client (send fails) or when all
/// runtime channels close (shutdown).
///
/// Internal protocol events (forwarding, backups, roles, subnets, gossip,
/// anti-spam, relay lifecycle) are filtered out of the façade — each drop is
/// traced at debug level. Applications that need full access should depend
/// on `tom-protocol` directly.
fn spawn_event_merger(
    mut messages: mpsc::Receiver<tom_protocol::DeliveredMessage>,
    mut statuses: mpsc::Receiver<tom_protocol::StatusChange>,
    mut events: mpsc::Receiver<tom_protocol::ProtocolEvent>,
    tx: mpsc::Sender<crate::Event>,
) {
    tokio::spawn(async move {
        loop {
            let mapped = tokio::select! {
                Some(m) = messages.recv() => Some(map_message(m)),
                Some(s) = statuses.recv() => Some(map_status(s)),
                Some(e) = events.recv() => map_protocol_event(e),
                else => break,
            };
            if let Some(event) = mapped {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        }
    });
}
