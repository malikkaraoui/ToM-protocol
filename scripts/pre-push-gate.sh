#!/usr/bin/env bash
# scripts/pre-push-gate.sh — Pre-push gate for claude-atelier projects
#
# 5 checks: secrets → tracked sensitive files → lint → build → tests
# Exit codes: 0 = pass, 1 = blocked
#
# Usage:
#   bash scripts/pre-push-gate.sh          # run all 5 checks
#   bash scripts/pre-push-gate.sh --quick  # secrets + tracked files only (2 checks)
#
# Stack auto-detection: the script detects the stack from the project
# files and adjusts lint/build/test commands accordingly.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

pass()  { echo -e "${GREEN}[PASS]${NC} $1"; }
fail()  { echo -e "${RED}[FAIL]${NC} $1"; exit 1; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
step()  { echo -e "\n${YELLOW}[$1/5]${NC} $2"; }

QUICK=false
[[ "${1:-}" == "--quick" ]] && QUICK=true

# ─── 1/5 Secrets in staged diff ──────────────────────────────────────────────

step 1 "Audit secrets dans le diff..."

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

pass "Aucun secret detecte"

# ─── 2/5 Tracked sensitive files ─────────────────────────────────────────────

step 2 "Fichiers sensibles trackes..."

TRACKED_SENSITIVE=$(git ls-files 2>/dev/null | grep -E '\.(env|pem|key|p12|pfx|keystore)$' || true)

if [[ -n "$TRACKED_SENSITIVE" ]]; then
    echo "$TRACKED_SENSITIVE"
    fail "Fichiers sensibles trackes par git. Les retirer avec git rm --cached."
fi

pass "Aucun fichier sensible tracke"

# Quick mode stops here
if $QUICK; then
    echo -e "\n${GREEN}GATE RAPIDE PASSEE${NC} (2/5 checks, --quick mode)"
    exit 0
fi

# ─── Stack detection ─────────────────────────────────────────────────────────

detect_stack() {
    if [[ -f "package.json" ]]; then
        echo "node"
    elif [[ -f "pyproject.toml" ]] || [[ -f "requirements.txt" ]] || [[ -f "setup.py" ]]; then
        echo "python"
    elif [[ -f "pom.xml" ]]; then
        echo "maven"
    elif [[ -f "build.gradle" ]] || [[ -f "build.gradle.kts" ]]; then
        echo "gradle"
    else
        echo "unknown"
    fi
}

STACK=$(detect_stack)

# ─── 3/5 Lint ─────────────────────────────────────────────────────────────────

step 3 "Lint ($STACK)..."

case "$STACK" in
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

# ─── 4/5 Build ────────────────────────────────────────────────────────────────

step 4 "Build ($STACK)..."

case "$STACK" in
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

# ─── 5/5 Tests ─────────────────────────────────────────────────────────────────

step 5 "Tests ($STACK)..."

case "$STACK" in
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

# ─── Done ──────────────────────────────────────────────────────────────────────

echo -e "\n${GREEN}GATE PASSEE${NC} — push autorise"
exit 0
