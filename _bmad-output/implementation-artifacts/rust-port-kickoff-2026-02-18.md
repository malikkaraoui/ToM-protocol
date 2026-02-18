# Rust Port Kickoff — 18 February 2026

## Meeting Context

Planning meeting for porting the ToM protocol layer from TypeScript to Rust, on top of the validated `tom-transport` crate (QUIC via iroh, 99.5% reliability on 4G CGNAT).

Participants: Winston (Architect), Amelia (Dev), John (PM), Quinn (QA), Mary (Analyst), Bob (SM), Sally (UX Designer).

Reference: [Campaign Report V4](../../results/CAMPAIGN-REPORT.md)

---

## Locked Decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| Crypto — Signing | Ed25519 via iroh `NodeId` | Already used by iroh; zero custom key generation |
| Crypto — Key exchange | X25519 (derived from Ed25519) | Standard curve conversion, `x25519-dalek` crate |
| Crypto — Symmetric encryption | XChaCha20-Poly1305 | Modern standard (RFC 8439 extended), 24-byte nonce (safe random generation), aligned with QUIC/TLS 1.3 |
| Wire format | MessagePack (serde + rmp-serde) | Standard, community-friendly, ~40% more compact than JSON |
| TypeScript coexistence | No — cut the cord | No need for JSON wire format compatibility |
| Gossip discovery | iroh native (replace PeerGossip TS) | Already validated in PoC-2/3, solves bug #4 (listener stale) |
| Demo | TUI with ratatui (Rust) | 100% Rust, cross-compiles to ARM64, terminal-native |
| Testing | Stress test per epic, proptest, regression suite | Every feature ships with its stress test |

## Crypto Crates

```toml
ed25519-dalek = { version = "2", features = ["rand_core"] }
x25519-dalek = { version = "2", features = ["static_secrets"] }
chacha20poly1305 = "0.10"   # XChaCha20Poly1305
rand = "0.8"
```

Key insight: iroh's `NodeId` IS an Ed25519 public key. No custom identity generation needed — reuse iroh's keypair for both transport and protocol signing.

---

## Roadmap

| Phase | Epic | Deliverable | Stress test mode |
| --- | --- | --- | --- |
| 1 | Foundations | `tom-protocol` crate: Envelope (MsgPack), types, errors | `crypto-bench` |
| 2 | Routing | 3 nodes: A -> relay C -> B, E2E encrypted, ACK | `route` |
| 3 | Discovery + Keepalive | Gossip iroh integration, bug #4 resolution | `discovery` |
| 4 | Backup + Roles | Offline delivery, dynamic role assignment | `offline` |
| 5 | Groups | GroupManager, GroupHub, HubElection, GroupSecurity | `fanout` |
| 6 | TUI + Integration | ratatui demo, full scenario automation | `scenario` |

**MVP = Phase 2 complete.** Two nodes exchanging E2E encrypted messages via a relay, with ACK confirmation.

---

## What We Port vs. What We Don't

### NOT ported (replaced or eliminated)

| TS Module | Reason |
| --- | --- |
| `transport/` (WebRTC) | Replaced by `tom-transport` (QUIC) |
| `bootstrap/` | Replaced by iroh gossip |
| `direct-path-manager.ts` | iroh handles direct paths natively |
| `offline-detector.ts` | Integrated in gossip (presence = active gossip) |
| `ephemeral-subnet.ts` | Out of MVP — research, not critical |
| SDK TypeScript | Cord cut |
| Demo web | Replaced by TUI ratatui |
| Signaling server | Dead — gossip is its successor |

### Ported (adapted)

| TS Module | Rust Target | Adaptation |
| --- | --- | --- |
| `types/envelope.ts` | `tom-protocol::envelope` | Binary format (MessagePack) |
| `errors/` | `tom-protocol::error` | `thiserror` crate |
| `identity/` | Reuse iroh `NodeId` | Zero custom code |
| `crypto/` | `tom-protocol::crypto` | XChaCha20-Poly1305 (was XSalsa20) |
| `routing/router.ts` | `tom-protocol::router` | Adapted for QUIC streams |
| `routing/relay-selector.ts` | `tom-protocol::relay` | Same logic, async |
| `routing/message-tracker.ts` | `tom-protocol::tracker` | Channels instead of callbacks |
| `discovery/network-topology.ts` | `tom-protocol::topology` | DashMap instead of Map |
| `discovery/peer-gossip.ts` | iroh gossip native | Zero porting |
| `discovery/heartbeat.ts` | Integrated in gossip | Simplified |
| `roles/role-manager.ts` | `tom-protocol::roles` | Same logic |
| `backup/*` | `tom-protocol::backup` | 4 files, fairly direct |
| `groups/*` | `tom-protocol::groups` | 6 files, dedicated epic |

---

## Epic 5 Detail — Groups (dedicated epic)

| Story | Source TS | Rust Target | Complexity |
| --- | --- | --- | --- |
| 5.1 Group types | `group-types.ts` | `groups/types.rs` | Low — enum + serde |
| 5.2 GroupManager | `group-manager.ts` | `groups/manager.rs` | High — ~400 LOC, state machine |
| 5.3 GroupHub | `group-hub.ts` | `groups/hub.rs` | Medium — fanout logic |
| 5.4 HubElection | `hub-election.ts` | `groups/election.rs` | Low — deterministic (lowest NodeId) |
| 5.5 GroupSecurity | `group-security.ts` | `groups/security.rs` | Medium — nonce tracking, anti-replay |

---

## Definition of Done (per story)

- [ ] Code compiles (`cargo check`)
- [ ] Unit tests pass (`cargo test -p tom-protocol`)
- [ ] Proptest for serde types and crypto
- [ ] Integration tests for multi-node interactions
- [ ] Corresponding `tom-stress` mode added (if applicable)
- [ ] Criterion benchmarks for critical paths
- [ ] Regression: ALL previous stress tests still pass
- [ ] No `unsafe` without documented justification
- [ ] Rustdoc on public types

---

## API Target

```rust
// Initialization
let node = TomNode::builder().bind().await?;
let my_id: NodeId = node.id();

// Send E2E encrypted message
node.send(recipient_id, payload).await?;

// Receive (decrypted)
let msg: IncomingMessage = node.recv().await?;

// Delivery status
node.on_status(|msg_id, status| { /* Pending -> Delivered */ });

// Discovery (automatic via iroh gossip)
node.on_peer_discovered(|peer_id, info| { ... });

// Groups
let group = node.create_group("Team", &[alice, bob]).await?;
node.send_group(group.id, payload).await?;

// Topology
let peers = node.peers();
let role = node.role();  // Client | Relay | Backup

// Shutdown
node.shutdown().await?;
```

---

## Key Architectural Decisions

1. **Async model:** tokio channels (mpsc) instead of callbacks — consistent with tom-transport
2. **Payload is opaque `Vec<u8>`:** protocol doesn't parse content, only routes and encrypts
3. **MessageType is an enum:** compile-time validation (not `String` like TS)
4. **TTL field added:** hop counter for relay depth control (locked decision #2: 24h max + hop limit)
5. **Single `encrypted: bool` flag:** replaces 3 optional fields from TS envelope
6. **No hop_timestamps in envelope:** monitoring lives outside the wire format

---

## Test Strategy

| Type | When | Tool | Example |
| --- | --- | --- | --- |
| Unit | During dev | `cargo test` | encrypt/decrypt roundtrip |
| Proptest | Per story | `proptest` crate | 1000 random envelopes serialize/deserialize |
| Integration | Story done | `cargo test --test integration` | 3 nodes on localhost |
| Stress | Story done | `tom-stress` (new mode) | 1000 messages in 60s |
| Benchmark | Story done | `criterion` | crypto throughput baseline |
| Regression | Every epic | `scripts/test-all.sh` | All previous modes replayed |
