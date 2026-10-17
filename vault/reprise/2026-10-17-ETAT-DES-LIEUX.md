---
date: 2026-10-17
tags: [etat-des-lieux, github, bonnes-pratiques, adn, qmd]
mandat: M0006
session: F01
base: a9e0984b6addba78408bf961e2f932af0551e872
---

# État des lieux ToM Protocol — 2026-10-17 (M0006, diagnostic, lecture seule)

> 4 sous-agents frais (A GitHub, B pratiques, C technique, D ADN), preuves lourdes recontrôlées.
> `[VÉRIFIÉ]` = relancé ou relu (« par X » = preuve du sous-agent, échantillonnée) ; `[HYPOTHÈSE]` = non prouvé.

## 1. Synthèse en 10 lignes

1. **CI rouge sur `main` depuis le 11/08** : dernier run CI vert = `31503186348` (2026-08-11) ; depuis, 9 échecs / 1 succès [VÉRIFIÉ]. Trois causes : toolchain `stable` non épinglée → lint `trailing semicolon in macro` sur `crates/tom-quinn/build.rs:5` (6 jobs Rust) ; 5 advisories RustSec (h2, rustls) ; devDependency `claude-atelier@0.28.2` introuvable (404) [VÉRIFIÉ].
2. **`main` sans aucune protection** (404, ruleset `[]`), tout est poussé en direct ; Dependabot désactivé ; pas de `SECURITY.md` [VÉRIFIÉ].
3. **Garde-fous locaux inopérants** : 12 scripts `hooks/*.sh` référencés par `.claude/settings.json` n'existent pas, et l'appel finit par `|| exit 0` (échec muet) ; `settings.json` commité en `bypassPermissions` avec `git push*` autorisé (seul `--force` refusé) [VÉRIFIÉ].
4. **Dépôt public qui expose l'infra perso** : IP publique du domicile dans 60 lignes / 34 fichiers suivis, dont `ssh root@82.67.95.8` (`CLAUDE.md:398`) [VÉRIFIÉ].
5. **Sain** : build workspace OK (1 min 43 s, 0 warning) ; `cargo test --workspace` 1 500 verts / 1 échec instable (2 tests réseau qui passent seuls) ; TypeScript 771 verts ; secret scanning actif ; 97/100 commits conventionnels [VÉRIFIÉ].
6. **Versions incohérentes** : tag `v2.1.0` posé à la main (16/07) sans release, PR release-please #56 « 2.1.0 » ouverte, `CHANGELOG.md` sans 2.1.0 et avec deux « 1.0.0 », crates en 0.2.0 [VÉRIFIÉ].
7. **Hygiène** : `cargo fmt --check` en écart sur 164 fichiers ; `.playwright-mcp/` (11 logs) suivi ; PDF/HTML à la racine de `docs/` ; 10 branches distantes mortes, `delete_branch_on_merge=false` [VÉRIFIÉ].
8. **Code vs ADN** : E2E, TTL 24 h, ACK signé, primitive PoP, réputation qui s'efface = présents ; consensus/L1, double dépense, micro-PoW, terme « Usage » du score, rôles Observateur/Gardien/Validateur = absents ; `n0_discovery: true` par défaut contredit « aucun relais externe » [VÉRIFIÉ].
9. **Documents fondateurs divergents** (rôles imprévisibles vs annoncés à l'avance ; token exclu vs jeton natif à terme ; Observateur abandonné le 12/07 vs pièce maîtresse) — non tranché, liste au §5.
10. **Le plus urgent** : remettre la CI au vert (lot 1), puis protéger `main` (lot 3) — sans CI verte, une protection bloquerait tout.

## 2. GitHub (A) — `malikkaraoui/ToM-protocol`

| Constat | Preuve | Gravité |
|---|---|---|
| CI rouge sur `main` depuis le 11/08 | `gh run list --workflow CI --branch main` : depuis 2026-08-11, `failure` ×9, `success` ×1 ; dernier vert `31503186348` [VÉRIFIÉ] | Bloquant |
| Cause A : lint rustc nouveau sur `cfg_aliases!` | log run `39036883418` : `error: trailing semicolon in macro used in expression position --> crates/tom-quinn/build.rs:5:5` ; CI sur `dtolnay/rust-toolchain@stable`, pas de `rust-toolchain.toml` [VÉRIFIÉ] | Bloquant |
| Cause B : cargo-deny, advisories non traitées | même log : RUSTSEC-2026-0258 (h2), -0285, -0346, -0348, -0349 (rustls) [VÉRIFIÉ] ; `deny.toml` compte 9 lignes `RUSTSEC-` (7 identifiants uniques selon C), le commentaire de `ci.yml:191-192` en annonce 10 [HYPOTHÈSE sur l'écart exact] | Bloquant (sécurité TLS) |
| Cause C : `pnpm install --frozen-lockfile` échoue | `ERR_PNPM_FETCH_404 … claude-atelier-0.28.2.tgz` ; `package.json:26` `"claude-atelier": "^0.28.2"` [VÉRIFIÉ] ; version dépubliée [HYPOTHÈSE] | Bloquant |
| `main` non protégée, aucun ruleset | `gh api …/branches/main/protection` → 404 « Branch not protected » ; `…/rulesets` → `[]` [VÉRIFIÉ] | Bloquant |
| Chaque push de vault relance toute la CI | `ci.yml:4-7` : `push` sur `main`, ni `paths-ignore` ni `concurrency` [VÉRIFIÉ par A] | Important |
| Dependabot désactivé, pas de `dependabot.yml` ; code scanning absent | `…/dependabot/alerts` → 403 « disabled » [VÉRIFIÉ] ; `…/code-scanning/alerts` → 404 [VÉRIFIÉ par A] | Important |
| Pas de `SECURITY.md`, `CODEOWNERS`, `CODE_OF_CONDUCT`, gabarit de PR | `community/profile` → `health_percentage: 57` [VÉRIFIÉ] | Important (SECURITY) / confort |
| Secret scanning + push protection actifs | `security_and_analysis` : `enabled` ×2, 0 alerte [VÉRIFIÉ] | Bon point |
| Release 2.1.0 en conflit | tag annoté `v2.1.0` du 2026-07-16, `gh release view v2.1.0` → « not found » ; PR #56 release-please « 2.1.0 » ouverte depuis 2026-07-15, run `action_required` [VÉRIFIÉ par A] | Important |
| CHANGELOG en retard + doublon | 1er titre `CHANGELOG.md:3` = 2.0.0 alors que `package.json:35` = 2.1.0 ; « 1.0.0 » en `:324` (2026-07-15) et `:716` (2026-02-05) [VÉRIFIÉ] | Important |
| Deux espaces de versions | release-please `release-type: node` (`release.yml:12-18`) ; crates 0.2.0, tom-quinn 0.16.1, quinn-proto 0.15.1, `packages/*` 0.0.1 [VÉRIFIÉ par A] | Important |
| 7 tags, 5 releases ; `README.md:100` pointe `v0.3.0` sans release | `gh release list` [VÉRIFIÉ par A] | Confort |
| Licence incohérente | `LICENSE` = MIT [VÉRIFIÉ] ; `tom-quinn*`/`tom-quinn-udp` `Cargo.toml:30` = `MIT OR Apache-2.0` sans texte Apache [VÉRIFIÉ par A] | Important |
| Réglages repo | `delete_branch_on_merge:false`, merge/squash/rebase tous permis, `topics:[]`, `homepage:null` [VÉRIFIÉ] | Confort |
| `ci.yml` sans bloc `permissions:` | `grep permissions .github/workflows/*.yml` : seulement `release*.yml` [VÉRIFIÉ par A] | Confort |
| Divers : `Release SDK Swift` jamais déclenché (aucun tag `sdk-swift/v*`) ; `.github/skills/` = 21 symlinks vers `.claude/skills/` ; `.github/ISSUE_BACKLOG.md` doublon d'issues fermées ; 1 issue ouverte (#58), 1 PR ouverte (#56) | `release-sdk-swift.yml:7-9`, `ls -la .github/skills`, `gh issue/pr list` [VÉRIFIÉ] | Confort |

**Branches distantes** (11 hors `main`) [VÉRIFIÉ par A] : 4 ancêtres de `main` sans commit propre (`chore/harnais-v1.6.0`, `chore/retrait-graphify`, `docs/qmd-imperatif`, `fix/connectioninner-leak`) ; 6 de PR mergées mais non ancestrales (squash [HYPOTHÈSE]) : `docs/readme-porte-entree` #41, `fix/rustsec-rustls-webpki` #42, `fix/rustsec-rand-slab` #43, `fix/rustsec-aws-lc` #44, `fix/rustsec-hickory-026` #45 (12/06), `claude/tom-protocol-audit-yf42jz` #53 (01/07) ; 1 vivante (release-please, PR #56).

## 3. Bonnes pratiques (B)

### 3.1 Règles internes non respectées

| Règle | Source | Écart | Preuve |
|---|---|---|---|
| Hooks bloquants (loop-master, review-auto, no-sign, tests-before-push…) | `.claude/CLAUDE.md` §3, §13, §25 ; `.claude/settings.json` | 12 chemins `/hooks/*.sh` référencés, dossier `hooks/` absent ; `.claude/hooks/` ne contient que `executor-ctx-monitor.sh` ; appel en `\|\| exit 0` → aucun blocage, aucun signal | `ls hooks` → absent ; `git show HEAD:.claude/settings.json` [VÉRIFIÉ] |
| Pas de bypass / push sous gate | `.claude/CLAUDE.md` §22, §24 | `settings.json` commité : `"defaultMode": "bypassPermissions"` (l.62) ; `allow` `git push*`/`git push:*` ; `deny` seulement `git push --force:*`, rien sur `--no-verify` | [VÉRIFIÉ] |
| Pre-push gate avant chaque push | `.claude/CLAUDE.md` §24 | `.husky/pre-push` ne lance pas `pre-push-gate.sh` ; la gate ne tourne qu'à la main ; elle fait `clippy --workspace` sans `--tests` et `test --workspace --lib` | [VÉRIFIÉ par B] |
| Aucun état dans un fichier chargé d'office | tronc harnais | `CLAUDE.md` : « Test Counts (à jour 2026-06-22) », « MAJ 2026-07-19/21 » ; `.claude/CLAUDE.md` §0 « build 141 », date 2026-04-17 | [VÉRIFIÉ par B] |
| `.claude/CLAUDE.md` ≤ 150 lignes | `.claude/CLAUDE.md:3` | 157 lignes | [VÉRIFIÉ par B] |
| Trailer `Co-Authored-By` | `.claude/CLAUDE.md` §13 « jamais signer » vs `config/harnais.json` `git.trailer` (et `trailers_retires` qui le retire) vs `CONTRIBUTING.md` (exemple avec trailer) | 3 documents, 3 règles ; 210 commits portent le trailer, 0 dans les 100 derniers | [VÉRIFIÉ par B] |
| Email d'auteur gmail | `scripts/pre-push-gate.sh` étape 0 | 20 commits sur `main` d'auteur `noreply@anthropic.com` (06/06 → 01/07) | `git log --format=%ae` → 20 [VÉRIFIÉ] |
| `rustfmt.toml` présent | `rustfmt.toml` | `cargo fmt --all -- --check` : exit 1, 164 fichiers, 1 189 hunks ; fmt ni en CI ni en gate | [VÉRIFIÉ par B, sortie `scratchpad/B/fmt.txt`] |
| `ed25519-dalek` = `=3.0.0-pre.1` | `CLAUDE.md` note 8 | `tom-gossip/Cargo.toml:39` et `tom-connect/Cargo.toml:51` en caret `"3.0.0-pre.1"` | [VÉRIFIÉ] |
| Clippy + tests de tout crate touché | `CLAUDE.md` « Cross-Crate Validation » | CI : pas de `clippy --workspace`, `tom-sdk`/`tom-tui`/`tom-gateway` jamais construits ni testés, `tom-protocol-ffi` sans `cargo test` (alors que `check-ffi.sh` en a un) | `grep` `ci.yml` → 0 occurrence des 3 crates [VÉRIFIÉ] |
| Known Limitations / réfs exactes | `CLAUDE.md`, `.claude/CLAUDE.md` | lignes dérivées (`rendezvous_entry_authentic` `:862`→`:1354`) ; absents : `test/hooks.js`, `scripts/switch_model.py`, `scripts/update-security.js` | [VÉRIFIÉ par B] |
| `CONTRIBUTING.md` à jour | — | décrit TypeScript/WebRTC + signaling ; `CLAUDE.md` dit QUIC et signaling DEPRECATED ; « sujets < 72 car. » non tenu (65/100) | [VÉRIFIÉ par B] |
| Vault committé en fin de mandat | tronc harnais | `vault/30-discoveries.md` modifié non commité (+ `.claude/settings.json`, `scripts/pre-push-gate.sh`) | `git status` [VÉRIFIÉ] |

### 3.2 Pratiques externes manquantes

| Pratique | Source | Présent ? | Utile ici | Coût | Prio |
|---|---|---|---|---|---|
| Toolchain épinglée (`rust-toolchain.toml`) | https://rust-lang.github.io/rustup/overrides.html | Non — cause directe du rouge | Oui | Faible | P1 |
| Protection de branche / rulesets | https://github.com/ossf/scorecard/blob/main/docs/checks.md (Branch-Protection) | Non | Oui, même mono-mainteneur | Faible | P1 |
| Permissions minimales des workflows | https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions ; Scorecard Token-Permissions | Non (`ci.yml`) ; `release.yml` en `contents: write` au niveau workflow | Oui | Faible | P1 |
| Actions épinglées par SHA | même guide GitHub ; Scorecard Pinned-Dependencies | Non (`@v4`, `@stable`, `@nightly`) | Oui | Faible | P1 |
| Dependabot (alertes + MAJ sécurité) | Scorecard Dependency-Update-Tool ; https://docs.github.com/en/code-security/getting-started/github-security-features | Non | Oui | Faible | P1 |
| `SECURITY.md` (divulgation) | Scorecard Security-Policy | Non | Oui, fort (protocole crypto public) | Faible | P1 |
| fmt + clippy workspace en CI, tous les membres couverts | pratique Rust | Non | Oui | Faible (+ reformatage 164 fichiers) | P1 |
| cargo-deny `[licenses]` | `deny.toml` | Non configuré | Oui (forks iroh/quinn MIT/Apache) | Faible | P2 |
| MSRV testée en CI | https://doc.rust-lang.org/cargo/reference/rust-version.html | Déclarée (1.88/1.89), jamais testée | Oui | Faible | P2 |
| `--locked` partout en CI | pratique Cargo | FFI seulement | Oui | Faible | P2 |
| Fuzzing des décodeurs (MessagePack, QUIC) | https://rust-fuzz.github.io/book/cargo-fuzz.html ; Scorecard Fuzzing | Non (proptest dans 3 crates) | Oui — octets non fiables | Moyen | P2 |
| SAST (CodeQL, gratuit en public) | Scorecard SAST | Non | Moyen | Faible | P2 |
| Signature/provenance des releases (XCFramework SPM) | Scorecard Signed-Releases | Checksum seul | Oui | Moyen | P2 |
| `#![forbid(unsafe_code)]` sur crates sans unsafe ; Miri ailleurs | pratique Rust [URL non ouverte] | Non | Oui (hors FFI) | Faible/moyen | P2 |
| Keep a Changelog + SemVer cohérents | https://keepachangelog.com , https://semver.org [non ouverts] | Partiel (release-please, voir §2) | Oui | Faible | P2 |
| cargo-vet ; CODEOWNERS / PR template / Code of conduct | https://github.com/mozilla/cargo-vet ; GitHub community standards | Non | Faible tant que mono-mainteneur | Moyen / faible | P3 |
| Acquis : Conventional Commits (`commitlint` + hook) ; pas de `pull_request_target` | https://www.conventionalcommits.org/en/v1.0.0/ ; Scorecard Dangerous-Workflow | Oui | — | — | — |

## 4. Santé technique (C)

| Commande | Durée | Résultat |
|---|---|---|
| `cargo build --workspace --locked` | 1 min 43 s | exit 0, 0 warning [VÉRIFIÉ, `scratchpad/C/build.out`] |
| `cargo test --workspace --locked` | ~8 min (compilation incluse) | arrêt au 1er échec : `tom-transport` `node::tests::gate_rejects_unlisted_peer` (`node.rs:1489`, « not allowed ») ; rejoué seul en `--test-threads=1` (comme la CI, `ci.yml:96`) : 48/48 verts en 29 s [VÉRIFIÉ] |
| `cargo test --workspace --locked --no-fail-fast` | 285 s | 49 suites, **1 500 passés, 1 échec, 10 ignorés** ; échec = `tom-sdk` `two_clients_exchange_message_via_tickets` (`tests/two_clients.rs:48`, timeout 30 s), rejoué seul : vert en 0,14 s [VÉRIFIÉ] → 2 tests réseau instables sous charge parallèle ; `tom-sdk` n'est pas en CI |
| `cargo clippy --workspace -- -D warnings` | — | **non lancé** localement (plafond disque, voir incident) ; en CI les 6 jobs clippy échouent sur `tom-quinn/build.rs:5` [VÉRIFIÉ] |
| `scripts/check-ffi.sh` | — | non lancé (écrit dans `target/`, plafond disque) |
| `pnpm -r run build` | 9 s | exit 0 (avertissement vite sans erreur) [VÉRIFIÉ par C] |
| `pnpm test` | 5,6 s | 43 fichiers, **771 tests verts** [VÉRIFIÉ par C] |
| `biome check .` | — | 147 fichiers, 0 erreur [VÉRIFIÉ par C] |
| `cargo fmt --all -- --check` | — | exit 1, 164 fichiers en écart [VÉRIFIÉ par B] |

- **Workspace** : 16 membres ; exclus `tom-protocol-ffi` (patch tvOS), `tom-quinn-udp`, `experiments/iroh-poc` (`Cargo.toml:2-7`) [VÉRIFIÉ]. `tom-quinn-udp` : exclu, aucun consommateur, `Cargo.toml` « AUTOMATICALLY GENERATED » (artefact de publication commité) ; `tom-sdk` : aucun consommateur Rust ni job CI → candidats à trancher [VÉRIFIÉ par C]. `patches/netwatch-0.13.0` non référencé [VÉRIFIÉ par C].
- **TypeScript legacy** : vivant en CI (job `typescript`), donc pas du code mort ; `tools/signaling-server` marqué DEPRECATED mais toujours construit via `pnpm-workspace.yaml` (`tools/*`) [VÉRIFIÉ par C]. Dernier commit `apps/demo` : 07/02 ; `packages/` : 18/07.
- **Fichiers suivis parasites** : `.playwright-mcp/` (11 logs, non ignoré) ; un PDF + deux HTML à la racine de `docs/` (doublons du `.md`) ; `results/` 122 fichiers. Pack 37,7 Mio, aucun fichier > 1 Mo [VÉRIFIÉ].
- **Dette** : 109 `TODO/FIXME/XXX/HACK` dans `crates/` (53 dans `tom-quinn-proto`) [VÉRIFIÉ] ; 37 `allow(dead_code|unused)` ; `unsafe` surtout FFI et `tom-quinn-udp`, aucun `forbid(unsafe_code)` ; `runtime/state.rs` = 324 Ko [VÉRIFIÉ par C].
- **`.gitignore`** : 0 fichier suivi ignoré ; motifs périmés `apps/relay-tvos/*` (dossier absent) ; redondance l.13-14 / l.32 [VÉRIFIÉ par C].
- **Toolchain** : pas de `rust-toolchain.toml` ; local rustc 1.97.0, CI 1.99.x ; MSRV jamais testée [VÉRIFIÉ].

**Incident pendant l'audit** [VÉRIFIÉ] : la chaîne cargo du sous-agent C a porté `target/` de 10 à 22 Go (plafond dur 20 Go) et est restée vivante à son retour (arrêt refusé au sous-agent). L'orchestrateur a arrêté le script parent (clippy n'a pas démarré), laissé finir `cargo test` (borné 900 s), rejoué en `--no-fail-fast`, puis `cargo clean` (règle « `target/` ≤ 20 Go » du `CLAUDE.md` ; 29,9 Gio de cache régénérable supprimés, aucun fichier suivi touché, `git status` inchangé). `crates/tom-protocol-ffi/target` (7 Go, préexistant) non touché. Leçon : un sous-agent qui compile le workspace complet doit recevoir un budget disque ET une borne de sortie (pas de chaîne de fond).

## 5. Code vs ADN (D)

Sources : résumé des fondateurs hors dépôt (v0.1 FR + « Step 2 ») et, dans le dépôt, `docs/tom-whitepaper-v1.md` (479 l., sans date interne, commit 06/03), `docs/MISSION.md` (rédigé 26/06), `docs/gitbook/whitepaper-v1.md` (stub), `docs/gitbook/whitepaper-v2.md` (plan de 29 l.), `_bmad-output/planning-artifacts/design-decisions.md` (7 LOCKED, fév. 2026). `[T]` = fait par le transport forké ; `[ToM]` = couche protocole.

| Principe | Statut | Preuve |
|---|---|---|
| P1 aucun serveur ni relais externe | **Contredit (partiel)** | `tom-transport/src/config.rs:172` `n0_discovery: true` par défaut → Pkarr/DNS n0 (`tom-relay/src/dns.rs:32`) ; `CLAUDE.md:129` « Relays are not optional » ; `DEFAULT_RELAY_URLS = &[]` (`config.rs:12`) ; DHT Mainline tierce [VÉRIFIÉ] ; ni Bluetooth ni Tor |
| P2 Subnets éphémères | **Partiel** | `discovery/subnet.rs` clusters BFS 3–10 nœuds, dissolution 5 min ; ni fusion, ni fork, ni veille ; non consommés par le routage |
| P3 Routage qui apprend | **Absent [ToM] / partiel [T]** | `relay.rs:~290` premier candidat par `last_seen` (`MostRecent`) [VÉRIFIÉ] ; RTT seulement dans `tom-transport/src/path.rs` |
| P4 Vie privée par défaut | **Partiel** | E2E réel (`crypto.rs`) ; `envelope.rs:14-36` `from`/`to`/`via` en clair pour les relais ; rétention 24 h |
| P5 L1 état présent, genèse glissante, snapshots | **Partiel** | purge TTL locale réelle ; aucune L1-registre ni snapshot ; « L1 » du code = phases de présence (`presence/mod.rs:1`) [VÉRIFIÉ] |
| P6 Consensus PoP, quorums tournants | **Partiel (PoP) / absent (consensus)** | primitive PoP (`presence/mod.rs:1`) [VÉRIFIÉ], 2–4 témoins de liveness (`presence/quorum.rs`) ; aucun quorum qui signe un état ; hub = NodeId le plus bas, prévisible |
| P7 Rôles dynamiques (6) | **Partiel** | `relay.rs:43` `PeerRole { Peer, Relay }` [VÉRIFIÉ] ; Observateur abandonné (`vault/20-decisions.md`, 12/07) ; Gardien/Archiveur/Validateur absents |
| P8 Score = Contribution − Usage | **Partiel** | `roles/scoring.rs` contribution seule, 0 « usage » [VÉRIFIÉ] ; aucun jeton |
| P9 Micro-PoW + sur-assignation | **Absent / partiel** | aucune PoW (mentions = commentaires) [VÉRIFIÉ] ; `roles/antispam.rs` débit progressif seul ; KL#13 ouvert |
| P10 Double dépense (State_W, Tx_W, Obs_W) | **Absent** | 0 occurrence `wallet/aggsig/from_commit/pedersen` [VÉRIFIÉ] — cohérent avec « paiements ensuite » |
| LOCKED 1 ACK | **Implémenté** | ACK signé vérifié avant changement de statut (`state.rs:~1189`) |
| LOCKED 2 TTL 24 h | **Partiel** | TTL par nœud ancré à l'origine, pas de purge globale garantie |
| LOCKED 3 L1 ancre | **Absent** | pas de L1 |
| LOCKED 4 Réputation | **Implémenté** | décroissance `scoring.rs:8`, pas de ban ; Relais→Peer = bascule au seuil |
| LOCKED 5 Anti-spam | **Partiel** | débit progressif, pas d'effort imposé |
| LOCKED 6 Invisibilité | **Partiel** | `NodeStatusFFI` expose `path_kind`, `relay_url_active`… |
| LOCKED 7 Universel | **Partiel** | groupes et `ReadReceipt` dans la couche protocole |

**Divergences entre documents (listées, non tranchées)** :
1. Rôles : « imprévisibles, sélectionnés au dernier moment » (`docs/tom-whitepaper-v1.md:15`, `MISSION.md:32`) vs « rôles annoncés à l'avance » (fondateurs hors dépôt).
2. Token : exclu (`MISSION.md:53` « Pas de token » [VÉRIFIÉ], v1 l.263-267) vs « jeton natif à terme » et jeton « mesure d'équilibre » (hors dépôt).
3. Observateur : cœur de la double dépense (v1 l.165, 181-189 ; hors dépôt Obs_W) vs abandon `vault/20-decisions.md` (12/07), révocation non tracée dans le v1.
4. Engagement « hash ou Merkle » (v1 l.177) vs non tranché ; quorum « milliers » (v1 l.15) vs k non fixé.
5. Subnets : par conversation (v1 l.145-155) vs cycle fusion/veille (hors dépôt) vs clusters BFS (code).
6. Routage qui apprend, Archiveur, « pas de Bluetooth », phasage paiements → jeton : hors dépôt seulement.
7. Ancienneté : v1 « Version 1.0 » sans date ; hors dépôt « v0.1 » mais plus formalisé — ordre inconnu [HYPOTHÈSE].
- Contradictions internes au dépôt : `CLAUDE.md:129` « Relays are not optional » vs `MISSION.md` §1.1 « aucun serveur, aucun relais » ; « Bootstrap Elimination DONE » vs `n0_discovery(true)` par défaut ; `vault/40-roadmap.md` R17 « URLs déjà en défaut committé » vs `DEFAULT_RELAY_URLS = &[]` ; `MISSION.md` §3/§5 (verrou #1 « ❌ ») périmé vs KL#5 résolu ; `vault/00-brief.md` périmé (phase 3, NAS `.21`, signaling présent) ; lien mort `docs/gitbook/whitepaper-v1.md:7` ; vocabulaire récent (« murmuration », « rendez-vous tournant », « L1-miroir ») absent de tout fondateur.

## 6. Plan de remise à niveau proposé (aucun lot exécuté ici)

| # | Lot (futur mandat) | Périmètre / fichiers | Niveau | Effort | Risque | Irréversible |
|---|---|---|---|---|---|---|
| 1 | **CI verte — outillage** | `rust-toolchain.toml` (épingler), `crates/tom-quinn/build.rs` (lint), `package.json`/`pnpm-lock.yaml` (`claude-atelier`) | N2 | 0,5 j | Faible | Non |
| 2 | **CI verte — advisories TLS** | `Cargo.toml`/`Cargo.lock` (h2, rustls), `deny.toml` (justifier tout ignore), lock FFI | **N3** (crypto/TLS) | 0,5–1 j | Moyen (montée rustls touche le transport) | Non |
| 3 | **Protéger `main`** | ruleset (checks requis, pas de force-push), `delete_branch_on_merge`, un seul mode de merge ; décider le sort des pushes de vault | N2 | 1 h | Moyen (peut bloquer le harnais) | Réglage réversible |
| 4 | **Sécurité dépôt** | Dependabot (`.github/dependabot.yml` + alertes), `SECURITY.md`, `permissions:` minimales, actions épinglées par SHA, CodeQL | N2 | 0,5 j | Faible | Non |
| 5 | **Couverture CI** | `ci.yml` : `clippy --workspace --all-targets`, `fmt --check`, `tom-sdk`/`tom-tui`/`tom-gateway`, `cargo test` FFI, job MSRV, `--locked`, `paths-ignore: vault/**`, `concurrency` | N2 | 1 j | Faible | Non |
| 6 | **Reformatage `cargo fmt`** | 164 fichiers, commit isolé (+ `.git-blame-ignore-revs`) | N1 | 1 h | Faible (gros diff, conflits avec branches ouvertes) | Non |
| 7 | **Garde-fous locaux** | `.claude/settings.json` (hooks manquants, `\|\| exit 0`, `bypassPermissions`, `deny` `--no-verify`), `.husky/pre-push` → gate | N2 | 0,5 j | Moyen (change la façon de travailler) | Non |
| 8 | **Exposition infra perso** | retirer IP publique/LAN, `ssh root@`, chemins `/Users/malik` des fichiers suivis (60 lignes / 34 fichiers pour l'IP publique) | N2 | 0,5 j | Faible | **Purge de l'historique = irréversible** (réécriture, déjà vécue le 12/07) |
| 9 | **Releases et versions** | PR #56, tag `v2.1.0` + release, `CHANGELOG.md` (doublon 1.0.0), politique de version des crates | N2 | 0,5 j | Faible | Publier une release = public |
| 10 | **Ménage du dépôt** | `.playwright-mcp/` (+ `.gitignore`), `docs/*.pdf`/`*.html`, `results/`, `.github/skills/`, `.github/ISSUE_BACKLOG.md`, motifs `.gitignore` périmés, licence Apache des forks (texte ou `Cargo.toml`), `tom-quinn-udp`, `patches/netwatch-0.13.0`, `ed25519-dalek` en `=` | N1/N2 | 0,5 j | Faible | Suppressions (récupérables par git) |
| 11 | **Branches mortes** | suppression des 10 branches listées §2 (après contrôle des PR squash) | N1 | 15 min | Faible | **Oui** (refs distantes) |
| 12 | **Docs de règles** | `CLAUDE.md`/`.claude/CLAUDE.md` (état/dates à sortir, ≤ 150 l., réfs mortes, Known Limitations), règle trailer unique, `CONTRIBUTING.md`, `vault/00-brief.md`, `docs/MISSION.md` §3/§5, lien mort gitbook | N1 | 1 j | Faible | Non |
| 13 | **Carte ADN ↔ code** | après décision « quel texte fait foi » : doc d'écart versionné + révocations tracées (Observateur, token), terme « L1 » | N2 | 1 j | Faible | Non |
| 14 | **Supply chain avancée** | `cargo-deny [licenses]`, fuzz `envelope` (MessagePack), `forbid(unsafe_code)`, provenance XCFramework | N2/N3 | 2–3 j | Faible | Non |

Ordre recommandé : 1 → 2 → 5 → 3 → 4, puis 6/7/8 (8 avant toute communication publique), 9–14 au fil de l'eau.

### Décisions à prendre par le fondateur

1. **Protection de `main`** : PR obligatoire pour tout, ou exception pour les commits de vault ? Quels checks requis ?
2. **Branches** : supprimer les 10 branches listées (irréversible côté distant) ?
3. **Infra perso dans un dépôt public** : simple retrait des fichiers, ou purge de l'historique (réécriture, irréversible) ?
4. **Quel texte fait foi pour l'ADN** : whitepaper v1 du dépôt, `MISSION.md`, ou les fondateurs v0.1 / « Step 2 » ? En particulier : rôles imprévisibles vs annoncés, token exclu vs jeton à terme, Observateur rétabli ou abandonné.
5. **Dépendance n0 par défaut** (`n0_discovery: true`) et DHT Mainline tierce : tolérées au regard de « aucun relais externe » ?
6. **`niveaux.critiques`** à établir (au moins : `crates/tom-protocol/src/crypto.rs`, `envelope.rs`, `presence/`, `tom-base` clés, `tom-quinn-proto` crypto, `rustls`/`ring`/`ed25519-dalek` dans les `Cargo.toml`, `deny.toml`, futur wallet).
7. **Trailer `Co-Authored-By`** : une seule règle (3 sources contradictoires aujourd'hui).
8. **Hooks `claude-atelier`** : les réinstaller (et réparer la devDependency 404) ou les retirer de `settings.json` ; garder `bypassPermissions` commité ?
9. **Versions** : release-please seul maître, ou tags manuels ? Les crates suivent-ils la version produit ?
10. **TypeScript legacy et `tools/signaling-server`** : garder en CI, geler ou archiver ? Sort de `tom-sdk` et `tom-quinn-udp`.

## Annexe — index QMD (étape 1)

- Avant : 187 fichiers, 3 collections, 187 en attente, mis à jour il y a 88–116 j. Ajouts (`collection add --mask`) : `depot` (racine `*.md` + `crates/ apps/ packages/ tools/ patches/ experiments/ scripts/ results/ deploy/ sdk/`), `claude`, `github`, `bmadsrc` (`_bmad/`) ; exclusions YAML `ignore:` dans `~/.config/qmd/tom.yml` : `vault/runtime/**`, `.claude/worktrees/**`, `**/target/**`, `**/.build/**`, `**/DerivedData/**`.
- Après : 528 fichiers pour 546 `.md` suivis (écart = 21 symlinks `.github/skills/` déjà indexés via `claude`) ; `embed` : 2 587 morceaux / 527 docs en 4 min 50 s, ligne `Pending` disparue du `status` ; requêtes MCP de contrôle OK (`depot/CLAUDE.md`, `vault/reprise/F01.md`) [VÉRIFIÉ]. Reste 96 morceaux orphelins (`qmd cleanup` non lancé). Le verrou d'embed est global à `~/.cache/qmd/` : un embed d'une autre fenêtre sur l'index par défaut a bloqué celui-ci ~6 min.
