# ToM Signaling Server

> **⚠️ TEMPORARY COMPONENT (ADR-002)**
>
> This server exists only for the PoC/Demo phase. It will be replaced by distributed DHT-based peer discovery as the network matures. All code in this package is marked for elimination.

## Purpose

Provides WebSocket-based signaling for ToM network bootstrap:

- **Node Registration**: Nodes announce themselves on connect
- **Presence Broadcasting**: Notifies peers when nodes join/leave
- **Signal Relay**: Forwards WebRTC signaling messages between peers
- **Heartbeat Distribution**: Broadcasts liveness signals for peer discovery
- **Role Assignment Broadcasting**: Distributes role changes network-wide

## Usage

```bash
# Start the signaling server
pnpm start

# Or with custom port
PORT=4000 pnpm start
```

## ADR-002 Replacement Roadmap

The signaling server follows a planned elimination path:

### Phase 1: Current State (Iterations 1-3)
- Single WebSocket server on fixed endpoint
- The "umbilical cord" for network bootstrap
- All nodes connect here to discover peers

### Phase 2: Redundancy (Iterations 4-5)
- Multiple signaling seed servers (VPS instances)
- Resilience through redundancy
- Nodes can connect to any available seed

### Phase 3: DHT Transition (Iteration 6 / Epic 7)
- Distributed hash table begins operating among nodes
- Seed servers become regular nodes
- Discovery moves to peer-to-peer

### Phase 4: Zero Infrastructure (Target State)
- No fixed servers required
- Network IS the infrastructure
- Topic hash provides stable "address"
- Discovery through ephemeral peers

## Technical Details

- **Port**: Default 3001 (configurable via PORT env)
- **Protocol**: WebSocket with JSON messages
- **State**: In-memory only (stateless design per ADR-001)

## File Structure

```
tools/signaling-server/
├── src/
│   ├── index.ts    # Exports and types (ADR-002 marked)
│   ├── server.ts   # Core signaling logic (ADR-002 marked)
│   └── cli.ts      # CLI entry point (ADR-002 marked)
├── package.json
└── README.md
```

## References

- [Architecture ADR-002](../../_bmad-output/planning-artifacts/architecture.md#ADR-002) — Signaling Bootstrap
- [Epic 7: Self-Sustaining Alpha Network](../../_bmad-output/planning-artifacts/epics.md#Epic-7) — DHT replacement
- [Story 7.1: Autonomous Peer Discovery](../../_bmad-output/planning-artifacts/epics.md#Story-7.1) — Implementation target
