# ADR-011 — PoP : Proof of Presence

> Statut : **fondateur / en conception** · 2026-07-10
> La présence dans ToM n'est pas déclarée, elle est **prouvée par le travail**.

## Contexte — le bug qui a tout déclenché

Test R13 sur vrais appareils (iPad/iPhone/Apple TV), petit LAN (~5 nœuds réels) :
chaque appareil rapporte **44-50 pairs `Online`** alors que `connected_peers()`
(vrai QUIC direct) ≈ 0. Cause (vérifiée, file:line) :

- Tout pair *découvert* est marqué `Online` **immédiatement, sans preuve de
  connexion réelle** : `state.rs:2341` (DHT), `:2440` (gossip announce),
  `:2473` (neighbor up), `:2101` (AddPeer) → `status: Online` + `record_heartbeat`.
- `relay.rs:124` `online_count()` compte **tous** les `Online`.
- Ré-annonces gossip + redécouverte DHT (60 s) **rafraîchissent le heartbeat**
  en boucle → un pair jamais joint directement reste `Online` indéfiniment.

Conséquence : le runtime traite 44-50 « pairs vivants » fantômes → appareils
contraints saturés → **chemin de contrôle affamé** (writes qui hangent >60 s).

**Ce n'est pas un détail : c'est un bug d'ÉCHELLE.** À 100 nœuds → 50 fantômes,
agaçant. À 1M+ nœuds simultanés (la cible) → tenir une présence *globale* est
**impossible**, l'appareil meurt. La présence doit être **locale au graphe
d'interaction**, jamais globale.

## Décision

**La présence est un Proof of Presence : un fait dérivé d'un travail réseau
constaté et vérifiable — jamais une déclaration.** On **supprime le heartbeat
déclaratif** (le ping « je suis là », vide donc falsifiable et rafraîchi par le
ouï-dire). Le signal de présence, à l'échelle visée, existe déjà en abondance :
c'est le flux de travail réel.

### Principe (une ligne)

> Un nœud est présent s'il détient une preuve **récente, de première main et
> vérifiable** d'avoir fait avancer le message d'un **autre**. Sinon, il est
> seulement *connu* (une adresse), pas *présent*.

## Pourquoi c'est l'essence de ToM (et pas un ajout)

PoP **unifie** quatre notions traitées séparément en **un seul signal** — « avoir
mis sa pierre » :

| Question | = le même signal |
|---|---|
| Présent ? | tu as travaillé récemment |
| Rôle ? | *quel* travail (relais/backup/observer/bootstrap) |
| Réputation ? | *combien*, en fondu |
| Sybil ? | un faux ne peut pas produire de vrai travail |

Parallèle : **PoW** prouve un calcul brûlé, **PoS** un capital gelé — **PoP**
prouve un **travail utile** : la preuve *est* le service rendu. Seule des trois
où prouver sa présence **fait avancer le réseau**.

## L'argument d'échelle (1M+ concurrents)

- **Centralisé (WhatsApp)** : la présence est un **coût** O(N) — des centaines
  de millions de sessions à tenir en datacenter. Plus de monde = plus cher.
- **ToM / PoP** : la présence est un **sous-produit** du routage déjà en cours.
  Zéro infra dédiée, et — l'inverse — **plus le réseau est dense, plus la preuve
  coule**. Se renforce avec la charge.
- À l'échelle, **aucun nœud n'est oisif** : chaque appareil est en permanence sur
  le chemin de relais de quelqu'un. Le « siège chauffé » n'existe que dans un
  réseau clairsemé. Le problème du nœud oisif s'évapore à 1M.

## Les 5 réponses de conception

### 1. Qu'est-ce qui compte comme travail
Crédit PoP **seulement quand un message d'un AUTRE a avancé grâce à toi, et que
je l'ai constaté** :
- **ACK de livraison signé** (décision #1) — preuve la plus forte.
- **Relais utile** — forward d'une enveloppe qui a *vraiment* atteint sa cible
  (fix red-team #7 : `record_relay` conditionné à un message tracké, destinataire ≠ relayeur).
- **Backup** — pas « j'ai stocké » (invérifiable) mais « j'ai **restitué** ».
- **Observe/bootstrap** — crédité si l'info a mené à une vraie connexion.
- **Jamais** : annonce gossip, ré-annonce d'autrui, keepalive vide, auto-déclaration.

### 2. Qui est témoin
Toujours **dyadique** (je t'ai vu faire) ou **scopé-relais** (ce relais a vu ces
pairs), **jamais « tout le monde dit »**. Un relais agrège ses observations de
première main et les publie **signées de lui** → « vu-par-X », pas vérité globale.

### 3. Comment c'est vérifiable
Signature + fraîcheur + usage unique — **zéro crypto nouvelle** :
- Brique atomique = **reçu signé**. ⇒ **rendre l'ACK signé obligatoire** (trou
  red-team ouvert « ACK non signé »). ACK signé sur `(message_id, ts)` ⇒ vivant à `ts`.
- Vue d'un relais = bundle signé `{témoin, présents:[{pair, ref_preuve, ts}], sig}`,
  cru **au prorata du PoP du relais** (fondu), spot-check du reçu possible.
- Anti-rejeu : preuve liée à `(message_id, nonce, ts)` → réutilise le nonce
  anti-replay + purge TTL de `router.rs`. Un ACK ne se rejoue pas (preuve *récente*
  exigée, `message_id` à usage unique).

### 4. Fenêtre de fondu
Pas binaire (décision #4). PoP = **score décroissant [0,1]** : chaque preuve
pousse vers 1, l'absence glisse vers 0. Seuils **dérivés, pas stockés**. Le siège
chauffé (score ~0) **sort tout seul** du set vivant — pas d'événement de purge.
Demi-vie ~30-60 s, fondu complet ~2-3 min. Plafond dur TTL 24 h (décision #2).
Fondu **local et gratuit** (chaque nœud vieillit ses scores au tick).

### 5. Consommation par un appareil faible
Inversion : l'appareil **arrête de calculer** la présence de N pairs. Il
**s'abonne** à la vue signée de son relais, **scopée à ses groupes** :
- Il ne garde que ses **propres preuves directes** + la vue du relais.
- Set vivant borné par « pairs de mes groupes à qui je parle », pas « tout le gossip ».
- Relais qui ment → refs ne vérifient pas / PoP relais faible → **changer de relais**
  (imposés par le réseau, pluriels, remplaçables). Pas de point de confiance unique.
- C'est le **two-tier red-team** (connu bypass / inconnus budget borné) généralisé.

## Migration (le vrai travail, pas la formule)

Aujourd'hui **routage + sélection de relais lisent le set `Online`** (gonflé). Le
dégonfler d'un coup peut les priver de relais. Donc **séparer deux lecteurs** :

| Lecteur | Lit | Source |
|---|---|---|
| Routage / sélection de relais | **Known** (joignable, carnet d'adresses) | découverte (gossip/DHT), sans exigence de vivacité |
| Présence / budget / online_count | **Live** (PoP) | travail constaté récent uniquement |

### Première brique shippable (avant le gros chantier)
Changement chirurgical qui écroule déjà les fantômes : **`record_heartbeat` ne
doit plus être appelé sur ré-annonce gossip ni redécouverte DHT.** Seulement sur :
inbound réel, ACK signé, relais témoin. + introduire `PeerStatus::Known`
(découvert, non compté vivant) distinct de `Online` (PoP). Router lit Known,
présence lit Online.

## Cohérence avec les 7 décisions verrouillées
#1 livraison=ACK (PoP s'y adosse) · #2 TTL 24 h (plafond) · #3 anchor pas
arbitrate (le relais *rapporte*, ne juge pas) · #4 réputation en fondu (le score
décroît) · #5 anti-spam progressif (budget inconnus) · #6 rôles imposés par le
réseau (présence = lecture du signal de contribution) · #7 fondation universelle.

## Challenges ouverts (à red-teamer AVANT de coder en profondeur)
1. **Cold-start / nouveau nœud** : zéro travail ⇒ PoP 0 ⇒ invisible ⇒ ne peut pas
   travailler pour gagner du PoP. Chicken-egg. Piste : starter-budget « inconnu »
   (two-tier) qu'on gravit.
2. **Relais menteur / témoin corrompu** : gonfle ou masque la présence, eclipse d'un
   pair faible qui dépend d'un seul relais.
3. **Farming de PoP / Sybil** : fabriquer du « travail » entre nœuds complices pour
   auto-créditer sans servir de vrais tiers.
4. **Rejeu / fraîcheur** : rejouer des reçus signés, exploiter la fenêtre de fondu.
5. **Dégradation du routage** : le split Known/Live prive-t-il la sélection de
   relais d'options → connectivité en berne ?
6. **Partition / oisiveté légitime** : un utilisateur joignable mais que personne
   ne sollicite pendant X — acceptable qu'il « fonde » ? (à 1M : non-problème ;
   aux bords du réseau : à border).

## Résultats du red-team adversarial (2026-07-10)

4 attaquants en parallèle, verdicts **vérifiés dans le code** (§5) :

**Réfuté (ne pas « corriger ») :**
- « ACK non signé » → **FAUX**. `state.rs` rejette tout ACK `!signature_valid`
  avant de promouvoir un statut (verrou #1). La fondation PoP (reçu signé) **existe**.

**Confirmé + corrigé cette nuit :**
- **Inflation de score par `bandwidth_ratio` non borné** (`scoring.rs`) : `relayed/received`
  pouvait exploser (ratio 10⁶+). **Corrigé** : `BANDWIDTH_RATIO_CAP = 3.0` (un 3× giver
  touche déjà le bonus max). Régression `bandwidth_ratio_is_capped_against_inflation`.

**Confirmé — raffinements du design (mon ADR conflatait deux échelles) :**
- **Présence (secondes) ≠ réputation (14h)**. Le decay du score de contribution est
  5%/h (demi-vie ~14h) — **correct pour la réputation**, pas pour la présence. Donc PoP
  a DEUX lectures du même travail : (a) *présence* = horodatage du dernier travail réel,
  fenêtre courte (~45s, seuil offline existant) ; (b) *réputation/known* = score accumulé,
  fondu lent. Le « 1 travail → 14h known » est acceptable **si** le travail exige de servir
  un vrai tiers (le cap ratio y aide ; résistance complète au self-farm = travail futur).

**Confirmé — prérequis AVANT la migration (couplage obligatoire) :**
- **`PeerStatus::Known` est indispensable.** Retirer `record_heartbeat` sur ouï-dire SANS
  ajouter `Known` **casse la sélection de relais** (`relay.rs` `select_best`/`online_relays`
  ne lisent que `Online`) → un relais joignable mais sans PoP récent serait exclu →
  connectivité en berne. La 1re brique = **deux changements couplés** : ajouter `Known` +
  router lit `Known|Online`, présence lit `Online`(PoP). Call-sites à auditer :
  `state.rs:2333/2432/2465` (record_heartbeat ouï-dire), `relay.rs:139/204`,
  `loop.rs:496/551` (`online_count`). **Le 1500 (message reçu réel) reste — c'est du vrai travail.**

**Confirmé — trou structurel dur (R14+) :**
- **Eclipse / témoin unique.** `select_path` (`relay.rs:251`) rend **un seul** relais. Un
  appareil faible dépendant d'un relais ne peut **pas détecter** que ce relais ment/censure
  sa vue de présence (la signature prouve l'origine, pas la véracité). Défense nécessaire :
  **quorum de 2-3 témoins-relais indépendants** pour une vue de présence, et/ou **ACK dyadique
  direct** (le pair signe vers le demandeur, bypass du témoin). Vrai problème de conception.

**Basse sévérité (noté, non urgent) :**
- Clock offset `presence_clock_offset_ms` non clampé → knob SIM **local** (un attaquant ne
  biaise que sa propre vue). Clamp = hygiène.
- Rejeu d'ACK après expiration du cache anti-replay (5 min) → impact faible (l'ACK est lié à
  un `message_id` à usage unique ; rejouer sur un message déjà livré = no-op idempotent).

## Prochaines étapes
- [ ] Red-team adversarial du design (les 6 challenges) → section « Défenses ».
- [ ] Spec de la preuve signée (format ACK signé, bundle relais, anti-replay).
- [ ] Première brique : `PeerStatus::Known` + `record_heartbeat` sur travail réel
      seulement + split lecteurs routage/présence. Validation cross-crate + stress réel.
