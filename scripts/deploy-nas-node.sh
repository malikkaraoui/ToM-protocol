#!/usr/bin/env bash
# Déploiement du nœud ToM sur le NAS (ARM64, Debian) — service systemd tom-node.service.
#
# Le NAS est géré par systemd (ExecStart=/usr/local/bin/tom-chat ..., Restart=always),
# PAS par un lancement manuel nohup — un ancien script visait /root/tom-node en nohup,
# ce qui se battait contre le service systemd (qui relançait l'ancien binaire toutes les
# 5s) et échouait de toute façon (voir scripts/nas-node-ctl.sh pour le contrôle du service).
# Corrigé 2026-07-13, même classe de bug que apps/tom-node-ios (script visant un chemin
# de déploiement obsolète pendant qu'un autre, réel, tournait déjà en parallèle).
#
# Usage: ./scripts/deploy-nas-node.sh [SSH_HOST] [SSH_PORT]
# Défaut : hôte public (82.67.95.8:2222) — l'IP LAN du NAS change au fil des redémarrages
# de VM (DHCP), l'hôte public via le port-forward Freebox est stable.

set -euo pipefail

SSH_HOST="${1:-82.67.95.8}"
SSH_PORT="${2:-2222}"
BINARY="target/aarch64-unknown-linux-musl/release/tom-chat"
REMOTE_BIN="/usr/local/bin/tom-chat"
NAS_SSH="ssh -p ${SSH_PORT} root@${SSH_HOST}"
NAS_SCP="scp -P ${SSH_PORT}"

if [[ ! -f "$BINARY" ]]; then
    echo "Binaire manquant — lancer: cargo zigbuild -p tom-tui --target aarch64-unknown-linux-musl --release"
    exit 1
fi

echo "→ Envoi du binaire vers ${SSH_HOST}:${SSH_PORT} (chemin temporaire, pas d'écrasement à chaud)..."
${NAS_SCP} "$BINARY" "root@${SSH_HOST}:${REMOTE_BIN}.new"

echo "→ Bascule atomique + redémarrage du service tom-node..."
${NAS_SSH} "
    mv ${REMOTE_BIN} ${REMOTE_BIN}.bak
    mv ${REMOTE_BIN}.new ${REMOTE_BIN}
    chmod +x ${REMOTE_BIN}
    systemctl restart tom-node
    sleep 2
    systemctl is-active tom-node
"

echo "✓ NAS node redéployé — statut: bash scripts/nas-node-ctl.sh status | logs: bash scripts/nas-node-ctl.sh logs"
