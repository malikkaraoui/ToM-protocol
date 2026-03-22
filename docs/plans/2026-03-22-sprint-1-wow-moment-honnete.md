# Sprint 1 — Wow Moment Honnête (22 mars 2026)

## Raison d'être

Ce sprint existe pour produire un **wow moment honnête, démontrable, mesurable et reproductible** à partir des briques réellement livrées dans ToM.

Le but n'est pas de faire une belle démo cosmétique.
Le but n'est pas non plus d'ajouter une nouvelle profondeur théorique.

Le but est de montrer, en conditions simples mais réelles, qu'un comportement réseau ToM :

- vit réellement,
- se comprend vite,
- se relance sans rituel,
- et tient assez bien pour mériter d'être montré.

---

## Décisions fermes de réunion

### D1 — Fin de la séquence linéaire

La séquence linéaire `A -> B -> C -> D -> E` est abandonnée.

Elle est remplacée par **3 rails parallèles couplés** :

- **Rail 1 — Wow moment démontrable**
- **Rail 2 — Boucle réseau vivante**
- **Rail 3 — Opérabilité**

### D2 — Livrable prioritaire central

Le **wow moment démontrable** devient le livrable prioritaire central du sprint.

Ce n'est pas un nice-to-have.
C'est le pivot qui permet de sortir des discussions purement internes.

### D3 — Mesure obligatoire

Le sprint est piloté par **5 métriques** :

- **M0** — test du couloir (compréhension extérieure en 30 secondes)
- **M1** — temps de setup
- **M2** — nombre de commandes nécessaires
- **M3** — taux de succès discovery sur 20 runs
- **M4** — stabilité sur 10 minutes

### D4 — Gate technique explicite

Le backlog de sprint contient un **gate technique** :

> **S1 doit passer avant toute tentative de polish ou de narration.**

On ne décore pas un moteur qui ne tourne pas.

---

## Définition de Done

Le sprint est considéré terminé quand :

1. un membre de l'équipe lance le scénario sans assistance ;
2. les métriques **M1 à M4** sont relevées et consignées ;
3. le **test du couloir M0** a été réalisé au moins une fois ;
4. la démo tourne en live sur le scénario de référence ;
5. les résultats sont publiés avec une courte rétrospective.

---

## Les 3 rails du sprint

### Rail 1 — Wow moment démontrable

Objectif : produire un scénario que quelqu'un comprend vite.

Cible :

- 3 nœuds
- un relay apparaît
- il est publié
- il est découvert
- il est utilisé
- cela se voit sans exiger une explication de 20 minutes

### Rail 2 — Boucle réseau vivante

Objectif : vérifier que le cœur discovery/publish/inject/maintain fonctionne réellement.

Cible :

- publication relay-ready
- découverte relay-ready
- injection transport
- message qui passe
- maintien ou expiration correcte selon l'état du publisher

### Rail 3 — Opérabilité

Objectif : réduire la friction pour exécuter le scénario.

Cible :

- flags TUI utiles
- sortie discovery lisible
- runbook court
- relance simple

---

## Backlog final corrigé

| # | Item | Rail | Note |
|---|------|------|------|
| S1 | **GATE** — Smoke test 3 process TUI séparés, relay auto-découvert | R2 | Précondition sprint |
| S2 | `--publish-relay` + wiring manquant (flags existants OK) | R3 | Resserré |
| S3 | Rendu events discovery dans TUI | R1+R3 | Après S2 |
| S4 | Scénario démo 3 nœuds — runbook 1 page | R1 | Après S1 |
| S5 | Mesure M1/M2 sur scénario de référence | R1 | Après S4 |
| CP | **Checkpoint métrique intermédiaire** | — | Après S5 |
| S6 | Corriger les 2 plus grosses frictions | R2+R3 | Après CP |
| S7 | 20 runs — succès binaire (4 critères) + timings | R1+R2 | Après S6 |
| S8 | Stabilité 10 min — duplicate/expiry | R2 | Après S6 |
| S9 | Test du couloir — observateur extérieur | R1 | Après S7+S8 |
| S10 | Publication résultats + rétro | — | Après S9 |

---

## Chemin critique

Chemin critique principal :

`S1 -> S4 -> S5 -> CP -> S6 -> S7/S8 -> S9 -> S10`

Travail parallèle autorisé :

- `S2` peut démarrer en parallèle de `S1`
- `S3` suit `S2`
- mais aucun polish de surface n'a priorité sur l'échec du gate `S1`

---

## Détail des métriques

### M0 — Test du couloir

Question :

> Une personne extérieure au chantier comprend-elle en 30 secondes ce qui se passe ?

Réussite si :

- elle identifie qu'un relay est apparu / a été découvert / a servi ;
- elle comprend le rôle du scénario sans qu'on lui fasse un cours sur l'architecture ;
- elle n'est pas noyée par des logs illisibles.

Ce n'est pas une métrique chiffrée stricte, mais c'est un juge de paix du wow moment.

### M1 — Temps de setup

Mesure :

Temps entre le démarrage du scénario et l'obtention d'un scénario vivant complet.

Cibles :

- **< 5 min** pour un membre de l'équipe
- **< 10 min** pour quelqu'un qui n'a pas suivi tous les détails récents

### M2 — Nombre de commandes

Mesure :

Nombre de commandes nécessaires pour lancer le scénario de référence.

Cible :

- **3 commandes max**
- idéalement 1 commande par nœud, sans étape intermédiaire manuelle opaque

### M3 — Taux de succès discovery sur 20 runs

Chaque run est évalué selon 4 critères binaires :

1. relay publié ✅ / ❌
2. relay découvert ✅ / ❌
3. transport enrichi ✅ / ❌
4. message passé ✅ / ❌

Un run est considéré comme **succès** si les 4 critères sont vrais.

Cible :

- **>= 90%** de succès local sur 20 runs

### M4 — Stabilité sur 10 minutes

Mesure :

Sur le scénario de référence maintenu 10 minutes :

- pas de duplicate insert abusif ;
- pas d'expiration abusive si publisher healthy ;
- expiration correcte après arrêt réel du publisher ;
- pas de crash / panic.

Cible :

- **100%** sur le scénario local de référence

---

## Critères binaires et timings pour S7

Pour éviter toute interprétation floue, chaque run S7 doit consigner :

### Critères binaires

- `published`: oui/non
- `discovered`: oui/non
- `transport_injected`: oui/non
- `message_passed`: oui/non

### Timings minimaux à relever

- temps `publication -> découverte`
- temps `découverte -> injection transport`
- temps `envoi -> réception`

Format attendu : tableau simple, pas narration libre.

---

## Checkpoint intermédiaire (CP)

Le checkpoint intermédiaire a lieu **après S5**.

Il sert à éviter de découvrir trop tard que le sprint repose sur une base trop fragile.

À ce point, on doit déjà avoir :

- le résultat de `S1`
- le runbook de référence `S4`
- une première mesure de `M1` et `M2`
- les premiers points de friction observés

Question de checkpoint :

> Est-ce qu'on a un scénario suffisamment réel et suffisamment relançable pour justifier la suite du sprint telle quelle ?

Si la réponse est non, on corrige avant d'aller plus loin.

---

## Responsabilités pressenties

Ces responsabilités sont indicatives ; elles peuvent être ajustées par l'équipe.

- **Winston** — vigilance sur `S1`, discipline du gate technique
- **John** — exigence sur le wow moment et `M0`
- **Mary** — structure et suivi des métriques
- **Amelia** — `S2` / `S3` côté TUI et flags
- **Quinn** — `S7` et tableau de réussite/échec
- **Paige** — `S4` runbook 1 page
- **Sally** — observation du test du couloir `S9`
- **Bob** — orchestration du backlog et du déroulé sprint

---

## Ce qui est explicitement hors sprint

Pour éviter la dérive, les sujets suivants ne sont **pas** des objectifs de ce sprint :

- refonte large de la TUI ;
- grand chantier de relay rotatif complet ;
- nouvelle couche de découverte théorique ;
- polish UI non relié au scénario de référence ;
- extension du périmètre produit sans impact sur le wow moment ou la preuve.

---

## Résultat attendu du sprint

À la fin de ce sprint, on doit pouvoir dire quelque chose de simple et défendable :

> Voici un scénario 3 nœuds reproductible, sans bricolage lourd, où un relay est publié, découvert, injecté dans le transport, utilisé par le réseau, et observé avec des mesures honnêtes.

Si on obtient cela, le sprint est réussi.
Si on n'obtient qu'un joli récit sans scénario relançable, il est raté.
