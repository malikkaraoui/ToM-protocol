---
kind: runtime
name: extended-thinking
loads_from: src/fr/CLAUDE.md §18
---

# Runtime — Extended Thinking

> Chargé à la demande. Définit les niveaux de raisonnement étendu et
> quand les invoquer.
>
> Source historique : §18 de `CLAUDE-core.md` (P1).

## Budget par défaut

```text
MAX_THINKING_TOKENS: 10000
```

Défini dans `src/templates/settings.json` (ex §15).

## Niveaux d'effort

| Commande | Budget indicatif | Quand |
|---|---|---|
| `/effort low` | ~2 000 tokens | Tâches simples, lookup, réponse factuelle |
| `/effort medium` | ~10 000 tokens (défaut) | Tâches standard, implémentation classique |
| `/effort high` | ~30 000+ tokens | Architecture critique, debug complexe, decision irréversible |

## Quand monter en `high`

- **Architecture** : choix structurant qui impactera plusieurs features
- **Debug difficile** : bug reproductible mais cause inconnue après une
  première investigation
- **Refactor large** : > 3 fichiers impactés avec interactions
- **Décision irréversible** : migration de schéma DB, rupture de contrat API

## Quand rester en `low`

- Renommage d'une variable
- Ajout d'un champ typé dans une interface existante
- Formatage, lint, correction de typo
- Question factuelle sur le code déjà lu

## Anti-patterns

- Utiliser `high` par défaut « pour être sûr » → gaspillage de tokens
- Utiliser `low` sur un debug complexe → sous-analyse, cause racine ratée
- Changer de niveau en plein milieu d'une réponse (incohérence)
