---
date: AAAA-MM-JJ
tags: [reprise, sessions, tableau-de-bord]
maintenu_par: orchestrateur
derniere_maj: AAAA-MM-JJTHH:MM:SS+00:00
---

# Tableau de bord — sessions du projet <projet>

<!-- rotation-index -->
> **Index court (rotation).** Seuls les derniers jours d'activité sont ici. L'historique complet
> est dans `vault/reprise/archive/index/00_INDEX-AAAA-MM.md` — le chercher par mots-clés, ne
> jamais le charger en entier. Le hook `pre-commit` livré avec ce scaffold refuse un commit qui
> ferait dépasser la limite d'octets à ce fichier, et imprime le remède : c'est le seul garde-fou
> contre un index de plusieurs centaines de kilo-octets relu à chaque démarrage de session.

<!--
CONVENTION D'ÉCRITURE — une entrée par mandat ou revue traité, ordre ANTI-CHRONOLOGIQUE
(le plus récent en tête, immédiatement sous ce bloc). Format d'une entrée :

## <horodatage ISO 8601 avec fuseau> — <Mxxxx|R0xx> (<Fxx>) : <verdict en UNE phrase>

- <fait vérifiable, avec son pointeur : rapport, branche, SHA — jamais un ressenti>
- <ce qui est mergé / non mergé, et ce qui bloque, nommé>
- <réserve ou trou assumé, s'il y en a un — l'absence de réserve se dit aussi>
- <leçon ou décision rattachée, par lien>

Règles :
- Le titre H2 porte le VERDICT, pas le sujet : il doit se lire seul, sans ouvrir le rapport.
- 2 à 4 puces, pas davantage : ce fichier est un index, pas un rapport.
- Jamais de narration : ce qui mérite d'être raconté va dans `vault/revues/`, et l'entrée pointe.
- Cette entrée est une DÉCLARATION, pas une preuve (voir `vault/runtime/README.md`).

Exemple (à supprimer à la première entrée réelle) :

## AAAA-MM-JJTHH:MM:SS+00:00 — <R0xx> (<Fxx>) doublage de <Mxxxx> : GO, mergé <sha court>

- Rapport : `vault/revues/AAAA-MM-JJ-<R0xx>-<slug>.md` ; branche `<branche>` (tip `<sha>`).
- Symptôme d'origine rejoué avant/après ; tests re-mesurés par le doubleur, pas repris du rapport d'auteur.
- Réserve non bloquante : <réserve nommée, ou « aucune »>.
-->
