---
date: 2026-10-17
tags: [harnais, pose, mission-0]
session: F01
mandat: commit-pose-harnais-v1.6.0
mandat_id: M0001
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F01-M0001
branche: chore/harnais-v1.6.0
niveau: N2
derniere_maj: 2026-10-17T15:13:15+0200
---

# F01 / M0001 — Enregistrer la pose du harnais v1.6.0 (premier mandat du projet)

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Contexte (autonome)

Le harnais v1.6.0 a été posé sur ce dépôt le 2026-10-17 vers 15:07 par `harnais equiper`. Le moteur ne committe rien : ses fichiers sont non suivis ou modifiés à la racine. Ce mandat les enregistre, sur une branche, avec relecture N2 indépendante, puis merge sur `main` (décision du fondateur #41 : sur `main`, commits directs réservés à `vault/` ; tout le reste passe par branche + doublage + merge).

Tu travailles **à la racine** `/Users/malik/Documents/tom-protocol` : les fichiers à committer n'existent que là. Le worktree du frontmatter sert au scratch de merge.

Convention : lis `.claude/skills/orchestration-bureau/SKILL.md` et `references/gabarits.md` (§2, §3, §4, §9) avant d'agir. `config/harnais.json` : `git.trailer` vaut le gabarit non rempli `Co-Authored-By: <signature du projet>` et les commits récents de `main` n'ont aucun trailer → **aucun trailer** dans tes messages ; dis-le au rapport.

## Fichiers de l'utilisateur — INTOUCHABLES

Modifiés à la racine, hors harnais, à laisser tels quels (jamais add, jamais checkout, jamais stash) :
- `scripts/pre-push-gate.sh`
- `vault/30-discoveries.md`
- `.claude/settings.local.json` (non suivi, réglage local : ne pas committer)
- `vault/echanges/F01.md` (ce mandat : non suivi, propriété de la fenêtre, jamais committé)

Tout autre fichier sale ou non suivi que ceux listés ici et dans la liste à committer → STOP.

## Gardes de précondition

1. `git fetch origin` puis `git rev-parse HEAD origin/main` : les deux = `fdae163f3754c9f49aa109f4e0b3a494a447b397`. Sinon STOP.
2. `git branch --show-current` = `main`. Sinon STOP.
3. `git status --short` ne liste QUE les chemins de ce mandat (liste à committer + intouchables). Sinon STOP.
4. `git diff -- CLAUDE.md` = exactement une ligne ajoutée en fin : `@~/.harnais/canon/templates/tronc/harnais.md`. `git diff -- .gitignore` = uniquement le bloc ajouté en fin commençant par `# harnais:registres`. Toute autre différence dans ces deux fichiers → STOP (contenu de l'utilisateur mêlé).

## Mission

1. Rituel de début : `statut: en-cours` dans ce frontmatter.
2. `git checkout -b chore/harnais-v1.6.0` puis `git branch --show-current` = `chore/harnais-v1.6.0`, sinon STOP.
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
   `git add -- <ces chemins>` puis commit, message par heredoc : `chore(harnais): pose du harnais v1.6.0`. Jamais `-a`/`-A`. Vérifie `git show --stat HEAD` : exactement ces fichiers.
4. `git push -u origin chore/harnais-v1.6.0` + `git ls-remote origin refs/heads/chore/harnais-v1.6.0` collé. Un hook (husky) qui refuse → STOP, sortie collée, jamais `--no-verify`.
5. **Doublage N2** : un sous-agent FRAIS (jamais `fork`), worktree détaché lecture seule sur le tip (`git worktree add --detach .claude/worktrees/F01-R001 <tip>`). Il vérifie : le commit ne contient que des fichiers du harnais ; aucun fichier de l'utilisateur ; `CLAUDE.md` = une seule ligne ajoutée ; `.gitignore` = seulement le bloc harnais ; aucun secret (`.env` absent du commit) ; `config/harnais.json` est du JSON valide (`node -e` ou `python3 -m json.tool`). Verdict par axe ✅/⚠️/⛔ avec la raison vérifiée. Tu écris son rapport dans `vault/revues/2026-10-17-R001-pose-harnais.md`, frontmatter fermé par `---` avec exactement une ligne `verdict: GO` (ou `RESERVE`), une ligne `tip: <sha 40 du tip>`, une ligne `niveau: N2`. RESERVE → STOP, pas de merge.
   Règles des sous-agents (verbatim) :
   ```
   (1) lecture/recherche parallele = agents FRAIS (jamais fork) ;
   (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent -- ces actes restent a la session mere ;
   (3) tout rapport de sous-agent est VERIFIE (citations fichier:ligne recontrolees) avant d'entrer dans un livrable ;
   (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ;
   (5) ATTENDRE LE RETOUR DE TOUS les sous-agents lances AVANT le premier rituel de fin. Attendre = rendre la main jusqu'a la notification, jamais sleep ni sondage. Echec ou arret de l'un = STOP propre, dit au rapport.
   ```
6. **Merge** (gabarits §9) : scratch `.claude/worktrees/F01-M0001` détaché sur `origin/main`, `merge --no-ff <tip> -m "Merge chore/harnais-v1.6.0 [R001]"`, copie et commit du rapport R001 dans le scratch (`docs(revues): R001 -- GO`). Tests : aucun code touché, sans objet — dis-le. Puis à la racine : `git checkout main` (les fichiers du harnais disparaissent un instant de l'arbre : normal) puis `git merge --ff-only $(git -C .claude/worktrees/F01-M0001 rev-parse HEAD)`, `git push origin main`, `git ls-remote origin refs/heads/main` collé. Refus `ff-only` → rapport, jamais de force. Ensuite `git worktree remove` du scratch et du worktree R001.
7. Vérifie à la fin : `git status --short` ne liste plus que les fichiers intouchables ci-dessus ; les fichiers du harnais sont présents à la racine.

## Conditions STOP

Garde en échec ; fichier inattendu ; hook qui refuse ; conflit au merge ; `ff-only` refusé deux fois ; doublage RESERVE ; tout imprévu sur l'état git (alors NE RIEN pousser et le dire).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous la forme `"${VAR:?}/${SOUS:?}"` ; jamais `--no-verify`.

## Preuves exigées

Sorties collées : gardes 1 à 4, `git show --stat` du commit, `ls-remote` de la branche et de `main`, `git log -1 --format=%B` du merge, `git status --short` final.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0001`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` : 5 lignes au plus, ce qui est fait et le SHA de `main`.
3. `vault/runtime/events.jsonl` (ajout, `src: "fenetre"`, `f: "F01"`, `mandat: "M0001"`) et `vault/runtime/state.json` (entrée `windows.F01`) : écrits, **jamais committés**.
4. Commit du vault restant (`vault/reprise/F01.md` seulement, jamais `vault/echanges/F01.md`) directement sur `main` : `docs(vault): reprise F01 M0001`, push, `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible` dans ce frontmatter.

## Rapport M0001


**Verdict : STOP propre à l'étape 3 (commit).** Le hook husky `pre-commit` du projet (`pnpm lint` = `biome check .`) refuse le commit. Rien n'est commité, rien n'est poussé, l'état git est revenu exactement à celui de départ.

### Gardes (toutes vertes)

- G1 : `git rev-parse HEAD origin/main` → `fdae163f3754c9f49aa109f4e0b3a494a447b397` ×2 ✅
- G2 : `git branch --show-current` → `main` ✅
- G3 : `git status --short --untracked-files=all` → seulement les 23 chemins à committer + `scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `vault/echanges/F01.md` ✅ (`.claude/settings.local.json` n'apparaît pas : ignoré par `~/.config/git/ignore:1`)
- G4 : `CLAUDE.md` = une seule ligne ajoutée `@~/.harnais/canon/templates/tronc/harnais.md` ; `.gitignore` = seulement le bloc final `# harnais:registres …` ✅

### Déroulé

1. `statut: en-cours` posé.
2. `git checkout -b chore/harnais-v1.6.0` → branche créée, mais `husky - post-checkout script failed (code 1)`. Ce hook est le hook **graphify** de l'utilisateur (`.husky/post-checkout`, reconstruction du graphe de connaissances) ; un post-checkout ne peut pas annuler le checkout (vérifié : branche courante = `chore/harnais-v1.6.0`, reflog `checkout: moving from main to chore/harnais-v1.6.0`). Non bloquant, signalé.
3. `git add -- <23 chemins>` puis `git commit -F -` (message `chore(harnais): pose du harnais v1.6.0`, **sans trailer** : `git.trailer` est le gabarit non rempli et les commits de `main` n'en ont pas) → **refusé** par `.husky/pre-commit` :
   ```
   ./vault/runtime/state.json format ━━━
   ./config/harnais.json format ━━━
     × Formatter would have printed the following content:
       34 │ - ····"efforts_autorises":·[ "low", "medium", "high", "xhigh", "max" ] (sur 7 lignes)
       34 │ + ····"efforts_autorises":·["low",·"medium",·"high",·"xhigh",·"max"],
   Found 2 errors.
    ELIFECYCLE  Command failed with exit code 1.
   husky - pre-commit script failed (code 1)
   ```
   `git show --stat HEAD` → toujours `fdae163` : aucun commit créé.
4. Remise en état (aucune commande interdite) : `git restore --staged -- <chemins indexés>` (arbre de travail intact), `git checkout main`, `git branch -d chore/harnais-v1.6.0` (pointait sur `fdae163`, aucun commit propre, jamais poussée : `git ls-remote origin refs/heads/chore/harnais-v1.6.0` → vide). État final : `main` = `origin/main` = `fdae163…`, `git status --short` identique à l'état initial.
5. Étapes 4 à 7 (push, doublage R001, merge) **non exécutées** : aucun sous-agent lancé, R001 non écrit, aucun worktree créé.

### Cause

Deux causes, toutes deux hors fichiers de l'utilisateur :
- `config/harnais.json`, posé par `harnais equiper`, n'est pas au format biome du projet (tableau court sur plusieurs lignes, `lineWidth: 120`).
- `biome check .` scanne tout l'arbre, y compris `vault/runtime/state.json` (gitignoré, écrit en continu par l'orchestrateur et le superviseur). Le pré-commit échouera donc sur **tout** commit du projet, y compris les commits `vault/` seuls, tant que ce fichier est dans le périmètre de biome. Reformater `harnais.json` ne suffit pas.

### Question à l'orchestrateur (arbitrage)

Comment faire passer le pré-commit biome ?
- **A (mon choix)** : ajouter `config/harnais.json` et `vault/runtime` à `files.ignore` de `biome.json` (ou activer `vcs.useIgnoreFile: true` pour que biome respecte `.gitignore`, puis exclure `config/harnais.json`). Ça règle les deux causes à la racine et durablement : les registres sont réécrits sans arrêt par le harnais, on ne peut pas les garder au format biome. Mais `biome.json` est un fichier de l'utilisateur, hors périmètre de ce mandat : il faut l'accord du fondateur ou un mandat qui l'inclut explicitement.
- B : reformater `config/harnais.json` avec `biome format --write`. Ne suffit pas seul (state.json échoue quand même), et le moteur `harnais equiper` réécrirait le fichier dans son format à la prochaine pose ou mise à jour.
- C : `HUSKY=0` / `--no-verify`. Interdit, écarté.

### Hors périmètre (à ouvrir en issue par l'orchestrateur)

- Le moteur `harnais equiper` devrait détecter un formateur JSON (biome/prettier) dans le projet, ou au moins le signaler, puisque ses fichiers JSON et ses registres entrent dans son périmètre.
- Le hook graphify `post-checkout` sort en code 1 sur un simple changement de branche (bruit sur chaque checkout du harnais).
- Pas de `gh issue` ouverte depuis cette fenêtre : le MCP GitHub n'a pas réussi à se connecter (`Authorization header is badly formatted`). Constats laissés ici pour l'orchestrateur.

### Tests

Aucun code touché : sans objet.
Scellé tests : sans objet
Contre-témoins : aucun

### Rituel vault

`vault/reprise/F01.md` écrit. Son commit sur `main` (`docs(vault): reprise F01 M0001`) est soumis au même hook biome : résultat ci-dessous.

Résultat : `git add -- vault/reprise/F01.md && git commit -m "docs(vault): reprise F01 M0001" -- vault/reprise/F01.md` → **refusé** par le même hook (`./config/harnais.json format … Found 1 error. husky - pre-commit script failed (code 1)`). HEAD toujours `fdae163…`, fichier retiré de l'index (`git restore --staged`), donc rien poussé, aucun `ls-remote` à coller. `vault/reprise/F01.md` reste non suivi sur disque.
Précision sur la cause : cette fois `vault/runtime/state.json` est passé, parce que je l'ai réécrit avec `json.dump(indent=2)`, ce qui tombe au format biome. Son échec dépend donc du format de **qui écrit en dernier** (orchestrateur ou superviseur). La cause reste réelle et intermittente, l'option A tient toujours.
Registres : une ligne `INCIDENT` (`src: fenetre`) ajoutée à `events.jsonl` ; `state.json.windows.F01` → `state: BLOQUE`. Non committés (gitignorés).
`git status --short` final : identique à l'état de départ, plus `vault/reprise/F01.md` non suivi (déjà couvert par `?? vault/reprise/`).

M0001
Implementation      ❌
Tests               ❌
Commit              ❌
Branch push         ❌
Review              ❌
Merge main          ❌
Main push           ❌
TASK STATUS         BLOCKED
PROJECT STATUS      NOT_INTEGRATED
