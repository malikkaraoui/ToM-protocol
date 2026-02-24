# Hub Failover Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Automatic hub failover with virus-like replication chain (Primary → Shadow → Candidate) and active watchdog detection (3-8s).

**Architecture:** The hub replicates itself across 3 nodes in a cascade chain. The shadow actively pings the primary every 3s; if 2 pings fail, it self-promotes. Members also report unreachable hubs, accelerating detection. The hub is a stateless pass-through — only member list + config are replicated.

**Tech Stack:** Rust, tom-protocol crate, MessagePack serialization, deterministic election (existing `elect_hub`)

---

### Task 1: New payload types and constants

**Files:**
- Modify: `crates/tom-protocol/src/group/types.rs:10-28` (constants) and `:155-242` (GroupPayload enum)
- Modify: `crates/tom-protocol/src/types.rs:7-37` (MessageType enum)
- Modify: `crates/tom-protocol/src/runtime/state.rs:29-44` (group_payload_to_message_type)

**Step 1: Write failing test for new payload serialization roundtrip**

Add at the bottom of the `#[cfg(test)] mod tests` block in `crates/tom-protocol/src/group/types.rs` (after the existing `sender_key_distribution_roundtrip` test):

```rust
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tom-protocol hub_shadow_sync_roundtrip -- --nocapture 2>&1 | head -30`
Expected: FAIL — `HubShadowSync` variant doesn't exist yet.

**Step 3: Add constants and payload variants**

In `crates/tom-protocol/src/group/types.rs`, add after the existing constants (after line 28):

```rust
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
```

In the `GroupPayload` enum (after the existing `SenderKeyDistribution` variant, before the closing `}`), add:

```rust
/// Shadow watchdog ping (shadow → primary).
HubPing { group_id: GroupId },

/// Primary response to shadow ping (primary → shadow).
HubPong { group_id: GroupId },

/// State sync from primary to shadow (primary → shadow).
HubShadowSync {
    group_id: GroupId,
    members: Vec<GroupMember>,
    candidate_id: Option<NodeId>,
    config_version: u64,
},

/// Candidate role assignment (shadow → candidate).
CandidateAssigned { group_id: GroupId },

/// Member reports hub unreachable (member → shadow).
HubUnreachable { group_id: GroupId },
```

In `crates/tom-protocol/src/types.rs`, add to the `MessageType` enum (after `GroupSenderKeyDistribution`):

```rust
GroupHubPing,
GroupHubPong,
GroupHubShadowSync,
GroupCandidateAssigned,
GroupHubUnreachable,
```

In `crates/tom-protocol/src/runtime/state.rs`, update `group_payload_to_message_type` (after the `SenderKeyDistribution` arm):

```rust
GroupPayload::HubPing { .. } => MessageType::GroupHubPing,
GroupPayload::HubPong { .. } => MessageType::GroupHubPong,
GroupPayload::HubShadowSync { .. } => MessageType::GroupHubShadowSync,
GroupPayload::CandidateAssigned { .. } => MessageType::GroupCandidateAssigned,
GroupPayload::HubUnreachable { .. } => MessageType::GroupHubUnreachable,
```

Also update `handle_payload` in `hub.rs` to add the new variants to the ignored arm (line ~103-110):

```rust
| GroupPayload::HubPing { .. }
| GroupPayload::HubPong { .. }
| GroupPayload::HubShadowSync { .. }
| GroupPayload::CandidateAssigned { .. }
| GroupPayload::HubUnreachable { .. } => vec![],
```

Also update `MessageType` roundtrip test in `types.rs` to include the new variants.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p tom-protocol -- hub_shadow_sync_roundtrip hub_ping_pong_roundtrip hub_unreachable_roundtrip candidate_assigned_roundtrip --nocapture 2>&1 | tail -20`
Expected: 4 tests PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/group/types.rs crates/tom-protocol/src/types.rs crates/tom-protocol/src/runtime/state.rs crates/tom-protocol/src/group/hub.rs
git commit -m "feat(failover): add hub failover payload types and constants"
```

---

### Task 2: Hub-side shadow management (assign shadow, respond to pings, send sync)

**Files:**
- Modify: `crates/tom-protocol/src/group/hub.rs` (GroupHub methods)
- Modify: `crates/tom-protocol/src/group/types.rs:91-103` (GroupInfo — add `shadow_id`, `candidate_id`)

**Step 1: Write failing tests**

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/tom-protocol/src/group/hub.rs`:

```rust
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

    let sync = hub.build_shadow_sync(&gid);
    assert!(sync.is_some());
    let (target, payload) = sync.unwrap();
    if let GroupPayload::HubShadowSync { members, .. } = &payload {
        // Should contain creator + bob
        assert!(members.len() >= 2);
    } else {
        panic!("expected HubShadowSync");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tom-protocol assign_shadow_on_group_create -- --nocapture 2>&1 | head -20`
Expected: FAIL — methods don't exist.

**Step 3: Implement hub-side shadow management**

Add fields to `GroupInfo` in `crates/tom-protocol/src/group/types.rs` (after `max_members` field):

```rust
/// Current shadow node (virus replication).
#[serde(default)]
pub shadow_id: Option<NodeId>,
/// Current candidate node (next shadow).
#[serde(default)]
pub candidate_id: Option<NodeId>,
```

Add methods to `GroupHub` in `crates/tom-protocol/src/group/hub.rs` (before the `// ── Rate Limiting` section):

```rust
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

/// Handle a HubPing from the shadow — respond with HubPong.
pub fn handle_hub_ping(&self, group_id: &GroupId, from: NodeId) -> Vec<GroupAction> {
    let Some(hub_group) = self.groups.get(group_id) else {
        return vec![];
    };
    // Only respond to the actual shadow
    if hub_group.info.shadow_id != Some(from) {
        return vec![];
    }
    vec![GroupAction::Send {
        to: from,
        payload: GroupPayload::HubPong {
            group_id: group_id.clone(),
        },
    }]
}
```

Update `handle_payload` to route HubPing to `handle_hub_ping` instead of ignoring it. Replace the ignored arm for `HubPing`:

```rust
GroupPayload::HubPing { ref group_id } => self.handle_hub_ping(group_id, from),
```

Also, in the existing `handle_join` and `handle_leave` methods, add shadow sync after member changes:
After the existing join/leave actions, append:

```rust
// After join: sync shadow if assigned
if let Some((target, payload)) = self.build_shadow_sync(&group_id) {
    actions.push(GroupAction::Send { to: target, payload });
}
```

**Step 4: Run tests**

Run: `cargo test -p tom-protocol -- assign_shadow hub_responds_pong shadow_sync_contains --nocapture 2>&1 | tail -20`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/group/hub.rs crates/tom-protocol/src/group/types.rs
git commit -m "feat(failover): hub-side shadow management — assign, ping/pong, sync"
```

---

### Task 3: Member-side shadow role (watchdog, promotion logic)

**Files:**
- Modify: `crates/tom-protocol/src/group/manager.rs`

**Step 1: Write failing tests**

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/tom-protocol/src/group/manager.rs`:

```rust
#[test]
fn handle_shadow_sync_stores_state() {
    let shadow_id = node_id(2);
    let mut mgr = GroupManager::new(shadow_id, "shadow".into());
    let hub = node_id(10);
    let group = make_test_group(node_id(1), hub);
    let gid = group.group_id.clone();
    mgr.handle_group_created(group);

    let actions = mgr.handle_shadow_sync(
        &gid,
        vec![
            GroupMember {
                node_id: node_id(1),
                username: "alice".into(),
                joined_at: 1000,
                role: GroupMemberRole::Member,
            },
            GroupMember {
                node_id: shadow_id,
                username: "shadow".into(),
                joined_at: 1000,
                role: GroupMemberRole::Member,
            },
        ],
        Some(node_id(3)),
        1,
    );

    assert!(mgr.is_shadow_for(&gid));
    assert!(actions.is_empty()); // sync is silent, no events
}

#[test]
fn shadow_promotes_on_ping_failures() {
    let shadow_id = node_id(2);
    let mut mgr = GroupManager::new(shadow_id, "shadow".into());
    let hub = node_id(10);
    let group = make_test_group(node_id(1), hub);
    let gid = group.group_id.clone();
    mgr.handle_group_created(group);
    mgr.handle_shadow_sync(
        &gid,
        vec![GroupMember {
            node_id: node_id(1),
            username: "alice".into(),
            joined_at: 1000,
            role: GroupMemberRole::Member,
        }],
        Some(node_id(3)),
        1,
    );

    // Simulate 2 ping failures
    let actions1 = mgr.record_ping_failure(&gid);
    assert!(actions1.is_empty(), "1 failure should not promote");

    let actions2 = mgr.record_ping_failure(&gid);
    assert!(!actions2.is_empty(), "2 failures should trigger promotion");

    // Should contain HubMigration broadcast
    let has_migration = actions2
        .iter()
        .any(|a| matches!(a, GroupAction::Broadcast { payload: GroupPayload::HubMigration { .. }, .. }));
    assert!(has_migration, "promotion should broadcast HubMigration");

    // Should no longer be shadow (now we're the hub)
    assert!(!mgr.is_shadow_for(&gid));
}

#[test]
fn hub_unreachable_accelerates_promotion() {
    let shadow_id = node_id(2);
    let mut mgr = GroupManager::new(shadow_id, "shadow".into());
    let hub = node_id(10);
    let group = make_test_group(node_id(1), hub);
    let gid = group.group_id.clone();
    mgr.handle_group_created(group);
    mgr.handle_shadow_sync(
        &gid,
        vec![GroupMember {
            node_id: node_id(1),
            username: "alice".into(),
            joined_at: 1000,
            role: GroupMemberRole::Member,
        }],
        Some(node_id(3)),
        1,
    );

    // 1 ping failure + 1 HubUnreachable = promotion
    mgr.record_ping_failure(&gid);
    let actions = mgr.handle_hub_unreachable(&gid, node_id(1));

    let has_migration = actions
        .iter()
        .any(|a| matches!(a, GroupAction::Broadcast { payload: GroupPayload::HubMigration { .. }, .. }));
    assert!(has_migration, "unreachable + 1 ping failure should promote");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tom-protocol handle_shadow_sync_stores_state -- --nocapture 2>&1 | head -20`
Expected: FAIL — methods don't exist.

**Step 3: Implement shadow role in GroupManager**

Add fields to `GroupManager` struct in `crates/tom-protocol/src/group/manager.rs` (after `pending_decrypt`):

```rust
/// Groups where we are the shadow (group_id → ShadowState).
shadow_state: HashMap<GroupId, ShadowState>,
```

Add the `ShadowState` struct before the `GroupManager` struct:

```rust
/// State for a group where we are the shadow.
#[derive(Debug)]
struct ShadowState {
    /// Synchronized member list from primary.
    members: Vec<GroupMember>,
    /// Candidate node id.
    candidate_id: Option<NodeId>,
    /// Config version from primary.
    config_version: u64,
    /// Consecutive ping failures.
    ping_failures: u32,
    /// Number of HubUnreachable reports received.
    unreachable_reports: u32,
}
```

Initialize `shadow_state: HashMap::new()` in `GroupManager::new()`.

Add methods:

```rust
// ── Shadow Role ──────────────────────────────────────────────────────

/// Are we the shadow for this group?
pub fn is_shadow_for(&self, group_id: &GroupId) -> bool {
    self.shadow_state.contains_key(group_id)
}

/// Handle HubShadowSync from primary — store replicated state.
pub fn handle_shadow_sync(
    &mut self,
    group_id: &GroupId,
    members: Vec<GroupMember>,
    candidate_id: Option<NodeId>,
    config_version: u64,
) -> Vec<GroupAction> {
    if !self.groups.contains_key(group_id) {
        return vec![];
    }

    self.shadow_state.insert(
        group_id.clone(),
        ShadowState {
            members,
            candidate_id,
            config_version,
            ping_failures: 0,
            unreachable_reports: 0,
        },
    );

    vec![]
}

/// Record a ping failure (no pong received). Returns promotion actions if threshold hit.
pub fn record_ping_failure(&mut self, group_id: &GroupId) -> Vec<GroupAction> {
    let Some(state) = self.shadow_state.get_mut(group_id) else {
        return vec![];
    };

    state.ping_failures += 1;

    if self.should_promote(group_id) {
        self.promote_to_primary(group_id)
    } else {
        vec![]
    }
}

/// Handle HubUnreachable report from a member. Combined with ping failures, may trigger promotion.
pub fn handle_hub_unreachable(
    &mut self,
    group_id: &GroupId,
    _reporter: NodeId,
) -> Vec<GroupAction> {
    let Some(state) = self.shadow_state.get_mut(group_id) else {
        return vec![];
    };

    state.unreachable_reports += 1;

    if self.should_promote(group_id) {
        self.promote_to_primary(group_id)
    } else {
        vec![]
    }
}

/// Check if we should promote: 2 ping failures, OR 1 ping failure + 1 unreachable report.
fn should_promote(&self, group_id: &GroupId) -> bool {
    let Some(state) = self.shadow_state.get(group_id) else {
        return false;
    };
    let total_signals = state.ping_failures + state.unreachable_reports;
    state.ping_failures >= SHADOW_PING_FAILURE_THRESHOLD || (state.ping_failures >= 1 && total_signals >= 2)
}

/// Promote ourselves from shadow to primary hub.
fn promote_to_primary(&mut self, group_id: &GroupId) -> Vec<GroupAction> {
    let Some(state) = self.shadow_state.remove(group_id) else {
        return vec![];
    };
    let Some(group) = self.groups.get_mut(group_id) else {
        return vec![];
    };

    let old_hub_id = group.hub_relay_id;
    group.hub_relay_id = self.local_id;
    group.members = state.members;
    group.shadow_id = None;
    group.candidate_id = None;

    // Broadcast HubMigration to all members
    let recipients: Vec<NodeId> = group
        .members
        .iter()
        .map(|m| m.node_id)
        .filter(|id| *id != self.local_id)
        .collect();

    vec![GroupAction::Broadcast {
        to: recipients,
        payload: GroupPayload::HubMigration {
            group_id: group_id.clone(),
            new_hub_id: self.local_id,
            old_hub_id,
        },
    }]
}

/// Reset ping failure counter (called when pong is received).
pub fn reset_ping_failures(&mut self, group_id: &GroupId) {
    if let Some(state) = self.shadow_state.get_mut(group_id) {
        state.ping_failures = 0;
        state.unreachable_reports = 0;
    }
}

/// Get shadow state for building ping actions in the runtime tick.
pub fn shadow_groups(&self) -> Vec<(&GroupId, NodeId)> {
    self.shadow_state
        .keys()
        .filter_map(|gid| {
            let group = self.groups.get(gid)?;
            Some((gid, group.hub_relay_id))
        })
        .collect()
}
```

Also clean up shadow state in `leave_group`:

```rust
self.shadow_state.remove(group_id);
```

**Step 4: Run tests**

Run: `cargo test -p tom-protocol -- handle_shadow_sync shadow_promotes hub_unreachable_accelerates --nocapture 2>&1 | tail -20`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/group/manager.rs
git commit -m "feat(failover): member-side shadow role — watchdog, ping tracking, promotion"
```

---

### Task 4: Runtime integration — shadow ping timer and payload dispatch

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`
- Modify: `crates/tom-protocol/src/runtime/loop.rs`
- Modify: `crates/tom-protocol/src/runtime/mod.rs` (new config field + events)

**Step 1: Add config and events**

In `crates/tom-protocol/src/runtime/mod.rs`, add to `RuntimeConfig`:

```rust
/// Interval for shadow ping (watchdog).
pub shadow_ping_interval: Duration,
```

In `Default for RuntimeConfig`:

```rust
shadow_ping_interval: Duration::from_secs(3),
```

Add new `ProtocolEvent` variants (after `GroupSecurityViolation`):

```rust
/// Shadow promoted to primary hub for a group.
GroupShadowPromoted {
    group_id: GroupId,
    new_hub_id: NodeId,
},
/// This node was assigned as candidate for a group.
GroupCandidateAssigned { group_id: GroupId },
/// Hub failover chain fully restored after a promotion.
GroupHubChainRestored { group_id: GroupId },
```

**Step 2: Add tick and dispatch methods in state.rs**

In `crates/tom-protocol/src/runtime/state.rs`, add a new tick method (after `tick_group_hub_heartbeat`):

```rust
/// Shadow watchdog tick — send HubPing to primary for each group we shadow.
pub fn tick_shadow_ping(&mut self) -> Vec<RuntimeEffect> {
    let shadow_groups: Vec<(GroupId, NodeId)> = self
        .group_manager
        .shadow_groups()
        .into_iter()
        .map(|(gid, hub)| (gid.clone(), hub))
        .collect();

    let mut effects = Vec::new();
    for (group_id, hub_id) in shadow_groups {
        let payload = GroupPayload::HubPing {
            group_id: group_id.clone(),
        };
        let payload_bytes = rmp_serde::to_vec(&payload).expect("group payload serialization");
        let via = self.relay_selector.select_path(hub_id, &self.topology);
        let envelope =
            EnvelopeBuilder::new(self.local_id, hub_id, MessageType::GroupHubPing, payload_bytes)
                .via(via)
                .sign(&self.secret_seed);

        // Send with timeout — if no pong after timeout, record failure
        effects.push(RuntimeEffect::SendEnvelope(envelope));
    }
    effects
}

/// Handle shadow ping timeout (called when pong wasn't received in time).
pub fn handle_shadow_ping_timeout(&mut self, group_id: &GroupId) -> Vec<RuntimeEffect> {
    let actions = self.group_manager.record_ping_failure(group_id);
    self.group_actions_to_effects(&actions)
}
```

In `handle_incoming_group`, add dispatch arms for the new payloads (in the big match, before the catch-all):

```rust
// Shadow ping from shadow → primary responds with pong
GroupPayload::HubPing { ref group_id } => {
    if self.group_hub.get_group(group_id).is_some() {
        self.group_hub.handle_hub_ping(group_id, envelope.from);
        // Return pong
        let pong = GroupPayload::HubPong { group_id: group_id.clone() };
        let payload_bytes = rmp_serde::to_vec(&pong).expect("pong serialization");
        let via = self.relay_selector.select_path(envelope.from, &self.topology);
        let env = EnvelopeBuilder::new(self.local_id, envelope.from, MessageType::GroupHubPong, payload_bytes)
            .via(via)
            .sign(&self.secret_seed);
        return vec![RuntimeEffect::SendEnvelope(env)];
    }
    vec![]
}

// Pong from primary → reset shadow ping failures
GroupPayload::HubPong { ref group_id } => {
    self.group_manager.reset_ping_failures(group_id);
    vec![]
}

// Shadow sync from primary → store replicated state
GroupPayload::HubShadowSync {
    ref group_id,
    ref members,
    candidate_id,
    config_version,
} => {
    let actions = self.group_manager.handle_shadow_sync(
        group_id,
        members.clone(),
        candidate_id,
        config_version,
    );
    self.group_actions_to_effects(&actions)
}

// Candidate assignment
GroupPayload::CandidateAssigned { ref group_id } => {
    vec![RuntimeEffect::Emit(ProtocolEvent::GroupCandidateAssigned {
        group_id: group_id.clone(),
    })]
}

// Member reports hub unreachable to shadow
GroupPayload::HubUnreachable { ref group_id } => {
    let actions = self.group_manager.handle_hub_unreachable(group_id, envelope.from);
    self.group_actions_to_effects(&actions)
}
```

**Step 3: Add shadow ping timer in loop.rs**

In `crates/tom-protocol/src/runtime/loop.rs`, add timer setup (after `group_hub_heartbeat`):

```rust
let mut shadow_ping = tokio::time::interval(state.config.shadow_ping_interval);
```

Skip first tick:

```rust
shadow_ping.tick().await;
```

Add timer arm in the `select!` loop (after the `group_hub_heartbeat` arm):

```rust
_ = shadow_ping.tick() => state.tick_shadow_ping(),
```

**Step 4: Run full test suite**

Run: `cargo test -p tom-protocol 2>&1 | tail -10`
Expected: All existing tests + new tests PASS. Also: `cargo clippy -p tom-protocol -- -D warnings 2>&1 | tail -10`

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/
git commit -m "feat(failover): runtime integration — shadow ping timer, payload dispatch"
```

---

### Task 5: Hub-side — trigger shadow assignment after join/create

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs` (after group create/join, call assign_shadow)

**Step 1: Write failing integration test**

Create a new test in `crates/tom-protocol/tests/group_integration.rs`:

```rust
/// Test that shadow is assigned when a group has enough members.
#[test]
fn shadow_assigned_after_group_setup() {
    let hub_id = node_id(10);
    let alice_id = node_id(1);
    let bob_id = node_id(2);

    let mut hub = GroupHub::new(hub_id);

    hub.handle_payload(
        GroupPayload::Create {
            group_name: "Failover Test".into(),
            creator_username: "alice".into(),
            initial_members: vec![bob_id],
        },
        alice_id,
    );
    let gid = hub.groups().next().unwrap().0.clone();
    hub.handle_join(alice_id, &gid, "alice".into());
    hub.handle_join(bob_id, &gid, "bob".into());

    let actions = hub.assign_shadow(&gid);
    assert!(!actions.is_empty());

    // Verify the group now has a shadow
    let group = hub.get_group(&gid).unwrap();
    assert!(group.shadow_id.is_some());
}
```

Note: You may need to add a `pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &GroupInfo)>` method to `GroupHub` to iterate groups, or use `get_group`.

**Step 2: Run test to verify it fails**

Run: `cargo test -p tom-protocol shadow_assigned_after_group_setup -- --nocapture 2>&1 | head -20`

**Step 3: Hook shadow assignment into runtime state**

In `crates/tom-protocol/src/runtime/state.rs`, in the `handle_incoming_group` method, after handling `GroupPayload::Join` responses from the hub (where the hub produces `Sync` + `MemberJoined`), add shadow assignment:

Find the section that handles `Create` and `Join` on the hub side. After the existing hub logic produces actions, append:

```rust
// After handling Join on the hub side, check if we should assign/update shadow
if self.group_hub.get_group(&group_id).is_some() {
    let shadow_actions = self.group_hub.assign_shadow(&group_id);
    effects.extend(self.group_actions_to_effects(&shadow_actions));
}
```

This should be done carefully — only when we're the hub and a member joins or leaves.

**Step 4: Run tests**

Run: `cargo test -p tom-protocol 2>&1 | tail -10`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs crates/tom-protocol/src/group/hub.rs crates/tom-protocol/tests/group_integration.rs
git commit -m "feat(failover): trigger shadow assignment on group creation and member join"
```

---

### Task 6: Full failover integration test

**Files:**
- Modify: `crates/tom-protocol/tests/group_integration.rs`

**Step 1: Write the comprehensive integration test**

```rust
/// Test complete hub failover: primary dies → shadow promotes → members re-route.
#[test]
fn hub_failover_shadow_promotes_on_primary_death() {
    use tom_protocol::*;

    let hub_id = node_id(10);
    let shadow_id = node_id(2);
    let candidate_id = node_id(3);
    let alice_id = node_id(1);

    // ── Setup: create group with hub, shadow, candidate ──
    let mut hub = GroupHub::new(hub_id);
    let mut shadow_mgr = GroupManager::new(shadow_id, "shadow".into());
    let mut alice_mgr = GroupManager::new(alice_id, "alice".into());

    // Create group on hub
    let hub_actions = hub.handle_payload(
        GroupPayload::Create {
            group_name: "Failover E2E".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice_id,
    );
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else { panic!() };
    let gid = group.group_id.clone();
    alice_mgr.handle_group_created(group.clone());

    // Join shadow and candidate
    hub.handle_join(shadow_id, &gid, "shadow".into());
    hub.handle_join(candidate_id, &gid, "candidate".into());
    shadow_mgr.handle_group_created(GroupInfo {
        hub_relay_id: hub_id,
        ..group.clone()
    });

    // Hub assigns shadow
    let assign_actions = hub.assign_shadow(&gid);
    assert!(!assign_actions.is_empty());

    // Deliver HubShadowSync to shadow
    for action in &assign_actions {
        if let GroupAction::Send {
            to,
            payload: GroupPayload::HubShadowSync {
                group_id,
                members,
                candidate_id: cand,
                config_version,
            },
        } = action
        {
            if *to == shadow_id {
                shadow_mgr.handle_shadow_sync(group_id, members.clone(), *cand, *config_version);
            }
        }
    }
    assert!(shadow_mgr.is_shadow_for(&gid));

    // ── Simulate primary death: 2 ping failures ──
    let fail1 = shadow_mgr.record_ping_failure(&gid);
    assert!(fail1.is_empty());

    let fail2 = shadow_mgr.record_ping_failure(&gid);
    assert!(!fail2.is_empty(), "should promote after 2 failures");

    // Verify HubMigration is broadcast
    let has_migration = fail2.iter().any(|a| {
        matches!(
            a,
            GroupAction::Broadcast {
                payload: GroupPayload::HubMigration { new_hub_id, .. },
                ..
            } if *new_hub_id == shadow_id
        )
    });
    assert!(has_migration, "shadow should broadcast HubMigration with itself as new hub");

    // Shadow is no longer shadow (it's now hub)
    assert!(!shadow_mgr.is_shadow_for(&gid));

    // ── Alice receives migration and re-routes ──
    let alice_actions = alice_mgr.handle_hub_migration(&gid, shadow_id);
    assert_eq!(alice_actions.len(), 1);
    assert!(matches!(
        &alice_actions[0],
        GroupAction::Event(GroupEvent::HubMigrated { new_hub_id, .. }) if *new_hub_id == shadow_id
    ));

    // Verify Alice now routes to shadow as hub
    let alice_group = alice_mgr.get_group(&gid).unwrap();
    assert_eq!(alice_group.hub_relay_id, shadow_id);
}
```

**Step 2: Run test**

Run: `cargo test -p tom-protocol hub_failover_shadow_promotes_on_primary_death -- --nocapture 2>&1 | tail -30`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/tom-protocol/tests/group_integration.rs
git commit -m "test(failover): E2E integration test — shadow promotes on primary death"
```

---

### Task 7: Clippy + full test suite + exports

**Files:**
- Modify: `crates/tom-protocol/src/group/mod.rs` (re-exports)
- Modify: `crates/tom-protocol/src/lib.rs` (pub exports)

**Step 1: Update exports**

In `crates/tom-protocol/src/group/mod.rs`, add to the `pub use types::` block:

```rust
SHADOW_PING_INTERVAL_MS, SHADOW_PING_TIMEOUT_MS, SHADOW_PING_FAILURE_THRESHOLD,
HUB_ACK_TIMEOUT_MS, CANDIDATE_ORPHAN_TIMEOUT_MS,
```

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings 2>&1 | tail -20`

Fix any clippy warnings (likely candidates: unused variables, missing `_` prefixes on unused params, Entry API suggestions).

**Step 3: Run full test suite**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: All tests PASS (should be ~270+ tests now).

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor(failover): clippy fixes + exports + full test suite passing"
```

---

### Task 8: Push to GitHub

**Step 1: Verify state**

Run: `git log --oneline -10` and `git status`

**Step 2: Push**

Run: `git push`
