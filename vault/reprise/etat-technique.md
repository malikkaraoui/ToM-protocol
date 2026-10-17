---
date: AAAA-MM-JJ
tags: [reprise, etat-technique]
maintenu_par: les mandats (chacun REMPLACE sa ligne)
derniere_maj: AAAA-MM-JJTHH:MM:SS+00:00
---

# État technique — lu à la demande

<!--
Ce fichier n'est PAS chargé d'office : on l'ouvre quand on a besoin d'un chiffre, d'une date ou
d'une version. C'est ici que va tout ce qui n'a pas sa place dans un fichier chargé d'office
(`CLAUDE.md`, rules, `SKILL.md`) : compteurs, mesures, « mesuré par Mxxxx », dates, versions.
Un fichier chargé d'office dit la RÈGLE ; celui-ci dit l'ÉTAT.

Règles d'écriture :
- Un fait par ligne, dans la table de sa section. Un fait qui change REMPLACE sa ligne : la valeur
  précédente vit dans git et dans les rapports, pas ici.
- Chaque ligne porte sa source (mandat, rapport) et la commande qui la rejoue : un chiffre sans
  source est un souvenir.
- C'est une DÉCLARATION, pas une preuve : en cas d'écart avec la commande rejouée, ce fichier a tort.
- Ce qui n'est plus vrai se supprime. Le récit va dans `vault/revues/`, la leçon dans `vault/lecons/`.
-->

## Mesures et compteurs

| Quoi | Valeur | Mesuré par | Le | Commande pour le rejouer |
|---|---|---|---|---|
| <suite de tests, taille, durée…> | <valeur> | <Mxxxx / R0xx> | AAAA-MM-JJ | `<commande>` |

## Versions et environnements

| Quoi | Valeur | Source | Le |
|---|---|---|---|
| <outil, service, cible déployée> | <version ou SHA> | <commande ou rapport> | AAAA-MM-JJ |

## Limites connues et dettes

- <limite nommée, en une ligne> — <issue ou rapport> (AAAA-MM-JJ)
