---
name: night-launch
description: "Prépare et vérifie tout pour le night-mode. Utiliser le soir avant de lancer Claude en autonomie."
figure: Veilleur de nuit
---

# Night Launch

> Il est tard. Le Veilleur passe entre les établis, touche du doigt
> le dernier commit, et vérifie que tout est en ordre avant la nuit.

Checklist complète, vérification des prérequis, génération du prompt.

## Procédure

### Étape 1 — Vérification des prérequis

Vérifie silencieusement, puis affiche :

```text
Night Launch — Prérequis
─────────────────────────
[?] .claudeignore en place
[?] settings.json : git push en deny
[?] settings.json : sudo en deny
[?] settings.json : rm -rf en deny
[?] settings.json : maxBudgetUsd défini
[?] scripts/pre-push-gate.sh exécutable
[?] Night Watchdog 🐶 configuré (demander à l'utilisateur)
[?] Specs écrites (fichier docs/specs.md ou équivalent)
```

Si un prérequis manque → bloquer et guider la correction.
Si tout est OK → passer à l'étape 2.

### Étape 2 — Demander les specs

"Où sont les specs pour cette nuit ?
1. `docs/specs.md` (fichier existant)
2. Je vais les écrire maintenant
3. Le prompt suffit, pas besoin de specs détaillées"

Si choix 2 → aider à écrire les specs (Objectif, Contexte, Critère de
réussite, Hors scope). Les sauver dans `docs/specs-night-YYYY-MM-DD.md`.

### Étape 3 — Générer le prompt de lancement

"Voici le prompt à coller dans le terminal :

```bash
claude --permission-mode acceptEdits \
  \"Implementer selon docs/specs.md. \
   Committer chaque etape de facon atomique. \
   Ne pas pusher. \
   Mettre a jour §0 de CLAUDE.md si necessaire. \
   Si tu boucles 3 fois sur le meme probleme, arrete et documente \
   le blocage dans docs/night-blocage-YYYY-MM-DD.md.\"
```

### Étape 4 — Rappels

```text
╔══════════════════════════════════════════════════╗
║  🌙 Night Launch — Rappels                      ║
╠══════════════════════════════════════════════════╣
║  • Watchdog 🐶 te préviendra par iMessage si    ║
║    Claude fige ou demande une permission         ║
║  • git push est bloqué — rien ne part en prod   ║
║  • Le matin :                                    ║
║    1. git log --oneline                          ║
║    2. bash scripts/pre-push-gate.sh              ║
║    3. git push (si gate verte)                   ║
║  • Budget plafonné à maxBudgetUsd                ║
╚══════════════════════════════════════════════════╝

Bonne nuit ! 🌙
```

## Règles

- Ne jamais lancer Claude en autonomie sans que TOUS les prérequis soient OK
- Le prompt inclut toujours "ne pas pusher" et "committer atomique"
- Le prompt inclut un circuit-breaker ("si tu boucles 3 fois, arrête")
- Rappeler le watchdog à chaque fois
