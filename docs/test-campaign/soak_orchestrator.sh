#!/usr/bin/env bash
# Orchestrateur autonome de tests d'endurance ToM (carte blanche 5h+).
# Enchaîne en boucle : montée volume, montée taille (jusqu'au plafond), et
# scénario destinataire hors-ligne (survie backup + livraison différée).
# Tout est horodaté dans soak_results.log. Conçu pour tourner seul en fond.
set -u
cd /Users/malik/Documents/tom-protocol

BIN=./target/release/tom-chat
CAMPAIGN_LOG=docs/test-campaign/campaign.log
OUT=docs/test-campaign/soak_results.log
BOTDIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/soak_bots
mkdir -p "$BOTDIR"

IPAD_DEV=1DC13ED3-A246-525E-8B71-7F62AA950A4A
IPAD_BUNDLE=malik.karaoui.TomNode-iOS
IPAD_NODE=10ca19aa3739bc283dfe6453e988a3b977476e2c70073747368cbec98cb3650b

ts() { date "+%Y-%m-%d %H:%M:%S"; }
log() { echo "[$(ts)] $*" >> "$OUT"; }

# Compte, dans la fenêtre récente du log campagne, les receptions par taille
# et le nombre de "too large" (doit rester 0 = fix tient).
snapshot() {
  local secs="${1:-120}"
  python3 - "$secs" <<'PY' >> "$OUT"
import sys, json, time
from datetime import datetime
from collections import defaultdict
secs = int(sys.argv[1]); now = time.time()
recv = defaultdict(int); sent = defaultdict(int); big = 0; devs = set()
try:
    with open("docs/test-campaign/campaign.log", "rb") as f:
        f.seek(0, 2); size = f.tell(); f.seek(max(0, size - 8_000_000))
        data = f.read().decode("utf-8", "ignore")
except Exception:
    data = ""
for line in data.splitlines():
    if not line.startswith("{"): continue
    try: j = json.loads(line)
    except Exception: continue
    try:
        t = datetime.fromisoformat(j["_recv_ts"]).timestamp()
    except Exception:
        t = now
    if now - t > secs: continue
    devs.add(j.get("appareil"))
    d = j.get("detail", "")
    if "too large" in d: big += 1
    if "📦" in d:
        try: recv[d.split("📦 ")[1].split(" #")[0]] += 1
        except Exception: pass
    if "CAMP↑" in d:
        try: sent[d.split("CAMP↑ ")[1].split(" #")[0]] += 1
        except Exception: pass
order = ["1 Ko","10 Ko","50 Ko","100 Ko","150 Ko","250 Ko"]
rline = " ".join(f"{k}={recv.get(k,0)}" for k in order if recv.get(k,0))
print(f"    fenêtre {secs}s | nœuds actifs: {sorted(x for x in devs if x)} | too_large={big}")
print(f"    reçus:   {rline}")
PY
}

# Combien de messages 'facteur' l'iPad a-t-il reçus (par son marqueur unique) ?
count_facteur_delivered() {
  local tag="$1"
  python3 - "$tag" <<'PY'
import sys
tag = sys.argv[1]
n = 0
try:
    with open("docs/test-campaign/campaign.log", "rb") as f:
        f.seek(0, 2); size = f.tell(); f.seek(max(0, size - 12_000_000))
        data = f.read().decode("utf-8", "ignore")
except Exception:
    data = ""
for line in data.splitlines():
    if '"appareil": "ipad"' in line and tag in line and "MSG from" in line:
        n += 1
print(n)
PY
}

ipad_pid() {
  xcrun devicectl device info processes --device "$IPAD_DEV" 2>/dev/null \
    | grep "TomNode.app/TomNode$" | awk '{print $1}' | head -1
}

log "════════════════════════════════════════════════════════════"
log "DÉMARRAGE ORCHESTRATEUR ENDURANCE — flotte build 9 (fix DoS)"
log "════════════════════════════════════════════════════════════"

CYCLE=0
while true; do
  CYCLE=$((CYCLE+1))
  log ""
  log "┌─ CYCLE $CYCLE ───────────────────────────────────────────"

  # ── PHASE A : baseline (la campagne des apps tourne déjà) ────────────
  log "│ PHASE A — baseline campagne apps (observation 3 min)"
  sleep 180
  snapshot 180

  # ── PHASE B : STRESS VOLUME — 4 bots locaux matraquent la flotte ─────
  log "│ PHASE B — stress volume : 4 bots locaux blast 50 Ko (5 min)"
  for i in 1 2 3 4; do
    "$BIN" --bot --campaign --username "blast$i" --node-label "blast$i" \
      > "$BOTDIR/blast$i.log" 2>&1 &
    echo $! > "$BOTDIR/blast$i.pid"
  done
  sleep 300
  snapshot 180
  for i in 1 2 3 4; do kill "$(cat "$BOTDIR/blast$i.pid" 2>/dev/null)" 2>/dev/null; done
  log "│   bots volume arrêtés"
  sleep 20

  # ── PHASE C : MONTÉE TAILLE jusqu'au plafond transport ───────────────
  log "│ PHASE C — montée taille vers Mac (paliers jusqu'au plafond ~256 Ko)"
  MAC_NODE=$(python3 - <<'PY'
import json
try:
    with open("docs/test-campaign/campaign.log","rb") as f:
        f.seek(0,2); s=f.tell(); f.seek(max(0,s-2_000_000)); data=f.read().decode("utf-8","ignore")
    for line in reversed(data.splitlines()):
        if '"node": "Mac"' in line:
            print(json.loads(line)["node_id"]); break
except Exception: pass
PY
)
  if [ -n "${MAC_NODE:-}" ] && [ "${#MAC_NODE}" -ge 60 ]; then
    "$BIN" --bot --username rampe --node-label rampe \
      --size-ramp "$MAC_NODE" \
      --ramp-sizes "100000,150000,200000,250000,300000,400000,600000" \
      > "$BOTDIR/rampe.log" 2>&1 &
    RP=$!
    sleep 120
    kill "$RP" 2>/dev/null
    log "│   résultat rampe taille :"
    grep -E "size_ramp_ok|size_ramp_echec|erreur_protocole" "$BOTDIR/rampe.log" 2>/dev/null \
      | python3 -c "
import sys, json
for l in sys.stdin:
    try: j=json.loads(l)
    except: continue
    print('│     '+j.get('event','?')+' '+j.get('detail','')[:70])" >> "$OUT" 2>/dev/null
  else
    log "│   (Mac node_id introuvable — rampe sautée ce cycle)"
  fi
  sleep 20

  # ── PHASE D : DESTINATAIRE HORS-LIGNE (survie + livraison différée) ───
  log "│ PHASE D — destinataire hors-ligne (iPad)"
  TAG="OFFLINE-C${CYCLE}-$(date +%H%M%S)"
  PID=$(ipad_pid)
  if [ -n "$PID" ]; then
    xcrun devicectl device process terminate --device "$IPAD_DEV" --pid "$PID" >/dev/null 2>&1
    log "│   T0 : iPad tué (pid $PID) — destinataire hors-ligne"
    sleep 5
    # Facteur : 15 messages numérotés vers l'iPad MORT (doivent être backupés)
    "$BIN" --bot --username facteur --node-label facteur \
      --size-ramp "$IPAD_NODE" \
      --ramp-sizes "$(python3 -c "print(','.join(str(2000+i) for i in range(15)))")" \
      > "$BOTDIR/facteur.log" 2>&1 &
    FP=$!
    log "│   facteur envoie 15 messages vers l'iPad hors-ligne (marqueur inutile — compte via #)"
    sleep 90
    SENT=$(grep -c "size_ramp_ok" "$BOTDIR/facteur.log" 2>/dev/null || echo 0)
    log "│   messages émis vers iPad mort : $SENT (attendus backupés par relais/expéditeurs)"
    # iPad reste mort encore 3 min (test de survie du message)
    log "│   iPad maintenu hors-ligne 3 min (survie du message)"
    sleep 180
    kill "$FP" 2>/dev/null
    # Résurrection
    RECV_BEFORE=$(count_facteur_delivered "facteur")
    xcrun devicectl device process launch --device "$IPAD_DEV" "$IPAD_BUNDLE" >/dev/null 2>&1
    log "│   T+ : iPad RESSUSCITÉ — mesure de la livraison différée (5 min)"
    sleep 300
    snapshot 300
    log "│   (analyse fine de la reprise iPad à faire par Claude au réveil)"
  else
    log "│   iPad déjà absent — scénario hors-ligne sauté ce cycle"
    sleep 60
  fi

  log "└─ FIN CYCLE $CYCLE ──────────────────────────────────────"
done
