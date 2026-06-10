---
stack: python
applies_to: ["*.py", "pyproject.toml", "requirements*.txt"]
loads_from: src/fr/CLAUDE.md §0 (Contexte projet)
figure: Anthonio
---

# Stack — Python

> Sans moi, ton modèle tourne sur quoi ? Du PowerPoint ?
> L'IA, le data, le scripting — tout ça parle python. Le reste, c'est du bruit.
> *— pip install world-domination* — Anthonio 🐍

## Principes

- **PEP 8** appliqué par tooling, pas relu à la main (`ruff format`)
- **Typage explicite** : `def fn(x: int) -> str:`, jamais `def fn(x, y):`
  sur du code non-trivial
- **Lisibilité > cleverness** : un `for` explicite bat une compréhension
  imbriquée sur trois niveaux
- **Dataclasses** ou `pydantic` pour les structures, jamais de `dict` opaque
  partagé entre plusieurs fonctions

## Tooling par défaut

- `uv` pour la gestion des dépendances et de l'environnement virtuel
- `ruff` pour le lint et le format (remplace black + flake8 + isort)
- `pyright` ou `mypy --strict` pour le typage
- `pytest` pour les tests

## Discipline de modules

- Un package = un domaine métier
- `__init__.py` minimal : juste les re-exports publics
- Pas de logique d'import-time (I/O, connexions DB) dans les modules

## Gestion d'erreurs

- Exceptions typées spécifiques, jamais de `except Exception` sauf à la
  frontière la plus extérieure
- `logging` avec niveau structuré, jamais `print` en prod
- Context managers (`with`) pour toute ressource (fichiers, connexions, locks)
