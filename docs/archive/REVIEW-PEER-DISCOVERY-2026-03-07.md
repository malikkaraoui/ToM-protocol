# Review critique — découverte automatique de pairs via relay

Date : 2026-03-07
Auteur : GitHub Copilot
Sujet : évaluation du plan "Relay-Assisted Gossip Discovery (PeerPresent)"

## Résumé exécutif

Le plan proposé part d'une **bonne intuition stratégique** : la découverte automatique des pairs présents sur un même relay est bien une des premières briques concrètes de l'autonomie du réseau.

En revanche, **le plan d'implémentation proposé est incomplet et, en l'état, incorrect sur un point d'intégration critique** : il ne branche pas correctement l'information de présence relay vers le mécanisme réel utilisé par `tom-gossip` pour dialer des peers.

Verdict sans ambigüité :

- **Idée validée** : oui.
- **Implémentation telle quelle invalidée** : oui.
- **Plan à faire évoluer avant codage** : impératif.

---

## Position ferme

### Ce qui est juste

- Le relay est un point légitime pour fournir un **hint initial de présence**.
- La découverte relay-locale est une vraie brique d'indépendance.
- `HyParView` / `Plumtree` sont ensuite les bons moteurs de propagation organique.
- L'absence de mécanisme de découverte entre pairs présents sur un même relay est aujourd'hui un manque réel.

### Ce qui est faux ou incomplet

- Ajouter `PeerPresent` puis appeler `node.add_peer_addr(...)` + `gossip.join_peers(...)` **ne suffit pas** pour faire fonctionner le gossip.
- Supprimer immédiatement `gossip_bootstrap_peers` et tous les fallbacks est **prématuré et risqué**.
- Faire un broadcast intégral de présence à tous les clients d'un relay est une **mauvaise décision de surface d'exposition** et une **mauvaise habitude de design réseau**.
- Le rollout est sous-estimé : le protocole relay actuel n'est **pas tolérant** à l'apparition d'un nouveau `FrameType` côté vieux client.

---

## Constat technique principal : le plan ne nourrit pas le bon pipeline

## Problème critique n°1 — mauvais point d'injection de l'adresse

Le défaut majeur du plan est ici :

1. `PeerPresent` arrive côté client relay.
2. Le runtime construit un `EndpointAddr`.
3. Le runtime appelle `node.add_peer_addr(addr)`.
4. Le runtime appelle `gossip_sender.join_peers(vec![endpoint_id])`.

**Le problème :** `tom-gossip` ne se connecte pas via le `ConnectionPool` de `tom-transport`.

### Réalité du code

Dans `crates/tom-transport/src/node.rs` :

- `TomNode::add_peer_addr()` ne fait qu'alimenter le `ConnectionPool`.

Dans `crates/tom-transport/src/connection.rs` :

- le `ConnectionPool` stocke des `EndpointAddr` pour les connexions transport/protocole.
- ce pool est utilisé par `send()` / `send_raw()` du transport.

Dans `crates/tom-gossip/src/net.rs` :

- le dialer gossip fait `endpoint.connect(endpoint_id, &alpn)`.
- il ne passe pas par `TomNode`.
- il ne consulte pas le `ConnectionPool`.

### Conclusion

`node.add_peer_addr(addr)` aide le **transport protocolaire**, mais **pas le dial du gossip**.

Donc, dans le plan actuel :

- le runtime dit au gossip de rejoindre un peer,
- mais le gossip n'a aucune garantie de connaître une adresse exploitable pour ce `EndpointId`.

Autrement dit :

> Le plan injecte l'information au mauvais endroit.

---

## Problème critique n°2 — il faut alimenter le lookup de l'`Endpoint`

La vraie cible d'intégration n'est pas seulement `TomNode`, mais le **`address_lookup` attaché à l'`Endpoint`**.

### Pourquoi

`endpoint.connect(endpoint_id, ...)` dépend du système de résolution d'adresse de `tom-connect`.

Le bon endroit pour rendre un peer joignable par `EndpointId` est donc :

- un `MemoryLookup`, ou
- un lookup équivalent injecté dans `Endpoint::builder().address_lookup(...)`

### Fait important déjà présent dans le repo

Le gossip possède déjà son propre annuaire léger :

- `crates/tom-gossip/src/net/address_lookup.rs`
- `GossipAddressLookup`

Et `tom-gossip` l'ajoute déjà à l'endpoint dans `spawn()`.

Cela confirme la lecture architecturelle suivante :

> Dans cet écosystème, la bonne façon de rendre un peer dialable par `EndpointId` est bien de nourrir le `address_lookup` de l'endpoint.

### Ce qu'il faut faire

Dans `tom-transport`, il faut introduire un lookup mémoire partagé avec l'`Endpoint`, par exemple :

- créer un `MemoryLookup` dans `TomNode::bind()`
- l'ajouter au builder de `Endpoint`
- le stocker dans `TomNode`
- faire évoluer `TomNode::add_peer_addr()` pour qu'il mette à jour à la fois :
  - le `ConnectionPool`
  - **et** ce `MemoryLookup`

Ainsi :

- le transport peut envoyer,
- le gossip peut dialer,
- et les deux couches utilisent enfin une source cohérente.

---

## Problème critique n°3 — le plan veut supprimer trop tôt les fallbacks

Claude propose de supprimer immédiatement :

- `gossip_bootstrap_peers`
- `RuntimeConfigFFI.gossip_bootstrap_peers`
- `bootstrapPeerId` dans tvOS
- `TOM_BOOTSTRAP_PEER` dans `tom-tui`

C'est une erreur de séquencement.

### Pourquoi c'est prématuré

`PeerPresent` ne couvre que le cas :

- même relay,
- même surface de visibilité,
- même îlot de connectivité relay.

Cela ne couvre pas entièrement :

- les pairs sur des relays distincts,
- les scénarios de transition,
- les cas de rollout partiel,
- les régressions terrain,
- les environnements où la découverte relay ne donne aucune information utile.

### Position recommandée

Conserver pour l'instant :

- `gossip_bootstrap_peers` comme fallback contrôlé,
- `TOM_BOOTSTRAP_PEER` comme override de debug / seed de secours,
- la config tvOS équivalente tant que la validation terrain n'est pas faite.

La suppression ne doit intervenir **qu'après** validation :

1. en local,
2. en intégration,
3. sur hardware réel,
4. et après observation de sessions multi-relay.

---

## Problème critique n°4 — compatibilité protocolaire sous-estimée

Le plan dit en substance :

- vieux client sur nouveau relay = on contrôle tout, donc update ensemble.

Ce n'est pas une analyse suffisante.

### Réalité du code

Dans `crates/tom-relay/src/client/conn.rs` :

- `RelayToClientMsg::from_bytes(...)` échoue si un nouveau frame type apparaît.

Dans `crates/tom-relay/src/protos/common.rs` :

- un `FrameType` inconnu donne `UnknownFrameType`.

Dans `crates/tom-relay/src/protos/relay.rs` :

- un type inattendu donne `InvalidFrameType`.

### Conséquence

Un vieux client face à un nouveau frame `PeerPresent` ne fera pas forcément "ignoré, continuons".
Il peut casser la lecture du flux relay.

### Conclusion

Il faut assumer explicitement l'une des deux stratégies :

#### Option A — rollout atomique

- update client + relay ensemble,
- activation du feature seulement quand tous les clients critiques sont à jour.

#### Option B — compat d'abord

- rendre le client tolérant aux frames inconnus,
- seulement ensuite introduire le nouveau frame côté relay.

En l'état du code, **l'option A est la plus réaliste à court terme**, mais elle doit être écrite noir sur blanc.

---

## Problème critique n°5 — le relay ne doit pas devenir un annuaire global implicite

Le plan propose :

- broadcast `PeerPresent(new)` à tous les clients existants,
- envoyer tous les `PeerPresent(existing)` au nouveau client.

Techniquement faisable. Architectuellement discutable.

### Pourquoi c'est mauvais

Ce design transforme le relay en :

- oracle de présence,
- quasi-annuaire central local,
- source de topologie trop riche.

Il y a alors une différence entre :

- "le relay aide au bootstrap de quelques voisins"
- et
- "le relay révèle l'ensemble des pairs présents"

La seconde version est trop centralisante dans son comportement, même si le relay reste stateless pour les messages.

### Recommandation ferme

Ne pas faire de full broadcast.

Faire un **échantillonnage borné**.

Exemple raisonnable :

- à la connexion d'un peer, sélectionner `k` pairs existants (par ex. 4, 8, 16),
- envoyer ces `PeerPresent` au nouveau client,
- éventuellement notifier le nouveau peer à ce même sous-ensemble.

### Pourquoi c'est meilleur

- compatible avec l'esprit HyParView,
- réduit la fuite de topologie,
- réduit les bursts O(n),
- suffit pour amorcer le mesh,
- reste une vraie brique d'indépendance sans dériver vers un mini-registre central.

---

## Observations complémentaires sur le code existant

## 1. Le pattern `EndpointGone` ne doit pas être copié aveuglément

Le framing / proto est un bon modèle.

Mais la sémantique métier existante autour de `EndpointGone` ne doit pas être considérée comme vérité absolue sans audit plus poussé.

Donc :

- oui pour copier la structure proto,
- non pour présumer que toute la logique serveur associée est le meilleur template.

## 2. `endpoints_present` existe déjà côté `ActiveRelayActor`

C'est un bon point d'ancrage.

Le nouveau frame peut naturellement :

- insérer l'`EndpointId` dans `endpoints_present`,
- émettre un événement `(EndpointId, RelayUrl)`.

## 3. Le gossip sait déjà enrichir le lookup d'adresse

Le pattern existe déjà dans :

- `crates/tom-gossip/src/net.rs`
- `OutEvent::PeerData(...)`
- `self.address_lookup.add(endpoint_addr)`

C'est une confirmation forte que la bonne intégration passe par le lookup.

---

## Plan corrigé recommandé

## Étape 1 — introduire un hint relay minimal

Dans `tom-relay` :

- ajouter un nouveau frame serveur → client,
- payload minimal : `EndpointId`,
- pas besoin d'envoyer le `RelayUrl` dans le frame : le client connaît déjà l'URL du relay actif.

Nom possible :

- `PeerPresent` si on garde la sémantique proposée,
- ou mieux `PeerHint` si on veut assumer explicitement la logique d'échantillonnage.

## Étape 2 — diffusion bornée, pas broadcast global

Dans `Clients::register()` :

- sélectionner un sous-ensemble borné de peers déjà connectés,
- notifier ce sous-ensemble au nouveau client,
- optionnellement notifier le nouveau pair à ce même sous-ensemble.

Éviter absolument le full mesh d'annonce à l'enregistrement.

## Étape 3 — remontée côté `tom-connect`

Dans `ActiveRelayActor` :

- traiter `PeerPresent`,
- `endpoints_present.insert(endpoint_id)`,
- émettre `(EndpointId, RelayUrl)` dans un channel dédié.

Cette partie du plan initial est globalement bonne.

## Étape 4 — lookup mémoire dans `tom-transport`

Dans `TomNode::bind()` :

- créer un `MemoryLookup`,
- l'ajouter au builder de l'endpoint,
- le conserver dans `TomNode`.

Faire évoluer `TomNode::add_peer_addr()` pour :

- mettre à jour `ConnectionPool`,
- mettre à jour `MemoryLookup`.

C'est le pivot nécessaire pour que le gossip puisse réellement dialer via `EndpointId`.

## Étape 5 — runtime protocol

Dans `runtime_loop` :

à réception d'un événement relay peer-present :

1. construire `EndpointAddr::new(endpoint_id).with_relay_url(relay_url)`
2. `node.add_peer_addr(addr).await`
3. `gossip_sender.join_peers(vec![endpoint_id]).await`
4. `state.handle_command(RuntimeCommand::AddPeer { node_id })`

L'ordre est important :

- l'adresse doit être injectée avant de demander au gossip de dialer.

## Étape 6 — conserver les fallbacks

Ne pas supprimer immédiatement :

- `gossip_bootstrap_peers`
- `RuntimeConfigFFI.gossip_bootstrap_peers`
- `bootstrapPeerId`
- `TOM_BOOTSTRAP_PEER`

Les conserver comme :

- fallback,
- override de debug,
- filet de sécurité pendant la transition.

## Étape 7 — rollout explicite

Documenter clairement que :

- le nouveau frame impose soit un rollout atomique,
- soit une phase préalable de compatibilité aux frames inconnus.

Ne pas laisser cet aspect implicite.

---

## Critères d'acceptation corrigés

Le bon objectif produit n'est pas :

> "la découverte complète du réseau sans bootstrap"

Le bon objectif produit est :

> "auto-bootstrap relay-local fiable, rapide, servant d'amorce au gossip".

### Acceptation minimale réaliste

- deux pairs sur le même relay se découvrent automatiquement en < 1 seconde,
- sans bootstrap hardcodé dans le cas nominal,
- le gossip établit un voisinage réel (`NeighborUp`),
- un message applicatif peut être envoyé après découverte.

### Acceptation système raisonnable

- la découverte relay-assistée n'empêche pas les autres modes de découverte,
- les fallbacks restent opérationnels,
- aucun comportement ne dépend exclusivement d'un seed manuel.

---

## Tests indispensables

## Tests proto / relay

- roundtrip encode/decode du nouveau frame,
- test serveur : register de plusieurs clients,
- test de sélection bornée `k`.

## Tests `tom-connect`

- `handle_relay_msg(PeerPresent)` met bien à jour `endpoints_present`,
- événement `(EndpointId, RelayUrl)` émis correctement.

## Tests `tom-transport`

- `TomNode::add_peer_addr()` met à jour le `ConnectionPool`,
- **et** le `MemoryLookup` associé à l'endpoint,
- un `endpoint.connect(endpoint_id, alpn)` devient possible après injection.

## Tests `tom-protocol`

- deux runtimes, même relay, zéro bootstrap,
- le gossip observe `NeighborUp`,
- message A → B livré.

## Tests de non-régression

- DHT fallback toujours fonctionnel,
- bootstrap manuel toujours fonctionnel,
- tvOS / FFI non cassés tant que la migration n'est pas terminée.

---

## Recommandation finale à Claude

Si Claude veut conserver l'ambition du plan, alors la version correcte à coder est :

1. **Oui** à un frame relay de présence / hint.
2. **Oui** à la remontée d'un événement client `(EndpointId, RelayUrl)`.
3. **Oui** à `join_peers()` dans le runtime.
4. **Mais seulement si** l'adresse est injectée dans le lookup de l'endpoint, pas seulement dans le pool transport.
5. **Non** à la suppression immédiate des fallbacks.
6. **Non** au full broadcast de présence ; préférer un échantillonnage borné.
7. **Oui** à une stratégie explicite de compatibilité / rollout.

---

## Formulation brute, si besoin d'être clair

Le plan actuel est stratégiquement prometteur mais techniquement mal branché.

Le défaut rédhibitoire est simple :

> l'information de présence relay est injectée dans `TomNode`, alors que le gossip diale via `Endpoint` et son `address_lookup`.

Tant que ce point n'est pas corrigé, le plan n'est pas suffisamment solide pour être implémenté proprement.

En l'état :

- l'idée mérite d'être poursuivie,
- le design doit être corrigé,
- le séquencement doit être durci,
- et la surface de découverte doit être mieux contrôlée.

---

## Conclusion

La découverte automatique via relay est probablement **la bonne première brique réelle** de l'indépendance du réseau.

Mais cette brique doit être construite avec rigueur :

- bon point d'injection,
- bon niveau de diffusion,
- bon maintien des fallbacks,
- bon plan de migration.

Sinon, on obtiendra un bootstrap qui "a l'air" élégant sur le papier mais qui restera fragile, trop couplé, et potentiellement centralisant dans son comportement.

---

## Plan d'implémentation recommandé

Ce plan est volontairement séquencé pour :

1. **réduire le risque**,
2. **garder les fallbacks vivants**,
3. **valider chaque couche séparément**,
4. **éviter un gros patch monolithique impossible à déboguer**.

Le principe directeur est simple :

> d'abord rendre l'architecture correcte,
> ensuite introduire le nouveau signal relay,
> ensuite brancher le runtime,
> enfin seulement envisager une simplification des fallbacks.

---

## Phase 0 — cadrage non négociable avant code

Avant le moindre patch, Claude doit figer les décisions suivantes.

### Décision 0.1 — sémantique du frame

Choisir explicitement entre :

- `PeerPresent` = sémantique forte : "ce pair est actuellement connecté à ce relay"
- `PeerHint` = sémantique plus honnête : "voici un pair probable à essayer"

### Recommandation

Si l'implémentation fait de l'échantillonnage borné, le nom **`PeerHint`** est meilleur.

Si l'équipe veut rester proche du plan initial, `PeerPresent` reste acceptable, mais il faudra alors documenter qu'il ne s'agit **pas** d'un annuaire exhaustif.

### Décision 0.2 — stratégie de diffusion

Décider dès maintenant :

- **pas de full broadcast**,
- **échantillonnage borné obligatoire**.

### Recommandation pratique

- `k = 8` par défaut,
- `k` configurable côté relay si besoin plus tard,
- tirage pseudo-aléatoire simple suffisant dans un premier temps.

### Décision 0.3 — rollout

Claude doit écrire noir sur blanc dans le plan technique :

- soit **rollout atomique client + relay**,
- soit **tolérance aux frames inconnus d'abord**, puis feature.

### Recommandation court terme

Pour aller vite sans casser le protocole :

- rollout atomique,
- activation conditionnée au fait que les clients cibles soient à jour.

---

## Phase 1 — corriger d'abord le point d'injection d'adresse

Cette phase est la plus importante.

Tant qu'elle n'existe pas, le reste n'est qu'un tuyau joliment peint qui ne mène pas au bon système.

## Objectif

Faire en sorte que lorsqu'on apprend un `EndpointAddr`, cet apprentissage soit visible à la fois pour :

- le transport ToM (`ConnectionPool`),
- et le gossip (`endpoint.connect(endpoint_id, ...)`).

## Fichiers à modifier

### `crates/tom-transport/src/node.rs`

À faire :

- ajouter un champ de type `MemoryLookup` dans `TomNode`,
- créer ce lookup dans `TomNode::bind()`,
- l'injecter dans le builder de `Endpoint` avant `bind()`.

### `crates/tom-transport/src/connection.rs`

Pas forcément de changement structurel lourd, mais vérifier que :

- le `ConnectionPool` reste le cache local de connexions/adresses,
- il ne devient pas la source unique de vérité sur la résolution.

### `crates/tom-transport/src/lib.rs`

Éventuellement export complémentaire si nécessaire, uniquement si l'API publique l'exige.

## Modifications attendues

### Étape 1.1 — création du lookup mémoire

Dans `TomNode::bind()` :

- créer `let memory_lookup = tom_connect::address_lookup::memory::MemoryLookup::new();`
- l'ajouter au builder via `.address_lookup(memory_lookup.clone())`

### Étape 1.2 — stockage dans `TomNode`

Ajouter un champ du style :

- `peer_lookup: tom_connect::address_lookup::memory::MemoryLookup`

### Étape 1.3 — évolution de `TomNode::add_peer_addr()`

Aujourd'hui :

- alimente seulement `self.pool.add_addr(...)`

Après patch :

- alimente `self.pool.add_addr(...)`
- alimente `self.peer_lookup.add_endpoint_info(addr.clone())`

### Étape 1.4 — invariant à garantir

Après cette phase, l'invariant doit être :

> toute adresse injectée via `TomNode::add_peer_addr()` doit être visible à la fois par le transport ToM et par le `address_lookup` de l'endpoint.

## Tests à écrire dans cette phase

### Test 1 — lookup alimenté

Créer un test qui vérifie que :

- après `node.add_peer_addr(addr)`,
- un `endpoint.connect(endpoint_id, alpn)` devient possible, ou au minimum que l'`EndpointId` est résoluble via lookup.

### Test 2 — non-régression transport

Vérifier que l'envoi protocolaire via le `ConnectionPool` continue de fonctionner comme avant.

## Condition de sortie de phase

On ne passe pas à la phase 2 tant que cette phrase n'est pas vraie :

> `join_peers(endpoint_id)` a une chance réelle de fonctionner si un `EndpointAddr` a été injecté auparavant.

---

## Phase 2 — introduire le nouveau frame relay côté protocole

Une fois le point d'injection corrigé, on peut ajouter le signal relay.

## Objectif

Permettre au relay d'envoyer un hint de découverte à un client connecté.

## Fichiers à modifier

### `crates/tom-relay/src/protos/common.rs`

À faire :

- ajouter un nouveau `FrameType`
- choisir un identifiant numérique libre après `Restarting = 12`

Exemple :

- `PeerPresent = 13`

### `crates/tom-relay/src/protos/relay.rs`

À faire :

- ajouter `RelayToClientMsg::PeerPresent(EndpointId)` ou `PeerHint(EndpointId)`
- implémenter :
  - `typ()`
  - `write_to()`
  - `encoded_len()`
  - `from_bytes()`
- ajouter snapshot tests / proptests

### `crates/tom-relay/src/server/client.rs`

À faire :

- ajouter une file de sortie dédiée analogue à `peer_gone`
- ajouter le bras `tokio::select!` correspondant
- écrire le frame sur le stream

### `crates/tom-relay/src/server/clients.rs`

À faire :

- lors de `register()`, sélectionner un sous-ensemble borné de pairs existants,
- envoyer ces hints au nouveau client,
- optionnellement notifier le nouveau client à ce même sous-ensemble.

## Très important

Claude ne doit **pas** faire :

- "broadcast à tous les existants"
- "envoyer tous les existants au nouveau"

La logique attendue est :

- lecture du set courant,
- exclusion du pair lui-même,
- réduction à `k` éléments max,
- envoi non bloquant / drop tolérant en cas de backpressure.

## Tests à écrire dans cette phase

### Test 1 — roundtrip proto

- encode/decode du nouveau frame

### Test 2 — snapshot bytes

- ajouter un cas au snapshot existant dans `protos/relay.rs`

### Test 3 — register bounded fanout

- enregistrer plusieurs clients,
- vérifier que le nouveau reçoit au plus `k` hints,
- vérifier que l'auto-référence n'est jamais envoyée.

## Condition de sortie de phase

Le relay doit être capable d'émettre proprement des hints sans casser les tests proto existants.

---

## Phase 3 — remonter l'événement dans `tom-connect`

## Objectif

Transformer un frame relay reçu en événement structuré côté client :

- `(EndpointId, RelayUrl)`

## Fichiers à modifier

### `crates/tom-connect/src/socket/transports/relay/actor.rs`

À faire :

- ajouter un sender dédié pour les peer-present events,
- le stocker dans `ActiveRelayActor`,
- traiter le nouveau `RelayToClientMsg::*` dans `handle_relay_msg()`.

Comportement attendu :

- `state.endpoints_present.insert(endpoint_id)`
- `try_send((endpoint_id, self.url.clone()))`

### `crates/tom-connect/src/socket/transports/relay.rs`

À faire :

- créer un `mpsc::channel` dédié,
- le faire vivre dans `RelayTransport`,
- exposer un `take_peer_present_rx(&mut self) -> Option<Receiver<...>>`

### `crates/tom-connect/src/socket/transports.rs`

Deux options possibles :

#### Option simple et explicite

- exposer un agrégateur des receivers relay côté `Transports`

#### Option acceptable court terme

- supposer un seul transport relay utile et récupérer le receiver du premier transport relay

### Recommandation

Pour un premier patch :

- prendre l'option simple mais correcte,
- agréger les events de tous les relay transports vers un seul receiver au niveau socket/endpoint.

## Point de vigilance

Le receiver doit être **extractible une seule fois**, comme les autres flux consommés par la couche supérieure.

## Tests à écrire dans cette phase

### Test 1 — `handle_relay_msg`

Vérifier que :

- le set `endpoints_present` est bien mis à jour,
- l'événement est bien émis.

### Test 2 — pas de panique si queue pleine

En cas de `try_send` impossible :

- warning/log,
- mais pas de crash,
- pas d'effet secondaire dangereux.

## Condition de sortie de phase

`tom-connect` doit être capable de fournir un flux d'événements de présence relay au-dessus de la couche transport relay.

---

## Phase 4 — exposer proprement au niveau `Endpoint` puis `TomNode`

## Objectif

Rendre le flux consommable par `tom-transport` sans bricolage opaque.

## Fichiers à modifier

### `crates/tom-connect/src/endpoint.rs`

À faire :

- ajouter un point d'accès public ou crate-visible pour extraire le receiver d'événements relay peer-present.

### `crates/tom-connect/src/socket.rs`

Si nécessaire, faire remonter l'agrégateur depuis la socket vers l'endpoint.

### `crates/tom-transport/src/node.rs`

À faire :

- récupérer le receiver depuis l'endpoint juste après `bind()`
- le stocker dans `TomNode`
- exposer `take_peer_present_rx()` côté `TomNode`

## Recommandation d'API

Préférer une API du style :

- `pub fn take_peer_present_rx(&mut self) -> Option<mpsc::Receiver<(EndpointId, RelayUrl)>>`

Pourquoi :

- cohérent avec une ownership unique du flux,
- évite les clones hasardeux,
- facile à intégrer dans la boucle runtime.

## Tests à écrire dans cette phase

### Test 1 — extraction unique

Vérifier que :

- le receiver peut être pris une fois,
- les appels suivants retournent `None`.

### Test 2 — transit complet

Injecter un événement en bas,
vérifier qu'il ressort en haut.

## Condition de sortie de phase

`TomNode` doit pouvoir fournir au runtime un flux propre d'événements relay peer-present.

---

## Phase 5 — intégrer le runtime proprement

## Objectif

À réception d'un hint relay, le runtime doit :

1. enregistrer l'adresse,
2. rendre le peer dialable par le gossip,
3. lancer un `join_peers`,
4. enrichir l'état protocolaire.

## Fichiers à modifier

### `crates/tom-protocol/src/runtime/mod.rs`

À faire :

- récupérer le receiver relay peer-present avant de déplacer `node` dans la boucle,
- le transmettre à `runtime_loop`.

### `crates/tom-protocol/src/runtime/loop.rs`

À faire :

- ajouter un nouvel arm `tokio::select!`
- ordre exact à respecter :
  1. construire `EndpointAddr`
  2. `node.add_peer_addr(addr).await`
  3. `gossip_sender.join_peers(vec![endpoint_id]).await`
  4. `state.handle_command(RuntimeCommand::AddPeer { node_id })`

## Détail important

L'étape 2 doit précéder l'étape 3.

Si Claude inverse cet ordre, il réintroduit exactement le bug conceptuel signalé dans cette review.

## Sémantique d'état attendue

À la réception d'un hint relay :

- on ne considère pas que le gossip est déjà connecté,
- on considère seulement qu'un peer devient **tentable / joignable**.

Donc :

- `AddPeer` est acceptable pour enrichir la topologie,
- mais il ne faut pas vendre cela comme une preuve de voisinage réel.

Le vrai voisinage reste confirmé par :

- `GossipEvent::NeighborUp(...)`

## Tests à écrire dans cette phase

### Test 1 — runtime event path

- réception d'un événement relay,
- `add_peer_addr` appelée,
- `join_peers` appelée,
- topologie enrichie.

### Test 2 — intégration deux nœuds même relay

- zéro bootstrap manuel,
- attente d'un `NeighborUp`,
- envoi d'un message applicatif.

## Condition de sortie de phase

Deux nœuds sur le même relay doivent découvrir et établir un voisinage gossip sans seed explicite.

---

## Phase 6 — garder les fallbacks, ne rien supprimer encore

## Objectif

Stabiliser la feature avant toute simplification de configuration.

## Fichiers à laisser en place pour l'instant

- `crates/tom-protocol/src/runtime/mod.rs` → `gossip_bootstrap_peers`
- `crates/tom-protocol-ffi/src/types.rs`
- `crates/tom-protocol-ffi/src/lib.rs`
- `apps/tom-node-tvos/TomNode/Services/TomNodeService.swift`
- `crates/tom-tui/src/main.rs`

## Ce qui est autorisé

Au maximum :

- commentaire de dépréciation,
- note "fallback / debug only",
- mais pas suppression fonctionnelle.

## Condition de sortie de phase

La feature relay-assistée doit avoir passé :

- tests unitaires,
- intégration locale,
- au moins un test réel Mac ↔ Apple TV / NAS.

---

## Phase 7 — campagne de validation réelle

## Objectif

Vérifier que le mécanisme fonctionne hors labo et qu'il n'introduit pas de comportement pathologique.

## Scénarios minimum

### Scénario A — 2 nœuds, même relay, zéro bootstrap

Attendus :

- découverte automatique,
- `NeighborUp`,
- premier message livré.

### Scénario B — redémarrage d'un pair

Attendus :

- redécouverte sans seed manuel,
- pas de double état cassé côté gossip.

### Scénario C — 3 à 10 nœuds sur même relay

Attendus :

- pas de tempête,
- pas de burst excessif,
- convergence du voisinage.

### Scénario D — pairs sur relays différents

Attendus :

- le système n'est pas pire qu'avant,
- les fallbacks continuent d'assurer la découverte.

## Métriques à observer

- temps jusqu'au premier `NeighborUp`
- temps jusqu'au premier message livré
- nombre de hints relay envoyés par connexion
- absence de saturation de queue
- absence de crash sur frame inconnu dans le périmètre déployé

---

## Phase 8 — nettoyage seulement après validation

Cette phase est optionnelle et ne doit pas être couplée au premier patch fonctionnel.

## Ce qui pourra être envisagé plus tard

- réduction du rôle de `gossip_bootstrap_peers`
- dépréciation plus agressive de `TOM_BOOTSTRAP_PEER`
- simplification de la config tvOS

## Ce qui ne doit être supprimé qu'avec preuve

Claude ne doit supprimer les seeds/fallbacks que si les preuves suivantes existent :

1. la découverte relay-locale est stable,
2. le multi-relay reste couvert par DHT/Pkarr/autres chemins,
3. les tests hardware réels passent,
4. il n'y a pas de régression de connectivité.

---

## Ordre de merge recommandé

L'ordre de merge doit suivre l'ordre des dépendances réelles.

### PR 1 — `tom-transport`: lookup mémoire partagé

Contenu :

- `MemoryLookup` dans `TomNode`
- `add_peer_addr()` enrichi
- tests de lookup / dialabilité

### PR 2 — `tom-relay`: nouveau frame + diffusion bornée

Contenu :

- frame proto
- serveur relay
- tests proto / register

### PR 3 — `tom-connect`: remontée d'événement relay

Contenu :

- handler client-side
- receiver exposé
- tests de flux

### PR 4 — `tom-protocol`: intégration runtime

Contenu :

- nouvel arm `runtime_loop`
- test d'intégration même relay, zéro bootstrap

### PR 5 — validation hardware / doc / dépréciations légères

Contenu :

- notes opératoires
- éventuelle doc de transition
- mais pas suppression des fallbacks

---

## Conditions de merge strictes

Claude ne devrait pas considérer le sujet comme terminé tant que les conditions suivantes ne sont pas remplies.

### Minimum technique

- tous les tests unitaires ajoutés passent,
- pas de régression visible sur les suites existantes pertinentes,
- au moins un test d'intégration 2 nœuds / même relay / zéro bootstrap passe.

### Minimum architecture

- l'adresse est injectée dans le lookup endpoint,
- pas seulement dans le `ConnectionPool`,
- la diffusion relay est bornée,
- le rollout est explicitement documenté.

### Minimum produit

- la feature améliore le cas nominal,
- sans supprimer les filets de sécurité existants.

---

## Anti-objectifs explicites

Pour éviter les faux bons patchs, voici ce que Claude ne doit pas faire.

### À ne pas faire

- gros patch unique cross-crates sans validation intermédiaire,
- suppression immédiate des bootstraps manuels,
- full broadcast de présence à tous les clients relay,
- ajout d'un signal runtime sans lookup endpoint,
- assimilation de `PeerPresent` à une preuve de voisinage gossip,
- rollout implicite non documenté.

### Si Claude fait l'un de ces choix

Alors il construit une feature :

- plus fragile,
- plus couplée,
- plus dure à déployer,
- et moins fidèle à l'objectif d'indépendance organique du réseau.

---

## Synthèse finale pour implémentation

Le plan d'implémentation à suivre est donc :

1. **Corriger d'abord le point d'injection d'adresse** avec un lookup mémoire partagé dans `TomNode`.
2. **Ajouter ensuite le frame relay** côté proto serveur/client.
3. **Faire remonter un événement structuré dans `tom-connect`**.
4. **L'intégrer dans le runtime** avec l'ordre correct : injecter l'adresse, puis `join_peers`.
5. **Conserver les fallbacks** pendant toute la montée en charge.
6. **Valider sur tests réels** avant de simplifier quoi que ce soit.

Si cet ordre n'est pas respecté, l'implémentation sera au mieux incomplète, au pire trompeuse.
