---
date: 2026-10-17
tags: [harnais, pose, mission-0]
session: F01
mandat: commit-pose-harnais-v1.6.0-biome
mandat_id: M0002
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F01-M0002
branche: chore/harnais-v1.6.0
niveau: N2
derniere_maj: 2026-10-17T15:18:47+0200
---

# F01 / M0002 — Enregistrer la pose du harnais v1.6.0 (reprise de M0001, avec biome.json)

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Contexte (autonome)

**Reprise de M0001** (archive : `vault/echanges/archive/2026-10-17-F01-M0001-commit-pose-harnais-v1.6.0.md`, lis son rapport). M0001 s'est arrêté proprement : le hook husky `pre-commit` (`pnpm lint` = `biome check .`) refuse `config/harnais.json` (format) et, selon qui l'a écrit en dernier, `vault/runtime/state.json`. **Décision du fondateur, 2026-10-17 15:17 (« OK, c'est bon ») : option A** — ajouter `config/harnais.json` et `vault/runtime` à `files.ignore` de `biome.json`, dans le même commit que la pose.

Le harnais v1.6.0 a été posé sur ce dépôt le 2026-10-17 vers 15:07 par `harnais equiper`. Le moteur ne committe rien : ses fichiers sont non suivis ou modifiés à la racine. Ce mandat les enregistre, sur une branche, avec relecture N2 indépendante, puis merge sur `main` (décision du fondateur #41 : sur `main`, commits directs réservés à `vault/` ; tout le reste passe par branche + doublage + merge).

Tu travailles **à la racine** `/Users/malik/Documents/tom-protocol` : les fichiers à committer n'existent que là. Le worktree du frontmatter sert au scratch de merge.

Convention : lis `.claude/skills/orchestration-bureau/SKILL.md` et `references/gabarits.md` (§2, §3, §4, §9) avant d'agir. `config/harnais.json` : `git.trailer` vaut le gabarit non rempli `Co-Authored-By: <signature du projet>` et les commits récents de `main` n'ont aucun trailer → **aucun trailer** dans tes messages ; dis-le au rapport.

## Fichiers de l'utilisateur — INTOUCHABLES

Modifiés à la racine, hors harnais, à laisser tels quels (jamais add, jamais checkout, jamais stash) :
- `scripts/pre-push-gate.sh`
- `vault/30-discoveries.md`
- `.claude/settings.local.json` (non suivi, réglage local : ne pas committer)
- `vault/echanges/F01.md` (ce mandat : non suivi, propriété de la fenêtre, jamais committé)
- `vault/echanges/archive/2026-10-17-F01-M0001-commit-pose-harnais-v1.6.0.md` et `vault/reprise/F01.md` (non suivis, laissés par M0001 : committés au rituel de fin, étape 4)

Tout autre fichier sale ou non suivi que ceux listés ici et dans la liste à committer → STOP.

## Gardes de précondition

1. `git fetch origin` puis `git rev-parse HEAD origin/main` : les deux = `fdae163f3754c9f49aa109f4e0b3a494a447b397`. Sinon STOP.
2. `git branch --show-current` = `main`. Sinon STOP.
3. `git status --short` ne liste QUE les chemins de ce mandat (liste à committer + intouchables). Sinon STOP.
4. `git diff -- biome.json` vide (fichier propre au départ), sinon STOP. `git diff -- CLAUDE.md` = exactement une ligne ajoutée en fin : `@~/.harnais/canon/templates/tronc/harnais.md`. `git diff -- .gitignore` = uniquement le bloc ajouté en fin commençant par `# harnais:registres`. Toute autre différence dans ces deux fichiers → STOP (contenu de l'utilisateur mêlé).

## Mission

1. Rituel de début : `statut: en-cours` dans ce frontmatter.
2. `git checkout -b chore/harnais-v1.6.0` (le hook `post-checkout` graphify de l'utilisateur sort en code 1 : connu, non bloquant si la branche courante est la bonne) puis `git branch --show-current` = `chore/harnais-v1.6.0`, sinon STOP.
3. Commit en pathspec explicite, ces chemins seulement :
   ```
   .claude/skills/orchestration-bureau/SKILL.md
   .claude/skills/orchestration-bureau/references/gabarits.md
   config/harnais.json
   scripts/harnais-hooks/README.md scripts/harnais-hooks/commit-msg scripts/harnais-hooks/pre-commit scripts/harnais-hooks/pre-merge-commit scripts/harnais-hooks/pre-push
   vault/decisions/.gitkeep vault/echanges/README.md vault/echanges/archive/.gitkeep vault/hypotheses/.gitkeep
   vault/lecons/tom-protocol/.gitkeep vault/lecons/transverse/.gitkeep
   vault/reprise/00_INDEX.md vault/reprise/CARNET_DE_BORD.md vault/reprise/CARTOGRAPHIE_BRANCHES.md
   vault/reprise/archive/index/.gitkeep vault/reprise/etat-technique.md
   vault/revues/.gitkeep vault/runtime/README.md
   .gitignore CLAUDE.md
   ```
   **Avant le commit** : dans `biome.json`, ajoute à la fin du tableau `files.ignore` exactement deux entrées, `"config/harnais.json"` et `"vault/runtime"`, rien d'autre (`git diff -- biome.json` = ces deux lignes, plus la virgule de la ligne précédente). Puis `pnpm lint` à la racine doit sortir en code 0 ; sinon STOP, sortie collée (n'élargis pas l'ignore au-delà de ces deux entrées, ne reformate aucun autre fichier). Ajoute `biome.json` à la liste ci-dessus.
   `git add -- <ces chemins>` puis commit, message par heredoc : `chore(harnais): pose du harnais v1.6.0`. Jamais `-a`/`-A`. Vérifie `git show --stat HEAD` : exactement ces fichiers.
4. `git push -u origin chore/harnais-v1.6.0` + `git ls-remote origin refs/heads/chore/harnais-v1.6.0` collé. Un hook (husky) qui refuse → STOP, sortie collée, jamais `--no-verify`.
5. **Doublage N2** : un sous-agent FRAIS (jamais `fork`), worktree détaché lecture seule sur le tip (`git worktree add --detach .claude/worktrees/F01-R001 <tip>`). Il vérifie : le commit ne contient que des fichiers du harnais ; aucun fichier de l'utilisateur ; `CLAUDE.md` = une seule ligne ajoutée ; `.gitignore` = seulement le bloc harnais ; `biome.json` = seulement les deux entrées d'ignore ; aucun secret (`.env` absent du commit) ; `config/harnais.json` est du JSON valide (`node -e` ou `python3 -m json.tool`). Verdict par axe ✅/⚠️/⛔ avec la raison vérifiée. Tu écris son rapport dans `vault/revues/2026-10-17-R001-pose-harnais.md`, frontmatter fermé par `---` avec exactement une ligne `verdict: GO` (ou `RESERVE`), une ligne `tip: <sha 40 du tip>`, une ligne `niveau: N2`. RESERVE → STOP, pas de merge.
   Règles des sous-agents (verbatim) :
   ```
   (1) lecture/recherche parallele = agents FRAIS (jamais fork) ;
   (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent -- ces actes restent a la session mere ;
   (3) tout rapport de sous-agent est VERIFIE (citations fichier:ligne recontrolees) avant d'entrer dans un livrable ;
   (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ;
   (5) ATTENDRE LE RETOUR DE TOUS les sous-agents lances AVANT le premier rituel de fin. Attendre = rendre la main jusqu'a la notification, jamais sleep ni sondage. Echec ou arret de l'un = STOP propre, dit au rapport.
   ```
6. **Merge** (gabarits §9) : scratch `.claude/worktrees/F01-M0002` détaché sur `origin/main`, `merge --no-ff <tip> -m "Merge chore/harnais-v1.6.0 [R001]"`, copie et commit du rapport R001 dans le scratch (`docs(revues): R001 -- GO`). Tests : aucun code touché ; rejoue `pnpm lint` sur le scratch (code 0 exigé), colle la sortie. Puis à la racine : `git checkout main` (les fichiers du harnais disparaissent un instant de l'arbre : normal) puis `git merge --ff-only $(git -C .claude/worktrees/F01-M0002 rev-parse HEAD)`, `git push origin main`, `git ls-remote origin refs/heads/main` collé. Refus `ff-only` → rapport, jamais de force. Ensuite `git worktree remove` du scratch et du worktree R001.
7. Vérifie à la fin : `git status --short` ne liste plus que les fichiers intouchables ci-dessus ; les fichiers du harnais sont présents à la racine.

## Conditions STOP

Garde en échec ; fichier inattendu ; hook qui refuse ; conflit au merge ; `ff-only` refusé deux fois ; doublage RESERVE ; tout imprévu sur l'état git (alors NE RIEN pousser et le dire).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous la forme `"${VAR:?}/${SOUS:?}"` ; jamais `--no-verify`.

## Preuves exigées

Sorties collées : `git diff -- biome.json`, `pnpm lint` (racine et scratch), gardes 1 à 4, `git show --stat` du commit, `ls-remote` de la branche et de `main`, `git log -1 --format=%B` du merge, `git status --short` final.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0002`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` : 5 lignes au plus, ce qui est fait et le SHA de `main`.
3. `vault/runtime/events.jsonl` (ajout, `src: "fenetre"`, `f: "F01"`, `mandat: "M0002"`) et `vault/runtime/state.json` (entrée `windows.F01`) : écrits, **jamais committés**.
4. Commit du vault restant directement sur `main` (`vault/reprise/F01.md`, réécrit pour M0002, et `vault/echanges/archive/2026-10-17-F01-M0001-commit-pose-harnais-v1.6.0.md` ; jamais `vault/echanges/F01.md`) : `docs(vault): reprise F01 M0002`, push, `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible` dans ce frontmatter.


## Rapport M0002


**Résultat : harnais v1.6.0 enregistré et intégré.** Branche `chore/harnais-v1.6.0` (tip `ff0ce89198ad73c6bfb733e334c449acfb91f33a`), doublage N2 R001 = GO, merge sur `main` = `8ed5de0b0b428529618d03b30bbdafa399ceb513`, poussé.

### Gardes de précondition

```
== G1  git fetch origin ; git rev-parse HEAD origin/main
fdae163f3754c9f49aa109f4e0b3a494a447b397
fdae163f3754c9f49aa109f4e0b3a494a447b397
== G2  git branch --show-current
main
== G3  git status --short -uall
 M .gitignore
 M CLAUDE.md
 M scripts/pre-push-gate.sh
 M vault/30-discoveries.md
?? .claude/skills/orchestration-bureau/SKILL.md
?? .claude/skills/orchestration-bureau/references/gabarits.md
?? config/harnais.json
?? scripts/harnais-hooks/{README.md,commit-msg,pre-commit,pre-merge-commit,pre-push}
?? vault/decisions/.gitkeep
?? vault/echanges/F01.md
?? vault/echanges/README.md
?? vault/echanges/archive/.gitkeep
?? vault/echanges/archive/2026-10-17-F01-M0001-commit-pose-harnais-v1.6.0.md
?? vault/hypotheses/.gitkeep
?? vault/lecons/tom-protocol/.gitkeep
?? vault/lecons/transverse/.gitkeep
?? vault/reprise/{00_INDEX.md,CARNET_DE_BORD.md,CARTOGRAPHIE_BRANCHES.md,F01.md,etat-technique.md}
?? vault/reprise/archive/index/.gitkeep
?? vault/revues/.gitkeep
?? vault/runtime/README.md
== G4  git diff -- biome.json   → vide
       git diff -- CLAUDE.md    → @@ -555,3 +555,4 @@ … +@~/.harnais/canon/templates/tronc/harnais.md (seule ligne)
       git diff -- .gitignore   → @@ -78,3 +78,57 @@ : uniquement le bloc ajouté en fin, ouvert par `# harnais:registres`
```
(G3 : les 5 fichiers `scripts/harnais-hooks/` et 5 de `vault/reprise/` sont abrégés en accolades ici ; la sortie réelle les liste un par ligne. `.claude/settings.local.json` n'apparaît pas : ignoré par git, donc hors risque.)
Toutes vertes.

### Déroulé

1. `statut: en-cours`.
2. `git checkout -b chore/harnais-v1.6.0` → `husky - post-checkout script failed (code 1)` (graphify, connu) ; `git branch --show-current` = `chore/harnais-v1.6.0`.
3. `biome.json` : deux entrées ajoutées en fin de `files.ignore`.
   ```
   $ git diff -- biome.json
   @@ -46,7 +46,9 @@
          "sdk/swift/**/Artifacts/**",
          "docs/spec/vectors/**",
          "patches/**",
   -      "graphify-out"
   +      "graphify-out",
   +      "config/harnais.json",
   +      "vault/runtime"
        ]
      }
    }
   $ pnpm lint   (racine)
   > tom-protocol-workspace@2.1.0 lint /Users/malik/Documents/tom-protocol
   > biome check .
   Checked 147 files in 139ms. No fixes applied.
   rc=0
   ```
   `git add -- <24 chemins>` puis `git commit -F -` (heredoc, message `chore(harnais): pose du harnais v1.6.0`). Le pre-commit husky passe.
   **Aucun trailer** : `git.trailer` vaut le gabarit non rempli `Co-Authored-By: <signature du projet>`, les commits de `main` n'en ont pas, et le `CLAUDE.md` projet (§13) interdit le trailer `Co-Authored-By`.
   ```
   $ git show --stat HEAD
   ff0ce89198ad73c6bfb733e334c449acfb91f33a chore(harnais): pose du harnais v1.6.0
    .claude/skills/orchestration-bureau/SKILL.md       | 410 ++++++++++++
    .../orchestration-bureau/references/gabarits.md    | 366 +++++++++++
    .gitignore                                         |  54 ++
    CLAUDE.md                                          |   1 +
    biome.json                                         |   4 +-
    config/harnais.json                                |  94 +++
    scripts/harnais-hooks/README.md                    | 372 +++++++++++
    scripts/harnais-hooks/commit-msg                   |  80 +++
    scripts/harnais-hooks/pre-commit                   | 691 +++++++++++++++++++++
    scripts/harnais-hooks/pre-merge-commit             |  14 +
    scripts/harnais-hooks/pre-push                     | 393 ++++++++++++
    vault/decisions/.gitkeep                           |   0
    vault/echanges/README.md                           |  82 +++
    vault/echanges/archive/.gitkeep                    |   0
    vault/hypotheses/.gitkeep                          |   0
    vault/lecons/tom-protocol/.gitkeep                 |   0
    vault/lecons/transverse/.gitkeep                   |   0
    vault/reprise/00_INDEX.md                          |  41 ++
    vault/reprise/CARNET_DE_BORD.md                    |  29 +
    vault/reprise/CARTOGRAPHIE_BRANCHES.md             |  56 ++
    vault/reprise/archive/index/.gitkeep               |   0
    vault/reprise/etat-technique.md                    |  39 ++
    vault/revues/.gitkeep                              |   0
    vault/runtime/README.md                            | 215 +++++++
    24 files changed, 2940 insertions(+), 1 deletion(-)
   ```
4. `git push -u origin chore/harnais-v1.6.0` → accepté (pre-push husky : `observability smoke skipped`).
   ```
   $ git ls-remote origin refs/heads/chore/harnais-v1.6.0
   ff0ce89198ad73c6bfb733e334c449acfb91f33a	refs/heads/chore/harnais-v1.6.0
   ```
5. Doublage N2 : sous-agent frais `general-purpose` (pas `fork`), worktree détaché `.claude/worktrees/F01-R001` sur `ff0ce89…`. Les 8 axes sont ✅ (périmètre, fichiers de l'utilisateur, `CLAUDE.md`, `.gitignore`, `biome.json`, secrets, JSON, lint). **Verdict GO.** J'ai recontrôlé ses citations (règle 3) : 24 fichiers, parent `fdae163…`, 0 ligne supprimée dans `CLAUDE.md`/`.gitignore`. Les deux faux positifs secrets ont été relus : `pre-commit:305` est une ligne de tirets, `pre-push:372` le SHA de l'arbre vide dans `--attr-source`. JSON valide, worktree propre. Le sous-agent est revenu avant les rituels de fin. Rapport : `vault/revues/2026-10-17-R001-pose-harnais.md` (`verdict: GO`, `tip: ff0ce89198ad73c6bfb733e334c449acfb91f33a`, `niveau: N2`).
6. Merge (§9) : `git fetch origin`, puis `main` = `origin/main` = `fdae163…` ; scratch `.claude/worktrees/F01-M0002` détaché sur `origin/main`. Ensuite `merge --no-ff ff0ce89…` → `e8debf3`, puis copie et commit de R001 → `8ed5de0` (`docs(revues): R001 -- GO`).
   Lint scratch : `pnpm lint` brut sort en `sh: biome: command not found` / `ELIFECYCLE`, rc=1. La cause est l'environnement, pas le contenu : un worktree n'a pas de `node_modules`. Contournement : même commande, binaire de la racine ajouté au PATH.
   ```
   $ PATH=/Users/malik/Documents/tom-protocol/node_modules/.bin:$PATH pnpm lint   (scratch)
   > tom-protocol-workspace@2.1.0 lint /Users/malik/Documents/tom-protocol/.claude/worktrees/F01-M0002
   > biome check .
   Checked 146 files in 123ms. No fixes applied.
   rc=0
   ```
   À la racine, la copie non suivie de R001 était identique (`cmp`) à celle du scratch. Je l'ai retirée (`rm --` du seul fichier, que j'avais créé) pour qu'elle ne bloque pas le ff. Ensuite `git checkout main` (post-checkout graphify code 1, connu) et `git merge --ff-only 8ed5de0…` → OK. `git push origin main` → `fdae163..8ed5de0  main -> main`. Le pre-push accepte le merge avec son rapport GO.
   ```
   $ git ls-remote origin refs/heads/main
   8ed5de0b0b428529618d03b30bbdafa399ceb513	refs/heads/main
   $ git log -1 --format=%B e8debf3
   Merge chore/harnais-v1.6.0 [R001]
   ```
   `git worktree remove` des deux worktrees (F01-M0002 et F01-R001) ; `git worktree list` = la racine seule.
7. État final :
   ```
   $ git status --short -uall
    M scripts/pre-push-gate.sh
    M vault/30-discoveries.md
   ?? vault/echanges/F01.md
   ?? vault/echanges/archive/2026-10-17-F01-M0001-commit-pose-harnais-v1.6.0.md
   ?? vault/reprise/F01.md
   ```
   Il ne reste que les intouchables. Les fichiers du harnais sont présents à la racine : `config/harnais.json`, `.claude/skills/orchestration-bureau/SKILL.md`, `scripts/harnais-hooks/pre-push`, `vault/runtime/README.md`.

### Hors périmètre (pour l'orchestrateur, pas d'issue ouverte : MCP GitHub non connecté)

- `pnpm lint` ne tourne pas dans un worktree neuf (pas de `node_modules`) : tout « rejoue le lint sur le scratch » futur échouera brut. Le PATH vers `node_modules/.bin` de la racine suffit, et pourrait aller dans `pieges_worktree[]` de `config/harnais.json`.
- Le hook graphify `post-checkout` sort toujours en code 1 à chaque checkout et à chaque `worktree add`.

### Tests

Aucun code touché : sans objet (lint vert racine et scratch).
Scellé tests : sans objet
Contre-témoins : aucun

### Rituel vault

`vault/reprise/F01.md` réécrit pour M0002. Les registres `vault/runtime/events.jsonl` (`MANDAT_TERMINE`, `src: fenetre`) et `state.json` (`windows.F01` → `TERMINE`) sont écrits mais pas committés, car gitignorés (`git check-ignore` confirme).
Commit `aebe9fc` `docs(vault): reprise F01 M0002` (`vault/reprise/F01.md` + archive M0001, sans `vault/echanges/F01.md`) poussé sur `main` :
```
   8ed5de0..aebe9fc  main -> main
$ git ls-remote origin refs/heads/main
aebe9fc3134b00d452ee7199c22e967e9daf35fe	refs/heads/main
```
`git status --short -uall` final : ` M scripts/pre-push-gate.sh`, ` M vault/30-discoveries.md`, `?? vault/echanges/F01.md`.

M0002
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ✅
Merge main          ✅
Main push           ✅
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED
