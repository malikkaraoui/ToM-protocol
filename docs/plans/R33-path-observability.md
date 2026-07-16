# Chantier #33 — Observabilité du path par pair (build 72)

> 2026-07-16 · pipeline loop-master (Chef→Codeur→Relecteur×2→vérif orchestrateur)
> Observabilité pure : **zéro changement de comportement réseau**, zéro URL en dur.

## Pourquoi

Le réseau affichait « 100% RELAY » sur la flotte alors que les logs trace du NAS (build 71,
`RUST_LOG` ciblé sur `tom_quinn_proto::iroh_hp` + `connection::paths` + `remote_map`) prouvaient :

- NAS↔Mac : DIRECT IPv4 LAN (`192.168.0.82`, ~7 ms)
- NAS↔iPad : Relay → **DIRECT IPv6 global** (`[2a01:…]`, ~4,5 ms) en ~3 s — **le fix #32 marche**
- NAS↔AppleTV : DIRECT via hairpin NAT public (`82.67.95.8`, ~12 ms)

L'affichage mentait pour deux raisons (vérifiées file:line avant le chantier) :

1. **FFI `types.rs`** : la conversion de `ProtocolEvent::PathChanged` omettait `event.remote`
   → `node_id: None` — Swift ne pouvait pas savoir de quel pair venait l'événement.
2. **FFI `lib.rs`** : `last_path` était un **singleton global** — le dernier event de n'importe
   quel pair écrasait l'état affiché (un seul pair en RELAY suffisait à afficher RELAY).
3. Bonus : `classify_path` (tom-transport) jetait l'adresse du path sélectionné → impossible
   de distinguer DIRECT-v4 / DIRECT-v6 / relay au niveau app.

Leçon appliquée : « observabilité = vérité terrain » (régression réseau de juillet survécue des
semaines derrière des indicateurs proxy verts).

## Ce qui a été livré

| Couche | Changement |
|---|---|
| `tom-transport` | `PathEvent.addr` (adresse du path sélectionné, `relay:<url>` ou `ip:port`) ; émission sur changement de kind **ou** d'addr (DIRECT v4→v6 émet) |
| `tom-protocol-ffi` | event `PathChanged` porte `node_id` + `path_addr` ; `last_paths` par pair (`HashMap<node_id, PathInfoFFI{kind,rtt_ms,addr}>`, borné 2048, purgé sur `PeerOffline`/`PeerStale`) ; status JSON : `paths_by_peer` + agrégat de compat = **pire path réel** (sévérité UNKNOWN > RELAY > DIRECT, rtt max à égalité) |
| `tom-tui` | `paths_by_peer` dans le status HTTP (`--status-port`), câblé en mode bot **et** TUI, borné/purgé pareil ; log collecteur UDP `path_change pair=… kind=… rtt_ms=… addr=…` |
| `TomProtocolKit` | `PathInfo` Codable, `pathsByPeer`/`pathAddr` dans `TomNodeStatus`, `path_addr` sur l'event ; Live Log « 🔀 Chemin <pair> → KIND addr » ; build 72 |
| App (iOS/tvOS/macOS) | carte Pairs : badge path par pair (`DIRECT v6 4ms`…) ; header : agrégat honnête (« 2 DIRECT · 1 RELAY ») au lieu du singleton |

## Findings de relecture (corrigés avant commit)

1. Agrégat `lp.values().next()` = entrée arbitraire de HashMap (commentaire mensonger) → pire-cas réel.
2. Maps jamais purgées/bornées → borne 2048 + purge sur départ du pair (FFI **et** tom-tui).
3. Tuple `(String,u64,String)` sérialisé en array JSON hétérogène → struct `PathInfoFFI` (Swift Codable).
4. JSON du status tom-tui reconstruit par `format!` + replace de guillemets → `serde_json` (échappement complet).
5. Tracking absent en mode TUI interactif (status server mentait hors --bot).
6. Écart de périmètre du Codeur sprint 2 : « fix » non justifié de `cfg_aliases!` dans `tom-quinn/build.rs` + `patches/netwatch` → **reverté** (interdiction de toucher le fork QUIC).

## Mesure (2026-07-16, flotte en 72) — LA RÉPONSE au #26

Snapshot `paths_by_peer` (iPad `:9091`, AppleTV `:9091`, NAS `:8085`) après ~2 min de convergence :

| Vue depuis | Vers | Path | RTT |
|---|---|---|---|
| iPad (WiFi) | AppleTV | **DIRECT** `192.168.0.76:49796` | 11 ms |
| AppleTV | iPad (WiFi) | **DIRECT** `192.168.0.23:59450` | 11 ms |
| iPad | Mac | DIRECT `192.168.0.82` | 12 ms |
| iPad | NAS | DIRECT (hairpin `82.67.95.8`) | 6 ms |
| AppleTV | Mac | DIRECT `192.168.0.82` | 9 ms |
| NAS | iPad / AppleTV | DIRECT (via `192.168.0.254`, NAT loopback box) | 5-10 ms |

**Zéro RELAY dans le snapshot. Le WiFi↔WiFi (iPad↔AppleTV) est DIRECT dans les deux sens.**
Conclusion : le « 100% RELAY » du matin était essentiellement le mensonge d'affichage (singleton) ;
l'hypothèse « pare-feu Freebox bloque le direct » est infirmée sur ce réseau à cet instant.
Reste à confirmer la STABILITÉ dans la durée (flapping DIRECT→UNKNOWN historique) — le collecteur
reçoit désormais un événement par transition, par pair, avec adresse : mesurable.

## Trou découvert et corrigé dans la foulée : watcher absent côté sortant

Premier snapshot : le Mac (qui *dial* ses pairs) avait une map **vide** alors qu'iPad/ATV le
voyaient DIRECT. Cause : `spawn_path_watcher` n'était posé que dans `accept()` (connexions
entrantes, `protocol.rs:263`) — jamais dans `ConnectionPool::get_or_connect` (sortantes).
Fix : le pool porte le même `path_event_tx` et pose le watcher sur toute connexion sortante
fraîche (`connection.rs`). Sans ça, la vue par pair est structurellement asymétrique.

Côté NAS, les cibles `RUST_LOG` trace restent actives dans l'unité systemd pour l'analyse fine.

## Accroc process consigné

Pendant la gate (cargo test --workspace), des nœuds de test ont bindé sur le LAN et ont été
découverts par la flotte (connexions DIRECT réelles du NAS vers `192.168.0.82:xxxxx`), et une
entrée DHT périmée `ipv6-check` (test manuel #32) est re-découverte en boucle. Chantier
d'herméticité des tests à ouvrir : bind loopback + découverte OFF en profil test.
