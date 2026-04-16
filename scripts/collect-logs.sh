#!/usr/bin/env bash
# collect-logs.sh — Collecteur central de logs pour le réseau ToM
#
# Écoute sur un port UDP et écrit les logs reçus dans :
#   - logs/all.jsonl       (tous les nœuds, entrelacés)
#   - logs/<node>.jsonl    (un fichier par nœud, extrait du champ "node")
#
# Usage :
#   ./scripts/collect-logs.sh              # port 9999 par défaut
#   ./scripts/collect-logs.sh 9998         # port personnalisé
#   ./scripts/collect-logs.sh 9999 --tail  # écoute + affiche en live
#
# Ctrl+C pour arrêter.

set -euo pipefail

PORT="${1:-9999}"
TAIL="${2:-}"
LOGDIR="logs"

mkdir -p "$LOGDIR"

echo "=== Collecteur ToM — écoute UDP :${PORT} ==="
echo "    Logs dans : ${LOGDIR}/"
echo "    Ctrl+C pour arrêter"
echo ""

TAIL_FLAG=""
if [ "$TAIL" = "--tail" ]; then
    TAIL_FLAG="1"
fi

python3 - "$PORT" "$LOGDIR" "$TAIL_FLAG" <<'PYEOF'
import sys
import socket
import json
import os

port = int(sys.argv[1])
logdir = sys.argv[2]
do_tail = sys.argv[3] == "1"

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind(("0.0.0.0", port))

all_log = open(os.path.join(logdir, "all.jsonl"), "a", buffering=1)
node_files = {}

def get_node_file(node):
    if node not in node_files:
        path = os.path.join(logdir, f"{node}.jsonl")
        node_files[node] = open(path, "a", buffering=1)
    return node_files[node]

try:
    while True:
        data, addr = sock.recvfrom(65535)
        for line in data.decode("utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line:
                continue
            all_log.write(line + "\n")
            try:
                d = json.loads(line)
                node = d.get("node", "unknown")
                get_node_file(node).write(line + "\n")
                if do_tail:
                    ts = d.get("ts", "?")
                    appareil = d.get("appareil", "")
                    label = f"{node}/{appareil}" if appareil else node
                    event = d.get("event", "?")
                    detail = d.get("detail", "")
                    phase = d.get("phase", "?")
                    taille = d.get("taille_reseau", "?")
                    role = d.get("role", "?")
                    src = d.get("source_amorcage", "")
                    src_tag = f" src={src}" if src else ""
                    print(f"{ts} [{label:>16}] {event:<25} {detail:<30} phase={phase} taille={taille} role={role}{src_tag}", flush=True)
            except Exception:
                get_node_file("unknown").write(line + "\n")
except KeyboardInterrupt:
    print("\n=== Collecteur arrêté ===")
finally:
    sock.close()
    all_log.close()
    for f in node_files.values():
        f.close()
PYEOF
