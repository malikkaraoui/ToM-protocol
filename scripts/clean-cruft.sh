#!/usr/bin/env bash
# clean-cruft.sh — Nettoyage anti-empilement du dépôt tom-protocol
#
# Supprime le cruft LOCAL non versionné qui s'accumule sur disque :
#   backups .claude obsolètes, worktrees orphelins, logs, .DS_Store, .bak.
# Optionnel (--builds) : purge aussi les artefacts de build régénérables
#   (target/ Rust, .build/ Swift) — ~60G.
#
# Usage :
#   bash scripts/clean-cruft.sh            # cruft mort uniquement (~14G), dry-run par défaut
#   bash scripts/clean-cruft.sh --apply    # exécute réellement
#   bash scripts/clean-cruft.sh --apply --builds   # + builds régénérables
#
# Aucune perte de code ou de travail versionné : tout est régénérable ou pur déchet.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

APPLY=0
BUILDS=0
BACKUP_MAX_AGE_DAYS=7   # backups .claude plus vieux que N jours

for arg in "$@"; do
  case "$arg" in
    --apply)  APPLY=1 ;;
    --builds) BUILDS=1 ;;
    *) echo "Argument inconnu: $arg" >&2; exit 1 ;;
  esac
done

run() {
  if [ "$APPLY" -eq 1 ]; then
    eval "$1"
  else
    echo "  [dry-run] $1"
  fi
}

echo "=== clean-cruft.sh ($([ "$APPLY" -eq 1 ] && echo APPLY || echo DRY-RUN)) ==="
echo "Racine: $ROOT"
BEFORE="$(du -sh . 2>/dev/null | cut -f1 || echo '?')"
echo "Taille avant: $BEFORE"
echo

# 1. Backups .claude obsolètes (date lue dans le NOM, pas le mtime — peu fiable)
echo "[1] Backups .claude obsolètes (> ${BACKUP_MAX_AGE_DAYS}j d'après le nom)"
now_epoch="$(date +%s)"
for d in .claude-backup-*; do
  [ -d "$d" ] || continue
  # extrait YYYYMMDD depuis .claude-backup-YYYYMMDD...
  ymd="$(echo "$d" | sed -nE 's/^\.claude-backup-([0-9]{8}).*/\1/p')"
  if [ -z "$ymd" ]; then
    echo "    ?? $d : date illisible dans le nom, ignoré"
    continue
  fi
  # macOS/BSD date d'abord, fallback GNU
  bk_epoch="$(date -j -f '%Y%m%d' "$ymd" +%s 2>/dev/null || date -d "$ymd" +%s 2>/dev/null || echo 0)"
  age_days=$(( (now_epoch - bk_epoch) / 86400 ))
  if [ "$bk_epoch" -gt 0 ] && [ "$age_days" -gt "$BACKUP_MAX_AGE_DAYS" ]; then
    run "rm -rf '$d'" && echo "    -> $d (${age_days}j)"
  else
    echo "    !! GARDÉ: $d (${age_days}j ≤ ${BACKUP_MAX_AGE_DAYS}j)"
  fi
done

# 2. Worktrees orphelins (aucun commit au-dessus de main)
echo "[2] Worktrees orphelins .claude/worktrees/*"
if [ -d .claude/worktrees ]; then
  for wt in .claude/worktrees/*/; do
    [ -d "$wt" ] || continue
    branch="claude/$(basename "$wt")"
    if git rev-parse --verify "$branch" >/dev/null 2>&1; then
      ahead="$(git log "main..$branch" --oneline 2>/dev/null | wc -l | tr -d ' ')"
    else
      ahead=0
    fi
    if [ "$ahead" = "0" ]; then
      run "rm -rf '$wt'" && echo "    -> $wt (0 commit non mergé)"
    else
      echo "    !! GARDÉ: $wt ($ahead commits non mergés sur $branch)"
    fi
  done
fi
run "git worktree prune"

# 3. Logs
echo "[3] Logs"
if [ -d logs ]; then
  run "rm -rf logs/*" && echo "    -> logs/*"
fi

# 4. Fichiers .bak et .DS_Store
echo "[4] .bak + .DS_Store"
run "find . -name '*.bak' -not -path './.git/*' -not -path '*/target/*' -delete"
run "find . -name '.DS_Store' -not -path './.git/*' -delete"

# 5. (optionnel) Builds régénérables
if [ "$BUILDS" -eq 1 ]; then
  echo "[5] Builds régénérables (cargo clean + .build Swift)"
  run "cargo clean"
  run "rm -rf crates/tom-protocol-ffi/target crates/tom-relay-ffi/target"
  run "rm -rf experiments/iroh-poc/target"
  run "find . -name '.build' -type d -prune -exec rm -rf {} +"
else
  echo "[5] Builds régénérables: IGNORÉS (ajouter --builds pour les purger)"
fi

echo
AFTER="$(du -sh . 2>/dev/null | cut -f1 || echo '?')"
echo "Taille avant: $BEFORE  ->  après: $AFTER"
[ "$APPLY" -eq 0 ] && echo "(dry-run — relancer avec --apply pour exécuter)"
