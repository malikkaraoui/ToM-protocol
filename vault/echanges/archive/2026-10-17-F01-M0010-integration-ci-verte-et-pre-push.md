---
date: 2026-10-17
tags: [ci, integration, pre-push]
session: F01
mandat: integration-ci-verte-et-pre-push
mandat_id: M0010
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol-scratch-M0010
branche: main
niveau: N2
derniere_maj: 2026-10-17T18:29:52+0200
---

# F01 / M0010 — Intégrer `fix/ci-verte-outillage` puis `chore/pre-push-ci-local` (CI verte + contrôle pre-push obligatoire)

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, https://github.com/tobi/qmd ; index `tom` à jour depuis M0006) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.
> **IMPÉRATIF (fondateur, 2026-10-17) : on ne pousse pas si ce n'est pas nickel.** Avant CHAQUE `git push`, tu rejoues en local les contrôles de la CI (`.github/workflows/ci.yml`) qui couvrent ce que tu as touché, et tu colles leurs sorties. Rouge = pas de push.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

Suite de M0007 (archive `vault/echanges/archive/2026-10-17-F01-M0007-ci-verte-outillage-et-pre-push-miroir.md`, lis son rapport via QMD/`Read` ciblé). M0007 a tout fait et fait relire (R004 **GO** au tip `537e9815fe98f38908141e7b6ba8e1daf435e0f6` ; `scripts/ci-local.sh` **tout vert** sur un scratch = `07fb527` + `537e981`), mais s'est arrêté proprement au push : `origin/main` avait bougé deux fois (commits `vault/` seulement). Feu vert du fondateur pour ces intégrations : 2026-10-17 16:39.
Aucune autre fenêtre ne merge pendant ce mandat. `fix/retrait-ip-publique` attend un arbitrage du fondateur : **ne la touche pas**.

Piège connu (#62) : un worktree **imbriqué** dans le dépôt (`.claude/worktrees/…`) fait remonter cargo au `Cargo.toml` racine et casse `tom-protocol-ffi` / `experiments/iroh-poc`. **Tes scratchs vont HORS du dépôt** : `/Users/malik/Documents/tom-protocol-scratch-M0010` (et `-R008` pour le doublage). Dans un scratch, `pnpm`/`biome` : `pnpm install --frozen-lockfile` local ou `PATH=/Users/malik/Documents/tom-protocol/node_modules/.bin:$PATH`.

## Gardes de précondition

1. `git fetch origin` ; `git rev-parse origin/main` = `c0655cacbd93251fa19a16800db9299497a7c5b7` (commits `vault/` seuls depuis `07fb527`). Autre valeur : vérifie que l'écart ne touche que `vault/` (`git diff --stat c0655ca origin/main`) ; sinon STOP.
2. `git rev-parse origin/fix/ci-verte-outillage` = `537e9815fe98f38908141e7b6ba8e1daf435e0f6`. La branche locale `chore/pre-push-ci-local` existe, tip `f19e09c…` (`git rev-parse chore/pre-push-ci-local`). Sinon STOP.
3. Racine : `git branch --show-current` = `main`, `git status --short` = intouchables seuls + `?? vault/echanges/…` non suivis. Sinon STOP.

## Mission

1. Rituel de début : `statut: en-cours`.
2. **Intégration 1 — `fix/ci-verte-outillage`** : `git worktree add --detach /Users/malik/Documents/tom-protocol-scratch-M0010 origin/main` ; `merge --no-ff 537e981… -m "Merge fix/ci-verte-outillage [R004]"` (message conforme à commitlint : M0007 a noté que le gabarit brut est refusé ; adapte le format, garde `[R004]`) ; récupère le rapport R004 depuis le commit `596e200` (`git show 596e200:vault/revues/2026-10-17-R004-ci-verte-outillage.md`) et committe-le (`docs(revues): R004 -- GO`) ; **`./scripts/ci-local.sh` complet sur le scratch : EXIT=0 exigé** (sortie résumée + durées collées). Flake `gate_rejects_unlisted_peer` (#63) : rejoue le job seul une fois ; rouge persistant → STOP. Puis à la racine `git merge --ff-only <HEAD scratch>` + `git push origin main` + `ls-remote` ; puis `gh run watch <id> --exit-status` (≤ 30 min) : **la CI de `main` doit être entièrement verte** — colle le résultat par job. Rouge → STOP, rien de plus (pas d'intégration 2).
3. **Intégration 2 — `chore/pre-push-ci-local`** (seulement si 2 est vert en CI) : dans un worktree hors dépôt sur cette branche, `merge` de `origin/main` dedans (jamais de rebase) ; vérifie que `.husky/pre-push` lance `./scripts/ci-local.sh` en premier, **sans échappatoire** (aucune variable qui le saute ; si `SKIP_OBSERVABILITY_SMOKE` existe, il ne doit concerner que le smoke) ; push de la branche (le hook tourne : il doit passer) ; **doublage N2** par sous-agent FRAIS, worktree détaché lecture seule `/Users/malik/Documents/tom-protocol-scratch-R008`, rapport `vault/revues/2026-10-17-R008-pre-push-ci-local.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40>`, `niveau: N2`) ; GO → merge `--no-ff` sur scratch, commit R008, `ff-only` racine, push (le hook tourne à ce push aussi : c'est la preuve), `gh run watch` vert.
4. Démonstration que le contrôle bloque : dans un worktree jetable hors dépôt, un commit qui casse volontairement `cargo fmt --check` ou un test, `git push --dry-run` ne suffit pas → utilise `git push origin HEAD:refs/heads/tmp/preuve-pre-push-M0010` : le hook doit **refuser** (sortie collée). Puis supprime le worktree jetable ; la branche distante ne doit pas exister (`ls-remote` vide).
5. Mets à jour `CONTRIBUTING.md` seulement si M0007 ne l'a pas fait correctement (remplacer, jamais ajouter).

## Règles des sous-agents (verbatim)

(1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre. **Aucun sous-agent ne lance de build ou de test cargo** (incident M0006 : `target/` à 23 Go) : les builds restent à la session mère.

## Hygiène disque

Plafond `target/` 20 Go (`CLAUDE.md` racine) : `du -sh target/` avant et après chaque build lourd ; au-delà de 15 Go, `cargo clean` avant de continuer. Rien ne tourne en fond à la sortie.

## Intouchables

`scripts/pre-push-gate.sh` (modifié par le fondateur, non committé), `vault/30-discoveries.md`, `.claude/settings.json`, les `vault/echanges/F0x.md` vivants (jamais committés), la branche `fix/retrait-ip-publique` (en attente d'arbitrage du fondateur).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée ; jamais `--no-verify` ; jamais de force-push.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0010`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F01"`, `mandat: "M0010"`) et `vault/runtime/state.json` (`windows.F01` seulement, relu juste avant d'écrire) : jamais committés.
4. Commit vault sur `main` (racine si elle est sur `main`, sinon scratch détaché) : `vault/reprise/F01.md` + `vault/echanges/archive/2026-10-17-F01-M0007-ci-verte-outillage-et-pre-push-miroir.md` ; `docs(vault): reprise F01 M0010` ; push ; `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.


## Rapport M0010


**En tête : INTEGRATED.** Les deux branches sont sur `main` (`a1039ea9f7851cf58110943e5ea95ed6aadb2950`), CI GitHub verte 12/12 aux deux étapes. **Mais la démo de blocage a révélé une faille du contrôle : dans un worktree/clone neuf, aucun hook ne tourne, en silence (#64).** Hooks installés, le refus est prouvé.

Scellé tests : sans objet (aucun test neuf livré ; `scripts/verifier-rendu.mjs` absent du dépôt)
Contre-témoins : aucun

### 0. Gardes
`origin/main` = `c0655ca…` ✔ ; `origin/fix/ci-verte-outillage` = `537e981…` ✔ ; `chore/pre-push-ci-local` = `f19e09c8ab661b052b069292cf96738e5ae04300` ✔ ; racine sur `main`. Écart noté : `?? vault/revues/2026-10-17-R007-retrait-ip-publique.md` (produit par M0009, hors `vault/echanges/`), sans effet sur ce mandat, non touché.

### 1. Intégration 1 — `fix/ci-verte-outillage` [R004]
- Scratch `/Users/malik/Documents/tom-protocol-scratch-M0010` détaché sur `c0655ca` ; `a2ad877` « Merge branch 'fix/ci-verte-outillage' [R004] » (forme acceptée par commitlint) ; `944819fd1635a7816477e08aa64230e1e7f8e901` « docs(revues): revue R004 -- GO » (rapport extrait de `596e200`). `git diff --stat 596e200 944819f` = `vault/` seul → code identique au merge relu.
- `./scripts/ci-local.sh` complet sur le scratch : **`[ci-local] TOUT VERT.` EXIT=0**, `advisories ok, bans ok, sources ok`. Durées : TS 1/4/4/1 s · smoke 39 s · protocol 38/47/28 s · transport 29/44/15 s · integration 27/162/12 s · fork 15 s + tests 5/3/17/12/13/18/40 s + clippy 8 s · dht 5/55/3 s · deny 1 s · ffi 9/7/71/41/0 s · macos 1/0 s · stress 8/2/3 s · poc 0/37/18/8 s. Pas de flake #63.
- Racine : la copie non suivie `vault/echanges/archive/2026-10-17-F01-M0006-…` bloquait le ff ; `cmp` = **identique** au blob de `c0655ca` → retirée, ff, restaurée (`cmp` OK). `git push origin main` : `c0655ca..944819f  main -> main` ; `ls-remote` = `944819fd1635a7816477e08aa64230e1e7f8e901 refs/heads/main`.
- `gh run watch 39050932647 --exit-status` → EXIT=0, **success 12/12** : TypeScript, Rust protocol, transport, cross-crate integration, fork, DHT, supply chain (cargo-deny), FFI, macOS, stress, PoC, Observability smoke.

### 2. Intégration 2 — `chore/pre-push-ci-local` [R008]
- Dans le scratch : `dce9ce1464456a10916d3346ba93d97ec47d6c35` = merge de `origin/main` dans la branche (pas de rebase). Écart vs `main` : `.husky/pre-push` (+3) et `CONTRIBUTING.md` (+6) seuls.
- Hook : `.husky/pre-push:3` `set -eu`, `:7` `./scripts/ci-local.sh` en premier ; `SKIP_/FORCE_OBSERVABILITY_SMOKE` (`:10`, `:25`) ne touchent que le smoke placé après. `ci-local.sh` ne lit aucune variable de saut (grep).
- **Push de la branche, 1re tentative** : hook `TOUT VERT` puis **EXIT=141 (SIGPIPE)**, branche absente du distant. Cause : `git push` ouvre la session SSH avant le hook ; ~20 min sans trafic → session fermée. **2e tentative**, seule différence `GIT_SSH_COMMAND="ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=10"` (le hook tourne pareil, aucun contournement) → `* [new branch] chore/pre-push-ci-local` ; `ls-remote` = `dce9ce1464456a10916d3346ba93d97ec47d6c35 refs/heads/chore/pre-push-ci-local`. → issue #65.
- **Doublage N2 R008** : sous-agent frais, worktree détaché `/Users/malik/Documents/tom-protocol-scratch-R008`, sans cargo. **verdict: GO**, `tip: dce9ce1464456a10916d3346ba93d97ec47d6c35`, `niveau: N2`. Vérifié moi-même : frontmatter exact, lignes citées (`pre-push:3/7/10/25`, `ci-local.sh:17-20`, `:77-78` = `die` si `timeout` absent, pas d'échappatoire), `git status` du worktree = rapport seul. Remarques non bloquantes → #66 (formulations CONTRIBUTING trop fortes).
- Scratch détaché sur `944819f` : `9207b40` « Merge branch 'chore/pre-push-ci-local' [R008] », `a1039ea9f7851cf58110943e5ea95ed6aadb2950` « docs(revues): revue R008 -- GO ». ff-only racine → `a1039ea`. Push depuis le scratch (même commit ; `target/` racine épargné) : **le nouveau hook a tourné** — `[ci-local] TOUT VERT.` (830 s cumulés), puis `944819f..a1039ea  HEAD -> main` ; `ls-remote` = `a1039ea9f7851cf58110943e5ea95ed6aadb2950 refs/heads/main`.
- `gh run watch 39054751463 --exit-status` → EXIT=0, **success 12/12** (mêmes jobs).

### 3. Démonstration du blocage — et faille découverte
- Worktree jetable `/Users/malik/Documents/tom-protocol-scratch-M0010-preuve` sur `a1039ea`, ajout de `packages/core/src/preuve-pre-push.test.ts` (`expect(1).toBe(2)`), commit.
- **1re tentative : PUSH ACCEPTÉ** (`* [new branch] HEAD -> tmp/preuve-pre-push-M0010`). Diagnostic (status/log d'abord) : `core.hooksPath=.husky/_`, et `.husky/_` **n'existe pas** dans un worktree neuf (il est généré par `prepare: husky` au `pnpm install`) → git ne lance aucun hook, sans message (le commit avait aussi échappé à commitlint). Branche distante supprimée aussitôt (`- [deleted]`). → **issue #64**.
- Après `pnpm install --frozen-lockfile` dans ce worktree (`.husky/_/pre-push` présent), même push → **refusé** :
```
 FAIL  |tom-protocol| src/preuve-pre-push.test.ts > preuve pre-push M0010 > casse volontairement
AssertionError: expected 1 to be 2 // Object.is equality
      Tests  1 failed | 771 passed (772)
[ci-local] ÉCHEC (code 1) à l'étape : typescript: pnpm test — pas de push.
husky - pre-push script failed (code 1)
error: failed to push some refs to 'github.com:malikkaraoui/ToM-protocol.git'
```
- `git ls-remote origin 'refs/heads/tmp/*'` → **0 ligne**. Worktree jetable supprimé.
- Écart au mandat : test TS cassé plutôt que `cargo fmt --check` — `ci.yml` (donc `ci-local.sh`) ne contient **aucun** `cargo fmt --check` ; casser fmt n'aurait rien prouvé. Le test TS échoue au 1er job (secondes).

### 4. CONTRIBUTING.md
Section « Before Pushing » de M0007 unique, sans doublon sur `main` → laissée telle quelle. Deux formulations plus fortes que le code (#66) : à reformuler, pas dans ce mandat (aurait changé le tip relu).

### 5. Hygiène
`target/` du scratch : **~28 Go cumulés par run** (target 20 Go + FFI 5,9 + poc 1,8) → `cargo clean` des trois après chaque run (4×). Racine `target/` 1,6 Go. Tous les worktrees retirés (`git worktree list` = racine seule) ; 0 process `tom-relay`/`ci-local`/`gh run` vivant.

### 6. Issues ouvertes
- **#64** (important) : pre-push/commit-msg inactifs en silence dans tout worktree/clone sans `pnpm install` — les worktrees du harnais poussent sans contrôle local. Piste : rituel « `pnpm install` + `test -x .husky/_/pre-push` avant tout commit » dans les gabarits de mandat.
- **#65** : hook ~15-20 min + SSH → SIGPIPE ; `ServerAliveInterval 30` le résout (prouvé). À documenter (CONTRIBUTING ou `~/.ssh/config`).
- **#66** : formulations CONTRIBUTING.

### 7. Pour l'orchestrateur
- Chaque push coûte désormais ~15-20 min de hook et ~28 Go de `target/` transitoire en worktree neuf : à intégrer aux plafonds disque des mandats (cargo clean entre pushes).
- Le commit cassé de la démo reste dangling dans la base d'objets locale (jamais sur une branche distante), sans effet.

### 8. Commit vault
`99aa090` « docs(vault): reprise F01 M0010 » (reprise + archive M0007), poussé depuis la racine, hook `TOUT VERT` : `a1039ea..99aa090  main -> main` ; `ls-remote` = `99aa090a38650d1dd3813d3a16c9c999c6605330 refs/heads/main`. `cargo clean` racine ensuite (25 + 11,8 + 1,8 Go).

M0010
Implementation      ✅ merges a2ad877, 9207b40
Tests               ✅ ci-local TOUT VERT ×3 + CI main 12/12 (39050932647, 39054751463)
Commit              ✅ 944819f, a1039ea
Branch push         ✅ chore/pre-push-ci-local dce9ce1
Review              ✅ R004 GO (537e981) · R008 GO (dce9ce1)
Merge main          ✅ a1039ea
Main push           ✅ a1039ea
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED
