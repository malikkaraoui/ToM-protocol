# Verdict OOM Freebox — fuite `ConnectionInner` (tom-quinn) — 2026-07-22

## Résumé exécutif

**Fuite mémoire RÉELLE, non bornée, ∝ churn de connexions QUIC**, prouvée sur le NAS musl
instrumenté. Cause : les connexions QUIC **abandonnées** (handles applicatifs droppés) ne sont
**jamais fermées activement** — le fork a unifié le `ref_count` (handles + driver + paths) dans un
seul `AtomicUsize`, donc `implicit_close` ne se déclenche que quand le **driver** part (dernier ref),
ce qui n'arrive qu'après un `drained` qui ne vient jamais de façon fiable sous churn/kill-brutal.
`ConnectionInner` (~250-400 Ko : buffers, state) s'accumule → OOM.

> ⚠️ Le fix touche le **cœur anti-deadlock** de tom-quinn (le `ref_count` manuel qui a résolu les
> gels terrain 13/07 et iOS 17/07). **NE PAS patcher à l'aveugle.** Design + validation Malik requis.
> La 2e fenêtre de travail touche le même fichier — risque de conflit.

---

## Preuve terrain (load-8 instrumenté, NAS musl ARM)

Binaire ARM instrumenté déployé : `/status` expose désormais `conns_quic_live` (=`Endpoint::open_connections()`
= `proto.connections.len()`) et `handshakes_accepted`. CSV : `/tmp/nas-load.csv`.

| phase | t | rss_kb | conns_quic | handshakes | pool | relais_acc |
|-------|---|--------|-----------|-----------|------|-----------|
| CHARGE | 19s | 92 844 | 131 | 54 | 12 | 196 |
| CHARGE | 46s | 142 436 | 383 | 173 | 12 | 249 |
| CHARGE | 81s | 218 396 | 659 | 311 | 12 | 294 |
| CHARGE | 117s | 265 324 | 836 | 413 | 12 | 325 |
| POST-KILL | 143s | 293 180 | **991** | 480 | 4 | 359 |
| POST-KILL | 173s | **293 268** | **330** | 481 | 4 | 369 |
| POST-KILL | 204s | **293 472** | 329 | 481 | 4 | 374 |

**Lectures décisives :**
1. `conns_quic` monte 131→991 en charge (churn ~4 conn/s, `handshakes` cumulés 54→480) pour **8 nœuds
   loadtest + ~5 flotte** → connexions massivement **non réutilisées / non fermées**.
2. Même à `conns_quic` FIXE (131, t19-37) le RSS montait déjà 92→134 Mo → 2 effets superposés
   (accumulation de connexions + mémoire par connexion sous trafic).
3. **POST-KILL, preuve absolue** : `conns_quic` chute 991→330 (le proto purge ~660 via
   `Drained ⇒ self.connections.try_remove`, tom-quinn-proto `endpoint.rs:132-137`) **mais le RSS reste
   FIGÉ à 293 Mo** (0 redescente). 660 connexions retirées de la map proto = **0 octet rendu** → la
   mémoire n'est PAS dans la SlotMap proto, elle est dans `ConnectionInner` (tom-quinn) qui survit.
4. Il reste **330 connexions fantômes pour 4 pairs réels** → une partie ne se ferme même jamais.

Sur **musl** (qui rend la mémoire proprement, cf. banc hermétique 2e fenêtre) un RSS bloqué = **vraie
fuite**, pas rétention allocateur.

---

## Localisation CONFIRMÉE (file:line)

### Le mécanisme ref_count est correct EN SOI
`tom-quinn/src/connection.rs` :
- `ConnectionRef(Arc<Arc<ConnectionInner>>)` (l.1244), `ref_count: AtomicUsize` **dans** `ConnectionInner`
  (l.1308, sorti du mutex d'état exprès pour l'anti-deadlock, commentaire l.1303-1307).
- `from_arc` (l.1248) : `Arc::clone` + `ref_count.fetch_add(1)`. `Clone` (l.1262) idem. `Drop` (l.1268) :
  `ref_count.fetch_update(-1)` ; **si prev==1 → `implicit_close`** (l.1279-1287). Symétrique, pas de bug ici.

### Le bug de CONCEPTION : plus de distinction handle ↔ driver
- `implicit_close` n'est appelé **qu'à UN endroit** : l.1286 (Drop, prev==1). Vérifié : aucun autre
  déclencheur, **aucun compteur de handles applicatifs séparé**.
- Or `ref_count` compte **TOUS** les `ConnectionRef` : le handle applicatif `Connection(ConnectionRef)`
  (l.316), le `ConnectionDriver(ConnectionRef)` (l.257), et chaque `Path{conn: ConnectionRef}`.
- Le **driver garde une ref tant qu'il vit** — il ne finit qu'à `is_drained()` (l.279-297).
- **Conséquence** : quand les handles applicatifs `Connection` sont droppés (le pool les remplace/purge),
  `ref_count` passe de N à N−1 **sans jamais atteindre 0** (le driver reste). `implicit_close` n'est
  donc **jamais** déclenché pour une connexion abandonnée. Elle n'est **pas fermée activement** — elle
  attend l'idle timeout proto (10s), qui sous kill-brutal (pas de `CONNECTION_CLOSE`) / churn ne draine
  pas de façon fiable → le driver tourne indéfiniment → sa ref survit → `ConnectionInner` jamais libérée.

> En **quinn standard**, le drop du dernier **handle applicatif** (hors driver) déclenche la fermeture.
> Le fork l'a perdu en unifiant le compteur. C'est la régression exacte.

### Innocentés (vérifiés par lecture)
- **`tom-relay`** (relais embarqué) : cycle client accept↔disconnect symétrique, `Actor` droppé à la
  fin de `run()`. `relais_accepts` n'est qu'un **proxy temporel du churn**.
- **DHT / rendez-vous** : `peers_known` PLAT (79→81) pendant que le RSS ×3.
- **Structures applicatives** : Topology cap 10k + TTL 24h, backup 64 Mio, pending 32 Mio — toutes bornées.
- **`ConnectionPool`** (tom-transport `connection.rs`) : `HashMap<NodeId, Connection>` = **une** conn/pair,
  purge active (`retain(close_reason().is_none())` l.84, `remove` l.164). Ne garde pas les churned.

---

## Fix proposé (DESIGN — à valider avant tout code)

**Restaurer la distinction handle applicatif ↔ driver**, sans toucher au `ref_count` atomique
anti-deadlock existant :

1. Ajouter `app_handle_count: AtomicUsize` dans `ConnectionInner` (séparé de `ref_count`).
2. L'incrémenter/décrémenter **UNIQUEMENT** dans `Connection` (le handle applicatif) — Clone/Drop
   manuels (aujourd'hui `Connection` a un `#[derive(Clone)]`, l.315). Le driver et les `Path` gardent
   leur `ConnectionRef` **sans** toucher `app_handle_count`.
3. Quand `app_handle_count` atteint 0 (dernier handle applicatif parti) : signaler la fermeture au
   driver via un `Notify` atomique (`shared.closed` existe déjà, l.1342) — **jamais le verrou d'état**
   (anti-deadlock préservé).
4. Le driver, à son prochain poll, voit le signal → `implicit_close` sous SON verrou → `drained` →
   la task finit → refs lâchées → `ConnectionInner` libérée **immédiatement** (plus d'attente idle 10s).

**PISTE 2 (atténuation ciblée, ne touche PAS le ref_count) — fermeture explicite au remplacement :**
Le churn passe par le POOL (`tom-transport/connection.rs`) : quand une nouvelle connexion d'un pair
arrive, `register_inbound` (l.81-93) `retain` les fermées + `insert` (écrase l'ancienne), et
`get_or_connect` (l.251) `insert` de même. **L'ancien handle `Connection` est droppé mais jamais
fermé** (`close_reason()` reste None → même bug). Fix ciblé : avant d'écraser/retirer une ancienne
connexion vivante, appeler `old.close(0, b"")` (API publique tom-connect) → fermeture active →
drained → libérée. Points : `register_inbound` (avant insert), `get_or_connect` l.251 (avant insert),
`remove`/`unregister_inbound`. **Moins risqué** (API publique, pas le mécanisme interne) mais
⚠️ **logique subtile #46b/#46c** (dial mutuel, fusion QUIC, quelle connexion LIT les entrants) —
fermer trop agressivement peut casser la réception. À implémenter en **worktree isolé** + valider
par re-run load-8 (`conns_quic` doit redescendre au nb de pairs, RSS avec) AVANT merge. Couvre le
cas dominant (churn via pool) sans toucher le cœur ; PISTE 1 (`app_handle_count`) reste le fix complet.

**PISTE 3 (churn) :** 4 conn/s pour 8 pairs est anormal — soit artefact du banc (connexions courtes
répétées), soit réutilisation incomplète. Réduire le churn atténue mais ne résout pas la fuite de fond.

**PISTE 4 (idle timer) :** fiabiliser `drive_timer` pour les connexions abandonnées (cf.
`tom-pathidle-timer-inoperant` — `permit_idle_reset` global ré-armait le mort).

---

## Instrumentation ajoutée (working tree, NON commitée)

Diagnostic `conns_quic_live` + `handshakes_accepted` exposés au `/status`, 4 crates :
- `tom-connect/src/endpoint.rs` : `Endpoint::open_connections()` + `accepted_handshakes()` (public).
- `tom-transport/src/node.rs` : `TomNode::open_connections()` + `accepted_handshakes()`.
- `tom-protocol/src/runtime/metrics.rs` : 2 champs snapshot + `set_conns_quic()` + `loop.rs` tick.
- `tom-tui/src/main.rs` : 2 champs dans le JSON `/status`.

Décider : garder (utile en prod pour l'observabilité connexion) ou retirer avant merge. Le juge
`conns_quic_live` sert aussi à **valider le fix** (doit redescendre à ~nb pairs et le RSS avec).

## Scripts
- `scratchpad/load-8-nas.sh` : banc oracle (capture `conns_quic`, oracle qui distingue fuite-connexion
  vs fuite-buffers). `scratchpad/deploy-nas.sh` : déploie l'ARM instrumenté + vérifie l'instrumentation.

## État
NAS restauré propre (35 Mo). Verdict + localisation confirmés file:line. **Prochain (avec Malik) :
valider le design du fix `app_handle_count`, l'implémenter en worktree, re-run load-8 → `conns_quic`
doit redescendre au nb de pairs ET le RSS avec.** Voir mémoire `tom-freebox-oom-carnet-rendezvous`.
