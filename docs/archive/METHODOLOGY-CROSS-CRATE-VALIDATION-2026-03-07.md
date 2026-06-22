# Méthodologie proposée — validation des patches Rust cross-crates

Date : 2026-03-07  
Auteur : GitHub Copilot  
Destinataire : Claude  
Sujet : faire évoluer la méthode de travail sur les patches Rust multi-crates, en particulier sur discovery / relay / gossip

## Pourquoi j’ouvre ce sujet

Le double aller-retour récent sur `PeerPresent` n’est pas un simple accident de parcours.

Il met en évidence un point méthodologique important :

> sur ce repo, un patch peut être **correct sur le fond**, **compilable localement**, et malgré tout **pas encore acceptable au niveau CI**.

Autrement dit, il existe ici plusieurs niveaux de validation :

1. **validité architecturelle**,
2. **validité compilation locale**,
3. **validité CI / lint / surface réelle d’intégration**.

Le problème n’est pas que le plan V3 était mauvais.
Le problème est que la barre de validation utilisée au moment du premier push était encore trop basse par rapport à la barre réelle du repo.

C’est précisément ce point que je propose de corriger dans la méthode.

---

## Position de fond

Je propose une règle simple et ferme.

> Un correctif Rust cross-crates n’est plus considéré comme “terminé” tant qu’il n’a pas passé au moins une validation `clippy -D warnings` représentative de la surface CI réellement impactée.

Et pour les sujets `discovery` / `relay` / `gossip`, j’ajoute une seconde règle :

> Il faut raisonner par **surface réelle d’intégration** et non par **crate local modifié seulement**.

Cela veut dire très concrètement :

- ne pas s’arrêter à “le crate compilait chez moi” ;
- ne pas s’arrêter à “le crate touché passe ses tests de compilation” ;
- ne pas supposer que le crate modifié est le seul endroit où la CI exercera le changement.

---

## Le problème de la méthode actuelle

La méthode implicite qui crée du déchet ressemble à ceci :

1. on modifie un crate,
2. on fait `cargo build -p <crate touché>`,
3. parfois on fait un test ciblé,
4. on pousse,
5. la CI révèle ensuite un autre niveau d’exigence.

Ce fonctionnement est insuffisant dès qu’un sujet traverse plusieurs couches.

Dans le cas `PeerPresent`, le changement touchait ou influençait au minimum :

- `tom-relay`,
- `tom-connect`,
- `tom-transport`,
- `tom-protocol`,
- et indirectement parfois `tom-stress` via la surface de validation CI.

Le sujet n’était donc pas “un petit patch local dans `tom-connect`”.
Le sujet était un **patch d’intégration cross-crates**.

---

## Nouvelle méthodologie proposée

## Règle 1 — distinguer trois niveaux de validation

Tout patch doit être pensé et validé à trois niveaux.

### Niveau A — validité architecturelle

Questions à poser :

- le design est-il cohérent avec le code réel ?
- le bon composant reçoit-il la bonne responsabilité ?
- les invariants inter-crates sont-ils respectés ?
- l’ordre des opérations est-il correct ?

Exemple `PeerPresent` :

- injection dans `MemoryLookup` et pas seulement dans `ConnectionPool`,
- `add_peer_addr()` avant `join_peers()`,
- agrégation multi-relay au bon niveau.

### Niveau B — validité compilation locale

Questions à poser :

- le crate touché compile-t-il ?
- le crate downstream compile-t-il ?
- les cibles de test pertinentes compilent-elles ?

Exemple :

- oubli de `mut`,
- type non accessible,
- import manquant,
- signature incorrecte.

### Niveau C — validité CI réelle

Questions à poser :

- `clippy -D warnings` passe-t-il sur la surface réellement concernée ?
- un job CI dépendant exerce-t-il un autre crate que celui qu’on a modifié ?
- la feature est-elle propre du point de vue du standard du repo, pas seulement du compilateur ?

Exemple :

- `clippy::type_complexity`,
- jobs `stress`, `relay`, `transport`, `protocol`,
- hooks ou checks repo plus stricts que le build local.

---

## Règle 2 — workflow minimum pour tout patch Rust cross-crates

Pour tout patch Rust cross-crates, je propose la séquence minimale suivante.

### Étape 1 — build du crate touché

```text
cargo build -p <crate touché>
```

Objectif :

- attraper les erreurs immédiates de compilation,
- corriger la plomberie locale.

### Étape 2 — build du crate downstream principal

```text
cargo build -p <crate downstream>
```

Objectif :

- vérifier que le changement se branche réellement dans la couche consommatrice,
- éviter les faux positifs “le bas compile, le haut casse”.

### Étape 3 — compilation des tests du crate touché

```text
cargo test -p <crate touché> --lib --no-run
```

Objectif :

- attraper les chemins `#[cfg(test)]`,
- éviter les oublis de champs, imports ou signatures dans les helpers de test.

### Étape 4 — `clippy` strict sur le crate touché

```text
cargo clippy -p <crate touché> -- -D warnings
```

Objectif :

- considérer enfin la qualité CI comme partie intégrante de “corrigé”.

### Étape 5 — `clippy` strict sur le crate CI réellement concerné

Si la CI cible un autre crate dépendant, ajouter :

```text
cargo clippy -p <crate CI concerné> -- -D warnings
```

Objectif :

- raisonner par **surface de validation réelle**, pas seulement par crate modifié.

---

## Application au cas `PeerPresent`

Dans le cas `PeerPresent`, les surfaces minimales pertinentes sont en général :

- `tom-connect`
- `tom-transport`
- `tom-protocol`
- et parfois `tom-stress` si le job CI le tire indirectement

Donc le minimum réaliste devient :

```text
cargo build -p tom-connect
cargo build -p tom-transport
cargo build -p tom-protocol
cargo test -p tom-connect --lib --no-run
cargo clippy -p tom-connect -- -D warnings
cargo clippy -p tom-protocol -- -D warnings
cargo clippy -p tom-stress -- -D warnings
```

Cette liste pourra être ajustée selon la nature exacte du patch, mais l’idée est là :

> la validation doit suivre le **chemin d’impact**, pas seulement l’endroit du diff.

---

## Règle 3 — pour les sujets discovery / relay, ajouter un réflexe spécifique

Sur les sujets `discovery` / `relay` / `gossip`, il faut systématiquement ajouter ce réflexe :

- vérifier les impacts sur jobs **relay**,
- vérifier les impacts sur jobs **discovery**,
- vérifier les impacts sur jobs **stress**,
- ne jamais supposer que seul le crate édité est concerné.

Pourquoi ?

Parce que ces sujets touchent souvent :

- la signalisation,
- la découverte,
- la connectivité effective,
- les surfaces d’intégration runtime,
- et parfois la robustesse de scénarios complets exercés ailleurs que dans le crate patché.

Ce sont donc des sujets à **large cône d’impact**.

---

## Règle 4 — définir une vraie “Definition of Done” pour les patches Rust

Je propose la définition suivante.

Un patch Rust cross-crates n’est “done” que si les cinq conditions suivantes sont vraies.

### 1. Le design est validé

- le bon composant porte la bonne responsabilité,
- le flow inter-crates est cohérent,
- les invariants critiques sont respectés.

### 2. Le crate touché compile

- pas de dette de compilation locale,
- pas d’erreur de base ignorée.

### 3. Le ou les downstreams compilent

- le patch ne casse pas la couche consommatrice,
- le câblage réel est validé.

### 4. `clippy -D warnings` passe sur au moins une surface représentative

- le patch est acceptable selon les standards du repo,
- pas seulement selon le compilateur.

### 5. La surface CI impactée a été anticipée

- on a identifié quel job va réellement exercer la modification,
- on n’attend pas passivement que GitHub serve de premier détecteur.

---

## Règle 5 — changer l’ordre mental du travail

L’erreur méthodologique la plus commune est celle-ci :

> “j’ai modifié `tom-connect`, donc je valide `tom-connect`.”

Je propose de remplacer ce réflexe par celui-ci :

> “j’ai modifié un point d’entrée qui impacte plusieurs surfaces ; je valide la chaîne réelle jusqu’à la surface CI qui portera le jugement final.”

C’est une petite différence de formulation.
Mais en pratique, elle change beaucoup de choses.

---

## Proposition de workflow opérationnel

Voici un workflow que je recommande pour Claude sur les prochains patches de ce type.

## Phase 0 — cartographie d’impact avant code

Avant de coder, identifier :

- crate modifié,
- crate consommateur direct,
- crate consommateur indirect,
- job CI le plus probable à casser.

### Exemple

Pour un patch `PeerPresent` :

- source : `tom-connect`
- downstream direct : `tom-transport`
- downstream d’intégration : `tom-protocol`
- surface CI potentielle : `tom-stress`

## Phase 1 — implémentation locale

Coder le patch avec l’objectif de faire passer :

- le crate touché,
- puis le downstream principal.

## Phase 2 — validation structurée

Valider dans cet ordre :

1. `build` crate touché,
2. `build` downstream,
3. `test --no-run` crate touché,
4. `clippy` crate touché,
5. `clippy` crate CI concerné.

## Phase 3 — seulement ensuite commit / push

Un patch ne doit pas être considéré comme “prêt à pousser” si l’étape `clippy` représentative n’a pas été faite.

---

## Ce que cette méthode évite

Cette méthode n’éliminera pas toutes les erreurs.
Mais elle élimine une catégorie précise de déchet :

- le patch qui compile localement,
- le patch qui a l’air correct,
- le patch qu’on pousse trop tôt,
- et la CI qui révèle ensuite une exigence prévisible qu’on aurait pu tester avant.

Dit autrement :

> elle réduit les aller-retours évitables.

Et c’est exactement ce qu’on veut ici.

---

## Objection possible

On pourrait dire :

> “ça rajoute des validations, donc ça ralentit.”

Réponse :

- oui, ça ajoute un peu de coût en local,
- mais ce coût est inférieur au coût d’un push raté + une CI rouge + un second commit de réparation.

Sur les patches cross-crates, surtout en Rust, ce compromis est largement rentable.

---

## Version courte de la règle proposée

Si je devais la condenser en une doctrine simple pour Claude :

> Pour tout patch Rust cross-crates, ne valide pas seulement le crate que tu as modifié. Valide la chaîne d’impact jusqu’à une surface `clippy -D warnings` représentative du job CI réel.

Et pour `discovery` / `relay` :

> Raisonne par surface d’intégration réelle (`connect`, `transport`, `protocol`, `stress`) et pas par localisation du diff.

---

## Recommandation finale

Je recommande d’adopter explicitement cette méthodologie pour la suite du chantier `PeerPresent` et, plus largement, pour tous les patches Rust multi-couches du repo.

### En pratique

Je propose la règle opérationnelle suivante :

- un patch Rust cross-crates n’est **pas terminé** après un `cargo build`,
- il n’est **pas terminé** après un `cargo test --no-run`,
- il n’est **considéré terminé** qu’après au moins un `clippy -D warnings` aligné sur la surface CI concernée.

C’est, selon moi, la bonne évolution de méthode après ce qu’on vient de vivre.

---

## Réponse courte prête à transmettre à Claude

Je propose qu’on change explicitement la méthode de validation pour les patches Rust cross-crates.

Nouveau standard :

1. `cargo build -p <crate touché>`
2. `cargo build -p <crate downstream>`
3. `cargo test -p <crate touché> --lib --no-run`
4. `cargo clippy -p <crate touché> -- -D warnings`
5. si la CI cible un autre crate dépendant : `cargo clippy -p <crate CI concerné> -- -D warnings`

Et sur les sujets `discovery` / `relay`, il faut en plus vérifier explicitement les surfaces `relay`, `discovery`, `stress`, sans supposer que seul le crate modifié compte.

Règle de travail proposée :

> on ne considère plus un correctif Rust comme terminé tant qu’il n’a pas passé au moins un `clippy -D warnings` représentatif du job CI réellement concerné.

C’est la bonne façon d’éviter le double aller-retour qu’on vient de vivre.
