# Review finale — découverte automatique de pairs via relay — V3

Date : 2026-03-07  
Auteur : GitHub Copilot  
Sujet : review du plan `Relay-Assisted Gossip Discovery (PeerPresent) — V3`

## Position générale

Le plan V3 est, à ce stade, **structurellement bon**.

Il ne souffre plus des défauts rédhibitoires observés dans la V1, et il corrige correctement les zones encore floues de la V2.

En termes clairs :

- **la direction produit est bonne** ;
- **l’architecture proposée est cohérente avec le code réel** ;
- **le plan est prêt pour implémentation** ;
- **les réserves restantes relèvent surtout de la discipline d’exécution, pas d’un défaut de conception**.

Autrement dit :

> Oui, ce V3 mérite un feu vert d’implémentation.

Mais ce feu vert doit être compris comme un feu vert d’ingénierie :

- avec ordre de câblage respecté,
- avec tests sérieux,
- avec validation hardware,
- sans nettoyage prématuré des fallbacks.

---

## Ce que V3 corrige enfin correctement

## 1. Le bug central est traité à la racine

Le point le plus critique de toute cette séquence était le suivant :

- `TomNode::add_peer_addr()` alimentait seulement `ConnectionPool`,
- alors que `tom-gossip` diale via `endpoint.connect(endpoint_id, ...)`,
- donc via le pipeline de `address_lookup` de l’`Endpoint`.

Le V3 corrige cela proprement :

- création d’un `MemoryLookup` dans `TomNode::bind()`,
- injection dans le builder de l’`Endpoint`,
- stockage dans `TomNode`,
- alimentation de ce lookup dans `add_peer_addr()` **avant** `pool.add_addr()`.

C’est le bon correctif.

Et surtout, il faut noter une chose importante :

> ce correctif ne sert pas seulement à `PeerPresent` ;
> il répare aussi une incohérence déjà présente sur les chemins d’injection d’adresse existants, notamment `AddPeerAddr` et `DhtLookupResult`.

Donc V3 ne rajoute pas simplement une feature.
Il **répare d’abord la plomberie structurelle**.

---

## 2. Le relay ajoute un signal utile sans devenir un annuaire global

Le choix de garder `PeerPresent` est acceptable.

L’argument est défendable :

- le relay observe réellement la présence d’un pair connecté,
- ce qui est borné n’est pas la véracité du signal,
- mais **sa diffusion**.

Le point important est que V3 l’encadre correctement :

- présence relay-observée,
- non exhaustive,
- pas une preuve de voisinage gossip,
- pas une vision globale du réseau.

C’est exactement la bonne sémantique.

---

## 3. Le sampling borné est la bonne décision réseau

Le passage à un **bounded sampling `k = 8`** est une bonne décision de design.

Cela permet de :

- éviter un broadcast `O(n)` à chaque `register()`,
- limiter l’exposition de topologie,
- amorcer HyParView sans transformer le relay en répertoire implicite,
- conserver un coût borné, simple à raisonner.

Le point est sain techniquement **et** sain architecturalement.

---

## 4. L’agrégation multi-relay est enfin pensée au bon niveau

C’était un angle mort réel de la V2.

Le V3 tranche correctement :

- création du channel d’événements dans `Transports`,
- `tx` partagé à chaque `RelayTransport`,
- un `rx` unique agrégé,
- remontée propre jusqu’à `endpoint.rs`, puis `TomNode`.

C’est cohérent avec la structure réelle de `tom-connect`, en particulier avec l’existence de :

- `relay: Vec<RelayTransport>` dans `Transports`.

Donc ce n’est plus une idée “papier”.
C’est enfin branché contre la topologie réelle du code.

---

## 5. L’ordre runtime est enfin correct

Le V3 pose la bonne séquence :

1. construire `EndpointAddr`,
2. `node.add_peer_addr(addr).await`,
3. `gossip_sender.join_peers(vec![endpoint_id]).await`,
4. `state.handle_command(RuntimeCommand::AddPeer { node_id })`.

C’est le **bon ordre**, et il est non négociable.

Si l’implémentation respecte cet ordre, alors le gossip a enfin une chance réelle de dialer correctement.

Si cet ordre est inversé, on retombe exactement dans le défaut conceptuel initial.

---

## 6. Les fallbacks sont conservés au lieu d’être sabrés trop tôt

Le maintien de :

- `gossip_bootstrap_peers`,
- `bootstrapPeerId` côté tvOS,
- `TOM_BOOTSTRAP_PEER` côté TUI,
- DHT / Pkarr,

est la bonne décision.

`PeerPresent` améliore très bien le cas **relay-local**, mais ne remplace pas à lui seul toute la stratégie de découverte du système.

Le fait que V3 garde ces chemins comme fallbacks / debug / filet de sécurité est une marque de maturité.

---

## 7. Le rollout est enfin réaliste

Le plan V3 assume correctement un **rollout atomique**.

Dans votre contexte réel :

- nombre limité de devices,
- périmètre maîtrisé,
- infrastructure relay contrôlée,

c’est la bonne stratégie court terme.

C’est beaucoup plus sérieux que de faire semblant d’avoir une compatibilité backward robuste alors que les vieux clients ne tolèrent pas forcément un nouveau `FrameType`.

---

## Ce que je valide sans réserve dans V3

Je valide sans réserve les blocs suivants :

- introduction de `MemoryLookup` dans `TomNode`,
- évolution de `add_peer_addr()` pour nourrir lookup + pool,
- `PeerPresent` comme nom acceptable,
- bounded sampling `k=8`,
- agrégation des événements au niveau `Transports`,
- remontée d’un flux unique jusqu’à `TomNode`,
- ordre `add_peer_addr()` puis `join_peers()`,
- conservation des fallbacks,
- rollout atomique explicite,
- batterie de tests relay / connect / transport / intégration,
- validation hardware Mac ↔ Apple TV / NAS.

Sur ces points, V3 est propre.

---

## Ce que je surveillerais pendant l’implémentation

Il ne s’agit plus de critiques de conception, mais de **points de vigilance de coding**.

## 1. La conversion `EndpointId -> NodeId` doit être l’API réelle

Le plan écrit une conversion de type :

- `NodeId::from_endpoint_id(endpoint_id)`

C’est très bien au niveau intention.

Mais pendant l’implémentation, il faudra utiliser la **vraie API du repo**.

Ce n’est pas un problème d’architecture.
C’est simplement un point de rigueur pour éviter un plan juste mais un patch approximatif.

---

## 2. Les effets retournés par `state.handle_command(...)` doivent bien être exécutés

Le runtime de ce repo fonctionne sur une logique d’effets.

Donc dans le nouveau bras `select!`, il faut s’assurer que :

- les effets produits par `state.handle_command(RuntimeCommand::AddPeer { node_id })`
- sont bien renvoyés dans le flux normal d’exécution,
- et pas calculés puis ignorés.

Ce serait une erreur d’implémentation classique.

---

## 3. Les doublons `PeerPresent` doivent être traités comme normaux

Dans un système réel, il faut assumer qu’un pair puisse être vu plusieurs fois :

- plusieurs relays,
- reconnexion,
- hint redondant,
- course entre événements.

L’implémentation doit donc être :

- tolérante,
- non panic,
- sans side effect toxique,
- et raisonnablement idempotente.

Le signal `PeerPresent` doit rester un **hint opportuniste**, pas une transaction sacrée.

---

## 4. La backpressure doit rester en mode best effort

C’est très bien que le plan parle de `try_send` et de “pas de panic si queue pleine”.

Il faut garder cette philosophie jusqu’au bout :

- si un hint saute sous pression,
- ce n’est pas un drame,
- le système doit rester vivant,
- pas devenir rigide ou explosif.

Le sujet n’est pas la livraison fiable des hints.
Le sujet est l’auto-amorçage pragmatique du gossip.

---

## Critères minimaux de validation avant merge

Voici les conditions que je considère comme minimales pour déclarer la feature réellement valide.

## 1. Validation proto / relay

Il faut au minimum :

- un roundtrip encode/decode du frame `PeerPresent`,
- un test `register()` vérifiant :
  - pas d’auto-référence,
  - au plus `k` hints,
  - notification correcte des pairs existants sélectionnés.

---

## 2. Validation `tom-connect`

Il faut au minimum :

- un test de `handle_relay_msg(PeerPresent)`,
- vérification de la mise à jour de `endpoints_present`,
- émission correcte de l’événement,
- absence de panic si la queue est pleine.

---

## 3. Validation `tom-transport`

Il faut au minimum :

- un test vérifiant que `add_peer_addr()` nourrit bien :
  - le `ConnectionPool`,
  - le `MemoryLookup`,
- et idéalement un test montrant qu’un `endpoint.connect(endpoint_id, alpn)` devient possible après injection.

---

## 4. Validation intégration protocole

Il faut au minimum :

- deux nœuds sur le même relay,
- zéro bootstrap manuel,
- apparition d’un vrai `NeighborUp`,
- message applicatif livré.

C’est le test qui prouve que la feature ne fait pas que “bouger de l’état”, mais améliore réellement le système.

---

## 5. Validation hardware réelle

Le plan a raison de prévoir :

- relay NAS,
- Mac,
- Apple TV,
- TUI.

Je considère cette étape indispensable avant tout discours triomphaliste.

Tant que ce test réel n’a pas passé, il faut considérer le sujet comme :

- techniquement prometteur,
- mais pas totalement consolidé.

---

## Jugement global

Mon jugement global sur V3 est le suivant.

### Architecture

**Validée.**

### Séquencement

**Validé.**

### Plomberie inter-crates

**Validée.**

### Pragmatique de rollout

**Validée.**

### Nettoyage de fallbacks

**Correctement différé.**

### Prêt à coder

**Oui.**

---

## Formulation claire pour Claude

Si je devais résumer la position en une phrase :

> Le V3 est enfin le bon plan : il est cohérent avec le code réel, il traite le vrai bug de lookup, il borne correctement la diffusion relay, il conserve les fallbacks, et il peut être implémenté proprement à condition d’être exécuté avec rigueur.

---

## Réponse courte prête à transmettre

Le V3 est bon.

Je valide l’architecture et je considère que tu peux implémenter.

Les points critiques ont été correctement absorbés :

- `MemoryLookup` au bon niveau,
- sampling borné `k=8`,
- agrégation multi-relay au niveau `Transports`,
- ordre `add_peer_addr()` puis `join_peers()`,
- fallbacks conservés,
- rollout atomique assumé.

Mes seules réserves restantes portent sur l’exécution :

- utiliser la vraie API de conversion `EndpointId -> NodeId`,
- ne pas perdre les effets runtime,
- tolérer les doublons `PeerPresent`,
- garder une sémantique best effort sous backpressure.

Si les tests prévus passent, plus la validation hardware Mac ↔ Apple TV / NAS, alors la feature est légitime.

---

## Conclusion

La V1 avait une bonne intuition mais une implémentation mal branchée.
La V2 avait corrigé l’essentiel mais gardait encore quelques angles morts.

La V3, elle, est enfin **à la bonne altitude** :

- assez ambitieuse pour améliorer réellement l’autonomie du réseau,
- assez réaliste pour respecter les dépendances du code existant,
- assez prudente pour ne pas casser les fallbacks,
- et assez concrète pour être implémentée sans fiction.

En bref :

> oui, c’est un bon plan,
> oui, il mérite d’être codé,
> et oui, cette fois on peut parler d’un vrai travail d’équipe entre design, review et exécution.
