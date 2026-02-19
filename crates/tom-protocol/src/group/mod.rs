/// Group messaging for ToM protocol.
///
/// Hub-and-spoke topology: one relay node acts as hub per group,
/// fanning out messages to all members. Pure state machines — no I/O.
pub mod election;
pub mod hub;
pub mod manager;
pub mod types;

pub use election::{elect_hub, ElectionReason, ElectionResult};
pub use hub::GroupHub;
pub use manager::GroupManager;
pub use types::{
    GroupAction, GroupEvent, GroupId, GroupInfo, GroupInvite, GroupMember, GroupMemberRole,
    GroupMessage, GroupPayload, LeaveReason,
};
