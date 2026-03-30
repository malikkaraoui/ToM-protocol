# Récapitulatif — échange méthodologique autour de PeerPresent, CI et validation cross-crates

Date : 2026-03-07  
Auteur : GitHub Copilot  
Contexte : suite de l’implémentation `Relay-Assisted Gossip Discovery (PeerPresent) V3`

## Pourquoi ce document existe

Cet échange a produit quelque chose de plus utile qu’un simple correctif ponctuel.

Il a fait émerger une conclusion méthodologique claire :

> sur ce repo, un patch Rust peut être **bon sur le fond**, **compiler localement**, et malgré tout **ne pas être encore réellement validé** tant qu’il n’a pas passé une surface CI représentative.

Ce document archive :

- le contexte technique,
- la séquence d’erreurs rencontrées,
- les leçons tirées,
- la méthode initialement proposée,
- les retours de Claude,
- la méthode révisée retenue,
- et la manière de formuler le sujet pour les prochains échanges.

---

## Contexte technique

Le chantier de fond concerné est `PeerPresent`, c’est-à-dire la découverte automatique de pairs présents sur un même relay.

### Rappel du problème

Avant ce chantier :

- le relay savait déjà quels pairs étaient connectés,
- il envoyait déjà `EndpointGone` quand un pair partait,
- mais il n’y avait pas l’équivalent `PeerPresent` quand un pair arrivait.

Conséquence :

- deux pairs connectés au même relay ne se découvraient pas automatiquement,
- le gossip nécessitait encore un bootstrap manuel ou un autre mécanisme de découverte.

### Bug pré-existant identifié

Le point le plus important de toute la séquence a été la correction de ce bug de plomberie :

- `TomNode::add_peer_addr()` alimentait seulement `ConnectionPool`,
- mais pas l’`address_lookup` de l’`Endpoint`,
- alors que `tom-gossip` diale via `endpoint.connect(endpoint_id)`.

Donc :

- une adresse pouvait être connue du transport,
- tout en restant invisible au gossip.

La correction V3 a consisté à :

- introduire `MemoryLookup` dans `TomNode`,
- l’injecter dans le builder de l’`Endpoint`,
- et nourrir ce lookup dans `add_peer_addr()` avant `pool.add_addr()`.

---

## Ce qui s’est passé pendant l’implémentation

Le sujet a ensuite révélé un point important :

> une feature peut être validée conceptuellement et rester encore incomplètement validée au niveau intégration réelle.

### Séquence observée

#### Étape 1 — correction du premier échec local

Un premier échec de compilation locale a été corrigé :

- oubli de mutabilité sur `transports` dans `crates/tom-connect/src/socket.rs`
- problème : `take_peer_present_rx()` nécessitait un accès mutable

Correctif :

- `let transports = ...` → `let mut transports = ...`

Validation locale ensuite :

- `cargo build -p tom-connect` ✅
- `cargo build -p tom-protocol` ✅
- `cargo test -p tom-connect --lib --no-run` ✅
- `cargo test -p tom-protocol --lib --no-run` ✅

#### Étape 2 — CI rouge sur un second niveau d’exigence

Après push, la CI a révélé un second problème différent :

- `clippy::type_complexity`
- sur le champ `peer_present_rx` dans `crates/tom-connect/src/socket.rs`

Cette erreur n’était pas une erreur de compilation, mais une erreur de conformité au standard du repo, car la CI exécute :

- `clippy -D warnings`

Correctif appliqué :

- introduction d’alias de types pour simplifier le type du receiver partagé

Validation ensuite :

- `cargo clippy -p tom-connect -- -D warnings` ✅
- `cargo clippy -p tom-stress -- -D warnings` ✅

---

## Leçon principale tirée de cet incident

L’incident n’a pas montré que le plan V3 était mauvais.
Il a montré autre chose :

> la bonne unité de validation sur ce repo n’est pas seulement “le crate touché compile”.

Il faut distinguer au moins trois niveaux de vérité.

### 1. Vérité architecturelle

Questions typiques :

- le bon composant porte-t-il la bonne responsabilité ?
- le flow inter-crates est-il cohérent ?
- les invariants critiques sont-ils respectés ?

Exemple `PeerPresent` :

- injection dans `MemoryLookup` et pas seulement dans `ConnectionPool`
- ordre `add_peer_addr()` puis `join_peers()`
- agrégation multi-relay au bon niveau

### 2. Vérité compilation locale

Questions typiques :

- le crate touché compile-t-il ?
- le downstream compile-t-il ?
- les chemins de test compilent-ils ?

Exemple :

- `mut` manquant
- import absent
- signature incorrecte

### 3. Vérité CI réelle

Questions typiques :

- `clippy -D warnings` passe-t-il ?
- un autre crate dépendant est-il exercé par un job CI plus large ?
- la patch est-elle acceptable selon les standards du repo, pas seulement du compilateur ?

Exemple :

- `clippy::type_complexity`
- surface `tom-stress` exercée indirectement

---

## Première proposition méthodologique

À partir de cet incident, la première règle proposée a été la suivante :

> un correctif Rust cross-crates n’est plus considéré comme “terminé” tant qu’il n’a pas passé au moins une validation `clippy -D warnings` représentative de la surface CI réellement impactée.

Et pour les sujets `discovery` / `relay` / `gossip` :

> il faut raisonner par **surface réelle d’intégration** et non par **crate local modifié seulement**.

### Workflow minimum proposé initialement

Pour tout patch Rust cross-crates :

```text
cargo build -p <crate touché>
cargo build -p <crate downstream>
cargo test -p <crate touché> --lib --no-run
cargo clippy -p <crate touché> -- -D warnings
```

Et si la CI cible un autre crate dépendant :

```text
cargo clippy -p <crate CI concerné> -- -D warnings
```

### Application au cas PeerPresent

Sur `PeerPresent`, les surfaces minimales identifiées étaient :

- `tom-connect`
- `tom-transport`
- `tom-protocol`
- parfois `tom-stress`

Une recommandation dédiée a donc été formulée pour les patches discovery/relay :

- vérifier explicitement les impacts sur les surfaces `relay`, `discovery`, `stress`
- ne pas supposer que seul le crate édité est concerné

---

## Retour de Claude sur cette première méthode

Claude a lu le document méthodologique initial et a répondu de manière globalement positive.

### Ce que Claude a validé

Claude a jugé pertinents les points suivants :

- le constat général : valider seulement le crate modifié ne suffit pas
- la `Phase 0` de cartographie d’impact avant code
- la `Definition of Done` en 5 points

### Objections / ajustements proposés par Claude

Claude a ensuite formulé 3 types de retours.

#### 1. Point factuel sur `tom-transport`

Claude a demandé si `tom-transport` existait réellement, disant ne pas le voir dans le workspace.

Vérification faite dans le repo :

- `crates/tom-transport/` existe bien
- avec `src/node.rs`, `src/connection.rs`, `src/lib.rs`, etc.

Conclusion :

> cette objection était factuellement incorrecte.

#### 2. Validation finale workspace

Claude a proposé d’ajouter explicitement :

```text
cargo clippy --workspace -- -D warnings
```

comme validation finale la plus sûre.

Cette remarque a été jugée **bonne**.

#### 3. Tests finaux workspace

Claude a aussi proposé d’ajouter :

```text
cargo test --workspace
```

comme validation complète finale, au-delà du simple `--no-run`.

Cette remarque a également été jugée **bonne**.

#### 4. Verbosité du document

Claude a estimé que le document initial était trop long pour être intégré dans `CLAUDE.md` ou une mémoire compacte.

Cette remarque a été jugée **bonne pour l’usage opérationnel**, avec la nuance suivante :

- le document long garde sa valeur comme document de cadrage et d’argumentation,
- mais une version courte est nécessaire pour la doctrine intégrable.

---

## Méthode révisée retenue

La version révisée retenue ne remplace pas la méthode initiale :

- elle la **resserre**,
- elle distingue mieux **validation itérative** et **validation finale**.

### Principe central

Le bon compromis n’est pas de choisir entre validation **par crate** et validation **workspace**.

Il faut les utiliser à des moments différents.

### Pendant le développement itératif

Valider vite et localement :

```text
cargo build -p <crate touché>
cargo build -p <crate downstream>
cargo test -p <crate touché> --lib --no-run
cargo clippy -p <crate touché> -- -D warnings
```

Si un crate CI dépendant important est évident :

```text
cargo clippy -p <crate CI concerné> -- -D warnings
```

### Avant commit / push final

Pour tout patch Rust cross-crates significatif, surtout sur discovery / relay / gossip / transport / runtime :

```text
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

### Doctrine finale formulée simplement

> En itératif, on valide vite par crate.  
> Avant de pousser, on valide large par surface CI réelle — et si le patch est suffisamment transversal, par `workspace`.

---

## Documents produits au cours de cet échange

Plusieurs documents ont été créés pour conserver la matière de travail.

### 1. Reviews du plan PeerPresent

- `docs/REVIEW-PEER-DISCOVERY-2026-03-07.md`
- `docs/REVIEW-PEER-DISCOVERY-V2-2026-03-07.md`
- `docs/REVIEW-PEER-DISCOVERY-V3-2026-03-07.md`

Ils tracent :

- la critique V1,
- les corrections V2,
- la validation finale V3.

### 2. Méthodologie longue

- `docs/METHODOLOGY-CROSS-CRATE-VALIDATION-2026-03-07.md`

Usage :

- ouvrir le sujet méthodologique,
- argumenter,
- justifier le changement de méthode.

### 3. Méthodologie courte

- `docs/METHODOLOGY-CROSS-CRATE-VALIDATION-SHORT-2026-03-07.md`

Usage :

- intégrer une doctrine opérationnelle dans une base d’instructions plus compacte,
- servir de standard quotidien.

---

## Ce qu’il faut retenir

### 1. Le plan V3 n’a pas été invalidé par les échecs rencontrés

Les échecs suivants :

- oubli de mutabilité,
- `clippy::type_complexity`,

n’étaient pas des réfutations du design.

Ils ont montré un écart entre :

- design validé,
- implémentation compilable,
- intégration totalement conforme au repo.

### 2. Le repo impose une vraie discipline d’intégration

Sur ce workspace :

- `cargo build` ne suffit pas,
- `cargo test --no-run` ne suffit pas,
- `clippy -D warnings` compte réellement,
- et la surface CI peut être plus large que le crate modifié.

### 3. Les sujets relay/discovery ont un large cône d’impact

Ces sujets touchent souvent :

- la signalisation,
- la découverte,
- la connectivité runtime,
- les surfaces d’intégration plus larges.

Ils doivent donc être présumés **cross-crates significatifs** jusqu’à preuve du contraire.

### 4. La bonne méthode est à deux vitesses

- **validation itérative par crate** pendant le développement
- **validation finale par workspace** avant push/merge sur les sujets transversaux

---

## Formulation opérationnelle finale

La règle de travail à retenir est la suivante :

> un patch Rust cross-crates n’est pas considéré comme terminé tant qu’il n’a pas passé une validation `clippy -D warnings` représentative de la surface CI réellement impactée ; et pour les patches transversaux significatifs, la validation finale doit inclure `cargo test --workspace` et `cargo clippy --workspace -- -D warnings`.

Pour les sujets `discovery` / `relay` / `gossip` :

> raisonner par surface d’impact réelle (`connect`, `transport`, `protocol`, `stress`) et non par simple localisation du diff.

---

## Utilité de cette archive

Cette archive sert à trois choses :

1. **trace** — conserver la logique qui a conduit au changement de méthode,
2. **qualité** — éviter de retomber dans une validation trop locale,
3. **maturité collective** — transformer un incident de CI en doctrine utile pour la suite.

En bref :

> le sujet n’est plus seulement “comment implémenter `PeerPresent`”,  
> mais aussi “comment valider proprement un patch Rust cross-crates sur ce repo”.
