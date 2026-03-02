# ToM Protocol - Claude/LLM Documentation

This document provides comprehensive guidance for LLMs working with the ToM Protocol codebase.

## Project Overview

**ToM (The Open Messaging)** is a decentralized peer-to-peer transport protocol where every device acts as both client and relay. Key principles:

- **No central servers**: Messages route through peer relays
- **Relay statelessness**: Relays forward without storing (pass-through only)
- **End-to-end encryption**: Only sender and recipient can read content
- **Dynamic roles**: Network assigns relay duties based on contribution
- **Self-organizing**: Gossip discovery and ephemeral subnets

## Repository Structure

```
tom-protocol/
├── crates/
│   ├── tom-connect/          # Transport layer (forked from iroh 0.96, ~15K LOC)
│   │                         #   MagicSock, Disco hole punching, Endpoint
│   ├── tom-relay/            # Relay server (forked from iroh-relay, ~8K LOC)
│   │                         #   Stateless relay, HTTP/HTTPS, --dev mode
│   ├── tom-gossip/           # Gossip protocol (forked from iroh-gossip, ~5K LOC)
│   ├── tom-quinn/            # QUIC runtime (forked from iroh-quinn, 6.5K LOC)
│   ├── tom-quinn-proto/      # QUIC protocol (forked from iroh-quinn-proto, 41K LOC)
│   ├── tom-base/             # Base types: PublicKey, SecretKey, NodeAddr (forked, 831 LOC)
│   ├── tom-metrics/          # Simplified metrics (Counter struct, ~100 LOC)
│   ├── tom-dht/              # DHT discovery wrapper (Mainline BEP-0044)
│   ├── tom-protocol/         # Protocol layer (original)
│   │   └── src/
│   │       ├── backup/       # Message backup (virus metaphor, TTL 24h)
│   │       ├── crypto/       # Ed25519 sign + X25519 DH + XChaCha20-Poly1305 + HKDF
│   │       ├── discovery/    # PeerAnnounce, HeartbeatTracker, EphemeralSubnets
│   │       ├── envelope/     # MessagePack wire format, EnvelopeBuilder
│   │       ├── group/        # GroupManager, GroupHub, hub failover, sender keys
│   │       ├── relay/        # RelaySelector, Topology
│   │       ├── roles/        # RoleManager, ContributionMetrics, scoring
│   │       ├── router/       # Router (Deliver/Forward/Reject/Ack/Drop)
│   │       ├── runtime/      # ProtocolRuntime, RuntimeState, effect pattern
│   │       ├── tracker/      # MessageTracker (status state machine)
│   │       └── types/        # NodeId, MessageType, MessageStatus
│   ├── tom-stress/           # Stress testing campaigns (Mac ↔ NAS)
│   └── tom-tui/              # TUI chat client (ratatui, --bot mode)
│
├── packages/                 # TypeScript (Phase 1, legacy)
│   ├── core/                 # Protocol primitives (771 tests)
│   └── sdk/                  # TomClient SDK
│
├── apps/
│   └── demo/                 # Browser demo (vanilla HTML/JS + Vite)
│
├── tools/
│   └── signaling-server/     # DEPRECATED — WebSocket bootstrap (Phase R7)
│
├── docs/plans/               # Design docs and implementation plans
└── _bmad-output/             # Planning artifacts (PRD, architecture, epics)
```

## Architecture (Post-Fork — Phase R7)

### Transport Stack

```
Application
    ↓
tom-protocol (ProtocolRuntime)     ← Protocol logic, groups, encryption
    ↓
tom-connect (Endpoint/MagicSock)   ← NAT traversal, hole punching, relay fallback
    ↓
tom-quinn (QUIC runtime)           ← Connection management
    ↓
tom-quinn-proto (QUIC protocol)    ← Wire protocol, crypto handshake
    ↓
UDP (iroh-quinn-udp)               ← Raw UDP I/O (not forked — netwatch compat)
```

### Fork Status

All critical iroh dependencies have been forked under the `tom-*` namespace (MIT license):

| Original | Fork | LOC | Notes |
|----------|------|-----|-------|
| iroh (endpoint+socket) | tom-connect | ~15K | MagicSock, Disco, hole punching |
| iroh-relay | tom-relay | ~8K | Stateless relay server |
| iroh-gossip | tom-gossip | ~5K | Gossip protocol |
| iroh-quinn | tom-quinn | 6.5K | QUIC runtime |
| iroh-quinn-proto | tom-quinn-proto | 41K | QUIC protocol |
| iroh-base | tom-base | 831 | PublicKey, SecretKey, NodeAddr |
| iroh-metrics | tom-metrics | ~100 | Simplified Counter struct |

**Not forked (intentional):**
- `iroh-quinn-udp` — netwatch exposes its types in public API, forking creates type mismatch
- `n0-error`, `n0-future`, `n0-watcher` — general-purpose utils, shared with external deps

### Wire Invariants (NEVER change)

These are baked into the protocol and must stay compatible with iroh network:
- `_iroh` DNS prefix (Pkarr/discovery)
- `.iroh.invalid` TLS SNI
- `X-Iroh-*` HTTP headers (relay protocol)
- `b"/iroh-qad/0"` ALPN
- `iroh.link` relay URLs

### Cargo Alias Trick

Consumers use package aliases to avoid source code changes:
```toml
quinn = { package = "tom-quinn" }     # source says quinn::Connection
```
Inside tom-quinn:
```toml
proto = { package = "tom-quinn-proto" } # source says proto::
```

## Key Architecture Decisions (ADRs)

### ADR-001: QUIC via Relay (updated from WebRTC)
All messages transit through at least one relay initially, then upgrade to direct QUIC. Relays are not optional — they ARE the architecture.

### ADR-002: Bootstrap Elimination (DONE — Phase R7)
- **Before**: WebSocket signaling server (temporary)
- **Now**: Own relay (`tom-relay --dev`) + Pkarr/DNS discovery
- `TOM_RELAY_URL` env var for custom relay
- `n0_discovery(true/false)` flag for Pkarr/DNS toggle

### ADR-003: Wire Format
MessagePack envelopes (rmp-serde). `signing_bytes()` EXCLUDES `ttl` (mutated by relays).

### ADR-004: Encryption Stack (Rust)
Ed25519 signing + X25519 DH + XChaCha20-Poly1305 + HKDF-SHA256. `encrypt_and_sign()` = encrypt-then-sign.

### ADR-005: Node Identity
Ed25519 keypair = node identity. Public key is the network address (NodeId).

### ADR-006: Unified Node Model
Every node runs identical code. Role is determined by network topology, not configuration.

### ADR-009: Message Backup (Virus Metaphor)
Messages for offline recipients self-replicate across backup nodes, self-delete when delivered or after 24h TTL.

## Foundational Design Decisions (LOCKED)

**These 7 decisions are non-negotiable and define ToM's character. All code must respect them.**

See full details: `_bmad-output/planning-artifacts/design-decisions.md`

| # | Decision | Rule |
|---|----------|------|
| 1 | **Delivery** | Message delivered ⟺ recipient emits ACK |
| 2 | **TTL** | 24h max lifespan, then global purge (no exceptions) |
| 3 | **L1 Role** | L1 anchors state, never arbitrates |
| 4 | **Reputation** | Progressive fade, no permanent bans |
| 5 | **Anti-spam** | "Sprinkler gets sprinkled" — progressive load, not exclusion |
| 6 | **Invisibility** | Protocol layer invisible to end users |
| 7 | **Scope** | Universal foundation (like TCP/IP), not a product |

**Before writing code, verify:**
- No user-visible protocol state
- L1 doesn't make operational decisions
- No permanent bans or binary states
- No message persistence beyond TTL

## Core Components (Rust)

### ProtocolRuntime

Single `tokio::select!` loop, spawned as a background task:

```rust
use tom_protocol::{ProtocolRuntime, RuntimeConfig, TomNodeConfig};

let node = TomNodeConfig::new()
    .n0_discovery(false)          // disable Pkarr/DNS
    .bind().await?;

let config = RuntimeConfig {
    username: "alice".into(),
    encryption: true,
    ..Default::default()
};

let channels = ProtocolRuntime::spawn(node, config);

// Send message
channels.handle.send_message(target_id, payload).await?;

// Receive messages (already decrypted + verified)
while let Some(msg) = channels.messages.recv().await {
    println!("From {}: {:?}", msg.from, msg.payload);
}
```

### RuntimeHandle

Clonable handle for interacting with the runtime:

```rust
let handle = channels.handle.clone();
handle.send_message(target, payload).await?;
handle.add_peer(peer_addr).await?;
handle.upsert_peer(node_id, addr_info).await?;
handle.shutdown().await?;
```

### Router

Pure decision engine — returns `RoutingAction` enum:

```rust
// Router decides: Deliver / Forward / Reject / Ack / ReadReceipt / Drop
let action = router.route(envelope, &topology);
match action {
    RoutingAction::Deliver(env) => { /* local delivery */ }
    RoutingAction::Forward(env, next_hop) => { /* relay */ }
    RoutingAction::Ack(msg_id, to) => { /* send ACK */ }
    _ => {}
}
```

### GroupManager + GroupHub

Hub-and-spoke group messaging:
- **GroupManager**: member-side state machine (join, leave, receive)
- **GroupHub**: hub-side fan-out, rate limiting (5 msg/sec/sender)
- **Hub election**: deterministic (lowest NodeId)
- **Hub failover**: Primary → Shadow → Candidate cascade (active watchdog, 3s ping, ~6s promote)
- **E2E**: Sender key encryption, key rotation on member leave

### Self-Send Interception

When `hub == local_id`, the runtime intercepts self-addressed operations locally instead of sending via QUIC network. Applies to: CreateGroup, SendGroupMessage, AcceptInvite, LeaveGroup, heartbeat tick.

## Message Flow

```
Sender → Router → Envelope (encrypt+sign) → QUIC → [Relay] → QUIC → Router → Recipient
                                                       ↓
                                                 Forward only
                                                 (no storage)
                                                       ↓
                                              Verify signature
                                              Route to next hop
```

Direct upgrade: after initial relay coordination, MagicSock upgrades to direct QUIC (hole punching).

## Implementation Patterns

### File Naming (Rust)
- `snake_case.rs` for files
- `PascalCase` for types
- Co-located tests: `#[cfg(test)] mod tests` in same file

### Error Handling
```rust
use tom_protocol::error::TomProtocolError;

return Err(TomProtocolError::PeerUnreachable(node_id));
```

### Effect Pattern (RuntimeState)
RuntimeState methods return `Vec<RoutingAction>` (effects), the runtime loop executes them:
```rust
let effects = state.handle_incoming(envelope);
for effect in effects {
    execute_effect(effect, &node, &state).await;
}
```

### Critical API Notes
- `Topology.upsert()` not `update_peer()`, `Topology.get()` not `get_peer()`
- `PeerInfo.last_seen` is `u64` (timestamp ms), NOT `Instant`
- NEVER wrap TomNode in `Arc<Mutex>` — deadlocks. Use single tokio task with `select!`
- Self-addressed ops need explicit local handling, not network round-trip
- `NodeId` has no `from_bytes` — use `SecretKey::generate(rng).public().to_string().parse()`

## Testing

### Rust Tests
```bash
cargo test --workspace              # All Rust tests (~700+)
cargo test -p tom-protocol          # Protocol tests only (346)
cargo test -p tom-quinn-proto       # QUIC proto tests (322)
cargo clippy --workspace -- -D warnings  # Lint (ALWAYS before push)
```

### TypeScript Tests (legacy)
```bash
pnpm test                           # All TS tests (771)
```

### Stress Testing
```bash
# Local campaign (30 scenarios)
cargo run -p tom-stress -- campaign --local

# Remote campaign (Mac ↔ NAS)
cargo run -p tom-stress -- campaign --responder-addr <NAS_ADDR>

# Cross-compile for NAS (ARM64)
cargo zigbuild -p tom-stress --target aarch64-unknown-linux-musl --release
```

### Test Counts
- 346 tests (tom-protocol)
- 322 tests (tom-quinn-proto)
- 22 tests (tom-quinn)
- 9 tests (tom-relay) + 12 tests (tom-gossip) + 4 tests (tom-metrics) + 2 tests (tom-base)
- 771 tests (TypeScript core, legacy)

## Deployment

### Own Relay (NAS)
```bash
# Freebox NAS — Debian VM, ARM64 Cortex-A72
ssh root@192.168.0.21               # Local
ssh root@82.67.95.8                 # Remote (port 22 redirect)

# tom-relay running on port 3340 (HTTP, no TLS)
/root/tom-relay --dev

# Environment variable for clients
TOM_RELAY_URL=http://192.168.0.21:3340

# Port forwarding: UDP 3340 → 82.67.95.8:3340 (public)
```

### Cross-compile ARM64
```bash
cargo zigbuild -p tom-stress --target aarch64-unknown-linux-musl --release
# SCP binary while process is running → "dest open Failure" — kill first
```

### n0_discovery Flag
```rust
// With N0 preset (Pkarr/DNS) — default
TomNodeConfig::new().n0_discovery(true).bind().await?;

// Without N0 (own relay only, no external deps)
TomNodeConfig::new().n0_discovery(false).bind().await?;
```

## Current Status

### TypeScript (Phase 1 — Complete)

| Epic | Description | Status |
|------|-------------|--------|
| 1-8 | Full protocol stack | ✅ Complete (771 tests) |

### Rust Native (Phase 2 — Active)

| Phase | Description | Status |
|-------|-------------|--------|
| R1 | Foundations (envelope, crypto, types) | ✅ Complete |
| R2 | Routing + ProtocolRuntime | ✅ Complete |
| R3 | Discovery + Keepalive (gossip) | ✅ Complete |
| R4 | Backup + Roles | ✅ Complete |
| R5 | Groups (hub, failover, sender keys, security) | ✅ Complete |
| R6 | TUI + Integration + Stress campaigns | ✅ Complete |
| R7 | Fork + Bootstrap Elimination | ✅ Complete |
| R8 | Production Hardening | Next |

### Stress Test Results
- Campaign V5: 250/250 Mac ↔ NAS (100% success)
- Campaign self-send fix: 232/232 Mac ↔ NAS (SSH tunnel)
- PoC hole punch: 100% across LAN/4G CGNAT/cross-border

## Important Notes for LLMs

1. **Fork is complete**: All critical iroh deps forked to `tom-*` namespace (Phase R7)
2. **Relays don't store**: Pass-through only, no persistence
3. **E2E is mandatory**: All messages encrypted (XChaCha20-Poly1305)
4. **Roles are network-assigned**: Nodes don't choose to be relays
5. **No blockchain**: This is a transport protocol, not a ledger
6. **Contribution matters**: Usage/contribution score affects role assignment
7. **Wire invariants are sacred**: Never change `_iroh` prefixes, ALPN, TLS SNI
8. **ed25519-dalek pin**: MUST use `=3.0.0-pre.1` (crypto type compat with quinn)
9. **Signaling server is DEPRECATED**: Use own relay + Pkarr/DNS

## Quick Commands

```bash
# Rust development
cargo build --workspace            # Build all
cargo test --workspace             # Test all
cargo clippy --workspace -- -D warnings  # Lint

# Run TUI chat
cargo run -p tom-tui -- --username alice

# Run stress test
cargo run -p tom-stress -- campaign --local

# Run relay (dev mode)
cargo run -p tom-relay -- --dev

# Cross-compile for ARM64
cargo zigbuild -p tom-stress --target aarch64-unknown-linux-musl --release

# TypeScript (legacy)
pnpm install && pnpm build && pnpm test
```
