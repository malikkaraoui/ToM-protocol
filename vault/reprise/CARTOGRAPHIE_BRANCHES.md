---
date: AAAA-MM-JJ
tags: [cartographie, branches, worktrees]
genere_par: <Fxx> <Mxxxx|R0xx> (lecture git réelle : fetch --prune, for-each-ref, worktree list, branch -vv, branch --merged/--no-merged origin/<principale>, rev-list ahead/behind)
derniere_maj: AAAA-MM-JJTHH:MM:SS+00:00
---

# Cartographie branches / worktrees vs `<principale>`

> **Ce fichier se régénère par LECTURE GIT RÉELLE, jamais de mémoire.** Une cartographie
> recopiée depuis la version précédente ou reconstituée « de tête » est un faux document : elle
> a l'apparence d'une preuve et la valeur d'un souvenir. Chaque régénération remplace
> INTÉGRALEMENT la version antérieure, et `genere_par:` dit par quel mandat et avec quelles
> commandes. Une ligne que la lecture git ne confirme pas ne figure pas dans le tableau.

Commandes de régénération (à rejouer dans cet ordre, sortie à la source du tableau) :

```
git fetch --prune
git for-each-ref --format='%(refname:short) %(objectname:short)' refs/heads
git worktree list
git branch -vv
git branch --merged origin/<principale> ; git branch --no-merged origin/<principale>
git rev-list --left-right --count origin/<principale>...<branche>
```

`<principale>` = `origin/<principale>` = `<sha>` au moment de la lecture.
Ahead/behind = `git rev-list --count` contre `origin/<principale>`.

## Branches actives / récentes

| Branche | Tip | vs origin/<principale> | Statut RÉEL |
|---|---|---|---|
| `<branche>` | `<sha>` | ahead <n> / behind <n> | **<MERGÉE / NON MERGÉE>** — <ce qu'elle porte, quel mandat, quel doublage l'attend ou l'a passée, quel worktree lui est attaché> |

<!--
« Statut RÉEL » = ce que git dit, pas ce que le rapport annonçait :
- MERGÉE  → le commit de merge existe sur la principale (donner son SHA).
- NON MERGÉE → dire ce qui bloque (doublage attendu, verdict NO-GO, arbitrage humain).
Un « ahead 0 / behind n » n'est PAS une preuve de merge : le contenu peut avoir été repris
autrement. Vérifier, puis écrire.
-->

## Branches anciennes, sans activité récente (non mergées, non arbitrées)

<!-- Liste courte, avec tip et date, sans commentaire : c'est une dette à arbitrer, pas un sujet. -->

## Branches distantes supprimées (`gone`, locales orphelines)

<!-- Issues de `git branch -vv` (marqueur `gone`). Ne rien supprimer ici : signaler seulement. -->

## Worktrees

| Worktree | Branche | À retirer ? |
|---|---|---|
| `<chemin>` | `<branche>` | <oui/non + pourquoi> |
