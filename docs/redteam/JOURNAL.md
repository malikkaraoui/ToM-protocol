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

### [BUILDS 23→27 · 2026-07-06 soir/nuit] Chaîne de 5 brèches corrigées + 1 ouverte — session boucle autonome

Enchaînement red-team continu (l'utilisateur a laissé les devices la nuit). Chaque
brèche : trouvée → attaque reproductible → fix → test régression → gate → build →
flotte redéployée → régression complète. Détail machine dans `journal.jsonl`/`corpus.jsonl`.

- **FINDING #4 (build23) — pres.sybil** : budget répondeur présence PAR-IDENTITÉ sans
  cap global → rotation d'identités = 10×N signatures Ed25519 (CPU DoS scalable). Le flood
  mono-identité défendait (10), le swarm bypassait. **Fix** : `RESPONDER_GLOBAL_BUDGET_PER_WINDOW=120`.
  Attaque live `presence-attack` step sybil : 200 signés → 110 (borné).
- **FINDING #5 (build24) — pres.starvation** : *second-ordre de mon propre fix #4*. Le cap
  global plat était partagé et prioritaire → un flood trivial d'inconnus (120/fenêtre) refuse
  la présence à TOUS les pairs légitimes (DoS famine). J'avais converti un DoS-CPU en DoS-famine.
  **Fix DEUX-VOIES** : les CONNUS (preuve de relais, score ≥ `RESPONDER_KNOWN_MIN_SCORE=1.0`,
  non-forgeable par sybil frais) contournent le cap ; seuls les inconnus le partagent.
- **FINDING #6 (build25) — ack.forged_recipient** : `mark_delivered`/`mark_read` ne liaient
  pas l'ACK au destinataire. Un relais malveillant (voit le message_id qu'il route) forge un
  ACK RecipientReceived signé de SA clé → l'expéditeur marque Delivered + arrête les retries
  alors que le message a pu être droppé. **Casse la décision LOCKED #1** + le modèle de
  non-confiance envers les relais. **Fix** : `entry.to == from` obligatoire, `pending.remove`
  seulement si livraison réelle, garde signature ajouté au ReadReceipt.
- **FINDING #7 (build26) — relay.score_pumping** : *sape le fix #5*. `record_relay(from)` était
  appelé INCONDITIONNELLEMENT sur ACK RelayForwarded, sans valider un vrai message émis.
  L'anti-replay ne bloque que le tuple identique → l'attaquant forge N ACK (message_id aléatoires)
  → pompe son score de relais sans rien relayer → devient "connu" → contourne le gate présence ET
  le cap inconnu #5. **Détruit la primitive de preuve-de-relais.** **Fix** : créditer seulement si
  `tracker.recipient_of(msgid)` existe ET `from != destinataire final`.
- **FINDING #8 (build27) — antispam.swarm_ingress** : même classe que #4, côté chat. Antispam
  per-sender (`min_rate=30/s`) sans cap global, et l'éviction LRU réinitialise les buckets →
  swarm inonde en agrégat (CPU verify). **Fix DEUX-VOIES** (motif #5 réutilisé) :
  `GLOBAL_STRANGER_RATE=200/s` partagé par les inconnus, connus exemptés.
- **FINDING #9 (OUVERT) — backup.store_flood** : store backup évince le plus-ancien-global sans
  quota par-déposant ; flood de ReplicationPayload arbitraires évince les backups légitimes
  (dégrade ADR-009). **NON corrigé volontairement** : subsystème LOCKED + endurance-testé, le fix
  (budget inconnu deux-voies + éviction équitable) doit être re-validé en endurance, pas rushé la
  nuit. Atténué par le cap #8 (200/s). Documenté pour implémentation soignée ultérieure.

**Régression complète build 27 (tout vert)** : lib 561/561 · presence-attack 6/6 ·
presence-storm 5/5 · chaos-monkey seed 7 6/6 (FINDING #1 tient, #2 resumed 24→68) ·
failover 8/8 · e2e 3/3 · group 8/8 · backup 4/4 · presence 5/5.

### Recul méthodologique (pour l'utilisateur — "savoir si notre méthodologie est bonne")
**Ce qui marche** :
1. *La boucle trouve des chaînes, pas des bugs isolés.* #5 est né de #4, #7 sape #5 — corriger une
   brèche a ouvert/exposé la suivante. Sans re-attaquer après chaque fix (« et si mon fix crée un
   trou ? »), #5 et #7 seraient passés inaperçus. **Le réflexe "attaquer mon propre fix" est le plus
   rentable de la session.**
2. *Une CLASSE se dégage :* « limite per-identité sans cap global → bypass par rotation d'identités »
   (présence #4, pumping #7, antispam #8, backup #9). Une fois nommée, on la cherche partout — c'est
   un multiplicateur. Le fix canonique (deux-voies : connus contournent, inconnus partagent) est
   réutilisable tel quel.
3. *Tests déterministes + régression live systématique.* Chaque fix a un test unitaire (rejoue la
   logique, rapide) ET une régression scénario réel (QUIC). Les deux niveaux ont attrapé des choses
   différentes (#5 prouvé en unit, #7 vérifié en live que le flux légitime n'est pas cassé).
**Limites / à surveiller** :
- Coût de redéploiement flotte élevé (~15 min/build : XCFramework + 3 apps + NAS). J'ai fini par
  batcher (sauter le déploiement 26 des apps ATV/Mac, passer direct au 27). À l'avenir : accumuler
  2-3 fixes par build si non-critiques, ou séparer "fix shippé git" de "flotte redéployée".
- FINDING #9 montre la bonne discipline (ne pas rusher un subsystème verrouillé) mais laisse une
  brèche ouverte — il faut un créneau dédié + re-validation endurance, pas la boucle nocturne.
- La plupart des brèches sont dans la couche présence/roles/antispam (récemment écrite). Les couches
  anciennes (transport forké, quinn, gossip) n'ont pas encore été attaquées frontalement — angle mort.

---

## Nuit 2026-07-10 — le bug d'échelle qui a accouché de PoP

Découvert **en testant R13 sur vrais devices** (iPad/Apple TV) : chaque nœud voyait **44-50 pairs
`Online`** avec `connected_peers()` ≈ 0. Cause vérifiée : tout pair *découvert* (DHT `state.rs:2341`,
gossip `:2440`, neighbor `:2473`) est marqué `Online` **sans preuve de connexion**, et la
re-découverte/ré-annonce rafraîchit le heartbeat en boucle → fantômes éternels. `online_count`
(`relay.rs:124`) les compte tous → **appareils contraints saturés → chemin de contrôle affamé**
(les writes device hangaient >60s pendant le test R13). **Bug d'ÉCHELLE** : à 100 nœuds = 50 fantômes
gênants ; à 1M = fatal.

**Doctrine née de là (ADR-011) : PoP — Proof of Presence.** La présence n'est pas déclarée, elle est
*prouvée par le travail* (ACK signé, relais utile, backup restitué). On supprime le heartbeat
déclaratif. Unifie présence = rôle = réputation = anti-Sybil (le signal « avoir mis sa pierre »).
Spec complète : `docs/plans/POP-PROOF-OF-PRESENCE.md`.

**Red-team du design lui-même** (4 attaquants, verdicts vérifiés §5) :
- RÉFUTÉ : « ACK non signé » (faux — `state.rs` rejette `!signature_valid`). Fondation OK.
- CORRIGÉ cette nuit : **inflation de score par `bandwidth_ratio` non borné** (`scoring.rs`) →
  `BANDWIDTH_RATIO_CAP=3.0` + régression. Fermait un farming de privilège « known » cheap.
- CHANTIER R14 (trop risqué à lander de nuit) : `PeerStatus::Known` (couplé au split lecteurs
  routage/présence, sinon casse `select_best`), séparation présence-courte/réputation-lente,
  et le trou dur **eclipse** (témoin-relais unique non détectable → quorum de témoins requis).

**Leçon méthodo :** le meilleur test n'est pas unitaire — c'est le **déploiement réel**. R13 passait
520+ tests unitaires ; il a fallu 3 vrais appareils sur un vrai réseau pour exposer (a) 2 bugs de
câblage R13, (b) le bug d'échelle des pairs fantômes qui a fait émerger PoP. **Tester sur devices via
API = non négociable.**

### Attaque frontale de la couche gossip (angle mort) — verdicts vérifiés §5

- **FINDING G-1 (CONFIRMÉ, wire-breaking) — PeerAnnounce non authentifié.** `PeerAnnounce`
  (`discovery/types.rs:37-48`) n'a **pas de champ `signature`**, contrairement à `RoleChangeAnnounce`
  et `RelayReadyAnnounce` (tous deux `verify_signature()`). Le handler `state.rs:2418-2443` **applique
  le rôle depuis l'annonce** (`Relay` si `roles` le contient) et upsert la topology, **sans vérifier
  que l'émetteur possède `node_id`**. Un attaquant forge `PeerAnnounce{node_id: victime, roles: […]}`
  → injecte de faux relais, downgrade un vrai relais en Peer, gonfle la topology de fantômes.
  *Pas* de MITM (l'`encryption_key` de l'annonce n'est jamais appliqué à ce chemin) ni d'OOM
  (topology bornée 10k + éviction, cf G-2). Fix = signer PeerAnnounce + vérifier avant d'appliquer,
  comme les deux autres annonces. **⚠️ Change le wire gossip → rollout flotte coordonné, sous
  supervision (risque de partition si partiel). NON déployé de nuit.**
- **FINDING G-2 (RÉFUTÉ)** — « topology non bornée → OOM » : faux. `MAX_PEERS=10_000` + éviction du
  plus-stale-non-online existent (`relay.rs:13,71`, fix build 28 / red-team #10).
- **G-3 (faible)** — champ `username`/`roles` de taille attaquant : borné par `read_lp` (≤4 KiB/msg)
  + MAX_PEERS. Validation de longueur inbound = hygiène, faible gain.
- **G-4 (= PoP)** — rejeu d'annonce rafraîchit `last_seen` : c'est exactement le ouï-dire que PoP
  supprime. Traité par le chantier R14, pas isolément.

**Discipline de nuit :** on ne lande/déploie QUE des fixes sûrs, non-wire, non-critiques-routage
(ex. cap ratio scoring). Les fixes wire-breaking (PeerAnnounce signé) ou routage-critiques
(PoP `Known`, quorum eclipse) sont **préparés + journalisés**, déployés sous supervision. Ne jamais
risquer une partition de la flotte réelle en autonomie nocturne.
