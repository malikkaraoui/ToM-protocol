#!/usr/bin/env bash
# Test E2E R13 — livraison de groupe différée (offline gap-fill), réseau réel.
#
# Monte un triangle isolé (hub + 2 membres) de vrais nœuds `tom-chat` sur
# localhost, pilotés par l'API HTTP de contrôle, et vérifie qu'un membre parti
# hors-ligne rattrape via SyncRequest/SyncResponse les messages émis pendant son
# absence — après un RESTART (persistance SQLite de l'appartenance + last_seq).
#
# C'est la régression de deux bugs que les tests unitaires ne voyaient pas :
#   1. /stop ne flushait pas l'état → appartenance perdue au restart
#      (fix: RuntimeHandle::save_now avant l'exit).
#   2. le rejoin de démarrage était émis AVANT toute connectivité → SyncRequest
#      dans le vide (fix: rejoin différé dans reconnect_check, gaté sur
#      topology.online_count() > 0).
#
# Prérequis : cargo build -p tom-tui (binaire target/debug/tom-chat).
# Usage     : bash scripts/test-r13-offline.sh
# Sortie    : "R13 OK" (exit 0) ou "R13 ECHEC" (exit 1).
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/tom-chat"
DIR="$(mktemp -d "${TMPDIR:-/tmp}/r13test.XXXXXX")"
RELAY="http://127.0.0.1:3401"
trap 'for p in 9101 9102 9103; do curl -s --max-time 2 "http://127.0.0.1:$p/stop" >/dev/null 2>&1; done; kill $(jobs -p) 2>/dev/null; rm -rf "$DIR"' EXIT

[ -x "$BIN" ] || { echo "!! binaire absent: $BIN — lance: cargo build -p tom-tui"; exit 1; }
get() { curl -s --max-time 4 "http://127.0.0.1:$1$2"; }

start_node() { # $1=label $2=ctrl $3=status ...extra
  local label=$1 ctrl=$2 stat=$3; shift 3
  TOM_RELAY_URL="$RELAY" "$BIN" --isolated --bot \
    --control-port "$ctrl" --status-port "$stat" \
    --data-dir "$DIR/$label" --key-path "$DIR/$label.key" \
    --node-label "$label" "$@" > "$DIR/$label.log" 2>&1 &
}

wait_id() { # $1=statusport
  local id="" i
  for i in $(seq 1 40); do
    id=$(get "$1" "/" | grep -o '"node_id":"[^"]*"' | head -1 | cut -d'"' -f4)
    [ -n "$id" ] && { echo "$id"; return 0; }
    sleep 0.5
  done
  return 1
}

start_node hub 9101 9201 --embedded-relay --embedded-relay-bind 127.0.0.1:3401
HUB=$(wait_id 9201) || { echo "!! hub KO"; exit 1; }
start_node m1 9102 9202 --bootstrap "$HUB"
start_node m2 9103 9203 --bootstrap "$HUB"
M1=$(wait_id 9202) || { echo "!! m1 KO"; exit 1; }
M2=$(wait_id 9203) || { echo "!! m2 KO"; exit 1; }
sleep 12  # maillage gossip via relais

get 9101 "/group/create?name=r13&members=$M1,$M2" >/dev/null
sleep 2
GID=$(get 9102 "/invites" | grep -o '"group_id":"[^"]*"' | head -1 | cut -d'"' -f4)
[ -n "$GID" ] || { echo "!! pas d'invite reçue"; exit 1; }
get 9102 "/group/accept?group=$GID" >/dev/null
get 9103 "/group/accept?group=$GID" >/dev/null
sleep 2

# SANITY online : 3 messages, m2 doit les recevoir.
for k in 1 2 3; do get 9102 "/group/send?group=$GID&size=40" >/dev/null; sleep 0.4; done
sleep 3
SAN=$(get 9103 "/inbox?contains=CTRL" | grep -o '"total":[0-9]*' | cut -d: -f2)
[ "${SAN:-0}" -ge 3 ] || { echo "!! SANITY: m2 devait recevoir >=3, a $SAN"; exit 1; }
echo "SANITY ok ($SAN messages online)"

# R13 : m2 offline, m1 envoie 5, m2 restart -> rattrapage.
get 9103 "/stop" >/dev/null; sleep 2
for k in 1 2 3 4 5; do get 9102 "/group/send?group=$GID&size=64" >/dev/null; sleep 0.4; done
sleep 2
start_node m2 9103 9203 --bootstrap "$HUB"
wait_id 9203 >/dev/null || { echo "!! m2 restart KO"; exit 1; }
sleep 30  # 1er reconnect_check (15s) déclenche le rejoin différé + gap-fill

TOTAL=$(get 9103 "/inbox?contains=CTRL" | grep -o '"total":[0-9]*' | cut -d: -f2)
# Après restart l'inbox repart de zéro : on doit voir AU MOINS les 5 rattrapés.
if [ "${TOTAL:-0}" -ge 5 ]; then
  echo "R13 OK — m2 a rattrapé $TOTAL messages offline via gap-fill"
  exit 0
else
  echo "R13 ECHEC — m2 n'a rattrapé que ${TOTAL:-0} messages (attendu >=5)"
  echo "logs: $DIR/*.log"
  exit 1
fi
