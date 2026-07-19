# ToM Protocol - Claude/LLM Documentation

This document provides comprehensive guidance for LLMs working with the ToM Protocol codebase.

## Project Overview

**ToM (The Open Messaging)** is a decentralized peer-to-peer transport protocol where every device acts as both client and relay. Key principles:

- **No central servers**: Messages route through peer relays
- **Relay statelessness**: Relays forward without storing (pass-through only)
- **End-to-end encryption**: Only sender and recipient can read content
- **Dynamic roles**: Network assigns relay duties based on contribution
- **Self-organizing**: Gossip discovery and ephemeral subnets

## Repository Structure

```
tom-protocol/
├── crates/
│   ├── tom-connect/          # Transport layer (forked from iroh 0.96, ~15K LOC)
│   │                         #   MagicSock, Disco hole punching, Endpoint
│   ├── tom-relay/            # Relay server (forked from iroh-relay, ~8K LOC)
│   │                         #   Stateless relay, HTTP/HTTPS, --dev mode
│   ├── tom-gossip/           # Gossip protocol (forked from iroh-gossip, ~5K LOC)
│   ├── tom-quinn/            # QUIC runtime (forked from iroh-quinn, 6.5K LOC)
│   ├── tom-quinn-proto/      # QUIC protocol (forked from iroh-quinn-proto, 41K LOC)
│   ├── tom-base/             # Base types: PublicKey, SecretKey, NodeAddr (forked, 831 LOC)
│   ├── tom-metrics/          # Simplified metrics (Counter struct, ~100 LOC)
│   ├── tom-dht/              # DHT discovery (Mainline BEP-0044) + rendez-vous partagé zéro-config (ADR-010)
│   ├── tom-protocol/         # Protocol layer (original)
│   │   └── src/
│   │       ├── backup/       # Message backup (virus metaphor, TTL 24h)
│   │       ├── crypto/       # Ed25519 sign + X25519 DH + XChaCha20-Poly1305 + HKDF
│   │       ├── discovery/    # PeerAnnounce, HeartbeatTracker, EphemeralSubnets
│   │       ├── envelope/     # MessagePack wire format, EnvelopeBuilder
│   │       ├── group/        # GroupManager, GroupHub, hub failover, sender keys
│   │       ├── relay/        # RelaySelector, Topology
│   │       ├── roles/        # RoleManager, ContributionMetrics, scoring
│   │       ├── router/       # Router (Deliver/Forward/Reject/Ack/Drop)
│   │       ├── runtime/      # ProtocolRuntime, RuntimeState, effect pattern
│   │       ├── tracker/      # MessageTracker (status state machine)
│   │       └── types/        # NodeId, MessageType, MessageStatus
│   ├── tom-stress/           # Stress testing campaigns (Mac ↔ NAS)
│   └── tom-tui/              # TUI chat client (ratatui, --bot mode)
│
├── packages/                 # TypeScript (Phase 1, legacy)
│   ├── core/                 # Protocol primitives (771 tests)
│   └── sdk/                  # TomClient SDK
│
├── apps/
│   └── demo/                 # Browser demo (vanilla HTML/JS + Vite)
│
├── tools/
│   └── signaling-server/     # DEPRECATED — WebSocket bootstrap (Phase R7)
│
├── docs/plans/               # Design docs and implementation plans
└── _bmad-output/             # Planning artifacts (PRD, architecture, epics)
```

## Architecture (Post-Fork — Phase R7)

### Transport Stack

```
Application
    ↓
tom-protocol (ProtocolRuntime)     ← Protocol logic, groups, encryption
    ↓
tom-connect (Endpoint/MagicSock)   ← NAT traversal, hole punching, relay fallback
    ↓
tom-quinn (QUIC runtime)           ← Connection management
    ↓
tom-quinn-proto (QUIC protocol)    ← Wire protocol, crypto handshake
    ↓
UDP (iroh-quinn-udp)               ← Raw UDP I/O (not forked — netwatch compat)
```

### Fork Status

> **Indépendance (à retenir)** : ToM est **né d'un fork d'iroh 0.96**, mais le protocole est désormais **autonome et souverain**. Tous les identifiants de protocole (DNS `_tom`, SNI `.tom.invalid`, en-têtes `X-Tom-*`, ALPN `tom-*`) sont sous notre namespace. Conséquence assumée : **ToM n'est PAS — et n'a pas vocation à rester — wire-compatible avec le réseau iroh public.** iroh est notre *point de départ historique*, pas une dépendance réseau. Les seuls résidus iroh sont des hostnames de services externes n0 (`dns.iroh.link`), actifs uniquement avec le preset `n0_discovery`, et remplaçables.

All critical iroh dependencies have been forked under the `tom-*` namespace (MIT license):

| Original | Fork | LOC | Notes |
|----------|------|-----|-------|
| iroh (endpoint+socket) | tom-connect | ~15K | MagicSock, Disco, hole punching |
| iroh-relay | tom-relay | ~8K | Stateless relay server |
| iroh-gossip | tom-gossip | ~5K | Gossip protocol |
| iroh-quinn | tom-quinn | 6.5K | QUIC runtime |
| iroh-quinn-proto | tom-quinn-proto | 41K | QUIC protocol |
| iroh-base | tom-base | 831 | PublicKey, SecretKey, NodeAddr |
| iroh-metrics | tom-metrics | ~100 | Simplified Counter struct |

**Not forked (intentional):**
- `iroh-quinn-udp` — netwatch exposes its types in public API, forking creates type mismatch
- `n0-error`, `n0-future`, `n0-watcher` — general-purpose utils, shared with external deps

### Wire Invariants (état réel — vérifié 2026-06-26)

⚠️ Les identifiants protocolaires ont été **migrés vers le namespace `tom-*`**. Conséquence : ToM n'est **PAS** wire-compatible avec le réseau iroh public (un nœud iroh ne peut pas parler à un nœud/relais ToM, et inversement). C'est volontaire (souveraineté du protocole), mais à ne PAS casser entre versions ToM :

| Invariant | Valeur réelle | Fichier:ligne |
|-----------|---------------|---------------|
| DNS record prefix | `_tom` | `tom-relay/src/endpoint_info.rs:739` |
| TLS SNI | `.tom.invalid` | `tom-connect/src/tls/name.rs:19` |
| HTTP headers (relay) | `X-Tom-*` (`X-Tom-NodeId`, `X-Tom-Challenge`, `X-Tom-Response`) | `tom-relay/src/main.rs:35`, `server.rs:59` |
| ALPN transport | `b"tom-protocol/transport/0"` | `tom-transport/src/lib.rs:116` |
| ALPN gossip | `b"/tom-gossip/1"` | `tom-gossip/src/net.rs:46` |

**Restent en dur sur l'infra iroh/n0 (services externes, actifs seulement avec le preset `n0_discovery`)** — PAS des invariants ToM, remplaçables : `dns.iroh.link` (`tom-relay/src/dns.rs:32`), `https://dns.iroh.link/pkarr` (`tom-connect/src/address_lookup/pkarr.rs:126`), `*.iroh-canary.iroh.link` (`tom-connect/src/endpoint.rs:1412`).

### Cargo Alias Trick

Consumers use package aliases to avoid source code changes:
```toml
quinn = { package = "tom-quinn" }     # source says quinn::Connection
```
Inside tom-quinn:
```toml
proto = { package = "tom-quinn-proto" } # source says proto::
```

## Key Architecture Decisions (ADRs)

### ADR-001: QUIC via Relay (updated from WebRTC)
All messages transit through at least one relay initially, then upgrade to direct QUIC. Relays are not optional — they ARE the architecture.

### ADR-002: Bootstrap Elimination (DONE — Phase R7)
- **Before**: WebSocket signaling server (temporary)
- **Now**: Own relay (`tom-relay --dev`) + Pkarr/DNS discovery
- `TOM_RELAY_URL` env var for custom relay
- `n0_discovery(true/false)` flag for Pkarr/DNS toggle

### ADR-003: Wire Format
MessagePack envelopes (rmp-serde). `signing_bytes()` EXCLUDES `ttl` (mutated by relays).

### ADR-004: Encryption Stack (Rust)
Ed25519 signing + X25519 DH + XChaCha20-Poly1305 + HKDF-SHA256. `encrypt_and_sign()` = encrypt-then-sign.

### ADR-005: Node Identity
Ed25519 keypair = node identity. Public key is the network address (NodeId).

### ADR-006: Unified Node Model
Every node runs identical code. Role is determined by network topology, not configuration.

### ADR-009: Message Backup (Virus Metaphor)
Messages for offline recipients self-replicate across backup nodes, self-delete when delivered or after 24h TTL.

### ADR-010: Zero-config DHT Rendezvous + Resilience (Phase R12 — 2026-06-22)
- **DHT rendezvous** (`tom-dht`): a constant namespace (`tom-protocol-rendezvous-v1`) derives 8 shared ed25519 "slot" keypairs. Every node publishes `{node_id, addrs}` into slot `hash(node_id) % 8` (BEP-0044 mutable, `seq = timestamp`); any node reads all slots to discover live peers with ZERO prior knowledge — no bootstrap peer, no privileged node. Fills the BEP-0044 enumeration gap (lookup-by-known-key only).
- **Relay reachability gate** (`state.rs::relay_url_is_globally_reachable`): an embedded relay is published to the *global* gossip ONLY if its address is reachable from outside the LAN. Private/loopback/link-local/CGNAT IPs and non-routable DNS (localhost, `.local`, `.internal`) stay LAN-only.
- **Isolation recovery** (`loop.rs` reconnect_check 15s + `bootstrap.rs::on_isolated`): on loss of connectivity the phase reverts `Converged → RelayAssist` and a fresh discovery round runs (reprobe relays + DHT republish + rendezvous + rejoin).
- **Bounded FFI teardown** (`tom-protocol-ffi`): `tom_node_stop`/`tom_node_free` tear down on a detached thread so the UI Stop never blocks.
- **iOS/tvOS anti-sleep**: silent-audio keepalive now resumes after audio interruptions (`interruptionNotification` + `mediaServicesWereReset`).
- ⚠️ Open gaps tracked in **Known Limitations** below (rendezvous squatting, zombie connections, iOS suspension).

## Foundational Design Decisions (LOCKED)

**These 7 decisions are non-negotiable and define ToM's character. All code must respect them.**

See full details: `_bmad-output/planning-artifacts/design-decisions.md`

| # | Decision | Rule |
|---|----------|------|
| 1 | **Delivery** | Message delivered ⟺ recipient emits ACK |
| 2 | **TTL** | 24h max lifespan, then global purge (no exceptions) |
| 3 | **L1 Role** | L1 anchors state, never arbitrates |
| 4 | **Reputation** | Progressive fade, no permanent bans |
| 5 | **Anti-spam** | "Sprinkler gets sprinkled" — progressive load, not exclusion |
| 6 | **Invisibility** | Protocol layer invisible to end users |
| 7 | **Scope** | Universal foundation (like TCP/IP), not a product |

**Before writing code, verify:**
- No user-visible protocol state
- L1 doesn't make operational decisions
- No permanent bans or binary states
- No message persistence beyond TTL

## Core Components (Rust)

### ProtocolRuntime

Single `tokio::select!` loop, spawned as a background task:

```rust
use tom_protocol::{ProtocolRuntime, RuntimeConfig, TomNodeConfig};

let node = TomNodeConfig::new()
    .n0_discovery(false)          // disable Pkarr/DNS
    .bind().await?;

let config = RuntimeConfig {
    username: "alice".into(),
    encryption: true,
    ..Default::default()
};

let channels = ProtocolRuntime::spawn(node, config);

// Send message
channels.handle.send_message(target_id, payload).await?;

// Receive messages (already decrypted + verified)
while let Some(msg) = channels.messages.recv().await {
    println!("From {}: {:?}", msg.from, msg.payload);
}
```

### RuntimeHandle

Clonable handle for interacting with the runtime:

```rust
let handle = channels.handle.clone();
handle.send_message(target, payload).await?;
handle.add_peer(peer_addr).await?;
handle.upsert_peer(node_id, addr_info).await?;
handle.shutdown().await?;
```

### Router

Pure decision engine — returns `RoutingAction` enum:

```rust
// Router decides: Deliver / Forward / Reject / Ack / ReadReceipt / Drop
let action = router.route(envelope, &topology);
match action {
    RoutingAction::Deliver(env) => { /* local delivery */ }
    RoutingAction::Forward(env, next_hop) => { /* relay */ }
    RoutingAction::Ack(msg_id, to) => { /* send ACK */ }
    _ => {}
}
```

### GroupManager + GroupHub

Hub-and-spoke group messaging:
- **GroupManager**: member-side state machine (join, leave, receive)
- **GroupHub**: hub-side fan-out, rate limiting (5 msg/sec/sender)
- **Hub election**: deterministic (lowest NodeId)
- **Hub failover**: Primary → Shadow → Candidate cascade (active watchdog, 3s ping, ~6s promote)
- **E2E**: Sender key encryption, key rotation on member leave

### Self-Send Interception

When `hub == local_id`, the runtime intercepts self-addressed operations locally instead of sending via QUIC network. Applies to: CreateGroup, SendGroupMessage, AcceptInvite, LeaveGroup, heartbeat tick.

## Message Flow

```
Sender → Router → Envelope (encrypt+sign) → QUIC → [Relay] → QUIC → Router → Recipient
                                                       ↓
                                                 Forward only
                                                 (no storage)
                                                       ↓
                                              Verify signature
                                              Route to next hop
```

Direct upgrade: after initial relay coordination, MagicSock upgrades to direct QUIC (hole punching).

## Implementation Patterns

### File Naming (Rust)
- `snake_case.rs` for files
- `PascalCase` for types
- Co-located tests: `#[cfg(test)] mod tests` in same file

### Error Handling
```rust
use tom_protocol::error::TomProtocolError;

return Err(TomProtocolError::PeerUnreachable(node_id));
```

### Effect Pattern (RuntimeState)
RuntimeState methods return `Vec<RoutingAction>` (effects), the runtime loop executes them:
```rust
let effects = state.handle_incoming(envelope);
for effect in effects {
    execute_effect(effect, &node, &state).await;
}
```

### Critical API Notes
- `Topology.upsert()` not `update_peer()`, `Topology.get()` not `get_peer()`
- `PeerInfo.last_seen` is `u64` (timestamp ms), NOT `Instant`
- NEVER wrap TomNode in `Arc<Mutex>` — deadlocks. Use single tokio task with `select!`
- Self-addressed ops need explicit local handling, not network round-trip
- `NodeId` has no `from_bytes` — use `SecretKey::generate(rng).public().to_string().parse()`

## Testing

### Cross-Crate Validation (OBLIGATOIRE)

Tout patch Rust touchant >1 crate doit suivre cette séquence :

1. **Phase 0 — Cartographier l'impact avant de coder** :
   - Identifier : crate modifié → downstream direct → downstream indirect → job CI concerné
   - Pour discovery/relay/gossip : toujours vérifier tom-connect → tom-transport → tom-protocol → tom-stress

1. **Dev itératif — feedback rapide par crate** :

   ```bash
   cargo build -p <crate touché>
   cargo build -p <crate downstream>
   cargo test -p <crate touché> --lib --no-run
   cargo clippy -p <crate touché> -- -D warnings
   ```

1. **Validation finale AVANT push** :

   ```bash
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```

> Un patch cross-crate n'est PAS terminé tant que clippy+test workspace ne passent pas.

### Règle "commit push" — NON-NÉGOCIABLE

Quand l'utilisateur demande "commit push" (ou toute variante commit + push) :

1. **AVANT le push**, exécuter systématiquement :

   ```bash
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```

1. Si clippy ou test échoue → corriger d'abord, ne PAS push
1. Pas d'exception, même pour "juste un petit fix"

### Hygiène disque — PLAFOND NON-NÉGOCIABLE (`target/` ≤ 20 Go)

`crates/target/` a atteint **130+ Go** le 2026-07-13 (disque à 36 Go libres) après une longue session de builds/tests répétés sans jamais nettoyer — jamais laisser ce scénario se reproduire.

- **Plafond dur : `target/` ne doit JAMAIS dépasser 20 Go.** Vérifier avec `du -sh target/`.
- Dès que `target/` approche 20 Go (ou en tout cas avant toute session longue de builds répétés) : `cargo clean` ou `bash scripts/clean-cruft.sh --apply --builds`.
- `scripts/clean-cruft.sh` fait déjà tout le travail :
  ```bash
  bash scripts/clean-cruft.sh              # dry-run — cruft mort (.claude-backup-*, logs, .bak) uniquement
  bash scripts/clean-cruft.sh --apply --builds  # exécute + purge target/, .build/, node_modules, DerivedData interne au repo
  ```
- Ne PAS lancer `--builds` pendant qu'une gate/build est en cours (ça casse le build en vol) — attendre la fin du process actif.
- Penser aussi à `~/Library/Developer/Xcode/DerivedData` (hors repo, régénéré par Xcode, peut monter à plusieurs Go) et `~/Library/Caches/org.swift.swiftpm`.
- Ce n'est PAS optionnel : une session autonome longue (mode nuit, boucle roadmap continue) DOIT inclure un `cargo clean` périodique, pas seulement en fin de crise.

### Rust Tests
```bash
cargo test --workspace              # All Rust tests (~700+)
cargo test -p tom-protocol          # Protocol tests only (346)
cargo test -p tom-quinn-proto       # QUIC proto tests (322)
cargo clippy --workspace -- -D warnings  # Lint (ALWAYS before push)
```

### TypeScript Tests (legacy)
```bash
pnpm test                           # All TS tests (771)
```

### Stress Testing
```bash
# Local campaign (30 scenarios)
cargo run -p tom-stress -- campaign --local

# Remote campaign (Mac ↔ NAS)
cargo run -p tom-stress -- campaign --responder-addr <NAS_ADDR>

# Cross-compile for NAS (ARM64)
cargo zigbuild -p tom-stress --target aarch64-unknown-linux-musl --release
```

### Test Counts (à jour 2026-06-22)
- 520 tests (tom-protocol lib) + integration tests
- 20 tests (tom-dht — dont chaos churn + récupération blackout, Testnet réel)
- 322 tests (tom-quinn-proto)
- 76 tests (tom-connect)
- 22 tests (tom-quinn)
- 9 tests (tom-relay) + 12 tests (tom-gossip) + 7 tests (tom-metrics) + 2 tests (tom-base)
- 771 tests (TypeScript core, legacy)

> `tom-protocol-ffi` est **exclu du workspace** (build `--locked`) → non couvert par `cargo test/clippy --workspace`. Le valider via `bash scripts/check-ffi.sh` (ou un `git worktree` propre) AVANT tout push touchant le FFI.

## Deployment

### Own Relay (NAS)
```bash
# Freebox NAS — Debian VM, ARM64 Cortex-A72
ssh root@192.168.0.21               # Local
ssh root@82.67.95.8                 # Remote (port 22 redirect)

# tom-relay running on port 3340 (HTTP, no TLS)
/root/tom-relay --dev

# Environment variable for clients
TOM_RELAY_URL=http://192.168.0.21:3340

# Port forwarding: UDP 3340 → 82.67.95.8:3340 (public)
```

### Cross-compile ARM64
```bash
cargo zigbuild -p tom-stress --target aarch64-unknown-linux-musl --release
# SCP binary while process is running → "dest open Failure" — kill first
```

### n0_discovery Flag
```rust
// With N0 preset (Pkarr/DNS) — default
TomNodeConfig::new().n0_discovery(true).bind().await?;

// Without N0 (own relay only, no external deps)
TomNodeConfig::new().n0_discovery(false).bind().await?;
```

## Current Status

### TypeScript (Phase 1 — Complete)

| Epic | Description | Status |
|------|-------------|--------|
| 1-8 | Full protocol stack | ✅ Complete (771 tests) |

### Rust Native (Phase 2 — Active)

| Phase | Description | Status |
|-------|-------------|--------|
| R1 | Foundations (envelope, crypto, types) | ✅ Complete |
| R2 | Routing + ProtocolRuntime | ✅ Complete |
| R3 | Discovery + Keepalive (gossip) | ✅ Complete |
| R4 | Backup + Roles | ✅ Complete |
| R5 | Groups (hub, failover, sender keys, security) | ✅ Complete |
| R6 | TUI + Integration + Stress campaigns | ✅ Complete |
| R7 | Fork + Bootstrap Elimination | ✅ Complete |
| R8 | Production Hardening | ✅ Complete |
| R9 | Consolidation (DHT, delivery reliability) | ✅ Complete |
| R10 | Group Recovery (rejoin, tracker persistence, liveness reset) | ✅ Complete |
| R11 | Security & Admin (antispam, nonce anti-replay, group admin controls) | ✅ Complete |
| R12 | Zero-config DHT rendezvous + resilience (isolation recovery, anti-sleep, bounded stop) | 🚧 Livré, trous ouverts (voir Known Limitations) |

### Stress Test Results
- Campaign V5: 250/250 Mac ↔ NAS (100% success)
- Campaign self-send fix: 232/232 Mac ↔ NAS (SSH tunnel)
- PoC hole punch: 100% across LAN/4G CGNAT/cross-border

## Known Limitations (audit adversarial 2026-06-22, réaudité file:line 2026-07-12)

Trous historiques de l'audit multi-agent. **Réaudit 2026-07-12 : les 3 items "Critique" et 2 des 4 items "Haut risque" étaient en fait déjà corrigés dans le code — seule cette liste n'avait pas été mise à jour.** Ne pas reproposer ces fixes comme neufs ; vérifier file:line avant de croire cette liste sur parole (elle a déjà menti une fois).

**Critique (Phase 1) — TOUS RÉSOLUS (vérifiés 2026-07-12) :**
1. ✅ **Connexions zombies** — RÉSOLU. `runtime/loop.rs:591-607` : `zombie = !connected.is_empty() && liveness_is_stale(last_inbound_at, now_ms(), LIVENESS_STALE_MS)` ; `if connected.is_empty() || zombie { bootstrap_phase.on_isolated() ... }` — un pair "connecté" mais silencieux déclenche bien la redécouverte, pas seulement une liste de connexions vide. Testé (`liveness_fresh_within_window`/`liveness_stale_past_window`/`liveness_future_inbound_never_stale`).
2. ✅ **Rendez-vous DHT squattable** — RÉSOLU pour la preuve-de-possession. `runtime/loop.rs:862-877` `rendezvous_entry_authentic()` vérifie une signature Ed25519 liée au `node_id` avant d'accepter toute entrée découverte (`loop.rs:908` : `found.into_iter().filter(rendezvous_entry_authentic)`) — un attaquant ne peut plus injecter de faux pairs. Testé (entrée non signée / falsifiée / node_id usurpé → rejetés). **Résiduel mineur** : `RENDEZVOUS_SLOTS` (`tom-dht/src/lib.rs:42`) toujours à **8**, pas augmenté — collisions de slots entre pairs légitimes possibles à grande échelle, mais ce n'est plus une faille de sécurité (juste un facteur de bruit), pas prioritaire.
3. ✅ **FFI double-teardown** — RÉSOLU. `tom-protocol-ffi/src/lib.rs:435-454` : `tom_node_stop`/`tom_node_free` convergent vers un unique `detached_teardown()` (contrat documenté en commentaire : "single teardown path"), différenciés uniquement par le flag `graceful`. Le wrapper Swift met le pointeur à `nil` après consommation (pas de second appel possible côté appelant).

**Haut risque (Phase 2) :**
4. 🟢 **Suspension iOS/tvOS — NON-OBJECTIF ASSUMÉ (décision produit 2026-07-15).** RÉCUPÉRATION en place : anti-veille audio résilient aux interruptions (resume) + scenePhase observer → au retour foreground restart complet (`forceReset`+`start` → re-découverte incl. rendez-vous) + le runtime se ré-amorce sur connexion zombie (#1, 45s). Résiduel inhérent à iOS : pendant une **vraie suspension** (app en arrière-plan), le process est gelé. **On ne fait PAS de background daemon.** Le modèle voulu : l'utilisateur se sert du réseau → au premier plan le nœud contribue pleinement ; en arrière-plan on tolère au plus quelques minutes de sursis (surtout sur mobile), pas plus. Pas de push APNs/VoIP, pas de wakeup permanent : ce serait contraire à la vision (couche invisible, pas un service toujours-actif énergivore). Filet anti-perte de message pendant la suspension : backup TTL 24h (le réseau retient le message pour le récepteur offline). **Ce n'est donc pas un trou à combler, c'est un choix.**
5. ✅ **Livraison (décision #1)** — RÉSOLU (vérifié 2026-07-11, file:line). ACK signé à l'émission (`state.rs:835/846/871`) ET vérifié à la réception : un ACK non signé/forgé est rejeté (`state.rs:898`) et les deux seules opérations qui accordent la confiance (`mark_relayed` 927, `mark_delivered` 935) sont à l'intérieur de ce gate ; anti-pumping FINDING #7 en bonus (commit `c3b7f9a`). Backup TTL (décision #2) : clampé à 24h à la création (`backup/types.rs:86` `.min(MAX_TTL_MS)` → le SQLite ne PEUT pas dépasser), purge câblée `tick_backup → cleanup_expired` (`backup/coordinator.rs:280`).
6. ✅ **Adresses directes DHT** — RÉSOLU par un filtrage délibérément DIFFÉRENT de `relay_url_is_globally_reachable` (pas "unifié" — unifier aurait cassé la découverte LAN). `loop.rs:794/812-821` `direct_addr_is_dialable()` rejette loopback/unspecified/link-local/broadcast/multicast/documentation mais GARDE les plages privées (nécessaire pour le same-LAN via DHT, commentaire explicite ligne 793). Une adresse candidate n'accorde aucune confiance protocolaire — au pire un dial gaspillé, pas une faille.
7. ✅ **Nonce anti-replay** — RÉSOLU (R11.2, déjà marqué complet dans Current Status). `router.rs:37` `NONCE_TTL = 24h` + `router.rs:29-31` `MAX_NONCE_CACHE=50_000` (LRU bornée) + purge temporelle `router.rs:197-205`. Testé (`nonce_replay_detected`).

**Conception (Phase 3) :**
8. ✅ **ProtocolEvent fuite d'état interne** — VÉRIFIÉ non-exposé : `RolePromoted`/`RoleDemoted`/`BackupStored`/`SenderThrottled` ne traversent NI le FFI NI Swift (le Live Log des apps vient du logging Swift, pas d'eux). Ce sont des events d'observabilité interne (tom-tui). La frontière end-user (#6) est donc propre. Conservés pour le debug ; à filtrer dans un futur produit end-user, pas un bug protocole.
9. 🔻 **Détection relay offline** — atténué par #1 (liveness 45s) + reprobe 15s (était ~105s). Un ping relais dédié 30s nécessiterait une API dans le transport forké (tom-connect) — reporté (gain marginal vs risque).
10. ✅ **Meshing** — VÉRIFIÉ par-design : un mesh gossip est un graphe *connexe* (via hubs), pas *complet* ; forcer un full-mesh N² chargerait les appareils faibles (Apple TV). La connectivité est maximisée par le rendez-vous (#2) + le rejoin 15s. Pas de dial-direct forcé (contre-productif).

**MAJ 2026-07-19 — deux entrées à ajouter à cette liste :**

11. ✅ **Rétention mémoire sans budget en octets — RÉSOLU (builds 126/127), mais classe de bug à
    surveiller.** Le backup store était borné en NOMBRE de messages (`MAX_TOTAL_MESSAGES=10_000`)
    et jamais en volume, et `pending_envelopes` (cache de réémission, `runtime/state.rs`) n'était
    borné du tout alors qu'il clone l'enveloppe complète, payload inclus. Terrain : relais NAS à
    688 Mo sur une VM de 920, OOM-killer à répétition, 0 pair, 8 366 échecs — **tout en affichant
    `phase: "connecte"` et vu « DIRECT v6 7 ms » par ses pairs**. Corrigé par `MAX_TOTAL_BYTES`
    (64 Mio, `backup/store.rs`) et `PendingEnvelopes` (32 Mio, `runtime/pending.rs`), chacun avec
    un point de mutation unique pour que le compteur ne puisse pas dériver. **4ᵉ occurrence de la
    même classe** (large-message, reassembly, backup, pending) → devant toute structure qui garde
    un `Vec<u8>` réseau, chercher le budget en OCTETS, pas en nombre d'entrées.
    Voir mémoire `tom-memory-retention-class-of-bug` et `docs/plans/fix-backup-store-budget-octets.md`.
12. 🔻 **Pic mémoire transitoire en charge — connu, différé (décision Malik 19/07).** 300 Mo poussés
    d'un coup → `RssAnon` 491 Mo, puis retour à 123 Mo stable. Ce n'est PAS une rétention (ça se
    libère), c'est le coût du trafic en vol non régulé ; mais c'est élevé pour une VM de 920 Mo.
    Piste si ça revient : réguler le débit d'émission, ou borner `send_window` QUIC (jamais tuné
    dans ToM). **À traiter APRÈS R14 et R15.**

**Ce qui reste RÉELLEMENT ouvert après ce réaudit** : le ping relais dédié (#9, explicitement déprioritisé) et le nombre de slots DHT (#2, mineur). Le résiduel iOS (#4) n'est plus « ouvert » : APNs/background est un **non-objectif assumé** (décision produit 2026-07-15), pas un chantier. Aucun trou "Critique" ouvert à cette date.

## Important Notes for LLMs

1. **Fork is complete**: All critical iroh deps forked to `tom-*` namespace (Phase R7)
2. **Relays don't store**: Pass-through only, no persistence
3. **E2E is mandatory**: All messages encrypted (XChaCha20-Poly1305)
4. **Roles are network-assigned**: Nodes don't choose to be relays
5. **No blockchain**: This is a transport protocol, not a ledger
6. **Contribution matters**: Usage/contribution score affects role assignment
7. **Wire invariants are sacred**: NE PAS changer les identifiants `tom-*` (préfixe DNS `_tom`, SNI `.tom.invalid`, en-têtes `X-Tom-*`, ALPN `tom-protocol/transport/0` et `/tom-gossip/1`) entre versions ToM. ⚠️ ToM n'est PAS compatible iroh (voir « Wire Invariants »).
8. **ed25519-dalek pin**: MUST use `=3.0.0-pre.1` (crypto type compat with quinn)
9. **Signaling server is DEPRECATED**: Use own relay + Pkarr/DNS
10. **FFI validation before push**: `tom-protocol-ffi` is excluded from the workspace (built `--locked`). `cargo test/clippy --workspace` does NOT cover it. Run `bash scripts/check-ffi.sh` — or build from a clean `git worktree` at HEAD — before any push touching the FFI/cross-crate. The working copy masks committed-state compile errors.
11. **Zero-config discovery**: nodes find each other via the shared DHT rendezvous (ADR-010), no privileged node. Anti-sleep / iOS suspension caveats apply (Known Limitations).

## Quick Commands

```bash
# Rust development
cargo build --workspace            # Build all
cargo test --workspace             # Test all
cargo clippy --workspace -- -D warnings  # Lint

# Run TUI chat
cargo run -p tom-tui -- --username alice

# Run stress test
cargo run -p tom-stress -- campaign --local

# Run relay (dev mode)
cargo run -p tom-relay -- --dev

# Cross-compile for ARM64
cargo zigbuild -p tom-stress --target aarch64-unknown-linux-musl --release

# TypeScript (legacy)
pnpm install && pnpm build && pnpm test
```
