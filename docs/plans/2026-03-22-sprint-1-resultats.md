# Sprint 1 — Résultats "Wow Moment Honnête"

**Date** : 22 mars 2026
**Durée effective** : ~4h (session unique)

---

## Résumé exécutif

Le sprint a livré un scénario 3 nœuds reproductible où un relay embarqué est publié, découvert via gossip, injecté dans le transport, et utilisé pour échanger des messages — le tout sans intervention humaine après lancement.

---

## Métriques mesurées

### M1 — Temps de setup

| Mesure | Résultat | Cible |
|--------|----------|-------|
| Temps launch → premier message | ~25s | < 5 min |
| **Verdict** | **PASS** | |

Avec binaires pré-compilés. Le temps est dominé par la convergence gossip (~10-15s) + discovery relay (~5s).

### M2 — Nombre de commandes

| Mesure | Résultat | Cible |
|--------|----------|-------|
| Commandes nécessaires | 4 (T0 relay + T1 publisher + T2 obs1 + T3 obs2) | ≤ 3 |
| **Verdict** | **FAIL** | |

T0 (relay bootstrap externe) est nécessaire car `n0_discovery` seul ne garantit pas la convergence initiale. La réduction à 3 commandes nécessite un ADR dédié (relay bootstrap embarqué dans le premier nœud, ou rendezvous alternatif).

### M3 — Taux de succès discovery sur 20 runs

| Mesure | Résultat | Cible |
|--------|----------|-------|
| Runs réussis (4/4 critères) | 18/20 = 90.0% | ≥ 90% |
| **Verdict** | **PASS** | |

**Critères par run** (binaire) :
1. `published` — relay embedded démarré
2. `discovered` — relay découvert via gossip
3. `transport_injected` — relay ajouté au transport
4. `message_passed` — message échangé via bot-ping

**Détail des 2 échecs** :
- **Run 2** : pub=1 disc=1 trans=1 msg=0 — timing race probable (message non reçu dans les 40s)
- **Run 4** : pub=1 disc=0 trans=0 msg=0 — **faux négatif du bug `grep -c`** (script corrigé depuis)

**Note** : le script S7 avait un bug `grep -c ... || echo 0` qui produisait des entiers invalides quand grep retournait exit 1. Corrigé en `|| true`. Le taux réel est probablement 19/20 (95%).

### M4 — Stabilité sur 10 minutes

| Critère | Résultat |
|---------|----------|
| M4.1 — Pas de duplicate insert per-node | **PASS** (1 insert/observer) |
| M4.2 — Pas d'expiration abusive (publisher healthy) | **PASS** (0 expiration) |
| M4.3 — Pas de crash/panic | **PASS** (0 panic) |
| M4.4 — Processus vivants à la fin | **PASS** (4/4 sur 20 checkpoints) |
| **Verdict global** | **PASS — 100%** |

**Stats détaillées (10 min)** :
- ~9900 messages échangés (publisher ↔ obs1)
- 44 relay discovered events (republications ~25s)
- 0 relay expired, 0 transport relay removed
- obs2 silencieux (pas de `--bot-ping`, pas ciblé par publisher — comportement attendu)

### M0 — Test du couloir

**En attente** — nécessite un observateur externe humain.

---

## Backlog — État final

| # | Item | Rail | Statut |
|---|------|------|--------|
| S1 | GATE — Smoke test 3 process | R2 | **PASS** |
| S2 | `--embedded-relay-publish` flags | R3 | **PASS** (pré-existant) |
| S3 | Rendu events discovery dans TUI | R1+R3 | **PASS** (7 events) |
| S4 | Runbook 1 page | R1 | **PASS** |
| S5 | Mesure M1/M2 | R1 | **PASS** (M1 OK, M2 NOK) |
| CP | Checkpoint métrique | — | **PASS** |
| S6 | Corriger frictions | R2+R3 | **PASS** (friction #1 corrigée) |
| S7 | 20 runs — 4 critères + timings | R1+R2 | **PASS** (90%) |
| S8 | Stabilité 10 min | R2 | **PASS** (100%) |
| S9 | Test du couloir | R1 | **En attente** |
| S10 | Publication résultats + rétro | — | **Ce document** |

---

## Commits livrés

| Hash | Description |
|------|-------------|
| `173b1a7` | feat(tui): add relay discovery event rendering (S3) |
| `be28a3f` | feat(tui): add --bot-ping flag |
| `32165b1` | fix(runtime): auto-start embedded relay (Bug A) |
| `a21c95f` | docs: add S1 smoke test runbook (S4) |
| `30aa77a` | test(runtime): regression test for auto-start |
| `6b1d98f` | feat(runtime): re-publish relay on GossipNeighborUp (S6) |

---

## Bugs trouvés et corrigés

### Bug A — Embedded relay jamais démarré automatiquement

**Symptôme** : `EmbeddedRelayService::new()` créé dans `loop.rs` mais jamais `start()` appelé.
**Cause** : le démarrage ne se faisait que via `RuntimeCommand::StartEmbeddedRelay` explicite, jamais utilisé.
**Fix** : ajout d'un bloc auto-start dans `loop.rs` quand `config.enable_embedded_relay == true`.
**Régression** : test `embedded_relay_auto_starts_when_enabled` ajouté.

### Friction #1 — Première publication ratée par les late joiners

**Symptôme** : le `RelayReadyAnnounce` initial arrivait avant que les observers rejoignent le gossip.
**Cause** : publication unique au démarrage, pas de re-publication sur nouveaux voisins.
**Fix** : re-publication sur chaque `GossipNeighborUp` quand le relay local est healthy.
**Tests** : 2 unit tests dans `state.rs`.

### Bug script S7 — `grep -c || echo 0`

**Symptôme** : `integer expression expected` sur certains runs.
**Cause** : `grep -c` retourne exit 1 quand count=0, `|| echo 0` ajoute un "0" supplémentaire → variable multi-ligne.
**Fix** : remplacé par `|| true`.

---

## Limites connues et honnêtes

1. **M2 = 4 commandes** (cible 3) — le relay bootstrap T0 reste nécessaire. Pas de contournement simple sans ADR.

2. **obs2 silencieux en S8** — le publisher ne ping qu'un seul peer (le premier découvert). obs2 découvre le relay et les peers mais n'échange aucun message. Ce n'est pas un bug, c'est une limitation du mode `--bot-ping` qui ne cible qu'un peer.

3. **Convergence n0 non garantie** — la discovery initiale dépend du relay bootstrap T0. Sans T0, les nœuds n'ont aucun point de rendez-vous garanti pour former le gossip.

4. **44 relay discovered events en 10 min** — chaque republication (interval 25s) génère un event par observer. C'est du bruit acceptable mais pas optimal. Un mécanisme de déduplication côté affichage pourrait réduire le bruit.

5. **Le taux M3 de 90% inclut probablement un faux négatif** — le run 4 a échoué à cause du bug `grep -c`, pas d'un vrai échec réseau. Le taux corrigé est probablement 95%.

---

## Rétrospective

### Ce qui a bien marché

- **Gate S1 efficace** — a forcé la découverte immédiate du Bug A (relay jamais démarré), qui bloquait tout.
- **Bot-ping** — ferme la boucle de preuve message sans intervention humaine.
- **Republication on NeighborUp** — élimine la race condition du warmup de manière élégante.
- **Scripts automatisés S7/S8** — mesures reproductibles, pas de narration subjective.

### Ce qui a moins bien marché

- **Bug script grep -c** — un bug d'outillage de mesure a pollué les résultats S7. Leçon : valider le script de test avec `bash -n` + dry run avant la campagne.
- **M2 non atteint** — 4 commandes au lieu de 3. Le relay bootstrap est un problème architectural, pas un problème de sprint.

### Décisions à prendre

- **ADR M2** : comment réduire à 3 commandes ? Options : relay bootstrap embarqué dans le premier nœud, n0_discovery comme fallback, nouveau mécanisme de rendezvous.
- **Rerun S7** : un run propre avec le script corrigé confirmerait le taux réel (probablement ≥ 95%).
- **Bot-ping multi-peer** : le publisher devrait pouvoir pinguer tous les peers découverts, pas seulement le premier.
