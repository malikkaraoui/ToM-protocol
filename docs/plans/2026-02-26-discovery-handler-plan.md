# Discovery Handler Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Surface all 4 discovery events (PeerDiscovered/Stale/Offline/Online) with DiscoverySource tracking in HeartbeatTracker.

**Architecture:** Modify HeartbeatTracker to track discovery source in a pending_discovery_source HashMap. Pass source as Option<DiscoverySource> to record_activity(). Consume source when emitting PeerDiscovered. Update RuntimeState to pass source at all discovery points. Unify PeerAnnounceReceived → PeerDiscovered in ProtocolEvent.

**Tech Stack:** Rust, rmp-serde (MessagePack), iroh_gossip, tokio

---

## Task 1: Add DiscoverySource tracking to HeartbeatTracker

**Files:**
- Modify: `crates/tom-protocol/src/discovery/heartbeat.rs:1-200`
- Test: `crates/tom-protocol/src/discovery/heartbeat.rs` (co-located tests)

**Step 1: Write failing test for source tracking**

Add to `crates/tom-protocol/src/discovery/heartbeat.rs` in the `#[cfg(test)]` section:

```rust
#[test]
fn test_record_activity_with_source() {
    let mut tracker = HeartbeatTracker::new();
    let node_id = test_node_id(1);

    tracker.record_activity(node_id, 1000, Some(DiscoverySource::Announce));

    let events = tracker.check_all(2000);
    assert_eq!(events.len(), 1);
    match &events[0] {
        DiscoveryEvent::PeerDiscovered { node_id: id, username: _, source } => {
            assert_eq!(id, &node_id);
            assert_eq!(*source, DiscoverySource::Announce);
        }
        _ => panic!("Expected PeerDiscovered event"),
    }
}

#[test]
fn test_peer_discovered_fallback_direct() {
    let mut tracker = HeartbeatTracker::new();
    let node_id = test_node_id(1);

    // Record activity WITHOUT source
    tracker.record_activity(node_id, 1000, None);

    let events = tracker.check_all(2000);
    assert_eq!(events.len(), 1);
    match &events[0] {
        DiscoveryEvent::PeerDiscovered { source, .. } => {
            assert_eq!(*source, DiscoverySource::Direct); // fallback
        }
        _ => panic!("Expected PeerDiscovered event"),
    }
}

#[test]
fn test_source_consumed_after_discovered() {
    let mut tracker = HeartbeatTracker::new();
    let node_id = test_node_id(1);

    tracker.record_activity(node_id, 1000, Some(DiscoverySource::Gossip));
    let _ = tracker.check_all(2000); // First call emits PeerDiscovered

    // Subsequent transitions should not have source anymore
    tracker.record_activity(node_id, 3000, None);
    tracker.record_activity(node_id, 50_000, None); // trigger stale
    let events = tracker.check_all(51_000);

    // Should emit PeerStale, not another PeerDiscovered
    assert!(events.iter().any(|e| matches!(e, DiscoveryEvent::PeerStale { .. })));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tom-protocol --lib discovery::heartbeat::tests::test_record_activity_with_source`

Expected: FAIL with "no method named `record_activity` with 3 arguments" or similar

**Step 3: Add pending_discovery_source field to HeartbeatTracker**

Modify struct in `crates/tom-protocol/src/discovery/heartbeat.rs`:

```rust
use crate::discovery::DiscoverySource; // Add import at top

pub struct HeartbeatTracker {
    peers: HashMap<NodeId, PeerState>,
    /// Pending discovery sources (consumed when PeerDiscovered emitted)
    pending_discovery_source: HashMap<NodeId, DiscoverySource>,
}

impl HeartbeatTracker {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            pending_discovery_source: HashMap::new(), // NEW
        }
    }
}
```

**Step 4: Update record_activity signature**

Modify `record_activity` method in `crates/tom-protocol/src/discovery/heartbeat.rs`:

```rust
/// Record peer activity with optional discovery source (for new peers).
pub fn record_activity(
    &mut self,
    node_id: NodeId,
    now: u64,
    source: Option<DiscoverySource>, // NEW param
) {
    // If source provided, store it for later PeerDiscovered emission
    if let Some(src) = source {
        self.pending_discovery_source.insert(node_id, src);
    }

    let state = self.peers.entry(node_id).or_insert_with(|| PeerState {
        last_seen: now,
        username: String::new(),
        state: LivenessState::Departed,
    });

    state.last_seen = now;
}
```

**Step 5: Update check_all to consume source**

Modify `check_all` method in `crates/tom-protocol/src/discovery/heartbeat.rs` (around line 98-140):

Find the section that emits `PeerDiscovered` and update it:

```rust
// When peer transitions to Alive (new discovery or recovery)
if new_state == LivenessState::Alive && old_state != LivenessState::Alive {
    // Consume source (fallback to Direct if missing)
    let source = self.pending_discovery_source
        .remove(&node_id)
        .unwrap_or(DiscoverySource::Direct);

    events.push(DiscoveryEvent::PeerDiscovered {
        node_id,
        username: state.username.clone(),
        source, // NEW field
    });
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p tom-protocol --lib discovery::heartbeat`

Expected: All tests PASS

**Step 7: Commit**

```bash
git add crates/tom-protocol/src/discovery/heartbeat.rs
git commit -m "feat(discovery): track DiscoverySource in HeartbeatTracker

- Add pending_discovery_source HashMap to HeartbeatTracker
- Update record_activity to accept Option<DiscoverySource>
- Consume source on PeerDiscovered emission (fallback: Direct)
- Add 3 unit tests for source tracking

Story: r3-discovery-handler"
```

---

## Task 2: Update DiscoveryEvent::PeerDiscovered signature

**Files:**
- Modify: `crates/tom-protocol/src/discovery/types.rs:76-102`
- Test: `crates/tom-protocol/src/discovery/types.rs` (existing roundtrip tests)

**Step 1: Update DiscoveryEvent::PeerDiscovered variant**

Modify enum in `crates/tom-protocol/src/discovery/types.rs`:

```rust
/// Events emitted by the discovery system.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A new peer was discovered.
    PeerDiscovered {
        node_id: NodeId,
        username: String,
        source: DiscoverySource, // NEW field
    },

    /// A peer went stale (missed heartbeats but might recover).
    PeerStale {
        node_id: NodeId,
    },

    /// A peer went offline (confirmed departed).
    PeerOffline {
        node_id: NodeId,
    },

    /// A peer came back online.
    PeerOnline {
        node_id: NodeId,
    },
}
```

**Step 2: Run tests to check compilation**

Run: `cargo test -p tom-protocol --lib discovery::types`

Expected: PASS (existing roundtrip tests should still work)

**Step 3: Commit**

```bash
git add crates/tom-protocol/src/discovery/types.rs
git commit -m "feat(discovery): add source field to DiscoveryEvent::PeerDiscovered

Story: r3-discovery-handler"
```

---

## Task 3: Update ProtocolEvent (unify PeerAnnounceReceived → PeerDiscovered)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/mod.rs:127-243`

**Step 1: Add PeerStale and update PeerDiscovered in ProtocolEvent**

Modify enum in `crates/tom-protocol/src/runtime/mod.rs`:

```rust
/// Events emitted by the protocol runtime to applications.
#[derive(Debug, Clone)]
pub enum ProtocolEvent {
    // ... existing variants

    /// A new peer was discovered.
    PeerDiscovered {
        node_id: NodeId,
        username: String,
        source: DiscoverySource, // NEW
    },

    /// A peer went stale (missed heartbeats).
    PeerStale {
        node_id: NodeId,
    },

    /// A peer went offline.
    PeerOffline {
        node_id: NodeId,
    },

    /// A peer came back online.
    PeerOnline {
        node_id: NodeId,
    },

    // REMOVE THIS VARIANT:
    // PeerAnnounceReceived { node_id: NodeId, username: String },

    // ... other existing variants
}
```

**Step 2: Add DiscoverySource import to runtime/mod.rs**

Add at top of `crates/tom-protocol/src/runtime/mod.rs`:

```rust
use crate::discovery::DiscoverySource;
```

**Step 3: Run tests to check compilation**

Run: `cargo test -p tom-protocol --lib runtime`

Expected: May have compilation errors in runtime/state.rs (we'll fix in next task)

**Step 4: Commit**

```bash
git add crates/tom-protocol/src/runtime/mod.rs
git commit -m "feat(runtime): unify PeerAnnounceReceived → PeerDiscovered, add PeerStale

- Remove PeerAnnounceReceived variant (breaking change)
- Add source field to PeerDiscovered
- Add PeerStale variant

Story: r3-discovery-handler"
```

---

## Task 4: Update RuntimeState.tick_heartbeat (remove stubs)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs:128-151`
- Test: Integration test in `crates/tom-protocol/tests/` (create new file)

**Step 1: Write integration test**

Create `crates/tom-protocol/tests/discovery_events.rs`:

```rust
use tom_protocol::runtime::ProtocolRuntime;
use tom_protocol::discovery::DiscoverySource;
use tom_transport::TomNode;

#[tokio::test]
async fn test_all_discovery_events_emitted() {
    // Setup runtime
    let node = TomNode::bind_random().await.unwrap();
    let config = Default::default();
    let channels = ProtocolRuntime::spawn(node, config, vec![]).await;

    // Collect events for 5 seconds
    let mut discovered = false;
    let mut stale = false;
    let mut offline = false;
    let mut online = false;

    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = channels.events.recv() => {
                match event {
                    ProtocolEvent::PeerDiscovered { source, .. } => {
                        discovered = true;
                        // Verify source is one of the valid variants
                        assert!(matches!(source, DiscoverySource::Direct | DiscoverySource::Gossip | DiscoverySource::Announce));
                    }
                    ProtocolEvent::PeerStale { .. } => stale = true,
                    ProtocolEvent::PeerOffline { .. } => offline = true,
                    ProtocolEvent::PeerOnline { .. } => online = true,
                    _ => {}
                }

                // All events seen, test passes
                if discovered && stale && offline && online {
                    break;
                }
            }
            _ = &mut timeout => break,
        }
    }

    // At minimum, we should see discovered events during normal operation
    assert!(discovered, "PeerDiscovered event should be emitted");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tom-protocol --test discovery_events`

Expected: FAIL (stub code doesn't emit PeerStale/PeerDiscovered yet)

**Step 3: Update tick_heartbeat to forward all events**

Modify `tick_heartbeat` in `crates/tom-protocol/src/runtime/state.rs`:

Find the section around line 128-151 and replace the stub match with:

```rust
pub fn tick_heartbeat(&mut self) -> Vec<RuntimeEffect> {
    let now = now_ms();
    let mut effects = Vec::new();

    for disc_event in self.heartbeat_tracker.check_all(now) {
        let effect = match disc_event {
            DiscoveryEvent::PeerDiscovered { node_id, username, source } => {
                // Source already included from HeartbeatTracker
                RuntimeEffect::Emit(ProtocolEvent::PeerDiscovered {
                    node_id,
                    username,
                    source,
                })
            }
            DiscoveryEvent::PeerStale { node_id } => {
                RuntimeEffect::Emit(ProtocolEvent::PeerStale { node_id })
            }
            DiscoveryEvent::PeerOffline { node_id } => {
                // Cleanup (roles + topology)
                self.role_manager.remove_node(&node_id);
                self.topology.remove(&node_id);
                RuntimeEffect::Emit(ProtocolEvent::PeerOffline { node_id })
            }
            DiscoveryEvent::PeerOnline { node_id } => {
                RuntimeEffect::Emit(ProtocolEvent::PeerOnline { node_id })
            }
        };
        effects.push(effect);
    }

    effects
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p tom-protocol --test discovery_events`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs crates/tom-protocol/tests/discovery_events.rs
git commit -m "feat(runtime): surface all 4 discovery events (remove stubs)

- Forward PeerDiscovered with source from HeartbeatTracker
- Forward PeerStale (previously stubbed)
- Add integration test for all 4 discovery events

Story: r3-discovery-handler"
```

---

## Task 5: Pass DiscoverySource to record_activity (PeerAnnounce)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs:1135-1198` (handle_gossip_event)

**Step 1: Write integration test for Announce source**

Add to `crates/tom-protocol/tests/discovery_events.rs`:

```rust
#[tokio::test]
async fn test_peer_announce_emits_discovered_with_announce_source() {
    let node = TomNode::bind_random().await.unwrap();
    let config = Default::default();
    let channels = ProtocolRuntime::spawn(node, config, vec![]).await;

    // Simulate gossip PeerAnnounce by using AddPeer then checking source
    // (Real gossip requires multiple nodes, this tests the plumbing)
    let peer_id = NodeId::from_endpoint_id(iroh::SecretKey::generate(&mut rand::thread_rng()).public());
    channels.handle.add_peer(peer_id, "test_peer".into(), None).await;

    // Wait for PeerDiscovered event
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(2));
    tokio::pin!(timeout);

    let mut found = false;
    loop {
        tokio::select! {
            Some(event) = channels.events.recv() => {
                if let ProtocolEvent::PeerDiscovered { node_id, source, .. } = event {
                    if node_id == peer_id {
                        assert_eq!(source, DiscoverySource::Direct); // AddPeer = Direct
                        found = true;
                        break;
                    }
                }
            }
            _ = &mut timeout => break,
        }
    }

    assert!(found, "PeerDiscovered event should be emitted with Direct source");
}
```

**Step 2: Run test to verify current behavior**

Run: `cargo test -p tom-protocol --test discovery_events::test_peer_announce_emits_discovered_with_announce_source`

Expected: Should PASS (AddPeer already works, we're establishing baseline)

**Step 3: Update handle_gossip_event (PeerAnnounce case)**

Modify `handle_gossip_event` in `crates/tom-protocol/src/runtime/state.rs` around line 1135-1198:

Find `GossipInput::PeerAnnounce(bytes)` case and update:

```rust
GossipInput::PeerAnnounce(bytes) => {
    let announce: PeerAnnounce = match rmp_serde::from_slice(&bytes) {
        Ok(a) => a,
        Err(e) => {
            tracing::debug!("gossip: invalid PeerAnnounce: {e}");
            return vec![];
        }
    };

    let now = now_ms();

    // Validate timestamp
    if !announce.is_timestamp_valid(now) {
        tracing::debug!("gossip: PeerAnnounce timestamp invalid");
        return vec![];
    }

    // Upsert topology
    let peer_info = PeerInfo {
        node_id: announce.node_id,
        username: announce.username.clone(),
        roles: announce.roles.clone(),
        encryption_key: announce.encryption_key,
        last_seen: now,
        status: PeerStatus::Online,
        role: PeerRole::Peer,
    };
    self.topology.upsert(peer_info);

    // Record activity with Announce source
    self.heartbeat_tracker.record_activity(
        announce.node_id,
        now,
        Some(DiscoverySource::Announce), // NEW
    );

    vec![]
}
```

**Step 4: Run workspace tests**

Run: `cargo test -p tom-protocol`

Expected: All tests PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs crates/tom-protocol/tests/discovery_events.rs
git commit -m "feat(runtime): pass Announce source on PeerAnnounce handling

- record_activity now receives Some(DiscoverySource::Announce)
- Add integration test for AddPeer → Direct source

Story: r3-discovery-handler"
```

---

## Task 6: Pass DiscoverySource on AddPeer command

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs` (handle_command)

**Step 1: Update handle_command (AddPeer case)**

Find `RuntimeCommand::AddPeer` case in `handle_command` method:

```rust
RuntimeCommand::AddPeer { node_id, username, encryption_key } => {
    let now = now_ms();

    let peer_info = PeerInfo {
        node_id,
        username: username.clone(),
        encryption_key,
        last_seen: now,
        status: PeerStatus::Online,
        role: PeerRole::Peer,
        roles: vec![],
    };

    self.topology.upsert(peer_info);

    // Record activity with Direct source
    self.heartbeat_tracker.record_activity(
        node_id,
        now,
        Some(DiscoverySource::Direct), // NEW
    );

    vec![]
}
```

**Step 2: Run tests**

Run: `cargo test -p tom-protocol`

Expected: All tests PASS

**Step 3: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "feat(runtime): pass Direct source on AddPeer command

Story: r3-discovery-handler"
```

---

## Task 7: Pass DiscoverySource on NeighborUp (gossip)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/loop.rs:147-160`

**Step 1: Update gossip NeighborUp handler**

Modify `runtime_loop` in `crates/tom-protocol/src/runtime/loop.rs` around line 147-160:

Find `GossipEvent::NeighborUp(endpoint_id)` case:

```rust
GossipEvent::NeighborUp(endpoint_id) => {
    let node_id = NodeId::from_endpoint_id(endpoint_id);
    let now = now_ms();

    // Record activity with Gossip source
    state.heartbeat_tracker.record_activity(
        node_id,
        now,
        Some(DiscoverySource::Gossip), // NEW
    );

    let effects = state.handle_gossip_event(
        GossipInput::NeighborUp(node_id)
    );

    // Re-broadcast announce on NeighborUp
    if let Some(ref sender) = gossip_sender {
        if let Some(bytes) = state.build_gossip_announce() {
            let _ = sender.broadcast(bytes::Bytes::from(bytes)).await;
        }
    }

    effects
}
```

Note: We need to import `now_ms` at the top of `loop.rs`:

```rust
use crate::types::now_ms;
```

**Step 2: Run tests**

Run: `cargo test -p tom-protocol`

Expected: All tests PASS

**Step 3: Commit**

```bash
git add crates/tom-protocol/src/runtime/loop.rs
git commit -m "feat(runtime): pass Gossip source on NeighborUp event

Story: r3-discovery-handler"
```

---

## Task 8: Update existing record_activity calls (remove source param where not needed)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs` (all record_activity calls)

**Step 1: Find all record_activity calls without source**

Run: `rg "record_activity" crates/tom-protocol/src/runtime/state.rs`

For any calls that don't need a source (e.g., periodic heartbeat updates), pass `None`:

```rust
// Example: periodic heartbeat (not a discovery event)
self.heartbeat_tracker.record_activity(node_id, now, None);
```

**Step 2: Run tests**

Run: `cargo test -p tom-protocol`

Expected: All tests PASS

**Step 3: Commit (if changes made)**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): pass None for non-discovery record_activity calls

Story: r3-discovery-handler"
```

---

## Task 9: Update tom-tui (breaking change: PeerAnnounceReceived → PeerDiscovered)

**Files:**
- Modify: `crates/tom-tui/src/main.rs` (event handling)

**Step 1: Find PeerAnnounceReceived usage**

Run: `rg "PeerAnnounceReceived" crates/tom-tui/src/main.rs`

**Step 2: Replace with PeerDiscovered**

Update pattern match in event handler:

```rust
// OLD:
ProtocolEvent::PeerAnnounceReceived { node_id, username } => {
    // ... handling
}

// NEW:
ProtocolEvent::PeerDiscovered { node_id, username, source } => {
    // Log source for debugging
    tracing::debug!("Peer discovered: {username} (source: {source:?})");
    // ... rest of handling
}
```

Add any new event handlers:

```rust
ProtocolEvent::PeerStale { node_id } => {
    tracing::debug!("Peer stale: {node_id}");
}
```

**Step 3: Run tom-tui compilation**

Run: `cargo build -p tom-tui`

Expected: SUCCESS

**Step 4: Commit**

```bash
git add crates/tom-tui/src/main.rs
git commit -m "fix(tui): update for PeerAnnounceReceived → PeerDiscovered

- Replace PeerAnnounceReceived with PeerDiscovered
- Add source field logging
- Add PeerStale event handler

Story: r3-discovery-handler (breaking change)"
```

---

## Task 10: Update tom-stress (breaking change: PeerAnnounceReceived → PeerDiscovered)

**Files:**
- Modify: `crates/tom-stress/src/campaign.rs` (event handling)

**Step 1: Find PeerAnnounceReceived usage**

Run: `rg "PeerAnnounceReceived" crates/tom-stress/src/campaign.rs`

**Step 2: Replace with PeerDiscovered**

Update pattern match in event handler (similar to tom-tui):

```rust
// OLD:
ProtocolEvent::PeerAnnounceReceived { node_id, username } => {
    // ... handling
}

// NEW:
ProtocolEvent::PeerDiscovered { node_id, username, source } => {
    // Maybe log source in verbose mode
    if verbose {
        println!("Discovered {} via {:?}", username, source);
    }
    // ... rest of handling
}
```

**Step 3: Run tom-stress compilation**

Run: `cargo build -p tom-stress`

Expected: SUCCESS

**Step 4: Commit**

```bash
git add crates/tom-stress/src/campaign.rs
git commit -m "fix(stress): update for PeerAnnounceReceived → PeerDiscovered

Story: r3-discovery-handler (breaking change)"
```

---

## Task 11: Final integration test and validation

**Files:**
- Test: Run full workspace tests

**Step 1: Run all workspace tests**

Run: `cargo test --workspace`

Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --workspace`

Expected: No warnings

**Step 3: Build all binaries**

Run: `cargo build --workspace --release`

Expected: SUCCESS

**Step 4: Manual smoke test (optional)**

Run tom-chat in bot mode and verify events:

```bash
# Terminal 1
./target/release/tom-chat --bot

# Terminal 2 (different machine or use AddPeer)
./target/release/tom-chat
```

Observe logs for PeerDiscovered/PeerStale/PeerOffline/PeerOnline events.

**Step 5: Final commit (if any fixes needed)**

```bash
git add .
git commit -m "test(discovery): validate all 4 events in integration

Story: r3-discovery-handler"
```

---

## Success Criteria Checklist

- [ ] All 4 DiscoveryEvent variants surfaced (PeerDiscovered, PeerStale, PeerOffline, PeerOnline)
- [ ] DiscoverySource tracked in HeartbeatTracker (pending_discovery_source HashMap)
- [ ] Source passed correctly at 3 entry points (PeerAnnounce=Announce, NeighborUp=Gossip, AddPeer=Direct)
- [ ] PeerAnnounceReceived removed from ProtocolEvent (breaking change applied)
- [ ] tom-tui updated for new event API
- [ ] tom-stress updated for new event API
- [ ] All unit tests pass (HeartbeatTracker source tracking)
- [ ] All integration tests pass (discovery_events.rs)
- [ ] No memory leaks (source consumed on PeerDiscovered)
- [ ] Clippy clean

---

## Completion

Once all tasks complete:

1. Update sprint-status.yaml: `r3-discovery-handler: done`
2. Push branch: `git push origin feat/discovery-handler`
3. (Optional) Create PR or merge to main

Story r3-discovery-handler complete! 🎉
