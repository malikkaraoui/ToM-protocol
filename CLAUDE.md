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
├── packages/
│   ├── core/                 # Protocol primitives (tom-protocol)
│   │   └── src/
│   │       ├── identity/     # Ed25519 keypair, identity persistence
│   │       ├── transport/    # WebRTC DataChannel management
│   │       ├── routing/      # Router, RelaySelector, RelayStats
│   │       ├── discovery/    # NetworkTopology, PeerGossip, EphemeralSubnets
│   │       ├── groups/       # GroupManager, GroupHub (multi-party)
│   │       ├── roles/        # RoleManager, dynamic assignment
│   │       ├── crypto/       # E2E encryption (TweetNaCl.js)
│   │       ├── backup/       # Message backup for offline recipients
│   │       ├── types/        # MessageEnvelope, shared types
│   │       └── errors/       # TomError, TomErrorCode
│   │
│   └── sdk/                  # Developer SDK (tom-sdk)
│       └── src/
│           └── tom-client.ts # TomClient - main integration point
│
├── apps/
│   └── demo/                 # Browser demo (vanilla HTML/JS + Vite)
│       ├── index.html        # Chat UI + Snake game
│       └── src/main.ts       # Demo application logic
│
├── tools/
│   └── signaling-server/     # Bootstrap server (temporary, marked for elimination)
│       └── src/
│           ├── server.ts     # WebSocket signaling logic
│           └── cli.ts        # CLI entry point
│
└── _bmad-output/             # Planning artifacts (PRD, architecture, epics)
```

## Key Architecture Decisions (ADRs)

### ADR-001: WebRTC DataChannel via Relay
All messages transit through at least one relay. Relays are not optional - they ARE the architecture.

### ADR-002: Bootstrap Elimination Roadmap
- **Current**: WebSocket signaling server (temporary)
- **Target**: Zero fixed infrastructure via DHT
- Code marked with `BOOTSTRAP LAYER` comments is transitional

### ADR-003: Wire Format
JSON envelopes: `{id, from, to, via, type, payload, timestamp, signature}`

### ADR-004: Encryption Stack
TweetNaCl.js: X25519 key exchange, XSalsa20-Poly1305 authenticated encryption

### ADR-005: Node Identity
Ed25519 keypair = node identity. Public key is the network address.

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
| 5 | **Anti-spam** | "Sprinkler gets sprinkled" - progressive load, not exclusion |
| 6 | **Invisibility** | Protocol layer invisible to end users |
| 7 | **Scope** | Universal foundation (like TCP/IP), not a product |

**Before writing code, verify:**
- No user-visible protocol state
- L1 doesn't make operational decisions
- No permanent bans or binary states
- No message persistence beyond TTL

## Core Components

### TomClient (SDK)

The main entry point for developers:

```typescript
import { TomClient } from 'tom-sdk';

const client = new TomClient({
  signalingUrl: 'wss://signaling.example.com',
  username: 'alice',
  encryption: true,  // E2E encryption (default: true)
});

await client.connect();

// Send message (automatically selects relay, encrypts)
await client.sendMessage(recipientNodeId, 'Hello!');

// Receive messages (automatically decrypts)
client.onMessage((envelope) => {
  const payload = envelope.payload as { text?: string };
  console.log(`Message from ${envelope.from}: ${payload.text}`);
});

// Track message status
client.onMessageStatusChanged((id, previousStatus, newStatus) => {
  // newStatus: 'pending' | 'sent' | 'relayed' | 'delivered' | 'read'
});
```

### Router (Core)

Handles message routing decisions:

```typescript
// Router decides: for me → deliver, not for me → forward
const router = new Router(nodeId, transport, {
  onMessageReceived: (envelope) => { /* handle */ },
  onAckReceived: (messageId) => { /* update status */ },
});

// Create and send envelope
const envelope = router.createEnvelope(to, 'chat', payload, relayChain);
router.routeMessage(envelope);
```

### RelaySelector (Core)

Selects optimal relay for message routing:

```typescript
const selector = new RelaySelector({ selfNodeId: nodeId });

// Select best relay based on topology
const { relayId, reason } = selector.selectBestRelay(targetNodeId, topology);

// Select alternate if primary fails
const alternate = selector.selectAlternateRelay(targetNodeId, topology, failedRelays);
```

### PeerGossip (Core)

Autonomous peer discovery:

```typescript
const gossip = new PeerGossip(nodeId, username, {
  onPeersDiscovered: (peers, via) => { /* new peers found */ },
  onPeerListRequested: (from) => { /* respond with known peers */ },
});

// Register bootstrap peers
gossip.addBootstrapPeer({ nodeId, username, encryptionKey });

// Periodic gossip rounds discover new peers
```

### EphemeralSubnetManager (Core)

Self-organizing subnets based on communication patterns:

```typescript
const subnets = new EphemeralSubnetManager(nodeId, {
  onSubnetFormed: (subnet) => { /* optimize routing */ },
  onSubnetDissolved: (subnetId, reason) => { /* cleanup */ },
});

// Track communications
subnets.recordCommunication(fromNode, toNode);

// Check subnet membership
if (subnets.areInSameSubnet(nodeA, nodeB)) {
  // Prefer intra-subnet routing
}
```

### GroupManager (Core)

Multi-party messaging with hub-and-spoke topology:

```typescript
const groups = new GroupManager(nodeId, {
  onGroupCreated: (group) => { /* notify UI */ },
  onInviteReceived: (invite) => { /* accept/decline */ },
  onGroupMessage: (groupId, msg) => { /* display */ },
});

// Create group
const group = groups.createGroup('Team Chat', ['member1', 'member2']);

// Send to group (routes through hub relay)
groups.sendMessage(groupId, 'Hello team!');
```

## Message Flow

```
Sender → RelaySelector → Router → Transport → [Relay Node] → Transport → Router → Recipient
         ↓                                        ↓
    Select relay                           Forward (no store)
         ↓                                        ↓
    Encrypt payload                        Verify signature
         ↓                                        ↓
    Sign envelope                          Route to next hop
```

## Implementation Patterns

### File Naming
- `kebab-case.ts` for files
- `PascalCase` for classes
- Co-located tests: `foo.ts` + `foo.test.ts`

### Error Handling
```typescript
import { TomError, TomErrorCode } from 'tom-protocol';

throw new TomError(TomErrorCode.PEER_UNREACHABLE, 'Node not found', { nodeId });
```

### Event Pattern
```typescript
// Components use typed callbacks, not EventEmitter
interface RouterEvents {
  onMessageReceived: (envelope: MessageEnvelope) => void;
  onAckReceived: (messageId: string) => void;
}
```

### Async/Await
Always use async/await, never raw Promises or callbacks.

## Testing

```bash
pnpm test           # Run all tests
pnpm test --watch   # Watch mode
pnpm lint           # Biome check
pnpm build          # Build all packages
```

Tests are co-located with source files. Use vitest:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';

describe('MyComponent', () => {
  it('should do something', () => {
    expect(result).toBe(expected);
  });
});
```

## Common Tasks

### Add a new message type

1. Update `MessageEnvelope.type` in `packages/core/src/types/envelope.ts`
2. Handle in `Router.handleIncomingMessage()`
3. Add SDK method in `TomClient`
4. Write tests

### Add a new protocol feature

1. Create module in `packages/core/src/`
2. Export from `packages/core/src/index.ts`
3. Integrate in `TomClient` (packages/sdk)
4. Add to demo if user-facing

### Debug message routing

```typescript
// Enable in demo by checking message path
const pathInfo = extractPathInfo(envelope);
console.log(pathInfo.hopCount, pathInfo.relayLatencies);
```

## Current Status

| Epic | Description | Status |
|------|-------------|--------|
| 1 | Project Foundation & Identity | ✅ Complete |
| 2 | First Message Through Relay | ✅ Complete |
| 3 | Dynamic Routing & Discovery | ✅ Complete |
| 4 | Bidirectional Conversation | ✅ Complete |
| 5 | Multi-Relay Transport | ✅ Complete |
| 6 | E2E Encryption | ✅ Complete |
| 7 | Self-Sustaining Network | ✅ Complete |
| 8 | LLM & Community Ecosystem | ✅ Complete |

## Network Stats

- **Tests**: 771 passing
- **Packages**: 4 (core, sdk, demo, signaling-server)
- **Target Scale**: 10-15 simultaneous nodes (alpha)

## Important Notes for LLMs

1. **Bootstrap is temporary**: All signaling server code is marked for future elimination
2. **Relays don't store**: Pass-through only, no persistence
3. **E2E is mandatory**: All messages encrypted from Epic 6 onward
4. **Roles are network-assigned**: Nodes don't choose to be relays
5. **No blockchain**: This is a transport protocol, not a ledger
6. **Contribution matters**: Usage/contribution score affects role assignment

## Quick Commands

```bash
# Development
pnpm install          # Install dependencies
pnpm build            # Build all packages
pnpm test             # Run tests
pnpm lint             # Check code quality

# Run demo locally
cd tools/signaling-server && pnpm build && node dist/cli.js  # Start signaling
cd apps/demo && pnpm dev                                      # Start demo UI

# Access demo at http://localhost:5173
# For multi-user testing, use local IP (e.g., http://192.168.x.x:5173)
```
