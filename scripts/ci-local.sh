#!/usr/bin/env bash
# scripts/ci-local.sh — rejoue en local les jobs de .github/workflows/ci.yml.
#
# Mêmes commandes, même ordre, mêmes options que la CI. S'arrête au premier
# échec (code ≠ 0) et affiche la durée de chaque étape.
#
# La CI n'a aucun filtre de chemins (`paths:`). Elle saute ses jobs de test
# uniquement quand seul vault/ change (job `changes`) ; ce script, lui, les
# lance toujours tous. Il n'existe volontairement aucun moyen d'en sauter un.
# C'est le hook .husky/pre-push qui ne l'appelle pas quand le push ne touche
# que vault/ (même règle que `changes`, #71 ; témoins : test-pre-push-vault.sh).
# Les deux méta-jobs de la CI ne sont pas rejoués : `changes` (détection) et
# `harnais-ok` (agrégat des résultats, seul check requis sur main). Si ci.yml
# change (job ajouté, retiré, filtre de chemins), le contrôle de dérive
# ci-dessous échoue : mettre ce script à jour.
#
# Écarts inévitables avec les runners GitHub (signalés, pas masqués) :
#   - ubuntu-latest → poste macOS : le job rust-macos est fidèle, les autres
#     tournent sur l'OS local ;
#   - Node 20 en CI : la version locale est affichée, un écart est signalé ;
#   - cargo-deny-action@v2 → binaire `cargo deny` local, mêmes arguments.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
CI_YML=".github/workflows/ci.yml"

# Ordre identique à ci.yml.
JOBS=(typescript observability-smoke rust-protocol rust-transport rust-integration
	rust-fork rust-dht rust-security rust-ffi rust-macos rust-stress rust-poc)
# Méta-jobs de ci.yml sans commande de test : jamais rejoués, exclus du contrôle de dérive.
META_JOBS=(changes harnais-ok)

SUMMARY=()
CURRENT=""

die() {
	echo "[ci-local] ERREUR : $*" >&2
	exit 1
}

on_exit() {
	local rc=$?
	echo
	echo "[ci-local] ===== Durées ====="
	local line
	for line in "${SUMMARY[@]+"${SUMMARY[@]}"}"; do echo "  $line"; done
	if [ "$rc" -ne 0 ]; then
		local where=""
		[ -z "$CURRENT" ] || where=" à l'étape : $CURRENT"
		echo "[ci-local] ÉCHEC (code $rc)$where — pas de push."
	else
		echo "[ci-local] TOUT VERT."
	fi
}
trap on_exit EXIT

# step <libellé> <commande...> : exécute, chronomètre, échoue au premier rouge.
step() {
	local label="$1"
	shift
	CURRENT="$label"
	echo
	echo "[ci-local] ▶ $label"
	echo "[ci-local]   \$ $*"
	local start=$SECONDS
	"$@"
	SUMMARY+=("$(printf '%5ss  %s' "$((SECONDS - start))" "$label")")
	CURRENT=""
}

# in_dir <dossier> <commande...> : équivalent de `working-directory:`.
in_dir() {
	local dir="$1"
	shift
	(cd "$dir" && "$@")
}

# with_timeout <minutes> <commande...> : équivalent de `timeout-minutes:`.
with_timeout() {
	local minutes="$1"
	shift
	local bin
	bin="$(command -v timeout || command -v gtimeout || true)"
	[ -n "$bin" ] || die "timeout/gtimeout introuvable (brew install coreutils)"
	"$bin" "$((minutes * 60))" "$@"
}

need() {
	command -v "$1" >/dev/null 2>&1 || die "outil requis absent : $1 ($2)"
}

# --- Contrôle de dérive : ce script doit refléter ci.yml ---------------------
# Liste les ids de jobs (clés à 2 espaces sous `jobs:`), dans l'ordre du fichier.
AWK_JOBS='/^jobs:/ { in_jobs = 1; next }
in_jobs && /^[^ \t]/ { in_jobs = 0 }
in_jobs && /^  [A-Za-z0-9_-]+:[ \t]*$/ { sub(/^  /, ""); sub(/:.*/, ""); print }'

check_drift() {
	[ -f "$CI_YML" ] || die "$CI_YML introuvable"
	if grep -Eq '^[[:space:]]+paths(-ignore)?:' "$CI_YML"; then
		die "$CI_YML filtre désormais par chemins : adapter ci-local.sh"
	fi
	local ci_jobs mine
	ci_jobs="$(awk "$AWK_JOBS" "$CI_YML" | grep -vxF -f <(printf '%s\n' "${META_JOBS[@]}"))"
	mine="$(printf '%s\n' "${JOBS[@]}")"
	if [ "$ci_jobs" != "$mine" ]; then
		echo "[ci-local] jobs ci.yml : $(echo "$ci_jobs" | tr '\n' ' ')" >&2
		echo "[ci-local] jobs script : $(echo "$mine" | tr '\n' ' ')" >&2
		die "dérive entre ci.yml et ci-local.sh (jobs ou ordre) : mettre le script à jour"
	fi
}

# --- Jobs (un par job de ci.yml) ---------------------------------------------
job_typescript() {
	need pnpm "corepack enable"
	local node_ver
	node_ver="$(node --version)"
	node_ver="${node_ver#v}"
	[ "${node_ver%%.*}" = "20" ] || echo "[ci-local] note : Node local v$node_ver, CI = Node 20"
	step "typescript: pnpm install --frozen-lockfile" pnpm install --frozen-lockfile
	step "typescript: pnpm build" pnpm build
	step "typescript: pnpm test" pnpm test
	step "typescript: pnpm lint" pnpm lint
}

job_observability_smoke() {
	need lsof "requis par le smoke"
	chmod +x scripts/run-observability-smoke-local.sh
	step "observability-smoke" ./scripts/run-observability-smoke-local.sh
}

job_rust_protocol() {
	step "rust-protocol: build" cargo build -p tom-protocol
	step "rust-protocol: test" cargo test -p tom-protocol
	step "rust-protocol: clippy" cargo clippy -p tom-protocol -- -D warnings
}

job_rust_transport() {
	step "rust-transport: build" cargo build -p tom-transport
	step "rust-transport: test" cargo test -p tom-transport -- --test-threads=1
	step "rust-transport: clippy" cargo clippy -p tom-transport -- -D warnings
}

job_rust_integration() {
	step "rust-integration: build" cargo build -p tom-integration-tests --tests
	step "rust-integration: test (timeout 10 min)" \
		with_timeout 10 cargo test -p tom-integration-tests -- --test-threads=1
	step "rust-integration: clippy" cargo clippy -p tom-integration-tests --tests -- -D warnings
}

job_rust_fork() {
	local fork=(-p tom-base -p tom-metrics -p tom-quinn-proto -p tom-quinn -p tom-connect -p tom-relay -p tom-gossip)
	step "rust-fork: build" cargo build "${fork[@]}"
	step "rust-fork: test tom-base" cargo test -p tom-base
	step "rust-fork: test tom-metrics" cargo test -p tom-metrics
	step "rust-fork: test tom-quinn-proto" cargo test -p tom-quinn-proto --lib
	step "rust-fork: test tom-quinn" cargo test -p tom-quinn --lib
	step "rust-fork: test tom-relay" cargo test -p tom-relay --lib
	step "rust-fork: test tom-gossip" cargo test -p tom-gossip --lib
	step "rust-fork: test tom-connect" cargo test -p tom-connect --lib
	step "rust-fork: clippy" cargo clippy "${fork[@]}" -- -D warnings
}

job_rust_dht() {
	step "rust-dht: build" cargo build -p tom-dht
	step "rust-dht: test" cargo test -p tom-dht --lib
	step "rust-dht: clippy" cargo clippy -p tom-dht -- -D warnings
}

job_rust_security() {
	need cargo-deny "cargo install cargo-deny"
	# Arguments effectifs de cargo-deny-action@v2 (défauts de son action.yml, visibles dans le log CI).
	step "rust-security: cargo deny" cargo deny --log-level warn --manifest-path ./Cargo.toml \
		--all-features check advisories bans sources
}

job_rust_ffi() {
	need cbindgen "cargo install cbindgen"
	step "rust-ffi: build tom-relay-ffi" cargo build -p tom-relay-ffi
	step "rust-ffi: clippy tom-relay-ffi" cargo clippy -p tom-relay-ffi -- -D warnings
	step "rust-ffi: build tom-protocol-ffi" in_dir crates/tom-protocol-ffi cargo build --locked
	step "rust-ffi: clippy tom-protocol-ffi" in_dir crates/tom-protocol-ffi cargo clippy --locked -- -D warnings
	step "rust-ffi: header drift" ./scripts/generate-ffi-header.sh --check
}

job_rust_macos() {
	step "rust-macos: build tom-relay-ffi" cargo build -p tom-relay-ffi
	step "rust-macos: build tom-protocol-ffi" in_dir crates/tom-protocol-ffi cargo build --locked
}

job_rust_stress() {
	step "rust-stress: build" cargo build -p tom-stress
	step "rust-stress: clippy" cargo clippy -p tom-stress -- -D warnings
	step "rust-stress: localhost test (timeout 5 min)" with_timeout 5 crates/tom-stress/scripts/test-localhost.sh
}

job_rust_poc() {
	local poc=experiments/iroh-poc
	step "rust-poc: debug target directory" in_dir "$poc" sh -c 'echo "pwd=$(pwd)"
		cargo metadata --format-version 1 --no-deps | python3 -c "import sys, json; print(\"target_directory=\" + json.load(sys.stdin)[\"target_directory\"])"'
	step "rust-poc: build" in_dir "$poc" cargo build --bin nat-test
	step "rust-poc: clippy" in_dir "$poc" cargo clippy --bin nat-test -- -D warnings
	step "rust-poc: localhost NAT test (timeout 3 min)" in_dir "$poc" with_timeout 3 ./scripts/test-localhost.sh
}

main() {
	check_drift
	need cargo "rustup"
	echo "[ci-local] toolchain : $(rustc --version) (épinglée par rust-toolchain.toml)"
	echo "[ci-local] base de comparaison : $(git rev-parse --short origin/main 2>/dev/null || echo '?') → HEAD $(git rev-parse --short HEAD)"
	local job
	for job in "${JOBS[@]}"; do
		echo
		echo "[ci-local] ===== job $job ====="
		"job_${job//-/_}"
	done
}

main "$@"
