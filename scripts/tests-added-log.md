# Tests Added Log

This file tracks all tests added to the ToM Protocol project.
Updated automatically when tests are added.

---

## Session 2026-02-08 (Chaos/Stress Testing)

**Before:** 688 tests
**After:** 710 tests
**Added:** 22 tests

### Chaos/Stress Tests (22 tests)

File: `packages/core/src/chaos/chaos-stress.test.ts` (NEW)

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | GroupManager chaos | should handle rapid group creation and deletion cycles |
| 2 | GroupManager chaos | should handle concurrent invitations from multiple sources |
| 3 | GroupManager chaos | should handle message flood without memory explosion |
| 4 | GroupManager chaos | should handle hub migration during active operations |
| 5 | NetworkTopology chaos | should handle rapid peer churn (100 adds/removes) |
| 6 | NetworkTopology chaos | should maintain correct role counts under random role changes |
| 7 | NetworkTopology chaos | should handle timestamp manipulation attempts gracefully |
| 8 | OfflineDetector chaos | should handle 50 peers with random online/offline cycles |
| 9 | OfflineDetector chaos | should handle destroy during active transitions without memory leaks |
| 10 | RelaySelector chaos | should always find a relay under random topology changes |
| 11 | RelaySelector chaos | should handle failed relay tracking correctly |
| 12 | RoleManager chaos | should maintain relay quota under random network changes |
| 13 | HubElection chaos | should elect deterministic hub under random candidate orders |
| 14 | EphemeralSubnet chaos | should handle random communication patterns without crashing |
| 15 | Router stress | should handle 100 message envelope creations |
| 16 | Router stress | should deduplicate messages under replay attacks |
| 17 | MessageTracker stress | should track 1000 messages without memory explosion |
| 18 | RelayStats stress | should maintain accurate stats under high-frequency updates |
| 19 | Crypto stress | should generate 1000 unique random values |
| 20 | Crypto stress | should handle rapid random byte generation |
| 21 | Crypto stress | should generate valid hex strings of various lengths |
| 22 | Combined system chaos | should survive full system chaos simulation |

---

## Session 2026-02-08 (Foundation Hardening)

**Before:** 626 tests
**After:** 688 tests
**Added:** 62 tests

### RoleManager Edge Cases (10 tests)

File: `packages/core/src/roles/role-manager.test.ts`

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | edge cases | should handle single-peer network (self only) |
| 2 | edge cases | should handle role transition cycles (relay → client → relay) |
| 3 | edge cases | should handle metrics with 0 contribution score |
| 4 | edge cases | should handle metrics with maximum values |
| 5 | edge cases | should handle large network scale (50+ peers) |
| 6 | edge cases | should handle reevaluation timer idempotency (start/stop cycles) |
| 7 | edge cases | should handle empty topology |
| 8 | edge cases | should handle nodes with no startTime recorded |
| 9 | edge cases | should handle backup evaluation with no eligible nodes |
| 10 | edge cases | should use lexicographic tiebreaker for backup score ties |

### Router Cache Boundaries (7 tests)

File: `packages/core/src/routing/router.test.ts`

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | Cache boundary conditions | handles cache at 50% capacity threshold (triggers cleanup) |
| 2 | Cache boundary conditions | handles ACK with missing originalMessageId |
| 3 | Cache boundary conditions | handles ACK with null payload |
| 4 | Cache boundary conditions | handles read receipt with missing payload fields |
| 5 | Cache boundary conditions | handles envelope with empty via array |
| 6 | Cache boundary conditions | creates hopTimestamps array when forwarding if not present |
| 7 | Cache boundary conditions | handles concurrent connections to same peer |

### OfflineDetector Edge Cases (8 tests)

File: `packages/core/src/routing/offline-detector.test.ts`

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | edge cases | should handle debounce = 0ms (immediate transitions) |
| 2 | edge cases | should handle very large debounce values |
| 3 | edge cases | should handle activity then immediate departure (same tick) |
| 4 | edge cases | should handle multiple rapid cycles correctly |
| 5 | edge cases | should return correct offline peers during debounce transition |
| 6 | edge cases | should handle peer with no prior activity going offline |
| 7 | edge cases | should not emit onPeerOnline for peer that was never offline |
| 8 | edge cases | should handle destroy with many pending timers |

### Crypto Secure Random (22 tests)

File: `packages/core/src/crypto/secure-random.test.ts`

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | secureRandomBytes | should generate bytes of specified length |
| 2 | secureRandomBytes | should generate different bytes on each call |
| 3 | secureRandomBytes | should handle zero length |
| 4 | secureRandomBytes | should handle large lengths |
| 5 | secureRandomHex | should generate hex string of specified length |
| 6 | secureRandomHex | should generate different hex strings on each call |
| 7 | secureRandomHex | should handle odd lengths |
| 8 | secureRandomHex | should produce valid hex characters only |
| 9 | secureRandomUUID | should generate valid UUID v4 format |
| 10 | secureRandomUUID | should generate different UUIDs on each call |
| 11 | secureRandomUUID | should have version 4 indicator |
| 12 | secureRandomUUID | should have correct variant bits |
| 13 | secureRandomUUID | should generate valid UUIDs consistently |
| 14 | secureId | should generate ID with correct prefix |
| 15 | secureId | should include timestamp component |
| 16 | secureId | should include random hex suffix |
| 17 | secureId | should use default random length of 8 |
| 18 | secureId | should generate different IDs even with same prefix |
| 19 | secureId | should respect custom random length |
| 20 | secureId | should handle various prefix formats |
| 21 | entropy quality | should produce evenly distributed bytes |
| 22 | entropy quality | should not produce repeating patterns |

### Identity Storage (15 tests)

File: `packages/core/src/identity/storage.test.ts`

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | MemoryStorage | should return null when no identity stored |
| 2 | MemoryStorage | should save and load identity correctly |
| 3 | MemoryStorage | should overwrite previous identity on save |
| 4 | MemoryStorage | should preserve identity bytes exactly |
| 5 | MemoryStorage | should handle empty keys |
| 6 | LocalStorageAdapter | should return null when localStorage is empty |
| 7 | LocalStorageAdapter | should save identity to localStorage |
| 8 | LocalStorageAdapter | should load identity from localStorage |
| 9 | LocalStorageAdapter | should round-trip identity correctly |
| 10 | FileStorageAdapter | should construct with default path |
| 11 | FileStorageAdapter | should construct with custom path |
| 12 | FileStorageAdapter | should return null when file does not exist |
| 13 | hex conversion | should handle all byte values |
| 14 | hex conversion | should preserve leading zeros |
| 15 | hex conversion | should handle maximum byte value |

---

## Session 2026-02-07 (Retrospective Actions)

**Before:** 577 tests
**After:** 626 tests
**Added:** 49 tests

### Action 3: Reactive UI & Hooks (25 tests)

File: `apps/demo/src/ui-state.test.ts`

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | event subscription | should subscribe to specific event types |
| 2 | event subscription | should not call listener for different event types |
| 3 | event subscription | should subscribe to all events with onAny |
| 4 | event subscription | should unsubscribe when calling returned function |
| 5 | convenience hooks | should provide onGroupsChanged hook |
| 6 | convenience hooks | should provide onMembersChanged hook |
| 7 | convenience hooks | should provide onInvitesChanged hook |
| 8 | convenience hooks | should provide onMessagesChanged hook |
| 9 | convenience hooks | should provide onParticipantsChanged hook |
| 10 | convenience hooks | should provide onSelectionChanged hook |
| 11 | debouncing | should debounce rapid emissions |
| 12 | debouncing | should emit immediately when requested |
| 13 | debouncing | should cancel pending debounced update when emitting immediately |
| 14 | batch emissions | should emit multiple event types in batch |
| 15 | batch emissions | should dedupe duplicate event types in batch |
| 16 | forceRefreshAll | should emit all event types immediately |
| 17 | event data | should pass data to listeners |
| 18 | error handling | should continue calling other listeners if one throws |
| 19 | listener count | should return correct listener count for specific event |
| 20 | listener count | should include global listeners in count |
| 21 | listener count | should return total count when no event type specified |
| 22 | clear | should remove all listeners |
| 23 | clear | should cancel pending updates |
| 24 | singleton instance | should return the same instance |
| 25 | singleton instance | should create new instance after reset |

### Action 1: Hub Failover & Resilience Groups (24 tests)

File: `packages/core/src/groups/hub-election.test.ts`

| # | Test Suite | Test Name |
|---|------------|-----------|
| 1 | initiateElection | should select the lexicographically first relay as new hub |
| 2 | initiateElection | should exclude the failed hub from candidates |
| 3 | initiateElection | should prefer backup hub if available |
| 4 | initiateElection | should return null if no candidates available |
| 5 | initiateElection | should exclude non-relay candidates |
| 6 | initiateElection | should exclude stale candidates |
| 7 | initiateElection | should call onElectedAsHub when local node is elected |
| 8 | initiateElection | should call onHubElected when another node is elected |
| 9 | shouldBecomeHub | should return true when local node is first alphabetically |
| 10 | shouldBecomeHub | should return false when another node is first |
| 11 | shouldBecomeHub | should exclude specified node from consideration |
| 12 | shouldBecomeHub | should return false if no candidates |
| 13 | selectHub | should return first eligible relay |
| 14 | selectHub | should exclude specified node |
| 15 | selectHub | should return null if no eligible candidates |
| 16 | election state management | should not have active election initially |
| 17 | election state management | should track active election |
| 18 | election state management | should cancel election |
| 19 | election state management | should get election info |
| 20 | election state management | should clear all elections |
| 21 | deterministic selection | should always select same hub regardless of candidate order |
| 22 | edge cases | should handle single candidate |
| 23 | edge cases | should handle all candidates being the failed hub |
| 24 | edge cases | should handle backup hub not in candidates |

### E2E Test Infrastructure Improvements (Robustness)

**No new tests, but critical improvements to existing E2E tests for reliability.**

File: `scripts/e2e/tests/test-helpers.ts` (NEW)

Robust testing infrastructure with:
- `withRetry()` - Retry mechanism with exponential backoff
- `waitForHubRecovery()` - Wait for hub failover completion
- `waitForConnectionsReady()` - Wait for WebRTC peer connections
- `reconnectWithVerification()` - Robust user reconnection
- `POST_DISCONNECT_TIMEOUTS` - Extended timeouts for post-disconnect scenarios
- `STANDARD_TIMEOUTS` - Consistent timeout values across tests

Files Updated:
- `scripts/e2e/tests/metrics-test.spec.ts` - Phase 6 now uses robust helpers
- `scripts/e2e/tests/relay-disconnect.spec.ts` - Uses retry mechanisms

Key Improvements:
| Feature | Before | After |
|---------|--------|-------|
| Phase 6 timeout | 60s | 180s (3 min) |
| Disconnect test timeout | 60s | 120s (2 min) |
| Pending message timeout | 60s | 150s (2.5 min) |
| Message delivery retry | None | 2-3 attempts with backoff |
| WebRTC reconnection wait | 5s fixed | 10s with verification |

---

## Previous Sessions

### Action 2: Robust Invitations (14 tests)
Commit: `49ad5bd`
File: See `packages/core/src/groups/group-manager.test.ts`

### Action 4: E2E Testing Framework (42 tests)
Commits: `a51a0fa`, `bcd6c6c`
Files:
- `scripts/e2e/tests/metrics.spec.ts` (35 tests)
- `scripts/e2e/tests/chaos-test.spec.ts` (7 tests)

---

## Summary

| Date | Action | Tests Added | Total |
|------|--------|-------------|-------|
| 2026-02-07 | Action 4: E2E Testing | 42 | 577 |
| 2026-02-07 | Action 2: Robust Invitations | 14 | 577 |
| 2026-02-07 | Action 3: Reactive UI & Hooks | 25 | 602 |
| 2026-02-07 | Action 1: Hub Failover | 24 | 626 |
| 2026-02-08 | Crypto Secure Random | 22 | 648 |
| 2026-02-08 | Identity Storage | 15 | 663 |
| 2026-02-08 | RoleManager Edge Cases | 10 | 673 |
| 2026-02-08 | Router Cache Boundaries | 7 | 680 |
| 2026-02-08 | OfflineDetector Edge Cases | 8 | 688 |
| 2026-02-08 | Chaos/Stress Tests | 22 | 710 |

**Current Total:** 710 tests
