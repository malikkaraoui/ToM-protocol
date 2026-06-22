#!/usr/bin/env bash
# Déploiement du nœud complet ToM sur le NAS (ARM64, Debian)
# Usage: ./scripts/deploy-nas-node.sh [NAS_HOST]

set -euo pipefail

NAS="${1:-192.168.0.21}"
BINARY="target/aarch64-unknown-linux-musl/release/tom-chat"
REMOTE_PATH="/root/tom-node"
STATUS_PORT=9091
RELAY_PORT=3340
PUBLIC_IP="82.67.95.8"
USERNAME="NAS"

if [[ ! -f "$BINARY" ]]; then
    echo "Binaire manquant — lancer: cargo zigbuild -p tom-tui --target aarch64-unknown-linux-musl --release"
    exit 1
fi

echo "→ Envoi du binaire vers $NAS..."
scp "$BINARY" "root@${NAS}:${REMOTE_PATH}"

echo "→ Arrêt des anciens processus (tom-relay, tom-chat, tom-node)..."
ssh "root@${NAS}" "pkill -f 'tom-relay|tom-chat|tom-node' 2>/dev/null; sleep 1; true"

echo "→ Lancement nœud complet..."
ssh "root@${NAS}" "
    chmod +x ${REMOTE_PATH}
    nohup ${REMOTE_PATH} \
        --bot \
        --self-relay \
        --username ${USERNAME} \
        --status-port ${STATUS_PORT} \
        --embedded-relay-advertise ${PUBLIC_IP} \
        > /var/log/tom-node.log 2>&1 &
    sleep 2
    curl -s http://localhost:${STATUS_PORT}/status || echo 'status server pas encore démarré'
"

echo "✓ NAS node déployé — logs: ssh root@${NAS} 'tail -f /var/log/tom-node.log'"
