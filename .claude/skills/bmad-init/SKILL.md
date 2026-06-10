---
name: bmad-init
description: "Installe BMAD-METHOD dans le projet courant. Réservé aux gros projets nécessitant un cycle complet analyse→plan→architecture→implémentation. Utiliser quand /atelier-setup propose BMAD ou quand l'utilisateur demande /bmad-init."
---

# BMAD Init

Tu installes la méthodologie BMAD dans le projet courant.

**BMAD-METHOD** fournit 6 agents spécialisés avec des personas :

| Agent | Nom | Rôle | Phase |
| --- | --- | --- | --- |
| Analyst | Mary | Business Analyst + Requirements | 1. Analyse |
| PM | John | Product Manager | 2. Planning |
| UX Designer | Sally | UX/UI Design | 2. Planning |
| Architect | Winston | System Architecture | 3. Solutioning |
| Developer | Amelia | Implementation | 4. Implementation |
| Tech Writer | Paige | Documentation | Transverse |

## Avertissement

⚠️ **BMAD est une méthodologie conséquente.** Elle est faite pour des
projets avec :

- Plusieurs semaines/mois de développement
- Besoin d'analyse marché, PRD, architecture formelle
- Plusieurs phases distinctes (analyse → plan → archi → implémentation)
- Artefacts documentaires structurés (product brief, PRD, epics, stories)

**Pour un script, un petit outil, ou un projet solo de quelques jours :
BMAD est du overkill.** Utilise l'atelier standard sans BMAD.

## Procédure

### Étape 1 — Confirmer

"Tu es sûr de vouloir installer BMAD ? C'est une méthodologie
complète avec 6 agents et un cycle de vie structuré.

[OUI] Installer BMAD | [NON] Annuler"

### Étape 2 — Installer

```bash
npx bmad-method install
```

L'installeur BMAD est interactif — il demande :
- Le dossier d'installation
- Les plateformes IA (Claude Code)
- Le nom de l'utilisateur
- La langue
- Le nom du projet

### Étape 3 — Confirmer l'installation

Après installation, vérifier que les dossiers existent :

```bash
ls _bmad/ .claude/skills/bmad-*
```

### Étape 4 — Guide de démarrage

"BMAD installé. Pour commencer :

1. Tape `/bmad-help` pour voir où tu en es
2. Commence par Mary (Business Analyst) : `/bmad-agent-analyst`
3. Elle te guidera vers le product brief, puis le PRD, puis l'architecture

Le cycle complet : Analyse → Planning → Solutioning → Implementation.
Chaque phase produit des artefacts que la suivante consomme.

Repo BMAD : https://github.com/bmad-code-org/BMAD-METHOD"

## Règles

- Ne jamais installer BMAD sans confirmation explicite
- Rappeler que c'est pour les gros projets
- Ne pas modifier les fichiers BMAD après installation
- BMAD et claude-atelier coexistent : les skills atelier restent actifs
