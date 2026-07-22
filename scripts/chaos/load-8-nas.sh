#!/usr/bin/env bash
# Test 8-nœuds sur le VRAI NAS instrumenté : 8 nœuds rejoignent le rendez-vous prod,
# se connectent au NAS + trafic, on lit QUEL compteur du NAS suit le RSS.
# Puis kill des 8 → le RSS redescend-il ? quel compteur redescend ?
# Pollution bornée : kill des 8 en fin + note reset carnet NAS.
set -uo pipefail
BIN=/Users/malik/Documents/tom-protocol/target/debug/tom-chat
NAS=root@192.168.0.83
NAS_RELAY=http://192.168.0.83:3340
NAS_ST=http://192.168.0.83:8085
K=${1:-8}; TICKS=${2:-12}; POST=${3:-5}   # 12×15s charge, 5×15s post-kill

log(){ echo "[$(date +%T)] $*"; }
nstat(){ curl -s -m4 $NAS_ST 2>/dev/null; }
num(){ echo "$1" | grep -o "\"$2\":[0-9]*" | head -1 | grep -o '[0-9]*$'; }
nas_rss(){ ssh -o ConnectTimeout=5 $NAS 'PID=$(systemctl show -p MainPID --value tom-node); echo "$PID $(grep -o "VmRSS:[[:space:]]*[0-9]*" /proc/$PID/status | grep -o "[0-9]*")"'; }

NJ=$(nstat)
NAS_ID=$(echo "$NJ" | grep -o '[0-9a-f]\{64\}' | head -1)
[ -z "$NAS_ID" ] && { log "ABORT: pas de NAS node_id (/status injoignable)"; exit 1; }
log "NAS id=$(echo $NAS_ID|cut -c1-12) baseline: rss=$(nas_rss) peers_known=$(num "$NJ" peers_known) pool=$(num "$NJ" connexions_pool) relais_acc=$(num "$NJ" relais_accepts_total)"

pkill -9 -f "target/debug/tom-chat" 2>/dev/null; sleep 1
PIDS=(); CTRLS=()
cleanup(){ log "cleanup 8 nœuds"; for p in "${PIDS[@]:-}"; do kill -9 "$p" 2>/dev/null; done; pkill -9 -f "target/debug/tom-chat" 2>/dev/null; }
trap cleanup EXIT TERM INT   # TERM/INT pour que `timeout` déclenche bien le kill des nœuds

# ── lancer K nœuds NON-isolés (rendez-vous prod) + relais=NAS ──
log "lance $K nœuds (rendez-vous prod + TOM_RELAY_URL=NAS)"
for i in $(seq 1 $K); do
  CP=$((19410+i))
  TOM_RELAY_URL=$NAS_RELAY "$BIN" --bot --node-label load$i --username loadtest$i \
    --control-port $CP >/tmp/load$i.log 2>&1 &
  PIDS+=($!); CTRLS+=($CP)
done

echo "phase,t_s,nas_pid,rss_kb,peers_known,pool,relais_actifs,relais_accepts,conns_quic,handshakes,taille_reseau" > /tmp/nas-load.csv
T0=$SECONDS
obs(){  # $1=phase
  local NJ RS PID_RSS
  NJ=$(nstat); PID_RSS=$(nas_rss); local PID=${PID_RSS%% *}; RS=${PID_RSS##* }
  local t=$((SECONDS-T0))
  log "[$1 t+${t}s] NAS rss=${RS}KB pid=$PID | conns_quic=$(num "$NJ" conns_quic_live) hs=$(num "$NJ" handshakes_accepted) | peers_known=$(num "$NJ" peers_known) pool=$(num "$NJ" connexions_pool) relais_actifs=$(num "$NJ" clients_relais_actifs) relais_acc=$(num "$NJ" relais_accepts_total) taille=$(num "$NJ" taille_reseau)"
  echo "$1,$t,$PID,$RS,$(num "$NJ" peers_known),$(num "$NJ" connexions_pool),$(num "$NJ" clients_relais_actifs),$(num "$NJ" relais_accepts_total),$(num "$NJ" conns_quic_live),$(num "$NJ" handshakes_accepted),$(num "$NJ" taille_reseau)" >> /tmp/nas-load.csv
}

# ── CHECK MONTAGE : les nœuds atteignent-ils le NAS ? (sinon test à vide) ──
sleep 18
CJ=$(nstat)
log "CHECK-MONTAGE (t+18s) NAS conns_quic=$(num "$CJ" conns_quic_live) hs=$(num "$CJ" handshakes_accepted) | peers_known=$(num "$CJ" peers_known) pool=$(num "$CJ" connexions_pool) relais_actifs=$(num "$CJ" clients_relais_actifs) relais_acc=$(num "$CJ" relais_accepts_total)"
log "  (baseline peers_known=5 pool=2 relais_acc=3 — si INCHANGÉ, les nœuds N'ATTEIGNENT PAS le NAS)"

# ── phase CHARGE : trafic vers le NAS + observation ──
for k in $(seq 1 $TICKS); do
  OKS=0
  for cp in "${CTRLS[@]}"; do
    R=$(curl -s -m8 "http://127.0.0.1:$cp/send?to=$NAS_ID&size=3000&count=8" 2>/dev/null)
    echo "$R" | grep -q '"ok":true' && OKS=$((OKS+1))
  done
  obs "CHARGE"
  log "  sends OK ce tick : $OKS/$K"
  sleep 8
done

# ── KILL des 8 + phase POST (redescente ?) ──
log "═══ KILL des $K nœuds ═══"
for p in "${PIDS[@]}"; do kill -9 "$p" 2>/dev/null; done
PIDS=()
for k in $(seq 1 $POST); do sleep 15; obs "POST-KILL"; done

log "═══ ORACLE (verdict fix) ═══"
awk -F, 'NR>1 {
  rss=$4+0; acc=$8+0; cq=$9+0; phase=$1;
  if (first==0){rss0=rss; acc0=acc; cq0=cq; first=1}
  if (rss>rssmax) rssmax=rss;
  if (cq>cqmax) cqmax=cq;
  if (phase=="POST-KILL"){rssf=rss; accf=acc; cqf=cq}
}
END {
  if(rssf==0){rssf=rss; accf=acc; cqf=cq}
  dacc=accf-acc0;
  red_rss=(rssf<rssmax*0.92)?"OUI":"NON";
  red_cq=(cqmax>0 && cqf<cqmax*0.5)?"OUI":"NON";
  printf "  RSS0=%dKB PIC=%dKB FINAL=%dKB | conns_quic: pic=%d final=%d | Δaccepts=%d\n",rss0,rssmax,rssf,cqmax,cqf,dacc;
  printf "  RSS redescend post-kill: %s | conns_quic redescend post-kill: %s\n",red_rss,red_cq;
  if (red_rss=="NON" && red_cq=="NON")
    printf "  VERDICT: FUITE CONNEXION — conns_quic ET RSS restent hauts post-kill → connexions non purgees (transport / ref_count)\n";
  else if (red_rss=="NON" && red_cq=="OUI")
    printf "  VERDICT: FUITE BUFFERS — conns_quic redescend mais RSS reste → memoire retenue HORS map connexions (buffers/alloc)\n";
  else
    printf "  VERDICT: PAS DE FUITE nette (RSS redescend post-kill)\n";
}' /tmp/nas-load.csv
log "═══ CSV → /tmp/nas-load.csv ═══"
column -t -s, /tmp/nas-load.csv 2>/dev/null || cat /tmp/nas-load.csv
log "⚠️ pollution : carnet NAS a gagné des entrées loadtest — vider state.db au redéploiement"
