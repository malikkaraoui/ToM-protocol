# Tests Added Log

This file tracks all tests added to the ToM Protocol project.
Updated automatically when tests are added.

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

**Current Total:** 626 tests
