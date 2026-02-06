# Security Findings - ToM Protocol

**Date:** February 6, 2026  
**Scope:** Full codebase security audit  
**Status:** 🔴 CRITICAL ISSUES FOUND

---

## Critical Findings

### 🔴 CRITICAL-001: Cryptographically Weak Random Number Generation

**Severity:** CRITICAL  
**CWE:** CWE-338 (Use of Cryptographically Weak Pseudo-Random Number Generator)  
**CVSS Score:** 7.5 (High)  

#### Description
The codebase uses JavaScript's `Math.random()` for generating security-sensitive identifiers across multiple modules. `Math.random()` is **not cryptographically secure** and produces predictable values.

#### Affected Locations

1. **Ephemeral Subnet IDs**  
   **File:** `packages/core/src/discovery/ephemeral-subnet.ts:269`
   ```typescript
   const subnetId = `subnet-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
   ```

2. **Gossip Protocol IDs**  
   **File:** `packages/core/src/discovery/peer-gossip.ts:380`
   ```typescript
   return `gossip-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
   ```

3. **Group Security Tokens**  
   **File:** `packages/core/src/groups/group-security.ts:28`
   ```typescript
   const hex = () => Math.floor(Math.random() * 16).toString(16);
   ```

4. **Group Manager IDs**  
   **File:** `packages/core/src/groups/group-manager.ts:582`
   ```typescript
   const hex = () => Math.floor(Math.random() * 16).toString(16);
   ```

#### Security Impact

**Subnet Hijacking:**
- Predictable subnet IDs allow attackers to:
  - Pre-generate subnet IDs and intercept subnet formation
  - Join subnets without proper discovery
  - Disrupt ephemeral subnet routing

**Gossip Protocol Attacks:**
- Predictable gossip IDs enable:
  - Message replay attacks
  - Gossip message forgery
  - Network topology poisoning

**Group Impersonation:**
- Weak group security tokens allow:
  - Unauthorized group access
  - Group membership forgery
  - Message injection into private groups

**Collision Risk:**
- `Math.random()` has only ~2^53 bits of entropy
- Birthday paradox: 50% collision probability at ~4 billion IDs
- Combined with timestamp, still insufficient for security

#### Exploitation Scenario

```typescript
// Attacker can predict future subnet IDs:
const timestamp = Date.now() + 1000; // 1 second in future
const predictedId = `subnet-${timestamp}-${guessedRandom()}`;

// Then intercept subnet formation by registering early
await interceptSubnet(predictedId);
```

#### Recommended Fix

**Solution 1: Use Node.js crypto module (Recommended)**
```typescript
import { randomBytes } from 'crypto';

function generateSecureId(prefix: string): string {
  const randomHex = randomBytes(16).toString('hex'); // 128 bits of entropy
  return `${prefix}-${Date.now()}-${randomHex}`;
}

// Usage:
const subnetId = generateSecureId('subnet');
const gossipId = generateSecureId('gossip');
```

**Solution 2: Use Web Crypto API (Browser-compatible)**
```typescript
function generateSecureId(prefix: string): string {
  const array = new Uint8Array(16);
  crypto.getRandomValues(array);
  const randomHex = Array.from(array, b => b.toString(16).padStart(2, '0')).join('');
  return `${prefix}-${Date.now()}-${randomHex}`;
}
```

**Solution 3: Use existing TweetNaCl (already a dependency)**
```typescript
import nacl from 'tweetnacl';

function generateSecureId(prefix: string): string {
  const random = nacl.randomBytes(16);
  const randomHex = Buffer.from(random).toString('hex');
  return `${prefix}-${Date.now()}-${randomHex}`;
}
```

#### Remediation Priority: IMMEDIATE

**Effort:** 2-4 hours  
**Risk if not fixed:** HIGH - Active protocol security compromised  

---

## Medium Severity Findings

### ⚠️ MEDIUM-001: Missing Input Validation on Message Envelopes

**Severity:** MEDIUM  
**CWE:** CWE-20 (Improper Input Validation)  
**CVSS Score:** 5.3 (Medium)

#### Description
The `Router` and `GroupHub` components process incoming `MessageEnvelope` objects without schema validation, trusting that all required fields exist and have valid types.

#### Affected Locations

1. **Router Message Handler**  
   **File:** `packages/core/src/routing/router.ts`
   ```typescript
   async handleIncomingMessage(envelope: MessageEnvelope) {
     // No validation that envelope.type, envelope.from exist
     // No type checking on payload structure
   }
   ```

2. **Group Hub Message Processing**  
   **File:** `packages/core/src/groups/group-hub.ts`
   ```typescript
   handleGroupMessage(envelope: MessageEnvelope) {
     // Assumes envelope structure is valid
     const { groupId, members } = envelope.payload;
   }
   ```

#### Security Impact

**Type Confusion:**
- Malformed envelopes can cause unexpected behavior
- Missing fields lead to `undefined` access → crashes
- Type mismatches bypass intended logic

**Resource Exhaustion:**
- Oversized payloads can exhaust memory
- Deeply nested objects can cause stack overflow
- Large arrays can freeze event loop

**Potential Exploits:**
```typescript
// Malicious envelope:
{
  type: "chat",
  from: null, // Should be string
  to: ["node1", "node2", ...Array(10000)], // Oversized
  payload: { /* 10MB nested object */ }
}
```

#### Recommended Fix

**Add Zod schema validation:**

```typescript
import { z } from 'zod';

const MessageEnvelopeSchema = z.object({
  id: z.string().min(1).max(128),
  from: z.string().min(1).max(64),
  to: z.string().min(1).max(64),
  via: z.array(z.string().max(64)).max(10).optional(),
  type: z.enum(['chat', 'ack', 'group', 'heartbeat', /* ... */]),
  payload: z.any(), // Further validation per type
  timestamp: z.number().int().positive(),
  signature: z.string().optional(),
});

// In Router:
handleIncomingMessage(rawEnvelope: unknown) {
  const parseResult = MessageEnvelopeSchema.safeParse(rawEnvelope);
  if (!parseResult.success) {
    throw new TomError(TomErrorCode.INVALID_MESSAGE, 'Invalid envelope', {
      errors: parseResult.error.errors
    });
  }
  const envelope = parseResult.data;
  // ... continue processing
}
```

#### Remediation Priority: HIGH

**Effort:** 6-8 hours  
**Risk if not fixed:** MEDIUM - Potential for crashes and DoS  

---

### ⚠️ MEDIUM-002: Insufficient Rate Limiting

**Severity:** MEDIUM  
**CWE:** CWE-770 (Allocation of Resources Without Limits or Throttling)  
**CVSS Score:** 5.0 (Medium)

#### Description
While `GroupHub` implements basic rate limiting (2 messages/second), there is no comprehensive rate limiting at the transport or connection level.

#### Affected Areas

1. **Transport Layer:** No connection rate limiting
2. **Router:** No message throughput caps
3. **SignalingServer:** No WebSocket connection limits

#### Security Impact

**Resource Exhaustion:**
- Malicious nodes can flood network with messages
- Memory exhaustion via message queue buildup
- CPU exhaustion processing invalid messages

**Network DoS:**
- Connection flooding attacks
- Bandwidth exhaustion
- Relay node overload

#### Recommended Fix

**Implement sliding window rate limiter:**

```typescript
class RateLimiter {
  private windows = new Map<string, number[]>();
  
  constructor(
    private maxRequests: number,
    private windowMs: number
  ) {}
  
  isAllowed(key: string): boolean {
    const now = Date.now();
    const window = this.windows.get(key) || [];
    
    // Remove old timestamps
    const validTimestamps = window.filter(ts => now - ts < this.windowMs);
    
    if (validTimestamps.length >= this.maxRequests) {
      return false;
    }
    
    validTimestamps.push(now);
    this.windows.set(key, validTimestamps);
    return true;
  }
}

// In TransportLayer:
private rateLimiter = new RateLimiter(100, 1000); // 100 msgs/sec

handleIncomingMessage(envelope: MessageEnvelope) {
  if (!this.rateLimiter.isAllowed(envelope.from)) {
    throw new TomError(TomErrorCode.RATE_LIMIT_EXCEEDED);
  }
  // ... process message
}
```

#### Remediation Priority: MEDIUM

**Effort:** 8-10 hours  
**Risk if not fixed:** MEDIUM - Potential DoS vulnerability  

---

### ⚠️ MEDIUM-003: Disabled Non-Null Assertions

**Severity:** LOW-MEDIUM  
**CWE:** CWE-476 (NULL Pointer Dereference)  
**CVSS Score:** 3.7 (Low)

#### Description
The Biome linter configuration disables the `noNonNullAssertion` rule, allowing use of the `!` operator throughout the codebase. This hides potential null safety issues.

#### Configuration
**File:** `biome.json`
```json
{
  "linter": {
    "rules": {
      "suspicious": {
        "noNonNullAssertion": "off"
      }
    }
  }
}
```

#### Security Impact

**Runtime Crashes:**
- Unchecked null assumptions can cause crashes
- Loss of message delivery guarantees
- Node instability

#### Recommended Fix

1. Enable the rule:
   ```json
   "noNonNullAssertion": "warn"
   ```

2. Replace assertions with safe access:
   ```typescript
   // Before:
   const peer = this.peers.get(nodeId)!;
   
   // After:
   const peer = this.peers.get(nodeId);
   if (!peer) {
     throw new TomError(TomErrorCode.PEER_NOT_FOUND);
   }
   ```

#### Remediation Priority: MEDIUM

**Effort:** 16-20 hours (requires codebase-wide changes)  
**Risk if not fixed:** LOW - Potential for runtime errors  

---

## Informational Findings

### ℹ️ INFO-001: Temporary Signaling Server Without TLS Enforcement

**Severity:** INFORMATIONAL  
**Note:** This is acknowledged as temporary (ADR-002) and will be replaced with DHT-based bootstrap. No immediate action required for temporary development use.

**File:** `tools/signaling-server/`

**Recommendation:** Ensure TLS is required for any production deployment before DHT replacement.

---

### ℹ️ INFO-002: No Circuit Breaker Pattern

**Severity:** INFORMATIONAL  

The codebase lacks circuit breaker patterns for handling failing peers or relays. Consider implementing:
- Automatic peer blacklisting after N failures
- Exponential backoff for reconnection
- Health checks before routing

---

## Dependency Security

### ✅ All Dependencies Secure

**Audit Date:** February 6, 2026  

| Dependency | Version | Known Vulnerabilities | Status |
|------------|---------|----------------------|--------|
| tweetnacl | 1.0.3 | None | ✅ SAFE |
| ws | 8.19.0 | None | ✅ SAFE |
| simple-peer | latest | None | ✅ SAFE |
| vite | 6.4.1 | None | ✅ SAFE |
| @biomejs/biome | 1.9.4 | None | ✅ SAFE |
| typescript | 5.9.3 | None | ✅ SAFE |
| vitest | 3.2.4 | None | ✅ SAFE |

**No vulnerable dependencies found.**

---

## Summary

| Severity | Count | Fixed | Remaining |
|----------|-------|-------|-----------|
| 🔴 CRITICAL | 1 | 0 | 1 |
| ⚠️ MEDIUM | 3 | 0 | 3 |
| ℹ️ INFO | 2 | N/A | N/A |
| **TOTAL** | **6** | **0** | **4** |

### Recommended Immediate Actions

1. **[CRITICAL-001]** Replace `Math.random()` with cryptographically secure RNG (4 files)
2. **[MEDIUM-001]** Add message envelope validation
3. **[MEDIUM-002]** Implement comprehensive rate limiting
4. **[MEDIUM-003]** Enable non-null assertion checks

### Timeline

- **Week 1:** Fix CRITICAL-001 (immediate risk)
- **Week 2:** Fix MEDIUM-001 and MEDIUM-002
- **Month 2:** Address MEDIUM-003 and INFO findings

---

**Report Prepared By:** Automated Security Analysis  
**Contact:** Security Team  
**Next Review:** After fixes implemented, then quarterly
