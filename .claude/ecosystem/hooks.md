---
kind: ecosystem
name: hooks
loads_from: src/fr/CLAUDE.md (header)
volatile: true
---

# Ecosystem — Hooks

> Chargé à la demande. Documente le système de hooks Claude Code :
> quels événements sont hookables, quand les utiliser, et pièges à éviter.

## Concept

Un **hook** est une commande shell exécutée par le harness Claude Code
en réponse à un événement. Configuré dans `settings.json` sous la clé
`"hooks"`. Le hook peut injecter du contexte dans la conversation ou
bloquer une action.

Les hooks sont exécutés par **le harness**, pas par Claude lui-même.
C'est la différence fondamentale : si tu veux qu'un comportement soit
automatique et déterministe, il faut un hook. Si tu veux qu'il soit
modulé par le raisonnement, il faut une règle dans CLAUDE.md.

## Événements hookables (non exhaustif)

| Événement | Déclenchement | Usage typique |
| --- | --- | --- |
| `SessionStart` | Début de session Claude | Injecter date, contexte projet, état repo |
| `UserPromptSubmit` | Avant traitement d'un message user | Ajouter du contexte, valider, rejeter |
| `PreToolUse` | Avant exécution d'un outil | Valider les arguments, bloquer certains outils |
| `PostToolUse` | Après exécution d'un outil | Lint, vérifications, injection de feedback |
| `Stop` | Fin de réponse de Claude | Notifier, logger, déclencher un hook externe |

> ⚠️ Les noms exacts des événements peuvent évoluer. Vérifier la doc
> Claude Code officielle avant de câbler un hook.

## Quand utiliser un hook

- **Comportement automatique non négociable** : injection d'horodatage,
  rappel de règles projet, vérification de secrets avant commit
- **Validation déterministe** : rejeter un outil interdit peu importe le
  raisonnement de Claude
- **Outillage externe** : déclencher un build/test/lint après une édition

## Quand NE PAS utiliser un hook

- **Comportement qui demande du jugement** : un hook n'a pas de contexte,
  il ne « comprend » pas — c'est du scripting. Pour du jugement, utiliser
  une règle dans CLAUDE.md ou un skill.
- **Règle qui change souvent** : un hook est figé dans `settings.json`,
  une règle dans CLAUDE.md se met à jour en un commit
- **Hook qui dépend d'un état de session volatil** : les hooks n'ont pas
  accès à l'historique complet de la conversation

## Pièges fréquents

- **Faux positifs lexicaux** : un hook `UserPromptSubmit` qui matche sur
  un mot-clé peut injecter des suggestions hors-sujet. Toujours les
  filtrer explicitement côté Claude (les signaler et les ignorer).
- **Saturation de la fenêtre** : un hook `SessionStart` qui injecte 10 KB
  de contexte à chaque session consomme des tokens avant même le premier
  message.
- **Hook silencieux qui bloque** : un `PreToolUse` qui rejette un outil
  sans message clair fait perdre du temps en debugging.
- **Hooks en chaîne** : plusieurs hooks sur le même événement s'accumulent
  sans coordination. Tester leur ordre d'exécution.

## Bonnes pratiques

- **Un hook, une responsabilité** : pas de script shell de 200 lignes
- **Exit codes explicites** : `0` = ok, `1` = erreur douce, `2+` = bloquant
- **Logs visibles** : écrire sur `stderr` si tu veux que Claude le voie
- **Idempotence** : le hook peut être rejoué plusieurs fois par session
- **Versionner le hook avec le projet** : `scripts/` ou `hooks/` du repo,
  pas `~/.claude/` global

## Cas d'usage claude-atelier

En P4, `bin/cli.js init` installera :

- `hooks/session-start.sh` — injecte la date + lit `§0` du projet
- `hooks/user-prompt-submit.sh` — validation custom

Ces scripts seront livrés dans le package NPM et copiés dans `.claude/hooks/`
du projet cible avec les chemins configurés dans `settings.json`.
