# ToM Protocol

**The Open Messaging** — a decentralized P2P transport protocol where every device is the network.

## Status: Phase 2 — Native Transport

| Phase | Description | Status |
|-------|-------------|--------|
| **Phase 1** | TypeScript protocol stack (WebRTC, signaling) | ✅ 8/8 epics |
| **Phase 2** | Rust native transport (QUIC, hole punching, E2E crypto) | ✅ Validated |
| **Phase 3** | Protocol convergence (TS + Rust unified) | In progress |

**1000+ tests** (771 TypeScript + 236 Rust) | **E2E encrypted** | **NAT traversal validated** | **Cross-border Suisse↔France**

## TL;DR

ToM is a transport layer protocol (not a blockchain) that transforms every connected device into both client and relay. No data centers, no speculative tokens, no infinite history.

**The idea:** leverage the dormant power of billions of devices to create a global communication BUS that's resilient and virtually free.

## What's proven (with data)

| Test | Result | Details |
|------|--------|---------|
| NAT hole punching | **100% success** | LAN, 4G CGNAT, cross-border CH↔FR |
| Stress test (4G highway) | **99.85%** | 2748/2752 pings, 54 min continuous |
| E2E encrypted chat | **Working** | Signed + encrypted envelopes, Mac↔NAS cross-border |
| Direct QUIC latency | **27-49ms** | After hole punch, no relay needed |

## Project Structure

```
tom-protocol/
├── crates/                          # Rust native stack
│   ├── tom-transport/               # QUIC transport (iroh), connection pool
│   ├── tom-protocol/                # Protocol logic (crypto, routing, groups, discovery, backup)
│   ├── tom-tui/                     # TUI chat client + bot mode
│   └── tom-stress/                  # Stress test binary
│
├── packages/                        # TypeScript stack (Phase 1)
│   ├── core/                        # Protocol primitives (tom-protocol)
│   └── sdk/                         # Developer SDK (tom-sdk)
│
├── apps/
│   └── demo/                        # Browser demo with multiplayer Snake
│
├── experiments/
│   └── iroh-poc/                    # NAT traversal PoC (4 scenarios validated)
│
├── tools/
│   ├── signaling-server/            # Bootstrap server (being replaced by QUIC)
│   ├── mcp-server/                  # MCP server for LLM interaction
│   └── vscode-extension/            # VS Code extension
│
├── docs/                            # Documentation (GitBook)
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
ProtocolRuntime (single tokio::select! loop)
├── Router          — deliver / forward / reject / ack
├── Topology        — peer state, heartbeat tracking
├── EnvelopeBuilder — encrypt-then-sign, MessagePack wire format
├── GroupManager    — member-side multi-party
├── GroupHub        — hub-side fan-out, deterministic failover
├── BackupStore     — TTL-based virus backup for offline peers
├── RelaySelector   — optimal relay selection
└── HeartbeatTracker — stale/offline detection
```

## Quick Start

### TypeScript Demo (browser)

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

## Testing

```bash
# TypeScript tests (771 tests)
pnpm test

# Rust tests (236 tests)
cargo test --workspace

# E2E browser tests (Playwright)
pnpm test:e2e
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

- [CLAUDE.md](CLAUDE.md) — Implementation guide for AI assistants
- [llms.txt](llms.txt) — Protocol quick reference
- [docs/](docs/) — GitBook documentation
- [Architecture](_bmad-output/planning-artifacts/architecture.md) — ADRs and design decisions
- [Design Decisions](_bmad-output/planning-artifacts/design-decisions.md) — 7 locked invariants

## License

MIT
