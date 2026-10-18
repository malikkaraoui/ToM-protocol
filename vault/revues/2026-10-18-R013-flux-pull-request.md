---
verdict: GO
tip: 409b47c377b2a17a39fa39d1af60876c5bd5f9fc
niveau: N2
date: 2026-10-18
mandat: M0014
branche: chore/flux-pull-request
base: cbbd554b244b9616fe4af3b72688331c5f45e322
---

# R013 — Flux par pull request (`harnais-ok`, `mode_fusion: pr`)

Doublage N2 par sous-agent frais, lecture seule, sur `cbbd554..409b47c` (3 commits : harnais v1.6.4,
vault en attente, CI). Références fichier:ligne recontrôlées par la session mère.

| # | Point | Résultat | Preuve |
|---|-------|----------|--------|
| 1 | Aucun job perdu | OK | 12 jobs avant → 14 après (+`changes`, +`harnais-ok`) ; comparaison YAML des 12 jobs hors `needs`/`if` ajoutés : aucune différence (commandes, `--test-threads=1`, timeouts, `working-directory`) |
| 2 | `harnais-ok` agrège tout | OK | `needs` = `changes` + 12 jobs, `if: always()` (`ci.yml:401-405`) ; 15 cas simulés : vert seulement si tout `success`, ou tout `skipped` avec `code=false` ; job absent de `needs`, annulé, sauté à tort, `changes` rouge → rouge. Garde : `needs` comparé à la liste des jobs du fichier (`ci.yml:416`) |
| 3 | Job `changes` | OK | bloc `run:` identique octet pour octet au modèle `~/.harnais/canon/templates/ci/ci.yml` ; `code=false` seulement si liste non vide et uniquement `vault/` |
| 4 | Déclencheurs, permissions | OK | `pull_request` sans filtre, `push` main, `merge_group` ; aucun `paths:` ; `permissions: contents: read` suffit aux jobs ; `actions/checkout@v7` existe (tag `v7`, dernière `v7.0.1`) |
| 5 | `scripts/ci-local.sh` | OK | `META_JOBS=(changes harnais-ok)` exclus du seul contrôle de dérive (`ci-local.sh:29,103`) ; dérive toujours détectée si un job est ajouté ou retiré |
| 6 | `config/harnais.json` | OK | JSON valide ; seul ajout sous `git` : `mode_fusion: "pr"`, `fusion_pr: {auto_merge: true, methode: "merge"}` ; lu `pr` par `lire_git` du hook pre-push |
| 7 | Intouchables | OK | aucun de `scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `.claude/settings*.json`, `docs/MISSION.md`, `CLAUDE.md`, `vault/echanges/F0*.md` dans la plage |
| 8 | IP publique | OK | aucun `82.67.95` / `2a01:e0a` dans le diff |

## Remarques (non bloquantes, deviennent des issues)

- **Auto-référence du contrôle** : `harnais-ok` lit le `ci.yml` de la PR. Une PR qui retire un job de
  `jobs:` ET de `needs:` passe verte sans ce job : la protection repose sur la relecture de `ci.yml`.
- `ci-local.sh` : un commentaire en colonne 0 au milieu de `jobs:` tronque la liste (échec sûr : dérive signalée).
