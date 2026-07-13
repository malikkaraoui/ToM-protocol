#!/usr/bin/env bash
# nas-node-ctl.sh — contrôle du nœud ToM sur le NAS
#
# Un seul service systemd : tom-node.service, lancé avec --self-relay
# (relais embarqué dans le même process, pas de service tom-relay séparé).
# Un ancien tom-relay.service référencé ici n'a jamais existé/plus jamais
# existé sur ce NAS — corrigé 2026-07-13 après un `status` qui échouait
# silencieusement dessus ("Unit tom-relay.service could not be found").
#
# Usage: ./scripts/nas-node-ctl.sh [status|stop|start|restart|logs]
set -euo pipefail

NAS_SSH="ssh -p 2222 root@82.67.95.8"
CMD="${1:-status}"

case "$CMD" in
  status)
    echo "=== tom-node (protocole + relais embarqué) ==="
    $NAS_SSH "systemctl status tom-node --no-pager | head -8" 2>&1
    ;;
  stop)
    $NAS_SSH "systemctl stop tom-node"
    $NAS_SSH "ss -tlnp | grep 3340 || echo 'port 3340: libre'" 2>&1
    echo "tom-node: arrêté"
    ;;
  start)
    $NAS_SSH "systemctl start tom-node"
    echo "tom-node: démarré"
    ;;
  restart)
    $NAS_SSH "systemctl restart tom-node"
    echo "tom-node: redémarré"
    ;;
  logs)
    $NAS_SSH "journalctl -u tom-node -n 20 --no-pager" 2>&1
    ;;
  *)
    echo "Usage: $0 [status|stop|start|restart|logs]"
    exit 1
    ;;
esac
