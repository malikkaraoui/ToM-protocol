# Story 4.3: Message Path Visualization

Status: done

## Story

As a user,
I want to optionally view the path my message took through the network,
so that I can understand the protocol's routing in action.

## Acceptance Criteria

1. **Given** a message has been delivered **When** the user activates path details view (toggle or click) **Then** the UI shows: relays used (via field), direct vs relayed, delivery timing **And** the path information is derived from envelope metadata — no extra network requests

2. **Given** a message traveled through a direct path **When** the user views path details **Then** the display shows "Direct" with no relay hops **And** the timing shows the reduced latency compared to relayed messages

3. **Given** path visualization is optional **When** the user has not activated it **Then** no path information is displayed — the chat remains clean and simple

## Tasks / Subtasks

- [x] Task 1: Create PathInfo type and extraction utilities (AC: #1, #2)
  - [x] Create `PathInfo` interface in `packages/core/src/types/path-info.ts`
  - [x] Fields: `routeType` ('direct' | 'relay'), `relayHops` (string[]), `sentAt`, `deliveredAt`, `latencyMs`
  - [x] Create `extractPathInfo(envelope: MessageEnvelope, receivedAt?: number): PathInfo` utility
  - [x] Create `formatLatency(latencyMs: number): string` utility for human-readable display
  - [x] Export from core package

- [x] Task 2: Verify MessageEnvelope timing metadata (AC: #1, #2)
  - [x] Verified `routeType` field exists (added in Story 4.1)
  - [x] Verified `timestamp` field exists for send timestamp
  - [x] Store `receivedAt` timestamp when message is delivered in SDK
  - [x] Calculate latency from timestamps

- [x] Task 3: Add getPathInfo method to SDK (AC: #1, #2)
  - [x] Add `getPathInfo(messageId): PathInfo | undefined` to TomClient
  - [x] Store received envelope metadata in `receivedEnvelopes` Map with receivedAt timestamp
  - [x] Return PathInfo derived from stored envelope data
  - [x] Export PathInfo type and formatLatency from SDK

- [x] Task 4: Implement path visualization toggle in Demo UI (AC: #3)
  - [x] Add "Show path details" checkbox in sidebar footer
  - [x] Store toggle state in localStorage for persistence
  - [x] Default to OFF (clean and simple per AC#3)

- [x] Task 5: Implement path details display in Demo UI (AC: #1, #2)
  - [x] Add path info section to received message bubbles
  - [x] Show route type: "⚡ direct" (green) or "🔀 relay" (cyan)
  - [x] Show relay hops if relayed (first 8 chars of each nodeId, joined with →)
  - [x] Show latency using formatLatency (e.g., "42ms")
  - [x] Hide path section when toggle is OFF

- [x] Task 6: Style path visualization (AC: #1, #2)
  - [x] Subtle styling with small font (10px), muted colors
  - [x] Background highlight for path info section
  - [x] Visual distinction: direct routes in green, relay routes in cyan
  - [x] Relay hops shown in orange for visibility

- [x] Task 7: Write tests
  - [x] Test: extractPathInfo correctly parses direct route
  - [x] Test: extractPathInfo correctly parses relay route with hops
  - [x] Test: extractPathInfo handles multiple relay hops
  - [x] Test: extractPathInfo infers route type from via field when routeType undefined
  - [x] Test: extractPathInfo handles negative latency (clock skew) by returning 0
  - [x] Test: extractPathInfo handles missing via field gracefully
  - [x] Test: formatLatency formats sub-second, 1-10s, and 10+s correctly
  - [x] Test: getPathInfo returns undefined for unknown message (via SDK tests)

- [x] Task 8: Build and validate
  - [x] Run `pnpm build` — zero errors
  - [x] Run `pnpm test` — 164 tests pass
  - [x] Run `pnpm lint` — zero warnings

## Dev Notes

### Architecture Compliance

- **ADR-001**: Path info uses existing relay metadata from via field
- **ADR-003**: No new wire format changes — use existing envelope fields
- **ADR-006**: Unified node model — PathInfo available on any node

### Implementation Notes

1. **PathInfo extraction**: Uses existing envelope fields (`via`, `routeType`, `timestamp`) plus a `receivedAt` timestamp captured when the message is delivered to compute path info.

2. **SDK storage**: Added `receivedEnvelopes` Map to TomClient that stores `{ envelope, receivedAt }` for each received message, enabling path info retrieval.

3. **Memory management**: `receivedEnvelopes` is cleaned up along with other message data in the periodic cleanup (every 5 minutes).

4. **Toggle persistence**: Uses localStorage with key `tom-show-path-details` to persist user preference.

5. **Visual design**: Path info appears below received messages only (not sent), with subtle styling that doesn't distract from message content.

### References

- [Source: architecture.md#ADR-001] — WebRTC DataChannel via Relay
- [Source: architecture.md#ADR-003] — Wire Format (envelope structure)
- [Source: epics.md#Story-4.3] — Message Path Visualization
- [Source: prd.md] — FR14 (View message path details)
- [Source: 4-1-direct-path-establishment.md] — routeType field implementation
- [Source: 4-2-delivery-confirmation-and-read-receipts.md] — messageOrigins pattern

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A

### Completion Notes List

1. **PathInfo type** created with `routeType`, `relayHops`, `sentAt`, `deliveredAt`, `latencyMs` fields
2. **extractPathInfo()** utility extracts path info from envelope metadata
3. **formatLatency()** utility formats latency for human-readable display (42ms, 1.2s, 15s)
4. **SDK getPathInfo()** method retrieves path info for received messages
5. **receivedEnvelopes Map** stores envelope + receivedAt for path retrieval
6. **Demo UI toggle** persists in localStorage, defaults to OFF
7. **Path display** shows route type icon (⚡/🔀), relay hops, and latency
8. **12 new tests** for path-info utilities
9. **Total test count**: 164 (up from 152)
10. **Also fixed**: Updated role-manager tests to match deterministic consensus algorithm

### File List

**New Files:**
- `packages/core/src/types/path-info.ts` - PathInfo interface, extractPathInfo, formatLatency
- `packages/core/src/types/path-info.test.ts` - 12 unit tests for path utilities

**Modified Files:**
- `packages/core/src/types/index.ts` - Export PathInfo, extractPathInfo, formatLatency
- `packages/core/src/index.ts` - Export path visualization types and utilities
- `packages/sdk/src/tom-client.ts` - Add receivedEnvelopes Map, getPathInfo method, cleanup
- `packages/sdk/src/index.ts` - Export PathInfo, formatLatency
- `apps/demo/index.html` - Add path toggle checkbox and path-info styles
- `apps/demo/src/main.ts` - Toggle state, path info rendering in messages
- `packages/core/src/roles/role-manager.test.ts` - Update tests for deterministic consensus
