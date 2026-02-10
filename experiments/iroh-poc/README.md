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
| PoC-4 | NAT traversal on real networks | Planned | - |

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

### nat-test (PoC-4)

Instrumented NAT traversal test. Outputs structured JSON events.
Monitors relay-to-direct upgrade (hole punch success), RTT per path, connection metrics.

```bash
# Machine A (NAS/VPS - listener):
./nat-test --listen --name NAS

# Machine B (MacBook - connector):
cargo run --release --bin nat-test -- --connect <NAS_ID> --name MacBook --pings 20

# Cross-compile for ARM64 Linux (Freebox NAS):
cargo zigbuild --release --bin nat-test --target aarch64-unknown-linux-musl
scp target/aarch64-unknown-linux-musl/release/nat-test root@freebox:~/
```

**Localhost baseline results:**
- Hole punch: **1.4s** relay→direct
- RTT relay: **121ms** (via euc1-1.relay.n0.iroh-canary.iroh.link)
- RTT direct: **2.6ms** (IPv6 local)
- 80% direct (1st ping relay, 4 remaining direct)

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

## Next Steps

1. **PoC-4**: Test hole punching across real NAT (4G, different WiFi networks)
2. **Fork**: Extract iroh connectivity + gossip modules
3. **Adapt**: Custom wire format, dynamic roles, virus backup
4. **Integrate**: Replace WebSocket signaling in TypeScript core
