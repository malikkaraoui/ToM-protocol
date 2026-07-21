# Design — Carnet de rendez-vous TOURNANT (pas de point fixe)

> Chantier de CONCEPTION (design-first, protocolaire LOCKED/red-teamé). Rédigé nuit
> 2026-07-21. **Statut : PROPOSITION — à valider par Malik AVANT tout code.**
> Mandat : `PROMPT-carnet-rendezvous-tournant.md` + `PROCHAINE-SESSION-carnet-rendezvous.md`.
> Sources : wp §3.4 (rôles pseudo-aléatoires) / §4.3 (sélection en cascade d'entropie),
> `prisme-des-roles.md` (écarts #1 rotation, #2 carnet), ADR-010, 7 décisions LOCKED.

## 1. Le problème (verbatim Malik)
> « le carnet avec les différents nœuds doit tourner demain et ne doit jamais être
> chez la même personne ; on pointe toujours du doigt la Freebox. »
> « un contact qui a déjà ses points de rendez-vous ne doit pas se retrouver dans le
> carnet de Monsieur-Madame-tout-le-monde. »

**But : comme le DNS, MAIS ça TOURNE.** Pas de point fixe.

### État actuel (ADR-010) — pourquoi ça cloche
8 slots DHT Mainline **statiques** dérivés d'une constante mondiale
(`tom-protocol-rendezvous-v1`). Chaque nœud publie `{node_id, addrs}` dans
`slot = hash(node_id) % 8` et **lit tous les slots** → *tout le monde détient le
carnet* → **diffusion de facto**. Le nœud stable 24/7 (Freebox) ramasse tout (789
entrées, 787 inconnus) : il devient l'**aspirateur** (cf. autopsie OOM). Ce n'est
PAS le « rôle tournant à quelques détenteurs » de la vision (écart #2, aucun design).

## 2. Distinction fondatrice (à graver)
| | CARNET DE CONTACTS | INFRA DE RENDEZ-VOUS |
|---|---|---|
| Quoi | les gens à qui J'ÉCRIS | les hôtes qui mettent 2 inconnus en lien |
| Taille | borné ~30 (Malik) | quelques dizaines de détenteurs, réseau-large |
| Nature | privé, local, durable | un RÔLE réseau-imposé, éphémère, TOURNANT |
| Gardé comme contact ? | oui | **JAMAIS** (un relais/hôte croisé n'est pas un ami) |

Aujourd'hui tout est mélangé dans la topology. Le rendez-vous tournant sépare les deux :
le carnet de contacts reste petit ; l'infra tourne et n'est jamais « collectionnée ».

## 3. Modèle cible

### 3.1 Le rôle « hôte de rendez-vous »
- **Assigné par le réseau** (topologie + contribution + PoP), jamais choisi
  (ADR-006, LOCKED #6). Un nœud ne décide pas d'être hôte.
- **Tourne** par sélection pseudo-aléatoire (wp §3.4) : à chaque **période**, un aléa
  vérifiable désigne les détenteurs. Personne ne se positionne à l'avance (§4.3).
- **Quelques dizaines** de détenteurs actifs (pas mondial), charge répartie.
- **Détention bornée** : un hôte ne garde que les publications de la période courante
  (TTL = période, LOCKED #2), pas un carnet mondial cumulatif → jamais d'aspirateur.
- **Fade, pas de ban** (LOCKED #4) : sortir du rôle = décroissance, pas exclusion.

### 3.2 Publication CIBLÉE (résout le chantier 2)
Un nœud X publie son `{node_id, addrs}` **signé** (preuve de possession, KL#2) chez
**ses K hôtes ACTUELS** — les hôtes que la fonction de sélection désigne pour X à la
période courante. Il ne publie PAS dans un slot mondial lu par tous. → un nœud « déjà
placé » ne pollue plus les carnets des autres.

### 3.3 Découverte par RECHERCHE (résout le chantier 1+2)
Qui cherche X **recalcule** les K hôtes que X aurait choisis (fonction déterministe :
`select(node_id_X, période, ensemble_Online)`) et **interroge ces hôtes seulement**.
Seul quelqu'un qui cherche X le trouve. Le mapping est déterministe *pour une période
donnée* (donc recalculable) mais **tourne** à chaque période (donc imprévisible à
l'avance). C'est un rendez-vous rotatif : K hôtes tournants par node_id au lieu de
8 slots fixes mondiaux.

## 4. Trois approches (à trancher — panel)
| | A. DHT-avec-rotation | B. Hôtes désignés (rendezvous hashing) | C. Hybride |
|---|---|---|---|
| Idée | garder Mainline, faire **tourner les clés de slot** par période (dérivées beacon+période) | pas de slot mondial : K hôtes par node_id via *rendezvous/consistent hashing* sur l'ensemble Online pondéré PoP, tournant par période | DHT pour amorçage grossier (trouver quelques hôtes vivants) + hôtes désignés pour publication/recherche ciblée |
| Tournant | ✅ (clé change) | ✅ (sélection change) | ✅ |
| Ciblé | ⚠️ un slot reste lu par plusieurs | ✅ K hôtes précis | ✅ |
| Dépend de | beacon d'entropie | vue cohérente de l'ensemble Online | les deux |
| Risque | slot encore semi-diffusif | divergence de vue → chercheur rate | complexité |
| Reco | — | **privilégiée** (la plus proche de la vision) avec amorçage DHT (C) pour trouver l'ensemble | — |

## 4bis. Algorithme de l'approche B (détail technique)
**Sélection des K hôtes (rendezvous hashing / HRW)** : pour un `node_id` X et une
période P, chaque candidat H (Online, éligible) reçoit un poids
`w(H) = hash(H_id ‖ X_id ‖ P) · f(contribution_H)`. Les **K plus hauts poids** sont
les hôtes de X pour P.
- **Déterministe** : tout chercheur qui connaît {ensemble candidat, P} recalcule les
  mêmes K hôtes → il sait QUI interroger pour trouver X.
- **Tournant** : P change → poids changent → hôtes changent. Pas de point fixe.
- **Réparti** : HRW distribue uniformément ; pondéré par contribution (LOCKED #6),
  borné par le fade (pas toujours les mêmes → anti-aspirateur).
- **Stable au churn** : ajouter/retirer un candidat ne déplace que ~1/N des mappings
  (propriété HRW) → robuste aux vues légèrement divergentes.

**Ensemble candidat** : nœuds Online (PoP constaté) éligibles au rôle. Découvert par
amorçage DHT grossier (C) — le DHT ne publie plus les adresses de tous, il sert juste
à savoir QUI peut héberger.

**Période P** : d'abord temporelle `P = floor(now / T)` (T ex. 1 h). À durcir avec un
**beacon d'entropie vérifiable** (imprévisibilité forte, §4.3) — sans lui, P est
précalculable donc grindable (cf. red-team). Rotation simple d'abord, beacon ensuite (§8).

**Publication (X)** : à chaque période, X calcule ses K hôtes pour P **et P+1**
(chevauchement), leur envoie `{node_id, addrs}` **signé** (preuve de possession),
TTL = période + grâce.

**Recherche** : qui cherche X calcule ses K hôtes pour P (et P−1/P+1 en grâce), les
interroge, retient la 1ʳᵉ réponse **signée valide** (quorum si divergence).

**Transition (grâce)** : republier chez les hôtes de P+1 AVANT expiration des
publications de P → 0 perte de lien pendant la rotation.

## 5. Red-team (modes de défaillance)
- **Fragmentation / vues divergentes** : si l'`ensemble_Online` diffère entre nœuds,
  publieur et chercheur calculent des hôtes différents → X introuvable. → mapping
  robuste (rendezvous hashing tolère l'ajout/retrait), K>1, **période de grâce**
  (chevauchement), amorçage DHT commun pour l'ensemble.
- **Rotation qui perd des liens** (transition de période) : anciennes publications
  expirent avant republication chez les nouveaux hôtes. → **republier chez les
  nouveaux AVANT expiration des anciens**, TTL = période + grâce.
- **Squatting / hôte malveillant** (intercepte, ment, ou se fait choisir pour une
  cible) : → preuve de possession signée (KL#2, déjà en place), **K hôtes redondants**
  (quorum de réponses), sélection **non-biaisable** (mur #1 Fable : grinding).
- **Grinding de l'entropie** (attaquant influence l'aléa pour héberger une cible
  précise) : → entropie **vérifiable et non-manipulable par un participant seul**
  (cascade §4.3) — c'est le problème de recherche OUVERT (écart #1, M1.2/M1.3).
- **Partition** : vues Online scindées → mappings incohérents. → dégradation
  gracieuse (recherche multi-période + multi-hôte), pas d'arbitrage L1 (LOCKED #3).
- **Le stable redevient-il aspirateur ?** (le risque à NE PAS reproduire) : si un
  nœud est toujours sélectionné (car stable/contributif), il redevient point fixe.
  → la rotation **borne le temps de détention** (fade) + **répartit** (pas toujours
  les mêmes) + l'hôte ne garde QUE la période (borné+TTL). L'anti-aspirateur est un
  **critère de conception dur**, pas un effet de bord.
- Croisé aux LOCKED : #3 (la sélection n'ARBITRE pas — elle est mécanique/vérifiable),
  #5 (un hôte surchargé est sur-assigné/fade, jamais exclu), #6 (assigné pas choisi),
  #7 (primitive universelle, pas un produit).

## 6. Migration depuis ADR-010 (incrémentale, non-cassante)
1. **Coexistence** : les 8 slots statiques restent (compat) ; le mécanisme tournant
   s'ajoute en parallèle, feature-flagged.
2. **Carnet de contacts borné** (~30) : séparé mais complémentaire — voir le chantier
   fuite/OOM. Sépare CONTACTS (bornés) de l'INFRA.
3. **Publication ciblée** chez les K hôtes tournants (au lieu du slot mondial).
4. **Recherche par recalcul** des K hôtes de la cible.
5. **Retrait des slots mondiaux** une fois le tournant validé (deux inconnus se
   trouvent de façon fiable via le nouveau chemin).

## 7. Scénarios de test (le banc R4 devient la non-régression)
- **R4-tournant** : deux inconnus (zéro connaissance) se trouvent via hôtes tournants
  + recherche, PAS via les slots statiques. Namespace de test dédié (règle d'or 20/07).
- **Rotation prouvée** : sur N périodes, la distribution des détenteurs tourne — AUCUN
  point fixe (mesurer qu'aucun node_id n'héberge > seuil sur la fenêtre).
- **Non-pollution** : un nœud « placé » n'apparaît dans le carnet QUE des nœuds qui le
  cherchent (0 fantôme chez les autres).
- **Résilience transition** : au changement de période, le lien tient (grâce +
  chevauchement) — 0 perte de découverte pendant la rotation.
- **Anti-squat** : un hôte forgé/malveillant est rejeté (preuve de possession + quorum K).

## 8. Prérequis & honnêteté (§5)
- La **rotation non-biaisable** (entropie vérifiable en cascade) est un **problème de
  recherche OUVERT** (murs #1/#2 Fable, écart #1). Ce design POSE le cadre mais la
  primitive d'entropie non-biaisable est un prérequis (M1.2/M1.3).
- **Chemin pragmatique proposé** : livrer d'abord une rotation SIMPLE (période
  temporelle + hash déterministe de l'ensemble Online) qui donne DÉJÀ le tournant +
  le ciblé + l'anti-aspirateur ; **durcir** l'imprévisibilité avec la cascade
  d'entropie quand la primitive PoP/cascade sera disponible. On ne bloque pas le
  tournant sur le problème de recherche non résolu.
- **Complémentarité avec le bug OOM** : le fix mémoire (chantier séparé) éteint le
  symptôme ; ce design supprime la **cause-conception** (le point fixe diffusif). Les
  DEUX sont nécessaires.

## 9. Décisions demandées à Malik (avant code)
1. Approche B (hôtes désignés) + amorçage DHT (C) — OK ?
2. Rotation SIMPLE d'abord (période + hash), cascade d'entropie ensuite — OK ?
3. Valeurs de départ : K (hôtes/node_id, ex. 3), durée de période (ex. 1 h), grâce.
4. Feature-flag de coexistence avec l'ADR-010 actuel — OK ?
