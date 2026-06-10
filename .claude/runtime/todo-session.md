---
kind: runtime
name: todo-session
loads_from: src/fr/CLAUDE.md §17
---

# Runtime — Todo & Session

> Chargé à la demande. Définit quand et comment utiliser le système de
> tâches (TodoWrite) et comment reprendre une session interrompue.
>
> Source historique : §17 de `CLAUDE-core.md` (P1).

## Quand utiliser le tracking de tâches

**Obligatoire si :**

- Plus de 3 fichiers impactés dans la même opération
- Plusieurs agents ou subagents spawnés
- Tâche qui traverse plusieurs phases (explore → plan → implement → verify)
- Reprise d'une session antérieure avec état partiel

**Pas nécessaire si :**

- Une seule édition localisée (< 2 fichiers)
- Question informationnelle, explication, review sans modification
- Enchaînement de 2–3 commandes triviales

## Pattern de statuts

```text
[ ] Analyser     ← pending
[→] Implémenter  ← in_progress (UN SEUL a la fois)
[✓] Tester       ← completed
[ ] Commit       ← pending
```

## Discipline d'utilisation

- **Exactement une tâche `in_progress`** à tout instant
- **Marquer `completed` immédiatement** après avoir fini (pas de batch)
- **Jamais `completed` si :** tests échouent, implémentation partielle,
  erreurs non résolues
- **Supprimer** les tâches devenues sans objet (pas de liste zombie)

## Reprise de session

```text
1. Lire l'état de la todo list (TodoRead ou équivalent)
2. Identifier le dernier [→] ou le premier [ ] pending
3. Reprendre a ce point, sans rejouer ce qui est deja [✓]
4. Si ambiguite -> demander confirmation a l'utilisateur avant d'agir
```

## Anti-patterns

- Créer une todo list pour une tâche triviale (< 3 étapes)
- Batch des `completed` en fin de session (perte de granularité)
- Laisser plusieurs tâches `in_progress` en parallèle
- Dupliquer une tâche au lieu de la modifier
