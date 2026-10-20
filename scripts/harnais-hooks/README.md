---
date: AAAA-MM-JJ
tags: [git, hooks, harnais]
---

# Hooks git du harnais

Quatre hooks, tous en `sh` POSIX, sans dépendance :

| Hook | Ce qu'il fait | Bloquant ? |
|---|---|---|
| `pre-commit` | Refuse un commit qui ferait dépasser la limite d'octets à l'index de reprise, ou qui fait sortir un fichier chargé d'office (`CLAUDE.md`, rules, `SKILL.md`, index, carnet) de son budget : taille en caractères, largeur de ligne. **Imprime le remède.** | Oui (c'est son objet) |
| `pre-merge-commit` | Rejoue `pre-commit` sur le résultat d'un merge sans conflit (git ne le fait pas). | Oui |
| `commit-msg` | Impose la signature du harnais (cascade des réglages : surcharge du projet, valeur commune, sinon la valeur par défaut intégrée), quelle que soit la signature proposée par l'outil appelant. | **Jamais** (`exit 0` en toute circonstance) |
| `pre-push` | Sur la branche principale, en mode `direct` (défaut) : ne laisse passer que les merges couverts par un rapport de doublage GO (`tip:` = le tip mergé) d'un niveau qui couvre les chemins touchés (`niveau:`, `niveaux` de la config), et les commits qui ne touchent que `vault/`. En mode `pr` (`git.mode_fusion`) : refuse tout push sur la principale, la fusion passe par une pull request. | Oui, **sur la branche principale seulement** |

## Activation

```
git config core.hooksPath <chemin de ce dossier dans le projet>
```

Par exemple, si le scaffold est copié dans `scripts/git-hooks/` :

```
git config core.hooksPath scripts/git-hooks
chmod +x scripts/git-hooks/pre-commit scripts/git-hooks/pre-merge-commit scripts/git-hooks/commit-msg scripts/git-hooks/pre-push
```

Vérifier : `git config --get core.hooksPath`. Le bit exécutable est nécessaire — un hook non
exécutable est ignoré **en silence**, et le garde-fou disparaît sans que rien ne le signale.

## Pourquoi hors de `.git/hooks`

`.git/hooks/` n'est **pas versionné** et n'est pas copié par un `git clone`. Un hook qui y vit
n'existe que sur la machine où quelqu'un l'a installé à la main : il protège un poste, pas le
projet. Déportés dans un dossier versionné pointé par `core.hooksPath`, les hooks sont revus
comme du code, voyagent avec le dépôt, et leur modification laisse une trace.

Contrepartie assumée : `core.hooksPath` est une configuration **locale**, à poser une fois par
clone (et par worktree qui ne partage pas la config). L'installation reste un geste explicite ;
c'est le contenu qui est partagé, pas l'activation.

## Paramétrage — rien n'est codé en dur

### `pre-commit`

**Index de reprise.** Bloc `rotation` de premier niveau du fichier de configuration ; une variable
d'environnement non vide passe devant la config, clé par clé :

| Clé de `rotation` | Variable | Défaut | Rôle |
|---|---|---|---|
| `index` | `ROTATION_INDEX` | `vault/reprise/00_INDEX.md` | Le fichier surveillé. |
| `index_limite_octets` | `ROTATION_LIMITE` | `80000` | La limite, en octets. |
| `remede` | `ROTATION_REMEDE` | texte à remplacer | La commande de rotation imprimée dans l'erreur. |

La taille mesurée est celle de l'objet **indexé**, pas celle du disque : c'est le contenu qui sera
réellement committé. Valeur invalide (limite nulle, négative, non entière, entre guillemets ; index
qui n'est pas une chaîne) : le défaut, et une ligne sur stderr — `ROTATION_LIMITE` invalide laisse
la limite de la config en place. Rien n'éteint le contrôle.

**D'où vient la config.** Le hook lit le fichier de configuration (`HARNAIS_CONFIG`, défaut
`config/harnais.json`) **dans git, jamais au disque** : la version indexée pour un commit, celle de
`<ref>` en mode contrôle. Un seuil ne se relève donc que par un changement committé, visible dans
le diff. Une config présente au disque mais absente de l'index n'est pas lue : défauts, et une
ligne sur stderr qui dit `git add`.

**Ce qui est jugé.** Le hook ne lit pas une liste de noms : il lit les **entrées** de l'arbre
(mode, objet, chemin) en `-z`, et mesure chaque fichier par son objet. Aucun nom n'est interprété
ni passé à une commande : guillemet, antislash, tabulation ou saut de ligne dans un nom ne changent
rien (un caractère de contrôle est affiché `?`). Trois conséquences :

- **casse** : un motif est comparé sans casse (ASCII). `Claude.md` est `CLAUDE.md` sur un disque
  insensible à la casse ; ailleurs, le hook surveille un fichier de plus, jamais un de moins ;
- **liens symboliques** : une entrée est jugée sous tous ses noms — le sien, et ceux que lui
  donnent les liens du dépôt. `CLAUDE.md -> docs/gros.md` : c'est `docs/gros.md` qui est mesuré,
  au commit qui pose le lien (comme un fichier neuf) et à chaque commit qui touche la cible. Un
  dossier lié (`.claude/rules -> ../regles`) donne de même un nom surveillé aux fichiers du
  dossier visé. Un lien surveillé dont la cible n'est pas un fichier du dépôt (chemin absolu, hors
  dépôt, absente, autre lien) est **refusé** : il n'y a rien à mesurer ;
- **panne** : liste illisible, objet illisible, tri interrompu → `BLOQUE`, jamais « rien à redire ».

**Fichiers chargés d'office (#113 ; M0198 pour la taille, M0202 pour le reste).** Un `CLAUDE.md`,
une rule, le `SKILL.md` d'un skill, l'index et le carnet de reprise sont relus à **chaque** démarrage
de session : ce qu'ils pèsent se paye avant que la session ait rien produit.

Constat de terrain qui fonde la règle : un `CLAUDE.md` passé de 12,6 k à 220 k caractères en neuf
jours, à nombre de lignes stable — 207 000 caractères dans une seule section, des lignes de
14 000 caractères. Les mandats n'ajoutaient pas de lignes, ils **allongeaient les puces** (compteurs,
« mesuré par Mxxxx », historique), et des branches parallèles s'additionnaient à la fusion. La
consigne « tenir le fichier à jour » en était la cause : seule une barrière automatique tient.

Le hook borne donc **deux** grandeurs, sur la version **indexée** :

| Mesure | Effet |
|---|---|
| taille ≤ 80 % du seuil | rien |
| taille > 80 % du seuil, ≤ seuil | `ATTENTION`, commit accepté |
| taille > seuil | `BLOQUE` : fichier, taille, seuil, taille avant le commit, purge — commit refusé |
| taille > seuil, mais le fichier l'était déjà **et ne grossit pas** | `ATTENTION` + purge, commit accepté |
| une ligne plus large que le seuil, **ajoutée ou modifiée** par le commit | `BLOQUE` : fichier, numéro et largeur des lignes, purge — commit refusé |
| une ligne trop large déjà là, **laissée intacte** | `ATTENTION` + purge, commit accepté |

Les deux tolérances (dernière ligne de chaque grandeur) existent pour qu'un projet qui hérite d'un
fichier hors budget puisse le purger par étapes au lieu de sauter le hook. Elles ne s'ouvrent que
sur ce qui existait **au même chemin** dans `HEAD` — et, dans un merge, dans **chaque parent** : ce
qu'un parent portait déjà n'est pas une aggravation (une branche ancienne reçoit la principale
sans replier les lignes larges que la principale tolère), la somme de deux croissances parallèles
en est une :

- taille : le fichier dépassait déjà et ne grossit pas. Renommer, ou supprimer puis ré-ajouter, ne
  l'ouvre pas (le fichier compte alors comme neuf) ;
- largeur : la ligne existait déjà **à l'identique** (autant de fois). Allonger une puce trop large,
  la raccourcir sans la ramener sous le seuil, la réécrire ou la dupliquer est un refus : une ligne
  qu'on touche ressort sous le seuil.

Bloc `instructions` de premier niveau du fichier de configuration (`HARNAIS_CONFIG`, défaut
`config/harnais.json` ; une clé de même nom dans un autre bloc est ignorée) :

```json
"instructions": {
  "max_caracteres": 40000,
  "max_ligne": 400,
  "fichiers": [
    "**/CLAUDE.md",
    "**/.claude/rules/**/*.md",
    "**/.claude/skills/**/SKILL.md",
    { "motif": "vault/reprise/00_INDEX.md", "max_caracteres": 80000 },
    "vault/reprise/CARNET_DE_BORD.md"
  ]
}
```

| Clé | Défaut | Rôle |
|---|---|---|
| `max_caracteres` | `40000` | Seuil de taille. `INSTRUCTIONS_MAX_CARACTERES` (environnement) est prioritaire ; s'il **relève** le seuil, une ligne sur stderr le dit (aucune trace versionnée). |
| `max_ligne` | `400` | Seuil de largeur d'une ligne. Pas de variable d'environnement. |
| `fichiers` | la liste ci-dessus | Les fichiers surveillés. Une entrée est un motif, ou un objet `{ "motif", "max_caracteres", "max_ligne" }` qui donne à ce motif ses propres seuils. **Le premier motif qui reconnaît un chemin décide.** La liste par défaut est **toujours ajoutée à la suite** : la config donne ses seuils à un motif, elle ne retire aucun fichier du contrôle. |

Motif : ancré à la racine du dépôt ; `**/` = zéro dossier ou plus ; `*` et `?` ne franchissent pas
`/` ; tout autre caractère est littéral ; comparaison sans casse. L'index garde par défaut son
budget de 80 000 (celui de la rotation, en octets, qui reste le premier à refuser) : ce contrôle-ci
lui ajoute la largeur.

**Le remède imprimé dépend du rôle du fichier.** Un fichier de règles (`CLAUDE.md`, rule,
`SKILL.md`) : n'y garder que des règles, l'état va dans `vault/reprise/etat-technique.md`. L'index
et le carnet de reprise portent de l'état daté, c'est leur rôle : le refus dit de **replier** la
ligne (une puce Markdown coupée garde son rendu), de faire tourner l'index, de réécrire le carnet —
jamais de les vider. Le rôle tient au chemin (les deux motifs de reprise de la liste par défaut),
pas à l'entrée de config qui règle ses seuils. Donner sa largeur au carnet :
`{ "motif": "vault/reprise/CARNET_DE_BORD.md", "max_ligne": <n> }`.

Les tailles se comptent en **caractères** (octets UTF-8 hors octets de continuation : même résultat
quelle que soit la locale du poste).

Fail-closed : seuil nul, négatif, non entier, entre guillemets → le défaut (ou le seuil général,
pour un seuil par motif) et une ligne sur stderr. Liste vide, qui n'est pas une liste, ou entrée
sans motif lisible → une ligne le dit, les entrées lisibles sont gardées, la liste par défaut suit
comme toujours. Config absente de git ou illisible → liste et seuils par défaut. Rien ne désactive
le contrôle ; il n'existe pas de valeur « désactivé ».

**Mode contrôle — pour le doublage.** Le hook se lance aussi à la main, en lecture seule :

```
sh <dossier des hooks>/pre-commit --controle [<ref>] [--base <ref>] [--lignes]
```

Une ligne `OK` / `ATTENTION` / `BLOQUE` par fichier chargé d'office de `<ref>` (défaut `HEAD`), puis
un total ; code 1 s'il y a un `BLOQUE`, 2 si l'appel ou une ref est invalide. Sans `--base` : l'état
absolu (tout dépassement est un `BLOQUE`) — la dette de purge. Avec `--base` : même tolérance que le
hook, seul ce que `<ref>` aggrave depuis la base bloque — c'est ce que juge un doublage, et c'est le
filet de ce qui a sauté le hook (`--no-verify`, merge en avance rapide). `--lignes` : chaque ligne
trop large est listée, une par ligne (numéro, largeur) au lieu des trois premières — la liste de
travail d'une purge (#131). Seuils et liste sont ceux de la config **de `<ref>`**. Dans le dépôt du
harnais, `scripts/controle-instructions.sh` est le même appel.

Limites connues, assumées :
- `git commit --no-verify` saute le hook, comme pour tout hook client ; un merge en avance rapide ne
  crée pas de commit et ne lance aucun hook ; `git cherry-pick` et `git rebase` ne lancent pas
  `pre-commit` — le mode contrôle du doublage est le filet ;
- `INSTRUCTIONS_MAX_CARACTERES` et `ROTATION_LIMITE` relèvent un seuil pour un commit sans trace
  versionnée : le premier le dit sur stderr, rien ne l'interdit (même famille que `--no-verify`) ;
- un seuil relevé dans la config **committée** passe : il est dans le diff, c'est au doublage de le voir ;
- un renommage compte comme un fichier neuf à son nouveau chemin (`--no-renames`) ;
- `CLAUDE.local.md` et les fichiers importés par `@chemin` ne sont pas dans la liste par défaut ;
- liens symboliques : un seul niveau est suivi (un lien vers un lien, ou un chemin qui traverse un
  autre dossier lié, n'est pas résolu : refus s'il est lui-même surveillé, ignoré sinon) ; un
  dossier lié vers l'extérieur du dépôt n'a aucun contenu dans git, le hook n'en voit rien ;
- casse : seules les lettres ASCII sont repliées (`É` / `é` restent distincts) ;
- les parents d'un merge automatique sont lus dans les variables `GITHEAD_<objet>` que git pose
  (`MERGE_HEAD` n'est pas encore écrit quand `pre-merge-commit` tourne) : une telle variable posée
  à la main ouvre la tolérance d'un merge à un commit ordinaire ;
- deux lignes qui ne diffèrent que par les octets de continuation d'un caractère accentué (`é` /
  `è`) sont tenues pour identiques par la tolérance de largeur : corriger un accent dans une ligne
  trop large passe, sans jamais l'allonger ;
- le compte est celui des points de code Unicode ; un outil qui compte autrement (unités UTF-16)
  peut donner un chiffre légèrement différent sur un texte riche en émojis.

Témoins : `node --test scripts/harnais-hooks/pre-commit.test.mjs` (`HOOK_PRE_COMMIT=<chemin>` pour
rejouer la suite sur une autre version du hook) — chaque témoin fait un vrai `git commit` dans un
dépôt jetable. La suite est aussi importée par `commit-msg.test.mjs` : elle tourne avec lui.

### `pre-merge-commit`

Git ne lance pas `pre-commit` sur un merge sans conflit. C'est par là que des branches parallèles,
chacune sous le seuil, s'additionnent. Ce hook exécute `pre-commit` (le fichier voisin) sur le
résultat du merge, comparé à `HEAD` — le premier parent : même règle, un seul code. Refusé, le
merge reste en cours : alléger le fichier, `git add`, `git commit` (ou `git merge --abort`). Si
`pre-commit` est absent ou non exécutable, le merge est refusé et le dit.

### `commit-msg`

La valeur du trailer ne vient jamais de la config du projet : c'est la **signature du harnais**. Elle se lit dans
`${HARNAIS_HOME:-~/.harnais}/reglages.json` (page Réglages du tableau), selon la cascade : **surcharge du projet**
(`projets.<nom>.signature`, `<nom>` = `projet.nom` de la config) > **valeur commune** (`commun.signature`) >
**valeur par défaut intégrée** (`Co-Authored-By: Malik & Claude`, identique à `SIGNATURE_PAR_DEFAUT` dans
`superviseur/signature.mjs`). Tant que `reglages.json` n'existe pas, l'ancien fichier `${HARNAIS_HOME}/signature`
(M0372) vaut la valeur commune. Entrée absente : défaut, en silence. Entrée illisible ou invalide : elle est ignorée,
la valeur en vigueur suivante s'applique, et une ligne sur stderr. Le hook sort 0 dans tous les cas. La cascade est
résolue en `node` (le hook ne peut pas importer `superviseur/reglages.mjs`) ; `commit-msg.test.mjs` vérifie la parité.

**Le hook ne bloque jamais un commit** : dégradation contrôlée,
jamais un commit perdu parce qu'une configuration manquait.

**Retrait des trailers concurrents — décision du 2026-09-23 (Malik, « Retrait automatique »,
#29).** Avant d'ajouter le trailer du projet, le hook retire les lignes qui matchent un motif
étendu (ERE, évalué par `awk`). Le motif est cherché, dans l'ordre :

1. `TRAILERS_RETIRES` (variable d'environnement) — même vide : vide = ne rien retirer ;
2. la clé `"trailers_retires"` du fichier de configuration (bloc `git`) — même vide : vide = ne
   rien retirer (opt-out explicite) ;
3. à défaut de clé : `^[Cc][Oo]-[Aa][Uu][Tt][Hh][Oo][Rr][Ee][Dd]-[Bb][Yy]:`.

Défaut (config livrée et repli) : **toute ligne `Co-Authored-By:` est retirée, sans casse** — Git
traite les clés de trailer sans casse, donc `Co-authored-by:` est retiré comme `Co-Authored-By:`.
Seules les lignes qui *commencent* par la clé sont visées : une mention dans le corps est intacte.
**La ligne exacte du trailer projet n'est jamais retirée**, même si elle matche le motif.

Fail-safe : config illisible ou motif invalide → rien n'est retiré, le trailer projet est quand
même ajouté, une ligne est écrite sur stderr, et le hook sort 0. Le hook ne refuse jamais un commit.

Règle révoquée (tracée, R4) : jusqu'au 2026-09-23, `TRAILERS_RETIRES` était vide par défaut et rien
n'était retiré (« geste destructeur, jamais appliqué par défaut »). Le commit `d7cefef` a montré
le coût : un trailer d'outil survivait à côté du trailer projet.

Témoins : `node --test scripts/harnais-hooks/commit-msg.test.mjs` (`HOOK_COMMIT_MSG=<chemin>` pour
rejouer la suite sur une autre version du hook).

Le hook est idempotent : relancé sur un message déjà signé, il ne duplique pas le trailer.

### `pre-push`

Décision du 2026-09-24 (Malik, #27 recadrée : « simple a efficace / on essai ») : la règle « rien ne
merge sans doublage indépendant » (convention §9) ne tient plus seulement par la discipline.

Entrée (`githooks(5)`, section pre-push) : une ligne par ref poussée,
`<local-ref> SP <local-object-name> SP <remote-ref> SP <remote-object-name> LF` ; ref absente du
distant = `<remote-object-name>` tout à zéro ; suppression = `<local-ref>` `(delete)` et
`<local-object-name>` tout à zéro.

**La règle, en une phrase (décision Malik du 2026-09-24 00:55, #41), en mode `direct`** : sur `main`,
seuls passent les merges avec rapport GO et les commits qui ne touchent que `vault/`. « Tout le reste
est refusé, et il faut passer par une branche, un doublage et un merge. »

Branche principale : clé `"branche_principale"` du bloc `"git"` de `HARNAIS_CONFIG` (défaut
`config/harnais.json`). Seul ce bloc compte : une clé de même nom dans un autre bloc est ignorée
(O4, témoin P28). Config absente, clé absente ou JSON illisible par le mini-lecteur : `main`.
Config présente mais non lisible (droits) : erreur interne, refus sur `main`.

**Mode de fusion** : clé `"mode_fusion"` du même bloc `"git"` (témoin M0300-8 : une clé d'un autre bloc
est ignorée).

- `"direct"` ou clé absente : le comportement décrit ci-dessous, inchangé.
- `"pr"` : **tout push vers la principale est refusé**, création, mise à jour et suppression comprises.
  La fusion passe par une pull request, faite par GitHub : `gh pr create --base <principale> --head
  <branche>`, puis `gh pr merge --auto`. Les branches de mandat ne sont pas contrôlées par ce hook : la
  preuve GO accompagne la PR, pas le push, et le trailer reste vérifié par `commit-msg`.
- Autre valeur : erreur interne, refus sur `main` par prudence (témoin M0300-7).

Pour chaque ligne visant `refs/heads/<branche principale>` :

1. **plage introduite** : `remote..local` ; distant nul ou inconnu localement : `local --not --remotes` ;
   suppression : laissée passer ;
2. **commits directs (O3)** : chaque commit **non-merge** de la chaîne `git rev-list --first-parent`
   de la plage ne doit toucher que des chemins sous `vault/` (journaux, archives, rapports). Les
   chemins sont lus par `git diff-tree --no-renames` : un renommage `scripts/x` → `vault/x` compte
   comme un chemin hors `vault/` (témoin P22). Un fast-forward de `main` vers le tip d'une branche
   est donc refusé : les commits de la branche deviennent premier parent (P18). Les commits amenés
   par le **second** parent d'un merge ne sont pas contrôlés un par un : le rapport GO du merge les
   couvre ;
3. **merges** : pour chaque commit de merge de la plage, `T` = son **second parent** (le tip mergé) :
   il faut, dans l'**arbre poussé** (`git ls-tree` sur `local`, jamais le working tree ni l'index),
   un fichier `vault/revues/*.md` dont le frontmatter
   - commence par `---` en première ligne et se **ferme** par une ligne `---` (O2) ;
   - porte **exactement une** ligne `verdict:`, égale à `GO` (O2) ;
   - porte **exactement une** ligne `tip:`, égale au **SHA complet** (40 caractères) de `T` (O1).

   - porte **au plus une** ligne `niveau:`, égale à `N1`, `N2` ou `N3` ; absente = `N2` (M0292).

   Le corps du rapport ne compte plus : un SHA cité en passant ne couvre rien (P24), un SHA court non
   plus (P25). Un rapport malformé ou contradictoire est ignoré, jamais une erreur (P26, P27, N8).
   **Exception** (option B, décision Malik du 2026-09-24 00:28, « go pour b ») : si `T` est **déjà
   publié** — ancêtre de `remote` quand il est non nul, sinon atteignable depuis une ref
   `refs/remotes/*` — le merge est ignoré : il n'introduit aucun travail non doublé. C'est le cas
   d'un merge de la principale **dans** une branche (base périmée, convention §8 ; témoins P13, P16).
   Un tip poussé sur une *autre* branche distante n'est **pas** « publié » au sens de cette règle
   quand `remote` est connu : le merge final d'une branche doublée reste exigé (témoin P17) ;
4. **niveau (M0292, décision Malik du 2026-10-15)** : plusieurs rapports GO pour `T` → le plus haut
   niveau compte (N11). `N3` passe. Sinon le diff du merge (`M^1..M`, `--no-renames`) est confronté à
   `niveaux.critiques`, lue dans la config du working tree **et** dans celles des arbres `M^1` et `M`
   (union : une branche ne retire pas son propre chemin de la liste, N10). **Aucun nom de fichier
   n'est relu dans une sortie de git pour décider** (loi des deux patchs, R166-1 puis R167-1) : git
   confronte lui-même le diff aux motifs, passés en pathspec `:(glob)<motif>` à `git diff --quiet`
   (1 = touché, 0 = non, autre = erreur → refus, R167-3). Motifs relatifs à la racine, grammaire des
   pathspecs glob de git : `**` traverse les dossiers, `*`, `?` et `[ab]` non, un motif couvre aussi
   tout ce qui est sous lui (`wallet` = `wallet/**`) ; `*.sol` ne vaut qu'à la racine, `**/*.sol`
   partout (N9) ; un motif que git rejette (`/wallet`) fait refuser le push. Chemin critique touché
   hors N3 → refus (N1, N6, R166-1, R167-1). `N1` exige en plus une liste non vide **dans une config que la branche ne
   peut pas écrire** — l'arbre `M^1`, ou `HARNAIS_CONFIG` s'il est absolu ; jamais le working tree ni
   l'arbre `M` (R166-2) — (N4, N4b) et un diff hors
   `vault/` (exclu par pathspec) d'au plus `niveaux.n1_max_lignes` lignes ajoutées + retirées, lues
   par `git diff --numstat` sans `-z` : un nom, cité par git, tient sur sa ligne (R167-2) ; défaut 300,
   la plus petite valeur lue ; binaire = 1 ligne ; N3, N3b) → sinon refus « passe en N2 ». Sans clé `niveaux` et sans
   ligne `niveau:`, rien ne change (N7). **Ce que git lit et que la branche peut écrire est neutralisé
   (M0293, R168)** : chaque motif est aussi passé en `:(glob)<motif>/**` (`wallet/*` couvre
   `wallet/sous/x.ts`, R168-1) ; `--ignore-submodules=none` (un `.gitmodules` `ignore = all` ne cache pas
   un sous-module, R168-2) ; `--numstat` lu avec `git --attr-source=<arbre vide>` (un `.gitattributes`
   `binary` / `-diff` ne réduit pas le compte, R168-3) ; `GIT_*_PATHSPECS` effacées (R168-4). **Le
   plafond N1 exige git ≥ 2.40** : en dessous, N1 est refusé avec un message clair (M0293-V1) ;
5. sinon refus (`exit 1`) : le commit fautif et **un** chemin hors `vault/`, ou le merge et le SHA
   de `T` qui manque, ou le chemin critique et le motif, ou le plafond N1, puis la marche à suivre.

Forme attendue d'un rapport de doublage (gabarit §9) :

```
---
verdict: GO
tip: <SHA complet, 40 caractères, du tip doublé>
niveau: <N1 | N2 | N3>   # facultatif, absent = N2
---
```

**Un rapport de doublage = une branche** : un doublage multi-branches à verdict global `RESERVE`, ou
une réserve levée par un mandat correctif sans re-doublage `GO`, ne couvre rien.

Rétro-test (historique, rien à réécrire) :
- M0056/M0057, règle d'alors (« le texte cite les 7 premiers caractères de `T` ») sur les six merges
  de l'arbre `a5c36e5` : 4 acceptés, **`1fc4f53` et `5473359` refusés a posteriori** —
  doublage multi-branches (R010) / réserve levée sans re-doublage (R009, M0029) ;
- M0058, règle `tip:` : les rapports antérieurs à R020 ne portent pas `tip:`. Même invocation
  (plage `21df1f5..a5c36e5`) : les **six** merges sont refusés a posteriori, aucun commit direct
  refusé (ce sont des rapports sous `vault/`). Plage réelle `a5c36e5..786da57` : **acceptée** — le
  merge `75428f8` est couvert par R020 (`verdict: GO`, `tip: 8a9b5840…`), committé en `786da57` et
  poussé avec lui ; poussé seul, `75428f8` serait refusé. Rituel d'alors, historique non réécrit ;
  le prochain merge fait selon le nouveau §9 passe (témoin P23).

**Conséquence sur le rituel de merge** : le rapport de doublage GO est committé dans le scratch,
**après** le `merge --no-ff` et **avant** le `ff-only`, pour partir dans le même push que le merge
(gabarit §9).

Fail-closed sur la règle ; erreur interne du hook (git absent, config illisible, commande git en
échec) : refus **sur la branche principale seulement**, un push de branche de travail n'est jamais
bloqué par un bug du hook.

Limites connues, assumées :
- `git push --no-verify` saute le hook (témoin P10) — les mandats l'interdisent déjà ; un filet côté
  superviseur (même contrôle refait sur `origin/main` à chaque cycle, alerte en cas d'écart) est
  prévu au lot 2 de #41 ;
- merge « truqué » : le contenu propre d'un commit de merge (résolution, ou modification ajoutée au
  moment du merge) n'est pas inspecté ; seul le rapport GO du second parent est exigé ;
- seul le second parent est contrôlé (un merge octopus n'est couvert que pour son deuxième parent) ;
- `pre-merge-commit` ne contrôle pas le niveau : au moment du merge, le rapport n'est pas encore
  committé (il l'est juste après, dans le scratch) ; la garde du niveau vit au push ;
- distant nul (premier push de la principale vers un distant) : la plage est `local --not --remotes`.
  « Publié » = atteignable depuis n'importe quelle ref `refs/remotes/*`, y compris une branche de
  travail déjà poussée — plus permissif que le cas nominal. Et sans aucune ref `refs/remotes/*`,
  **tout** l'historique de premier parent est examiné : le premier push d'un dépôt neuf est refusé
  dès qu'un commit direct touche hors `vault/` (témoin P20b) — l'amorçage se fait avant de poser
  `core.hooksPath` ;
- un commit vide (aucun chemin) sur la principale passe (il ne touche rien hors `vault/`) ;
- le hook n'est actif que sur les clones où `core.hooksPath` pointe ce dossier.

Témoins : `node --test scripts/harnais-hooks/pre-push.test.mjs` (`HOOK_PRE_PUSH=<chemin>` pour
rejouer la suite sur une autre version du hook) — chaque témoin fait un vrai `git push` vers un
distant nu jetable.
