# Story 3.1: Peer Discovery Protocol

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a network node,
I want to discover other participants beyond my direct connections,
so that I can find potential relays and recipients across the network.

## Acceptance Criteria

1. **Given** a node is connected to the network **When** a new node joins or an existing node updates its presence **Then** the discovery mechanism propagates presence information to reachable peers **And** each node maintains an up-to-date network topology map

2. **Given** a node goes offline **When** its heartbeat stops or connection drops **Then** the discovery mechanism propagates the departure within 3 seconds **And** all nodes remove it from their topology map

3. **Given** the network has more participants than a single node's direct connections **When** the node queries the discovery layer **Then** it receives information about indirect peers (reachable through relays) **And** the information includes the peer's public key, username, and reachable relay path

## Tasks / Subtasks

- [ ] Task 1: Create PeerDiscovery module in core (AC: #1, #2, #3)
  - [ ] Create `packages/core/src/discovery/peer-discovery.ts`
  - [ ] Define `PeerInfo` interface: `{ nodeId: string, username: string, publicKey: string, reachableVia: string[], lastSeen: number, role: NodeRole }`
  - [ ] Define `NetworkTopology` class managing the topology map (Map<nodeId, PeerInfo>)
  - [ ] Implement `addPeer(info: PeerInfo)` — add/update peer in topology
  - [ ] Implement `removePeer(nodeId: string)` — remove peer from topology
  - [ ] Implement `getPeer(nodeId: string): PeerInfo | undefined`
  - [ ] Implement `getReachablePeers(): PeerInfo[]` — all known peers
  - [ ] Implement `getIndirectPeers(): PeerInfo[]` — peers reachable only via relay
  - [ ] Create barrel export `packages/core/src/discovery/index.ts`

- [ ] Task 2: Implement heartbeat mechanism (AC: #2)
  - [ ] Add `heartbeat` message type to the protocol
  - [ ] Implement `HeartbeatManager` class in `packages/core/src/discovery/heartbeat.ts`
  - [ ] Send periodic heartbeats (configurable interval, default 5s)
  - [ ] Track last-seen timestamp per peer
  - [ ] Detect peer departure when no heartbeat for 3s (NFR2)
  - [ ] Emit `peer:stale` and `peer:departed` events

- [ ] Task 3: Implement presence propagation via signaling server (AC: #1, #2)
  - [ ] Add `presence` message type to signaling server
  - [ ] When a node joins: broadcast `presence:join` with PeerInfo to all connected nodes
  - [ ] When a node leaves: broadcast `presence:leave` with nodeId
  - [ ] Update signaling server to include `publicKey` in participant data
  - [ ] Update `tools/signaling-server/src/server.ts` to handle new message types

- [ ] Task 4: Integrate discovery into TomClient SDK (AC: #1, #3)
  - [ ] Add `NetworkTopology` instance to TomClient
  - [ ] Update `onParticipants` handler to build topology from signaling data
  - [ ] Add `onPeerDiscovered(handler)` and `onPeerDeparted(handler)` callbacks
  - [ ] Expose `getTopology(): PeerInfo[]` method on TomClient
  - [ ] Wire heartbeat manager into connect/disconnect lifecycle

- [ ] Task 5: Update demo UI to show discovery info (AC: #1, #3)
  - [ ] Show peer status (online/stale) in participant list
  - [ ] Display indirect peers with relay path info
  - [ ] Show topology stats in status bar (total peers, direct, indirect)

- [ ] Task 6: Write tests (AC: #1, #2, #3)
  - [ ] Create `packages/core/src/discovery/network-topology.test.ts`
  - [ ] Create `packages/core/src/discovery/heartbeat.test.ts`
  - [ ] Test: add/remove/query peers in topology
  - [ ] Test: heartbeat timeout triggers peer removal within 3s
  - [ ] Test: presence propagation via signaling
  - [ ] Test: indirect peer resolution with relay paths
  - [ ] Update signaling server tests for new message types

- [ ] Task 7: Build and validate
  - [ ] Run `pnpm build` — zero errors
  - [ ] Run `pnpm test` — all tests pass
  - [ ] Run `pnpm lint` — zero warnings
  - [ ] Export new types from `packages/core/src/index.ts`

## Dev Notes

### Architecture Compliance

- **ADR-002**: Signaling server is still the bootstrap mechanism for iterations 1-3. Peer discovery builds ON TOP of the signaling server, not replacing it. The signaling server gains presence awareness. [Source: architecture.md#ADR-002]
- **ADR-006**: Unified node model — every node runs `NetworkTopology`. No special discovery code per role. The topology creates the role, not the code. [Source: architecture.md#ADR-006]
- **ADR-007**: Role model — roles are network-imposed. This story introduces the `NodeRole` type but does NOT implement role assignment (that's Story 3.2). [Source: architecture.md#ADR-007]
- **NFR2**: <3s peer discovery — heartbeat timeout must be within 3 seconds. [Source: architecture.md#NFR2]
- **ADR-003**: Wire format JSON — all new message types (heartbeat, presence) use the existing MessageEnvelope format. [Source: architecture.md#ADR-003]

### Critical Boundaries

- **DO NOT** implement role assignment — that is Story 3.2
- **DO NOT** implement automatic relay selection — that is Story 3.3
- **DO NOT** add WebRTC DataChannels — current PoC still uses signaling server relay (from Epic 2)
- **DO** add the `NodeRole` type definition (needed by PeerInfo) but default all nodes to `'client'` role
- **DO** keep the signaling server as the sole transport (ADR-002 iterations 1-3)

### Previous Story Learnings (from Epic 2)

- `crypto.randomUUID()` not available in non-secure contexts (mobile HTTP) — use the fallback pattern from router.ts
- biome `noNonNullAssertion` rule is strict — use `as Type` casts or null guards, never `!`
- biome format is strict — always run `pnpm biome check --write .` before committing
- SDK `sendMessage` needed to be async to await `connectToPeer` — keep async patterns in mind
- Signaling server relay pattern: wrap envelope in `{ type: 'signal', from, to, payload: { type, envelope } }`
- Import ordering: biome enforces specific import order, run lint:fix

### Git Intelligence

Recent commits show the pattern: feat/fix prefix, one commit per story or fix. Files touched in last 3 commits:
- `packages/core/src/routing/router.ts` — Router with ACK
- `packages/sdk/src/tom-client.ts` — TomClient SDK wrapper
- `tools/signaling-server/src/server.ts` — signaling with relay
- `apps/demo/src/main.ts` — demo UI

### Project Structure Notes

New files to create:
```
packages/core/src/discovery/
├── index.ts                    # Barrel export
├── network-topology.ts         # NetworkTopology class + PeerInfo type
├── heartbeat.ts                # HeartbeatManager class
├── network-topology.test.ts    # Topology unit tests
└── heartbeat.test.ts           # Heartbeat unit tests
```

Existing files to modify:
- `packages/core/src/index.ts` — add discovery exports
- `packages/core/src/types/events.ts` — add peer:discovered, peer:departed, peer:stale events
- `tools/signaling-server/src/server.ts` — add presence message handling
- `tools/signaling-server/src/index.ts` — add presence types
- `packages/sdk/src/tom-client.ts` — integrate NetworkTopology + HeartbeatManager
- `apps/demo/src/main.ts` — show discovery info in UI

### References

- [Source: architecture.md#ADR-002] — Signaling Bootstrap
- [Source: architecture.md#ADR-006] — Unified Node Model
- [Source: architecture.md#ADR-007] — Role Model
- [Source: epics.md#Story 3.1] — Acceptance criteria
- [Source: architecture.md#NFR2] — <3s peer discovery

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Fixed signaling server tests to use waitForMessageOfType to handle new presence messages

### Completion Notes List

- All 7 tasks completed
- 16 new tests (9 topology + 7 heartbeat), 62 total passing
- Build, test, lint all green

### File List

- packages/core/src/discovery/network-topology.ts (new)
- packages/core/src/discovery/heartbeat.ts (new)
- packages/core/src/discovery/index.ts (new)
- packages/core/src/discovery/network-topology.test.ts (new)
- packages/core/src/discovery/heartbeat.test.ts (new)
- packages/core/src/index.ts (modified)
- packages/core/src/types/events.ts (modified)
- packages/sdk/src/tom-client.ts (modified)
- tools/signaling-server/src/index.ts (modified)
- tools/signaling-server/src/server.ts (modified)
- tools/signaling-server/src/server.test.ts (modified)
- apps/demo/index.html (modified)
- apps/demo/src/main.ts (modified)
