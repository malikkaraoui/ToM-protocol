# Review critique — implémentation PeerPresent V3

Date : 2026-03-07  
Auteur : GitHub Copilot  
Sujet : review critique de l’implémentation `Relay-Assisted Gossip Discovery (PeerPresent)`

## Verdict global

L’implémentation est **globalement solide sur l’architecture** et le câblage principal `relay -> connect -> transport -> runtime` est cohérent avec le plan V3 validé.

Mais je n’irais **pas jusqu’à “mergeable sans réserve”** en l’état.

Il existe un **défaut de cohérence runtime important** : l’ordre critique `add_peer_addr()` puis `join_peers()` est correctement respecté dans le nouveau bras `PeerPresent`, **mais il reste inversé dans les handlers existants `AddPeerAddr` et `DhtLookupResult`**. Cela réintroduit précisément le problème conceptuel que `MemoryLookup` était censé corriger sur ces deux chemins.

Conclusion courte :

> bonne implémentation de la chaîne `PeerPresent`, mais correction incomplète de l’invariant d’ordre sur l’ensemble des chemins d’injection d’adresse.

---

## Ce qui est solide

- Le design général reste bon et conforme au plan V3.
- Le `MemoryLookup` dans `TomNode` est bien le bon correctif structurel.
- Le relay introduit `PeerPresent` proprement au niveau proto.
- Le sampling borné `k=8` est sain.
- La propagation `tom-connect` est mécaniquement cohérente.
- Le pattern take-once du receiver est globalement propre.
- Le bras `PeerPresent` dans `runtime_loop` respecte bien l’ordre attendu :
  1. injection d’adresse,
  2. `join_peers()`,
  3. enrichissement topologie.
- Les fallbacks ne sont pas supprimés prématurément.
- La validation `clippy --workspace` et `test --workspace` propre est un bon signal.

---

## Bloquants éventuels

### 1. Incohérence d’ordre dans `AddPeerAddr` et `DhtLookupResult`

Dans `crates/tom-protocol/src/runtime/loop.rs` :

- `RuntimeCommand::AddPeerAddr { addr }`
- `RuntimeCommand::DhtLookupResult { ref addr }`

font encore :

1. `join_peers(...)`
2. puis `node.add_peer_addr(...)`

alors que le bras `PeerPresent` fait correctement :

1. `node.add_peer_addr(...)`
2. puis `join_peers(...)`

C’est un vrai problème.

### Pourquoi c’est important

Parce que le bug historique corrigé par `MemoryLookup` concernait précisément la visibilité de l’adresse pour le gossip.

Donc si `join_peers()` est lancé avant l’injection de l’adresse sur ces chemins :

- le gossip peut dialer trop tôt,
- `trigger_address_lookup()` peut encore ne rien voir,
- et on réintroduit le défaut initial sur les chemins manuels / DHT.

### Pourquoi je considère ça sérieux

Ce n’est pas juste une question d’élégance ou de symétrie.

C’est un problème de cohérence fonctionnelle entre :

- le nouveau chemin `PeerPresent`,
- et les chemins existants `AddPeerAddr` / `DhtLookupResult`.

Or le résumé d’implémentation prétend justement avoir aussi renforcé ces chemins.

### Correction attendue

Dans les handlers `AddPeerAddr` et `DhtLookupResult` :

- injecter d’abord l’adresse via `node.add_peer_addr(...)`
- puis appeler `join_peers(...)`

Tant que cela n’est pas remis dans le bon ordre, je considère que la correction du bug pré-existant n’est pas complètement consolidée.

---

## Risques non bloquants

### 1. Chaîne `peer_present_tx` un peu couplée — gravité modérée

La chaîne :

`ActiveRelayActor -> RelayActor -> RelayTransport -> Transports -> Handle -> Endpoint -> TomNode`

est assez longue et très mécanique.

Elle n’est pas fondamentalement mauvaise, mais elle a deux inconvénients :

- forte propagation d’une responsabilité très spécifique,
- élargissement de l’API interne pour un seul type d’événement.

### Mon avis

Ce n’est **pas bloquant**.
Le couplage est acceptable pour une première intégration.

### Alternative envisageable

À moyen terme, une abstraction plus propre serait :

- un flux d’événements de transport générique,
- par exemple un `TransportEvent` ou `RelayEvent`,
- exposé une seule fois vers le haut.

Cela réduirait la dette si d’autres événements relay remontent plus tard.

Pour ce patch précis, je ne demanderais pas ce refactor avant merge.

---

### 2. Le support “multi-relay” est plus suggéré que réellement exercé — gravité modérée

Le câblage `tx.clone()` vers chaque `RelayTransport` laisse penser à un support multi-relay agrégé.

Mais dans `crates/tom-connect/src/socket.rs`, `Handle::new()` contient toujours :

- rejet si plus d’un `TransportConfig::Relay`

Donc :

- l’agrégation est structurellement prévue,
- mais la vraie situation runtime reste **single relay transport**.

### Mon avis

Ce n’est pas un bug.
Mais il faut éviter de sur-vendre cette partie comme “multi-relay validé”.

C’est plutôt :

> design prêt pour l’agrégation, mais invariant actuel encore mono-relay.

---

### 3. Risque de perte de hints sous charge — gravité faible à modérée

Le channel `128` dans `Transports` n’introduit pas de deadlock :

- les producteurs utilisent `try_send`,
- il n’y a pas de blocage circulaire,
- le receiver est consommé par un seul owner.

Le vrai risque n’est pas le deadlock, mais la perte silencieuse de hints sous burst.

### Mon avis

Ce risque est acceptable **si** `PeerPresent` reste explicitement traité comme un hint best-effort.

Donc :

- pas bloquant,
- mais mériterait une note de design claire,
- et idéalement un test ou un metric autour des drops si cela devient important.

---

### 4. `Arc<Mutex<Option<Receiver>>>` dans `Handle` — gravité faible

Pour un `Handle` clonable avec sémantique take-once, ce pattern est acceptable.

Pourquoi :

- la méthode est synchrone,
- le lock est pris très brièvement,
- il n’y a pas d’`await` sous verrou,
- le receiver n’est extrait qu’une fois.

Je ne vois pas de risque réaliste de deadlock ici.

### Mon avis

- **pattern correct en l’état**
- pas élégant au sens “beau framework”,
- mais tout à fait raisonnable pour cette sémantique

Si ce pattern réapparaît plusieurs fois dans le codebase, il pourra être factorisé dans un petit helper `TakeOnce<T>`.

---

## Tests manquants ou insuffisants

### 1. Il manque un test d’intégration explicite `PeerPresent -> join_peers -> NeighborUp`

C’est le plus gros manque de couverture à mes yeux.

Je n’ai pas trouvé de test d’intégration explicite qui prouve en une seule chaîne :

1. réception d’un `PeerPresent`,
2. injection d’adresse,
3. `join_peers()`,
4. apparition d’un vrai `NeighborUp`,
5. message applicatif ensuite livré.

Or c’est **le cœur produit** de la feature.

Le fait que `test --workspace` passe ne prouve pas automatiquement cela.

### Recommandation

Ajouter un test d’intégration dédié, par exemple avec :

- 2 nœuds,
- même relay,
- zéro bootstrap manuel,
- attente explicite de `ProtocolEvent::GossipNeighborUp` ou équivalent,
- puis envoi d’un message.

---

### 2. Il manque un test qui couvre les chemins `AddPeerAddr` / `DhtLookupResult` avec l’invariant d’ordre

C’est précisément l’endroit où l’implémentation actuelle est incohérente.

Il faudrait un test qui prouve que :

- sur injection manuelle d’adresse,
- l’adresse devient visible du gossip avant tentative de `join_peers()`.

Sans ce test, la régression que je pointe peut facilement revenir — ou rester invisible si d’autres mécanismes masquent le problème.

---

### 3. Les tests relay semblent surtout adaptés pour tolérer `PeerPresent`, pas encore assez démonstratifs sur sa valeur comportementale

Les helpers `recv_data_frame()` / `recv_data_client()` qui skippent `PeerPresent` sont utiles pour stabiliser les tests existants.

Mais ils jouent surtout un rôle de compatibilité de test.

Ils ne remplacent pas des tests plus positifs du type :

- le nouveau client reçoit bien au plus `k` hints,
- les pairs sélectionnés reçoivent bien l’annonce du nouveau,
- l’auto-référence est impossible,
- le caractère bidirectionnel du sampling est bien garanti.

Je soupçonne qu’une partie de cela est testée indirectement, mais la preuve comportementale pourrait être plus explicite.

---

## Réponses aux questions spécifiques

### 1. La chaîne `peer_present_tx` est-elle trop couplée ?

**Réponse courte :** un peu, mais pas au point de bloquer.

Elle est plus couplée que ce que j’aimerais à long terme, car un événement très spécifique traverse beaucoup de couches.

Mais :

- le couplage reste mécanique,
- il ne déforme pas les responsabilités fondamentales,
- et il est acceptable pour une première intégration.

**Alternative préférable à moyen terme :** un `TransportEvent` unique remonté depuis `Transports`.

---

### 2. Le `Arc<Mutex<Option<Receiver>>>` dans `Handle` est-il le bon pattern ?

**Réponse courte :** oui, c’est un bon pattern pragmatique ici.

Je ne vois ni deadlock réaliste ni contention significative.

C’est un take-once standard dans une struct clonable.

---

### 3. Risque de deadlock ou contention sur le channel 128 dans `Transports` ?

**Réponse courte :** pas de deadlock ; contention faible ; perte de hints possible mais cohérente avec une sémantique best-effort.

Le risque principal n’est pas la sécurité du système, mais la perte silencieuse de hints sous burst.

Je classerais cela en **risque non bloquant**.

---

### 4. L’ordering `add_peer_addr()` puis `join_peers()` est-il garanti dans le bras `select!` ?

**Réponse courte :**

- **oui dans le nouveau bras `PeerPresent`**,
- **non de façon cohérente sur l’ensemble du runtime**.

Le bras `PeerPresent` est correct.
Mais les handlers `AddPeerAddr` et `DhtLookupResult` restent dans le mauvais ordre.

Donc la bonne réponse professionnelle n’est pas “oui” mais :

> oui localement dans ce bras, non globalement dans l’implémentation.

---

### 5. Manque-t-il un test d’intégration spécifique `PeerPresent -> gossip join` ?

**Oui. Clairement oui.**

Et je le considère comme la lacune principale de couverture, juste après le problème d’ordre runtime.

---

## Recommandations de merge

### Décision

**mergeable with reservations**

### Réserves obligatoires à lever rapidement

1. corriger l’ordre dans `AddPeerAddr` et `DhtLookupResult` ;
2. ajouter au moins un test d’intégration explicite `PeerPresent -> NeighborUp -> message livré`.

### Réserves non bloquantes

- documenter plus explicitement que la chaîne actuelle est best-effort ;
- ne pas sur-vendre le support multi-relay tant que `Handle::new()` reste mono-relay ;
- envisager plus tard une abstraction `TransportEvent` si d’autres signaux remontent.

---

## Conclusion finale

Le chantier est **bien avancé** et la base posée est sérieuse.

La correction `MemoryLookup` est la bonne. La chaîne de propagation est crédible. Le nouveau bras runtime `PeerPresent` est bien ordonné.

Mais la review sérieuse fait apparaître une incohérence importante :

> l’invariant d’ordre que ce patch introduit correctement dans le chemin `PeerPresent` n’a pas été réappliqué aux deux chemins existants `AddPeerAddr` et `DhtLookupResult`.

C’est le principal point à corriger pour rendre la base réellement consolidée.
