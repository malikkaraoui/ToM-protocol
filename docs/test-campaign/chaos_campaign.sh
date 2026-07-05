#!/usr/bin/env bash
# Campagne CHAOS longue durée : gros fichiers (jusqu'à 64 Mo), tailles et cibles
# aléatoires, bursts, churn de nœuds (kill/revive), pilotée par l'API de contrôle.
# Nœuds de contrôle locaux qui matraquent la vraie flotte + s'auto-perturbent.
set -u
cd /Users/malik/Documents/tom-protocol

BIN=./target/release/tom-chat
OUT=docs/test-campaign/chaos_results.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/chaos
mkdir -p "$DIR"; : > "$OUT"

# PRNG déterministe-ish basé sur l'horloge (Math.random interdit ailleurs, ici bash)
rnd() { echo $(( ( $(date +%s%N) / 1000 + $$ ) % $1 )); }

ts() { date "+%H:%M:%S"; }
log() { echo "[$(ts)] $*" | tee -a "$OUT"; }

# 3 nœuds de contrôle (identités stables) sur ports 9801-9803
for n in 1 2 3; do
  "$BIN" --bot --username chaos$n --node-label chaos$n --key-path "$DIR/c$n.key" \
    --control-port 980$n --log-udp 192.168.0.255:9999 > "$DIR/c$n.log" 2>&1 &
done
sleep 5
# attendre qu'au moins un ait des pairs
for i in $(seq 1 24); do
  sleep 5
  NP=$(curl -s -m3 "http://127.0.0.1:9801/peers" 2>/dev/null | python3 -c "import sys,json;print(len(json.load(sys.stdin)['pairs']))" 2>/dev/null || echo 0)
  [ "${NP:-0}" -ge 1 ] && break
done
log "═══════════════════════════════════════════════════════════"
log "CAMPAGNE CHAOS DÉMARRÉE — 3 nœuds de contrôle, gros fichiers ≤ 64 Mo"
log "═══════════════════════════════════════════════════════════"

# palette de tailles, du petit au TRÈS gros
SIZES=(5000 50000 200000 500000 2000000 5000000 16000000 32000000 64000000)

# compte les CTRL:<size>: reçus dans le log UDP (fenêtre récente)
recu() { python3 - "$1" <<'PY'
import sys,json,time
from datetime import datetime
size=sys.argv[1]; now=time.time(); n=0
try:
    with open("docs/test-campaign/campaign.log","rb") as f:
        f.seek(0,2); s=f.tell(); f.seek(max(0,s-6_000_000)); data=f.read().decode("utf-8","ignore")
except Exception: data=""
for line in data.splitlines():
    if f"CTRL:{size}:" in line and "MSG from" in line:
        try:
            j=json.loads(line)
            if now-datetime.fromisoformat(j["_recv_ts"]).timestamp()<50: n+=1
        except Exception: pass
print(n)
PY
}

CYCLE=0
while true; do
  CYCLE=$((CYCLE+1))
  log ""
  log "┌─ CHAOS CYCLE $CYCLE ─────────────────────────────────────"

  # 1) rafale aléatoire : 3-6 salves de tailles aléatoires via /sendall
  SALVES=$(( $(rnd 4) + 3 ))
  for s in $(seq 1 $SALVES); do
    PORT=980$(( $(rnd 3) + 1 ))
    SZ=${SIZES[$(rnd ${#SIZES[@]})]}
    CNT=$(( $(rnd 3) + 1 ))
    B=$(recu "$SZ")
    R=$(curl -s -m 120 -X POST "http://127.0.0.1:$PORT/sendall?size=$SZ&count=$CNT" 2>/dev/null)
    EN=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin).get('envoyes',0))" 2>/dev/null || echo "?")
    sleep $(( SZ / 4000000 + 6 ))   # plus c'est gros, plus on attend
    A=$(recu "$SZ")
    log "│ salve $s : taille=$SZ ×$CNT via c${PORT: -1} → envoyés=$EN reçus=$(( A - B ))"
  done

  # 2) CHURN : 1 fois sur 2, on tue+relance un nœud de contrôle au hasard
  if [ "$(rnd 2)" -eq 0 ]; then
    K=$(( $(rnd 3) + 1 ))
    log "│ 🌀 churn : kill+relance chaos$K"
    pkill -f "username chaos$K" 2>/dev/null
    sleep $(( $(rnd 8) + 3 ))
    "$BIN" --bot --username chaos$K --node-label chaos$K --key-path "$DIR/c$K.key" \
      --control-port 980$K --log-udp 192.168.0.255:9999 > "$DIR/c$K.log" 2>&1 &
  fi

  # 3) état de santé
  ALIVE=$(tail -c 400000 docs/test-campaign/campaign.log 2>/dev/null | python3 -c "
import sys,json,time
from datetime import datetime
now=time.time();s=set()
for l in sys.stdin:
    try: j=json.loads(l)
    except: continue
    try:
        if now-datetime.fromisoformat(j['_recv_ts']).timestamp()<60: s.add(j.get('appareil'))
    except: pass
print(sorted(x for x in s if x))" 2>/dev/null)
  BIG=$(tail -c 3000000 docs/test-campaign/campaign.log 2>/dev/null | grep -c "too large" || echo 0)
  log "└─ fin cycle $CYCLE | flotte active: $ALIVE | too_large=$BIG"
  sleep $(( $(rnd 10) + 5 ))
done
