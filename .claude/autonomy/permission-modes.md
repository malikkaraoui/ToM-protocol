---
kind: autonomy
name: permission-modes
loads_from: src/fr/CLAUDE.md §23
replaces: src/fr/autonomy/_legacy.md (partiel)
---

# Autonomy — Modes de permission

> Chargé à la demande. Documente les modes de permission disponibles pour
> contrôler l'autonomie de Claude Code lors d'une session.
>
> Config consolidée : `../../templates/settings.json`.

## Table des modes

| Mode | Comportement | Quand l'utiliser |
| --- | --- | --- |
| `default` | Confirmation à chaque action sensible | **Dev interactif** — mode normal |
| `acceptEdits` | Auto-approuve les éditions, demande pour Bash | Tâches longues, mode nuit, Plan Pro |
| `plan` | Plan validé puis exécution libre | Tâches complexes avec contrôle stratégique |
| `auto` | Classifier IA décide | Plans Team / Enterprise uniquement |
| `bypassPermissions` | Aucun garde-fou | Docker / sandbox isolé **exclusivement** |

> **Plan Pro** : `acceptEdits` est le mode autonome disponible sans
> upgrade. `auto` nécessite Team ou Enterprise.

## Règles par mode

### `default`

- Chaque action risquée déclenche une confirmation utilisateur
- Le plus sûr, le plus lent
- Obligatoire pour les opérations sur des systèmes partagés

### `acceptEdits`

- **`allow` / `deny` lists obligatoires** dans `settings.json`
- `maxBudgetUsd` **obligatoire** (disjoncteur automatique)
- `git push` **toujours** en `deny`
- `sudo`, `rm -rf`, `git reset --hard` **toujours** en `deny`
- Détails de la config → `../../templates/settings.json`

### `plan`

- Claude produit un plan détaillé
- L'utilisateur valide le plan avant exécution
- L'exécution est ensuite autonome dans le périmètre du plan validé

### `auto`

- Classifier IA décide quelles actions sont sûres
- **Plan Team/Enterprise uniquement** (sorti le 24 mars 2026 d'après le
  texte legacy — à vérifier si cette date est toujours exacte)
- Non disponible en Plan Pro

### `bypassPermissions`

- **Jamais** sur une machine de dev
- **Uniquement** dans un container/sandbox isolé
- Utilisable pour des expérimentations destructives contrôlées

## Choix du mode par contexte

| Contexte | Mode recommandé |
| --- | --- |
| Dev interactif classique | `default` |
| Implémentation d'un plan validé | `acceptEdits` avec lists strictes |
| Nuit / batch long | `acceptEdits` + `maxBudgetUsd` + night-mode (voir `./night-mode.md`) |
| Refactor risqué multi-fichiers | `plan` d'abord, puis `acceptEdits` dans worktree |
| Expérimentation destructive | `bypassPermissions` dans un sandbox jetable |

## Anti-patterns

- Activer `acceptEdits` sans `maxBudgetUsd` (risque boucle infinie)
- Activer `acceptEdits` sans `.claudeignore` (risque leak de secrets)
- Passer en `bypassPermissions` sur un repo réel
- Utiliser `default` pour une tâche longue et triviale (perte de temps
  sur les confirmations)
