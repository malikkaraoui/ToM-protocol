# Preuve Cross-Machine — 27 mars 2026

**Statut** : VERROUILLÉE
**Commit de référence** : `0126105` (dual-stack relay bind + bot-ping fix)
**Machines** : MacBook Pro M1 (Mac) ↔ Freebox NAS Debian ARM64 (NAS)

---

## Contexte

Le scénario cross-machine était bloqué par deux bugs :

1. **Relay embarqué publiait `127.0.0.1`** — adresse inaccessible depuis une autre machine
2. **`--bot-ping` ciblait un peer anonyme `n0`** — pas un vrai nœud ToM

Après correction (commit `0126105`), validation complète Mac ↔ NAS.

---

## Avant fix (pré-27 mars)

| Aspect | État |
|--------|------|
| Discovery gossip | Partiel — GossipNeighborUp intermittent |
| Relay publication | `http://127.0.0.1:PORT` — **inaccessible cross-machine** |
| Bot-ping target | Peer `n0` anonyme (4f1e897c…) — **pas un nœud ToM** |
| Messages échangés | **0** message utile entre nœuds ToM |
| Connexion directe | Blackout 2min, path instability |

## Après fix (27 mars 2026)

| Aspect | État |
|--------|------|
| Discovery gossip | **OK** — GossipNeighborUp cross-machine |
| Relay publication | `http://192.168.0.83:33471/` — **IP LAN NAS, accessible** |
| Bot-ping target | `nas-publisher`, `mac-obs1` — **peers nommés** |
| Messages échangés | **~20K en 60s** (obs1=10214, obs2=4384, NAS=5842) |
| Connexion directe | **PathEvent::Direct, RTT 45ms** — hole punch réussi |

---

## Fixes appliqués

### Fix 1 — Dual-stack relay bind + IP detection

**Fichier** : `crates/tom-protocol/src/runtime/embedded_relay.rs`

- Bind changé de `127.0.0.1:0` → `[::]:0` (dual-stack, toutes interfaces)
- Auto-détection IP outbound via UDP socket trick (connect `8.8.8.8:80` sans envoyer de trafic)
- Résultat : relay publie `http://192.168.0.83:PORT` au lieu de `http://127.0.0.1:PORT`

### Fix 2 — Bot-ping named peer targeting

**Fichier** : `crates/tom-tui/src/main.rs`

- Avant : `--bot-ping` ciblait le premier `PeerDiscovered`, souvent un peer `n0` anonyme (Pkarr/DNS)
- Après : filtre les `PeerDiscovered` → ne sélectionne que les peers avec `username` non vide
- Logique extraite dans `select_ping_target()`, couverte par 5 tests de régression

---

## Régression bot-ping — Tests automatisés

5 tests unitaires dans `crates/tom-tui/src/main.rs` :

| Test | Vérifie |
|------|---------|
| `bot_ping_skips_anonymous_peer` | Peer sans username → **rejeté** |
| `bot_ping_selects_named_peer` | Peer avec username → **sélectionné** |
| `bot_ping_target_locked_after_first` | Cible verrouillée → peers suivants ignorés |
| `bot_ping_anonymous_then_named` | Anonyme puis nommé → **nommé gagne** |
| `bot_ping_ignores_non_discovery_events` | Events non-discovery → ignorés |

```
$ cargo test -p tom-tui
running 14 tests
test tests::bot_ping_selects_named_peer ... ok
test tests::bot_ping_skips_anonymous_peer ... ok
test tests::bot_ping_target_locked_after_first ... ok
test tests::bot_ping_anonymous_then_named ... ok
test tests::bot_ping_ignores_non_discovery_events ... ok
...
test result: ok. 14 passed; 0 failed
```

---

## Campagne cross-machine — Run de référence

### Setup

- **NAS** : `tom-relay --dev` (port 3340) + `tom-chat --username nas-publisher --bot --bot-ping 3 --embedded-relay --embedded-relay-publish`
- **Mac obs1** : `tom-chat --username mac-obs1 --bot --bot-ping 3 --relay-discovery <NAS_NODE_ID>`
- **Mac obs2** : `tom-chat --username mac-obs2 --bot --bot-ping 3 --relay-discovery <NAS_NODE_ID>`
- **Tunnel SSH** : `ssh -f -N -L 3340:127.0.0.1:3340 -p 2222 root@82.67.95.8`
- **Durée** : 60 secondes

### Résultats

| Run | Peer ciblé (obs1) | Peer ciblé (obs2) | Messages OK | Path Direct | RTT | Verdict |
|-----|--------------------|--------------------|-------------|-------------|-----|---------|
| 1 | nas-publisher | mac-obs1 | ~20K total | oui | 45ms | **PASS** |

### Extraits de logs clés

#### Preuve A — Bot-ping cible un peer nommé

```
[bot] Ping target set: <nas-publisher-id> "nas-publisher"
[bot] Ping target set: <mac-obs1-id> "mac-obs1"
```

Les peers anonymes `n0` (découverts via Pkarr/DNS) sont **ignorés**.

#### Preuve B — Discovery cross-machine complète

```
[event] Peer discovered: <id> "nas-publisher" (via Gossip)
[event] Peer discovered: <id> "mac-obs1" (via Gossip)
[event] Peer discovered: <id> "mac-obs2" (via Gossip)
```

Les 3 nœuds ToM se découvrent mutuellement via gossip.

#### Preuve C — Relay embarqué publie une IP LAN utile

```
Embedded relay started: http://192.168.0.83:33471/
[event] Relay discovered: <nas-id> → http://192.168.0.83:33471/
[event] Transport relay added: http://192.168.0.83:33471/
```

Plus de `127.0.0.1`.

#### Preuve D — Upgrade en connexion directe QUIC

```
[event] Path changed: Direct
```

RTT observé : **45ms** (hole punch réussi via MagicSock).

#### Preuve E — Messages réellement échangés

```
obs1: 10214 messages
obs2:  4384 messages
NAS:   5842 messages
Total: ~20K en 60 secondes
```

---

## Comparaison campagne 20 mars (V1) → 27 mars

| Aspect | 20 mars V1 | 27 mars | Delta |
|--------|-----------|---------|-------|
| Discovery | Partiel | **Complet** | Fix NeighborUp + relay publish |
| Relay embarqué | `127.0.0.1` | `192.168.0.83` | Fix dual-stack + IP detect |
| Bot-ping target | Peer `n0` anonyme | **Peer nommé** | Fix username filter |
| Messages | FAIL (burst 30%, endurance 53%) | **~20K en 60s** | Fix auto-start + bot-ping |
| Connexion directe | Blackout 2min | **Direct 45ms** | Hole punch stable |

---

## Limites connues

1. **IP publiée = LAN** (`192.168.0.83`) — pas routable depuis Internet sans tunnel SSH
2. **IPv6 résoudrait ce problème** — adresse globale routable directement (planifié Bloc B')
3. **Tunnel SSH nécessaire** pour le relay bootstrap (IPv4 LAN NAS cassé)
4. **1 run de référence** — pas 5-10, mais le run est convaincant (~20K messages, 0 erreurs)

---

## Critères d'acceptation — Verdict

| # | Critère | Résultat |
|---|---------|----------|
| 1 | `--bot-ping` cible un peer nommé, pas un `n0` anonyme | **PASS** — 5 tests de régression + preuve terrain |
| 2 | Au moins un run cross-machine avec messages réellement échangés | **PASS** — ~20K messages en 60s |
| 3 | Au moins un run montre `Path changed: Direct` | **PASS** — Direct QUIC, RTT 45ms |
| 4 | Tout documenté dans un artefact partageable | **PASS** — ce document |

**Conclusion : preuve cross-machine VERROUILLÉE.**

---

## Prochaines étapes (hors scope de cette preuve)

- Campagne additionnelle 5-10 runs si besoin d'audit
- ADR M2 (réduction à 3 commandes)
- Bloc B' IPv6-first
