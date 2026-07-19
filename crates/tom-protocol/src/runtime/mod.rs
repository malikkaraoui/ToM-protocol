/// Protocol runtime — integrates all protocol modules into a live event loop.
///
/// The runtime owns a `TomNode` (transport) and all protocol state (router,
/// topology, tracker, heartbeat). It exposes a channel-based API so the
/// application (TUI, bot, SDK) never touches raw bytes or protocol internals.
pub mod embedded_relay;
pub mod bootstrap;
mod clock_skew;
mod effect;
mod executor;
mod r#loop;
pub mod metrics;
mod pending;
mod state;
mod transport;

pub use effect::RuntimeEffect;
pub use embedded_relay::{
    EmbeddedRelayConfig, EmbeddedRelayPublicationState, EmbeddedRelayService,
    EmbeddedRelayStatus, LocalEmbeddedRelayState,
};
pub use bootstrap::{BootstrapPhase, BootstrapSource};
pub use metrics::{MetricsSnapshot, ProtocolMetrics};
pub use state::{GossipInput, RuntimeState};
pub use transport::Transport;

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tom_connect::{EndpointAddr, RelayUrl};
use tom_transport::{PathEvent, TomNode};

use crate::discovery::DiscoverySource;
use crate::group::{GroupId, GroupInfo, GroupInvite, GroupMember, GroupMessage, LeaveReason};
use crate::relay::PeerInfo;
use crate::tracker::StatusChange;
use crate::types::NodeId;

// ── Configuration ─────────────────────────────────────────────────────

/// Configuration for the protocol runtime.
pub struct RuntimeConfig {
    /// Enable E2E encryption for outbound messages.
    pub encryption: bool,
    /// Interval for router cache cleanup.
    pub cache_cleanup_interval: Duration,
    /// Interval for heartbeat liveness checks.
    pub heartbeat_interval: Duration,
    /// Interval for message tracker eviction.
    pub tracker_cleanup_interval: Duration,
    /// Local username for group membership.
    pub username: String,
    /// Application build number (non-authoritative hint, 0 = unknown).
    pub app_build: u32,
    /// Interval for group hub heartbeats.
    pub group_hub_heartbeat_interval: Duration,
    /// Interval for backup maintenance ticks.
    pub backup_tick_interval: Duration,
    /// Interval for gossip peer announcements.
    pub gossip_announce_interval: Duration,
    /// Bootstrap peers to join the gossip discovery network.
    pub gossip_bootstrap_peers: Vec<crate::types::NodeId>,
    /// Interval for shadow ping (watchdog).
    pub shadow_ping_interval: Duration,
    /// Enable DHT-based peer discovery (Phase R7.1).
    pub enable_dht: bool,
    /// Directory for persistent state (SQLite). None = ephemeral (no persistence).
    pub data_dir: Option<PathBuf>,
    /// Anti-spam configuration (progressive rate limiting).
    pub antispam_config: crate::roles::AntiSpamConfig,
    /// Enable the embedded relay server (Phase R16).
    pub enable_embedded_relay: bool,
    /// Enable publication of the embedded relay to the network.
    /// Requires `enable_embedded_relay` to be true.
    /// When false, relay starts but is not announced via gossip.
    pub enable_embedded_relay_publication: bool,
    /// TTL for relay registry entries (how long a discovered relay stays valid without refresh).
    pub relay_registry_ttl: Duration,
    /// Enable dynamic injection of discovered relay URLs into the transport layer.
    /// When false (default), RelayRegistry observes but does not mutate the Endpoint.
    pub enable_transport_relay_discovery: bool,
    /// Interval for periodic republication of RelayReadyAnnounce when the embedded
    /// relay is healthy and publication is enabled. Piggybacks on the heartbeat tick,
    /// so effective cadence is granularized by `heartbeat_interval`.
    /// Default: `relay_registry_ttl / 2`.
    pub relay_publish_interval: Duration,
    /// Bind address for the embedded relay server.
    /// Default: `[::]:0` (dual-stack, all interfaces, OS-assigned port).
    pub embedded_relay_bind_addr: std::net::SocketAddr,
    /// Advertised IP for the embedded relay URL published to the network.
    /// When `None`, the runtime auto-detects the machine's outbound IP.
    /// Set explicitly when auto-detection picks the wrong interface.
    pub embedded_relay_advertise_addr: Option<std::net::IpAddr>,
    /// L1-001: anti-Sybil gate — minimum LOCAL contribution score an
    /// attester must have (as observed by US) for its presence attestation
    /// to be accepted. Default: `presence::RELAY_CONTRIBUTION_MIN` (2.0).
    ///
    /// ⚠️ Lowering this weakens the Sybil defense. `0.0` accepts any
    /// well-formed signed attestation — ONLY for fleet plumbing tests
    /// (phase 1 of the L1-001 runbook), never a production default.
    pub presence_contribution_min: f64,
    /// L1-001: when set, the runtime automatically challenges up to 8
    /// Online peers at this interval (auto-probe). Feeds the Live Log on
    /// devices without any UI work. Default: `None` (off).
    pub presence_probe_interval: Option<Duration>,
    /// SIMULATION HOOK (test-only, default 0) — signed offset in ms added to
    /// the node's presence clock. Lets a harness inject clock skew to prove
    /// the anti-NTP hardening: freshness is judged on the challenger's OWN
    /// (internally consistent) clock, and an attester's declared timestamp is
    /// ignored. NOT exposed via FFI on purpose — it must never reach a
    /// production app. See `L1-001-matrice-flotte.md`.
    pub presence_clock_offset_ms: i64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            encryption: true,
            cache_cleanup_interval: Duration::from_secs(300),
            heartbeat_interval: Duration::from_secs(5),
            tracker_cleanup_interval: Duration::from_secs(300),
            username: "anonymous".to_string(),
            app_build: 0,
            group_hub_heartbeat_interval: Duration::from_secs(30),
            backup_tick_interval: Duration::from_secs(60),
            gossip_announce_interval: Duration::from_secs(10),
            gossip_bootstrap_peers: Vec::new(),
            shadow_ping_interval: Duration::from_secs(3),
            enable_dht: true, // Phase R7.1: Enable by default
            data_dir: None,
            antispam_config: crate::roles::AntiSpamConfig::default(),
            enable_embedded_relay: false,
            enable_embedded_relay_publication: false,
            relay_registry_ttl: Duration::from_secs(600), // 10 min
            enable_transport_relay_discovery: false,
            relay_publish_interval: Duration::from_secs(300), // 10 min TTL / 2
            embedded_relay_bind_addr: std::net::SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, 0)),
            embedded_relay_advertise_addr: None,
            presence_contribution_min: crate::presence::RELAY_CONTRIBUTION_MIN,
            presence_probe_interval: None,
            presence_clock_offset_ms: 0,
        }
    }
}

// ── Commands (app → runtime) ──────────────────────────────────────────

/// Commands the application sends to the runtime event loop.
pub enum RuntimeCommand {
    /// Send a chat message to a peer.
    SendMessage { to: NodeId, payload: Vec<u8> },
    /// Send a read receipt for a previously received message.
    SendReadReceipt {
        to: NodeId,
        original_message_id: String,
    },
    /// Register a peer in the network (triggers discovery via iroh).
    AddPeer { node_id: NodeId, source: DiscoverySource },
    /// Register a peer with its full network address (for direct connectivity).
    AddPeerAddr { addr: EndpointAddr },
    /// Update topology: add or refresh a peer.
    UpsertPeer { info: PeerInfo },
    /// Remove a peer from topology.
    RemovePeer { node_id: NodeId },
    /// Request current connected peers.
    GetConnectedPeers {
        reply: oneshot::Sender<Vec<NodeId>>,
    },
    // ── Group commands ──────────────────────────────
    /// Create a new group. This node becomes a member; hub_relay_id hosts the group.
    CreateGroup {
        name: String,
        hub_relay_id: NodeId,
        initial_members: Vec<NodeId>,
        invite_only: bool,
    },
    /// Accept a pending group invitation.
    AcceptInvite { group_id: GroupId },
    /// Decline a pending group invitation.
    DeclineInvite { group_id: GroupId },
    /// Leave a group.
    LeaveGroup { group_id: GroupId },
    /// Send a text message to a group.
    SendGroupMessage { group_id: GroupId, text: String },
    /// Query: list groups we belong to.
    GetGroups {
        reply: oneshot::Sender<Vec<GroupInfo>>,
    },
    /// Admin kicks a member from a group.
    KickMember { group_id: GroupId, target_id: NodeId },
    /// Admin changes a member's role.
    UpdateMemberRole {
        group_id: GroupId,
        target_id: NodeId,
        new_role: crate::group::GroupMemberRole,
    },
    /// Admin invites a member to an existing group.
    InviteMember { group_id: GroupId, target_id: NodeId },
    /// Query: list pending invitations.
    GetPendingInvites {
        reply: oneshot::Sender<Vec<GroupInvite>>,
    },
    // ── Role queries ──────────────────────────────
    /// Query role metrics for a specific peer (debug).
    GetRoleMetrics {
        node_id: NodeId,
        reply: oneshot::Sender<Option<crate::roles::RoleMetrics>>,
    },
    /// Query all peers with their scores (debug/dashboard).
    GetAllRoleScores {
        reply: oneshot::Sender<Vec<(NodeId, f64, crate::relay::PeerRole)>>,
    },
    // ── DHT discovery ──────────────────────────────
    /// DHT lookup completed — inject discovered address into transport.
    DhtLookupResult { addr: tom_dht::DhtNodeAddr },
    // ── Embedded relay ──────────────────────────────
    /// Start the embedded relay server.
    StartEmbeddedRelay { config: EmbeddedRelayConfig },
    /// Stop the embedded relay server.
    StopEmbeddedRelay,
    /// Embedded relay started successfully (loop → state feedback).
    EmbeddedRelayStarted { url: RelayUrl },
    /// Embedded relay failed to start (loop → state feedback).
    EmbeddedRelayFailed { error: String },
    /// Embedded relay stopped (loop → state feedback).
    EmbeddedRelayStopped,
    /// UPnP port mapping obtained for embedded relay (loop → state feedback).
    /// External address (IP + port) is now publicly reachable.
    /// Phase R13: Auto-detection of public relay address via IGD.
    EmbeddedRelayPortMapped {
        external_addr: std::net::SocketAddr,
    },
    /// Query the relay registry (read-only snapshot).
    GetKnownRelays {
        reply: oneshot::Sender<Vec<crate::discovery::RelayRegistryEntry>>,
    },
    // ── Presence (L1-001) ──────────────────────────
    /// Issue a presence challenge toward a peer (we become the challenger).
    CheckPresence { target: NodeId },
    /// Challenge many peers at once (stress driving).
    CheckPresenceMany { targets: Vec<NodeId> },
    /// Challenge every Online peer right now (on-demand probe burst).
    CheckPresenceAllOnline,
    /// Query the current presence aggregation window (seed + count).
    GetPresenceSeed {
        reply: oneshot::Sender<([u8; 32], usize)>,
    },
    /// Query the lifetime presence counters (per-outcome relevés).
    GetPresenceMetrics {
        reply: oneshot::Sender<crate::presence::PresenceMetrics>,
    },
    /// SIMULATION HOOK (test-only): set the presence clock offset live.
    SetPresenceClockOffset { offset_ms: i64 },
    /// Inject raw gossip bytes into the runtime for processing.
    ///
    /// Test/debug bridge: feeds bytes through the same deserialization pipeline
    /// as real gossip (PeerAnnounce, RoleChangeAnnounce, RelayReadyAnnounce).
    /// Effects are executed through the loop interceptor, including transport
    /// relay discovery if enabled.
    InjectGossipBytes { bytes: Vec<u8> },
    /// Query clock skew metrics (median inter-peer time delta).
    GetClockSkew {
        reply: oneshot::Sender<(Option<i64>, usize)>,
    },
    /// Graceful shutdown.
    Shutdown,
    /// Force an immediate state flush to persistent storage, replying when done.
    /// Graceful-stop paths use this so group membership + last_seq (R13 offline
    /// gap-fill) survive a stop/restart even within the periodic save window.
    SaveState {
        reply: oneshot::Sender<()>,
    },
}

// ── Events (runtime → app) ───────────────────────────────────────────

/// A delivered message from the network (decrypted, verified).
#[derive(Debug, Clone)]
pub struct DeliveredMessage {
    pub from: NodeId,
    pub payload: Vec<u8>,
    pub envelope_id: String,
    pub timestamp: u64,
    pub signature_valid: bool,
    pub was_encrypted: bool,
}

/// Protocol-level events the application may want to observe.
#[derive(Debug, Clone)]
pub enum ProtocolEvent {
    /// A new peer was discovered.
    PeerDiscovered {
        node_id: NodeId,
        username: String,
        app_build: u32,
        source: DiscoverySource,
    },
    /// A peer went stale (missed heartbeats but might recover).
    PeerStale { node_id: NodeId },
    /// A peer went offline (confirmed departed).
    PeerOffline { node_id: NodeId },
    /// A peer came back online after being stale/offline.
    PeerOnline { node_id: NodeId },
    /// A message was rejected by the router.
    MessageRejected { reason: String },
    /// We forwarded a message as relay.
    Forwarded {
        envelope_id: String,
        next_hop: NodeId,
    },
    /// Path changed for a peer (relay ↔ direct).
    PathChanged { event: PathEvent },
    /// Runtime encountered a non-fatal error.
    Error { description: String },
    // ── Presence events (L1-001) ──────────────────
    /// A presence attestation we solicited was verified and accepted.
    PresenceAttestationReceived {
        attester_id: NodeId,
        challenge_id: String,
        /// Round-trip measured on OUR clock (challenge issue → acceptance).
        latency_ms: u64,
    },
    // ── Group events ──────────────────────────────
    /// A group was created (we are a member).
    GroupCreated { group: GroupInfo },
    /// We received a group invitation.
    GroupInviteReceived { invite: GroupInvite },
    /// We joined a group (after accepting invite).
    GroupJoined {
        group_id: GroupId,
        group_name: String,
    },
    /// A member joined a group we belong to.
    GroupMemberJoined {
        group_id: GroupId,
        member: GroupMember,
    },
    /// A member left a group we belong to.
    GroupMemberLeft {
        group_id: GroupId,
        node_id: NodeId,
        username: String,
        reason: LeaveReason,
    },
    /// A group message was received.
    GroupMessageReceived { message: GroupMessage },
    /// The hub for a group migrated to a new node.
    GroupHubMigrated {
        group_id: GroupId,
        new_hub_id: NodeId,
    },
    /// A group security violation was detected (non-member or bad signature).
    GroupSecurityViolation {
        group_id: GroupId,
        node_id: NodeId,
        reason: String,
    },
    /// A member's role was changed by an admin.
    GroupMemberRoleChanged {
        group_id: GroupId,
        node_id: NodeId,
        new_role: crate::group::GroupMemberRole,
    },
    /// Shadow promoted to primary hub for a group.
    GroupShadowPromoted {
        group_id: GroupId,
        new_hub_id: NodeId,
    },
    /// This node was assigned as candidate for a group.
    GroupCandidateAssigned { group_id: GroupId },
    /// Hub failover chain fully restored after a promotion.
    GroupHubChainRestored { group_id: GroupId },
    // ── Discovery events ──────────────────────────
    /// A gossip neighbor connected.
    GossipNeighborUp { node_id: NodeId },
    /// A gossip neighbor disconnected.
    GossipNeighborDown { node_id: NodeId },
    // ── Subnet events ─────────────────────────────
    /// An ephemeral subnet was formed from communication patterns.
    SubnetFormed {
        subnet_id: String,
        members: Vec<NodeId>,
    },
    /// An ephemeral subnet was dissolved.
    SubnetDissolved { subnet_id: String, reason: String },
    // ── Role events ───────────────────────────────
    /// A peer was promoted to Relay based on contribution score.
    RolePromoted { node_id: NodeId, score: f64 },
    /// A peer was demoted to Peer due to low contribution score.
    RoleDemoted { node_id: NodeId, score: f64 },
    /// Our local role changed (update gossip announces).
    LocalRoleChanged { new_role: crate::relay::PeerRole },
    // ── Backup events ─────────────────────────────
    /// A message was stored as backup for an offline recipient.
    BackupStored {
        message_id: String,
        recipient_id: NodeId,
    },
    /// A backed-up message was delivered to its recipient.
    BackupDelivered {
        message_id: String,
        recipient_id: NodeId,
    },
    /// A backed-up message expired (TTL).
    BackupExpired {
        message_id: String,
        recipient_id: NodeId,
    },
    // ── Delivery events ─────────────────────────────
    /// A tracked outgoing message changed status
    /// (Pending → Sent → Relayed → Delivered / Failed). Surfaces the
    /// per-message lifecycle to the UI so it can show « en cours → délivré ».
    MessageStatusChanged {
        message_id: String,
        to: NodeId,
        status: crate::types::MessageStatus,
    },
    /// A message delivery was retried after ACK timeout.
    DeliveryRetry {
        message_id: String,
        to: NodeId,
        attempt: u8,
    },
    /// A message delivery failed after all retries exhausted.
    DeliveryTimeout {
        message_id: String,
        to: NodeId,
        last_status: crate::types::MessageStatus,
    },
    // ── Anti-spam events ─────────────────────────────
    /// A sender was throttled by progressive rate limiting.
    SenderThrottled {
        node_id: NodeId,
        score: f64,
        current_rate: f64,
    },
    // ── Embedded relay events ─────────────────────────
    /// The embedded relay started and is listening.
    EmbeddedRelayStarted { url: RelayUrl },
    /// The embedded relay failed to start.
    EmbeddedRelayFailed { error: String },
    /// The embedded relay was stopped.
    EmbeddedRelayStopped,
    /// A remote node published a relay-ready announcement.
    RelayReadyReceived {
        node_id: NodeId,
        relay_url: RelayUrl,
    },
    /// A relay registry entry expired (no refresh within TTL).
    RelayRegistryExpired {
        node_id: NodeId,
        relay_url: RelayUrl,
    },
    /// A discovered relay was injected into the transport layer.
    TransportRelayInserted {
        relay_url: RelayUrl,
    },
    /// A discovered relay was removed from the transport layer.
    TransportRelayRemoved {
        relay_url: RelayUrl,
    },
    // ── Discovery Timing (Instrumentation — Phase R12.1) ───────────────
    /// Discovery timing landmark for cold-start analysis.
    DiscoveryTiming {
        elapsed_ms: u64,
        detail: String,
    },
}

// ── RuntimeHandle (app-facing API) ───────────────────────────────────

/// Handle to communicate with a running ProtocolRuntime.
///
/// Cheap to clone. All methods are non-blocking channel sends.
#[derive(Clone)]
pub struct RuntimeHandle {
    cmd_tx: mpsc::Sender<RuntimeCommand>,
    local_id: NodeId,
    metrics: ProtocolMetrics,
}

impl RuntimeHandle {
    /// This node's identity.
    pub fn local_id(&self) -> NodeId {
        self.local_id
    }

    /// Send a chat message to a peer.
    ///
    /// The runtime handles relay selection, encryption, signing,
    /// serialization, transport, and status tracking.
    pub async fn send_message(&self, to: NodeId, payload: Vec<u8>) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::SendMessage { to, payload })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Send a read receipt for a message we received.
    pub async fn send_read_receipt(
        &self,
        to: NodeId,
        original_message_id: String,
    ) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::SendReadReceipt {
                to,
                original_message_id,
            })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Issue a presence challenge toward a peer (L1-001).
    ///
    /// The result arrives asynchronously as
    /// `ProtocolEvent::PresenceAttestationReceived` — or nothing at all if
    /// the peer is absent, lying, or below the local anti-Sybil gate.
    pub async fn check_presence(&self, target: NodeId) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::CheckPresence { target })
            .await;
    }

    /// Challenge many peers at once (L1-001 stress driving).
    pub async fn check_presence_many(&self, targets: Vec<NodeId>) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::CheckPresenceMany { targets })
            .await;
    }

    /// Challenge every Online peer right now (on-demand burst).
    pub async fn check_presence_all_online(&self) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::CheckPresenceAllOnline)
            .await;
    }

    /// Current presence entropy seed and attestation count (L1-001 window).
    pub async fn presence_seed(&self) -> Option<([u8; 32], usize)> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(RuntimeCommand::GetPresenceSeed { reply: tx })
            .await
            .ok()?;
        rx.await.ok()
    }

    /// Lifetime presence counters (per-outcome relevés for stress campaigns).
    pub async fn presence_metrics(&self) -> Option<crate::presence::PresenceMetrics> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(RuntimeCommand::GetPresenceMetrics { reply: tx })
            .await
            .ok()?;
        rx.await.ok()
    }

    /// SIMULATION HOOK (test-only): inject a presence clock offset (ms) live.
    pub async fn set_presence_clock_offset(&self, offset_ms: i64) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::SetPresenceClockOffset { offset_ms })
            .await;
    }

    /// Clock skew observability: median inter-peer time delta (ms) and sample count.
    /// Returns (median_ms, sample_count). median_ms is None if < 5 samples collected.
    pub async fn clock_skew(&self) -> Option<(Option<i64>, usize)> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(RuntimeCommand::GetClockSkew { reply: tx })
            .await
            .ok()?;
        rx.await.ok()
    }

    /// Register a peer in the network (triggers iroh discovery).
    pub async fn add_peer(&self, node_id: NodeId) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::AddPeer { node_id, source: DiscoverySource::Direct })
            .await;
    }

    /// Register a peer with its full network address.
    ///
    /// Use this when you have the peer's `EndpointAddr` (e.g. from a local
    /// peer exchange) to enable direct connectivity without relay discovery.
    pub async fn add_peer_addr(&self, addr: EndpointAddr) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::AddPeerAddr { addr })
            .await;
    }

    /// Update topology with peer information.
    pub async fn upsert_peer(&self, info: PeerInfo) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::UpsertPeer { info })
            .await;
    }

    /// Remove a peer from topology.
    pub async fn remove_peer(&self, node_id: NodeId) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::RemovePeer { node_id })
            .await;
    }

    /// Get currently connected peers.
    pub async fn connected_peers(&self) -> Vec<NodeId> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::GetConnectedPeers { reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    // ── Group methods ──────────────────────────────

    /// Create a new group. hub_relay_id will host the group state.
    pub async fn create_group(
        &self,
        name: String,
        hub_relay_id: NodeId,
        initial_members: Vec<NodeId>,
    ) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::CreateGroup {
                name,
                hub_relay_id,
                initial_members,
                invite_only: false,
            })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Create an invite-only group. Only explicitly invited members can join.
    pub async fn create_group_invite_only(
        &self,
        name: String,
        hub_relay_id: NodeId,
        initial_members: Vec<NodeId>,
    ) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::CreateGroup {
                name,
                hub_relay_id,
                initial_members,
                invite_only: true,
            })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Accept a pending group invitation.
    pub async fn accept_invite(&self, group_id: GroupId) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::AcceptInvite { group_id })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Decline a pending group invitation.
    pub async fn decline_invite(&self, group_id: GroupId) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::DeclineInvite { group_id })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Leave a group.
    pub async fn leave_group(&self, group_id: GroupId) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::LeaveGroup { group_id })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Send a text message to a group.
    pub async fn send_group_message(
        &self,
        group_id: GroupId,
        text: String,
    ) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::SendGroupMessage { group_id, text })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Get all groups we belong to.
    pub async fn groups(&self) -> Vec<GroupInfo> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::GetGroups { reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    /// Kick a member from a group (admin only).
    pub async fn kick_member(
        &self,
        group_id: GroupId,
        target_id: NodeId,
    ) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::KickMember { group_id, target_id })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Change a member's role in a group (admin only).
    pub async fn update_member_role(
        &self,
        group_id: GroupId,
        target_id: NodeId,
        new_role: crate::group::GroupMemberRole,
    ) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::UpdateMemberRole {
                group_id,
                target_id,
                new_role,
            })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Invite a member to an existing group (admin only).
    pub async fn invite_member(
        &self,
        group_id: GroupId,
        target_id: NodeId,
    ) -> Result<(), crate::TomProtocolError> {
        self.cmd_tx
            .send(RuntimeCommand::InviteMember { group_id, target_id })
            .await
            .map_err(|_| crate::TomProtocolError::InvalidEnvelope {
                reason: "runtime shut down".into(),
            })
    }

    /// Get pending group invitations.
    pub async fn pending_invites(&self) -> Vec<GroupInvite> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::GetPendingInvites { reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    // ── Role queries ──────────────────────────────

    /// Get role metrics for a peer (debug).
    pub async fn get_role_metrics(&self, node_id: NodeId) -> Option<crate::roles::RoleMetrics> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::GetRoleMetrics {
                node_id,
                reply: tx,
            })
            .await;
        rx.await.ok().flatten()
    }

    /// Get all peers with their role scores (debug).
    pub async fn get_all_role_scores(
        &self,
    ) -> Vec<(NodeId, f64, crate::relay::PeerRole)> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::GetAllRoleScores { reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    // ── Embedded relay ──────────────────────────────

    /// Start the embedded relay server.
    pub async fn start_embedded_relay(&self, config: EmbeddedRelayConfig) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::StartEmbeddedRelay { config })
            .await;
    }

    /// Stop the embedded relay server.
    pub async fn stop_embedded_relay(&self) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::StopEmbeddedRelay)
            .await;
    }

    /// Query all known relays from the registry (read-only snapshot, sorted by freshest first).
    pub async fn get_known_relays(&self) -> Vec<crate::discovery::RelayRegistryEntry> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::GetKnownRelays { reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    /// Inject raw gossip bytes into the runtime for processing.
    ///
    /// Test/debug bridge: feeds bytes through the same deserialization and
    /// effect pipeline as real gossip events, including the loop interceptor
    /// for transport relay discovery.
    pub async fn inject_gossip_bytes(&self, bytes: Vec<u8>) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::InjectGossipBytes { bytes })
            .await;
    }

    /// Graceful shutdown.
    pub async fn shutdown(&self) {
        let _ = self.cmd_tx.send(RuntimeCommand::Shutdown).await;
    }

    /// Force an immediate state flush to persistent storage, awaiting completion.
    /// Call before a hard exit (e.g. `/stop`) so recent group membership and
    /// R13 `last_seq` are durably persisted for offline gap-fill after restart.
    pub async fn save_now(&self) {
        let (tx, rx) = oneshot::channel();
        if self
            .cmd_tx
            .send(RuntimeCommand::SaveState { reply: tx })
            .await
            .is_ok()
        {
            let _ = rx.await;
        }
    }

    /// Get a snapshot of all protocol metrics.
    pub fn metrics(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }
}

// ── RuntimeChannels ──────────────────────────────────────────────────

/// Channels returned to the application when the runtime starts.
pub struct RuntimeChannels {
    /// Handle to send commands to the runtime.
    pub handle: RuntimeHandle,
    /// Receive delivered messages (decrypted, verified).
    pub messages: mpsc::Receiver<DeliveredMessage>,
    /// Receive status changes for sent messages.
    pub status_changes: mpsc::Receiver<StatusChange>,
    /// Receive protocol-level events.
    pub events: mpsc::Receiver<ProtocolEvent>,
}

// ── ProtocolRuntime ──────────────────────────────────────────────────

/// The protocol runtime — spawn it and communicate via channels.
pub struct ProtocolRuntime;

impl ProtocolRuntime {
    /// Create and start the protocol runtime.
    ///
    /// Takes ownership of the `TomNode`. Returns channels for the application.
    /// Spawns the event loop as a tokio task.
    pub fn spawn(node: TomNode, config: RuntimeConfig) -> RuntimeChannels {
        let local_id = node.id();
        let secret_seed = node.secret_key_seed();

        // Shared metrics (Arc-backed, safe to clone)
        let metrics = ProtocolMetrics::new();

        // Command channel (app -> runtime)
        let (cmd_tx, cmd_rx) = mpsc::channel::<RuntimeCommand>(512);

        // Event channels (runtime -> app)
        // Very large buffers to prevent blocking even during burst phases
        let (msg_tx, msg_rx) = mpsc::channel::<DeliveredMessage>(16384);
        let (status_tx, status_rx) = mpsc::channel::<StatusChange>(4096);
        let (event_tx, event_rx) = mpsc::channel::<ProtocolEvent>(4096);

        // Subscribe to path events before moving node
        let path_rx = node.path_events();

        // Clone gossip handle before moving node
        let gossip = node.gossip().clone();
        let mut gossip_bootstrap_peers = config.gossip_bootstrap_peers.clone();

        // Create pure protocol state
        let state = RuntimeState::new(local_id, secret_seed, config);

        // Auto-reconnect: inject persisted peers as bootstrap targets so the node
        // redials them on restart without relay assistance.
        //
        // BORNÉ aux pairs vus récemment + cap dur. La topologie persistée
        // accumule les identités mortes (nœuds de test, fantômes du
        // rendezvous) : mesuré « auto-reconnect: 951 peers queued » sur le
        // Mac — le gossip les dialait TOUS en série (~300 ms d'échec chacun),
        // noyant les vrais pairs pendant ~4 min à chaque démarrage. Mêmes
        // bornes que le rejoin NeighborDown (REJOIN_RECENT_WINDOW_MS/16).
        const BOOTSTRAP_RECENT_WINDOW_MS: u64 = 5 * 60 * 1_000;
        const BOOTSTRAP_MAX_PEERS: usize = 16;
        let now = crate::types::now_ms();
        let mut recent_peers: Vec<_> = state
            .topology
            .peers()
            // `last_seen <= now` : défense en profondeur contre un timestamp
            // futur (déjà clampé à la source dans handle_role_announce, mais on
            // ne fait pas confiance à l'invariant ici). Sans ça, un last_seen
            // futur passait saturating_sub=0 ET remontait en tête du tri desc,
            // évinçant les vrais pairs (red-team #CRITIQUE, déni de reconnexion).
            .filter(|p| p.last_seen <= now && now - p.last_seen < BOOTSTRAP_RECENT_WINDOW_MS)
            .collect();
        recent_peers.sort_by_key(|p| std::cmp::Reverse(p.last_seen));
        for peer in recent_peers.into_iter().take(BOOTSTRAP_MAX_PEERS) {
            if !gossip_bootstrap_peers.contains(&peer.node_id) {
                gossip_bootstrap_peers.push(peer.node_id);
            }
        }
        if !gossip_bootstrap_peers.is_empty() {
            tracing::info!(
                count = gossip_bootstrap_peers.len(),
                "auto-reconnect: {} peers queued for gossip bootstrap",
                gossip_bootstrap_peers.len()
            );
        }

        // Spawn the event loop (thin orchestrator + executor)
        let loop_metrics = metrics.clone();
        let loop_cmd_tx = cmd_tx.clone();
        tokio::spawn(r#loop::runtime_loop(
            node,
            state,
            gossip_bootstrap_peers,
            loop_cmd_tx,
            cmd_rx,
            msg_tx,
            status_tx,
            event_tx,
            path_rx,
            gossip,
            loop_metrics,
        ));

        RuntimeChannels {
            handle: RuntimeHandle { cmd_tx, local_id, metrics },
            messages: msg_rx,
            status_changes: status_rx,
            events: event_rx,
        }
    }
}
