---
date: 2026-10-18
tags: [vision, genese, philosophie, cible, lecture-profonde]
session: F01
mandat: genese-et-vision-de-tom
mandat_id: M0016
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'high' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol-scratch-M0016
branche: docs/genese-vision-2026-10
niveau: N2
derniere_maj: 2026-10-18T13:51:25+0200
---

# F01 / M0016 — Genèse et vision de ToM : lecture profonde de 8 mois de projet, évolution point par point, logique, validation

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, index `tom` ; rafraîchir avant de commencer : `qmd --index tom update && qmd --index tom embed`, jamais `qmd update` seul). `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI.**
> Effort `high` (hors barème `medium`) : conception de fond, structurante pour tout le projet — justifié par le fondateur le 2026-10-18.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Pourquoi ce mandat (autonome)

ToM existe depuis ~8 mois (premiers documents fin janvier 2026, premier commit le 2026-02-02, ~933 commits). La vision a beaucoup évolué : livre blanc v0.1, « Step 2 », livres blancs v1/v2, `design-decisions`, ADR, `docs/archive/`, ~100 plans dans `docs/plans/`, `VISION-CONSOLIDEE-2026-07-16`, `CARTE-rendezvous-tournant` (24/07), `docs/MISSION.md`, `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md`. **Aucun document ne retrace cette évolution ni n'en explique la logique.** Le fondateur n'a pas rouvert le projet depuis longtemps et l'orchestrateur le reprend : il faut un socle commun, validé, avant de reprendre le développement.

**Ce n'est PAS un travail de copie.** Le fondateur, verbatim (2026-10-18 13:06) : « C'est un réel travail de fond, de lecture profonde et global, de comparer la vision il y a 6 mois et son évolution point par point et explique la logique qui se cache derrière et surtout la valider. Car si c'est que de la copie c'est pas ce que j'attends. C'est lecture, croiser les sources, priorise la plus récente (en stipulant pourquoi ce choix ou cette évolution, à cause du technique, ou autre chose) … c'est un vrai travail de brainstorming si on veut reprendre convenablement ce projet et le faire passer un step. »

## Paroles du fondateur du 2026-10-18 (source la plus récente, verbatim — à intégrer comme telles)

- 11:24, sur les rôles : « on va pas dire que tous les noeuds sont des menteurs. Il faut agir avant : le travail de véracité de l'information émanant d'un noeud doit être validé avant même son émission. C'est pour cela qu'il faut un mécanisme de multi signature. J'imagine qu'un noeud dit "je suis relais" … ses pairs (congénères qui l'entourent, au nombre de 7, comme les étourneaux) acquiescent. Soit le rôle est donné par plusieurs et donc juste repris (le hash) par l'acteur qui diffuse son rôle, soit avant la diffusion, une validation par les noeuds qui entourent. Si un noeud agit mal malgré cela, alors il a sa note qui se dégrade, et donc hors jeu, ou des tâches sans importance. » ; « petit c'est dur voire impossible ; une fois le réseau mature et étendu, c'est magiquement redoutable. On ne pense pas ce réseau pour 10 personnes ni même pour 1000 mais pour plusieurs millions voire milliards. » ; « le mécanisme global doit être fluide, robuste, et naturellement beau, plus il y a de monde plus le réseau est fort. »
- 13:49, sur les plateformes : « On ne vise pas iOS. C'est une plateforme qui risque juste d'héberger le SDK (couche de communication) pour des centaines d'apps. On ne vise pas seulement iOS : on vise à être une couche protocolaire au même titre que TCP/IP. »
  → Conséquence de méthode : **une contrainte d'une plateforme (iOS, Apple TV, NAS…) n'est jamais une cause de vision**, au plus une contrainte d'implémentation locale. Si une évolution de la vision s'explique par une telle contrainte, tu le dis et tu la marques ⚠️ (la vision a peut-être été déformée par le terrain de test).

## Sources

1. **Sources fondatrices hors dépôt**, déposées pour toi (NON versionnées, chemin ignoré par git — ne les committe jamais, ne les copie pas ailleurs dans le dépôt) : `vault/runtime/log/sources-fondateur/` → `whitepaper-v0.1-FR.{pdf,txt}` (PDF créé le 2026-01-30) et `Step2-TheOpenMessaging.{pdf,txt}` (« Step 2 », document créé le 2026-01-30). Ce sont les plus anciennes traces de la vision.
2. **Le dépôt**, lu sur `origin/main` (worktree, jamais la racine) : tous les `.md` utiles — `docs/` (dont `archive/`, `plans/`, `gitbook/`, `spec/`, `redteam/`, `handoffs/`), `_bmad-output/`, `vault/` (décisions, reprise, revues, `30-discoveries.md`, `40-roadmap.md`, `00-brief.md`), `CLAUDE.md` racine et imbriqués, `README*`. **Exclus** : `_bmad/` (cadre BMAD, pas de la vision), `.claude/` et `.github/skills/` (outillage).
3. **L'histoire git** : dates de création et de dernière modification de chaque source (`git log --follow --format='%ad' --date=short -- <f>`), et messages de commits qui portent un changement de cap (`git log --format='%ad %h %s' --date=short` filtré par mots-clés : vision, décision, LOCKED, ADR, rôle, témoin, PoP, purge, amorçage, n0, relais, subnet…).
4. **Le code**, seulement pour l'écart vision ↔ réalité : partir de `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md` (déjà cité fichier:ligne) et compléter par sondage ciblé, jamais une relecture exhaustive.

## Gardes de précondition

1. `git -C /Users/malik/Documents/tom-protocol fetch origin` ; `git merge-base --is-ancestor aaeee846a1c6ed42d1f9d0f6e4f132cd7cc97473 origin/main`. Sinon STOP.
2. `grep -h '^statut' vault/echanges/F02.md vault/echanges/F03.md` : aucune autre fenêtre `en-cours`. Sinon STOP.
3. `ls vault/runtime/log/sources-fondateur/` liste les 4 fichiers. Sinon STOP.

## Méthode — 4 phases

**Phase 0 — préparation (toi).** Rituel de début `statut: en-cours`. Worktree hors dépôt `/Users/malik/Documents/tom-protocol-scratch-M0016` sur une nouvelle branche `docs/genese-vision-2026-10` depuis `origin/main` ; `pnpm install --frozen-lockfile` puis `test -x .husky/_/pre-push && echo HOOKS_OK` (sans `HOOKS_OK`, ni commit ni push). Aucun build cargo dans ce mandat.

**Phase 1 — inventaire daté (toi).** Tableau de toutes les sources de vision : chemin, première et dernière date git, taille, nature (fondateur / livre blanc / décision / plan / revue / journal / ADR / code-commentaire), et **statut** (en vigueur / remplacée par X / abandonnée / brouillon). Les deux sources fondatrices y figurent avec leur date. Fichier : `vault/decisions/genese/00-inventaire.md`.

**Phase 2 — essaim de lecture : agents Haiku.** Lance **~20 sous-agents FRAIS en parallèle, modèle `haiku`** (outil Agent, `model: "haiku"`, lecture seule), chacun sur une tranche disjointe de l'inventaire (par dossier et par période ; les gros dossiers comme `docs/plans/` sont coupés en 4 à 6 tranches par date ; un agent dédié à l'histoire git ; un agent dédié aux sources fondatrices). Chaque agent rend, pour CHAQUE affirmation de vision qu'il trouve, une ligne :
`thème | affirmation (≤ 2 lignes, citation courte) | fichier:ligne | date de la source | statut dans la source (affirmé / proposé / abandonné / question ouverte) | indice de la cause si la source en donne une`.
Thèmes (un agent peut en ouvrir un nouveau s'il le justifie) : (1) finalité, échelle, couche protocolaire universelle ; (2) plateformes, SDK, formes de livraison, nœuds 24/24 vs sessions courtes ; (3) rôles et leur attribution (PoP, quorums tournants, annonce à l'avance vs imprévisible, cosignature) ; (4) observation, témoins, murmuration ~7, gossip ; (5) note, réputation, jeton « grain de riz », usage/contribution ; (6) anti-spam « arroseur arrosé » ; (7) amorçage, découverte, rendez-vous, zéro tiers, zéro dépense ; (8) relais et transport ; (9) purge, TTL, backup « virus », état présent, L1 miroir/ancre ; (10) subnets, cercles réels, vases communicants ; (11) chiffrement et identité ; (12) invisibilité et universalité ; (13) valeur, double dépense, engagements de wallet ; (14) consensus, L1, genèse glissante.
Consigne à recopier dans le prompt de chaque agent : lecture seule, aucun fichier écrit, aucune commande git autre que `git show`/`git log`, aucune IP ni secret recopié (remplacer par `<IP>`), rendre la liste brute sans synthèse. Agrège tout dans `vault/decisions/genese/10-extraits.md` (trié par thème puis par date).

**Phase 3 — synthèse, toi (Opus), thème par thème.** Tu lis **toi-même, en entier**, les sources pivots : les 2 sources fondatrices, `docs/tom-whitepaper-v1.md`, `docs/gitbook/whitepaper-v1.md` et `-v2.md`, `docs/gitbook/design-decisions.md`, `_bmad-output/planning-artifacts/design-decisions.md`, `docs/plans/VISION-CONSOLIDEE-2026-07-16.md`, `docs/plans/CARTE-rendezvous-tournant.md`, `docs/plans/charte-cibles-agressives.md`, `docs/MISSION.md`, `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md` (et toute autre source que l'inventaire révèle pivot). Pour chaque thème, une section :
- **Au départ** (janvier–février 2026) : ce que disaient les premières sources.
- **Trajectoire** : les jalons datés, chacun avec sa preuve (fichier:ligne ou SHA).
- **Aujourd'hui** : la position en vigueur. Règle : **la plus récente l'emporte**, mais tu dis **pourquoi** elle a gagné, et si elle **régresse** par rapport à un principe fondateur, tu ne l'aplatis pas : tu le signales.
- **Pourquoi ça a bougé** : cause classée — *technique* (limite découverte en codant), *terrain* (mesure, incident), *sécurité* (red team, audit), *économie* (zéro dépense), *décision du fondateur*, *contrainte de plateforme* (⚠️ voir plus haut), *inconnue* (dis-le).
- **Verdict sur la logique** : ✅ tient / ⚠️ tension / ⛔ contradiction / ◻ trou — avec la raison. Vérifie aussi la cohérence **entre** thèmes (ex. rôles ↔ note ↔ témoins ↔ échelle).
- **Écart avec le code** : une ligne, avec fichier:ligne (s'appuyer sur le document ADN du 17/10, compléter par sondage).
- **Pour passer un step** : 1 à 3 pistes, étiquetées **proposition** (jamais présentées comme décidées).
Chaque affirmation que tu reprends d'un extrait Haiku est **recontrôlée par toi** à son fichier:ligne avant d'entrer dans la synthèse (un extrait non recontrôlé ne sert pas de preuve).

**Phase 4 — contradicteur (doublage N2).** Un sous-agent FRAIS, modèle `opus`, lecture seule, sans voir ton raisonnement : il tire au hasard ≥ 15 % des citations du document et les recontrôle ; il attaque la logique de chaque verdict ✅ (cherche le contre-exemple) et vérifie qu'aucune évolution n'est expliquée sans preuve. Rapport `vault/revues/2026-10-18-R015-genese-vision.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40 du commit doublé>`, `niveau: N2`). Réserve bloquante seulement si une citation est fausse ou une conclusion contredite par une source ; tu corriges puis une seule re-relecture.

## Livrables (tous sous `vault/decisions/`)

- `2026-10-18-GENESE-vision.md` — le document principal : en tête, **la vision de ToM en une page** (ce qu'est ToM, pour qui, à quelle échelle, ses principes non négociables), puis les 14 sections thématiques, puis une section **« Cohérence d'ensemble »** (le système tient-il comme un organisme ? où sont les tensions ?), puis **« Questions que seul le fondateur peut trancher »** (numérotées, chacune avec les options et ta recommandation argumentée).
- `genese/00-inventaire.md`, `genese/10-extraits.md` (annexes).
- Écriture : français simple, phrases courtes, aucun jargon gratuit ; étiquettes `[VÉRIFIÉ]` (vu à fichier:ligne) / `[HYPOTHÈSE]` ; **aucune IP, aucun nom d'hôte, aucun secret** (certaines sources en contiennent : `vault/30-discoveries.md` — remplace par `<IP>`).

## Budget de temps

Le superviseur coupe une session au bout de 240 min. Vise **≤ 200 min**. Si tu n'as pas fini à 180 min : livre ce qui est fait (sections terminées complètes, les autres listées « non traitées »), c'est un STOP propre — jamais un document qui fait semblant d'être complet.

## Intouchables

Tout fichier hors `vault/decisions/` et `vault/revues/` (et `vault/reprise/F01.md`, `vault/echanges/archive/` pour les rituels) ; `docs/MISSION.md` (lu, jamais modifié) ; `vault/runtime/log/sources-fondateur/` (jamais committé) ; la racine du dépôt (seul `git fetch` permis) ; `~/.harnais/` ; les `vault/echanges/F0x.md` vivants (jamais committés). Aucun code modifié.

## Règles des sous-agents (verbatim)

(1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre. **Aucun sous-agent ne lance de build ou de test cargo**, aucun n'écrit de fichier.

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée `"${VAR:?}/${SOUS:?}"` ; jamais `--no-verify` ; jamais de force-push ; aucun push sur `main` ; pas de trailer `Co-Authored-By` (`.claude/CLAUDE.md` §13).

## Fusion (mode `pr`)

Commit `docs(vision): genese et vision de ToM -- lecture profonde` (pathspec explicite : les fichiers de `vault/decisions/` créés). Push (`GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30'`, vault seul → la CI locale doit se sauter, `time` collé) → `gh pr create -B main -H docs/genese-vision-2026-10 -t "M0016 -- genese et vision de ToM"` → commit du rapport R015 sur la branche → push → `gh pr merge docs/genese-vision-2026-10 --auto --merge --delete-branch --match-head-commit <HEAD>` → fusion attendue (`gh pr view --json state,mergedAt`, borne 30 min), sortie collée. Avant chaque commit : `git status --short` et `git check-ignore vault/runtime/log/sources-fondateur/*` collés (les sources ne partent jamais).

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0016`) : ce que tu as lu (nombre de sources, nombre d'agents, nombre d'extraits, nombre recontrôlés), les 5 constats les plus importants en clair, les questions au fondateur (renvoi au document), puis le footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F01"`, `mandat: "M0016"`) et `vault/runtime/state.json` (`windows.F01` seulement, relu juste avant d'écrire) : jamais committés.
4. Reprise : nouvelle branche `docs/reprise-F01-M0016` depuis `origin/main` à jour (worktree hors dépôt `-reprise`, même préparation), commit pathspec `vault/reprise/F01.md` + `vault/echanges/archive/2026-10-18-F01-M0015-racine-par-pr-et-pre-push-vault-seul.md` (copié depuis la racine) ; `docs(vault): reprise F01 M0016` ; push ; `ls-remote` collé ; PR + auto-merge ; fusion attendue (borne 30 min).
5. **Dernier geste** : `statut: reponse-disponible`.

## Rapport M0016


**Livré et fusionné** : PR #77 → `main` au merge `91b0c33` (2026-10-18 12:42 UTC). Document principal `vault/decisions/2026-10-18-GENESE-vision.md` ; annexes `genese/00-inventaire.md`, `genese/10-extraits.md` ; doublage `vault/revues/2026-10-18-R015-genese-vision.md` (GO sur `ebca4d8`). Aucun code modifié. Durée ≈ 55 min sur 240.

**Ce qui a été lu**
- 2 sources fondatrices en entier (livre blanc v0.1, Step2) + les pivots de phase 3 en entier ; 264 sources inventoriées.
- Essaim : 23 lots, 24 exécutions Haiku (A21 refusé une fois au plafond de 20 agents simultanés puis relancé ; A23 lancé deux fois par erreur, les deux listes gardées ; A14 a lancé une fois `git status --short`, lecture seule, hors consigne).
- 3 530 extraits agrégés (non vérifiés ligne à ligne, signalé en tête de l'agrégat).
- **189 vérifications fichier:ligne par moi**, dont **70 extraits Haiku recontrôlés** ; aucun extrait non relu ne sert de preuve. Contradicteur : 58 citations tirées au tour 1 (32 %), 16 au tour 2.

**Les 5 constats clés**
1. **La vision converge vers un seul moteur** : le travail constaté par ~7 voisins (présence, rôle co-signé, note, rendez-vous, un jour la double dépense). Les paroles du 18/10 (validation *avant* émission) ferment la boucle et réconcilient les deux textes de janvier : rôles « annoncés » (Step2 l.106-108) et « imprévisibles » (wp-v1 l.211-217) → imprévisibles jusqu'au tirage, co-signés ensuite.
2. **Le code fait l'inverse sur les rôles** : un rôle annoncé est appliqué tel quel (`runtime/state.rs:943`) ; une annonce d'un pair inconnu l'inscrit même `Online` (`:953-957`), contre ADR-011.
3. **Le « virus » ne se propage pas en production** : `replicate_to` (`backup/coordinator.rs:218`) n'a aucun appelant hors tests ; le gardien est l'émetteur. README et MISSION l.47 sont en avance sur le code. Idem `record_bytes_relayed` : terme du score inerte.
4. **« Plus de monde = plus fort » n'est vrai que sous conditions** : contre un attaquant qui grossit avec le réseau, la masse seule n'aide pas (L1-002 l.89-98). Formulation proposée : « la masse amplifie, les règles locales protègent ».
5. **La plateforme a façonné les pratiques, pas la vision** — sauf : SDK Apple seul, outils Freebox, échantillon d'un seul foyer. Trouvé au doublage : les événements internes (rôles, backup, anti-spam) traversent le FFI jusqu'à l'app (`tom-protocol-ffi/src/types.rs:489-560`), contrairement à CLAUDE.md limite #8 → **issue #78** (hors périmètre).

**Questions au fondateur** : F1 à F9 en fin du document principal (rôles imprévisibles/annoncés, 7 voisins cible ou plafond, formulation de la masse, monnaie « à terme » de Step2, exception TTL pour L2, propagation du virus, « exclu temporairement » de wp-v1, formule équilibre ≈ 0, premier SDK non-Apple). Chacune avec options et recommandation.

**Incident et leçon** : le premier commit de R015 a échoué (`git commit -- <chemin>` sur un fichier non suivi : il faut `git add` avant) ; la commande enchaînée a poussé sans nouveau commit, ce qui a lancé la CI locale complète ; je l'ai arrêtée avant l'auto-merge (qui aurait fusionné sans le rapport), puis `git status/log/reflog` : rien perdu, aucun processus restant, PR intacte. Leçon : ne jamais enchaîner commit → push → merge dans une seule commande ; vérifier le SHA de HEAD entre chaque étape. Le `reflog` montre aussi un `reset: moving to HEAD` antérieur à mes commits (création du worktree), sans effet.
Note : l'index QMD pointait sur la racine (`7bf72ce`), en retard sur `aaeee84` ; toutes les citations ont été relues dans le worktree. Branche distante `docs/genese-vision-2026-10` non supprimée par l'auto-merge (`ls-remote` : `88cd153`).

**Sorties collées**
- `git status --short` avant commit 1 : `?? vault/decisions/2026-10-18-GENESE-vision.md` / `?? vault/decisions/genese/` ; avant commit 2 : ` M vault/decisions/2026-10-18-GENESE-vision.md` ; avant commit 3 : `A  vault/revues/2026-10-18-R015-genese-vision.md`.
- `git check-ignore vault/runtime/log/sources-fondateur/*` (racine) : les 4 fichiers listés (Step2 .pdf/.txt, whitepaper-v0.1 .pdf/.txt) → ignorés, jamais committés.
- Push 1 : `[new branch] docs/genese-vision-2026-10`, `2,871 total` ; push 2 : `8e6f622..ebca4d8`, `2,151 total` ; push 3 : `ebca4d8..88cd153`, `2,257 total` (pre-push : CI locale sautée, vault seul).
- `git ls-remote` : `88cd153f287e64df8b158910a3ab86b2f0ae13be refs/heads/docs/genese-vision-2026-10` ; `91b0c334da9c35d4e9964aef1c5103473b5ad59e refs/heads/main`.
- `gh pr view 77 --json state,mergedAt,mergeCommit` : `{"mergeCommit":{"oid":"91b0c334da9c35d4e9964aef1c5103473b5ad59e"},"mergedAt":"2026-10-18T12:42:14Z","state":"MERGED"}` ; checks : `harnais-ok pass`, `changes pass`, jobs Rust/TS `skipping` (vault seul).

- Reprise : branche `docs/reprise-F01-M0016` @ `18a0f53` (`ls-remote` : `18a0f53473682431ab250bcceda63718a33999e0`), PR #79 fusionnée `{"mergeCommit":{"oid":"be8e18ed9d1ab9e52336cb01ec568c668da35f1d"},"mergedAt":"2026-10-18T12:45:18Z","state":"MERGED"}` ; `events.jsonl` (MERGE M0016) et `state.json` (`windows.F01` = REPONSE) écrits, non committés. Le vérificateur `scripts/verifier-rendu.mjs` n'existe pas dans ce dépôt (non lancé).

Scellé tests : sans objet (mandat sans test)
Contre-témoins : aucun

M0016
Implementation      ✅ vault/decisions/2026-10-18-GENESE-vision.md + genese/ (8e6f622, ebca4d8)
Tests               N/A (documentation seule, aucun code)
Commit              ✅ 8e6f622, ebca4d8, 88cd153
Branch push         ✅ docs/genese-vision-2026-10 @ 88cd153
Review              ✅ R015 N2 GO, tip ebca4d8 (tour 1 RESERVE levée)
Merge main          ✅ 91b0c33 (PR #77)
Main push           ✅ origin/main = 91b0c33
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED
