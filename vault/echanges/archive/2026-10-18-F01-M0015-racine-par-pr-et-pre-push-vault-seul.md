---
date: 2026-10-18
tags: [pre-push, vault, racine, ci-local]
session: F01
mandat: racine-par-pr-et-pre-push-vault-seul
mandat_id: M0015
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol-scratch-M0015
branche: chore/racine-et-pre-push-vault
niveau: N2
derniere_maj: 2026-10-18T10:49:09+0200
---

# F01 / M0015 — (1) Enregistrer par PR 3 fichiers de la racine ; (2) #71 : pas de CI locale pour un push `vault/` seul

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, index `tom`) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.
> **IMPÉRATIF (fondateur) : on ne pousse pas si ce n'est pas nickel.** Jamais `--no-verify`.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

- **Décision du fondateur, 2026-10-18 10:10 (verbatim)** : « Feu vert : un seul mandat N2. (1) Enregistrer pre-push-gate.sh, 30-discoveries.md et settings.json par une PR. (2) Issue #71 : pre-push-gate.sh ne relance pas les tests quand le push ne touche que vault/. Preuve : un push qui ne change que vault/ passe en moins de 10 s, et un push qui touche du code relance bien les tests. L'alerte « main hors règle » vient du harnais : ne rien faire de ton côté, l'orchestration la corrige. »
- Ces trois fichiers étaient jusqu'ici **intouchables** ; la décision ci-dessus lève l'interdit **pour ce mandat seulement et pour ces trois fichiers seulement** : `scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `.claude/settings.json`.
- `main` est protégée (ruleset, M0014) : **aucun push direct**, tout passe par PR + check `harnais-ok` + auto-merge. `origin/main` attendu ≥ `d8b1cd6c785d1763b4598ccceae30db8d3a4e539`.
- La racine `/Users/malik/Documents/tom-protocol` est en retard (`7bf72ce`) et sale. Contre `origin/main` (lu 2026-10-18 ~10:05) : `scripts/pre-push-gate.sh` ≈146 lignes changées, `vault/30-discoveries.md` +1, `.claude/settings.json` -1. Ces versions de la racine sont celles à enregistrer.
- **Issue #71 (texte réel)** : « La CI saute désormais ses jobs de test quand seul `vault/` change (job `changes`), mais `.husky/pre-push` lance `scripts/ci-local.sh` sans condition : ≈15 min et ≈20 Go de `target/` par push de reprise/revue. Piste : appliquer la même règle que `changes` (liste non vide, uniquement `vault/`, base connue ; sinon tout rejouer). » **Attention** : le mécanisme en cause est `.husky/pre-push` → `scripts/ci-local.sh`, pas `scripts/pre-push-gate.sh`. Vérifie-le toi-même (qui appelle quoi au push : `git config core.hooksPath`, `.husky/_/pre-push`, `.husky/pre-push`, `scripts/harnais-hooks/pre-push`, et si `pre-push-gate.sh` est appelé par un hook). Corrige **là où les tests sont réellement relancés** ; si `pre-push-gate.sh` est aussi sur ce chemin, applique-lui la même règle.
- Ne touche **pas** à l'alerte `MAIN_HORS_REGLE` ni à `config/harnais.json` : c'est le harnais qui s'en charge.

## Préparation obligatoire du worktree (issues #62, #64, #65)

- Worktree **hors du dépôt** : `/Users/malik/Documents/tom-protocol-scratch-M0015`, nouvelle branche `chore/racine-et-pre-push-vault` depuis `origin/main`.
- Avant tout commit : `pnpm install --frozen-lockfile` puis `test -x .husky/_/pre-push && echo HOOKS_OK` — sans `HOOKS_OK`, ni commit ni push.
- Push avec `GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30'`. Jamais `--no-verify`.
- Disque : `du -sh target/` avant/après ; `cargo clean` dans le worktree après le dernier run.

## Gardes de précondition

1. `git -C /Users/malik/Documents/tom-protocol fetch origin` ; `git merge-base --is-ancestor d8b1cd6c785d1763b4598ccceae30db8d3a4e539 origin/main`. Sinon STOP.
2. `grep -h '^statut' vault/echanges/F02.md vault/echanges/F03.md` : aucune autre fenêtre `en-cours`. Sinon STOP.
3. `gh auth status` OK. Sinon STOP.

## Mission

1. Rituel de début : `statut: en-cours`. Worktree + préparation ci-dessus.
2. **(1) Les 3 fichiers de la racine** : colle au rapport `git -C <racine> diff origin/main -- scripts/pre-push-gate.sh vault/30-discoveries.md .claude/settings.json` (repli `<details>` si long) avec, pour chacun, une phrase sur ce que ça change. **Aucune IP, aucun secret, aucun nom d'hôte** ne doit partir dans un commit : `grep -nE '([0-9]{1,3}\.){3}[0-9]{1,3}|ssh |token|secret|password|api[_-]?key' ` sur les trois → si un résultat est un vrai secret/IP : STOP (rapport, rien de committé). `.claude/settings.json` doit rester du JSON valide (`python3 -m json.tool`). Copie les trois fichiers de la racine dans le worktree (`cp --`), commit pathspec explicite : `chore: enregistrer pre-push-gate, 30-discoveries et settings de la racine`.
3. **(2) #71** : dans le hook qui relance la CI locale, saute `scripts/ci-local.sh` **seulement si** tous les fichiers changés par TOUTES les refs poussées sont sous `vault/`. Règle stricte (à documenter en commentaire, en français) :
   - lire les refs sur **stdin** (`<local_ref> <local_sha> <remote_ref> <remote_sha>`, format git pre-push) ;
   - suppression de ref (`local_sha` nul) → rien à tester pour cette ref ;
   - `remote_sha` connu → base = `remote_sha` ; ref neuve (`remote_sha` nul) → base = `git merge-base origin/main <local_sha>` ; base introuvable → **tout rejouer** ;
   - liste des fichiers (`git diff --name-only <base> <local_sha>`) vide, en erreur, ou contenant un seul chemin hors `vault/` → **tout rejouer** ;
   - message explicite dans les deux cas (`[pre-push] vault/ seul : CI locale sautée` / `[pre-push] code touché : CI locale complète`).
   - Le reste du hook (smoke observabilité, etc.) garde son comportement. Si le hook utilise déjà stdin plus bas, ne le consomme pas deux fois (lis-le une fois dans une variable).
   - Si `scripts/ci-local.sh` contient un contrôle de dérive qui compare le hook ou `ci.yml`, il doit rester vert.
   Tests du hook **sans réseau et sans push** : un petit script de test (`scripts/test-pre-push-vault.sh` ou équivalent existant) qui crée un dépôt jetable dans `$(mktemp -d)`, et vérifie au moins : vault seul → sauté ; code seul → rejoué ; vault + code → rejoué ; ref neuve vault seul → sauté ; suppression de ref → sauté ; base introuvable → rejoué. Le script remplace `ci-local.sh` par un témoin (variable d'environnement ou stub dans le dépôt jetable), jamais en modifiant le vrai. Sorties collées. Commit : `fix(pre-push): sauter la CI locale quand seul vault/ change (#71)`.
4. **Preuves terrain (exigées par le fondateur)** :
   - **Push de code** : le premier push de la branche (qui touche `.husky/`/`scripts/`) doit lancer la CI locale complète : colle les premières lignes du hook (`code touché : CI locale complète`) et la fin verte, avec `time`.
   - **Push `vault/` seul** : après `gh pr create`, commit du rapport R014 (vault seul) puis `time git push` : doit afficher `vault/ seul : CI locale sautée` et durer **moins de 10 s** (sortie `time` collée). Plus de 10 s → STOP (rapport, cause mesurée).
5. **Doublage N2** par sous-agent FRAIS (lecture seule) : relit la règle du point 3 contre les 6 cas, cherche un chemin qui ferait sauter la CI alors que du code change (refs multiples, renommage `vault/x → src/x`, chemin `vault` sans slash, sous-module, base introuvable), vérifie qu'aucune IP/secret n'est committé et que le JSON est valide. Rapport `vault/revues/2026-10-18-R014-racine-et-pre-push-vault.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40 du dernier commit de code>`, `niveau: N2`). Réserve bloquante seulement si un test rouge la prouve ; remarques → issues.

## Intouchables

`docs/MISSION.md`, `config/harnais.json`, `.claude/settings.local.json`, `~/.harnais/` (aucune lecture), les `vault/echanges/F0x.md` vivants (jamais committés). La racine : **aucune** écriture (seul `git fetch` permis) ; tu y lis les 3 fichiers, tu ne les modifies pas.

## Règles des sous-agents (verbatim)

(1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre. **Aucun sous-agent ne lance de build ou de test cargo** (incident M0006) : les builds restent à la session mère.

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée `"${VAR:?}/${SOUS:?}"` ; jamais `--no-verify` ; jamais de force-push ; aucun push sur `main` ; aucun commit non signé autrement que la convention du projet (pas de trailer `Co-Authored-By`, `.claude/CLAUDE.md` §13).

## Fusion (mode `pr`, gabarit §9)

Push de la branche → `gh pr create -B main -H chore/racine-et-pre-push-vault -t "M0015 -- racine par PR + pre-push vault seul (#71)"` (corps : `Closes #71`) → commit du rapport R014 sur la branche → push (preuve vault seul, point 4) → `gh pr merge chore/racine-et-pre-push-vault --auto --merge --delete-branch --match-head-commit <HEAD>` → attendre la fusion (`gh pr view --json state,mergedAt`, borne 40 min), sortie collée → `gh run list --branch main -L 2` + `gh run watch <id> --exit-status` collés. `harnais-ok` rouge → rapport, aucun contournement.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0015`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F01"`, `mandat: "M0015"`) et `vault/runtime/state.json` (`windows.F01` seulement, relu juste avant d'écrire) : jamais committés.
4. Reprise : nouvelle branche `docs/reprise-F01-M0015` depuis `origin/main` à jour (worktree hors dépôt `/Users/malik/Documents/tom-protocol-scratch-M0015-reprise`, même préparation), commit pathspec : `vault/reprise/F01.md` + `vault/echanges/archive/2026-10-17-F01-M0012-adn-mission-decisions-ecarts-cible.md` (copié depuis la racine) ; `docs(vault): reprise F01 M0015` ; push (c'est un 2e push vault seul : `time` collé) ; `ls-remote` collé ; PR + auto-merge ; fusion attendue (borne 30 min).
5. `cargo clean` dans les worktrees ; worktrees laissés en place, signalés.
6. **Dernier geste** : `statut: reponse-disponible`.

## Rapport M0015


**En tête : INTEGRATED.** PR #75 fusionnée sur `main` (`ca809109b788c7e4bb310b7753abe5214b19e888`), CI `main` verte ; reprise PR #76 fusionnée (`aaeee846a1c6ed42d1f9d0f6e4f132cd7cc97473`). Push vault seul : **2,29 s** et **2,18 s**, CI locale sautée ; push de code : CI locale complète **TOUT VERT** en 10 min 54 s. Le doublage a prouvé **deux fail-open successifs** dans ma règle (même classe : décision « sauter » dépendant d'une E/S shell), corrigés avant GO — voir §4.

Scellé tests : indisponible (`scripts/verifier-rendu.mjs` absent du dépôt)
Contre-témoins : aucun

### 0. Gardes
- `git fetch origin` : `origin/main` = `d8b1cd6c785d1763b4598ccceae30db8d3a4e539` ; `merge-base --is-ancestor d8b1cd6… origin/main` → OK.
- F02 et F03 : `statut: reponse-disponible` (aucune `en-cours`). `gh auth status` : connecté (malikkaraoui).
- Worktree `/Users/malik/Documents/tom-protocol-scratch-M0015`, branche `chore/racine-et-pre-push-vault` depuis `origin/main` ; `pnpm install --frozen-lockfile` → `HOOKS_OK`. `target/` absent avant.

### 1. Qui relance les tests au push (vérifié)
`git config core.hooksPath` = `.husky/_` ; `.husky/_/pre-push` → `.husky/_/h` → `sh -e .husky/pre-push` → `./scripts/ci-local.sh` (sans condition). `scripts/pre-push-gate.sh` n'est appelé par **aucun** hook (grep sur `scripts/ .husky/ .github/ package.json Makefile` : seules ses propres lignes d'usage) ; `scripts/harnais-hooks/pre-push` n'est pas installé (hooksPath = `.husky/_`). Correctif donc dans `.husky/pre-push` seul ; `pre-push-gate.sh` non modifié au-delà de la copie racine.

### 2. Les 3 fichiers de la racine — commit `bc973c8cb603c92f988e0dbf8c526357c726aaa7`
- `scripts/pre-push-gate.sh` : passe à la version canonique claude-atelier v2.0.0 — étape 0 sync remote (fetch + `pull --rebase` si en retard, refus si divergence sauf `FORCE_PUSH=1`) + email d'auteur paramétrable (`GATE_ALLOWED_EMAIL` / `gate.allowedemail`), scan secrets full-tree à haute confiance, étape 6 review pluggable ; commentaires francisés.
- `vault/30-discoveries.md` : +1 ligne (2026-08-20, transfert inter-projets ToM → 1RR attesté).
- `.claude/settings.json` : retire `"github@claude-plugins-official": true` (cohérent avec « GitHub = gh CLI »).
- Scan `grep -nE '([0-9]{1,3}\.){3}[0-9]{1,3}|ssh |token|secret|password|api[_-]?key'` sur les **lignes ajoutées** : 4 résultats, tous le mot « secret(s) » dans des commentaires/libellés de `pre-push-gate.sh` (l. 8, 70, 71, 75 du diff) → aucun secret, aucune IP, aucun hôte. Sur fichiers entiers : `30-discoveries.md` contient 10 occurrences (IP LAN/publique) **déjà présentes sur `origin/main`**, aucune ajoutée (suivi : #67). `python3 -m json.tool .claude/settings.json` → `JSON_OK`.
- Après fusion : `git diff --quiet ca80910 -- <3 fichiers>` à la racine → identiques à `main`.

<details><summary>diff racine vs origin/main d8b1cd6 (265 lignes ; IP des lignes de contexte masquées)</summary>

```diff
diff --git a/.claude/settings.json b/.claude/settings.json
index df335fc..1998afa 100644
--- a/.claude/settings.json
+++ b/.claude/settings.json
@@ -168,7 +168,6 @@
     "frontend-design@claude-plugins-official": true,
     "code-review@claude-plugins-official": true,
     "context7@claude-plugins-official": true,
-    "github@claude-plugins-official": true,
     "feature-dev@claude-plugins-official": true,
     "code-simplifier@claude-plugins-official": true,
     "skill-creator@claude-plugins-official": true,
diff --git a/scripts/pre-push-gate.sh b/scripts/pre-push-gate.sh
index d1ce5ec..d83575f 100755
--- a/scripts/pre-push-gate.sh
+++ b/scripts/pre-push-gate.sh
@@ -1,18 +1,28 @@
 #!/usr/bin/env bash
 # scripts/pre-push-gate.sh — Pre-push gate for claude-atelier projects
 #
-# 5 checks: secrets → tracked sensitive files → lint → build → tests
+# CANONIQUE : ce fichier vit dans claude-atelier et est distribué à l'identique
+# dans tous les projets. Toute amélioration se fait ICI puis se redistribue —
+# jamais dans une copie locale (leçon : 4 variantes divergentes découvertes le
+# 2026-08-06, chacune avec une amélioration que les autres n'avaient pas).
+#
+# 6 checks : [0] sync remote + email auteur → [1] secrets (diff + full-tree)
+#          → [2] fichiers sensibles trackés → [3] lint → [4] build → [5] tests
+#          → [6] review (pluggable)
 # Exit codes: 0 = pass, 1 = blocked
 #
 # Usage:
-#   bash scripts/pre-push-gate.sh          # run all 5 checks
-#   bash scripts/pre-push-gate.sh --quick  # secrets + tracked files only (2 checks)
+#   bash scripts/pre-push-gate.sh          # tous les checks
+#   bash scripts/pre-push-gate.sh --quick  # checks 0-2 uniquement
 #
-# Stack auto-detection: the script detects the stack from the project
-# files and adjusts lint/build/test commands accordingly.
+# Spécifique projet : si scripts/pre-push-gate-step6.sh existe, il est sourcé
+# comme étape 6 (ex. workflow handoff Copilot de Boites-a-livres). Le reste du
+# fichier doit rester identique au canonique.
 
 set -euo pipefail
 
+GATE_VERSION="2.0.0"
+
 RED='\033[0;31m'
 GREEN='\033[0;32m'
 YELLOW='\033[0;33m'
@@ -21,21 +31,57 @@ NC='\033[0m'
 pass()  { echo -e "${GREEN}[PASS]${NC} $1"; }
 fail()  { echo -e "${RED}[FAIL]${NC} $1"; exit 1; }
 warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
-step()  { echo -e "\n${YELLOW}[$1/5]${NC} $2"; }
+step()  { echo -e "\n${YELLOW}[$1/6]${NC} $2"; }
+
+echo -e "pre-push-gate v${GATE_VERSION}"
 
 QUICK=false
 [[ "${1:-}" == "--quick" ]] && QUICK=true
 
-# ─── 0/5 Email d'auteur (anti-typo hmail) ────────────────────────────────────
-#
-# Un typo `hmail` (au lieu de `gmail`) a rendu 148 commits NON ATTRIBUÉS sur
-# GitHub (email inconnu du compte → invisibles dans l'activité). Ce gate bloque
-# tout commit à pousser dont l'auteur OU le committer n'est pas l'email gmail
-# lié au compte. Les comptes de service (github noreply, bots) sont tolérés.
+# ─── 0/6-a Remote sync check ──────────────────────────────────────────────────
+# Si le remote a des commits que le local n'a pas, les intégrer maintenant
+# plutôt que de se faire rejeter après 6 checks.
+if git rev-parse --abbrev-ref --symbolic-full-name '@{u}' >/dev/null 2>&1; then
+    git fetch --quiet 2>/dev/null || true
+    LOCAL=$(git rev-parse HEAD 2>/dev/null)
+    REMOTE=$(git rev-parse '@{u}' 2>/dev/null || echo "$LOCAL")
+    BASE=$(git merge-base "$LOCAL" "$REMOTE" 2>/dev/null || echo "$LOCAL")
+    if [[ "$LOCAL" == "$REMOTE" ]]; then
+        : # Déjà synchronisé, rien à faire
+    elif [[ "$BASE" == "$LOCAL" ]]; then
+        # Remote ahead, local behind → rebase simple
+        echo -e "${YELLOW}[SYNC]${NC} Remote a des commits non intégrés — rebase automatique..."
+        if ! git pull --rebase --quiet; then
+            git rebase --abort 2>/dev/null || true
+            echo -e "${RED}[FAIL]${NC} Rebase échoué et annulé. Résous manuellement puis relance."
+            exit 1
+        fi
+        echo -e "${GREEN}[SYNC]${NC} Rebase OK — local à jour avec remote."
+    elif [[ "$BASE" == "$REMOTE" ]]; then
+        : # Local ahead (fast-forward possible), rien à faire
+    else
+        # Divergence réelle : local ET remote ont chacun des commits
+        if [[ "${FORCE_PUSH:-}" == "1" ]]; then
+            echo -e "${YELLOW}[SYNC]${NC} FORCE_PUSH=1 — divergence acceptée (rebase intentionnel)."
+        else
+            echo -e "${RED}[FAIL]${NC} Branches divergentes (local et remote ont chacun des commits)."
+            echo -e "         Fais 'git pull --rebase' manuellement, ou FORCE_PUSH=1 si rebase intentionnel."
+            exit 1
+        fi
+    fi
+fi
 
-echo -e "\n${YELLOW}[0/5]${NC} Email d'auteur des commits à pousser..."
+# ─── 0/6-b Email d'auteur (anti-typo hmail) ──────────────────────────────────
+# Un typo `hmail` (au lieu de `gmail`) a rendu 172 commits NON ATTRIBUÉS sur
+# GitHub, sur DEUX projets à 24 h d'écart (1RR : 39, partie politique : 133) —
+# deux réécritures d'historique pour le corriger. Ce check bloque tout commit à
+# pousser dont l'auteur OU le committer n'est pas l'email attendu. Les comptes
+# de service (github noreply, bots) sont tolérés.
+# Override : GATE_ALLOWED_EMAIL env, ou `git config gate.allowedemail`.
 
-ALLOWED_EMAIL="karaoui.malik@gmail.com"
+step 0 "Sync remote + email d'auteur..."
+
+ALLOWED_EMAIL="${GATE_ALLOWED_EMAIL:-$(git config gate.allowedemail 2>/dev/null || echo "karaoui.malik@gmail.com")}"
 EMAIL_RANGE="origin/main..HEAD"
 git rev-parse --verify -q origin/main >/dev/null 2>&1 || EMAIL_RANGE="HEAD"
 BAD_EMAILS=$(git log "$EMAIL_RANGE" --format='%ae%n%ce' 2>/dev/null | sort -u \
@@ -45,11 +91,16 @@ if [[ -n "$BAD_EMAILS" ]]; then
     echo "$BAD_EMAILS" | sed 's/^/    /'
     fail "Email d'auteur/committer ≠ ${ALLOWED_EMAIL}. Corriger AVANT push (git commit --amend --reset-author, ou filter-branch pour plusieurs commits)."
 fi
-pass "Emails d'auteur OK (${ALLOWED_EMAIL})"
+pass "Sync remote OK · emails d'auteur OK (${ALLOWED_EMAIL})"
+
+# ─── Auto-sync SECURITY.md ────────────────────────────────────────────────────
+if [[ -f "scripts/update-security.js" ]] && [[ -f "package.json" ]]; then
+    node scripts/update-security.js 2>/dev/null || true
+fi
 
-# ─── 1/5 Secrets in staged diff ──────────────────────────────────────────────
+# ─── 1/6 Secrets ──────────────────────────────────────────────────────────────
 
-step 1 "Audit secrets dans le diff..."
+step 1 "Audit secrets (diff + full-tree)..."
 
 SECRET_PATTERNS='(api_key|api[-_]?secret|secret[-_]?key|access[-_]?token|auth[-_]?token|password|private_key|BEGIN RSA|BEGIN OPENSSH|BEGIN EC|sk-[a-zA-Z0-9]{20,}|AIza[a-zA-Z0-9_-]{35}|AKIA[A-Z0-9]{16}|ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|ya29\.[a-zA-Z0-9_-]+)'
 
@@ -65,9 +116,23 @@ if git diff HEAD 2>/dev/null | grep -iEq "$SECRET_PATTERNS"; then
     warn "Pattern suspect detecte dans les changements non staged. Verifier avant push."
 fi
 
+# Filet full-tree : un secret DEJA commite (hors diff) doit aussi bloquer le push.
+# Le scan de diff ci-dessus rate les clés introduites lors d'un commit anterieur
+# et jamais re-touchees (cas de la clé web Firebase du 10/07). On rescanne donc
+# TOUT l'arbre tracké, mais uniquement sur des formats de clés à haute confiance,
+# pour éviter les faux positifs des mots génériques (api_key, password...).
+HARDSECRET='(sk-[a-zA-Z0-9]{20,}|AIza[a-zA-Z0-9_-]{35}|AKIA[A-Z0-9]{16}|ghp_[a-zA-Z0-9]{36}|gho_[a-zA-Z0-9]{36}|ya29\.[a-zA-Z0-9_-]+|-----BEGIN (RSA|OPENSSH|EC) PRIVATE KEY-----)'
+if git grep -InE "$HARDSECRET" -- ':!scripts/pre-push-gate.sh' 2>/dev/null | grep -q .; then
+    echo ""
+    git grep -InE "$HARDSECRET" -- ':!scripts/pre-push-gate.sh' 2>/dev/null \
+        | sed -E 's/(AIza|AKIA|sk-|ghp_|gho_)[A-Za-z0-9_-]+/\1…[masque]/g' | head -20
+    echo ""
+    fail "SECRET present dans un fichier tracke (hors diff). Rotation + purge necessaires."
+fi
+
 pass "Aucun secret detecte"
 
-# ─── 2/5 Tracked sensitive files ─────────────────────────────────────────────
+# ─── 2/6 Tracked sensitive files ─────────────────────────────────────────────
 
 step 2 "Fichiers sensibles trackes..."
 
@@ -82,17 +147,16 @@ pass "Aucun fichier sensible tracke"
 
 # Quick mode stops here
 if $QUICK; then
-    echo -e "\n${GREEN}GATE RAPIDE PASSEE${NC} (2/5 checks, --quick mode)"
+    echo -e "\n${GREEN}GATE RAPIDE PASSEE${NC} (checks 0-2, --quick mode)"
     exit 0
 fi
 
 # ─── Stack detection ─────────────────────────────────────────────────────────
 #
-# A repo can carry more than one stack at once (e.g. this one: Rust workspace
-# + legacy TS package.json at root) — detect ALL of them and gate on each,
-# instead of stopping at the first match. A single `unknown` used to mean
-# Rust changes silently skipped lint/build/test whenever package.json was
-# also present.
+# Un repo peut porter plusieurs stacks à la fois (ex. workspace Rust + package.json
+# legacy à la racine) — détecter TOUTES et gater sur chacune, au lieu de s'arrêter
+# à la première. Un simple `unknown` faisait sauter lint/build/test Rust en silence
+# dès qu'un package.json était aussi présent.
 
 detect_stacks() {
     local stacks=()
@@ -110,7 +174,7 @@ detect_stacks() {
 
 read -ra STACKS <<< "$(detect_stacks)"
 
-# ─── 3/5 Lint ─────────────────────────────────────────────────────────────────
+# ─── 3/6 Lint ─────────────────────────────────────────────────────────────────
 
 step 3 "Lint (${STACKS[*]})..."
 
@@ -167,7 +231,7 @@ case "$STACK" in
 esac
 done
 
-# ─── 4/5 Build ────────────────────────────────────────────────────────────────
+# ─── 4/6 Build ────────────────────────────────────────────────────────────────
 
 step 4 "Build (${STACKS[*]})..."
 
@@ -210,19 +274,18 @@ case "$STACK" in
 esac
 done
 
-# ─── 5/5 Tests ─────────────────────────────────────────────────────────────────
+# ─── 5/6 Tests ────────────────────────────────────────────────────────────────
 
 step 5 "Tests (${STACKS[*]})..."
 
 for STACK in "${STACKS[@]}"; do
 case "$STACK" in
     rust)
-        # HERMÉTICITÉ (#5) : la gate LOCALE ne lance QUE les tests unitaires
-        # (--lib, hermétiques). Les tests d'INTÉGRATION (tests/) créent de vrais
-        # nœuds qui bindent le LAN + publient au rendezvous DHT partagé →
-        # ils rejoignaient la flotte de PROD (« anonymous » en DIRECT sur les
-        # appareils de l'utilisateur). La CI GitHub (runner isolé, hors LAN)
-        # exécute la couverture d'intégration complète — voir .github/workflows.
+        # HERMÉTICITÉ : la gate LOCALE ne lance QUE les tests unitaires
+        # (--lib, hermétiques). Les tests d'INTÉGRATION (tests/) peuvent créer
+        # de vrais nœuds/serveurs qui touchent le LAN ou des services partagés
+        # (incident tom-protocol : des nœuds de test rejoignaient la flotte de
+        # PROD). La CI (runner isolé) exécute la couverture d'intégration.
         if cargo test --workspace --lib 2>&1 | tail -30; then
             pass "Tests OK (cargo test --lib ; intégration → CI isolée)"
         else
@@ -267,6 +330,23 @@ case "$STACK" in
 esac
 done
 
+# ─── 6/6 Review (pluggable) ──────────────────────────────────────────────────
+
+step 6 "Review..."
+
+STEP6_LOCAL="$(cd "$(dirname "$0")" && pwd)/pre-push-gate-step6.sh"
+if [[ -f "$STEP6_LOCAL" ]]; then
+    # Étape 6 spécifique au projet (ex. dette handoff Copilot)
+    # shellcheck disable=SC1090
+    source "$STEP6_LOCAL"
+elif [[ -f ".claude/hooks/guard-review-auto.sh" ]] || [[ -f "hooks/guard-review-auto.sh" ]]; then
+    # Gate §25 = hook guard-review-auto.sh (PreToolUse sur git push, exit 2 si diff ≥ 50 lignes)
+    # Cette étape est informative uniquement — le hook est le vrai verrou.
+    pass "Review oracle gate active via hook guard-review-auto.sh — §25 OK"
+else
+    warn "Pas de review gate configurée (ni step6 local, ni hook guard-review-auto). Voir §25."
+fi
+
 # ─── Done ──────────────────────────────────────────────────────────────────────
 
 echo -e "\n${GREEN}GATE PASSEE${NC} — push autorise"
diff --git a/vault/30-discoveries.md b/vault/30-discoveries.md
index 1924559..f362785 100644
--- a/vault/30-discoveries.md
+++ b/vault/30-discoveries.md
@@ -9,6 +9,7 @@
 
 | Date | Découverte | Impact | Source |
 |------|-----------|--------|--------|
+| 2026-08-20 | **🔁 TRANSFERT INTER-PROJETS RÉUSSI (attestation Malik) : le savoir-faire acquis sur ToM — Freebox/NAS (VM Debian, systemd, IP LAN dynamique, port-forwarding, bornes `RuntimeMaxSec`) et la doctrine des services longue durée (borner chaque process sur une timeline, timeout + vérification d'effet, jamais de fire-and-forget) — a été réutilisé sur le projet 1RR (plateforme de paris Solana) pour faire tourner un watcher 24/24** qui détecte la fin des paris et déclenche les résolutions (oracle ou LLM). Valide la règle R5 (leçon sans nom de projet = transverse) dans le sens inverse : les leçons ToM remontées au vault central ont effectivement servi ailleurs. | Preuve que l'investissement « leçons transverses » (R5, mémoire `bound-every-node-and-campaign-on-a-timeline`, stack freebox) paie concrètement hors ToM | Attestation utilisateur (Malik, 2026-08-20) ; projet 1RR (watcher résolutions) |
 | 2026-08-11 | **🪵 CANAL LOG — ÉTAPE 1 (S1) commitée (`0e41628`) : groupe PUBLIC auto-porté.** Objectif (idée Malik) : voir les logs de TOUS les devices — iPhone nomade compris — via NOTRE réseau, sans rien exposer. Cheminement (3 approches) : log-udp broadcast **bloqué par iOS** (pas d'entitlement multicast) → collecteur UDP public sur la Freebox (marche mais expose un port) → **groupe « log » dans le protocole (dogfooding des groupes)**. **S1 = orchestration runtime** : au démarrage chaque nœud **rejoint-ou-crée** le groupe d'id RÉSERVÉ `__tomlog__` (broadcast `Join` → timeout 3 s → `create` public `invite_only=false`). ⚠️ **Bug critique débusqué en reprenant le pipeline loop-master (qui a STALLÉ sur le long build Rust) : `handle_create` tirait `GroupId::new()` = uuid ALÉATOIRE → aucune convergence** ; fix = id DÉTERMINISTE (= le name) pour le well-known. Relu Fable (adversarial) : **BLOQUANT** collision (un user créant « log ») → corrigé par l'id réservé `__tomlog__`. 3 tests d'intégration verts (convergence 2 et 3 nœuds + public). **CI rejoué EXACT** (job `tom-protocol` vert ; flake `tom-transport::gate_rejects_unlisted_peer` = parallélisme → fix `ci.yml --test-threads=1`) ; FFI `check-ffi` vert. **Reste** : S2 (émission des logs → groupe, throttlé, filtré UI) + S3 (route bot `:9300/group/inbox`) + build/déploiement flotte → test LIVE sur les 4 appareils. **Étape 2** = génome immortel via backup service (portage collectif, le groupe survit à son créateur). | Fondation du canal log commitée+validée ; bug de convergence évité avant terrain ; CI durci | commit `0e41628`, `docs/plans/etape1-groupe-log-public.md`, relecture Fable, `.github/workflows/ci.yml` |
 | 2026-08-09 | **✅ ENDURANCE FREEBOX 18 JOURS = fix OOM VALIDÉ EN RÉEL.** Retour de 15 j de vacances : la Freebox (relais NAS) a tourné **17 j 23 h sans interruption**, joignable en public (`nc <IP masquée>:3340`) depuis la Corse ET le wifi du bateau en pleine mer. `tom-node` (PID tom-chat) : **NRestarts=107** (≈ 1 cycle / 4 h = **RuntimeMaxSec PROPRE**, pas des crashs), **RSS 50 Mo STABLE** (Memory=30 M ; VM 147/920, avail 684), **0 OOM-kill** (`journalctl -k` 16 j VIDE), load ~0, ne chauffe pas (Malik confirme). `/status:8085` : `phase=connecte`, `conns_quic_live=3` (borné), sert de relais (2 clients). → **le fix `ConnectionInner` (build 140, pool-close détaché) + `RuntimeMaxSec=4h` tiennent en endurance longue durée** (vs OOM 688 Mo de juillet). **Finding mineur** : spam `Failed to connect to relay server` vers `euc1-1.relay.n0.iroh-canary.iroh.link` toutes ~25 s (résidu `n0_discovery`, `endpoint.rs:1412`) — bruit inutile, self-relay non affecté, à couper. Terrain maison : Apple TV (`f3aa5706`) active/connectée ; iPhone (`7a784ac9`) **hors ligne** (app pas en foreground — doctrine iOS). Collecteur `:9999` tourne mais écrit dans un fichier **unlinké** (`/private/tmp/tom_collector.log` supprimé-mais-ouvert, 248 Mo). | Fix OOM validé TERRAIN sur 18 j — plus une hypothèse ; 1 finding relais iroh mort à couper | `ssh root@<IP masquée>` (systemctl/journalctl/free/ps), `/status:8085`, sniff UDP `:9999` |
 | 2026-07-29 | **🌍 VALIDATION TERRAIN NOMADE RÉPÉTÉE (attestation Malik, vacances Corse) : iPhone en 4G ou WiFi, DEPUIS LA CORSE, échange des messages ToM avec la Freebox restée au domicile — RÉUSSI À PLUSIEURS REPRISES, hors labo/PoC.** Confirme en **usage réel prolongé et cross-border** la thèse cœur : un nœud nomade rejoint le réseau **zéro-config** (rendez-vous DHT ADR-010) et **échange par-dessus internet** sans setup manuel. Complète le jalon 17/07 (2 iPhones, session unique, ligne dédiée) par un usage **répété et self-servi par Malik lui-même**. ⚠️ **Non instrumenté ici** (l'infra debug — collecteur :9999, Mac, SSH NAS — est restée au domicile) : le **chemin exact n'est pas capturé** (direct hole-punch vs relais Freebox indéterminé) ; rappel archi : la Freebox = **seul relais public ToM connu de la flotte** (ToM non compatible relais iroh) → elle sert soit de fallback relais soit de correspondant. À réinstrumenter au retour si on veut le détail du chemin. | Preuve d'usage RÉEL du scénario cœur (nomade cross-border), au-delà du PoC/jalon — « la vision marche en vacances » | Attestation utilisateur (Malik, 2026-07-29, vacances Corse) ; cohérent avec jalon 17/07 (2 iPhones cellulaire→direct) + PoC hole-punch 4G CGNAT/cross-border |
```
</details>

### 3. #71 — `.husky/pre-push`
Commits `191c0f1` (règle + témoins + commentaire `ci-local.sh`), `bf3548f`, `8210373750a7db63fde106a0cd6cb76181e7b562` (correctifs issus du doublage). Règle (commentée en français dans le hook) : stdin lu une fois (`PUSH_REFS="$(cat)"`) ; suppression → rien à tester ; base = `remote_sha` ou `merge-base origin/main <local_sha>` pour une ref neuve ; base absente (`cat-file -e`), diff en erreur, liste vide, un chemin hors `vault/`, stdin vide → tout rejouer ; `git -c core.quotePath=false diff --no-renames --name-only` ; test final **en mémoire** (`only_vault_paths` : IFS = saut de ligne, `set -f`, `case vault/*`), sans pipe ni heredoc. Messages `[pre-push] vault/ seul : CI locale sautée` / `[pre-push] code touché : CI locale complète`. Smoke observabilité inchangé. `ci-local.sh` : seul son en-tête est complété (le contrôle de dérive est intact et vert).

Témoins `bash scripts/test-pre-push-vault.sh` (dépôt jetable `mktemp -d`, faux origin local, `ci-local.sh` remplacé par un stub qui écrit un marqueur ; aucun réseau, aucun push) :
```
OK  vault seul → CI sautée
OK  code seul → CI complète
OK  vault + code → CI complète
OK  ref neuve vault seul → CI sautée
OK  suppression de ref → CI sautée
OK  base introuvable → CI complète
OK  refs multiples vault+code → CI complète
OK  suppression + code → CI complète
OK  renommage code → vault/ → CI complète
OK  fichier « vault » sans / → CI complète
OK  liste > 64 Ko avec code → CI complète
OK  liste > 64 Ko vault seul → CI sautée
OK  liste > 64 Ko avec code, E/S bridées → CI complète
OK  aucun changement → CI complète
OK  stdin vide → CI complète
OK  ref neuve sans origin/main → CI complète
TOUS LES TÉMOINS VERTS
```
Aussi vert sous `dash`, et lancé depuis un autre répertoire. Mutants : sans `--no-renames` → KO renommage ; `^vault` sans slash → KO « vault sans / » ; hook `191c0f1` → KO > 64 Ko ; hook `bf3548f` → KO E/S bridées ; grep en erreur (hook `bf3548f`) → rejoue. Hook de base `d8b1cd6` : 16/16 KO (aucun message attendu).

### 4. Doublage N2 et incidents
- Passe 1 (`191c0f1`) : **RESERVE prouvée** — `printf | grep -qv` sous `pipefail` (`/bin/sh` macOS = bash 3.2) : liste > 64 Ko → SIGPIPE 141 → CI sautée avec du code (reproduit par moi : `aaa/code.rs` + 4000 fichiers vault → « vault/ seul »).
- Passe 2 (`bf3548f`, heredoc + `[ $? -eq 1 ]`) : **RESERVE prouvée** — `ulimit -f 1` : le heredoc ne peut créer son fichier temporaire, grep ne tourne pas, code 1 → CI sautée.
- Passe 3 (`8210373`) : **GO**, rapport `vault/revues/2026-10-18-R014-racine-et-pre-push-vault.md` (`verdict: GO`, `tip: 8210373750a7db63fde106a0cd6cb76181e7b562`, `niveau: N2`) ; frontmatter vérifié.
- ⚠️ **Règle « deux correctifs de suite sur la même classe »** : les deux fail-open relèvent de la même classe (la décision de sauter dépendait d'une E/S shell qui, en échouant, produisait le code « pas de code trouvé »). Le mécanisme a été changé (plus aucune E/S dans la décision) plutôt que rapiécé une 3e fois ; à garder en tête pour #74.
- **Incidents push** : push 1 (pointe `191c0f1`) et push 2 (`bf3548f`) **arrêtés volontairement** pendant ci-local (`kill -TERM` du `ci-local.sh`) dès la RESERVE connue — et parce que j'avais modifié `.husky/pre-push` pendant que `sh` l'exécutait (lecture au fil de l'eau → fin non fiable). Les deux ont échoué proprement (`husky - pre-push script failed (code 143)`, `ls-remote` vide). Après chaque incident : `git status/log/reflog` propres. Le push 2 avait fait tourner TS, smoke, protocol, transport, integration sans échec avant l'arrêt. Constat : sur SIGTERM, `ci-local.sh` affiche « TOUT VERT. » (code 143 conservé) → **#73**.
- Remarques non bloquantes → **#74** (`diff.ignoreSubmodules=all` local masque un gitlink ; témoins `Vault/`/`vaultx/` ; sémantique « code puis revert » ; fail-open théorique du job `changes` si grep échoue).

### 5. Preuves terrain
**Push de code** (push 3, pointe `8210373`, nouvelle ref) :
```
[pre-push] code touché : CI locale complète
[ci-local] toolchain : rustc 1.99.1 (58cf8f9c3 2026-10-13) (épinglée par rust-toolchain.toml)
[ci-local] base de comparaison : d8b1cd6 → HEAD 8210373
…
     79s  rust-ffi: build tom-protocol-ffi
     49s  rust-ffi: clippy tom-protocol-ffi
      1s  rust-ffi: header drift
      0s  rust-macos: build tom-relay-ffi
      0s  rust-macos: build tom-protocol-ffi
      9s  rust-stress: build
      2s  rust-stress: clippy
      3s  rust-stress: localhost test (timeout 5 min)
      0s  rust-poc: debug target directory
     41s  rust-poc: build
     17s  rust-poc: clippy
      8s  rust-poc: localhost NAT test (timeout 3 min)
[ci-local] TOUT VERT.
[pre-push] observability smoke skipped (aucun changement relay/discovery/monitoring)
 * [new branch]      chore/racine-et-pre-push-vault -> chore/racine-et-pre-push-vault
GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30' git push -u origin   1273,95s user 175,31s system 221% cpu 10:54,35 total
```
**Push vault seul** (commit R014 `6d00cda`) :
```
[pre-push] vault/ seul : CI locale sautée
[pre-push] observability smoke skipped (aucun changement relay/discovery/monitoring)
   8210373..6d00cda  chore/racine-et-pre-push-vault -> chore/racine-et-pre-push-vault
GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30' git push 2>&1  0,04s user 0,04s system 3% cpu 2,289 total
```
`ls-remote` : `6d00cda253166ab1dde3d768c12d8609d96209db refs/heads/chore/racine-et-pre-push-vault`.

### 6. Fusion
- PR **#75** (`Closes #71`) ; `gh pr merge --auto --merge --delete-branch --match-head-commit 6d00cda…` ; checks PR tous `pass` (dont `changes`, `harnais-ok`, 12 jobs) ; `{"merge":"ca809109b788c7e4bb310b7753abe5214b19e888","mergedAt":"2026-10-18T08:41:43Z","state":"MERGED"}`.
- `gh run list --branch main -L 2` : `completed success … Release … 39112436225` ; `completed success … CI … 39112436205 4m21s`. `gh run watch 39112436205 --exit-status` puis `gh run view` → `{"conclusion":"success","headSha":"ca809109b788…","status":"completed"}`.

### 7. Reprise
- Worktree `/Users/malik/Documents/tom-protocol-scratch-M0015-reprise`, branche `docs/reprise-F01-M0015` depuis `origin/main` = `ca80910` ; `HOOKS_OK`. Commit `fff0e39a69ff23211374e39b968f464da8e81ef0` « docs(vault): reprise F01 M0015 » (`vault/reprise/F01.md` + archive F01 M0012 copiée de la racine).
- 2e push vault seul (ref neuve) :
```
[pre-push] vault/ seul : CI locale sautée
 * [new branch]      docs/reprise-F01-M0015 -> docs/reprise-F01-M0015
GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30' git push -u origin  2>&1  0,05s user 0,05s system 4% cpu 2,177 total
fff0e39a69ff23211374e39b968f464da8e81ef0	refs/heads/docs/reprise-F01-M0015
```
- PR **#76**, auto-merge ; CI : `changes` pass, jobs de test `skipping`, `harnais-ok` pass ; `{"merge":"aaeee846a1c6ed42d1f9d0f6e4f132cd7cc97473","mergedAt":"2026-10-18T08:47:49Z","state":"MERGED"}`.
- `vault/runtime/events.jsonl` (MANDAT_RENDU) et `state.json` (`windows.F01` seul) écrits, non committés.

### 8. Disque et nettoyage
- `target/` du worktree M0015 : 21 Go après le push 3 → `cargo clean` (24,7 GiB libérés) ; worktree reprise : rien à nettoyer. Worktrees **laissés en place** : `/Users/malik/Documents/tom-protocol-scratch-M0015`, `/Users/malik/Documents/tom-protocol-scratch-M0015-reprise`.
- Racine : aucune écriture hors `git fetch`, `vault/echanges/F01.md` et `vault/runtime/` (ni pull, ni checkout ; elle reste en retard sur `main`).
- Dossiers jetables des sous-agents non supprimés (rm refusé par les permissions) : `/var/folders/xz/4d9mvzyd7yz7lzyxs435qhwm0000gn/T/tmp.{c0bOqrBy7T,4zz0uzpuAd,QVB7FzJdOB,xJuUe9FVM5,nc0xZwt2EY,EkrB5sZkL2,GXEidhwwRb}` — sans enjeu (temp système).

M0015
Implementation      ✅ bc973c8, 191c0f1, bf3548f, 8210373
Tests               ✅ test-pre-push-vault.sh 16/16 · ci-local TOUT VERT (push 3) · CI main 39112436205 success
Commit              ✅ 8210373, 6d00cda
Branch push         ✅ chore/racine-et-pre-push-vault 6d00cda
Review              ✅ R014 GO (8210373)
Merge main          ✅ ca80910
Main push           ✅ ca80910 (PR #75, auto-merge)
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED
