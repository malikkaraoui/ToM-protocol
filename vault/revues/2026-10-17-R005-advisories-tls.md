---
date: 2026-10-17
revue: R005
mandat: M0008
branche: fix/advisories-tls
verdict: RESERVE
tip: 448d07c7696b15abc9f33d545f1f1a49c5abf2ee
niveau: N3
---

# R005 — Doublage N3 advisories TLS (h2 / rustls)

## Verdict

**RESERVE**, pour un seul défaut prouvé : le commit contient des **rétrogradations non déclarées** de dépendances du transport QUIC (`socket2` 0.6.2 → 0.5.10, plusieurs `windows-sys`), absentes du périmètre annoncé. Les advisories sont bien résolues et l'API TLS n'a pas de rupture. Le correctif est prouvé (section 3) : un lock restreint aux montées voulues passe `cargo metadata --locked` (racine et FFI) et `cargo deny`. Il n'est pas appliqué au worktree (lecture seule).

## 1. Advisories résolues

- HEAD (`448d07c`) : `cargo deny check advisories bans sources` → `advisories ok, bans ok, sources ok`.
- Base (`9278dca`, exportée en scratch) : `advisories FAILED, bans ok, sources ok` avec les 5 erreurs visées : RUSTSEC-2026-0258 (h2), -0285, -0346, -0348, -0349 (rustls). Le changement résout exactement ces 5 advisories.
- `git diff 9278dca 448d07c -- deny.toml` : vide. Aucun `ignore` ajouté. 7 entrées `"RUSTSEC-` actives (`deny.toml:20-24`, `:34-35`) ; le commentaire `.github/workflows/ci.yml:192` (« 7 advisories ignorées ») est exact.
- Versions dans les deux locks : rustls 0.23.46 (≥ 0.23.45/0.23.46 requis), h2 0.4.20 (≥ 0.4.16 requis), rustls-webpki 0.103.15. Identiques dans `Cargo.lock` et `crates/tom-protocol-ffi/Cargo.lock`.
- Remarque : `warning[advisory-not-detected]` sur `deny.toml:21` (RUSTSEC-2026-0002, lru) — présent aussi sur la base ; l'ignore semble ne plus matcher. Non bloquant, hors périmètre.

## 2. Ruptures et API TLS

**Changelogs lus** : notes GitHub rustls 0.23.37 → 0.23.46 (`gh api repos/rustls/rustls/releases`), h2 0.4.14 → 0.4.20 (`gh api`, et `~/.cargo/registry/src/*/h2-0.4.20/CHANGELOG.md`). MSRV inchangée sur les sous-versions utiles.

Ruptures et changements de comportement relevés :
- rustls 0.23.44 : `KeyLogFile` crée des fichiers en lecture seule propriétaire (positif). ML-DSA activé par défaut dans **aws-lc-rs** uniquement (non actif, voir §3).
- rustls 0.23.45/0.23.46 : corrections de sécurité (alignement handshake, GHSA-2mjx, Brotli, Debug des secrets de session, ECH, KX hints). Changements de comportement : rejet des `change_cipher_spec` protégés (PR 3295), gestion des mises à jour de clés TLS 1.3 (PR 3342), limites de confidentialité TLS 1.2 (PR 3332) — ces deux dernières non lues dans le source, non évaluées finement.
- h2 0.4.15 : rejet des trames sur streams sans HEADERS envoyés ; 0.4.16 : rejet des en-têtes de connexion dans `send_trailers`, rejet des DATA trop petites en rafale, GOAWAY à stream id non nul rejeté (0.4.20) ; 0.4.18/0.4.19 : nouvelle option `data_frame_budget` (défaut auto modifié en 0.4.19).

**Diff d'API publique rustls 0.23.36 → 0.23.46** (les deux versions sont dans le registre, comparées par `grep 'pub '`) : uniquement additif (`ClientConfig.check_selected_alpn`, `ClientConfig.send_ticket_request`, `ServerConfig.max_tls13_tickets`, `TicketRequest`, `MLKEM1024`) plus `tls13()` passé en `const fn`. `src/verify.rs` identique (trait `ServerCertVerifier` inchangé). `src/quic.rs` sans changement de signature. Aucun littéral de struct `rustls::ClientConfig {`/`ServerConfig {` dans `crates/` (les littéraux trouvés sont ceux de quinn-proto).

**Diff d'API publique h2 0.4.13 → 0.4.20** : changements de signatures (`GoAway::load`, `recv_headers`, `poll_complete`, etc.) ; `grep 'h2::' crates/` = aucun usage direct. h2 est atteint via hyper (feature http2) et hickory-net : les changements de comportement s'appliquent indirectement.

**Usages contrôlés (fichier:ligne)** :
- `crates/tom-quinn-proto/src/crypto/rustls.rs:301-305, 345-350, 482-485, 525-526` : builders `builder_with_provider`, `dangerous()`, `quic::ClientConnection::new(config, version, name, params)` et `ServerConnection::new(config, version, params)` — signatures inchangées (`quic.rs:163`, `:266`).
- `crates/tom-relay/src/client.rs:479-530` : `NoCertVerifier` implémente les 4 méthodes requises du trait (`verify.rs`, identique entre 0.23.36 et 0.23.46).
- `crates/tom-relay/src/client/tls.rs:63-81` : `with_safe_default_protocol_versions`, `Resumption::default()` (struct `client_conn.rs:480`, existe). Pas de rupture de signature. Peut être touché par PR 3332 (TLS 1.2) : non évalué.
- `crates/tom-connect/src/tls.rs:88,116` et `crates/tom-relay/src/quic.rs:482` (ce dernier dans `mod tests`, `quic.rs:359`) : `KeyLogFile::new()`, actif seulement si `SSLKEYLOGFILE` — changement de permissions, positif.
- Debug des valeurs de session (GHSA-c8m5) : aucun `ClientSessionValue` dans `crates/`.
- ECH : aucun usage trouvé dans `crates/`.
- `crates/tom-connect/src/tls/verifier.rs:12-22` : `webpki::ring` et `WebPkiSupportedAlgorithms` — non modifiés par la mise à jour (rustls-webpki 0.103.13 → 0.103.15, pas de changement d'API vérifié dans cette passe).

**MSRV** : rustls 0.23.46 = 1.71, h2 0.4.20 = 1.63, aws-lc-rs 1.18.2 = 1.71.0, aws-lc-sys 0.46.0 = 1.71.0, rustls-webpki 0.103.15 = 1.71, bindgen 0.73.2 = 1.71.0. Toutes ≤ `rust-version = "1.89"` du workspace (`Cargo.toml:14`) et de la FFI (`crates/tom-protocol-ffi/Cargo.toml:5`).

## 3. Locks

**Périmètre** : `git diff --name-only 9278dca 448d07c` = exactement `.github/workflows/ci.yml`, `Cargo.lock`, `crates/tom-protocol-ffi/Cargo.lock`. Aucun des fichiers « intrus » listés (`crates/tom-quinn/build.rs`, `rust-toolchain.toml`, `package.json`, `pnpm-lock.yaml`, `scripts/ci-local.sh`, `.husky/pre-push`, `CONTRIBUTING.md`) n'apparaît.

**Montées attendues (vérifiées par script sur les deux locks)** :
- Racine : rustls 0.23.36→0.23.46, rustls-webpki 0.103.13→0.103.15, h2 0.4.13→0.4.20, aws-lc-rs 1.17.0→1.18.2 ; ajouts aws-lc-sys 0.46.0, aws-lc-fips-sys 0.14.3, bindgen 0.73.2, shlex 2.0.1 (bindgen 0.72.1 et shlex 1.3.0 subsistent). Côté `tom-quinn` : dépendances optionnelles aws (aws-lc-sys 0.41.0 / aws-lc-fips-sys 0.13.14 conservées).
- FFI : rustls 0.23.37→0.23.46, rustls-webpki 0.103.13→0.103.15, h2 0.4.13→0.4.20, aws-lc-rs 1.16.1→1.18.2, aws-lc-sys 0.38.0→0.46.0.

**Défaut prouvé — rétrogradations non déclarées** (`lockdiff` sur les paires de locks) :
- `socket2 0.6.2 → 0.5.10` pour `tom-quinn 0.16.1`, `iroh-quinn-udp 0.8.0` (et 0.7.0), `quinn-udp 0.5.14`, `quinn 0.11.9`, `hyper-util 0.1.20` (racine et FFI).
- `windows-sys 0.61.2 → 0.52.0/0.48.0/0.59.0/0.60.2` pour `errno`, `rustix`, `tempfile`, `seize`, `winapi-util`, `nu-ansi-term`, `rustls-platform-verifier`, `iroh-quinn-udp`, `quinn-udp`.
- Impact vérifié : `cargo tree -i socket2@0.5.10 --workspace --target all -e normal` remonte à `tom-quinn v0.16.1` et `iroh-quinn-udp v0.8.0` (couche UDP QUIC), donc le transport tourne désormais sur socket2 0.5.10 au lieu de 0.6.2. Non déclaré dans les messages de commit, non requis par les advisories.
- Cause non forcée par les manifestes : la base (`9278dca`) passe `cargo metadata --locked` (EXIT 0 sur export propre), `socket2` accepte `>=0.5, <0.7` (quinn-udp, hyper-util) et `resolver = "2"` (`Cargo.toml:8`), donc le MSRV n'est pas en cause. La séquence `cargo update -p rustls --precise 0.23.46` puis `cargo update -p h2 --precise 0.4.20` rejouée sur la base produit **exactement** le lock de la tête (diff = 0 ligne). Je n'ai pas identifié pourquoi cargo réécrit ces arêtes ; c'est constaté, pas expliqué.

**Correctif prouvé (en scratch, rien écrit dans le worktree)** :
- `$SCR/restore.py` (script d'arêtes : rétablit `socket2 0.6.2` et `windows-sys` d'origine uniquement sur les paquets concernés).
- `/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/c4b4dd53-1f03-459e-a538-15940352b20d/scratchpad/root-restored.lock` : 16 arêtes restaurées. Validé : `cargo metadata --locked` EXIT 0 sur l'arbre de tête (export `448d07c`), et `cargo deny check advisories bans sources` → `advisories ok, bans ok, sources ok`. Diff restant vs base = uniquement les montées attendues et les arêtes aws optionnelles.
- `.../scratchpad/ffi-restored.lock` : 14 arêtes restaurées. Validé : `cargo metadata --locked` EXIT 0 dans l'export FFI (le lock de tête l'est aussi). Diff restant vs base = les 5 montées FFI.

**aws-lc « non compilé » — vérifié pour la racine** :
- `cargo tree -i aws-lc-rs/aws-lc-sys/aws-lc-fips-sys` (versions précises) : aucun paquet trouvé dans le graphe du workspace (`--target all -e features`).
- `cargo tree --workspace --target all -e features,normal,build,dev | grep -i aws-lc` : vide.
- Raison côté manifestes : `crates/tom-quinn-proto/Cargo.toml:150-154` et `crates/tom-connect/Cargo.toml:16` désactivent les features par défaut de rustls (`default-features = false`, features `ring` seule) ; `tom-quinn-proto` default = `rustls-ring` (`Cargo.toml:59`).
- **FFI : non vérifié par cargo** (interdit dans ce worktree). Déduit : même crates path, même lock, aws-lc-rs n'apparaît dans le lock FFI que comme arête de rustls/rustls-webpki/quinn-proto.

## 4. Vérifications de périmètre et de cohérence

- `cargo metadata --locked` : racine HEAD EXIT 0 ; FFI HEAD EXIT 0 (export scratch).
- `ci.yml` : seul changement = commentaire ligne 192. Aucune commande CI modifiée.
- Le commentaire « 7 advisories ignorées » correspond à `deny.toml` (7 entrées actives).

## Remarques (non bloquantes)

- `deny.toml:21` (RUSTSEC-2026-0002) : ignore possiblement obsolète (`advisory-not-detected`), déjà sur la base.
- Changements de comportement h2 (rejets plus stricts, budget DATA) : pas de test de non-régression dans ce commit ; ils touchent les clients HTTP/2 indirects (hickory, reqwest/hyper). À surveiller sur le chemin DoH/HTTP si utilisé.
- rustls PR 3342 (timing des mises à jour de clés 1.3) et PR 3332 (TLS 1.2) : non lus dans le source ; impact sur le TLS-TCP du relais (`tom-relay/src/client/tls.rs`) non évalué.
- La rétrogradation socket2 est aussi présente dans le lock FFI, donc dans le binaire XCFramework iOS/tvOS : à traiter dans le même correctif.

## Non vérifié

- Aucun `cargo build/test/check/clippy` (interdit) : les 1501 tests, la compilation et clippy annoncés par l'auteur ne sont pas revérifiés.
- `cargo tree` dans le worktree FFI : non fait (nested workspace), seulement déduction depuis les manifestes et le lock.
- Impact runtime du passage socket2 0.6.2 → 0.5.10 : non mesuré ; compatibilité API de `iroh-quinn-udp`/`quinn-udp` avec socket2 0.5.10 non vérifiée par compilation.
- Lecture détaillée des PR rustls 3332, 3342 et du code de mise à jour des clés : non faite.
- Le lock restauré (`root-restored.lock`, `ffi-restored.lock`) n'est pas appliqué ni commité ; il reste à l'auteur de le reprendre puis de relancer `cargo deny` et `cargo build --locked` côté session mère.
