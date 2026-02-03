# Story 3.2: Dynamic Role Assignment

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a node joining the network,
I want to be automatically assigned a role based on my capabilities and network needs,
so that the network self-organizes without manual configuration.

## Acceptance Criteria

1. **Given** a new node joins the network **When** role assignment evaluates the node **Then** a role is assigned (client, relay, observer) within 3 seconds of joining (NFR2) **And** the role is communicated to the node and broadcast to the network

2. **Given** the network lacks relay capacity **When** a capable node is evaluated for role assignment **Then** the node is assigned the relay role in addition to its client role **And** the node begins accepting forwarding requests

3. **Given** network conditions change (nodes join/leave) **When** the role assignment system re-evaluates **Then** roles can be reassigned dynamically without node restart **And** role transitions are seamless — no messages lost during transition

## Tasks / Subtasks

- [x] Task 1: Create RoleManager in core (AC: #1, #2, #3)
  - [x] Create `packages/core/src/roles/role-manager.ts`
  - [x] Define `NodeRole` type: `'client' | 'relay' | 'observer'` (already exists in network-topology.ts, consolidate)
  - [x] Define `RoleAssignment` interface: `{ nodeId: string, roles: NodeRole[], assignedAt: number }`
  - [x] Define `RoleManagerEvents` interface: `{ 'role-changed': { nodeId: string, oldRoles: NodeRole[], newRoles: NodeRole[] } }`
  - [x] Implement `RoleManager` class with `evaluateNode(nodeId: string, topology: NetworkTopology): NodeRole[]`
  - [x] Implement role assignment logic: if network has <2 relay-capable nodes and node is eligible → assign relay + client
  - [x] Implement `getCurrentRoles(nodeId: string): NodeRole[]`
  - [x] Implement `reassignRoles(topology: NetworkTopology): Map<string, NodeRole[]>` for bulk re-evaluation
  - [x] Create barrel export `packages/core/src/roles/index.ts`

- [x] Task 2: Implement role assignment algorithm (AC: #1, #2)
  - [x] Define relay eligibility criteria: node must be online (not stale/offline), connected for >5s
  - [x] Implement relay capacity check: count nodes with relay role in topology
  - [x] If relay count < ceil(totalNodes / 3) → promote eligible non-relay nodes to relay
  - [x] If relay count > ceil(totalNodes / 2) → demote least-recently-assigned relays to client-only
  - [x] Default role for new nodes: `['client']`
  - [x] Re-evaluate on: peer join, peer departure, periodic interval (30s)

- [x] Task 3: Add role-related signaling messages (AC: #1, #3)
  - [x] Add `role-assign` message type to signaling server
  - [x] Server broadcasts `role-assign` with `{ nodeId, roles }` to all connected nodes
  - [x] Server tracks current role per connected node
  - [x] Update `tools/signaling-server/src/index.ts` SignalingMessage types
  - [x] Update `tools/signaling-server/src/server.ts` to handle role messages

- [x] Task 4: Integrate RoleManager into TomClient SDK (AC: #1, #2, #3)
  - [x] Add `RoleManager` instance to TomClient
  - [x] On connect: request initial role assignment from self-evaluation
  - [x] Broadcast own role to network via signaling server
  - [x] Listen for `role-assign` messages and update local RoleManager
  - [x] Add `onRoleChanged(handler)` callback to TomClient
  - [x] Expose `getCurrentRoles(): NodeRole[]` on TomClient
  - [x] Wire role re-evaluation into peer join/departure events

- [x] Task 5: Update NetworkTopology to track roles (AC: #1, #3)
  - [x] PeerInfo already has `role: NodeRole` field — extend to `roles: NodeRole[]` (a node can have multiple roles)
  - [x] Update topology to track role assignments from signaling messages
  - [x] Add `getRelayNodes(): PeerInfo[]` — filter peers with relay role
  - [x] Add `getNodesByRole(role: NodeRole): PeerInfo[]` — generic filter

- [x] Task 6: Update demo UI to show role (AC: #1)
  - [x] Show current node's role(s) next to node ID
  - [x] Show peer roles in participant list (small badge: "R" for relay, "C" for client)
  - [x] Update topology stats to include relay count

- [x] Task 7: Write tests (AC: #1, #2, #3)
  - [x] Create `packages/core/src/roles/role-manager.test.ts`
  - [x] Test: new node gets default client role
  - [x] Test: relay role assigned when network needs relays
  - [x] Test: relay role removed when too many relays
  - [x] Test: role re-evaluation on peer join/departure
  - [x] Test: role change events emitted correctly
  - [x] Update signaling server tests for role-assign messages

- [x] Task 8: Build and validate
  - [x] Run `pnpm build` — zero errors
  - [x] Run `pnpm test` — all tests pass
  - [x] Run `pnpm lint` — zero warnings
  - [x] Export new types from `packages/core/src/index.ts`

## Dev Notes

### Architecture Compliance

- **ADR-006**: Unified node model — every node runs RoleManager. The topology creates the role, not the code. No if/else "am I a relay?" [Source: architecture.md#ADR-006]
- **ADR-007**: Role model — roles are network-imposed, not chosen by the user. Relay duty is assigned based on availability, score, and network needs. A node does NOT volunteer to relay. [Source: architecture.md#ADR-007]
- **NFR2**: Role assignment must complete within 3 seconds of joining. [Source: architecture.md#NFR2]
- **ADR-002**: Signaling server still the bootstrap for iterations 1-3. Role messages flow through signaling. [Source: architecture.md#ADR-002]

### Critical Boundaries

- **DO NOT** implement automatic relay selection for message routing — that is Story 3.3
- **DO NOT** implement contribution/usage scoring — that is Epic 5 (Story 5.4)
- **DO NOT** implement guardian or validator roles — those are future iterations
- **DO** keep the signaling server as the sole transport (ADR-002 iterations 1-3)
- **DO** implement the role assignment as a self-evaluation that each node runs locally based on network topology
- **DO** broadcast role assignments via signaling server so all nodes have consistent view
- **Important**: PeerInfo currently has `role: NodeRole` (singular). This story changes it to `roles: NodeRole[]` (plural) since a node can be both client AND relay. This is a breaking change to the PeerInfo interface — update all consumers.

### Previous Story Learnings (from Story 3.1)

- NetworkTopology and HeartbeatManager are in `packages/core/src/discovery/`
- `NodeRole` type already defined in `network-topology.ts` as `'client' | 'relay' | 'bootstrap'` — consolidate with roles module, add `'observer'`
- HeartbeatManager interval=5s, timeout=10s works well — don't change
- Don't remove peers from topology on heartbeat timeout — only on presence:leave (learned from bug fixes)
- Signaling server must be rebuilt AND restarted after code changes
- Vite HMR does NOT rebuild workspace deps — must `pnpm build` + restart server + hard refresh
- biome `noNonNullAssertion` — use `as Type` casts or null guards
- `crypto.randomUUID()` fallback needed for mobile Safari HTTP

### Git Intelligence

Recent commits (most recent first):
- `4bf6986` fix: don't remove peers from topology on heartbeat timeout
- `8eaa760` fix: keep peers alive via heartbeat and periodic UI refresh
- `12d89be` fix: heartbeat timeout must be greater than send interval
- `4e6a61f` fix: sync topology with participants list on connect
- `284abea` feat: implement peer discovery protocol (Story 3.1)

Key patterns from commits:
- feat/fix prefix convention
- Files commonly modified together: tom-client.ts, server.ts, main.ts
- Discovery module pattern: class + events + barrel export

### Project Structure Notes

New files to create:
```
packages/core/src/roles/
├── index.ts                    # Barrel export
├── role-manager.ts             # RoleManager class + RoleAssignment type
└── role-manager.test.ts        # Role assignment unit tests
```

Existing files to modify:
- `packages/core/src/discovery/network-topology.ts` — change `role: NodeRole` to `roles: NodeRole[]`, add `getRelayNodes()`, `getNodesByRole()`
- `packages/core/src/discovery/network-topology.test.ts` — update for roles array
- `packages/core/src/index.ts` — add roles exports
- `packages/core/src/types/events.ts` — add `role:changed` event type
- `tools/signaling-server/src/index.ts` — add `role-assign` message type
- `tools/signaling-server/src/server.ts` — handle role-assign messages
- `packages/sdk/src/tom-client.ts` — integrate RoleManager, add onRoleChanged, getCurrentRoles
- `apps/demo/src/main.ts` — show role badges in UI
- `apps/demo/index.html` — CSS for role badges

### References

- [Source: architecture.md#ADR-006] — Unified Node Model
- [Source: architecture.md#ADR-007] — Role Model
- [Source: architecture.md#NFR2] — <3s role assignment
- [Source: epics.md#Story 3.2] — Acceptance criteria
- [Source: architecture.md#Implementation Patterns] — Event-driven, typed EventEmitter

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Role eligibility requires 5s delay before being considered for relay
- Tests needed longer topology staleThreshold (60s) to avoid peers going offline during 30s periodic re-evaluation test

### Completion Notes List

- All 8 tasks completed
- 12 new role-manager tests, 2 new topology tests, 1 new signaling test = 15 new tests
- Total: 77 tests passing
- Build, test, lint all green
- Breaking change: PeerInfo.role → PeerInfo.roles (array)
- Added 'observer' to NodeRole type

### File List

- packages/core/src/roles/role-manager.ts (new)
- packages/core/src/roles/index.ts (new)
- packages/core/src/roles/role-manager.test.ts (new)
- packages/core/src/discovery/network-topology.ts (modified)
- packages/core/src/discovery/network-topology.test.ts (modified)
- packages/core/src/index.ts (modified)
- packages/core/src/types/events.ts (modified)
- tools/signaling-server/src/index.ts (modified)
- tools/signaling-server/src/server.ts (modified)
- tools/signaling-server/src/server.test.ts (modified)
- packages/sdk/src/tom-client.ts (modified)
- apps/demo/src/main.ts (modified)
- apps/demo/index.html (modified)
