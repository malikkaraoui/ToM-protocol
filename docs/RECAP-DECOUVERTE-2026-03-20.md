# Récapitulatif global — découverte décentralisée (20 mars 2026)

## Résumé exécutif

La découverte décentralisée de ToM repose aujourd'hui sur **trois briques distinctes** :

1. **PeerPresent via relay** — un relay annonce de façon best-effort qu'un peer est présent.
2. **Gossip** — les nodes propagent des annonces de présence et de rôle.
3. **DHT** — publication/résolution d'adresses via Mainline DHT (BEP-0044).

Le point le plus solide dans le code actuel est le chemin **`PeerPresent -> injection d'adresse -> gossip join`**. C'est lui qui rend crédible le scénario où plusieurs nodes connectés au même relay se découvrent rapidement sans échange manuel d'adresse complète.

En revanche, plusieurs formulations trop ambitieuses doivent être nuancées :

- le **gossip ne propage pas les coordonnées réseau complètes** ;
- la **DHT n'implémente pas une découverte “magique sans connaître personne”** au sens fort ;
- le **relay rotatif** existe surtout comme **logique de rôle et de sélection**, pas encore comme système automatisé de relais réellement tournants.

---

## 1. Ce qui est effectivement implémenté

### 1.1 PeerPresent (relay-assisted)

Ce que le code confirme :

- `PeerPresent` existe bien dans `tom-relay`.
- le frame type est bien **13** ;
- la diffusion est **bornée** avec un échantillonnage `k = 8` ;
- le relay envoie un **hint best-effort**, non exhaustif, sans persistance ;
- côté transport/runtime, le hint reçu est bien transformé en tentative réelle de voisinage.

Chaîne observée dans le code :

1. le relay émet `PeerPresent`
2. `tom-connect` le reçoit et le pousse dans un channel
3. `tom-protocol` récupère l'événement
4. le runtime fait `add_peer_addr(...)`
5. puis `gossip.join_peers(...)`
6. puis enrichit la topologie protocole avec `AddPeer`

Conclusion : **PeerPresent n'est pas cosmétique**. Il est branché proprement et constitue aujourd'hui la partie la plus crédible de la découverte automatique.

### 1.2 Gossip

Ce que le gossip fait réellement :

- abonnement à un topic de découverte partagé ;
- émission périodique de `PeerAnnounce` ;
- observation des événements `NeighborUp` / `NeighborDown` ;
- mise à jour de la topologie et du suivi de vivacité.

Ce que transporte `PeerAnnounce` :

- `node_id`
- `username`
- `roles`
- `encryption_key`
- `timestamp`

Conclusion : le gossip **propage bien l'existence d'un peer**, mais **pas son adresse réseau complète**.

### 1.3 DHT (BEP-0044)

Le crate `tom-dht` est réel et implémente :

- publication d'un enregistrement signé ;
- lookup par clé publique ;
- filtrage des enregistrements trop anciens ;
- intégration au runtime.

Le runtime :

- publie l'adresse du node au démarrage ;
- republie périodiquement ;
- peut lancer un lookup DHT pour un peer.

Conclusion : la brique DHT existe bien, elle n'est pas fictive.

---

## 2. Ce que le code prouve vraiment

### 2.1 Oui : découverte rapide entre nodes partageant le même relay

Le code rend plausible le scénario suivant :

- plusieurs nodes se connectent au même relay ;
- le relay envoie des hints `PeerPresent` ;
- les nodes injectent l'adresse relay associée ;
- gossip établit un voisinage ;
- les messages peuvent ensuite circuler.

C'est le point le plus solide du système aujourd'hui.

### 2.2 Oui : le relay reste stateless sur cette fonctionnalité

Le relay :

- ne sert pas d'annuaire persistant ;
- n'expose qu'une présence observée localement ;
- émet des hints best-effort ;
- borne la diffusion.

Nuance importante : il y a bien **une infrastructure relay**, donc on ne parle pas de “zéro infra”, mais bien de **pas de serveur central persistant qui orchestre toute la découverte**.

### 2.3 Oui : un `NodeId` seul peut parfois suffire si le relay est connu

Quand aucune adresse complète n'est stockée pour une cible, le transport peut tenter une connexion via :

- `NodeId + relay_url par défaut`

Cela permet certains scénarios où la connaissance du `NodeId` combinée à un relay partagé est suffisante pour tenter la connexion.

---

## 3. Nuances importantes et limites actuelles

### 3.1 Le gossip ne donne pas automatiquement de quoi joindre tous les peers

Affirmation à nuancer :

> “Si A connaît B et B connaît C, alors A apprend C.”

C'est vrai **au sens topologique / présence**.

Mais ce n'est **pas forcément vrai au sens connectivité complète** car `PeerAnnounce` ne transporte pas les coordonnées réseau détaillées.

Formulation plus exacte :

> Le gossip permet surtout de propager l'existence, la vivacité et certains métadonnées d'un peer, pas de distribuer à lui seul tout le matériel de connexion nécessaire.

### 3.2 La DHT n'est pas aujourd'hui une découverte entièrement autonome sans point de départ

Dans le runtime, le lookup DHT est déclenché lorsqu'on a déjà un `NodeId` à résoudre.

Donc la DHT sert surtout à :

- **résoudre l'adresse d'un peer déjà identifié**.

Formulation à éviter :

> “Fonctionne même sans relay, même sans connaître personne.”

Formulation plus honnête :

> La DHT permet de retrouver les coordonnées d'un peer à partir de son identité, mais elle ne remplace pas à elle seule une stratégie complète d'apparition initiale des peers côté application.

### 3.3 Les tests les plus importants ne sont pas tous verrouillés en CI normale

Plusieurs tests critiques de découverte sont en `#[ignore]`, notamment autour de :

- PeerPresent auto-discovery ;
- auto-discovery sans connect manuel ;
- DHT end-to-end.

Conséquence :

- cela ne prouve pas que la fonctionnalité ne marche pas ;
- mais cela montre que **la valeur produit principale n'est pas encore solidement verrouillée en exécution standard de la suite de tests**.

### 3.4 Le discours sur le DHT n'est pas totalement aligné avec les tests

Le code DHT est réel, mais certains commentaires de tests racontent encore un scénario de fallback ou de stub.

Conséquence :

- soit les tests n'ont pas été remis à jour ;
- soit la confiance dans le chemin DHT réel reste incomplète ;
- soit les deux.

Dans tous les cas, cela affaiblit la lisibilité globale du niveau de maturité.

---

## 4. Sur le relay rotatif

### Ce qui existe déjà

Le code contient :

- une logique de rôles `Peer` / `Relay` ;
- un scoring de contribution ;
- promotion / démotion ;
- un `RelaySelector` ;
- une logique de sélection de chemin côté protocole.

### Ce qui ne semble pas encore complètement implémenté

Je ne vois pas, dans l'état lu :

- le démarrage automatique d'un vrai `tom-relay` par un node promu ;
- la publication/exposition complète de ce nouveau relay comme infra réseau ;
- une migration automatique généralisée des autres nodes vers ce relay ;
- un véritable système de relay rotatif de bout en bout exploitable sans intervention.

Conclusion :

> Le relay rotatif existe aujourd'hui surtout comme **capacité logique / architecture cible**, pas encore comme **feature réseau intégrée de bout en bout**.

---

## 5. Évaluation franche des affirmations entendues

### Correct

- PeerPresent existe vraiment.
- `type 13` est correct.
- l'échantillonnage borné `k=8` est correct.
- la chaîne relay -> transport -> runtime -> gossip est réelle.
- le relay ne stocke pas la présence de manière persistante.
- plusieurs nodes sur le même relay peuvent se découvrir rapidement sans échange manuel d'adresse complète.

### Correct mais trop vendeur

- “Le gossip découvre organiquement le réseau.”
- “La DHT fonctionne même sans connaître personne.”
- “Le relay rotatif est la suite naturelle déjà presque là.”

### Reformulation plus honnête

- le gossip diffuse surtout de la **présence** et de la **métadonnée**, pas toutes les coordonnées réseau ;
- la DHT aide à **résoudre** un peer déjà identifié ;
- le relay rotatif est **préparé conceptuellement**, pas encore livré comme boucle complète autonome.

---

## 6. Conclusion générale

Le travail réalisé sur la découverte n'est **pas du pipeau**. Il y a une vraie base technique, et la partie **PeerPresent + raccord runtime/gossip** est sérieuse.

Le système actuel semble crédible pour :

- faire se rencontrer rapidement quelques nodes connectés à un même relay ;
- enrichir la topologie avec des événements de présence ;
- préparer une architecture plus décentralisée.

Mais il faut rester exact sur ce qui est déjà prouvé :

- **oui** à la découverte relay-assisted utile ;
- **oui** au gossip comme propagation de présence ;
- **oui** au DHT comme brique réelle ;
- **non** à l'idée que tout le problème de découverte universelle est entièrement résolu de manière générale et parfaitement prouvée ;
- **non** à l'idée que le relay rotatif complet est déjà effectivement là.

En une phrase :

> Le cœur PeerPresent est solide, mais le récit global sur la découverte reste aujourd'hui plus ambitieux que ce que le code et les tests prouvent de façon incontestable.
