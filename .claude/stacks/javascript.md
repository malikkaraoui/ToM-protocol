---
stack: javascript
applies_to: ["*.js", "*.jsx", "*.ts", "*.tsx", "*.mjs", "*.cjs"]
loads_from: src/fr/CLAUDE.md §0 (Contexte projet)
figure: Nael
---

# Stack — JavaScript / TypeScript

> Un `;` oublié. Un `any` glissé. Une promesse non catchée.
> Pour Nael, c'est pas un détail — c'est une faute. Et il le fait savoir.
> *— Le compilateur ne pardonne pas. Moi non plus.* — Nael 🔷

## Principes

- **Fonctions pures** par défaut, side-effects isolés en frontière
- **Immutabilité** : préférer `const` + nouvelles copies à la mutation
- **Typage strict** : zéro `any`, zéro `as unknown as X`, zéro `@ts-ignore`
  sans commentaire justifiant la dette technique
- `map` / `filter` / `reduce` > boucles impératives **quand** la lisibilité
  est au moins égale ; pas de chaînes illisibles pour le principe
- **Zéro `var`**, `let` uniquement si réassignation indispensable

## Discipline de modules

- Un module = une responsabilité
- Exports nommés > export default (refactor-friendly)
- Chemins relatifs courts uniquement ; chemins longs → alias (`@/`)

## Qualité runtime

- Jamais de `console.log` laissé en production → logger structuré
- Erreurs typées : jamais `throw "string"`, toujours une `Error` ou sous-classe
- `Promise.all` pour le parallélisme légitime, `for await` pour le séquentiel
- Gestion d'erreurs : `try/catch` **à la frontière**, pas à chaque ligne

## Tests

- Unitaires : Vitest ou Jest selon ce que la stack impose
- Couverture obligatoire : logique métier, transformations, edge cases
- E2E : Playwright pour les flows critiques uniquement
