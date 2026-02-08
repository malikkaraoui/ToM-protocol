Contribuer

Modèle micro-session

ToM est pensé pour des contributions courtes, focalisées, terminables en 30 à 60 minutes.
Objectif : multiplier le débit de contributions (humains + assistants LLM) sans exploser la complexité.

Niveaux de complexité

- micro : < 30 min (un fichier, doc, JSDoc)
- small : 30–60 min (2–3 fichiers, un test, un fix)
- medium : 1–2 heures (feature ou refactor limité)

Règles simples

- Une PR = un changement logique
- Ajouter des tests quand on touche au comportement
- Respecter la direction des dépendances (demo → sdk → core)
- Garder le protocole invisible côté user

Sources

- Guide complet : ../../CONTRIBUTING.md
- Backlog prêt : ../../.github/ISSUE_BACKLOG.md
- Guide LLM : ../../CLAUDE.md
