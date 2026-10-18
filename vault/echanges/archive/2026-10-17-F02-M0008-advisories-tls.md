---
date: 2026-10-17
tags: [securite, tls, cargo-deny]
session: F02
mandat: advisories-tls-h2-rustls
mandat_id: M0008
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'high' 'Va lire ton prompt dans vault/echanges/F02.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F02-M0008
branche: fix/advisories-tls
niveau: N3
derniere_maj: 2026-10-17T16:40:52+0200
---

# F02 / M0008 — Advisories RustSec h2 / rustls (N3, doublage en chaîne)

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, https://github.com/tobi/qmd ; index `tom` à jour depuis M0006) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.
> **IMPÉRATIF (fondateur, 2026-10-17) : on ne pousse pas si ce n'est pas nickel.** Avant CHAQUE `git push`, tu rejoues en local les contrôles de la CI (`.github/workflows/ci.yml`) qui couvrent ce que tu as touché, et tu colles leurs sorties. Rouge = pas de push.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

Feu vert du fondateur 2026-10-17 16:39. État des lieux M0006 (`vault/reprise/2026-10-17-ETAT-DES-LIEUX.md`, §2 et lot 2, via QMD) : `cargo deny` fait échouer la CI de `main` sur RUSTSEC-2026-0258 (h2), -0285, -0346, -0348, -0349 (rustls) [VÉRIFIÉ par M0006, log run `39036883418`]. `deny.toml` contient 9 lignes `RUSTSEC-` déjà ignorées (7 identifiants uniques) ; le commentaire de `ci.yml:191-192` en annonce un autre nombre.
Zone **critique** (TLS, transport chiffré) : niveau **N3**, doublage en chaîne, loi des deux patchs (`.claude/skills/orchestration-bureau/SKILL.md` §9).

## Fenêtre parallèle

F01/M0007 touche en même temps : `crates/tom-quinn/build.rs`, `rust-toolchain.toml`, `package.json`, `pnpm-lock.yaml`, `scripts/ci-local.sh`, `.husky/pre-push`, `CONTRIBUTING.md`. **Tu n'y touches pas.** Note : la CI peut rester rouge sur le lint `tom-quinn/build.rs` tant que M0007 n'est pas mergé ; ton critère est `cargo deny check` + build/tests/clippy des crates touchés, pas le vert global.

## Gardes de précondition

1. `git fetch origin` ; `git merge-base --is-ancestor 9278dcab25e69b3b4a05e980542997cd870bc3c5 origin/main` (code 0), sinon STOP.
2. Travail dans un **worktree** créé depuis `origin/main` : `git worktree add -b fix/advisories-tls .claude/worktrees/F02-M0008 origin/main`. Jamais de `checkout` à la racine (une autre fenêtre y travaille aussi). Dans un worktree, `pnpm` et `biome` ont besoin de `PATH=/Users/malik/Documents/tom-protocol/node_modules/.bin:$PATH` (issue #58) ou d'un `pnpm install --frozen-lockfile` local au worktree.

## Mission

1. Rituel de début : `statut: en-cours` (dans `vault/echanges/F02.md`, à la racine).
2. `cargo deny check advisories` dans le worktree : liste exacte des advisories (id, crate, version, version corrigée, chemin de dépendance `cargo tree -i <crate>`). Tableau au rapport.
3. Pour chacune : **mettre à jour** vers la version corrigée (`cargo update -p <crate> --precise <v>` d'abord ; montée de version majeure dans un `Cargo.toml` seulement si nécessaire). Un `ignore` dans `deny.toml` n'est permis que si aucune version corrigée n'existe ou si la faille ne s'applique pas au code (preuve fichier:ligne + justification d'une ligne à côté de l'ignore, avec date de réexamen). Profite-en pour aligner le commentaire de `ci.yml:191-192` sur la réalité **seulement si** M0007 ne touche pas `ci.yml` (il ne le touche pas) — sinon laisse.
4. Lock FFI (`tom-protocol-ffi`, exclu du workspace, patch tvOS) : même traitement dans son propre `Cargo.lock`, ou STOP si le patch tvOS l'empêche.
5. Contrôles (sorties collées, durées) : `cargo deny check` (vert), `cargo build --workspace --locked`, `cargo test --workspace --locked --no-fail-fast` (les 2 tests instables connus `tom-transport gate_rejects_unlisted_peer` et `tom-sdk two_clients_exchange_message_via_tickets` : rejoue-les seuls et dis-le), `cargo clippy -p <chaque crate dont la dépendance TLS a bougé> -- -D warnings` (le lint `tom-quinn/build.rs` de M0007 peut rester : note-le, ne le corrige pas), build + test FFI comme `ci.yml` le fait.
6. Commits pathspec explicites, sans trailer, `fix(deps): …` ; push de la branche + `ls-remote`.
7. **Doublage N3 en chaîne** : premier doubleur = sous-agent FRAIS, worktree détaché lecture seule `.claude/worktrees/F02-R005`, rapport `vault/revues/2026-10-17-R005-advisories-tls.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40>`, `niveau: N3`). Il vérifie : chaque advisory résolue ou ignorée avec preuve ; aucune régression d'API TLS (changelogs rustls/h2 lus, ruptures listées, usages dans le code recontrôlés fichier:ligne) ; `Cargo.lock` cohérent ; aucun fichier de M0007. RESERVE prouvée par un test rouge → correctif, puis **second doubleur frais** (R006) sur le tip corrigé ; deux correctifs sur la même classe de défaut → STOP (loi des deux patchs).
8. Merge (bloc « Merge » ci-dessous), rapport R005 (et R006 le cas échéant) committé dans le scratch. Après push : `gh run watch` ; `cargo deny` doit être vert en CI ; colle le résultat de chaque job.

## Règles des sous-agents (verbatim)

(1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre. **Aucun sous-agent ne lance de build ou de test cargo** (incident M0006 : `target/` à 23 Go) : les builds restent à la session mère.

## Hygiène disque

Plafond `target/` 20 Go (`CLAUDE.md` racine) : `du -sh target/` avant et après chaque build lourd ; au-delà de 15 Go, `cargo clean` avant de continuer. Rien ne tourne en fond à la sortie.

## Intouchables

`scripts/pre-push-gate.sh` (modifié par le fondateur, non committé), `vault/30-discoveries.md`, `.claude/settings.json`, les `vault/echanges/F0x.md` vivants (jamais committés), tout fichier de l'autre fenêtre (voir « Fenêtre parallèle »).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée ; jamais `--no-verify` ; jamais de force-push.

## Merge (gabarits §9) et intégration

Scratch détaché sur `origin/main` à jour (`git fetch`), `merge --no-ff <tip> -m "Merge <branche> [<Rxxx>]"`, **contrôles CI concernés rejoués sur le scratch** (sorties collées), copie + commit du rapport de doublage (`docs(revues): <Rxxx> -- GO`). Puis : racine sur `main` et `git status --short` = intouchables seuls → `git merge --ff-only <HEAD scratch>` à la racine + `git push origin main` ; racine occupée → `git push origin <HEAD scratch>:refs/heads/main` depuis le scratch (fast-forward seulement). `origin/main` a bougé → refaire le scratch UNE fois. `git ls-remote origin refs/heads/main` collé. Après le push : `gh run list --branch main -L 3` et `gh run watch <id> --exit-status` (attente bornée à 30 min) — sortie collée ; le résultat CI réel va au rapport tel quel. Worktrees retirés à la fin.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0008`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F02.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F02"`, `mandat: "M0008"`) et `vault/runtime/state.json` (`windows.F02` seulement, relu juste avant d'écrire) : jamais committés.
4. Commit vault sur `main` (racine si elle est sur `main`, sinon scratch détaché) : `vault/reprise/F02.md` + `vault/echanges/archive/2026-10-17-F02-M0005-qmd-imperatif-en-tete.md` ; `docs(vault): reprise F02 M0008` ; push ; `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.

## Rapport M0008

Résultat : les 5 advisories (RUSTSEC-2026-0258 h2 ; -0285, -0346, -0348, -0349 rustls) sont corrigées par mise à jour, sans aucun `ignore`. R005 a rendu RESERVE sur une rétrogradation de `socket2` ; elle est corrigée et R006 rend GO. Le tout est intégré sur `main` (`aa6dab8f4730a834d3b491d00928d8f8343a5cb8`). En CI, `cargo-deny` passe au **vert** ; c'est le seul job qui change d'état par rapport au run de `9278dca`.

**Gardes** : `merge-base --is-ancestor 9278dca… origin/main` = 0 ; `origin/main` = `9278dcab25e69b3b4a05e980542997cd870bc3c5`. Worktree `.claude/worktrees/F02-M0008`.
Note QMD : `mcp__qmd__get vault/echanges/F02.md` a servi l'ancienne version du fichier (M0005, `statut: reponse-disponible`), alors que le disque portait M0008. Les fichiers vivants de `vault/echanges/` doivent donc se relire sur disque. Sinon, on risque de réexécuter ou de sauter un mandat.

**Advisories** (`cargo deny check advisories`, base) :

| ID | crate | version | corrigée en | chemin (`cargo tree -i`) | traitement |
|---|---|---|---|---|---|
| RUSTSEC-2026-0258 | h2 | 0.4.13 | ≥0.4.16 | hickory-net/hyper → hickory-resolver, reqwest → tom-connect, tom-relay, tom-transport | `cargo update -p h2 --precise 0.4.20` |
| RUSTSEC-2026-0285 | rustls | 0.23.36 | ≥0.23.45 | hickory, hyper-rustls/reqwest, rustls-platform-verifier → tom-quinn-proto, tom-connect, tom-relay | `cargo update -p rustls --precise 0.23.46` |
| RUSTSEC-2026-0346 / -0348 / -0349 | rustls | 0.23.36 | ≥0.23.46 | idem | idem |

Effets induits : rustls-webpki 0.103.13→0.103.15 ; aws-lc-rs 1.17.0→1.18.2, avec de nouvelles entrées optionnelles aws-lc-sys 0.46.0, aws-lc-fips-sys 0.14.3, bindgen 0.73.2 et shlex 2.0.1. Dans le workspace, aws-lc n'est **pas** compilé : `cargo tree -i aws-lc-rs --workspace --target all` ne trouve rien, car le code utilise `ring`. `deny.toml` est inchangé. Le commentaire `ci.yml:192` passe de « 10 » à « 7 advisories ignorées », ce qui correspond aux 7 entrées actives.

**FFI** (lock propre, mêmes mises à jour) : rustls 0.23.37→0.23.46, h2 0.4.13→0.4.20, aws-lc-rs 1.16.1→1.18.2, aws-lc-sys 0.38.0→0.46.0. Ici aws-lc **est** compilé (via `pkarr 5.0.3` → `reqwest` → `hyper-rustls`), et c'était déjà le cas à la base. R006 relève que la mise à jour corrige aussi RUSTSEC-2026-0044/-0048 (AWS-LC). Le worktree est imbriqué dans le dépôt, donc cargo y remonte jusqu'au workspace racine : les opérations FFI ont été faites dans un worktree détaché hors dépôt (scratchpad).

**Commits** (`fix/advisories-tls`) :
- `568e7a9` : lock racine et commentaire `ci.yml`.
- `448d07c` : lock FFI.
- `d15da52` : correctif R005. `cargo update -p` avait ré-résolu des contraintes à plage (`socket2 >=0.5, <0.7` dans `tom-quinn`) et basculé, sans le vouloir, 6 arêtes `socket2 0.6.2→0.5.10` (tom-quinn, iroh-quinn-udp, quinn-udp, quinn, hyper-util) et 10 arêtes `windows-sys`. Ces arêtes reviennent à l'identique de la base (16 côté racine, 14 côté FFI) et le résultat est accepté par cargo en `--locked`. Contrôle rouge→vert : sur `448d07c`, 13 paquets dérivent hors des crates montées volontairement ; sur `d15da52`, 0. `cargo tree -i socket2@0.6.2` remonte de nouveau à `tom-quinn`.

**Contrôles locaux sur `d15da52`** :
- `cargo deny check advisories bans sources` → `advisories ok, bans ok, sources ok`. `cargo deny check` sans argument échoue sur `licenses`, check volontairement différé (en-tête de `deny.toml`) et non lancé par la CI.
- `cargo build --workspace --locked` : 1 min 15 s, OK.
- `cargo test --workspace --locked --no-fail-fast` : 5 min 49 s, **1501 passés, 0 échec**, 10 ignorés. Les deux tests instables connus, `gate_rejects_unlisted_peer` et `two_clients_exchange_message_via_tickets`, passent dans le run complet, donc je ne les ai pas rejoués seuls.
- `cargo clippy --workspace --locked -- -D warnings` (1 min 16 s) et `-p tom-integration-tests --tests` (1 min 05 s) : 0 warning. Le lint `tom-quinn/build.rs` ne se reproduit pas avec rustc 1.97 en local.
- FFI : `cargo build --locked` en 1 min 12 s, `cargo clippy --locked -- -D warnings` en 24 s, `cargo test --locked` (32 passés, 0 échec) en 53 s. `generate-ffi-header.sh --check` donne « header à jour ✓ ».
- Hors exigences : cross-compilation FFI pour `aarch64-apple-tvos` (2 min 46 s) et `aarch64-apple-ios` (2 min 09 s), toolchain `nightly-2026-07-01`, OK. Elle confirme qu'aws-lc-sys 0.46 se construit pour les tranches de l'XCFramework.
- Disque : `target/` nettoyé deux fois (17,5 Gio à chaque fois) en arrivant à 14 puis 15 Go.

**Branche** : `d15da521d6d54386fb6d7589ccf3b64c60b3427e	refs/heads/fix/advisories-tls`

**Doublage N3** :
- R005 (sous-agent frais, `F02-R005` sur `448d07c`) → **RESERVE**, pour une rétrogradation socket2/windows-sys prouvée par le diff du lock et `cargo tree`. Il n'y avait pas de test rouge : j'ai jugé que le défaut relevait de mon propre patch et l'ai corrigé (1er correctif).
- R006 (second sous-agent frais, `F02-R006` sur `d15da52`) → **GO**.
- Revérifié moi-même : le diff d'arêtes, `deny.toml:20-24,34-35`, `ci.yml:192`, `tom-relay/src/client/tls.rs`, `KeyLogFile` (`tom-connect/src/tls.rs:88,116`), l'absence d'usage `h2::` et d'`install_default` dans `crates/`, et `rustls-0.23.46/src/crypto/mod.rs:243-256`.
- Remarques non bloquantes de R006 : `tokio-rustls-acme` dans le FFI contient des `builder()` nus (préexistant) ; ML-DSA est maintenant par défaut dans le provider aws-lc de `reqwest 0.13`, côté FFI.

**Merge** : scratch `F02-merge` sur `origin/main` (`9278dca`, inchangé), `merge --no-ff` → `be02935` (arbre identique à `d15da52`). Contrôles rejoués dessus :
- `cargo deny` : ok.
- `cargo metadata --locked` : OK.
- `cargo build --workspace --locked` : 1 min 46 s.
- `cargo clippy --workspace --locked -D warnings` : 58 s, 0 warning.
- Build FFI `--locked` : 2 min 05 s.

Les revues R005 et R006 sont committées dans `aa6dab8`. La racine était sur `main` : `git status --short` ne montrait que les intouchables et des fichiers `vault/echanges/` non suivis d'autres fenêtres, aucun touché par le merge. J'ai donc fait `merge --ff-only aa6dab8` puis `git push origin main` (`9278dca..aa6dab8`).
`aa6dab8f4730a834d3b491d00928d8f8343a5cb8	refs/heads/main`

**CI réelle** (`gh run watch 39045644560 --exit-status`, rc=1) :
- succès : **Rust supply chain (cargo-deny)**, Observability smoke, Rust DHT, Rust macOS (FFI build), Rust PoC ;
- échec : Rust transport, Rust protocol, Rust stress, Rust FFI, Rust cross-crate integration, Rust fork crates. Les 6 échouent uniquement sur `could not compile tom-quinn (build script)`, `cfg_aliases` « trailing semicolon » ; 0 autre erreur dans les logs. C'est le lint `build.rs` du ressort de M0007 ;
- échec : TypeScript, sur `ERR_PNPM_FETCH_404 claude-atelier-0.28.2.tgz` (`package.json`/`pnpm-lock`, ressort de M0007) ;
- diff des jobs par rapport au run de `9278dca` (39040092235) : **un seul changement, cargo-deny failure → success**.

**Hors périmètre** : issue #59, qui couvre RUSTSEC-2026-0204 (crossbeam-epoch 0.9.18, lock FFI seulement ; la CI ne lance pas `cargo deny` sur ce lock) et l'ignore `RUSTSEC-2026-0002` qui n'est plus rencontré.
Worktrees retirés : `F02-M0008`, `F02-R005`, `F02-R006`, `F02-merge`, scratch FFI. La suppression des exports de revue (~100 Mo, scratchpad de session) a été refusée par le système de permissions ; ils restent sur place, sans incidence sur le dépôt.

**Vault** : `vault/reprise/F02.md` + archive M0005 committés et poussés (`docs(vault): reprise F02 M0008`) : `07fb527b70ada04ef78e7e5f93048d8e1d4e39ea	refs/heads/main`. `vault/runtime/` mis à jour, non committé (ignoré).

Scellé tests : sans objet (mandat sans test neuf ; `scripts/verifier-rendu.mjs` absent du dépôt)
Contre-témoins : aucun

M0008
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ✅
Merge main          ✅
Main push           ✅
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED
