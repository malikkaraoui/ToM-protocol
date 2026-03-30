# Plan : Apple TV comme 3e noeud (Mac ↔ NAS ↔ Apple TV)

Date : 2026-03-30
Prerequis : fix self-relay (5c38cdf), CLI tom-tui opérable (e6f5f3d)

## Rôles

| Device | Rôle | Software | Réseau |
|--------|------|----------|--------|
| NAS | Publisher + self-relay (:3340) | tom-chat (ARM64 musl) | WAN 82.67.95.8:3340 |
| Mac | Observer + bot-ping | tom-chat (native) | LAN/WAN |
| Apple TV | Observer + auto-echo | app tvOS (FFI) | LAN WiFi |

## Commandes exactes

### NAS (SSH)

```bash
ssh -p 2222 root@82.67.95.8
systemctl stop tom-relay.service
TOM_RELAY_URL=http://0.0.0.0:3340 /root/tom-chat \
  --username nas-pub --bot --bot-ping 5 --self-relay --relay-ttl 60 \
  > /tmp/atv-nas.log 2>&1
# noter le Node ID affiché
```

### Mac

```bash
TOM_RELAY_URL=http://82.67.95.8:3340 target/release/tom-chat \
  --username mac-obs --bot --bot-ping 5 \
  --relay-discovery --relay-ttl 60 \
  --bootstrap <NAS_NODE_ID> \
  2>&1 | tee /tmp/atv-mac.log
```

### Apple TV (app TomNode)

1. Settings :
   - Relay URL : `http://82.67.95.8:3340`
   - Bootstrap Peer : `<NAS_NODE_ID>`
   - Auto-Echo : ON
   - DHT : ON, n0Discovery : ON
   - UDP Log Export : ON, host = `<MAC_LAN_IP>`, port = `9999`
2. Status → Start

### Mac (log receiver Apple TV, terminal séparé)

```bash
nc -u -l 9999 > /tmp/atv-appletv.log &
```

## Phase 1 : Smoke test (2 min)

### Critères PASS/FAIL

| # | Critère | Source | PASS si |
|---|---------|--------|---------|
| 1 | NAS embedded relay started | atv-nas.log | `Embedded relay started` sans FAILED |
| 2 | Mac découvre NAS | atv-mac.log | `Peer discovered: ... "nas-pub"` |
| 3 | Mac ↔ NAS messages | atv-mac.log | ≥ 5 `[bot] #` lignes |
| 4 | Apple TV runtime started | app LogView | `Runtime started` |
| 5 | Apple TV découvre NAS | app LogView ou UDP log | `PeerDiscovered` event |
| 6 | Apple TV reçoit un ping | app LogView | `MSG from ...` |
| 7 | Apple TV auto-echo reply | atv-nas.log ou atv-mac.log | message ECHO:/PONG: reçu |

**PASS = 7/7.** Si critère 4 ou 5 échoue, c'est un problème FFI/tvOS, pas le protocole.

## Phase 2 : Mini stress (10 min)

Laisser tourner 10 min après le smoke test. Mesurer :

| Métrique | Source | Attendu |
|----------|--------|---------|
| Messages NAS total | atv-nas.log `grep -c "^\[bot\] #"` | > 5000 |
| Messages Mac total | atv-mac.log `grep -c "^\[bot\] #"` | > 5000 |
| Echos Apple TV | app echoCount | > 0 (proportionnel aux pings reçus) |
| Erreurs | grep error/panic tous logs | 0 |
| Path upgrade | `Path changed: Direct` | ≥ 1 paire |

## Collecte des résultats

```bash
# Mac
grep -c "^\[bot\] #" /tmp/atv-mac.log
grep -c "ECHO:\|PONG:" /tmp/atv-mac.log

# NAS
ssh -p 2222 root@82.67.95.8 'grep -c "^\[bot\] #" /tmp/atv-nas.log'

# Apple TV (via UDP log)
grep -c "MSG from\|ECHO" /tmp/atv-appletv.log

# Erreurs
grep -ci "error\|panic\|FAILED" /tmp/atv-mac.log /tmp/atv-nas.log /tmp/atv-appletv.log
```

## Validation observée (run du 2026-03-30)

### Verdict

**Smoke test : PASS.**

Le trio **NAS ↔ Mac ↔ Apple TV** est monté correctement :

- Apple TV runtime started
- Apple TV discovery du NAS via gossip
- Apple TV discovery du Mac via announce
- Apple TV reçoit des messages du NAS
- Apple TV auto-echo renvoie des réponses visibles côté NAS
- aucune erreur détectée dans les logs consultés

### Signaux observés

#### Apple TV (UDP logs)

- `Runtime started`
- `Bootstrap peers: ["4e28f470"]`
- `PEER DISCOVERED: 4f1e897c...6a84 via Gossip`
- `PEER DISCOVERED: mac-obs (d4e9f612...5a64) via Announce`
- `Peers: 2 → 3`
- nombreux `MSG from 7a1900e6...`
- `ECHO #200 → 7a1900e6: ECHO:recu 5/5 malik (msg #190)`

#### Mac

- `Peer discovered: 7a1900e6…` (NAS)
- `Gossip neighbor up: 4f1e897c…`
- `Transport relay added: http://192.168.0.83:3340/`

#### NAS

- embedded relay démarré
- trafic bot continu
- réception d'`ECHO:` en provenance de l'Apple TV

### Métriques rapides du run

| Métrique | Valeur observée |
|----------|------------------|
| Apple TV `MSG from` | 1579 |
| Apple TV `ECHO #` | 25 |
| Apple TV erreurs | 0 |
| Mac `[bot] #` | 62 |
| Mac erreurs | 0 |
| NAS `[bot] #` | 1509 |
| NAS erreurs | 0 |

### Conclusion pratique

Le scénario **Apple TV comme 3e noeud** est validé au moins en mode smoke + trafic réel :

- la TV rejoint le réseau,
- découvre les autres pairs,
- échange effectivement des messages,
- et répond en auto-echo sans erreur visible.

Le prochain cran pertinent est un **mini stress 10 min** sur cette base, sans changer l'architecture.

## Cleanup

```bash
# Mac : Ctrl+C tom-chat, kill nc
# NAS :
ssh -p 2222 root@82.67.95.8 'kill $(pidof tom-chat); systemctl start tom-relay.service'
# Apple TV : Status → Stop
```

## Risques connus

- **Ce run utilisait un bootstrap manuel** : pour cette validation, le `bootstrapPeerId` pointait vers le NAS courant. Depuis le patch du 30 mars, l'app tvOS ne shippe plus de relay ni de bootstrap hardcodés par défaut ; il faut désormais les fournir explicitement pour un scénario seedé, ou laisser vide pour tester l'amorçage organique.
- **bot-ping mono-target** : le NAS ne ping qu'un seul peer (Mac ou Apple TV, pas les deux). L'Apple TV reçoit des messages seulement si elle est le premier peer nommé découvert, ou si Mac lui envoie des pings. Contournement : Mac envoie des pings, Apple TV echo.
- **500ms polling tvOS** : les messages apparaissent avec un léger délai dans l'app (polling FFI, pas push).
- **Anti-sleep** : l'app tvOS a un silent audio loop pour empêcher la mise en veille. Vérifié fonctionnel.

## Prérequis avant exécution

1. Rebuild FFI device : `cd apps/tom-node-tvos && make ffi-device`
2. Deploy sur Apple TV via Xcode (signing requis)
3. Cross-compile tom-chat pour NAS : `cargo zigbuild -p tom-tui --target aarch64-unknown-linux-musl --release`
4. Build tom-chat Mac : `cargo build -p tom-tui --release`
