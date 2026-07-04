#!/usr/bin/env bash
# Test matriciel complet (build 11) : 1) table de validation par taille de
# paquet, 2) scénario destinataire hors-ligne (backup multi-messages, timing
# de livraison différée, effacement). Nœuds locaux pilotés = timing précis.
set -u
cd /Users/malik/Documents/tom-protocol

BIN=./target/release/tom-chat
OUT=docs/test-campaign/matrix_results.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/matrix
mkdir -p "$DIR"
: > "$OUT"

ts() { date "+%H:%M:%S"; }
log() { echo "[$(ts)] $*" | tee -a "$OUT"; }

# Destinataire R (identité stable) + expéditeur S, ports de contrôle.
"$BIN" --bot --username R --node-label R --key-path "$DIR/R.key" \
  --control-port 9501 --log-udp 127.0.0.1:9998 > "$DIR/R.log" 2>&1 &
"$BIN" --bot --username S --node-label S --key-path "$DIR/S.key" \
  --control-port 9502 --log-udp 127.0.0.1:9998 > "$DIR/S.log" 2>&1 &
sleep 3
RID=$(grep -m1 '^  node ' "$DIR/R.log" | awk '{print $2}')

# attendre S<->R connectés
for i in $(seq 1 20); do
  sleep 3
  NP=$(curl -s -m3 "http://127.0.0.1:9502/peers" 2>/dev/null | python3 -c "import sys,json;print(len(json.load(sys.stdin)['pairs']))" 2>/dev/null || echo 0)
  [ "${NP:-0}" -ge 1 ] && break
done
log "S connecté (R=${RID:0:8})"

# compte les CTRL:<size>: reçus par R (dans R.log)
recu() { grep -c "CTRL:$1:" "$DIR/R.log" 2>/dev/null || echo 0; }

log "═══════════ 1) TABLE DE VALIDATION PAR TAILLE ═══════════"
log "taille | envoyé | reçu | verdict"
for SZ in 1000 20000 100000 200000 300000 1000000 3000000 10000000 30000000; do
  B=$(recu "$SZ")
  curl -s -m 40 -X POST "http://127.0.0.1:9502/send?to=$RID&size=$SZ" >/dev/null 2>&1
  sleep 8
  A=$(recu "$SZ")
  D=$((A - B))
  V=$([ "$D" -ge 1 ] && echo "✅ OK" || echo "❌ KO")
  log "$(printf '%9d' $SZ) |   1    |  $D   | $V"
done

log ""
log "═══════════ 2) DESTINATAIRE HORS-LIGNE + BACKUP ═══════════"
# baseline avant coupure
BASE=$(grep -c "CTRL:5000:" "$DIR/R.log" 2>/dev/null || echo 0)
RPID=$(pgrep -f "tom-chat --bot --username R")
log "R en ligne (pid $RPID). Coupure imminente."
kill "$RPID" 2>/dev/null
T_OFF=$(date +%s)
log "T0 = $(ts) : R HORS-LIGNE"

# S envoie 8 messages (marqueur taille 5000) vers R mort
for k in $(seq 1 8); do
  curl -s -m5 -X POST "http://127.0.0.1:9502/send?to=$RID&size=5000" >/dev/null 2>&1
  sleep 1
done
log "8 messages envoyés vers R hors-ligne (doivent être backupés)"

# R reste mort 90s (survie du backup)
log "R maintenu hors-ligne 90s..."
sleep 90

# résurrection + chronométrage de la livraison différée
"$BIN" --bot --username R --node-label R --key-path "$DIR/R.key" \
  --control-port 9501 --log-udp 127.0.0.1:9998 > "$DIR/R2.log" 2>&1 &
T_ON=$(date +%s)
log "T+ = $(ts) : R RESSUSCITÉ — chronométrage de la livraison différée"

# poll : combien de messages backupés livrés + à quel instant
DELIVERED=0
for i in $(seq 1 60); do
  sleep 3
  N=$(grep -c "CTRL:5000:" "$DIR/R2.log" 2>/dev/null || echo 0)
  if [ "$N" -gt "$DELIVERED" ]; then
    DELIVERED=$N
    log "  +$(( $(date +%s) - T_ON ))s : $DELIVERED/8 messages backupés livrés"
  fi
  [ "$DELIVERED" -ge 8 ] && break
done
log "RÉSULTAT hors-ligne : $DELIVERED/8 messages backupés livrés après $(( $(date +%s) - T_ON ))s"

# effacement : le backup est-il purgé côté S après confirmation ?
sleep 10
BK=$(curl -s -m3 "http://127.0.0.1:9502/metrics" 2>/dev/null)
log "métriques S après livraison : $BK"

log "═══════════ FIN ═══════════"
