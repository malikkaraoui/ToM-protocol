//! Unified event stream delivered to the application.
//!
//! The runtime exposes three channels (messages, status changes, protocol
//! events); the SDK merges them into a single [`Event`] stream so the
//! application has one loop to write. Mapping deliberately hides the types
//! of internal/forked crates: relay URLs become `String`, path events become
//! plain fields.

use tom_protocol::{
    DeliveredMessage, GroupId, GroupInfo, GroupInvite, GroupMember, GroupMessage, LeaveReason,
    MessageStatus, NodeId, ProtocolEvent,
};
use tom_protocol::tracker::StatusChange;
use tom_transport::PathKind;

/// A message received from the network, decrypted and signature-checked.
#[derive(Debug, Clone)]
pub struct Message {
    /// Sender identity.
    pub from: NodeId,
    /// Decrypted payload bytes.
    pub payload: Vec<u8>,
    /// Unique envelope id (use it for read receipts).
    pub id: String,
    /// Sender timestamp (unix epoch, milliseconds).
    pub timestamp: u64,
    /// Whether the Ed25519 signature verified.
    pub signature_valid: bool,
    /// Whether the payload was end-to-end encrypted in transit.
    pub was_encrypted: bool,
}

impl Message {
    /// Payload interpreted as UTF-8 text (lossy).
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.payload).into_owned()
    }
}

/// Unified SDK event.
///
/// Marked `#[non_exhaustive]`: new variants may be added in minor releases,
/// always keep a wildcard arm when matching.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// A direct message arrived (decrypted, verified).
    MessageReceived(Message),
    /// A message we sent changed status (sent → relayed → delivered → read).
    MessageStatusChanged {
        /// Envelope id of the tracked message.
        message_id: String,
        /// Previous status.
        previous: MessageStatus,
        /// New status.
        current: MessageStatus,
    },
    /// A new peer was discovered.
    PeerDiscovered {
        /// Peer identity.
        node_id: NodeId,
        /// Username announced by the peer.
        username: String,
    },
    /// A peer came (back) online.
    PeerOnline {
        /// Peer identity.
        node_id: NodeId,
    },
    /// A peer went offline.
    PeerOffline {
        /// Peer identity.
        node_id: NodeId,
    },
    /// The network path to a peer changed (relay ↔ direct).
    PathChanged {
        /// The remote peer.
        peer: NodeId,
        /// True when the connection is direct (hole-punched), false via relay.
        direct: bool,
        /// Round-trip time on the new path, in milliseconds.
        rtt_ms: u64,
    },
    /// A group was created and we are a member.
    GroupCreated {
        /// Full group description.
        group: GroupInfo,
    },
    /// We received an invitation to join a group.
    GroupInviteReceived {
        /// The pending invitation.
        invite: GroupInvite,
    },
    /// We joined a group (after accepting an invite).
    GroupJoined {
        /// Group identifier.
        group_id: GroupId,
        /// Human-readable group name.
        group_name: String,
    },
    /// A member joined a group we belong to.
    GroupMemberJoined {
        /// Group identifier.
        group_id: GroupId,
        /// The new member.
        member: GroupMember,
    },
    /// A member left a group we belong to.
    GroupMemberLeft {
        /// Group identifier.
        group_id: GroupId,
        /// The departed member.
        node_id: NodeId,
        /// Their username.
        username: String,
        /// Why they left.
        reason: LeaveReason,
    },
    /// A group message arrived.
    GroupMessageReceived {
        /// The message (sender, text, group id).
        message: GroupMessage,
    },
    /// The hub hosting a group migrated to another node (failover).
    GroupHubMigrated {
        /// Group identifier.
        group_id: GroupId,
        /// New hub identity.
        new_hub_id: NodeId,
    },
    /// A peer announced a relay it operates.
    RelayDiscovered {
        /// Operator identity.
        node_id: NodeId,
        /// Relay URL (string form).
        url: String,
    },
    /// An outbound message is being retried after an ACK timeout.
    DeliveryRetry {
        /// Envelope id.
        message_id: String,
        /// Recipient.
        to: NodeId,
        /// Retry attempt number.
        attempt: u8,
    },
    /// An outbound message failed after all retries.
    DeliveryTimeout {
        /// Envelope id.
        message_id: String,
        /// Recipient.
        to: NodeId,
        /// Last known status before giving up.
        last_status: MessageStatus,
    },
    /// A non-fatal runtime error worth surfacing.
    Error {
        /// Human-readable description.
        description: String,
    },
}

pub(crate) fn map_message(m: DeliveredMessage) -> Event {
    Event::MessageReceived(Message {
        from: m.from,
        payload: m.payload,
        id: m.envelope_id,
        timestamp: m.timestamp,
        signature_valid: m.signature_valid,
        was_encrypted: m.was_encrypted,
    })
}

pub(crate) fn map_status(s: StatusChange) -> Event {
    Event::MessageStatusChanged {
        message_id: s.message_id,
        previous: s.previous,
        current: s.current,
    }
}

/// Maps a protocol event to an SDK event.
///
/// Returns `None` for internal events not exposed by the façade
/// (forwarding, backups, roles, subnets, gossip neighbors, anti-spam,
/// embedded relay lifecycle). Applications that need those should depend
/// on `tom-protocol` directly.
pub(crate) fn map_protocol_event(e: ProtocolEvent) -> Option<Event> {
    match e {
        ProtocolEvent::PeerDiscovered { node_id, username, .. } => {
            Some(Event::PeerDiscovered { node_id, username })
        }
        ProtocolEvent::PeerOnline { node_id } => Some(Event::PeerOnline { node_id }),
        ProtocolEvent::PeerOffline { node_id } => Some(Event::PeerOffline { node_id }),
        ProtocolEvent::PathChanged { event } => Some(Event::PathChanged {
            peer: event.remote,
            direct: matches!(event.kind, PathKind::Direct),
            rtt_ms: event.rtt.as_millis() as u64,
        }),
        ProtocolEvent::GroupCreated { group } => Some(Event::GroupCreated { group }),
        ProtocolEvent::GroupInviteReceived { invite } => {
            Some(Event::GroupInviteReceived { invite })
        }
        ProtocolEvent::GroupJoined { group_id, group_name } => {
            Some(Event::GroupJoined { group_id, group_name })
        }
        ProtocolEvent::GroupMemberJoined { group_id, member } => {
            Some(Event::GroupMemberJoined { group_id, member })
        }
        ProtocolEvent::GroupMemberLeft { group_id, node_id, username, reason } => {
            Some(Event::GroupMemberLeft { group_id, node_id, username, reason })
        }
        ProtocolEvent::GroupMessageReceived { message } => {
            Some(Event::GroupMessageReceived { message })
        }
        ProtocolEvent::GroupHubMigrated { group_id, new_hub_id } => {
            Some(Event::GroupHubMigrated { group_id, new_hub_id })
        }
        ProtocolEvent::RelayReadyReceived { node_id, relay_url } => Some(Event::RelayDiscovered {
            node_id,
            url: relay_url.to_string(),
        }),
        ProtocolEvent::DeliveryRetry { message_id, to, attempt } => {
            Some(Event::DeliveryRetry { message_id, to, attempt })
        }
        ProtocolEvent::DeliveryTimeout { message_id, to, last_status } => {
            Some(Event::DeliveryTimeout { message_id, to, last_status })
        }
        ProtocolEvent::MessageRejected { reason } => Some(Event::Error {
            description: format!("message rejected: {reason}"),
        }),
        ProtocolEvent::Error { description } => Some(Event::Error { description }),
        _ => None,
    }
}
