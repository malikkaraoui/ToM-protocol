# Relay Registry — Consommation passive des RelayReadyAnnounce

**Date**: 2026-03-20
**Statut**: Validé
**Prérequis**: Phase R16 relay publication (commit c2d9fe6)

## Objectif

Registry local des relays publiés via `RelayReadyAnnounce`. Réception, stockage, expiration, observation — **aucune décision de routing, aucune auto-sélection, aucune migration**.

## Principe clé : fraîcheur d'entrée ≠ liveness du registry

- `is_fresh()` = barrière d'acceptation d'une annonce entrante (1h de tolérance passée, 5 min futur)
- TTL registry = durée de vie opérationnelle de l'entrée stockée localement (défaut 10 min)
- Une annonce de 25 min peut passer `is_fresh()` mais l'entrée registry expire si non rafraîchie dans les 10 min
- L'expiration se base uniquement sur l'horloge locale (`refreshed_at`), jamais sur le timestamp distant

## Data Model

```rust
// discovery/relay_registry.rs

pub const DEFAULT_RELAY_REGISTRY_TTL_MS: u64 = 10 * 60 * 1000; // 10 min

pub struct RelayRegistryEntry {
    pub node_id: NodeId,
    pub relay_url: RelayUrl,
    pub announced_at: u64,    // timestamp du RelayReadyAnnounce (horloge distante, observabilité)
    pub refreshed_at: u64,    // now_ms() local à la réception
    pub expires_at: u64,      // refreshed_at + ttl_ms
}

pub struct RelayRegistry {
    entries: HashMap<NodeId, RelayRegistryEntry>,
    ttl_ms: u64,
}
```

### Invariants

- **Clé = NodeId** : un node = un relay max. Republication avec URL différente = overwrite total.
- `announced_at` stocké pour observabilité, **jamais utilisé pour l'expiration**.
- `expires_at` calculé uniquement dans `upsert()`, jamais bricolé ailleurs.

## API

```rust
impl RelayRegistry {
    pub fn new(ttl_ms: u64) -> Self
    pub fn upsert(&mut self, node_id: NodeId, relay_url: RelayUrl, announced_at: u64, now: u64) -> bool
    pub fn prune(&mut self, now: u64) -> Vec<RelayRegistryEntry>
    pub fn get(&self, node_id: &NodeId) -> Option<&RelayRegistryEntry>
    pub fn all(&self) -> impl Iterator<Item = &RelayRegistryEntry>
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
}

impl Default for RelayRegistry {
    fn default() -> Self { Self::new(DEFAULT_RELAY_REGISTRY_TTL_MS) }
}
```

### Choix API

- `upsert()` prend des valeurs décomposées (pas le `RelayReadyAnnounce` entier) — le registry ne connaît pas la crypto
- `upsert()` retourne `bool` : `true` = nouvelle entrée, `false` = refresh/overwrite
- `prune()` retourne `Vec<RelayRegistryEntry>` (entrées complètes) pour que RuntimeState puisse émettre `RelayRegistryExpired { node_id, relay_url }` sans gymnastics
- `all()` = snapshot brut du storage. Contrat : prune avant lecture via tick runtime.

## Intégration RuntimeState

### Config

```rust
// RuntimeConfig
pub relay_registry_ttl: Duration,  // défaut 10 min
```

Conversion unique dans `RuntimeState::new()` : `config.relay_registry_ttl.as_millis() as u64`

### Champ

```rust
// RuntimeState
pub(crate) relay_registry: RelayRegistry,
```

### handle_relay_ready_announce() modifié

```
1. ignore self           (existant)
2. verify signature      (existant)
3. check is_fresh()      (existant)
4. relay_registry.upsert(node_id, relay_url, announced_at, now)  ← NOUVEAU
5. émettre RelayReadyReceived   (existant)
```

### Prune

Branché sur le tick `heartbeat_interval` (5s) existant. Pas de nouveau timer.

```
tick_heartbeat():
    ... heartbeat normal ...
    let expired = relay_registry.prune(now_ms());
    for entry in expired {
        emit RelayRegistryExpired { node_id: entry.node_id, relay_url: entry.relay_url }
    }
```

### RuntimeHandle (read-only)

```rust
pub async fn get_known_relays(&self) -> Vec<RelayRegistryEntry>
```

Via `RuntimeCommand::GetKnownRelays { reply: oneshot::Sender<Vec<RelayRegistryEntry>> }`.

Retourne trié par `refreshed_at` décroissant.

## Events

| Event | Rôle | Quand |
|-------|------|-------|
| `RelayReadyReceived { node_id, relay_url }` | Existant, inchangé | Annonce reçue et validée |
| `RelayRegistryExpired { node_id, relay_url }` | Nouveau | Entrée purgée par TTL |

Pas de `RelayRegistryUpdated` — `RelayReadyReceived` couvre déjà ce rôle.

## Placement

`discovery/relay_registry.rs` — même pattern que `HeartbeatTracker` et `EphemeralSubnetManager`.

## Ce qui est hors scope

- Auto-sélection de relay
- Migration de relay
- Routing basé sur le registry
- Persistence du registry (éphémère, reconstruit via gossip)

## Fichiers impactés

| Fichier | Modification |
|---------|-------------|
| `discovery/relay_registry.rs` | **NOUVEAU** — struct + API + tests unitaires |
| `discovery/mod.rs` | Export du module |
| `runtime/mod.rs` | `RuntimeConfig` + `RuntimeCommand::GetKnownRelays` + `ProtocolEvent::RelayRegistryExpired` + `RuntimeHandle::get_known_relays()` |
| `runtime/state.rs` | Champ `relay_registry`, init, `handle_relay_ready_announce()` modifié, prune dans tick |

---

## Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Passive relay registry consuming validated RelayReadyAnnounce via gossip — store, expire, observe, no routing.

**Architecture:** New `RelayRegistry` struct in `discovery/relay_registry.rs`, owned by `RuntimeState`. Upsert on validated announce, prune on heartbeat tick, read-only query via `RuntimeHandle`.

**Tech Stack:** Rust, `HashMap<NodeId, RelayRegistryEntry>`, `tom_connect::RelayUrl`, existing effect pattern.

---

### Task 1: RelayRegistry struct + new/default + upsert — tests first

**Files:**
- Create: `crates/tom-protocol/src/discovery/relay_registry.rs`

**Step 1: Write the failing tests for new + upsert**

```rust
//! Relay registry — local store of discovered relay-ready peers.
//!
//! Consumes validated RelayReadyAnnounce data. No crypto, no validation —
//! that's the caller's job. This is pure storage + TTL expiration.

use std::collections::HashMap;
use tom_connect::RelayUrl;
use crate::types::NodeId;

pub const DEFAULT_RELAY_REGISTRY_TTL_MS: u64 = 10 * 60 * 1000; // 10 min

#[derive(Debug, Clone)]
pub struct RelayRegistryEntry {
    pub node_id: NodeId,
    pub relay_url: RelayUrl,
    pub announced_at: u64,
    pub refreshed_at: u64,
    pub expires_at: u64,
}

pub struct RelayRegistry {
    entries: HashMap<NodeId, RelayRegistryEntry>,
    ttl_ms: u64,
}

impl RelayRegistry {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl_ms,
        }
    }

    /// Upsert a relay entry. Returns true if this is a new entry (not a refresh).
    pub fn upsert(
        &mut self,
        node_id: NodeId,
        relay_url: RelayUrl,
        announced_at: u64,
        now: u64,
    ) -> bool {
        todo!()
    }
}

impl Default for RelayRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_RELAY_REGISTRY_TTL_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_url() -> RelayUrl {
        "http://127.0.0.1:3340".parse().unwrap()
    }

    fn test_node_id(seed: u64) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn new_registry_is_empty() {
        let reg = RelayRegistry::new(600_000);
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn upsert_new_entry_returns_true() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        let url = test_url();
        let is_new = reg.upsert(id, url, 1000, 2000);
        assert!(is_new);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn upsert_refresh_returns_false() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        let url = test_url();
        assert!(reg.upsert(id, url.clone(), 1000, 2000));
        let is_new = reg.upsert(id, url, 3000, 4000);
        assert!(!is_new, "refresh should return false");
    }

    #[test]
    fn upsert_overwrites_url() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        let url1 = test_url();
        let url2: RelayUrl = "http://10.0.0.1:4444".parse().unwrap();
        reg.upsert(id, url1, 1000, 2000);
        reg.upsert(id, url2.clone(), 3000, 4000);
        let entry = reg.get(&id).unwrap();
        assert_eq!(entry.relay_url, url2);
        assert_eq!(entry.refreshed_at, 4000);
    }

    #[test]
    fn upsert_sets_expires_at() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        reg.upsert(id, test_url(), 1000, 2000);
        let entry = reg.get(&id).unwrap();
        assert_eq!(entry.expires_at, 2000 + 600_000);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tom-protocol --lib relay_registry -- --no-run 2>&1 | head -20`
Expected: Compiles but `todo!()` will panic at runtime.

**Step 3: Implement upsert + get + len + is_empty + all**

Replace the `todo!()` in `upsert` with:

```rust
    pub fn upsert(
        &mut self,
        node_id: NodeId,
        relay_url: RelayUrl,
        announced_at: u64,
        now: u64,
    ) -> bool {
        let is_new = !self.entries.contains_key(&node_id);
        self.entries.insert(
            node_id,
            RelayRegistryEntry {
                node_id,
                relay_url,
                announced_at,
                refreshed_at: now,
                expires_at: now + self.ttl_ms,
            },
        );
        is_new
    }

    pub fn get(&self, node_id: &NodeId) -> Option<&RelayRegistryEntry> {
        self.entries.get(node_id)
    }

    pub fn all(&self) -> impl Iterator<Item = &RelayRegistryEntry> {
        self.entries.values()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p tom-protocol --lib relay_registry`
Expected: 5 tests PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/discovery/relay_registry.rs
git commit -m "feat(discovery): add RelayRegistry struct with upsert + tests"
```

---

### Task 2: RelayRegistry prune + tests

**Files:**
- Modify: `crates/tom-protocol/src/discovery/relay_registry.rs`

**Step 1: Write failing tests for prune**

Add to the `tests` module:

```rust
    #[test]
    fn prune_removes_expired_entries() {
        let mut reg = RelayRegistry::new(1000); // 1s TTL
        let id1 = test_node_id(1);
        let id2 = test_node_id(2);
        reg.upsert(id1, test_url(), 100, 100);
        reg.upsert(id2, "http://10.0.0.1:5555".parse().unwrap(), 200, 200);

        // At t=1200, id1 expired (100+1000=1100 < 1200), id2 still alive (200+1000=1200 == 1200)
        let expired = reg.prune(1200);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].node_id, id1);
        assert_eq!(reg.len(), 1);
        assert!(reg.get(&id2).is_some());
    }

    #[test]
    fn prune_returns_empty_when_nothing_expired() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        reg.upsert(id, test_url(), 1000, 1000);
        let expired = reg.prune(2000);
        assert!(expired.is_empty());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn prune_returns_full_entries() {
        let mut reg = RelayRegistry::new(100);
        let id = test_node_id(1);
        let url = test_url();
        reg.upsert(id, url.clone(), 50, 50);
        let expired = reg.prune(200);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].relay_url, url);
        assert_eq!(expired[0].announced_at, 50);
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tom-protocol --lib relay_registry -- --no-run`
Expected: Compile error — `prune` not defined yet.

**Step 3: Implement prune**

Add to `impl RelayRegistry`:

```rust
    /// Remove expired entries. Returns the removed entries (for event emission).
    pub fn prune(&mut self, now: u64) -> Vec<RelayRegistryEntry> {
        let mut expired = Vec::new();
        self.entries.retain(|_, entry| {
            if entry.expires_at < now {
                expired.push(entry.clone());
                false
            } else {
                true
            }
        });
        expired
    }
```

**Step 4: Run tests**

Run: `cargo test -p tom-protocol --lib relay_registry`
Expected: 8 tests PASS

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/discovery/relay_registry.rs
git commit -m "feat(discovery): add RelayRegistry.prune() with TTL expiration"
```

---

### Task 3: Export RelayRegistry from discovery module

**Files:**
- Modify: `crates/tom-protocol/src/discovery/mod.rs`

**Step 1: Add module + re-export**

Add after `pub mod relay_announce;`:

```rust
pub mod relay_registry;
```

Add to the re-exports:

```rust
pub use relay_registry::{RelayRegistry, RelayRegistryEntry, DEFAULT_RELAY_REGISTRY_TTL_MS};
```

**Step 2: Verify it compiles**

Run: `cargo build -p tom-protocol`
Expected: OK

**Step 3: Commit**

```bash
git add crates/tom-protocol/src/discovery/mod.rs
git commit -m "feat(discovery): export RelayRegistry module"
```

---

### Task 4: RuntimeConfig + ProtocolEvent + RuntimeCommand

**Files:**
- Modify: `crates/tom-protocol/src/runtime/mod.rs`

**Step 1: Add `relay_registry_ttl` to RuntimeConfig**

In `RuntimeConfig` struct, after `enable_embedded_relay_publication`:

```rust
    /// TTL for relay registry entries (how long a discovered relay stays valid without refresh).
    pub relay_registry_ttl: Duration,
```

In `Default for RuntimeConfig`, after `enable_embedded_relay_publication: false`:

```rust
            relay_registry_ttl: Duration::from_secs(600), // 10 min
```

**Step 2: Add `RelayRegistryExpired` to ProtocolEvent**

After the `RelayReadyReceived` variant:

```rust
    /// A relay registry entry expired (no refresh within TTL).
    RelayRegistryExpired {
        node_id: NodeId,
        relay_url: RelayUrl,
    },
```

**Step 3: Add `GetKnownRelays` to RuntimeCommand**

After `EmbeddedRelayStopped`:

```rust
    /// Query the relay registry (read-only snapshot).
    GetKnownRelays {
        reply: oneshot::Sender<Vec<crate::discovery::RelayRegistryEntry>>,
    },
```

**Step 4: Add `get_known_relays` to RuntimeHandle**

Add method to `impl RuntimeHandle`:

```rust
    /// Query all known relays from the registry (read-only snapshot, sorted by freshest first).
    pub async fn get_known_relays(&self) -> Vec<crate::discovery::RelayRegistryEntry> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(RuntimeCommand::GetKnownRelays { reply: tx }).await;
        rx.await.unwrap_or_default()
    }
```

**Step 5: Verify it compiles**

Run: `cargo build -p tom-protocol`
Expected: May warn about unused variant — that's fine, wiring comes in Task 5.

**Step 6: Commit**

```bash
git add crates/tom-protocol/src/runtime/mod.rs
git commit -m "feat(runtime): add relay registry config, events, and command"
```

---

### Task 5: Wire RelayRegistry into RuntimeState

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Step 1: Add field to RuntimeState**

After the `embedded_relay_publication` field (~line 107):

```rust
    // Phase R16: Relay registry (passive consumption of RelayReadyAnnounce)
    pub(crate) relay_registry: crate::discovery::RelayRegistry,
```

**Step 2: Initialize in RuntimeState::new()**

After the `embedded_relay_publication: super::EmbeddedRelayPublicationState::NotPublished,` line in the `Self { ... }` block:

```rust
            relay_registry: crate::discovery::RelayRegistry::new(
                config.relay_registry_ttl.as_millis() as u64,
            ),
```

**Step 3: Add import for `now_ms` if not already present**

Already imported at line 14: `use crate::types::{now_ms, ...}` — no change needed.

**Step 4: Modify `handle_relay_ready_announce()`**

In `handle_relay_ready_announce()` (~line 675), before the `RelayReadyReceived` emit, add the upsert:

```rust
        // Store in registry
        self.relay_registry.upsert(
            announce.node_id,
            announce.relay_url.clone(),
            announce.timestamp,
            now_ms(),
        );
```

**Step 5: Add prune to `tick_heartbeat()`**

At the end of `tick_heartbeat()`, before the final `effects` return (~line 403):

```rust
        // Prune expired relay registry entries
        let expired_relays = self.relay_registry.prune(now_ms());
        for entry in expired_relays {
            effects.push(RuntimeEffect::Emit(ProtocolEvent::RelayRegistryExpired {
                node_id: entry.node_id,
                relay_url: entry.relay_url,
            }));
        }
```

**Step 6: Handle `GetKnownRelays` in `handle_command()`**

Add a match arm in `handle_command()`, after the `GetAllRoleScores` arm:

```rust
            RuntimeCommand::GetKnownRelays { reply } => {
                let mut relays: Vec<_> = self.relay_registry.all().cloned().collect();
                relays.sort_by(|a, b| b.refreshed_at.cmp(&a.refreshed_at));
                let _ = reply.send(relays);
                Vec::new()
            }
```

**Step 7: Verify it compiles**

Run: `cargo build -p tom-protocol`
Expected: OK

**Step 8: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "feat(runtime): wire RelayRegistry into RuntimeState"
```

---

### Task 6: Integration tests in state.rs

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs` (test module)

**Step 1: Write tests**

Add to the existing `#[cfg(test)] mod tests` at the bottom:

```rust
    // ── Relay Registry integration tests ────────────────────────────────

    #[test]
    fn relay_ready_announce_stores_in_registry() {
        let mut state = default_state(60);
        let (other_id, other_seed) = keypair(100);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:3340".parse().unwrap();
        let now = now_ms();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id, url.clone(), now, &other_seed,
        );

        let effects = state.handle_relay_ready_announce(announce);

        // Should emit RelayReadyReceived
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0],
            RuntimeEffect::Emit(ProtocolEvent::RelayReadyReceived { node_id, relay_url })
            if *node_id == other_id && relay_url == &url));

        // Should be in registry
        let entry = state.relay_registry.get(&other_id).expect("should be in registry");
        assert_eq!(entry.relay_url, url);
        assert_eq!(entry.announced_at, now);
    }

    #[test]
    fn relay_registry_prune_via_tick_heartbeat() {
        let mut state = default_state(61);
        let (other_id, other_seed) = keypair(101);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:3340".parse().unwrap();

        // Insert with TTL=0 so it expires immediately
        state.relay_registry = crate::discovery::RelayRegistry::new(0);
        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id, url.clone(), now_ms(), &other_seed,
        );
        state.handle_relay_ready_announce(announce);
        assert_eq!(state.relay_registry.len(), 1);

        // Tick heartbeat should prune the expired entry
        let effects = state.tick_heartbeat();
        assert!(state.relay_registry.is_empty());

        let has_expired = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::RelayRegistryExpired {
                node_id, relay_url
            }) if *node_id == other_id && relay_url == &url)
        });
        assert!(has_expired, "should emit RelayRegistryExpired");
    }

    #[test]
    fn get_known_relays_sorted_by_freshest() {
        let mut state = default_state(62);
        let (id1, seed1) = keypair(201);
        let (id2, seed2) = keypair(202);
        let url1: tom_connect::RelayUrl = "http://10.0.0.1:3340".parse().unwrap();
        let url2: tom_connect::RelayUrl = "http://10.0.0.2:3340".parse().unwrap();

        // Insert id1 first (older), then id2 (newer)
        let a1 = crate::discovery::RelayReadyAnnounce::new(id1, url1, 1000, &seed1);
        let a2 = crate::discovery::RelayReadyAnnounce::new(id2, url2, 2000, &seed2);
        state.handle_relay_ready_announce(a1);
        state.handle_relay_ready_announce(a2);

        // Query via handle_command
        let (tx, rx) = tokio::sync::oneshot::channel();
        state.handle_command(super::RuntimeCommand::GetKnownRelays { reply: tx });
        let relays = rx.try_recv().expect("should receive");
        assert_eq!(relays.len(), 2);
        // id2 should be first (more recent refreshed_at)
        assert_eq!(relays[0].node_id, id2);
        assert_eq!(relays[1].node_id, id1);
    }
```

**Step 2: Run tests**

Run: `cargo test -p tom-protocol --lib relay_registry && cargo test -p tom-protocol --lib relay_ready_announce`
Then: `cargo test -p tom-protocol --lib "get_known_relays_sorted\|relay_registry_prune_via\|relay_ready_announce_stores"`

**Step 3: Verify all pass**

Expected: All tests PASS

**Step 4: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "test(runtime): add relay registry integration tests"
```

---

### Task 7: Clippy + full workspace validation

**Files:** None (validation only)

**Step 1: Clippy workspace**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Clean

**Step 2: Full test suite**

Run: `cargo test --workspace`
Expected: ~1030+ tests PASS, 0 failures

**Step 3: Final commit (squash or leave as-is)**

If all clean, no action needed. If clippy fixes required, fix and commit.

---

### Task 8: Push

Run: `git push`
