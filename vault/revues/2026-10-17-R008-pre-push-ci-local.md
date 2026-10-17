---
date: 2026-10-17
revue: R008
mandat: M0010
branche: chore/pre-push-ci-local
verdict: GO
tip: dce9ce1464456a10916d3346ba93d97ec47d6c35
niveau: N2
---

# Revue R008 — pre-push ci-local (chore/pre-push-ci-local)

Relecture indépendante (N2) en lecture seule, worktree détaché `/Users/malik/Documents/tom-protocol-scratch-R008` (HEAD = `dce9ce1`). Aucun build, test, cargo, pnpm install, commit ni push. Le script `ci-local.sh` n'a pas été exécuté ; les vérifications sont statiques (lecture, grep, awk sur `ci.yml`).

Écart vs `origin/main` (`944819f`) : `.husky/pre-push` (+3 lignes) et `CONTRIBUTING.md` (+6 lignes), rien d'autre (`git diff 944819f HEAD --stat`).

## Vérifications

| # | Point | Résultat | Preuve (fichier:ligne) |
|---|-------|----------|------------------------|
| 1a | Hook lance `ci-local.sh` en premier | OK | `.husky/pre-push:3` (`set -eu`), `:6` commentaire, `:7` `./scripts/ci-local.sh` ; aucune instruction avant ligne 7 hors `set` |
| 1b | Code ≠ 0 interrompt le hook | OK | `set -e` actif (`pre-push:3`) : un retour ≠ 0 de la ligne 7 termine le hook avant le smoke ; `ci-local.sh:17` `set -euo pipefail` ; `ci-local.sh:41-44` le trap EXIT conserve `rc` et n'avale rien |
| 1c | Aucune variable/option ne saute le contrôle | OK | `ci-local.sh` ne lit aucune variable d'environnement métier (seul `$SECONDS`, `:59`) ; `SKIP_OBSERVABILITY_SMOKE` et `FORCE_OBSERVABILITY_SMOKE` ne sont lus qu'aux `pre-push:10` et `:25`, donc après la ligne 7 |
| 1d | Permissions | OK | `git ls-files -s` : `.husky/pre-push` et `scripts/ci-local.sh` en `100755` |
| 2a | Échappatoires dans `ci-local.sh` | Aucune trouvée | `grep` `exit 0` : aucun ; `|| true` : seulement `:77` (`command -v timeout \|\| gtimeout \|\| true`, sans effet sur le verdict : absence → `die` `:78`) ; `need` `:82-84` échoue si outil absent ; `with_timeout` `:73-80` propage le code (124 de timeout → échec) |
| 2b | Échec propagé | OK | `step` `:52-63` : la commande est lancée sans protection, `set -e` arrête le script ; `on_exit` `:35-49` affiche « ÉCHEC » et sort avec `rc` ; `main` `:200-213` appelé sans contexte conditionnel (`:213`) |
| 2c | Lancement depuis un sous-répertoire | OK | Git exécute les hooks à la racine du worktree (githooks(5)) ; `ci-local.sh:19-20` recalcule `ROOT="$(git rev-parse --show-toplevel)"` et `cd "$ROOT"`, donc indépendant du cwd |
| 2d | `.husky/_/h` | Absent dans ce worktree | `ls .husky/_` : absent. `core.hooksPath=.husky/_` (config partagée) ; le dossier est généré par `prepare: husky` (`package.json:19`) dans le dépôt principal. Ce n'est pas un défaut de la branche (voir remarque 3) |
| 3 | Fidélité jobs/ordre vs `ci.yml` | OK (commandes principales) | `awk` sur `ci.yml` (jobs, dans l'ordre) = `typescript observability-smoke rust-protocol rust-transport rust-integration rust-fork rust-dht rust-security rust-ffi rust-macos rust-stress rust-poc`, identique à `JOBS` (`ci-local.sh:24-25`). Commandes comparées ligne à ligne : typescript (`ci.yml:23-29` ↔ `ci-local.sh:114-117`), rust-protocol (`:68-74` ↔ `:127-129`), rust-transport (`:89-99` ↔ `:133-135`, `--test-threads=1` présent), rust-integration (`:114-126` ↔ `:139-142`, timeout 10 min présent), rust-fork (`:141-161` ↔ `:147-155`, ordre des tests identique), rust-dht (`:176-183` ↔ `:159-161`), rust-ffi (`:215-240` ↔ `:173-177`, ordre identique), rust-macos (`:259-264` ↔ `:181-182`), rust-stress (`:279-286` ↔ `:186-188`, timeout 5 min), rust-poc (`:305-317` ↔ `:193-197`, timeout 3 min). Scripts référencés présents et exécutables : `generate-ffi-header.sh` (supporte `--check`, ligne 17), `run-observability-smoke-local.sh`, `crates/tom-stress/scripts/test-localhost.sh`, `experiments/iroh-poc/scripts/test-localhost.sh` |
| 3b | Contrôle de dérive | Partiel | `ci-local.sh:92-105` compare seulement ids de jobs, ordre et présence de `paths:` (`:94`). Il ne détecte PAS une modification de commande, un `continue-on-error`, ou un `timeout-minutes` changé (voir remarque 2) |
| 4 | CONTRIBUTING exact, pas de doublon | OK | Section unique `## Before Pushing` (`CONTRIBUTING.md:162`, seule occurrence ; titres existants `:149 Testing`, `:168 Architecture Overview`). Contenu vérifié : hook → `ci-local.sh` (vrai, `pre-push:7`), « même ordre, mêmes commandes » (vrai, point 3), « s'arrête au premier échec » (vrai, `ci-local.sh:4`), `./scripts/ci-local.sh` exécutable à la main (vrai, `100755`). Formulation « fails when ci.yml changes without the script following » trop large (voir remarque 2) |
| 5 | Worktree imbriqué `.claude/worktrees/…` (#62) | Non reproduit (interdit de lancer) | Raisonnement statique : `ROOT` vient de `git rev-parse --show-toplevel` (`ci-local.sh:19`), donc le worktree imbriqué est utilisé comme racine. Cargo prend le `Cargo.toml` le plus proche, qui existe dans chaque worktree. Le risque décrit en #62 reste possible si un job tourne dans un chemin sans manifeste (ex. `crates/tom-protocol-ffi`, exclu du workspace). Remarque, pas bloquant (connu et tracé) |

## Verdict

**GO.** Aucun défaut prouvé : pas d'échappatoire dans le code du hook ni du script, échec propagé à `git push`, fidélité des jobs vérifiée ligne à ligne sur les commandes principales.

## Remarques (non bloquantes)

1. **`--no-verify` reste une échappatoire git.** Un hook client ne peut pas empêcher `git push --no-verify` ni un `core.hooksPath` changé. `ci-local.sh:6` (commentaire du hook) et `CONTRIBUTING.md:165` disent « sans échappatoire » / « forbidden » : c'est une règle de conduite, pas une garantie. Le vrai verrou serait la CI sur `main` ou une protection de branche. À dire dans le doc plutôt qu'à promettre.
2. **Contrôle de dérive limité.** `check_drift` (`ci-local.sh:92-105`) ne voit que les ids, l'ordre et les `paths:`. `CONTRIBUTING.md:165` (« fails when ci.yml changes without the script following ») surestime : une commande ou un `timeout-minutes` modifié dans `ci.yml` passerait inaperçu. Reformuler, ou étendre le contrôle.
3. **`.husky/_` non versionné.** Le hook n'existe que si `pnpm install` a tourné (`prepare: husky`). Dans un worktree neuf, `core.hooksPath=.husky/_` pointe vers rien et le hook ne tourne pas. Préexistant, pas introduit par la branche, mais à savoir pour le worktree R008 et tout nouveau worktree.
4. **Smoke d'observabilité joué deux fois.** `ci-local.sh` lance déjà `observability-smoke` inconditionnellement (`:120-124`). Si le push touche relay/discovery, `pre-push:25-32` le relance (`pre-push:32`). Coût en temps, pas un contournement ; à éliminer si on veut un push plus rapide.
5. **Pushes de suppression.** Le hook se déclenche aussi pour `git push origin --delete <branche>` ou un push de tag ; la suite complète (dont cargo et pnpm) tourne alors pour rien. Pas un bloquant ; à filtrer sur les refs stdin si ça gêne.
6. **`cargo-deny` : arguments non vérifiés contre `action.yml`.** `ci-local.sh:167-168` reprend `--log-level warn --manifest-path ./Cargo.toml --all-features` comme défauts de `cargo-deny-action@v2`. Le commentaire `:166` l'affirme, mais l'action n'a pas été consultée (pas d'accès réseau dans cette revue). À confirmer avant de s'y fier.
7. **Effet de bord mineur.** `job_observability_smoke` fait `chmod +x` (`ci-local.sh:122`). Sans effet ici (mode déjà `755` dans git), mais modifie le fichier à chaque run.
8. **Sémantique de `pipefail` dans `sh -c`.** `ci-local.sh:193-194` : le pipe `cargo metadata | python3` tourne sous `sh`, où `pipefail` n'est pas actif. Un échec de cargo est quand même détecté, car python3 échoue sur une entrée vide. Même comportement que la CI (`ci.yml:306-308`).
9. **Limites de cette revue.** Rien n'a été exécuté (consigne). Le comportement à l'exécution (échec réel → code ≠ 0 → push refusé, worktree imbriqué #62) reste à prouver par un passage réel sur le worktree du mandat, pas par cette lecture statique.
