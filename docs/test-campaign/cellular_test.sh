#!/usr/bin/env bash
# Test gros paquets vers l'iPhone en cellulaire (5G/4G/3G).
# On ne peut pas piloter l'iPhone (CGNAT + pas d'API dans l'app) : on mesure
# la livraison par le ROUND-TRIP — le nœud de contrôle envoie, l'iPhone reçoit
# (réassemble les chunks) et écho en retour → on chronomètre l'aller-retour.
set -u
cd /Users/malik/Documents/tom-protocol
OUT=docs/test-campaign/cellular_results.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/cell
: > "$OUT"
ts() { date "+%H:%M:%S"; }
log() { echo "[$(ts)] $*" | tee -a "$OUT"; }

log "En attente de découverte de l'iPhone (5G) par le nœud de contrôle..."
IPH=""
for i in $(seq 1 120); do   # jusqu'à ~10 min
  sleep 5
  IPH=$(curl -s -m3 "http://127.0.0.1:9701/peers" 2>/dev/null | python3 -c "
import sys,json
try:
    for x in json.load(sys.stdin)['pairs']:
        if x.startswith('96c217ea'): print(x); break
except: pass" 2>/dev/null)
  [ -n "$IPH" ] && { log "✅ iPhone découvert après ~$((i*5))s : ${IPH:0:12}"; break; }
done
[ -z "$IPH" ] && { log "❌ iPhone jamais découvert en 10 min (5G trop faible ?)"; exit 0; }

# Round-trip : le nœud 'cell' logge les échos reçus ('recu 5/5') de l'iPhone.
echos() { grep -c "recu 5/5" "$DIR/cell.log" 2>/dev/null | head -1 || echo 0; }

log "═══ GROS PAQUETS VERS iPHONE CELLULAIRE (round-trip) ═══"
for SZ in 20000 100000 300000 1000000 3000000 10000000; do
  B=$(echos)
  T=$(date +%s)
  curl -s -m 90 -X POST "http://127.0.0.1:9701/send?to=$IPH&size=$SZ" >/dev/null 2>&1
  # attendre un écho de retour (jusqu'à 60s pour les gros sur 5G faible)
  DELIV="timeout"
  for j in $(seq 1 60); do
    sleep 1
    A=$(echos)
    if [ "${A:-0}" -gt "${B:-0}" ]; then DELIV="$(( $(date +%s) - T ))s"; break; fi
  done
  log "  $(printf '%9d' $SZ) octets → round-trip: $DELIV"
done
log "═══ FIN test cellulaire ═══"
