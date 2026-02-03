# Story 3.3: Automatic Relay Selection

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a sender,
I want the network to automatically select the best relay for my message,
so that I don't need to know the network topology to send a message.

## Acceptance Criteria

1. **Given** sender A wants to send a message to recipient B **When** A does not specify a relay in the via field **Then** the routing layer selects the best available relay based on network topology **And** the message is sent through the selected relay transparently

2. **Given** multiple relays are available to route a message **When** the routing layer selects a relay **Then** it chooses based on proximity (fewest hops) and availability **And** the selected relay is populated in the via field before sending

3. **Given** no relay is available to reach the recipient **When** the routing layer fails to find a path **Then** a PEER_UNREACHABLE error is returned to the sender **And** the error includes context about why no path was found

## Tasks / Subtasks

- [x] Task 1: Create RelaySelector in core (AC: #1, #2)
  - [x] Create `packages/core/src/routing/relay-selector.ts`
  - [x] Define `RelaySelectionResult` interface: `{ relayId: string, reason: string } | { relayId: null, reason: string }`
  - [x] Implement `RelaySelector` class with `selectBestRelay(to: NodeId, topology: NetworkTopology): RelaySelectionResult`
  - [x] Selection algorithm: prefer relays that are online (not stale), have relay role, fewest hops to recipient
  - [x] Create barrel export `packages/core/src/routing/index.ts`
  - [x] Export from `packages/core/src/index.ts`

- [x] Task 2: Implement relay selection algorithm (AC: #1, #2, #3)
  - [x] Get all nodes with relay role from topology: `topology.getRelayNodes()`
  - [x] Filter by online status: `topology.getPeerStatus(nodeId) === 'online'`
  - [x] If recipient is directly reachable (no relay needed), return `{ relayId: null, reason: 'direct-path' }`
  - [x] If no relays available, return `{ relayId: null, reason: 'no-relays-available' }`
  - [x] Rank remaining relays by: 1) online status, 2) proximity (currently all same hop=1), 3) recency (lastSeen)
  - [x] Return best relay: `{ relayId: bestRelayId, reason: 'best-available' }`

- [x] Task 3: Integrate RelaySelector into TomClient SDK (AC: #1, #2, #3)
  - [x] Import `RelaySelector` in `packages/sdk/src/tom-client.ts`
  - [x] Create RelaySelector instance in TomClient constructor
  - [x] Modify `sendMessage()`: if no relayId provided, call `relaySelector.selectBestRelay(to, topology)`
  - [x] If relay found, use it; if not and reason is 'no-relays-available', throw PEER_UNREACHABLE error
  - [x] If reason is 'direct-path', attempt direct connection (current behavior)
  - [x] Add error context to PEER_UNREACHABLE: `{ to, reason: 'no relay available to reach recipient' }`

- [x] Task 4: Update demo UI to use automatic relay (AC: #1)
  - [x] Modify `apps/demo/src/main.ts` `sendMessage()` function
  - [x] Remove manual relay selection logic (currently picks first online peer as relay)
  - [x] Pass undefined/null as relayId to let SDK auto-select
  - [x] Display relay used in message status (optional): "sent via R" or "sent direct"

- [x] Task 5: Handle edge cases (AC: #3)
  - [x] If all relays are stale/offline, return appropriate error
  - [x] If recipient is self, reject early with error
  - [x] If topology is empty (only self), return PEER_UNREACHABLE
  - [x] Log relay selection decisions at debug level

- [x] Task 6: Write tests (AC: #1, #2, #3)
  - [x] Create `packages/core/src/routing/relay-selector.test.ts`
  - [x] Test: returns null when no relays in topology
  - [x] Test: selects only online relays (not stale/offline)
  - [x] Test: selects relay with relay role (not client-only)
  - [x] Test: returns null with reason 'direct-path' when recipient is direct peer
  - [x] Test: returns error context when no path found
  - [x] Test: prefers relay with most recent lastSeen when multiple available
  - [x] Update SDK tests if needed

- [x] Task 7: Build and validate
  - [x] Run `pnpm build` — zero errors
  - [x] Run `pnpm test` — all tests pass
  - [x] Run `pnpm lint` — zero warnings
  - [x] Export new types from `packages/core/src/index.ts`

## Dev Notes

### Architecture Compliance

- **ADR-006**: Unified node model — relay selection is transparent to the node. Every node can be a relay, the network decides. [Source: architecture.md#ADR-006]
- **ADR-007**: Role model — relay is network-imposed. Selection prioritizes nodes with assigned relay role. [Source: architecture.md#ADR-007]
- **ADR-001**: Messages always go through a relay — no direct A→B in protocol. But for iteration 2, we fall back to signaling relay if no dedicated relay available. [Source: architecture.md#ADR-001]

### Critical Boundaries

- **DO NOT** implement multi-hop routing (A→R1→R2→B) — that is Story 5.1
- **DO NOT** implement relay failover/rerouting — that is Story 5.2
- **DO NOT** implement load balancing or contribution scoring in relay selection — that is Epic 5
- **DO** use topology.getRelayNodes() from Story 3.2 to find relay candidates
- **DO** use topology.getPeerStatus() to filter online-only relays
- **DO** keep selection simple for iteration 2: online + relay role + most recent lastSeen
- **Important**: Current demo picks any online peer as relay. This story fixes that to use proper relay role.

### Previous Story Learnings (from Story 3.2)

- RoleManager is in `packages/core/src/roles/role-manager.ts`
- NetworkTopology has `getRelayNodes()` and `getNodesByRole(role)` methods
- PeerInfo has `roles: NodeRole[]` array (breaking change from 3.2)
- Topology `getPeerStatus()` returns 'online' | 'stale' | 'offline'
- Test timing with vitest fake timers — use `vi.advanceTimersByTime()` for eligibility delays
- 77 tests currently passing

### Git Intelligence

Recent commits (most recent first):
- `1fe1d1e` fix: make chat UI responsive for mobile devices
- `e772bbf` feat: implement dynamic role assignment (Story 3.2)
- `4bf6986` fix: don't remove peers from topology on heartbeat timeout
- `8eaa760` fix: keep peers alive via heartbeat and periodic UI refresh
- `284abea` feat: implement peer discovery protocol (Story 3.1)

Key patterns from commits:
- feat/fix prefix convention
- Files commonly modified together: tom-client.ts, main.ts
- New modules follow: class + tests + barrel export pattern

### Project Structure Notes

New files to create:
```
packages/core/src/routing/
├── index.ts                    # Barrel export
├── relay-selector.ts           # RelaySelector class
└── relay-selector.test.ts      # Unit tests
```

Existing files to modify:
- `packages/core/src/index.ts` — add routing exports
- `packages/sdk/src/tom-client.ts` — integrate RelaySelector, modify sendMessage()
- `apps/demo/src/main.ts` — remove manual relay selection, use auto-select

### References

- [Source: architecture.md#ADR-006] — Unified Node Model
- [Source: architecture.md#ADR-007] — Role Model
- [Source: epics.md#Story 3.3] — Acceptance criteria
- [Source: architecture.md#Implementation Patterns] — Event-driven, typed EventEmitter

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- No debug issues encountered during implementation

### Completion Notes List

- Created RelaySelector class in packages/core/src/routing/relay-selector.ts
- Implemented selection algorithm: online + relay role + most recent lastSeen
- Integrated into TomClient SDK with auto-selection when relayId not provided
- Updated demo UI to use automatic relay selection (removed manual relay picking)
- Edge cases handled: recipient-is-self, no-peers, no-relays-available, offline relays
- 9 new tests for RelaySelector, all passing
- Total: 86 tests passing
- Build, test, lint all green

### File List

- packages/core/src/routing/relay-selector.ts (new)
- packages/core/src/routing/relay-selector.test.ts (new)
- packages/core/src/routing/index.ts (modified)
- packages/core/src/index.ts (modified)
- packages/sdk/src/tom-client.ts (modified)
- apps/demo/src/main.ts (modified)
