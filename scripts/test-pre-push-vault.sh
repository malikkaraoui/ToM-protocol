#!/usr/bin/env bash
# scripts/test-pre-push-vault.sh — témoins de la règle « vault/ seul » de .husky/pre-push (#71).
#
# Sans réseau et sans push : crée un dépôt jetable dans $(mktemp -d), y copie le
# VRAI .husky/pre-push, remplace scripts/ci-local.sh par un témoin qui écrit un
# fichier marqueur, puis lance le hook avec des refs sur stdin (format git
# pre-push). Le vrai ci-local.sh n'est jamais modifié ni exécuté.
set -euo pipefail

HOOK="${HOOK_PRE_PUSH:-$(git rev-parse --show-toplevel)/.husky/pre-push}"
[ -f "$HOOK" ] || { echo "hook introuvable : $HOOK" >&2; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "${TMP:?}"' EXIT
Z=0000000000000000000000000000000000000000

g() { git -C "$TMP/work" -c core.hooksPath=/dev/null -c commit.gpgsign=false "$@"; }
commit() { # commit <message> <fichier>... : écrit chaque fichier puis committe
	local msg="$1"; shift
	local f
	for f in "$@"; do mkdir -p "$(dirname "$TMP/work/$f")"; echo "$msg" >> "$TMP/work/$f"; done
	g add -A >/dev/null && g commit -qm "$msg" && g rev-parse HEAD
}

git init -q --bare "$TMP/remote.git"
git init -q -b main "$TMP/work"
g config user.email test@example.invalid
g config user.name test
mkdir -p "$TMP/work/scripts"
cat > "$TMP/work/scripts/ci-local.sh" <<'EOF'
#!/bin/sh
echo "[stub] ci-local lancée"
touch "$CI_MARK"
EOF
chmod +x "$TMP/work/scripts/ci-local.sh"
BASE="$(commit base README.md)"
g remote add origin "$TMP/remote.git"
g push -q origin main
g fetch -q origin

# Branches de travail, chacune partant de BASE.
branch() { g checkout -q -B "$1" "$BASE"; }
branch vault;   V="$(commit v vault/notes.md vault/revues/r.md)"
branch code;    C="$(commit c crates/x.rs)"
branch mixed;   M="$(commit m vault/notes.md scripts/y.sh)"
branch novault; N="$(commit n vault)"                       # fichier « vault » sans slash
branch ren;     mkdir -p "$TMP/work/vault"; g mv README.md vault/README.md; g commit -qm ren; R="$(g rev-parse HEAD)"
g checkout -q main

FAIL=0
# run <nom> <attendu: sautée|complète> <stdin>
run() {
	local name="$1" want="$2" refs="$3" out mark="$TMP/mark"
	rm -f "$mark"
	if ! out="$(cd "$TMP/work" && printf '%s' "$refs" | CI_MARK="$mark" sh -e "$HOOK" 2>&1)"; then
		echo "KO  $name : le hook a échoué"; echo "$out" | sed 's/^/    /'; FAIL=1; return
	fi
	local got
	if [ -e "$mark" ]; then got="complète"; else got="sautée"; fi
	if [ "$got" = "$want" ] && echo "$out" | grep -q "CI locale $want"; then
		echo "OK  $name → CI $got"
	else
		echo "KO  $name : attendu $want, obtenu $got"; echo "$out" | sed 's/^/    /'; FAIL=1
	fi
}

run "vault seul"                sautée   "refs/heads/v $V refs/heads/v $BASE"$'\n'
run "code seul"                 complète "refs/heads/c $C refs/heads/c $BASE"$'\n'
run "vault + code"              complète "refs/heads/m $M refs/heads/m $BASE"$'\n'
run "ref neuve vault seul"      sautée   "refs/heads/v $V refs/heads/v $Z"$'\n'
run "suppression de ref"        sautée   "(delete) $Z refs/heads/v $BASE"$'\n'
run "base introuvable"          complète "refs/heads/v $V refs/heads/v 1234567890123456789012345678901234567890"$'\n'
run "refs multiples vault+code" complète "refs/heads/v $V refs/heads/v $BASE"$'\n'"refs/heads/c $C refs/heads/c $BASE"$'\n'
run "suppression + code"        complète "(delete) $Z refs/heads/v $BASE"$'\n'"refs/heads/c $C refs/heads/c $BASE"$'\n'
run "renommage code → vault/"   complète "refs/heads/r $R refs/heads/r $BASE"$'\n'
run "fichier « vault » sans /"  complète "refs/heads/n $N refs/heads/n $BASE"$'\n'
run "aucun changement"          complète "refs/heads/v $BASE refs/heads/v $BASE"$'\n'
run "stdin vide"                complète ""
g update-ref -d refs/remotes/origin/main
run "ref neuve sans origin/main" complète "refs/heads/v $V refs/heads/v $Z"$'\n'

[ "$FAIL" = 0 ] && echo "TOUS LES TÉMOINS VERTS" || { echo "ÉCHEC"; exit 1; }
