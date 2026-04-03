#!/usr/bin/env bash
# launch-noyau.sh — Lance le noyau stable du réseau ToM sur le MacBook Pro
#
# Lance 3 nœuds ToM en parallèle + le collecteur de logs.
# Chaque nœud a son nom, ses logs UDP, et sa page d'état.
#
# Usage :
#   ./scripts/launch-noyau.sh          # lancer tout
#   ./scripts/launch-noyau.sh stop     # arrêter tout
#
# Pré-requis :
#   - NAS relay actif sur 192.168.0.83:3340
#   - cargo build fait (tom-chat compilé)

set -euo pipefail

RELAY_URL="http://192.168.0.83:3340"
COLLECTOR_IP="192.168.0.70"
COLLECTOR_PORT="9999"
LOG_DIR="logs"
RUST_LOG="${RUST_LOG:-info}"

# Ports d'état HTTP par nœud
STATUS_PORT_1=8081
STATUS_PORT_2=8082
STATUS_PORT_3=8083

# PID file pour pouvoir tout arrêter
PID_FILE="/tmp/tom-noyau-pids"

stop_all() {
    echo "=== Arrêt du noyau ToM ==="
    if [ -f "$PID_FILE" ]; then
        while read -r pid; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null && echo "  arrêté: $pid"
            fi
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi
    echo "=== Noyau arrêté ==="
    exit 0
}

if [ "${1:-}" = "stop" ]; then
    stop_all
fi

# Cleanup si on Ctrl+C
trap stop_all INT TERM

mkdir -p "$LOG_DIR"
> "$PID_FILE"

echo "=== Noyau ToM — MacBook Pro ==="
echo "    Relay NAS : $RELAY_URL"
echo "    Collecteur: $COLLECTOR_IP:$COLLECTOR_PORT"
echo "    Logs dans : $LOG_DIR/"
echo ""

# Vérifier que le relay NAS répond
if ! curl -s --connect-timeout 3 "$RELAY_URL/health" | grep -q '"ok"'; then
    echo "ERREUR : le relay NAS ne répond pas sur $RELAY_URL"
    echo "Vérifier que tom-relay tourne sur le NAS."
    exit 1
fi
echo "  NAS relay : OK"

# Vérifier que le binaire existe
TOM_CHAT="./target/release/tom-chat"
if [ ! -f "$TOM_CHAT" ]; then
    TOM_CHAT="./target/debug/tom-chat"
fi
if [ ! -f "$TOM_CHAT" ]; then
    echo "ERREUR : binaire tom-chat introuvable. Faire : cargo build -p tom-tui --release"
    exit 1
fi
echo "  Binaire   : $TOM_CHAT"
echo ""

# Lancer le collecteur de logs Python (fiable, écoute 0.0.0.0)
echo "  Lancement collecteur de logs (port $COLLECTOR_PORT)..."
python3 ./scripts/collect-logs.py "$COLLECTOR_PORT" > "$LOG_DIR/collecteur.out" 2>&1 &
COLLECTOR_PID=$!
echo "$COLLECTOR_PID" >> "$PID_FILE"
echo "  Collecteur: PID $COLLECTOR_PID"
sleep 1

# Lancer les 3 nœuds
for i in 1 2 3; do
    LABEL="mbp-$i"
    PORT_STATUS=$((STATUS_PORT_1 + i - 1))

    echo "  Lancement $LABEL (status: :$PORT_STATUS)..."

    # Seul mbp-1 lance un relay embarqué, les autres sont des nœuds simples
    RELAY_FLAG=""
    if [ "$i" = "1" ]; then
        RELAY_FLAG="--self-relay"
    fi

    TOM_RELAY_URL="$RELAY_URL" RUST_LOG="$RUST_LOG" \
        "$TOM_CHAT" \
        --bot \
        --bot-ping 10 \
        $RELAY_FLAG \
        --node-label "$LABEL" \
        --log-udp "$COLLECTOR_IP:$COLLECTOR_PORT" \
        --status-port "$PORT_STATUS" \
        --username "$LABEL" \
        > "$LOG_DIR/$LABEL.out" 2>&1 &

    NODE_PID=$!
    echo "$NODE_PID" >> "$PID_FILE"
    echo "  $LABEL    : PID $NODE_PID"

    # Petit délai entre chaque nœud pour éviter les conflits de port
    sleep 2
done

echo ""
echo "=== 3 nœuds lancés ==="
echo ""
echo "  Pages d'état :"
echo "    curl http://localhost:$STATUS_PORT_1   # mbp-1"
echo "    curl http://localhost:$STATUS_PORT_2   # mbp-2"
echo "    curl http://localhost:$STATUS_PORT_3   # mbp-3"
echo ""
echo "  Logs live :"
echo "    tail -f $LOG_DIR/all.jsonl"
echo "    tail -f $LOG_DIR/mbp-1.jsonl"
echo ""
echo "  MacBook Air (à lancer manuellement) :"
echo "    TOM_RELAY_URL=$RELAY_URL RUST_LOG=info ~/tom-chat --bot --node-label macair --log-udp 192.168.0.70:$COLLECTOR_PORT --status-port 8084 --username macair"
echo ""
echo "  Arrêter : ./scripts/launch-noyau.sh stop"
echo "  Ou : Ctrl+C"
echo ""

# Attendre (le script reste vivant pour le trap Ctrl+C)
echo "=== En attente... Ctrl+C pour arrêter ==="
wait
