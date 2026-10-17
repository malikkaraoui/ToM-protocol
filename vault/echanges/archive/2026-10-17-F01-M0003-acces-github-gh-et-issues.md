---
date: 2026-10-17
tags: [github, gh, issues]
session: F01
mandat: acces-github-gh-et-issues
mandat_id: M0003
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F01-M0003
branche: main
niveau: N2
derniere_maj: 2026-10-17T15:28:59+0200
---

# F01 / M0003 — Vérifier l'accès GitHub des sessions (gh) et ouvrir les issues en attente

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Contexte (autonome)

Décision du fondateur, 2026-10-17 15:22 : « il faut à tout prix que l'équipe de codeurs ait un accès total au dépôt sur GitHub ». Dépôt : `malikkaraoui/ToM-protocol` (remote `origin` = `git@github.com:malikkaraoui/ToM-protocol.git`).
Constat de M0001/M0002 (archives dans `vault/echanges/archive/`) : aucune issue n'a pu être ouverte, le **connecteur MCP GitHub** échoue (`Authorization header is badly formatted`). Or `.claude/settings.json` du projet dit : MCP Copilot retiré, **GitHub passe par la CLI `gh`**. Personne n'a encore essayé `gh`.

**Règle pour ce mandat et les suivants : GitHub = `gh` CLI, jamais le connecteur MCP GitHub.**

Aucun code touché. Pas de branche : seul le vault est committé, directement sur `main` (autorisé pour `vault/` seul, décision #41).

## Gardes de précondition

1. `git fetch origin` puis `git rev-parse HEAD origin/main` : les deux = `aebe9fc3134b00d452ee7199c22e967e9daf35fe`. Sinon STOP.
2. `git branch --show-current` = `main`. Sinon STOP.

## Mission

1. Rituel de début : `statut: en-cours` dans ce frontmatter.
2. **Diagnostic `gh`** (lecture seule, aucune sortie de jeton dans le rapport : masque toute valeur de jeton) :
   - `command -v gh && gh --version`
   - `gh auth status` (sortie collée, jeton masqué)
   - `gh api repos/malikkaraoui/ToM-protocol --jq '{full_name, private, permissions}'` : on attend `permissions.admin` ou `push` = `true`.
   - `gh api user --jq .login`
   - `gh` absent, non connecté, ou sans `push` → **STOP propre** : écris exactement ce qui manque et la commande que le fondateur doit taper lui-même dans un terminal (ex. `gh auth login`, avec les choix à faire), puis rituels de fin. Ne lance jamais `gh auth login` toi-même (interactif).
3. **Connecteur MCP GitHub** (diagnostic seulement, ne modifie rien) : trouve d'où il vient, sans afficher de secret : `claude mcp list` (si la commande existe), puis cherche le nom du serveur GitHub dans `~/.claude.json`, `~/.claude/settings.json`, `.claude/settings.json`, `.claude/settings.local.json`, `.mcp.json` (`grep -n -i github` en n'affichant que les noms de clés, jamais une valeur d'en-tête ou de jeton). Rapporte : fichier, nom du serveur, type d'auth attendu, et ta recommandation (le retirer, ou le réparer) **sans rien changer**.
4. **Issues**, seulement si l'étape 2 est verte, avec `gh issue create --repo malikkaraoui/ToM-protocol`. Avant chaque création, `gh issue list --repo malikkaraoui/ToM-protocol --search "<mots clés>" --state all` : doublon trouvé → ne crée pas, cite son numéro.
   a. **Hook `post-checkout` graphify sort en code 1 à chaque changement de branche et à chaque `git worktree add`.** Lis `.husky/post-checkout` en entier pour trouver la cause probable (sans l'exécuter : il reconstruit `graphify-out/`). Corps : symptôme (vu par M0001 et M0002 le 2026-10-17), fichier, cause probable étiquetée [HYPOTHÈSE] ou [VÉRIFIÉ], piste de correction. Pas de correctif dans ce mandat.
   b. **`pnpm lint` échoue dans un worktree neuf** (`biome: command not found`, pas de `node_modules`) ; contournement vérifié par M0002 : `PATH=<racine>/node_modules/.bin:$PATH pnpm lint`. Piste : l'ajouter à `pieges_worktree[]` de `config/harnais.json`.
   Colle l'URL de chaque issue créée.
5. Ne touche à aucun fichier hors `vault/`. Intouchables (modifiés par le fondateur, jamais add/checkout/stash) : `scripts/pre-push-gate.sh`, `vault/30-discoveries.md`. `vault/echanges/F01.md` n'est jamais committé.

## Conditions STOP

Garde en échec ; `gh` indisponible ou sans droit d'écriture (étape 2) ; toute sortie qui afficherait un secret (tu t'arrêtes avant de la coller) ; tout imprévu git.

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; jamais `--no-verify` ; jamais un secret dans un fichier ou un rapport.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0003`), footer de statut (gabarits §4) en toute dernière ligne (Implementation = diagnostic fait ; Commit/Main push = commit du vault ; Review/Merge/Branch push = sans objet, écrire ✅ seulement si fait, sinon ❌ avec « sans objet » au rapport).
2. `vault/reprise/F01.md` réécrit pour M0003 (5 lignes au plus).
3. `vault/runtime/events.jsonl` (ajout, `src: "fenetre"`, `f: "F01"`, `mandat: "M0003"`) et `vault/runtime/state.json` (entrée `windows.F01`) : écrits, **jamais committés**.
4. Commit sur `main`, pathspec explicite : `vault/reprise/F01.md` et `vault/echanges/archive/2026-10-17-F01-M0002-commit-pose-harnais-v1.6.0-biome.md` ; message `docs(vault): reprise F01 M0003`, sans trailer (`git.trailer` non rempli). Push, `git ls-remote origin refs/heads/main` collé.
5. **Dernier geste** : `statut: reponse-disponible` dans ce frontmatter.

## Rapport M0003


Gardes : `HEAD` = `origin/main` = `aebe9fc3134b00d452ee7199c22e967e9daf35fe`, branche `main` → OK.

### 1. Diagnostic `gh` — VERT
```
/opt/homebrew/bin/gh
gh version 2.95.0 (2026-06-17)
github.com
  ✓ Logged in to github.com account malikkaraoui (keyring)
  - Active account: true
  - Git operations protocol: https
  - Token: gho_****
  - Token scopes: 'gist', 'read:org', 'repo', 'workflow'
{"full_name":"malikkaraoui/ToM-protocol","permissions":{"admin":true,"maintain":true,"pull":true,"push":true,"triage":true},"private":false}
malikkaraoui
```
Les sessions ont un accès total au dépôt via `gh` (admin + push). Rien à faire côté fondateur.

### 2. Connecteur MCP GitHub — diagnostic, rien modifié
- `claude mcp list` : `plugin:github:github: https://api.githubcopilot.com/mcp/ (HTTP) - ⊘ Disabled for this project`.
- Origine : plugin `github@claude-plugins-official` ; son `.mcp.json` (`~/.claude/plugins/cache/claude-plugins-official/github/<hash>/.mcp.json`) envoie `Authorization: Bearer ${GITHUB_PERSONAL_ACCESS_TOKEN}`.
- Cause [VÉRIFIÉ] : `GITHUB_PERSONAL_ACCESS_TOKEN` n'est pas définie dans l'environnement → en-tête `Bearer ` vide → `Authorization header is badly formatted`. Le serveur est en plus l'endpoint **Copilot** (abonnement retiré selon `.claude/settings.json`, clé `mcpServers._comment_github`).
- Activation : `~/.claude/settings.json` `enabledPlugins."github@claude-plugins-official": false` (global) ; `.claude/settings.json` projet le remettait à `true` ; `~/.claude.json` le liste dans `disabledMcpServers` pour tom-protocol, 1RR, Boites-a-livres.
- **Recommandation : le retirer** (garder `false`/absent dans `.claude/settings.json` projet). `gh` couvre le besoin, sans dépendance Copilot ni jeton en variable d'environnement.
- ⚠️ Constat : la ligne `"github@claude-plugins-official": true` de `.claude/settings.json` a été **supprimée dans la copie de travail à 15:30:16**, avant le démarrage de cette session (15:30:20) — pas par moi (je n'ai fait que des lectures). Laissée telle quelle, non committée : à confirmer par le fondateur.

### 3. Issues
Recherches de doublons (`gh issue list --state all` sur « post-checkout graphify », « graphify hook », « biome worktree », « pnpm lint worktree », « biome command not found ») : aucun résultat.
- a. https://github.com/malikkaraoui/ToM-protocol/issues/57 — hook `post-checkout` graphify. Cause [VÉRIFIÉ] : le wrapper husky lance le hook en `sh -e` ; `graphify-out/` est suivi par git (existe dans tout worktree) ; graphify désinstallé (Python épinglé `~/.local/share/uv/tools/graphifyy/bin/python` absent) ; ligne 56 `GRAPHIFY_BIN=$(command -v graphify 2>/dev/null)` échoue → `sh -e` sort en 1 avant le repli `exit 0` ligne 84. Mécanisme reproduit isolément (`sh -e -c 'X=$(command -v absent 2>/dev/null); echo …'` → code 1). Commentaire ajouté : `post-commit` sort aussi en code 1 (vu au commit `dedaa55`) — https://github.com/malikkaraoui/ToM-protocol/issues/57#issuecomment-6234467901
- b. https://github.com/malikkaraoui/ToM-protocol/issues/58 — `pnpm lint` dans un worktree neuf, contournement `PATH=<racine>/node_modules/.bin:$PATH`, piste `pieges_worktree[]` (aujourd'hui `[]`).

### 4. Vault
Commit `dedaa553642585b73f5831e1e9e2a5cb758729c8` (`docs(vault): reprise F01 M0003`, sans trailer, auteur gmail) : `vault/reprise/F01.md` + archive M0002. Hooks : pre-commit biome vert ; post-commit graphify code 1 (non bloquant, cf. #57).
```
dedaa553642585b73f5831e1e9e2a5cb758729c8	refs/heads/main
```
`events.jsonl` et `state.json` écrits, non committés (ignorés par git). Intouchables (`scripts/pre-push-gate.sh`, `vault/30-discoveries.md`) non touchés. Tests / Branch push / Review / Merge main : sans objet (aucun code, commit vault direct sur `main`, décision #41).

Scellé tests : sans objet
Contre-témoins : aucun

M0003
Implementation      ✅
Tests               ❌
Commit              ✅
Branch push         ❌
Review              ❌
Merge main          ❌
Main push           ✅
TASK STATUS         READY_FOR_INTEGRATION
PROJECT STATUS      NOT_INTEGRATED
