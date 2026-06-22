# Code Review Prompt - Phase R7.3 Fork (iroh → ToM)

**Date**: 2026-02-27
**Phase**: R7.3 - The Fork (Tasks 1 & 2 Complete)
**Reviewer**: Copilot GPT-5.3 Codex / Claude / Human
**Goal**: Catch blind spots, validate fork integrity, ensure wire compatibility

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
- Kept dependencies: `iroh-base`, `tom-relay`, `iroh-metrics`
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

**Check for leftover module references**:
```bash
cd crates/tom-connect && rg "iroh::" src/ | grep -v "iroh-base" | grep -v "tom-relay" | grep -v "iroh-metrics"
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
cargo test -p tom-connect 2>&1 | head -50
cargo test -p tom-relay 2>&1 | head -50
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

#### 7.1 🚨 BLIND SPOT: Inline Comments & Strings
**Question**: Are there "iroh" references in non-import contexts?

**Run this comprehensive search**:
```bash
cd crates/tom-connect
rg -i "\biroh\b" src/ --type rust | grep -v "iroh-base" | grep -v "tom-relay" | grep -v "iroh-metrics" | grep -v "iroh-quinn"

cd ../tom-relay
rg -i "\biroh\b" src/ --type rust
```

**What to check**:

1. **ALPN strings**:
   - `b"n0/iroh/test"` → Should be `b"n0/tom/test"`?
   - File: `endpoint.rs`, `connection.rs`, `address_lookup.rs`
   - **Decision needed**: Break test compatibility or keep?

2. **HTTP headers**:
   - `X-Iroh-Challenge`, `X-Iroh-NodeId`, `X-Iroh-Response`
   - File: `tom-relay/src/main.rs`, `tom-relay/src/server.rs`, `tom-connect/src/net_report/reportgen.rs`
   - **Decision needed**: Wire protocol - must stay compatible with iroh relay network

3. **DNS labels**:
   - `_iroh` TXT records → **KEEP** (wire format!)
   - File: `tom-relay/src/endpoint_info.rs`

4. **TLS names**:
   - `.iroh.invalid` → **KEEP** (wire format for compatibility!)
   - File: `tom-connect/src/tls/name.rs`

5. **External URLs**:
   - `iroh.link`, `iroh-canary`, `iroh.network` → **KEEP** (external relays)
   - `docs.rs/iroh-tickets` → **UPDATE** or remove
   - `github.com/n0-computer/iroh` → **KEEP** (attribution)
   - Files: `defaults.rs`, `address_lookup/pkarr.rs`, `endpoint_info.rs`

6. **Test data**:
   - Hardcoded "iroh" in test fixtures → Review case-by-case
   - File: Various test modules

7. **Error messages**:
   - User-facing strings with "iroh" → Should be generic
   - Check: `bail!()`, `tracing::error!()`, etc.

8. **Metric names**:
   - Prometheus label names with "iroh_" → **KEEP** for now?
   - File: `metrics.rs` modules

**Decision matrix**:
| Reference | Action | Reason | Files |
|-----------|--------|--------|-------|
| DERP wire protocol | **KEEP** | Compatibility with iroh network | `protos/relay.rs` |
| DNS TXT `_iroh` | **KEEP** | Wire format standard | `endpoint_info.rs` |
| TLS `.iroh.invalid` | **KEEP** | Certificate validation | `tls/name.rs` |
| HTTP headers | **KEEP** | Relay server protocol | `server.rs`, `main.rs` |
| Test ALPN | **CHANGE?** | Internal only, no compat risk | `endpoint.rs:TEST_ALPN` |
| External relay URLs | **KEEP** | Using n0's infrastructure | `defaults.rs` |
| Doc links to docs.rs | **REMOVE/UPDATE** | Misleading | `address_lookup/memory.rs` |
| GitHub issue links | **KEEP** | Historical context | `socket.rs` |
| Comments mentioning iroh | **UPDATE** | Clarity | All files |

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

**ALPN verification**:
```bash
rg "ALPN" crates/tom-relay/src/
# Should find: b"/iroh-qad/0" (QUIC address discovery)
# Do NOT change - this is wire protocol!
```

**HTTP header verification**:
```bash
rg "X-Iroh-" crates/tom-relay/src/
# Should find: X-Iroh-Challenge, X-Iroh-Response, X-Iroh-NodeId
# Do NOT change - relay clients expect these!
```

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

#### 11. 🚨 BLIND SPOT: Inline Documentation Examples
**Question**: Do doc comment examples still reference `iroh::`?

**Check**:
```bash
rg '```rust' -A 10 crates/tom-connect/src/ | rg 'iroh::'
rg '```' -A 10 crates/tom-relay/src/ | rg 'iroh'
```

**What to look for**:
```rust
/// # Example
/// ```rust
/// use iroh::Endpoint;  // ← BROKEN
/// ```
```

Should be:
```rust
/// # Example
/// ```rust
/// use tom_connect::Endpoint;  // ← FIXED
/// ```
```

---

#### 12. 🚨 BLIND SPOT: Cfg Aliases and Build-Time Codegen
**Question**: Does `build.rs` define custom `cfg` names that reference `iroh`?

**Check**:
```bash
cat crates/tom-connect/build.rs 2>/dev/null || echo "No build.rs"
cat crates/tom-relay/build.rs 2>/dev/null || echo "No build.rs"
```

**Look for**:
- `cfg_aliases::cfg_aliases!` macro calls
- Custom `cfg(iroh_*)` definitions
- Should be renamed to `cfg(tom_*)`

**Example**:
```rust
// build.rs
cfg_aliases::cfg_aliases! {
    wasm_browser: { all(target_family = "wasm", target_os = "unknown") },
    iroh_docsrs: { all(doc, not(doctest)) },  // ← Should be tom_docsrs?
}
```

---

#### 13. 🚨 BLIND SPOT: Metric Names (Prometheus)
**Question**: Do Prometheus metrics still have `iroh_` prefix?

**Check**:
```bash
rg 'describe_|register_' crates/tom-connect/src/metrics.rs -A 2 2>/dev/null || echo "No metrics.rs"
```

**Look for**:
```rust
describe_counter!("iroh_endpoint_connections_total", ...);  // ← Should be tom_connect_*
```

**Decision needed**:
- Keep `iroh_*` for compatibility with existing dashboards?
- Change to `tom_*` for clarity?

---

#### 14. 🚨 BLIND SPOT: Macro-Generated Code
**Question**: Do macros expand to code with `iroh` references?

**Files to review manually**:
- Any `macro_rules!` definitions
- `#[derive]` macros with custom attributes
- Procedural macros (`#[iroh_macro]`?)

**Check**:
```bash
rg 'macro_rules!' crates/tom-connect/src/
rg '#\[derive' crates/tom-connect/src/ | head -30
```

---

#### 15. 🚨 BLIND SPOT: Test Utilities and Fixtures
**Question**: Are there hardcoded test keys/IDs with "iroh" metadata?

**Check**:
```bash
rg 'const.*TEST' crates/tom-connect/src/ --type rust -A 2
rg 'lazy_static' crates/tom-connect/src/ --type rust -A 5
```

**Example**:
```rust
const TEST_NODE_ID: &str = "iroh_test_node_abc123";  // ← Cosmetic, but update for clarity
```

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

### Priority 4 (Wire Compatibility - DO NOT BREAK)
11. `crates/tom-connect/src/tls/name.rs` - `.iroh.invalid` TLS names
12. `crates/tom-relay/src/endpoint_info.rs` - `_iroh` DNS records
13. `crates/tom-relay/src/protos/relay.rs` - DERP frame format
14. `crates/tom-relay/src/server.rs` - HTTP headers (`X-Iroh-*`)
15. `crates/tom-relay/src/quic.rs` - ALPN `b"/iroh-qad/0"`

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

**Check fork-specific APIs**:
```bash
rg 'WeakConnectionHandle|PathId|UnorderedRecvStream' crates/tom-connect/src/
```

### 3. Warnings Explosion
**Issue**: 263 warnings in tom-connect (all `wasm_browser` cfg)
**Impact**: Noise in build output
**Question**: Should we suppress these with `#[allow(unexpected_cfgs)]`?

**Check**:
```bash
cargo build -p tom-connect 2>&1 | grep 'unexpected_cfgs'
```

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

## Questions for Reviewer (Answer ALL 15)

1. **Namespace**: Did we miss any `iroh::` or `iroh_relay::` module references?
2. **API Breaking**: Are `NodeId`/`NodeAddr` type aliases safe?
3. **Dependencies**: Will iroh-base v0.96 + tom-connect coexist peacefully?
4. **Tests**: Should we verify all copied tests still pass?
5. **Compatibility**: Is relay wire protocol definitely unchanged?
6. **Security**: Any crypto operations accidentally modified?
7. **Edition**: Is edition 2024 requirement acceptable?
8. **Fork Strategy**: Should we plan to eventually drop iroh-quinn dependency?
9. **Warnings**: How to handle 263 `wasm_browser` warnings?
10. **Documentation**: Are all doc links still valid?
11. **Wire Format**: Are these safe to keep? `_iroh` DNS, `.iroh.invalid` TLS, `X-Iroh-*` headers, `b"/iroh-qad/0"` ALPN?
12. **Test ALPN**: Should `b"n0/iroh/test"` become `b"n0/tom/test"`?
13. **Metrics**: Keep `iroh_*` prefix or change to `tom_*`?
14. **Build Cfg**: Should `iroh_docsrs` → `tom_docsrs`?
15. **Comments**: Update all inline comments mentioning "iroh" for clarity?

---

## How to Use This Review

### For Copilot GPT-5.3 Codex:
```
You are reviewing a strategic fork of iroh v0.96.0 (21,756 lines transport + 11,783 lines relay).

GOAL: Catch blind spots the initial implementation missed.

EXECUTE:
1. Read this entire document (scroll to bottom)
2. Run EVERY bash command in sections 1-15
3. Answer ALL 15 questions with evidence from code
4. Pay special attention to 🚨 BLIND SPOT sections
5. Review Priority 4 files (wire compatibility) first

FOCUS AREAS:
- Wire protocol compatibility (DERP, DNS, TLS, HTTP headers, ALPN)
- Namespace leaks (iroh::, iroh_relay::)
- Test ALPN strings (cosmetic vs breaking)
- HTTP headers (relay server protocol)
- Metric names (prometheus labels)
- Build-time cfg (iroh_docsrs)
- Inline comments/docs (clarity)

REPORT FORMAT:
## Findings

### ✅ Correct (no action needed)
- [List what's properly done]

### ⚠️ Needs Review (decision required)
- [List items needing human decision]

### 🔴 Must Fix (breaking or incorrect)
- [List critical issues with file:line references]

### 💡 Recommendations
- [Optional improvements]

START NOW. Be rigorous. This is 33,539 lines of networking code - details matter.
```

### For Human Reviewer:
1. Read this entire document
2. Run the grep/rg commands in each section
3. Review the "Priority 4" files first (wire compatibility)
4. Then review "Priority 1" files (API surface)
5. Answer all 15 questions at the end
6. Document findings in a separate file

---

## Success Criteria

**Fork is successful if**:
- ✅ No `iroh::` module references outside allowed imports (`iroh-base`, `tom-relay`, `iroh-metrics`, `iroh-quinn`)
- ✅ All public API types correctly aliased (`NodeId`, `NodeAddr`)
- ✅ Wire protocol compatibility preserved:
  - DERP frame format unchanged
  - DNS `_iroh` label unchanged
  - TLS `.iroh.invalid` unchanged
  - HTTP headers `X-Iroh-*` unchanged (relay server)
  - ALPN `b"/iroh-qad/0"` unchanged (QUIC address discovery)
- ✅ No crypto operations accidentally changed
- ✅ All tests compile (passing is Task 3 goal)
- ✅ Documentation links valid (no broken `[`iroh::`]` refs)
- ✅ Build succeeds on clean workspace
- ⚠️ Test ALPN `b"n0/iroh/test"` - decision needed
- ⚠️ Metric names `iroh_*` - decision needed
- ⚠️ Comments mentioning "iroh" - update for clarity

---

**Generated**: 2026-02-27
**By**: Claude Sonnet 4.5 (ToM Protocol Development)
**For**: R7.3 Fork Quality Assurance
**Reviewer**: Copilot GPT-5.3 Codex / Claude Opus / Human Expert

**Instructions**: This review is designed to catch blind spots from the initial fork. Run all commands, answer all questions, and be especially rigorous on wire protocol compatibility (Priority 4 files). The success of this fork depends on maintaining compatibility while achieving independence.
