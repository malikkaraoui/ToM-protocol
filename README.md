# ToM Protocol

**The Open Messaging** — a decentralized P2P transport protocol where every device is the network.

No central servers. No data centers. No tokens. Every phone, laptop, TV box and NAS that runs ToM is simultaneously a **client**, a **relay** and a **backup node**. Messages find their way through the devices of the network itself — end-to-end encrypted, signed, and gone after 24 hours.

> ToM is a **transport layer**, not a product. Think TCP/IP, not WhatsApp: a universal foundation that apps build on, invisible to end users.

---

## Table of contents

- [Why ToM exists](#why-tom-exists)
- [How it works in 60 seconds](#how-it-works-in-60-seconds)
- [Status](#status)
- [What's proven (with data)](#whats-proven-with-data)
- [Integrate ToM — three doors](#-integrate-tom--three-doors)
- [Quick start](#quick-start)
- [Architecture](#architecture)
- [Core concepts](#core-concepts)
- [Security model](#security-model)
- [Resilience](#resilience)
- [Repository structure](#repository-structure)
- [Testing](#testing)
- [Deployment](#deployment)
- [Known limitations & non-goals](#known-limitations--non-goals)
- [Contributing](#contributing)
- [Documentation](#documentation)

---

## Why ToM exists

Modern messaging depends on someone else's computer. Centralized messengers route your words through data centers you don't control, under jurisdictions you didn't choose, with an energy and infrastructure cost that only makes sense at ad-scale. Federated alternatives still need servers; blockchain messengers turn conversation into speculation.

ToM starts from a different observation: **billions of connected devices sit idle most of the day.** A phone on WiFi, an Apple TV in a living room, a NAS in a closet — together they already form a global mesh. ToM is the protocol that turns that dormant capacity into a resilient, virtually free communication bus:

- **No central servers** — messages route through peer relays; any node can be one.
- **Relay statelessness** — relays forward, they never store. Pass-through only.
- **End-to-end encryption** — only sender and recipient can read content. Always on, not an option.
- **Ephemeral by design** — 24h TTL, then global purge. No infinite history, anywhere.
- **Self-organizing** — roles (relay, backup, observer…) are assigned by the network based on contribution, not by configuration.
- **Zero-config** — nodes discover each other with no bootstrap peer, no account, no privileged node (see [Zero-config discovery](#zero-config-discovery-adr-010)).

## How it works in 60 seconds

1. **Identity = keypair.** A node generates an Ed25519 keypair; the public key *is* its network address (`NodeId`). No registration, no phone number, no server-side account.
2. **Discovery is zero-config.** Nodes find each other through several independent channels: local mDNS, gossip (HyParView), and a shared DHT rendezvous (BitTorrent Mainline, BEP-0044) that requires no prior knowledge of any peer.
3. **First contact goes through a relay.** Any reachable node can serve as a stateless relay. The relay coordinates the QUIC handshake between two peers behind NATs.
4. **Then the connection goes direct.** MagicSock hole-punching upgrades the relayed connection to a direct QUIC path when the NATs allow it (validated across LAN, 4G CGNAT and cross-border links).
5. **Every message is encrypted then signed.** X25519 Diffie-Hellman + XChaCha20-Poly1305 for confidentiality, Ed25519 for authenticity — the relay sees opaque bytes and a routing header, nothing else.
6. **Delivery means ACK.** A message counts as delivered if and only if the recipient emits a *signed* acknowledgment. If the recipient is offline, the message self-replicates across backup nodes ("positive virus") and self-deletes on delivery — or after 24h, whichever comes first.
7. **Groups are hub-and-spoke with automatic failover.** A deterministic hub fans messages out; a shadow hub watches it and promotes itself in ~3–6s if the hub dies. No consensus round, no split-brain.

## Status

| Phase | Description | Status |
|-------|-------------|--------|
| **Phase 1** | TypeScript protocol stack (WebRTC, signaling) | ✅ Complete — 8/8 epics, 771 tests (legacy, archived) |
| **Phase 2** | Rust native protocol (QUIC, hole punching, E2E crypto, groups, DHT) | ✅ **R1–R12 complete** |
| **Phase 3** | SDKs, public specs & hardening — make ToM yours | 🚀 Active |

**Phase 2 milestones, all shipped:** envelope + crypto foundations (R1), routing + runtime (R2), gossip discovery + keepalive (R3), backup + roles (R4), groups with hub failover + sender keys (R5), TUI + stress campaigns (R6), full iroh fork + bootstrap elimination (R7), production hardening (R8), DHT + delivery reliability (R9), group recovery (R10), security & admin — anti-spam, anti-replay, group admin (R11), zero-config DHT rendezvous + isolation recovery (R12).

**Current work (Phase 3):** hardening the live multi-device fleet (Mac, iPad, iPhone, Apple TV, NAS running daily), improving the same-LAN direct path (IPv6 hole punching through client-isolating WiFi), and sender-side flow control. Progress is tracked in [`docs/plans/`](docs/plans/).

## What's proven (with data)

Everything below was measured on real hardware and real networks — not simulations.

| Test | Result | Details |
|------|--------|---------|
| NAT hole punching | **100% success** | LAN, 4G CGNAT, cross-border Switzerland↔France |
| Stress test (4G highway) | **99.85%** | 2748/2752 pings, 54 min continuous on the A40, surviving tunnels and cell handoffs |
| Campaign V5 (channel pump) | **100%** — 250/250 Mac↔NAS | Deadlock class eliminated by background channel pump |
| E2E encrypted chat | **Working** | Signed + encrypted envelopes, Mac↔NAS cross-border |
| Group messaging + hub failover | **Working** | Deterministic failover, ~3–6s detection, chain restore |
| Direct QUIC latency | **27–49 ms** | After hole punch, no relay in the path |
| Multi-device live fleet | **iPhone (3G/4G/5G) ↔ iPad (WiFi) ↔ Apple TV ↔ MacBook ↔ NAS** | Real cellular reconnection, decentralized DHT rendezvous, no fixed relay |
| Large messages (chunking) | **1 KB → 64 MB delivered** | Auto-segmentation over QUIC, reassembled end-to-end; verified at 300 KB / 3 MB / 10 MB / 64 MB |
| Offline recipient + backup (ADR-009) | **8/8 delivered in 3 s on return** | Messages survive recipient offline, redelivered on rejoin, purged after ACK — no duplicates |
| Node rejoin after restart | **~6–10 s** | Kill → relaunch → back on the network (LAN) |

### NAT traversal detail

Tested with the `tom-stress` binary, cross-compiled to static ARM64, deployed on a Freebox Delta NAS (Debian, Cortex-A72):

| Scenario | Topology | Hole punch time | Direct RTT | Direct % |
|----------|----------|-----------------|------------|----------|
| LAN WiFi | Same network | 0.37 s | 49 ms | 100% |
| 4G CGNAT | iPhone hotspot ↔ home WiFi | 2.9 s | 107 ms | 90% |
| Cross-border | School WiFi (CH) ↔ Freebox (FR) | 1.4 s | 32 ms | 95% |

## 🚪 Integrate ToM — three doors

### 1 · Rust — `tom-sdk` (15 lines to a live node)

```toml
[dependencies]
tom-sdk = { git = "https://github.com/malikkaraoui/ToM-protocol", tag = "v0.3.0" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use tom_sdk::{Event, TomClientBuilder};

#[tokio::main]
async fn main() -> Result<(), tom_sdk::TomSdkError> {
    let mut client = TomClientBuilder::new().username("alice").connect().await?;
    println!("my identity: {}", client.id());
    while let Some(event) = client.next_event().await {
        if let Event::MessageReceived(msg) = event {
            client.send_text(msg.from, "got it!").await?; // E2E encrypted, signed
        }
    }
    Ok(())
}
```

No infrastructure needed on a LAN: exchange opaque **connectivity tickets** (`client.ticket()` ↔ `client.add_peer_ticket(...)`) — QR code or copy/paste. Full guide: [`crates/tom-sdk/README.md`](crates/tom-sdk/README.md) · runnable examples in [`crates/tom-sdk/examples/`](crates/tom-sdk/examples/).

### 2 · Apple — `TomProtocolKit` (iOS 16+ / tvOS 16+ / macOS 13+)

Swift Package wrapping the Rust core (XCFramework, 5 slices). Build once, add as a local package in Xcode:

```bash
bash scripts/build-tom-protocol-ffi-xcframework.sh
bash scripts/sync-xcframework-to-package.sh
# Xcode → File → Add Package Dependencies → Add Local… → sdk/swift/TomProtocolKit
```

Releases (zip + SPM checksum) are automated on `sdk-swift/v*` tags. Guide: [`sdk/swift/TomProtocolKit/README.md`](sdk/swift/TomProtocolKit/README.md). The unified multi-platform app in [`apps/tom-node-tvos/`](apps/tom-node-tvos/) (iOS, tvOS, macOS targets) is the reference integration — it's the app running on the live fleet.

### 3 · Any language — implement the protocol

Normative specs, no Rust required, byte-for-byte verifiable:

- [`docs/spec/tom-wire-v1.md`](docs/spec/tom-wire-v1.md) — envelope wire format (MessagePack), signatures, TTL, relay rules
- [`docs/spec/tom-crypto-v1.md`](docs/spec/tom-crypto-v1.md) — Ed25519/X25519, HKDF, XChaCha20-Poly1305, encrypt-then-sign
- [`docs/spec/vectors/`](docs/spec/vectors/) — self-verified test vectors (expected bytes + intermediate values)

## Quick start

### Rust SDK (recommended)

```bash
git clone https://github.com/malikkaraoui/ToM-protocol.git && cd ToM-protocol

# Two local nodes exchange an E2E-encrypted message via tickets — no infra
cargo run -p tom-sdk --example 01_send_message
# Group chat: invite, join, fan-out
cargo run -p tom-sdk --example 02_group_chat
# Self-hosted relay, zero external dependency
cargo run -p tom-relay -- --dev   # then:
RELAY=http://localhost:3340 cargo run -p tom-sdk --example 03_own_relay
```

### Native P2P chat (TUI)

```bash
cargo build --release -p tom-tui

./target/release/tom-chat <peer-node-id>   # TUI chat, connect to a peer
./target/release/tom-chat --bot            # headless bot mode (auto-responds)
```

### TypeScript demo (browser, legacy)

```bash
pnpm install && pnpm build
./scripts/start-demo.sh                    # opens http://localhost:5173
```

## Architecture

### Transport stack (Rust)

```
Application
    ↓
tom-protocol   (ProtocolRuntime)   ← protocol logic, groups, encryption, backup, roles
    ↓
tom-transport  (QUIC connectivity) ← connection lifecycle, hole punching
    ↓
tom-connect    (Endpoint/MagicSock)← NAT traversal, Disco, relay fallback
    ↓
tom-quinn / tom-quinn-proto        ← QUIC runtime + wire protocol
    ↓
UDP
```

### Protocol runtime

```
ProtocolRuntime (single tokio::select! loop + background channel pump)
├── Router           — pure decision engine: deliver / forward / reject / ack / drop
├── Topology         — peer state, heartbeat tracking
├── EnvelopeBuilder  — encrypt-then-sign, MessagePack wire format
├── GroupManager     — member-side multi-party state + shadow watchdog
├── GroupHub         — hub-side fan-out, Primary→Shadow→Candidate chain
├── BackupStore      — TTL-bounded "virus" backup for offline peers
├── RelaySelector    — relay selection from live topology
└── HeartbeatTracker — stale/offline detection (gossip IS the keepalive)
```

The runtime uses an **effect pattern**: state transitions return a list of routing actions, and the single event loop executes them. No shared-state locking, no `Arc<Mutex>` around the node — deadlock-free by construction (proven the hard way in stress campaigns).

### A sovereign fork of iroh

ToM was born as a fork of [iroh](https://github.com/n0-computer/iroh) 0.96 and is now **autonomous**. All critical dependencies live in this repo under the `tom-*` namespace (MIT):

| Original | Fork | Approx. LOC | Role |
|----------|------|------|-------|
| iroh (endpoint + socket) | `tom-connect` | ~15K | MagicSock, Disco, hole punching |
| iroh-relay | `tom-relay` | ~8K | Stateless relay server |
| iroh-gossip | `tom-gossip` | ~5K | Gossip / membership |
| iroh-quinn | `tom-quinn` | ~6.5K | QUIC runtime |
| iroh-quinn-proto | `tom-quinn-proto` | ~41K | QUIC wire protocol |
| iroh-base | `tom-base` | ~800 | PublicKey, SecretKey, NodeAddr |
| iroh-metrics | `tom-metrics` | ~100 | Metrics counters |

All protocol identifiers are under the ToM namespace (DNS `_tom`, TLS SNI `.tom.invalid`, HTTP `X-Tom-*` headers, ALPN `tom-protocol/transport/0` and `/tom-gossip/1`). Consequence, fully assumed: **ToM is not wire-compatible with the public iroh network** — iroh is the historical starting point, not a network dependency. Governance and invariants: [`docs/FORK-GOVERNANCE.md`](docs/FORK-GOVERNANCE.md).

### Dual stack (Phase 1 vs Phase 2)

| Layer | TypeScript (Phase 1, legacy) | Rust (Phase 2, current) |
|-------|---------------------|----------------|
| **Identity** | Ed25519 (TweetNaCl.js) | Ed25519 (ed25519-dalek) |
| **Transport** | WebRTC DataChannel | QUIC + hole punching |
| **Encryption** | X25519 + XSalsa20-Poly1305 | X25519 + XChaCha20-Poly1305 + HKDF-SHA256 |
| **Discovery** | Gossip + ephemeral subnets | mDNS + gossip (HyParView) + DHT rendezvous + optional Pkarr |
| **Wire format** | JSON envelopes | MessagePack (signed + encrypted) |
| **Routing** | Dynamic relay selection | Router + RelaySelector + ProtocolRuntime |

## Core concepts

### 7 locked design decisions

These are non-negotiable and define ToM's character; all code is reviewed against them.

| # | Decision | Rule |
|---|----------|------|
| 1 | **Delivery** | A message is delivered ⟺ the recipient emits a signed ACK |
| 2 | **TTL** | 24h max lifespan, then global purge — no exceptions |
| 3 | **L1 role** | L1 anchors state, never arbitrates |
| 4 | **Reputation** | Progressive fade, no permanent bans |
| 5 | **Anti-spam** | "The sprinkler gets sprinkled" — progressive load-shedding, never exclusion |
| 6 | **Invisibility** | The protocol layer is invisible to end users |
| 7 | **Scope** | Universal foundation (like TCP/IP), not a product |

### Dynamic roles, not configuration

Every node runs **identical code**. The network assigns roles — client, relay, backup, observer — based on topology and contribution. A node that relays well gets more relay duty; a node that misbehaves fades in reputation (and can always fade back in). There is no "server build" and no operator switch to flip.

### Backup as a positive virus (ADR-009)

When a recipient is offline, the message replicates across backup nodes like a benign infection: it spreads enough copies to survive churn, then **self-destructs** the moment a signed ACK proves delivery — or when the 24h TTL expires. Storage is bounded, ephemeral and epidemic, not archival.

### Proof of Presence

No energy-hungry proof-of-work, no plutocratic proof-of-stake. Nodes attest each other's presence with signed, latency-bounded challenges. You earn standing in the network by *being there and behaving well* — that's it.

### Zero-config discovery (ADR-010)

The hardest problem in serverless P2P is the first peer. ToM solves it with a **shared DHT rendezvous**: a constant namespace derives a small set of well-known slots in the BitTorrent Mainline DHT; every node publishes its signed `{node_id, addrs}` into its slot and reads all slots to find live peers — **zero prior knowledge, no bootstrap peer, no privileged node**. Entries are Ed25519-authenticated, so the rendezvous cannot be squatted with forged peers. On top of that: mDNS for the local network, gossip for steady-state, and an embedded relay is announced globally only if it is verifiably reachable from outside the LAN.

## Security model

- **Encrypt-then-sign, always.** X25519 ECDH + HKDF-SHA256 derive per-pair keys; XChaCha20-Poly1305 encrypts; Ed25519 signs the envelope. Relays verify signatures and routing headers, never plaintext.
- **Signed ACKs.** Delivery status is only granted on a cryptographically verified acknowledgment — a forged or unsigned ACK is rejected, so an attacker can't mark messages "delivered" out from under the sender.
- **Nonce anti-replay.** A bounded LRU nonce cache (24h TTL, aligned with message TTL) rejects replayed envelopes.
- **Authenticated rendezvous.** DHT rendezvous entries carry an Ed25519 proof of possession of the announced `node_id`; unauthenticated or tampered entries are dropped.
- **DoS hardening from red-teaming.** Iterative adversarial audits produced fixes now covered by regression tests: bounded chunk reassembly (per-message and global memory budgets), serialization inflation fixes for large payloads, per-identity rate limits *with* global caps, group hub rate limiting (5 msg/s/sender) with grace for known reconnecting peers.
- **Supply chain.** `cargo deny` (RustSec advisories, license and source bans) runs in CI; the crypto stack pins `ed25519-dalek` for type compatibility across the forked QUIC stack.

Threat-model notes and remaining open items are tracked honestly in [`CLAUDE.md` → Known Limitations](CLAUDE.md#known-limitations-audit-adversarial-2026-06-22).

## Resilience

### Hub failover (groups)

| Component | Mechanism | Recovery time |
|-----------|-----------|---------------|
| **Primary hub** | Hosts the group, fans out messages | — |
| **Shadow hub** | Active watchdog, pings primary every 3 s | Auto-promotes after 2 missed pings (~6 s) |
| **Candidate** | Deterministic self-election (lowest NodeId) | Becomes shadow after promotion |
| **Chain restore** | Shadow→Primary, Candidate→Shadow | Automatic, no user intervention |

Fast path: 1 missed ping + 1 `HubUnreachable` report from a member → promote in ~3 s. The hub is stateless (only the member list and group config are replicated), and deterministic election prevents split-brain without any consensus protocol.

### Isolation recovery

A node that loses connectivity — or holds only **zombie connections** (connected sockets with no inbound traffic) — detects it via a liveness window, reverts its bootstrap phase, and runs a fresh discovery round: relay reprobe, DHT republish, rendezvous read, gossip rejoin. Sleep/wake on Apple platforms triggers a full restart-and-rediscover cycle.

### Storm-proof maintenance

All periodic maintenance timers (republish, rejoin, backup sweep…) skip missed ticks instead of bursting after a stall — a device resuming from suspension rejoins smoothly instead of hammering the network with a backlog of timer fires.

## Repository structure

```
tom-protocol/
├── crates/                          # Rust native stack (the real ToM)
│   ├── tom-sdk/                     # 🚪 High-level SDK (start here)
│   ├── tom-protocol/                # Protocol engine: crypto, routing, groups, discovery, backup, roles
│   ├── tom-transport/               # QUIC transport layer, hole punching
│   ├── tom-connect/                 # Endpoint/MagicSock (iroh fork): NAT traversal, Disco, relay fallback
│   ├── tom-relay/                   # Stateless relay server (--dev mode for local)
│   ├── tom-gossip/                  # Gossip membership/broadcast (HyParView)
│   ├── tom-dht/                     # Mainline DHT discovery + zero-config rendezvous (BEP-0044)
│   ├── tom-quinn/ tom-quinn-proto/  # Forked QUIC runtime + wire protocol
│   ├── tom-quinn-udp/ tom-base/ tom-metrics/  # Forked support crates
│   ├── tom-protocol-ffi/            # C ABI for native apps (cbindgen header)
│   ├── tom-relay-ffi/               # C ABI for embedding a relay (Swift/tvOS)
│   ├── tom-gateway/                 # CLI to auto-configure a Freebox router for relaying
│   ├── tom-tui/                     # `tom-chat`: TUI chat client + headless bot mode
│   ├── tom-stress/                  # Stress-test campaigns (Mac ↔ NAS, LAN/WAN)
│   └── tom-integration-tests/       # Cross-crate integration tests
│
├── sdk/swift/TomProtocolKit/        # 🚪 Swift Package (iOS/tvOS/macOS)
├── apps/
│   ├── tom-node-tvos/               # Unified native app (iOS + tvOS + macOS targets) — the live fleet
│   ├── infra-web-client/            # Infra dashboard web client (Vite)
│   └── demo/                        # Browser demo (Phase 1, legacy)
│
├── docs/spec/                       # 🚪 Normative specs + test vectors
├── docs/plans/                      # Design docs, ADRs-in-progress, chantier journals
├── packages/                        # TypeScript stack (Phase 1 — legacy)
├── llms.txt                         # LLM quick reference
├── CLAUDE.md                        # Detailed implementation guide (humans welcome too)
└── CONTRIBUTING.md                  # Micro-session contribution model
```

## Testing

Over **1,000 automated tests** across the Rust workspace (protocol, QUIC fork, DHT, transport, relay, gossip), plus 771 TypeScript tests on the legacy stack, plus real-hardware stress campaigns.

```bash
cargo test --workspace                    # Rust tests
cargo clippy --workspace -- -D warnings   # lint gate (mandatory before every push)
cargo deny check advisories bans sources  # supply chain
pnpm test                                 # TypeScript (legacy)

# Stress campaigns (real network)
cargo run -p tom-stress -- campaign --local
cargo run -p tom-stress -- campaign --responder-addr <REMOTE_ADDR>
```

The DHT crate includes chaos tests (node churn, full blackout recovery) that run against the *real* BitTorrent Mainline testnet. Delivery guarantees (ADR-009 backup/redelivery) are locked by deterministic endurance tests.

## Deployment

ToM's reference "infrastructure" is deliberately humble — that's the point:

- **Relay on anything.** `cargo run -p tom-relay -- --dev` gives you a relay on `http://localhost:3340`. Point nodes at it with `TOM_RELAY_URL`. The production reference relay runs on a **Freebox Delta NAS** (Debian VM, ARM64 Cortex-A72, <1 GB RAM).
- **Cross-compile static ARM64** for NAS / Raspberry Pi class hardware:
  ```bash
  cargo zigbuild -p tom-tui --target aarch64-unknown-linux-musl --release
  ```
- **Router auto-config.** `tom-gateway` is a CLI that configures a Freebox router (port forwarding) for relay duty.
- **No external dependency required.** Pkarr/DNS discovery (`n0_discovery`) is an optional preset, off by default in sovereign deployments; the DHT rendezvous and your own relay cover discovery end-to-end.

## Known limitations & non-goals

Honesty section — what ToM does *not* do, on purpose or not yet:

- **iOS/tvOS background operation is a non-goal.** When the app is suspended, the node is frozen — by design. ToM does not ship a background daemon, push notifications (APNs/VoIP) or a permanent wakeup: an always-on, battery-hungry service would contradict the invisible-layer vision. The model: while you use the network (foreground), your device contributes fully; in background it gets at most a short grace period. The 24h backup replication is the safety net — the network holds messages for you while your device sleeps.
- **Same-LAN direct path is being hardened.** On WiFi networks with client isolation, nodes may stay on the relay path instead of upgrading to a direct connection (IPv6 hole punching through such networks is active work). Messages still flow — through the relay — but with a latency detour.
- **Mesh is connected, not complete.** Gossip guarantees a *connected* graph via hubs, not an N² full mesh — deliberate, to protect weak devices (an Apple TV shouldn't hold 50 QUIC connections).
- **The TypeScript stack is legacy.** Phase 1 is kept for reference and the browser demo; all active development is Rust.

The full adversarial-audit ledger (what was found, what's fixed with `file:line` evidence, what remains) lives in [`CLAUDE.md` → Known Limitations](CLAUDE.md#known-limitations-audit-adversarial-2026-06-22).

## Contributing

ToM uses a **micro-session contribution model** — small, focused changes completable in 30–60 minutes, with a hard validation gate (clippy + full workspace tests) before every push. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentation

- [docs/spec/](docs/spec/) — **Normative protocol specs + test vectors** (implement ToM in any language)
- [crates/tom-sdk/README.md](crates/tom-sdk/README.md) — Rust SDK guide
- [sdk/swift/TomProtocolKit/README.md](sdk/swift/TomProtocolKit/README.md) — Apple SDK guide
- [docs/FORK-GOVERNANCE.md](docs/FORK-GOVERNANCE.md) — iroh fork governance & wire invariants
- [docs/plans/](docs/plans/) — design docs and chantier journals (the project's engineering diary)
- [CLAUDE.md](CLAUDE.md) — implementation guide for AI assistants (and curious humans)
- [llms.txt](llms.txt) — protocol quick reference
- [Architecture](_bmad-output/planning-artifacts/architecture.md) — ADRs and design decisions
- [Design Decisions](_bmad-output/planning-artifacts/design-decisions.md) — the 7 locked invariants

## License

MIT
