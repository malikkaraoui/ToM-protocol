# Transport Relay Discovery — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Consommer les RelayReadyAnnounce du registry pour enrichir dynamiquement les relays transport (tom-connect Endpoint), sans toucher au routage applicatif (Topology/RelaySelector).

**Architecture:** RuntimeState produit des effets `InsertTransportRelay` / `RemoveTransportRelay`. La loop les intercepte, appelle `node.insert_relay()` / `node.remove_relay()`, et maintient un `HashSet<RelayUrl>` local pour distinguer relays découverts vs statiques. Activable via `enable_transport_relay_discovery: bool` (défaut `false`).

**Tech Stack:** Rust, tom-connect `Endpoint::insert_relay/remove_relay`, tom-relay `RelayConfig`, effect pattern existant.

---

## Invariants NON-NÉGOCIABLES

1. **RuntimeState ne stocke pas l'état opérationnel transport** — le `HashSet<RelayUrl>` des relays dynamiques vit dans la loop
2. **RelayRegistry ne pilote pas Topology** — aucune mutation de `PeerRole`, `PeerInfo`, `PeerStatus`
3. **RelayRegistry ne pilote pas RelaySelector** — aucun impact sur `select_best`, `select_path`
4. **Une URL statique n'est jamais retirée** par expiration discovery
5. **Une URL partagée par plusieurs entrées actives** n'est jamais retirée prématurément
6. **Un overwrite old_url → new_url** ne laisse pas d'ancienne URL orpheline si plus référencée
7. **Aucun insert/remove transport si `enable_transport_relay_discovery == false`**
8. **Relays découverts dynamiquement → `quic: None`** — le signal discovery ne porte pas la capacité QUIC. QUIC activé uniquement pour les relays configurés statiquement ou un futur payload discovery qui l'annonce explicitement.

---

## A. Structures et états

### A1. `UpsertResult` enum (relay_registry.rs)

Remplace le `bool` actuel de `upsert()` :

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertResult {
    /// Nouvelle entrée insérée (node_id inconnu avant).
    Inserted,
    /// Entrée existante rafraîchie, même URL.
    RefreshedSameUrl,
    /// Entrée existante mise à jour avec une nouvelle URL.
    UpdatedUrl {
        old_url: RelayUrl,
    },
}

impl UpsertResult {
    pub fn is_new(&self) -> bool {
        matches!(self, UpsertResult::Inserted)
    }
}
```

### A2. Helper `has_active_url()` (relay_registry.rs)

Pour vérifier avant remove si l'URL est encore référencée :

```rust
/// Returns true if at least one active entry points to this URL.
pub fn has_active_url(&self, url: &RelayUrl) -> bool {
    self.entries.values().any(|e| &e.relay_url == url)
}
```

### A3. `discovered_transport_relays: HashSet<RelayUrl>` (loop.rs)

État local dans la loop, à côté de `embedded_relay` et `gossip_sender` :

```rust
// Transport relay discovery state (NOT in RuntimeState — pure transport concern)
let mut discovered_transport_relays: HashSet<RelayUrl> = HashSet::new();
```

**Invariant** : contient uniquement les URLs insérées via discovery. Les URLs statiques (boot, TOM_RELAY_URL, fallback) n'y sont jamais.

---

## B. Nouvelles signatures API

### B1. `RelayRegistry::upsert()` — breaking change

```rust
pub fn upsert(
    &mut self,
    node_id: NodeId,
    relay_url: RelayUrl,
    announced_at: u64,
    now: u64,
) -> UpsertResult
```

### B2. `RelayRegistry::has_active_url()`

```rust
pub fn has_active_url(&self, url: &RelayUrl) -> bool
```

### B3. `TomNode::insert_relay()` et `TomNode::remove_relay()` (tom-transport, cross-crate)

Le champ `endpoint` de TomNode est privé. Il faut exposer des wrappers publics :

```rust
// Dans crates/tom-transport/src/node.rs
use tom_connect::RelayUrl;
use tom_relay::RelayConfig;
use std::sync::Arc;

/// Insert a relay into the transport layer's relay map.
/// Returns the previous config if the URL already existed.
pub async fn insert_relay(
    &self,
    relay: RelayUrl,
    config: Arc<RelayConfig>,
) -> Option<Arc<RelayConfig>> {
    self.endpoint.insert_relay(relay, config).await
}

/// Remove a relay from the transport layer's relay map.
/// Returns the removed config if it existed.
pub async fn remove_relay(&self, relay: &RelayUrl) -> Option<Arc<RelayConfig>> {
    self.endpoint.remove_relay(relay).await
}
```

### B4. `RuntimeEffect` — 2 nouvelles variantes (effect.rs)

```rust
/// Insert a discovered relay into the transport layer.
/// Handled by loop interceptor (needs node access).
InsertTransportRelay {
    relay_url: RelayUrl,
},

/// Remove a discovered relay from the transport layer.
/// Only if the URL is no longer referenced by any active registry entry
/// and was inserted via discovery (not static).
/// Handled by loop interceptor (needs node access).
RemoveTransportRelay {
    relay_url: RelayUrl,
},
```

### B5. `RuntimeConfig` — nouveau flag

```rust
/// Enable dynamic injection of discovered relay URLs into the transport layer.
/// When false (default), RelayRegistry observes but does not mutate the Endpoint.
pub enable_transport_relay_discovery: bool,
// Default: false
```

### B6. `ProtocolEvent` — 2 nouveaux variants (optionnels, pour observabilité)

```rust
/// A discovered relay was injected into the transport layer.
TransportRelayInserted {
    relay_url: RelayUrl,
},

/// A discovered relay was removed from the transport layer.
TransportRelayRemoved {
    relay_url: RelayUrl,
},
```

---

## C. Flux announce (handle_relay_ready_announce)

Pipeline complet dans `state.rs`, `handle_relay_ready_announce()` :

```
1. Ignore self (announce.node_id == self.local_id) → return empty
2. verify_signature() → reject if invalid
3. is_fresh(now_ms()) → reject if stale
4. relay_registry.upsert(node_id, relay_url, timestamp, now_ms()) → UpsertResult
5. Emit RelayReadyReceived { node_id, relay_url }
6. Si config.enable_transport_relay_discovery == true :
   match upsert_result {
       Inserted => emit InsertTransportRelay { relay_url }
       UpdatedUrl { old_url } => {
           emit InsertTransportRelay { relay_url: new_url }
           // Retirer l'ancienne seulement si plus référencée
           if !relay_registry.has_active_url(&old_url) {
               emit RemoveTransportRelay { relay_url: old_url }
           }
       }
       RefreshedSameUrl => { /* no transport mutation needed */ }
   }
```

---

## D. Flux prune (tick_heartbeat)

Pipeline dans `state.rs`, `tick_heartbeat()`, après le prune existant :

```
1. relay_registry.prune(now_ms()) → Vec<RelayRegistryEntry>
2. Pour chaque entrée expirée :
   a. Emit RelayRegistryExpired { node_id, relay_url }
   b. Si config.enable_transport_relay_discovery == true :
      Si !relay_registry.has_active_url(&relay_url) :
         emit RemoveTransportRelay { relay_url }
```

**Point critique** : `has_active_url()` est appelé APRÈS le prune, donc il vérifie sur les entrées restantes. Si plusieurs nodes publiaient la même URL et qu'un seul expire, l'URL reste tant qu'un producteur actif existe.

---

## E. Flux overwrite d'URL

Cas : node X publiait `http://a:3340`, maintenant publie `http://b:3340`.

```
upsert(X, "http://b:3340", ...) → UpdatedUrl { old_url: "http://a:3340" }

Si enable_transport_relay_discovery:
  1. InsertTransportRelay { relay_url: "http://b:3340" }
  2. Vérifier: has_active_url("http://a:3340") ?
     - Si un autre node Y pointe encore vers "http://a:3340" → ne pas retirer
     - Si personne d'autre → RemoveTransportRelay { relay_url: "http://a:3340" }
```

---

## F. Gestion static vs discovered (loop.rs)

### F1. Insert (loop interceptor)

```rust
RuntimeEffect::InsertTransportRelay { relay_url } => {
    if !discovered_transport_relays.contains(&relay_url) {
        // Conservative config: quic: None — discovery signal doesn't advertise QUIC capability
        let config = Arc::new(RelayConfig { url: relay_url.clone(), quic: None });
        node.insert_relay(relay_url.clone(), config).await;
        discovered_transport_relays.insert(relay_url.clone());
        tracing::info!("transport: inserted discovered relay {relay_url}");
        // Emit event pour observabilité
        if event_tx.try_send(ProtocolEvent::TransportRelayInserted {
            relay_url,
        }).is_err() {}
    }
    // Si déjà dans discovered_transport_relays → skip (idempotent)
}
```

### F2. Remove (loop interceptor)

```rust
RuntimeEffect::RemoveTransportRelay { relay_url } => {
    // ONLY remove if we inserted it via discovery
    if discovered_transport_relays.remove(&relay_url) {
        node.remove_relay(&relay_url).await;
        tracing::info!("transport: removed discovered relay {relay_url}");
        if event_tx.try_send(ProtocolEvent::TransportRelayRemoved {
            relay_url,
        }).is_err() {}
    }
    // Si pas dans discovered_transport_relays → URL statique, ne pas toucher
}
```

**Invariant garanti** : les URLs statiques (configurées au boot via TOM_RELAY_URL, fallback, etc.) ne sont jamais dans `discovered_transport_relays`, donc jamais retirées.

---

## G. Fichiers impactés

| Fichier | Modification |
|---------|-------------|
| `crates/tom-transport/src/node.rs` | Ajouter `pub async fn insert_relay()` + `remove_relay()` |
| `crates/tom-protocol/src/discovery/relay_registry.rs` | `UpsertResult` enum, changer `upsert() -> UpsertResult`, ajouter `has_active_url()`, mettre à jour tests |
| `crates/tom-protocol/src/runtime/effect.rs` | Ajouter `InsertTransportRelay`, `RemoveTransportRelay` |
| `crates/tom-protocol/src/runtime/mod.rs` | Ajouter `enable_transport_relay_discovery` à RuntimeConfig, `TransportRelayInserted`/`Removed` à ProtocolEvent |
| `crates/tom-protocol/src/runtime/state.rs` | Modifier `handle_relay_ready_announce()` et `tick_heartbeat()` pour émettre les effets transport |
| `crates/tom-protocol/src/runtime/loop.rs` | Ajouter `discovered_transport_relays`, intercepter les 2 nouveaux effets |
| `crates/tom-protocol/src/runtime/executor.rs` | Ajouter fallback arms pour les 2 nouveaux effets (log debug si atteint) |

---

## H. Plan de tests

### H1. Unit tests — relay_registry.rs

| Test | Description |
|------|-------------|
| `upsert_new_returns_inserted` | Nouveau node → `UpsertResult::Inserted` |
| `upsert_same_url_returns_refreshed` | Même node, même URL → `RefreshedSameUrl` |
| `upsert_different_url_returns_updated` | Même node, URL différente → `UpdatedUrl { old_url }` |
| `has_active_url_true_when_present` | URL présente dans au moins une entrée active |
| `has_active_url_false_when_absent` | URL absente de toutes les entrées |
| `has_active_url_false_after_prune` | URL disparue après prune |
| `has_active_url_shared_url` | Deux nodes pointent vers la même URL, un expire, l'URL reste active |

### H2. Tests existants — adapter au nouveau retour

| Test existant | Adaptation |
|---------------|-----------|
| `upsert_new_entry_returns_true` | → `assert_eq!(result, UpsertResult::Inserted)` |
| `upsert_refresh_returns_false` | → `assert_eq!(result, UpsertResult::RefreshedSameUrl)` |
| Tous les autres | Vérifier qu'ils compilent avec le nouveau type |

### H3. Runtime tests — state.rs

| Test | Description |
|------|-------------|
| `announce_with_flag_off_no_transport_effect` | flag false → effects = [RelayReadyReceived] seulement |
| `announce_with_flag_on_emits_insert` | flag true + nouvelle URL → [RelayReadyReceived, InsertTransportRelay] |
| `announce_refresh_same_url_no_insert` | flag true + même URL refresh → [RelayReadyReceived] (pas d'insert) |
| `announce_url_change_emits_insert_and_remove` | flag true + URL changée → [RelayReadyReceived, InsertTransportRelay(new), RemoveTransportRelay(old)] |
| `announce_url_change_shared_url_no_remove` | flag true + URL changée mais ancienne URL encore active → [RelayReadyReceived, InsertTransportRelay(new)] (pas de remove) |
| `prune_with_flag_on_emits_remove` | flag true + expiration → [RelayRegistryExpired, RemoveTransportRelay] |
| `prune_shared_url_no_remove` | flag true + expiration mais URL partagée → [RelayRegistryExpired] (pas de remove) |

### H4. Non-régression architecturale

| Test | Description |
|------|-------------|
| `no_topology_mutation_on_announce` | Après announce, Topology inchangée |
| `no_relay_selector_impact` | RelaySelector.select_best() identique avant/après announce |

### H5. Cross-crate — tom-transport

| Test | Description |
|------|-------------|
| `insert_relay_compiles` | TomNode::insert_relay() compile et est appelable |
| `remove_relay_compiles` | TomNode::remove_relay() compile et est appelable |

(Tests d'intégration transport = hors scope de ce chantier, car nécessitent un Endpoint réel. Les wrappers sont triviaux.)

---

## I. Ordre des commits recommandé

### Commit 1 : `feat(transport): expose insert_relay/remove_relay on TomNode`
- Fichier : `crates/tom-transport/src/node.rs`
- 2 méthodes publiques wrapper
- Validation : `cargo build -p tom-transport`

### Commit 2 : `feat(discovery): enrich upsert() with UpsertResult enum`
- Fichier : `crates/tom-protocol/src/discovery/relay_registry.rs`
- `UpsertResult` enum + `has_active_url()` + adaptation upsert + nouveaux tests + adaptation tests existants
- Validation : `cargo test -p tom-protocol --lib relay_registry`

### Commit 3 : `feat(runtime): add transport relay discovery config, effects, events`
- Fichiers : `effect.rs`, `mod.rs`, `executor.rs`
- `InsertTransportRelay`, `RemoveTransportRelay` dans RuntimeEffect
- `enable_transport_relay_discovery` dans RuntimeConfig
- `TransportRelayInserted`, `TransportRelayRemoved` dans ProtocolEvent
- Fallback arms dans executor
- Validation : `cargo build -p tom-protocol`

### Commit 4 : `feat(runtime): emit transport effects from state on announce/prune`
- Fichier : `crates/tom-protocol/src/runtime/state.rs`
- Modifier `handle_relay_ready_announce()` + `tick_heartbeat()`
- Adapter le call site existant de `upsert()` (bool → UpsertResult)
- Validation : `cargo build -p tom-protocol`

### Commit 5 : `feat(runtime): intercept transport relay effects in loop`
- Fichier : `crates/tom-protocol/src/runtime/loop.rs`
- `discovered_transport_relays` HashSet
- Intercepteur pour `InsertTransportRelay` / `RemoveTransportRelay`
- Validation : `cargo build -p tom-protocol`

### Commit 6 : `test(runtime): add transport relay discovery tests`
- Fichier : `crates/tom-protocol/src/runtime/state.rs` (module tests)
- Tous les tests H3 + H4
- Validation : `cargo test -p tom-protocol --lib`

### Commit 7 : `chore: clippy + workspace validation`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`

### Commit 8 : `git push`

---

## J. Risques identifiés

| Risque | Mitigation |
|--------|-----------|
| QUIC discovery sur relays HTTP-only | **TRANCHÉ** : `RelayConfig { url, quic: None }` pour tous les relays découverts dynamiquement. QUIC réservé aux relays statiques ou futur protocole discovery explicite. |
| Connexion relay ~3-10s après insert | Comportement normal, pas bloquant |
| Boucle insert/remove si announce oscille | Le registre est par NodeId, un seul état par node, pas d'oscillation possible |
| URL statique retirée par erreur | Garanti impossible : `discovered_transport_relays` ne contient que les URLs insérées via discovery |
| Cross-crate breaking change sur tom-transport | Seul ajout de méthodes publiques, pas de breaking change |
