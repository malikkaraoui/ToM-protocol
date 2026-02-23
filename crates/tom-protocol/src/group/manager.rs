/// GroupManager — member-side group state machine.
///
/// Pure decision engine: no I/O. Returns `Vec<GroupAction>` that the
/// caller executes via the transport layer.
///
/// Tracks: groups we belong to, pending invites, message history.
use std::collections::HashMap;

use crate::group::types::*;
use crate::types::{now_ms, NodeId};

/// Member-side group state manager.
///
/// Handles group lifecycle from the perspective of a regular member:
/// creating groups, receiving/accepting invites, tracking members,
/// storing message history.
pub struct GroupManager {
    /// Our node identity.
    #[allow(dead_code)] // Used by GroupHub and future admin checks
    local_id: NodeId,
    /// Our display name.
    local_username: String,
    /// Groups we're a member of (group_id → GroupInfo).
    groups: HashMap<GroupId, GroupInfo>,
    /// Pending invitations (group_id → GroupInvite).
    pending_invites: HashMap<GroupId, GroupInvite>,
    /// Message history per group (group_id → messages).
    message_history: HashMap<GroupId, Vec<GroupMessage>>,
    /// Max messages to keep per group.
    max_history_per_group: usize,
}

impl GroupManager {
    /// Create a new GroupManager for the local node.
    pub fn new(local_id: NodeId, local_username: String) -> Self {
        Self {
            local_id,
            local_username,
            groups: HashMap::new(),
            pending_invites: HashMap::new(),
            message_history: HashMap::new(),
            max_history_per_group: MAX_SYNC_MESSAGES,
        }
    }

    // ── Queries ──────────────────────────────────────────────────────────

    /// Get all groups we belong to.
    pub fn all_groups(&self) -> Vec<&GroupInfo> {
        self.groups.values().collect()
    }

    /// Get a specific group.
    pub fn get_group(&self, group_id: &GroupId) -> Option<&GroupInfo> {
        self.groups.get(group_id)
    }

    /// Check if we're in a group.
    pub fn is_in_group(&self, group_id: &GroupId) -> bool {
        self.groups.contains_key(group_id)
    }

    /// Get pending invitations.
    pub fn pending_invites(&self) -> Vec<&GroupInvite> {
        self.pending_invites.values().collect()
    }

    /// Get message history for a group.
    pub fn message_history(&self, group_id: &GroupId) -> &[GroupMessage] {
        self.message_history
            .get(group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get recent messages for sync (when we're also a hub).
    pub fn messages_for_sync(&self, group_id: &GroupId) -> Vec<GroupMessage> {
        self.message_history
            .get(group_id)
            .cloned()
            .unwrap_or_default()
    }

    // ── Group Creation ───────────────────────────────────────────────────

    /// Initiate group creation. Returns actions to send to the hub.
    ///
    /// The caller should send the `GroupPayload::Create` to the designated
    /// hub relay. The hub will respond with `GroupPayload::Created`.
    pub fn create_group(
        &self,
        name: String,
        hub_relay_id: NodeId,
        initial_members: Vec<NodeId>,
    ) -> Vec<GroupAction> {
        vec![GroupAction::Send {
            to: hub_relay_id,
            payload: GroupPayload::Create {
                group_name: name,
                creator_username: self.local_username.clone(),
                initial_members,
            },
        }]
    }

    /// Handle group creation confirmation from hub.
    pub fn handle_group_created(&mut self, group: GroupInfo) -> Vec<GroupAction> {
        let group_id = group.group_id.clone();
        self.groups.insert(group_id, group.clone());
        self.message_history
            .entry(group.group_id.clone())
            .or_default();
        vec![GroupAction::Event(GroupEvent::GroupCreated(group))]
    }

    // ── Invitations ──────────────────────────────────────────────────────

    /// Handle an incoming invitation.
    pub fn handle_invite(
        &mut self,
        group_id: GroupId,
        group_name: String,
        inviter_id: NodeId,
        inviter_username: String,
        hub_relay_id: NodeId,
    ) -> Vec<GroupAction> {
        // Ignore if we're already in the group
        if self.groups.contains_key(&group_id) {
            return vec![];
        }

        let invite = GroupInvite {
            group_id: group_id.clone(),
            group_name,
            inviter_id,
            inviter_username,
            hub_relay_id,
            invited_at: now_ms(),
            expires_at: now_ms() + INVITE_TTL_MS,
        };

        self.pending_invites.insert(group_id, invite.clone());
        vec![GroupAction::Event(GroupEvent::InviteReceived(invite))]
    }

    /// Accept a pending invitation. Returns actions to send join request to hub.
    pub fn accept_invite(&mut self, group_id: &GroupId) -> Vec<GroupAction> {
        let Some(invite) = self.pending_invites.remove(group_id) else {
            return vec![];
        };

        if invite.is_expired(now_ms()) {
            return vec![];
        }

        vec![GroupAction::Send {
            to: invite.hub_relay_id,
            payload: GroupPayload::Join {
                group_id: group_id.clone(),
                username: self.local_username.clone(),
            },
        }]
    }

    /// Decline a pending invitation.
    pub fn decline_invite(&mut self, group_id: &GroupId) -> bool {
        self.pending_invites.remove(group_id).is_some()
    }

    /// Remove expired invites. Returns number removed.
    pub fn cleanup_expired_invites(&mut self) -> usize {
        let now = now_ms();
        let before = self.pending_invites.len();
        self.pending_invites.retain(|_, inv| !inv.is_expired(now));
        before - self.pending_invites.len()
    }

    // ── Membership Changes ───────────────────────────────────────────────

    /// Handle group sync from hub (we just joined successfully).
    pub fn handle_group_sync(
        &mut self,
        group: GroupInfo,
        recent_messages: Vec<GroupMessage>,
    ) -> Vec<GroupAction> {
        let group_id = group.group_id.clone();
        let group_name = group.name.clone();
        self.groups.insert(group_id.clone(), group);

        // Store synced messages
        let history = self.message_history.entry(group_id.clone()).or_default();
        for msg in recent_messages {
            if history.len() < self.max_history_per_group {
                history.push(msg);
            }
        }

        vec![GroupAction::Event(GroupEvent::Joined {
            group_id,
            group_name,
        })]
    }

    /// Handle notification that a new member joined one of our groups.
    pub fn handle_member_joined(
        &mut self,
        group_id: &GroupId,
        member: GroupMember,
    ) -> Vec<GroupAction> {
        let Some(group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        // Don't add duplicates
        if group.is_member(&member.node_id) {
            return vec![];
        }

        group.members.push(member.clone());
        group.last_activity_at = now_ms();

        vec![GroupAction::Event(GroupEvent::MemberJoined {
            group_id: group_id.clone(),
            member,
        })]
    }

    /// Handle notification that a member left one of our groups.
    pub fn handle_member_left(
        &mut self,
        group_id: &GroupId,
        node_id: &NodeId,
        username: String,
        reason: LeaveReason,
    ) -> Vec<GroupAction> {
        let Some(group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        group.members.retain(|m| m.node_id != *node_id);
        group.last_activity_at = now_ms();

        vec![GroupAction::Event(GroupEvent::MemberLeft {
            group_id: group_id.clone(),
            node_id: *node_id,
            username,
            reason,
        })]
    }

    /// Leave a group voluntarily. Returns actions to notify the hub.
    pub fn leave_group(&mut self, group_id: &GroupId) -> Vec<GroupAction> {
        let Some(group) = self.groups.remove(group_id) else {
            return vec![];
        };

        self.message_history.remove(group_id);

        vec![GroupAction::Send {
            to: group.hub_relay_id,
            payload: GroupPayload::Leave {
                group_id: group_id.clone(),
            },
        }]
    }

    // ── Messages ─────────────────────────────────────────────────────────

    /// Handle an incoming group message.
    pub fn handle_message(&mut self, message: GroupMessage) -> Vec<GroupAction> {
        let group_id = &message.group_id;
        if !self.groups.contains_key(group_id) {
            return vec![];
        }

        // Update last activity
        if let Some(group) = self.groups.get_mut(group_id) {
            group.last_activity_at = now_ms();
        }

        // Store in history
        let history = self
            .message_history
            .entry(group_id.clone())
            .or_default();
        history.push(message.clone());

        // Trim if over capacity
        if history.len() > self.max_history_per_group {
            let excess = history.len() - self.max_history_per_group;
            history.drain(..excess);
        }

        vec![GroupAction::Event(GroupEvent::MessageReceived(message))]
    }

    // ── Hub Migration ────────────────────────────────────────────────────

    /// Handle hub migration notification.
    pub fn handle_hub_migration(
        &mut self,
        group_id: &GroupId,
        new_hub_id: NodeId,
    ) -> Vec<GroupAction> {
        let Some(group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        group.hub_relay_id = new_hub_id;
        group.last_activity_at = now_ms();

        vec![GroupAction::Event(GroupEvent::HubMigrated {
            group_id: group_id.clone(),
            new_hub_id,
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn make_manager() -> GroupManager {
        GroupManager::new(node_id(1), "alice".into())
    }

    fn make_test_group(admin_id: NodeId, hub_id: NodeId) -> GroupInfo {
        GroupInfo {
            group_id: GroupId::from("grp-test".to_string()),
            name: "Test Group".into(),
            hub_relay_id: hub_id,
            backup_hub_id: None,
            members: vec![GroupMember {
                node_id: admin_id,
                username: "alice".into(),
                joined_at: 1000,
                role: GroupMemberRole::Admin,
            }],
            created_by: admin_id,
            created_at: 1000,
            last_activity_at: 1000,
            max_members: MAX_GROUP_MEMBERS,
        }
    }

    #[test]
    fn create_group_sends_to_hub() {
        let mgr = make_manager();
        let hub = node_id(10);
        let bob = node_id(2);

        let actions = mgr.create_group("Test".into(), hub, vec![bob]);
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            GroupAction::Send { to, payload } => {
                assert_eq!(*to, hub);
                match payload {
                    GroupPayload::Create {
                        group_name,
                        initial_members,
                        ..
                    } => {
                        assert_eq!(group_name, "Test");
                        assert_eq!(initial_members.len(), 1);
                    }
                    _ => panic!("expected Create payload"),
                }
            }
            _ => panic!("expected Send action"),
        }
    }

    #[test]
    fn handle_group_created() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();

        let actions = mgr.handle_group_created(group);
        assert_eq!(actions.len(), 1);
        assert!(mgr.is_in_group(&gid));
        assert!(mgr.get_group(&gid).is_some());
    }

    #[test]
    fn invite_flow() {
        let mut mgr = make_manager();
        let gid = GroupId::from("grp-invite".to_string());
        let hub = node_id(10);
        let inviter = node_id(2);

        // Receive invite
        let actions = mgr.handle_invite(
            gid.clone(),
            "Cool Group".into(),
            inviter,
            "bob".into(),
            hub,
        );
        assert_eq!(actions.len(), 1);
        assert_eq!(mgr.pending_invites().len(), 1);

        // Accept invite
        let actions = mgr.accept_invite(&gid);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Send { to, payload } => {
                assert_eq!(*to, hub);
                match payload {
                    GroupPayload::Join { group_id, .. } => {
                        assert_eq!(*group_id, gid);
                    }
                    _ => panic!("expected Join payload"),
                }
            }
            _ => panic!("expected Send action"),
        }

        // Invite removed after accept
        assert_eq!(mgr.pending_invites().len(), 0);
    }

    #[test]
    fn decline_invite() {
        let mut mgr = make_manager();
        let gid = GroupId::from("grp-decline".to_string());
        let hub = node_id(10);

        mgr.handle_invite(gid.clone(), "Group".into(), node_id(2), "bob".into(), hub);
        assert_eq!(mgr.pending_invites().len(), 1);

        assert!(mgr.decline_invite(&gid));
        assert_eq!(mgr.pending_invites().len(), 0);

        // Decline nonexistent
        assert!(!mgr.decline_invite(&gid));
    }

    #[test]
    fn ignore_invite_if_already_member() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();

        mgr.handle_group_created(group);

        // Invite for a group we're already in — should be ignored
        let actions = mgr.handle_invite(gid, "Test".into(), node_id(2), "bob".into(), hub);
        assert!(actions.is_empty());
    }

    #[test]
    fn handle_group_sync() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();

        let msg = GroupMessage {
            group_id: gid.clone(),
            message_id: "msg-1".into(),
            sender_id: node_id(2),
            sender_username: "bob".into(),
            text: "Welcome!".into(),
            ciphertext: Vec::new(),
            nonce: [0u8; 24],
            key_epoch: 0,
            encrypted: false,
            sent_at: 1000,
            sender_signature: Vec::new(),
        };

        let actions = mgr.handle_group_sync(group, vec![msg]);
        assert_eq!(actions.len(), 1);
        assert!(mgr.is_in_group(&gid));
        assert_eq!(mgr.message_history(&gid).len(), 1);
    }

    #[test]
    fn handle_member_joined() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();
        mgr.handle_group_created(group);

        let new_member = GroupMember {
            node_id: node_id(3),
            username: "charlie".into(),
            joined_at: 2000,
            role: GroupMemberRole::Member,
        };

        let actions = mgr.handle_member_joined(&gid, new_member);
        assert_eq!(actions.len(), 1);
        assert_eq!(mgr.get_group(&gid).unwrap().member_count(), 2);
    }

    #[test]
    fn handle_member_joined_ignores_duplicate() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();
        mgr.handle_group_created(group);

        // Try to add same member twice
        let member = GroupMember {
            node_id: node_id(1), // already admin
            username: "alice".into(),
            joined_at: 2000,
            role: GroupMemberRole::Member,
        };
        let actions = mgr.handle_member_joined(&gid, member);
        assert!(actions.is_empty());
    }

    #[test]
    fn handle_member_left() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let mut group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();
        let bob = node_id(2);
        group.members.push(GroupMember {
            node_id: bob,
            username: "bob".into(),
            joined_at: 1000,
            role: GroupMemberRole::Member,
        });
        mgr.handle_group_created(group);

        assert_eq!(mgr.get_group(&gid).unwrap().member_count(), 2);

        let actions = mgr.handle_member_left(&gid, &bob, "bob".into(), LeaveReason::Voluntary);
        assert_eq!(actions.len(), 1);
        assert_eq!(mgr.get_group(&gid).unwrap().member_count(), 1);
    }

    #[test]
    fn leave_group() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();
        mgr.handle_group_created(group);
        assert!(mgr.is_in_group(&gid));

        let actions = mgr.leave_group(&gid);
        assert_eq!(actions.len(), 1);
        assert!(!mgr.is_in_group(&gid));

        match &actions[0] {
            GroupAction::Send { to, payload } => {
                assert_eq!(*to, hub);
                assert!(matches!(payload, GroupPayload::Leave { .. }));
            }
            _ => panic!("expected Send action"),
        }
    }

    #[test]
    fn handle_message() {
        let mut mgr = make_manager();
        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();
        mgr.handle_group_created(group);

        let msg = GroupMessage::new(gid.clone(), node_id(2), "bob".into(), "Hello!".into());
        let actions = mgr.handle_message(msg);
        assert_eq!(actions.len(), 1);
        assert_eq!(mgr.message_history(&gid).len(), 1);
    }

    #[test]
    fn message_history_trimmed() {
        let mut mgr = make_manager();
        mgr.max_history_per_group = 3;

        let hub = node_id(10);
        let group = make_test_group(node_id(1), hub);
        let gid = group.group_id.clone();
        mgr.handle_group_created(group);

        for i in 0..5 {
            let msg = GroupMessage {
                group_id: gid.clone(),
                message_id: format!("msg-{}", i),
                sender_id: node_id(2),
                sender_username: "bob".into(),
                text: format!("Message {}", i),
                ciphertext: Vec::new(),
                nonce: [0u8; 24],
                key_epoch: 0,
                encrypted: false,
                sent_at: 1000 + i as u64,
                sender_signature: Vec::new(),
            };
            mgr.handle_message(msg);
        }

        let history = mgr.message_history(&gid);
        assert_eq!(history.len(), 3);
        // Should keep the most recent 3
        assert_eq!(history[0].text, "Message 2");
        assert_eq!(history[2].text, "Message 4");
    }

    #[test]
    fn ignore_message_for_unknown_group() {
        let mut mgr = make_manager();
        let msg = GroupMessage::new(
            GroupId::from("unknown".to_string()),
            node_id(2),
            "bob".into(),
            "Lost".into(),
        );
        let actions = mgr.handle_message(msg);
        assert!(actions.is_empty());
    }

    #[test]
    fn handle_hub_migration() {
        let mut mgr = make_manager();
        let old_hub = node_id(10);
        let new_hub = node_id(11);
        let group = make_test_group(node_id(1), old_hub);
        let gid = group.group_id.clone();
        mgr.handle_group_created(group);

        assert_eq!(mgr.get_group(&gid).unwrap().hub_relay_id, old_hub);

        let actions = mgr.handle_hub_migration(&gid, new_hub);
        assert_eq!(actions.len(), 1);
        assert_eq!(mgr.get_group(&gid).unwrap().hub_relay_id, new_hub);
    }

    #[test]
    fn all_groups_query() {
        let mut mgr = make_manager();
        assert_eq!(mgr.all_groups().len(), 0);

        let hub = node_id(10);
        let g1 = make_test_group(node_id(1), hub);
        mgr.handle_group_created(g1);

        let mut g2 = make_test_group(node_id(1), hub);
        g2.group_id = GroupId::from("grp-2".to_string());
        mgr.handle_group_created(g2);

        assert_eq!(mgr.all_groups().len(), 2);
    }
}
