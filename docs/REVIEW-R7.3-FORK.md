# Code Review Prompt - Phase R7.3 Fork (iroh → ToM)

**Date**: 2026-02-27
**Phase**: R7.3 - The Fork (Tasks 1 & 2 Complete)
**Reviewer**: Copilot / Second AI / Human reviewer

---

## Context

We just completed a strategic fork of iroh v0.96.0 at the socket/endpoint boundary:
- **Task 1**: Forked transport layer (21,756 lines) → `crates/tom-connect/`
- **Task 2**: Forked relay server (11,783 lines) → `crates/tom-relay/`
- **Total**: 33,539 lines of battle-tested networking code

**Goal**: Independent transport layer while keeping protocol layer (`tom-protocol`) intact.

---

## What We Did

### 1. tom-connect (Transport Layer Fork)

**Copied from** `iroh v0.96.0/iroh/src/`:
- `socket.rs` (2293 lines) - UDP socket with relay/direct multiplexing
- `socket/` (11 files) - Transport selection, relay actor, path state
- `endpoint.rs` (3061 lines) - Public API wrapper over Quinn
- `endpoint/` - Connection handling, QUIC integration
- `address_lookup/` - DNS + Pkarr discovery
- `net_report/` - Network diagnostics (STUN, NAT detection)
- `tls/`, `dns.rs`, `defaults.rs`, `metrics.rs`, `util.rs`

**Key decisions**:
- Edition 2024 (for let chains syntax)
- Uses `iroh-quinn` fork (NOT standard quinn) - for WeakConnectionHandle, PathId, etc.
- Kept dependencies: `iroh-base`, `iroh-relay`, `iroh-metrics`
- Compiles: ✅ 0 errors, 263 warnings (all `wasm_browser` cfg - benign)

**File**: `crates/tom-connect/src/lib.rs`
```rust
// Re-exports for ToM compatibility
pub type NodeId = PublicKey;     // iroh-base::PublicKey
pub type NodeAddr = EndpointAddr; // iroh-base::EndpointAddr
```

### 2. tom-relay (Relay Server Fork)

**Copied from** `iroh-relay v0.96.0/`:
- `main.rs` - CLI binary (TLS, config, metrics)
- `server/` - HTTP/HTTPS relay implementation
- `client/` - Relay client (WebSocket upgrade)
- `protos/` - DERP protocol (Tailscale-derived)
- `dns.rs`, `endpoint_info.rs`, `quic.rs`, etc.

**Changes made**:
- Namespace: `iroh_relay::` → `tom_relay::` (16 occurrences via sed)
- Comments: `iroh-relay` → `tom-relay`
- Dependencies: Added `server` feature to `tokio-websockets`
- Removed placeholder `src/bin/server.rs`, using real `src/main.rs`
- Compiles: ✅ Binary runs (`tom-relay --help` works)

---

## Review Checklist

### 🔍 Critical Areas to Review

#### 1. Namespace Consistency
**Question**: Did we miss any `iroh::` or `iroh_relay::` references?

```bash
# Check for leftover iroh references
cd crates/tom-connect && rg "iroh::" src/ | grep -v "iroh-base" | grep -v "iroh-relay" | grep -v "iroh-metrics"
cd crates/tom-relay && rg "iroh_relay::" src/
```

**Potential blind spots**:
- Imports in test modules (`#[cfg(test)]`)
- Macro expansions
- Doc comments with code examples
- Error messages with hardcoded strings

---

#### 2. Type Aliases & API Compatibility
**Question**: Are our type aliases (`NodeId`, `NodeAddr`) used consistently?

**Check these files**:
- `crates/tom-connect/src/lib.rs` - Type aliases defined
- `crates/tom-connect/src/endpoint.rs` - Should use `PublicKey`, not `NodeId`
- `crates/tom-connect/src/socket.rs` - Internal usage patterns

**Potential issues**:
- Mixing `PublicKey` and `NodeId` in same module
- Missing `From`/`Into` implementations
- Type inference failures

---

#### 3. Dependencies - Version Conflicts
**Question**: Are all dependency versions compatible across crates?

**Check**:
```toml
# tom-connect/Cargo.toml
quinn = { package = "iroh-quinn", version = "0.16" }
iroh-base = "0.96.0"

# tom-relay/Cargo.toml
quinn = { package = "iroh-quinn", version = "0.16" }
iroh-base = "0.96.0"

# tom-protocol/Cargo.toml (NOT migrated yet)
iroh = "0.96"  # ← Will this conflict with tom-connect?
```

**Potential blind spots**:
- Duplicate `quinn` crates (iroh-quinn vs standard quinn)
- `ed25519-dalek` version mismatch (v2 in tom-protocol, v3-pre in tom-connect)
- `curve25519-dalek` version conflicts

---

#### 4. Feature Flags & Conditional Compilation
**Question**: Did we preserve all necessary feature flags?

**Check**:
- `#[cfg(feature = "metrics")]` usage
- `#[cfg(feature = "server")]` in tom-relay
- `#[cfg(not(wasm_browser))]` - are these still valid?
- `#[cfg(iroh_docsrs)]` - should be changed to `tom_docsrs` or removed?

**File**: `crates/tom-relay/src/lib.rs:29`
```rust
#![cfg_attr(iroh_docsrs, feature(doc_cfg))]  // ← Should this be tom_docsrs?
```

---

#### 5. Build Scripts & Code Generation
**Question**: Does `build.rs` generate any code that references `iroh`?

**Check**:
- `crates/tom-relay/build.rs` - What does `vergen-gitcl` generate?
- Are there any `include!()` macros loading generated code?
- Proto compilation (if any)?

---

#### 6. Tests - Do They Still Exist?
**Question**: Did we copy tests? Do they compile?

```bash
# Check for test modules
rg "#\[cfg\(test\)\]" crates/tom-connect/src/
rg "#\[test\]" crates/tom-connect/src/

# Try running them
cargo test -p tom-connect
cargo test -p tom-relay
```

**Potential issues**:
- Tests reference `iroh::test_utils`
- Test data hardcoded with "iroh" strings
- Integration tests in `tests/` directory not copied

---

#### 7. Documentation Links
**Question**: Are all `[doc links]` in comments still valid?

**Pattern to search**:
```rust
/// See [`iroh::Endpoint`] for details  // ← BROKEN
/// See [`Endpoint`] for details        // ← OK (relative)
/// See [`crate::Endpoint`] for details // ← OK (absolute)
```

**Check**:
```bash
rg "\[`iroh::" crates/tom-connect/src/
rg "\[`iroh_relay::" crates/tom-relay/src/
```

---

#### 8. Error Messages & User-Facing Strings
**Question**: Do error messages still say "iroh"?

**Examples to find**:
```rust
bail!("iroh connection failed");  // ← Should be "tom-connect" or generic
tracing::error!("iroh-relay down"); // ← Should be "tom-relay"
```

**Check**:
```bash
rg '"iroh' crates/tom-connect/src/
rg '"iroh' crates/tom-relay/src/
```

---

#### 9. Wire Protocol Compatibility
**Question**: Is tom-relay still compatible with iroh relays?

**Critical invariants**:
- DERP protocol version unchanged
- Message frame format identical
- Handshake sequence preserved
- WebSocket upgrade path compatible

**File to verify**: `crates/tom-relay/src/protos/relay.rs`
- Check `MAGIC`, `VERSION` constants
- Verify `FrameType` enum unchanged
- Confirm serialization format (postcard)

---

#### 10. Security - Crypto Primitives
**Question**: Are crypto operations unchanged?

**Check**:
- TLS configuration in `crates/tom-connect/src/tls/`
- Key derivation in `endpoint.rs`
- Certificate validation in `tls/verifier.rs`
- Signature verification (if any relay-level sigs exist)

**Potential blind spot**:
- Did we accidentally change RNG initialization?
- Are all `SecretKey` instances properly zeroized on drop?

---

## Specific Files to Deep-Review

### Priority 1 (API Surface)
1. `crates/tom-connect/src/lib.rs` - Public exports
2. `crates/tom-connect/src/endpoint.rs` - Main API
3. `crates/tom-relay/src/lib.rs` - Relay public API
4. `crates/tom-relay/src/main.rs` - CLI interface

### Priority 2 (Core Logic)
5. `crates/tom-connect/src/socket.rs` - Path multiplexing
6. `crates/tom-connect/src/socket/transports/relay/actor.rs` - Relay client
7. `crates/tom-relay/src/server.rs` - Relay server core
8. `crates/tom-relay/src/protos/relay.rs` - Wire protocol

### Priority 3 (Supporting)
9. `crates/tom-connect/src/address_lookup/dns.rs` - DNS discovery
10. `crates/tom-connect/src/net_report.rs` - STUN/diagnostics

---

## Known Issues to Validate

### 1. Edition 2024 Requirement
**Issue**: Code uses let chains (`if let X && let Y`)
**Impact**: Requires Rust 1.89+ (edition 2024)
**Question**: Is this acceptable? Should we backport to 2021 syntax?

### 2. iroh-quinn Dependency
**Issue**: We depend on iroh's fork of Quinn, not standard Quinn
**Impact**: Tightly coupled to n0-computer ecosystem
**Question**: Can we migrate to standard quinn later? What features are fork-specific?

### 3. Warnings Explosion
**Issue**: 263 warnings in tom-connect (all `wasm_browser` cfg)
**Impact**: Noise in build output
**Question**: Should we suppress these with `#[allow(unexpected_cfgs)]`?

---

## Next Steps (NOT Done Yet)

### Task 3: Migrate Protocol Layer
**Goal**: Replace `tom-transport` with `tom-connect` in:
- `crates/tom-protocol/Cargo.toml`
- `crates/tom-tui/Cargo.toml`
- `crates/tom-stress/Cargo.toml`

**Expected breakage**:
```rust
// OLD (tom-transport wrapper)
let node = TomNode::bind().await?;
let id = node.id();

// NEW (direct tom-connect)
let endpoint = Endpoint::bind().await?;
let id = endpoint.node_id(); // Or endpoint.id()?
```

**Review needed**: API migration guide

---

## Questions for Reviewer

1. **Namespace**: Did we miss any `iroh::` or `iroh_relay::` references?
2. **API Breaking**: Are `NodeId`/`NodeAddr` type aliases safe?
3. **Dependencies**: Will iroh-base v0.96 + tom-connect coexist peacefully?
4. **Tests**: Should we verify all copied tests still pass?
5. **Compatibility**: Is relay wire protocol definitely unchanged?
6. **Security**: Any crypto operations accidentally modified?
7. **Edition**: Is edition 2024 requirement acceptable?
8. **Fork Strategy**: Should we plan to eventually drop iroh-quinn dependency?
9. **Warnings**: How to handle 263 `wasm_browser` warnings?
10. **Documentation**: Are all doc links still valid?

---

## How to Use This Review

### For Copilot:
```
Read docs/REVIEW-R7.3-FORK.md and answer all 10 questions in "Questions for Reviewer".
Then run the bash checks in each section and report findings.
Focus on sections marked "Potential blind spots".
```

### For Human Reviewer:
1. Read this entire document
2. Run the grep/rg commands in each section
3. Review the "Priority 1" files line-by-line
4. Answer the 10 questions at the end
5. Document findings in a separate file

---

## Success Criteria

**Fork is successful if**:
- ✅ No `iroh::` references outside `iroh-base`/`iroh-relay` imports
- ✅ All public API types correctly aliased
- ✅ Wire protocol compatibility preserved
- ✅ No crypto operations accidentally changed
- ✅ All tests compile (passing is Task 3 goal)
- ✅ Documentation links valid
- ✅ Build succeeds on clean workspace

---

**Generated**: 2026-02-27
**By**: Claude Sonnet 4.5 (ToM Protocol Development)
**For**: R7.3 Fork Quality Assurance
