---
date: 2026-10-18
tags: [passation, orchestrateur, cowork, reprise]
auteur: Cowork (session du 2026-10-17)
statut: a-lire-en-premier
---

# Passation Cowork → successeur (2026-10-18)

> Lis ce fichier en entier avant toute action. Tout ce qui est marqué [VÉRIFIÉ] l'a été le 2026-10-18 ~08:40 ; re-vérifie avant de t'appuyer dessus.

## 1. Ton rôle et tes règles (non négociables)

- Charge le skill `anthropic-skills:orchestrateur-cowork` à chaque session. **Cowork n'exécute jamais** : il écrit des mandats pour le superviseur, lit les rapports, contre-vérifie, informe Malik.
- **Feu vert explicite de Malik** avant de poser un mandat. Sans « oui / go / envoie », tu proposes et tu attends. Un refus d'appel d'outil = STOP, tu attends.
- Réponses à Malik : **5 lignes**, français, direct, zéro complaisance, étiquettes `[VÉRIFIÉ] / [MÉMOIRE] / [HYPOTHÈSE]`. Toute affirmation d'état est vérifiée avant d'être dite. Re-vérifie les conclusions des mandats avant de les relayer.
- **Zéro écriture git** par Cowork (pas de commit, pas de push, pas même `git status` pour décider : lis les fichiers). Les commits vault sont faits par les fenêtres dans leurs rituels de fin.
- **Ne lis/écris rien dans `~/.harnais`** sauf le guichet (`~/.harnais/guichet` : `vivant.json`, `demandes/`, `reponses/`).
- **Intouchables** : `scripts/pre-push-gate.sh`, `vault/30-discoveries.md`, `.claude/settings.json`.
- **Interdits permanents** : force-push, `--no-verify`, réécriture d'historique (décidé par Malik : l'IP reste dans l'historique public).
- **Secrets / IP** : jamais en clair dans un mandat, un rapport, un commit, une réponse. Placeholders `<IP_DOMICILE>`, `<HOST_SSH>`.
- **QMD est un IMPÉRATIF** pour lire/chercher le markdown : index `tom` (`qmd --index tom query|get …`), rafraîchi par `qmd --index tom update && qmd --index tom embed` (jamais `qmd update` seul : il met à jour l'index global de la machine). GitHub via `gh` (le container Cowork n'a pas `gh` ; ce sont les fenêtres qui l'utilisent).

## 2. Mécanique du harnais (v1.6.0) [VÉRIFIÉ]

- Convention : `.claude/skills/orchestration-bureau/SKILL.md` + `references/gabarits.md` ; config `config/harnais.json`.
- **Fenêtres plates** F01, F02, F03 (`vault/echanges/Fxx.md`). Un mandat = un fichier Fxx.md avec frontmatter (`mandat_id`, `statut`, `commande`, `worktree`, `branche`, `niveau`).
- **Cycle du statut** : `en-pose` → `pret-a-lancer` (le superviseur lance) → `en-cours` (la fenêtre) → `reponse-disponible` (dernier geste de la fenêtre).
- **Avant de poser** : archiver l'ancien Fxx.md dans `vault/echanges/archive/AAAA-MM-JJ-Fxx-Mxxxx-<slug>.md` ; écrire le nouveau ; ajouter `MANDAT_POSE` dans `vault/runtime/events.jsonl` ; mettre `vault/runtime/state.json` à jour (`next_mandat_id`, `next_revue_id`, `windows.Fxx`) ; passer `statut: pret-a-lancer` ; vérifier l'événement `LANCEMENT` (src `sup`) ~30-60 s après.
- Événements utiles : `LANCEMENT`, `SESSION_IDLE` (extrait de la fenêtre), `RAPPORT`, `MANDAT_TERMINE`, `PANNEAU_FERME`, `MANDAT_SUSPECT`, `OUTIL_ECHEC` (souvent bénin).
- **Niveaux** : N2 = doublage par sous-agent frais en lecture seule, rapport `vault/revues/AAAA-MM-JJ-R0xx-<slug>.md` (frontmatter exact `verdict: GO|RESERVE`, `tip: <sha 40>`, `niveau: N2`). `niveaux.critiques` est vide → pas de N1 pour l'instant.
- **Merge (gabarits §9)** : scratch détaché hors dépôt sur `origin/main` à jour, `merge --no-ff <tip> -m "Merge <branche> [Rxxx]"`, contrôles rejoués, commit du rapport de revue, push fast-forward vers `main`, `git ls-remote` collé, `gh run watch` vert exigé. Si `origin/main` a bougé : refaire le scratch UNE fois, sinon STOP.
- **Push** : le hook pre-push lance `scripts/ci-local.sh` (≈15-20 min, rejoue les 12 jobs CI). « On ne pousse pas si ce n'est pas nickel. »
- **Préparation obligatoire de tout worktree** (bloc à copier dans chaque mandat) : worktrees **hors du dépôt** `/Users/malik/Documents/tom-protocol-scratch-<Mxxxx>` (#62) ; `pnpm install --frozen-lockfile` puis `test -x .husky/_/pre-push && echo HOOKS_OK` avant tout commit (#64) ; `GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30'` (#65) ; `cargo clean` après le dernier run (~25 Gio par worktree).
- **Modèle de mandat** : le plus récent et complet est `vault/echanges/F03.md` (M0013) — tête QMD/gh/pre-push, Contexte autonome, Préparation, Gardes, Mission, Règles des sous-agents, Hygiène disque, Intouchables, Interdits, Merge §9, Rituels de fin. Copie-le.
- **Réveils** : après un lancement, programme un réveil +15 min et un filet +60 min (send_later), supprime-les quand c'est fini.

## 3. État au 2026-10-18 ~08:40 [VÉRIFIÉ]

- `origin/main` = `cbbd554` (docs(vault): reprise F03 M0013), précédé de `93f1833` (revues R007 R009 R012). CI GitHub verte sur les derniers runs rapportés (39070424966, 39072516585 : 12/12).
- F01 / M0012 : **INTÉGRÉ** (`7af7a45`, R010 GO). `docs/MISSION.md` porte les décisions du 17/10 + §8 « Questions ouvertes » ; carte des écarts et cible : `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md`. Issue #68 (remarques non bloquantes).
- F03 / M0013 : **INTÉGRÉ**. #63 corrigé (test seulement, 30/30 + 10/10 sous charge, R011 GO, issue fermée) ; `fix/retrait-ip-publique` intégrée (R012 GO), branche distante supprimée. Note : M0013 a été lancé alors que mon appel de pose avait été refusé par Malik à 22:24 — origine du lancement non élucidée, à demander à Malik si besoin.
- F02 : libre (M0008 intégré).
- `state.json` : `next_mandat_id` M0014, `next_revue_id` R013.
- **Racine du dépôt en retard** sur `7bf72ce` et sale : `M .claude/settings.json`, `M scripts/pre-push-gate.sh`, `M vault/30-discoveries.md` (intouchables modifiés — origine à identifier, probablement harnais/Malik ; ne pas toucher), plus des non suivis dont `vault/revues/R007`/`R009` (désormais suivis sur `main` → bloquent un `ff-only` à la racine) et ce fichier. Décision à prendre avec Malik avant tout mandat qui travaille à la racine.
- NAS/VM Freebox : SSH durci par Malik le 17/10 (root par clé seulement, `PasswordAuthentication no`, clés copiées pour l'utilisateur freebox). Test de connexion depuis le Mac non confirmé.

## 4. Décisions du fondateur à respecter

- `docs/MISSION.md` **fait foi** (la dernière version l'emporte).
- Jeton = « grain de riz », note interne de participation, pas de l'argent. Rôles attribués par le réseau selon besoin, note, contexte d'appareil (puissance, place, fiabilité de connexion). Témoins : « tout le monde s'observe jusqu'à 7, vol des étourneaux ». Subnet = cercle réel de contacts, vases communicants. Double dépense : chantier ouvert.
- **Zéro dépense externe** pour l'amorçage. **Pas d'arrière-plan sur mobile** : logique « temps d'écran agrégé » ; nœuds 24/24 = box, Raspberry Pi, VM Freebox (et à terme objets branchés en permanence).
- n0 actif par défaut dans l'app = **incohérent** pour Malik ; à couper **après** un amorçage interne prouvé (sinon la 4G casse).
- Méthodologie cible : `main` protégée + PR + checks requis + merge queue. Le prompt a été confié au **Cowork du projet harnais** ; quand la nouvelle version du harnais sort, la poser sur tom-protocol puis protéger `main`.
- Le faux « accès macOS refusé » du guichet = délai de 5 s quand le superviseur est en retard ; correction côté projet harnais.

## 5. Ce qui attend (proposé, pas lancé)

1. **M0014 « passage à l'échelle »** (proposé à Malik le 17/10 au soir, pas encore de feu vert) : agents en parallèle sur 5 chantiers — (1) amorçage 100 % interne sans n0/DHT publique, observation en lecture seule des nœuds vivants (VM Freebox, iPad, Mac, iPhone, Apple TV — demander lesquels sont libres) ; (2) simulation 10/100/500 nœuds (briques existantes : `crates/tom-stress`, `crates/tom-gossip/src/proto/sim.rs`), sessions ~1 min + nœuds 24/24 ; (3) manques d'une vraie messagerie (hors ligne/store-and-forward, groupes, multi-appareils, clés) ; (4) SDK (`crates/tom-sdk`) pour « son WhatsApp » ; (5) quels rôles un appareil peut tenir au premier plan seulement. À articuler avec la cible C1-C5 de M0012 (C1 = prouver l'amorçage maison sans tiers, sans code).
2. Identifier les nœuds « anonymous » vus dans l'app Mac le 17/10 (1 ms LAN + un à 135 ms) : `ps aux | grep -iE "tom|node"` et `ss -ulpn` sur la VM — Malik n'a pas encore renvoyé la sortie.
3. Constats hors périmètre de M0012 : `vault/00-brief.md` périmé, `vault/40-roadmap.md:293` (R17) périmé, `CLAUDE.md` ADR-002 « Bootstrap Elimination DONE » faux tant que n0 est actif ; question de date de MISSION (1/07 vs « rédigé 2026-06-26 »).
4. Lots de l'état des lieux (`vault/reprise/2026-10-17-ETAT-DES-LIEUX.md`) : protection de `main` (après harnais), sécurité (Dependabot, SECURITY.md, permissions CI), reformat `cargo fmt`, branches mortes (décision Malik), releases/CHANGELOG, `.claude/settings.json` bypassPermissions/hooks manquants, liste `niveaux.critiques`.
5. Issues ouvertes : #59 (advisory crossbeam FFI), #60 (bump cfg_aliases), #62, #64 (hooks absents en worktree neuf — critique, contourné par le bloc Préparation), #65 (hook long / keepalive SSH), #66 (wording CONTRIBUTING), #67 (limites de la garde IP), #68.
6. Leçon R5 (M0012) : un push vers `main` de ~20 min se fait doubler par toute fenêtre qui pousse entre-temps → **sérialiser** les pushes vers `main` (ne pas lancer deux mandats qui intègrent en même temps) en attendant PR + merge queue.

## 6. Mes erreurs, pour ne pas les refaire

- Diagnostic « accès macOS refusé » non vérifié → Malik envoyé en boucle. Vérifie avant de prescrire.
- Mandats mal conçus (pre-commit biome ignoré, règle de test fausse, garde réversible → 4 tours de revue). Lis le code avant d'écrire la règle.
- Commits vault sur un `main` rouge → nombreux échecs CI publics, Malik furieux. Rien ne part sur `main` sans CI locale verte.
- Deux mandats intégrant en parallèle → charge CPU → flake #63 → STOP de M0011 et run perdu de M0012.
- Hypothèse présentée trop vite comme probable (nœuds « anonymous » = tests) puis infirmée par `ps`. Étiquette tout.
- Lecture de `harnais.log` dans `~/.harnais` (interdit), `rm -r` non gardé (M0004) : règles violées, reconnues.
