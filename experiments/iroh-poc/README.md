# ToM Protocol - iroh PoC

Proof of Concept for NAT traversal & hole punching using [iroh](https://github.com/n0-computer/iroh) (Chemin C).

## Strategy

1. PoC with iroh as dependency - learn the internals
2. Strategic fork - extract needed modules, adapt to ToM
3. Full independence from n0-computer

## PoC Results

| Phase | Description | Status | Key Metrics |
|-------|-------------|--------|-------------|
| PoC-1 | Two nodes, QUIC echo | DONE | Connect: 289ms, RTT: 125ms |
| PoC-2 | Gossip peer discovery | DONE | Neighbor: 257ms, broadcast: instant |
| PoC-3 | Gossip discovery + direct QUIC chat | DONE | Discovery: 3s, msg delivery: 4.8s |
| PoC-4 | NAT traversal on real networks | DONE | Hole punch: 100% success, 4 scenarios |

## Binaries

### echo-server + node (PoC-1)

```bash
# Terminal 1: Start echo server
cargo run --bin echo-server

# Terminal 2: Connect and send message
cargo run --bin node -- <ENDPOINT_ID> "Hello from ToM!"
```

### gossip-node (PoC-2)

```bash
# Terminal 1: Alice starts the gossip network
cargo run --bin gossip-node -- --name Alice

# Terminal 2: Bob joins via Alice's EndpointId
cargo run --bin gossip-node -- --name Bob --peer <ALICE_ENDPOINT_ID>

# Terminal 3: Charlie joins via anyone
cargo run --bin gossip-node -- --name Charlie --peer <BOB_ENDPOINT_ID>
```

### chat-node (PoC-3)

Gossip discovery + direct QUIC messaging. Closest to ToM's target architecture:
gossip for peer discovery, direct QUIC streams for message delivery.

```bash
# Terminal 1: Alice starts
cargo run --bin chat-node -- --name Alice

# Terminal 2: Bob joins via Alice's EndpointId
cargo run --bin chat-node -- --name Bob --peer <ALICE_ENDPOINT_ID>

# Type messages in either terminal - sent via direct QUIC to all discovered peers
# /peers to list discovered peers
```

### nat-test v2 (PoC-4)

Instrumented NAT traversal test. Outputs structured JSON events.
Monitors relay-to-direct upgrade (hole punch success), RTT per path, connection metrics.

**v2 features**: `--continuous` mode (infinite pings, rolling summaries) and
automatic reconnection on connection loss — designed for in-motion testing
(car, train, border crossing, network switching).

```bash
# Machine A (NAS/VPS - listener):
./nat-test --listen --name NAS

# Machine B (MacBook - fixed ping count):
cargo run --release --bin nat-test -- --connect <NAS_ID> --name MacBook --pings 20

# Machine B (MacBook - continuous for in-motion testing):
cargo run --release --bin nat-test -- --connect <NAS_ID> --name MacBook --continuous

# Continuous with custom settings:
cargo run --release --bin nat-test -- --connect <NAS_ID> --name MacBook \
    --continuous --delay 1000 --summary-interval 30 --max-reconnects 0

# Cross-compile for ARM64 Linux (Freebox NAS):
cargo zigbuild --release --bin nat-test --target aarch64-unknown-linux-musl
scp target/aarch64-unknown-linux-musl/release/nat-test root@freebox:~/
```

**nat-test v2 CLI options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--listen` | - | Listener mode (wait for connections) |
| `--connect <ID>` | - | Connect to peer by EndpointId |
| `--name <NAME>` | Node | Display name |
| `--pings <N>` | 20 | Number of pings (ignored in continuous) |
| `--delay <MS>` | 2000 | Delay between pings |
| `--continuous` | false | Infinite pings until Ctrl+C |
| `--summary-interval <N>` | 50 | Rolling summary every N pings |
| `--max-reconnects <N>` | 10 | Max reconnect attempts (0=unlimited) |

**JSON events emitted:**

| Event | When |
|-------|------|
| `started` | Binary launches |
| `path_change` | Connection switches relay↔direct |
| `ping` | Each successful ping/pong |
| `hole_punch` | First direct path established |
| `disconnected` | Connection lost |
| `reconnecting` | Attempting reconnection |
| `reconnected` | Connection restored |
| `summary` | End of run (or rolling every N pings) |

**PoC-4 Real-World NAT Traversal Results:**

| Scenario | Network Topology | Hole Punch | Upgrade Time | RTT Direct | Direct % |
|----------|-----------------|-----------|-------------|-----------|----------|
| Localhost | Same machine | 1.4s | 1.4s | 2.6ms | 80% |
| LAN WiFi | MacBook ↔ Freebox NAS (same network) | 0.37s | 0.37s | 49ms | 100% |
| 4G CGNAT | MacBook (iPhone hotspot) ↔ NAS (home WiFi) | 2.9s | 2.9s | 107ms | 90% |
| Cross-border | MacBook (school WiFi, Switzerland) ↔ NAS (France) | 0.33s | 1.4s | 32ms | 95% |

**Key findings:**
- **100% hole punch success** across all 4 scenarios (residential NAT, CGNAT, cross-border)
- Relay used only for first 1-3 pings, then direct UDP path established
- CGNAT (4G operator) is the hardest to punch through (~3s), but still succeeds
- Cross-border Switzerland↔France achieves **32ms direct RTT** (school guest WiFi behind NAT)
- Relay: `euc1-1.relay.n0.iroh-canary.iroh.link` (auto-assigned, EU region)
- All connections E2E encrypted via QUIC TLS (automatic)

## What iroh Gives Us

| Feature | How It Works |
|---------|--------------|
| Identity | EndpointId = Ed25519 public key (same model as ToM) |
| NAT traversal | UDP hole punching, ~90% direct in production |
| Relay fallback | Stateless relays, E2E encrypted, auto-assigned |
| Discovery | DNS + Pkarr signed packets (+ mDNS, DHT planned) |
| Gossip | HyParView/PlumTree epidemic broadcast trees |
| Transport | QUIC (multiplexed, encrypted, no head-of-line blocking) |
| Protocols | Composable via ALPN identifiers |

## What ToM Adds On Top

| Feature | ToM-specific |
|---------|--------------|
| Dynamic roles | Relay = role assigned by network, not by choice |
| Virus backup | Messages self-replicate to survive 24h TTL |
| Contribution scoring | Usage affects role assignment |
| Wire format | JSON envelopes with signatures |
| Groups | Hub-and-spoke multi-party messaging |

## Architecture

```
gossip-node (PoC-2)
    |
    +- Endpoint::bind()          --> QUIC socket + key generation
    +- Gossip::builder().spawn() --> HyParView/PlumTree protocol
    +- Router (gossip ALPN)      --> Accept incoming gossip connections
    +- subscribe(topic, peers)   --> Join gossip swarm
    +- broadcast(msg)            --> Epidemic dissemination
    |
    +-- iroh handles automatically:
        +- DNS/Pkarr discovery
        +- UDP hole punching
        +- Relay fallback (euc1-1.relay.n0.iroh-canary.iroh.link)
        +- QUIC E2E encryption

chat-node (PoC-3) - ToM target architecture
    |
    +- Gossip layer (discovery only)
    |   +- subscribe(topic) → ANNOUNCE messages
    |   +- NeighborUp → re-announce for reliable discovery
    |   +- PeerMap: EndpointId → name
    |
    +- Direct QUIC layer (payload)
    |   +- connect(peer_addr, CHAT_ALPN)
    |   +- open_bi() → write message → finish
    |   +- Receiver: accept_bi() → read_to_end → display
    |
    +- Two ALPNs on same Router:
        +- iroh_gossip::ALPN → gossip protocol
        +- tom-protocol/poc/chat/0 → direct messages
```

## Dependencies

- **iroh 0.96** - Core connectivity (QUIC, hole punch, relay)
- **iroh-gossip 0.96** - Epidemic broadcast (HyParView/PlumTree)
- **tokio** - Async runtime
- **clap** - CLI argument parsing

## Test Environment

| Device | Role | Details |
|--------|------|---------|
| MacBook Pro | Connector | macOS, native Rust binary |
| Freebox Delta NAS | Listener | Debian VM, ARM64 (Cortex-A72), cross-compiled static binary |
| iPhone 12 Pro | Network provider | USB tethering for 4G CGNAT tests |

## V2 Test Campaign (planned)

| Scenario | What It Tests |
|----------|--------------|
| School WiFi (CH) | Restrictive NAT, guest network |
| 4G/5G Swiss operator | CGNAT, operator-level NAT |
| Moving car | Relay handoff, cell tower changes |
| Border crossing (CH↔FR) | Network switch mid-session |
| Weak coverage / tunnel | Disconnection + reconnection |
| Network switch (WiFi→4G) | Mid-session network change |

## Next Steps

1. ~~**Fork architecture**~~: Done — see [FORK-ARCHITECTURE.md](FORK-ARCHITECTURE.md)
2. ~~**CI**~~: Done — Rust build + clippy + localhost test in GitHub Actions
3. **V2 test campaign**: In-motion NAT tests — see [V2-TEST-CAMPAIGN.md](V2-TEST-CAMPAIGN.md)
4. **Fork execution**: Extract `tom-connect` + `tom-relay` from iroh (after 0.97/1.0-rc)
5. **Adapt**: Custom wire format, dynamic roles, virus backup
6. **Integrate**: Replace WebSocket signaling in TypeScript core
