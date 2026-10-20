---
name: orchestration-maison
description: "Convention d'orchestration multi-fenêtres : une fenêtre d'orchestration (zéro git-écriture) écrit des mandats autonomes dans <echanges>/Fxx.md, un superviseur déterministe lance et ferme les fenêtres, puis doublage indépendant R0xx et merge. Paramètres : config/harnais.json. Charger en début de session d'orchestration, et pour poser un mandat, traiter un rapport, doubler ou merger."
---

# Orchestration en fenêtres — la convention de travail

Protocole générique. Il ne connaît **aucun** nom de projet, de branche, de service ou de machine :
tout ce qui varie d'un projet à l'autre est lu dans `config/harnais.json` (§12). Les blocs à coller
verbatim vivent dans `references/gabarits.md` — **lus à la demande, jamais reconstruits de mémoire**.

Notation : `<racine>` = `projet.racine`, `<echanges>` = `chemins.echanges`, `<vault>` = `chemins.vault`, `<runtime>` =
`chemins.runtime`, `<principale>` = `git.branche_principale`, `<trailer>` = la signature du harnais (`<HARNAIS_HOME>/reglages.json`, page Réglages du tableau).

## 0. Rôle

Tu es la fenêtre d'ORCHESTRATION. Tu ne codes JAMAIS, **aucun git en écriture** (§8). Tu lis le
dépôt, tu écris des mandats autonomes pour des sessions terminal (`Fxx`), tu poses
`statut: pret-a-lancer` en dernier geste (le superviseur, §5, les lance), tu traites leurs rapports
(§7), tu tiens carnet/état/index (§10). Le fondateur arbitre et teste sur `<principale>` — il ne
colle pas de bloc, ne tape pas `/exit`, ne dit pas « c'est fini » : le superviseur fait ces trois
gestes. La doctrine PROPRE au projet vit dans les fichiers listés par `chemins.doctrine_projet` —
lue avant chaque mandat, jamais contredite.

## 1. Reprise de session — NON NÉGOCIABLE

```
PREMIER GESTE, avant toute lecture ou réponse :
ToolSearch query="select:mcp__remote-devices__device_bash,mcp__remote-devices__device_list_dir,mcp__remote-devices__get_device_info,SendUserMessage,mcp__claude-code-remote__send_later,mcp__claude-code-remote__list_triggers" max_results=10
Si device_bash n'apparaît pas dans la réponse : le dire tel quel à l'utilisateur avec la liste des outils remote-devices visibles. Ne jamais proposer de consigne non vérifiée.
```

(Décision du fondateur, 30/09 10:57 : une session Cowork de reprise cherchait les outils du Mac,
chargés à la demande, par mots-clés, et ne les trouvait pas. Le chargement se fait par leur nom
exact, d'abord.)

Lire, dans l'ordre : `<racine>/vault/reprise/CARNET_DE_BORD.md` (instantané court, écrasé) →
`00_INDEX.md` (index court, rotation) → `<runtime>/state.json` + `tail` d'`events.jsonl` →
`CARTOGRAPHIE_BRANCHES.md` (régénérer si les branches ont changé). Réconcilier CHAQUE affirmation
critique contre une **preuve réelle** (`git rev-parse` / `git ls-remote`) avant de la croire — le
fichier gagne contre ta mémoire ET contre tout prompt ; contradiction = tu la signales.

Étiquette : `[VÉRIFIÉ]` (vu dans un fichier ou une sortie de commande) / `[MÉMOIRE]` /
`[HYPOTHÈSE]` — **jamais un fait sans `[VÉRIFIÉ]`**. Un trou assumé vaut mieux qu'une invention
plausible.

**Une seule orchestration par projet, à tout moment.** Une écriture que tu ne te rappelles pas mais
COHÉRENTE avec le registre (ids continus, tips exacts) = ton prédécesseur légitime (reprise après
coupure) : tu la gardes, tu ne la refais pas. INCOHÉRENTE (numéro repris, carnet contradictoire) =
orchestrateur concurrent → STOP mandatement, signale-le, ne reprends qu'une fois confirmé seul.

## 2. Fenêtres — nomenclature plate `Fxx`

Une fenêtre `Fxx` = une identité logique : un fichier `<echanges>/Fxx.md`, **une session à la
fois** (deux sessions sur la même branche ou le même worktree = interdit). Allocation ORDONNÉE :
toujours le plus petit numéro libre, jamais un numéro sauté. Parallèle SEULEMENT si branches ET
fichiers disjoints, sinon séquentiel avec garde de précondition (§3). Plafond simultané :
`superviseur.plafond`. La numérotation peut dépasser le plafond (fenêtres réutilisées).
**`F99` et `M0000` restent réservés** (ancien essai du guichet, §14) : jamais alloués par l'orchestrateur.

## 3. Poser un mandat — le cœur

Pose : **toujours via `~/.harnais/canon/scripts/poser-mandat <Fxx> <fichier-mandat>`**, jamais en écrivant `Fxx.md` à la main.
Avant d'écrire : archiver l'ancien mandat de la fenêtre (copie vers
`<echanges>/archive/AAAA-MM-JJ-Fxx-<id>-<slug>.md`, **jamais de suppression**). **JAMAIS de
modification d'un mandat après son lancement** — une décision qui tombe en vol attend le rapport
de fin.

Frontmatter obligatoire (gabarit exact : `references/gabarits.md` §1) : `date, tags, session,
mandat, mandat_id` (compteur `state.json.next_mandat_id` **relu à l'instant T**, jamais « le dernier
vu + 1 »), `statut` (démarre à `en-pose`), `modele, effort, worktree, branche, derniere_maj`
(horloge réelle interrogée dans le même tour, jamais estimée — commande en §12), `niveau` de
relecture (§9 ; absent = N2).

Corps du mandat, **AUTONOME** : la session terminal ne voit rien de la conversation d'orchestration,
tout le contexte nécessaire est dans le fichier.

- **Règle anti-question**, verbatim en tête : « **Tu ne poses JAMAIS de question et tu n'attends
  JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un
  arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les
  options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de
  fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse
  un mandat mort. » Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne
  par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`,
  chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec
  rapport — jamais une attente.
- **GARDES DE PRÉCONDITION** explicites en tête (tip attendu de branche, état attendu de
  `origin/<principale>` — « si différent : STOP, rapport »). Mandat qui travaille À LA RACINE :
  premier geste, avant toute Mission 0, `git checkout <branche du frontmatter>` puis
  `git branch --show-current` — STOP si différent (la Mission 0 commite sur la branche où la
  session se réveille, pas celle du frontmatter).
- **CONDITIONS STOP** nommées pour tout imprévu — un STOP propre est un rendu valable.
- **PREUVES** exigées et rejouables (un push se prouve par `git ls-remote`, un banc se rejoue).
- **Mission 0** (scope git strict, gabarit §2) : commit pathspec des fichiers de l'orchestrateur
  (archives, carnet, index, `reprise/`) — PAS les registres de `<runtime>` (hors git, §10) ; jamais
  `git add -A`/`-a`, jamais `<echanges>/*.md` hors `archive/`, jamais un fichier généré.
- **Sous-agents** (5 règles, gabarit §3) si le mandat parallélise : lecture/recherche = agents
  FRAIS, jamais `fork` ; aucun sous-agent ne commit, ne pousse, ne modifie `<runtime>`/`reprise/`,
  ni ne tue un autre agent ; leur rapport est VÉRIFIÉ (fichier:ligne recontrôlés) avant le
  livrable ; après incident : `git status`/`diff`/`log`/`reflog` AVANT toute autre action ;
  ATTENDRE le retour de tous les sous-agents avant le premier rituel de fin (rendre la main
  jusqu'à leur notification, jamais `sleep` ni sondage) — échec ou arrêt de l'un = STOP propre,
  dit au rapport.
- **Preuve d'écran** si le mandat touche l'affichage : captures RÉELLES avant/après dans
  `vault/revues/captures-<id>/`, citées au rapport — un harnais local, un composant isolé ou un
  `git diff` n'en tiennent jamais lieu. Impossible → axe UX **pas GO** (⚠️ « non vérifié à
  l'écran ») + checklist de test pour le fondateur (3 à 5 gestes concrets : URL, action, **ce
  qu'il doit voir**). Gabarit §10.
- **Interdits permanents**, dans tout mandat : jamais de `sleep` (une attente n'est pas un
  travail) ; jamais de tâche de fond vivante à la sortie de la session ; `rm -rf` seulement sous
  forme gardée `"${VAR:?}/${SOUS:?}"`.
- **Pièges du projet** : recopier verbatim `pieges_worktree[]` de la config (outils à relancer,
  contraintes de capture). Liste vide = rien à coller. Worktree neuf : préparation par
  `scripts/preparer-worktree.sh` (livré par M0302), hors du dépôt, dans `chemins.worktrees_hors_depot`
  (absent = `~/.harnais/worktrees/<projet.nom>/`).
- **`CLAUDE.md` imbriqués et rules à `paths:`** : ils font foi (§12). Un mandat qui change un
  comportement décrit dans l'un d'eux le met à jour **dans le même commit, en REMPLAÇANT le texte
  périmé, jamais en ajoutant** (consigne : gabarit §13).
- **Jamais d'état dans un fichier chargé d'office** (`CLAUDE.md`, rules, `SKILL.md`) : compteurs,
  `Mxxxx`, dates, « mesuré par », historique → `vault/reprise/etat-technique.md`, lu à la demande
  (§12). Le hook `pre-commit` borne leur taille et leur largeur de ligne : un refus se règle par la
  purge, jamais en relevant le seuil.

Rituel de DÉBUT : `statut: en-cours` À LA RACINE du frontmatter, avant tout.

Rituels de FIN, dans l'ordre : réponse dans le fichier d'échange → `vault/reprise/Fxx.md` →
événements dans `events.jsonl` + `state.json` (écrits, **jamais committés**, §10) →
`CARTOGRAPHIE_BRANCHES.md` si les branches ont changé → commit(s) des autres fichiers avec
`<trailer>`, **vault compris : obligatoire (§8)** → push + `git ls-remote origin refs/heads/<branche>` **collé** → footer de statut
(gabarit §4, dernière ligne du rapport) → **`statut: reponse-disponible` posé EN DERNIER GESTE** —
c'est ce qui ferme la fenêtre pour le superviseur (§5).

**Dernier geste ORCHESTRATEUR, à la pose** (gabarit §6) : `statut: pret-a-lancer`, APRÈS les
registres — rien à donner au fondateur, le superviseur lance en quelques secondes. Exception
manuelle (mandat lancé à la main) : garder `statut: prompt-disponible` + bloc de lancement classique
(gabarit §5) ; re-présenté, panneau ⚠️ obligatoire collé au-dessus — **par défaut un bloc ne se
redonne jamais**.

## 4. Barème modèle/effort — à appliquer À CHAQUE mandat

Le barème vit dans `modeles.bareme` de la config, pas ici. Sa forme : une ligne par classe de
tâche, du mécanique léger à la conception critique irréversible. Règles invariantes :

- Le superviseur est un **passe-plat** : il exécute la `commande:` du mandat telle quelle
  (gabarit §1) et n'en contrôle que la forme. Le choix du modèle et de l'effort est le tien.
- `modeles.refutes[]` liste les valeurs essayées et rejetées par l'outil : ne jamais les reproposer.
- Par défaut, `modeles.bareme.defaut` ; une autre classe ou un effort supérieur se justifie en une
  ligne dans le mandat.
- Un doublage adversarial et une conception irréversible ne se font jamais au rabais : la rigueur
  vient du mandat et du doublage, pas de l'effort.

## 5. Superviseur — le bureau autonome

Un démon déterministe (composant `superviseur/`, **pas un LLM**, zéro dépendance) lance chaque
mandat `pret-a-lancer`, écoute les événements de session, ferme la fenêtre à `reponse-disponible`,
et journalise dans `events.jsonl` — **seules écritures qu'il fait dans le dépôt** (plus, si un `Fxx.md` est écrasé en cours de session, la copie sous `archive/`, `MANDAT_ECRASE`). Autorité complète :
`superviseur/README.md`.

Il ne fait JAMAIS : écrire `state.json`, le carnet, `vault/reprise/`, ni le frontmatter d'un
`Fxx.md` ; relancer au-delà du plafond de relances **cumulées** ; envoyer un signal hors du panneau
d'un mandat rendu. À la fin d'un mandat rendu, il termine les processus de SON panneau (SIGTERM,
puis SIGKILL) — ceux de son terminal, et ceux que la session a lancés hors du terminal, reconnus au
marqueur du panneau dans leur environnement —, ferme le panneau, réessaie, puis alerte. Jamais un
processus choisi par son nom, son port ou son dossier, jamais sans l'une de ces deux preuves. Le
bureau naît avec le panneau du premier mandat et se ferme quand il ne porte plus de mandat et
qu'il n'y reste que des shells au repos.
Toute incohérence devient un `MANDAT_SUSPECT` — **jamais une réparation, jamais un silence**.

Libérer un `MANDAT_SUSPECT` : **frontmatter D'ABORD** (`statut` hors `pret-a-lancer`), **verrou
ENSUITE** (gabarit §7). L'ordre inverse relance le mandat au cycle suivant.

Le fondateur ne déplace ni ne ferme JAMAIS l'espace de travail d'un mandat actif (→ SUSPECT).

Pièges structurels, vrais sur toute machine : les autorisations système sont liées au **binaire
exact** de l'interpréteur (une mise à jour le retire en silence) ; les hooks sont portés par le
fichier de réglages dédié du superviseur, **jamais par les réglages du dépôt** ; un agent ne peut
pas s'auto-terminer par un simple texte, seul un observateur externe le peut.

## 6. Réveil de l'orchestrateur

Chaque réveil porte un **nom unique et daté** : `Réveil <projet.nom> AAAA-MM-JJ HH:MM` pour le
réveil court, `Filet <projet.nom> AAAA-MM-JJ HH:MM` pour le filet — date et heure de **tir** prévues,
horloge `projet.timezone`. Le **message** de chacun **contient son nom exact** — c'est par lui qu'on
le retrouve pour le supprimer, y compris parmi les tâches déjà tirées.

Tant qu'un mandat tourne : programmer un réveil à `+15 min` (gabarit §8). **Au tir, premier geste** :
supprimer le réveil qui vient de tirer, retrouvé par son nom exact dans la liste des tâches
planifiées — **avant tout autre travail**, jamais le modifier (une modification demande une
approbation humaine et bloque la boucle). Puis lire les frontmatters des `Fxx.md` actifs + la fin
d'`events.jsonl` ; `reponse-disponible` → traiter (§7) puis poser le suivant (§3) ; sinon, une ligne
au fondateur. Puis, **seulement s'il faut continuer**, créer UN nouveau réveil. Git en lecture
toujours avec `--no-optional-locks`, jamais `git status` nu.

Invariant : **au plus un réveil court + un filet vivants** par fenêtre d'orchestration ; filet à
`+60 min` si le réveil court échoue, supprimé et recréé (jamais deux filets vivants) quand le réveil
court tire et qu'on en recrée un.

Fin de tâche : **zéro réveil de cette fenêtre d'orchestration** dans la liste des tâches planifiées —
le vérifier et le dire au fondateur en une ligne.

Périmètre : on ne supprime que ses propres réveils — jamais une tâche planifiée récurrente, ni celle
d'une autre fenêtre ou d'un autre projet, sans accord explicite du fondateur.

## 7. Traiter un rapport de fin

Footer de statut d'abord, **puis** la narration — contre-vérifier SHA, refs et fichiers cités ; une
contradiction footer/narration = STOP, ne pas fermer la fenêtre. Mise à jour de `00_INDEX.md` + du
carnet À CHAQUE événement (prompt prêt, rendu ✅/⚠️/⛔, merge soldé avec SHA, décision du fondateur
**verbatim et datée** — une décision actée ne se re-débat pas sans demande explicite).

Relecture au niveau du mandat OBLIGATOIRE avant tout merge (§9). **Une réserve bloquante n'autorise
PAS le merge** : correctif d'abord, re-relecture selon le niveau, puis merge — sauf acceptation
explicite et datée du fondateur. Une remarque ne bloque rien : elle est en issue. Mandat suivant
posé aussitôt (§3).

Deux rapports contradictoires (« poussé » vs « pas poussé ») = course probable : vérifier l'état
réel, **jamais refaire une opération sur une simple affirmation**.

## 8. Git — règles de sang

**Orchestrateur : ZÉRO git-écriture, jamais** — les sessions terminal font le git ; lecture toujours
`git --no-optional-locks`. Sessions : `git add` CIBLÉ, jamais `-a`/`-A`.

Signature : `<trailer>`, message composé manuellement (heredoc), jamais le trailer par défaut de
l'outil ; ne jamais présumer un rituel « acté » appliqué — vérifier le message RÉEL
(`git log -1 --format=%B <sha>`) au moins une fois par mandat.

Base de branche : toujours depuis `origin/<principale>` à jour, avec la garde « origin/<principale>
doit contenir tel SHA ». Base périmée = **MERGE de `<principale>` dans la branche, jamais de
rebase** (les doublages référencent des SHA existants).

Toute branche créée ou avancée est poussée, preuve `git push` + `git ls-remote` collée au rapport —
même un STOP en cours de route pousse le travail déjà commité, **sauf** si le STOP porte lui-même
sur un état git incertain (alors NE RIEN pousser, et le signaler).

**Vault : commit OBLIGATOIRE à la fin de chaque mandat, push au choix du projet.** Avant
`reponse-disponible`, tout fichier nouveau ou modifié sous `<vault>` (rapport, reprise, archives,
décisions, leçons) est committé, `git add` ciblé — sauf les registres de `<runtime>` (hors git, §10)
et le `Fxx.md` vivant d'une autre fenêtre. Preuve collée au rapport : `git status --short -- <vault>`
ne liste plus aucun de ces fichiers. Un vault non committé est du travail perdu ou invisible.
Le push de ces commits suit `git.pousser_vault` (§12) : `true`, ou clé absente → push +
`git ls-remote` collé ; `false` → aucun push exigé pour un commit qui ne touche que `<vault>`, et le
rapport l'écrit (« vault committé, non poussé : `git.pousser_vault: false` »). La clé ne règle que
cela : une branche de code reste poussée. En mode `pr` (ci-dessous), ce commit va dans la branche du
mandat et part avec la PR, jamais sur `<principale>`. En mode `direct`, **avant tout merge vers
`<principale>`, la principale locale et `origin/<principale>` sont identiques** : avec `false`, les
commits du vault en attente sont poussés d'abord — sinon le scratch diverge de la racine et le
`ff-only` est refusé.

Merge vers `<principale>`, selon `git.mode_fusion` (config ; absent = `direct`) :

- **`direct`** (inchangé) : scratch détaché + `merge --no-ff`, tests **sur ce résultat**, PUIS
  `merge --ff-only` depuis la racine + push + `ls-remote` collé (bloc exact : gabarit §9). Refus
  `ff-only` = rapport, **jamais de force** ; si `origin/<principale>` a bougé entre le scratch et le
  `ff-only`, refaire le scratch UNE fois.
- **`pr`** : **aucun push sur `<principale>`**. Branche du mandat poussée → `gh pr create` → rapport
  de doublage `verdict: GO` committé SUR la branche → `gh pr merge --auto --<git.fusion_pr.methode>
  --delete-branch --match-head-commit <HEAD>` (`<HEAD>` = commit du rapport, qui cite le tip doublé ;
  `auto_merge: false` → PR ouverte sans `--auto`, fusion par Malik) → GitHub fusionne quand
  `harnais-ok` est vert (bloc exact : gabarit §9). Refus ou `harnais-ok` rouge = rapport, jamais de
  contournement. Le hook `pre-push` du harnais refuse tout push sur
`<principale>` qui introduit un merge sans rapport de doublage `verdict: GO` citant le tip mergé,
ou dont le `niveau:` ne couvre pas les chemins touchés (§9).

JAMAIS `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la
racine partagée. `git commit -- <chemins>` explicite toujours (index partagé) ; commit AVANT toute
mutation ; `.git/index.lock` orphelin sans git actif → `rm` + journal.

Conflits : fichiers générés → on régénère ; additif attendu → « garder les deux » + hunk cité au
rapport ; **tout le reste → STOP**.

Tout merge touchant un chemin listé dans `gardes_deploiement[]` exige la preuve de déploiement
définie par cette garde (gabarit §11) avant de déclarer `PROJECT STATUS: INTEGRATED`. Cible
injoignable → rien de forcé : consigner le SHA en attente dans `state.json.projet`, le statut reste
`NOT_INTEGRATED`.

## 9. Doublage — la méthode trio

**Niveau de relecture**, choisi par l'orchestrateur à la pose (`niveau:` du frontmatter, absent = N2) :

- **N1 léger** (UX, texte, style, petit composant sans logique métier) : tests et preuve d'écran par
  l'auteur ; relecture par un sous-agent FRAIS de la même session, une passe, rapport court
  (`verdict:`, `tip:`, `niveau: N1`) ; jamais de re-relecture.
- **N2 standard** : doublage indépendant (session ou sous-agent frais), une passe ; au plus une
  re-relecture, et seulement sur réserve bloquante.
- **N3 critique** : tout ce paragraphe, sans allègement (doublage en chaîne, loi des deux patchs).

Plancher : clé `niveaux` de la config (`critiques[]`, `n1_max_lignes`), liste établie et tenue par
le Cowork du projet. Le hook `pre-push` refuse un merge qui touche un chemin critique hors N3, un N1
sans liste ou au-delà du plafond (« passe en N2 »).

**Réserve bloquante ou remarque** (tous niveaux) : une réserve ne bloque que si un test rouge la
prouve (en N3, aussi un axe ⛔ que ce paragraphe exige : budget, preuve d'écran, symptôme non
résolu). Tout le reste est une remarque : issue GitHub ouverte par la session, jamais un nouveau
tour. Le doubleur classe chaque point ; `verdict: RESERVE` exige au moins une réserve bloquante.

Doublage N2/N3 **indépendant** : session séparée ou sous-agent frais, worktree détaché **lecture
seule**, auteur ≠ doubleur. Numérotation `R0xx` continue (`state.json.next_revue_id` relu à l'instant T),
rapport dans `vault/revues/AAAA-MM-JJ-R0xx-<slug>.md`. En mode `direct`, ce rapport est committé dans le scratch de merge, après le
`merge --no-ff` et avant le `ff-only`, pour partir dans le même push que le merge ; en mode `pr`, il
est committé sur la branche du mandat après `gh pr create`, avant `gh pr merge` (gabarit §9).

Garde d'entrée : « les fichiers de la branche sont **intacts** côté `<principale>` depuis le
merge-base » (`git log $(git merge-base origin/<principale> <tip>)..origin/<principale> -- <chemins>`
vide) — **jamais** « `<principale>` n'a pas bougé », qui produit des STOP à tort. Pas de re-rebase :
`merge --no-ff` gère l'historique disjoint. Lire au tip (`git show <sha>:<chemin>`), jamais le
working tree de la racine. Vérifier `git log -1 -- <echanges>/Fxx.md` (un échange jamais committé
est un piège classique). Un STOP en voie de mandat → footer `BLOCKED`, jamais `READY`.

Verdict **par axe**, avec la RAISON vérifiée : ✅ GO / ⚠️ remarque (issue) / ⛔ réserve bloquante.
⛔ → correction puis re-relecture sur le tip corrigé : N2 une seule, N3 **EN CHAÎNE** (R0xx suivant).

**Loi des deux patchs** : 2 correctifs successifs sur la même classe de défaut = le MÉCANISME est en
cause, on reconçoit, on ne patche plus. « Préexistant, hors périmètre » au 3e rapport = mandat dédié.

Le doublage vérifie le **SYMPTÔME signalé à l'origine**, pas seulement la sûreté du code — et si le
mandat touche l'affichage, il exige la preuve d'écran ou la checklist fondateur (§3), et **refait les
captures lui-même**.

Il vérifie aussi les `CLAUDE.md` imbriqués et les rules à `paths:` (§12) : comportement touché par
le diff → mis à jour dans le même commit, par remplacement ; `CLAUDE.md` de dossier → commandes et
pièges, rien d'autre (question verbatim : gabarit §13).

Il **contrôle le budget des fichiers chargés d'office** (§12) : sortie collée de
`sh "$(git config core.hooksPath)/pre-commit" --controle <tip> --base <merge-base>` (taille et
largeur de ligne, une ligne OK / ATTENTION / BLOQUE par fichier) — un `BLOQUE`, ou de l'état ajouté
dans l'un de ces fichiers, = axe ⛔.

## 10. Registres — aucun n'est une preuve

`vault/reprise/CARNET_DE_BORD.md` (écrasé, court, lu en premier) · `00_INDEX.md` (rotation, garde
`pre-commit` qui refuse au-delà de `rotation.index_limite_octets`) · `<runtime>/state.json`
(compact, écrasé, `last_event` ≤ 200 caractères, clés hors schéma dans `projet{}`) ·
`<runtime>/events.jsonl` (append-only, `ts` = horloge réelle du même tour, vocabulaire figé par
`<runtime>/README.md`) · `CARTOGRAPHIE_BRANCHES.md` (vue git pure, régénérée par lecture réelle).

**`state.json` et `events.jsonl` sont HORS GIT** (gitignorés) : écrits à la racine, jamais committés
par personne, donc ils ne bloquent jamais un `ff-only`. Tout le reste du vault est DANS git, committé à
chaque fin de mandat (§8). L'orchestrateur **RELIT** les registres après
chaque rendu (un rituel de fin peut les avoir écrasés).

Tout est une DÉCLARATION à réconcilier contre une preuve réelle avant usage critique (§1) — **jamais
une preuve en soi**.

## 11. Passation

Une session d'orchestration qui s'arrête écrit `vault/reprise/AAAA-MM-JJ-PASSATION-<slug>.md`
(gabarit : `templates/gabarits/passation.md`) : ce qui a été fait · ce qui est en vol · **les
questions ouvertes, non tranchées à la place du fondateur** · les premiers gestes de la reprise dans
l'ordre · le prompt de démarrage de la session suivante, à copier-coller tel quel. Préambule
obligatoire : en cas d'écart entre ce document et les registres, **les registres gagnent**.
Ligne obligatoire avant de clore : **zéro réveil vivant** de cette fenêtre d'orchestration — vérifié
dans la liste des tâches planifiées, nom par nom, et dit au fondateur en une ligne.

## 12. Configuration du projet — où vivent les paramètres

Toute valeur propre au projet est lue dans **`<racine>/config/harnais.json`** — source unique, lue
aussi par le superviseur. Documentation clé par clé et exemple rempli : `config.example.md`, qui
reste dans le **canon** du harnais et n'est pas copié dans les projets (son exemple parle d'un autre
projet que le tien : le lire, jamais le recopier).
La convention ne code en dur aucun chemin, aucune branche, aucun modèle, aucune machine.

Horloge : `TZ=<projet.timezone> date +%Y-%m-%dT%H:%M:%S%z`, interrogée dans le tour où elle est
écrite.

**`CLAUDE.md` de dossier = commandes et pièges** (décision du fondateur, 2026-09-23, #22). Il ne se
charge que si la session travaille dans ce dossier : il sert à cibler plus finement, pas à déplacer
du volume hors de la racine.

1. Le `CLAUDE.md` d'un sous-dossier contient les **commandes et les pièges d'environnement** de ce
   dossier, **rien d'autre**.
2. Doctrine → la rule à `paths:` qui couvre ce même périmètre (`.claude/rules/*.md`).
3. Historique et compteurs datés (paragraphes `Mxxxx`, « au JJ/MM : N tests ») → `vault/`.
4. Ce qui coûte de l'argent ou touche une ressource irréversible **reste à la racine** : un
   `CLAUDE.md` imbriqué ne se charge pas quand la session agit en Bash sans lire de fichier du
   dossier, un garde-fou ne peut donc pas dépendre de ce chargement.

**Source de vérité** : tout `CLAUDE.md` imbriqué et toute rule à `paths:` font foi, au même titre
que le `CLAUDE.md` racine. Un mandat qui change un comportement décrit dans l'un d'eux le met à jour
**dans le même commit, en remplaçant le texte périmé** ; le doublage le vérifie (§9).

**Fichiers chargés d'office : un budget, et jamais d'état** (#113). `CLAUDE.md` (racine ou
imbriqué), rules, `SKILL.md` des skills, index et carnet de reprise sont relus à chaque démarrage
de session. Constat de terrain : les mandats n'y ajoutaient pas de lignes, ils allongeaient les
puces, et la consigne « tenir à jour » en était la cause. La règle tient donc par une barrière :

1. **Remplacer, jamais ajouter.** Mettre un de ces fichiers à jour, c'est réécrire la règle périmée
   à sa place : il ne grossit pas parce qu'un mandat est passé.
2. **Jamais d'état** dans un `CLAUDE.md`, une rule ou un `SKILL.md` : ni compteur, ni `Mxxxx`, ni
   date, ni « mesuré par », ni historique. L'état vit dans `vault/reprise/etat-technique.md`,
   **lu à la demande**.
3. **Budget** : bloc `instructions` de la config (`max_caracteres`, `max_ligne`, `fichiers`). Le
   hook `pre-commit` refuse le dépassement de taille et toute ligne trop large ajoutée ou
   modifiée ; `pre-merge-commit` rejoue le contrôle sur un merge. Un refus se règle par la purge :
   jamais en relevant le seuil, jamais par `--no-verify`.
4. **Le doublage contrôle** taille et largeur (§9).

Consigne, question, commande de contrôle et exemple : gabarit §13.

## 13. Où lire ensuite

`references/gabarits.md` — à CHAQUE pose de mandat ou traitement de rapport, blocs verbatim.
`templates/gabarits/` — les documents de départ (mandat, doublage, passation, décision, leçon).
`superviseur/README.md` — autorité du superviseur, machine d'états, procédure de libération.

**Source de vérité unique** : la convention vit dans le dépôt du projet. Un skill de compte n'en est
qu'un **POINTEUR** (texte exact : `references/gabarits.md` §12), qui STOP si le dépôt n'est pas
accessible, et qui ne change que si le CHEMIN de la source change — aucune version ne s'y grave.

## 14. Équiper un projet — une commande

Un projet s'équipe par **un seul moteur** : `~/.harnais/canon/bin/harnais equiper <dossier>`, lancé au
Terminal, par la skill `harnais`, ou par le guichet (Cowork dépose une demande, le superviseur lance le
moteur en processus enfant). En moins de 3 minutes, sans question ni accord : pose idempotente, hooks git
actifs, confiance de Claude Code pré-accordée, inscription au superviseur (relu à chaud) ; un dossier qui
range plusieurs dépôts les équipe tous. **Le moteur ne committe rien** : arbre sale compris, il pose
par-dessus sans toucher aux changements de l'utilisateur. Le **premier mandat** du projet committe les
fichiers du harnais, en pathspec explicite (Mission 0, gabarit §2). Principe : sur le périmètre de
l'utilisateur, on **informe**, on ne bloque pas. Autorité : `superviseur/README.md` § Guichet d'équipement.

**`F99` et `M0000` restent réservés** (essai du guichet jusqu'à v1.4.4, retiré en v1.5.0) : jamais
alloués par l'orchestrateur ; `<echanges>/F99.md` et ses archives restent ignorés par git (fragment
`.gitignore`), et la réconciliation ignore les événements `f=F99` d'un projet équipé avant v1.5.0.
