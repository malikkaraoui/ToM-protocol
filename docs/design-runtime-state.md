# Design : Refactoring RuntimeState (Approche A)

## Le probleme

Aujourd'hui, **toute la logique du protocole vit dans une seule fonction** : `runtime_loop()`.

Cette fonction fait **1366 lignes**. Elle recoit **11 parametres**. Elle contient :
- 11 sous-systemes (router, topology, tracker, heartbeat, groups, backup, subnets, roles...)
- 11 branches `select!` (transport, commandes, timers, gossip...)
- 17 fonctions helper (handle_incoming_chat, handle_send_message, etc.)

Le probleme concret :
- **Impossible de tester unitairement** : chaque handler a besoin du vrai `TomNode` (reseau)
- **Impossible de tester un sous-systeme isolement** : tout est enchevetre
- **Chaque modif touche la god function** : risque de casser autre chose

## L'idee centrale

**Separer la reflexion de l'action.**

Aujourd'hui, quand un message arrive, le code fait tout d'un coup :
1. Decide quoi faire (router, decrypter, verifier...)
2. Envoie des bytes sur le reseau (`node.send_raw()`)
3. Envoie des events aux channels (`event_tx.send()`)

Le probleme : les etapes 2 et 3 sont de l'I/O (reseau, channels). On ne peut pas
les tester sans un vrai noeud iroh qui tourne.

**La solution** : les methodes de logique ne font QUE l'etape 1. Elles retournent
une **liste d'intentions** (qu'on appelle `RuntimeEffect`). C'est la boucle principale
qui execute ensuite ces intentions.

## Analogie simple

Pense a un chef de cuisine et un serveur :

**Aujourd'hui** : Le chef cuisine ET sert les plats lui-meme. Pour tester sa cuisine,
il faut un vrai restaurant avec des vrais clients.

**Apres refactoring** : Le chef cuisine et pose les plats sur le passe. Le serveur
les emporte. Pour tester la cuisine, on regarde juste ce qui est pose sur le passe,
pas besoin de clients.

- **Le chef** = `RuntimeState` (logique pure, prend des decisions)
- **Le passe** = `Vec<RuntimeEffect>` (liste d'intentions)
- **Le serveur** = `execute_effects()` (execute les intentions sur le reseau)

## Architecture visuelle

```
AVANT (tout melange) :
┌──────────────────────────────────────────────┐
│  runtime_loop() — 1366 lignes                │
│                                              │
│  Logique + Reseau + Channels = tout melange  │
│  Non testable sans vrai TomNode              │
└──────────────────────────────────────────────┘

APRES (separe) :
┌────────────────────────────────────────────────────┐
│  runtime_loop() — ~120 lignes                      │
│  Role : orchestrateur mince                        │
│  • Ecoute les events (select!)                     │
│  • Appelle state.handle_*() → recoit des effets    │
│  • Passe les effets a execute_effects()            │
└───────────┬──────────────────────┬─────────────────┘
            │                      │
            ▼                      ▼
┌───────────────────┐   ┌──────────────────────┐
│  RuntimeState     │   │  execute_effects()   │
│                   │   │                      │
│  Contient :       │   │  Recoit les effets   │
│  - router         │   │  et les execute :    │
│  - topology       │   │  - SendEnvelope      │
│  - tracker        │   │    → node.send_raw() │
│  - heartbeat      │   │  - DeliverMessage    │
│  - group_manager  │   │    → msg_tx.send()   │
│  - group_hub      │   │  - Emit(event)       │
│  - backup         │   │    → event_tx.send() │
│  - subnets        │   │                      │
│  - role_manager   │   │  Utilise le trait     │
│                   │   │  Transport pour le    │
│  Methodes pures : │   │  reseau               │
│  Pas de reseau    │   └──────────┬───────────┘
│  Pas de channels  │              │
│  Pas d'async      │              ▼
│  Retourne des     │   ┌──────────────────────┐
│  Vec<Effect>      │   │  Transport (trait)   │
│                   │   │                      │
│  TESTABLE !       │   │  En production :     │
└───────────────────┘   │    → TomNode (iroh)  │
                        │  En test :           │
                        │    → MockTransport   │
                        │      (enregistre les │
                        │       envois)        │
                        └──────────────────────┘
```

## Les 4 nouveaux fichiers

### 1. `effect.rs` — Les intentions

Un enum qui represente tout ce que la logique peut "vouloir faire" :

```rust
pub enum RuntimeEffect {
    /// Envoyer une enveloppe (le premier hop est calcule automatiquement)
    SendEnvelope(Envelope),

    /// Envoyer une enveloppe a un noeud precis (hop explicite)
    SendEnvelopeTo { target: NodeId, envelope: Envelope },

    /// Livrer un message decrypte a l'application (TUI, bot...)
    DeliverMessage(DeliveredMessage),

    /// Notifier un changement de statut (pending → sent → relayed → delivered)
    StatusChange(StatusChange),

    /// Emettre un evenement protocole (peer offline, group created, etc.)
    Emit(ProtocolEvent),

    /// Essayer d'envoyer — si ca echoue, executer le plan B (backup)
    SendWithBackupFallback {
        envelope: Envelope,
        on_success: Vec<RuntimeEffect>,  // ex: marquer "sent"
        on_failure: Vec<RuntimeEffect>,  // ex: stocker en backup + event erreur
    },
}
```

**Pourquoi `SendWithBackupFallback` ?**
Quand on envoie un message chat, si le destinataire est hors ligne, on doit stocker
le message en backup. Mais la logique pure ne sait pas si l'envoi va reussir ou non
(c'est du reseau). Donc elle dit : "essaie d'envoyer, et si ca rate, voila quoi faire".
L'executeur teste le resultat et applique le bon plan.

### 2. `transport.rs` — L'abstraction reseau

```rust
/// Ce que le runtime a besoin du reseau.
/// En production : TomNode (iroh, QUIC, vraies connexions)
/// En test : MockTransport (enregistre tout, ne fait rien)
pub trait Transport: Send {
    async fn send_raw(&self, target: NodeId, data: &[u8]) -> Result<(), String>;
    async fn recv_raw(&mut self) -> Result<(NodeId, Vec<u8>), String>;
    async fn connected_peers(&self) -> Vec<NodeId>;
    async fn shutdown(self) -> Result<(), String>;
}
```

Le `MockTransport` pour les tests :

```rust
/// Faux transport qui enregistre les envois pour verification
pub struct MockTransport {
    /// Chaque appel a send_raw() est enregistre ici
    pub sent: Vec<(NodeId, Vec<u8>)>,
    /// File de messages a "recevoir" (injectes par le test)
    pub incoming: VecDeque<(NodeId, Vec<u8>)>,
}
```

### 3. `state.rs` — Le cerveau (logique pure)

Le struct qui contient TOUT l'etat du protocole :

```rust
pub struct RuntimeState {
    // Identite
    local_id: NodeId,
    secret_seed: [u8; 32],
    config: RuntimeConfig,

    // Modules protocole
    router: Router,              // Decide : livrer / relayer / rejeter
    relay_selector: RelaySelector, // Choisit le meilleur relai
    topology: Topology,          // Carte des peers connus
    tracker: MessageTracker,     // Suivi des statuts (sent/relayed/delivered)
    heartbeat: HeartbeatTracker, // Detection online/offline

    // Groupes
    group_manager: GroupManager, // Cote membre (mes groupes)
    group_hub: GroupHub,         // Cote hub (groupes que j'heberge)

    // Backup
    backup: BackupCoordinator,   // Messages virus pour peers offline

    // Decouverte
    subnets: EphemeralSubnetManager, // Sous-reseaux auto-formes
    role_manager: RoleManager,       // Promotion/demotion Peer ↔ Relay
    local_roles: Vec<PeerRole>,      // Nos roles actuels (pour gossip)
}
```

**Ses methodes — toutes pures, toutes synchrones :**

```rust
impl RuntimeState {
    // === Arrivee de donnees ===

    /// Un message arrive du reseau. Parse, verifie, route.
    /// Retourne les effets a executer (livrer, relayer, ACK, etc.)
    pub fn handle_incoming(&mut self, raw_data: &[u8]) -> Vec<RuntimeEffect>

    // === Commandes de l'application ===

    /// L'appli veut envoyer un message. Construit l'enveloppe, chiffre, signe.
    pub fn handle_send_message(&mut self, to: NodeId, payload: Vec<u8>) -> Vec<RuntimeEffect>

    /// L'appli veut envoyer un message groupe. Route vers le hub.
    pub fn handle_send_group_message(&mut self, group_id: GroupId, text: String) -> Vec<RuntimeEffect>

    /// L'appli veut envoyer un accuse de lecture.
    pub fn handle_send_read_receipt(&mut self, to: NodeId, msg_id: String) -> Vec<RuntimeEffect>

    /// Commandes de gestion des peers (add, upsert, remove).
    pub fn handle_peer_command(&mut self, cmd: RuntimeCommand) -> Vec<RuntimeEffect>

    /// Commandes groupe (create, accept invite, leave...).
    pub fn handle_group_command(&mut self, cmd: RuntimeCommand) -> Vec<RuntimeEffect>

    // === Timers periodiques ===

    /// Nettoyage cache du router (toutes les 5 min)
    pub fn tick_cache_cleanup(&mut self) -> Vec<RuntimeEffect>

    /// Eviction des messages expires du tracker (toutes les 5 min)
    pub fn tick_tracker_cleanup(&mut self) -> Vec<RuntimeEffect>

    /// Verification heartbeat : qui est online/offline ? (toutes les 5s)
    /// Si un peer revient online → livre ses backups
    pub fn tick_heartbeat(&mut self) -> Vec<RuntimeEffect>

    /// Heartbeat du hub groupe (toutes les 30s)
    pub fn tick_group_hub_heartbeat(&mut self) -> Vec<RuntimeEffect>

    /// Maintenance backup : expiration TTL, replication (toutes les 60s)
    pub fn tick_backup(&mut self) -> Vec<RuntimeEffect>

    /// Evaluation des sous-reseaux ephemeres (toutes les 30s)
    pub fn tick_subnets(&mut self) -> Vec<RuntimeEffect>

    /// Evaluation des roles : promotion/demotion (toutes les 60s)
    pub fn tick_roles(&mut self) -> Vec<RuntimeEffect>

    // === Gossip ===

    /// Un event gossip arrive (peer annonce, neighbor up/down)
    pub fn handle_gossip_event(&mut self, event: GossipEvent) -> Vec<RuntimeEffect>

    /// Construit l'annonce gossip periodique (toutes les 10s)
    /// Retourne les bytes a broadcaster (pas un RuntimeEffect car c'est gossip, pas QUIC)
    pub fn build_gossip_announce(&self) -> Option<Vec<u8>>
}
```

### 4. `executor.rs` — Le bras (execute les intentions)

```rust
/// Prend une liste d'effets et les execute concretement.
/// C'est le seul endroit qui touche au reseau et aux channels.
async fn execute_effects<T: Transport>(
    effects: Vec<RuntimeEffect>,
    transport: &T,
    msg_tx: &Sender<DeliveredMessage>,   // vers l'appli : messages recus
    status_tx: &Sender<StatusChange>,    // vers l'appli : changements de statut
    event_tx: &Sender<ProtocolEvent>,    // vers l'appli : events protocole
) {
    for effect in effects {
        match effect {
            RuntimeEffect::SendEnvelope(envelope) => {
                // Calcule le premier hop (relai ou direct)
                let target = envelope.via.first().copied().unwrap_or(envelope.to);
                // Serialise et envoie sur le reseau
                if let Ok(bytes) = envelope.to_bytes() {
                    if let Err(e) = transport.send_raw(target, &bytes).await {
                        // Erreur reseau → notifie l'appli
                        let _ = event_tx.send(ProtocolEvent::Error { ... }).await;
                    }
                }
            }

            RuntimeEffect::DeliverMessage(msg) => {
                // Message decrypte pret → envoie a l'appli (TUI, bot...)
                let _ = msg_tx.send(msg).await;
            }

            RuntimeEffect::StatusChange(change) => {
                // pending → sent → relayed → delivered → read
                let _ = status_tx.send(change).await;
            }

            RuntimeEffect::Emit(event) => {
                // Evenement protocole (peer offline, group created, etc.)
                let _ = event_tx.send(event).await;
            }

            RuntimeEffect::SendWithBackupFallback { envelope, on_success, on_failure } => {
                let target = envelope.via.first().copied().unwrap_or(envelope.to);
                if let Ok(bytes) = envelope.to_bytes() {
                    if transport.send_raw(target, &bytes).await.is_ok() {
                        // Envoi reussi → execute les effets "succes"
                        execute_effects(on_success, transport, msg_tx, status_tx, event_tx).await;
                    } else {
                        // Envoi echoue → execute les effets "echec" (backup)
                        execute_effects(on_failure, transport, msg_tx, status_tx, event_tx).await;
                    }
                }
            }
            // ...
        }
    }
}
```

## Exemple concret : cycle de vie d'un message entrant

```
1. Un message arrive du reseau (bytes bruts via QUIC)
   │
   ▼
2. La boucle select! le recoit dans la branche transport
   │
   ▼
3. Appelle state.handle_incoming(raw_bytes)
   │
   ├── Parse le MessagePack → Envelope
   ├── Verifie la signature Ed25519
   ├── Enregistre le heartbeat du sender
   ├── Si c'est pour nous : decrypte (XChaCha20-Poly1305)
   ├── Le Router decide : Deliver / Forward / Reject
   │
   ▼
4. Retourne un Vec<RuntimeEffect>, par exemple :
   [
     DeliverMessage { from: alice, payload: "salut", ... },
     SendEnvelope(ack_envelope),   // ACK de reception
   ]
   │
   ▼
5. execute_effects() prend la liste :
   ├── DeliverMessage → msg_tx.send() → le TUI affiche "salut"
   └── SendEnvelope   → node.send_raw() → ACK part vers Alice
```

## Exemple concret : envoi avec backup

```
1. L'appli veut envoyer "hello" a Bob (qui est peut-etre offline)
   │
   ▼
2. state.handle_send_message(bob_id, "hello")
   │
   ├── relay_selector choisit le meilleur relai
   ├── Chiffre avec la cle publique de Bob
   ├── Signe avec notre cle privee
   ├── Prepare le backup au cas ou
   │
   ▼
3. Retourne :
   SendWithBackupFallback {
     envelope: <l'enveloppe chiffree+signee>,
     on_success: [StatusChange(pending → sent)],
     on_failure: [
       Emit(Error("Bob offline, message backup")),
       Emit(BackupStored { message_id, recipient: bob }),
     ],
   }
   │
   ▼
4. execute_effects() essaie d'envoyer :
   ├── Si OK  → execute on_success → statut passe a "sent"
   └── Si KO  → execute on_failure → message stocke en backup
                                      (sera re-livre quand Bob revient)
```

## La boucle apres refactoring (~120 lignes)

```rust
loop {
    // 1. Ecouter les events (selecteur asynchrone)
    let effects = tokio::select! {
        // Donnees du reseau
        result = transport.recv_raw() => {
            match result {
                Ok((_from, data)) => state.handle_incoming(&data),
                Err(e) => vec![RuntimeEffect::Emit(error_event(e))],
            }
        }
        // Commandes de l'application
        Some(cmd) = cmd_rx.recv() => state.handle_command(cmd),

        // Timers
        _ = heartbeat_check.tick()   => state.tick_heartbeat(),
        _ = backup_tick.tick()       => state.tick_backup(),
        _ = subnet_eval.tick()       => state.tick_subnets(),
        _ = role_eval.tick()         => state.tick_roles(),
        // ... etc pour chaque timer

        // Gossip
        event = gossip_next() => state.handle_gossip_event(event),

        else => break,
    };

    // 2. Executer les intentions
    execute_effects(effects, &transport, &msg_tx, &status_tx, &event_tx).await;
}
```

Compare avec les 1366 lignes actuelles. Meme comportement, mais :
- **La logique** est dans `state.rs` (testable)
- **L'I/O** est dans `executor.rs` (5 match arms)
- **L'orchestration** est dans `loop.rs` (~120 lignes, lisible)

## Comment on teste (sans reseau)

```rust
#[test]
fn message_arrive_est_decrypte_et_livre() {
    // Cree deux identites
    let (alice_id, alice_seed) = test_keypair();
    let (bob_id, bob_seed) = test_keypair();

    // Cree le RuntimeState de Bob
    let mut state = RuntimeState::new(bob_id, bob_seed, RuntimeConfig::default());

    // Alice construit un message chiffre pour Bob
    let envelope = EnvelopeBuilder::new(
        alice_id, bob_id, MessageType::Chat, b"salut bob".to_vec()
    ).encrypt_and_sign(&alice_seed, &bob_id.as_bytes()).unwrap();

    let raw = envelope.to_bytes().unwrap();

    // Bob traite le message
    let effects = state.handle_incoming(&raw);

    // Verifie : 2 effets (livraison + ACK)
    assert_eq!(effects.len(), 2);

    // Effet 1 : le message est decrypte et pret a afficher
    match &effects[0] {
        RuntimeEffect::DeliverMessage(msg) => {
            assert_eq!(msg.from, alice_id);
            assert_eq!(msg.payload, b"salut bob");
            assert!(msg.was_encrypted);
            assert!(msg.signature_valid);
        }
        _ => panic!("attendu DeliverMessage"),
    }

    // Effet 2 : un ACK est pret a envoyer a Alice
    assert!(matches!(&effects[1], RuntimeEffect::SendEnvelope(_)));
}

#[test]
fn heartbeat_detecte_peer_offline() {
    let (local_id, seed) = test_keypair();
    let mut state = RuntimeState::new(local_id, seed, RuntimeConfig::default());

    // Ajoute un peer
    let (peer_id, _) = test_keypair();
    state.topology.upsert(PeerInfo {
        node_id: peer_id,
        role: PeerRole::Peer,
        status: PeerStatus::Online,
        last_seen: now_ms(),
    });
    state.heartbeat.record_heartbeat(peer_id);

    // Simule 45s sans heartbeat (seuil offline)
    // ... avance le temps ...

    let effects = state.tick_heartbeat();

    // Devrait emettre PeerOffline
    assert!(effects.iter().any(|e| matches!(
        e, RuntimeEffect::Emit(ProtocolEvent::PeerOffline { node_id }) if *node_id == peer_id
    )));
}
```

**Zero reseau. Zero TomNode. Zero iroh. Tests purs et rapides.**

## Ce qui ne change PAS

- **`mod.rs`** : RuntimeConfig, RuntimeCommand, RuntimeHandle, RuntimeChannels, ProtocolEvent,
  DeliveredMessage — tout ca reste identique. L'API externe ne bouge pas.
- **Le TUI et le bot** : ils utilisent RuntimeHandle/RuntimeChannels, qui ne changent pas.
- **Les 261 tests existants** : ils testent les modules individuels (Router, GroupManager, etc.)
  qui ne sont pas modifies. Ils continuent de passer.

## Ordre de migration (12 etapes)

| # | Etape | Risque | Verification |
|---|-------|--------|--------------|
| 1 | Creer `effect.rs` (juste l'enum) | Zero | `cargo check` |
| 2 | Creer `transport.rs` (trait + impl TomNode) | Faible | `cargo check` |
| 3 | Creer `state.rs` (struct + new()) | Zero | `cargo check` |
| 4 | Migrer `tick_cache_cleanup` (trivial, 1 ligne) | Zero | `cargo test` |
| 5 | Migrer `tick_tracker_cleanup` (trivial) | Zero | `cargo test` |
| 6 | Migrer `handle_incoming_chat` (le plus gros) | Moyen | `cargo test` |
| 7 | Migrer `handle_incoming_group` | Moyen | `cargo test` |
| 8 | Migrer `handle_incoming_backup` | Moyen | `cargo test` |
| 9 | Migrer `handle_send_message` + backup fallback | Moyen | `cargo test` |
| 10 | Migrer tous les timers restants | Faible | `cargo test` |
| 11 | Creer `executor.rs` + reecrire `loop.rs` | Eleve | `cargo test` complet |
| 12 | Ecrire les tests unitaires RuntimeState | Zero | Ajout de couverture |

**Chaque etape compile. Chaque etape est verifiable independamment.**

## Risques identifies et parades

| Risque | Explication | Parade |
|--------|-------------|--------|
| `handle_send_message` a besoin du resultat reseau | Il doit savoir si l'envoi echoue pour activer le backup | `SendWithBackupFallback` : effet en 2 phases (succes/echec) |
| `GetConnectedPeers` a besoin du transport | C'est une query vers le reseau, pas de la logique pure | Reste dans la boucle, cas special avec `oneshot` reply |
| Gossip announce retourne des bytes, pas des effets | Le gossip broadcast utilise un channel separe (pas QUIC) | Gere en inline dans la branche select!, pas via effects |
| Casser les tests d'integration existants | Les stress tests et E2E utilisent `ProtocolRuntime::spawn()` | Migration methode par methode, `cargo test` apres chaque etape |
| `deliver_backups_for_peer` est complexe | Boucle sur les backups + envoi + confirmation | Decompose en 2 methodes : `prepare_backup_delivery()` → effets, execute par le loop |
