# RuntimeState Refactoring — Plan d'implementation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extraire la god function `runtime_loop()` (1366 lignes) en un `RuntimeState` testable + trait `Transport` + pattern `RuntimeEffect`.

**Architecture:** Les methodes pures sur `RuntimeState` retournent `Vec<RuntimeEffect>`. La boucle `select!` reste un orchestrateur mince (~120 lignes). L'I/O passe par un trait `Transport` (impl TomNode en prod, MockTransport en test).

**Tech Stack:** Rust, tokio, iroh 0.96, rmp-serde (MessagePack)

**Design doc:** `docs/design-runtime-state.md`

---

## Task 1 : Creer `effect.rs` — L'enum RuntimeEffect

**Files:**
- Create: `crates/tom-protocol/src/runtime/effect.rs`
- Modify: `crates/tom-protocol/src/runtime/mod.rs` (ajouter `mod effect;` + re-export)

**Step 1: Creer le fichier effect.rs**

```rust
// crates/tom-protocol/src/runtime/effect.rs

use crate::envelope::Envelope;
use crate::tracker::StatusChange;
use crate::types::NodeId;

use super::{DeliveredMessage, ProtocolEvent};

/// Intention produite par la logique pure de RuntimeState.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// La boucle principale execute ensuite ces effets via Transport + channels.
#[derive(Debug)]
pub enum RuntimeEffect {
    /// Envoyer une enveloppe au premier hop (relay ou direct).
    SendEnvelope(Envelope),

    /// Envoyer une enveloppe a un noeud precis (hop explicite).
    SendEnvelopeTo {
        target: NodeId,
        envelope: Envelope,
    },

    /// Livrer un message decrypte a l'application (TUI, bot...).
    DeliverMessage(DeliveredMessage),

    /// Notifier un changement de statut (pending → sent → relayed → delivered → read).
    StatusChange(StatusChange),

    /// Emettre un evenement protocole (peer offline, group created, etc.).
    Emit(ProtocolEvent),

    /// Essayer d'envoyer — si le transport echoue, executer le plan B.
    /// Utilise pour le backup automatique quand un peer est offline.
    SendWithBackupFallback {
        envelope: Envelope,
        on_success: Vec<RuntimeEffect>,
        on_failure: Vec<RuntimeEffect>,
    },
}
```

**Step 2: Declarer le module dans mod.rs**

Dans `crates/tom-protocol/src/runtime/mod.rs`, ajouter apres `mod r#loop;` :

```rust
mod effect;
pub use effect::RuntimeEffect;
```

**Step 3: Re-exporter depuis lib.rs**

Dans `crates/tom-protocol/src/lib.rs`, ajouter `RuntimeEffect` aux re-exports du runtime :

```rust
pub use runtime::{
    DeliveredMessage, ProtocolEvent, ProtocolRuntime, RuntimeChannels, RuntimeConfig,
    RuntimeEffect, RuntimeHandle,
};
```

**Step 4: Verifier la compilation**

Run: `cargo check -p tom-protocol`
Expected: OK (l'enum est defini mais pas encore utilise)

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/effect.rs crates/tom-protocol/src/runtime/mod.rs crates/tom-protocol/src/lib.rs
git commit -m "refactor(runtime): add RuntimeEffect enum — intent pattern for pure logic"
```

---

## Task 2 : Creer `transport.rs` — Le trait Transport

**Files:**
- Create: `crates/tom-protocol/src/runtime/transport.rs`
- Modify: `crates/tom-protocol/src/runtime/mod.rs` (ajouter `mod transport;`)
- Modify: `crates/tom-protocol/Cargo.toml` (ajouter `async-trait`)

**Step 1: Ajouter async-trait aux dependances**

Dans `crates/tom-protocol/Cargo.toml`, section `[dependencies]`, ajouter :

```toml
async-trait = "0.1"
```

**Step 2: Creer transport.rs**

```rust
// crates/tom-protocol/src/runtime/transport.rs

use crate::types::NodeId;

/// Abstraction reseau pour le runtime.
///
/// En production : impl par TomNode (iroh QUIC).
/// En test : impl par MockTransport (enregistre les envois).
#[async_trait::async_trait]
pub trait Transport: Send {
    /// Envoyer des bytes bruts a un noeud cible.
    async fn send_raw(&self, target: NodeId, data: &[u8]) -> Result<(), String>;

    /// Lister les peers actuellement connectes.
    async fn connected_peers(&self) -> Vec<NodeId>;
}

// ── Impl pour TomNode (production) ──────────────────────────────────

#[async_trait::async_trait]
impl Transport for tom_transport::TomNode {
    async fn send_raw(&self, target: NodeId, data: &[u8]) -> Result<(), String> {
        tom_transport::TomNode::send_raw(self, target, data)
            .await
            .map_err(|e| e.to_string())
    }

    async fn connected_peers(&self) -> Vec<NodeId> {
        tom_transport::TomNode::connected_peers(self).await
    }
}

// ── MockTransport (tests) ───────────────────────────────────────────

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// Faux transport qui enregistre les envois pour verification.
    ///
    /// Usage dans les tests :
    /// ```ignore
    /// let mock = MockTransport::new();
    /// // ... appeler du code qui utilise le transport ...
    /// let sends = mock.sent();
    /// assert_eq!(sends.len(), 1);
    /// assert_eq!(sends[0].0, target_node_id);
    /// ```
    #[derive(Clone)]
    pub struct MockTransport {
        sent: Arc<Mutex<Vec<(NodeId, Vec<u8>)>>>,
        peers: Arc<Mutex<Vec<NodeId>>>,
        /// Si true, send_raw echoue (pour tester le backup fallback).
        fail_sends: Arc<Mutex<bool>>,
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self {
                sent: Arc::new(Mutex::new(Vec::new())),
                peers: Arc::new(Mutex::new(Vec::new())),
                fail_sends: Arc::new(Mutex::new(false)),
            }
        }

        /// Recuperer la liste des envois effectues.
        pub fn sent(&self) -> Vec<(NodeId, Vec<u8>)> {
            self.sent.lock().unwrap().clone()
        }

        /// Configurer les peers "connectes".
        pub fn set_peers(&self, peers: Vec<NodeId>) {
            *self.peers.lock().unwrap() = peers;
        }

        /// Faire echouer tous les envois (pour tester le backup).
        pub fn set_fail_sends(&self, fail: bool) {
            *self.fail_sends.lock().unwrap() = fail;
        }

        /// Vider les envois enregistres.
        pub fn clear_sent(&self) {
            self.sent.lock().unwrap().clear();
        }
    }

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn send_raw(&self, target: NodeId, data: &[u8]) -> Result<(), String> {
            if *self.fail_sends.lock().unwrap() {
                return Err("mock: send failed".to_string());
            }
            self.sent.lock().unwrap().push((target, data.to_vec()));
            Ok(())
        }

        async fn connected_peers(&self) -> Vec<NodeId> {
            self.peers.lock().unwrap().clone()
        }
    }
}
```

**Step 3: Declarer le module dans mod.rs**

Dans `crates/tom-protocol/src/runtime/mod.rs`, ajouter apres `mod effect;` :

```rust
mod transport;
pub use transport::Transport;
```

**Step 4: Verifier la compilation**

Run: `cargo check -p tom-protocol`
Expected: OK

Run: `cargo test -p tom-protocol`
Expected: 261 tests passent toujours

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/transport.rs crates/tom-protocol/src/runtime/mod.rs crates/tom-protocol/Cargo.toml
git commit -m "refactor(runtime): add Transport trait + MockTransport for testability"
```

---

## Task 3 : Creer `state.rs` — Le struct RuntimeState (vide + constructeur)

**Files:**
- Create: `crates/tom-protocol/src/runtime/state.rs`
- Modify: `crates/tom-protocol/src/runtime/mod.rs` (ajouter `mod state;` + re-export)

**Step 1: Creer state.rs avec le struct et le constructeur**

```rust
// crates/tom-protocol/src/runtime/state.rs

use crate::backup::BackupCoordinator;
use crate::discovery::{EphemeralSubnetManager, HeartbeatTracker};
use crate::group::{GroupHub, GroupManager};
use crate::relay::{PeerRole, RelaySelector, Topology};
use crate::roles::RoleManager;
use crate::router::Router;
use crate::tracker::MessageTracker;
use crate::types::NodeId;

use super::RuntimeConfig;

/// Etat complet du protocole — logique pure, zero async, zero reseau.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// Aucune methode ne touche au reseau ni aux channels.
pub struct RuntimeState {
    pub(crate) local_id: NodeId,
    pub(crate) secret_seed: [u8; 32],
    pub(crate) config: RuntimeConfig,

    // Protocol modules
    pub(crate) router: Router,
    pub(crate) relay_selector: RelaySelector,
    pub(crate) topology: Topology,
    pub(crate) tracker: MessageTracker,
    pub(crate) heartbeat: HeartbeatTracker,

    // Group
    pub(crate) group_manager: GroupManager,
    pub(crate) group_hub: GroupHub,

    // Backup
    pub(crate) backup: BackupCoordinator,

    // Discovery
    pub(crate) subnets: EphemeralSubnetManager,
    pub(crate) role_manager: RoleManager,
    pub(crate) local_roles: Vec<PeerRole>,
}

impl RuntimeState {
    /// Creer un nouvel etat de protocole.
    pub fn new(local_id: NodeId, secret_seed: [u8; 32], config: RuntimeConfig) -> Self {
        Self {
            router: Router::new(local_id),
            relay_selector: RelaySelector::new(local_id),
            topology: Topology::new(),
            tracker: MessageTracker::new(),
            heartbeat: HeartbeatTracker::new(),
            group_manager: GroupManager::new(local_id, config.username.clone()),
            group_hub: GroupHub::new(local_id),
            backup: BackupCoordinator::new(local_id),
            subnets: EphemeralSubnetManager::new(local_id),
            role_manager: RoleManager::new(local_id),
            local_roles: vec![PeerRole::Peer],
            local_id,
            secret_seed,
            config,
        }
    }
}
```

**Step 2: Declarer le module dans mod.rs**

Dans `crates/tom-protocol/src/runtime/mod.rs`, ajouter apres `mod transport;` :

```rust
mod state;
pub use state::RuntimeState;
```

Et dans `crates/tom-protocol/src/lib.rs`, ajouter `RuntimeState` aux re-exports :

```rust
pub use runtime::{
    DeliveredMessage, ProtocolEvent, ProtocolRuntime, RuntimeChannels, RuntimeConfig,
    RuntimeEffect, RuntimeHandle, RuntimeState,
};
```

**Step 3: Verifier**

Run: `cargo check -p tom-protocol`
Expected: OK (struct cree mais pas utilise — peut avoir un warning unused, c'est OK)

**Step 4: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs crates/tom-protocol/src/runtime/mod.rs crates/tom-protocol/src/lib.rs
git commit -m "refactor(runtime): add RuntimeState struct with constructor"
```

---

## Task 4 : Migrer les ticks triviaux (cache_cleanup, tracker_cleanup)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs` (ajouter 2 methodes)

**Step 1: Ecrire le test pour tick_cache_cleanup**

Ajouter en bas de `state.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::effect::RuntimeEffect;

    /// Genere un NodeId deterministe a partir d'un seed (meme pattern que router.rs).
    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    /// Genere une paire (NodeId, secret_seed) pour les tests.
    fn keypair(seed: u8) -> (NodeId, [u8; 32]) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        let node_id: NodeId = secret.public().to_string().parse().unwrap();
        let seed_bytes = secret.to_bytes();
        (node_id, seed_bytes)
    }

    fn default_state(seed: u8) -> RuntimeState {
        let (id, secret) = keypair(seed);
        RuntimeState::new(id, secret, RuntimeConfig::default())
    }

    #[test]
    fn tick_cache_cleanup_retourne_aucun_effet() {
        let mut state = default_state(1);
        let effects = state.tick_cache_cleanup();
        assert!(effects.is_empty(), "cache cleanup ne produit aucun effet visible");
    }

    #[test]
    fn tick_tracker_cleanup_retourne_aucun_effet() {
        let mut state = default_state(1);
        let effects = state.tick_tracker_cleanup();
        assert!(effects.is_empty(), "tracker cleanup ne produit aucun effet visible");
    }
}
```

**Step 2: Verifier que les tests echouent**

Run: `cargo test -p tom-protocol tick_cache_cleanup`
Expected: FAIL — method `tick_cache_cleanup` not found

**Step 3: Implementer les methodes**

Ajouter dans le bloc `impl RuntimeState` de `state.rs` :

```rust
    // ── Ticks triviaux ──────────────────────────────────────────────

    /// Nettoyage periodique du cache de dedup du router.
    pub fn tick_cache_cleanup(&mut self) -> Vec<super::effect::RuntimeEffect> {
        self.router.cleanup_caches();
        Vec::new()
    }

    /// Eviction periodique des messages expires du tracker.
    pub fn tick_tracker_cleanup(&mut self) -> Vec<super::effect::RuntimeEffect> {
        self.tracker.evict_expired();
        Vec::new()
    }
```

**Step 4: Verifier que les tests passent**

Run: `cargo test -p tom-protocol tick_cache_cleanup tick_tracker_cleanup`
Expected: 2 tests PASS

Run: `cargo test -p tom-protocol`
Expected: 263 tests (261 + 2 nouveaux)

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): migrate tick_cache_cleanup + tick_tracker_cleanup to RuntimeState"
```

---

## Task 5 : Migrer tick_heartbeat (detection online/offline)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Step 1: Ecrire le test**

Ajouter dans le bloc `tests` de `state.rs` :

```rust
    #[test]
    fn tick_heartbeat_detecte_peer_offline() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Ajoute un peer vu il y a longtemps (> seuil offline)
        use crate::relay::{PeerInfo, PeerStatus};
        let old_time = crate::types::now_ms().saturating_sub(50_000); // 50s ago
        state.topology.upsert(PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: old_time,
        });
        state.heartbeat.record_heartbeat_at(peer, old_time);

        let effects = state.tick_heartbeat();

        // Devrait emettre PeerOffline
        assert!(effects.iter().any(|e| matches!(
            e,
            RuntimeEffect::Emit(super::super::ProtocolEvent::PeerOffline { node_id }) if *node_id == peer
        )), "heartbeat devrait detecter le peer offline");
    }

    #[test]
    fn tick_heartbeat_peer_en_ligne_aucun_effet() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Ajoute un peer vu a l'instant
        use crate::relay::{PeerInfo, PeerStatus};
        state.topology.upsert(PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: crate::types::now_ms(),
        });
        state.heartbeat.record_heartbeat(peer);

        let effects = state.tick_heartbeat();

        // Aucun PeerOffline
        assert!(!effects.iter().any(|e| matches!(
            e,
            RuntimeEffect::Emit(super::super::ProtocolEvent::PeerOffline { .. })
        )), "peer en ligne ne devrait pas etre marque offline");
    }
```

**Step 2: Verifier que les tests echouent**

Run: `cargo test -p tom-protocol tick_heartbeat`
Expected: FAIL — method `tick_heartbeat` not found

**Step 3: Implementer**

```rust
    /// Verification periodique des heartbeats.
    /// Detecte les peers offline, nettoie les subnets et roles, livre les backups.
    pub fn tick_heartbeat(&mut self) -> Vec<super::effect::RuntimeEffect> {
        use crate::discovery::DiscoveryEvent;
        let mut effects = Vec::new();

        let events = self.heartbeat.check_all(&mut self.topology);
        for disc_event in events {
            match disc_event {
                DiscoveryEvent::PeerOffline { node_id } => {
                    let subnet_events = self.subnets.remove_node(&node_id);
                    for se in &subnet_events {
                        effects.extend(self.surface_subnet_event(se));
                    }
                    self.role_manager.remove_node(&node_id);
                    effects.push(super::effect::RuntimeEffect::Emit(
                        super::ProtocolEvent::PeerOffline { node_id },
                    ));
                }
                DiscoveryEvent::PeerDiscovered { node_id, .. } => {
                    effects.push(super::effect::RuntimeEffect::Emit(
                        super::ProtocolEvent::PeerDiscovered { node_id },
                    ));
                    // Prepare la livraison des backups pour ce peer
                    effects.extend(self.prepare_backup_delivery(node_id));
                }
                _ => {} // PeerStale — ignore for MVP
            }
        }
        self.heartbeat.cleanup_departed();
        effects
    }

    /// Convertit un SubnetEvent en RuntimeEffect.
    fn surface_subnet_event(
        &self,
        event: &crate::discovery::SubnetEvent,
    ) -> Vec<super::effect::RuntimeEffect> {
        use crate::discovery::SubnetEvent;
        match event {
            SubnetEvent::SubnetFormed { subnet } => vec![super::effect::RuntimeEffect::Emit(
                super::ProtocolEvent::SubnetFormed {
                    subnet_id: subnet.subnet_id.clone(),
                    members: subnet.members.iter().copied().collect(),
                },
            )],
            SubnetEvent::SubnetDissolved { subnet_id, reason } => {
                vec![super::effect::RuntimeEffect::Emit(
                    super::ProtocolEvent::SubnetDissolved {
                        subnet_id: subnet_id.clone(),
                        reason: format!("{reason:?}"),
                    },
                )]
            }
            _ => Vec::new(),
        }
    }

    /// Prepare les effets pour livrer les backups d'un peer qui revient online.
    /// Retourne des SendWithBackupFallback pour chaque message stocke.
    fn prepare_backup_delivery(&mut self, peer_id: NodeId) -> Vec<super::effect::RuntimeEffect> {
        use crate::envelope::EnvelopeBuilder;
        use crate::types::MessageType;

        let entries: Vec<(String, Vec<u8>)> = self
            .backup
            .store()
            .get_for_recipient(&peer_id)
            .into_iter()
            .map(|e| (e.message_id.clone(), e.payload.clone()))
            .collect();

        if entries.is_empty() {
            return Vec::new();
        }

        let mut effects = Vec::new();
        for (_message_id, payload) in &entries {
            let via = self.relay_selector.select_path(peer_id, &self.topology);
            let builder = EnvelopeBuilder::new(
                self.local_id,
                peer_id,
                MessageType::Chat,
                payload.clone(),
            )
            .via(via);

            let envelope = if self.config.encryption {
                let recipient_pk = peer_id.as_bytes();
                match builder.encrypt_and_sign(&self.secret_seed, &recipient_pk) {
                    Ok(env) => env,
                    Err(_) => continue,
                }
            } else {
                builder.sign(&self.secret_seed)
            };

            let envelope_id = envelope.id.clone();

            // Track + mark_sent en cas de succes
            let on_success = if let Some(change) = self.tracker.track(envelope_id.clone(), peer_id) {
                let mut s = vec![super::effect::RuntimeEffect::StatusChange(change)];
                if let Some(sent_change) = self.tracker.mark_sent(&envelope_id) {
                    s.push(super::effect::RuntimeEffect::StatusChange(sent_change));
                }
                s.push(super::effect::RuntimeEffect::Emit(
                    super::ProtocolEvent::BackupDelivered {
                        message_id: _message_id.clone(),
                        recipient_id: peer_id,
                    },
                ));
                s
            } else {
                Vec::new()
            };

            effects.push(super::effect::RuntimeEffect::SendWithBackupFallback {
                envelope,
                on_success,
                on_failure: Vec::new(), // Le message est deja en backup, pas besoin de re-stocker
            });
        }
        effects
    }
```

**Step 4: Verifier**

Run: `cargo test -p tom-protocol tick_heartbeat`
Expected: 2 tests PASS

Run: `cargo test -p tom-protocol`
Expected: tous les tests passent

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): migrate tick_heartbeat + backup delivery to RuntimeState"
```

---

## Task 6 : Migrer les ticks restants (group_hub, backup, subnets, roles)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Step 1: Ecrire les tests**

```rust
    #[test]
    fn tick_subnets_sur_etat_vierge_aucun_effet() {
        let mut state = default_state(1);
        let effects = state.tick_subnets();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_roles_sur_etat_vierge_aucun_effet() {
        let mut state = default_state(1);
        let effects = state.tick_roles();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_backup_sur_etat_vierge_aucun_effet() {
        let mut state = default_state(1);
        let effects = state.tick_backup();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_group_hub_heartbeat_sur_etat_vierge_aucun_effet() {
        let mut state = default_state(1);
        let effects = state.tick_group_hub_heartbeat();
        assert!(effects.is_empty());
    }
```

**Step 2: Verifier que les tests echouent**

Run: `cargo test -p tom-protocol tick_subnets tick_roles tick_backup tick_group_hub`
Expected: FAIL — methods not found

**Step 3: Implementer les 4 methodes**

```rust
    /// Evaluation periodique des sous-reseaux ephemeres.
    pub fn tick_subnets(&mut self) -> Vec<super::effect::RuntimeEffect> {
        let events = self.subnets.evaluate(crate::types::now_ms());
        let mut effects = Vec::new();
        for event in &events {
            effects.extend(self.surface_subnet_event(event));
        }
        effects
    }

    /// Evaluation periodique des roles (promotion/demotion Peer ↔ Relay).
    pub fn tick_roles(&mut self) -> Vec<super::effect::RuntimeEffect> {
        let actions = self.role_manager.evaluate(&mut self.topology, crate::types::now_ms());
        let mut effects = Vec::new();
        for action in &actions {
            effects.extend(self.surface_role_action(action));
        }
        effects
    }

    /// Maintenance periodique du backup (TTL, replication).
    pub fn tick_backup(&mut self) -> Vec<super::effect::RuntimeEffect> {
        let actions = self.backup.tick(crate::types::now_ms());
        self.backup_actions_to_effects(&actions)
    }

    /// Heartbeat periodique du hub groupe.
    pub fn tick_group_hub_heartbeat(&mut self) -> Vec<super::effect::RuntimeEffect> {
        let actions = self.group_hub.heartbeat_actions();
        self.group_actions_to_effects(&actions)
    }

    /// Convertit un RoleAction en RuntimeEffect.
    fn surface_role_action(
        &mut self,
        action: &crate::roles::RoleAction,
    ) -> Vec<super::effect::RuntimeEffect> {
        use crate::roles::RoleAction;
        let proto_event = match action {
            RoleAction::Promoted { node_id, score } => super::ProtocolEvent::RolePromoted {
                node_id: *node_id,
                score: *score,
            },
            RoleAction::Demoted { node_id, score } => super::ProtocolEvent::RoleDemoted {
                node_id: *node_id,
                score: *score,
            },
            RoleAction::LocalRoleChanged { new_role } => {
                self.local_roles = vec![*new_role];
                super::ProtocolEvent::LocalRoleChanged {
                    new_role: *new_role,
                }
            }
        };
        vec![super::effect::RuntimeEffect::Emit(proto_event)]
    }

    /// Convertit des GroupAction en RuntimeEffect.
    fn group_actions_to_effects(
        &self,
        actions: &[crate::group::GroupAction],
    ) -> Vec<super::effect::RuntimeEffect> {
        use crate::envelope::EnvelopeBuilder;
        use crate::group::GroupAction;

        let mut effects = Vec::new();
        for action in actions {
            match action {
                GroupAction::Send { to, payload } => {
                    let msg_type = group_payload_to_message_type(payload);
                    let payload_bytes =
                        rmp_serde::to_vec(payload).expect("group payload serialization");
                    let via = self.relay_selector.select_path(*to, &self.topology);
                    let envelope =
                        EnvelopeBuilder::new(self.local_id, *to, msg_type, payload_bytes)
                            .via(via)
                            .sign(&self.secret_seed);
                    effects.push(super::effect::RuntimeEffect::SendEnvelope(envelope));
                }
                GroupAction::Broadcast { to, payload } => {
                    let msg_type = group_payload_to_message_type(payload);
                    let payload_bytes =
                        rmp_serde::to_vec(payload).expect("group payload serialization");
                    for target in to {
                        let via = self.relay_selector.select_path(*target, &self.topology);
                        let envelope = EnvelopeBuilder::new(
                            self.local_id,
                            *target,
                            msg_type,
                            payload_bytes.clone(),
                        )
                        .via(via)
                        .sign(&self.secret_seed);
                        effects.push(super::effect::RuntimeEffect::SendEnvelope(envelope));
                    }
                }
                GroupAction::Event(event) => {
                    effects.extend(self.surface_group_event(event));
                }
                GroupAction::None => {}
            }
        }
        effects
    }

    /// Convertit des BackupAction en RuntimeEffect.
    fn backup_actions_to_effects(
        &self,
        actions: &[crate::backup::BackupAction],
    ) -> Vec<super::effect::RuntimeEffect> {
        use crate::backup::BackupAction;
        use crate::envelope::EnvelopeBuilder;
        use crate::relay::PeerStatus;
        use crate::types::MessageType;

        let mut effects = Vec::new();
        for action in actions {
            match action {
                BackupAction::Replicate { target, payload } => {
                    let bytes =
                        rmp_serde::to_vec(payload).expect("backup replication serialization");
                    let via = self.relay_selector.select_path(*target, &self.topology);
                    let envelope = EnvelopeBuilder::new(
                        self.local_id,
                        *target,
                        MessageType::BackupReplicate,
                        bytes,
                    )
                    .via(via)
                    .sign(&self.secret_seed);
                    effects.push(super::effect::RuntimeEffect::SendEnvelope(envelope));
                }
                BackupAction::ConfirmDelivery {
                    message_ids,
                    recipient_id: _,
                } => {
                    let bytes =
                        rmp_serde::to_vec(message_ids).expect("backup confirm serialization");
                    for peer in self.topology.peers() {
                        if peer.node_id != self.local_id && peer.status == PeerStatus::Online {
                            let envelope = EnvelopeBuilder::new(
                                self.local_id,
                                peer.node_id,
                                MessageType::BackupConfirmDelivery,
                                bytes.clone(),
                            )
                            .sign(&self.secret_seed);
                            effects.push(super::effect::RuntimeEffect::SendEnvelope(envelope));
                        }
                    }
                }
                BackupAction::QueryPending { recipient_id } => {
                    let bytes =
                        rmp_serde::to_vec(recipient_id).expect("backup query serialization");
                    for peer in self.topology.peers() {
                        if peer.node_id != self.local_id && peer.status == PeerStatus::Online {
                            let envelope = EnvelopeBuilder::new(
                                self.local_id,
                                peer.node_id,
                                MessageType::BackupQuery,
                                bytes.clone(),
                            )
                            .sign(&self.secret_seed);
                            effects.push(super::effect::RuntimeEffect::SendEnvelope(envelope));
                        }
                    }
                }
                BackupAction::Event(event) => {
                    effects.extend(self.surface_backup_event(event));
                }
            }
        }
        effects
    }

    /// Convertit un GroupEvent en RuntimeEffect.
    fn surface_group_event(
        &self,
        event: &crate::group::GroupEvent,
    ) -> Vec<super::effect::RuntimeEffect> {
        use crate::group::GroupEvent;
        let proto_event = match event {
            GroupEvent::GroupCreated(info) => super::ProtocolEvent::GroupCreated {
                group: info.clone(),
            },
            GroupEvent::InviteReceived(invite) => super::ProtocolEvent::GroupInviteReceived {
                invite: invite.clone(),
            },
            GroupEvent::Joined { group_id, group_name } => super::ProtocolEvent::GroupJoined {
                group_id: group_id.clone(),
                group_name: group_name.clone(),
            },
            GroupEvent::MemberJoined { group_id, member } => {
                super::ProtocolEvent::GroupMemberJoined {
                    group_id: group_id.clone(),
                    member: member.clone(),
                }
            }
            GroupEvent::MemberLeft { group_id, node_id, username, reason } => {
                super::ProtocolEvent::GroupMemberLeft {
                    group_id: group_id.clone(),
                    node_id: *node_id,
                    username: username.clone(),
                    reason: *reason,
                }
            }
            GroupEvent::MessageReceived(msg) => super::ProtocolEvent::GroupMessageReceived {
                message: msg.clone(),
            },
            GroupEvent::HubMigrated { group_id, new_hub_id } => {
                super::ProtocolEvent::GroupHubMigrated {
                    group_id: group_id.clone(),
                    new_hub_id: *new_hub_id,
                }
            }
            GroupEvent::SecurityViolation { group_id, node_id, reason } => {
                super::ProtocolEvent::GroupSecurityViolation {
                    group_id: group_id.clone(),
                    node_id: *node_id,
                    reason: reason.clone(),
                }
            }
        };
        vec![super::effect::RuntimeEffect::Emit(proto_event)]
    }

    /// Convertit un BackupEvent en RuntimeEffect.
    fn surface_backup_event(
        &self,
        event: &crate::backup::BackupEvent,
    ) -> Vec<super::effect::RuntimeEffect> {
        use crate::backup::BackupEvent;
        match event {
            BackupEvent::MessageStored { message_id, recipient_id } => {
                vec![super::effect::RuntimeEffect::Emit(
                    super::ProtocolEvent::BackupStored {
                        message_id: message_id.clone(),
                        recipient_id: *recipient_id,
                    },
                )]
            }
            BackupEvent::MessageDelivered { message_id, recipient_id } => {
                vec![super::effect::RuntimeEffect::Emit(
                    super::ProtocolEvent::BackupDelivered {
                        message_id: message_id.clone(),
                        recipient_id: *recipient_id,
                    },
                )]
            }
            BackupEvent::MessageExpired { message_id, recipient_id } => {
                vec![super::effect::RuntimeEffect::Emit(
                    super::ProtocolEvent::BackupExpired {
                        message_id: message_id.clone(),
                        recipient_id: *recipient_id,
                    },
                )]
            }
            _ => Vec::new(),
        }
    }

    /// Construit l'annonce gossip periodique.
    /// Retourne les bytes a broadcaster (None si rien a annoncer).
    pub fn build_gossip_announce(&self) -> Option<Vec<u8>> {
        let announce = crate::discovery::PeerAnnounce::new(
            self.local_id,
            self.config.username.clone(),
            self.local_roles.clone(),
        );
        rmp_serde::to_vec(&announce).ok()
    }
```

Note: `group_payload_to_message_type` est une fonction libre existante dans `loop.rs`. Elle devra etre accessible depuis `state.rs`. Pour l'instant, la copier en haut de `state.rs` comme `fn` privee :

```rust
fn group_payload_to_message_type(payload: &crate::group::GroupPayload) -> crate::types::MessageType {
    use crate::group::GroupPayload;
    use crate::types::MessageType;
    match payload {
        GroupPayload::Create { .. } => MessageType::GroupCreate,
        GroupPayload::Created { .. } => MessageType::GroupCreated,
        GroupPayload::Invite { .. } => MessageType::GroupInvite,
        GroupPayload::Join { .. } => MessageType::GroupJoin,
        GroupPayload::Sync { .. } => MessageType::GroupSync,
        GroupPayload::Message(_) => MessageType::GroupMessage,
        GroupPayload::Leave { .. } => MessageType::GroupLeave,
        GroupPayload::MemberJoined { .. } => MessageType::GroupMemberJoined,
        GroupPayload::MemberLeft { .. } => MessageType::GroupMemberLeft,
        GroupPayload::DeliveryAck { .. } => MessageType::GroupDeliveryAck,
        GroupPayload::HubMigration { .. } => MessageType::GroupHubMigration,
        GroupPayload::HubHeartbeat { .. } => MessageType::GroupHubHeartbeat,
    }
}
```

**Step 4: Verifier**

Run: `cargo test -p tom-protocol tick_subnets tick_roles tick_backup tick_group_hub`
Expected: 4 tests PASS

Run: `cargo test -p tom-protocol`
Expected: tous les tests passent

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): migrate all tick methods to RuntimeState"
```

---

## Task 7 : Migrer handle_incoming_chat (le handler principal)

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Step 1: Ecrire les tests**

```rust
    #[test]
    fn handle_incoming_chat_livre_et_ack() {
        let (alice_id, alice_seed) = keypair(10);
        let (bob_id, bob_seed) = keypair(20);
        let mut state = RuntimeState::new(bob_id, bob_seed, RuntimeConfig::default());

        // Alice envoie un message signe (non chiffre pour simplifier)
        let envelope = EnvelopeBuilder::new(
            alice_id, bob_id, MessageType::Chat, b"salut bob".to_vec(),
        ).sign(&alice_seed);

        let sig_valid = envelope.verify_signature().is_ok();
        let effects = state.handle_incoming_chat(envelope, sig_valid);

        // 2 effets : livraison + ACK
        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[0], RuntimeEffect::DeliverMessage(msg)
            if msg.payload == b"salut bob" && msg.from == alice_id && msg.signature_valid));
        assert!(matches!(&effects[1], RuntimeEffect::SendEnvelope(ack)
            if ack.to == alice_id && ack.msg_type == MessageType::Ack));
    }

    #[test]
    fn handle_incoming_chat_chiffre_decrypte() {
        let (alice_id, alice_seed) = keypair(10);
        let (bob_id, bob_seed) = keypair(20);
        let mut state = RuntimeState::new(bob_id, bob_seed, RuntimeConfig::default());

        let envelope = EnvelopeBuilder::new(
            alice_id, bob_id, MessageType::Chat, b"secret".to_vec(),
        ).encrypt_and_sign(&alice_seed, &bob_id.as_bytes()).unwrap();

        let sig_valid = envelope.verify_signature().is_ok();
        let effects = state.handle_incoming_chat(envelope, sig_valid);

        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[0], RuntimeEffect::DeliverMessage(msg)
            if msg.payload == b"secret" && msg.was_encrypted));
    }

    #[test]
    fn handle_incoming_chat_forward_si_pas_pour_nous() {
        let (alice_id, alice_seed) = keypair(10);
        let (bob_id, _) = keypair(20);
        let (relay_id, relay_seed) = keypair(30);
        let mut state = RuntimeState::new(relay_id, relay_seed, RuntimeConfig::default());

        // Message d'Alice vers Bob, qui arrive chez relay_id (pas le destinataire)
        let mut envelope = EnvelopeBuilder::new(
            alice_id, bob_id, MessageType::Chat, b"for bob".to_vec(),
        ).sign(&alice_seed);
        // Le router a besoin que via contienne le relay pour le forward
        // Le router detecte que to != local_id et fait Forward

        let effects = state.handle_incoming_chat(envelope, true);

        // Devrait Forward (2 envois: message + relay ACK) + 1 event Forwarded
        assert!(effects.len() >= 2, "forward devrait produire au moins 2 effets, got {}", effects.len());
        assert!(effects.iter().any(|e| matches!(e, RuntimeEffect::Emit(
            super::super::ProtocolEvent::Forwarded { .. }
        ))), "devrait emettre un event Forwarded");
    }

    #[test]
    fn handle_incoming_chat_dedup_drop() {
        let (alice_id, alice_seed) = keypair(10);
        let (bob_id, bob_seed) = keypair(20);
        let mut state = RuntimeState::new(bob_id, bob_seed, RuntimeConfig::default());

        let envelope = EnvelopeBuilder::new(
            alice_id, bob_id, MessageType::Chat, b"once".to_vec(),
        ).sign(&alice_seed);

        // Premier passage → livre
        let effects1 = state.handle_incoming_chat(envelope.clone(), true);
        assert!(effects1.iter().any(|e| matches!(e, RuntimeEffect::DeliverMessage(_))));

        // Deuxieme passage (meme ID) → drop
        let effects2 = state.handle_incoming_chat(envelope, true);
        assert!(effects2.is_empty(), "doublon devrait etre drop");
    }
```

**Step 2: Verifier que les tests echouent**

Run: `cargo test -p tom-protocol handle_incoming_chat`
Expected: FAIL — method not found

**Step 3: Implementer**

```rust
    /// Traiter un message Chat / Ack / ReadReceipt / Heartbeat entrant.
    pub fn handle_incoming_chat(
        &mut self,
        envelope: crate::envelope::Envelope,
        signature_valid: bool,
    ) -> Vec<super::effect::RuntimeEffect> {
        use crate::router::{AckType, RoutingAction};

        let mut effects = Vec::new();
        let action = self.router.route(envelope);

        match action {
            RoutingAction::Deliver { mut envelope, response } => {
                let was_encrypted = envelope.encrypted;
                if envelope.encrypted {
                    if let Err(e) = envelope.decrypt_payload(&self.secret_seed) {
                        effects.push(super::effect::RuntimeEffect::Emit(
                            super::ProtocolEvent::Error {
                                description: format!(
                                    "decrypt failed from {}: {e}",
                                    envelope.from
                                ),
                            },
                        ));
                        return effects;
                    }
                }

                effects.push(super::effect::RuntimeEffect::DeliverMessage(
                    super::DeliveredMessage {
                        from: envelope.from,
                        payload: envelope.payload,
                        envelope_id: envelope.id,
                        timestamp: envelope.timestamp,
                        signature_valid,
                        was_encrypted,
                    },
                ));

                let mut ack = response;
                ack.sign(&self.secret_seed);
                effects.push(super::effect::RuntimeEffect::SendEnvelope(ack));
            }

            RoutingAction::Forward {
                envelope,
                next_hop,
                relay_ack,
            } => {
                let envelope_id = envelope.id.clone();
                let sender = envelope.from;

                self.role_manager
                    .record_relay(sender, crate::types::now_ms());

                effects.push(super::effect::RuntimeEffect::SendEnvelopeTo {
                    target: next_hop,
                    envelope: envelope.clone(),
                });

                let mut ack = relay_ack;
                ack.sign(&self.secret_seed);
                effects.push(super::effect::RuntimeEffect::SendEnvelopeTo {
                    target: sender,
                    envelope: ack,
                });

                effects.push(super::effect::RuntimeEffect::Emit(
                    super::ProtocolEvent::Forwarded {
                        envelope_id,
                        next_hop,
                    },
                ));
            }

            RoutingAction::Ack {
                original_message_id,
                ack_type,
                ..
            } => {
                let change = match ack_type {
                    AckType::RelayForwarded => self.tracker.mark_relayed(&original_message_id),
                    AckType::RecipientReceived => {
                        self.tracker.mark_delivered(&original_message_id)
                    }
                };
                if let Some(change) = change {
                    effects.push(super::effect::RuntimeEffect::StatusChange(change));
                }
            }

            RoutingAction::ReadReceipt {
                original_message_id,
                ..
            } => {
                if let Some(change) = self.tracker.mark_read(&original_message_id) {
                    effects.push(super::effect::RuntimeEffect::StatusChange(change));
                }
            }

            RoutingAction::Reject { reason } => {
                effects.push(super::effect::RuntimeEffect::Emit(
                    super::ProtocolEvent::MessageRejected { reason },
                ));
            }

            RoutingAction::Drop => {}
        }
        effects
    }
```

**Step 4: Verifier**

Run: `cargo test -p tom-protocol handle_incoming_chat`
Expected: 4 tests PASS

Run: `cargo test -p tom-protocol`
Expected: tous les tests passent

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): migrate handle_incoming_chat to RuntimeState — pure logic"
```

---

## Task 8 : Migrer handle_incoming_group + handle_incoming_backup

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Step 1: Implementer handle_incoming_group**

Meme pattern que handle_incoming_chat — deserialise GroupPayload, dispatch hub/member,
retourne des effets via `group_actions_to_effects()`. Le code est une copie directe de
`loop.rs:631-730` avec `node` et `event_tx` remplaces par des effects.

**Step 2: Implementer handle_incoming_backup**

Meme pattern — dispatch par MessageType, convertit les BackupAction en effets via
`backup_actions_to_effects()`. Copie directe de `loop.rs:735-816`.

**Step 3: Implementer handle_incoming (point d'entree unifie)**

```rust
    /// Point d'entree unique pour les donnees du reseau.
    /// Parse l'enveloppe, verifie la signature, dispatch par type.
    pub fn handle_incoming(&mut self, raw_data: &[u8]) -> Vec<super::effect::RuntimeEffect> {
        let envelope = match crate::envelope::Envelope::from_bytes(raw_data) {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("bad envelope: {e}");
                return Vec::new();
            }
        };

        let signature_valid = if envelope.is_signed() {
            envelope.verify_signature().is_ok()
        } else {
            false
        };

        // Heartbeat + auto-register
        self.heartbeat.record_heartbeat(envelope.from);
        if self.topology.get(&envelope.from).is_none() {
            self.topology.upsert(crate::relay::PeerInfo {
                node_id: envelope.from,
                role: PeerRole::Peer,
                status: crate::relay::PeerStatus::Online,
                last_seen: crate::types::now_ms(),
            });
        }

        match envelope.msg_type {
            MessageType::Chat | MessageType::Ack | MessageType::ReadReceipt | MessageType::Heartbeat => {
                if envelope.msg_type == MessageType::Chat {
                    self.subnets.record_communication(envelope.from, self.local_id, crate::types::now_ms());
                }
                self.handle_incoming_chat(envelope, signature_valid)
            }
            // Group types
            MessageType::GroupCreate | MessageType::GroupCreated | MessageType::GroupInvite
            | MessageType::GroupJoin | MessageType::GroupSync | MessageType::GroupMessage
            | MessageType::GroupLeave | MessageType::GroupMemberJoined | MessageType::GroupMemberLeft
            | MessageType::GroupHubMigration | MessageType::GroupDeliveryAck | MessageType::GroupHubHeartbeat => {
                self.handle_incoming_group(envelope)
            }
            // Backup types
            MessageType::BackupStore | MessageType::BackupDeliver | MessageType::BackupReplicate
            | MessageType::BackupReplicateAck | MessageType::BackupQuery
            | MessageType::BackupQueryResponse | MessageType::BackupConfirmDelivery => {
                self.handle_incoming_backup(&envelope)
            }
            MessageType::PeerAnnounce => {
                self.handle_peer_announce(&envelope)
            }
        }
    }
```

**Step 4: Tests + verification**

Run: `cargo test -p tom-protocol`
Expected: tous les tests passent

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): migrate handle_incoming (group + backup + announce) to RuntimeState"
```

---

## Task 9 : Migrer handle_send_message + handle_command

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Step 1: Implementer handle_send_message**

Point cle : utilise `SendWithBackupFallback` pour le cas offline.

```rust
    /// Envoyer un message chat — chiffre, signe, avec backup fallback.
    pub fn handle_send_message(
        &mut self,
        to: NodeId,
        payload: Vec<u8>,
    ) -> Vec<super::effect::RuntimeEffect> {
        // ... relay selection, encrypt, sign (copie de loop.rs:821-907)
        // ... retourne SendWithBackupFallback { envelope, on_success, on_failure }
    }
```

**Step 2: Implementer handle_command (dispatch toutes les commandes)**

```rust
    /// Dispatcher une commande de l'application.
    /// Retourne les effets, SAUF GetConnectedPeers et Shutdown
    /// qui sont geres directement dans la boucle.
    pub fn handle_command(&mut self, cmd: super::RuntimeCommand) -> Vec<super::effect::RuntimeEffect> {
        match cmd {
            RuntimeCommand::SendMessage { to, payload } => {
                self.subnets.record_communication(self.local_id, to, now_ms());
                self.handle_send_message(to, payload)
            }
            RuntimeCommand::SendGroupMessage { group_id, text } => {
                self.handle_send_group_message(group_id, text)
            }
            RuntimeCommand::SendReadReceipt { to, original_message_id } => {
                self.handle_send_read_receipt(to, original_message_id)
            }
            // Peer commands
            RuntimeCommand::AddPeer { node_id } => { ... Vec::new() }
            RuntimeCommand::UpsertPeer { info } => { ... Vec::new() }
            RuntimeCommand::RemovePeer { node_id } => { ... Vec::new() }
            // Group commands
            RuntimeCommand::CreateGroup { .. } => { ... group_actions_to_effects }
            RuntimeCommand::AcceptInvite { .. } => { ... }
            RuntimeCommand::DeclineInvite { .. } => { ... Vec::new() }
            RuntimeCommand::LeaveGroup { .. } => { ... }
            // Queries — handled in loop, not here
            RuntimeCommand::GetConnectedPeers { reply } => {
                // SPECIAL: needs transport, will be handled in loop
                Vec::new()
            }
            RuntimeCommand::GetGroups { reply } => {
                let groups = self.group_manager.all_groups().into_iter().cloned().collect();
                let _ = reply.send(groups);
                Vec::new()
            }
            RuntimeCommand::GetPendingInvites { reply } => {
                let invites = self.group_manager.pending_invites().into_iter().cloned().collect();
                let _ = reply.send(invites);
                Vec::new()
            }
            RuntimeCommand::Shutdown => Vec::new(), // handled in loop
        }
    }
```

**Step 3: Tests**

```rust
    #[test]
    fn handle_send_message_produit_fallback_effect() {
        let (alice_id, alice_seed) = keypair(10);
        let mut state = RuntimeState::new(alice_id, alice_seed, RuntimeConfig::default());

        let (bob_id, _) = keypair(20);
        let effects = state.handle_send_message(bob_id, b"hello".to_vec());

        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], RuntimeEffect::SendWithBackupFallback { .. }));
    }
```

**Step 4: Verifier**

Run: `cargo test -p tom-protocol`
Expected: tous les tests passent

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): migrate handle_command + handle_send_message to RuntimeState"
```

---

## Task 10 : Migrer handle_gossip_event

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Step 1: Implementer**

Convertit les GossipEvent (Received, NeighborUp, NeighborDown, Lagged) en effets.
NeighborUp declenche aussi `build_gossip_announce()` — retourne un effet special
ou gere dans la boucle.

Note: On utilise un type interne `GossipInput` plutot que `iroh_gossip::api::Event`
directement, pour eviter de leaker les types iroh dans l'API publique de state.

**Step 2: Tests**

```rust
    #[test]
    fn handle_gossip_neighbor_up_ajoute_peer() {
        let mut state = default_state(1);
        let peer = node_id(2);

        let effects = state.handle_gossip_neighbor_up(peer);

        assert!(state.topology.get(&peer).is_some(), "peer devrait etre dans la topology");
        assert!(effects.iter().any(|e| matches!(
            e, RuntimeEffect::Emit(super::super::ProtocolEvent::GossipNeighborUp { node_id }) if *node_id == peer
        )));
    }
```

**Step 3: Verifier + Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "refactor(runtime): migrate gossip event handling to RuntimeState"
```

---

## Task 11 : Creer `executor.rs` + reecrire `loop.rs`

**Files:**
- Create: `crates/tom-protocol/src/runtime/executor.rs`
- Rewrite: `crates/tom-protocol/src/runtime/loop.rs`
- Modify: `crates/tom-protocol/src/runtime/mod.rs`

C'est l'etape critique. La boucle est reecrite pour :
1. Creer un `RuntimeState` au lieu de variables locales
2. Chaque branche select! appelle `state.handle_*()` ou `state.tick_*()`
3. Les effets retournes sont passes a `execute_effects()`
4. Les cas speciaux (GetConnectedPeers, Shutdown, gossip broadcast) restent inline

**Step 1: Creer executor.rs**

Le `execute_effects()` fait un match sur chaque variant de `RuntimeEffect` et execute
l'I/O correspondante (transport.send_raw, msg_tx.send, event_tx.send...).

Inclut le cas `SendWithBackupFallback` avec test du resultat.

**Step 2: Reecrire loop.rs**

La nouvelle boucle fait ~120 lignes. La signature change :

```rust
pub(super) async fn runtime_loop<T: Transport>(
    transport: T,
    mut state: RuntimeState,
    mut cmd_rx: mpsc::Receiver<RuntimeCommand>,
    ...
)
```

**Step 3: Adapter ProtocolRuntime::spawn() dans mod.rs**

```rust
pub fn spawn(node: TomNode, config: RuntimeConfig) -> RuntimeChannels {
    let local_id = node.id();
    let secret_seed = node.secret_key_seed();
    let state = RuntimeState::new(local_id, secret_seed, config.clone());
    // ... channels ...
    tokio::spawn(r#loop::runtime_loop(node, state, ...));
    // ...
}
```

**Step 4: Verifier** (etape critique)

Run: `cargo test --workspace`
Expected: 261 tests Rust passent (aucune regression)

Run: `cargo clippy --workspace`
Expected: zero warning

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/runtime/
git commit -m "refactor(runtime): rewrite loop.rs as thin orchestrator + executor — 1366→~120 lines"
```

---

## Task 12 : Nettoyer — supprimer le code mort de loop.rs

**Files:**
- Modify: `crates/tom-protocol/src/runtime/loop.rs`

Supprimer toutes les anciennes fonctions helper qui sont maintenant dans `state.rs` :
- `handle_incoming_chat()`
- `handle_incoming_group()`
- `handle_incoming_backup()`
- `handle_send_message()`
- `handle_send_group_message()`
- `handle_send_read_receipt()`
- `deliver_backups_for_peer()`
- `execute_group_actions()`
- `execute_backup_actions()`
- `surface_group_event()`
- `surface_backup_event()`
- `surface_subnet_event()`
- `surface_role_action()`
- `group_payload_to_message_type()`
- `send_envelope()`
- `send_envelope_to()`

**Step 1: Supprimer**
**Step 2: Verifier**

Run: `cargo test --workspace`
Run: `cargo clippy --workspace`

**Step 3: Commit**

```bash
git add crates/tom-protocol/src/runtime/loop.rs
git commit -m "refactor(runtime): remove dead helper functions from loop.rs"
```

---

## Task 13 : Tests d'integration RuntimeState

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs` (bloc `#[cfg(test)]`)

Ecrire 8-10 tests supplementaires couvrant les scenarios cles :

1. `message_chiffre_decrypte_et_livre` — E2E encrypt/decrypt
2. `message_forward_via_relay` — relay fait Forward
3. `ack_met_a_jour_tracker` — ACK → status change
4. `read_receipt_met_a_jour_tracker` — ReadReceipt → status read
5. `send_message_produit_backup_fallback` — SendWithBackupFallback
6. `group_create_produit_send_effects` — create group → enveloppes
7. `heartbeat_offline_declenche_cleanup` — offline → subnet/role cleanup
8. `gossip_announce_produit_bytes` — build_gossip_announce() non-None

**Step 1: Ecrire les tests**
**Step 2: Verifier**

Run: `cargo test -p tom-protocol`
Expected: ~275+ tests

**Step 3: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "test(runtime): add unit tests for RuntimeState — pure logic coverage"
```

---

## Recapitulatif des commits

| # | Message | Fichiers |
|---|---------|----------|
| 1 | `refactor(runtime): add RuntimeEffect enum` | effect.rs, mod.rs, lib.rs |
| 2 | `refactor(runtime): add Transport trait + MockTransport` | transport.rs, mod.rs, Cargo.toml |
| 3 | `refactor(runtime): add RuntimeState struct` | state.rs, mod.rs, lib.rs |
| 4 | `refactor(runtime): migrate tick_cache/tracker_cleanup` | state.rs |
| 5 | `refactor(runtime): migrate tick_heartbeat + backup delivery` | state.rs |
| 6 | `refactor(runtime): migrate all tick methods` | state.rs |
| 7 | `refactor(runtime): migrate handle_incoming_chat` | state.rs |
| 8 | `refactor(runtime): migrate handle_incoming (group+backup)` | state.rs |
| 9 | `refactor(runtime): migrate handle_command + send_message` | state.rs |
| 10 | `refactor(runtime): migrate gossip event handling` | state.rs |
| 11 | `refactor(runtime): rewrite loop.rs as thin orchestrator` | executor.rs, loop.rs, mod.rs |
| 12 | `refactor(runtime): remove dead helper functions` | loop.rs |
| 13 | `test(runtime): add RuntimeState unit tests` | state.rs |
