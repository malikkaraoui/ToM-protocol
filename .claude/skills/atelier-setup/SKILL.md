---
name: atelier-setup
description: "Onboarding interactif post-install. Vérifie la config, guide le setup du watchdog et du review-reminder, propose BMAD et QMD. Utiliser après 'npx claude-atelier init' ou quand l'utilisateur dit /atelier-setup."
---

# Atelier Setup

Tu guides l'utilisateur dans la configuration complète de son atelier.
Checklist interactive — chaque point est vérifié puis coché.

## Checklist

Vérifie chaque point dans l'ordre. Pour chaque point :
- Si OK → affiche `[✅]` et passe au suivant
- Si KO → affiche `[❌]` avec les instructions pour corriger, attends
  que l'utilisateur confirme avant de continuer

### 1. Fichiers de base

```text
[?] .claude/CLAUDE.md installé et ≤ 150 lignes
[?] .claude/settings.json avec permissions Bash(*) + deny list
[?] .claudeignore à la racine
[?] .gitignore à la racine
[?] scripts/pre-push-gate.sh installé et exécutable
```

Si un fichier manque → proposer `npx claude-atelier init` ou
`node bin/cli.js init` si dans le repo source.

### 2. Contexte projet (§0)

Lire `.claude/CLAUDE.md` et vérifier que §0 est rempli :

```text
[?] Projet courant ≠ "—"
[?] Stack définie
[?] Repo défini
```

Si §0 est vide → demander à l'utilisateur :
"Quel est le nom du projet ? La stack ? Le repo GitHub ?"
Puis éditer §0 avec les réponses.

### 3. Night Watchdog 🐶

```text
[?] Tâche planifiée "Night Watchdog" configurée dans l'app Claude desktop
```

Impossible à vérifier programmatiquement (c'est dans l'app desktop).
Demander à l'utilisateur :

"As-tu configuré la tâche planifiée **Night Watchdog 🐶** dans l'app
Claude desktop (Programmé → + Nouvelle tâche) ?

Si non, voici le prompt à coller :

```
Tu es un watchdog pour une session Claude Code de nuit.
[... prompt v4 complet ici, voir src/fr/autonomy/night-mode.md ...]
```

Nom: `night-watchdog` | Modèle: Haiku 4.5 | Fréquence: Horaire

Tape [OK] quand c'est fait, ou [SKIP] pour plus tard."

### 4. Review Reminder

```text
[?] Tâche planifiée "Review Reminder" configurée
```

Même approche — demander, fournir le prompt, attendre confirmation.

### 5. BMAD-METHOD (optionnel)

```text
[ ] BMAD-METHOD pour les gros projets
```

"Ce projet est-il un **gros projet** nécessitant un cycle complet
analyse → plan → architecture → implémentation ?

BMAD-METHOD fournit 6 agents spécialisés (analyste, PM, architecte,
dev, UX, tech writer) et un workflow structuré en 4 phases.

⚠️ C'est une méthodologie conséquente. Pour un petit projet ou un
script, ce n'est pas nécessaire.

[OUI] Installer BMAD | [NON] Continuer sans"

Si oui → `npx bmad-method install` dans le projet.

### 6. QMD (optionnel, conditionnel)

Compter les fichiers `.md` dans le projet :
`find . -name '*.md' -not -path './.git/*' -not -path './node_modules/*' | wc -l`

Si < 5 → ne pas proposer, passer.
Si ≥ 5 → proposer :

"Tu as **[N] fichiers markdown** dans ce projet. QMD peut les indexer
pour retrouver du contexte rapidement (plans, bugs, reviews).

[OUI] Installer QMD | [NON] Pas maintenant"

Si oui → guider l'installation de QMD (voir `src/fr/ecosystem/qmd-integration.md`).

### 7. Résumé

Afficher le résumé final :

```text
╔══════════════════════════════════════════════════╗
║  🔧 Atelier configuré !                          ║
╠══════════════════════════════════════════════════╣
║  [✅] Config de base (5/5)                       ║
║  [✅] Contexte projet §0 rempli                  ║
║  [✅] Night Watchdog 🐶 actif                    ║
║  [✅] Review Reminder actif                      ║
║  [—]  BMAD : non installé (petit projet)        ║
║  [—]  QMD : non installé (< 5 fichiers .md)     ║
╠══════════════════════════════════════════════════╣
║  Tape /atelier-help pour voir les commandes.    ║
╚══════════════════════════════════════════════════╝
```

## Règles

- Toujours vérifier avant de corriger
- Ne jamais forcer une installation (proposer, pas imposer)
- BMAD et QMD sont **optionnels** — ne pas insister
- Respecter les choix de l'utilisateur (SKIP est valide)
