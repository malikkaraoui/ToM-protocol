---
date: AAAA-MM-JJ
tags: [runtime, orchestration, gouvernance]
statut: actif
---

# vault/runtime/ — état persistant du bureau d'orchestration

## Pourquoi ce dossier existe

Le point de défaillance d'un bureau d'orchestration n'est pas la capacité à exécuter
plusieurs fenêtres en parallèle — c'est **l'orchestrateur lui-même** : fenêtre de contexte,
compaction, redémarrage de session. Autant de sources d'oublis et de dérives (fenêtre
confondue, SHA perdu, événement supposé réalisé parce qu'attendu, état conversationnel
mélangé à l'état réel du dépôt).

**Principe directeur :**

> L'orchestrateur peut oublier. Le système, lui, ne doit pas oublier.
> Ce qui est écrit n'est pas forcément vrai ; ce qui est vérifié doit pouvoir être rejoué.

Ce dossier ne remplace pas la mémoire de l'orchestrateur — il la rend **non nécessaire**
pour les faits qui comptent : où en est chaque fenêtre, quel mandat elle exécute, sur
quelle branche et quel SHA, ce qui s'est réellement passé.

## État, événement, preuve — trois choses différentes

| Notion | Où | Nature |
|---|---|---|
| **État** | `state.json` | Un REGISTRE déclaratif — ce que l'orchestrateur croit vrai au dernier événement traité. **Pas une preuve.** |
| **Événement** | `events.jsonl` | Un JOURNAL append-only — ce qui a été *rapporté* comme s'étant passé, horodaté. **Pas une preuve que le fait Git/fichier sous-jacent est vrai.** |
| **Preuve** | `git status`, `git rev-parse`, `git ls-remote`, existence de fichier, sortie de commande collée | La seule chose qui fait foi. Rejouable, vérifiable à l'instant T. |

`state.json` et `events.jsonl` documentent ce qui a été **DÉCLARÉ**. Une déclaration n'est
jamais promue au rang de preuve par le seul fait d'avoir été écrite. Avant toute décision
critique (§ Réconciliation), on vérifie contre une preuve réelle — jamais contre le
registre seul.

## Structure

- `state.json` — état courant des fenêtres (une seule version, écrasée à chaque mise à
  jour : ce n'est **pas** un historique, c'est un instantané).
- `events.jsonl` — journal append-only, une ligne JSON par événement, jamais réécrit.
  Une correction du passé = un **nouvel** événement, jamais l'édition d'une ligne existante.
- `README.md` — ce fichier (versionné ; les deux registres ci-dessus ne le sont pas,
  voir le fragment `.gitignore` livré avec ce scaffold).

## Cycle de vie

1. Un événement significatif se produit (allocation de fenêtre, pose de mandat, rapport
   traité, commit, push, revue, merge, libération de fenêtre, incident).
2. Une ligne est ajoutée à `events.jsonl` (append, jamais d'édition).
3. `state.json` est mis à jour en conséquence (remplacement de l'entrée de la **seule**
   fenêtre concernée — jamais une réécriture globale du fichier par une session).
4. Avant toute décision critique : réconciliation. L'état déclaré est comparé à une preuve
   réelle. Contradiction → `CONFLICT`, jamais une déduction.

## Règles d'écriture

- `events.jsonl` : **APPEND ONLY**. Jamais éditer ni supprimer une ligne. Une ligne = un
  objet JSON complet, sur une seule ligne, valide isolément.
- `state.json` : remplacement de section, **jamais d'invention de valeur** — un champ
  inconnu reste `null`, jamais une supposition plausible.
- Aucun SHA enregistré ici n'est une garantie qu'il est encore exact au moment de la
  lecture : revérifier avant tout usage critique (`git rev-parse`, `git ls-remote`).
- **`state.json` est un REGISTRE, pas un journal.** Ce fichier est relu en entier au
  démarrage de chaque session d'orchestration : chaque octet en trop s'y paye à chaque
  démarrage. Règles dures :
  - `windows.<Fxx>.last_event` : **≤ 200 caractères**. Le détail complet est déjà dans
    `events.jsonl` ou dans le rapport référencé — ne jamais y raconter, seulement pointer.
  - Une fenêtre terminée ne garde que `{state, mandat, branch, sha}` : tout le reste est
    redondant avec `events.jsonl` une fois le travail intégré.
  - Aucun champ ne porte de narration : si le contenu existe déjà dans le carnet ou dans
    `vault/decisions/`, le champ **pointe dessus** ; sinon le contenu est déplacé verbatim
    dans un fichier `vault/decisions/` dédié et le champ ne garde que le pointeur.
- **Schéma canonique vs espace projet.** Les clés canoniques sont celles listées ci-dessous.
  Tout ce qui est propre au projet (états d'infrastructure, jalons, paramètres maison) va
  dans l'objet `projet{}` et **nulle part ailleurs** : cela évite qu'un registre générique
  se transforme en fourre-tour de clés ad hoc datées, impossibles à lire pour un outil.

### Schéma canonique de `state.json`

```json
{
  "schema_version": 1,
  "updated_at": null,
  "next_mandat_id": "M0001",
  "next_revue_id": "R001",
  "windows": {},
  "last_reconciliation": null,
  "projet": {}
}
```

- `windows.<Fxx>` : `{state, mandat, branch, sha, statut, last_event, heartbeat_at}` —
  `state` = vue orchestrateur (`LIBRE` / `EN_COURS` / `RENDU` / `IDLE`), `statut` = copie du
  frontmatter de `vault/echanges/<Fxx>.md`, `heartbeat_at` = dernier battement déclaré ou `null`.
- `next_mandat_id` / `next_revue_id` : compteurs simples, jamais réutilisés, jamais
  rétroactifs. Ils suffisent — **pas de fichier « registre des mandats » séparé** : la chaîne
  mandat → fenêtre → branche → SHA → rapport → doublage → merge se reconstruit par
  `grep '"mandat": "<Mxxxx>"' vault/runtime/events.jsonl`, jamais par un fichier tenu à la
  main en double de la vérité.
- `projet` : espace libre du projet. Hors schéma canonique, jamais lu par un outil générique.

## Vocabulaire d'événement — FIGÉ

Un journal sans schéma tenu devient illisible en quelques centaines de lignes : plusieurs
générations de noms cohabitent, plusieurs synonymes désignent la même notion, et plus aucune
requête ne peut être écrite avec certitude. **Ce vocabulaire est figé au premier jour du
projet et ne bouge plus** ; un besoin nouveau ajoute un `type`, ne renomme jamais l'existant.

Champs **obligatoires** sur toute ligne :

| Champ | Type | Sens |
|---|---|---|
| `ts` | chaîne ISO 8601 avec fuseau | Horodatage réel du moment où la ligne est écrite. |
| `type` | chaîne | Le type d'événement, en MAJUSCULES_FRANÇAIS, pris dans la liste ci-dessous. |
| `src` | chaîne | Qui écrit : `orch` (orchestrateur), `sup` (superviseur), `fenetre`. |

Champs **optionnels normalisés** — un seul nom par notion, aucun synonyme admis :

| Champ | Sens | Noms INTERDITS (ne jamais réintroduire) |
|---|---|---|
| `f` | La fenêtre concernée (`F01`, `F02`, …) | `window`, `fenetre`, `session_id` |
| `mandat` | L'identifiant du mandat ou de la revue (`Mxxxx` / `R0xx`) | `mandat_id`, `id` |
| `note` | Texte libre court (≤ 200 caractères, factuel) | `detail`, `details`, `msg`, `extrait`, `commentaire` |

Tout autre champ est propre à un `type` donné et doit être documenté ici avant d'être écrit
(par exemple `branch`, `sha`, `preuve` sur un `MERGE`).

### Types posés par l'orchestrateur

| `type` | Quand | Champs attendus |
|---|---|---|
| `MANDAT_POSE` | Un mandat vient d'être écrit dans `vault/echanges/<Fxx>.md` | `f`, `mandat`, `note` |
| `RAPPORT_TRAITE` | L'orchestrateur a lu et traité un rendu de fenêtre | `f`, `mandat`, `note` |
| `DECISION` | Un arbitrage humain est acté | `note` (+ `mandat` si rattaché) |
| `MERGE` | Une branche est intégrée à la branche principale | `mandat`, `branch`, `sha`, `preuve` |
| `RECONCILIATION` | L'état déclaré vient d'être comparé à une preuve réelle | `note` (+ `f` si ciblée) |
| `CHECKPOINT` | Point d'étape volontaire de l'orchestrateur | `note` |
| `INCIDENT` | Anomalie constatée (perte, blocage, contradiction non résolue) | `note` (+ `f`, `mandat`) |

Exemple de ligne conforme :

```json
{"ts":"AAAA-MM-JJTHH:MM:SS+00:00","type":"MANDAT_POSE","src":"orch","f":"<Fxx>","mandat":"<Mxxxx>","note":"pose du mandat <slug>, branche a creer depuis la principale"}
```

### Types posés par le superviseur

Le superviseur (démarrage/fin de session, verrous, panneaux, mandats suspects) écrit dans le
**même** journal, avec `src: "sup"` et le même vocabulaire de champs. **Sa liste de `type` est
définie dans son propre README** — elle n'est pas redéfinie ici, pour éviter deux définitions
concurrentes qui divergeront. Si le projet n'a pas de superviseur, aucune ligne `src: "sup"`
n'existe : ce n'est pas une anomalie.

## Heartbeat — contrat

Un heartbeat signifie : *« cette session déclare encore être active à cet instant »*. Il NE
signifie PAS : *« le travail est réellement en cours et correct »*. Ces deux notions restent
séparées — un heartbeat frais est une preuve de **présence déclarée**, jamais une preuve de
progrès.

Fraîcheur **calculée à la LECTURE, jamais stockée** — un statut stocké (`"fresh": true`)
deviendrait faux à la seconde suivante et mentirait au premier lecteur :

- `fresh` : `now - heartbeat_at < seuil`
- `stale` : `now - heartbeat_at >= seuil`
- `unknown` : `heartbeat_at` vaut `null` (jamais déclaré, ou fenêtre jamais allouée)

Le seuil est un paramètre de projet (clé `heartbeat_stale_after_minutes`), à poser sur la base
des durées de session **observées**, et à réviser dès qu'un usage réel existe. Tant qu'aucune
mesure n'existe, le seuil est un choix provisoire assumé, pas un acquis.

## Réconciliation — obligatoire avant toute décision critique

Décisions critiques (liste non limitative) : lancer un mandat sur une fenêtre déclarée libre,
considérer une fenêtre comme terminée, annoncer qu'un push existe, déclencher un doublage,
lancer un merge, libérer une fenêtre, reprendre après un redémarrage de session.

Comparer :

```
État déclaré                          État vérifiable
  ├── vault/runtime/state.json          ├── git status
  ├── vault/runtime/events.jsonl        ├── git rev-parse
  ├── vault/echanges/<Fxx>.md           ├── git branch -vv
  └── vault/reprise/00_INDEX.md         ├── git ls-remote
                                        ├── existence de fichier
                                        └── autre preuve adaptée
```

En cas de contradiction : **`ÉTAT = UNKNOWN / CONFLICT`**, jamais une déduction plausible.
Un trou assumé vaut mieux qu'un état inventé. La réconciliation est journalisée
(`type: RECONCILIATION`) et `last_reconciliation` est mis à jour.

## Hiérarchie de source de vérité (du plus fiable au moins fiable)

1. Preuves réelles Git / fichiers / commandes **rejouées à l'instant présent**
2. `vault/runtime/state.json` + `events.jsonl` (registre — jamais une preuve en soi)
3. `vault/echanges/<Fxx>.md` (canal vivant de la fenêtre)
4. `vault/reprise/00_INDEX.md` et `vault/reprise/<Fxx>.md`
5. Mémoire conversationnelle de l'orchestrateur

`state.json` et `00_INDEX.md` ne doivent **jamais** être présentés comme des preuves en
eux-mêmes — seulement comme des déclarations à vérifier.

## Dégradation contrôlée

Le protocole doit continuer à fonctionner si ce dossier est absent, incomplet ou inconnu
d'une session ancienne.

- Comportement attendu : **dégradation contrôlée** — on retombe sur la hiérarchie ci-dessus
  (canal de fenêtre, reprise, mémoire), on **signale** l'absence, on ne bloque pas le travail.
- Comportement interdit : **inventer l'état manquant** pour combler le vide.
