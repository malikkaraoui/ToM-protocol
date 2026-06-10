---
stack: react-vite
applies_to: ["vite.config.*", "src/**/*.jsx", "src/**/*.tsx"]
loads_from: src/fr/CLAUDE.md §0 (Contexte projet)
status: stub
figure: Nicolas & Fazia
---

# Stack — React + Vite

> Avant nous, le web c'était du sable mouillé dans une passoire.
> 0.3s de hot reload — t'as pas connu avant, tu peux pas comprendre.
> *— Fast by default. Slow by choice.* — Nicolas & Fazia ⚡

## Périmètre prévu (P3)

- Conventions composants fonctionnels (zéro class component)
- Hooks règles (`eslint-plugin-react-hooks`)
- State management : local d'abord, Zustand / Jotai si justifié
- Structure de dossiers `src/features/` vs `src/components/`
- Vite config : alias, env vars, build targets
- Tests : Vitest + Testing Library
- Accessibilité de base (aria, focus, contraste)
- Perf : memo, lazy, suspense, code splitting
