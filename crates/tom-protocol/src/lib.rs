//! ToM Protocol layer.
//!
//! Implements routing, encryption, discovery, and group messaging
//! on top of `tom-transport` (QUIC via iroh).
//!
//! Wire format: MessagePack (compact binary).
//! Crypto: Ed25519 signatures + XChaCha20-Poly1305 encryption.

pub mod backup;
pub mod crypto;
pub mod discovery;
pub mod envelope;
pub mod error;
pub mod group;
pub mod relay;
pub mod roles;
pub mod router;
pub mod runtime;
pub mod tracker;
pub mod types;

pub use backup::{
    BackupAction, BackupCoordinator, BackupEntry, BackupEvent, BackupStore, HostFactors,
    ReplicationPayload,
};
pub use crypto::EncryptedPayload;
pub use discovery::{
    DiscoveryEvent, DiscoverySource, DissolveReason, EphemeralSubnetManager, HeartbeatTracker,
    LivenessState, PeerAnnounce, SubnetEvent, SubnetInfo,
};
pub use envelope::{Envelope, EnvelopeBuilder};
pub use error::TomProtocolError;
pub use group::{
    elect_hub, ElectionReason, ElectionResult, GroupAction, GroupEvent, GroupHub, GroupId,
    GroupInfo, GroupInvite, GroupMember, GroupManager, GroupMemberRole, GroupMessage, GroupPayload,
    LeaveReason,
};
pub use relay::{PeerInfo, PeerRole, PeerStatus, RelaySelector, Topology};
pub use roles::{ContributionMetrics, RoleAction, RoleManager};
pub use router::{AckPayload, AckType, ReadReceiptPayload, Router, RoutingAction};
pub use tracker::{MessageTracker, StatusChange};
pub use runtime::{
    DeliveredMessage, ProtocolEvent, ProtocolRuntime, RuntimeChannels, RuntimeConfig,
    RuntimeHandle,
};
pub use types::{now_ms, MessageStatus, MessageType, NodeId};
