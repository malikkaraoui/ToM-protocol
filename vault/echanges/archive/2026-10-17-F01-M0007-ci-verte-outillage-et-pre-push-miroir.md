---
date: 2026-10-17
tags: [ci, toolchain, pre-push]
session: F01
mandat: ci-verte-outillage-et-pre-push-miroir
mandat_id: M0007
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'high' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F01-M0007
branche: fix/ci-verte-outillage
niveau: N2
derniere_maj: 2026-10-17T18:24:50+0200
---

# F01 / M0007 — CI verte (outillage) + contrôle pre-push qui rejoue la CI

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, https://github.com/tobi/qmd ; index `tom` à jour depuis M0006) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.
> **IMPÉRATIF (fondateur, 2026-10-17) : on ne pousse pas si ce n'est pas nickel.** Avant CHAQUE `git push`, tu rejoues en local les contrôles de la CI (`.github/workflows/ci.yml`) qui couvrent ce que tu as touché, et tu colles leurs sorties. Rouge = pas de push.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

Fondateur, 2026-10-17, verbatim : « trop de failed sur les push GitHub, je ne veux pas de ça, donc obligatoire pre-push !! » ; « on ne push pas si c'est pas nickel ». Feu vert 16:39.
État des lieux M0006 (`vault/reprise/2026-10-17-ETAT-DES-LIEUX.md`, lis §2, §3, §4 et lots 1, 5, 7 via QMD) : CI `main` rouge 9 fois sur 10 depuis le 11/08 (dernier vert run `31503186348`). Causes [VÉRIFIÉ par M0006] :
- A. toolchain non épinglée (`dtolnay/rust-toolchain@stable`, pas de `rust-toolchain.toml` ; local 1.97, CI 1.99.x) → lint `trailing semicolon in macro used in expression position` sur `crates/tom-quinn/build.rs:5` (`cfg_aliases!`), qui casse les 6 jobs clippy ;
- B. cargo-deny : advisories RUSTSEC h2/rustls → **hors de ce mandat** (F02/M0008, N3, en parallèle) ;
- C. `package.json:26` `"claude-atelier": "^0.28.2"` → 404 au `pnpm install --frozen-lockfile` de la CI.
- D. `.husky/pre-push` ne lance qu'un smoke conditionnel ; `scripts/pre-push-gate.sh` n'est appelé par rien et ne ressemble pas à la CI.

## Fenêtre parallèle

F02/M0008 corrige en même temps les advisories (fichiers : `Cargo.toml` des crates touchés, `Cargo.lock`, `deny.toml`, lock FFI). **Tu ne touches à aucun de ces fichiers.** Si ton correctif l'exige, STOP.

## Gardes de précondition

1. `git fetch origin` ; `git merge-base --is-ancestor 9278dcab25e69b3b4a05e980542997cd870bc3c5 origin/main` (code 0), sinon STOP.
2. Travail dans un **worktree** créé depuis `origin/main` : `git worktree add -b fix/ci-verte-outillage .claude/worktrees/F01-M0007 origin/main`. Jamais de `checkout` à la racine (une autre fenêtre y travaille aussi). Dans un worktree, `pnpm` et `biome` ont besoin de `PATH=/Users/malik/Documents/tom-protocol/node_modules/.bin:$PATH` (issue #58) ou d'un `pnpm install --frozen-lockfile` local au worktree.

## Mission

1. Rituel de début : `statut: en-cours` (dans `vault/echanges/F01.md`, à la racine).
2. **Cause A** : corrige le lint dans `crates/tom-quinn/build.rs` (vrai correctif du code, pas d'`allow` global ; si `cfg_aliases!` impose la forme, `#[allow(...)]` ciblé sur la ligne avec commentaire d'une ligne qui dit pourquoi). Ajoute `rust-toolchain.toml` à la racine, épinglé sur la **version stable exacte utilisée par le dernier run CI** (lis-la dans son log : `gh run view <id> --log | grep rustc`), composants `clippy`, `rustfmt`. Ne change pas la MSRV déclarée. Si un autre `rust-toolchain*` ou le patch tvOS de `tom-protocol-ffi` s'y oppose : STOP.
3. **Cause C** : `claude-atelier` — vérifie `npm view claude-atelier versions` ; si le paquet n'existe pas sur le registre public, c'est une dépendance d'outillage privée/locale : retire-la de `package.json` + `pnpm-lock.yaml` (`pnpm remove`), et vérifie qu'aucun script ne l'importe (`git grep claude-atelier`). S'il est utilisé par un script, STOP avec la liste.
4. **Cause D — le contrôle obligatoire** : crée `scripts/ci-local.sh` qui **rejoue les jobs de `.github/workflows/ci.yml`** (mêmes commandes, même ordre, mêmes options, `set -euo pipefail`, code ≠ 0 au premier échec, durée par étape affichée). Il lit le diff `origin/main...HEAD` pour sauter les jobs dont les chemins ne sont pas touchés (TypeScript, Rust par crate, FFI, smoke) **seulement si la CI fait de même** ; sinon il les lance. `cargo deny check` inclus (il restera rouge tant que M0008 n'est pas mergé : c'est attendu, dis-le). Branche-le dans `.husky/pre-push` (garde le smoke conditionnel existant après). Un mode `CI_LOCAL_SKIP=<job>` est **interdit** (pas d'échappatoire). Documente en 3 lignes dans `CONTRIBUTING.md` (remplacer, pas ajouter si une section push existe).
   - Ne touche pas à `scripts/pre-push-gate.sh` (intouchable).
5. Lance `scripts/ci-local.sh` en entier dans le worktree : tout vert sauf `cargo deny` (advisories de M0008) attendu. Colle les sorties et durées. Tout autre rouge : corrige si c'est dans ton périmètre, sinon STOP.
6. **Deux branches, dans cet ordre** (le hook pre-push branché sur `ci-local.sh` bloquerait tout push tant que `cargo deny` est rouge, et le contourner est interdit) :
   - `fix/ci-verte-outillage` : causes A + C + `scripts/ci-local.sh` **non branché** (le pre-push de cette branche reste l'ancien). Commits pathspec explicites, sans trailer, conventional (`fix(ci): …`, `build: …`). Push + `ls-remote`.
   - `chore/pre-push-ci-local`, créée depuis le tip de la précédente : le branchement dans `.husky/pre-push` + les 3 lignes de `CONTRIBUTING.md`. Pousse-la **avant** que le hook ne soit actif dans ton arbre (commit puis push : si le push déclenche déjà le nouveau hook et qu'il est rouge sur `cargo deny` seul, STOP propre : note-le, laisse la branche locale, l'orchestrateur la fera pousser après M0008). **Elle n'est pas mergée** par toi : l'orchestrateur la fera doubler et merger après M0008.
7. **Doublage N2** de `fix/ci-verte-outillage` : sous-agent FRAIS, worktree détaché lecture seule `.claude/worktrees/F01-R004`, rapport `vault/revues/2026-10-17-R004-ci-verte-outillage.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40>`, `niveau: N2`). Il vérifie : lint réellement corrigé (pas masqué largement), toolchain = version du dernier run CI, `claude-atelier` sans usage, `ci-local.sh` fidèle à `ci.yml` (job par job, tableau), aucun fichier de M0008 touché. Il ne lance pas cargo (règle sous-agents) : il lit tes sorties collées et le code.
8. Merge de `fix/ci-verte-outillage` (bloc « Merge » ci-dessous). Après push, la CI de `main` restera rouge sur `cargo deny` seulement : vérifie-le avec `gh run watch` et colle-le. Tout autre job rouge = à signaler en tête du rapport.

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

1. Rapport ci-dessous (`## Rapport M0007`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F01"`, `mandat: "M0007"`) et `vault/runtime/state.json` (`windows.F01` seulement, relu juste avant d'écrire) : jamais committés.
4. Commit vault sur `main` (racine si elle est sur `main`, sinon scratch détaché) : `vault/reprise/F01.md` + `vault/echanges/archive/2026-10-17-F01-M0006-etat-des-lieux-github-adn.md` ; `docs(vault): reprise F01 M0007` ; push ; `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.

## Rapport M0007


**En tête : STOP propre au push de `main`.** Tout le reste est fait et vérifié. Le merge relu et testé existe, mais `origin/main` a bougé deux fois pendant le rituel de merge. Le gabarit §9 n'autorise à refaire le scratch qu'une fois, et cette fois a déjà servi pour le merge de M0008. Le merge prêt à intégrer est poussé sur `merge/fix-ci-verte-outillage-R004` (`596e200470fc0b964073f71cac3791f273fad007`).

Scellé tests : sans objet (aucun test neuf ; `scripts/verifier-rendu.mjs` absent du dépôt)
Contre-témoins : aucun

### 1. Ce qui est livré — branche `fix/ci-verte-outillage`, tip `537e9815fe98f38908141e7b6ba8e1daf435e0f6`

| Commit | Contenu |
|---|---|
| `30649a6` fix(ci) | `rust-toolchain.toml` épinglé sur `1.99.1` (`clippy`, `rustfmt`). C'est la version du run CI `39040092235` (`rustc 1.99.1 (58cf8f9c3 2026-10-13)` dans son log). `#[allow(semicolon_in_expressions_from_non_local_macros)]` ciblé sur `fn main` de `tom-quinn/build.rs`, `tom-connect/build.rs` et `tom-relay/build.rs` |
| `9de4e18` build | retrait de `claude-atelier` (`package.json`, `pnpm-lock.yaml` : 270 lignes supprimées, 0 ajoutée) |
| `79428ed` fix(ci) | lints 1.99.1 jusque-là masqués, inventoriés en une seule passe (`clippy --keep-going`) : `deprecated` sur `fetch_update` (tom-quinn ×2, tom-metrics) ; `clippy::result_large_err` (relais de tom-connect ×3) ; `clippy::double_must_use` (trait `Transport`, généré par `async_trait`) ; `clippy::drain_collect` (tom-tui, corrigé pour de vrai par `std::mem::take`) |
| `04c345d` ci | `scripts/ci-local.sh` (mode 100755), **non branché** |
| `537e981` fix(ci) | réserve R004 : arguments réels de `cargo-deny-action@v2` |

Pourquoi des `allow` ciblés plutôt que de vrais correctifs (chaque raison est prouvée) :
- **cfg_aliases** : la 0.2.1 émet des `;` dans son expansion. La 0.2.2 les retire (diff des sources du registre, lignes 268, 287, 305). Le bump modifie `Cargo.lock`, fichier de M0008 → issue #60.
- **fetch_update** : sous `rustc +1.88.0` (MSRV de tom-quinn), `try_update` donne `error[E0658]: use of unstable library feature atomic_try_update`.
- **result_large_err** : boxer la source (`Dial { source: Box<DialError> }`) est refusé par n0-error (`Box<DialError>: n0_error::StackError` non satisfait).
- Un `allow` posé sur l'instruction est ignoré par rustc (`unused attribute allow`), d'où sa place sur `fn main`.

MSRV inchangée. Les builds tvOS/XCFramework forcent `cargo +nightly…`, ce qui prime sur `rust-toolchain.toml` : aucun conflit. Aucun autre fichier `rust-toolchain*` dans le dépôt.

Cause C : `npm view claude-atelier` → `Unpublished on 2026-09-30`. Aucun script ne l'importe (`git grep`). Seules des docs de slash-commands citent `npx claude-atelier`.

### 2. `scripts/ci-local.sh`

Il rejoue les 12 jobs de `ci.yml`, dans l'ordre, avec les mêmes commandes et options (`--test-threads=1`, `--tests`, `--lib`, `--locked`, `working-directory`, timeouts 10/5/3 min). Il tourne sous `set -euo pipefail`, s'arrête au premier échec, affiche la durée de chaque étape et dit « pas de push » en cas de rouge.
- `ci.yml` n'a **aucun filtre de chemins**. Le script lance donc tous les jobs, sans aucun mode de saut.
- Contrôle de dérive : le script échoue si la liste ou l'ordre des jobs de `ci.yml` change, ou si un `paths:` apparaît. Testé dans deux mini-dépôts (job renommé, filtre ajouté) → `exit 1`.
- Compatible bash 3.2 (`/bin/bash` de macOS). Deux pièges y ont été corrigés.
- `cargo deny --log-level warn --manifest-path ./Cargo.toml --all-features check advisories bans sources` reproduit à l'identique l'appel de l'action, d'après son `action.yml` et le log CI.

### 3. Preuves d'exécution

Run complet et **ininterrompu** de `./scripts/ci-local.sh` sur le scratch de merge `d0430d5` (= `origin/main` `07fb527`, qui inclut M0008, + `537e981`). Résultat : **TOUT VERT, EXIT=0**, `advisories ok, bans ok, sources ok`, stress 5/5, NAT 7/7.
```
3s typescript: pnpm install --frozen-lockfile | 10s pnpm build | 8s pnpm test | 1s pnpm lint
3s observability-smoke
0s/36s/1s rust-protocol build/test/clippy | 1s/36s/1s rust-transport build/test/clippy
1s/167s/16s rust-integration build/test/clippy
24s rust-fork build | tests 9s base, 4s metrics, 31s quinn-proto, 17s quinn, 20s relay, 26s gossip, 55s connect | 9s clippy
5s/55s/4s rust-dht | 1s rust-security: cargo deny
12s/14s/77s/66s/1s rust-ffi (relay-ffi build/clippy, protocol-ffi build/clippy, header drift)
1s/1s rust-macos | 23s/3s/6s rust-stress | 0s/0s/1s/8s rust-poc
[ci-local] TOUT VERT.
```
Runs précédents, au tip `04c345d` avant M0008 : vert jusqu'à `cargo deny`, rouge attendu sur RUSTSEC-2026-0258/0285/0346/0348/0349 (même ensemble que le log CI). Les 4 jobs suivants étaient verts eux aussi.

Avant correctif, sous 1.99.1 : `cargo clippy -p tom-quinn -- -D warnings` → `trailing semicolon in macro used in expression position --> crates/tom-quinn/build.rs:5:5`. Le rouge CI est donc bien reproduit localement.

Incidents de run, chacun diagnostiqué :
- Un run a été invalidé par une **mise en veille du Mac à 17:54:40** (`Low Power Sleep … Charge:1%`, 570 s, puis réveil à 18:04:10 au branchement du secteur, d'après `pmset -g log`). Pendant cette veille, `idle_then_reply` a échoué. J'ai relancé à l'identique : vert.
- `gate_rejects_unlisted_peer` (tom-transport) a échoué une fois sous charge : `closed by peer: not allowed`. En isolé, 8 succès sur 8. Aucun échec dans les 15 derniers runs CI → issue #63.

### 4. Branche `chore/pre-push-ci-local` — LOCALE, non poussée

Tip local `f19e09c` : `c648897`, qui branche `./scripts/ci-local.sh` en tête de `.husky/pre-push` (avant le smoke conditionnel existant, que `SKIP_OBSERVABILITY_SMOKE` ne peut pas contourner), et ajoute 3 lignes « Before Pushing » dans `CONTRIBUTING.md`. S'y ajoute un merge de `537e981`.
- Le push a déclenché le nouveau hook, qui a **bloqué le push** : `husky - pre-push script failed (code 101)`, `failed to push some refs`. C'est la preuve que le contrôle fonctionne.
- Le rouge venait du flake `gate_rejects_unlisted_peer` (#63), arrivé avant `cargo deny`, qui était lui-même rouge à ce moment-là. Conformément au mandat, la branche reste locale et l'orchestrateur la poussera. Comme M0008 est mergé, il faut d'abord y merger le nouveau `main`.
- **Bloquant à connaître avant de la merger (#62)** : depuis un worktree imbriqué (`.claude/worktrees/…`), cargo remonte jusqu'au `Cargo.toml` de la racine. `tom-protocol-ffi` et `experiments/iroh-poc` échouent alors avec `current package believes it's in a workspace when it's not`. Une fois le hook actif, **tout push depuis un worktree du harnais serait bloqué**. C'est pourquoi j'ai lancé mes runs dans un worktree externe (scratchpad).

### 5. Doublage N2 R004

Sous-agent frais, worktree détaché `.claude/worktrees/F01-R004`, sans cargo.
- 1er verdict **RESERVE** au tip `04c345d` : `cargo-deny-action@v2` ajoute `--all-features` (son `action.yml`, ligne 17). Je l'ai vérifié à la source et dans le log CI (`arguments: --all-features`), puis corrigé en `537e981`.
- Contre-vérification : **GO** au tip `537e9815fe98f38908141e7b6ba8e1daf435e0f6`.
- Le rapport contient le tableau job par job et des remarques R-a à R-k : contrôle de dérive limité aux ids de jobs, arbre de travail testé plutôt que HEAD, `timeout` contrôlé tard, warnings netwatch, etc.
- J'ai vérifié le rapport moi-même : frontmatter exact, `git status` du worktree = rapport seul.

### 6. Merge — STOP propre

- Scratch n°1 sur `9278dca` → `1e18dcb`. Contrôles rejoués : vert sauf `cargo deny`.
- Pendant ce temps, `origin/main` est passé à `07fb527` (M0008 mergé : locks rustls 0.23.46 / h2 0.4.20 ; un seul commentaire change dans `ci.yml`, sans dérive). J'ai refait le scratch **une fois** : `d0430d5` + rapport R004 `596e200` (`docs(revues): revue R004 -- GO`). Run complet vert (§3).
- Au moment de pousser, `origin/main` = `91b80412ae948c5ef0cab72df7c2df684839ec55` (`docs(vault): reprise F03 M0009`, seulement `vault/reprise/F03.md`). Le fast-forward est donc impossible, et le gabarit interdit un deuxième refait.
- **Question que j'aurais posée** : refaire le scratch une deuxième fois, quand le seul delta est un fichier vault ?
  - Options : (a) STOP ; (b) refaire sur `91b8041` et pousser ; (c) pousser le merge sur une branche pour l'orchestrateur.
  - **Choix appliqué : (a) + (c).** J'aurais choisi (b), puisque le delta est hors code et ne change aucun contrôle CI, mais la règle est explicite.
  - Pour intégrer : scratch sur `origin/main`, `merge --no-ff 537e981`, copier `vault/revues/2026-10-17-R004-ci-verte-outillage.md` depuis `596e200`, rejouer `ci-local.sh` (hors worktree imbriqué, #62), puis fast-forward.
- Effet attendu après intégration : **CI `main` entièrement verte**, puisque M0008 est déjà mergé. Non vérifié par `gh run watch`, faute de push sur `main`.
- `git ls-remote` à 18:20 :
```
537e9815fe98f38908141e7b6ba8e1daf435e0f6	refs/heads/fix/ci-verte-outillage
91b80412ae948c5ef0cab72df7c2df684839ec55	refs/heads/main
596e200470fc0b964073f71cac3791f273fad007	refs/heads/merge/fix-ci-verte-outillage-R004
```

### 7. Écarts au gabarit (signalés)

- commitlint (`.husky/commit-msg`, config-conventional) refuse `Merge fix/ci-verte-outillage [R004]` (sujet vide) et `docs(revues): R004 -- GO` (subject-case). J'ai utilisé `Merge branch 'fix/ci-verte-outillage' [R004]` et `docs(revues): revue R004 -- GO`, sans `--no-verify`. Les gabarits §9 sont à aligner.
- Le scratch était dans le scratchpad et non dans `.claude/worktrees/F01-merge`, à cause de #62.
- `core.fileMode=false` : le bit exécutable passe par `git update-index --chmod=+x`. J'ai amendé `04c345d` avant tout push.
- Effet de bord machine : `rustup` s'est auto-mis à jour en 1.29.1 et les toolchains 1.99.1 et 1.88.0 (minimal) sont installées.

### 8. Issues ouvertes et hygiène

- Issues : #60 (bump cfg_aliases 0.2.2 puis retrait des `allow`), #61 (clé de lint dépréciée `unused-async` + bench tom-quinn absent), #62 (worktrees imbriqués et cargo), #63 (flake `gate_rejects_unlisted_peer`).
- Disque : chaque `target/` est nettoyé dès 15 Go ou plus. Un run complet monte un `target/` vierge jusqu'à 18–21 Go : le plafond de 20 Go a été atteint deux fois en fin de run, avec `cargo clean` juste après. État final : `target/` nettoyés.
- Aucun process `tom-*`/`nat-test` résiduel. Aucune tâche de fond.

Leçons candidates à la remontée (R5) :
1. Un worktree imbriqué dans le dépôt casse les crates cargo exclus du workspace.
2. Un rouge en masque un autre : il faut inventorier toute la dette lint avec `--keep-going` avant de corriger.
3. `pmset -g log` d'abord quand un test temporel échoue sur portable.
4. Les gabarits de commit doivent passer commitlint.

### 9. Rituels de fin

- Commit vault `c0655ca` (`docs(vault): reprise F01 M0007`) : `vault/reprise/F01.md` + `vault/echanges/archive/2026-10-17-F01-M0006-etat-des-lieux-github-adn.md`, poussé depuis un scratch détaché sur `91b8041` (fast-forward). `git ls-remote origin refs/heads/main` → `c0655cacbd93251fa19a16800db9299497a7c5b7`. Ce push lance la CI `main` sur un arbre **sans** ce correctif : elle restera rouge (clippy) jusqu'à l'intégration de `596e200`. Je ne l'ai pas surveillée, car ce n'est pas le push de merge.
- La racine contient encore une copie non suivie de ce fichier d'archive, désormais suivi sur `main`. Un futur `merge --ff-only` à la racine la refusera : il faudra la retirer d'abord (contenu identique).
- `vault/runtime/events.jsonl` (RAPPORT, `src: fenetre`) et `state.json` (`windows.F01`, `state: RENDU`) sont écrits et ignorés par git.
- Worktrees `F01-M0007`, `F01-R004` et le scratch retirés (`git worktree list` = racine seule). Les branches locales `fix/ci-verte-outillage` (@537e981, poussée) et `chore/pre-push-ci-local` (@f19e09c, locale) sont conservées. Aucun process ni tâche de fond restant.

M0007
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ✅
Merge main          ❌
Main push           ❌
TASK STATUS         READY_FOR_INTEGRATION
PROJECT STATUS      NOT_INTEGRATED
