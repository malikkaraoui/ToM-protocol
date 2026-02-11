# Fork Architecture: iroh Independence Plan

> How ToM extracts transport-layer sovereignty from iroh without repeating
> the mistakes of the Hyperswarm/UDX ecosystem.

## Context

ToM chose **Chemin C**: PoC with iroh as dependency, then strategic fork.
The PoCs are done (100% hole punch, 4 real-world scenarios). This document
is the blueprint for Phase 2 — extracting what we need, discarding what we
don't, and never depending on n0-computer's roadmap again.

---

## Part 1: Five Lessons from UDX and the Hyperswarm Graveyard

The Dat → Hypercore Protocol → Holepunch chain is a case study in what
happens when you build on someone else's transport layer.

### Lesson 1: Org renames break everything silently

Dat Project (2013) → Hypercore Protocol (May 2020) → Holepunch (July 2022).
Three GitHub orgs in 7 years. Every rename broke import paths, npm scopes,
and documentation URLs. Downstream projects that pinned to `@hyperswarm/dht`
or `hypercore-protocol/*` found their dependencies pointing at deprecated
orgs. No code changed — the rug just moved.

**ToM rule**: Own the namespace. Our crates, our org, our registry names.
A fork means `tom-quic`, not `iroh-for-tom`.

### Lesson 2: Single-maintainer depth kills ecosystems

Mathias Buus (mafintosh) is brilliant — 926+ GitHub repos, the engine behind
the entire Hyperswarm stack. But `libudx` has 4 open issues, 11 closed, and
17 forks. `hypercore-protocol` on npm shows "not healthy version release
cadence" with 1 open source maintainer. When one person's priorities shift,
the entire stack stalls.

iroh has the same risk profile: small n0-computer team, VC-funded (funded by
n0 inc.), not yet profitable. Their pivot from "Rust IPFS" to "dial by key"
in early 2023 was the right call technically, but it orphaned every project
that had built on iroh-as-IPFS.

**ToM rule**: Fork the code, not the team. We need the bytes, not the
maintainers. Budget for understanding every line we take.

### Lesson 3: Deep dependency chains create invisible fragility

Holepunch's stack: Keet → Pear Runtime → Bare Runtime (custom Node.js fork)
→ Hyperswarm → HyperDHT → udx-native → libudx. Six layers deep. To use
their P2P transport, you need their JavaScript runtime. As one HN commenter
noted: adopting Holepunch means adopting "an entire REMAKE of nodejs."

iroh is cleaner (Rust crates, explicit boundaries), but the chain still
exists: your app → iroh → iroh-quinn (Quinn fork) → rustls → ring.

**ToM rule**: Fork at the socket boundary. Take MagicSock + relay client.
Everything above (Router, protocols, discovery) we rewrite to ToM semantics.
Everything below (Quinn, rustls) we use as standard upstream deps, not forks.

### Lesson 4: Tether money doesn't mean Tether forever

Holepunch is funded by Tether ($10M initial, restructured under "Tether Data"
in May 2024). Tether announced $1B across 5 divisions. Sounds safe. But
Tether's core business is stablecoins, not P2P transport. When priorities
shift — and they always shift — Holepunch becomes a line item to cut.

iroh's n0-computer has similar dynamics: VC-funded startup building
infrastructure. If iroh doesn't find product-market fit by 1.0, the team
may pivot, acqui-hire, or wind down.

**ToM rule**: The fork must be self-sustaining from day one. No "we'll update
when upstream updates." If n0-computer disappears tomorrow, ToM's transport
layer keeps working unchanged.

### Lesson 5: Beaker Browser died because P2P transport isn't enough

Beaker Browser (archived December 27, 2022) was the most visible Dat/Hyper
consumer. Creator Paul Frazee's post-mortem: Dat sites weren't accessible in
other browsers, no mobile existed, pure P2P created "fundamental challenges
around discovery, delivery, availability, and performance." The transport
worked. The product didn't.

**ToM rule**: The transport layer is a means, not an end. Fork only what
serves ToM's actual product needs (messaging, relay roles, virus backup).
Don't fork iroh-blobs or iroh-docs just because they exist.

---

## Part 2: What iroh Is Made Of

### Crate Map (v0.96)

```
iroh (main crate)
├── Endpoint          — Connection lifecycle, bind/connect/accept
├── MagicSock         — Path multiplexer (relay, IPv4, IPv6, direct)
├── Hole punching     — Disco-inspired, integrated in MagicSock
├── Discovery trait   — Pluggable: DNS, Pkarr, mDNS, static
└── Router            — ALPN-based protocol dispatch

iroh-base
├── EndpointId        — Ed25519 public key (= identity)
├── EndpointAddr      — ID + relay URL + IP addresses
├── RelayUrl          — Relay server URL
└── Hash, Ticket      — Content addressing, sharing

iroh-relay
├── Relay server      — WebSocket-only, stateless forwarding
├── Relay client      — Connect, authenticate, send/receive
├── Wire protocol     — Datagrams + ECN byte + anti-replay challenge
└── STUN              — Reflexive address discovery

iroh-quinn (Quinn fork)
├── QUIC transport    — Full QUIC implementation
├── Congestion ctrl   — Reset on path switch (the reason for the fork)
└── Crypto            — rustls integration
```

### Key insight: why they forked Quinn

When MagicSock switches from relay → direct, Quinn's congestion controller
doesn't know the path changed. Latency took ~20 seconds to stabilize.
iroh-quinn resets congestion state on path switch → ~3 seconds.

The fork is intentionally small. They sync upstream regularly. As QUIC
multipath (IETF drafts) matures, changes may merge back to Quinn.

### Relay protocol lineage

iroh's relay is a **Rust reimplementation** of Tailscale's DERP protocol
(BSD-3-Clause). Not wire-compatible with Tailscale — different framing,
auth, and addressing. Key differences:

| | Tailscale DERP | iroh Relay |
|---|---|---|
| Language | Go | Rust |
| Transport | HTTP upgrade → binary | WebSocket only |
| Identity | WireGuard keys | Ed25519 (EndpointId) |
| Auth | WireGuard handshake | Ed25519 + TLS keying material (RFC 9729) |
| Addressing | Region numbers | URLs |

---

## Part 3: The Fork — What We Take, What We Build

### Layer model

```
┌─────────────────────────────────────────┐
│  ToM Application Layer                  │  ← We build (SDK, groups, backup)
├─────────────────────────────────────────┤
│  ToM Protocol Layer                     │  ← We build (roles, wire format,
│  (roles, routing, virus backup,         │    contribution scoring, envelope
│   contribution scoring, envelopes)      │    signatures)
├─────────────────────────────────────────┤
│  tom-connect (forked from iroh)         │  ← We fork + own
│  ├── MagicSock (path multiplexer)       │
│  ├── Hole punching (Disco-inspired)     │
│  ├── Relay client                       │
│  └── Discovery (DNS + Pkarr + DHT)     │
├─────────────────────────────────────────┤
│  tom-relay (forked from iroh-relay)     │  ← We fork + own
│  └── Stateless relay server             │
├─────────────────────────────────────────┤
│  Quinn (upstream, not forked)           │  ← Standard dep
│  └── QUIC transport + TLS              │
├─────────────────────────────────────────┤
│  OS / UDP sockets                       │
└─────────────────────────────────────────┘
```

### What we fork (5 crates → 2 crates)

| Source | Target | Why |
|--------|--------|-----|
| `iroh` (MagicSock, hole punch, discovery) | `tom-connect` | Core connectivity — the reason we chose iroh |
| `iroh-base` (EndpointId, EndpointAddr) | `tom-connect` (inline) | Types are small, merge into main crate |
| `iroh-relay` (server + client) | `tom-relay` | Relay is central to ToM's architecture |
| `iroh-quinn` | **Not forked** — use upstream Quinn | iroh's Quinn fork is tiny (congestion reset). We apply the same patch to upstream Quinn or contribute it. Maintaining a Quinn fork is high cost for minimal gain. |
| `netwatcher` + `portmapper` | `tom-connect` deps (upstream) | Utility crates, no fork needed, use as-is |

### What we don't take

| iroh crate | Why skip |
|------------|----------|
| iroh-blobs | Content-addressed transfer — ToM is messaging, not file sharing |
| iroh-docs | Collaborative documents — not in ToM's scope |
| iroh-gossip | **We rewrite**. HyParView/PlumTree is great but ToM needs gossip integrated with role assignment and contribution scoring. Different membership semantics. |
| iroh-dns-server | ToM targets DHT-first discovery, not DNS-first |
| iroh Router | ToM's Router already exists (TypeScript). Rust Router will be ToM-specific. |

### What we rewrite (ToM-specific)

| Component | Why rewrite, not fork |
|-----------|----------------------|
| Gossip | ToM gossip carries role announcements, contribution proofs, and peer reputation — not just topic messages. Different protocol. |
| Discovery | DHT-first (Hyperswarm philosophy), not DNS-first. Pkarr stays (good for zero-infra), DNS becomes optional. |
| Router/Protocol dispatch | ToM envelope format, relay chain selection, backup routing — all ToM-specific semantics. |
| Identity | Same Ed25519 model, but ToM adds contribution scoring and progressive reputation. Thin wrapper. |

---

## Part 4: The iroh API Churn Problem

iroh has had **breaking changes in every release** from 0.90 to 0.96:

| Version | Date | What broke |
|---------|------|------------|
| 0.28 | Mid-2024 | Crate split: blobs, docs, gossip extracted |
| 0.29 | Dec 2024 | iroh-net → iroh rename. Node → Router. CLI removed. |
| 0.90 | Sep 2024 | Canary series. Concrete errors. x509 removed. |
| 0.91 | Oct 2024 | Relay wire protocol rewrite. WebSocket-only. |
| 0.94 | Late 2024 | NodeId → EndpointId. NodeAddr → EndpointAddr. |
| 0.95 | Nov 2025 | snafu → n0-error. Discovery → AddressLookup. |
| 0.96 | Jan 2026 | Address prioritization. Custom transports. |

15 releases in 13 months, all breaking. This is exactly why we fork:

- **Pinning to 0.96** means missing security fixes and improvements.
- **Tracking upstream** means constant migration work.
- **Forking** means we take the snapshot that works and evolve on our own terms.

iroh targets **1.0-rc in Q1 2026** (0.97 on Feb 23, 2026). Our fork timing
matters: fork *after* 0.97/1.0-rc for maximum stability of the wire protocol,
but don't wait for 1.0 if it slips.

**Decision**: Fork from 0.97 or 1.0-rc (whichever comes first and stabilizes
the relay wire protocol). Until then, PoC stays on 0.96 as a learning tool.

---

## Part 5: Fork Execution Plan

### Phase 1: Understand (✅ Done)

- [x] PoC-1 through PoC-4 — iroh's connectivity model validated
- [x] 100% hole punch success across 4 real-world scenarios
- [x] iroh API surface mapped (Endpoint, MagicSock, Gossip, Relay)
- [x] Key learnings documented (this document + README + MEMORY.md)

### Phase 2: Extract (Next — when iroh hits 0.97/1.0-rc)

1. **Create `tom-connect` crate**
   - Copy iroh's MagicSock, hole punching, discovery modules
   - Inline iroh-base types (EndpointId → TomNodeId, etc.)
   - Strip protocol dispatch (Router, ALPN) — we build our own
   - Strip all iroh-blobs/docs/gossip integration points
   - Apply Quinn congestion-reset patch to upstream Quinn (no Quinn fork)

2. **Create `tom-relay` crate**
   - Copy iroh-relay server + client
   - Add ToM-specific: relay role assignment headers, contribution tracking
   - Keep wire protocol compatible initially (can use n0 relays during transition)

3. **Rename everything**
   - `iroh::Endpoint` → `tom_connect::Endpoint`
   - `EndpointId` → `NodeId` (ToM terminology)
   - `EndpointAddr` → `NodeAddr`
   - No `iroh` in any public API or crate name

4. **Write tests**
   - Port iroh's connection tests to tom-connect
   - Localhost hole punch test (already have the script)
   - Relay fallback test
   - Path switch test (relay → direct → relay)

### Phase 3: Adapt (After extraction)

1. **DHT-first discovery**
   - Integrate Mainline DHT (BEP-0044) as primary discovery
   - Pkarr as secondary (already uses BEP-0044 under the hood)
   - DNS as optional fallback

2. **ToM gossip protocol**
   - Rewrite on top of tom-connect (not iroh-gossip fork)
   - Integrate role announcements, contribution proofs
   - Progressive reputation in membership decisions

3. **ToM wire format**
   - JSON envelopes with signatures over tom-connect QUIC streams
   - Relay chain in envelope headers
   - Virus backup routing metadata

### Phase 4: Integrate (Replace TypeScript signaling)

1. **Rust core + TypeScript SDK via WASM or FFI**
   - tom-connect compiled to library
   - TypeScript SDK calls into Rust for transport
   - Signaling server eliminated

2. **Migration path**
   - WebSocket signaling → tom-connect relay (parallel operation)
   - Gradual cutover, feature flag per transport
   - Demo app validates both paths

---

## Part 6: Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| iroh 1.0 slips past Q2 2026 | Medium | Low | Fork from 0.97 if stable enough |
| MagicSock internals too coupled to iroh | Medium | High | PoC-4 proves the boundary is clean enough. Budget 2 weeks for extraction. |
| Quinn congestion patch rejected upstream | Low | Low | Maintain minimal patch file (~50 lines). Not a full fork. |
| n0-computer relay servers shut down | Low | High | tom-relay is self-hostable from day one. Deploy our own during Phase 2. |
| iroh relicenses (currently MIT) | Very Low | High | Fork is MIT-licensed at fork point. Subsequent changes are ours. |
| QUIC multipath makes our Quinn patch obsolete | Medium | Positive | Less code to maintain. Welcome outcome. |

---

## Appendix: iroh vs Tailscale Lineage

iroh is **not a Tailscale fork**. It's a Rust reimplementation inspired by:

- **MagicSock** concept (path multiplexer over relay + direct)
- **DERP** concept (encrypted relay as fallback)
- **Disco** protocol (hole punching coordination)

All reimplemented from scratch in Rust over QUIC (not WireGuard).
Tailscale's code is BSD-3-Clause, iroh's is MIT. No license conflict.
