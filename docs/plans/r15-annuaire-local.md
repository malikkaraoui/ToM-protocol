# R15 — Annuaire local (mémoire des pairs) : cartographie + conception

> Créé le 2026-07-18. Chantier roadmap `vault/40-roadmap.md:213-216`.
> Ordre imposé par le prompt de reprise : **cartographier l'existant AVANT de concevoir**, pour
> ne pas dupliquer l'anti-ravivage (builds 121/122).

## §1 Ce qui est DÉJÀ persisté (vérifié sur pièces 2026-07-18)

### Table `peers` — schéma V4 (`storage/schema.rs:66-72`, `CURRENT_VERSION=4` ligne 7)

```sql
CREATE TABLE IF NOT EXISTS peers (
    node_id   TEXT PRIMARY KEY,
    role      TEXT NOT NULL,   -- Peer | Relay
    status    TEXT NOT NULL,   -- Online | Offline | Stale | Known
    last_seen INTEGER NOT NULL -- unix ms
);
```

`PeerInfo` en RAM (`relay.rs:66-72`) porte exactement les mêmes 4 champs. **Aucune adresse,
aucun relay_url, aucun path_kind — ni sur disque, ni en mémoire topologie.**

Autres tables persistées (hors sujet R15) : `groups`, `sender_keys`, `hub_groups`,
`contribution_metrics`, `tracked_messages`, `hub_message_history`.

### Cycle de vie (anti-ravivage, builds 121/122)

- **Save toutes les 30 s** ; au save, M2 élague `status != Online && now - last_seen > TOPOLOGY_TTL_MS (24 h)`.
- **Load** : même filtre M2 (migration gratuite des bases anciennes), puis **tous les pairs
  chargés sont forcés `Offline`** (aucune connexion QUIC ne survit au restart).
- **M3** `Topology::evict_stale(now)` au tick 60 s ; un `Online` n'est jamais évincé.
- Bootstrap : les pairs vus < 5 min (`BOOTSTRAP_RECENT_WINDOW_MS`), triés par `last_seen`
  desc, cap 16 (`BOOTSTRAP_MAX_PEERS`) sont réinjectés comme pairs gossip.

### Le transport ne persiste RIEN

`tom-connect` garde `EndpointAddr` (addrs + relay_urls) et le `PathKind` en RAM seulement.
Backup store ADR-009 : **RAM pure** (`backup/store.rs:19-31`, `HashMap`), pas de SQLite —
contrairement à ce que laisse croire une lecture rapide de la doc.

### Conséquence : ce que coûte un restart aujourd'hui

Au redémarrage, un nœud connaît les *identités* de ses pairs mais **aucune adresse**. Toute
reconnexion repasse donc par la découverte : mDNS (LAN, ~1-2 s), gossip bootstrap, rendez-vous
DHT (publication détachée ~30 s, tick 60 s). C'est exactement le délai que R15 veut supprimer.

## §2 Delta R15 = strictement les adresses

| Donnée | Persistée ? | R15 |
|---|---|---|
| node_id, role, status, last_seen | ✅ déjà | inchangé |
| dernières adresses directes (LAN / publique / v6) | ❌ | **à ajouter** |
| relais habituel | ❌ | **à ajouter** |
| dernier `path_kind` (Direct/Relay) | ❌ | **à ajouter** |
| expiration | TTL dur 24 h (M2/M3) | **expiration douce** à définir (décision #4 : fade, pas de binaire) |

Aucun recouvrement avec l'anti-ravivage : celui-ci gère la *présence*, R15 gère la
*joignabilité*. Les deux doivent rester cohérents (§3).

## §3 Le risque central : R15 peut RESSUSCITER l'empoisonnement de topologie

C'est le point qui doit gouverner toute la conception. Historique :
- 17/07 : 1286 pairs persistés, fantômes ravivés par gossip → boucles gelées 84-218 s.
- 18/07 : 280 pairs fantômes sur le Mac tuaient l'émission (766 AUTO-PING failed) ; guérison
  par purge du state.db.
- Builds 121/122 : anti-ravivage (M1 filtre re-dial, M2 élagage save/load, M3 evict_stale).

**R15 persiste des adresses dialables. Mal conçu, il redonne au nœud exactement ce qui lui
manquait pour re-dialer des fantômes au démarrage — en pire, puisque les adresses survivent
maintenant au restart.** Contraintes non négociables qui en découlent :

1. **Une adresse persistée n'accorde AUCUNE présence.** Elle ne crée jamais un pair, ne change
   jamais un `status`, ne compte jamais dans `taille_reseau`. Elle n'est qu'un *candidat de dial*
   pour un pair déjà connu par ailleurs.
2. **Budget de dial borné au démarrage** : réutiliser le cap existant (16 pairs bootstrap,
   fenêtre de fraîcheur), pas un dial de toute la table.
3. **Expiration au moins aussi stricte que M2** : une adresse ne peut pas survivre à son pair.
   Si M2 élague le pair, ses adresses partent avec (`ON DELETE CASCADE` ou colonnes dans `peers`).
4. **Un échec de dial doit dégrader l'entrée** (décision #4 : fade progressif), pour qu'une
   adresse morte ne soit pas retentée indéfiniment à chaque boot.

## §4 Conception proposée (à red-teamer avant code)

### §4.1 Schéma — V5, colonnes dans `peers` plutôt que table 1:N

```sql
ALTER TABLE peers ADD COLUMN direct_addrs_json TEXT;  -- ["192.168.0.23:59455","[2a01:...]:60085"]
ALTER TABLE peers ADD COLUMN preferred_relay_url TEXT;
ALTER TABLE peers ADD COLUMN last_path_kind TEXT;     -- "Direct" | "Relay"
ALTER TABLE peers ADD COLUMN addr_fail_count INTEGER NOT NULL DEFAULT 0;
```

Pourquoi des colonnes et pas une table `peer_addresses(node_id, addr, kind, last_seen)` :
la contrainte 3 (les adresses meurent avec le pair) est gratuite, le save reste un seul
`INSERT OR REPLACE`, et le volume est trivial (quelques adresses par pair, cap explicite).
Une table 1:N se justifierait seulement si on voulait un historique par adresse — inutile ici.
**Cap dur : 8 adresses par pair**, alignées sur l'esprit de `MAX_DHT_ADDRS=32`.

### §4.2 Alimentation

Source unique : les `PathEvent` du transport (déjà consommés par le runtime pour
`paths_by_peer`, cf. `runtime/loop.rs:438-447`). Quand un chemin **DIRECT signé et vivant**
est observé vers un pair, son adresse est mémorisée. Jamais depuis une annonce non vérifiée —
même exigence que M1 (reset de throttle uniquement sur trafic direct signé).

### §4.3 Usage au démarrage — dial parallèle

Aujourd'hui : load → tous Offline → attendre la découverte.
Avec R15 : load → pour les pairs bootstrap éligibles (< 5 min, cap 16), lancer **en parallèle**
(a) le dial des adresses en cache et (b) la découverte normale (mDNS/DHT/gossip). Le premier
qui aboutit gagne ; le cache n'inhibe jamais la découverte fraîche (sinon une adresse périmée
retarderait la reconnexion au lieu de l'accélérer).

### §4.4 Expiration douce (décision #4)

- `addr_fail_count++` à chaque dial infructueux, remis à 0 sur trafic direct signé.
- À `addr_fail_count >= 3`, l'adresse cesse d'être tentée au boot mais **reste stockée**
  (elle peut redevenir valide : même LAN retrouvé, DHCP identique). Pas de suppression brutale.
- Élagage effectif : porté par M2 (le pair disparaît → ses adresses aussi). Pas de second TTL
  concurrent — un seul horizon temporel dans le système (24 h).

### §4.5 Conformité LOCKED

#1 livraison, #2 TTL 24 h (aucun horizon nouveau), #3 L1 (non concerné), #4 fade progressif
(§4.4, pas de ban), #5 anti-spam (le cap de dial borne la charge), #6 invisibilité (aucun état
exposé à l'UI ; `paths_by_peer` reste du debug), #7 scope (mécanisme de transport générique).

## §5 Validation exigée avant de déclarer R15 fait

- Test déterministe : un nœud redémarré retrouve un pair du cache **sans DHT** (harnais isolé
  `--isolated`, DHT off depuis le build 124).
- Non-régression anti-ravivage : après restart, `taille_reseau` ne croît pas et aucun dial
  vers un pair > 24 h (le piège du 17/07 rejoué exprès, avec un state.db pollué en fixture).
- Mesure réelle : délai reconnexion après restart, avant/après, sur la flotte — via le runbook
  `docs/plans/RUNBOOK-TESTS.md`, sur connexions réelles jamais sur un proxy.

## §6 Red-team + contre-vérification sur pièces (2026-07-18)

Un agent adversarial a rendu un verdict **BLOQUANT** sur 7 findings. **Chaque finding a été
revérifié dans le code** (leçon [[verify-subagent-security-shortcuts]]) : 3 ne survivent pas.

### RÉFUTÉ — F3 « PathEvent non authentifié » (l'agent annonçait MAJEUR)

L'agent écrivait lui-même « jamais lu, donc hypothèse ». Le code dit l'inverse :
`tom-transport/src/protocol.rs:241` → `let remote = NodeId::from_endpoint_id(connection.remote_id())`,
et `spawn_path_watcher(&connection, remote, …)` (protocol.rs:263 entrant, connection.rs:236 sortant).
Le `remote` d'un `PathEvent` est **l'identité de la connexion QUIC établie**, donc authentifiée par
le handshake TLS où le NodeId EST la clé publique Ed25519 (ADR-005). On ne peut pas fabriquer une
connexion QUIC en se faisant passer pour Bob sans la clé privée de Bob.
→ **L'alimentation §4.2 est saine.** Une adresse apprise sur connexion établie est liée
cryptographiquement au pair. Pas de correction nécessaire.

### RÉFUTÉ — F1 dans sa forme annoncée (« BLOQUANT »), MAIS un vrai trou en dessous

L'agent se contredisait (« les Online sont chargés intacts » puis « forcé Offline s'applique à
tous »). Réalité vérifiée, en deux temps :
- `storage/mod.rs:463-475` : le load **préserve** `"Online" => PeerStatus::Online`, et le filtre
  TTL M2 est gardé par `if status != Online && âge > TOPOLOGY_TTL_MS { continue }`.
  **Donc un pair sauvegardé `Online` échappe à l'élagage M2 au load, quel que soit son âge.**
  Le commentaire juste au-dessus (« Un pair rechargé n'est jamais Online ») **décrit l'intention,
  pas ce que fait cette fonction** — la doc ment localement.
- `runtime/state.rs:218-221` : le forçage `peer.status = PeerStatus::Offline` existe bien, mais
  **après** le load, à l'insertion en topologie.

Conséquence réelle **aujourd'hui** : exposition bornée à ~30 s (au save suivant le pair est
Offline, donc M2 s'applique). Sauf en **crash-loop** (redémarrages < 30 s d'uptime) où les
fantômes `Online` sont rechargés indéfiniment.
Conséquence **si R15 existait** : ces fantômes-là arriveraient avec des adresses dialables, et
le dial parallèle §4.3 les tenterait *avant* l'élagage. C'est le scénario à fermer.

→ **Correction retenue** : le cache d'adresses n'est chargé QUE pour les pairs qui survivent au
filtre M2, en appliquant le filtre d'âge **indépendamment du statut** (`Online` persisté n'est
pas une présence, c'est un souvenir). À traiter comme un fix autonome, indépendant de R15.

### RÉFUTÉ — F5 « addr_fail_count ≥ 3 = ban déguisé (LOCKED #4) »

Le doc dit déjà « reste stockée », « pas de suppression brutale », et l'entrée redevient valide
sur trafic direct signé. Un arrêt de *tentative* réversible n'est pas un ban (#4 interdit
l'exclusion permanente et binaire). Correction cosmétique retenue quand même : décrémenter
progressivement plutôt qu'un seuil sec, pour coller à l'esprit du fade.

### RETENUS (vérifiés valides)

- **F4 — budgets de dial** : le dial cache doit passer par le `RedialGovernor` existant
  (throttle 60 s/pair, cap 3 en vol) et non créer une 3ᵉ source de dial non gouvernée.
  Contrainte à graver dans §4.3.
- **F6 — le gain est-il réel ?** Argument solide : sur le LAN mDNS répond en 1-2 s (gain R15
  marginal), et hors-LAN les adresses en cache sont justement les plus susceptibles d'être
  périmées (IP dynamique, ports NAT). **R15 pourrait n'avoir un vrai gain que sur le relais
  habituel, pas sur les adresses directes.**
- **F7 — test de non-résurrection manquant** : exiger une fixture `state.db` polluée rejouée
  contre R15. À ajouter au §5.

### Découverte annexe (hors R15, utile)

`tom-transport/src/protocol.rs:250-257` : le pool **apprend déjà** la route relais d'un pair
(`Auto-learned relay route for {} via {}`) — en RAM. L'alternative ci-dessous s'appuie dessus.

## §7 Décision : R15 réduit à « relais habituel », adresses directes écartées

Le croisement F6 + F4 + le risque §3 change la conception. Le rapport gain/risque des deux
moitiés de R15 est très différent :

| Moitié | Gain | Risque |
|---|---|---|
| Persister les **adresses directes** | marginal sur LAN (mDNS 1-2 s), peu fiable hors-LAN (IP/ports volatils) | élevé : ré-ouvre le vecteur d'empoisonnement, dials vers machines tierces (DHCP réattribué) |
| Persister le **relais habituel** | réel : évite d'attendre la découverte DHT (~30 s) pour retrouver un point de rendez-vous | faible : une URL de relais n'est pas une machine du LAN, déjà apprise et utilisée en RAM |

**Décision (à valider par Malik)** : implémenter **R15-lite = `preferred_relay_url` seul**
(1 colonne, alimentée par la route relais déjà auto-apprise, expirant avec le pair via M2), et
**écarter la persistance des adresses directes** jusqu'à ce qu'une mesure prouve un gain —
mesure qui relève de R14 (reconnexion hors-LAN), pas d'une intuition.

## §8 Statut

- Cartographie : ✅ vérifiée sur pièces.
- Conception initiale : ✅ rédigée, ❌ **partiellement invalidée par sa propre red-team** (§7).
- Red-team : ✅ faite, ✅ contre-vérifiée (3 findings sur 7 réfutés).
- Fix autonome : ✅ **livré** (élagage M2 au load sans garde `Online`, commit `73f9438` —
  test `m2_stale_ghosts_pruned_on_save_and_load` durci).
- **R15-lite : ✅ LIVRÉ (build 129, 2026-07-19)**, validé par le brief de reprise de Malik.
  Réalisation :
  - Schéma **V5** : colonne `preferred_relay_url` dans `peers` (`storage/schema.rs::migrate_v5`).
  - Apprentissage : `RuntimeState::note_path_event` — PathEvent RELAY authentifié uniquement
    (`remote` sort du handshake QUIC, §6/F3). Purge alignée topologie au tick
    (`tick_cache_cleanup`), filtrage au save : une route ne survit jamais à son pair.
  - Load : routes chargées UNIQUEMENT pour les pairs survivant à M2 (même ligne SQL) —
    non-résurrection par construction, testé (`relay_routes_roundtrip_and_die_with_peer`).
  - Semis au démarrage : `runtime/loop.rs` injecte les routes dans le pool transport
    (candidats de dial, AUCUNE présence accordée, aucun dial déclenché — F4 respecté :
    pas de 3ᵉ source de dial).
  - **Test déterministe §5 : ✅** `tom-integration-tests/tests/r15_relay_cache.rs` — relais
    embarqué réel, phase 1 apprentissage, restart sans AUCUNE découverte ni relais configuré,
    livraison via la route persistée seule. ⚠️ Leçon : les nœuds de test doivent être liés
    au **loopback** (`bind_addr 127.0.0.1:0`) — en bind wildcard, la vraie flotte du LAN les
    découvre et les auto-pingue (fuite d'herméticité déjà connue du banc chaos, toujours
    non élucidée — le canal de découverte résiduel reste à identifier).
- Reste (terrain) : mesure I10 (gain de reconnexion ≥ 2×) sur la flotte réelle, et décision
  d'activer `--data-dir` sur le service NAS (aujourd'hui éphémère, donc R15-lite sans effet
  sur le NAS tant que le flag n'est pas ajouté à l'unit systemd).
