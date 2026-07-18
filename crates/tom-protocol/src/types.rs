use serde::{Deserialize, Serialize};

pub use tom_transport::{now_ms, NodeId};

/// Message type — determines how the protocol handles the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    Chat,
    Ack,
    ReadReceipt,
    Heartbeat,
    // Group lifecycle
    GroupCreate,
    GroupCreated,
    GroupInvite,
    GroupJoin,
    GroupSync,
    GroupMessage,
    GroupLeave,
    // Group broadcasts (hub → members)
    GroupMemberJoined,
    GroupMemberLeft,
    GroupHubMigration,
    GroupDeliveryAck,
    GroupHubHeartbeat,
    GroupSenderKeyDistribution,
    GroupHubPing,
    GroupHubPong,
    GroupHubShadowSync,
    GroupCandidateAssigned,
    GroupShadowAssigned,
    GroupHubUnreachable,
    // Group admin controls (R11.3)
    GroupKickMember,
    GroupUpdateMemberRole,
    GroupMemberRoleChanged,
    GroupInviteMember,
    // Offline delivery gap-fill (R13)
    GroupSyncRequest,
    GroupSyncResponse,
    // Backup
    BackupStore,
    BackupDeliver,
    BackupReplicate,
    BackupReplicateAck,
    BackupQuery,
    BackupQueryResponse,
    BackupConfirmDelivery,
    // Network
    PeerAnnounce,
    // Proof of Presence (L1-001) — appended LAST: wire compat requires
    // never inserting variants mid-enum (MessagePack index-based encoding).
    PresenceChallenge,
    PresenceAttestation,
    // Signed relay presence view (L1-003) — appended LAST (wire compat):
    // MessagePack encodes enum variants by index, so new variants must never
    // be inserted mid-enum, only appended.
    PresenceSubscribe,
    RelayPresenceView,
}

/// Delivery status pipeline for a message.
///
/// Follows the progression: Pending -> Sent -> Relayed -> Delivered -> Read.
/// `Failed` is a terminal state set explicitly after ACK timeout + retries exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending = 0,
    Sent = 1,
    Relayed = 2,
    Delivered = 3,
    Read = 4,
    Failed = 5,
}

/// Maximum relay depth (hops) for a message.
pub const MAX_TTL: u32 = 4;

/// Default TTL for new envelopes.
pub const DEFAULT_TTL: u32 = 4;

/// Préfixe de username des nœuds de test (transparence, décision 2026-07-18).
///
/// Tout nœud spawné par un harnais (tom-stress : invariants, chaos, storm…)
/// DOIT porter ce préfixe. Les vrais nœuds les affichent comme « nœud de test
/// éphémère » et ne les choisissent JAMAIS comme cible automatique (ping,
/// auto-connect). On ne cherche plus l'herméticité parfaite : on marque, on
/// liste, on avertit. Registre : docs/plans/banc-test-chaos.md.
pub const TEST_NODE_PREFIX: &str = "TEST-";

/// Vrai si ce username désigne un nœud de test éphémère (préfixe `TEST-`).
pub fn is_test_node_username(username: &str) -> bool {
    username.starts_with(TEST_NODE_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_node_username_detection() {
        assert!(is_test_node_username("TEST-invariants-0"));
        assert!(is_test_node_username("TEST-chaos-alice"));
        assert!(!is_test_node_username("iPhone"));
        assert!(!is_test_node_username("test-minuscule")); // préfixe strict, casse comprise
        assert!(!is_test_node_username(""));
    }

    #[test]
    fn test_message_type_roundtrip_msgpack() {
        let types = [
            MessageType::Chat,
            MessageType::Ack,
            MessageType::ReadReceipt,
            MessageType::Heartbeat,
            MessageType::GroupCreate,
            MessageType::GroupCreated,
            MessageType::GroupInvite,
            MessageType::GroupJoin,
            MessageType::GroupSync,
            MessageType::GroupMessage,
            MessageType::GroupLeave,
            MessageType::GroupMemberJoined,
            MessageType::GroupMemberLeft,
            MessageType::GroupHubMigration,
            MessageType::GroupDeliveryAck,
            MessageType::GroupHubHeartbeat,
            MessageType::GroupSenderKeyDistribution,
            MessageType::GroupHubPing,
            MessageType::GroupHubPong,
            MessageType::GroupHubShadowSync,
            MessageType::GroupCandidateAssigned,
            MessageType::GroupShadowAssigned,
            MessageType::GroupHubUnreachable,
            MessageType::GroupKickMember,
            MessageType::GroupUpdateMemberRole,
            MessageType::GroupMemberRoleChanged,
            MessageType::GroupInviteMember,
            MessageType::GroupSyncRequest,
            MessageType::GroupSyncResponse,
            MessageType::BackupStore,
            MessageType::BackupDeliver,
            MessageType::BackupReplicate,
            MessageType::BackupReplicateAck,
            MessageType::BackupQuery,
            MessageType::BackupQueryResponse,
            MessageType::BackupConfirmDelivery,
            MessageType::PeerAnnounce,
            MessageType::PresenceChallenge,
            MessageType::PresenceAttestation,
        ];

        for msg_type in &types {
            let bytes = rmp_serde::to_vec(msg_type).expect("serialize");
            let decoded: MessageType = rmp_serde::from_slice(&bytes).expect("deserialize");
            assert_eq!(*msg_type, decoded, "roundtrip failed for {:?}", msg_type);
        }
    }

    #[test]
    fn test_message_status_ordering() {
        assert!(MessageStatus::Pending < MessageStatus::Sent);
        assert!(MessageStatus::Sent < MessageStatus::Relayed);
        assert!(MessageStatus::Relayed < MessageStatus::Delivered);
        assert!(MessageStatus::Delivered < MessageStatus::Read);
    }

    #[test]
    fn test_message_status_roundtrip_msgpack() {
        let statuses = [
            MessageStatus::Pending,
            MessageStatus::Sent,
            MessageStatus::Relayed,
            MessageStatus::Delivered,
            MessageStatus::Read,
        ];

        for status in &statuses {
            let bytes = rmp_serde::to_vec(status).expect("serialize");
            let decoded: MessageStatus = rmp_serde::from_slice(&bytes).expect("deserialize");
            assert_eq!(*status, decoded);
        }
    }
}
