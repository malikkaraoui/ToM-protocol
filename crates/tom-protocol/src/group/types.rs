/// Group data structures for ToM protocol.
///
/// Hub-and-spoke topology: one relay acts as hub for each group,
/// fanning out messages to all members.
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::types::{now_ms, NodeId};

// ── Constants ────────────────────────────────────────────────────────────

/// Maximum members per group.
pub const MAX_GROUP_MEMBERS: usize = 50;

/// Invite TTL (24 hours, matching ToM design decision #2).
pub const INVITE_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// Hub heartbeat interval (30 seconds).
pub const HUB_HEARTBEAT_INTERVAL_MS: u64 = 30_000;

/// Missed heartbeats before hub is considered failed.
pub const HUB_FAILURE_THRESHOLD: u32 = 3;

/// Max messages kept in hub history for sync to new members.
pub const MAX_SYNC_MESSAGES: usize = 100;

/// Rate limit: messages per second per sender in a group.
pub const GROUP_RATE_LIMIT_PER_SECOND: u32 = 5;

/// Shadow pings primary every 3s.
pub const SHADOW_PING_INTERVAL_MS: u64 = 3_000;

/// Timeout per shadow ping (2s).
pub const SHADOW_PING_TIMEOUT_MS: u64 = 2_000;

/// Shadow promotes after 2 consecutive missed pings.
pub const SHADOW_PING_FAILURE_THRESHOLD: u32 = 2;

/// Member timeout before sending HubUnreachable to shadow (3s).
pub const HUB_ACK_TIMEOUT_MS: u64 = 3_000;

/// Candidate self-promotes after this timeout with no contact from primary or shadow.
pub const CANDIDATE_ORPHAN_TIMEOUT_MS: u64 = 30_000;

// ── GroupId ──────────────────────────────────────────────────────────────

/// Unique group identifier (e.g., "grp-<uuid>").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId(pub String);

impl GroupId {
    /// Create a new random group ID.
    pub fn new() -> Self {
        Self(format!("grp-{}", uuid::Uuid::new_v4()))
    }
}

impl Default for GroupId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for GroupId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for GroupId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ── GroupMemberRole ──────────────────────────────────────────────────────

/// Role within a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GroupMemberRole {
    /// Group creator — can invite, kick, dissolve.
    Admin,
    /// Regular member — can send messages, leave.
    Member,
}

// ── GroupMember ──────────────────────────────────────────────────────────

/// A member in a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMember {
    pub node_id: NodeId,
    pub username: String,
    pub joined_at: u64,
    pub role: GroupMemberRole,
}

// ── GroupInfo ────────────────────────────────────────────────────────────

/// Full group state — shared between manager and hub.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupInfo {
    pub group_id: GroupId,
    pub name: String,
    pub hub_relay_id: NodeId,
    pub backup_hub_id: Option<NodeId>,
    pub members: Vec<GroupMember>,
    pub created_by: NodeId,
    pub created_at: u64,
    pub last_activity_at: u64,
    pub max_members: usize,
    /// Current shadow node (virus replication).
    #[serde(default)]
    pub shadow_id: Option<NodeId>,
    /// Current candidate node (next shadow).
    #[serde(default)]
    pub candidate_id: Option<NodeId>,
}

impl GroupInfo {
    /// Check if a node is a member of this group.
    pub fn is_member(&self, node_id: &NodeId) -> bool {
        self.members.iter().any(|m| m.node_id == *node_id)
    }

    /// Check if a node is an admin of this group.
    pub fn is_admin(&self, node_id: &NodeId) -> bool {
        self.members
            .iter()
            .any(|m| m.node_id == *node_id && m.role == GroupMemberRole::Admin)
    }

    /// Get a member by node ID.
    pub fn get_member(&self, node_id: &NodeId) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.node_id == *node_id)
    }

    /// Number of current members.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Whether the group is at capacity.
    pub fn is_full(&self) -> bool {
        self.members.len() >= self.max_members
    }
}

// ── GroupInvite ──────────────────────────────────────────────────────────

/// A pending invitation to join a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInvite {
    pub group_id: GroupId,
    pub group_name: String,
    pub inviter_id: NodeId,
    pub inviter_username: String,
    pub hub_relay_id: NodeId,
    pub invited_at: u64,
    pub expires_at: u64,
}

impl GroupInvite {
    /// Whether this invite has expired.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at
    }
}

// ── GroupPayload ─────────────────────────────────────────────────────────

/// Group-specific payload — serialized into `Envelope.payload`.
///
/// Each variant maps to a group protocol message type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GroupPayload {
    /// Admin creates a new group (member → hub).
    Create {
        group_name: String,
        creator_username: String,
        initial_members: Vec<NodeId>,
    },

    /// Hub confirms group creation (hub → creator).
    Created {
        group: GroupInfo,
    },

    /// Hub sends invitation to a potential member (hub → invitee).
    Invite {
        group_id: GroupId,
        group_name: String,
        inviter_id: NodeId,
        inviter_username: String,
    },

    /// Invitee accepts and requests to join (invitee → hub).
    Join {
        group_id: GroupId,
        username: String,
    },

    /// Hub sends full group state to new member (hub → new member).
    Sync {
        group: GroupInfo,
        recent_messages: Vec<GroupMessage>,
    },

    /// Chat message within the group (member → hub, hub → members).
    Message(GroupMessage),

    /// Member voluntarily leaves (member → hub).
    Leave {
        group_id: GroupId,
    },

    /// Hub broadcasts that a new member joined (hub → members).
    MemberJoined {
        group_id: GroupId,
        member: GroupMember,
    },

    /// Hub broadcasts that a member left or was kicked (hub → members).
    MemberLeft {
        group_id: GroupId,
        node_id: NodeId,
        username: String,
        reason: LeaveReason,
    },

    /// Member confirms receipt of a group message (member → hub).
    DeliveryAck {
        group_id: GroupId,
        message_id: String,
    },

    /// Hub announces migration to a new hub (hub → members).
    HubMigration {
        group_id: GroupId,
        new_hub_id: NodeId,
        old_hub_id: NodeId,
    },

    /// Hub health check (hub → members).
    HubHeartbeat {
        group_id: GroupId,
        member_count: usize,
    },

    /// Distribution of a member's Sender Key to other members.
    SenderKeyDistribution {
        group_id: GroupId,
        from: NodeId,
        epoch: u32,
        encrypted_keys: Vec<EncryptedSenderKey>,
    },

    /// Shadow watchdog ping (shadow -> primary).
    HubPing { group_id: GroupId },

    /// Primary response to shadow ping (primary -> shadow).
    HubPong { group_id: GroupId },

    /// State sync from primary to shadow (primary -> shadow).
    HubShadowSync {
        group_id: GroupId,
        members: Vec<GroupMember>,
        candidate_id: Option<NodeId>,
        config_version: u64,
    },

    /// Candidate role assignment (shadow -> candidate).
    CandidateAssigned { group_id: GroupId },

    /// Member reports hub unreachable (member -> shadow).
    HubUnreachable { group_id: GroupId },
}

// ── GroupMessage ──────────────────────────────────────────────────────────

/// A single message in a group conversation.
///
/// Supports both plaintext (backward-compatible) and encrypted (Sender Key) modes.
/// When `encrypted` is true, content is in `ciphertext`/`nonce`; when false, in `text`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupMessage {
    pub group_id: GroupId,
    pub message_id: String,
    pub sender_id: NodeId,
    #[serde(default)]
    pub sender_username: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub ciphertext: Vec<u8>,
    #[serde(default)]
    pub nonce: [u8; 24],
    #[serde(default)]
    pub key_epoch: u32,
    #[serde(default)]
    pub encrypted: bool,
    pub sent_at: u64,
    #[serde(default)]
    pub sender_signature: Vec<u8>,
}

impl GroupMessage {
    /// Create a new plaintext group message (unsigned).
    pub fn new(
        group_id: GroupId,
        sender_id: NodeId,
        sender_username: String,
        text: String,
    ) -> Self {
        Self {
            group_id,
            message_id: uuid::Uuid::new_v4().to_string(),
            sender_id,
            sender_username,
            text,
            ciphertext: Vec::new(),
            nonce: [0u8; 24],
            key_epoch: 0,
            encrypted: false,
            sent_at: now_ms(),
            sender_signature: Vec::new(),
        }
    }

    /// Create a new encrypted group message.
    pub fn new_encrypted(
        group_id: GroupId,
        sender_id: NodeId,
        username: String,
        text: String,
        sender_key: &[u8; 32],
        key_epoch: u32,
    ) -> Self {
        let content = GroupMessageContent { username, text };
        let content_bytes = rmp_serde::to_vec(&content).expect("content serialization");
        let (ciphertext, nonce) = crate::crypto::encrypt_group_message(&content_bytes, sender_key);
        Self {
            group_id,
            message_id: uuid::Uuid::new_v4().to_string(),
            sender_id,
            sender_username: String::new(),
            text: String::new(),
            ciphertext,
            nonce,
            key_epoch,
            encrypted: true,
            sent_at: now_ms(),
            sender_signature: Vec::new(),
        }
    }

    /// Decrypt this message's ciphertext using the sender's key.
    ///
    /// For plaintext messages, returns the content directly without decryption.
    pub fn decrypt(
        &self,
        sender_key: &[u8; 32],
    ) -> Result<GroupMessageContent, crate::TomProtocolError> {
        if !self.encrypted {
            return Ok(GroupMessageContent {
                username: self.sender_username.clone(),
                text: self.text.clone(),
            });
        }
        let plaintext_bytes =
            crate::crypto::decrypt_group_message(&self.ciphertext, &self.nonce, sender_key)?;
        rmp_serde::from_slice(&plaintext_bytes).map_err(|e| {
            crate::TomProtocolError::Deserialization(format!("group message content: {e}"))
        })
    }

    /// Deterministic bytes for signing — signs ciphertext when encrypted, text when plaintext.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.group_id.0.as_bytes());
        buf.extend_from_slice(self.message_id.as_bytes());
        buf.extend_from_slice(&self.sender_id.as_bytes());
        if self.encrypted {
            buf.extend_from_slice(&self.ciphertext);
            buf.extend_from_slice(&self.nonce);
            buf.extend_from_slice(&self.key_epoch.to_le_bytes());
        } else {
            buf.extend_from_slice(self.text.as_bytes());
        }
        buf.extend_from_slice(&self.sent_at.to_le_bytes());
        buf
    }

    /// Sign this message with the sender's Ed25519 secret key seed.
    pub fn sign(&mut self, secret_seed: &[u8; 32]) {
        use ed25519_dalek::{Signer, SigningKey};
        let signing_key = SigningKey::from_bytes(secret_seed);
        let signature = signing_key.sign(&self.signing_bytes());
        self.sender_signature = signature.to_bytes().to_vec();
    }

    /// Verify the sender's signature against `sender_id` public key.
    ///
    /// Returns `true` if the signature is valid, `false` if missing or invalid.
    pub fn verify_signature(&self) -> bool {
        if self.sender_signature.len() != 64 {
            return false;
        }
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        let pk_bytes: [u8; 32] = self.sender_id.as_bytes();
        let Ok(verifying_key) = VerifyingKey::from_bytes(&pk_bytes) else {
            return false;
        };
        let Ok(sig_array): Result<&[u8; 64], _> = self.sender_signature.as_slice().try_into()
        else {
            return false;
        };
        let sig = Signature::from_bytes(sig_array);
        verifying_key.verify(&self.signing_bytes(), &sig).is_ok()
    }

    /// Whether this message has been signed.
    pub fn is_signed(&self) -> bool {
        self.sender_signature.len() == 64
    }
}

// ── LeaveReason ──────────────────────────────────────────────────────────

/// Why a member left the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaveReason {
    /// Member left voluntarily.
    Voluntary,
    /// Member was kicked by an admin.
    Kicked,
    /// Member went offline and was cleaned up.
    Timeout,
}

// ── Sender Key Encryption ─────────────────────────────────────────────

/// A member's Sender Key — used to encrypt their outgoing group messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SenderKeyEntry {
    pub owner_id: NodeId,
    pub key: [u8; 32],
    pub epoch: u32,
    pub created_at: u64,
}

/// A Sender Key encrypted for a specific recipient (1-to-1 encryption).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncryptedSenderKey {
    pub recipient_id: NodeId,
    pub encrypted_key: crate::crypto::EncryptedPayload,
}

/// Plaintext content inside an encrypted group message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupMessageContent {
    pub username: String,
    pub text: String,
}

// ── GroupAction ──────────────────────────────────────────────────────────

/// Actions returned by GroupManager — the caller executes them via transport.
///
/// Pure decision engine pattern (same as Router → RoutingAction).
#[derive(Debug)]
pub enum GroupAction {
    /// Send a payload to a specific node.
    Send {
        to: NodeId,
        payload: GroupPayload,
    },

    /// Broadcast a payload to multiple nodes.
    Broadcast {
        to: Vec<NodeId>,
        payload: GroupPayload,
    },

    /// A group event occurred (for application-layer callbacks).
    Event(GroupEvent),

    /// No action needed.
    None,
}

// ── GroupEvent ────────────────────────────────────────────────────────────

/// Application-visible group events.
#[derive(Debug, Clone)]
pub enum GroupEvent {
    /// A group was created (we're the admin).
    GroupCreated(GroupInfo),

    /// We received an invitation.
    InviteReceived(GroupInvite),

    /// We successfully joined a group.
    Joined {
        group_id: GroupId,
        group_name: String,
    },

    /// A new member joined one of our groups.
    MemberJoined {
        group_id: GroupId,
        member: GroupMember,
    },

    /// A member left one of our groups.
    MemberLeft {
        group_id: GroupId,
        node_id: NodeId,
        username: String,
        reason: LeaveReason,
    },

    /// We received a group message.
    MessageReceived(GroupMessage),

    /// Hub migrated to a new node.
    HubMigrated {
        group_id: GroupId,
        new_hub_id: NodeId,
    },

    /// Security violation detected (non-member or invalid signature).
    SecurityViolation {
        group_id: GroupId,
        node_id: NodeId,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a deterministic NodeId from a seed byte.
    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn group_id_format() {
        let id = GroupId::new();
        assert!(id.0.starts_with("grp-"));
        assert!(id.0.len() > 10);
    }

    #[test]
    fn group_id_display() {
        let id = GroupId::from("grp-test-123".to_string());
        assert_eq!(format!("{}", id), "grp-test-123");
    }

    #[test]
    fn group_id_roundtrip() {
        let id = GroupId::new();
        let bytes = rmp_serde::to_vec(&id).expect("serialize");
        let decoded: GroupId = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(id, decoded);
    }

    fn make_member(seed: u8, role: GroupMemberRole) -> GroupMember {
        GroupMember {
            node_id: node_id(seed),
            username: format!("user-{}", seed),
            joined_at: 1000,
            role,
        }
    }

    fn make_group() -> GroupInfo {
        let admin = make_member(1, GroupMemberRole::Admin);
        let member = make_member(2, GroupMemberRole::Member);
        GroupInfo {
            group_id: GroupId::from("grp-test".to_string()),
            name: "Test Group".into(),
            hub_relay_id: node_id(10),
            backup_hub_id: None,
            members: vec![admin.clone(), member],
            created_by: admin.node_id,
            created_at: 1000,
            last_activity_at: 1000,
            max_members: MAX_GROUP_MEMBERS,
            shadow_id: None,
            candidate_id: None,
        }
    }

    #[test]
    fn group_info_membership() {
        let group = make_group();
        let admin_id = node_id(1);
        let member_id = node_id(2);
        let stranger_id = node_id(99);

        assert!(group.is_member(&admin_id));
        assert!(group.is_member(&member_id));
        assert!(!group.is_member(&stranger_id));

        assert!(group.is_admin(&admin_id));
        assert!(!group.is_admin(&member_id));
        assert!(!group.is_admin(&stranger_id));
    }

    #[test]
    fn group_info_capacity() {
        let mut group = make_group();
        assert_eq!(group.member_count(), 2);
        assert!(!group.is_full());

        group.max_members = 2;
        assert!(group.is_full());
    }

    #[test]
    fn group_invite_expiry() {
        let invite = GroupInvite {
            group_id: GroupId::from("grp-1".to_string()),
            group_name: "Test".into(),
            inviter_id: node_id(1),
            inviter_username: "alice".into(),
            hub_relay_id: node_id(10),
            invited_at: 1000,
            expires_at: 1000 + INVITE_TTL_MS,
        };

        assert!(!invite.is_expired(1000));
        assert!(!invite.is_expired(1000 + INVITE_TTL_MS - 1));
        assert!(invite.is_expired(1000 + INVITE_TTL_MS));
        assert!(invite.is_expired(1000 + INVITE_TTL_MS + 1));
    }

    #[test]
    fn group_message_new() {
        let msg = GroupMessage::new(
            GroupId::from("grp-1".to_string()),
            node_id(1),
            "alice".into(),
            "Hello group!".into(),
        );

        assert_eq!(msg.group_id, GroupId::from("grp-1".to_string()));
        assert_eq!(msg.text, "Hello group!");
        assert!(!msg.message_id.is_empty());
        assert!(msg.sent_at > 0);
    }

    #[test]
    fn group_payload_roundtrip() {
        let payloads = vec![
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![node_id(2)],
            },
            GroupPayload::Join {
                group_id: GroupId::from("grp-1".to_string()),
                username: "bob".into(),
            },
            GroupPayload::Leave {
                group_id: GroupId::from("grp-1".to_string()),
            },
            GroupPayload::DeliveryAck {
                group_id: GroupId::from("grp-1".to_string()),
                message_id: "msg-1".into(),
            },
            GroupPayload::HubHeartbeat {
                group_id: GroupId::from("grp-1".to_string()),
                member_count: 5,
            },
        ];

        for payload in &payloads {
            let bytes = rmp_serde::to_vec(payload).expect("serialize");
            let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
            assert_eq!(*payload, decoded, "roundtrip failed for {:?}", payload);
        }
    }

    #[test]
    fn group_member_role_roundtrip() {
        for role in [GroupMemberRole::Admin, GroupMemberRole::Member] {
            let bytes = rmp_serde::to_vec(&role).expect("serialize");
            let decoded: GroupMemberRole = rmp_serde::from_slice(&bytes).expect("deserialize");
            assert_eq!(role, decoded);
        }
    }

    #[test]
    fn leave_reason_roundtrip() {
        for reason in [LeaveReason::Voluntary, LeaveReason::Kicked, LeaveReason::Timeout] {
            let bytes = rmp_serde::to_vec(&reason).expect("serialize");
            let decoded: LeaveReason = rmp_serde::from_slice(&bytes).expect("deserialize");
            assert_eq!(reason, decoded);
        }
    }

    #[test]
    fn group_info_roundtrip() {
        let group = make_group();
        let bytes = rmp_serde::to_vec(&group).expect("serialize");
        let decoded: GroupInfo = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(decoded.group_id, group.group_id);
        assert_eq!(decoded.name, group.name);
        assert_eq!(decoded.members.len(), group.members.len());
    }

    fn secret_seed(seed: u8) -> [u8; 32] {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.to_bytes()
    }

    #[test]
    fn group_message_sign_verify() {
        let seed = secret_seed(1);
        let sender = node_id(1);
        let mut msg = GroupMessage::new(
            GroupId::from("grp-1".to_string()),
            sender,
            "alice".into(),
            "Hello signed!".into(),
        );

        assert!(!msg.is_signed());
        assert!(!msg.verify_signature());

        msg.sign(&seed);
        assert!(msg.is_signed());
        assert!(msg.verify_signature(), "signature should be valid");
    }

    #[test]
    fn group_message_tampered_text_fails() {
        let seed = secret_seed(1);
        let sender = node_id(1);
        let mut msg = GroupMessage::new(
            GroupId::from("grp-1".to_string()),
            sender,
            "alice".into(),
            "Original text".into(),
        );
        msg.sign(&seed);
        assert!(msg.verify_signature());

        msg.text = "Tampered text".into();
        assert!(!msg.verify_signature(), "tampered text should fail verification");
    }

    #[test]
    fn group_message_wrong_key_fails() {
        let seed1 = secret_seed(1);
        let sender1 = node_id(1);
        let sender2 = node_id(2);
        let mut msg = GroupMessage::new(
            GroupId::from("grp-1".to_string()),
            sender1,
            "alice".into(),
            "Hello".into(),
        );
        msg.sign(&seed1);
        assert!(msg.verify_signature());

        // Swap sender_id to a different key
        msg.sender_id = sender2;
        assert!(!msg.verify_signature(), "wrong sender key should fail");
    }

    #[test]
    fn group_message_signed_roundtrip() {
        let seed = secret_seed(1);
        let sender = node_id(1);
        let mut msg = GroupMessage::new(
            GroupId::from("grp-1".to_string()),
            sender,
            "alice".into(),
            "Signed message".into(),
        );
        msg.sign(&seed);

        let bytes = rmp_serde::to_vec(&msg).expect("serialize");
        let decoded: GroupMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert!(decoded.verify_signature(), "signature should survive msgpack roundtrip");
    }

    #[test]
    fn sender_key_entry_roundtrip() {
        let entry = SenderKeyEntry {
            owner_id: node_id(1),
            key: [42u8; 32],
            epoch: 1,
            created_at: 1000,
        };
        let bytes = rmp_serde::to_vec(&entry).expect("serialize");
        let decoded: SenderKeyEntry = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(entry, decoded);
    }

    #[test]
    fn group_message_content_roundtrip() {
        let content = GroupMessageContent {
            username: "alice".into(),
            text: "Hello!".into(),
        };
        let bytes = rmp_serde::to_vec(&content).expect("serialize");
        let decoded: GroupMessageContent = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(content, decoded);
    }

    #[test]
    fn encrypted_group_message_roundtrip() {
        let seed = secret_seed(1);
        let sender = node_id(1);
        let key = [7u8; 32];
        let mut msg = GroupMessage::new_encrypted(
            GroupId::from("grp-enc".to_string()),
            sender,
            "alice".into(),
            "Secret message".into(),
            &key,
            1,
        );
        assert!(msg.encrypted);
        assert!(msg.text.is_empty());
        assert!(!msg.ciphertext.is_empty());
        msg.sign(&seed);
        assert!(msg.verify_signature());
        let bytes = rmp_serde::to_vec(&msg).expect("serialize");
        let decoded: GroupMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert!(decoded.verify_signature());
        assert!(decoded.encrypted);
        let content = decoded.decrypt(&key).unwrap();
        assert_eq!(content.username, "alice");
        assert_eq!(content.text, "Secret message");
    }

    #[test]
    fn encrypted_group_message_wrong_key_fails() {
        let key1 = [7u8; 32];
        let key2 = [8u8; 32];
        let msg = GroupMessage::new_encrypted(
            GroupId::from("grp-enc".to_string()),
            node_id(1),
            "alice".into(),
            "Secret".into(),
            &key1,
            1,
        );
        assert!(msg.decrypt(&key2).is_err());
    }

    #[test]
    fn plaintext_message_decrypt_returns_content() {
        let msg = GroupMessage::new(
            GroupId::from("grp-1".to_string()),
            node_id(1),
            "alice".into(),
            "Plain text".into(),
        );
        let content = msg.decrypt(&[0u8; 32]).unwrap();
        assert_eq!(content.username, "alice");
        assert_eq!(content.text, "Plain text");
    }

    #[test]
    fn sender_key_distribution_roundtrip() {
        let payload = GroupPayload::SenderKeyDistribution {
            group_id: GroupId::from("grp-1".to_string()),
            from: node_id(1),
            epoch: 1,
            encrypted_keys: vec![],
        };
        let bytes = rmp_serde::to_vec(&payload).expect("serialize");
        let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(payload, decoded);
    }

    #[test]
    fn hub_shadow_sync_roundtrip() {
        let payload = GroupPayload::HubShadowSync {
            group_id: GroupId::from("grp-1".to_string()),
            members: vec![GroupMember {
                node_id: node_id(1),
                username: "alice".into(),
                joined_at: 1000,
                role: GroupMemberRole::Member,
            }],
            candidate_id: Some(node_id(3)),
            config_version: 1,
        };
        let bytes = rmp_serde::to_vec(&payload).expect("serialize");
        let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(payload, decoded);
    }

    #[test]
    fn hub_ping_pong_roundtrip() {
        let ping = GroupPayload::HubPing {
            group_id: GroupId::from("grp-1".to_string()),
        };
        let bytes = rmp_serde::to_vec(&ping).expect("serialize");
        let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(ping, decoded);

        let pong = GroupPayload::HubPong {
            group_id: GroupId::from("grp-1".to_string()),
        };
        let bytes = rmp_serde::to_vec(&pong).expect("serialize");
        let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(pong, decoded);
    }

    #[test]
    fn hub_unreachable_roundtrip() {
        let payload = GroupPayload::HubUnreachable {
            group_id: GroupId::from("grp-1".to_string()),
        };
        let bytes = rmp_serde::to_vec(&payload).expect("serialize");
        let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(payload, decoded);
    }

    #[test]
    fn candidate_assigned_roundtrip() {
        let payload = GroupPayload::CandidateAssigned {
            group_id: GroupId::from("grp-1".to_string()),
        };
        let bytes = rmp_serde::to_vec(&payload).expect("serialize");
        let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(payload, decoded);
    }
}
