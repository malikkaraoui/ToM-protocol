#!/usr/bin/env bash
# Test de survie iPad sous gros fichiers (build 13, fix décode-lazy).
# Mesure, à chaque étape : l'iPad reste-t-il VIVANT (continue de loguer) et
# renvoie-t-il l'écho "recu 5/5" (donc a traité le message sans mourir) ?
# Envoie depuis les nœuds de contrôle chaos (ports 9801-9803) déjà en place.
set -u
cd /Users/malik/Documents/tom-protocol
OUT=docs/test-campaign/ipad_survival.log
: > "$OUT"
ts() { date "+%H:%M:%S"; }
log() { echo "[$(ts)] $*" | tee -a "$OUT"; }

IPAD=10ca19aa3739bc283dfe6453e988a3b977476e2c70073747368cbec98cb3650b
CLOG=docs/test-campaign/campaign.log

# iPad vivant ? (a logué il y a < N secondes)
ipad_age() {
  tail -c 200000 "$CLOG" 2>/dev/null | python3 -c "
import sys,json,time
from datetime import datetime
now=time.time();b=None
for l in sys.stdin:
    try: j=json.loads(l)
    except: continue
    if j.get('appareil')!='ipad': continue
    try:
        a=now-datetime.fromisoformat(j['_recv_ts']).timestamp()
        if b is None or a<b: b=a
    except: pass
print(int(b) if b is not None else 9999)
" 2>/dev/null || echo 9999
}
# nb d'échos "recu 5/5" vus dans campaign.log récemment (l'iPad a répondu)
echos_recent() {
  tail -c 400000 "$CLOG" 2>/dev/null | python3 -c "
import sys,json,time
from datetime import datetime
now=time.time();n=0
for l in sys.stdin:
    try: j=json.loads(l)
    except: continue
    if 'recu 5/5' not in str(j.get('message','')): continue
    try:
        if now-datetime.fromisoformat(j['_recv_ts']).timestamp()<25: n+=1
    except: pass
print(n)
" 2>/dev/null || echo 0
}

log "╔════════════════════════════════════════════════════════╗"
log "║  TEST SURVIE iPad — build 13 (fix décode-lazy réception) ║"
log "╚════════════════════════════════════════════════════════╝"
log "iPad vivant au départ : âge=$(ipad_age)s"

survive_check() {  # $1 label
  local a; a=$(ipad_age)
  if [ "$a" -lt 45 ]; then log "   ✅ iPad VIVANT après $1 (âge=${a}s)"; return 0
  else log "   ❌ iPad MUET après $1 (âge=${a}s) — possiblement tué"; return 1; fi
}

# ── 1) fichiers croissants, un par un ──
log ""; log "── 1) Fichiers croissants (1 à la fois) ──"
for SZ in 5000000 16000000 32000000 64000000; do
  E0=$(echos_recent)
  curl -s -m 90 -X POST "http://127.0.0.1:9801/send?to=$IPAD&size=$SZ" >/dev/null 2>&1
  sleep $(( SZ / 8000000 + 8 ))
  E1=$(echos_recent)
  log "   $(printf '%9d' $SZ) octets → écho round-trip: $([ "$E1" -gt "$E0" ] && echo OUI || echo non)"
  survive_check "$(( SZ/1000000 ))Mo"
done

# ── 2) burst concurrent modéré (3×16 Mo simultanés) ──
log ""; log "── 2) Burst concurrent 3×16 Mo ──"
for p in 1 2 3; do curl -s -m 90 -X POST "http://127.0.0.1:980$p/send?to=$IPAD&size=16000000" >/dev/null 2>&1 & done
sleep 25
survive_check "burst 3×16Mo"

# ── 3) LE barrage brutal qui l'avait tué (64 + 2×32 + 3×16 = ~176 Mo) ──
log ""; log "── 3) Barrage brutal (~176 Mo concurrents — avait tué build 12) ──"
curl -s -m 120 -X POST "http://127.0.0.1:9801/send?to=$IPAD&size=64000000" >/dev/null 2>&1 &
curl -s -m 120 -X POST "http://127.0.0.1:9802/sendall?size=32000000&count=2" >/dev/null 2>&1 &
curl -s -m 120 -X POST "http://127.0.0.1:9803/sendall?size=16000000&count=3" >/dev/null 2>&1 &
log "   barrage lancé — surveillance survie 90s"
DEAD=0
for i in $(seq 1 9); do
  sleep 10
  a=$(ipad_age)
  log "   +$(( i*10 ))s : iPad âge=${a}s $([ "$a" -lt 45 ] && echo '(vivant)' || echo '(MUET)')"
  [ "$a" -ge 60 ] && DEAD=1
done
log ""
if [ "$DEAD" -eq 0 ]; then
  log "🎉 RÉSULTAT : iPad a SURVÉCU au barrage brutal (fix décode-lazy validé)"
else
  log "⚠️ RÉSULTAT : iPad muet sous barrage — la charge reste létale, creuser (CPU/mémoire résiduel)"
fi
log "╚═══════════════════ FIN TEST SURVIE ═══════════════════╝"
