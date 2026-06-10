---
kind: ecosystem
name: skills
loads_from: src/fr/CLAUDE.md (header + §16)
---

# Ecosystem — Skills

> Chargé à la demande. Explique ce qu'est un skill Claude Code, comment
> l'invoquer, quand en créer un, et comment coexister avec les skills
> fournis par des plugins.

## Concept

Un **skill** est un bloc de connaissance/procédure chargé dynamiquement dans
le contexte via l'outil `Skill`. Deux origines :

1. **Skills locaux** : écrits par l'utilisateur dans `~/.claude/skills/` ou
   `.claude/skills/` du projet
2. **Skills bundlés** : fournis par un plugin installé depuis un marketplace

Invoqué via :

- L'outil `Skill` (appelé par Claude quand un match est détecté)
- Une slash command user-facing (`/brainstorming`, `/commit`, etc.)

## Quand utiliser un skill

- **Quand il en existe un pertinent** : la règle « si 1 % de chance qu'un
  skill s'applique, il faut l'invoquer » vient du skill `using-superpowers`.
  À adapter au contexte : ne pas invoquer aveuglément chaque match lexical.
- **Avant une tâche créative ou d'implémentation** : les skills `brainstorming`
  et `test-driven-development` se chargent de la discipline.
- **Avant de toucher à un domaine spécifique** : `frontend-design` pour du UI,
  `systematic-debugging` pour un bug.

## Quand ne PAS utiliser un skill

- **Le skill suggéré n'a aucun rapport avec la tâche réelle** (faux positif
  d'un hook `UserPromptSubmit` qui matche sur un mot-clé). Le signaler et
  continuer.
- **Le skill demanderait plus de cérémonie que la tâche** (ex: invoquer
  `writing-plans` pour renommer une variable).
- **Le skill contredit une règle absolue du §21** (ex: un skill qui
  forcerait à inventer → §5 prime).

## Hygiène

- **Ne jamais masquer un skill non utilisé** : si un skill est suggéré mais
  ignoré, expliquer pourquoi en une phrase.
- **Cumul de skills** : maximum 2-3 simultanés pour un même tour. Au-delà,
  on perd le focus.
- **Skills rigides vs flexibles** : les skills rigides (TDD, debugging) se
  suivent à la lettre ; les flexibles (patterns) s'adaptent au contexte.
  Le skill lui-même l'indique.

## Créer un skill

Format minimal :

```markdown
---
name: mon-skill
description: Description courte utilisee pour detecter si le skill s'applique
---

# Contenu du skill

Procedure, regles, templates, checklist.
```

Placement :

- `~/.claude/skills/<name>.md` pour un skill global
- `.claude/skills/<name>.md` pour un skill propre au projet
- Plugin `skills/<name>.md` si le skill est distribué via marketplace

## Anti-patterns

- Créer un skill pour une règle triviale qui tient dans `CLAUDE.md`
- Écrire un skill qui duplique un skill existant avec un nom différent
- Laisser le harness exécuter un skill sans comprendre ce qu'il fait
