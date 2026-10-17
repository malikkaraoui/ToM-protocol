---
date: 2026-10-17
tags: [securite, ip, integration]
session: F03
mandat: integration-retrait-ip-publique
mandat_id: M0011
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F03.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol-scratch-M0011
branche: fix/retrait-ip-publique
niveau: N2
derniere_maj: 2026-10-17T21:56:49+0200
---

# F03 / M0011 — Intégrer `fix/retrait-ip-publique` (IP du domicile + mot de passe root NAS hors de la version publique)

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, https://github.com/tobi/qmd ; index `tom` à jour depuis M0006) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.
> **IMPÉRATIF (fondateur, 2026-10-17) : on ne pousse pas si ce n'est pas nickel.** Avant CHAQUE `git push`, tu rejoues en local les contrôles de la CI (`.github/workflows/ci.yml`) qui couvrent ce que tu as touché, et tu colles leurs sorties. Rouge = pas de push.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

M0009 (archive `vault/echanges/archive/2026-10-17-F03-M0009-retrait-ip-publique-et-audit-ssh.md`, lis son rapport) a retiré du tip l'IP publique du domicile, celle de la box de la belle-famille et ses ports d'administration, et le **mot de passe root du NAS** qui était en clair dans `scripts/car-test.sh`. Branche poussée, tip `e7f6ce8e6fa67a8045f9df5478d9c7ee88de2186`. Le doublage R007 a rendu RESERVE 4 fois, **toujours sur la garde anti-retour** (`scripts/verifier-ip-interdites.sh` : ne voit pas une IP dans un nom de fichier, une cible de lien, un fichier indexé supprimé du disque). M0009 s'est arrêté proprement et a demandé un arbitrage.
**Décision du fondateur, 2026-10-17 20:43 : option A** — fusionner la branche telle quelle ; les limites de la garde deviennent une issue (aucune n'est prouvée par un test rouge). Le mot de passe root a été changé et l'accès SSH durci par le fondateur le 17/10 vers 18:30.
`main` a avancé depuis (`99aa090…`, CI GitHub verte ; contrôle pre-push = `scripts/ci-local.sh` désormais actif).

## Préparation obligatoire de tout worktree (leçons du 2026-10-17, issues #62, #64, #65)

- Worktrees et scratchs **HORS du dépôt** : `/Users/malik/Documents/tom-protocol-scratch-<Mxxxx>[-suffixe]` (un worktree imbriqué fait remonter cargo au `Cargo.toml` racine).
- **Avant tout commit dans un worktree** : `pnpm install --frozen-lockfile` puis `test -x .husky/_/pre-push && echo HOOKS_OK` — sans `HOOKS_OK`, aucun commit ni push (sinon le contrôle pre-push ne tourne pas : #64).
- Le push déclenche `scripts/ci-local.sh` (≈15-20 min) : pousse avec `GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30'` (#65). Jamais `--no-verify`.
- Disque : `cargo clean` dans chaque scratch après son dernier run (≈28 Go par worktree neuf).

## Gardes de précondition

1. `git fetch origin` ; `git rev-parse origin/fix/retrait-ip-publique` = `e7f6ce8e6fa67a8045f9df5478d9c7ee88de2186` ; `git merge-base --is-ancestor 99aa090a38650d1dd3813d3a16c9c999c6605330 origin/main`. Sinon STOP.
2. Une autre fenêtre (F01/M0012) travaille en parallèle sur `docs/MISSION.md` et `vault/` uniquement. Tu n'y touches pas.

## Mission

1. Rituel de début : `statut: en-cours`.
2. Scratch hors dépôt sur la branche ; **merge de `origin/main` dans la branche** (jamais de rebase). Conflits : additifs → garder les deux ; tout le reste → STOP. Puis relance la garde et `git grep` des valeurs interdites (sans jamais les afficher : compte seulement) : 0 occurrence hors intouchables listés par M0009.
3. Ouvre l'issue des limites de la garde (`gh issue create`, sans aucune IP en clair) : noms de fichiers, cibles de liens, index vs disque ; piste = lire les objets git (`git ls-files -s` + `git cat-file --batch`).
4. Ajoute en tête de `scripts/verifier-ip-interdites.sh` un commentaire de 2 lignes qui dit ces limites (remplacer si un commentaire équivalent existe).
5. **Doublage N2 court** par sous-agent FRAIS, worktree détaché lecture seule hors dépôt : vérifie uniquement (a) plus aucune occurrence des valeurs retirées dans l'arbre (comptes, pas valeurs), (b) scripts sans valeur par défaut publique, (c) test `state.rs` n'utilise pas de variable d'environnement, (d) JSONL valides, (e) le merge de `main` n'a rien cassé (sorties de `ci-local.sh` que TU colles). Rapport `vault/revues/2026-10-17-R009-retrait-ip-publique.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40>`, `niveau: N2`) qui cite les réserves R007 comme « acceptées par le fondateur le 2026-10-17 20:43 → issue #<n> ». Committe aussi `vault/revues/2026-10-17-R007-retrait-ip-publique.md` (présent non suivi à la racine) dans le même commit que R009.
6. Push de la branche (hook complet), puis merge (bloc ci-dessous) avec `[R009]`.

## Règles des sous-agents (verbatim)

(1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre. **Aucun sous-agent ne lance de build ou de test cargo** (incident M0006 : `target/` à 23 Go) : les builds restent à la session mère.

## Hygiène disque

Plafond `target/` 20 Go (`CLAUDE.md` racine) : `du -sh target/` avant et après chaque build lourd ; au-delà de 15 Go, `cargo clean` avant de continuer. Rien ne tourne en fond à la sortie.

## Intouchables

`scripts/pre-push-gate.sh` (modifié par le fondateur, non committé), `vault/30-discoveries.md`, `.claude/settings.json`, les `vault/echanges/F0x.md` vivants (jamais committés), `docs/MISSION.md` et tout fichier de F01/M0012.

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée ; jamais `--no-verify` ; jamais de force-push.

## Merge (gabarits §9) et intégration

Scratch détaché sur `origin/main` à jour (`git fetch`), `merge --no-ff <tip> -m "Merge <branche> [<Rxxx>]"`, **contrôles CI concernés rejoués sur le scratch** (sorties collées), copie + commit du rapport de doublage (`docs(revues): <Rxxx> -- GO`). Puis : racine sur `main` et `git status --short` = intouchables seuls → `git merge --ff-only <HEAD scratch>` à la racine + `git push origin main` ; racine occupée → `git push origin <HEAD scratch>:refs/heads/main` depuis le scratch (fast-forward seulement). `origin/main` a bougé → refaire le scratch UNE fois. `git ls-remote origin refs/heads/main` collé. Après le push : `gh run list --branch main -L 3` et `gh run watch <id> --exit-status` (attente bornée à 30 min) — sortie collée ; le résultat CI réel va au rapport tel quel. Worktrees retirés à la fin.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0011`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F03.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F03"`, `mandat: "M0011"`) et `vault/runtime/state.json` (`windows.F03` seulement, relu juste avant d'écrire) : jamais committés.
4. Commit vault sur `main` (racine si elle est sur `main`, sinon scratch détaché) : `vault/reprise/F03.md` + `vault/echanges/archive/2026-10-17-F03-M0009-retrait-ip-publique-et-audit-ssh.md` ; `docs(vault): reprise F03 M0011` ; push ; `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.


## Rapport M0011


**Résultat : STOP propre — rien n'est poussé ni mergé.** Le travail est fait et relu (R009 **GO**), mais le contrôle pre-push obligatoire (`scripts/ci-local.sh`) est rouge 2 fois de suite sur le test flaky connu `tom-transport::node::tests::gate_rejects_unlisted_peer` (issue #63). Ce test n'est pas touché par la branche, et le même arbre était passé vert juste avant. Règle « rouge = pas de push » : j'arrête.

Scellé tests : sans objet (`scripts/verifier-rendu.mjs` absent du dépôt)
Contre-témoins : aucun

### 0. Arbitrage demandé (la question que j'aurais posée)

**Question** : le hook pre-push bloque sur un flake connu (#63), étranger à la branche. Comment pousser ?

| Option | Contenu | Coût / risque |
|---|---|---|
| **A (mon choix)** | Corriger d'abord #63 dans un mandat court (le test exige un `send` OK de C alors que le gate peut déjà le refuser : accepter `Err` « not allowed » côté C, ou poser la whitelist avant la connexion de C). Puis relancer M0011 depuis la branche locale `3ef96ea` (push + merge, R009 déjà GO). | Un mandat de plus, mais le hook reste sans échappatoire. Vaut pour tous les pushes : tant que #63 est flaky, chaque push est une loterie. |
| B | Relancer le push quand aucune autre fenêtre ne compile (le flake semble lié à la charge CPU : F01/M0012 lançait des tests cargo en même temps). | Pas de garantie ; 20 min par essai. |
| C | Pousser avec contournement du hook. | Interdit (`--no-verify`). Écarté. |

Raison du choix A : deux rouges de suite sur la même cause = le mécanisme est en cause (tronc commun). Corriger le test est hors du périmètre de M0011.

### 1. Ce qui est fait (local, branche `fix/retrait-ip-publique` = `3ef96eabd8b7d80ccc7871827db01256e75e52ba`, NON poussée ; `origin` reste à `e7f6ce8`)

- Préconditions : `origin/fix/retrait-ip-publique` = `e7f6ce8e6fa67a8045f9df5478d9c7ee88de2186` ✅, `99aa090` ancêtre de `origin/main` ✅.
- `2f2432b` : `Merge branch 'main' into fix/retrait-ip-publique` (merge de `origin/main` 99aa090, aucun conflit). Écart : ce commit a d'abord été créé avant `pnpm install` (hooks absents) avec un message que commitlint refuse ; corrigé par `--amend` (non poussé) au format de `main`, hooks actifs. `pnpm lint` rejoué : OK.
- `3ef96ea` : `fix(securite): retrait IP ramenee par main + limites de la garde en en-tete (#67)`.
  - Le merge de `main` ramenait **1 occurrence** de l'IP du domicile : `vault/echanges/archive/2026-10-17-F01-M0006-etat-des-lieux-github-adn.md:111` (archive F01 M0006, pas M0012) → `root@<HOST_SSH>`.
  - En-tête de `scripts/verifier-ip-interdites.sh` : 2 lignes ajoutées (lecture disque, pas objets ; noms de fichiers, cibles de liens, index vs disque ; renvoi #67).
- Issue des limites de la garde : **#67** https://github.com/malikkaraoui/ToM-protocol/issues/67 (aucune IP en clair).
- Comptes (valeurs jamais affichées) :
```
garde (disque) au tip      : verifier-ip-interdites: OK (aucune IP interdite), rc=0
objets git @2f2432b (merge): noms_de_fichiers=0 contenus_hors_exemptions=1 (l'archive F01-M0006) contenus_exemptes=7
objets git arbre indexé (= 3ef96ea): noms_de_fichiers=0 contenus_hors_exemptions=0 contenus_exemptes=7
```
  Les 7 exemptées sont dans `vault/30-discoveries.md` (intouchable).

### 2. Contrôles CI rejoués (`scripts/ci-local.sh` sur le scratch, HEAD `3ef96ea`)

Run 1 (manuel, avant doublage) — **vert** :
```
[ci-local] base de comparaison : 99aa090 → HEAD 3ef96ea
… typescript (install/build/test/lint), observability-smoke, rust-protocol, rust-transport, rust-integration,
  rust-fork, rust-dht, rust-security (cargo deny), rust-ffi (+ header drift), rust-macos, rust-stress, rust-poc
[ci-local] TOUT VERT.
34 lignes « test result » : passed=1354 failed=0 ; 0 ligne FAILED/panicked
```
Run 2 (hook pre-push, `git push origin fix/retrait-ip-publique`) — **rouge**, push refusé :
```
test node::tests::gate_rejects_unlisted_peer ... FAILED
thread 'node::tests::gate_rejects_unlisted_peer' panicked at crates/tom-transport/src/node.rs:1489:52:
C send: Send { node_id: NodeId(23a4b4711c64...), source: closed by peer: not allowed (code 1) }
test result: FAILED. 47 passed; 1 failed
[ci-local] ÉCHEC (code 101) à l'étape : rust-transport: test — pas de push.
e7f6ce8e6fa67a8045f9df5478d9c7ee88de2186	refs/heads/fix/retrait-ip-publique
```
Run 3 (une seule nouvelle tentative du push) — **rouge**, même test, même ligne, même `ls-remote` (`e7f6ce8`). Les deux fois, F01/M0012 compilait/testait en parallèle (`cargo test -p tom-dht` vu dans `ps`). Constat ajouté sur #63 : https://github.com/malikkaraoui/ToM-protocol/issues/63#issuecomment-6239597944

Disque : `cargo clean` après chaque run (24,7 Gio, puis 12,1 Gio ×2) ; worktree retiré.

### 3. Doublage R009 (N2, sous-agent frais, worktree détaché lecture seule) — **GO**, tip `3ef96ea`

(a) scan indépendant des objets git : 1 499 blobs, 0 occurrence hors exemption, 0 nom de fichier ; garde OK ; aucun `sshpass -p` littéral, `TOM_NAS_SSH_PASS` sans défaut · (b) 3 scripts en `${VAR:?…}`, `bash -n` OK, sortie rc=1 sans variables avant tout réseau · (c) `state.rs:7698` `embedded_relay_port_mapped_publishes_global_url` : pas de `std::env`/`env!`/`option_env!`, IP `1.2.3.4` · (d) 3 451 lignes JSONL dans 4 fichiers, 0 invalide · (e) aucun marqueur de conflit, diff `99aa090..3ef96ea` = 78 fichiers du périmètre, run 1 de `ci-local.sh` TOUT VERT vérifié par le relecteur dans le journal. Réserves R007 : acceptées par le fondateur le 2026-10-17 20:43 → issue #67.
J'ai recontrôlé moi-même (b) et (c) (fichier:ligne) : conformes.
Rapport : `vault/revues/2026-10-17-R009-retrait-ip-publique.md` (frontmatter `verdict: GO`, `tip: 3ef96ea…`, `niveau: N2`). **Non committé** avec R007 : le mandat les committe dans le scratch de merge, qui n'a pas eu lieu. Les deux restent non suivis à la racine pour la reprise (0 valeur interdite dans l'un et l'autre, vérifié par le scan objets). Note : le R009 GO porte sur `3ef96ea` ; reprendre ce tip tel quel garde le GO valide.

### 4. Reprise

1. Corriger #63 (mandat court). 2. Depuis la racine : la branche locale `fix/retrait-ip-publique` est à `3ef96ea` (23 commits d'avance sur origin) → worktree hors dépôt, `pnpm install --frozen-lockfile` + `HOOKS_OK`, merge de `origin/main` si elle a bougé (puis nouveau doublage court), push, merge §9 avec `[R009]` + commit R007/R009.

### 5. Remarques

- Le relecteur n'a pas pu exécuter le test de détection sur valeur plantée (permission refusée, non contournée) ; M0009 l'avait fait (9/9).
- Les valeurs retirées restent dans l'historique git public (choix du fondateur) ; mot de passe changé le 17/10 vers 18:30 d'après le mandat.
- Rituels : `vault/reprise/F03.md` + archive M0009 committés sur `main` depuis la racine (`7bf72ce docs(vault): reprise F03 M0011`, sans trailer, auteur gmail). Push : hook `ci-local.sh` **TOUT VERT** cette fois (le flake #63 est intermittent, ce qui appuie l'option B en dépannage), `99aa090..7bf72ce  main -> main` :
```
7bf72ce8796e92ac80879003514ec77cd7a3c4ec	refs/heads/main
```
- `vault/runtime/events.jsonl` + `state.json` (`windows.F03`) mis à jour, non committés (ignorés). `cargo clean` à la racine (24,6 Gio). Worktrees M0011 retirés ; aucun process ni tâche de fond de cette fenêtre (les `cargo`/`ci-local` encore visibles tournent dans `tom-protocol-scratch-M0012-merge`, fenêtre F01).

M0011
Implementation      ✅
Tests               ❌
Commit              ✅
Branch push         ❌
Review              ✅
Merge main          ❌
Main push           ❌
TASK STATUS         BLOCKED
PROJECT STATUS      NOT_INTEGRATED
