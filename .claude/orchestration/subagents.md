---
kind: orchestration
name: subagents
status: new (P3.c)
volatile: true
loads_from: src/fr/CLAUDE.md §16
---

# Orchestration — Catalogue des subagents

> **Satellite nouveau.** L'ancien CLAUDE-core.md ne mentionnait que
> Fork/Teammate/Worktree. Le harness Claude Code expose pourtant un
> catalogue riche de subagents spécialisés. Ce fichier les documente.

## Subagents natifs (outil `Agent`)

Ces subagents sont disponibles via l'outil `Agent` en spécifiant
`subagent_type`. Chacun a ses propres outils disponibles.

### Exploration & recherche

| Subagent | Quand l'utiliser | Coût |
| --- | --- | --- |
| `Explore` | Recherche large dans un codebase, patterns, conventions | Faible (rapide) |
| `general-purpose` | Tâches multi-step autonomes, recherche complexe | Moyen |

- **`Explore`** : rapide, spécialisé. Préciser le niveau de thoroughness :
  `quick`, `medium`, `very thorough`. Ne peut pas éditer de fichiers.
- **`general-purpose`** : polyvalent, tous les outils sauf Agent.
  Pour quand `Explore` ne suffit pas.

### Planification & architecture

| Subagent | Quand l'utiliser | Coût |
| --- | --- | --- |
| `Plan` | Concevoir un plan d'implémentation avant de coder | Moyen |
| `feature-dev:code-architect` | Architecture de feature complète avec blueprint | Élevé |
| `feature-dev:code-explorer` | Analyse en profondeur d'une feature existante | Moyen |

- **`Plan`** : architecte logiciel. Plans step-by-step, fichiers critiques,
  trade-offs. Ne peut pas éditer.
- **`feature-dev:code-architect`** : va plus loin que `Plan` — blueprints
  complets avec fichiers à créer/modifier, composants, data flows.
- **`feature-dev:code-explorer`** : trace les chemins d'exécution, mappe
  l'architecture en couches, documente les dépendances.

### Qualité & review

| Subagent | Quand l'utiliser | Coût |
| --- | --- | --- |
| `feature-dev:code-reviewer` | Review de code avec filtrage par confiance | Moyen |
| `superpowers:code-reviewer` | Review contre un plan + standards | Élevé |
| `code-simplifier:code-simplifier` | Simplifier du code récemment modifié | Moyen |

- **`feature-dev:code-reviewer`** : bugs, sécurité, qualité. Filtrage
  confidence-based → ne rapporte que les vrais problèmes.
- **`superpowers:code-reviewer`** : review contre le plan original +
  standards. Utiliser quand une étape majeure d'un plan est terminée.
- **`code-simplifier`** : refine, simplifie, clarifie. Peut éditer.

### Développement guidé

| Subagent | Quand l'utiliser | Coût |
| --- | --- | --- |
| `feature-dev` (skill) | Développement de feature guidé avec compréhension codebase | Élevé |

- Invoqué via la skill `/feature-dev`, pas directement comme subagent.
  Inclut code-explorer + code-architect + implémentation.

## Subagents de plugins

Les plugins installés exposent des subagents supplémentaires :

- **`vercel-plugin:*`** : `deployment-expert`, `ai-architect`,
  `performance-optimizer` — pour les projets Vercel/Next.js
- **`Notion:*`** : `database-query`, `create-task`, `search` — pour
  l'intégration Notion

> ⚠️ Ces subagents ne sont disponibles que si le plugin est installé
> et actif. Vérifier dans §0 « MCPs actifs ».

## Quand utiliser un subagent vs travailler directement

| Situation | Recommandation |
| --- | --- |
| Recherche dans un gros codebase | `Explore` ou `general-purpose` |
| Question simple, 1 fichier | Pas de subagent, lire directement |
| Plan d'une feature multi-fichiers | `Plan` ou `feature-dev:code-architect` |
| Review après implémentation | `feature-dev:code-reviewer` |
| Simplification post-feature | `code-simplifier` |
| Tâche parallèle indépendante | Fork avec le subagent approprié |
| Tâche avec coordination requise | Teammate (voir `./modes.md`) |

## Discipline

- **Un subagent par tâche** : ne pas surcharger un subagent avec 5 objectifs
- **Prompt complet** : le subagent ne voit pas la conversation parente.
  Inclure tout le contexte nécessaire dans le prompt.
- **Vérifier la sortie** : les résultats du subagent ne sont pas visibles
  par l'utilisateur — Claude doit résumer ce qu'il a appris.
- **Nettoyer** : les agents consomment des tokens même en idle.
  Pas de spawn gratuit.
- **Coût** : `Explore` (rapide, pas cher) ≠ `feature-dev:code-architect`
  (lent, coûteux). Adapter au besoin réel.

## Anti-patterns

- Spawner un `general-purpose` pour lire un fichier (utiliser Read)
- Spawner un `Plan` pour une tâche triviale (< 3 fichiers)
- Utiliser `superpowers:code-reviewer` sans plan à comparer
- Spawner sans briefer (prompt vague = résultat vague)
- Spawner 8 subagents en parallèle sur un budget Pro
