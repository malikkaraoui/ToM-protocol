# Story 1.3: Shared Types & Error Foundation

Status: done

## Story

As a developer,
I want shared TypeScript types for MessageEnvelope, TomError, TomErrorCode, and event definitions,
so that all packages use a consistent type system from day one.

## Acceptance Criteria

1. **Given** the core package is imported by another package **When** the developer accesses the public API **Then** MessageEnvelope interface is available with fields: id, from, to, via, type, payload, timestamp, signature
2. **Given** the core package is imported **When** the developer handles errors **Then** TomError class extends Error with code (TomErrorCode) and optional context
3. **Given** the core package is imported **When** the developer checks error codes **Then** TomErrorCode union type includes: TRANSPORT_FAILED, PEER_UNREACHABLE, SIGNALING_TIMEOUT, INVALID_ENVELOPE, IDENTITY_MISSING, RELAY_REJECTED, CRYPTO_FAILED
4. **Given** the core package is imported **When** the developer uses typed events **Then** event type definitions are available for the typed EventEmitter pattern
5. **Given** all types are defined **When** the project builds **Then** all types are exported as ESM and CJS via tsup dual build

## Tasks / Subtasks

- [x] Task 1: Create MessageEnvelope types (AC: #1)
  - [x] Create `packages/core/src/types/envelope.ts`
  - [x] Define MessageEnvelope interface with fields: id, from, to, via, type, payload, timestamp, signature
  - [x] Use NodeId type from identity module for from/to/via fields

- [x] Task 2: Create error types (AC: #2, #3)
  - [x] Create `packages/core/src/errors/tom-error.ts`
  - [x] Define TomErrorCode union type
  - [x] Implement TomError class extending Error with code and optional context

- [x] Task 3: Create event type definitions (AC: #4)
  - [x] Create `packages/core/src/types/events.ts`
  - [x] Define TomEventMap interface for typed EventEmitter pattern
  - [x] Define event types: message:received, message:sent, peer:connected, peer:disconnected, identity:ready

- [x] Task 4: Create barrel exports (AC: #5)
  - [x] Create `packages/core/src/types/index.ts`
  - [x] Create `packages/core/src/errors/index.ts`
  - [x] Update `packages/core/src/index.ts` with re-exports

- [x] Task 5: Write tests (AC: #1, #2, #3, #4)
  - [x] Create `packages/core/src/errors/tom-error.test.ts`
  - [x] Create `packages/core/src/types/envelope.test.ts`

- [x] Task 6: Build and validate (AC: #5)
  - [x] Run `pnpm build` — zero errors
  - [x] Run `pnpm test` — all tests pass
  - [x] Run `pnpm lint` — zero warnings

## Dev Notes

### Architecture Compliance

- **ADR-003**: Wire format JSON — MessageEnvelope: `{id, from, to, via, type, payload, timestamp, signature}` [Source: architecture.md#ADR-003]
- **ADR-006**: Unified node model — types shared across all node roles [Source: architecture.md#ADR-006]
- Use NodeId from identity module (Story 1.2) for `from`, `to`, `via` fields

### References

- [Source: architecture.md#ADR-003] — Wire Format
- [Source: epics.md#Story 1.3] — acceptance criteria
- [Source: 1-2-node-identity-generation-and-persistence.md] — previous story

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- No issues encountered

### Completion Notes List

- All 6 tasks completed
- 6 new tests (5 TomError + 1 MessageEnvelope), 21 total passing
- Build, test, lint all green

### File List

- packages/core/src/index.ts (modified)
- packages/core/src/types/index.ts (new)
- packages/core/src/types/envelope.ts (new)
- packages/core/src/types/envelope.test.ts (new)
- packages/core/src/types/events.ts (new)
- packages/core/src/errors/index.ts (new)
- packages/core/src/errors/tom-error.ts (new)
- packages/core/src/errors/tom-error.test.ts (new)
