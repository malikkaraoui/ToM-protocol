---
kind: autonomy
name: loop-watchers
loads_from: src/fr/CLAUDE.md §23
status: canonical
replaces: src/fr/autonomy/_legacy.md + src/fr/orchestration/_legacy.md (dedoublonnage)
---

# Autonomy — Loop watchers

> **Source canonique unique.** Avant P3, `/loop` était documenté aux deux
> endroits (`autonomy/_legacy.md` + `orchestration/_legacy.md`) avec risque
> de drift. Ce fichier est désormais la seule référence.

## Concept

`/loop` est une commande Claude Code qui exécute un prompt (ou une autre
slash command) à intervalle régulier jusqu'à ce qu'on l'annule. Utile
pour du monitoring léger, des polls, ou des tâches récurrentes courtes.

> ⚠️ `/loop` dépend du plugin ou de la version de Claude Code qui le
> fournit (ex: `ralph-loop` dans l'installation courante). Vérifier
> qu'il est disponible avant de s'y appuyer.

## Syntaxe

```bash
/loop <intervalle> <commande ou prompt>
```

Exemples :

```bash
/loop 5m check if deployment is complete
/loop 10m run tests, fix failures if any
/loop 15m /review-pr 1234
/loop 20m git log origin/main..HEAD --oneline
```

L'intervalle accepte `Xs` (secondes), `Xm` (minutes), `Xh` (heures).
Défaut typique : `10m` si omis.

## Quand utiliser `/loop`

- **Monitoring d'un déploiement long** : poll toutes les N minutes jusqu'à
  « deployment complete »
- **Tests qui flaky** : relancer une suite jusqu'à stabilisation
- **Review périodique d'une PR** active
- **Watch d'un dashboard externe** via un script shell

## Quand NE PAS utiliser `/loop`

- **Intervalle < 1 min** : overhead > gain, sature les tokens
- **Tâche qui exécute des actions destructives** : un loop sur `git
  push --force` ou `rm` est suicidaire
- **Remplacement d'un vrai système de monitoring** : pour de la prod,
  utiliser Grafana/Datadog/PagerDuty, pas `/loop`
- **Longue durée sans `maxBudgetUsd`** : chaque tick consomme des tokens

## Discipline

- **Toujours stopper le loop** explicitement quand la tâche est faite
  (ne pas laisser tourner en arrière-plan oublié)
- **Lister les loops actifs dans `§0`** si la session est longue
- **Combiner avec `maxBudgetUsd`** : disjoncteur si ça dérape
- **Prompt de loop court et idempotent** : chaque tick doit être safe à
  rejouer (pas de side-effects qui s'accumulent)

## Anti-patterns

- `/loop 30s /some-heavy-skill` → consomme des milliers de tokens par heure
- `/loop` sur une tâche sans condition d'arrêt claire → tourne indéfiniment
- Plusieurs loops simultanés sur le même sujet → incohérence + gaspillage
- `/loop /plugin install <x>` ou autre commande non idempotente

## Voir aussi

- `./night-mode.md` — pattern de tâche de nuit avec `/loop` éventuel pour
  poll d'état
- `./permission-modes.md` — `/loop` est soumis au mode de permission courant
