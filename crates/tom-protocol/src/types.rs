use serde::{Deserialize, Serialize};

pub use tom_transport::NodeId;

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
}

/// Delivery status pipeline for a message.
///
/// Follows the progression: Pending -> Sent -> Relayed -> Delivered -> Read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending = 0,
    Sent = 1,
    Relayed = 2,
    Delivered = 3,
    Read = 4,
}

/// Maximum relay depth (hops) for a message.
pub const MAX_TTL: u32 = 4;

/// Default TTL for new envelopes.
pub const DEFAULT_TTL: u32 = 4;

#[cfg(test)]
mod tests {
    use super::*;

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
            MessageType::BackupStore,
            MessageType::BackupDeliver,
            MessageType::BackupReplicate,
            MessageType::BackupReplicateAck,
            MessageType::BackupQuery,
            MessageType::BackupQueryResponse,
            MessageType::BackupConfirmDelivery,
            MessageType::PeerAnnounce,
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
