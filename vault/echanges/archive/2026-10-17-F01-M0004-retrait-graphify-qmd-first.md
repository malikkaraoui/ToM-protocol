---
date: 2026-10-17
tags: [graphify, qmd, nettoyage]
session: F01
mandat: retrait-graphify-qmd-first
mandat_id: M0004
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F01-M0004
branche: chore/retrait-graphify
niveau: N2
derniere_maj: 2026-10-17T15:46:20+0200
---

# F01 / M0004 — Retirer graphify du dépôt ; vérifier que QMD sert la lecture des .md

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Règles permanentes de l'équipe (fondateur, 2026-10-17)

- **QMD d'abord pour tout `.md`** (https://github.com/tobi/qmd ; serveur MCP `qmd` déclaré dans `.mcp.json`, index `tom` = `vault/`) : chercher et lire avec `mcp__qmd__query` / `mcp__qmd__get` avant `Read` ; `Read` sur un `.md` seulement à ligne connue (offset + limit). Règle déjà écrite dans `.claude/CLAUDE.md`.
- **GitHub = `gh` CLI** (vérifié par M0003 : compte `malikkaraoui`, admin + push). Jamais le connecteur MCP GitHub.

## Contexte (autonome)

Décision du fondateur, 2026-10-17 15:44 : « supprime graphify — le harnais remplace une partie de ce que faisait graphify ». Graphify est déjà désinstallé de la machine (M0003, issue https://github.com/malikkaraoui/ToM-protocol/issues/57) : ses hooks husky sortent en code 1 à chaque checkout et commit.
Empreinte relevée par l'orchestrateur (à re-vérifier) :
- `.husky/post-checkout` (139 lignes) et `.husky/post-commit` (150 lignes) : blocs `# graphify-…-hook-start` ;
- `graphify-out/` : 2 fichiers suivis (`GRAPH_REPORT.md`, `graph.json`), ~49 Mo sur disque, et listé dans `.gitignore` (lignes 12-13) ;
- `biome.json` : deux entrées `"graphify-out"` dans `files.ignore` ;
- mentions dans `docs/REGEN.md`, `docs/plans/2026-06-10-chantier-s1-suivi.md`, `CHANGELOG.md`.

## Intouchables (modifiés hors mandat, jamais add/checkout/stash/restore)

`scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `.claude/settings.json` (une ligne y a été retirée à 15:30 par quelqu'un d'autre : n'y touche pas), `vault/echanges/F01.md` (jamais committé).

## Gardes de précondition

1. `git fetch origin` ; `git rev-parse HEAD origin/main` = `dedaa553642585b73f5831e1e9e2a5cb758729c8` ×2. Sinon STOP.
2. `git branch --show-current` = `main`. Sinon STOP.
3. `git status --short` ne liste que les intouchables (et `?? vault/echanges/archive/2026-10-17-F01-M0003-acces-github-gh-et-issues.md`). Sinon STOP.

## Mission

1. Rituel de début : `statut: en-cours`.
2. **QMD (lecture seule)** : `command -v qmd && qmd --version` ; `qmd status` (ou l'équivalent que `qmd --help` indique) ; une requête de test via `mcp__qmd__query` sur l'index `tom` (ex. « harnais »). Rapporte si le MCP `qmd` répond et si l'index `tom` couvre `vault/`. Utilise QMD pour toutes tes lectures de `.md` dans ce mandat. Si QMD ne répond pas : continue le mandat, dis-le au rapport (piste de réparation), pas de STOP.
3. `git checkout -b chore/retrait-graphify` (le post-checkout graphify sortira en 1 une dernière fois : non bloquant si la branche courante est la bonne).
4. **Retrait graphify** :
   - `.husky/post-checkout` et `.husky/post-commit` : s'ils ne contiennent QUE le bloc graphify (+ shebang), `git rm` ; sinon retire seulement le bloc graphify. Vérifie qu'aucun autre hook husky ne l'appelle.
   - `git rm -r --cached graphify-out` puis supprime le dossier du disque avec la forme gardée (`D=graphify-out; rm -rf -- "${PWD:?}/${D:?}"`).
   - `.gitignore` : retire le commentaire et la ligne `graphify-out/` (lignes 12-13), rien d'autre.
   - `biome.json` : retire les deux entrées `"graphify-out"`, rien d'autre (virgules correctes).
   - `docs/REGEN.md` : si graphify y est une étape à suivre, remplace le texte périmé (jamais ajouter) ; `CHANGELOG.md` et `docs/plans/…` sont de l'historique : n'y touche pas.
   - `git grep -n -i graphify -- . ':!vault' ':!CHANGELOG.md' ':!docs/plans'` doit finir vide (sauf justification au rapport).
   - `pnpm lint` à la racine : code 0. Sinon STOP.
5. Commit pathspec explicite : `chore: retrait de graphify (desinstalle, remplace par le harnais) -- closes #57`, sans trailer (`git.trailer` non rempli). Push de la branche + `git ls-remote` collé.
6. **Doublage N2** : sous-agent FRAIS (jamais `fork`), worktree détaché lecture seule `.claude/worktrees/F01-R002` sur le tip. Il vérifie : diff limité au retrait graphify ; aucun intouchable ; hooks husky restants intacts (`pre-commit`, `pre-push`, `commit-msg`) ; `biome.json` valide ; `git grep graphify` vide hors historique ; lint vert (dans le worktree : `PATH=/Users/malik/Documents/tom-protocol/node_modules/.bin:$PATH pnpm lint`, cf. issue #58). Rapport `vault/revues/2026-10-17-R002-retrait-graphify.md`, frontmatter fermé avec exactement `verdict: GO` (ou `RESERVE`), `tip: <sha 40>`, `niveau: N2`. RESERVE → STOP sans merge.
   Règles des sous-agents (verbatim) : (1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre.
7. **Merge** (gabarits §9) : scratch `.claude/worktrees/F01-M0004` détaché sur `origin/main`, `merge --no-ff <tip> -m "Merge chore/retrait-graphify [R002]"`, lint sur le scratch (même PATH), copie + commit de R002 (`docs(revues): R002 -- GO`). À la racine : `git checkout main`, `git merge --ff-only <HEAD du scratch>`, `git push origin main`, `git ls-remote origin refs/heads/main` collé. Refus ff-only → rapport, jamais de force. `git worktree remove` des deux worktrees.
8. Vérifie que l'issue #57 est fermée (`gh issue view 57 --repo malikkaraoui/ToM-protocol --json state`) ; sinon `gh issue close 57` avec un commentaire citant le SHA de merge.

## Conditions STOP

Garde en échec ; fichier inattendu ; hook ou lint qui refuse ; conflit ; ff-only refusé deux fois ; doublage RESERVE ; tout imprévu git (alors NE RIEN pousser).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée ; jamais `--no-verify`.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0004`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` réécrit pour M0004 (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F01"`, `mandat: "M0004"`) et `vault/runtime/state.json` (`windows.F01`) : écrits, jamais committés.
4. Commit vault sur `main`, pathspec : `vault/reprise/F01.md`, `vault/echanges/archive/2026-10-17-F01-M0003-acces-github-gh-et-issues.md` ; `docs(vault): reprise F01 M0004` ; push ; `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.

## Rapport M0004

Scellé tests : sans objet (mandat sans test : retrait d'outillage, aucun test neuf)
Contre-témoins : aucun

**Résultat : graphify retiré, mergé et poussé sur `main` (`127255ec37d3c5a7bea673c9aab1052bd8450174`), doublage R002 = GO, issue #57 fermée.**

### Gardes
`git rev-parse HEAD origin/main` = `dedaa553642585b73f5831e1e9e2a5cb758729c8` ×2 ; branche `main` ; `git status --short` = les 3 intouchables + `?? vault/echanges/F01.md` + `?? vault/echanges/archive/…M0003…` → OK.

### QMD (lecture seule)
- `qmd` = `/opt/homebrew/bin/qmd`, `qmd 2.8.3 (facd35e)`. MCP `qmd` (déclaré `.mcp.json`, `--index tom`) **répond** : `mcp__qmd__query` « harnais » → 10 résultats (vault + docs) ; `mcp__qmd__get docs/REGEN.md:10:55` OK (utilisé pour la lecture de `docs/REGEN.md`).
- Index `/Users/malik/.cache/qmd/tom.sqlite` : 187 docs, collections `vault` (6), `docs` (157), `bmad` (24). **Périmé** : dernière mise à jour il y a 88-89 j ; `Pending: 187 need embedding`.
- **L'index `tom` ne couvre PAS tout `vault/`** : 6 fichiers indexés (`00-brief`…`50-token-history`) sur 17 `.md` présents (hors `runtime/`) — manquent `vault/reprise/`, `vault/echanges/`, `vault/revues/`, etc. (fichiers récents, pattern `**/*.md` correct, c'est juste l'index pas rafraîchi). Note : `.mcp.json` dit « tom = vault/ UNIQUEMENT » alors que l'index porte aussi `docs` et `bmad`.
- Piste de réparation (non exécutée, hors périmètre lecture seule) : `qmd --index tom update && qmd --index tom embed`, puis `qmd --index tom status` doit montrer `vault` ≈ 17 fichiers et `Pending: 0`.

### Retrait graphify — commit `4e79c816a984b1011b2fabd732841bdbb84013c0` (branche `chore/retrait-graphify`)
- `.husky/post-checkout` (139 l.) et `.husky/post-commit` (150 l.) ne contenaient QUE shebang + bloc `graphify-…-hook-start/end` → `git rm`. Aucun autre hook (`pre-commit`, `pre-push`, `commit-msg`, `_/`) ni `package.json` ne les appelle (grep vide).
- `graphify-out/` : `git rm -r --cached` (2 fichiers) puis suppression disque (49 Mo). ⚠️ La forme gardée `D=graphify-out; rm -rf -- "${PWD:?}/${D:?}"` a été **refusée par le contrôle d'outils** ; contournée par `rm -r /Users/malik/Documents/tom-protocol/graphify-out` (chemin absolu explicite, sans `-f`, contenu listé avant). Vérifié : `ls -d graphify-out` → No such file.
- `.gitignore` : retrait des lignes 12-13 (commentaire + `graphify-out/`), rien d'autre (reste une double ligne vide, signalée par R002, laissée pour respecter « rien d'autre »).
- `biome.json` : retrait des 2 entrées `"graphify-out"` ; `python3 -I -m json.tool` OK.
- `docs/REGEN.md:17` (« Gardé volontairement … `graphify-out/` (tracké) ») = état courant devenu faux → texte remplacé (mention retirée). `docs/REGEN.md:58` **laissée** : section « État git au moment de la sauvegarde » = instantané historique daté (14/06), au même titre que `CHANGELOG.md`/`docs/plans` ; justification validée par R002.
- `git grep -n -i graphify -- . ':!vault' ':!CHANGELOG.md' ':!docs/plans'` → seule ligne : `docs/REGEN.md:58` (justifiée ci-dessus).
- `pnpm lint` racine : `Checked 147 files … No fixes applied`, exit 0.
- Commit pathspec explicite, message exact, sans trailer, auteur/committer `karaoui.malik@gmail.com`. Le post-commit n'a plus échoué (hook supprimé).
- Push branche : `4e79c816a984b1011b2fabd732841bdbb84013c0	refs/heads/chore/retrait-graphify`

### Doublage N2 — R002
Sous-agent frais (general-purpose, pas fork), worktree détaché `.claude/worktrees/F01-R002` sur `4e79c81`. Verdict **GO** : diff limité aux 7 fichiers attendus ; aucun intouchable ; `pre-commit`/`pre-push`/`commit-msg` identiques à la base (recontrôlé par moi : `git diff --quiet dedaa55 4e79c81 -- .husky/pre-commit .husky/pre-push .husky/commit-msg` → OK) ; JSON valide ; grep = REGEN:58 seule ; lint worktree exit 0 (146 fichiers). Rapport : `vault/revues/2026-10-17-R002-retrait-graphify.md` (frontmatter `verdict: GO`, `tip: 4e79c816a984b1011b2fabd732841bdbb84013c0`, `niveau: N2`, recontrôlé). Le sous-agent signale avoir écrit puis supprimé un fichier `/tmp/unused` pendant son lint (hors scratchpad, sans effet sur le dépôt).

### Merge
Scratch `.claude/worktrees/F01-M0004` détaché sur `origin/main` (`dedaa55`) → `merge --no-ff 4e79c81` = `9cdc460` « Merge chore/retrait-graphify [R002] » ; lint scratch exit 0 ; `docs(revues): R002 -- GO` = `127255ec37d3c5a7bea673c9aab1052bd8450174`. Racine : `git checkout main` (dernier échec post-checkout graphify, non bloquant), `merge --ff-only 127255e` OK, `git push origin main` `dedaa55..127255e`.
`git ls-remote origin refs/heads/main` → `127255ec37d3c5a7bea673c9aab1052bd8450174	refs/heads/main`
Les deux worktrees retirés (`git worktree list` = racine seule).

### Issue #57
`gh issue view 57 --json state,closedAt` → `{"closedAt":"2026-10-17T13:49:58Z","state":"CLOSED"}` (fermée automatiquement par `closes #57` au push de `main`).

### Hors périmètre constaté (non corrigé)
- `?? vault/echanges/F02.md` est apparu pendant le mandat (mandat M0005 d'une autre fenêtre, cf. `events.jsonl`) : non touché.
- Index QMD `tom` périmé (voir plus haut) — à traiter par mandat dédié ou par l'orchestrateur.

### Rituels de fin
`vault/reprise/F01.md` réécrit ; `vault/runtime/events.jsonl` (`RAPPORT`, src fenetre) + `state.json` (`windows.F01`) écrits, non committés. Commit vault `docs(vault): reprise F01 M0004` = `5bee2371c5e9e92e0ffa96b07acedced2a9b5944` (pathspec : `vault/reprise/F01.md` + archive M0003), poussé `127255e..5bee237`.
`git ls-remote origin refs/heads/main` → `5bee2371c5e9e92e0ffa96b07acedced2a9b5944	refs/heads/main`

M0004
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ✅
Merge main          ✅
Main push           ✅
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED

