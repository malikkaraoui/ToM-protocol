# Permanent Issue Backlog

This file contains 20+ micro-session tasks that can be created as GitHub issues.
Maintainers: Copy these to GitHub issues using the Micro-Session Task template.

---

## Testing Tasks

### 1. Add edge case tests for RelaySelector
- **Complexity**: small (30-60 min)
- **Category**: testing
- **Area**: core/routing
- **Objective**: Add tests for edge cases in relay selection
- **Files**: `packages/core/src/routing/relay-selector.test.ts`
- **Acceptance**:
  - [ ] Test when all relays are failed
  - [ ] Test when target is self
  - [ ] Test with empty topology

### 2. Add stress tests for PeerGossip
- **Complexity**: small (30-60 min)
- **Category**: testing
- **Area**: core/discovery
- **Objective**: Test gossip behavior under high peer churn
- **Files**: `packages/core/src/discovery/peer-gossip.test.ts`
- **Acceptance**:
  - [ ] Test with 50+ peers joining/leaving
  - [ ] Test deduplication under rapid updates

### 3. Add integration tests for TomClient lifecycle
- **Complexity**: medium (1-2 hours)
- **Category**: testing
- **Area**: sdk
- **Objective**: Test full connect/disconnect/reconnect cycle
- **Files**: `packages/sdk/src/tom-client.test.ts`
- **Acceptance**:
  - [ ] Test graceful disconnect
  - [ ] Test reconnection with same identity
  - [ ] Test message delivery after reconnect

### 4. Add tests for GroupManager message ordering
- **Complexity**: small (30-60 min)
- **Category**: testing
- **Area**: core/groups
- **Objective**: Verify messages maintain order within groups
- **Files**: `packages/core/src/groups/group-manager.test.ts`
- **Acceptance**:
  - [ ] Test message sequence numbers
  - [ ] Test out-of-order delivery handling

### 5. Add tests for encryption key rotation
- **Complexity**: small (30-60 min)
- **Category**: testing
- **Area**: core/crypto
- **Objective**: Test key rotation scenarios
- **Files**: `packages/core/src/crypto/encryption.test.ts`
- **Acceptance**:
  - [ ] Test key rotation mid-conversation
  - [ ] Test messages with old keys rejected

---

## Documentation Tasks

### 6. Add JSDoc to all Router public methods
- **Complexity**: micro (< 30 min)
- **Category**: analysis
- **Area**: core/routing
- **Objective**: Document Router API with JSDoc
- **Files**: `packages/core/src/routing/router.ts`
- **Acceptance**:
  - [ ] All public methods have @param and @returns
  - [ ] Examples in complex methods

### 7. Add JSDoc to NetworkTopology
- **Complexity**: micro (< 30 min)
- **Category**: analysis
- **Area**: core/discovery
- **Objective**: Document NetworkTopology API
- **Files**: `packages/core/src/discovery/network-topology.ts`
- **Acceptance**:
  - [ ] All public methods documented
  - [ ] Type definitions have descriptions

### 8. Document MCP server tool responses
- **Complexity**: micro (< 30 min)
- **Category**: analysis
- **Area**: mcp-server
- **Objective**: Add examples to MCP tool documentation
- **Files**: `tools/mcp-server/README.md`
- **Acceptance**:
  - [ ] Each tool has example request/response
  - [ ] Error cases documented

### 9. Create troubleshooting guide
- **Complexity**: small (30-60 min)
- **Category**: analysis
- **Area**: documentation
- **Objective**: Document common issues and solutions
- **Files**: Create `docs/TROUBLESHOOTING.md`
- **Acceptance**:
  - [ ] WebRTC connection issues
  - [ ] Signaling server problems
  - [ ] Build/test failures

### 10. Document demo keyboard shortcuts
- **Complexity**: micro (< 30 min)
- **Category**: analysis
- **Area**: demo
- **Objective**: Document Snake game controls in demo
- **Files**: `apps/demo/README.md`
- **Acceptance**:
  - [ ] All keyboard shortcuts listed
  - [ ] Game rules explained

---

## Building Tasks

### 11. Add connection quality indicator to SDK
- **Complexity**: medium (1-2 hours)
- **Category**: building
- **Area**: sdk
- **Objective**: Expose connection quality to SDK users
- **Files**: `packages/sdk/src/tom-client.ts`
- **Acceptance**:
  - [ ] `onConnectionQualityChange` callback
  - [ ] Quality levels: good, degraded, poor
  - [ ] Test coverage

### 12. Add message retry with exponential backoff
- **Complexity**: small (30-60 min)
- **Category**: building
- **Area**: core/routing
- **Objective**: Implement retry logic for failed messages
- **Files**: `packages/core/src/routing/router.ts`
- **Acceptance**:
  - [ ] Max 3 retries
  - [ ] Exponential backoff (1s, 2s, 4s)
  - [ ] Tests for retry scenarios

### 13. Add typing indicator support
- **Complexity**: medium (1-2 hours)
- **Category**: building
- **Area**: sdk
- **Objective**: Add typing indicator to chat
- **Files**: `packages/sdk/src/tom-client.ts`, `apps/demo/src/main.ts`
- **Acceptance**:
  - [ ] `sendTypingIndicator(peerId)` method
  - [ ] `onTypingIndicator` callback
  - [ ] Demo UI shows typing state

### 14. Add message read receipts to demo UI
- **Complexity**: small (30-60 min)
- **Category**: building
- **Area**: demo
- **Objective**: Show read receipts in chat UI
- **Files**: `apps/demo/src/main.ts`, `apps/demo/index.html`
- **Acceptance**:
  - [ ] Double-check icon for read messages
  - [ ] Single-check for delivered

### 15. Add network stats display to demo
- **Complexity**: small (30-60 min)
- **Category**: building
- **Area**: demo
- **Objective**: Show network stats in demo UI
- **Files**: `apps/demo/src/main.ts`
- **Acceptance**:
  - [ ] Active connections count
  - [ ] Messages sent/received
  - [ ] Current relay

---

## Verification Tasks

### 16. Audit TomError usage consistency
- **Complexity**: small (30-60 min)
- **Category**: verification
- **Area**: core
- **Objective**: Ensure all errors use TomError
- **Files**: All `packages/core/src/**/*.ts`
- **Acceptance**:
  - [ ] No raw `throw new Error()`
  - [ ] Consistent error codes
  - [ ] Report findings in PR

### 17. Verify ADR compliance in crypto module
- **Complexity**: small (30-60 min)
- **Category**: verification
- **Area**: core/crypto
- **Objective**: Verify crypto follows ADR-004
- **Files**: `packages/core/src/crypto/`
- **Acceptance**:
  - [ ] Uses TweetNaCl.js
  - [ ] X25519 for key exchange
  - [ ] XSalsa20-Poly1305 for encryption

### 18. Review signaling server for security issues
- **Complexity**: small (30-60 min)
- **Category**: verification
- **Area**: signaling-server
- **Objective**: Security audit of signaling server
- **Files**: `tools/signaling-server/src/`
- **Acceptance**:
  - [ ] No sensitive data logging
  - [ ] Rate limiting present
  - [ ] Input validation complete

### 19. Verify all exports in index.ts files
- **Complexity**: micro (< 30 min)
- **Category**: verification
- **Area**: core
- **Objective**: Ensure all public APIs are exported
- **Files**: `packages/core/src/index.ts`, `packages/sdk/src/index.ts`
- **Acceptance**:
  - [ ] All public classes exported
  - [ ] All public types exported
  - [ ] No internal-only exports

### 20. Check test coverage gaps
- **Complexity**: small (30-60 min)
- **Category**: verification
- **Area**: core
- **Objective**: Identify untested code paths
- **Files**: Run coverage report
- **Acceptance**:
  - [ ] Generate coverage report
  - [ ] List uncovered lines
  - [ ] Create follow-up issues

---

## CI/CD Tasks

### 21. Add test coverage reporting to CI
- **Complexity**: small (30-60 min)
- **Category**: building
- **Area**: ci-cd
- **Objective**: Add coverage report to CI pipeline
- **Files**: `.github/workflows/ci.yml`, `vitest.config.ts`
- **Acceptance**:
  - [ ] Coverage report generated
  - [ ] Report uploaded as artifact
  - [ ] Threshold enforcement (optional)

### 22. Add build size tracking
- **Complexity**: small (30-60 min)
- **Category**: building
- **Area**: ci-cd
- **Objective**: Track bundle size in CI
- **Files**: `.github/workflows/ci.yml`
- **Acceptance**:
  - [ ] Report bundle sizes
  - [ ] Compare with previous build
  - [ ] Warn on significant increase

### 23. Add dependency audit to CI
- **Complexity**: micro (< 30 min)
- **Category**: building
- **Area**: ci-cd
- **Objective**: Add `pnpm audit` to CI
- **Files**: `.github/workflows/ci.yml`
- **Acceptance**:
  - [ ] `pnpm audit` runs in CI
  - [ ] Failures are warnings (not blocking)

---

## Refactoring Tasks

### 24. Extract message validation to separate module
- **Complexity**: medium (1-2 hours)
- **Category**: building
- **Area**: core/routing
- **Objective**: Move validation logic out of Router
- **Files**: `packages/core/src/routing/router.ts`
- **Acceptance**:
  - [ ] Create `message-validator.ts`
  - [ ] Move validation functions
  - [ ] Update imports
  - [ ] All tests pass

### 25. Simplify EphemeralSubnetManager API
- **Complexity**: small (30-60 min)
- **Category**: building
- **Area**: core/discovery
- **Objective**: Reduce API surface complexity
- **Files**: `packages/core/src/discovery/ephemeral-subnet.ts`
- **Acceptance**:
  - [ ] Consolidate similar methods
  - [ ] Update callers
  - [ ] Tests pass

---

## Stats

- **Total Issues**: 25
- **By Complexity**: micro (5), small (14), medium (6)
- **By Category**: testing (5), analysis (5), building (10), verification (5)

---

*Last updated: February 2026*
