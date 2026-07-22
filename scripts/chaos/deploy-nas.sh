#!/usr/bin/env bash
# Déploie le tom-chat ARM instrumenté sur le NAS (service tom-node.service).
# Stop propre (évite le Restart=always pendant le scp) → backup → scp → start → vérif.
set -uo pipefail   # PAS -e : on garantit le restart du service même en cas d'échec
NAS=root@192.168.0.83
BIN=/Users/malik/Documents/tom-protocol/target/aarch64-unknown-linux-musl/release/tom-chat

[ -f "$BIN" ] || { echo "ABORT: binaire ARM absent ($BIN) — zigbuild pas fini ?"; exit 1; }
LOCAL_MD5=$(md5 -q "$BIN" 2>/dev/null || md5sum "$BIN" | awk '{print $1}')
echo "1) binaire local md5=$LOCAL_MD5 taille=$(ls -la "$BIN" | awk '{print $5}')"

echo "2) stop service (désactive Restart=always le temps du scp)"
ssh -o ConnectTimeout=6 $NAS 'systemctl stop tom-node; sleep 2; pkill -9 tom-chat 2>/dev/null || true; sleep 1; echo "  stoppé, RSS restants:"; ps -eo comm | grep -c tom-chat || true'

echo "3) backup ancien binaire"
ssh $NAS 'cp -f /usr/local/bin/tom-chat /usr/local/bin/tom-chat.bak.pre-instru 2>/dev/null || true'

echo "4) scp nouveau binaire"
if ! scp "$BIN" $NAS:/usr/local/bin/tom-chat; then
  echo "⚠️ SCP ÉCHEC — je restaure le .bak et relance le service (NAS ne doit PAS rester down)"
  ssh $NAS 'cp -f /usr/local/bin/tom-chat.bak.pre-instru /usr/local/bin/tom-chat 2>/dev/null; systemctl start tom-node'
  exit 1
fi

echo "5) md5 distant (doit == local)"
REMOTE_MD5=$(ssh $NAS 'md5sum /usr/local/bin/tom-chat | awk "{print \$1}"')
echo "   remote md5=$REMOTE_MD5"
[ "$LOCAL_MD5" = "$REMOTE_MD5" ] && echo "   ✅ md5 OK" || echo "   ⚠️ md5 DIFFÈRE"

echo "6) start service"
ssh $NAS 'systemctl start tom-node; sleep 7'

echo "7) vérif instrumentation présente dans /status"
ssh $NAS 'curl -s -m4 http://127.0.0.1:8085/ | grep -oE "\"(conns_quic_live|handshakes_accepted|relais_accepts_total|peers_known)\":[0-9]+" || echo "  ⚠️ compteurs ABSENTS du /status"'

echo "8) baseline (RSS + phase + carnet)"
ssh $NAS 'PID=$(systemctl show -p MainPID --value tom-node); echo "  PID=$PID"; grep VmRSS /proc/$PID/status; curl -s -m4 http://127.0.0.1:8085/ | grep -oE "\"(phase|taille_reseau|peers_known)\":[^,}]+"'
echo "DÉPLOIEMENT TERMINÉ"
