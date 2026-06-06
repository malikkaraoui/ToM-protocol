#!/usr/bin/env bash
# launch-5nodes.sh — Lance un réseau local ToM à 5 nœuds pour l'endurance 6h
#
# Topologie :
#   T0 : tom-relay --dev (relay bootstrap local, port 3340)
#   T1 : publisher  — embedded-relay + gossip publication + bot-ping 10s
#   T2 : relay1     — relay-discovery activé + bot
#   T3 : observer1  — relay-discovery + bot
#   T4 : observer2  — relay-discovery + bot
#   T5 : observer3  — relay-discovery + bot
#
# Usage :
#   ./scripts/launch-5nodes.sh           # lancer tout
#   ./scripts/launch-5nodes.sh stop      # arrêter tout
#
# Pré-requis :
#   - cargo build -p tom-relay -p tom-tui --release (ou debug)
#   - Port 3340 libre sur localhost

set -euo pipefail

RELAY_PORT=3340
RELAY_URL="http://127.0.0.1:${RELAY_PORT}"
COLLECTOR_IP="127.0.0.1"
COLLECTOR_PORT="9999"
LOG_DIR="logs/endurance"
RUST_LOG="${RUST_LOG:-info}"
BOT_PING_SECS=10

# Ports HTTP de status par nœud
STATUS_PORT_PUBLISHER=8081
STATUS_PORT_RELAY1=8082
STATUS_PORT_OBS1=8083
STATUS_PORT_OBS2=8084
STATUS_PORT_OBS3=8085

PID_FILE="/tmp/tom-5nodes-pids"

# ── Helpers ───────────────────────────────────────────────────────────────

log() { echo "[$(date '+%H:%M:%S')] $*"; }

stop_all() {
    log "=== Arrêt du réseau 5 nœuds ==="
    if [ -f "$PID_FILE" ]; then
        while read -r pid; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null && log "  arrêté: PID $pid"
            fi
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi
    log "=== Réseau arrêté ==="
    exit 0
}

if [ "${1:-}" = "stop" ]; then
    stop_all
fi

trap stop_all INT TERM

# ── Résolution des binaires ───────────────────────────────────────────────

find_binary() {
    local name="$1"
    local release="./target/release/${name}"
    local debug="./target/debug/${name}"
    if [ -f "$release" ]; then
        echo "$release"
    elif [ -f "$debug" ]; then
        echo "$debug"
    else
        echo ""
    fi
}

TOM_RELAY=$(find_binary "tom-relay")
TOM_CHAT=$(find_binary "tom-chat")

if [ -z "$TOM_RELAY" ]; then
    log "ERREUR : binaire tom-relay introuvable."
    log "  Faire : cargo build -p tom-relay --release"
    exit 1
fi

if [ -z "$TOM_CHAT" ]; then
    log "ERREUR : binaire tom-chat introuvable."
    log "  Faire : cargo build -p tom-tui --release"
    exit 1
fi

mkdir -p "$LOG_DIR"
> "$PID_FILE"

log "=== Réseau ToM local — 5 nœuds (endurance 6h) ==="
log "    Relay local : $RELAY_URL"
log "    Collecteur  : $COLLECTOR_IP:$COLLECTOR_PORT"
log "    Logs dans   : $LOG_DIR/"
log "    Binaire TUI : $TOM_CHAT"
log "    Binaire RLY : $TOM_RELAY"
log ""

# ── T0 : Relay bootstrap local ────────────────────────────────────────────

log "  [T0] Lancement tom-relay --dev (port $RELAY_PORT)..."
"$TOM_RELAY" --dev \
    > "$LOG_DIR/relay-bootstrap.out" 2>&1 &
RELAY_PID=$!
echo "$RELAY_PID" >> "$PID_FILE"
log "  [T0] tom-relay : PID $RELAY_PID"

# Attendre que le relay soit prêt (health check)
log "  [T0] Attente du relay..."
RELAY_WAIT=0
until curl -s --connect-timeout 1 "${RELAY_URL}/health" > /dev/null 2>&1; do
    RELAY_WAIT=$((RELAY_WAIT + 1))
    if [ $RELAY_WAIT -ge 15 ]; then
        log "ERREUR : relay ne répond pas après 15s. Vérifier $LOG_DIR/relay-bootstrap.out"
        exit 1
    fi
    sleep 1
done
log "  [T0] Relay prêt (${RELAY_WAIT}s)"
log ""

# ── T1 : Publisher (embedded-relay + gossip publication) ─────────────────

log "  [T1] Lancement publisher (embedded-relay + bot-ping ${BOT_PING_SECS}s)..."
TOM_RELAY_URL="$RELAY_URL" RUST_LOG="$RUST_LOG" \
    "$TOM_CHAT" \
    --bot \
    --bot-ping "$BOT_PING_SECS" \
    --self-relay \
    --embedded-relay-bind "0.0.0.0:0" \
    --node-label "publisher" \
    --node-appareil "linux" \
    --log-udp "${COLLECTOR_IP}:${COLLECTOR_PORT}" \
    --status-port "$STATUS_PORT_PUBLISHER" \
    --username "publisher" \
    > "$LOG_DIR/publisher.out" 2>&1 &
PUBLISHER_PID=$!
echo "$PUBLISHER_PID" >> "$PID_FILE"
log "  [T1] publisher : PID $PUBLISHER_PID (status: :$STATUS_PORT_PUBLISHER)"
sleep 3

# ── T2-T5 : Observers (relay-discovery) ──────────────────────────────────

declare -a OBSERVER_LABELS=("relay1" "observer1" "observer2" "observer3")
declare -a OBSERVER_PORTS=("$STATUS_PORT_RELAY1" "$STATUS_PORT_OBS1" "$STATUS_PORT_OBS2" "$STATUS_PORT_OBS3")

for i in 0 1 2 3; do
    LABEL="${OBSERVER_LABELS[$i]}"
    STATUS_PORT="${OBSERVER_PORTS[$i]}"
    SLOT=$((i + 2))

    log "  [T${SLOT}] Lancement ${LABEL} (relay-discovery + bot)..."
    TOM_RELAY_URL="$RELAY_URL" RUST_LOG="$RUST_LOG" \
        "$TOM_CHAT" \
        --bot \
        --bot-ping "$BOT_PING_SECS" \
        --relay-discovery \
        --relay-ttl 600 \
        --node-label "${LABEL}" \
        --node-appareil "linux" \
        --log-udp "${COLLECTOR_IP}:${COLLECTOR_PORT}" \
        --status-port "$STATUS_PORT" \
        --username "${LABEL}" \
        > "$LOG_DIR/${LABEL}.out" 2>&1 &
    NODE_PID=$!
    echo "$NODE_PID" >> "$PID_FILE"
    log "  [T${SLOT}] ${LABEL}   : PID $NODE_PID (status: :$STATUS_PORT)"
    sleep 2
done

# ── Collecteur de logs ────────────────────────────────────────────────────

log ""
log "  Lancement collecteur de logs (port $COLLECTOR_PORT)..."
python3 ./scripts/collect-logs.sh "$COLLECTOR_PORT" --tail \
    > "$LOG_DIR/collecteur.out" 2>&1 &
COLL_PID=$!
# Note : collect-logs.sh est un script shell wrappant python3.
# Si non disponible, on lance directement le collecteur python minimal.
if ! kill -0 "$COLL_PID" 2>/dev/null; then
    python3 - "$COLLECTOR_PORT" "$LOG_DIR" "0" <<'PYEOF' > "$LOG_DIR/collecteur.out" 2>&1 &
import sys, socket, json, os
port = int(sys.argv[1]); logdir = sys.argv[2]
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind(("0.0.0.0", port))
all_log = open(os.path.join(logdir, "all.jsonl"), "a", buffering=1)
node_files = {}
def get_f(n):
    if n not in node_files:
        node_files[n] = open(os.path.join(logdir, f"{n}.jsonl"), "a", buffering=1)
    return node_files[n]
try:
    while True:
        data, _ = sock.recvfrom(65535)
        for line in data.decode("utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line: continue
            all_log.write(line + "\n")
            try:
                d = json.loads(line)
                get_f(d.get("node", "unknown")).write(line + "\n")
            except Exception:
                get_f("unknown").write(line + "\n")
except KeyboardInterrupt:
    pass
PYEOF
    COLL_PID=$!
fi
echo "$COLL_PID" >> "$PID_FILE"
log "  Collecteur : PID $COLL_PID"

# ── Résumé ────────────────────────────────────────────────────────────────

log ""
log "=== 5 nœuds + 1 relay lancés ==="
log ""
log "  Pages d'état :"
log "    curl http://localhost:$STATUS_PORT_PUBLISHER  # publisher"
log "    curl http://localhost:$STATUS_PORT_RELAY1     # relay1"
log "    curl http://localhost:$STATUS_PORT_OBS1       # observer1"
log "    curl http://localhost:$STATUS_PORT_OBS2       # observer2"
log "    curl http://localhost:$STATUS_PORT_OBS3       # observer3"
log ""
log "  Logs live :"
log "    tail -f $LOG_DIR/all.jsonl"
log "    tail -f $LOG_DIR/publisher.jsonl"
log ""
log "  Scénario endurance (6h) :"
log "    cargo run -p tom-stress -- endurance"
log ""
log "  Arrêter : ./scripts/launch-5nodes.sh stop"
log "  Ou : Ctrl+C"
log ""
log "=== En attente... Ctrl+C pour arrêter ==="
wait
