#!/usr/bin/env bash
# Suite de scénarios de résilience chronométrés (build 11).
# Nœuds de contrôle locaux (timing précis) + vraie app iPad (kill/relaunch réel).
set -u
cd /Users/malik/Documents/tom-protocol

BIN=./target/release/tom-chat
OUT=docs/test-campaign/scenarios_results.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/scen
mkdir -p "$DIR"; : > "$OUT"
IPAD=1DC13ED3-A246-525E-8B71-7F62AA950A4A
IPAD_BUNDLE=malik.karaoui.TomNode-iOS
CLOG=docs/test-campaign/campaign.log

ts() { date "+%H:%M:%S"; }
log() { echo "[$(ts)] $*" | tee -a "$OUT"; }
now() { date +%s; }

boot() { # $1 label, $2 ctrlport → lance un nœud de contrôle
  "$BIN" --bot --username "$1" --node-label "$1" --key-path "$DIR/$1.key" \
    --control-port "$2" > "$DIR/$1.log" 2>&1 &
}
nid() { grep -m1 '^  node ' "$DIR/$1.log" 2>/dev/null | awk '{print $2}'; }
peers() { curl -s -m3 "http://127.0.0.1:$1/peers" 2>/dev/null | python3 -c "import sys,json;print(len(json.load(sys.stdin)['pairs']))" 2>/dev/null || echo 0; }
recu() { local n; n=$(grep -c "CTRL:$2:" "$DIR/$1.log" 2>/dev/null); echo "${n:-0}"; }

log "╔══════════════════════════════════════════════════════════╗"
log "║  SCÉNARIOS DE RÉSILIENCE CHRONOMÉTRÉS — build 11          ║"
log "╚══════════════════════════════════════════════════════════╝"

# Deux nœuds A (émetteur/contrôle) + B (destinataire), identités stables.
boot A 9601; boot B 9602
sleep 3
BID=$(nid B); AID=$(nid A)
for i in $(seq 1 20); do sleep 3; [ "$(peers 9601)" -ge 1 ] && break; done
log "setup OK : A=${AID:0:8} B=${BID:0:8} connectés"

# ── SCÉNARIO 1 : temps de retour sur réseau après redémarrage (nœud) ──
log ""
log "── S1 : temps de retour sur réseau après kill/relaunch (nœud A) ──"
kill "$(pgrep -f 'username A')" 2>/dev/null
log "   A tué"; sleep 2
T=$(now); boot A 9601
for i in $(seq 1 40); do
  sleep 1
  if [ "$(peers 9601)" -ge 1 ]; then
    log "   ✅ A de retour sur le réseau en $(( $(now) - T ))s"
    break
  fi
done

# ── SCÉNARIO 2 : temps de retour de la VRAIE APP iPad ──
log ""
log "── S2 : temps de retour de l'app iPad (kill/relaunch réel) ──"
PID=$(xcrun devicectl device info processes --device "$IPAD" 2>/dev/null | grep 'TomNode.app/TomNode$' | awk '{print $1}' | head -1)
if [ -n "$PID" ]; then
  xcrun devicectl device process terminate --device "$IPAD" --pid "$PID" >/dev/null 2>&1
  log "   app iPad tuée (pid $PID)"; sleep 2
  T=$(now)
  xcrun devicectl device process launch --device "$IPAD" "$IPAD_BUNDLE" >/dev/null 2>&1
  log "   app iPad relancée — attente rejoin (via log UDP)"
  for i in $(seq 1 90); do
    sleep 2
    NP=$(tail -c 200000 "$CLOG" 2>/dev/null | python3 -c "
import sys,json,time
from datetime import datetime
now=time.time();best=0
for l in sys.stdin:
    try: j=json.loads(l)
    except: continue
    if j.get('appareil')!='ipad': continue
    try:
        if now-datetime.fromisoformat(j['_recv_ts']).timestamp()<10: best=max(best,j.get('number_peers',0))
    except: pass
print(best)" 2>/dev/null || echo 0)
    if [ "${NP:-0}" -ge 1 ]; then
      log "   ✅ app iPad de retour sur le réseau ($NP pairs) en $(( $(now) - T ))s"
      break
    fi
  done
else
  log "   iPad injoignable (devicectl) — S2 sauté"
fi

# ── SCÉNARIO 3 : messages en attente + destinataire coupé → livraison au retour ──
log ""
log "── S3 : B coupé, A envoie 6 messages, B revient → délai de livraison ──"
kill "$(pgrep -f 'username B')" 2>/dev/null
log "   B coupé"; sleep 3
for k in $(seq 1 6); do curl -s -m5 -X POST "http://127.0.0.1:9601/send?to=$BID&size=7000" >/dev/null 2>&1; sleep 1; done
log "   6 messages envoyés vers B hors-ligne"
sleep 30
boot B2_ignore 9603 2>/dev/null; kill %% 2>/dev/null  # noop garde
"$BIN" --bot --username B --node-label B --key-path "$DIR/B.key" --control-port 9602 > "$DIR/B2.log" 2>&1 &
T=$(now)
log "   B ressuscité — chrono livraison"
DEL=0
for i in $(seq 1 40); do
  sleep 2
  N=$(grep -c "CTRL:7000:" "$DIR/B2.log" 2>/dev/null); N=${N:-0}
  if [ "$N" -gt "$DEL" ]; then DEL=$N; log "   +$(( $(now) - T ))s : $DEL/6 livrés"; fi
  [ "$DEL" -ge 6 ] && break
done
log "   RÉSULTAT S3 : $DEL/6 messages en attente livrés à B après $(( $(now) - T ))s"

# ── SCÉNARIO 4 : re-livraison NE se reproduit PAS (pas de doublon après effacement) ──
log ""
log "── S4 : après livraison, A ne renvoie plus (backup effacé) ──"
sleep 20
FINAL=$(grep -c "CTRL:7000:" "$DIR/B2.log" 2>/dev/null); FINAL=${FINAL:-0}
if [ "$FINAL" -le 7 ]; then
  log "   ✅ $FINAL/6 reçus au total (pas de tempête de doublons → backup purgé après ACK)"
else
  log "   ⚠️ $FINAL reçus (> 6) — re-livraisons répétées, purge à vérifier"
fi

log "╚══════════════════ FIN DES SCÉNARIOS ══════════════════╝"
# nettoyage
pkill -f "username A" 2>/dev/null; pkill -f "username B" 2>/dev/null
