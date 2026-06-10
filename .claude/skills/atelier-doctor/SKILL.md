---
name: atelier-doctor
description: "Diagnostic complet de l'installation claude-atelier (27+ checks). Utiliser pour vérifier que tout est en ordre."
figure: Inspecteur
---

# Atelier Doctor

> L'Inspecteur sort sa grille. 27 points de contrôle. Il ne négocie pas.

Diagnostic complet de l'installation.

## Procédure

1. Exécute `node test/doctor.js` (si dans le repo source) ou le doctor
   embarqué dans le package.
2. Affiche le résultat avec le tableau HEALTHY / UNHEALTHY.
3. Si des erreurs sont détectées → recommander les actions correctives.
4. Si HEALTHY → "Tout est en ordre. Rien à signaler."

Ce skill est un wrapper autour de `test/doctor.js`. Il ajoute la couche
de recommandation que le script seul ne fait pas.

## Checks supplémentaires (au-delà de doctor.js)

- §0 de CLAUDE.md est-il rempli ? (doctor.js ne vérifie pas le contenu)
- Le dernier handoff date de quand ? (> 1 semaine = recommander /review-copilot)
- Le watchdog est-il configuré ? (demander à l'utilisateur)
