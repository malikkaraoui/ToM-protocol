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
| PoC-3 | NAT traversal on real networks | Planned | - |

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
```

## Dependencies

- **iroh 0.96** - Core connectivity (QUIC, hole punch, relay)
- **iroh-gossip 0.96** - Epidemic broadcast (HyParView/PlumTree)
- **tokio** - Async runtime
- **clap** - CLI argument parsing

## Next Steps

1. **PoC-3**: Test hole punching across real NAT (4G, different WiFi networks)
2. **Fork**: Extract iroh connectivity + gossip modules
3. **Adapt**: Custom wire format, dynamic roles, virus backup
4. **Integrate**: Replace WebSocket signaling in TypeScript core
