#!/usr/bin/env bash
# Test d'endurance LOCAL et déterministe du backup/livraison-différée (ADR-009).
# Indépendant de la suspension des devices : sender + recipient sont des
# processus locaux que l'on contrôle. Boucle : R en ligne → S envoie (baseline)
# → R TUÉ → S envoie N messages (doivent survivre en backup) → attente →
# R RESSUSCITÉ (même identité) → mesure des messages livrés en différé.
set -u
cd /Users/malik/Documents/tom-protocol

BIN=./target/release/tom-chat
OUT=docs/test-campaign/soak_results.log
DIR=/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/e13bf67e-aa4e-4be2-9b17-4c7adda91f45/scratchpad/backup_soak
mkdir -p "$DIR"
RKEY="$DIR/recipient.key"   # identité stable du destinataire (survit aux redémarrages)

ts() { date "+%Y-%m-%d %H:%M:%S"; }
log() { echo "[$(ts)] $*" >> "$OUT"; }

# node_id d'un bot depuis son log (ligne bot_start)
node_of() { grep -m1 '"event":"bot_start"' "$1" 2>/dev/null | python3 -c "import sys,json;print(json.load(sys.stdin.__class__.__mro__ and sys.stdin)['node_id']) if False else print(json.loads(sys.stdin.readline())['node_id'])" 2>/dev/null; }
node_of2() { grep -m1 '^  node ' "$1" 2>/dev/null | awk '{print $2}'; }

# combien de messages 'S->R #k' R a-t-il reçus, filtre par plage de tailles
recv_count() { # $1=logfile du recipient
  python3 -c "
import json
n=0
for l in open('$1'):
    try: j=json.loads(l)
    except: continue
    if j.get('event')=='message_recu' and 'BACKUPTEST' in j.get('detail',''):
        n+=1
print(n)
" 2>/dev/null
}

log "════════════════════════════════════════════════════════════"
log "TEST BACKUP/LIVRAISON-DIFFÉRÉE LOCAL (ADR-009) — build 9"
log "════════════════════════════════════════════════════════════"

CYCLE=0
while true; do
  CYCLE=$((CYCLE+1))
  log ""
  log "┌─ BACKUP CYCLE $CYCLE ────────────────────────────────────"

  # 1. Destinataire R en ligne (identité stable)
  "$BIN" --bot --username dest --node-label dest --key-path "$RKEY" \
    > "$DIR/R.log" 2>&1 &
  RPID=$!
  sleep 20
  RNODE=$(node_of2 "$DIR/R.log")
  if [ -z "${RNODE:-}" ] || [ "${#RNODE}" -lt 60 ]; then
    log "│ ✗ destinataire n'a pas démarré — cycle avorté"; kill $RPID 2>/dev/null; sleep 30; continue
  fi
  log "│ destinataire R en ligne : ${RNODE:0:8}"

  # 2. Expéditeur S envoie 5 messages BASELINE (R en ligne → doit livrer)
  #    On encode 'BACKUPTEST' dans le payload via une taille marqueur unique.
  "$BIN" --bot --username fact --node-label fact --key-path "$DIR/S.key" \
    > "$DIR/S.log" 2>&1 &
  SPID=$!
  sleep 15
  # S découvre R et lui envoie via size-ramp (payloads numérotés). Le contenu
  # 'A'*n ne porte pas 'BACKUPTEST' — donc on marque autrement : on compte les
  # MSG reçus par R depuis S (node de S) autour des fenêtres online/offline.
  SNODE=$(node_of2 "$DIR/S.log")
  log "│ expéditeur S : ${SNODE:0:8}"

  # recompte de départ (messages déjà reçus par R depuis S)
  base_recv() { python3 -c "
import json
n=0
for l in open('$DIR/R.log'):
    try: j=json.loads(l)
    except: continue
    if j.get('event')=='message_recu' and '${SNODE:0:8}' in j.get('detail',''):
        n+=1
print(n)
" 2>/dev/null; }

  # Relancer S en mode rampe ciblée vers R (baseline 5 msgs)
  kill $SPID 2>/dev/null; sleep 2
  "$BIN" --bot --username fact --node-label fact --key-path "$DIR/S.key" \
    --size-ramp "$RNODE" --ramp-sizes "1001,1002,1003,1004,1005" \
    > "$DIR/S.log" 2>&1 &
  SPID=$!
  sleep 45
  R_ONLINE=$(base_recv)
  log "│ baseline (R en ligne) : R a reçu $R_ONLINE msg de S"

  # 3. TUER R → destinataire hors-ligne
  kill $RPID 2>/dev/null
  log "│ T0 : R TUÉ (hors-ligne)"
  sleep 3

  # 4. S envoie 15 messages vers R MORT (doivent être backupés/retentés)
  kill $SPID 2>/dev/null; sleep 2
  "$BIN" --bot --username fact --node-label fact --key-path "$DIR/S.key" \
    --size-ramp "$RNODE" --ramp-sizes "2001,2002,2003,2004,2005,2006,2007,2008,2009,2010,2011,2012,2013,2014,2015" \
    > "$DIR/S.log" 2>&1 &
  SPID=$!
  sleep 60
  S_SENT_OFFLINE=$(grep -c "size_ramp_ok" "$DIR/S.log" 2>/dev/null || echo 0)
  S_BACKUP=$(grep -c "backed up" "$DIR/S.log" 2>/dev/null || echo 0)
  log "│ S a émis $S_SENT_OFFLINE msgs vers R hors-ligne (backed up détectés: $S_BACKUP)"

  # 5. R reste mort 3 min (survie du message)
  log "│ R maintenu hors-ligne 3 min (survie du backup)"
  sleep 180

  # 6. RESSUSCITER R (même identité) → doit recevoir le backlog
  "$BIN" --bot --username dest --node-label dest --key-path "$RKEY" \
    > "$DIR/R2.log" 2>&1 &
  RPID=$!
  log "│ T+ : R RESSUSCITÉ (même identité) — attente livraison différée 4 min"
  sleep 240

  # 7. Mesure : combien de msgs 'offline' R a-t-il reçus après résurrection ?
  R_AFTER=$(python3 -c "
import json
n=0
for l in open('$DIR/R2.log'):
    try: j=json.loads(l)
    except: continue
    if j.get('event')=='message_recu' and '${SNODE:0:8}' in j.get('detail',''):
        n+=1
print(n)
" 2>/dev/null)
  log "│ ═══ RÉSULTAT CYCLE $CYCLE ═══"
  log "│   baseline (R en ligne)      : $R_ONLINE reçus"
  log "│   émis vers R hors-ligne     : $S_SENT_OFFLINE (backed up: $S_BACKUP)"
  log "│   livrés en différé au retour: $R_AFTER"
  if [ "${R_AFTER:-0}" -ge 1 ]; then
    log "│   ✅ LIVRAISON DIFFÉRÉE FONCTIONNELLE ($R_AFTER msgs récupérés après retour)"
  else
    log "│   ⚠️ AUCUN message backupé livré au retour — ADR-009 à investiguer"
  fi

  # nettoyage
  kill $RPID $SPID 2>/dev/null; sleep 5
  log "└─ FIN BACKUP CYCLE $CYCLE ───────────────────────────────"
done
