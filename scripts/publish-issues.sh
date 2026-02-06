#!/bin/bash
# Publish micro-session issues to GitHub
# Run: ./scripts/publish-issues.sh

set -e

REPO="malikkaraoui/ToM-protocol"
DELAY=2  # Seconds between issues to avoid rate limiting

echo "Publishing 25 micro-session issues to GitHub..."
echo "Repository: $REPO"
echo ""

# Check gh auth
if ! gh auth status &>/dev/null; then
  echo "Error: Not authenticated. Run 'gh auth login' first."
  exit 1
fi

# Create labels if they don't exist
echo "Creating labels..."
gh label create "micro" --description "< 30 min task" --color "c5def5" --repo "$REPO" 2>/dev/null || true
gh label create "small" --description "30-60 min task" --color "bfd4f2" --repo "$REPO" 2>/dev/null || true
gh label create "medium" --description "1-2 hours task" --color "a2d5f2" --repo "$REPO" 2>/dev/null || true
gh label create "testing" --description "Add tests, improve coverage" --color "0e8a16" --repo "$REPO" 2>/dev/null || true
gh label create "verification" --description "Code review, audit" --color "fbca04" --repo "$REPO" 2>/dev/null || true
gh label create "building" --description "New features, bug fixes" --color "1d76db" --repo "$REPO" 2>/dev/null || true
gh label create "analysis" --description "Investigation, documentation" --color "5319e7" --repo "$REPO" 2>/dev/null || true
gh label create "core/routing" --description "Routing module" --color "d4c5f9" --repo "$REPO" 2>/dev/null || true
gh label create "core/discovery" --description "Discovery module" --color "d4c5f9" --repo "$REPO" 2>/dev/null || true
gh label create "core/crypto" --description "Crypto module" --color "d4c5f9" --repo "$REPO" 2>/dev/null || true
gh label create "core/groups" --description "Groups module" --color "d4c5f9" --repo "$REPO" 2>/dev/null || true
gh label create "sdk" --description "SDK package" --color "c2e0c6" --repo "$REPO" 2>/dev/null || true
gh label create "demo" --description "Demo app" --color "c2e0c6" --repo "$REPO" 2>/dev/null || true
gh label create "mcp-server" --description "MCP server" --color "c2e0c6" --repo "$REPO" 2>/dev/null || true
gh label create "signaling-server" --description "Signaling server" --color "c2e0c6" --repo "$REPO" 2>/dev/null || true
gh label create "ci-cd" --description "CI/CD pipeline" --color "ededed" --repo "$REPO" 2>/dev/null || true
echo "Labels created."
echo ""

count=0

# ============================================
# TESTING TASKS
# ============================================

echo "Creating testing issues..."

# Issue 1
gh issue create --repo "$REPO" \
  --title "[testing] Add edge case tests for RelaySelector" \
  --label "good first issue,help wanted,small,testing,core/routing" \
  --body "$(cat <<'EOF'
## Objective
Add tests for edge cases in relay selection.

## Files
- `packages/core/src/routing/relay-selector.test.ts`

## Acceptance Criteria
- [ ] Test when all relays are failed
- [ ] Test when target is self
- [ ] Test with empty topology

## Complexity
**small** (30-60 min)

## Context
Look at existing tests in `relay-selector.test.ts` for patterns. Use `vi.fn()` for mock callbacks.
EOF
)"
((count++))
echo "  Created issue $count: Add edge case tests for RelaySelector"
sleep $DELAY

# Issue 2
gh issue create --repo "$REPO" \
  --title "[testing] Add stress tests for PeerGossip" \
  --label "good first issue,help wanted,small,testing,core/discovery" \
  --body "$(cat <<'EOF'
## Objective
Test gossip behavior under high peer churn.

## Files
- `packages/core/src/discovery/peer-gossip.test.ts`

## Acceptance Criteria
- [ ] Test with 50+ peers joining/leaving
- [ ] Test deduplication under rapid updates

## Complexity
**small** (30-60 min)

## Context
Use loops to simulate rapid peer changes. Verify no duplicate peer entries.
EOF
)"
((count++))
echo "  Created issue $count: Add stress tests for PeerGossip"
sleep $DELAY

# Issue 3
gh issue create --repo "$REPO" \
  --title "[testing] Add integration tests for TomClient lifecycle" \
  --label "good first issue,help wanted,medium,testing,sdk" \
  --body "$(cat <<'EOF'
## Objective
Test full connect/disconnect/reconnect cycle.

## Files
- `packages/sdk/src/tom-client.test.ts`

## Acceptance Criteria
- [ ] Test graceful disconnect
- [ ] Test reconnection with same identity
- [ ] Test message delivery after reconnect

## Complexity
**medium** (1-2 hours)

## Context
May require mocking the signaling server. Check existing test setup patterns.
EOF
)"
((count++))
echo "  Created issue $count: Add integration tests for TomClient lifecycle"
sleep $DELAY

# Issue 4
gh issue create --repo "$REPO" \
  --title "[testing] Add tests for GroupManager message ordering" \
  --label "good first issue,help wanted,small,testing,core/groups" \
  --body "$(cat <<'EOF'
## Objective
Verify messages maintain order within groups.

## Files
- `packages/core/src/groups/group-manager.test.ts`

## Acceptance Criteria
- [ ] Test message sequence numbers
- [ ] Test out-of-order delivery handling

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Add tests for GroupManager message ordering"
sleep $DELAY

# Issue 5
gh issue create --repo "$REPO" \
  --title "[testing] Add tests for encryption key rotation" \
  --label "good first issue,help wanted,small,testing,core/crypto" \
  --body "$(cat <<'EOF'
## Objective
Test key rotation scenarios.

## Files
- `packages/core/src/crypto/encryption.test.ts`

## Acceptance Criteria
- [ ] Test key rotation mid-conversation
- [ ] Test messages with old keys rejected

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Add tests for encryption key rotation"
sleep $DELAY

# ============================================
# DOCUMENTATION TASKS
# ============================================

echo "Creating documentation issues..."

# Issue 6
gh issue create --repo "$REPO" \
  --title "[docs] Add JSDoc to all Router public methods" \
  --label "good first issue,help wanted,micro,analysis,core/routing" \
  --body "$(cat <<'EOF'
## Objective
Document Router API with JSDoc.

## Files
- `packages/core/src/routing/router.ts`

## Acceptance Criteria
- [ ] All public methods have @param and @returns
- [ ] Examples in complex methods

## Complexity
**micro** (< 30 min)
EOF
)"
((count++))
echo "  Created issue $count: Add JSDoc to all Router public methods"
sleep $DELAY

# Issue 7
gh issue create --repo "$REPO" \
  --title "[docs] Add JSDoc to NetworkTopology" \
  --label "good first issue,help wanted,micro,analysis,core/discovery" \
  --body "$(cat <<'EOF'
## Objective
Document NetworkTopology API.

## Files
- `packages/core/src/discovery/network-topology.ts`

## Acceptance Criteria
- [ ] All public methods documented
- [ ] Type definitions have descriptions

## Complexity
**micro** (< 30 min)
EOF
)"
((count++))
echo "  Created issue $count: Add JSDoc to NetworkTopology"
sleep $DELAY

# Issue 8
gh issue create --repo "$REPO" \
  --title "[docs] Document MCP server tool responses" \
  --label "good first issue,help wanted,micro,analysis,mcp-server" \
  --body "$(cat <<'EOF'
## Objective
Add examples to MCP tool documentation.

## Files
- `tools/mcp-server/README.md`

## Acceptance Criteria
- [ ] Each tool has example request/response
- [ ] Error cases documented

## Complexity
**micro** (< 30 min)

## Note
README already exists with basic docs. Add more detailed examples.
EOF
)"
((count++))
echo "  Created issue $count: Document MCP server tool responses"
sleep $DELAY

# Issue 9
gh issue create --repo "$REPO" \
  --title "[docs] Create troubleshooting guide" \
  --label "good first issue,help wanted,small,analysis,documentation" \
  --body "$(cat <<'EOF'
## Objective
Document common issues and solutions.

## Files
- Create `docs/TROUBLESHOOTING.md`

## Acceptance Criteria
- [ ] WebRTC connection issues
- [ ] Signaling server problems
- [ ] Build/test failures

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Create troubleshooting guide"
sleep $DELAY

# Issue 10
gh issue create --repo "$REPO" \
  --title "[docs] Document demo keyboard shortcuts" \
  --label "good first issue,help wanted,micro,analysis,demo" \
  --body "$(cat <<'EOF'
## Objective
Document Snake game controls in demo.

## Files
- `apps/demo/README.md`

## Acceptance Criteria
- [ ] All keyboard shortcuts listed
- [ ] Game rules explained

## Complexity
**micro** (< 30 min)
EOF
)"
((count++))
echo "  Created issue $count: Document demo keyboard shortcuts"
sleep $DELAY

# ============================================
# BUILDING TASKS
# ============================================

echo "Creating building issues..."

# Issue 11
gh issue create --repo "$REPO" \
  --title "[feature] Add connection quality indicator to SDK" \
  --label "good first issue,help wanted,medium,building,sdk" \
  --body "$(cat <<'EOF'
## Objective
Expose connection quality to SDK users.

## Files
- `packages/sdk/src/tom-client.ts`

## Acceptance Criteria
- [ ] `onConnectionQualityChange` callback
- [ ] Quality levels: good, degraded, poor
- [ ] Test coverage

## Complexity
**medium** (1-2 hours)
EOF
)"
((count++))
echo "  Created issue $count: Add connection quality indicator to SDK"
sleep $DELAY

# Issue 12
gh issue create --repo "$REPO" \
  --title "[feature] Add message retry with exponential backoff" \
  --label "good first issue,help wanted,small,building,core/routing" \
  --body "$(cat <<'EOF'
## Objective
Implement retry logic for failed messages.

## Files
- `packages/core/src/routing/router.ts`

## Acceptance Criteria
- [ ] Max 3 retries
- [ ] Exponential backoff (1s, 2s, 4s)
- [ ] Tests for retry scenarios

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Add message retry with exponential backoff"
sleep $DELAY

# Issue 13
gh issue create --repo "$REPO" \
  --title "[feature] Add typing indicator support" \
  --label "good first issue,help wanted,medium,building,sdk" \
  --body "$(cat <<'EOF'
## Objective
Add typing indicator to chat.

## Files
- `packages/sdk/src/tom-client.ts`
- `apps/demo/src/main.ts`

## Acceptance Criteria
- [ ] `sendTypingIndicator(peerId)` method
- [ ] `onTypingIndicator` callback
- [ ] Demo UI shows typing state

## Complexity
**medium** (1-2 hours)
EOF
)"
((count++))
echo "  Created issue $count: Add typing indicator support"
sleep $DELAY

# Issue 14
gh issue create --repo "$REPO" \
  --title "[feature] Add message read receipts to demo UI" \
  --label "good first issue,help wanted,small,building,demo" \
  --body "$(cat <<'EOF'
## Objective
Show read receipts in chat UI.

## Files
- `apps/demo/src/main.ts`
- `apps/demo/index.html`

## Acceptance Criteria
- [ ] Double-check icon for read messages
- [ ] Single-check for delivered

## Complexity
**small** (30-60 min)

## Context
Read receipts are already supported in the SDK. This is about displaying them in the demo UI.
EOF
)"
((count++))
echo "  Created issue $count: Add message read receipts to demo UI"
sleep $DELAY

# Issue 15
gh issue create --repo "$REPO" \
  --title "[feature] Add network stats display to demo" \
  --label "good first issue,help wanted,small,building,demo" \
  --body "$(cat <<'EOF'
## Objective
Show network stats in demo UI.

## Files
- `apps/demo/src/main.ts`

## Acceptance Criteria
- [ ] Active connections count
- [ ] Messages sent/received
- [ ] Current relay

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Add network stats display to demo"
sleep $DELAY

# ============================================
# VERIFICATION TASKS
# ============================================

echo "Creating verification issues..."

# Issue 16
gh issue create --repo "$REPO" \
  --title "[verification] Audit TomError usage consistency" \
  --label "good first issue,help wanted,small,verification" \
  --body "$(cat <<'EOF'
## Objective
Ensure all errors use TomError.

## Files
- All `packages/core/src/**/*.ts`

## Acceptance Criteria
- [ ] No raw `throw new Error()`
- [ ] Consistent error codes
- [ ] Report findings in PR

## Complexity
**small** (30-60 min)

## Context
Search for `throw new Error` and verify all should be `throw new TomError`.
EOF
)"
((count++))
echo "  Created issue $count: Audit TomError usage consistency"
sleep $DELAY

# Issue 17
gh issue create --repo "$REPO" \
  --title "[verification] Verify ADR compliance in crypto module" \
  --label "good first issue,help wanted,small,verification,core/crypto" \
  --body "$(cat <<'EOF'
## Objective
Verify crypto follows ADR-004.

## Files
- `packages/core/src/crypto/`
- `_bmad-output/planning-artifacts/architecture.md` (ADR-004)

## Acceptance Criteria
- [ ] Uses TweetNaCl.js
- [ ] X25519 for key exchange
- [ ] XSalsa20-Poly1305 for encryption

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Verify ADR compliance in crypto module"
sleep $DELAY

# Issue 18
gh issue create --repo "$REPO" \
  --title "[verification] Review signaling server for security issues" \
  --label "good first issue,help wanted,small,verification,signaling-server" \
  --body "$(cat <<'EOF'
## Objective
Security audit of signaling server.

## Files
- `tools/signaling-server/src/`

## Acceptance Criteria
- [ ] No sensitive data logging
- [ ] Rate limiting present
- [ ] Input validation complete

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Review signaling server for security issues"
sleep $DELAY

# Issue 19
gh issue create --repo "$REPO" \
  --title "[verification] Verify all exports in index.ts files" \
  --label "good first issue,help wanted,micro,verification" \
  --body "$(cat <<'EOF'
## Objective
Ensure all public APIs are exported.

## Files
- `packages/core/src/index.ts`
- `packages/sdk/src/index.ts`

## Acceptance Criteria
- [ ] All public classes exported
- [ ] All public types exported
- [ ] No internal-only exports

## Complexity
**micro** (< 30 min)
EOF
)"
((count++))
echo "  Created issue $count: Verify all exports in index.ts files"
sleep $DELAY

# Issue 20
gh issue create --repo "$REPO" \
  --title "[verification] Check test coverage gaps" \
  --label "good first issue,help wanted,small,verification" \
  --body "$(cat <<'EOF'
## Objective
Identify untested code paths.

## Files
- Run coverage report

## Acceptance Criteria
- [ ] Generate coverage report
- [ ] List uncovered lines
- [ ] Create follow-up issues

## Complexity
**small** (30-60 min)

## How to run
```bash
pnpm test -- --coverage
```
EOF
)"
((count++))
echo "  Created issue $count: Check test coverage gaps"
sleep $DELAY

# ============================================
# CI/CD TASKS
# ============================================

echo "Creating CI/CD issues..."

# Issue 21
gh issue create --repo "$REPO" \
  --title "[ci] Add test coverage reporting to CI" \
  --label "good first issue,help wanted,small,building,ci-cd" \
  --body "$(cat <<'EOF'
## Objective
Add coverage report to CI pipeline.

## Files
- `.github/workflows/ci.yml`
- `vitest.config.ts`

## Acceptance Criteria
- [ ] Coverage report generated
- [ ] Report uploaded as artifact
- [ ] Threshold enforcement (optional)

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Add test coverage reporting to CI"
sleep $DELAY

# Issue 22
gh issue create --repo "$REPO" \
  --title "[ci] Add build size tracking" \
  --label "good first issue,help wanted,small,building,ci-cd" \
  --body "$(cat <<'EOF'
## Objective
Track bundle size in CI.

## Files
- `.github/workflows/ci.yml`

## Acceptance Criteria
- [ ] Report bundle sizes
- [ ] Compare with previous build
- [ ] Warn on significant increase

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Add build size tracking"
sleep $DELAY

# Issue 23
gh issue create --repo "$REPO" \
  --title "[ci] Add dependency audit to CI" \
  --label "good first issue,help wanted,micro,building,ci-cd" \
  --body "$(cat <<'EOF'
## Objective
Add `pnpm audit` to CI.

## Files
- `.github/workflows/ci.yml`

## Acceptance Criteria
- [ ] `pnpm audit` runs in CI
- [ ] Failures are warnings (not blocking)

## Complexity
**micro** (< 30 min)
EOF
)"
((count++))
echo "  Created issue $count: Add dependency audit to CI"
sleep $DELAY

# ============================================
# REFACTORING TASKS
# ============================================

echo "Creating refactoring issues..."

# Issue 24
gh issue create --repo "$REPO" \
  --title "[refactor] Extract message validation to separate module" \
  --label "good first issue,help wanted,medium,building,core/routing" \
  --body "$(cat <<'EOF'
## Objective
Move validation logic out of Router.

## Files
- `packages/core/src/routing/router.ts`

## Acceptance Criteria
- [ ] Create `message-validator.ts`
- [ ] Move validation functions
- [ ] Update imports
- [ ] All tests pass

## Complexity
**medium** (1-2 hours)
EOF
)"
((count++))
echo "  Created issue $count: Extract message validation to separate module"
sleep $DELAY

# Issue 25
gh issue create --repo "$REPO" \
  --title "[refactor] Simplify EphemeralSubnetManager API" \
  --label "good first issue,help wanted,small,building,core/discovery" \
  --body "$(cat <<'EOF'
## Objective
Reduce API surface complexity.

## Files
- `packages/core/src/discovery/ephemeral-subnet.ts`

## Acceptance Criteria
- [ ] Consolidate similar methods
- [ ] Update callers
- [ ] Tests pass

## Complexity
**small** (30-60 min)
EOF
)"
((count++))
echo "  Created issue $count: Simplify EphemeralSubnetManager API"
sleep $DELAY

# ============================================
# SUMMARY
# ============================================

echo ""
echo "=========================================="
echo "Published $count issues to GitHub!"
echo "=========================================="
echo ""
echo "View them at: https://github.com/$REPO/issues"
echo ""
echo "Stats:"
echo "  - Testing: 5 issues"
echo "  - Documentation: 5 issues"
echo "  - Building/Features: 10 issues"
echo "  - Verification: 5 issues"
echo ""
echo "By complexity:"
echo "  - micro (< 30 min): 5"
echo "  - small (30-60 min): 14"
echo "  - medium (1-2 hours): 6"
