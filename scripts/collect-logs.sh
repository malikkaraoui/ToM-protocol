#!/usr/bin/env bash
# collect-logs.sh — Collecteur central de logs pour le réseau ToM
#
# Écoute sur un port UDP et écrit les logs reçus dans :
#   - logs/all.jsonl       (tous les nœuds, entrelacés)
#   - logs/<node>.jsonl    (un fichier par nœud, extrait du champ "node")
#
# Alertes (lignes rouges en stdout) :
#   - Nœud silencieux depuis >30s
#   - Nœud bloqué en phase "lan_probe" depuis >5min
#   - Compteur global : messages totaux envoyés/reçus
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
echo "    Alertes   : silence >30s, lan_probe >5min"
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
import time

port = int(sys.argv[1])
logdir = sys.argv[2]
do_tail = sys.argv[3] == "1"

# Couleurs ANSI
RED   = "\033[31m"
RESET = "\033[0m"
CYAN  = "\033[36m"

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.settimeout(5.0)  # timeout pour la boucle d'alerte
sock.bind(("0.0.0.0", port))

all_log = open(os.path.join(logdir, "all.jsonl"), "a", buffering=1)
node_files = {}

# Tracking des métriques par nœud
node_last_seen = {}       # label → timestamp float (epoch s)
node_lan_probe_since = {} # label → timestamp float (epoch s)
node_msgs_sent = {}       # label → int
node_msgs_recv = {}       # label → int

# Horodatage de la dernière alerte par nœud (pour éviter le spam)
node_silence_alerted = {}  # label → timestamp float
node_lanprobe_alerted = {} # label → timestamp float

SILENCE_THRESHOLD_S  = 30
LAN_PROBE_THRESHOLD_S = 5 * 60
ALERT_COOLDOWN_S      = 60  # ne répéter une alerte que toutes les 60s


def get_node_file(node):
    if node not in node_files:
        path = os.path.join(logdir, f"{node}.jsonl")
        node_files[node] = open(path, "a", buffering=1)
    return node_files[node]


def check_alerts(now):
    """Vérifier les conditions d'alerte sur tous les nœuds vus."""
    if not node_last_seen:
        return

    for label, last in list(node_last_seen.items()):
        silent_for = now - last

        # Alerte : silence >30s
        if silent_for > SILENCE_THRESHOLD_S:
            last_alerted = node_silence_alerted.get(label, 0)
            if now - last_alerted > ALERT_COOLDOWN_S:
                print(
                    f"{RED}[ALERTE] {label} silencieux depuis {silent_for:.0f}s "
                    f"(dernière réception il y a {silent_for:.0f}s){RESET}",
                    flush=True
                )
                node_silence_alerted[label] = now

        # Alerte : bloqué en lan_probe >5min
        if label in node_lan_probe_since:
            probe_for = now - node_lan_probe_since[label]
            if probe_for > LAN_PROBE_THRESHOLD_S:
                last_alerted = node_lanprobe_alerted.get(label, 0)
                if now - last_alerted > ALERT_COOLDOWN_S:
                    print(
                        f"{RED}[ALERTE] {label} bloqué en phase lan_probe depuis "
                        f"{probe_for / 60:.1f}min !{RESET}",
                        flush=True
                    )
                    node_lanprobe_alerted[label] = now


def print_global_counters():
    """Afficher le compteur global des messages."""
    total_sent = sum(node_msgs_sent.values())
    total_recv = sum(node_msgs_recv.values())
    loss = 0.0
    if total_sent > 0:
        loss = max(0.0, 1.0 - total_recv / total_sent) * 100.0
    print(
        f"{CYAN}[STATS] total_sent={total_sent} total_recv={total_recv} loss={loss:.1f}%{RESET}",
        flush=True
    )


last_stats_print = time.monotonic()
STATS_INTERVAL_S = 60.0

try:
    while True:
        now = time.monotonic()

        # Imprimer stats périodiquement
        if now - last_stats_print >= STATS_INTERVAL_S:
            print_global_counters()
            last_stats_print = now

        # Vérifier alertes
        check_alerts(now)

        # Recevoir un paquet UDP (avec timeout pour revenir à la boucle d'alerte)
        try:
            data, addr = sock.recvfrom(65535)
        except socket.timeout:
            continue

        for line in data.decode("utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line:
                continue
            all_log.write(line + "\n")
            try:
                d = json.loads(line)
                node = d.get("node", "unknown")
                get_node_file(node).write(line + "\n")

                # Mettre à jour les métriques de suivi
                node_last_seen[node] = time.monotonic()
                # Réinitialiser l'alerte silence
                node_silence_alerted.pop(node, None)

                # Suivi phase lan_probe
                phase = str(d.get("phase", "")).lower()
                if "lanprobe" in phase or "lan_probe" in phase:
                    if node not in node_lan_probe_since:
                        node_lan_probe_since[node] = time.monotonic()
                else:
                    # Phase changée — réinitialiser
                    node_lan_probe_since.pop(node, None)
                    node_lanprobe_alerted.pop(node, None)

                # Compteurs de messages
                msgs_sent = d.get("msgs_sent", 0)
                msgs_recv = d.get("msgs_recv", 0)
                if isinstance(msgs_sent, (int, float)):
                    node_msgs_sent[node] = int(msgs_sent)
                if isinstance(msgs_recv, (int, float)):
                    node_msgs_recv[node] = int(msgs_recv)

                if do_tail:
                    ts        = d.get("ts", "?")
                    appareil  = d.get("appareil", "")
                    label     = f"{node}/{appareil}" if appareil else node
                    event     = d.get("event", "?")
                    detail    = d.get("detail", "")
                    phase_raw = d.get("phase", "?")
                    taille    = d.get("taille_reseau", "?")
                    role      = d.get("role", "?")
                    src       = d.get("source_amorcage", "")
                    src_tag   = f" src={src}" if src else ""
                    print(
                        f"{ts} [{label:>16}] {event:<25} {detail:<30} "
                        f"phase={phase_raw} taille={taille} role={role}{src_tag}",
                        flush=True
                    )
            except Exception:
                get_node_file("unknown").write(line + "\n")

except KeyboardInterrupt:
    print("\n")
    print_global_counters()
    print("\n=== Collecteur arrêté ===")
finally:
    sock.close()
    all_log.close()
    for f in node_files.values():
        f.close()
PYEOF
