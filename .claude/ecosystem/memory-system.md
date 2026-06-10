---
kind: ecosystem
name: memory-system
loads_from: src/fr/CLAUDE.md (header)
---

# Ecosystem — Auto Memory System

> Chargé à la demande. Documente le système de mémoire persistante
> fichier-à-fichier fourni par le harness Claude Code.

## Concept

Le harness Claude Code maintient un répertoire local par projet :

```text
~/.claude/projects/<project-slug>/memory/
├── MEMORY.md           ← index plat, charge a chaque session
├── user_*.md           ← memoires de type 'user'
├── feedback_*.md       ← memoires de type 'feedback'
├── project_*.md        ← memoires de type 'project'
└── reference_*.md      ← memoires de type 'reference'
```

Chaque fichier a un frontmatter `name` / `description` / `type` et un corps
court. `MEMORY.md` est un **index** qui liste les fichiers en une ligne chacun.

## Quand écrire une mémoire

| Type | Déclenchement |
| --- | --- |
| `user` | L'utilisateur révèle son rôle, ses responsabilités, son niveau |
| `feedback` | L'utilisateur corrige un comportement OU valide un choix non évident |
| `project` | Décision/motivation non dérivable du code ou du git log |
| `reference` | Pointeur vers un système externe (Linear, Slack, Grafana…) |

## Ce qu'il ne faut PAS écrire en mémoire

- Patterns de code, conventions, architecture → lisibles dans le repo
- Git history, qui a changé quoi → `git log` / `git blame` font foi
- Recettes de fix → le commit message suffit
- Détails éphémères : tâche en cours, état temporaire, contexte de
  conversation

## Discipline de lecture

- **Les mémoires vieillissent.** Avant d'agir sur une mémoire qui nomme
  un fichier ou une fonction, vérifier que l'artefact existe toujours.
- **Pour une question « récente » ou « actuelle »** : préférer `git log`
  ou la lecture du code au recall d'une mémoire — la mémoire est une
  photo figée.
- **Conflit mémoire vs code courant** : le code observé gagne, la mémoire
  est mise à jour ou supprimée.

## Discipline d'écriture

- **Deux étapes** : écrire le fichier détaillé, puis ajouter un pointeur
  d'une ligne dans `MEMORY.md`
- **Ne jamais dupliquer** : vérifier si une mémoire proche existe avant
  d'en créer une nouvelle
- **Structurer les feedbacks/projects** : corps avec **Why:** et
  **How to apply:** — le « pourquoi » permet de juger les cas limites

## Interaction avec §0 CLAUDE.md

Le système de mémoire est **complémentaire** à `§0` :

- `§0` porte l'état courant du projet (phase, stack, endpoints actifs)
- Memoires portent des règles transversales (préférences user, feedback,
  refs externes)

Ne pas dupliquer : si une info doit être dans `§0`, elle ne doit pas être
en mémoire et inversement.

## Anti-patterns

- Écrire en mémoire une info déjà présente dans `§0` ou dans `CLAUDE.md`
- Créer une mémoire de type `project` sans date absolue (les dates
  relatives comme « Jeudi » deviennent inutilisables)
- Laisser une mémoire obsolète polluer les décisions futures
- Citer une mémoire à l'utilisateur comme s'il s'agissait d'une vérité
  absolue sans la vérifier
