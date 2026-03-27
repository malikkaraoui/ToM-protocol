# Cross-machine proof lock — consignes d'exécution

**Date** : 27 mars 2026  
**Statut** : actif  
**Priorité** : haute  
**But** : verrouiller la preuve fonctionnelle cross-machine avant toute nouvelle évolution de logique.

---

## Contexte

Le scénario cross-machine a franchi un cap réel :

- le relay embarqué publie désormais une **IP LAN utile** au lieu de `127.0.0.1`
- le bug `--bot-ping` a été identifié : la cible choisie était le **premier peer découvert**, souvent un peer `n0` anonyme
- après correction, `--bot-ping` cible un **peer nommé** (peer ToM réel)
- les messages passent désormais **cross-machine**
- la connexion peut upgrader en **direct QUIC** (`Path changed: Direct`, RTT observé ~45 ms)

Conclusion :
le blocage principal n'était pas un problème fondamental d'architecture, mais un mélange de :
1. publication d'une mauvaise adresse relay
2. sélection d'une mauvaise cible côté bot

---

## Cap à tenir

**Ne plus toucher à la logique produit tant que la preuve n'est pas verrouillée.**

Priorité immédiate :
1. figer la preuve
2. ajouter les régressions minimales
3. rejouer une campagne cross-machine propre
4. documenter les résultats

**Ne pas rouvrir M2 / ADR / refonte design tant que cette preuve n'est pas stabilisée.**

---

## Ce qu'il faut faire maintenant

### 1) Verrouiller la preuve technique

Produire une preuve explicite sur 3 points :

#### A. Régression bot-ping
Montrer que `--bot-ping` ne cible plus un peer `n0` anonyme, mais un peer ToM nommé.

**Attendu** :
- trace ou test montrant qu'un peer sans username n'est pas choisi en priorité
- trace ou test montrant qu'un peer avec username est choisi

#### B. Preuve du peer ciblé
Capturer noir sur blanc :
- `Ping target set: ... "nas-publisher"`
- ou `Ping target set: ... "mac-obs1"`

**Attendu** :
- la cible choisie doit être un peer nommé
- la preuve doit être archivable dans un document ou log de campagne

#### C. Preuve de l'upgrade direct
Capturer noir sur blanc :
- `Path changed: Direct`
- RTT observé

**Attendu** :
- au moins un run de référence où le passage en direct est visible
- la preuve doit montrer que le relay bootstrap n'est pas le chemin final des messages

---

### 2) Rejouer le scénario cross-machine proprement

Faire une campagne courte et documentée.

#### Minimum requis
- **1 run de référence** complet et propre

#### Souhaitable
- **5 à 10 runs** si le temps et les machines le permettent

#### Pour chaque run, noter
- peer ciblé par chaque bot
- nombre de messages échangés
- présence ou absence de `Path changed: Direct`
- RTT si direct
- verdict binaire : `PASS` ou `FAIL`

#### Format conseillé
Tableau synthétique par run :

| Run | Peer ciblé | Messages OK | Path Direct | RTT | Verdict |
|-----|------------|-------------|-------------|-----|---------|
| 1 | nas-publisher | oui | oui | 45 ms | PASS |

---

### 3) Documenter la preuve avant / après

Il faut produire un document simple, factuel, partageable.

## Narrative minimale obligatoire

### Avant fix
- peer `n0` anonyme ciblé
- discovery OK
- 0 message utile entre les 3 nœuds ToM

### Après fix
- peer nommé ciblé
- messages réels échangés entre les nœuds ToM
- `Path changed: Direct`
- hole punch / direct QUIC prouvé

---

## Non-objectifs explicites

Tant que cette phase n'est pas verrouillée, **ne pas** :

- refactorer la logique runtime sans nécessité
- ouvrir un chantier ADR M2
- changer plusieurs comportements à la fois
- mélanger validation cross-machine et redesign produit
- “améliorer” le système sans preuve mesurée

---

## Livrables attendus

### Livrable 1 — Régression / preuve bot-ping
Au choix :
- test automatisé
- ou preuve textuelle reproductible si le test unitaire n'est pas faisable rapidement

### Livrable 2 — Campagne cross-machine
- 1 run de référence minimum
- idéalement 5 à 10 runs
- tableau de résultats

### Livrable 3 — Document de preuve
Un `.md` avec :
- contexte
- avant / après
- extraits de logs clés
- tableau des runs
- conclusion claire

---

## Critères d'acceptation

La phase est considérée comme verrouillée si les 4 points suivants sont vrais :

1. `--bot-ping` cible un **peer nommé** et non un peer `n0` anonyme
2. au moins un run cross-machine montre des **messages réellement échangés**
3. au moins un run montre `Path changed: Direct`
4. tout cela est **documenté** dans un artefact partageable

---

## Ordre d'exécution imposé

1. ajouter preuve / régression bot-ping
2. lancer un run de référence cross-machine
3. capturer peer ciblé + messages + path direct + RTT
4. lancer campagne courte additionnelle si possible
5. écrire le document de preuve
6. seulement ensuite discuter suite produit / ADR

---

## Décision de pilotage

**Cap validé : verrouiller la preuve cross-machine maintenant, ne pas rouvrir M2 tant que ce n'est pas figé.**

En cas d'hésitation entre “ajouter une feature” et “mieux prouver l'existant”, choisir systématiquement **la preuve**.
