---
date: 2026-10-17
tags: [qmd, claude-md, doctrine]
session: F02
mandat: qmd-imperatif-en-tete
mandat_id: M0005
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F02.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F02-M0005
branche: docs/qmd-imperatif
niveau: N2
derniere_maj: 2026-10-17T15:49:28+0200
---

# F02 / M0005 — QMD impératif, remis en tête des instructions

> **IMPÉRATIF (fondateur, 2026-10-17) : QMD d'abord pour tout `.md`.** Chercher et lire avec `mcp__qmd__query` / `mcp__qmd__get` (https://github.com/tobi/qmd, serveur `qmd` de `.mcp.json`) avant tout `Read`. `Read` sur un `.md` seulement à ligne connue (offset + limit). GitHub = `gh` CLI, jamais le connecteur MCP GitHub.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Contexte (autonome)

Décision du fondateur, 2026-10-17 15:48, verbatim : « QMD : remets-le en haut car c'est un moyen rapide et économe de faire ce genre de tâche dans un projet si gros que le nôtre. C'est un impératif. »
Aujourd'hui la règle est enfouie : `.claude/CLAUDE.md`, section §15 « Token Management », ligne ~88 : `**QMD-first** : pour tout .md, utiliser mcp__qmd__get/mcp__qmd__query avant Read. Read sur .md uniquement si ligne exacte connue (offset+limit obligatoire).`
Le `CLAUDE.md` racine (32 Ko) commence par « # ToM Protocol - Claude/LLM Documentation » sans la mentionner.
Une autre fenêtre (F01/M0004, branche `chore/retrait-graphify`) tourne en parallèle sur des fichiers disjoints (`.husky/`, `graphify-out/`, `.gitignore`, `biome.json`, `docs/REGEN.md`). Tu ne touches à aucun de ces fichiers ; elle ne touche pas aux tiens.

## Intouchables

`scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `.claude/settings.json`, `vault/echanges/F01.md`, `vault/echanges/F02.md` (jamais committés), tout fichier de F01/M0004 listé ci-dessus.

## Gardes de précondition

1. `git fetch origin` ; `git merge-base --is-ancestor dedaa553642585b73f5831e1e9e2a5cb758729c8 origin/main` (code 0), sinon STOP. Note le SHA d'`origin/main`.
2. Tu travailles dans un **worktree** (la racine peut être occupée par F01) : `git worktree add -b docs/qmd-imperatif .claude/worktrees/F02-M0005 origin/main`, puis tout le travail dans ce dossier. Jamais de `checkout` à la racine avant l'étape merge.

## Mission

1. Rituel de début : `statut: en-cours` dans ce frontmatter (à la racine, `vault/echanges/F02.md`).
2. **QMD (lecture seule)** : vérifie que `mcp__qmd__query` répond ; dis au rapport **quels dossiers l'index `tom` couvre** (`qmd status` / `qmd collection list` ou ce que `qmd --help` indique). Le commentaire de `.mcp.json` dit `vault/` seul : si `docs/` (et les autres `.md` du dépôt) n'y sont pas, **ne change pas l'index** : chiffre ce qui manque (nombre de `.md` hors `vault/`, taille) et propose la commande exacte pour l'élargir, pour décision du fondateur.
3. **Remonter la règle** (dans le worktree) :
   - En tête du `CLAUDE.md` racine, juste sous le titre H1, un bloc court (≤ 4 lignes, ≤ 400 caractères par ligne) marqué **IMPÉRATIF**, reprenant la règle QMD-first (outils `mcp__qmd__query` / `mcp__qmd__get`, `Read` sur `.md` seulement à ligne connue avec offset + limit) et la raison en 6 mots au plus (rapide, économe en contexte).
   - Dans `.claude/CLAUDE.md` §15, **remplace** la ligne `**QMD-first** …` par un renvoi d'une ligne vers ce bloc (« QMD-first : voir l'impératif en tête du CLAUDE.md racine »). Remplacer, jamais ajouter : le total des deux fichiers ne doit pas grossir de plus de ~300 caractères. Aucune date, aucun `Mxxxx` dans ces fichiers.
   - Contrôle de budget : `sh "$(git config core.hooksPath)/pre-commit" --controle HEAD` n'existe que pour les hooks du harnais ; si inapplicable (hooks husky), mesure toi-même `wc -c` et la ligne la plus longue modifiée, colle-les. Budget : 40 000 caractères par fichier, 400 par ligne.
4. Commit pathspec (`CLAUDE.md`, `.claude/CLAUDE.md`) : `docs(claude): QMD imperatif en tete des instructions`, sans trailer. Lint du worktree : `PATH=/Users/malik/Documents/tom-protocol/node_modules/.bin:$PATH pnpm lint` (issue #58) ; push branche + `ls-remote` collé.
5. **Doublage N2** : sous-agent FRAIS (jamais `fork`), worktree détaché lecture seule `.claude/worktrees/F02-R003` sur le tip : diff limité aux deux fichiers ; règle QMD présente en tête, une seule fois, fidèle à l'ancienne ; ancienne ligne remplacée (pas dupliquée) ; aucun état (date, Mxxxx) ajouté ; budget respecté. Rapport `vault/revues/2026-10-17-R003-qmd-imperatif.md`, frontmatter avec exactement `verdict: GO` (ou `RESERVE`), `tip: <sha 40>`, `niveau: N2`. RESERVE → STOP sans merge.
   Règles des sous-agents (verbatim) : (1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre.
6. **Merge** (gabarits §9) : scratch `.claude/worktrees/F02-merge` détaché sur `origin/main` à jour (`git fetch` d'abord), `merge --no-ff <tip> -m "Merge docs/qmd-imperatif [R003]"`, copie + commit de R003. Puis à la racine : **seulement si `git branch --show-current` = `main` et que F01 n'est pas en train de merger** (`vault/echanges/F01.md` `statut` = `reponse-disponible`, ou `git status --short` à la racine ne montre que les intouchables) : `git merge --ff-only <HEAD du scratch>`, `git push origin main`, `ls-remote` collé. Racine occupée (autre branche checkoutée) → n'y touche pas : pousse directement `git push origin <HEAD du scratch>:refs/heads/main` depuis le scratch (fast-forward seulement, refus = rapport), `ls-remote` collé, et note que la racine devra faire `git merge --ff-only origin/main` plus tard. Si `origin/main` a bougé (F01 a mergé), refais le scratch UNE fois. Puis `git worktree remove` des trois worktrees.

## Conditions STOP

Garde en échec ; fichier inattendu dans le diff ; lint rouge ; conflit ; fast-forward refusé deux fois ; doublage RESERVE ; tout imprévu git (alors NE RIEN pousser).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée ; jamais `--no-verify`, jamais de force-push.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0005`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F02.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F02"`, `mandat: "M0005"`) et `vault/runtime/state.json` (`windows.F02` seulement, relu juste avant d'écrire : F01 l'écrit aussi) : jamais committés.
4. Commit vault `vault/reprise/F02.md` seul, sur `main` (à la racine si elle est sur `main`, sinon depuis un scratch détaché sur `origin/main`) : `docs(vault): reprise F02 M0005`, push, `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.

## Rapport M0005


Résultat : règle QMD-first remontée en tête du `CLAUDE.md` racine, ligne §15 de `.claude/CLAUDE.md` remplacée par un renvoi ; R003 GO ; intégré sur `main` (`43ca9aa8cc43e1369f31b1bf3ca1c549310f7b01`).

**Gardes** : `merge-base --is-ancestor dedaa55… origin/main` = 0 ; `origin/main` au départ = `127255ec37d3c5a7bea673c9aab1052bd8450174`. Worktree `.claude/worktrees/F02-M0005` sur `origin/main`.

**QMD (lecture seule, index non modifié)** :
- `mcp__qmd__query` répond (contrôle positif « rendez-vous tournant » → `vault/40-roadmap.md:54`, `docs/plans/prompt-reprise-roles.md:17`). La requête `QMD-first` ne renvoie rien : les `CLAUDE.md` ne sont pas indexés.
- Index `tom` (`~/.cache/qmd/tom.sqlite`, 16,1 Mo) couvre 3 collections : `vault` → `vault/` (6 fichiers indexés / 17 suivis), `docs` → `docs/` (157 / 174), `bmad` → `_bmad-output/` (24 / 24). Index périmé : mis à jour il y a 88-116 j, 187 documents en attente d'embedding. Le `_comment` de `.mcp.json` (« vault/ du repo UNIQUEMENT ») est donc faux : `docs/` et `_bmad-output/` y sont.
- Manque : 328 `.md` suivis hors `vault/`, `docs/`, `_bmad-output/` (~2,1 Mo de texte), dont les `.md` racine (`CLAUDE.md`, `AGENTS.md`, `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`), `_bmad/` (179), `.claude/` (100 : commands, skills, stacks…), `.github/skills/` (21), `tools/`, `experiments/`.
- Proposition pour décision du fondateur : (1) rafraîchir l'existant : `qmd --index tom update && qmd --index tom embed` ; (2) élargir : `qmd --index tom collection add /Users/malik/Documents/tom-protocol --name racine --mask '*.md'` puis `update` + `embed`. Syntaxe exacte de `collection add` (options `--name`/`--mask`) **non vérifiée** : `qmd --help` ne détaille pas ses options ; à confirmer avant de lancer. Corriger aussi le `_comment` de `.mcp.json`.

**Changement** (`4d269b9e3911a4d5be3305a030850dd93f82d0ac`, branche `docs/qmd-imperatif`) :
- `CLAUDE.md:3-4` : bloc `> **IMPÉRATIF — QMD d'abord pour tout .md**…` + `> Raison : rapide, économe en contexte.`
- `.claude/CLAUDE.md:88` : `**QMD-first** : voir l'impératif en tête du CLAUDE.md racine.` (remplace l'ancienne ligne).
- Budget (hooks husky, contrôle du harnais inapplicable, mesure manuelle) : `wc -c` 32 302 (`CLAUDE.md`) + 12 727 (`.claude/CLAUDE.md`) = 45 029, contre 44 854 avant → +175. Lignes ajoutées : 235, 40, 65 caractères (≤ 400). Aucune date ni `Mxxxx`.
- Lint : `pnpm lint` → `Checked 146 files … No fixes applied.` (rc 0).
- Branche poussée : `4d269b9e3911a4d5be3305a030850dd93f82d0ac	refs/heads/docs/qmd-imperatif`.

**Doublage N2** : sous-agent frais, worktree détaché `F02-R003` sur le tip → `vault/revues/2026-10-17-R003-qmd-imperatif.md`, `verdict: GO`. Revérifié : frontmatter, `CLAUDE.md:3-4`, `.claude/CLAUDE.md:88`. Remarques non bloquantes : le bloc ne mentionne pas `--index tom` ; formulation légèrement différente de l'ancienne ligne, même sens.

**Merge** : `origin/main` avait bougé (`5bee237` reprise F01 M0004) → scratch `F02-merge` sur `5bee237`, `merge --no-ff` (`d8ec874`), commit R003 (`43ca9aa`). Racine sur `main`, F01 `statut: reponse-disponible`, `git status --short` = intouchables seuls → `merge --ff-only 43ca9aa`, `git push origin main` (`5bee237..43ca9aa`).
`43ca9aa8cc43e1369f31b1bf3ca1c549310f7b01	refs/heads/main`
Worktrees `F02-M0005`, `F02-R003`, `F02-merge` retirés.

**Vault** : reprise committée et poussée (`docs(vault): reprise F02 M0005`) : `a9e0984b6addba78408bf961e2f932af0551e872	refs/heads/main`. `vault/runtime/` mis à jour, non committé (ignoré).

Scellé tests : sans objet (mandat sans test)
Contre-témoins : aucun

M0005
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ✅
Merge main          ✅
Main push           ✅
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED
