# Validation du design — relay embedé (20 mars 2026)

## Verdict global

Oui, le design proposé est **bon dans sa direction générale**.

Il respecte les principes importants du codebase :

- `RuntimeState` reste pur ;
- le runtime continue de fonctionner selon le pattern existant `RuntimeState -> RuntimeEffect -> boucle async -> exécution réelle` ;
- le relay embedé vit hors de l'état pur ;
- les retours d'état peuvent être réinjectés proprement dans la machine de contrôle.

En revanche, je recommande **de ne pas coder exactement la proposition telle quelle** sans corriger 4 points structurants.

---

## Ce que je valide tel quel

### 1. Le fait d'embedder directement `tom-relay`

C'est le bon choix.

`tom-relay` expose déjà une API library exploitable directement :

- `Server::spawn(config) -> Result<Server>`
- `server.shutdown()`

Donc il n'y a **pas besoin** de lancer un process séparé.

### 2. Le respect du pattern architectural du runtime

C'est exactement ce qu'il faut faire :

- `RuntimeState` ne démarre rien lui-même ;
- il exprime une intention via un `RuntimeEffect` ;
- la boucle async exécute réellement le démarrage ou l'arrêt ;
- l'état pur reste sans I/O.

### 3. La séparation des 4 états conceptuels

Cette séparation est juste et nécessaire :

1. rôle logique (`PeerRole::Relay`)
2. relay démarré
3. relay sain (`healthy`)
4. relay publiable

C'est une très bonne base de design.

### 4. L'idée d'un service dédié

Avoir un `EmbeddedRelayService` séparé est la bonne approche.

Le lifecycle d'un vrai relay embarqué ne doit pas être dilué dans :

- la topo ;
- le rôle logique ;
- la sélection de chemin ;
- ou l'état pur du protocole.

---

## Les 4 corrections importantes à faire avant d'implémenter

## 1. L'emplacement `src/relay/embedded.rs` n'est pas le bon

### Problème

Il existe déjà :

- `crates/tom-protocol/src/relay.rs`

Créer en plus :

- `crates/tom-protocol/src/relay/embedded.rs`

introduit une ambiguïté/modularisation inutile à ce stade.

### Recommandation

Mettre le service dans un endroit plus cohérent avec sa nature runtime/orchestration, par exemple :

- `crates/tom-protocol/src/runtime/embedded_relay.rs`
- ou `crates/tom-protocol/src/embedded_relay.rs`

### Recommandation finale

Le meilleur emplacement est probablement :

- `crates/tom-protocol/src/runtime/embedded_relay.rs`

Parce qu'il s'agit d'un **service de lifecycle async**, pas d'une logique de topo/sélection relay.

---

## 2. Il faut un retour vers `RuntimeState`, pas seulement des événements applicatifs

### Problème

Les événements proposés :

- `EmbeddedRelayStarted { url }`
- `EmbeddedRelayFailed { error }`
- `EmbeddedRelayStopped`

sont bien pour l'observabilité côté app/logs/UI.

Mais ils ne suffisent pas à eux seuls à maintenir une machine d'état interne cohérente.

### Risque concret

Si `RuntimeState` ne reçoit pas un retour de contrôle explicite, alors il peut :

- réémettre `StartEmbeddedRelay` alors que le relay tourne déjà ;
- ne pas savoir qu'un démarrage a échoué ;
- ne pas pouvoir mettre en place un backoff ;
- confondre rôle logique et disponibilité réelle.

### Recommandation

Il faut deux couches distinctes :

#### 1. retour de contrôle vers le runtime

Par exemple :

- `RuntimeCommand::EmbeddedRelayStarted { url }`
- `RuntimeCommand::EmbeddedRelayFailed { error }`
- `RuntimeCommand::EmbeddedRelayStopped`

#### 2. événements app-facing

Par exemple :

- `ProtocolEvent::EmbeddedRelayStarted { url }`
- `ProtocolEvent::EmbeddedRelayFailed { error }`
- `ProtocolEvent::EmbeddedRelayStopped`

### Pourquoi c'est important

Ainsi :

- la boucle async démarre/arrête le service ;
- elle réinjecte un retour dans le runtime ;
- `RuntimeState` reste la source de vérité logique ;
- l'app reçoit aussi des événements lisibles.

C'est la solution propre.

---

## 3. `public_url` est un mauvais nom à cette étape

### Problème

Le terme `public_url` suppose déjà quelque chose de plus fort que ce qui est réellement acquis.

À ce stade, le chantier ne prouve pas encore :

- joignabilité publique réelle ;
- publication au réseau ;
- adoption par d'autres nodes ;
- reachability externe confirmée.

### Recommandation

Utiliser un nom factuel, par exemple :

- `bound_relay_url`
- `listen_url`
- `local_relay_url`

### Recommandation finale

Le meilleur nom ici est probablement :

- `bound_relay_url`

Pourquoi :

- il exprime une URL effective issue du bind ;
- il ne sur-vend pas la notion de relay public ;
- il reste compatible avec une future séparation entre "bind effectif" et "relay publiable".

---

## 4. `StartEmbeddedRelay { bind_addr }` est probablement trop pauvre

### Problème

Un simple `SocketAddr` va devenir vite trop limité.

Très rapidement, tu vas vouloir distinguer au moins :

- bind fixe ou port `0` ;
- mode dev / embedded ;
- HTTP only ou TLS ;
- access policy ;
- éventuellement une URL override plus tard.

### Recommandation

Créer dès maintenant une petite config dédiée :

- `EmbeddedRelayConfig { bind_addr, ... }`

Même si le MVP ne remplit qu'un ou deux champs.

### Pourquoi c'est mieux

- API plus stable ;
- moins de casse au prochain patch ;
- meilleure lisibilité de l'intention ;
- meilleure extensibilité sans dette immédiate.

---

## Design corrigé recommandé

## Fichier

- `crates/tom-protocol/src/runtime/embedded_relay.rs`

## Types recommandés

### `EmbeddedRelayStatus`

- `Stopped`
- `Starting`
- `Healthy`
- `Failed(String)`

### `EmbeddedRelayConfig`

MVP minimal :

- `bind_addr: SocketAddr`

Évolutif ensuite vers :

- `dev_mode: bool`
- config TLS
- access config
- override d'URL, etc.

### `EmbeddedRelayService`

Structure recommandée :

- `status: EmbeddedRelayStatus`
- `server: Option<tom_relay::server::Server>`
- `bind_addr: SocketAddr`
- `bound_relay_url: Option<RelayUrl>`

### API recommandée

- `async fn start(config) -> Result<()>`
- `async fn stop()`
- `fn status() -> EmbeddedRelayStatus`
- `fn bound_relay_url() -> Option<RelayUrl>`

---

## Runtime integration recommandée

### RuntimeEffect

Ajouter des effets du style :

- `StartEmbeddedRelay { config: EmbeddedRelayConfig }`
- `StopEmbeddedRelay`

### Retour boucle -> runtime

Ajouter des commandes de retour du style :

- `RuntimeCommand::EmbeddedRelayStarted { url }`
- `RuntimeCommand::EmbeddedRelayFailed { error }`
- `RuntimeCommand::EmbeddedRelayStopped`

### Events app-facing

Ajouter des événements du style :

- `ProtocolEvent::EmbeddedRelayStarted { url }`
- `ProtocolEvent::EmbeddedRelayFailed { error }`
- `ProtocolEvent::EmbeddedRelayStopped`

### Boucle async

Rôle attendu :

- reçoit `StartEmbeddedRelay`
- appelle `service.start(...)`
- réinjecte `RuntimeCommand::EmbeddedRelayStarted / Failed`
- émet en parallèle un `ProtocolEvent` pour l'observabilité

Même logique pour `stop()`.

---

## Point de vigilance sur le mot `Healthy`

Il faut être **très conservateur** sur cette notion.

À cette étape, `Healthy` doit vouloir dire seulement :

- `Server::spawn(...)` a réussi ;
- le bind s'est fait correctement ;
- une URL effective est disponible ;
- aucune erreur immédiate d'initialisation n'est survenue.

`Healthy` ne doit **pas** signifier :

- publiable au réseau ;
- joignable publiquement ;
- validé WAN ;
- automatiquement sélectionnable comme relay de remplacement.

Formulation recommandée dans le code/doc :

> `Healthy` = serveur embarqué sain du point de vue lifecycle local, pas encore "publiable réseau".

---

## Risques à éviter absolument

### 1. Mélanger rôle logique et disponibilité réelle

Ne jamais faire l'équation :

- `PeerRole::Relay == relay réseau prêt`

Ce serait faux, et dangereux pour la suite.

### 2. Publier un relay trop tôt

Tant qu'il n'est pas formellement validé comme sain et que la politique de publication n'existe pas, **ne rien publier**.

### 3. Faire un patch trop large

Ne pas mélanger dans ce patch :

- relay embedé ;
- publication ;
- migration ;
- relay rotatif complet ;
- extension discovery.

Le patch doit rester **petit, lisible, testable**.

### 4. Surcharger `PeerAnnounce`

Ne pas utiliser ce chantier comme prétexte pour injecter des URLs de relay dans `PeerAnnounce`.

Ce serait un mauvais couplage sémantique.

---

## Critères d'acceptation recommandés

Le chantier ne devrait pas être considéré terminé tant que les points suivants ne sont pas vrais :

- un node peut démarrer un vrai relay embarqué ;
- l'arrêt est propre ;
- l'échec de bind est remonté proprement ;
- l'état du relay est observable ;
- `RuntimeState` ne contient toujours aucune I/O ;
- la boucle async pilote le lifecycle ;
- des retours sont réinjectés vers le runtime ;
- aucun mécanisme de publication automatique n'est activé ;
- des tests couvrent :
  - start
  - stop
  - bind failure
  - non-publication prématurée

---

## Réponse synthétique à donner à Claude

Design validé dans l'ensemble.

Tu es sur la bonne architecture :

- `RuntimeState` reste pur ;
- le relay embedé vit côté orchestration async ;
- rôle logique, relay démarré, healthy et publiable restent séparés.

Mais corrige avant de coder :

1. ne mets pas ça sous `src/relay/embedded.rs` à cause du `relay.rs` existant ;
2. ajoute un vrai retour boucle -> `RuntimeState`, pas seulement des `ProtocolEvent` ;
3. remplace `public_url` par `bound_relay_url` ou équivalent ;
4. utilise une petite `EmbeddedRelayConfig` plutôt qu'un simple `bind_addr` dans l'effet.

---

## Conclusion

Le design proposé est **bon dans son intention et dans son architecture générale**.

Avec les 4 ajustements ci-dessus, il devient un **très bon point de départ pour coder proprement** le chantier relay embedé sans casser les principes du protocole ni créer de dette inutile.
