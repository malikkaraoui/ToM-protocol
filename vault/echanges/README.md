---
date: AAAA-MM-JJ
tags: [echanges, canal, convention]
---

# Convention `vault/echanges/` — le canal d'une fenêtre

## Ce que c'est

**Un fichier par fenêtre**, nommé d'après elle : `vault/echanges/<Fxx>.md` (`F01`, `F02`, …).
Ce fichier est à la fois le **mandat** posé par l'orchestrateur et le **rapport** rendu par la
session qui l'exécute : le rapport est APPENDÉ au même fichier, sous un titre `## Rapport <ID>`.
Rien n'est effacé, rien n'est réécrit en place.

Le fichier de fenêtre est un **canal**, pas une archive : il ne porte qu'un mandat à la fois.
Quand la chaîne est soldée, il part dans `archive/`.

## Machine d'états du frontmatter

Le champ `statut:` du frontmatter est l'état de la fenêtre. Il est lu par des outils (et par
l'humain) : ces quatre valeurs sont les seules admises, et chaque transition a **un seul**
auteur légitime.

```
en-pose  ──(orchestrateur)──>  pret-a-lancer  ──(session)──>  en-cours  ──(session)──>  reponse-disponible
   ▲                                                                                            │
   └──────────────────────── (orchestrateur, après traitement + archivage) ─────────────────────┘
```

| Statut | Signification | Qui l'écrit |
|---|---|---|
| `en-pose` | Le mandat est en cours de rédaction. **Il ne doit pas être lancé.** | Orchestrateur |
| `pret-a-lancer` | Le mandat est complet et peut être exécuté. C'est le seul statut qui autorise un lancement. | Orchestrateur, en **dernier** geste de la pose |
| `en-cours` | La session a pris le mandat et travaille. | La session, en **premier** geste |
| `reponse-disponible` | Le rapport est écrit, le vault committé, les pushs exigés faits, la session a fini. | La session, en **dernier** geste |

Deux règles qui font tenir la machine :

1. **`pret-a-lancer` est écrit en dernier**, une fois le mandat entièrement rédigé. Un mandat
   à moitié écrit mais déjà marqué lançable sera lancé à moitié écrit.
2. **`reponse-disponible` est écrit en dernier**, une fois le rapport appendé, les registres
   mis à jour, le vault committé et le push prouvé (celui du vault suit `git.pousser_vault`). C'est le signal « tu peux lire » — s'il arrive avant que tout
   soit écrit, l'orchestrateur lit un état incomplet et le croit final.

## Qui a le droit d'écrire quoi

| Zone | Orchestrateur | Session de la fenêtre | Autre fenêtre |
|---|---|---|---|
| Corps du mandat (`<Fxx>.md`, avant le rapport) | Écrit | **Ne modifie jamais** | Jamais |
| `## Rapport <ID>` appendé au même fichier | Lit | Écrit | Jamais |
| `statut:` du frontmatter | `en-pose`, `pret-a-lancer` | `en-cours`, `reponse-disponible` | Jamais |
| `vault/echanges/<autre Fxx>.md` | Écrit (pose) | **Jamais** | Jamais |
| `vault/runtime/state.json` | `windows` de ses poses | `windows.<sa fenêtre>` **seule** | Jamais |
| `vault/runtime/events.jsonl` | Append | Append | Append |

Le mandat lui-même nomme explicitement les fichiers de fenêtre à ne pas toucher (§ Interdits) :
un fichier `<Fxx>.md` peut être la **pièce de référence** d'un doublage en cours, et l'écraser
détruit la preuve sur laquelle ce doublage s'appuie.

## Archivage

Quand une chaîne est soldée (mandat rendu, rapport traité, et doublage tranché s'il y en avait
un), le fichier est **déplacé** — jamais copié, jamais vidé — vers :

```
vault/echanges/archive/AAAA-MM-JJ-<Fxx>-<ID>-<slug>.md
```

`<ID>` est l'identifiant du mandat (`Mxxxx`) ou de la revue (`R0xx`), `<slug>` un titre court en
minuscules avec tirets. La fenêtre repart alors d'un fichier neuf à la pose suivante. **Rien
n'est jamais supprimé de `archive/`** : c'est la trace de ce qui a réellement été demandé, mot
pour mot, en regard de ce qui a été rendu.

## Distinction avec `vault/reprise/<Fxx>.md`

- `vault/echanges/<Fxx>.md` = le **canal vivant** : mandat intégral + rapport intégral.
- `vault/reprise/<Fxx>.md` = le **point d'ancrage court** (frontmatter `date`, `mandat_id`,
  `statut` + un résumé de quelques lignes), lu en reprise pour savoir où on en est sans charger
  tout le canal.

Les deux vivent en parallèle et ne se remplacent pas. Ni l'un ni l'autre n'est une preuve :
voir la hiérarchie de source de vérité dans `vault/runtime/README.md`.
