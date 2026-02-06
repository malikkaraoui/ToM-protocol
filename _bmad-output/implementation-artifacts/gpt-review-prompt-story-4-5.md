# GPT 5.2 Codex Review Request - Story 4.5

## Context

ToM Protocol is a decentralized P2P messaging protocol. Story 4.5 implements a multiplayer Snake game to demonstrate the bidirectional real-time transport capabilities.

## Files to Review

### Core Game Logic
1. `apps/demo/src/game/game-types.ts` - Message protocol types and validation guards
2. `apps/demo/src/game/snake-game.ts` - Game engine (tick-based, collision detection, scoring)
3. `apps/demo/src/game/snake-renderer.ts` - Canvas rendering
4. `apps/demo/src/game/game-controller.ts` - Session orchestration (invitations, state sync, resilience)

### Integration
5. `apps/demo/src/main.ts` - Demo app integration (lines 20-21, 78-84, 102-127, 148-150, 656-828)
6. `apps/demo/index.html` - Game UI elements and styles (lines 58-72)

### Tests
7. `apps/demo/src/game/snake-game.test.ts` - 24 game engine tests
8. `apps/demo/src/game/game-types.test.ts` - 21 type guard tests
9. `apps/demo/src/game/game-controller.test.ts` - 20 controller tests

## Review Focus Areas

### 1. Security Hardening
- **Rate limiting**: INPUT_RATE_LIMIT_MS = 50ms for P2 inputs (game-controller.ts:82-83)
- **Payload validation**: Type guards with bounds checking (game-types.ts:145-206)
- **State validation**: Tick monotonicity, coordinate bounds (snake-game.ts:374-427)
- **Sender verification**: All handlers verify fromPeerId === session.peerId

**Question**: Are there additional attack vectors (DoS, state manipulation, replay attacks)?

### 2. Network Architecture
- P1 (host) is authoritative - runs game loop, sends state updates
- P2 (client) sends input only, receives state updates
- Direct path preferred, relay fallback automatic
- Connection quality indicator (direct/relay/disconnected)

**Question**: Is the P1-authoritative model secure against cheating P1?

### 3. Game Logic
- Toroidal grid (wraparound, no wall collision)
- Collision rules: longer snake wins on head-to-head or body collision
- Self-collision = immediate loss
- Food consumption = growth + score

**Question**: Any edge cases in collision detection logic?

### 4. State Synchronization
- GameStatePayload sent every tick (100ms default)
- P2 validates: tick monotonicity, coordinate bounds, score validity
- Rejected states logged but don't crash

**Question**: Is 100ms tick rate optimal for network conditions?

### 5. Test Coverage
- 65 game-related tests (24 + 21 + 20)
- Total project: 559 tests passing
- Missing: Integration tests with real TomClient

**Question**: What additional test scenarios would improve coverage?

## Specific Code Sections for Deep Review

### Collision Detection (snake-game.ts:189-235)
```typescript
// Check collisions before moving
const p1SelfCollision = this.checkSelfCollision(newHead1, 'p1');
const p2SelfCollision = this.checkSelfCollision(newHead2, 'p2');
const p1HitsP2 = this.checkOpponentCollision(newHead1, 'p2');
const p2HitsP1 = this.checkOpponentCollision(newHead2, 'p1');
const headToHead = this.pointsEqual(newHead1, newHead2);
```

### State Validation (snake-game.ts:374-427)
```typescript
// Fix #9: Verify tick monotony (must be > current, prevent replay attacks)
if (payload.tick <= this.state.tick) {
  console.warn('[SnakeGame] Rejected state: tick not monotonically increasing');
  return;
}
```

### Rate Limiting (game-controller.ts:410-415)
```typescript
// Fix #8: Rate limit incoming input (prevent DoS)
const now = Date.now();
if (now - this.lastRemoteInputTime < INPUT_RATE_LIMIT_MS) {
  return; // Rate limited, ignore
}
this.lastRemoteInputTime = now;
```

## Expected Output

Please provide:
1. **Security findings** (HIGH/MEDIUM/LOW severity)
2. **Code quality issues** (maintainability, performance)
3. **Suggested improvements** with code examples
4. **Test gap analysis**

## Project Stats
- Tests: 559 passing
- Lint: 0 warnings (Biome)
- Build: OK (all packages)
- Architecture: ADR-001 compliant (game uses existing transport, no protocol changes)
