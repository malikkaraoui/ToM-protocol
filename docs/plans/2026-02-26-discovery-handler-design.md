# Discovery Handler — Complete Event Surfacing

**Date:** 2026-02-26
**Status:** Design Approved
**Story:** r3-discovery-handler (Phase R3 — Discovery + Keepalive)

## Context

HeartbeatTracker generates 4 discovery events (PeerDiscovered, PeerStale, PeerOffline, PeerOnline) but only 2 are currently surfaced to applications (PeerOffline, PeerOnline). PeerStale and PeerDiscovered are stubbed with `// log or ignore for MVP` comments.

**Goal:** Surface all 4 discovery events to applications with DiscoverySource metadata for debugging/observability.

**Current State:**
- File: `runtime/state.rs:128-151` — tick_heartbeat() drops PeerStale and PeerDiscovered
- DiscoveryEvent enum exists with all 4 variants (discovery/types.rs:76-102)
- DiscoverySource enum exists but unused (discovery/types.rs:104-113)
- PeerAnnounceReceived event exists separately (runtime/mod.rs:204-207)

## Architecture

### Events (ProtocolEvent)

```rust
pub enum ProtocolEvent {
    /// A peer was discovered (consolidates PeerAnnounceReceived)
    PeerDiscovered {
        node_id: NodeId,
        username: String,
        source: DiscoverySource,  // NEW: how we discovered them
    },

    /// Peer missed heartbeats but might recover
    PeerStale {
        node_id: NodeId,
    },

    /// Peer confirmed departed
    PeerOffline {
        node_id: NodeId,
    },

    /// Peer came back online
    PeerOnline {
        node_id: NodeId,
    },

    // ... existing events
}
```

**Breaking Change:** PeerAnnounceReceived → PeerDiscovered (acceptable: tom-chat/tom-stress only, not published)

**DiscoverySource (unchanged):**
```rust
pub enum DiscoverySource {
    Direct,   // Bootstrap/AddPeer command
    Gossip,   // Learned via iroh_gossip NeighborUp
    Announce, // Peer announced itself via PeerAnnounce
}
```

### Source Tracking

**Decision:** HeartbeatTracker tracks discovery source (not RuntimeState cache).

**Rationale:**
- Single source of truth (liveness + source co-located)
- No timing mismatch (source consumed when event emitted)
- Automatic cleanup (HashMap.remove on PeerDiscovered)
- Explicit API (source passed to record_activity)

**HeartbeatTracker changes:**

```rust
pub struct HeartbeatTracker {
    peers: HashMap<NodeId, PeerState>,
    pending_discovery_source: HashMap<NodeId, DiscoverySource>,  // NEW
}

impl HeartbeatTracker {
    /// Record activity with optional discovery source (for new peers)
    pub fn record_activity(
        &mut self,
        node_id: NodeId,
        now: u64,
        source: Option<DiscoverySource>,  // NEW param
    ) {
        if let Some(src) = source {
            self.pending_discovery_source.insert(node_id, src);
        }

        // ... existing logic (update last_seen, etc.)
    }

    pub fn check_all(&mut self, now: u64) -> Vec<DiscoveryEvent> {
        // ...

        // When emitting PeerDiscovered, consume source
        if new_state == LivenessState::Alive && old_state != LivenessState::Alive {
            let source = self.pending_discovery_source
                .remove(&node_id)  // consume + cleanup
                .unwrap_or(DiscoverySource::Direct);  // fallback

            events.push(DiscoveryEvent::PeerDiscovered {
                node_id,
                username,
                source,
            });
        }

        // ... other state transitions
    }
}
```

## Event Flow

### 1. Gossip PeerAnnounce → DiscoverySource::Announce

```rust
// runtime/state.rs (handle_gossip_event)
GossipInput::PeerAnnounce(bytes) => {
    let announce: PeerAnnounce = rmp_serde::from_slice(&bytes)?;

    // Validate timestamp
    if !announce.is_timestamp_valid(now) {
        return vec![]; // reject
    }

    // Upsert topology
    self.topology.upsert(peer_info);

    // Record activity + source
    self.heartbeat_tracker.record_activity(
        announce.node_id,
        now,
        Some(DiscoverySource::Announce),  // NEW
    );

    vec![]  // HeartbeatTracker emits PeerDiscovered later
}
```

### 2. Gossip NeighborUp → DiscoverySource::Gossip

```rust
// runtime/loop.rs (gossip event handling)
GossipEvent::NeighborUp(endpoint_id) => {
    let node_id = NodeId::from_endpoint_id(endpoint_id);

    // Record source (peer learned via gossip)
    state.heartbeat_tracker.record_activity(
        node_id,
        now,
        Some(DiscoverySource::Gossip),  // NEW
    );

    // Re-broadcast our announce (key learning from PoC-3)
    if let Some(bytes) = state.build_gossip_announce() {
        sender.broadcast(bytes).await;
    }

    vec![]
}
```

### 3. RuntimeCommand::AddPeer → DiscoverySource::Direct

```rust
// runtime/state.rs (handle_command)
RuntimeCommand::AddPeer { node_id, username, encryption_key } => {
    let peer_info = PeerInfo {
        node_id,
        username: username.clone(),
        encryption_key,
        // ...
    };

    self.topology.upsert(peer_info);
    self.heartbeat_tracker.record_activity(
        node_id,
        now,
        Some(DiscoverySource::Direct),  // NEW
    );

    vec![]
}
```

### 4. RuntimeState.tick_heartbeat() — Forward all events

```rust
pub fn tick_heartbeat(&mut self) -> Vec<RuntimeEffect> {
    let now = now_ms();
    let mut effects = Vec::new();

    for disc_event in self.heartbeat_tracker.check_all(now) {
        let effect = match disc_event {
            DiscoveryEvent::PeerDiscovered { node_id, username, source } => {
                // Source already included, just forward
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

## Error Handling

| Edge Case | Behavior |
|-----------|----------|
| Source missing on PeerDiscovered | Fallback to `DiscoverySource::Direct` |
| PeerAnnounce invalid timestamp | Reject (existing logic in `is_timestamp_valid()`) |
| Duplicate discovery (multiple sources) | Last source wins (HashMap.insert overwrites) |
| Peer never transitions to Alive | Source cleaned up on next check_all() |

## Testing Strategy

### Unit Tests (discovery/heartbeat.rs)

1. **test_record_activity_with_source** — Verify source stored correctly
2. **test_peer_discovered_consumes_source** — Verify source consumed after emission
3. **test_peer_discovered_fallback_direct** — Verify fallback when source missing
4. **test_duplicate_source_overwrites** — Verify last source wins on duplicate

### Integration Tests (runtime/state.rs or integration/)

1. **test_peer_announce_emits_discovered_with_announce_source** — Full flow: PeerAnnounce → PeerDiscovered with Announce source
2. **test_add_peer_emits_discovered_with_direct_source** — Full flow: AddPeer → PeerDiscovered with Direct source
3. **test_all_discovery_events_emitted** — Verify all 4 events emitted (Discovered/Stale/Offline/Online)

## Files Modified

- `crates/tom-protocol/src/discovery/heartbeat.rs` — Add pending_discovery_source, update record_activity/check_all
- `crates/tom-protocol/src/runtime/state.rs` — Remove stubs in tick_heartbeat, pass source to record_activity
- `crates/tom-protocol/src/runtime/mod.rs` — Add PeerStale, unify PeerAnnounceReceived → PeerDiscovered
- `crates/tom-protocol/src/runtime/loop.rs` — Pass source on NeighborUp
- `crates/tom-tui/src/main.rs` — Update event handler (PeerAnnounceReceived → PeerDiscovered)
- `crates/tom-stress/src/campaign.rs` — Update event handler (PeerAnnounceReceived → PeerDiscovered)

## Breaking Changes

**PeerAnnounceReceived → PeerDiscovered:**
- Affects: tom-tui, tom-stress (internal binaries, not published)
- Migration: Replace pattern match `PeerAnnounceReceived { node_id, username }` with `PeerDiscovered { node_id, username, source }`
- Acceptable: No external API consumers

## Success Criteria

- [ ] All 4 DiscoveryEvent variants emitted to applications
- [ ] DiscoverySource correctly tracked and surfaced on PeerDiscovered
- [ ] PeerAnnounceReceived removed (unified into PeerDiscovered)
- [ ] No memory leaks (source cache cleaned up)
- [ ] All tests pass (unit + integration)
- [ ] tom-tui/tom-stress updated for new event API
