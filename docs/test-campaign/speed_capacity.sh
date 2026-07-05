#!/usr/bin/env bash
# Test VITESSE + CAPACITÉ par appareil (build 17).
# Depuis le nœud de contrôle c1 (port 9801) : envoie une taille croissante à
# chaque device, mesure le round-trip (le device réassemble + renvoie l'écho
# "recu 5/5"). Round-trip ≈ temps de transfert + écho → débit. Livraison = écho reçu.
set -u
cd /Users/malik/Documents/tom-protocol
OUT=docs/test-campaign/speed_capacity.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/chaos
CLOG="$DIR/spd.log"
: > "$OUT"
ts() { date "+%H:%M:%S"; }
log() { echo "$*" | tee -a "$OUT"; }

PORT=9801
# node_ids (préfixes complets connus)
# node_ids COMPLETS codés en dur — le serveur HTTP du nœud de contrôle se bloque
# pendant les gros transferts, donc on n'appelle PAS /peers en cours de test.
declare -a NAMES=(iPad AppleTV iPhone)
declare -a NIDS=(
  10ca19aa3739bc283dfe6453e988a3b977476e2c70073747368cbec98cb3650b
  5ed8638c9262f90206add73dc398aa5b6b9057020e66bd805237b42321d0bb3c
  96c217ea93eee66a3f2b28013497b46ff52de6ee9e31331445869137b2b3c20d
)
resolve() { echo "$1"; }  # identité : node_id déjà complet
echos() { grep -c "recu 5/5" "$CLOG" 2>/dev/null | head -1; }

SIZES=(20000 100000 1000000 5000000 16000000 32000000 64000000)
fmt() { local n=$1; if [ "$n" -ge 1000000 ]; then echo "$((n/1000000))Mo"; else echo "$((n/1000))Ko"; fi; }

log "╔════════════════════════════════════════════════════════════╗"
log "║  TEST VITESSE + CAPACITÉ — build 17 — $(ts)                 ║"
log "╚════════════════════════════════════════════════════════════╝"
log ""
log "$(printf '%-9s | %8s | %10s | %12s' 'appareil' 'taille' 'round-trip' 'débit')"
log "$(printf '%.0s─' {1..52})"

for i in "${!NAMES[@]}"; do
  NAME="${NAMES[$i]}"
  NID=$(resolve "${NIDS[$i]}")
  if [ -z "$NID" ]; then log "$(printf '%-9s | %8s' "$NAME" 'ABSENT (pas vu par c1)')"; continue; fi
  for SZ in "${SIZES[@]}"; do
    B=$(echos); B=${B:-0}
    T0=$(python3 -c "import time;print(time.time())")
    curl -s -m 120 -X POST "http://127.0.0.1:$PORT/send?to=$NID&size=$SZ" >/dev/null 2>&1
    RT=""; DEBIT="—"
    for j in $(seq 1 120); do
      sleep 1
      A=$(echos); A=${A:-0}
      if [ "$A" -gt "$B" ]; then
        RT=$(python3 -c "import time;print(round(time.time()-$T0,1))")
        # débit Mo/s (round-trip inclut aller gros + retour petit)
        DEBIT=$(python3 -c "rt=$RT; print(f'{($SZ/1e6)/rt:.1f} Mo/s' if rt>0 else '—')")
        break
      fi
    done
    if [ -z "$RT" ]; then
      log "$(printf '%-9s | %8s | %10s | %12s' "$NAME" "$(fmt $SZ)" 'TIMEOUT' '❌ non livré')"
    else
      log "$(printf '%-9s | %8s | %9ss | %12s' "$NAME" "$(fmt $SZ)" "$RT" "$DEBIT")"
    fi
  done
  log "$(printf '%.0s─' {1..52})"
done
log ""
log "Fin — $(ts)"
