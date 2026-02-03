# Story 3.4: Dual-Role Node (Client + Relay)

Status: done

## Story

As a network participant,
I want my node to simultaneously send/receive my own messages and relay messages for others,
So that the network grows stronger as more participants join.

## Acceptance Criteria

1. **Given** a node is assigned both client and relay roles **When** it receives a message addressed to another node **Then** it forwards the message as a relay without interfering with its own messaging **And** relay forwarding and personal messaging share the transport layer without conflicts

2. **Given** a dual-role node is sending its own message **When** a relay request arrives simultaneously **Then** both operations complete without blocking each other **And** the event system handles both message flows independently

3. **Given** a dual-role node's relay responsibilities increase **When** the node detects performance degradation for its own messages **Then** it emits a capacity warning event **And** the role system can redistribute relay duties to other capable nodes

## Tasks / Subtasks

- [x] Task 1: Add relay statistics tracking (AC: #3)
  - [x] Create `packages/core/src/routing/relay-stats.ts`
  - [x] Track: messages relayed count, average relay latency, own messages count
  - [x] Expose `getStats()` method for monitoring

- [x] Task 2: Add capacity warning system (AC: #3)
  - [x] Define capacity thresholds (e.g., relay:own ratio > 10:1)
  - [x] Emit `capacity:warning` event when threshold exceeded
  - [x] Add `onCapacityWarning` callback to router events

- [x] Task 3: Integrate relay stats into Router (AC: #1, #2, #3)
  - [x] Track relay events in handleIncoming when forwarding
  - [x] Track own message events in sendViaRelay
  - [x] Compute and check capacity after each operation

- [x] Task 4: Integrate into TomClient SDK (AC: #3)
  - [x] Expose `getRelayStats()` on TomClient
  - [x] Add `onCapacityWarning(handler)` callback
  - [x] Emit status when capacity warning triggered

- [x] Task 5: Update demo UI to show relay activity (AC: #1, #3)
  - [x] Show relay stats in topology stats area
  - [x] Visual indicator when acting as relay
  - [x] Warning display when capacity exceeded

- [x] Task 6: Write tests (AC: #1, #2, #3)
  - [x] Create `packages/core/src/routing/relay-stats.test.ts`
  - [x] Test: relay count increments on forward
  - [x] Test: own message count increments on send
  - [x] Test: capacity warning emits at threshold
  - [x] Test: concurrent relay and send operations

- [x] Task 7: Build and validate
  - [x] Run `pnpm build` — zero errors
  - [x] Run `pnpm test` — all tests pass
  - [x] Run `pnpm lint` — zero warnings

## Dev Notes

### Architecture Compliance

- **ADR-006**: Unified node model — every node runs the same code and can relay. The role is network-assigned but doesn't gate behavior.
- **ADR-007**: Relay is network-imposed. A node with relay role should prioritize forwarding but also handles its own messages.

### Critical Boundaries

- **DO NOT** implement relay selection in this story — that was Story 3.3
- **DO NOT** implement multi-hop routing — that is Story 5.1
- **DO** track relay statistics for capacity monitoring
- **DO** emit events when capacity thresholds are exceeded
- **Current state**: Router already forwards messages not addressed to local node

### Previous Story Learnings

- Router.handleIncoming already forwards messages to next hop
- RoleManager assigns relay role based on network needs
- Transport layer handles async operations naturally
- 86 tests before this story

### References

- [Source: architecture.md#ADR-006] — Unified Node Model
- [Source: architecture.md#ADR-007] — Role Model

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- No issues encountered

### Completion Notes List

- Created RelayStats class for tracking relay/own message counts
- Integrated capacity warning system (threshold-based)
- Added getRelayStats() and onCapacityWarning() to TomClient
- Demo UI now shows "relayed" count in stats
- 8 new tests for RelayStats
- Total: 94 tests passing
- Build, test, lint all green

### File List

- packages/core/src/routing/relay-stats.ts (new)
- packages/core/src/routing/relay-stats.test.ts (new)
- packages/core/src/routing/index.ts (modified)
- packages/core/src/index.ts (modified)
- packages/sdk/src/tom-client.ts (modified)
- apps/demo/src/main.ts (modified)
