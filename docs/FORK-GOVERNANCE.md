# Fork Governance - "iroh" References Policy

**Date**: 2026-02-27
**Applies to**: tom-connect, tom-relay
**Purpose**: Prevent regression in fork maintenance

---

## Context

tom-connect and tom-relay are strategic forks of iroh v0.96.0. We maintain **wire protocol compatibility** with the iroh network while achieving **code independence**.

This document defines which "iroh" references are **allowed** vs **prohibited** to prevent accidental breakage in future PRs.

---

## Decision Matrix

| Category | Action | Reason | Examples |
|----------|--------|--------|----------|
| **Wire Protocol** | ✅ KEEP | Compatibility with iroh relay network | `_iroh` DNS TXT, `.iroh.invalid` TLS, DERP frames |
| **HTTP Headers (Relay)** | ✅ KEEP | Relay server protocol | `X-Iroh-Challenge`, `X-Iroh-NodeId`, `X-Iroh-Response` |
| **ALPN (Wire)** | ✅ KEEP | QUIC address discovery protocol | `b"/iroh-qad/0"` |
| **External Infra** | ✅ KEEP | Using n0's relay infrastructure | `iroh.link`, `iroh-canary`, `iroh.network` |
| **Attribution** | ✅ KEEP | Compliance & historical context | `github.com/n0-computer/iroh` links, fork notices |
| **Environment Variables** | ⚠️ REVIEW | May need migration path | `IROH_FORCE_STAGING_RELAYS` → `TOM_FORCE_STAGING_RELAYS`? |
| **Test ALPN** | ⚠️ DECIDE | Internal only, no compat risk | `b"n0/iroh/test"` → decision pending |
| **Metrics (Prometheus)** | ⚠️ DECIDE | Dashboard compatibility vs clarity | `iroh_*` → `tom_*`? |
| **Doc Links (External)** | 🔴 REMOVE | Misleading in ToM context | `docs.rs/iroh-tickets`, `iroh.computer/docs` |
| **Module Namespaces** | 🔴 FORBIDDEN | Breaking import paths | `iroh::`, `iroh_relay::` (except allowed deps) |
| **Inline Comments** | 💡 UPDATE | Clarity for future maintainers | "forked from iroh" → OK, "see iroh docs" → UPDATE |

---

## Allowed Dependencies (Never Remove)

These are **permanent bridges** to the iroh ecosystem:

```toml
# tom-connect/Cargo.toml
iroh-base = "0.96.0"           # EndpointId, RelayUrl, SecretKey
tom-relay = { path = "../tom-relay" }  # RelayMap, RelayConfig (our fork)
iroh-metrics = "0.38"          # Prometheus metrics framework
quinn = { package = "iroh-quinn" }  # Quinn fork (WeakConnectionHandle, PathId)
quinn-proto = { package = "iroh-quinn-proto" }
quinn-udp = { package = "iroh-quinn-udp" }

# tom-relay/Cargo.toml
iroh-base = "0.96.0"
iroh-metrics = "0.38"
quinn = { package = "iroh-quinn" }
```

**Never import**: `iroh` main crate (we forked its internals).

---

## Wire Protocol Invariants (DO NOT BREAK)

These must remain **byte-for-byte identical** with iroh v0.96:

### 1. DERP Protocol (Relay Wire Format)
**File**: `crates/tom-relay/src/protos/relay.rs`

```rust
// KEEP THESE UNCHANGED
pub const MAGIC: &[u8; 8] = b"DERP\x00\x00\x00\x01";
pub const VERSION: u32 = 1;

pub enum FrameType {
    ServerKey = 0x01,
    ClientInfo = 0x02,
    SendPacket = 0x04,
    // ... rest unchanged
}
```

**Test**: Wire format compatibility with iroh relays.

### 2. DNS TXT Records
**File**: `crates/tom-relay/src/endpoint_info.rs`

```rust
// KEEP THIS LABEL
const IROH_TXT_NAME: &str = "_iroh";  // ← DO NOT CHANGE

// Example: _iroh.{endpoint_id}.dns.iroh.link.
```

**Why**: Pkarr DNS discovery standard shared with iroh network.

### 3. TLS Certificate Names
**File**: `crates/tom-connect/src/tls/name.rs`

```rust
// KEEP THIS SUFFIX
const TLS_SUFFIX: &str = "iroh.invalid";  // ← DO NOT CHANGE

// Example: 7dl2ff6emqi2qol3l382krodedij45bn3nh479hqo14a32qpr8kg.iroh.invalid
```

**Why**: Certificate validation in QUIC handshake.

### 4. HTTP Headers (Relay Server)
**Files**:
- `crates/tom-relay/src/server.rs`
- `crates/tom-relay/src/main.rs`
- `crates/tom-connect/src/net_report/reportgen.rs`

```rust
// KEEP THESE HEADERS
const NO_CONTENT_CHALLENGE_HEADER: &str = "X-Iroh-Challenge";
const NO_CONTENT_RESPONSE_HEADER: &str = "X-Iroh-Response";
const X_IROH_ENDPOINT_ID: &str = "X-Iroh-NodeId";
```

**Why**: Relay clients (including iroh nodes) expect these headers.

### 5. QUIC ALPN (Address Discovery)
**File**: `crates/tom-relay/src/quic.rs`

```rust
// KEEP THIS ALPN
pub const ALPN_QUIC_ADDR_DISC: &[u8] = b"/iroh-qad/0";  // ← DO NOT CHANGE
```

**Why**: QUIC address discovery protocol shared with iroh network.

---

## Pending Decisions (⚠️ Needs Team Input)

### Decision 1: Test ALPN Strings
**Current**:
```rust
// crates/tom-connect/src/endpoint.rs:1548
const TEST_ALPN: &[u8] = b"n0/iroh/test";
```

**Options**:
- A) Keep `b"n0/iroh/test"` (historical continuity with forked tests)
- B) Change to `b"n0/tom/test"` (ToM identity, no compat risk)

**Impact**: Internal test-only, zero production risk.

**Recommendation**: **B** - Update to `b"n0/tom/test"` for clarity.

### Decision 2: Prometheus Metrics Naming
**Current**: No explicit `iroh_*` prefix hardcoded (uses generic `endpoint`, `socket`, etc.).

**Question**: Should we enforce `tom_*` prefix convention?

**Options**:
- A) Leave as-is (generic names)
- B) Add explicit `tom_` prefix for dashboards
- C) Wait until Task 3 migration (when tom-protocol integrates)

**Recommendation**: **C** - Decide during dashboard setup.

### Decision 3: Environment Variables
**Found**: `IROH_FORCE_STAGING_RELAYS` referenced in code.

**Question**: Migrate to `TOM_FORCE_STAGING_RELAYS`?

**Impact**: User-facing CLI, breaks existing scripts.

**Recommendation**: Support **both** during transition, deprecate `IROH_*` in v0.2.

---

## PR Checklist (Prevent Regressions)

Before merging any PR touching tom-connect or tom-relay, verify:

- [ ] No new `iroh::` or `iroh_relay::` module imports (outside allowed deps)
- [ ] No changes to wire protocol files:
  - [ ] `tom-relay/src/protos/relay.rs` (DERP frames)
  - [ ] `tom-relay/src/endpoint_info.rs` (`_iroh` DNS label)
  - [ ] `tom-connect/src/tls/name.rs` (`.iroh.invalid` suffix)
  - [ ] `tom-relay/src/server.rs` (`X-Iroh-*` headers)
  - [ ] `tom-relay/src/quic.rs` (`/iroh-qad/0` ALPN)
- [ ] No new doc links to `docs.rs/iroh-*`
- [ ] Comments mentioning "iroh" are either:
  - Attribution (OK)
  - Wire protocol explanation (OK)
  - Generic clarity (UPDATE if confusing)

---

## Review Command (Run Before Merge)

```bash
# Detect forbidden namespace leaks
rg "iroh::" crates/tom-connect/src crates/tom-relay/src \
  | grep -v "iroh-base" \
  | grep -v "tom-relay" \
  | grep -v "iroh-metrics" \
  | grep -v "iroh-quinn"

# Detect new external doc links (should be empty)
rg "docs\.rs/iroh-" crates/tom-connect/src crates/tom-relay/src

# Verify wire protocol files unchanged (compare git diff)
git diff main -- \
  crates/tom-relay/src/protos/relay.rs \
  crates/tom-relay/src/endpoint_info.rs \
  crates/tom-connect/src/tls/name.rs \
  crates/tom-relay/src/server.rs \
  crates/tom-relay/src/quic.rs
```

If any of these commands return results, **review carefully** before merge.

---

## Rationale

**Why keep some "iroh" references?**

1. **Network Compatibility**: tom-connect/tom-relay nodes can communicate with iroh v0.96 nodes via shared relay infrastructure.
2. **Protocol Standards**: DNS, TLS, HTTP standards don't change with a fork.
3. **Ecosystem Bridge**: We use iroh's proven crypto/networking libraries (iroh-base, iroh-quinn) rather than reinventing.

**Why remove others?**

1. **API Clarity**: Developers using ToM shouldn't see confusing iroh-specific docs.
2. **Independence**: We own the codebase and can diverge from iroh's roadmap.
3. **Branding**: ToM Protocol has its own identity, separate from n0.computer.

---

## History

- **2026-02-27**: Initial fork (R7.3) - 33,539 lines from iroh v0.96.0
- **2026-02-27**: Review prompt created, findings documented
- **2026-02-27**: This governance doc created to prevent regressions

---

**Maintained by**: ToM Protocol Core Team
**Questions?**: Open issue with label `fork-governance`
