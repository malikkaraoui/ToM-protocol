#!/usr/bin/env bash
# Campagne autonome "gros paquets" pilotée par l'API de contrôle.
# Un nœud de contrôle Mac (port 9400) envoie des paquets de taille croissante
# à TOUS ses pairs (/sendall) et relève la livraison via les status servers.
# Objectif : valider le chunking (300 Ko → 10 Mo) sur la vraie flotte build 10.
set -u
cd /Users/malik/Documents/tom-protocol

BIN=./target/release/tom-chat
OUT=docs/test-campaign/soak_results.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/bigpacket
mkdir -p "$DIR"
CTRL=9400

ts() { date "+%Y-%m-%d %H:%M:%S"; }
log() { echo "[$(ts)] $*" >> "$OUT"; }

CAMPAIGN_LOG=docs/test-campaign/campaign.log

# Compte, dans le log UDP, les messages CTRL:<size>: REÇUS (marqueur unique de
# nos envois de contrôle) dans la fenêtre récente — robuste, sans IP devinée.
recus_ctrl() {  # $1 = taille exacte en octets
  local size="$1"
  python3 - "$size" <<'PY'
import sys, json, time
from datetime import datetime
size = sys.argv[1]; now = time.time(); n = 0; devs = set()
try:
    with open("docs/test-campaign/campaign.log", "rb") as f:
        f.seek(0, 2); sz = f.tell(); f.seek(max(0, sz - 4_000_000))
        data = f.read().decode("utf-8", "ignore")
except Exception:
    data = ""
for line in data.splitlines():
    if f"CTRL:{size}:" not in line or "MSG from" not in line:
        continue
    try:
        j = json.loads(line)
        if now - datetime.fromisoformat(j["_recv_ts"]).timestamp() < 40:
            n += 1; devs.add(j.get("appareil"))
    except Exception:
        pass
print(f"{n} ({','.join(sorted(x for x in devs if x))})")
PY
}

log "════════════════════════════════════════════════════════════"
log "CAMPAGNE GROS PAQUETS (chunking) — flotte build 10"
log "════════════════════════════════════════════════════════════"

# Nœud de contrôle dédié
"$BIN" --bot --username ctrl --node-label ctrl --key-path "$DIR/ctrl.key" \
  --control-port $CTRL --log-udp 192.168.0.255:9999 > "$DIR/ctrl.log" 2>&1 &
CTRLPID=$!
log "nœud de contrôle démarré (port $CTRL, pid $CTRLPID)"

# Attente de connexion à la flotte
for i in $(seq 1 24); do
  sleep 5
  NP=$(curl -s -m 3 "http://127.0.0.1:$CTRL/peers" 2>/dev/null | python3 -c "import sys,json;print(len(json.load(sys.stdin)['pairs']))" 2>/dev/null || echo 0)
  [ "${NP:-0}" -ge 1 ] && { log "connecté à $NP pair(s) après ~$((i*5))s"; break; }
done

SIZES=(20000 100000 300000 1000000 3000000 10000000)
NAMES=("20 Ko" "100 Ko" "300 Ko" "1 Mo" "3 Mo" "10 Mo")
COUNT=5

CYCLE=0
while true; do
  CYCLE=$((CYCLE+1))
  log ""
  log "┌─ CAMPAGNE CYCLE $CYCLE ──────────────────────────────────"
  NP=$(curl -s -m 3 "http://127.0.0.1:$CTRL/peers" 2>/dev/null | python3 -c "import sys,json;print(len(json.load(sys.stdin)['pairs']))" 2>/dev/null || echo 0)
  log "│ pairs connectés : ${NP:-0}"

  for idx in "${!SIZES[@]}"; do
    SZ=${SIZES[$idx]}; NM=${NAMES[$idx]}
    # /sendall : envoie COUNT messages de SZ octets à chaque pair connecté
    RESP=$(curl -s -m 40 -X POST "http://127.0.0.1:$CTRL/sendall?size=$SZ&count=$COUNT" 2>/dev/null)
    ENVOYES=$(echo "$RESP" | python3 -c "import sys,json;print(json.load(sys.stdin).get('envoyes',0))" 2>/dev/null || echo "?")
    # laisser le temps de livrer + réassembler (gros = plus long)
    sleep 15
    RECUS=$(recus_ctrl "$SZ")
    log "│ $NM : envoyés=$ENVOYES  reçus=$RECUS"
  done

  log "└─ FIN CYCLE $CYCLE ───────────────────────────────────────"
  sleep 20
done
