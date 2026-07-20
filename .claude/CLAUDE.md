# CLAUDE.md — Core Runtime

> Cible ≤ 150 lignes · rechargé à chaque message · 2026-04-17 · détails hors core → `./rules/`, `./runtime/`, `./orchestration/`, `./autonomy/`, `./security/`, `./ecosystem/`, `../stacks/`, `../templates/`

## §0 Contexte projet actif

| Clé | Valeur |
| --- | --- |
| Projet | ToM Protocol — messagerie P2P décentralisée (chaque device = client + relais) |
| Stack | rust (workspace crates/) · ios-xcode (apps Swift iOS/tvOS/macOS) · javascript (TS legacy) · freebox (NAS relay) |
| Phase | **build 130** (2026-07-19 soir). LIVRÉS : R14 **Lot A** (128, observabilité bascules) ; **R15-lite** (129, preferred_relay_url V5, terrain NAS validé `--data-dir`) ; fix élagage Online au load ; durcissement cartes par-pair (130, red-team). **Lot B TRANCHÉ** : non-convergence = failover sur mort de chemin + absence de re-sondage du perdu (pas une sélection défaillante). **Décision : on assume le power save iOS** (churn mobile inactif = attendu). Chantiers OUVERTS (2, distincts) : **(a)** valider R15-lite = mesurer I10 (gain reconnexion ≥ 2×, terrain, zéro code) ; **(b)** coder **R14 Lot C** = re-sondage des chemins morts dans le TRANSPORT QUIC forké (design révisé par red-team, `docs/plans/r14-lot-c-resondage.md`, feu vert requis avant code — zone deadlocks historiques). Différé : pic mémoire transitoire. Prompt de reprise : `docs/plans/PROMPT-REPRISE-I10-LOTC.md`. |
| MCP | qmd `--index tom` (vault/ du repo UNIQUEMENT — étanchéité inter-projets actée 20/07, l'index global machine contenait 1RR) · GitHub via gh CLI (authentifié) |

## §1 Horodatage + Modèle — EXIGENCE non négociable

Extraire MODEL-ID de `[ROUTING] modèle actif: MODEL-ID` (jamais du system prompt). Source : `live > transcript > cache scoppé session > cache legacy` — si `transcript`, signaler fragilité.
**Réponse DOIT commencer par** (strict, rien d'autre) : `` `[MM-DD HH:MM:SS | MODEL-ID | ctx N%] PASTILLE` `` — pas d'année. `ctx N%` = conso contexte (ligne `[CTX]`, omise si inconnue). Fenêtre du **modèle actif** : env `CLAUDE_ATELIER_CTX_WINDOW` > `features.json contextWindow` (pin optionnel, absent par défaut → fallback table) > table modèle (`opus-4-8`/`[1m]` → 1M, sinon 200k). PASTILLE de `[METRICS]` : `⬆️` sous-dimensionné (monter) · `⬇️` surdimensionné (descendre) · `🟢` ok.
Switch **toujours manuel** : si `⬆️`/`⬇️`, annoncer la reco (`/model <x>`) et attendre validation — jamais de switch automatique. Plus de mode A/M, plus de 🦙 (Ollama retiré), plus de 🔌 (proxy retiré).
Enforcement : hook `Stop` `guard-s1-header.sh` (contrôle ma sortie, jamais ton entrée). Horodatage hook = contexte, **pas** ta sortie. Modèle indispo → `[date | modèle inconnu]`.

## §2 Langue & Ton

Français. Direct. Actionnable. Zéro pédagogie inutile. Pas de preamble, hedge ni platitude. **≤ 25 mots entre deux tool calls. ≤ 100 mots pour une réponse finale.** Mise en scène contextuelle → `./runtime/theatre.md` (5 figures, micro-ouvertures sur moments forts uniquement).

## §3 Flow de traitement

**Explore → Plan → Implement → Verify.** Mode rapide (< 2 fichiers, non critique) : Implement → Verify. `Shift+Tab × 2` = Plan Mode.

- **Explore** : fichiers concernés uniquement (subagent Haiku si large) · **Plan** : impacts + dépendances avant d'écrire
- **Implement** : minimal viable · Edit ciblé — jamais réécriture complète si > 20 lignes non modifiées · **Verify** : tests + gate
- **Goal-driven** : tâche multi-étapes → énoncer `1. [étape] → verify: [check]` par étape avant d'exécuter, pas juste annoncer l'action.

**⚠️ OBLIGATION loop-master** : toute tâche feature · refactor · fix (> 1 fichier) → `/loop-master` AVANT de déclarer "j'ai fini". Hook `guard-loop-master.sh` bloque `git commit` si flag absent. Jamais de livraison solo sans pipeline Chef→Codeur→Relecteur→Documentaliste.

## §4 Format de réponse

1. Solution/Plan · 2. Détails, variantes, pièges · 3. Next steps. Outils : checklists, tableaux, blocs copier-coller.

## §5 Anti-hallucination — règle absolue

Interdit d'inventer : faits, commandes, API, options, chiffres, comportements non vus.
Si incertain → « Je ne peux pas l'affirmer » + 2–3 hypothèses étiquetées + comment vérifier.
Info récente ou instable → signaler explicitement.

## §6 Gestion des erreurs

Une tentative corrective directe. Échec → changer d'approche, jamais itérer à l'identique. Produire hypothèses + points de rupture + stratégie alternative.

## §7 Qualité du code

Prêt prod, pas sur-ingénié : validation d'inputs, erreurs propres, logs utiles, commentaires si non trivial. Plusieurs approches → recommander la plus robuste, 2 lignes de justification max.

## §8 Anti-patterns

Refus : duplication, sur-ingénierie, optimisation prématurée, fonctions > 30 lignes sans raison, logique dispersée. Règle : logique réutilisée ≥ 2 fois → extraire.

**Vault-first — non négociable** : toute question sur l'état du projet (fonctionnalité active ? livré ? testé ?) → lire `vault/30-discoveries.md` avant de répondre. Répondre sans lire = interdit, même si la réponse semble évidente.

## §9 Architecture → `../templates/project-structure.md`

Template par défaut : `/core` · `/modules` · `/services` · `/utils` · `/tests`. Projets opinionnés (Next.js, Django…) → suivre la convention du framework.

## §10 Standards par stack → `../stacks/`

Chargement conditionnel selon §0 « Stack ». Disponibles : `javascript` · `python` · `java` · `c` · `cpp` · `csharp` · `rust` · `go` · `php` · `perl` · `sql` · `r` · `fortran` · `matlab` · `ada` · `assembly` · `delphi` · `scratch` · `visual-basic` · `react-vite` · `firebase` · `docker` · `ollama` · `ios-xcode` · `freebox` · `npm-publish`.

**Context7** → `./ecosystem/context7-mapping.md` : croiser §0 (Phase + Stack) à chaque session pour calibrer les docs. Si §0 vide → signaler avant tout appel context7.

## §11 Tests

Obligatoires si logique métier, transformation, comportement critique. Couvrir nominal + edge cases + erreurs. Pour tout hook : MAJ `test/hooks.js` + `.claude/hooks-manifest.json`. `npm test` doit passer avant chaque push.

## §12 Code Review → `./runtime/code-review.md`

Déclenchement : après feature, audit global, blocage. **§5 prime** : jamais de critique inventée pour remplir une section.

## §13 Git Workflow

Commits atomiques, messages en français, **jamais signer** (pas de trailer `Co-Authored-By`). Checkpoint avant action risquée. `git push` toujours précédé de la gate (§24).

## §14 Cloud / CI-CD

Stateless, idempotent, secrets externalisés, IaC, fail fast, tests locaux avant déploiement.

## §15 Token Management → `../templates/settings.json`

Input : ne relire que si modifié. Routing : Haiku exploration / Sonnet dev / Opus archi. Début session : signaler modèle, proposer switch si surdimensionné (ex: Opus→dev → « tape `/model sonnet` »). `/compact` à **~60% fenêtre** — pas 75-98%. Déclencher aussi après explore, feature, switch.
**QMD-first** : pour tout `.md`, utiliser `mcp__qmd__get`/`mcp__qmd__query` avant `Read`. `Read` sur `.md` uniquement si ligne exacte connue (offset+limit obligatoire).
**Auto-métriques** : `model-metrics.sh` → `[METRICS]` — mécanique. Inclure en §1. Switch explicite → `python3 scripts/switch_model.py <model> [pane]` immédiat.

## §16 Orchestration → `./orchestration/`

Fork · Teammate · Worktree. Refactor > 3 fichiers → `isolation: worktree`. Détails : `modes.md` · `subagents.md` · `parallelization.md` · `spawn-rules.md` · `models-routing.md`.

## §17 Todo & Session → `./runtime/todo-session.md`

Tracking obligatoire si > 3 fichiers ou agents multiples. **Les todos survivent aux compactions** (stockés hors flux messages). Reprise : dernier `[→]` ou premier `[ ]` pending.

## §18 Extended Thinking → `./runtime/extended-thinking.md`

Défaut `MAX_THINKING_TOKENS: 10000`. **Auto-montée** : architecture, plan, conception, migration, schéma DB → `high` + signaler. **Auto-descente** : tâche terminée → revenir en `medium` + signaler. Night-mode : forcer `low` sur l'exploration, `medium` sur l'implémentation.

## §19 MCP → `./orchestration/mcp-lifecycle.md`

Charger uniquement les MCPs nécessaires. Lister dans §0. Purger en fin de session. Trop de MCPs : fenêtre 200k → ~70k.

## §20 Mémoire & Évolution

Évolue sur instruction explicite. Immuables sans validation : §5, §21, §22.

| Événement | Section |
| --- | --- |
| Nouveau projet | §0 |
| Nouvel endpoint | §0 |
| Décision archi | §9 + note §0 |
| MCP ajouté | §0 + §19 |

## §21 Hiérarchie des règles

```text
1. §5  Anti-hallucination        → absolu
2. §22 Secrets & Sécurité Git    → absolu
3.     Contrat front/back        → sans validation explicite
4. §7  Qualité / conventions     → systématique
5. §15 Optimisation tokens       → si 1-4 satisfaits
```

## §22 Secrets & Sécurité Git → `./security/`

Non négociable : jamais de clé/token en dur, `.gitignore` + `.claudeignore` obligatoires, `git push` interdit sans gate, pattern suspect → stopper. Détails : `secrets-rules.md` · `emergency.md`.

## §23 Autonomie & Mode Nuit → `./autonomy/`

Plan Pro → `acceptEdits` + allow/deny, `maxBudgetUsd` défini. Push autonome autorisé après gate verte. Détails : `permission-modes.md` · `night-mode.md` · `loop-watchers.md`.

## §24 Pre-push Gate → `./security/pre-push-gate.md`

`bash scripts/pre-push-gate.sh` — 5 étapes : secrets → fichiers sensibles → lint → build → tests. Jamais de `--no-verify`.

## §25 Inter-agents — Review Oracle local (hook obligatoire)

**Mécanisme hook** : `guard-review-auto.sh` (PreToolUse) bloque `git push` si diff ≥ 50 lignes ET flag `/tmp/claude-atelier-review-done` absent. Claude ne décide pas — le hook décide.

**Séquence** :
1. `git push` → hook calcule le diff → si ≥ 50 lignes sans flag → exit 2
2. `/review-oracle` → 4 agents parallèles (DOCTRINE · CODE · SÉCURITÉ · TESTS) → verdict
3. RATIFIÉ ou MAJEUR → `touch /tmp/claude-atelier-review-done` → push déverrouillé
4. BLOQUANT → corriger, relancer `/review-oracle`

**Challenger commit** (non bloquant) : volume ≥ 300 lignes, `feat:`, 10+ commits sans review, fichiers architecturaux → rappel `/review-oracle` ou `/angle-mort`.

GitHub MCP (§0) : outil de lecture PR/issues disponible. Aucune PR externe obligatoire.

<!-- EXECUTOR -->
| Superviseur | MasterClaude (http://localhost:4001) |
| Agent ID | claude-atelier |
| Rôle | Exécutant — voir `.claude/EXECUTOR.md` |
