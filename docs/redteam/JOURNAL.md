# ToM Red Team — Journal (narratif)

> Notes scrupuleuses de la boucle d'attaque autonome. Voir `PROTOCOLE-RED-TEAM.md`
> pour la doctrine, `journal.jsonl` pour les relevés machine, `corpus.jsonl` pour
> le corpus de régression. Rien n'est jamais effacé.

## Format d'une entrée de perçage
```
### [BUILD n · date] BREACH — <attack.id> (seed <s>)
- Symptôme : <ce que le réseau a fait de mal>
- Cause racine : <file:line>
- Patch : <ce qui a été durci> (commit <sha>)
- Re-test : DÉFENDU ✓ · Régression corpus : verte ✓
```

## Historique

### [BUILD 21 · 2026-07-06] FINDING #1 — CONFIRMÉ : un nœud devient INSENSIBLE aux requêtes après churn+skew
- **Attaque** : `chaos.monkey` seed 7 (5 kills, 5 revives, 6 skews, min alive=2).
- **Verdict** : **PERÇÉ (reproductible)**. Harnais propre (0 zombie), localisé.
- **Symptôme précis** (localisé par instrumentation) :
  - Le fire-and-forget `check_presence_all_online()` (send-only) **passe** (< 3s) sur les 4 nœuds.
  - MAIS `presence_metrics()` (requête→**réponse** via oneshot) **timeout à 3s sur le nœud 3**.
  - → La boucle runtime du nœud 3 **n'émet plus de réponse** : elle traite (ou file)
    les sends mais ne répond plus aux requêtes. Le nœud est un **trou noir** : vivant
    en apparence (handle up), mais inexploitable.
  - Sans garde-timeout, le scénario **hang** indéfiniment sur ce nœud (d'où les EXIT=124).
- **Portée** : c'est une **vraie fragilité protocole**, pas un artefact de test —
  un vrai device qui perd tous ses pairs (min alive=2) + subit du chaos d'horloge
  peut plausiblement toucher le même chemin. Un nœud ne doit JAMAIS se figer.
- **Hypothèse de cause racine (à confirmer au fix)** : **isolation recovery**
  (`loop.rs::reconnect_check` 15s → `bootstrap.rs::on_isolated`, ADR-010). Quand le
  nœud 3 se retrouve isolé (ses pairs tués), la reprise d'isolement lance
  reprobe relais + DHT republish + rendez-vous. Avec `relay_urls(vec![])` +
  `n0_discovery(false)`, un de ces appels **bloque/spin la boucle select!**,
  empêchant le drain des commandes et l'émission des réponses.
  - Hypothèses alternatives : (b) accumulation de tasks d'envoi qui timeout vers
    pairs morts (starvation) ; (c) offset d'horloge extrême non nettoyé.
- **Reproduction** : `cargo run -p tom-stress --bin tom-stress -- chaos-monkey --seed 7`
  → step « presence resumes » FAIL + `[collect] node 3: presence_metrics() TIMEOUT`.
- **Atténuation immédiate (outil)** : gardes-timeout 3s sur toutes les requêtes handle
  du scénario → plus de hang, le perçage est rapporté proprement (FAIL, pas freeze).
- **CAUSE RACINE LOCALISÉE** (`crates/tom-protocol/src/runtime/loop.rs`, branche
  `reconnect_check` ~L538-600) : le bloc d'isolation-recovery **await plusieurs
  opérations réseau DIRECTEMENT dans le `tokio::select!`** :
  - `node.connected_peers().await`
  - `state.publish_to_dht(...).await`
  - `node.reprobe_relays().await`  ← suspect n°1 (probing lent sans relais)
  - `sender.join_peers(known_eids).await`  ← suspect n°2 (gossip vers pairs morts)
  Pendant CHACUN de ces `.await`, le `select!` **ne peut rien traiter d'autre** :
  ni drainer `cmd_rx`, ni répondre aux oneshot des requêtes. Le nœud isolé (churn)
  se fige donc le temps de ces opérations. Modèle déjà correct à côté : les
  lookups DHT et `spawn_rendezvous_round` sont **spawnés** (non awaités) — la
  recovery, elle, ne l'est pas.
- **FIX protocole (prochaine itération, cœur du runtime → session dédiée)** :
  1. Confirmer par instrumentation laquelle de ces `.await` bloque (marqueurs autour
     de chaque appel dans reconnect_check).
  2. **Spawner** les opérations de recovery (comme les sends/lookups) OU les borner
     par `tokio::time::timeout`, pour que le `select!` **draine toujours** commandes
     et requêtes, même en isolation. Contrainte : `node`/`sender` doivent être
     clonables dans le spawn (vérifier `node.sender()` vs API `reprobe_relays`).
  3. Re-tester chaos.monkey seed 7 → DÉFENDU ; corpus vert.
- **✅ CORRIGÉ (build 22)** : bornage timeout (`RECOVERY_OP_TIMEOUT = 1500ms`) sur
  TOUS les `.await` réseau de la branche `reconnect_check` (`connected_peers`,
  `publish_to_dht`, `join_peers`, `reprobe_relays`). Ce sont des ops best-effort
  (résultats jetés, rejouées toutes les 15s) → un await qui pend passe d'« infini »
  à « 1,5s puis la boucle draine son backlog de requêtes ». Le trou noir permanent
  disparaît. `connected_peers` timeout → défaut « isolé » (déclenche la recovery,
  ne fige pas). Note : les lookups DHT et `spawn_rendezvous_round` étaient déjà
  spawnés — cohérent avec la nouvelle borne.
- **Régression** : `chaos.monkey` seeds 7/42/99 → step « nodes stay responsive »
  **PASS** (réponse en 3-23ms, était timeout infini). Test de régression permanent
  intégré au scénario (assertion dure sur la réactivité).
- **Statut corpus** : `was_breach: true` → maintenant DÉFENDU. Commit du fix : voir git.

### [BUILD 22 · 2026-07-06] FINDING #2 (ouvert) — la présence ne REPREND pas toujours après churn
- **Attaque** : `chaos.monkey` (même). Distinct de #1 (nœud figé) : ici les nœuds
  sont **réactifs**, mais aucune NOUVELLE acceptation ne se produit après re-mesh
  sur certains seeds (seed 7 : accepted 0→0 ; seed 42 : 11→29 « resumed »). **Seed-dépendant**.
- **Verdict** : dégradation fonctionnelle intermittente, PAS un crash/freeze.
- **Hypothèse** : `check_presence_all_online` ne challenge que les pairs `status==Online` ;
  après revive+`add_peer_addr`, un pair ne repasse Online qu'au heartbeat/gossip. Dans le
  setup synthétique (relay_urls vides, pas de bootstrap gossip), le marquage Online peut
  ne jamais arriver → challenge-all-online saute ces pairs → 0 acceptation.
- **Portée** : possiblement un **artefact de test** (pas de heartbeat/gossip dans le setup
  isolé) plutôt qu'un bug protocole — à confirmer sur la **vraie flotte** (iPhone×2/iPad/
  NAS/Mac) où heartbeats + gossip marquent les pairs Online en quelques s.
- **Assertion** : NON-FATALE dans le scénario (observation loggée) tant que non tranché.
- **Statut corpus** : `was_breach: null` (observation), à re-tester sur flotte réelle.

Attaques déjà prouvées DÉFENDUES en amont (tests runtime + scénarios, à porter au corpus dès le 1er run) :
- pres.forge, pres.replay, pres.usurp, pres.reflect, pres.skew, pres.mem, pres.flood (budget)
  → 14 tests d'intégration + storm 5/5 + chaos-monkey. Servent de baseline « déjà vert ».
