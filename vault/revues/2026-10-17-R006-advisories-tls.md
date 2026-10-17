---
date: 2026-10-17
revue: R006
mandat: M0008
branche: fix/advisories-tls
verdict: GO
tip: d15da521d6d54386fb6d7589ccf3b64c60b3427e
niveau: N3
---

# R006 — Second doublage N3 advisories TLS (après correctif R005)

Périmètre de la revue : worktree détaché `F02-R006` (HEAD `d15da52`, `git status` vide avant et après la revue). Aucun build, test, check, clippy ni run. Les outils autorisés ont servi : `cargo deny`, `cargo tree`, `cargo metadata --locked`, lecture du registre et de `gh`.

## 1. Advisories résolues

- Racine : `cargo deny check advisories bans sources` → `advisories ok, bans ok, sources ok` (sortie vue sur le tip).
- FFI (lock `crates/tom-protocol-ffi/Cargo.lock` au tip, depuis le worktree hors dépôt `F02-ffi`, HEAD `d15da52`) : `advisories FAILED` avec **une seule** erreur, `RUSTSEC-2026-0204` (crossbeam-epoch 0.9.18 via moka 0.12.14), hors périmètre comme prévu. Aucune autre erreur.
- Base `9278dca` (exportée dans le scratchpad, `deny.toml` de la base copié) :
  - racine : `advisories FAILED` avec 5 erreurs : `Debug` sessions TLS (rustls), Brotli (rustls), h2 DATA vides, handshake TLS 1.3 niveau de chiffrement (rustls), ServerHello non authentifié (rustls). Ce sont les 5 advisories visées.
  - FFI : 8 erreurs, dont RUSTSEC-2026-0258, -0285, -0346, -0348, -0349, **et** RUSTSEC-2026-0044 et -0048 (AWS-LC : Name Constraints, CRL Distribution Point), ainsi que 0204. Le tip ne remonte ni 0044 ni 0048. Le correctif résout donc aussi deux advisories AWS-LC côté FFI qui ne figurent pas dans le message de commit (bonne nouvelle, pas un défaut).
- `git diff 9278dca d15da52 -- deny.toml` : vide. Aucun `ignore` ajouté.
- `.github/workflows/ci.yml` : seul changement, ligne 192 (`# 7 advisories ignorées dans deny.toml`). Les 7 entrées `"RUSTSEC-` actives sont bien dans `deny.toml:20-24` et `:34-35`.

## 2. Défaut R005 corrigé : arêtes du lock

Comparaison indépendante (script `R006/compare.py` et `lockparse.py` dans le scratchpad, sans le script de l'auteur). Pour chaque paquet présent à l'identique (nom + version + source) dans la base et au tip, j'ai comparé l'ensemble de ses arêtes résolues.

- Racine : 644 paquets à la base, 648 au tip, 640 communs. 15 paquets communs ont des arêtes modifiées. **Aucune arête modifiée n'est hors liste blanche** (rustls, rustls-webpki, h2, aws-lc-*, bindgen, shlex) : `UNEXPECTED count: 0`.
- FFI : 533 paquets chacun, 528 communs, 16 paquets à arêtes modifiées, `UNEXPECTED count: 0`.
- Arêtes `socket2` et `windows-sys` : comptes identiques base/tip, dans les deux locks (racine : socket2 0.6.2 ×11, socket2 0.5.10 ×1, windows-sys 0.61.2 ×17, etc. ; FFI : socket2 0.6.2 ×11, windows-sys 0.61.2 ×12, etc.). Le défaut R005 est corrigé.
- `cargo tree -i socket2@0.6.2 --workspace --target all -e normal --locked` (racine) remonte bien à `tom-quinn v0.16.1` puis `iroh-quinn-udp v0.8.0` (et `iroh-quinn-udp v0.7.0`, `hyper-util`).
- Paquets présents uniquement à la base (racine) : aws-lc-rs 1.17.0, h2 0.4.13, rustls 0.23.36, rustls-webpki 0.103.13. Uniquement au tip : aws-lc-fips-sys 0.14.3, aws-lc-rs 1.18.2, aws-lc-sys 0.46.0, bindgen 0.73.2, h2 0.4.20, rustls 0.23.46, rustls-webpki 0.103.15, shlex 2.0.1. Tous sont des montées voulues ou des ajouts liés (bindgen/shlex/aws-lc-sys).

## 3. Locks valides

- `cargo metadata --locked --format-version 1 >/dev/null` : racine EXIT 0 ; FFI EXIT 0 (depuis `F02-ffi/crates/tom-protocol-ffi`, HEAD `d15da52`).
- Versions identiques dans les deux locks (vérifié par lecture) : `Cargo.lock:4136-4137` et `crates/tom-protocol-ffi/Cargo.lock:3230-3231` rustls 0.23.46 ; `Cargo.lock:1925-1926` et `crates/tom-protocol-ffi/Cargo.lock:1310-1311` h2 0.4.20 ; `Cargo.lock:4235-4236` et `:3329-3330` rustls-webpki 0.103.15 ; aws-lc-rs 1.18.2 aux lignes `Cargo.lock:425` et `crates/tom-protocol-ffi/Cargo.lock:241`.
- Orphelins : aucun paquet à source sans arête entrante (`tip-root` 648 paquets, `tip-ffi` 533 paquets, 0 orphelin). Cargo n'a pas besoin de réécrire les deux locks en `--locked`.

## 4. aws-lc dans le FFI et CryptoProvider

**Constat de l'auteur confirmé.**
- Racine : `cargo tree -i aws-lc-rs --workspace --target all -e normal --locked` → `package ID specification aws-lc-rs did not match any packages`. aws-lc-rs est dans le lock (via `tom-quinn-proto` optionnel) mais hors du graphe de features du workspace. rustls racine n'a que les features `ring`, `std`, `tls12`, `logging`, `log`. `tom-quinn-proto` racine : `rustls-ring` seulement.
- FFI : chemin réel `aws-lc-rs 1.18.2` ← `rustls` ← `hyper-rustls` (feature `aws-lc-rs`) ← `reqwest` (feature `__rustls-aws-lc-rs`) ← `pkarr 5.0.3`. `reqwest 0.13.2` utilise aws-lc par défaut (`reqwest-0.13.2/src/async_impl/client.rs:2460-2464`). `tom-quinn-proto` FFI : features `ring` et `rustls-ring` seulement, donc `configured_provider()` (`crates/tom-quinn-proto/src/crypto/rustls.rs:583-587`) retient ring.
- **Déjà vrai à la base** : `cargo tree -i aws-lc-rs --target all -e normal,features --locked` dans un export `git archive 9278dca` (scratchpad `R006/base-src`) donne aws-lc-rs 1.16.1 par le même chemin (`hyper-rustls feature "aws-lc-rs"` → `reqwest feature "__rustls-aws-lc-rs"` ← `pkarr v5.0.3`). Les features rustls FFI sont identiques base/tip (`aws_lc_rs`, `ring`, `logging`, `std`, `tls12`, `log`).

**Réponse à « rustls peut-il choisir aws-lc par défaut dans le FFI ? » : non, ni à la base ni au tip.**
- rustls 0.23.37 (`rustls-0.23.37/src/crypto/mod.rs:243-256`) et 0.23.46 (`rustls-0.23.46/src/crypto/mod.rs:243-256`) : `get_default_or_install_from_crate_features()` ne sélectionne rien quand les deux features `ring` et `aws-lc-rs` sont actives (`from_crate_features()` renvoie None) et **panique** au premier appel `ClientConfig::builder()` / `ServerConfig::builder()` sans provider installé. Logique identique dans les deux versions.
- Aucun `install_default` ni `get_default` dans `crates/` (grep vide hors commentaires de `patches/`).
- Appels nus de `builder()` dans l'arbre FFI : `tokio-rustls-acme 0.9.0` (`config.rs:64`, `acceptor.rs:20`, `incoming.rs:68`), compilé dans le FFI par le feature `default` de tom-relay (`tokio-rustls-acme` optionnel activé par défaut). Ce chemin ne tourne que pour la gestion ACME du relais ; je n'ai pas tracé qu'il est inatteignable depuis le FFI (remarque 1).
- `reqwest 0.12.28` (`reqwest-0.12.28/src/async_impl/client.rs:763-771`) prend le provider process-default, sinon **ring** (fallback explicite). `reqwest 0.13.2` prend **aws-lc** (voir ci-dessus). Pré-existant, identique base/tip.
- Crates ToM : provider explicite ring partout (`crates/tom-connect/src/tls.rs:72,106`, `crates/tom-connect/src/socket.rs:1105`, `crates/tom-relay/src/client.rs:483`, `crates/tom-relay/src/client/tls.rs:66`, `crates/tom-quinn-proto/src/crypto/rustls.rs:301,345,482`).

**Changements aws-lc-rs 1.16.1 → 1.18.2 / aws-lc-sys 0.38.0 → 0.46.0** (notes `gh release view` v1.17.0 à v1.18.2, `aws/aws-lc-rs`) :
- MSRV `1.71.0` (aws-lc-rs 1.16.1, 1.18.2 ; aws-lc-sys 0.38.0, 0.46.0) : inférieur au `rust-version = "1.89"` (`Cargo.toml:14`).
- 1.18.0 : bascule aws-lc-fips-sys vers FIPS 4.x (la note demande de pinner `<1.18.0` pour rester en FIPS 3.x). **Non concerné** : `aws-lc-fips-sys` n'est dans aucun arbre compilé (`cargo tree -i aws-lc-fips-sys` : "did not match any packages", racine et FFI).
- 1.18.0 : optimisation de taille automatique quand `opt-level` vaut `s` ou `z`. Aucun `opt-level`, `OPT_LEVEL` ou `CARGO_PROFILE` dans `scripts/`, `.cargo/`, `crates/tom-protocol-ffi/Cargo.toml` et le Cargo.toml racine (grep vide). Pas de changement de comportement à attendre.
- 1.18.0 : `KeyEncryptionKey::unwrap` ne déborde plus ; zeroize des clés KEK. Non atteint par ToM (aucun usage relevé).
- 1.18.0 : `rsa::KeyPair::from_components` (API nouvelle), bindgen 0.72 → 0.73 (1.18.2).
- 1.18.2 : aws-lc-sys 0.46 aligné sur AWS-LC v5.11 (Ed25519 rejette les clés publiques non canoniques, etc.). Pas de nouvelle build-dep : `aws-lc-sys` 0.46 a la même liste de build-deps que 0.38 (`bindgen`, `cc`, `cmake`, `dunce`, `fs_extra`), et `fips = ["dep:bindgen"]` est nouveau.
- **Non vérifié** : `aws-lc-sys/build.rs` et les changements de chaîne de build (cmake, cibles Apple) entre 0.38 et 0.46 n'ont pas été lus ligne à ligne. La cross-compilation iOS/tvOS n'a pas été refaite (build interdit).

**ML-DSA par défaut (rustls 0.23.44)** : R005 l'a noté « non actif ». C'est vrai pour le workspace (aws-lc absent), mais pas pour le FFI, où le provider aws-lc sert `reqwest 0.13.2` (pkarr). Le code ML-DSA se trouve dans `rustls-0.23.46/src/crypto/aws_lc_rs/sign.rs`. Il n'y a pas d'effet de sécurité attendu (ML-DSA non présent dans la WebPKI publique, et pas de certificat ML-DSA côté pkarr), mais l'ensemble de schémas annoncé change dans ce client. Remarque, pas un défaut prouvé.

## 5. Régression d'API TLS et h2

Recontrôle des affirmations de R005 :
- `crates/tom-relay/src/client.rs:499-528` : `impl ServerCertVerifier for NoCertVerifier` implémente bien `verify_server_cert`, `verify_tls12_signature`, `verify_tls13_signature`, `supported_verify_schemes` (exact).
- `crates/tom-relay/src/client/tls.rs:66-80` : `builder_with_provider(ring)`, `with_safe_default_protocol_versions()`, `Resumption::default()` (exact).
- `crates/tom-connect/src/tls/verifier.rs:16` : `use webpki::ring as webpki_algs` (exact, R005 disait 12-22 pour le bloc).
- Diff public API rustls `quic.rs` et `verify.rs` entre 0.23.37 et 0.23.46 (`grep "pub "`) : vide. Cohérent avec « additif » pour ce périmètre.
- h2 0.4.20 : `CHANGELOG.md` de la registry confirme 0.4.15 (rejet des trames sur streams sans HEADERS envoyés), 0.4.18/0.4.19 (`data_frame_budget`, défaut modifié), 0.4.20 (GOAWAY stream id non nul). **Écart** : R005 attribue à 0.4.16 le « rejet des en-têtes de connexion dans `send_trailers` » ; le CHANGELOG 0.4.16 ne le mentionne pas, et je n'ai pas identifié la version qui a introduit ce contrôle. Il existe bien dans 0.4.20 : `h2-0.4.20/src/proto/streams/send.rs:312-330` (`send_trailers` appelle `check_headers`). Le mécanisme est réel, l'attribution de version ne l'est pas.
- h2 est compilé en production : `hyper` avec feature `http2` actif (racine et FFI). Je n'ai pas tracé chaque parent exact de cette feature. ToM n'appelle pas h2 directement (R005 et `grep` vérifié).

**PR rustls 3332 et 3342 (non évaluées par R005)** :
- 3332 « conn: be more careful about confidentiality limits with 1.2 » (mergée 2026-10-06). Diff lu sur les fichiers `rustls/src/conn/mod.rs`, `send.rs`, `split.rs`, `error/mod.rs` : le changement est le rejet d'écriture (`EncryptError`) à la limite de confidentialité TLS 1.2, avec alerte `NoRenegotiation`. Les limites par défaut TLS 1.2 sont `1 << 24` pour AES-GCM (`rustls-0.23.46/src/crypto/ring/tls12.rs:53-95` et `aws_lc_rs/tls12.rs:55-97`) et `u64::MAX` pour ChaCha/Gcm ailleurs. Impact sur `crates/tom-relay/src/client/tls.rs:66-80` (TLS-TCP vers le relais, ring, TLS 1.2 + 1.3) : seulement après environ 2^24 enregistrements sous une même clé. Effet concret dans ToM : négligeable, mais non mesuré.
- 3342 « Timely 1.3 key updates » (mergée 2026-10-12). Diff de `rustls/src/quic.rs` : refactor des signatures `handshake_secrets` / `traffic_secrets` vers une struct `SecretUpdate` (hunk de 112 lignes, aucun changement de logique visible dans les lignes lues). Le changement de comportement (key_update placé sur l'enregistrement précédant la limite) ne touche que les connexions TLS 1.3 au-delà de 2^23 enregistrements sous une clé. `crates/tom-quinn-proto/src/crypto/rustls.rs` n'utilise aucun `key_update`/`KeyUpdate` (grep vide) : le QUIC n'est pas concerné par la mise à jour de clés TLS 1.3. Impact TCP relais : même remarque de volume que 3332.
- Restent non lus : la totalité des diffs de 3332 et 3342 au-delà des 120 premières lignes, le code de `rustls/src/conn/send.rs` lié à 3332.

## Remarques (non bloquantes)

1. `tokio-rustls-acme` compilé dans le FFI (feature `default` de tom-relay) contient des appels nus `ClientConfig::builder()` / `ServerConfig::builder()` (`tokio-rustls-acme-0.9.0/src/config.rs:64`, `acceptor.rs:20`, `incoming.rs:68`), qui panique avec les deux providers actifs. Pré-existant (mêmes features et mêmes versions de tokio-rustls-acme à la base). Non atteint par le FFI à ma connaissance ; à vérifier dans une session de build si ACME est un jour exposé dans le FFI. Hors périmètre de ce correctif.
2. Dans le FFI, `reqwest 0.12.28` et `reqwest 0.13.2` ne choisissent pas le même provider sans installation explicite (ring pour 0.12 en fallback, aws-lc pour 0.13). Pré-existant. Un `install_default` explicite côté FFI supprimerait l'ambiguïté : suggestion pour un chantier séparé.
3. Le commit annonce 5 advisories ; le FFI en résout 7 (dont RUSTSEC-2026-0044 et -0048, AWS-LC). Le message de commit gagnerait à le mentionner.
4. R005 attribue le contrôle `send_trailers` à h2 0.4.16 sans preuve. Le contrôle existe en 0.4.20 (`send.rs:312-330`). Corriger l'attribution dans la note de revue, pas le code.
5. R005 : « ML-DSA non actif » n'est vrai que pour le workspace (voir section 4).

## Non vérifié

- Aucun `cargo build`, `test`, `check`, `clippy` ni cross-compilation iOS/tvOS (interdit dans cette session). Les affirmations de l'auteur sur 1501 tests, clippy et les cibles Apple ne sont pas revérifiées.
- `aws-lc-sys/build.rs` 0.38 → 0.46 et chaîne de build Apple : non lus.
- PR rustls 3332 et 3342 : lecture partielle (premières ~120 lignes de diff chacune).
- Dépendance h2 exacte portée par `hyper` `http2` (parent direct) : non tracée.
- `cargo deny` FFI exécuté depuis le worktree `F02-ffi` (hors dépôt) : `deny.toml` y est la copie du tip, identique à la base (`git diff` vide sur `deny.toml` racine). Le `deny.toml` du dossier FFI n'a pas été comparé séparément.
- Runtime : aucun test de non-régression TLS/QUIC ni mesure de la rétrogradation socket2 (déjà noté par R005).
