# 2026-03-30 — Bootstrap pump skeleton (Copilot branch)

## Statut
Document de travail sur la branche `copilot/bootstrap-pump-skeleton`.

But : transformer les constats validés (Copilot + Claude) en **squelette d’implémentation concret**.

## Ce qui existe déjà

### Déjà branché
- `PeerPresent -> add_peer_addr() -> gossip.join_peers() -> AddPeer` existe déjà dans `crates/tom-protocol/src/runtime/loop.rs`.
- `DhtLookupResult -> add_peer_addr() -> gossip.join_peers()` existe déjà dans `crates/tom-protocol/src/runtime/loop.rs`.
- `MemoryLookup` existe déjà dans `crates/tom-transport/src/node.rs` et sert de point d’injection central.

### Lacune principale réelle
- `MdnsAddressLookup` existe dans `crates/tom-connect/src/address_lookup/mdns.rs`
- **mais il n’est ni activé dans la dépendance `tom-connect`, ni monté dans `TomNode`, ni consommé par `runtime/loop.rs`**.

Autrement dit :
- le chemin relay-assisted existe
- le chemin DHT lookup existe
- **le chemin LAN-first n’existe pas encore en production**

## Décision de squelette

Ne pas créer une grosse “méga couche bootstrap” tout de suite.

Créer au contraire un squelette minimal en 3 niveaux :

1. **niveau transport (`tom-transport`)**
   - expose les signaux d’amorçage locaux
2. **niveau runtime (`tom-protocol`)**
   - orchestre les phases et unifie les signaux
3. **niveau observabilité**
   - expose quelques événements/debug sans rendre le mécanisme visible à l’utilisateur final

## Cadencement recommandé

### Phase 0 — état actuel conservé
Sources déjà exploitées :
- `PeerPresent`
- `DhtLookupResult`
- `AddPeerAddr` manuel

### Phase 1 — LAN-first réel
Ajouter :
- `mDNS discovery -> add_peer_addr() -> gossip.join_peers()`

Objectif :
- zéro saisie manuelle sur réseau local
- Apple TV / Mac / NAS sur même LAN = auto-amorçage

### Phase 2 — phase bootstrap explicite mais interne
Ajouter une petite machine de phase **interne** :
- `LanProbe`
- `RelayAssist`
- `DhtAssist`
- `Converged`
- `Idle`

Objectif :
- éviter une soupe de signaux opportunistes
- cadencer les timeouts et la télémétrie

### Phase 3 — optimisation relay-assisted
Évaluer ensuite :
- `PEER_PRESENT_K = 16` puis éventuellement `32`
- instrumentation avant décision
- option `recently_seen` seulement si les mesures le justifient

## Types à introduire

## 1. `tom-transport/src/bootstrap.rs` (nouveau fichier)

Créer un petit module interne pour les signaux d’amorçage.

```rust
pub enum BootstrapHint {
    MdnsDiscovered {
        endpoint_addr: tom_connect::EndpointAddr,
    },
}
```

Version minimale :
- on ne met **que mDNS** ici au début
- `PeerPresent` reste là où il est déjà
- `DHT` reste là où il est déjà

Pourquoi minimal :
- ne pas refactorer trois pipelines en même temps
- brancher d’abord la seule vraie source manquante

## 2. `tom-protocol/src/runtime/bootstrap.rs` (nouveau fichier)

Créer un petit module runtime pour le pilotage interne.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapPhase {
    Idle,
    LanProbe,
    RelayAssist,
    DhtAssist,
    Converged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapSource {
    Mdns,
    PeerPresent,
    Dht,
    Manual,
}
```

Optionnel ensuite :

```rust
#[derive(Debug, Clone)]
pub enum BootstrapEvent {
    HintAccepted { source: BootstrapSource, node_id: NodeId },
    PhaseChanged { from: BootstrapPhase, to: BootstrapPhase },
    TimedOut { phase: BootstrapPhase },
}
```

Décision importante :
- **types internes d’abord**
- pas d’API publique figée trop tôt

## 3. Évolution optionnelle de `DiscoverySource`

`crates/tom-protocol/src/discovery/types.rs` contient déjà :
- `Direct`
- `Gossip`
- `Announce`
- `Dht`

Ne pas le casser tout de suite.

Au début :
- `BootstrapSource` reste interne au runtime
- on décidera plus tard si `DiscoverySource` doit gagner :
  - `Mdns`
  - `PeerPresent`

Pourquoi :
- éviter un churn API prématuré
- garder la compatibilité des consommateurs de `ProtocolEvent`

## Fichiers à toucher — version minimale utile

### 1. `crates/tom-transport/Cargo.toml`
Activer la feature mDNS de `tom-connect`.

Aujourd’hui :
```toml
tom-connect = { path = "../tom-connect" }
```

Cible minimale :
```toml
tom-connect = { path = "../tom-connect", features = ["address-lookup-mdns"] }
```

Sans ça, le plan mDNS est fictif.

### 2. `crates/tom-transport/src/config.rs`
Ajouter une option explicite :

```rust
pub(crate) enable_local_discovery: bool,
```

Default recommandé :
- `true` en dev/local
- ou `false` si on veut ultra-conservateur

Ma reco :
- `true` par défaut côté transport natif
- désactivable explicitement

Builder :
```rust
pub fn local_discovery(mut self, enabled: bool) -> Self
```

### 3. `crates/tom-transport/src/bootstrap.rs` (nouveau)
Contient :
- `BootstrapHint`
- helper de forward mDNS -> channel runtime-friendly

### 4. `crates/tom-transport/src/node.rs`
Ajouter :
- création de `MdnsAddressLookup` après `endpoint.bind()`
- `endpoint.address_lookup().add(mdns.clone())`
- souscription `mdns.subscribe().await`
- task de forward vers `mpsc::Receiver<BootstrapHint>`
- stockage dans `TomNode`
- méthode `take_bootstrap_hint_rx()`

Pattern concret attendu :

```rust
let mdns = MdnsAddressLookup::builder().build(endpoint.id())?;
endpoint.address_lookup().add(mdns.clone());
let mut mdns_stream = mdns.subscribe().await;
let (bootstrap_tx, bootstrap_rx) = mpsc::channel(64);

tokio::spawn(async move {
    while let Some(event) = mdns_stream.next().await {
        if let DiscoveryEvent::Discovered { endpoint_info, .. } = event {
            let _ = bootstrap_tx.send(BootstrapHint::MdnsDiscovered {
                endpoint_addr: endpoint_info.to_endpoint_addr(),
            }).await;
        }
    }
});
```

### 5. `crates/tom-protocol/src/runtime/bootstrap.rs` (nouveau)
Contient :
- `BootstrapPhase`
- `BootstrapSource`
- helpers de transition simples

Exemple :
- startup => `LanProbe`
- si hint mDNS accepté => `Converged`
- si timeout LAN => `RelayAssist`
- si aucun progrès ensuite => `DhtAssist`

### 6. `crates/tom-protocol/src/runtime/mod.rs`
Re-export des types bootstrap internes si utile au runtime.

Option debug plus tard :
- `ProtocolEvent::BootstrapStateChanged`
- `ProtocolEvent::BootstrapHintAccepted`

Mais **pas obligatoire en étape 1**.

### 7. `crates/tom-protocol/src/runtime/loop.rs`
Ajouter une nouvelle branche `tokio::select!` pour consommer `bootstrap_hint_rx`.

Chemin cible :
- `BootstrapHint::MdnsDiscovered { endpoint_addr }`
- `node.add_peer_addr(endpoint_addr).await`
- `gossip.join_peers(vec![endpoint_id]).await`
- `state.handle_command(RuntimeCommand::AddPeer { node_id })`

Et factoriser ce pattern dans un helper local pour éviter le copier-coller avec :
- `PeerPresent`
- `DhtLookupResult`
- futur `mDNS`

Helper recommandé :

```rust
async fn bootstrap_join_peer(
    node: &TomNode,
    gossip_sender: Option<&tom_gossip::net::GossipSender>,
    endpoint_addr: tom_connect::EndpointAddr,
    node_id: NodeId,
) {
    let endpoint_id = endpoint_addr.id;
    node.add_peer_addr(endpoint_addr).await;
    if let Some(sender) = gossip_sender {
        let _ = sender.join_peers(vec![endpoint_id]).await;
    }
}
```

### 8. `crates/tom-relay/src/server/clients.rs`
Pas de changement immédiat obligatoire.

Étape d’évaluation seulement :
- tester `PEER_PRESENT_K = 16`
- mesurer
- puis éventuellement `32`

Ne pas coupler ça à l’implémentation mDNS.

## Événements / observabilité

## Version minimale
Ne pas toucher `ProtocolEvent` tout de suite.

Utiliser d’abord :
- logs `tracing::info!`
- instrumentation de phase

## Version 2 possible
Ajouter ensuite :

```rust
ProtocolEvent::BootstrapStateChanged { phase: BootstrapPhase }
ProtocolEvent::BootstrapHintAccepted { source: BootstrapSource, node_id: NodeId }
```

Utile pour :
- `tom-tui`
- `tom-stress`
- diagnostics Apple TV / Mac / NAS

## Pourquoi je ne le mets pas en étape 1
Parce que le vrai objectif immédiat est :
- faire marcher le LAN-first
- unifier le pipeline de join
- réduire le risque API

## Tests à ajouter

### `tom-transport`
- bind avec `local_discovery(false)` -> pas de bootstrap channel mDNS
- bind avec `local_discovery(true)` -> channel bootstrap présent
- conversion `DiscoveryEvent::Discovered -> BootstrapHint::MdnsDiscovered`

### `tom-protocol`
- test runtime : `BootstrapHint::MdnsDiscovered` injecte bien l’adresse puis `join_peers()`
- test de phase : `LanProbe -> Converged` sur premier hint utile
- test de timeout : `LanProbe -> RelayAssist` sans hint

### `tom-integration-tests`
Ajouter plus tard un test du type :
- deux nodes même LAN / relay disabled / local discovery enabled
- découverte mutuelle sans bootstrap manuel

## Séquence d’implémentation recommandée

### Lot 1 — plomberie minimale
1. activer la feature `address-lookup-mdns`
2. ajouter `enable_local_discovery` à `TomNodeConfig`
3. créer `tom-transport/src/bootstrap.rs`
4. forwarder mDNS -> `BootstrapHint`
5. exposer `take_bootstrap_hint_rx()`

### Lot 2 — runtime unifié
6. ajouter `runtime/bootstrap.rs`
7. consommer `bootstrap_hint_rx` dans `runtime/loop.rs`
8. factoriser `add_peer_addr() + join_peers()` dans un helper local
9. journaliser la phase bootstrap

### Lot 3 — mesure relay-assisted
10. benchmark simple sur `PEER_PRESENT_K = 8 / 16 / 32`
11. décider si `k` augmente
12. décider si `recently_seen` vaut la peine

## Synthèse courte
Le plus petit chemin utile est :

- **ne pas refaire PeerPresent**
- **ne pas partir sur DHT topic maintenant**
- **brancher mDNS pour de vrai**
- **introduire une mini phase bootstrap interne**
- **unifier les chemins d’injection d’adresse**

En une phrase :

> la prochaine vraie avancée n’est pas une nouvelle théorie de bootstrap, c’est un `bootstrap_hint_rx` alimenté par mDNS dans `TomNode`, consommé par `runtime/loop.rs`, avec une petite machine de phase interne et zéro impact utilisateur.
