---
date: 2026-10-17
revue: R004
mandat: M0007
branche: fix/ci-verte-outillage
verdict: GO
tip: 537e9815fe98f38908141e7b6ba8e1daf435e0f6
niveau: N2
---

# R004 — Doublage N2 CI verte outillage (M0007)

Relecture indépendante du diff `9278dca..04c345d` (4 commits, 12 fichiers). Aucun `cargo` ni `pnpm` lancé. Travail fait en lecture de code, `git` et `gh` en lecture, `npm view`, `rustc --version`, `/bin/bash -n`, et lecture des logs de la session (non relancés).

## Verdict

**GO** au tip `537e981`. La réserve initiale (job `rust-security` de `scripts/ci-local.sh` sans les arguments de `cargo-deny-action@v2`) est levée par le commit `537e981` : voir « Contre-vérification » en fin de rapport. Les points 1 à 6 sont conformes, avec des remarques non bloquantes.

---

## 1. Lints corrigés ciblés — conforme (remarques de précision)

Inventaire complet des `#[allow]` ajoutés : 10 attributs, tous au niveau item (fonction, instruction, trait). Aucun `#![allow]` crate-level. `git diff --name-only 9278dca..04c345d` ne touche ni `clippy.toml`, ni `.cargo/config`, ni `[lints]`, ni `RUSTFLAGS`.

**a. `semicolon_in_expressions_from_non_local_macros` — build.rs de tom-connect, tom-quinn, tom-relay (`#[allow]` sur `fn main`).**
- Justification exacte. Diff des sources cfg_aliases : `0.2.1/src/lib.rs` lignes 268, 287, 305 portent un `;` fautif (`]);`, `] $($current)* $tok);`, `);`) que `0.2.2` supprime exactement à ces lignes (`diff` : `268c268`, `287c287`, `305c305`).
- Effet prouvé : dans `ci-local-run3.log` (étape rust-ffi, qui compile tom-relay et tom-quinn), les seuls `trailing semicolon` restants viennent de `patches/netwatch-0.13.0/build.rs:5` et `patches/netwatch-0.14.0/build.rs:5` (6 + 6 occurrences). Aucun ne vient des trois build.rs corrigés.
- Le bump 0.2.2 est bien bloqué par `Cargo.lock`, hors périmètre. Remarque R-h ci-dessous.

**b. `deprecated` (`fetch_update`) — tom-metrics `Gauge::dec` (lib.rs:92-95), tom-quinn `connection.rs:1325` (Drop de `ConnectionRef`) et `connection.rs:1393` (`app_handle_dropped`).**
- Inventaire complet : `git grep fetch_update 9278dca -- '*.rs'` donne 3 sites, et `04c345d` a les 3 allow correspondants. Aucun site oublié.
- Le warning réel est dans `ci-local-run1.log` (lignes 6681, 6724, 6737, 6774, 6787, 6801) : `renamed to try_update for consistency`. La déprécation est donc prouvée sur 1.99.1.
- Justification « try_update instable sous la MSRV 1.88 » : exacte pour tom-quinn (`crates/tom-quinn/Cargo.toml:14`, `rust-version = "1.88"`).
- Précision pour tom-metrics : son `Cargo.toml` n'a aucune clé `rust-version` (lignes 1-10). Le commentaire parle donc de 1.88 à tort pour ce crate. La MSRV du workspace est 1.89 (`Cargo.toml:14`).
- Conclusion qui tient quand même en 1.89 : le rust-src local du toolchain `1.89.0-sbpf-solana-v1.54` montre `try_update` en `#[unstable(feature = "atomic_try_update")]` (`atomic.rs:1392`, `:2094`). Réserve de méthode : c'est un fork Solana de 1.89, pas l'upstream. Le rust-src de 1.99.1 n'est pas installé, donc on ne peut pas re-vérifier la déprécation localement.
- L'allow sur `let` (connection.rs:1393) couvre une seule instruction : c'est le périmètre minimal.

**c. `clippy::result_large_err` — tom-connect `relay/actor.rs` : `run_once` (l.363), `run_connected` (l.504), `run_sending` (l.729).**
- Inventaire complet : les trois fonctions qui retournent `Result<_, RelayConnectionError>` sont exactement celles allow (grep `Result<…, RelayConnectionError|RunError|DialError>` sur tom-connect).
- Justification exacte : `clippy-ws.log` lignes 145, 159, 171 : `the largest variant contains at least 128 bytes`. `RelayConnectionError` (actor.rs:217-224) embarque `DialError` dans `Dial { source: DialError }`.
- Refus de `Box<DialError>` par n0-error : non reproduit (pas de cargo). Ce qu'on peut lire : `n0-error-0.1.3/src/any.rs:24` n'a que `Stack(Box<dyn StackError>)`. Cela ne prouve pas le refus, mais ne le contredit pas.

**d. `double_must_use` — trait `Transport` (tom-protocol `runtime/transport.rs:8-10`).**
- `clippy-ws2.log` lignes 17-40 : `this function has a #[must_use] attribute with no message, but returns a type already considered as #[must_use]`, `the return type is pinned boxed n0_future::Future`, `this error originates in the attribute macro async_trait::async_trait`. La justification est exacte.
- L'allow est au niveau du trait : il couvre les méthodes. La preuve d'efficacité est `ci-local-run2.log` : aucune ligne `error:` après l'étape rust-protocol clippy (vérifié après retrait des codes ANSI).

**e. `drain_collect` — tom-tui `main.rs:1111` (correctif réel, pas d'allow).**
- Ancien code (9278dca:1111) : `app.input.drain(..).collect::<String>()`. `app.input` est un `String` (main.rs:788). `std::mem::take(&mut app.input)` transmet le même texte et laisse `app.input` vide dans les deux cas.
- Seule différence : la capacité de `app.input` (drain la conserve, take la rend à 0). Négligeable pour une saisie utilisateur. Conforme.

**Correctif évident raté ?** Non, rien de bloquant. Alternatives non retenues, à titre de remarque :
- `Gauge::dec` (tom-metrics) : une boucle `compare_exchange` supprimerait l'allow sans toucher à Cargo. Local au crate, mais demande une réécriture d'atomique.
- cfg_aliases : émettre les `cargo:rustc-cfg` à la main dans les trois build.rs supprime l'allow, mais laisse une build-dependency morte et touche trois fichiers. Plus invasif. L'allow ciblé est acceptable.
- `result_large_err` : pas d'alternative simple sans toucher aux types d'erreur.

## 2. Toolchain épinglée et MSRV — conforme

- `rust-toolchain.toml` (8 lignes) : `channel = "1.99.1"`, `components = ["clippy", "rustfmt"]`. Clippy est bien présent.
- Dernier run CI `39040092235` (headSha `9278dcab…`, conclusion `failure`) : `gh run view 39040092235 --log` donne 30 lignes `rustc 1.99.1 (58cf8f9c3 2026-10-13)` pour 10 jobs (3 lignes par job). Les 10 lignes `1.99.0` sont toutes dans le texte `(from rustc 1.99.0 …)` de la ligne `updated`. Aucun job n'a tourné en 1.99.0.
- MSRV inchangée : aucun `Cargo.toml` dans le diff. `rust-version` reste à `1.89` (racine) et `1.88` pour tom-quinn, tom-quinn-proto, tom-quinn-udp.
- Scripts Apple : `scripts/build-tom-protocol-ffi-xcframework.sh:85,92`, `scripts/build-apple-tv-relay-ffi.sh:22,28` et `scripts/build-apple-tv-relay.sh:21,24` appellent `cargo +"${TOOLCHAIN}"`. Le `+` prime sur le fichier. Leurs défauts sont des nightly (`xcframework:40` = `nightly-2026-07-01-…`). Aucun appel `cargo` nu dans ces scripts ni dans `release-sdk-swift.yml` (`git grep` vérifié).

## 3. claude-atelier retiré — conforme

- `package.json` : une seule ligne supprimée (`"claude-atelier": "^0.28.2"`).
- `pnpm-lock.yaml` : `git diff --numstat 9278dca..04c345d` = `0 270`. Le diff ne contient que des suppressions (`git diff … | grep '^+[^+]'` vide). Entrée d'importer supprimée (hunk `@@ -20,9 +20,6 @@`). 37 clés `packages`/`snapshots` supprimées (paquets transitifs de claude-atelier, par exemple better-sqlite3, prebuild-install, tar-fs).
- Contrôle de cohérence par script Python (heuristique, pas pnpm) : aucune des 37 clés supprimées n'est encore présente dans le lock HEAD, et aucune référence de dépendance pendante. `pnpm install --frozen-lockfile` n'a pas été exécuté (interdit). Cohérence non prouvée par pnpm.
- Usage : `git grep claude-atelier 04c345d` n'a aucune occurrence dans `scripts/`, `.husky/`, `test/`, `apps/`, `packages/`, `tools/`, et aucun `node_modules/.bin/claude…`. Les occurrences restantes sont des docs, des skills, et des slash-commands qui appellent `npx claude-atelier features` (`.claude/commands/atelier-config.md:32-41`). Ces appels passent par le registre et sont déjà cassés par la dépublication, pas par ce commit. Hors périmètre, à noter.
- `npm view claude-atelier versions` : `404 Unpublished on 2026-09-30T04:56:31.136Z`. Confirmé.

## 4. `scripts/ci-local.sh` fidèle à `ci.yml` — réserve levée (537e981)

Tableau et constats ci-dessous au tip `04c345d` (avant correctif). Le défaut rust-security est corrigé par `537e981` (voir Contre-vérification).

Tableau job par job (référence : `ci.yml` lignes, script : fonctions `job_*`) :

| Job ci.yml | Étapes / commandes ci.yml | Commandes ci-local.sh | Fidèle | Écart |
|---|---|---|---|---|
| typescript | `pnpm install --frozen-lockfile`, `pnpm build`, `pnpm test`, `pnpm lint` (l.23-29) | identiques, mêmes 4 lignes (L108-118) | oui | Node 20 non imposé (version affichée, écart noté) |
| observability-smoke | `apt-get install lsof` ; `chmod +x` + `./scripts/run-observability-smoke-local.sh` (l.45-53) | `need lsof` ; `chmod +x` + même script (L120-124) | oui | installation apt remplacée par un contrôle de présence (écart d'environnement) |
| rust-protocol | build / test / clippy `-p tom-protocol -- -D warnings` (l.68-74) | identiques (L126-130) | oui | — |
| rust-transport | build / `test -p tom-transport -- --test-threads=1` / clippy (l.89-99) | identiques (L132-136) | oui | — |
| rust-integration | build `--tests` ; `test -- --test-threads=1` timeout 10 ; clippy `--tests -D warnings` (l.114-126) | identiques, `with_timeout 10` (L138-143) | oui, sauf | `timeout`/`gtimeout` absent par défaut sur macOS : échec en milieu de job (R-c) |
| rust-fork | build 7 crates ; test base, metrics, quinn-proto `--lib`, quinn `--lib`, relay `--lib`, gossip `--lib`, connect `--lib` ; clippy 7 crates (l.141-161) | identiques, même ordre, même liste (L145-157) | oui | — |
| rust-dht | build ; `test --lib` ; clippy (l.176-183) | identiques (L158-162) | oui | — |
| rust-security | `EmbarkStudios/cargo-deny-action@v2`, `command: check advisories bans sources` (l.194-196) | `cargo deny check advisories bans sources` (L164-167) | **NON** | l'action passe `--log-level warn --manifest-path ./Cargo.toml --all-features` par défaut. Le script omet `--all-features`. Voir ci-dessous. |
| rust-ffi | build relay-ffi ; clippy relay-ffi ; build protocol-ffi `--locked` (working-dir) ; clippy `--locked` ; install cbindgen ; `generate-ffi-header.sh --check` (l.215-240) | identiques ; cbindgen vérifié en amont (`need`) (L169-176) | oui | ordre : présence de cbindgen contrôlée avant le build (inoffensif) |
| rust-macos | build relay-ffi ; build `--locked` protocol-ffi (l.259-264) | identiques (L178-181) | oui | macOS local = `macos-latest` (seul job sur OS identique) |
| rust-stress | build ; clippy ; `test-localhost.sh` timeout 5 (l.279-286) | identiques, `with_timeout 5` (L183-187) | oui | — |
| rust-poc | `defaults.run.working-directory: experiments/iroh-poc` ; debug metadata ; build `--bin nat-test` ; clippy `--bin nat-test` ; `test-localhost.sh` timeout 3 (l.293-317) | identiques via `in_dir` (L189-196) | oui | — |

**Défaut prouvé : job `rust-security`.** Preuve de l'écart :
- `gh api repos/EmbarkStudios/cargo-deny-action/contents/action.yml` : `arguments` a pour défaut `--all-features` (ligne 17). L'entrypoint passe `--log-level ${log-level}`, `--manifest-path`, `${arguments}` puis `${command}`.
- La CI exécute donc `cargo deny --log-level warn --manifest-path ./Cargo.toml --all-features check advisories bans sources`.
- Le script exécute `cargo deny check advisories bans sources` (L164-167).
- L'en-tête du script (lignes 11-12) classe cet écart parmi les « écarts inévitables ». Il ne l'est pas : le flag se reproduit à l'identique.
- Effet : `--all-features` élargit le graphe contrôlé aux dépendances optionnelles. Le run local (`ci-local-run2.log`, fin de fichier) donne `advisories FAILED, bans ok, sources ok` sans ce flag. Je ne peux pas dire si le run CI ajouterait des échecs : `cargo deny` n'a pas été lancé.
- Impact aujourd'hui : rust-security est rouge en CI de toute façon (M0008). Le risque est pour le moment où M0008 sera fusionné : le script pourrait afficher vert alors que la CI échoue sur une advisory ou un ban optionnel.

**Correctif demandé (une ligne, fichier autorisé) :** dans `job_rust_security`, remplacer par
`step "rust-security: cargo deny" cargo deny --all-features check advisories bans sources`
(`--log-level warn` optionnel, cosmétique). Je n'ai pas exécuté ce correctif (lecture seule). Correctif appliqué en `537e981` avec `--log-level warn --manifest-path ./Cargo.toml --all-features` : voir Contre-vérification.

**Autres constats du point 4 (non bloquants) :**
- Ordre, arrêt au premier échec et durée par étape : conformes. `set -euo pipefail` présent (ligne 17). Le trap `on_exit` affiche les durées en cas d'échec, puis « ÉCHEC … pas de push ».
- Aucun mode de saut : `grep -i 'skip|CI_LOCAL|bypass'` ne trouve que `--no-deps` (chaîne dans le job poc). Aucun `CI_LOCAL_SKIP`.
- Contrôle de dérive : `check_drift` vérifie la présence d'un `paths:` et compare la liste des ids de jobs à `JOBS`. Replay de l'awk sur `ci.yml` (HEAD) : sortie identique au tableau `JOBS` (12 jobs, même ordre). Le contrôle ne vérifie pas les commandes : voir R-a.
- Bash : `/bin/bash -n` passe sous `GNU bash 3.2.57` (seul bash du PATH). Les runs de la session ont tourné sous ce même bash : compatibilité 3.2 prouvée pour les étapes exécutées.
- Preuves d'exécution (lues, pas relancées) : run2 va jusqu'à `rust-security` (toutes les étapes avant sont vertes, aucune ligne `error:` hors advisories RUSTSEC) ; run3 couvre rust-ffi, rust-macos, rust-stress, rust-poc et se termine par `TOUT VERT` (EXIT=0). La copie du script de run3 a des fonctions `job_*` identiques à celles du commit (diff vérifié).
- Limite de preuve : ces runs sont sur macOS local, pas sur les runners Ubuntu. Le vert local ne vaut pas CI verte ; c'est pour ça qu'un run CI réel reste nécessaire.

## 5. Fichiers interdits ou de M0008 — conforme

`git diff --name-only 9278dca..04c345d` :
```
crates/tom-connect/build.rs
crates/tom-connect/src/socket/transports/relay/actor.rs
crates/tom-metrics/src/lib.rs
crates/tom-protocol/src/runtime/transport.rs
crates/tom-quinn/build.rs
crates/tom-quinn/src/connection.rs
crates/tom-relay/build.rs
crates/tom-tui/src/main.rs
package.json
pnpm-lock.yaml
rust-toolchain.toml
scripts/ci-local.sh
```
Aucun des fichiers interdits n'apparaît : `Cargo.toml` (racine et crates), `Cargo.lock`, `deny.toml`, `crates/tom-protocol-ffi/Cargo.lock`, `scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `.claude/settings.json`.
- `.husky/pre-push` : absent du diff. Son contenu (lu) ne référence pas `ci-local`. Le script n'est pas branché, comme annoncé.
- Worktree : `git status --porcelain` vide avant l'écriture de ce rapport.

## 6. Messages de commit — conforme

- Types conventionnels : `ci:`, `fix(ci):`, `build:`, `fix(ci):`.
- Auteur et committer sur les 4 commits : `Malik <karaoui.malik@gmail.com>` (gmail exigé, vérifié).
- `%(trailers)` vide sur les 4 commits : aucun `Co-Authored-By`.
- Remarque informative : le corps de `04c345d` cite la branche `chore/pre-push-ci-local`, qui existe localement. Ce n'est pas une erreur.

---

## Remarques (non bloquantes)

- **R-a. Contrôle de dérive partiel.** `check_drift` ne compare que les ids de jobs, leur ordre et l'absence de `paths:`. Une modification d'une commande dans `ci.yml` passe inaperçue. Piste : hasher les lignes `run:` de chaque job, ou une liste de commandes attendues.
- **R-b. Arbre de travail, pas HEAD.** Le script lance les commandes sur l'arbre courant, sans garde `git status --porcelain`. Des fichiers non suivis ou ignorés peuvent faire passer un vert local là où un clone propre (CI) échoue. Piste : avertir ou refuser si l'arbre est sale.
- **R-c. Dépendances découvertes en cours de run.** `timeout`/`gtimeout` (macOS n'en a pas par défaut) n'est contrôlé qu'au moment de `rust-integration`, après les builds. `node` n'a pas de `need` dans `job_typescript`. Piste : tout regrouper dans `need` au démarrage.
- **R-d. Node.** Node 20 n'est pas imposé, seulement affiché. Acceptable, déjà annoncé dans l'en-tête.
- **R-e. Warnings netwatch.** `patches/netwatch-0.13.0/build.rs:5` et `0.14.0` émettent encore `trailing semicolon` (12 occurrences dans run3). Non bloquant : ces crates ne sont pas cibles de `clippy -D warnings`. Le même motif peut revenir si le patch est mis à jour.
- **R-f. Précision du commentaire tom-metrics.** Voir point 1b : « MSRV 1.88 » est inexact pour tom-metrics (pas de `rust-version`). Formulation plus juste : « instable jusqu'au moins 1.89 (MSRV workspace) ».
- **R-g. `Cargo.toml` hors périmètre (à ouvrir en issue).** `lints.clippy.unused-async` est déprécié au profit de `unused_async` (warnings `tom-base/Cargo.toml:50`, `tom-gossip/Cargo.toml:68`). Fichier interdit à M0007 : ne pas corriger ici.
- **R-h. Alternatives cfg_aliases et fetch_update.** Voir point 1 : une réécriture de build.rs sans cfg_aliases, et une boucle `compare_exchange` pour `Gauge::dec`. Ni l'une ni l'autre n'est requise.
- **R-i. `drain` vs `mem::take`.** Même comportement fonctionnel. Seule la capacité du `String` de saisie tombe à 0 après Enter. Négligeable.
- **R-j. Docs claude-atelier.** `npx claude-atelier` reste dans `.claude/commands/atelier-config.md` et les skills atelier-*. Déjà cassé par la dépublication, à traiter séparément.
- **R-k. Preuve CI.** Le vrai critère reste un run `ci.yml` vert sur Ubuntu (et `rust-macos` sur macos-latest) après fusion de M0008. Rien dans ce mandat ne le prouve encore.

---

## Contre-vérification (tip 537e981)

Périmètre : `git diff 04c345d..537e981` = 1 fichier, `scripts/ci-local.sh` (+4 / -2), un seul commit `537e981`. Aucun cargo, aucun commit, aucun autre fichier modifié (`git status --porcelain` ne liste que ce rapport, non suivi).

**Levée de la réserve : prouvée.**
- Commande locale désormais dans `job_rust_security` : `cargo deny --log-level warn --manifest-path ./Cargo.toml --all-features check advisories bans sources`.
- Source de vérité, action `EmbarkStudios/cargo-deny-action@v2` (`gh api …/contents/action.yml`) : la liste `args:` est, dans l'ordre, `${rust-version}`, `${credentials}`, `${ssh-key}`, `${ssh-known-hosts}`, `--log-level ${log-level}`, `--manifest-path ${manifest-path}`, `${arguments}`, `${command}`, `${command-arguments}`.
- Source de l'entrypoint, `entrypoint.sh` de l'action (`gh api …/contents/entrypoint.sh`) : après `shift 4`, la commande exécutée est `cargo-deny $*`. Les arguments restants sont donc exactement `--log-level warn --manifest-path ./Cargo.toml --all-features check advisories bans sources`. L'ordre du script correspond à celui de l'action.
- Valeurs effectives dans la CI : `gh run view 39040092235 --log` (job « Rust supply chain ») affiche `command: check advisories bans sources`, `arguments: --all-features`, `manifest-path: ./Cargo.toml`, `log-level: warn`. Concordance exacte.
- Les `rust-version`, `credentials`, `ssh-key` et `ssh-known-hosts` vides sont des positions de l'entrypoint, non des flags : aucun flag supplémentaire à reproduire.

**Ensemble d'advisories : concordant.** Les identifiants RUSTSEC du log CI du run 39040092235 (job « Rust supply chain ») sont `RUSTSEC-2026-0002, 0258, 0285, 0346, 0348, 0349`, soit exactement les six de la sortie locale collée par la session mère (5 erreurs + avertissement 0002 ligne deny.toml:21). Je n'ai pas relancé `cargo deny` : la concordance repose sur la comparaison des logs, pas sur une exécution.

**Ce qui reste hors de ma preuve :** la sortie locale `advisories FAILED, bans ok, sources ok` avec `--all-features` est collée, pas revérifiée. Le rouge de rust-security reste attendu (M0008), comme prévu par le mandat.

**Autres points du rapport :** inchangés. Les conclusions des points 1, 2, 3, 5 et 6 ne dépendent pas de ce commit (il ne touche que `scripts/ci-local.sh`). Les remarques R-a à R-k restent valables.
