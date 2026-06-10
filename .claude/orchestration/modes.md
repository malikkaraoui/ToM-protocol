---
kind: orchestration
name: modes
loads_from: src/fr/CLAUDE.md §16
replaces: src/fr/orchestration/_legacy.md (partiel)
---

# Orchestration — Modes d'isolation

> Chargé à la demande. Documente les 3 modes d'isolation disponibles
> pour faire travailler Claude en parallèle ou en isolation.

## Les 3 modes

| Mode | Isolation | Communication | Usage |
| --- | --- | --- | --- |
| **Fork** (subagent) | Contexte propre | Résultat → parent uniquement | Exploration, tâche ciblée |
| **Teammate** (Agent Teams) | Contexte + mailbox | Peer-to-peer entre agents | Coordination temps réel |
| **Worktree** | Contexte + branche git isolée | Via commits / merge | Refactor risqué, feature parallèle |

## Fork (subagent)

- Claude spawne un sous-agent avec un contexte restreint
- Le sous-agent exécute sa tâche et renvoie son résultat au parent
- **Pas de communication bidirectionnelle** : le parent reçoit, c'est tout
- Idéal pour les tâches isolées : exploration codebase, recherche, lint

## Teammate (Agent Teams)

- Plusieurs agents travaillent en parallèle avec une mailbox partagée
- Communication peer-to-peer (un agent peut envoyer un message à un autre)
- Nécessite l'activation explicite :

```json
{ "env": { "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1" } }
```

- **Worktree est le seul mode où les fichiers sont réellement isolés.**
  Les Teammates partagent le même filesystem — répartir les fichiers
  pour éviter les conflits d'écriture.

## Worktree

- Git worktree = branche isolée dans un répertoire séparé
- Les modifications ne touchent pas la branche courante tant que le merge
  n'est pas fait
- Activé via `isolation: worktree` dans le prompt d'Agent
- **Règle absolue** : refactor > 3 fichiers → toujours `isolation: worktree`

## Choix du mode

| Situation | Mode recommandé |
| --- | --- |
| Exploration codebase large | Fork — modèle Haiku |
| Feature + tests + doc simultanés | Teammate × 3 |
| Audit sécurité + perf en parallèle | Teammate × 2 |
| Refactor multi-fichiers risqué | Worktree |
| Tâches indépendantes sans coordination | Fork parallèles |
| Tâche < 2 min ou triviale | Pas de parallélisation |

## Anti-patterns

- Utiliser Teammate quand Fork suffit (overhead coordination inutile)
- Faire travailler deux Teammates sur les mêmes fichiers (conflits)
- Oublier `isolation: worktree` sur un refactor large (modifications
  sur la branche courante, risque de casse)
- Spawner 10 agents en parallèle avec un budget token serré

## Voir aussi

- `./spawn-rules.md` — règles de spawn (prompt, limites, nettoyage)
- `./parallelization.md` — quand paralléliser et quand ne pas
- `./subagents.md` — catalogue des subagents disponibles
