# ToM Protocol - iroh PoC

Proof of Concept for NAT traversal & hole punching using [iroh](https://github.com/n0-computer/iroh).

## Goal

Validate iroh as a transport foundation for ToM Protocol before strategic fork (Chemin C).

## PoC Phases

| Phase | Description | Status |
|-------|-------------|--------|
| PoC-1 | Two nodes, direct connection + echo | In Progress |
| PoC-2 | iroh-gossip peer discovery | Planned |
| PoC-3 | NAT traversal on real networks (4G, residential WiFi) | Planned |

## Quick Start

### Terminal 1: Start echo server

```bash
cd experiments/iroh-poc
cargo run --bin echo-server
```

Copy the printed `Endpoint ID`.

### Terminal 2: Connect and send message

```bash
cd experiments/iroh-poc
cargo run --bin node -- <ENDPOINT_ID> "Hello from ToM!"
```

## What We're Measuring

- Connection establishment time (direct vs relay)
- Round-trip latency
- Hole punching success rate
- Relay fallback behavior
- Gossip discovery time (PoC-2)

## Architecture

```
echo-server                        node
    |                                |
    +- Endpoint::bind()              +- Endpoint::bind()
    +- Router (TOM_ALPN)             +- endpoint.connect(target)
    +- accept_bi()                   +- open_bi()
    +- read -> echo back             +- write -> read echo
    |                                |
    +-- iroh handles:                +-- iroh handles:
        +- Discovery (DNS/Pkarr)         +- Discovery (DNS/Pkarr)
        +- Hole punching (UDP)           +- Hole punching (UDP)
        +- Relay fallback                +- Relay fallback
        +- QUIC encryption               +- QUIC encryption
```

## Dependencies

- **iroh 0.96** - Core connectivity (QUIC, hole punch, relay)
- **iroh-gossip 0.96** - Epidemic broadcast for peer discovery (PoC-2)
- **tokio** - Async runtime
- **clap** - CLI argument parsing

## Next Steps (Post-PoC)

1. Fork iroh modules we need
2. Adapt to ToM Protocol wire format
3. Integrate with existing TypeScript core via FFI
4. Replace WebSocket signaling server
