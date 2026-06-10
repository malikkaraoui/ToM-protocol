---
name: angle-mort
description: "Review anti-complaisance avant release. Cherche le fragile, le manquant, le faux-confort. Ne félicite pas, n'embellit pas, n'arrondit pas."
figure: Inspecteur
---

# Angle Mort — Le miroir dur

> La convocation de l'Inspecteur.
> Quand la feature te paraît "finie" trop vite, quand la release approche,
> quand tu veux qu'un autre regard casse le confort : tu appelles `/angle-mort`.

Review anti-complaisance. La question est toujours la même :
**"Qu'est-ce que je ne vois pas parce que je suis trop dedans ?"**

## Procédure

Identique à `/review-copilot` sauf :

1. Pas de choix de sujet — la question est toujours la même
2. La section "Question précise" du handoff est :

```text
Quels sont les angles morts, incohérences ou faiblesses que tu
détectes ? Focus sur ce qui MANQUE ou ce qui est FRAGILE — pas sur
ce qui est déjà bien. Sois direct et technique.
```

3. Les contraintes sont :

```text
- Ne pas proposer de réécrire ce qui fonctionne
- Ne pas proposer de changements cosmétiques
- Se concentrer sur les risques réels (sécurité, fiabilité, UX cassée)
- Tout ce que l'auteur ne voit plus parce qu'il est trop dedans
```

Le reste (collecte contexte, fichiers à lire, prompt copier-coller,
commit) est identique à `/review-copilot`.
