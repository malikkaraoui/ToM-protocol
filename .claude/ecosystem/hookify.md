---
kind: ecosystem
name: hookify
description: "Transformer une erreur répétée en hook d'enforcement. Le processus pour ne jamais répéter deux fois la même erreur."
---

# Hookify — De l'erreur au rail

> Une règle dans CLAUDE.md est une intention.
> Un hook est un rail.
> Hookify, c'est transformer l'une en l'autre.

## Le principe

Chaque fois qu'une règle est violée malgré le CLAUDE.md, la question
est : peut-on l'enforcer par un mécanisme système ?

```text
Erreur observée
      ↓
Est-ce répétable ? (pattern, pas cas isolé)
      ↓
Peut-on détecter la condition en bash ?
      ↓  oui
Quel hook ? PreToolUse / PostToolUse / UserPromptSubmit
      ↓
Écrire le script → brancher dans settings.json → tester
      ↓
Rail posé. L'erreur ne peut plus se reproduire.
```

## Quand hookifier

| Signal | Action |
| --- | --- |
| Même erreur > 2 fois | Hookifier |
| Règle dans CLAUDE.md jamais respectée | Hookifier |
| "Je pensais que Claude allait..." | Hookifier |
| Fix manuel répété | Hookifier |

## Ce qui n'est PAS hookifiable

| Règle | Pourquoi |
| --- | --- |
| Anti-hallucination | Jugement sur le contenu généré |
| Qualité du code | Jugement subjectif |
| Anti-patterns | Contexte-dépendant |

## Hooks disponibles dans Claude Code

| Hook | Quand | Usage typique |
| --- | --- | --- |
| `PreToolUse` | Avant l'exécution d'un outil | Bloquer une commande dangereuse |
| `PostToolUse` | Après l'exécution | Détecter un résultat problématique |
| `UserPromptSubmit` | À chaque message | Injecter du contexte, détecter des patterns |
| `SessionStart` | Au démarrage | Diagnostic initial |
| `Stop` | Quand Claude s'arrête | Vérification finale |

## Template de hook

```bash
#!/bin/bash
# [NOM] — §[numéro] : [description de la règle enforcée]
# Type: PreToolUse | PostToolUse | UserPromptSubmit

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | grep -o '"command"[[:space:]]*:[[:space:]]*"[^"]*"' \
  | head -1 | sed 's/"command"[[:space:]]*:[[:space:]]*"//;s/"$//')

# Condition de déclenchement
if echo "$COMMAND" | grep -qi "[pattern]"; then
  # Vérification
  if [condition_problème]; then
    echo "[BLOCKED] §[X] : [message clair à Claude]"
    exit 2  # exit 2 = bloquant (PreToolUse) ou signal (PostToolUse)
  fi
fi
```

## Brancher dans settings.json

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash hooks/[nom-du-hook].sh",
            "if": "Bash([pattern]*)"
          }
        ]
      }
    ]
  }
}
```

## Hookify en pratique — exemples posés dans l'atelier

| Erreur originelle | Hook créé |
| --- | --- |
| Claude signait les commits | `guard-no-sign.sh` PreToolUse |
| Claude écrivait en anglais dans les commits | `guard-commit-french.sh` PreToolUse |
| Claude pushait sans tests | `guard-tests-before-push.sh` PreToolUse |
| Claude tournait sur Opus pour du dev | `routing-check.sh` UserPromptSubmit |
| Claude bouclait 3× sur la même erreur | `guard-anti-loop.sh` PostToolUse |
| Claude oubliait de proposer /review-copilot | `guard-review-auto.sh` PostToolUse |
| Stack iOS non chargée automatiquement | `routing-check.sh` détection mots-clés |

## Commande

```
/hookify [description de l'erreur observée]
```

Claude analyse, propose le hook, l'écrit, le branche.
