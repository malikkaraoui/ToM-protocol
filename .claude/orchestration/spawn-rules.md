---
kind: orchestration
name: spawn-rules
loads_from: src/fr/CLAUDE.md §16
replaces: src/fr/orchestration/_legacy.md (partiel)
---

# Orchestration — Règles de spawn

> Chargé à la demande. Règles pour spawner des agents (Fork, Teammate, Worktree).

## Règles fondamentales

1. **Prompt court et ciblé** : inclure le contexte nécessaire sans
   superflu. Le subagent ne voit **pas** la conversation parente.
2. **5–6 tâches max par agent** : au-delà, l'overhead de coordination
   dépasse le gain.
3. **Fichiers distincts par agent** : chaque agent = ses propres fichiers,
   zéro overlap. Si deux agents touchent le même fichier → conflit garanti.
4. **Nettoyer dès la tâche terminée** : les agents consomment des tokens
   même en idle. Ne pas laisser traîner.
5. **Architecture plate** : un Teammate ne peut pas spawner d'autres
   Teammates. Seul le parent spawne.

## Template de prompt de spawn

```text
Contexte : [description courte du projet et de l'etat actuel]
Tache : [objectif precis en 1-2 phrases]
Fichiers concernes : [liste explicite]
Contraintes : [hors-scope, regles a respecter]
Sortie attendue : [ce que l'agent doit rendre — diff, rapport, code]
```

## Limites pratiques

| Paramètre | Recommandation |
| --- | --- |
| Agents parallèles | ≤ 4 en pratique |
| Tâches par agent | ≤ 5-6 |
| Fichiers par agent | Explicitement listés, non partagés |
| Budget par agent | Proportionnel à la tâche, jamais illimité |
| Modèle subagent | Haiku pour exploration, Sonnet pour implémentation |

## Anti-patterns

- **Prompt vague** : « explore le codebase et dis-moi ce que tu trouves »
  → résultat vague et coûteux
- **Spawn gratuit** : spawner un agent « au cas où » sans tâche claire
- **Fichiers partagés** : deux agents qui touchent le même fichier
- **Chaîne de spawn** : agent A spawne agent B qui spawne agent C →
  explosion de complexité et de coût
- **Agent orphelin** : un agent terminé mais jamais nettoyé, qui consomme
  en idle

## Voir aussi

- `./modes.md` — Fork vs Teammate vs Worktree
- `./subagents.md` — catalogue des subagents disponibles
- `./parallelization.md` — quand paralléliser
