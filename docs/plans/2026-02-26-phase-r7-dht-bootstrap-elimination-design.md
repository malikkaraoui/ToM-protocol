# Phase R7: DHT + Bootstrap Elimination — Design Document

**Date:** 2026-02-26
**Status:** Draft
**Context:** Post-Phase R6 (352 tests, all core features complete)

---

## Goal

Eliminate the WebSocket signaling server (bootstrap layer) by implementing DHT-first peer discovery. Transform ToM from "P2P with bootstrap" to "truly serverless P2P".

## Success Criteria

1. ✅ DHT-based peer discovery functional (Mainline DHT BEP-0044)
2. ✅ Signaling server optional (graceful fallback, not required)
3. ✅ All existing tests pass with DHT discovery
4. ✅ 0-infra deployment path documented
5. ✅ Fork independence: tom-connect + tom-relay replace iroh dependency

---

## Architecture Overview

### Current State (Phase R6)

```
┌─────────────┐
│  TypeScript │  ← Demo app
│  packages/  │
└──────┬──────┘
       │ WebSocket signaling (bootstrap)
       ↓
┌─────────────┐
│ tom-protocol│  ← Rust protocol layer (groups, roles, backup)
│ tom-transport│  ← Rust transport (iroh wrapper)
└──────┬──────┘
       │ iroh 0.96 (dependency)
       ↓
┌─────────────┐
│   iroh      │  ← MagicSock, QUIC, relay, Pkarr, iroh-gossip
└─────────────┘
```

**Problem:** WebSocket signaling is a centralized bottleneck. Bootstrap VPS required for demo/production.

---

### Target State (Phase R7)

```
┌─────────────┐
│  TypeScript │  ← Demo app
│  packages/  │
└──────┬──────┘
       │ (no WebSocket signaling)
       ↓
┌─────────────┐
│ tom-protocol│  ← Protocol layer
│ tom-connect │  ← NEW: Forked from iroh (MagicSock, hole punch, discovery)
│ tom-relay   │  ← NEW: Forked from iroh-relay (stateless relay)
└──────┬──────┘
       │ Mainline DHT (BEP-0044) primary discovery
       │ Pkarr (BEP-0044) secondary
       │ iroh public relays (fallback)
       ↓
┌─────────────┐
│    Quinn    │  ← Upstream QUIC (NOT forked)
│   rustls    │
└─────────────┘
```

**Key change:** DHT replaces signaling. Discovery is distributed. No single point of failure.

---

## Design Decisions

### DD-1: Fork Boundary — Socket Layer Only

**Decision:** Fork iroh at the socket boundary (MagicSock + relay client). Do NOT fork Quinn, rustls, or protocol layers.

**Rationale:**
- iroh's MagicSock (path multiplexer) + hole punching are proven (PoC-4: 100% success)
- Quinn congestion-reset patch is tiny (~50 lines), apply to upstream
- Protocol layer (Router, groups, roles) is already ToM-specific
- Minimize fork surface = minimize maintenance burden

**Modules we fork:**
- `iroh` → `tom-connect` (MagicSock, hole punch, discovery traits)
- `iroh-base` → inline into `tom-connect` (EndpointId, EndpointAddr types)
- `iroh-relay` → `tom-relay` (relay server + client)

**Modules we DON'T fork:**
- iroh-blobs, iroh-docs (not needed)
- iroh-gossip (we already have tom-protocol gossip)
- iroh-quinn (use upstream Quinn + patch)
- netwatcher, portmapper (use as upstream deps)

---

### DD-2: DHT-First Discovery Philosophy

**Decision:** Mainline DHT (BEP-0044) is the primary discovery mechanism. Pkarr and DNS are fallbacks.

**Rationale:**
- Mainline DHT: 20M+ BitTorrent nodes, proven at scale, zero-infra
- Pkarr uses BEP-0044 under the hood (compatible)
- DNS requires infrastructure (iroh's iroh-dns-server)
- ToM philosophy: "zero fixed infrastructure" → DHT aligns

**Discovery priority:**
1. Mainline DHT (check for peer by NodeId hash)
2. Pkarr (signed records, 24h TTL)
3. iroh public relays (last resort fallback)

---

### DD-3: Bootstrap Elimination = Graceful Degradation

**Decision:** Keep WebSocket signaling as an OPTIONAL bootstrap accelerator during transition. Do NOT hard-remove it until DHT is battle-tested.

**Rationale:**
- DHT cold-start can take 3-10 seconds
- WebSocket signaling gives instant peer discovery for demos
- Migration path: run both in parallel, measure DHT success rate, then deprecate signaling
- Safety: if DHT fails (firewall/ISP blocks), signaling is fallback

**Transition phases:**
1. **Phase R7.1:** DHT primary, signaling fallback (both work)
2. **Phase R7.2:** DHT-only mode available (feature flag)
3. **Phase R7.3:** Signaling deprecated (docs updated, marked obsolete)
4. **Phase R7.4:** Signaling removed (breaking change, major version bump)

---

### DD-4: Naming Conventions

**Decision:** Rename all iroh-specific terminology to ToM-specific.

**Rationale:**
- Avoid confusion with upstream iroh
- Make fork ownership explicit
- Legal clarity (MIT license, but distinct namespace)

| iroh | tom-connect |
|------|-------------|
| `iroh::Endpoint` | `tom_connect::Endpoint` |
| `EndpointId` | `NodeId` |
| `EndpointAddr` | `NodeAddr` |
| `MagicSock` | `MagicSock` (keep, it's a good name) |
| `iroh_relay` | `tom_relay` |

---

## Component Breakdown

### 1. tom-connect (New Crate)

**Purpose:** Transport-layer connectivity. Replaces `tom-transport` (which is currently a thin iroh wrapper).

**Responsibilities:**
- UDP socket binding (IPv4 + IPv6)
- Hole punching (Disco-inspired, integrated in MagicSock)
- Path multiplexing (relay + direct, automatic upgrade)
- Discovery trait (DHT + Pkarr + relay fallback)
- QUIC endpoint management

**API surface (for tom-protocol):**
```rust
pub struct Endpoint { /* ... */ }

impl Endpoint {
    pub async fn bind(config: Config) -> Result<Self>;
    pub fn node_id(&self) -> NodeId;
    pub fn node_addr(&self) -> NodeAddr;
    pub async fn connect(&self, addr: NodeAddr) -> Result<Connection>;
    pub fn accept(&self) -> impl Stream<Item = Connecting>;
}

pub struct Connection { /* Quinn wrapper */ }
```

**Dependencies:**
- quinn (upstream, NOT forked)
- rustls
- mainline-dht (new dep for BEP-0044)
- pkarr (already in iroh deps)

---

### 2. tom-relay (New Crate)

**Purpose:** Stateless relay server + client. Forked from iroh-relay.

**Responsibilities:**
- Relay server (WebSocket, stateless forwarding)
- Relay client (connect, authenticate, send/receive)
- Wire protocol (datagrams + ECN + anti-replay challenge)
- STUN (reflexive address discovery)

**Why fork iroh-relay:**
- ToM adds contribution tracking headers (relay bandwidth stats)
- Role assignment (dynamic relay selection based on contribution)
- Wire protocol stays compatible with iroh relays during transition

**Deployment:**
- Self-hostable (Docker + fly.io one-liner)
- Public relays run by community (optional)
- iroh's n0-computer relays work as fallback

---

### 3. DHT Integration (mainline-dht crate)

**Purpose:** Distributed peer discovery via Mainline DHT (BEP-0044).

**Flow:**
1. Node starts → joins Mainline DHT (bootstrap from known nodes)
2. Node publishes: `put(hash(NodeId), signed_NodeAddr)` → 24h TTL
3. Peer lookup: `get(hash(target_NodeId))` → returns signed NodeAddr
4. Connect via MagicSock (relay or direct)

**Storage model:**
- Key: `SHA1(NodeId)` (20 bytes, DHT-compatible)
- Value: `signed { NodeAddr, timestamp, relay_urls, direct_addrs }`
- TTL: 24h (re-publish every 12h)

**Failure modes:**
- DHT bootstrap fails → fallback to Pkarr
- DHT returns stale address → MagicSock tries relay
- No DHT response within 5s → try signaling (if enabled)

---

## Migration Path

### Step 1: Fork Preparation (Current Phase)

1. ✅ Upgrade to iroh 0.97 or 1.0-rc (wait for wire protocol stability)
2. ✅ Document iroh API usage (what we call, what we don't)
3. ✅ Create tom-connect skeleton (copy iroh modules)
4. ✅ Create tom-relay skeleton (copy iroh-relay)

### Step 2: DHT Integration

1. ✅ Add mainline-dht dependency
2. ✅ Implement DHT discovery trait (tom-connect)
3. ✅ NodeAddr publish/lookup via DHT
4. ✅ Integration tests (2 nodes find each other via DHT)

### Step 3: Replace tom-transport

1. ✅ Swap `tom-transport` for `tom-connect` in tom-protocol
2. ✅ Update all API calls (EndpointId → NodeId, etc.)
3. ✅ Run all 352 tests → verify pass
4. ✅ Update demos (TypeScript → WASM bindings if needed)

### Step 4: Signaling Deprecation

1. ✅ Feature flag: `--dht-only` mode
2. ✅ Measure DHT success rate (Campaign V7 with DHT)
3. ✅ Update docs: "signaling optional"
4. ✅ Archive signaling-server crate (mark deprecated)

---

## Risk Analysis

| Risk | Impact | Mitigation |
|------|--------|------------|
| DHT bootstrap slow (10s+) | High | Keep signaling as fallback during R7 |
| Mainline DHT blocked by ISP | Medium | Pkarr + relay fallback |
| Fork maintenance burden | High | Minimal fork surface (socket only) |
| iroh wire protocol breaks | Medium | Fork from stable 0.97/1.0-rc |
| Quinn patch rejected upstream | Low | 50-line patch easy to maintain |
| Community relay spam | Medium | Rate limiting + reputation (Phase R8) |

---

## Open Questions

1. **WASM support:** Can mainline-dht run in browser? (likely no → use Pkarr only in browser)
2. **DHT spam:** How to prevent malicious NodeAddr pollution? (BEP-0044 has mutable storage + signature, but need anti-spam)
3. **Relay incentives:** How do we reward community relay operators? (Phase R8: token/contribution model)

---

## References

- `experiments/iroh-poc/FORK-ARCHITECTURE.md` — Fork strategy
- `_bmad-output/planning-artifacts/architecture.md` — ADR-002 (bootstrap elimination)
- BEP-0044: Storing arbitrary data in the DHT (http://bittorrent.org/beps/bep_0044.html)
- iroh documentation: https://iroh.computer/docs
