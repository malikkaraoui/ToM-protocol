#!/usr/bin/env bash
# nas-node-ctl.sh — contrôle du nœud ToM sur le NAS
# Usage: ./scripts/nas-node-ctl.sh [status|stop|start|restart|relay-stop|relay-start|logs]
set -euo pipefail

NAS_SSH="ssh -p 2222 root@82.67.95.8"
CMD="${1:-status}"

case "$CMD" in
  status)
    echo "=== tom-node (protocole) ==="
    $NAS_SSH "systemctl status tom-node --no-pager | head -8" 2>&1
    echo ""
    echo "=== tom-relay (QUIC NAT) ==="
    $NAS_SSH "systemctl status tom-relay --no-pager | head -8" 2>&1
    ;;
  stop)
    $NAS_SSH "systemctl stop tom-node tom-relay"
    $NAS_SSH "ss -tlnp | grep 3340 || echo 'port 3340: libre'" 2>&1
    echo "tom-node + tom-relay: arrêtés"
    ;;
  start)
    $NAS_SSH "systemctl start tom-relay tom-node"
    echo "tom-relay + tom-node: démarrés"
    ;;
  restart)
    $NAS_SSH "systemctl restart tom-relay tom-node"
    echo "tom-relay + tom-node: redémarrés"
    ;;
  relay-stop)
    $NAS_SSH "systemctl stop tom-relay"
    $NAS_SSH "ss -tlnp | grep 3340 || echo 'port 3340: libre'" 2>&1
    echo "tom-relay: arrêté"
    ;;
  relay-start)
    $NAS_SSH "systemctl start tom-relay"
    echo "tom-relay: démarré"
    ;;
  logs)
    $NAS_SSH "journalctl -u tom-node -n 20 --no-pager" 2>&1
    ;;
  *)
    echo "Usage: $0 [status|stop|start|restart|relay-stop|relay-start|logs]"
    exit 1
    ;;
esac
