# Dynamic Role Assignment - Bandwidth Tracking & Network Coordination

**Date:** 2026-02-24
**Status:** Approved
**Approach:** Incremental (Bottom-Up)

## Context

Dynamic Role Assignment is a core pillar of the ToM Protocol. The base implementation exists in `crates/tom-protocol/src/roles/` with contribution scoring and automatic promotion/demotion (Peer ↔ Relay). This design extends the existing system with:

1. **Bandwidth tracking** (mentioned in requirements, not yet implemented)
2. **Minimal observability** (debug-focused, not user-facing)
3. **Network coordination** (gossip broadcast of role changes)

## Current State

### Existing Implementation

- **ContributionMetrics** ([roles/scoring.rs](../../crates/tom-protocol/src/roles/scoring.rs))
  Tracks: `messages_relayed`, `relay_failures`, `uptime`, `first_seen`, `last_activity`
  Formula: `relay_count × 1.0 + success_rate × 5.0 + uptime_hours × 0.5`
  Decay: 5% per hour (progressive, no permanent bans — Design Decision #4)

- **RoleManager** ([roles/manager.rs](../../crates/tom-protocol/src/roles/manager.rs))
  Promotion threshold: score ≥ 10.0
  Demotion threshold: score < 2.0
  Returns `RoleAction`: Promoted/Demoted/LocalRoleChanged

- **Runtime Integration** ([runtime/state.rs](../../crates/tom-protocol/src/runtime/state.rs))
  `tick_roles()` evaluates every 60 seconds
  `record_relay()` called on each successful relay
  Emits: `RolePromoted`, `RoleDemoted`, `LocalRoleChanged`

- **Test Coverage**
  5 integration tests in [roles_integration.rs](../../crates/tom-protocol/tests/roles_integration.rs)
  Full lifecycle: activity → promotion → idleness → demotion

### What's Missing

1. **Bandwidth metrics**: Only relay count and uptime are tracked, not bytes relayed
2. **Network sync**: Role changes are local-only, not broadcast to peers
3. **Observability**: Limited debug capabilities (queries for metrics)

## Architecture

### Layered Design

```
┌─────────────────────────────────────────────────────────┐
│  Layer 4: Network Sync (discovery/role_sync.rs)        │
│  - RoleChangeAnnounce via gossip                        │
│  - Signature validation, throttling                     │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│  Layer 3: Runtime Integration (runtime/state.rs)       │
│  - record_bytes_relayed() on Forwarded action           │
│  - surface_role_action() → broadcast announce           │
│  - tick_roles() every 60s                               │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│  Layer 2: Role Manager (roles/manager.rs)              │
│  - evaluate() with extended scoring formula             │
│  - record_bytes_relayed(node_id, bytes, now)           │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│  Layer 1: Metrics (roles/scoring.rs)                   │
│  - bytes_relayed: u64 (new)                             │
│  - bytes_received: u64 (new)                            │
│  - Existing: messages_relayed, uptime, failures         │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

```
Message relayed
    ↓
record_relay() + record_bytes_relayed()
    ↓
Metrics update (ContributionMetrics)
    ↓
tick_roles() every 60s
    ↓
Score evaluation → promotion/demotion
    ↓
Emit ProtocolEvent + broadcast via gossip
    ↓
Peers receive RoleChangeAnnounce
    ↓
Validate signature + score plausibility
    ↓
Update local Topology
```

## Scoring Formula

### Extended Formula

```rust
let relay_count = self.messages_relayed as f64;
let success_rate = if total_attempts > 0 {
    self.messages_relayed as f64 / total_attempts as f64
} else {
    1.0
};
let uptime_hours = self.total_uptime_ms as f64 / 3_600_000.0;

// New metrics
let bandwidth_mb = self.bytes_relayed as f64 / 1_048_576.0;
let bandwidth_ratio = if self.bytes_received > 0 {
    self.bytes_relayed as f64 / self.bytes_received as f64
} else {
    1.0  // Default: assume 100% relay efficiency
};

// Complete score
raw_score = relay_count * 1.0          // Base contribution
          + success_rate * 5.0         // Reliability (highest weight)
          + uptime_hours * 0.5         // Stability
          + bandwidth_mb * 0.2         // Volume handled
          + bandwidth_ratio * 1.5;     // Efficiency (give > take)
```

### Weight Rationale

| Metric | Weight | Justification |
|--------|--------|---------------|
| `relay_count` | 1.0 | Baseline: each relay = 1 point |
| `success_rate` | 5.0 | **Most important**: reliability > quantity |
| `uptime_hours` | 0.5 | Stability: 2h uptime = 1 relay |
| `bandwidth_mb` | 0.2 | Volume: 5 MB relayed = 1 relay |
| `bandwidth_ratio` | 1.5 | **Priority**: give > take = good citizen |

### Score Examples

**Heavy Relay:**
- 100 relays, 95% success, 10h uptime
- 500 MB relayed, 200 MB received (ratio = 2.5)
- **Score:** 100×1.0 + 0.95×5.0 + 10×0.5 + 500×0.2 + 2.5×1.5 = **213.5**

**Leech (takes more than gives):**
- 5 relays, 100% success, 5h uptime
- 10 MB relayed, 200 MB received (ratio = 0.05)
- **Score:** 5×1.0 + 1.0×5.0 + 5×0.5 + 10×0.2 + 0.05×1.5 = **14.6** ← below promotion threshold

### Tunable Constants

```rust
// roles/scoring.rs
pub const RELAY_COUNT_WEIGHT: f64 = 1.0;
pub const SUCCESS_RATE_WEIGHT: f64 = 5.0;
pub const UPTIME_WEIGHT: f64 = 0.5;
pub const BANDWIDTH_MB_WEIGHT: f64 = 0.2;      // NEW
pub const BANDWIDTH_RATIO_WEIGHT: f64 = 1.5;   // NEW

// Note: These will be adjustable based on beta testing feedback
```

## Observability

### Principle
Observability is for **debug purposes only**, not user-facing UI. If the system works, that's what matters. Logic is documented in BMAD and design docs.

### Minimal Implementation

**Existing Events (kept):**
- `RolePromoted { node_id, score }`
- `RoleDemoted { node_id, score }`
- `LocalRoleChanged { new_role }`

**New Capabilities (optional, for debug):**
- `RuntimeCommand::GetRoleMetrics { node_id, reply }` → query specific peer's score
- `RuntimeCommand::GetAllRoleScores { reply }` → list all peers with scores

**No new events added** (RoleMetricsUpdated, RoleEvaluationCompleted skipped for now).

## Network Coordination

### Gossip Broadcast

When a peer's role changes (Peer → Relay or Relay → Peer), it broadcasts the change via gossip so other nodes can update their local `Topology`.

### Message Format

```rust
// discovery/role_sync.rs (new module)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleChangeAnnounce {
    pub node_id: NodeId,
    pub new_role: PeerRole,
    pub score: f64,              // For validation
    pub timestamp: u64,
    pub signature: Vec<u8>,      // Prevent impersonation
}
```

### Security Validation

When receiving a `RoleChangeAnnounce`:

1. **Verify signature** (prevent identity spoofing)
2. **Verify score plausibility** (if peer announces score 1000 but never seen relaying → suspicious)
3. **Throttle** (max 1 announce per peer every 30s, prevent spam)

### Integration Flow

```rust
// runtime/state.rs
fn surface_role_action(&mut self, action: &RoleAction) -> Vec<RuntimeEffect> {
    match action {
        RoleAction::Promoted { node_id, score } => {
            let evt = ProtocolEvent::RolePromoted {
                node_id: *node_id,
                score: *score
            };
            let mut effects = vec![RuntimeEffect::Emit(evt)];

            // If it's us, broadcast via gossip
            if *node_id == self.local_id {
                let announce = self.build_role_change_announce(
                    PeerRole::Relay,
                    *score
                );
                effects.push(RuntimeEffect::BroadcastGossip(announce));
            }
            effects
        }
        // Same for Demoted and LocalRoleChanged
    }
}
```

### Design Note

This gossip coordination is **initial implementation**. It will evolve based on beta testing:
- Current: broadcast on change
- Future possibilities: periodic sync, conflict resolution, Byzantine fault tolerance

## Implementation Strategy

### Approach: Incremental (Bottom-Up)

Validated with user. Proceed in 3 phases:

**Phase 1: Bandwidth Tracking**
- Extend `ContributionMetrics` with `bytes_relayed`, `bytes_received`
- Add `record_bytes_relayed()` to `RoleManager`
- Update scoring formula
- Add tests for new metrics

**Phase 2: Observability**
- Add `GetRoleMetrics` and `GetAllRoleScores` commands
- Export `RoleMetrics` struct
- Keep events unchanged (existing ones sufficient)

**Phase 3: Network Coordination**
- Create `discovery/role_sync.rs` with `RoleChangeAnnounce`
- Implement `surface_role_action()` with gossip broadcast
- Add signature validation and throttling
- Integration tests for role sync across peers

### Test Coverage

- Unit tests for extended scoring formula
- Integration tests for bandwidth tracking
- Network sync tests (2 nodes, role change propagation)
- Stress test campaign phase (optional, for validation)

## Alignment with Design Decisions

- **DD#4 (Progressive fade, no permanent bans):** Score decay remains 5% per hour
- **DD#6 (Invisibility):** Role changes are internal, not user-visible
- **DD#7 (Scope):** Universal foundation layer, not application logic

## References

- TypeScript implementation: `_bmad-output/implementation-artifacts/3-2-dynamic-role-assignment.md`
- ADR-006: Unified node model (all nodes run identical code, role is network-assigned)
- ADR-007: Network-imposed roles (nodes don't choose to be relays)
- Stress Campaign V5: Channel pump pattern applicable for high-volume role events

---

**Next Step:** Create implementation plan with writing-plans skill.
