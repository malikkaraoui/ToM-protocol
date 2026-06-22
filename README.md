# ToM Protocol

**The Open Messaging** — a decentralized P2P transport protocol where every device is the network.

## Status: Phase 3 — SDK & Adoption

| Phase | Description | Status |
|-------|-------------|--------|
| **Phase 1** | TypeScript protocol stack (WebRTC, signaling) | ✅ 8/8 epics (legacy) |
| **Phase 2** | Rust native protocol (QUIC, hole punching, E2E crypto, groups) | ✅ R1–R11 complete · 🚧 R12 zero-config DHT rendezvous + resilience ([phases](CLAUDE.md#current-status) · [known limitations](CLAUDE.md#known-limitations-audit-adversarial-2026-06-22)) |
| **Phase 3** | SDKs & public specs — make ToM yours | 🚀 Rust SDK, Swift Package and wire specs shipped |

**~1200 Rust + 771 TypeScript tests** | **E2E encrypted** | **NAT traversal validated** | **Cross-border Suisse↔France** | **Hub failover validated**

> Audit adversarial 2026-06-22 : trous ouverts connus (zombies, squatting DHT, suspension iOS…) — voir [Known Limitations](CLAUDE.md#known-limitations-audit-adversarial-2026-06-22).

## TL;DR

ToM is a transport layer protocol (not a blockchain) that transforms every connected device into both client and relay. No data centers, no speculative tokens, no infinite history.

**The idea:** leverage the dormant power of billions of devices to create a global communication BUS that's resilient and virtually free.

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

Releases (zip + SPM checksum) are automated on `sdk-swift/v*` tags. Guide: [`sdk/swift/TomProtocolKit/README.md`](sdk/swift/TomProtocolKit/README.md).

### 3 · Any language — implement the protocol

Normative specs, no Rust required, byte-for-byte verifiable:

- [`docs/spec/tom-wire-v1.md`](docs/spec/tom-wire-v1.md) — envelope wire format (MessagePack), signatures, TTL, relay rules
- [`docs/spec/tom-crypto-v1.md`](docs/spec/tom-crypto-v1.md) — Ed25519/X25519, HKDF, XChaCha20-Poly1305, encrypt-then-sign
- [`docs/spec/vectors/tom-vectors-v1.json`](docs/spec/vectors/tom-vectors-v1.json) — 7 self-verified test vectors (expected bytes + intermediate values)

## What's proven (with data)

| Test | Result | Details |
|------|--------|---------|
| NAT hole punching | **100% success** | LAN, 4G CGNAT, cross-border CH↔FR |
| Stress test (4G highway) | **99.85%** | 2748/2752 pings, 54 min continuous |
| Campaign V5 (channel pump) | **100% local, 250/250 Mac↔NAS** | Channel pump fix = definitive solution |
| E2E encrypted chat | **Working** | Signed + encrypted envelopes, Mac↔NAS cross-border |
| Group messaging + hub failover | **Working** | Virus-like replication, deterministic failover, ~3-6s detection |
| Direct QUIC latency | **27-49ms** | After hole punch, no relay needed |

## Project Structure

```
tom-protocol/
├── crates/                          # Rust native stack
│   ├── tom-sdk/                     # 🚪 High-level SDK (start here)
│   ├── tom-protocol/                # Protocol engine (crypto, routing, groups, discovery, backup)
│   ├── tom-transport/               # QUIC transport, hole punching
│   ├── tom-connect/ tom-relay/ …    # Forked iroh stack (tom-* namespace, see docs/FORK-GOVERNANCE.md)
│   ├── tom-protocol-ffi/            # C ABI for native apps (cbindgen header)
│   ├── tom-tui/                     # TUI chat client + bot mode
│   └── tom-stress/                  # Stress test campaigns
│
├── sdk/swift/TomProtocolKit/        # 🚪 Swift Package (iOS/tvOS/macOS)
│
├── docs/spec/                       # 🚪 Normative specs + test vectors
│
├── apps/                            # Native apps (iOS, tvOS, dev dashboard)
│
├── packages/                        # TypeScript stack (Phase 1 — legacy, archive planned)
│
├── docs/plans/                      # Design docs + chantier journals
├── llms.txt                         # LLM quick reference
├── CLAUDE.md                        # Detailed LLM guide
└── CONTRIBUTING.md                  # Micro-session contribution model
```

## Architecture

### Dual Stack

| Layer | TypeScript (Phase 1) | Rust (Phase 2) |
|-------|---------------------|----------------|
| **Identity** | Ed25519 (TweetNaCl.js) | Ed25519 (ed25519-dalek) |
| **Transport** | WebRTC DataChannel | QUIC (iroh) + hole punching |
| **Encryption** | X25519 + XSalsa20-Poly1305 | X25519 + XChaCha20-Poly1305 + HKDF-SHA256 |
| **Discovery** | Gossip + ephemeral subnets | Gossip (HyParView) + Pkarr |
| **Wire format** | JSON envelopes | MessagePack (signed + encrypted) |
| **Routing** | Dynamic relay selection | Router + RelaySelector + ProtocolRuntime |

### Rust Protocol Stack

```
ProtocolRuntime (single tokio::select! loop + background channel pump)
├── Router          — deliver / forward / reject / ack
├── Topology        — peer state, heartbeat tracking
├── EnvelopeBuilder — encrypt-then-sign, MessagePack wire format
├── GroupManager    — member-side multi-party + shadow watchdog
├── GroupHub        — hub-side fan-out, Primary→Shadow→Candidate chain
├── BackupStore     — TTL-based virus backup for offline peers
├── RelaySelector   — optimal relay selection
└── HeartbeatTracker — stale/offline detection (gossip IS keepalive)

Channel Architecture:
├── RuntimeChannels  — async mpsc channels (messages, events, status)
└── Background pump  — continuous drain prevents deadlocks
```

## Quick Start

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

### TypeScript Demo (browser, legacy)

```bash
git clone https://github.com/malikkaraoui/ToM-protocol.git
cd tom-protocol
pnpm install && pnpm build

# Run demo (opens browser + signaling server)
./scripts/start-demo.sh
# Open multiple tabs at http://localhost:5173
```

### Rust Chat (native P2P)

```bash
# Build
cargo build --release -p tom-tui

# Run TUI chat (connect to a peer)
./target/release/tom-chat <peer-node-id>

# Run as headless bot (auto-responds)
./target/release/tom-chat --bot

# Cross-compile for ARM64 (NAS, Raspberry Pi)
cargo zigbuild --target aarch64-unknown-linux-musl --release -p tom-tui
```

## NAT Traversal Results

Tested with `tom-stress` binary, cross-compiled ARM64 static, deployed on Freebox Delta NAS (Debian, Cortex-A72).

| Scenario | Topology | Hole punch time | RTT direct | Direct % |
|----------|----------|-----------------|------------|----------|
| LAN WiFi | Same network | 0.37s | 49ms | 100% |
| 4G CGNAT | iPhone hotspot ↔ home WiFi | 2.9s | 107ms | 90% |
| Cross-border | School WiFi (CH) ↔ Freebox (FR) | 1.4s | 32ms | 95% |

Stress test on highway (A40, France↔Switzerland): **99.85%** reliability over 2752 pings, 54 minutes continuous, surviving tunnel outages and cell tower handoffs.

## Hub Failover & Resilience

**Group messaging uses virus-like replication** for hub resilience:

| Component | Mechanism | Recovery time |
|-----------|-----------|---------------|
| **Primary hub** | Hosts group, manages members | N/A |
| **Shadow hub** | Active watchdog, pings primary every 3s | Auto-promotes on 2 missed pings (~6s) |
| **Candidate** | Deterministic self-election (lowest NodeId) | Becomes shadow after promotion |
| **Chain restore** | After promotion: Shadow→Primary, Candidate→Shadow | Automatic, no user intervention |

**Detection modes:**

- 2 missed pings → promote (~6s)
- 1 missed ping + 1 HubUnreachable from member → fast promote (~3s)

**Key properties:**

- Hub is stateless (only member list + config replicated)
- Deterministic election prevents split-brain
- No consensus needed (virus-like cascade)

## Campaign V5 Results

**Channel pump architecture** solved the deadlock problem definitively:

| Phase | Local (same machine) | Mac ↔ NAS (LAN) |
| ----- | -------------------- | --------------- |
| Ping | 20/20 (0% loss) | 20/20 (0% loss), 631ms avg |
| Burst | 300/300 (0% loss) | 90/90 (0% loss), 572ms avg |
| E2E | 20/20 (0% loss) | 20/20 (0% loss), 287ms avg |
| Group | ✅ Local hub fix | ⚠️ Timing issue (created OK) |
| Endurance | N/A | 120/120 (0% loss), 1.5ms avg |

**Key fixes:**

1. **Background channel pump** — continuous drain prevents `try_send()` drops
2. **Local hub detection** — self-addressed group operations handled locally (no network round-trip)
3. **Low-memory tuning** — volume adaptation for constrained devices (Freebox NAS: 957MB RAM)

## Testing

```bash
# Rust tests (1200+ across the workspace)
cargo test --workspace

# Lint gate (mandatory before push)
cargo clippy --workspace -- -D warnings

# TypeScript tests (771 tests, legacy stack)
pnpm test

# Supply chain (RustSec advisories, sources, bans)
cargo deny check advisories bans sources
```

## Core Concepts

### Proof of Presence (PoP)

No energy-hungry PoW, no capitalist PoS. You validate because you're there and you behave well.

### Dynamic Roles

Every node can be: **Client, Relay, Observer, Guardian, Validator.**
Roles are assigned dynamically based on network needs and contribution.

### 7 Locked Design Decisions

| # | Decision | Rule |
|---|----------|------|
| 1 | **Delivery** | Message delivered ⟺ recipient emits ACK |
| 2 | **TTL** | 24h max lifespan, then global purge |
| 3 | **L1 Role** | L1 anchors state, never arbitrates |
| 4 | **Reputation** | Progressive fade, no permanent bans |
| 5 | **Anti-spam** | Progressive load, not exclusion |
| 6 | **Invisibility** | Protocol layer invisible to end users |
| 7 | **Scope** | Universal foundation (like TCP/IP), not a product |

## Contributing

ToM uses a **micro-session contribution model** — small, focused changes completable in 30-60 minutes.

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Documentation

- [docs/spec/](docs/spec/) — **Normative protocol specs + test vectors** (implement ToM in any language)
- [crates/tom-sdk/README.md](crates/tom-sdk/README.md) — Rust SDK guide
- [sdk/swift/TomProtocolKit/README.md](sdk/swift/TomProtocolKit/README.md) — Apple SDK guide
- [docs/FORK-GOVERNANCE.md](docs/FORK-GOVERNANCE.md) — iroh fork governance & wire invariants
- [CLAUDE.md](CLAUDE.md) — Implementation guide for AI assistants
- [llms.txt](llms.txt) — Protocol quick reference
- [Architecture](_bmad-output/planning-artifacts/architecture.md) — ADRs and design decisions
- [Design Decisions](_bmad-output/planning-artifacts/design-decisions.md) — 7 locked invariants

## License

MIT
