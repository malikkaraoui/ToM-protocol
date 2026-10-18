#!/usr/bin/env bash
# scripts/pre-push-gate.sh — Pre-push gate for claude-atelier projects
#
# CANONIQUE : ce fichier vit dans claude-atelier et est distribué à l'identique
# dans tous les projets. Toute amélioration se fait ICI puis se redistribue —
# jamais dans une copie locale (leçon : 4 variantes divergentes découvertes le
# 2026-08-06, chacune avec une amélioration que les autres n'avaient pas).
#
# 6 checks : [0] sync remote + email auteur → [1] secrets (diff + full-tree)
#          → [2] fichiers sensibles trackés → [3] lint → [4] build → [5] tests
#          → [6] review (pluggable)
# Exit codes: 0 = pass, 1 = blocked
#
# Usage:
#   bash scripts/pre-push-gate.sh          # tous les checks
#   bash scripts/pre-push-gate.sh --quick  # checks 0-2 uniquement
#
# Spécifique projet : si scripts/pre-push-gate-step6.sh existe, il est sourcé
# comme étape 6 (ex. workflow handoff Copilot de Boites-a-livres). Le reste du
# fichier doit rester identique au canonique.

set -euo pipefail

GATE_VERSION="2.0.0"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

pass()  { echo -e "${GREEN}[PASS]${NC} $1"; }
fail()  { echo -e "${RED}[FAIL]${NC} $1"; exit 1; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
step()  { echo -e "\n${YELLOW}[$1/6]${NC} $2"; }

echo -e "pre-push-gate v${GATE_VERSION}"

QUICK=false
[[ "${1:-}" == "--quick" ]] && QUICK=true

# ─── 0/6-a Remote sync check ──────────────────────────────────────────────────
# Si le remote a des commits que le local n'a pas, les intégrer maintenant
# plutôt que de se faire rejeter après 6 checks.
if git rev-parse --abbrev-ref --symbolic-full-name '@{u}' >/dev/null 2>&1; then
    git fetch --quiet 2>/dev/null || true
    LOCAL=$(git rev-parse HEAD 2>/dev/null)
    REMOTE=$(git rev-parse '@{u}' 2>/dev/null || echo "$LOCAL")
    BASE=$(git merge-base "$LOCAL" "$REMOTE" 2>/dev/null || echo "$LOCAL")
    if [[ "$LOCAL" == "$REMOTE" ]]; then
        : # Déjà synchronisé, rien à faire
    elif [[ "$BASE" == "$LOCAL" ]]; then
        # Remote ahead, local behind → rebase simple
        echo -e "${YELLOW}[SYNC]${NC} Remote a des commits non intégrés — rebase automatique..."
        if ! git pull --rebase --quiet; then
            git rebase --abort 2>/dev/null || true
            echo -e "${RED}[FAIL]${NC} Rebase échoué et annulé. Résous manuellement puis relance."
            exit 1
        fi
        echo -e "${GREEN}[SYNC]${NC} Rebase OK — local à jour avec remote."
    elif [[ "$BASE" == "$REMOTE" ]]; then
        : # Local ahead (fast-forward possible), rien à faire
    else
        # Divergence réelle : local ET remote ont chacun des commits
        if [[ "${FORCE_PUSH:-}" == "1" ]]; then
            echo -e "${YELLOW}[SYNC]${NC} FORCE_PUSH=1 — divergence acceptée (rebase intentionnel)."
        else
            echo -e "${RED}[FAIL]${NC} Branches divergentes (local et remote ont chacun des commits)."
            echo -e "         Fais 'git pull --rebase' manuellement, ou FORCE_PUSH=1 si rebase intentionnel."
            exit 1
        fi
    fi
fi

# ─── 0/6-b Email d'auteur (anti-typo hmail) ──────────────────────────────────
# Un typo `hmail` (au lieu de `gmail`) a rendu 172 commits NON ATTRIBUÉS sur
# GitHub, sur DEUX projets à 24 h d'écart (1RR : 39, partie politique : 133) —
# deux réécritures d'historique pour le corriger. Ce check bloque tout commit à
# pousser dont l'auteur OU le committer n'est pas l'email attendu. Les comptes
# de service (github noreply, bots) sont tolérés.
# Override : GATE_ALLOWED_EMAIL env, ou `git config gate.allowedemail`.

step 0 "Sync remote + email d'auteur..."

ALLOWED_EMAIL="${GATE_ALLOWED_EMAIL:-$(git config gate.allowedemail 2>/dev/null || echo "karaoui.malik@gmail.com")}"
EMAIL_RANGE="origin/main..HEAD"
git rev-parse --verify -q origin/main >/dev/null 2>&1 || EMAIL_RANGE="HEAD"
BAD_EMAILS=$(git log "$EMAIL_RANGE" --format='%ae%n%ce' 2>/dev/null | sort -u \
    | grep -vE "^(${ALLOWED_EMAIL}|[0-9]+\+.*@users\.noreply\.github\.com|.*\[bot\]@.*|noreply@github\.com)$" || true)
if [[ -n "$BAD_EMAILS" ]]; then
    echo "  Emails non autorisés détectés :"
    echo "$BAD_EMAILS" | sed 's/^/    /'
    fail "Email d'auteur/committer ≠ ${ALLOWED_EMAIL}. Corriger AVANT push (git commit --amend --reset-author, ou filter-branch pour plusieurs commits)."
fi
pass "Sync remote OK · emails d'auteur OK (${ALLOWED_EMAIL})"

# ─── Auto-sync SECURITY.md ────────────────────────────────────────────────────
if [[ -f "scripts/update-security.js" ]] && [[ -f "package.json" ]]; then
    node scripts/update-security.js 2>/dev/null || true
fi

# ─── 1/6 Secrets ──────────────────────────────────────────────────────────────

step 1 "Audit secrets (diff + full-tree)..."

SECRET_PATTERNS='(api_key|api[-_]?secret|secret[-_]?key|access[-_]?token|auth[-_]?token|password|private_key|BEGIN RSA|BEGIN OPENSSH|BEGIN EC|sk-[a-zA-Z0-9]{20,}|AIza[a-zA-Z0-9_-]{35}|AKIA[A-Z0-9]{16}|ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|ya29\.[a-zA-Z0-9_-]+)'

if git diff --cached --diff-filter=ACMR 2>/dev/null | grep -iEq "$SECRET_PATTERNS"; then
    echo ""
    git diff --cached --diff-filter=ACMR | grep -iE "$SECRET_PATTERNS" | head -20
    echo ""
    fail "SECRET DETECTE dans le diff staged. Push annule."
fi

# Also check unstaged changes that would be pushed
if git diff HEAD 2>/dev/null | grep -iEq "$SECRET_PATTERNS"; then
    warn "Pattern suspect detecte dans les changements non staged. Verifier avant push."
fi

# Filet full-tree : un secret DEJA commite (hors diff) doit aussi bloquer le push.
# Le scan de diff ci-dessus rate les clés introduites lors d'un commit anterieur
# et jamais re-touchees (cas de la clé web Firebase du 10/07). On rescanne donc
# TOUT l'arbre tracké, mais uniquement sur des formats de clés à haute confiance,
# pour éviter les faux positifs des mots génériques (api_key, password...).
HARDSECRET='(sk-[a-zA-Z0-9]{20,}|AIza[a-zA-Z0-9_-]{35}|AKIA[A-Z0-9]{16}|ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|ya29\.[a-zA-Z0-9_-]+|-----BEGIN (RSA|OPENSSH|EC) PRIVATE KEY-----)'
if git grep -InE "$HARDSECRET" -- ':!scripts/pre-push-gate.sh' 2>/dev/null | grep -q .; then
    echo ""
    git grep -InE "$HARDSECRET" -- ':!scripts/pre-push-gate.sh' 2>/dev/null \
        | sed -E 's/(AIza|AKIA|sk-|ghp_|gho_)[A-Za-z0-9_-]+/\1…[masque]/g' | head -20
    echo ""
    fail "SECRET present dans un fichier tracke (hors diff). Rotation + purge necessaires."
fi

pass "Aucun secret detecte"

# ─── 2/6 Tracked sensitive files ─────────────────────────────────────────────

step 2 "Fichiers sensibles trackes..."

TRACKED_SENSITIVE=$(git ls-files 2>/dev/null | grep -E '\.(env|pem|key|p12|pfx|keystore)$' || true)

if [[ -n "$TRACKED_SENSITIVE" ]]; then
    echo "$TRACKED_SENSITIVE"
    fail "Fichiers sensibles trackes par git. Les retirer avec git rm --cached."
fi

pass "Aucun fichier sensible tracke"

# Quick mode stops here
if $QUICK; then
    echo -e "\n${GREEN}GATE RAPIDE PASSEE${NC} (checks 0-2, --quick mode)"
    exit 0
fi

# ─── Stack detection ─────────────────────────────────────────────────────────
#
# Un repo peut porter plusieurs stacks à la fois (ex. workspace Rust + package.json
# legacy à la racine) — détecter TOUTES et gater sur chacune, au lieu de s'arrêter
# à la première. Un simple `unknown` faisait sauter lint/build/test Rust en silence
# dès qu'un package.json était aussi présent.

detect_stacks() {
    local stacks=()
    [[ -f "Cargo.toml" ]] && stacks+=("rust")
    [[ -f "package.json" ]] && stacks+=("node")
    { [[ -f "pyproject.toml" ]] || [[ -f "requirements.txt" ]] || [[ -f "setup.py" ]]; } && stacks+=("python")
    [[ -f "pom.xml" ]] && stacks+=("maven")
    { [[ -f "build.gradle" ]] || [[ -f "build.gradle.kts" ]]; } && stacks+=("gradle")
    if [[ ${#stacks[@]} -eq 0 ]]; then
        echo "unknown"
    else
        echo "${stacks[@]}"
    fi
}

read -ra STACKS <<< "$(detect_stacks)"

# ─── 3/6 Lint ─────────────────────────────────────────────────────────────────

step 3 "Lint (${STACKS[*]})..."

for STACK in "${STACKS[@]}"; do
case "$STACK" in
    rust)
        if cargo clippy --workspace -- -D warnings 2>&1 | tail -30; then
            pass "Lint OK (cargo clippy)"
        else
            fail "Lint echoue (cargo clippy)"
        fi
        ;;
    node)
        if npm run lint --if-present 2>&1 | tail -5; then
            pass "Lint OK"
        else
            fail "Lint echoue"
        fi
        ;;
    python)
        if command -v ruff &>/dev/null; then
            if ruff check . 2>&1 | tail -5; then
                pass "Lint OK (ruff)"
            else
                fail "Lint echoue (ruff)"
            fi
        elif command -v flake8 &>/dev/null; then
            if flake8 . 2>&1 | tail -5; then
                pass "Lint OK (flake8)"
            else
                fail "Lint echoue (flake8)"
            fi
        else
            warn "Pas de linter Python installe (ruff ou flake8). Lint skippe."
        fi
        ;;
    maven)
        if mvn checkstyle:check -q 2>&1 | tail -5; then
            pass "Lint OK"
        else
            fail "Lint echoue"
        fi
        ;;
    gradle)
        if ./gradlew checkstyleMain -q 2>&1 | tail -5; then
            pass "Lint OK"
        else
            fail "Lint echoue"
        fi
        ;;
    *)
        warn "Stack non reconnue. Lint skippe."
        ;;
esac
done

# ─── 4/6 Build ────────────────────────────────────────────────────────────────

step 4 "Build (${STACKS[*]})..."

for STACK in "${STACKS[@]}"; do
case "$STACK" in
    rust)
        if cargo build --workspace 2>&1 | tail -20; then
            pass "Build OK (cargo build)"
        else
            fail "Build echoue (cargo build)"
        fi
        ;;
    node)
        if npm run build --if-present 2>&1 | tail -10; then
            pass "Build OK"
        else
            fail "Build echoue"
        fi
        ;;
    python)
        warn "Pas de step de build standard pour Python. Skippe."
        ;;
    maven)
        if mvn compile -q 2>&1 | tail -10; then
            pass "Build OK"
        else
            fail "Build echoue"
        fi
        ;;
    gradle)
        if ./gradlew compileJava -q 2>&1 | tail -10; then
            pass "Build OK"
        else
            fail "Build echoue"
        fi
        ;;
    *)
        warn "Stack non reconnue. Build skippe."
        ;;
esac
done

# ─── 5/6 Tests ────────────────────────────────────────────────────────────────

step 5 "Tests (${STACKS[*]})..."

for STACK in "${STACKS[@]}"; do
case "$STACK" in
    rust)
        # HERMÉTICITÉ : la gate LOCALE ne lance QUE les tests unitaires
        # (--lib, hermétiques). Les tests d'INTÉGRATION (tests/) peuvent créer
        # de vrais nœuds/serveurs qui touchent le LAN ou des services partagés
        # (incident tom-protocol : des nœuds de test rejoignaient la flotte de
        # PROD). La CI (runner isolé) exécute la couverture d'intégration.
        if cargo test --workspace --lib 2>&1 | tail -30; then
            pass "Tests OK (cargo test --lib ; intégration → CI isolée)"
        else
            fail "Tests echoues (cargo test --lib)"
        fi
        ;;
    node)
        if npm test -- --passWithNoTests 2>&1 | tail -20; then
            pass "Tests OK"
        else
            fail "Tests echoues"
        fi
        ;;
    python)
        if command -v pytest &>/dev/null; then
            if pytest --tb=short -q 2>&1 | tail -20; then
                pass "Tests OK (pytest)"
            else
                fail "Tests echoues (pytest)"
            fi
        else
            warn "pytest non installe. Tests skippes."
        fi
        ;;
    maven)
        if mvn test -q 2>&1 | tail -20; then
            pass "Tests OK"
        else
            fail "Tests echoues"
        fi
        ;;
    gradle)
        if ./gradlew test -q 2>&1 | tail -20; then
            pass "Tests OK"
        else
            fail "Tests echoues"
        fi
        ;;
    *)
        warn "Stack non reconnue. Tests skippes."
        ;;
esac
done

# ─── 6/6 Review (pluggable) ──────────────────────────────────────────────────

step 6 "Review..."

STEP6_LOCAL="$(cd "$(dirname "$0")" && pwd)/pre-push-gate-step6.sh"
if [[ -f "$STEP6_LOCAL" ]]; then
    # Étape 6 spécifique au projet (ex. dette handoff Copilot)
    # shellcheck disable=SC1090
    source "$STEP6_LOCAL"
elif [[ -f ".claude/hooks/guard-review-auto.sh" ]] || [[ -f "hooks/guard-review-auto.sh" ]]; then
    # Gate §25 = hook guard-review-auto.sh (PreToolUse sur git push, exit 2 si diff ≥ 50 lignes)
    # Cette étape est informative uniquement — le hook est le vrai verrou.
    pass "Review oracle gate active via hook guard-review-auto.sh — §25 OK"
else
    warn "Pas de review gate configurée (ni step6 local, ni hook guard-review-auto). Voir §25."
fi

# ─── Done ──────────────────────────────────────────────────────────────────────

echo -e "\n${GREEN}GATE PASSEE${NC} — push autorise"
exit 0
