#!/usr/bin/env python3
"""Collecteur central de logs ToM — UDP listener.

Écoute sur 0.0.0.0:PORT (toutes les interfaces) et écrit :
  - logs/all.jsonl       (tous les nœuds)
  - logs/<node>.jsonl    (par nœud)
  - stdout               (live, coloré)

Usage :
  python3 scripts/collect-logs.py              # port 9999
  python3 scripts/collect-logs.py 9998         # port custom
"""
import json
import os
import socket
import sys
from datetime import datetime

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 9999
LOGDIR = "logs"
os.makedirs(LOGDIR, exist_ok=True)

# Couleurs par nœud
COLORS = [
    "\033[36m",   # cyan
    "\033[32m",   # green
    "\033[33m",   # yellow
    "\033[35m",   # magenta
    "\033[34m",   # blue
    "\033[91m",   # light red
    "\033[92m",   # light green
    "\033[93m",   # light yellow
    "\033[94m",   # light blue
    "\033[95m",   # light magenta
]
RESET = "\033[0m"
node_colors = {}
color_idx = 0

def get_color(node):
    global color_idx
    if node not in node_colors:
        node_colors[node] = COLORS[color_idx % len(COLORS)]
        color_idx += 1
    return node_colors[node]

# Ouvrir le socket UDP sur toutes les interfaces
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
sock.bind(("0.0.0.0", PORT))

print(f"=== Collecteur ToM — UDP :{PORT} sur 0.0.0.0 ===")
print(f"    Logs dans : {LOGDIR}/")
print(f"    Ctrl+C pour arrêter")
print()

all_file = open(os.path.join(LOGDIR, "all.jsonl"), "a", buffering=1)

try:
    while True:
        data, addr = sock.recvfrom(65535)
        line = data.decode("utf-8", errors="replace").strip()
        if not line:
            continue

        # Écrire dans all.jsonl
        all_file.write(line + "\n")

        # Extraire le nom du nœud
        node = "unknown"
        try:
            d = json.loads(line)
            node = d.get("node", "unknown")
        except:
            pass

        # Écrire dans le fichier du nœud
        node_path = os.path.join(LOGDIR, f"{node}.jsonl")
        with open(node_path, "a") as f:
            f.write(line + "\n")

        # Affichage live coloré
        color = get_color(node)
        try:
            d = json.loads(line)
            ts = d.get("ts", "?")
            event = d.get("event", "?")
            detail = str(d.get("detail", ""))[:50]
            phase = d.get("phase", "?")
            taille = d.get("taille_reseau", "?")
            role = d.get("role", "?")
            print(f"{color}{ts} [{node:>10}] {event:<25} {detail:<50} phase={phase} taille={taille} role={role}{RESET}")
        except:
            print(f"{color}[{node:>10}] {line[:120]}{RESET}")

        sys.stdout.flush()

except KeyboardInterrupt:
    print("\n=== Collecteur arrêté ===")
finally:
    all_file.close()
    sock.close()
