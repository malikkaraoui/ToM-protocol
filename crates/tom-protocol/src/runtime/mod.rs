/// Protocol runtime — integrates all protocol modules into a live event loop.
///
/// The runtime owns a `TomNode` (transport) and all protocol state (router,
/// topology, tracker, heartbeat). It exposes a channel-based API so the
/// application (TUI, bot, SDK) never touches raw bytes or protocol internals.
mod r#loop;

use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tom_transport::{PathEvent, TomNode};

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
    /// Interval for group hub heartbeats.
    pub group_hub_heartbeat_interval: Duration,
    /// Interval for backup maintenance ticks.
    pub backup_tick_interval: Duration,
    /// Interval for gossip peer announcements.
    pub gossip_announce_interval: Duration,
    /// Bootstrap peers to join the gossip discovery network.
    pub gossip_bootstrap_peers: Vec<crate::types::NodeId>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            encryption: true,
            cache_cleanup_interval: Duration::from_secs(300),
            heartbeat_interval: Duration::from_secs(5),
            tracker_cleanup_interval: Duration::from_secs(300),
            username: "anonymous".to_string(),
            group_hub_heartbeat_interval: Duration::from_secs(30),
            backup_tick_interval: Duration::from_secs(60),
            gossip_announce_interval: Duration::from_secs(10),
            gossip_bootstrap_peers: Vec::new(),
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
    AddPeer { node_id: NodeId },
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
    /// Query: list pending invitations.
    GetPendingInvites {
        reply: oneshot::Sender<Vec<GroupInvite>>,
    },
    /// Graceful shutdown.
    Shutdown,
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
    /// A peer was discovered via heartbeat.
    PeerDiscovered { node_id: NodeId },
    /// A peer went offline.
    PeerOffline { node_id: NodeId },
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
    // ── Discovery events ──────────────────────────
    /// A peer announced itself via gossip.
    PeerAnnounceReceived {
        node_id: NodeId,
        username: String,
    },
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
}

// ── RuntimeHandle (app-facing API) ───────────────────────────────────

/// Handle to communicate with a running ProtocolRuntime.
///
/// Cheap to clone. All methods are non-blocking channel sends.
#[derive(Clone)]
pub struct RuntimeHandle {
    cmd_tx: mpsc::Sender<RuntimeCommand>,
    local_id: NodeId,
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

    /// Register a peer in the network (triggers iroh discovery).
    pub async fn add_peer(&self, node_id: NodeId) {
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::AddPeer { node_id })
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

    /// Get pending group invitations.
    pub async fn pending_invites(&self) -> Vec<GroupInvite> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(RuntimeCommand::GetPendingInvites { reply: tx })
            .await;
        rx.await.unwrap_or_default()
    }

    /// Graceful shutdown.
    pub async fn shutdown(&self) {
        let _ = self.cmd_tx.send(RuntimeCommand::Shutdown).await;
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

        // Command channel (app → runtime)
        let (cmd_tx, cmd_rx) = mpsc::channel::<RuntimeCommand>(64);

        // Event channels (runtime → app)
        let (msg_tx, msg_rx) = mpsc::channel::<DeliveredMessage>(64);
        let (status_tx, status_rx) = mpsc::channel::<StatusChange>(64);
        let (event_tx, event_rx) = mpsc::channel::<ProtocolEvent>(64);

        // Subscribe to path events before moving node
        let path_rx = node.path_events();

        // Clone gossip handle before moving node
        let gossip = node.gossip().clone();
        let gossip_bootstrap_peers = config.gossip_bootstrap_peers.clone();

        // Spawn the event loop
        tokio::spawn(r#loop::runtime_loop(
            node,
            local_id,
            secret_seed,
            config,
            cmd_rx,
            msg_tx,
            status_tx,
            event_tx,
            path_rx,
            gossip,
            gossip_bootstrap_peers,
        ));

        RuntimeChannels {
            handle: RuntimeHandle { cmd_tx, local_id },
            messages: msg_rx,
            status_changes: status_rx,
            events: event_rx,
        }
    }
}
