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

# Fonction cleanup
cleanup() {
    echo ""
    echo "=== Collecteur arrêté ==="
    exit 0
}
trap cleanup INT TERM

# Vérifier que nc (netcat) est disponible
if ! command -v nc &>/dev/null; then
    echo "ERREUR : nc (netcat) non trouvé. Installer avec : brew install netcat"
    exit 1
fi

# Boucle principale : nc -u -l écoute un seul datagramme sur macOS,
# donc on boucle pour rester à l'écoute.
while true; do
    nc -u -l "$PORT" 2>/dev/null | while IFS= read -r line; do
        # Écrire dans all.jsonl
        echo "$line" >> "${LOGDIR}/all.jsonl"

        # Extraire le nom du nœud pour le fichier dédié
        node=$(echo "$line" | python3 -c "import sys,json; print(json.loads(sys.stdin.read()).get('node','unknown'))" 2>/dev/null || echo "unknown")
        echo "$line" >> "${LOGDIR}/${node}.jsonl"

        # Affichage live si --tail
        if [ "$TAIL" = "--tail" ]; then
            echo "$line" | python3 -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    ts = d.get('ts', '?')
    node = d.get('node', '?')
    event = d.get('event', '?')
    detail = d.get('detail', '')
    phase = d.get('phase', '?')
    taille = d.get('taille_reseau', '?')
    role = d.get('role', '?')
    print(f'{ts} [{node:>10}] {event:<25} {detail:<30} phase={phase} taille={taille} role={role}')
except:
    print(sys.stdin.read(), end='')
" 2>/dev/null || echo "$line"
        fi
    done
done
