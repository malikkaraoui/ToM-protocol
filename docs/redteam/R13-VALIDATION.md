# R13 — Validation réelle de la livraison de groupe offline (gap-fill)

**Date :** 2026-07-09 · **Méthode :** test E2E via API HTTP de contrôle sur vrais
nœuds (triangle isolé hub + 2 membres, localhost, `--isolated`).

## Contexte

R13 (offline-delivery groupe) était **codé + testé unitairement** (seq monotone,
persistance SQLite `hub_message_history`, `last_seqs`, `SyncRequest/SyncResponse`).
Objectif : le **valider en conditions réelles** — les tests unitaires ne
couvrent pas le câblage runtime ni le vrai chemin réseau.

Pour ça, on a d'abord dû **développer l'API de test** (Phase A) :
- `tom-node` : inbox borné + endpoints `/inbox`, `/invites`, `/group/accept` ;
- flags `--data-dir` (persistance SQLite : R13 était **inactif** sur le binaire,
  `data_dir` jamais configuré) et `--isolated` (couper n0+mDNS → pas de
  pollution du vrai parc).

Script de régression : `scripts/test-r13-offline.sh` (exit 0 = OK).

## Ce que le test réel a révélé (invisible en test unitaire)

### Bug 1 — `/stop` ne persistait pas → appartenance perdue au restart
La sauvegarde d'état est périodique (30s) + au shutdown gracieux (`loop.rs:710`).
Mais `/stop` fait `process::exit(0)`, qui **court-circuite** ce flush. Un membre
stoppé < 30s après avoir rejoint un groupe perdait son appartenance → au restart,
`groups:0`, aucun rejoin possible.
**Fix :** `RuntimeCommand::SaveState { reply }` + `RuntimeHandle::save_now()`,
appelé avant l'exit dans `/stop`. Généralisable : **stopper/backgrounder une app
doit persister groupe + last_seq**.

### Bug 2 — rejoin cold-start émis AVANT la connectivité → SyncRequest dans le vide
`build_rejoin_effects()` (Join + SyncRequest par groupe restauré) était appelé
**une seule fois, pré-boucle** (`loop.rs:191`), avant toute connexion au hub.
Les requêtes partaient dans le vide et n'étaient jamais retentées → **aucun
rattrapage sur restart à froid**. (Le rejoin ne se déclenchait que sur
isolation-recovery, pas au démarrage.)
**Fix :** rejoin **différé** — flag `rejoin_pending` armé si `group_count() > 0`,
(re)déclenché dans `reconnect_check` quand `topology.online_count() > 0`.
Note : `connected_peers()` (QUIC direct) est **vide** en maillage relais-only —
inutilisable comme signal de connectivité ; `topology.online_count()` est fiable.

## Résultat

```
SANITY ok (3 messages online)
R13 OK — m2 a rattrapé 5 messages offline via gap-fill (seq 3..7, ordonnés)
```

m2 offline → m1 émet 5 messages → m2 restart (même identité + data-dir) →
restore du groupe → rejoin différé → SyncRequest → le hub sert les 5 manqués
depuis SQLite → livrés via `GroupMessageReceived`, dans l'ordre.

## Leçon méthodo

Deux bugs de **câblage runtime** que 500+ tests unitaires ne voyaient pas, tous
deux exposés en < 1h par un test E2E sur vrais nœuds via API. Confirme la règle :
**toute fonction se valide sur de vrais nœuds via API, pas seulement en unitaire.**

## Reste à faire

- **Phase C** : exposer l'API groupe côté FFI/Swift pour tester R13 sur les vrais
  devices Apple (iPad/AppleTV/iPhone), pas seulement en CLI.
- Rejoin sur isolation-recovery runtime (pas seulement cold-start) : ré-armer
  `rejoin_pending` sur transition d'isolement — reporté (dépend du signal de
  connectivité en mode relais-only).
