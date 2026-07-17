# Anti-ravivage des fantômes de topologie — doc de conception

> Design-first (2026-07-17, chantier #2 post-marathon transport). AUCUN code tant que cette
> conception n'est pas validée. Périmètre : l'oubli des identités mortes du store de pairs.
> Complémentaire du chantier « harnais en `enable_dht:false` » (tarir la fabrique à fantômes).

## Contexte et symptôme mesuré (17/07)

- `state.db` du Mac : **1286 pairs** dont des dizaines d'identités de test jetables ;
  au démarrage « auto-reconnect: **951 peers queued** » (gossip les dialait TOUS en série).
- Effet à l'époque : tempêtes de dial → verrou-otage → boucles gelées 84-218 s. Depuis build
  104 (dial hors verrou), plus de gel : le résiduel est du **gaspillage permanent** (dials/joins
  vers des morts toutes les 15 s, bruit réseau, batterie) et une **pollution durable** du store.
- Purge manuelle (state.db vidé appareil par appareil) = pansement, pas une politique.

## Carte VÉRIFIÉE des écritures de `last_seen` (sur pièces — 2 rapports d'agents CORRIGÉS)

⚠️ Méthode : deux passes d'agent Explore ont produit des ancrages faux ou périmés (fichier
inexistant, thèse « annonce horodatée à la réception » démentie par le code). Chaque ligne
ci-dessous a été relue directement dans le source. Ne pas réutiliser les rapports bruts.

| Site | Fichier:ligne | Écrit quoi | Verdict |
|------|---------------|------------|---------|
| Réception d'envelope authentifiée | `runtime/state.rs:1740-1745` | `last_seen=now`, force `Online` (« message receipt is proof of liveness ») | **DIRECT — légitime, ne pas toucher** |
| Annonce directe du pair (QUIC) | `runtime/state.rs:1557-1561` | `last_seen=now`, `Online` | **DIRECT — légitime** |
| `UpsertPeer` (API locale : mDNS, FFI) | `runtime/state.rs:2600-2608` | upsert + heartbeat source `Direct` | **DIRECT local — légitime** |
| Gossip `PeerAnnounce` relayé | `runtime/state.rs:2955-2962` → `mark_known` | **RIEN sur pair existant** (`mark_known` early-return `state.rs:1597-1600`) ; insert `Known` si nouveau | **DÉJÀ NEUTRALISÉ (ADR-011)** |
| Rendezvous DHT (`DhtLookupResult`) | `runtime/state.rs:2836-2838` → `mark_known` | idem — « Discovery only (ADR-011 ghost-peer fix) » | **DÉJÀ NEUTRALISÉ (ADR-011)** |
| Presence view L1-003 (quorum de témoins signés) | `runtime/state.rs:2243-2250` | promotion `Known/Stale/Offline → Online`, `last_seen=now` | **INDIRECT mais quorum-attesté — ne pas toucher** |
| Role announce gossipé | `runtime/state.rs:775-780` | `last_seen = announce.timestamp.min(now)` (timestamp SIGNÉ, clamp anti-futur red-team) | ⚠️ wart : peut **régresser** un `last_seen` plus frais (vieille annonce relayée) |
| Restore state.db au boot | `runtime/state.rs:214-218` | `last_seen` PRESERVÉ, status forcé `Offline` | inerte |
| Dégradation `Online→Stale` | `runtime/state.rs:2128-2134` | status seul | inerte |

**Conclusion de la carte : le « ravivage » de `last_seen` par gossip/DHT n'existe plus** —
ADR-011 l'a fermé. Un fantôme ne redevient jamais « frais ». Le problème est ailleurs :

## Le vrai problème (reformulé après vérification)

- **P1 — Mémoire sans oubli.** `Topology` : `MAX_PEERS = 10_000` (`relay.rs:13`) avec éviction
  du plus vieux non-Online **uniquement sous pression d'insertion** (`relay.rs:91-108`). Aucune
  purge temporelle, ni en mémoire ni en base (`storage/mod.rs` save/load intégral, restore
  `Offline` mais PRÉSENT). Une identité de test de 10 s vit des semaines.
- **P2 — Rejoin 15 s sans filtre.** `runtime/loop.rs:762-780` : à chaque tick,
  `known_node_ids = topology.peers()` **en entier** → join gossip/dials best-effort vers CHAQUE
  identité stockée, y compris 951 fantômes. La borne-16 du bootstrap
  (`runtime/mod.rs:935-952`, fenêtre 5 min) ne protège que le DÉMARRAGE, pas le régime 15 s.
- **P3 — Fabrique à fantômes.** Harnais de test + pre-push gate sèment des identités 10 s
  dans le rendezvous DHT partagé (chantier séparé `enable_dht:false`).

## Objectifs / non-objectifs

- **Objectifs** : oubli progressif des identités plus jamais vues ; coût réseau ~zéro pour les
  fantômes ; state.db auto-nettoyant ; AUCUNE régression sur : décision #4 (fade, pas de ban),
  ADR-011 (ne pas affamer la découverte), rejoin des pairs vivants, failover de groupe, LAN.
- **Non-objectifs** : réputation négative, blacklist, changement de wire format, toucher au
  mécanisme L1-003 (presence quorum) ou aux signaux de découverte.

## Conception — 3 mesures + 1 wart

### M1 — Filtre d'âge au rejoin 15 s (tue la tempête, plus petit patch)

**Le pattern existe déjà dans le crate** : le rejoin sur `NeighborDown` (`loop.rs:626-650`)
filtre `now - last_seen < REJOIN_RECENT_WINDOW_MS` (5 min, `loop.rs:227`) + tri par fraîcheur
+ cap `REJOIN_MAX_PEERS`, avec un commentaire décrivant EXACTEMENT notre problème (« the
topology holds hundreds of DHT-rendezvous entries that are mostly noise »). Le bootstrap
(`runtime/mod.rs:935-952`) applique les mêmes bornes. **Seul le tick reconnect 15 s
(`loop.rs:762-780`) est passé à travers** : il embarque `topology.peers()` en entier.

M1 = appliquer au tick 15 s le MÊME filtre que ses deux frères (mêmes constantes
`REJOIN_RECENT_WINDOW_MS`/`REJOIN_MAX_PEERS`, + clause `status ∈ {Online, Stale}` toujours
éligible pour ne jamais lâcher un pair vivant). Pas de nouvelle constante, pure cohérence.

- Un fantôme reste en mémoire (pas de ban) mais **ne coûte plus rien** toutes les 15 s.
- Un pair ancien qui revient est re-découvert par les canaux normaux (rendezvous ADR-010,
  gossip, mDNS → `mark_known` → trafic direct → `Online`) — chemin déjà en place, zéro ajout.
- Risque « nœud isolé avec uniquement des pairs >5 min » : le rendezvous DHT est le filet
  zéro-état ; à VALIDER en canari par la métrique de reconvergence <5 s (recette_finale.sh).

### M2 — TTL d'oubli en base (l'hygiène)

`storage/mod.rs` : au save ET au load, écarter les pairs `now - last_seen > TOPOLOGY_TTL_MS`.
Proposition : **7 jours** (c'est un carnet d'adresses, pas un message : 24 h évincerait le
laptop éteint un long week-end ; 7 j reste un fade naturel). Alternative 24 h si l'on préfère
la cohérence stricte « rien ne survit 24 h » — **à trancher à la review de ce doc**.

- Le scénario « 1286 pairs » devient impossible en régime permanent ; migration gratuite
  (le premier load post-patch nettoie les bases existantes).

### M3 — Éviction proactive douce (le plafond réel)

Sur le tick 60 s existant (state_save) : retirer de la Topology mémoire les pairs non-Online
d'âge > `TOPOLOGY_TTL_MS` (même constante que M2). `MAX_PEERS=10_000` reste en garde-fou dur.

### Wart — role announce régressif

`state.rs:779` : `last_seen = max(peer.last_seen, announce.timestamp.min(now))` — une vieille
annonce relayée ne doit ni rajeunir (déjà le cas) ni VIEILLIR un pair (bug actuel, bénin).

## Invariants respectés (checklist)

- **Décision #4 (fade, pas de ban)** : l'oubli est une éviction de cache ; tout signal frais
  (annonce, mDNS, dial entrant, rendezvous) réinsère instantanément via `mark_known`. Aucun
  état « banni », aucune mémoire négative.
- **ADR-011** : intact — on ne touche à aucun signal de découverte, seulement au COÛT récurrent
  (rejoin) et à la RÉTENTION (TTL). Leçon 13-14/07 gardée en tête : un fix défensif ne doit pas
  affamer la découverte en silence → d'où la clause « Online/Stale toujours éligibles » (M1) et
  la validation par métrique de reconvergence, pas au feeling.
- **L1-003 / groupes / LAN** : non touchés.

## Recette (métriques AVANT flotte — garde-fou du 17/07)

1. Canari Mac uniquement, state.db pollué RÉEL (copie d'avant purge si disponible, sinon seedé) :
   - taille du store avant/après load (M2), « auto-reconnect: N peers queued » à froid,
   - zéro join/dial vers les identités mortes au tick 15 s (log),
   - reconvergence <5 s conservée + 15 min de stabilité pairs/ticks (recette_finale.sh),
   - A/B avec un témoin non patché.
2. Puis flotte, appareil par appareil, `conn_tracker.py` en juge.

## Plan de livraison (chaque étape shippable, canari d'abord)

1. **M1 + wart** (petit diff `loop.rs` + `state.rs`, tests unitaires du filtre).
2. **M2 + M3** (même constante, test de migration sur base polluée réelle).
3. Chantier `enable_dht:false` des harnais (indépendant, complémentaire — P3).

## Questions ouvertes pour la review du doc

- TTL base/mémoire : **7 j** (proposé) ou 24 h (cohérence stricte) ?
- M1 : faut-il un plancher « garder quand même les K plus récents même hors fenêtre » pour un
  nœud longtemps isolé, ou le rendezvous suffit-il (position actuelle : il suffit, ADR-010 —
  et c'est déjà le statu quo du bootstrap/NeighborDown sans incident depuis leur pose) ?
