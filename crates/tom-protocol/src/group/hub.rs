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

use serde::{Deserialize, Serialize};

use crate::group::types::*;
use crate::types::{now_ms, NodeId};

/// Serializable snapshot of GroupHub's persistent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupHubSnapshot {
    pub groups: HashMap<GroupId, GroupInfo>,
    /// Invited sets per group (for invite-only groups). Restored on startup.
    #[serde(default)]
    pub invited_sets: HashMap<GroupId, HashSet<NodeId>>,
    /// Next sequence number per group (for offline delivery gap-fill).
    #[serde(default)]
    pub next_seqs: HashMap<GroupId, u64>,
}

/// Maximum age of a group message before the hub rejects it (5 minutes).
const MESSAGE_MAX_AGE_MS: u64 = 5 * 60 * 1000;

/// Maximum clock skew (future) allowed for group messages (30 seconds).
const MESSAGE_MAX_FUTURE_MS: u64 = 30 * 1000;

/// Hub-side state for a single group.
struct HubGroup {
    info: GroupInfo,
    /// Recent messages (for sync to joining members).
    message_history: VecDeque<GroupMessage>,
    /// Monotonically increasing sequence number for messages in this group.
    next_seq: u64,
    /// Rate limiting: sender → (window_start, count).
    rate_limits: HashMap<NodeId, (Instant, u32)>,
    /// Dedup: seen message IDs (bounded).
    seen_message_ids: HashSet<String>,
    /// Anti-replay: seen nonces for encrypted messages (bounded).
    seen_nonces: HashSet<[u8; 24]>,
    /// Nodes that have been invited (for invite-only groups).
    invited_set: HashSet<NodeId>,
    /// Latest sender key payload per sender (for proactive re-sync fanout).
    latest_sender_keys: HashMap<NodeId, (u32, Vec<EncryptedSenderKey>)>,
    /// Epoch state per sender.
    sender_epoch_state: HashMap<NodeId, SenderEpochState>,
    /// Group message counter since last rotation trigger.
    group_msg_since_rotation: u64,
    /// Last rotation trigger timestamp.
    last_rotation_trigger_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct SenderEpochState {
    current_epoch: u32,
    previous_epoch: Option<u32>,
    grace_until_ms: u64,
}

enum EpochDecision {
    Accept,
    Reject,
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

    /// Get the in-memory message history for a group (R13 gap-fill).
    pub fn message_history(&self, group_id: &GroupId) -> Option<&VecDeque<GroupMessage>> {
        self.groups.get(group_id).map(|g| &g.message_history)
    }

    /// Remove messages older than `max_age_ms` from in-memory history.
    /// Returns number of messages purged across all groups.
    pub fn cleanup_expired_messages(&mut self, now_ms: u64, max_age_ms: u64) -> usize {
        let cutoff = now_ms.saturating_sub(max_age_ms);
        let mut total = 0;
        for hub_group in self.groups.values_mut() {
            let before = hub_group.message_history.len();
            hub_group.message_history.retain(|msg| msg.sent_at >= cutoff);
            total += before - hub_group.message_history.len();
        }
        total
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
                invite_only,
            } => self.handle_create(from, group_name, creator_username, initial_members, invite_only),

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
                epoch,
                ref encrypted_keys,
            } => self.handle_sender_key_distribution(from, group_id, epoch, encrypted_keys),

            GroupPayload::HubPing { ref group_id } => self.handle_hub_ping(group_id, from),

            // Admin controls (R11.3)
            GroupPayload::KickMember {
                ref group_id,
                target_id,
            } => self.kick_member(group_id, &from, &target_id),

            GroupPayload::UpdateMemberRole {
                ref group_id,
                target_id,
                new_role,
            } => self.update_member_role(group_id, &from, &target_id, new_role),

            GroupPayload::InviteMember {
                ref group_id,
                target_id,
            } => self.invite_member(group_id, &from, target_id),

            // Hub doesn't process these (they're outgoing from hub or failover-specific)
            GroupPayload::Created { .. }
            | GroupPayload::Invite { .. }
            | GroupPayload::Sync { .. }
            | GroupPayload::MemberJoined { .. }
            | GroupPayload::MemberLeft { .. }
            | GroupPayload::MemberRoleChanged { .. }
            | GroupPayload::HubMigration { .. }
            | GroupPayload::HubHeartbeat { .. }
            | GroupPayload::HubPong { .. }
            | GroupPayload::HubShadowSync { .. }
            | GroupPayload::CandidateAssigned { .. }
            | GroupPayload::HubUnreachable { .. }
            // SyncRequest/SyncResponse handled by runtime, not hub
            | GroupPayload::SyncRequest { .. }
            | GroupPayload::SyncResponse { .. } => vec![],
        }
    }

    // ── Group Creation ───────────────────────────────────────────────────

    fn handle_create(
        &mut self,
        creator: NodeId,
        group_name: String,
        creator_username: String,
        initial_members: Vec<NodeId>,
        invite_only: bool,
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
            invite_only,
        };

        // Build invited set from initial members (for invite-only enforcement)
        let invited_set: HashSet<NodeId> = initial_members.iter().copied().collect();

        let hub_group = HubGroup {
            info: info.clone(),
            message_history: VecDeque::new(),
            next_seq: 0,
            rate_limits: HashMap::new(),
            seen_message_ids: HashSet::new(),
            seen_nonces: HashSet::new(),
            invited_set,
            latest_sender_keys: HashMap::new(),
            sender_epoch_state: HashMap::new(),
            group_msg_since_rotation: 0,
            last_rotation_trigger_ms: 0,
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

        // Already a member? Re-sync them (they may have restarted).
        if hub_group.info.is_member(&joiner) {
            let recent: Vec<GroupMessage> = hub_group.message_history.iter().cloned().collect();
            return vec![GroupAction::Send {
                to: joiner,
                payload: GroupPayload::Sync {
                    group: hub_group.info.clone(),
                    recent_messages: recent,
                },
            }];
        }

        // Invite-only: reject uninvited joiners (creator always allowed)
        if hub_group.info.invite_only
            && joiner != hub_group.info.created_by
            && !hub_group.invited_set.contains(&joiner)
        {
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
        hub_group.invited_set.remove(&joiner);

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

        // Proactive sender-key replay for rejoining member (R14 fallback support).
        for (sender_id, (epoch, encrypted_keys)) in &hub_group.latest_sender_keys {
            if let Some(ek) = encrypted_keys.iter().find(|ek| ek.recipient_id == joiner) {
                actions.push(GroupAction::Send {
                    to: joiner,
                    payload: GroupPayload::SenderKeyDistribution {
                        group_id: group_id.clone(),
                        from: *sender_id,
                        epoch: *epoch,
                        encrypted_keys: vec![ek.clone()],
                    },
                });
            }
        }

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

    fn handle_message(&mut self, from: NodeId, mut msg: GroupMessage) -> Vec<GroupAction> {
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

        // Epoch fallback policy (R14.2): accept epoch-1 during grace, reject after.
        if msg.encrypted {
            match self.check_epoch_policy(&group_id, &from, msg.key_epoch) {
                EpochDecision::Accept => {}
                EpochDecision::Reject => {
                    let mut actions = vec![GroupAction::Event(GroupEvent::SecurityViolation {
                        group_id: group_id.clone(),
                        node_id: from,
                        reason: "sender key epoch mismatch (grace expired), re-sync required".into(),
                    })];
                    actions.extend(self.build_proactive_sender_key_replay(&group_id, from));
                    return actions;
                }
            }
        }

        // Rate limit check (mutable borrow scoped)
        if !self.check_rate_limit(&group_id, &from) {
            return vec![];
        }

        // Dedup check (mutable borrow scoped)
        if !self.check_dedup(&group_id, &message_id) {
            return vec![];
        }

        // Assign monotonic sequence number and store
        let recipients = {
            let hub_group = self.groups.get_mut(&group_id).unwrap();
            hub_group.info.last_activity_at = now_ms();

            // Assign hub sequence number (immutable per group, monotonically increasing)
            msg.seq = hub_group.next_seq;
            hub_group.next_seq += 1;

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

        let mut actions = Vec::new();
        if !recipients.is_empty() {
            actions.push(GroupAction::Broadcast {
                to: recipients,
                payload: GroupPayload::Message(msg),
            });
        }
        actions.extend(self.maybe_trigger_rotation(&group_id));
        actions
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
        &mut self,
        from: NodeId,
        group_id: &GroupId,
        epoch: u32,
        encrypted_keys: &[EncryptedSenderKey],
    ) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        if !hub_group.info.is_member(&from) {
            return vec![GroupAction::Event(GroupEvent::SecurityViolation {
                group_id: group_id.clone(),
                node_id: from,
                reason: "non-member sent sender key distribution".into(),
            })];
        }

        // Cache latest distribution + epoch transition state.
        if let Some(state) = hub_group.sender_epoch_state.get_mut(&from) {
            if epoch > state.current_epoch {
                state.previous_epoch = Some(state.current_epoch);
                state.current_epoch = epoch;
                state.grace_until_ms = now_ms().saturating_add(SENDER_KEY_EPOCH_GRACE_MS);
            }
        } else {
            hub_group.sender_epoch_state.insert(
                from,
                SenderEpochState {
                    current_epoch: epoch,
                    previous_epoch: None,
                    grace_until_ms: 0,
                },
            );
        }
        hub_group
            .latest_sender_keys
            .insert(from, (epoch, encrypted_keys.to_vec()));

        let mut actions = Vec::new();
        for ek in encrypted_keys {
            if hub_group.info.is_member(&ek.recipient_id) {
                actions.push(GroupAction::Send {
                    to: ek.recipient_id,
                    payload: GroupPayload::SenderKeyDistribution {
                        group_id: group_id.clone(),
                        from,
                        epoch,
                        encrypted_keys: vec![ek.clone()],
                    },
                });
            }
        }
        actions
    }

    fn check_epoch_policy(
        &self,
        group_id: &GroupId,
        sender: &NodeId,
        msg_epoch: u32,
    ) -> EpochDecision {
        let Some(hub_group) = self.groups.get(group_id) else {
            return EpochDecision::Reject;
        };
        let Some(state) = hub_group.sender_epoch_state.get(sender) else {
            return EpochDecision::Accept;
        };

        if msg_epoch == state.current_epoch {
            return EpochDecision::Accept;
        }
        if state.previous_epoch == Some(msg_epoch) && now_ms() <= state.grace_until_ms {
            return EpochDecision::Accept;
        }
        EpochDecision::Reject
    }

    fn build_proactive_sender_key_replay(&self, group_id: &GroupId, target: NodeId) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get(group_id) else {
            return vec![];
        };

        let mut actions = Vec::new();
        for (sender_id, (epoch, encrypted_keys)) in &hub_group.latest_sender_keys {
            if let Some(ek) = encrypted_keys.iter().find(|ek| ek.recipient_id == target) {
                actions.push(GroupAction::Send {
                    to: target,
                    payload: GroupPayload::SenderKeyDistribution {
                        group_id: group_id.clone(),
                        from: *sender_id,
                        epoch: *epoch,
                        encrypted_keys: vec![ek.clone()],
                    },
                });
            }
        }
        actions
    }

    fn maybe_trigger_rotation(&mut self, group_id: &GroupId) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![];
        };
        if hub_group.last_rotation_trigger_ms == 0 {
            hub_group.last_rotation_trigger_ms = now_ms();
        }
        hub_group.group_msg_since_rotation = hub_group.group_msg_since_rotation.saturating_add(1);
        let now = now_ms();
        let elapsed = now.saturating_sub(hub_group.last_rotation_trigger_ms);

        let should_trigger = hub_group.group_msg_since_rotation >= SENDER_KEY_ROTATE_MAX_MESSAGES
            || elapsed >= SENDER_KEY_ROTATE_MAX_AGE_MS;
        if !should_trigger {
            return vec![];
        }

        // Anti-spam: max one trigger per hour.
        if hub_group.last_rotation_trigger_ms > 0 && elapsed < SENDER_KEY_ROTATE_RATE_LIMIT_MS {
            return vec![];
        }

        hub_group.last_rotation_trigger_ms = now;
        hub_group.group_msg_since_rotation = 0;

        let group_members = hub_group.info.members.clone();
        let latest_sender_keys = hub_group.latest_sender_keys.clone();

        let mut actions = Vec::new();
        for (sender_id, (epoch, encrypted_keys)) in latest_sender_keys {
            for ek in encrypted_keys {
                if group_members.iter().any(|m| m.node_id == ek.recipient_id) {
                    actions.push(GroupAction::Send {
                        to: ek.recipient_id,
                        payload: GroupPayload::SenderKeyDistribution {
                            group_id: group_id.clone(),
                            from: sender_id,
                            epoch,
                            encrypted_keys: vec![ek],
                        },
                    });
                }
            }
        }
        actions
    }

    /// Purge sender key cache entries older than max age (e.g. >7 days).
    /// Returns number of purged sender entries.
    pub fn purge_expired_sender_keys(&mut self, now_ms: u64, max_age_ms: u64) -> usize {
        let cutoff = now_ms.saturating_sub(max_age_ms);
        let mut purged = 0usize;

        for hub_group in self.groups.values_mut() {
            let before_cache = hub_group.latest_sender_keys.len();
            if hub_group.info.last_activity_at < cutoff {
                hub_group.latest_sender_keys.clear();
                hub_group.sender_epoch_state.clear();
            }
            purged += before_cache.saturating_sub(hub_group.latest_sender_keys.len());
        }

        purged
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

        // Derive next_seq from imported messages (continue from highest seq).
        let next_seq = messages.iter().map(|m| m.seq).max().map_or(0, |s| s + 1);

        let hub_group = HubGroup {
            info,
            message_history: messages.into_iter().collect(),
            next_seq,
            rate_limits: HashMap::new(),
            seen_message_ids: HashSet::new(),
            seen_nonces: HashSet::new(),
            invited_set: HashSet::new(),
            latest_sender_keys: HashMap::new(),
            sender_epoch_state: HashMap::new(),
            group_msg_since_rotation: 0,
            last_rotation_trigger_ms: 0,
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

    // ── Update Member Role (R11.3) ──────────────────────────────────────

    /// Change a member's role (admin action).
    ///
    /// Last-admin protection: cannot demote the only admin.
    pub fn update_member_role(
        &mut self,
        group_id: &GroupId,
        admin: &NodeId,
        target: &NodeId,
        new_role: GroupMemberRole,
    ) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        // Only admins can change roles
        if !hub_group.info.is_admin(admin) {
            return vec![];
        }

        // Target must be a member
        let Some(member) = hub_group.info.members.iter().find(|m| m.node_id == *target) else {
            return vec![];
        };

        // No-op if already the target role
        if member.role == new_role {
            return vec![];
        }

        // Last-admin protection: can't demote the only admin
        if new_role == GroupMemberRole::Member {
            let admin_count = hub_group
                .info
                .members
                .iter()
                .filter(|m| m.role == GroupMemberRole::Admin)
                .count();
            if admin_count <= 1 {
                return vec![];
            }
        }

        // Apply the role change
        if let Some(m) = hub_group.info.members.iter_mut().find(|m| m.node_id == *target) {
            m.role = new_role;
        }
        hub_group.info.last_activity_at = now_ms();

        // Broadcast to all members
        let recipients: Vec<NodeId> = hub_group
            .info
            .members
            .iter()
            .map(|m| m.node_id)
            .collect();

        let mut actions = vec![GroupAction::Broadcast {
            to: recipients,
            payload: GroupPayload::MemberRoleChanged {
                group_id: group_id.clone(),
                node_id: *target,
                new_role,
            },
        }];

        // Sync shadow
        let group_id_for_sync = group_id.clone();
        if let Some((target_node, payload)) = self.build_shadow_sync(&group_id_for_sync) {
            actions.push(GroupAction::Send { to: target_node, payload });
        }

        actions
    }

    // ── Invite Member (R11.3) ─────────────────────────────────────────

    /// Admin invites a new member to an existing group.
    pub fn invite_member(
        &mut self,
        group_id: &GroupId,
        admin: &NodeId,
        target: NodeId,
    ) -> Vec<GroupAction> {
        let Some(hub_group) = self.groups.get_mut(group_id) else {
            return vec![];
        };

        // Only admins can invite
        if !hub_group.info.is_admin(admin) {
            return vec![];
        }

        // Already a member
        if hub_group.info.is_member(&target) {
            return vec![];
        }

        // Group full
        if hub_group.info.is_full() {
            return vec![];
        }

        // Track invitation (for invite-only enforcement)
        hub_group.invited_set.insert(target);

        // Get admin username for the invite
        let admin_username = hub_group
            .info
            .get_member(admin)
            .map(|m| m.username.clone())
            .unwrap_or_default();

        vec![GroupAction::Send {
            to: target,
            payload: GroupPayload::Invite {
                group_id: group_id.clone(),
                group_name: hub_group.info.name.clone(),
                inviter_id: *admin,
                inviter_username: admin_username,
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

    // ── Persistence ──────────────────────────────────────────────────────

    /// Extract a serializable snapshot of persistent state.
    ///
    /// Ephemeral state (rate limits, dedup sets, nonces) is excluded.
    pub fn snapshot(&self) -> GroupHubSnapshot {
        GroupHubSnapshot {
            groups: self.groups.iter().map(|(id, hg)| {
                (id.clone(), hg.info.clone())
            }).collect(),
            invited_sets: self.groups.iter()
                .filter(|(_, hg)| !hg.invited_set.is_empty())
                .map(|(id, hg)| (id.clone(), hg.invited_set.clone()))
                .collect(),
            next_seqs: self.groups.iter()
                .map(|(id, hg)| (id.clone(), hg.next_seq))
                .collect(),
        }
    }

    /// Restore persistent state from a snapshot.
    pub fn restore(&mut self, snapshot: GroupHubSnapshot) {
        for (group_id, info) in snapshot.groups {
            let invited_set = snapshot.invited_sets
                .get(&group_id)
                .cloned()
                .unwrap_or_default();
            let next_seq = snapshot.next_seqs
                .get(&group_id)
                .copied()
                .unwrap_or(0);
            let hub_group = HubGroup {
                info,
                message_history: VecDeque::new(),
                next_seq,
                rate_limits: HashMap::new(),
                seen_message_ids: HashSet::new(),
                seen_nonces: HashSet::new(),
                invited_set,
                latest_sender_keys: HashMap::new(),
                sender_epoch_state: HashMap::new(),
                group_msg_since_rotation: 0,
                last_rotation_trigger_ms: 0,
            };
            self.groups.insert(group_id, hub_group);
        }
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
    fn join_duplicate_triggers_resync() {
        let mut hub = make_hub();
        let alice = node_id(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        // Alice tries to join again (e.g., after restart) — should get re-sync
        let actions = hub.handle_join(alice, &gid, "alice".into());
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Send { to, payload } => {
                assert_eq!(*to, alice);
                assert!(matches!(payload, GroupPayload::Sync { .. }));
            }
            other => panic!("expected Send(Sync), got: {other:?}"),
        }
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
            seq: 0,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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

        // Epoch should be forwarded as-is (no hardcoded 0).
        for action in &actions {
            let GroupAction::Send {
                payload: GroupPayload::SenderKeyDistribution { epoch, .. },
                ..
            } = action
            else {
                panic!("expected SenderKeyDistribution action");
            };
            assert_eq!(*epoch, 1);
        }
    }

    #[test]
    fn epoch_mismatch_after_grace_rejected_and_replayed() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "R14".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Cache epoch 1 then epoch 2 distribution for alice.
        let _ = hub.handle_payload(
            GroupPayload::SenderKeyDistribution {
                group_id: gid.clone(),
                from: alice,
                epoch: 1,
                encrypted_keys: vec![
                    EncryptedSenderKey {
                        recipient_id: bob,
                        encrypted_key: crate::crypto::EncryptedPayload {
                            ciphertext: vec![1],
                            nonce: [0u8; 24],
                            ephemeral_pk: [0u8; 32],
                        },
                    },
                    EncryptedSenderKey {
                        recipient_id: alice,
                        encrypted_key: crate::crypto::EncryptedPayload {
                            ciphertext: vec![9],
                            nonce: [0u8; 24],
                            ephemeral_pk: [0u8; 32],
                        },
                    },
                ],
            },
            alice,
        );
        let _ = hub.handle_payload(
            GroupPayload::SenderKeyDistribution {
                group_id: gid.clone(),
                from: alice,
                epoch: 2,
                encrypted_keys: vec![
                    EncryptedSenderKey {
                        recipient_id: bob,
                        encrypted_key: crate::crypto::EncryptedPayload {
                            ciphertext: vec![2],
                            nonce: [0u8; 24],
                            ephemeral_pk: [0u8; 32],
                        },
                    },
                    EncryptedSenderKey {
                        recipient_id: alice,
                        encrypted_key: crate::crypto::EncryptedPayload {
                            ciphertext: vec![8],
                            nonce: [0u8; 24],
                            ephemeral_pk: [0u8; 32],
                        },
                    },
                ],
            },
            alice,
        );

        // Force grace expiry in internal state for deterministic test.
        if let Some(g) = hub.groups.get_mut(&gid) {
            if let Some(st) = g.sender_epoch_state.get_mut(&alice) {
                st.grace_until_ms = 0;
            }
        }

        let (_, alice_secret) = keypair(1);
        let mut stale = GroupMessage::new_encrypted(
            gid.clone(),
            alice,
            "alice".into(),
            "stale epoch".into(),
            &[7u8; 32],
            1,
        );
        stale.sign(&alice_secret);

        let actions = hub.handle_message(alice, stale);
        assert!(!actions.is_empty());
        let has_reject = actions.iter().any(|a| {
            matches!(
                a,
                GroupAction::Event(GroupEvent::SecurityViolation { reason, .. })
                if reason.contains("grace expired")
            )
        });
        let has_replay = actions.iter().any(|a| {
            matches!(
                a,
                GroupAction::Send {
                    to,
                    payload: GroupPayload::SenderKeyDistribution { .. }
                } if *to == alice
            )
        });
        assert!(has_reject, "expected epoch mismatch rejection");
        assert!(has_replay, "expected proactive sender-key replay to sender");
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
                invite_only: false,
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
            seq: 0,
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
            seq: 0,
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
                invite_only: false,
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

    // ── R11.3 Admin Controls Tests ────────────────────────────────────

    #[test]
    fn kick_member_via_payload() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Kick via payload (wire protocol path)
        let actions = hub.handle_payload(
            GroupPayload::KickMember {
                group_id: gid.clone(),
                target_id: bob,
            },
            alice,
        );

        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            GroupAction::Broadcast { payload: GroupPayload::MemberLeft { reason: LeaveReason::Kicked, .. }, .. }
        ));
        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 1);
    }

    #[test]
    fn update_member_role_promote() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Promote bob to Admin
        let actions = hub.update_member_role(&gid, &alice, &bob, GroupMemberRole::Admin);

        // Should broadcast MemberRoleChanged to all members
        assert!(!actions.is_empty());
        match &actions[0] {
            GroupAction::Broadcast { to, payload } => {
                assert!(to.contains(&alice));
                assert!(to.contains(&bob));
                assert!(matches!(
                    payload,
                    GroupPayload::MemberRoleChanged { new_role: GroupMemberRole::Admin, .. }
                ));
            }
            _ => panic!("expected Broadcast"),
        }

        // Verify role changed
        let group = hub.get_group(&gid).unwrap();
        assert!(group.is_admin(&bob));
    }

    #[test]
    fn update_member_role_demote() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // First promote bob so we have 2 admins
        hub.update_member_role(&gid, &alice, &bob, GroupMemberRole::Admin);
        assert!(hub.get_group(&gid).unwrap().is_admin(&bob));

        // Now demote bob back to Member
        let actions = hub.update_member_role(&gid, &alice, &bob, GroupMemberRole::Member);
        assert!(!actions.is_empty());
        match &actions[0] {
            GroupAction::Broadcast { payload, .. } => {
                assert!(matches!(
                    payload,
                    GroupPayload::MemberRoleChanged { new_role: GroupMemberRole::Member, .. }
                ));
            }
            _ => panic!("expected Broadcast"),
        }
        assert!(!hub.get_group(&gid).unwrap().is_admin(&bob));
    }

    #[test]
    fn update_member_role_last_admin_rejected() {
        let mut hub = make_hub();
        let alice = node_id(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        // Alice is the only admin — can't demote self
        let actions = hub.update_member_role(&gid, &alice, &alice, GroupMemberRole::Member);
        assert!(actions.is_empty(), "should reject demotion of last admin");
        assert!(hub.get_group(&gid).unwrap().is_admin(&alice));
    }

    #[test]
    fn invite_member_sends_invite() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Test".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        // Admin invites bob
        let actions = hub.invite_member(&gid, &alice, bob);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GroupAction::Send { to, payload } => {
                assert_eq!(*to, bob);
                assert!(matches!(payload, GroupPayload::Invite { .. }));
            }
            _ => panic!("expected Send(Invite)"),
        }

        // Bob should be in invited_set
        assert!(hub.groups.get(&gid).unwrap().invited_set.contains(&bob));
    }

    #[test]
    fn invite_only_rejects_uninvited_join() {
        let mut hub = make_hub();
        let alice = node_id(1);
        let bob = node_id(2);
        let charlie = node_id(3);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "Private".into(),
                creator_username: "alice".into(),
                initial_members: vec![bob],
                invite_only: true,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        // Bob was in initial_members → should be in invited_set → can join
        let actions = hub.handle_join(bob, &gid, "bob".into());
        assert!(!actions.is_empty(), "invited bob should join");
        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 2);

        // Charlie was NOT invited → should be rejected
        let actions = hub.handle_join(charlie, &gid, "charlie".into());
        assert!(actions.is_empty(), "uninvited charlie should be rejected");
        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 2);

        // Now invite charlie, then he can join
        hub.invite_member(&gid, &alice, charlie);
        let actions = hub.handle_join(charlie, &gid, "charlie".into());
        assert!(!actions.is_empty(), "invited charlie should join");
        assert_eq!(hub.get_group(&gid).unwrap().member_count(), 3);
    }

    // ── R13.1: Sequence number tests ──────────────────────────────────

    #[test]
    fn hub_assigns_monotonic_seq() {
        let mut hub = make_hub();
        let (alice, alice_secret) = keypair(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "SeqTest".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Send 5 messages, check seq increments 0,1,2,3,4
        for expected_seq in 0u64..5 {
            let mut msg = GroupMessage::new(
                gid.clone(), alice, "alice".into(), format!("msg-{expected_seq}"),
            );
            msg.sign(&alice_secret);
            let actions = hub.handle_message(alice, msg);
            assert_eq!(actions.len(), 1);

            // Extract the broadcasted message and check seq
            match &actions[0] {
                GroupAction::Broadcast { payload: GroupPayload::Message(m), .. } => {
                    assert_eq!(m.seq, expected_seq, "seq should be {expected_seq}");
                }
                other => panic!("expected Broadcast Message, got: {other:?}"),
            }
        }

        // Verify next_seq counter
        assert_eq!(hub.groups[&gid].next_seq, 5);
    }

    #[test]
    fn seq_survives_snapshot_restore() {
        let mut hub = make_hub();
        let (alice, alice_secret) = keypair(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "PersistSeq".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Send 3 messages (seq 0,1,2)
        for _ in 0..3 {
            let mut msg = GroupMessage::new(gid.clone(), alice, "alice".into(), "x".into());
            msg.sign(&alice_secret);
            hub.handle_message(alice, msg);
        }
        assert_eq!(hub.groups[&gid].next_seq, 3);

        // Snapshot and restore
        let snapshot = hub.snapshot();
        assert_eq!(snapshot.next_seqs[&gid], 3);

        let mut hub2 = GroupHub::new(alice);
        hub2.restore(snapshot);
        assert_eq!(hub2.groups[&gid].next_seq, 3);

        // Next message should get seq 3
        let mut msg = GroupMessage::new(gid.clone(), alice, "alice".into(), "after-restore".into());
        msg.sign(&alice_secret);
        let actions = hub2.handle_message(alice, msg);
        match &actions[0] {
            GroupAction::Broadcast { payload: GroupPayload::Message(m), .. } => {
                assert_eq!(m.seq, 3);
            }
            other => panic!("expected Broadcast Message, got: {other:?}"),
        }
    }

    #[test]
    fn import_group_derives_next_seq_from_history() {
        let mut hub = GroupHub::new(node_id(1));

        let info = GroupInfo {
            group_id: GroupId::from("grp-import".to_string()),
            name: "Imported".into(),
            created_by: node_id(2),
            created_at: 1000,
            hub_relay_id: node_id(1),
            backup_hub_id: None,
            members: vec![],
            max_members: 50,
            last_activity_at: 2000,
            shadow_id: None,
            candidate_id: None,
            invite_only: false,
        };

        let mut messages = vec![];
        for i in 0..3 {
            let mut m = GroupMessage::new(
                info.group_id.clone(), node_id(2), "bob".into(), format!("imported-{i}"),
            );
            m.seq = i as u64 + 10; // simulate existing seq 10,11,12
            messages.push(m);
        }

        hub.import_group(info, messages);
        let gid = GroupId::from("grp-import".to_string());
        assert_eq!(hub.groups[&gid].next_seq, 13, "next_seq should be max(10,11,12)+1 = 13");
    }

    #[test]
    fn message_history_returns_correct_messages() {
        let mut hub = make_hub();
        let (alice, alice_secret) = keypair(1);
        let bob = node_id(2);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "HistTest".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();
        hub.handle_join(bob, &gid, "bob".into());

        // Send 5 signed messages through the hub
        for i in 0..5 {
            let mut msg = GroupMessage::new(
                gid.clone(), alice, "alice".into(), format!("msg-{i}"),
            );
            msg.sign(&alice_secret);
            let _ = hub.handle_message(alice, msg);
        }

        // Verify message_history contains 5 messages with seq 0..4
        let history = hub.message_history(&gid).unwrap();
        assert_eq!(history.len(), 5);
        for (i, msg) in history.iter().enumerate() {
            assert_eq!(msg.seq, i as u64);
        }
    }

    #[test]
    fn cleanup_expired_messages_removes_old() {
        let mut hub = make_hub();
        let (alice, _alice_secret) = keypair(1);

        hub.handle_payload(
            GroupPayload::Create {
                group_name: "CleanupTest".into(),
                creator_username: "alice".into(),
                initial_members: vec![],
                invite_only: false,
            },
            alice,
        );
        let gid = hub.groups.keys().next().unwrap().clone();

        // Import messages with controlled sent_at timestamps
        let mut messages = vec![];
        for i in 0..5 {
            let mut m = GroupMessage::new(
                gid.clone(), alice, "alice".into(), format!("msg-{i}"),
            );
            m.seq = i as u64;
            // First 3 messages are "old" (25h ago), last 2 are "recent" (1h ago)
            if i < 3 {
                m.sent_at = 1000; // very old
            } else {
                m.sent_at = 100_000_000; // recent
            }
            messages.push(m);
        }

        // Replace with known history
        hub.groups.get_mut(&gid).unwrap().message_history = messages.into_iter().collect();
        assert_eq!(hub.message_history(&gid).unwrap().len(), 5);

        // Cleanup with cutoff that keeps only messages with sent_at >= 50_000_000
        let now = 100_000_000 + 1000;
        let max_age = 50_000_001; // cutoff = now - max_age = 50_000_999
        let purged = hub.cleanup_expired_messages(now, max_age);
        assert_eq!(purged, 3, "should purge 3 old messages");
        assert_eq!(hub.message_history(&gid).unwrap().len(), 2);
    }
}
