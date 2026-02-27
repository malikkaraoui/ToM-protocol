/// GroupHub — hub-side fan-out engine for group messaging.
///
/// Runs on relay nodes. Pure state machine: receives group payloads,
/// returns `Vec<GroupAction>` for the caller to send via transport.
///
/// Responsibilities:
/// - Manage group membership (create, join, leave, kick)
/// - Fan out messages to all members except sender
/// - Rate limiting per sender per group
/// - Message dedup via nonce/ID tracking
/// - Message history for sync to new members
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use crate::group::types::*;
use crate::types::{now_ms, NodeId};

/// Maximum age of a group message before the hub rejects it (5 minutes).
const MESSAGE_MAX_AGE_MS: u64 = 5 * 60 * 1000;

/// Maximum clock skew (future) allowed for group messages (30 seconds).
const MESSAGE_MAX_FUTURE_MS: u64 = 30 * 1000;

/// Hub-side state for a single group.
struct HubGroup {
    info: GroupInfo,
    /// Recent messages (for sync to joining members).
    message_history: VecDeque<GroupMessage>,
    /// Rate limiting: sender → (window_start, count).
    rate_limits: HashMap<NodeId, (Instant, u32)>,
    /// Dedup: seen message IDs (bounded).
    seen_message_ids: HashSet<String>,
    /// Anti-replay: seen nonces for encrypted messages (bounded).
    seen_nonces: HashSet<[u8; 24]>,
}

/// Hub-side fan-out engine for all groups managed by this relay.
pub struct GroupHub {
    /// This hub's node identity.
    hub_id: NodeId,
    /// Groups managed by this hub (group_id → HubGroup).
    groups: HashMap<GroupId, HubGroup>,
    /// Max messages per group history.
    max_messages_per_group: usize,
    /// Max total messages across all groups (memory protection).
    max_total_messages: usize,
    /// Current total messages across all groups.
    total_messages: usize,
    /// Max dedup entries per group.
    max_dedup_entries: usize,
}

impl GroupHub {
    /// Create a new GroupHub for the given relay node.
    pub fn new(hub_id: NodeId) -> Self {
        Self {
            hub_id,
            groups: HashMap::new(),
            max_messages_per_group: MAX_SYNC_MESSAGES,
            max_total_messages: 10_000,
            total_messages: 0,
            max_dedup_entries: 10_000,
        }
    }

    /// Number of groups managed.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Get group info.
    pub fn get_group(&self, group_id: &GroupId) -> Option<&GroupInfo> {
        self.groups.get(group_id).map(|g| &g.info)
    }

    /// Iterate over all hosted groups (group_id, group_info).
    pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &GroupInfo)> {
        self.groups.iter().map(|(id, g)| (id, &g.info))
    }

    /// Process an incoming group payload from a node.
    ///
    /// Returns actions the caller should execute (send/broadcast).
    pub fn handle_payload(
        &mut self,
        payload: GroupPayload,
        from: NodeId,
    ) -> Vec<GroupAction> {
        match payload {
            GroupPayload::Create {
                group_name,
                creator_username,
                initial_members,
            } => self.handle_create(from, group_name, creator_username, initial_members),

            GroupPayload::Join { group_id, username } => {
                self.handle_join(from, &group_id, username)
            }

            GroupPayload::Leave { group_id } => self.handle_leave(from, &group_id),

            GroupPayload::Message(msg) => self.handle_message(from, msg),

            GroupPayload::DeliveryAck {
                group_id,
                message_id,
            } => self.handle_delivery_ack(from, &group_id, &message_id),

            GroupPayload::SenderKeyDistribution {
                ref group_id,
                from: _,
                epoch: _,
                ref encrypted_keys,
            } => self.handle_sender_key_distribution(from, group_id, encrypted_keys),

            GroupPayload::HubPing { ref group_id } => self.handle_hub_ping(group_id, from),

            // Hub doesn't process these (they're outgoing from hub or failover-specific)
            GroupPayload::Created { .. }
            | GroupPayload::Invite { .. }
            | GroupPayload::Sync { .. }
            | GroupPayload::MemberJoined { .. }
            | GroupPayload::MemberLeft { .. }
            | GroupPayload::HubMigration { .. }
            | GroupPayload::HubHeartbeat { .. }
            | GroupPayload::HubPong { .. }
            | GroupPayload::HubShadowSync { .. }
            | GroupPayload::CandidateAssigned { .. }
            | GroupPayload::HubUnreachable { .. } => vec![],
        }
    }

    // ── Group Creation ───────────────────────────────────────────────────

    fn handle_create(
        &mut self,
        creator: NodeId,
        group_name: String,
        creator_username: String,
        initial_members: Vec<NodeId>,
    ) -> Vec<GroupAction> {
        let group_id = GroupId::new();
        let now = now_ms();

        let admin = GroupMember {
            node_id: creator,
            username: creator_username,
            joined_at: now,
            role: GroupMemberRole::Admin,
        };

        let info = GroupInfo {
            group_id: group_id.clone(),
            name: group_name.clone(),
            hub_relay_id: self.hub_id,
            backup_hub_id: None,
            members: vec![admin],
            created_by: creator,
            created_at: now,
            last_activity_at: now,
            max_members: MAX_GROUP_MEMBERS,
            shadow_id: None,
            candidate_id: None,
        };

        let hub_group = HubGroup {
            info: info.clone(),
            message_history: VecDeque::new(),
            rate_limits: HashMap::new(),
            seen_message_ids: HashSet::new(),
            seen_nonces: HashSet::new(),
        };

        self.groups.insert(group_id.clone(), hub_group);

        let mut actions = vec![];

        // Confirm creation to creator
        actions.push(GroupAction::Send {
            to: creator,
            payload: GroupPayload::Created {
                group: info.clone(),
            },
        });

        // Send invitations to initial members
        for member_id in initial_members {
            if member_id == creator {
                continue; // Don't invite yourself
            }
            actions.push(GroupAction::Send {
                to: member_id,
                payload: GroupPayload::Invite {
                    group_id: group_id.clone(),
                    group_name: group_name.clone(),
                    inviter_id: creator,
                    inviter_username: info.created_by.to_string(),
                },
            });
        }

        actions
    }

    // ── Join ─────────────────────────────────────────────────────────────

    fn handle_join(
        &mut self,
        joiner: NodeId,
        group_id: &GroupId,
        username: String,
    ) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![]; // Group doesn't exist
        };

        // Already a member?
        if hub_group.info.is_member(&joiner) {
            return vec![];
        }

        // Group full?
        if hub_group.info.is_full() {
            return vec![];
        }

        let now = now_ms();
        let new_member = GroupMember {
            node_id: joiner,
            username: username.clone(),
            joined_at: now,
            role: GroupMemberRole::Member,
        };

        hub_group.info.members.push(new_member.clone());
        hub_group.info.last_activity_at = now;

        let mut actions = vec![];

        // Send sync to new member (group state + recent messages)
        let recent: Vec<GroupMessage> = hub_group.message_history.iter().cloned().collect();
        actions.push(GroupAction::Send {
            to: joiner,
            payload: GroupPayload::Sync {
                group: hub_group.info.clone(),
                recent_messages: recent,
            },
        });

        // Broadcast member joined to existing members (except new member)
        let existing: Vec<NodeId> = hub_group
            .info
            .members
            .iter()
            .filter(|m| m.node_id != joiner)
            .map(|m| m.node_id)
            .collect();

        if !existing.is_empty() {
            actions.push(GroupAction::Broadcast {
                to: existing,
                payload: GroupPayload::MemberJoined {
                    group_id: group_id.clone(),
                    member: new_member,
                },
            });
        }

        // After join: sync shadow if assigned
        let group_id_for_sync = group_id.clone();
        if let Some((target, payload)) = self.build_shadow_sync(&group_id_for_sync) {
            actions.push(GroupAction::Send { to: target, payload });
        }

        actions
    }

    // ── Leave ────────────────────────────────────────────────────────────

    fn handle_leave(&mut self, leaver: NodeId, group_id: &GroupId) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        let username = hub_group
            .info
            .get_member(&leaver)
            .map(|m| m.username.clone())
            .unwrap_or_default();

        hub_group.info.members.retain(|m| m.node_id != leaver);
        hub_group.info.last_activity_at = now_ms();

        // If no members left, remove the group
        if hub_group.info.members.is_empty() {
            self.groups.remove(group_id);
            return vec![];
        }

        // Broadcast departure to remaining members
        let remaining: Vec<NodeId> = hub_group
            .info
            .members
            .iter()
            .map(|m| m.node_id)
            .collect();

        let mut actions = vec![GroupAction::Broadcast {
            to: remaining,
            payload: GroupPayload::MemberLeft {
                group_id: group_id.clone(),
                node_id: leaver,
                username,
                reason: LeaveReason::Voluntary,
            },
        }];

        // After leave: sync shadow if assigned
        let group_id_for_sync = group_id.clone();
        if let Some((target, payload)) = self.build_shadow_sync(&group_id_for_sync) {
            actions.push(GroupAction::Send { to: target, payload });
        }

        actions
    }

    // ── Message Fanout ───────────────────────────────────────────────────

    fn handle_message(&mut self, from: NodeId, msg: GroupMessage) -> Vec<GroupAction> {
        let group_id = msg.group_id.clone();
        let message_id = msg.message_id.clone();

        // Check membership (immutable borrow)
        {
            let Some(hub_group) = self.groups.get(&group_id) else {
                return vec![];
            };
            if !hub_group.info.is_member(&from) {
                return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                    group_id,
                    node_id: from,
                    reason: "non-member attempted to send message".into(),
                })];
            }
        }

        // Mandatory signature: reject unsigned messages
        if !msg.is_signed() {
            return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                group_id,
                node_id: from,
                reason: "unsigned message rejected".into(),
            })];
        }

        // Verify sender signature
        if !msg.verify_signature() {
            return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                group_id,
                node_id: from,
                reason: "invalid message signature".into(),
            })];
        }

        // Timestamp validation: reject messages too old or too far in the future
        let now = now_ms();
        if msg.sent_at + MESSAGE_MAX_AGE_MS < now {
            return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                group_id,
                node_id: from,
                reason: "message timestamp too old (>5min)".into(),
            })];
        }
        if msg.sent_at > now + MESSAGE_MAX_FUTURE_MS {
            return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                group_id,
                node_id: from,
                reason: "message timestamp in the future (>30s)".into(),
            })];
        }

        // Nonce anti-replay for encrypted messages
        if msg.encrypted && !self.check_nonce(&group_id, &msg.nonce) {
            return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                group_id,
                node_id: from,
                reason: "nonce replay detected".into(),
            })];
        }

        // Rate limit check (mutable borrow scoped)
        if !self.check_rate_limit(&group_id, &from) {
            return vec![];
        }

        // Dedup check (mutable borrow scoped)
        if !self.check_dedup(&group_id, &message_id) {
            return vec![];
        }

        // Store message and collect recipients
        let recipients = {
            let hub_group = self.groups.get_mut(&group_id).unwrap();
            hub_group.info.last_activity_at = now_ms();

            // Store in history
            hub_group.message_history.push_back(msg.clone());
            self.total_messages += 1;

            // Trim per-group history
            while hub_group.message_history.len() > self.max_messages_per_group {
                hub_group.message_history.pop_front();
                self.total_messages = self.total_messages.saturating_sub(1);
            }

            // Collect recipients before releasing borrow
            hub_group
                .info
                .members
                .iter()
                .filter(|m| m.node_id != from)
                .map(|m| m.node_id)
                .collect::<Vec<_>>()
        };

        // Trim global messages if over capacity
        if self.total_messages > self.max_total_messages {
            self.trim_oldest_messages();
        }

        if recipients.is_empty() {
            return vec![];
        }

        vec![GroupAction::Broadcast {
            to: recipients,
            payload: GroupPayload::Message(msg),
        }]
    }

    // ── Delivery ACK ─────────────────────────────────────────────────────

    fn handle_delivery_ack(
        &self,
        _from: NodeId,
        _group_id: &GroupId,
        _message_id: &str,
    ) -> Vec<GroupAction> {
        // Track delivery confirmation (for future delivery tracking)
        // Currently a no-op — will be used for read receipts / delivery status
        vec![]
    }

    // ── Sender Key Distribution ─────────────────────────────────────────

    /// Fan out sender key distribution to individual recipients.
    ///
    /// The hub cannot read the keys (they are encrypted per-recipient).
    /// It simply delivers each encrypted key to the intended recipient.
    fn handle_sender_key_distribution(
        &self,
        from: NodeId,
        group_id: &GroupId,
        encrypted_keys: &[EncryptedSenderKey],
    ) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get(group_id) else {
            return vec![];
        };

        if !hub_group.info.is_member(&from) {
            return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                group_id: group_id.clone(),
                node_id: from,
                reason: "non-member sent sender key distribution".into(),
            })];
        }

        let mut actions = Vec::new();
        for ek in encrypted_keys {
            if hub_group.info.is_member(&ek.recipient_id) {
                actions.push(GroupAction::Send {
                    to: ek.recipient_id,
                    payload: GroupPayload::SenderKeyDistribution {
                        group_id: group_id.clone(),
                        from,
                        epoch: 0,
                        encrypted_keys: vec![ek.clone()],
                    },
                });
            }
        }
        actions
    }

    // ── Rate Limiting ────────────────────────────────────────────────────

    fn check_rate_limit(&mut self, group_id: &GroupId, sender: &NodeId) -> bool {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return false;
        };

        let now = Instant::now();
        let entry = hub_group.rate_limits.entry(*sender).or_insert((now, 0));

        // Reset window if > 1 second elapsed
        if now.duration_since(entry.0).as_secs() >= 1 {
            *entry = (now, 1);
            return true;
        }

        entry.1 += 1;
        entry.1 <= GROUP_RATE_LIMIT_PER_SECOND
    }

    // ── Nonce Anti-Replay ──────────────────────────────────────────────

    /// Check nonce for replay. Returns `true` if nonce is new, `false` if replayed.
    fn check_nonce(&mut self, group_id: &GroupId, nonce: &[u8; 24]) -> bool {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return false;
        };

        // Evict half when at capacity (same strategy as dedup)
        if hub_group.seen_nonces.len() >= self.max_dedup_entries {
            let to_keep = self.max_dedup_entries / 2;
            let drain: Vec<_> = hub_group.seen_nonces.iter().copied().take(hub_group.seen_nonces.len() - to_keep).collect();
            for n in drain {
                hub_group.seen_nonces.remove(&n);
            }
        }

        hub_group.seen_nonces.insert(*nonce)
    }

    // ── Dedup ────────────────────────────────────────────────────────────

    fn check_dedup(&mut self, group_id: &GroupId, message_id: &str) -> bool {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return false;
        };

        // Evict half when at capacity (retains recent entries better than clear())
        if hub_group.seen_message_ids.len() >= self.max_dedup_entries {
            let to_keep = self.max_dedup_entries / 2;
            let drain: Vec<_> = hub_group
                .seen_message_ids
                .iter()
                .take(hub_group.seen_message_ids.len() - to_keep)
                .cloned()
                .collect();
            for id in drain {
                hub_group.seen_message_ids.remove(&id);
            }
        }

        hub_group.seen_message_ids.insert(message_id.to_string())
    }

    // ── Memory Management ────────────────────────────────────────────────

    fn trim_oldest_messages(&mut self) {
        // Remove oldest messages from the group with the most messages
        let target = self.max_total_messages * 9 / 10; // Trim to 90%

        while self.total_messages > target {
            // Find group with most messages
            let largest = self
                .groups
                .iter()
                .max_by_key(|(_, g)| g.message_history.len())
                .map(|(id, _)| id.clone());

            if let Some(gid) = largest {
                if let Some(group) = self.groups.get_mut(&gid) {
                    if group.message_history.pop_front().is_some() {
                        self.total_messages = self.total_messages.saturating_sub(1);
                    } else {
                        break; // No more messages to trim
                    }
                }
            } else {
                break;
            }
        }
    }

    // ── Hub Migration ────────────────────────────────────────────────────

    /// Export group state for migration to a new hub.
    pub fn export_group(&self, group_id: &GroupId) -> Option<GroupInfo> {
        self.groups.get(group_id).map(|g| g.info.clone())
    }

    /// Import a group from another hub (migration).
    pub fn import_group(&mut self, info: GroupInfo, messages: Vec<GroupMessage>) {
        let group_id = info.group_id.clone();
        let msg_count = messages.len();

        let hub_group = HubGroup {
            info,
            message_history: messages.into_iter().collect(),
            rate_limits: HashMap::new(),
            seen_message_ids: HashSet::new(),
            seen_nonces: HashSet::new(),
        };

        self.groups.insert(group_id, hub_group);
        self.total_messages += msg_count;
    }

    /// Generate heartbeat actions for all groups.
    pub fn heartbeat_actions(&self) -> Vec<GroupAction> {
        let mut actions = vec![];

        for hub_group in self.groups.values() {
            let recipients: Vec<NodeId> = hub_group
                .info
                .members
                .iter()
                .map(|m| m.node_id)
                .collect();

            if !recipients.is_empty() {
                actions.push(GroupAction::Broadcast {
                    to: recipients,
                    payload: GroupPayload::HubHeartbeat {
                        group_id: hub_group.info.group_id.clone(),
                        member_count: hub_group.info.member_count(),
                    },
                });
            }
        }

        actions
    }

    /// Kick a member from a group (admin action, initiated externally).
    pub fn kick_member(
        &mut self,
        group_id: &GroupId,
        admin: &NodeId,
        target: &NodeId,
    ) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        // Only admins can kick
        if !hub_group.info.is_admin(admin) {
            return vec![];
        }

        // Can't kick yourself
        if admin == target {
            return vec![];
        }

        let username = hub_group
            .info
            .get_member(target)
            .map(|m| m.username.clone())
            .unwrap_or_default();

        hub_group.info.members.retain(|m| m.node_id != *target);
        hub_group.info.last_activity_at = now_ms();

        // Notify all remaining members (including the kicked person)
        let mut recipients: Vec<NodeId> = hub_group
            .info
            .members
            .iter()
            .map(|m| m.node_id)
            .collect();
        recipients.push(*target); // Notify the kicked member too

        vec![GroupAction::Broadcast {
            to: recipients,
            payload: GroupPayload::MemberLeft {
                group_id: group_id.clone(),
                node_id: *target,
                username,
                reason: LeaveReason::Kicked,
            },
        }]
    }

    // ── Hub Failover (Primary Side) ─────────────────────────────────────

    /// Assign a shadow for a group. Uses deterministic election (lowest NodeId
    /// among members, excluding the hub itself).
    pub fn assign_shadow(&mut self, group_id: &GroupId) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        // Pick shadow: lowest NodeId among members, excluding hub
        let mut candidates: Vec<NodeId> = hub_group
            .info
            .members
            .iter()
            .map(|m| m.node_id)
            .filter(|id| *id != self.hub_id)
            .collect();
        candidates.sort_by_key(|a| a.to_string());

        let shadow_id = candidates.first().copied();
        hub_group.info.shadow_id = shadow_id;

        if let Some(shadow) = shadow_id {
            // Also pick candidate: next member after shadow
            let candidate_id = candidates.get(1).copied();
            hub_group.info.candidate_id = candidate_id;

            let mut actions = vec![GroupAction::Send {
                to: shadow,
                payload: GroupPayload::HubShadowSync {
                    group_id: group_id.clone(),
                    members: hub_group.info.members.clone(),
                    candidate_id,
                    config_version: hub_group.info.last_activity_at,
                },
            }];

            if let Some(cand) = candidate_id {
                actions.push(GroupAction::Send {
                    to: cand,
                    payload: GroupPayload::CandidateAssigned {
                        group_id: group_id.clone(),
                    },
                });
            }

            actions
        } else {
            vec![]
        }
    }

    /// Build a HubShadowSync payload for a group (used on member changes).
    pub fn build_shadow_sync(&self, group_id: &GroupId) -> Option<(NodeId, GroupPayload)> {
        let hub_group = self.groups.get(group_id)?;
        let shadow = hub_group.info.shadow_id?;
        Some((
            shadow,
            GroupPayload::HubShadowSync {
                group_id: group_id.clone(),
                members: hub_group.info.members.clone(),
                candidate_id: hub_group.info.candidate_id,
                config_version: hub_group.info.last_activity_at,
            },
        ))
    }

    /// Handle a HubPing from a member — respond with HubPong.
    pub fn handle_hub_ping(&self, group_id: &GroupId, from: NodeId) -> Vec<GroupAction> {
        // Verify group exists and sender is a member
        let Some(hub_group) = self.groups.get(group_id) else {
            return vec![];
        };
        // Respond to any member who pings (the shadow might not be assigned yet)
        if !hub_group.info.members.iter().any(|m| m.node_id == from) {
            return vec![];
        }
        vec![GroupAction::Send {
            to: from,
            payload: GroupPayload::HubPong {
                group_id: group_id.clone(),
            },
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        keypair(seed).0
    }

    fn keypair(seed: u8) -> (NodeId, [u8; 32]) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        let node_id: NodeId = secret.public().to_string().parse().unwrap();
        (node_id, secret.to_bytes())
    }

    /// Create a signed GroupMessage (mandatory since security hardening).
    fn signed_msg(group_id: GroupId, sender_seed: u8, text: &str) -> GroupMessage {
        let (sender_id, secret) = keypair(sender_seed);
        let mut msg = GroupMessage::new(group_id, sender_id, "test".into(), text.into());
        msg.sign(&secret);
        msg
    }

    fn make_hub() -> GroupHub {
        GroupHub::new(node_id(10))
    }

    #[test]
    fn create_group() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        let actions = hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![bob],
            },
            alice,
        );

        assert_eq!(hub.group_count(), 1);

        // Should have: Created (to alice) + Invite (to bob)
        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], GroupAction::Send { to, payload: GroupPayload::Created { .. } } if *to == alice));
        assert!(matches!(&actions[1], GroupAction::Send { to, payload: GroupPayload::Invite { .. } } if *to == bob));
    }

    #[test]
    fn create_group_doesnt_invite_creator() {
        let mut hub = make_hub();
        let alice = node_id(1);

        let actions = hub.handle_payload(
            GroupPayload::Create {
                group_name: "Solo".into(),
                creator_username: "alice".into(),
                initial_members: vec![alice], // invite self
            },
            alice,
        );

        // Only Created confirmation, no self-invite
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], GroupAction::Send { payload: GroupPayload::Created { .. }, .. }));
    }

    #[test]
    fn join_group() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        // Create group
        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );

        let gid = hub.groups.keys().next().unwrap().clone();

        // Bob joins
        let actions = hub.handle_join(bob, &gid, "bob".into());

        // Should: Sync (to bob) + MemberJoined broadcast (to alice)
        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], GroupAction::Send { to, payload: GroupPayload::Sync { .. } } if *to == bob));
        assert!(matches!(&actions[1], GroupAction::Broadcast { payload: GroupPayload::MemberJoined { .. }, .. }));

        let group = hub.get_group(&gid).unwrap();
        assert_eq!(group.member_count(), 2);
    }

    #[test]
    fn join_duplicate_ignored() {
        let mut hub = make_hub();
        let alice = node_id(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        // Alice tries to join again
        let actions = hub.handle_join(alice, &gid, "alice".into());
        assert!(actions.is_empty());
    }

    #[test]
    fn join_full_group_rejected() {
        let mut hub = make_hub();
        let alice = node_id(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Tiny".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        // Set max to 1
        hub.groups.get_mut(&gid).unwrap().info.max_members = 1;

        let bob = node_id(2);
        let actions = hub.handle_join(bob, &gid, "bob".into());
        assert!(actions.is_empty());
    }

    #[test]
    fn leave_group() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 2);

        let actions = hub.handle_leave(bob, &gid);
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], GroupAction::Broadcast { payload: GroupPayload::MemberLeft { reason: LeaveReason::Voluntary, .. }, .. }));
        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 1);
    }

    #[test]
    fn leave_last_member_removes_group() {
        let mut hub = make_hub();
        let alice = node_id(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Solo".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        let actions = hub.handle_leave(alice, &gid);
        assert!(actions.is_empty()); // No one to notify
        assert_eq!(hub.group_count(), 0);
    }

    #[test]
    fn message_fanout() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let charlie = node_id(3);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Chat".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());
        hub.handle_join(charlie, &gid, "charlie".into());

        // Alice sends a signed message
        let msg = signed_msg(gid.clone(), 1, "Hello!");
        let actions = hub.handle_message(alice, msg);

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Broadcast { to, payload } => {
                // Should go to bob and charlie, NOT alice
                assert_eq!(to.len(), 2);
                assert!(to.contains(&bob));
                assert!(to.contains(&charlie));
                assert!(!to.contains(&alice));
                assert!(matches!(payload, GroupPayload::Message(_)));
            }
            _ => panic!("expected Broadcast action"),
        }
    }

    #[test]
    fn message_from_nonmember_ignored() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let stranger = node_id(99);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        let msg = GroupMessage::new(gid, stranger, "stranger".into(), "Sneak!".into());
        let actions = hub.handle_message(stranger, msg);
        // Non-member now returns SecurityViolation instead of silent drop
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], GroupAction::Event(GroupEvent::SecurityViolation { .. })));
    }

    #[test]
    fn rate_limiting() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Send up to the rate limit (all signed)
        for i in 0..GROUP_RATE_LIMIT_PER_SECOND {
            let msg = signed_msg(gid.clone(), 1, &format!("msg-{i}"));
            let actions = hub.handle_message(alice, msg);
            assert_eq!(actions.len(), 1, "message {} should succeed", i);
        }

        // Next one should be rate-limited
        let msg = signed_msg(gid.clone(), 1, "spam");
        let actions = hub.handle_message(alice, msg);
        assert!(actions.is_empty(), "should be rate-limited");
    }

    #[test]
    fn dedup_prevents_replay() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let (_, alice_secret) = keypair(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        let mut msg = GroupMessage {
            group_id: gid.clone(),
            message_id: "fixed-id".into(),
            sender_id: alice,
            sender_username: "alice".into(),
            text: "Hello".into(),
            ciphertext: Vec::new(),
            nonce: [0u8; 24],
            key_epoch: 0,
            encrypted: false,
            sent_at: now_ms(),
            sender_signature: Vec::new(),
        };
        msg.sign(&alice_secret);

        // First send succeeds
        let actions = hub.handle_message(alice, msg.clone());
        assert_eq!(actions.len(), 1);

        // Replay blocked
        let actions = hub.handle_message(alice, msg);
        assert!(actions.is_empty());
    }

    #[test]
    fn message_history_for_sync() {
        let mut hub = make_hub();
        hub.max_messages_per_group = 3;

        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Send 5 signed messages (history keeps last 3)
        for i in 0..5 {
            let msg = signed_msg(gid.clone(), 1, &format!("msg-{i}"));
            hub.handle_message(alice, msg);
        }

        // New member joins — should get last 3 messages in sync
        let charlie = node_id(3);
        let actions = hub.handle_join(charlie, &gid, "charlie".into());

        // Find the Sync action
        let sync = actions.iter().find(|a| matches!(a, GroupAction::Send { payload: GroupPayload::Sync { .. }, .. }));
        assert!(sync.is_some());

        if let GroupAction::Send { payload: GroupPayload::Sync { recent_messages, .. }, .. } = sync.unwrap() {
            assert_eq!(recent_messages.len(), 3);
        }
    }

    #[test]
    fn kick_member() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Admin kicks bob
        let actions = hub.kick_member(&gid, &alice, &bob);
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            GroupAction::Broadcast { to, payload } => {
                // Both alice and bob notified
                assert!(to.contains(&alice));
                assert!(to.contains(&bob));
                assert!(matches!(payload, GroupPayload::MemberLeft { reason: LeaveReason::Kicked, .. }));
            }
            _ => panic!("expected Broadcast"),
        }

        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 1);
    }

    #[test]
    fn non_admin_cant_kick() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Bob (non-admin) tries to kick alice
        let actions = hub.kick_member(&gid, &bob, &alice);
        assert!(actions.is_empty());
        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 2);
    }

    #[test]
    fn heartbeat_actions() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        let actions = hub.heartbeat_actions();
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            GroupAction::Broadcast { to, payload } => {
                assert_eq!(to.len(), 2);
                assert!(matches!(payload, GroupPayload::HubHeartbeat { member_count: 2, .. }));
            }
            _ => panic!("expected Broadcast"),
        }
    }

    #[test]
    fn export_import_migration() {
        let mut hub1 = make_hub();
        let alice = node_id(1);

        hub1.handle_payload(
            GroupPayload::Create {
                group_name: "Migrate".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub1.groups.keys().next().unwrap().clone();

        // Export from hub1
        let exported = hub1.export_group(&gid).unwrap();
        assert_eq!(exported.name, "Migrate");

        // Import into hub2
        let mut hub2 = GroupHub::new(node_id(11));
        hub2.import_group(exported, vec![]);
        assert_eq!(hub2.group_count(), 1);
        assert!(hub2.get_group(&gid).is_some());
    }

    // ── Sender Key Distribution Tests ─────────────────────────────────

    #[test]
    fn sender_key_distribution_fanout() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let charlie = node_id(3);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "E2E".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());
        hub.handle_join(charlie, &gid, "charlie".into());

        let actions = hub.handle_payload(
            GroupPayload::SenderKeyDistribution {
                group_id: gid.clone(),
                from: alice,
                epoch: 1,
                encrypted_keys: vec![
                    EncryptedSenderKey {
                        recipient_id: bob,
                        encrypted_key: crate::crypto::EncryptedPayload {
                            ciphertext: vec![1, 2, 3],
                            nonce: [0u8; 24],
                            ephemeral_pk: [0u8; 32],
                        },
                    },
                    EncryptedSenderKey {
                        recipient_id: charlie,
                        encrypted_key: crate::crypto::EncryptedPayload {
                            ciphertext: vec![4, 5, 6],
                            nonce: [0u8; 24],
                            ephemeral_pk: [0u8; 32],
                        },
                    },
                ],
            },
            alice,
        );

        assert_eq!(actions.len(), 2);
        assert!(
            matches!(&actions[0], GroupAction::Send { to, .. } if *to == bob)
        );
        assert!(
            matches!(&actions[1], GroupAction::Send { to, .. } if *to == charlie)
        );
    }

    // ── Hub Failover Tests ────────────────────────────────────────────

    #[test]
    fn assign_shadow_on_group_create() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let charlie = node_id(3);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Failover".into(),
                creator_username: "alice".into(),
                initial_members: vec![bob, charlie],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());
        hub.handle_join(charlie, &gid, "charlie".into());

        let actions = hub.assign_shadow(&gid);
        assert!(!actions.is_empty(), "should produce HubShadowSync action");

        let shadow_sync_found = actions.iter().any(|a| {
            matches!(a, GroupAction::Send { payload: GroupPayload::HubShadowSync { .. }, .. })
        });
        assert!(shadow_sync_found, "should send HubShadowSync to shadow");
    }

    #[test]
    fn hub_responds_pong_to_ping() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let shadow = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Pong".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(shadow, &gid, "shadow".into());

        let actions = hub.handle_hub_ping(&gid, shadow);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            GroupAction::Send { to, payload: GroupPayload::HubPong { .. } } if *to == shadow
        ));
    }

    #[test]
    fn shadow_sync_contains_current_members() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Sync".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Assign shadow first so build_shadow_sync can find the target
        hub.assign_shadow(&gid);

        let sync = hub.build_shadow_sync(&gid);
        assert!(sync.is_some());
        let (_, payload) = sync.unwrap();
        if let GroupPayload::HubShadowSync { members, .. } = &payload {
            // Should contain creator + bob
            assert!(members.len() >= 2);
        } else {
            panic!("expected HubShadowSync");
        }
    }

    #[test]
    fn sender_key_from_nonmember_rejected() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let stranger = node_id(99);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Secure".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        let actions = hub.handle_payload(
            GroupPayload::SenderKeyDistribution {
                group_id: gid,
                from: stranger,
                epoch: 1,
                encrypted_keys: vec![],
            },
            stranger,
        );

        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            GroupAction::Event(GroupEvent::SecurityViolation { .. })
        ));
    }

    // ── r5: Security hardening tests ────────────────────────────────

    #[test]
    fn unsigned_message_rejected() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Unsigned message should be rejected
        let msg = GroupMessage::new(gid.clone(), alice, "alice".into(), "unsigned".into());
        let actions = hub.handle_message(alice, msg);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Event(GroupEvent::SecurityViolation { reason, .. }) => {
                assert!(reason.contains("unsigned"), "reason: {reason}");
            }
            other => panic!("expected SecurityViolation, got: {other:?}"),
        }
    }

    #[test]
    fn forged_signature_rejected() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let (_, wrong_secret) = keypair(99); // Wrong key

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        let mut msg = GroupMessage::new(gid.clone(), alice, "alice".into(), "forged".into());
        msg.sign(&wrong_secret); // Signed with wrong key
        let actions = hub.handle_message(alice, msg);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Event(GroupEvent::SecurityViolation { reason, .. }) => {
                assert!(reason.contains("invalid"), "reason: {reason}");
            }
            other => panic!("expected SecurityViolation, got: {other:?}"),
        }
    }

    #[test]
    fn old_timestamp_rejected() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let (_, alice_secret) = keypair(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Message with timestamp 10 minutes ago
        let mut msg = GroupMessage::new(gid.clone(), alice, "alice".into(), "old".into());
        msg.sent_at = now_ms().saturating_sub(10 * 60 * 1000);
        msg.sign(&alice_secret);

        let actions = hub.handle_message(alice, msg);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Event(GroupEvent::SecurityViolation { reason, .. }) => {
                assert!(reason.contains("too old"), "reason: {reason}");
            }
            other => panic!("expected SecurityViolation for old timestamp, got: {other:?}"),
        }
    }

    #[test]
    fn future_timestamp_rejected() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let (_, alice_secret) = keypair(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Message 2 minutes in the future
        let mut msg = GroupMessage::new(gid.clone(), alice, "alice".into(), "future".into());
        msg.sent_at = now_ms() + 2 * 60 * 1000;
        msg.sign(&alice_secret);

        let actions = hub.handle_message(alice, msg);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Event(GroupEvent::SecurityViolation { reason, .. }) => {
                assert!(reason.contains("future"), "reason: {reason}");
            }
            other => panic!("expected SecurityViolation for future timestamp, got: {other:?}"),
        }
    }

    #[test]
    fn nonce_replay_detected() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let (_, alice_secret) = keypair(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        let nonce = [42u8; 24];

        // First encrypted message with this nonce
        let mut msg1 = GroupMessage {
            group_id: gid.clone(),
            message_id: "msg-1".into(),
            sender_id: alice,
            sender_username: "alice".into(),
            text: String::new(),
            ciphertext: vec![1, 2, 3],
            nonce,
            key_epoch: 1,
            encrypted: true,
            sent_at: now_ms(),
            sender_signature: Vec::new(),
        };
        msg1.sign(&alice_secret);
        let actions = hub.handle_message(alice, msg1);
        assert_eq!(actions.len(), 1, "first message should succeed");

        // Second message with SAME nonce but different message_id
        let mut msg2 = GroupMessage {
            group_id: gid.clone(),
            message_id: "msg-2".into(),
            sender_id: alice,
            sender_username: "alice".into(),
            text: String::new(),
            ciphertext: vec![4, 5, 6],
            nonce, // Same nonce!
            key_epoch: 1,
            encrypted: true,
            sent_at: now_ms(),
            sender_signature: Vec::new(),
        };
        msg2.sign(&alice_secret);
        let actions = hub.handle_message(alice, msg2);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Event(GroupEvent::SecurityViolation { reason, .. }) => {
                assert!(reason.contains("nonce replay"), "reason: {reason}");
            }
            other => panic!("expected nonce replay SecurityViolation, got: {other:?}"),
        }
    }

    #[test]
    fn dedup_eviction_retains_recent() {
        let mut hub = make_hub();
        hub.max_dedup_entries = 4; // Low limit for testing

        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Send 4 messages to fill dedup set (within rate limit of 5/sec)
        for i in 0..4 {
            let msg = signed_msg(gid.clone(), 1, &format!("fill-{i}"));
            hub.handle_message(alice, msg);
        }

        // 5th message triggers eviction
        let msg = signed_msg(gid.clone(), 1, "trigger-evict");
        let actions = hub.handle_message(alice, msg);
        assert_eq!(actions.len(), 1, "5th message should succeed after eviction");

        // Verify dedup set is reduced but not empty
        let hub_group = hub.groups.get(&gid).unwrap();
        assert!(
            hub_group.seen_message_ids.len() <= 4,
            "dedup set should be bounded: {}",
            hub_group.seen_message_ids.len()
        );
        assert!(
            hub_group.seen_message_ids.len() >= 2,
            "dedup set should retain recent entries: {}",
            hub_group.seen_message_ids.len()
        );
    }
}
