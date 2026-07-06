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
- **Décision session** : fix NON appliqué ici (changement au cœur du `select!` loop,
  risqué à chaud en fin de longue session ; §6). Diagnostic complet livré, patch queué.
- **Statut corpus** : `was_breach: true` — test de régression permanent.

Attaques déjà prouvées DÉFENDUES en amont (tests runtime + scénarios, à porter au corpus dès le 1er run) :
- pres.forge, pres.replay, pres.usurp, pres.reflect, pres.skew, pres.mem, pres.flood (budget)
  → 14 tests d'intégration + storm 5/5 + chaos-monkey. Servent de baseline « déjà vert ».
