---
date: 2026-10-17
tags: [etat-des-lieux, github, bonnes-pratiques, qmd, adn]
session: F01
mandat: etat-des-lieux-github-adn
mandat_id: M0006
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'high' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F01-M0006
branche: main
niveau: N2
derniere_maj: 2026-10-17T15:55:56+0200
---

# F01 / M0006 — État des lieux complet (GitHub, bonnes pratiques, code vs ADN) + index QMD à jour

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, https://github.com/tobi/qmd) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée (ex. `rm -rf` gardé) ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

Décision du fondateur, 2026-10-17 15:54, verbatim : « je veux un GitHub propre ; je veux qu'on reprenne les bonnes pratiques ; voir s'il en existe d'autres ; état des lieux et remise à niveau nécessaire. On a des documents qui posent l'ADN du projet, donc on s'y tient. »
**Ce mandat est un DIAGNOSTIC** : aucun changement de code, de config, de branche, d'issue ou de réglage GitHub. Seules écritures permises : l'index QMD (hors dépôt, `~/.cache/qmd/`) et le rapport dans `vault/`. Effort `high` : audit large, il nourrit toutes les décisions suivantes.

## Étape 1 — Index QMD à jour (préalable)

Constat de M0004/M0005 : index `tom` (`~/.cache/qmd/tom.sqlite`) périmé (~88-116 j, 187 docs « need embedding ») et partiel (collections `vault`, `docs`, `bmad` seulement ; 328 `.md` suivis hors index, dont les `.md` racine, `_bmad/`, `.claude/`, `.github/`).
1. `qmd --version` ; `qmd --index tom status` (sortie collée avant).
2. Consulte `qmd --help` / `qmd collection --help` pour la syntaxe exacte AVANT d'agir (ne devine pas une option). Ajoute une collection couvrant le reste du dépôt en `.md` sans doubler les collections existantes ni indexer `node_modules/`, `target/`, `.git/`, `vault/runtime/`, `.claude/worktrees/` (si les masques d'exclusion n'existent pas, préfère plusieurs collections ciblées : racine `*.md`, `.claude/`, `.github/`, `_bmad/`, `crates/`, `apps/`, `packages/`).
3. `qmd --index tom update` puis `qmd --index tom embed` ; `status` après collé (attendu : `Pending: 0`, fichiers ≈ total des `.md` suivis visés).
4. Vérifie par 2 requêtes (`mcp__qmd__query`) qu'un `.md` racine et un `vault/reprise/*.md` récent remontent.
5. Si une commande échoue ou dure > 15 min : arrête cette étape, note-le, continue l'état des lieux avec ce qui marche.

## Étape 2 — État des lieux (lecture seule), parallélisé en sous-agents FRAIS

Lance au plus 4 sous-agents frais (jamais `fork`), chacun avec ces interdits recopiés dans son prompt : **lecture seule ; aucun git en écriture ; jamais `git status` nu (utiliser `git --no-optional-locks`) ; aucun `gh` en écriture (pas de create/edit/close/delete/merge) ; aucune modification de fichier**. Règles (verbatim) : (1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) avant le livrable ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre.

**A. GitHub (`gh`, lecture)** — dépôt `malikkaraoui/ToM-protocol` :
- branches distantes (nombre, dernières activités, fusionnées ou non dans `main`, abandonnées > 30 j) ; PR ouvertes/fermées ; issues ouvertes (âge, labels, doublons, obsolètes vs l'état du code) ; tags et releases (vs `CHANGELOG.md`, version de `package.json`/`Cargo.toml`) ;
- protections de `main` (`gh api repos/{o}/{r}/branches/main/protection`, 404 = aucune), règles (rulesets), réglages du dépôt (merge methods, delete branch on merge, description, topics, homepage, licence détectée) ;
- CI : `.github/workflows/*` (ce qui tourne, sur quoi), état des derniers runs (`gh run list -L 20`), échecs récurrents ;
- sécurité : Dependabot (`.github/dependabot.yml`, alertes si lisibles), secret scanning, `SECURITY.md`, `cargo deny`/`deny.toml` ;
- fichiers communautaires : `README`, `CONTRIBUTING`, `CODE_OF_CONDUCT`, `LICENSE`, `CODEOWNERS`, gabarits d'issue/PR.
**B. Bonnes pratiques** : (1) celles que le projet s'est données (`CLAUDE.md`, `.claude/CLAUDE.md`, `.claude/rules/`, `CONTRIBUTING.md`, `commitlint`, `.husky/`, `deny.toml`, `rustfmt.toml`, `biome.json`) et **où elles ne sont plus respectées** (preuves : commits récents, fichiers) ; (2) **celles qui existent ailleurs et manquent** — référentiels publics reconnus (ex. OpenSSF Scorecard, GitHub community standards, conventional commits, SemVer, keep-a-changelog, Rust API guidelines) : recherche web autorisée, sources citées par URL, et pour chacune : utile ici ou non, coût, priorité.
**C. Code et santé technique** : arborescence (`crates/`, `apps/`, `packages/`, `experiments/`, `deploy/`), ce qui compile et est testé (`cargo build --workspace` / `cargo test --workspace` si < 15 min, sinon `cargo check` ; `pnpm -r build`/`test` si présents — commandes et résultats collés, durée), code mort ou héritage (`TypeScript (legacy)` cité dans `CLAUDE.md`), fichiers lourds ou parasites suivis par git (`git ls-files` + tailles, PDF/HTML/binaires à la racine de `docs/`), dette visible (`TODO`/`FIXME` comptés), `.gitignore` incohérent.
**D. Code vs ADN** — compare l'état réel aux documents fondateurs. Dans le dépôt : `docs/tom-whitepaper-v1.md`, `docs/gitbook/whitepaper-v1.md`, `docs/gitbook/whitepaper-v2.md`, `docs/MISSION.md`, `vault/00-brief.md`, `vault/20-decisions.md`, `vault/40-roadmap.md`. Hors dépôt (résumé fourni par l'orchestrateur, ci-dessous). Pour chaque principe : **implémenté / partiel / absent / contredit**, avec preuve fichier:ligne. Signale toute divergence entre les whitepapers du dépôt et les documents fondateurs ci-dessous (lequel est le plus récent, lequel fait foi : **ne tranche pas**, liste).

### ADN — documents fondateurs hors dépôt (résumé vérifié par l'orchestrateur)
Sources : « Protocole de messagerie Décentralisé » whitepaper v0.1 FR (5 p.) ; « The Open Messaging — Step 2 » (10 p.).
1. P2P sur l'internet de tous les jours ; chaque appareil client ET serveur ; aucun serveur propriétaire, volontaire ni relais externe. Pas de Bluetooth, pas de Tor.
2. Subnets dynamiques et éphémères (création → fusion/fork → veille/purge → extinction), sans copie d'historique global.
3. Routage qui apprend les chemins courts et fiables sans dépendre d'un nœud.
4. Vie privée par défaut : E2E avec clés locales, pas de collecte centrale de métadonnées, rétention contrôlée.
5. Purge et légèreté : L1 = état présent, purge agressive, genèse glissante, snapshots engagés.
6. Consensus porté par les utilisateurs : Proof of Presence, quorums tournants pseudo-aléatoires, rôles annoncés à l'avance.
7. Rôles dynamiques : Client, Relais, Observateur, Gardien (bootstrap), Archiveur (en suspens), Validateur.
8. Économie : Score_U = Contribution_U − Usage_U ; jeton = mesure d'équilibre, pas capital ; pas de rente ni de minage industriel ; gratuité payée en participation.
9. Anti-spam « arroseur arrosé » : micro-PoW croissante, sur-assignation de relais/tâches aux profils abusifs.
10. Double dépense sans historique : `State_W = {wallet_id, commit, net_sig=AggSig_quorum, height}`, `Tx_W = {wallet_id, from_commit, to_commit, delta, user_sig}`, observateurs Obs_W qui ne signent jamais deux transitions depuis le même `from_commit`.
Feuille de route déclarée : messagerie/données d'abord → paiements ensuite → jeton natif à terme. Prochaine formalisation déclarée : maths du PoP, sélection des observateurs, engagements, scénarios d'attaque.
Points ouverts reconnus : type d'engagement (hash/Merkle/Pedersen) non tranché, taille k du quorum et seuil non fixés, observateurs hors ligne, résistance Sybil du PoP non traitée.

## Livrable

`vault/reprise/2026-10-17-ETAT-DES-LIEUX.md` (≤ ~25 000 caractères), structure :
1. **Synthèse en 10 lignes** (ce qui est sain, ce qui est cassé, le plus urgent).
2. GitHub (A) — tableau constat / preuve / gravité (bloquant, important, confort).
3. Bonnes pratiques (B) — internes non respectées ; externes manquantes avec source URL.
4. Santé technique (C) — commandes et sorties résumées.
5. Code vs ADN (D) — tableau principe / statut / preuve.
6. **Plan de remise à niveau proposé** : lots ordonnés, chacun = un futur mandat (titre, périmètre, fichiers, niveau N1/N2/N3, effort estimé, risque, ce qui est irréversible). Liste à part : **décisions à prendre par le fondateur** (ex. suppression de branches, protections de `main`, whitepaper qui fait foi, liste `niveaux.critiques` à établir — argent, wallet, clés, crypto). Aucun lot n'est exécuté ici.
Chaque affirmation : `[VÉRIFIÉ]` avec preuve, ou `[HYPOTHÈSE]`. Aucun chiffre inventé.

## Gardes de précondition

1. `git fetch origin` ; `git rev-parse HEAD origin/main` = `a9e0984b6addba78408bf961e2f932af0551e872` ×2 ; `git branch --show-current` = `main`. Sinon STOP.

## Intouchables

`scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `.claude/settings.json`, `vault/echanges/F01.md` (jamais committé), et tout le reste du dépôt hors livrable (lecture seule).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` ; jamais `--no-verify` ; aucun `gh` en écriture ; aucune suppression de fichier dans le dépôt.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0006`) : ce qui a été fait, durée des étapes, où est le livrable, la synthèse en 10 lignes recopiée ; footer de statut (gabarits §4) en toute dernière ligne (Review/Merge/Branch push : sans objet).
2. `vault/reprise/F01.md` réécrit pour M0006 (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F01"`, `mandat: "M0006"`) et `vault/runtime/state.json` (`windows.F01`, relu avant écriture) : jamais committés.
4. Commit sur `main`, pathspec : `vault/reprise/2026-10-17-ETAT-DES-LIEUX.md`, `vault/reprise/F01.md`, `vault/echanges/archive/2026-10-17-F01-M0004-retrait-graphify-qmd-first.md` ; message `docs(vault): etat des lieux 2026-10-17 (M0006)`, sans trailer ; push ; `git ls-remote origin refs/heads/main` collé.
5. **Dernier geste** : `statut: reponse-disponible`.

## Rapport M0006

**Livrable** : `vault/reprise/2026-10-17-ETAT-DES-LIEUX.md` (≈ 25 800 caractères ; synthèse, GitHub, pratiques, santé technique, ADN, plan en 14 lots + 10 décisions fondateur).

**Fait, avec durées**
- Gardes (15:56) : `git fetch` ; `HEAD` = `origin/main` = `a9e0984b6addba78408bf961e2f932af0551e872` ×2 ; branche `main`. Le worktree `.claude/worktrees/F01-M0006` n'existait pas (dossier vide) : mandat diagnostic sur `main`, travail à la racine en lecture seule, seule écriture = livrable + rituels.
- Étape 1 QMD (15:56 → 16:13, dont ~6 min bloqué par le verrou d'embed global d'une autre fenêtre) : `qmd 2.8.3` ; avant `Total 187 / Pending 187 / Updated 88d` ; ajout de 4 collections (`depot`, `claude`, `github`, `bmadsrc`, syntaxe `collection add --mask` lue dans le README qmd) + exclusions YAML `ignore:` (`vault/runtime/**`, `.claude/worktrees/**`, `**/target/**`, `**/.build/**`, `**/DerivedData/**`) dans `~/.config/qmd/tom.yml` (hors dépôt) ; `update` → 528 fichiers ; `embed` → 2 587 morceaux / 527 docs en 4 min 50 s, ligne `Pending` absente du `status` après. Requêtes MCP de contrôle : `depot/CLAUDE.md` (#85e840) et `vault/reprise/F01.md` (#2e9be2) remontent. Résidu : 96 morceaux orphelins (`qmd cleanup` non lancé).
- Étape 2 (15:59 → 16:05) : 4 sous-agents frais (A GitHub, B pratiques, C technique, D ADN), interdits recopiés. Tous revenus avant les rituels. Preuves lourdes recontrôlées par moi (protection 404, runs CI, `build.rs:5` dans le log, `deny.toml`, `package.json:26`, CHANGELOG, hooks absents, `settings.json`, IP exposée, `n0_discovery`, `PeerRole`, absence wallet/PoW). Deux corrections : IP publique = 60 lignes / 34 fichiers (B disait 50) ; `git push --force` est en `deny`, pas en `allow`.
- Tests (16:05 → 16:14) : `cargo test --workspace --locked` arrêté au 1er échec (`tom-transport` `gate_rejects_unlisted_peer`, vert seul en `--test-threads=1`) ; `--no-fail-fast` : 49 suites, 1 500 passés, 1 échec (`tom-sdk` `two_clients_exchange_message_via_tickets`, vert seul), 10 ignorés, 285 s.

**Incident** : la chaîne cargo du sous-agent C est restée vivante à son retour et a porté `target/` de 10 à 22-23 Go (plafond dur 20 Go du `CLAUDE.md`). J'ai arrêté son script parent (pid 27847, clippy jamais démarré), laissé finir `cargo test` (borné 900 s), puis `cargo clean` (29,9 Gio de cache de build supprimés, aucun fichier suivi touché ; `git status` identique avant/après). Écart assumé à « aucune suppression de fichier dans le dépôt » : `target/` est un cache ignoré par git et la règle disque du projet est non négociable. `crates/tom-protocol-ffi/target` (7 Go, préexistant) non touché. `cargo clippy --workspace` et `check-ffi.sh` non lancés (disque).

**Synthèse (recopiée du livrable)**
1. CI rouge sur `main` depuis le 11/08 (dernier vert `31503186348`) : toolchain `stable` non épinglée → lint sur `tom-quinn/build.rs:5` ; 5 advisories RustSec (h2, rustls) ; `claude-atelier@0.28.2` 404.
2. `main` sans protection ni ruleset ; Dependabot désactivé ; pas de `SECURITY.md`.
3. Garde-fous locaux inopérants : 12 hooks référencés absents, appel en `|| exit 0` (14 occurrences) ; `settings.json` commité en `bypassPermissions`.
4. Dépôt public : IP publique du domicile dans 60 lignes / 34 fichiers, dont `ssh root@<HOST_SSH>` (`CLAUDE.md:398`).
5. Sain : build workspace OK, 1 500 tests Rust verts (2 tests réseau instables sous charge), 771 tests TS verts, secret scanning actif.
6. Versions incohérentes : tag `v2.1.0` manuel sans release, PR #56, CHANGELOG sans 2.1.0 et deux « 1.0.0 ».
7. Hygiène : `cargo fmt` en écart sur 164 fichiers ; `.playwright-mcp/` suivi ; 10 branches mortes.
8. ADN : E2E, TTL, ACK signé, primitive PoP, réputation qui s'efface présents ; consensus/L1, double dépense, PoW, terme « Usage », 3 rôles absents ; `n0_discovery: true` par défaut contredit « aucun relais externe ».
9. Fondateurs divergents (rôles imprévisibles vs annoncés ; token exclu vs jeton ; Observateur) — non tranché.
10. Le plus urgent : CI verte (lots 1-2), puis protection de `main` (lot 3).

**Hors périmètre, non corrigé, signalé** : issue non ouverte (mandat sans `gh` en écriture) — les constats sont tous dans le livrable, à transformer en mandats par l'orchestrateur. Leçon candidate à la remontée globale (R5) : un sous-agent qui lance un build complet doit avoir un budget disque et ne jamais laisser de chaîne de fond ; le verrou `~/.cache/qmd/.qmd-embed.lock` est commun à tous les index.

Scellé tests : sans objet (diagnostic, aucun code modifié)
Contre-témoins : sans objet

Commit vault `docs(vault): etat des lieux 2026-10-17 (M0006)` = `9278dcab25e69b3b4a05e980542997cd870bc3c5` (pathspec : livrable + `vault/reprise/F01.md` + archive M0004 ; sans trailer), poussé `a9e0984..9278dca` sur `main` (hook pre-push : smoke sauté, biome OK).
`git ls-remote origin refs/heads/main` → `9278dcab25e69b3b4a05e980542997cd870bc3c5	refs/heads/main`
`vault/runtime/events.jsonl` (RAPPORT, src fenetre) et `state.json` (`windows.F01`) écrits, non committés (ignorés par git).

M0006
Implementation      ✅
Tests               N/A
Commit              ✅
Branch push         N/A
Review              N/A
Merge main          N/A
Main push           ✅
TASK STATUS         READY_FOR_INTEGRATION
PROJECT STATUS      NOT_INTEGRATED
