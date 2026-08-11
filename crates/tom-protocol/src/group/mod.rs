/// Group messaging for ToM protocol.
///
/// Hub-and-spoke topology: one relay node acts as hub per group,
/// fanning out messages to all members. Pure state machines — no I/O.
pub mod election;
pub mod hub;
pub mod manager;
pub mod types;

// ── Well-known group constants ───────────────────────────────────
/// The well-known group ID for the public log channel. Every node targets this
/// same ID for joining/creating. Le préfixe `__` le RÉSERVE au système : un
/// groupe utilisateur ne portera jamais cet id, donc pas de collision (personne
/// ne crée volontairement un groupe nommé `__tomlog__`).
/// TODO(sécu, post-S1) : barrière DURE = refuser au protocole toute création
/// utilisateur d'un `name` préfixé `__` (aujourd'hui la réservation est par
/// convention, cf. relecture Fable BLOQUANT #1).
pub const LOG_GROUP_ID: &str = "__tomlog__";

pub use election::{elect_hub, ElectionReason, ElectionResult};
pub use hub::{GroupHub, GroupHubSnapshot};
pub use manager::{GroupManager, GroupManagerSnapshot};
pub use types::{
    EncryptedSenderKey, GroupAction, GroupEvent, GroupId, GroupInfo, GroupInvite, GroupMember,
    GroupMemberRole, GroupMessage, GroupMessageContent, GroupPayload, LeaveReason, SenderKeyEntry,
    CANDIDATE_ORPHAN_TIMEOUT_MS, HUB_ACK_TIMEOUT_MS, SHADOW_PING_FAILURE_THRESHOLD,
    SHADOW_PING_INTERVAL_MS, SHADOW_PING_TIMEOUT_MS, SENDER_KEY_EPOCH_GRACE_MS,
    SENDER_KEY_PURGE_MAX_AGE_MS, SENDER_KEY_ROTATE_MAX_AGE_MS,
    SENDER_KEY_ROTATE_MAX_MESSAGES, SENDER_KEY_ROTATE_RATE_LIMIT_MS,
};
