---
kind: orchestration
name: models-routing
loads_from: src/fr/CLAUDE.md §15
replaces: src/fr/orchestration/_legacy.md (partiel)
---

# Orchestration — Routing des modèles

> Chargé à la demande. Définit quel modèle utiliser pour quel rôle.
> Les paramètres de routing sont configurés dans `../../templates/settings.json`.

## Table de routing

| Rôle | Modèle | Justification |
| --- | --- | --- |
| **Session principale** | Sonnet | Bon ratio coût/qualité, assez rapide |
| **Exploration / recherche** | Haiku | Rapide, pas cher, suffisant pour lire + chercher |
| **Implémentation standard** | Sonnet | Qualité code suffisante pour du code de prod |
| **Tests / lint / vérification** | Haiku | Tâche mécanique, pas besoin de raisonnement profond |
| **Architecture critique** | Opus | Raisonnement profond, décisions structurantes |
| **Debug complexe** | Opus (via `/effort high`) | Quand Sonnet tourne en rond sur un bug |
| **Subagent Explore** | Haiku (défaut via `CLAUDE_CODE_SUBAGENT_MODEL`) | Configurable dans settings.json |

## Quand monter vers Opus

- **Décision architecturale irréversible** (choix de stack, structure DB)
- **Bug résistant** : Sonnet a échoué après 2+ tentatives
- **Plan complexe** : > 10 fichiers impactés, interactions non triviales
- **Review critique** : code sécurité, crypto, authentification

## Quand rester sur Haiku

- Exploration codebase (grep glorifié)
- Exécution de tests et lecture de logs
- Lint, format, vérification syntaxique
- Tâches mécaniques répétitives

## Anti-patterns

- **Opus par défaut** : gaspille 10× le budget sans gain sur les tâches simples
- **Haiku pour l'implémentation** : qualité code insuffisante pour du code prod
- **Ne jamais monter vers Opus** : des économies de tokens qui coûtent des heures de debug
- **Ignorer `CLAUDE_CODE_SUBAGENT_MODEL`** : laisser les subagents utiliser le modèle principal au lieu de Haiku

## Configuration

```json
{
  "model": "sonnet",
  "env": {
    "CLAUDE_CODE_SUBAGENT_MODEL": "haiku"
  }
}
```

Voir `../../templates/settings.json` pour la config complète.
