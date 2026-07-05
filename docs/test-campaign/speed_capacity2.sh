#!/usr/bin/env bash
# Test vitesse/capacité LEAN — iPad + Apple TV (foreground) via spd2 (port 9811).
# Timeout court (45s) + ABANDON anticipé du device au 1er timeout (pas de grind).
# Cap à 32 Mo (l'envoi 64 Mo stalle le nœud émetteur — bug séparé à traiter).
set -u
cd /Users/malik/Documents/tom-protocol
OUT=docs/test-campaign/speed_capacity2.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/chaos
CLOG="$DIR/spd2.log"; PORT=9811
: > "$OUT"; ts() { date "+%H:%M:%S"; }; log() { echo "$*" | tee -a "$OUT"; }
echos() { local n; n=$(grep -c "recu 5/5" "$CLOG" 2>/dev/null); echo "${n:-0}"; }
fmt() { local n=$1; [ "$n" -ge 1000000 ] && echo "$((n/1000000))Mo" || echo "$((n/1000))Ko"; }

declare -a NAMES=(iPad AppleTV)
declare -a NIDS=(
  10ca19aa3739bc283dfe6453e988a3b977476e2c70073747368cbec98cb3650b
  5ed8638c9262f90206add73dc398aa5b6b9057020e66bd805237b42321d0bb3c
)
SIZES=(20000 100000 1000000 5000000 16000000 32000000)

log "═══ VITESSE + CAPACITÉ (lean) — build 17 — $(ts) ═══"
log "$(printf '%-9s | %8s | %10s | %10s' 'appareil' 'taille' 'round-trip' 'débit')"
log "$(printf '%.0s─' {1..46})"
for i in "${!NAMES[@]}"; do
  NAME="${NAMES[$i]}"; NID="${NIDS[$i]}"
  for SZ in "${SIZES[@]}"; do
    B=$(echos)
    T0=$(python3 -c "import time;print(time.time())")
    curl -s -m 60 -X POST "http://127.0.0.1:$PORT/send?to=$NID&size=$SZ" >/dev/null 2>&1
    RT=""
    for j in $(seq 1 45); do
      sleep 1
      [ "$(echos)" -gt "$B" ] && { RT=$(python3 -c "import time;print(round(time.time()-$T0,1))"); break; }
    done
    if [ -z "$RT" ]; then
      log "$(printf '%-9s | %8s | %10s | %10s' "$NAME" "$(fmt $SZ)" 'TIMEOUT' '❌')"
      log "  ↳ $NAME injoignable à cette taille — abandon (probable passage en fond)"
      break
    else
      DEBIT=$(python3 -c "rt=$RT;print(f'{($SZ/1e6)/rt:.1f} Mo/s' if rt>0 and $SZ>=1000000 else '—')")
      log "$(printf '%-9s | %8s | %9ss | %10s' "$NAME" "$(fmt $SZ)" "$RT" "$DEBIT")"
    fi
  done
  log "$(printf '%.0s─' {1..46})"
done
log "Fin — $(ts)"
