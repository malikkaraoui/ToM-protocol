# Directives pour Claude — chantier relay embedé (20 mars 2026)

## Objectif du chantier

Tu travailles sur **un seul chantier** : permettre à un node ToM de **démarrer, superviser et arrêter un vrai `tom-relay` embarqué dans son propre process**, de manière fiable, observable et testable.

Ce chantier est la **fondation** du futur relay rotatif. Il ne s'agit **pas** encore d'implémenter la rotation complète.

---

## Scope exact

### In scope

- démarrage d'un vrai `tom-relay` embarqué par node ;
- arrêt propre ;
- supervision minimale ;
- état observable (`starting`, `healthy`, `stopped`, `failed`) ;
- pilotage depuis le runtime sans casser l'architecture actuelle ;
- tests unitaires et tests d'intégration ciblés.

### Hors scope absolu pour cette étape

Ne pas implémenter maintenant :

- publication automatique du nouveau relay au réseau ;
- migration automatique des autres nodes vers ce relay ;
- consommation distribuée de nouveaux relays publiés ;
- rotation globale de relay de bout en bout ;
- refonte globale de la découverte.

En clair : **on construit le socle d'exécution d'un relay embarqué**, pas le système complet de relay rotatif.

---

## Travail demandé avant toute grosse modification

### 1. Cartographier l'existant

Inspecte précisément :

- `crates/tom-relay` — API serveur réutilisable en embarqué ;
- `crates/tom-transport` — cycle de vie du node, config relay, intégration transport ;
- `crates/tom-protocol` — `RuntimeState`, `runtime_loop`, `RuntimeEffect`, `RuntimeCommand`, `RoleManager` ;
- tous les endroits où le mot "relay" désigne :
  - un **rôle logique** ;
  - un **vrai serveur relay** ;
  - une **URL de relay** ;
  - un **chemin réseau via relay**.

Objectif : **séparer proprement les concepts** avant de toucher au code.

### 2. Produire un mini design avant patch large

Avant toute refacto un peu sérieuse, documente clairement :

- où vit le composant `EmbeddedRelay` ;
- qui décide du `start` / `stop` ;
- qui observe l'état du relay ;
- comment on remonte `healthy` / `failed` ;
- comment on évite d'introduire de l'async I/O dans l'état pur.

---

## Contraintes d'architecture non négociables

### Règle 1 — ne pas mettre le lifecycle réseau dans `RuntimeState`

`RuntimeState` doit rester **pur** :

- pas de spawn de tâche ;
- pas de serveur réseau ;
- pas de handle async ;
- pas de supervision I/O directe.

Si l'état veut un relay local, il doit **émettre une intention**, pas le démarrer lui-même.

**Approche attendue :**

- `RuntimeState` produit un `RuntimeEffect`
- la boucle / l'executor async exécute réellement le démarrage ou l'arrêt
- l'état observe le résultat via événements ou commandes de retour

### Règle 2 — ne pas confondre rôle logique et relay réellement prêt

Un node peut être :

- promu `PeerRole::Relay` au niveau logique ;
- mais ne pas encore avoir de relay embarqué démarré ;
- ou avoir un relay démarré mais non sain ;
- ou être sain localement mais pas encore publiable au réseau.

Il faut séparer explicitement :

- `role logique`
- `relay embarqué démarré`
- `relay embarqué sain`
- `relay publiable / annonçable`

Sans cette séparation, vous allez annoncer des faux relays.

### Règle 3 — aucune publication automatique tant que la santé n'est pas prouvée

Même si le relay démarre localement :

- ne pas le propager ;
- ne pas le traiter comme alternatif valide ;
- ne pas le publier à d'autres nodes ;
- tant que le bind, l'URL effective et l'état sain ne sont pas confirmés.

### Règle 4 — ne pas casser les invariants wire existants

Le chantier doit **ajouter une capacité**, pas casser :

- `PeerPresent`
- framing relay existant
- comportement actuel de découverte
- invariants wire/protocol déjà en place

---

## Architecture recommandée

### Recommandation forte

Introduire un composant dédié du type :

- `EmbeddedRelayService`
- ou `EmbeddedRelayManager`

Ce composant doit vivre **hors de `RuntimeState`**, côté orchestration async.

### Répartition recommandée des responsabilités

#### `RuntimeState`

Décide qu'un relay local est souhaité ou non.

#### `RuntimeEffect`

Exprime des intentions du style :

- `StartEmbeddedRelay`
- `StopEmbeddedRelay`
- éventuellement `CheckEmbeddedRelayHealth`

#### boucle / executor async

Effectue réellement :

- le démarrage du relay ;
- l'arrêt ;
- la supervision minimale ;
- la remontée d'état vers le runtime.

#### service dédié

Encapsule :

- le serveur `tom-relay` embarqué ;
- le handle de vie ;
- le statut ;
- l'URL effective ;
- les erreurs de démarrage.

---

## API cible minimale

Je veux voir émerger une abstraction claire, avec une forme équivalente à :

- `start(config) -> Result<RelayInstance, Error>`
- `stop()`
- `status() -> Starting | Healthy | Stopped | Failed`
- `public_url() -> Option<RelayUrl>`

Les noms exacts importent moins que la séparation nette des responsabilités.

---

## Découpage de travail attendu

### Phase A — capacité locale embarquée

But :

- démarrer un vrai `tom-relay` dans le même process qu'un node ;
- bind configurable ;
- arrêt propre ;
- supervision basique.

Livrable attendu :

- un service local testable indépendamment ;
- aucune publication réseau automatique.

### Phase B — branchement propre au runtime

But :

- permettre au runtime de piloter `start` / `stop` ;
- sans introduire d'async dans `RuntimeState`.

Livrable attendu :

- nouveaux effets / événements / commandes si nécessaire ;
- séparation claire entre rôle logique et disponibilité réelle.

### Phase C — observabilité minimale

Ajouter au minimum :

- logs clairs ;
- état courant ;
- URL effective ;
- erreurs de bind / port / démarrage ;
- événement explicite en cas d'échec.

Sans observabilité, le debug sera horrible plus tard.

---

## Pièges à éviter

### 1. Faux positif de relay utilisable

Un relay qui écoute seulement en local ou sur une interface non exploitable par d'autres peers n'est **pas** encore un relay publiable.

Distinguer :

- relay démarré localement ;
- relay réellement exploitable par d'autres nodes.

### 2. Surcharger `PeerAnnounce`

Ne pas transformer `PeerAnnounce` en gros message fourre-tout pour annoncer :

- URL de relay ;
- reachability ;
- état du relay embarqué ;
- métadonnées réseau détaillées.

Si publication il y a plus tard, elle mérite probablement un message dédié, signé, versionné et sémantiquement propre.

### 3. Coupler promotion de rôle et démarrage sans garde-fous

Évite un comportement du type :

- rôle promu -> démarrage immédiat sans contrôle

Prévoir au minimum :

- feature flag,
- config explicite,
- ou mécanisme progressif.

Sinon chaque promotion logique déclenchera des serveurs partout. Mauvais plan.

### 4. Patch trop large

Ne mélange pas dans un seul patch :

- relay embedé ;
- publication ;
- migration ;
- découverte ;
- rotation automatique ;
- politique distribuée.

Le patch doit rester **petit, testable, incrémental**.

---

## Critères d'acceptation minimum

Le chantier n'est **pas terminé** tant que les points suivants ne sont pas vrais :

- un node peut démarrer un `tom-relay` embarqué ;
- un node peut l'arrêter proprement ;
- les erreurs de démarrage sont remontées proprement ;
- l'état du relay est observable ;
- le runtime peut piloter ce lifecycle sans I/O dans `RuntimeState` ;
- au moins les tests suivants existent :
  - test unitaire du manager/service ;
  - test d'intégration start/stop ;
  - test d'échec de bind ;
  - test garantissant qu'aucune publication automatique n'a lieu tant que le relay n'est pas sain.

---

## Ce que j'attends du patch

Je veux un patch qui :

- introduit une abstraction propre pour le relay embarqué ;
- garde les responsabilités bien séparées ;
- ne mélange pas rôle logique et serveur effectif ;
- reste incrémental ;
- ajoute les tests nécessaires.

Je ne veux **pas** d'un patch qui tente en même temps de livrer le relay rotatif complet.

---

## Ordre conseillé pour la suite

Une fois ce point 1 terminé proprement, l'ordre conseillé est :

1. relay embarqué local ;
2. état santé / prêt ou non ;
3. publication explicite du relay ;
4. consommation de cette publication par les autres nodes ;
5. migration automatique ;
6. politique de rotation.

---

## Formulation synthétique à garder en tête

> Tu ne construis pas encore le relay rotatif.
> Tu construis le socle d'exécution d'un vrai relay embarqué, proprement isolé, supervisé et observable.

---

## Version ultra-courte

Commence par le point 1 uniquement : un vrai `tom-relay` embarqué par node, start/stop propre, état santé observable, aucune publication automatique tant que ce n'est pas prêt.

Ne mets rien d'async dans `RuntimeState`. Passe par un service dédié + effets/runtime loop.

Ne mélange pas rôle logique `Relay` et relay réellement disponible.

Hors scope : publication, migration, rotation complète.

Livre un patch incrémental avec tests de démarrage, arrêt, échec de bind et état healthy.
