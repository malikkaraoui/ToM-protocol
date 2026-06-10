---
name: token-routing
description: "Explique et configure le routing Haiku/Sonnet/Opus pour économiser les tokens. Utiliser quand l'utilisateur s'inquiète du coût ou veut optimiser."
figure: Intendant
---

# Token Routing

> L'Intendant lève un sourcil. Opus pour un grep ? Il sort la table
> de routing et pointe du doigt la bonne ligne.

Assigner le bon modèle au bon rôle. Le but : ne pas brûler le budget.

## Table de routing

| Rôle | Modèle | Pourquoi |
| --- | --- | --- |
| **Session principale** | Sonnet | Bon ratio coût/qualité |
| **Subagents Explore** | Haiku | Rapide, pas cher, suffisant pour lire |
| **Tests / lint** | Haiku | Tâche mécanique |
| **Architecture critique** | Opus | Raisonnement profond |
| **Debug bloquant** | Opus (via `/effort high`) | Quand Sonnet tourne en rond |

## Configuration actuelle

Vérifie `.claude/settings.json` :

```json
{
  "model": "sonnet",
  "env": {
    "CLAUDE_CODE_SUBAGENT_MODEL": "haiku",
    "MAX_THINKING_TOKENS": "10000"
  }
}
```

## Le problème que ça résout

Sans routing, une session de nuit peut consommer **10× plus de tokens**
qu'une session routée :

- Opus pour explorer 50 fichiers = 💸💸💸
- Haiku pour explorer 50 fichiers = 💰 (50× moins cher)

Le routing n'est pas de l'optimisation prématurée. C'est de la
**survie budgétaire** en night-mode.

## Cycle montée / descente automatique

```text
low ──────── medium ──────── high
 ↑  auto-descente  ↑  auto-montée  ↑
 │                 │                │
 │  rename, typo   │  implémentation│  architecture
 │  lookup, grep   │  bug fix       │  plan, conception
 │  format, lint   │  feature       │  migration DB
 │                 │  tests         │  debug bloquant
 │  ~2k tokens     │  ~10k tokens   │  ~30k+ tokens
```

**Montée** : Claude détecte un champ lexical complexe → **vérifie le
niveau actuel d'abord**. Si déjà en `high` ou `max` → ne rien dire.
Sinon → monte et signale « Je monte en effort high pour cette tâche. »

**Descente** : la tâche complexe est terminée → redescend et signale
« Tâche architecturale terminée, je redescends en effort medium. »

**Night-mode** : forcer `low` pour l'exploration (grep, read),
`medium` pour l'implémentation. Jamais `high` sauf bug bloquant
(le but premier : ne pas brûler tous les tokens en une nuit).

## Changer de modèle en cours de session

La commande `/model` fonctionne mid-session dans Claude Code :

```
/model haiku    # passer en Haiku (exploration, watchdog)
/model sonnet   # passer en Sonnet (dev standard)
/model opus     # passer en Opus (architecture, debug bloquant)
```

Claude signale si le modèle actif est surdimensionné pour la tâche en
cours et suggère la commande exacte.

## Action

Si la config n'est pas en place → proposer de l'ajouter dans
settings.json. Si elle est en place → afficher la table et confirmer
que tout est configuré.
