# Review critique — découverte automatique de pairs via relay — V2

Date : 2026-03-07
Auteur : GitHub Copilot
Sujet : review du plan `Relay-Assisted Gossip Discovery (PeerPresent) — V2`

## Résumé exécutif

Le plan V2 est **nettement meilleur** que la première version.

Il intègre correctement les quatre corrections structurelles les plus importantes :

- `MemoryLookup` comme prérequis d'architecture,
- diffusion bornée côté relay,
- conservation des fallbacks,
- ordre correct `add_peer_addr()` puis `join_peers()`.

En l'état, le plan V2 n'est plus "mal branché" comme la V1.
Il devient **techniquement défendable**.

Mais il n'est **pas encore prêt à exécution sans réserve**.

Verdict :

- **Direction générale : validée**
- **Cœur d'architecture : validé**
- **Prêt à coder tel quel : non, pas encore**
- **Niveau réel : bon plan avec 4 clarifications obligatoires**

---

## Ce que le plan V2 a bien corrigé

## 1. Le vrai pivot a été compris : `MemoryLookup`

C'est le point le plus important de toute cette séquence, et V2 l'intègre correctement.

Le plan reconnaît explicitement que :

- `TomNode::add_peer_addr()` n'alimente aujourd'hui que `ConnectionPool`,
- `tom-gossip` diale via `endpoint.connect(endpoint_id)`,
- cette chaîne passe par `resolve_remote()` puis `trigger_address_lookup()`,
- donc sans `MemoryLookup`, les adresses relay injectées ne sont pas visibles au gossip.

C'est exact.

Et surtout, V2 comprend une chose essentielle :

> le correctif `MemoryLookup` ne sert pas seulement à `PeerPresent` ;
> il corrige aussi une incohérence existante sur les chemins d'injection d'adresse déjà présents.

C'est une bonne lecture architecturelle.

---

## 2. L'ordre critique est désormais bon

V2 pose explicitement la séquence :

1. construire `EndpointAddr`
2. `node.add_peer_addr(addr)`
3. `gossip.join_peers(...)`
4. `state.handle_command(AddPeer)`

C'est le bon ordre.

Si cet ordre est respecté dans le code, la feature a enfin une chance d'être fonctionnelle du premier coup.

---

## 3. Le full broadcast a été abandonné

Très bonne correction.

Le passage à un **bounded sampling `k=8`** :

- supprime la dérive "oracle de présence total",
- évite le coût en `O(n)` à l'enregistrement,
- reste suffisant pour amorcer HyParView,
- colle mieux à l'esprit d'un réseau organique.

Cette décision est saine.

---

## 4. Les fallbacks ne sont plus supprimés brutalement

Le fait de garder :

- DHT,
- Pkarr,
- bootstrap manuel,
- chemins de secours existants,

est une bonne décision.

Le plan V2 a compris que `PeerPresent` ne couvre que :

- le cas relay-local,
- pas le multi-relay,
- pas toute la découverte système.

C'est correct.

---

## 5. Le rollout est revenu à une réalité opérationnelle

Le choix :

- rollout atomique,
- 2-3 commits logiques,
- périmètre limité aux devices maîtrisés,

est cohérent avec le contexte réel.

Le sujet n'a pas besoin de bureaucratie de review artificielle ; il a besoin d'un ordre d'attaque propre.

---

## Ce que je valide sans réserve dans V2

Je valide les blocs suivants :

- ajout du `MemoryLookup` dans `TomNode::bind()`
- évolution de `TomNode::add_peer_addr()` pour nourrir lookup + pool
- `PeerPresent` comme nom acceptable
- sampling borné `k=8`
- remontée d'un événement `(EndpointId, RelayUrl)`
- runtime : injection d'adresse avant `join_peers`
- conservation des fallbacks
- tests relay / connect / transport / intégration
- validation hardware Mac ↔ Apple TV / NAS

Sur ces points, V2 est sur de bons rails.

---

## Les 4 corrections obligatoires avant exécution

## Correction obligatoire n°1 — clarifier l'agrégation des événements `PeerPresent`

C'est le plus gros angle mort restant du plan V2.

Le plan dit :

- `RelayTransport::new()` crée un channel,
- `take_peer_present_rx()` est exposé,
- `MagicSock -> endpoint.rs`
- puis `TomNode` récupère le flux.

Mais il ne dit pas clairement :

- où l'agrégation se fait si plusieurs relay transports existent,
- qui possède le receiver final,
- et si l'ownership du flux est unique ou implicite.

### Pourquoi c'est important

Dans `tom-connect`, la couche transport n'est pas conceptuellement un singleton garanti :

- il y a `Transports`,
- avec potentiellement un ensemble de transports relay,
- et donc la question "un flux ou plusieurs ?" doit être explicitement tranchée.

### Ce qui manque dans V2

Il faut écrire noir sur blanc une de ces deux options.

#### Option A — patch limité explicitement

> Ce patch suppose un unique `RelayTransport` pertinent ; l'agrégation multi-relay est hors périmètre immédiat.

#### Option B — design propre dès maintenant

> Les événements `PeerPresent` de tous les `RelayTransport` sont agrégés dans un flux unique au niveau `Socket` / `Endpoint`, puis exposés une seule fois vers `TomNode`.

### Recommandation

Si Claude veut faire un patch réellement propre :

- prendre l'option B,
- expliciter **où vit l'agrégateur**,
- expliciter **qui possède le receiver final**.

Tant que ce point n'est pas écrit, le plan reste incomplet sur la plomberie d'événements.

---

## Correction obligatoire n°2 — corriger le pseudo-code de construction de `EndpointAddr`

Le plan V2 propose à un endroit une construction de type :

- `EndpointAddr::from_parts(endpoint_id, Some(relay_url), [])`

Cette signature ressemble à du pseudo-code, pas à une API confirmée du repo.

### Risque

Ce genre de détail peut sembler mineur, mais il révèle souvent un plan encore "papier".

### Ce qu'il faut faire

Le plan doit être corrigé pour utiliser une construction alignée avec l'API réelle.

Exemple attendu conceptuellement :

- `EndpointAddr::new(endpoint_id).with_relay_url(relay_url)`

ou l'équivalent exact présent dans le code.

### Exigence

Avant exécution, Claude doit remplacer toute pseudo-signature inventée par une forme API-réelle ou au minimum par une formulation neutre.

---

## Correction obligatoire n°3 — reformuler le périmètre du bug pré-existant

Le plan V2 dit en substance :

> ce bug affecte aussi DHT et AddPeer existants.

Cette phrase est **un peu trop large**.

### Ce qui est juste

Le bug affecte clairement les chemins qui injectent réellement une adresse via `add_peer_addr()`, notamment :

- `AddPeerAddr`
- `DhtLookupResult`

### Ce qui est trop large

`AddPeer { node_id }` tout seul n'injecte pas forcément d'adresse.

Donc dire :

- "le bug affecte AddPeer existant"

n'est pas la formulation la plus précise.

### Formulation recommandée

> Le bug affecte les chemins qui injectent une adresse via `TomNode::add_peer_addr()`, en particulier `AddPeerAddr` et `DhtLookupResult`.

Cette formulation est plus exacte et évite de sur-vendre le périmètre du correctif.

---

## Correction obligatoire n°4 — retirer l'idée de déprécier techniquement les fallbacks dans ce patch

Le plan V2 propose notamment :

- de marquer `gossip_bootstrap_peers` comme `#[deprecated]`,
- et de supprimer `bootstrapPeerId` côté tvOS.

Je considère que c'est trop tôt.

### Pourquoi c'est une mauvaise idée dans ce patch

Parce que :

- les fallbacks sont encore utiles pendant la transition,
- le sujet n'est pas encore validé terrain,
- `#[deprecated]` peut générer du bruit ou des warnings inutiles,
- et vous avez potentiellement une hygiène `clippy -D warnings` à préserver.

### Recommandation

Dans ce premier chantier :

- **pas de `#[deprecated]`**,
- **pas de suppression fonctionnelle de l'UI fallback**,
- uniquement :
  - doc de transition,
  - commentaire "fallback / debug only",
  - ou masquage visuel léger si besoin.

### Formulation ferme

Le plan V2 doit conserver les fallbacks **sans dépréciation technique active** tant que la feature relay-assistée n'a pas été validée localement et sur hardware réel.

---

## Points que je challenge, mais qui ne bloquent pas forcément

## 1. `PeerPresent` au lieu de `PeerHint`

Je n'en fais pas un point de blocage.

L'argument de Claude est recevable :

- le relay constate un fait réel,
- ce qui est borné, c'est la diffusion,
- pas la véracité de l'information.

C'est défendable.

### Condition

Si le nom `PeerPresent` est conservé, il faut documenter que :

- c'est une présence relay-observée,
- non exhaustive,
- et certainement pas une preuve de voisinage gossip.

---

## 2. Le nombre de PRs / commits

Je n'en fais pas un débat.

Dans votre contexte :

- repo maîtrisé,
- devices maîtrisés,
- petite équipe,
- cadence rapide,

2 à 3 commits logiques suffisent.

Le plus important n'est pas le nombre de PRs, mais l'absence de patch bouillie multi-couches sans points de contrôle.

---

## 3. Le warning sur `EndpointGone`

V2 a raison de dire que le warning était surtout théorique si on ne copie que :

- le framing,
- l'encode/decode,
- la mécanique de queue,

et pas la logique de diffusion.

Donc ce point n'est pas bloquant.

---

## Ce que j'attends du code, pas seulement du plan

Pour considérer V2 comme vraiment solide, je veux voir apparaître dans le code final les propriétés suivantes.

## Propriété 1 — `add_peer_addr()` devient enfin cohérent

Après patch :

- toute adresse injectée doit être visible par le transport protocolaire,
- **et** par l'endpoint utilisé par gossip.

## Propriété 2 — le runtime ne ment pas

À réception d'un `PeerPresent` :

- le runtime peut enrichir la topologie,
- mais ne doit pas traiter ce signal comme un voisinage déjà établi.

Le vrai voisinage reste confirmé par :

- `GossipEvent::NeighborUp`

## Propriété 3 — la diffusion relay est bornée et non explosive

À l'enregistrement d'un pair :

- le nombre de frames envoyés doit être borné par `k`,
- pas dépendant linéairement du nombre total de clients connectés.

## Propriété 4 — les fallbacks continuent à marcher

Après le patch, le système ne doit pas être moins robuste que l'état actuel sur :

- DHT,
- bootstrap manuel,
- tvOS,
- TUI,
- scénarios cross-relay.

---

## Ce que je considère comme conditions minimales de merge

## Minimum technique

- les tests unitaires ajoutés passent,
- l'API `EndpointAddr` utilisée dans le plan correspond à l'API réelle,
- le flux `peer_present_rx` a une ownership claire,
- il n'y a pas de suppression prématurée de fallback.

## Minimum intégration

- 2 nœuds sur le même relay,
- zéro bootstrap manuel,
- `NeighborUp`,
- message applicatif livré.

## Minimum architecture

- l'injection d'adresse est faite dans le lookup endpoint,
- pas seulement dans `ConnectionPool`.

---

## Verdict final

Le plan V2 mérite d'être poursuivi.

Il a absorbé l'essentiel de la critique précédente.

Il n'est plus dans la catégorie :

- "plan séduisant mais structurellement faux"

Il entre maintenant dans la catégorie :

- **"bon plan, mais qui doit encore préciser sa plomberie et éviter une dépréciation trop tôt"**.

### Jugement synthétique

- **Architecture** : bonne
- **Séquencement** : bon
- **Pragmatisme** : bon
- **Précision d'API** : encore insuffisante par endroits
- **Plomberie d'événements** : à clarifier avant codage

### En une phrase

> V2 est globalement validé, à condition de corriger 4 points :
> l'agrégation des événements relay, la construction réelle de `EndpointAddr`, la formulation du périmètre du bug, et l'abandon de la dépréciation technique trop précoce des fallbacks.

---

## Réponse courte prête à envoyer à Claude

V2 est globalement bon.
Le cœur est validé : `MemoryLookup`, sampling borné, ordre `add_peer_addr()` puis `join_peers()`, fallbacks conservés.

Avant exécution, corrige 4 points :

1. clarifie où s'agrège exactement le flux `PeerPresent` s'il y a plusieurs `RelayTransport`,
2. remplace tout pseudo-code `EndpointAddr::from_parts(...)` par la vraie construction API-compatible,
3. nuance la phrase "le bug affecte AddPeer" → c'est surtout `AddPeerAddr` / `DhtLookupResult`,
4. retire `#[deprecated]` / suppression UI des fallbacks de ce premier patch.

Après ça, le plan devient franchement solide.
