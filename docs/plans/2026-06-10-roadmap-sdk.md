# Roadmap SDK — ToM Protocol

> Date : 2026-06-10 · Prérequis : lecture de `2026-06-10-audit-global-pre-sdk.md`
> Objectif : permettre à un développeur tiers de s'approprier le protocole ToM **le plus vite possible**, sans sacrifier la base.
> Principe directeur : ne rien réécrire — le cœur Rust est bon. Le travail est de l'**emballage** (API, distribution, docs, spec).
>
> **Décisions D1-D5 tranchées le 2026-06-10** (cf. audit §6) : tag git · Apple/SPM d'abord · TS legacy archivé · spec dès S1 · façade `tom-sdk`. La roadmap ci-dessous est exécutable telle quelle.

---

## Stratégie en une phrase

Livrer d'abord un **SDK Apple (Swift Package binaire)** car la chaîne FFI → XCFramework existe déjà, en parallèle d'un **crate Rust consommable par tag git** ; publier la **spec protocole** pour les implémenteurs tiers ; reporter crates.io (bloqué dalek pre-release), Android et WASM à des phases ultérieures.

```
S0 Hygiène ──► S1 API SDK Rust ──► S2 Distribution Apple ──► S4 Plateformes suivantes
                      │
                      └────────► S3 Spec protocole (parallélisable dès S1)
```

---

## Phase S0 — Hygiène de base (≈ 2-3 jours)

But : repo propre et CI digne de confiance avant d'exposer quoi que ce soit publiquement. Tâches indépendantes, parallélisables, zéro risque fonctionnel.

| # | Tâche | Détail | Fichiers |
|---|---|---|---|
| S0.1 | Purger `.build/` tvOS de git | `git rm -r --cached apps/tom-node-tvos/.build/` + ajouter `.build/` au .gitignore | `.gitignore`, `apps/tom-node-tvos/.gitignore` |
| S0.2 | Supprimer les Cargo.lock des crates exclus | tom-protocol-ffi, experiments/iroh-poc, patches/* | `git rm` ×3-4 |
| S0.3 | Réintégrer `tom-protocol-ffi` au workspace | La raison d'exclusion (« ne compile pas ») est obsolète. Vérifier `cargo clippy --workspace` passe ensuite | `Cargo.toml:3` |
| S0.4 | `[workspace.lints]` + `rust-version` partout | MSRV `1.89` sur les crates originaux ; hériter les lints dans chaque membre | Cargo.toml racine + 7 crates |
| S0.5 | CI sécurité | Job `cargo audit` + `deny.toml` minimal (licenses, yanked, sources git) + job `cargo deny check` | `.github/workflows/ci.yml`, `deny.toml` |
| S0.6 | CI macOS | Au minimum `cargo build -p tom-protocol-ffi` sur `macos-latest` (la cible SDK n°1 est Apple et n'est jamais buildée en CI) | `ci.yml` |
| S0.7 | rustfmt.toml | Defaults explicites, pour figer le formatage avant contributions externes | racine |

**Critère de sortie** : `cargo clippy --workspace -- -D warnings` + `cargo test --workspace` verts avec tom-protocol-ffi inclus ; CI augmentée verte ; `git status` propre.

---

## Phase S1 — API SDK Rust (≈ 1 semaine)

But : livrer le crate **`tom-sdk`** (décision D5) — façade fine au-dessus de `tom-protocol` qu'un tiers consomme sans connaître les crates internes. `tom-protocol` reste le moteur ; `tom-sdk` est le contrat public versionné.

### S1.0 — Créer le crate façade `tom-sdk`

- Nouveau membre workspace `crates/tom-sdk` : expose un client haut niveau (connect, send, events, groupes) calqué sur l'usage réel de tom-tui ; re-exporte uniquement les types nécessaires (NodeId, ProtocolEvent filtré, erreurs).
- Ne PAS re-exporter `RuntimeCommand` ni les effects internes.
- Les tâches S1.1-S1.3 ci-dessous s'appliquent à la frontière `tom-sdk` (et par ricochet nettoient tom-protocol).

### S1.1 — Étanchéifier l'API (le chantier le plus important)

Masquer les types forkés qui fuient (`runtime/mod.rs:134, 196, 252, 363, 371`) :

| Type fuyant | Wrapper proposé | Notes |
|---|---|---|
| `tom_connect::EndpointAddr` | `PeerAddr` (newtype) | conversions `From`/`TryFrom` internes |
| `tom_connect::RelayUrl` | `RelayAddr` ou `String` validée | déjà `Display`/`FromStr` côté iroh |
| `tom_transport::PathEvent` | `PathChange` (enum ToM) | ne garder que les variantes utiles au consommateur |

Règle de vérification : `cargo doc -p tom-protocol` ne doit montrer **aucun type d'un crate `tom-*` interne** dans les signatures publiques. Ajouter un test de non-régression si possible (ex. compile-fail ou revue manuelle documentée).

### S1.2 — Erreurs

- Documenter chaque variante de `TomProtocolError` (`error.rs`).
- `#[non_exhaustive]` sur `TomProtocolError`, `ProtocolEvent`, `RuntimeCommand` (liberté d'évolution post-publication).
- Optionnel (post-v0.3) : typer les variantes `String` (`Crypto`, `Serialization`).

### S1.3 — Documentation et exemples

- `#![deny(missing_docs)]` dans `lib.rs` + combler (~20 docstrings).
- Dossier `crates/tom-protocol/examples/` : `01_send_message.rs` (spawn + send + receive), `02_group_chat.rs`, `03_embedded_relay.rs` — calqués sur `tests/runtime_integration.rs` et l'usage de tom-tui.
- Doc-tests `# Examples` sur `ProtocolRuntime::spawn`, `RuntimeHandle::send_message`, `RuntimeConfig`.
- `crates/tom-protocol/README.md` : quickstart 20 lignes.
- Métadonnées Cargo.toml : `license = "MIT"`, `repository`, `keywords`, `authors`.

### S1.4 — Corrections ponctuelles

- Remplacer les `unwrap()` de sérialisation metrics (`runtime/metrics.rs:205, 229`, `runtime/bootstrap.rs:142`) par un fallback loggé.
- Statuer sur les `allow(dead_code)` (`group/manager.rs:37`, `runtime/bootstrap.rs:6,38`) : implémenter ou supprimer.

**Critère de sortie** : un projet externe vide avec `tom-sdk = { git = "...", tag = "v0.3.0" }` compile les 3 exemples sans rien importer d'autre (décision D1 : distribution par tag git). Tag `v0.3.0` posé.

---

## Phase S2 — SDK Apple distribué (≈ 1 semaine, le « quick win » visible)

But : `https://github.com/<org>/tom-sdk-swift` ajoutable dans Xcode en 30 secondes.

| # | Tâche | Détail |
|---|---|---|
| S2.1 | Générer le header avec **cbindgen** | Remplace `include/tom_protocol_ffi.h` manuel ; check CI « header à jour » |
| S2.2 | **Swift Package** `TomProtocolKit` | `Package.swift` avec `binaryTarget` (XCFramework zippé + checksum) + target source contenant les wrappers Swift **dé-dupliqués** (TomNodeWrapper, TomModels, TomError, TomNodeService — actuellement copiés ×2 dans les apps) |
| S2.3 | Release automation | Workflow GitHub : build XCFramework (le script existe : `scripts/build-tom-protocol-ffi-xcframework.sh`) → zip → checksum → GitHub Release sur tag `sdk-swift/vX.Y.Z` |
| S2.4 | Migrer tom-node-ios/tvos vers le package | Les apps deviennent les premiers consommateurs du SDK (dogfooding) ; supprime ~1200 lignes dupliquées et les chemins hardcodés `../tom-node-tvos/build/` |
| S2.5 | Doc d'intégration | README du package : add package → 10 lignes de Swift → message envoyé. Mention claire du modèle polling (et de sa latence 500 ms) |

Améliorations FFI **optionnelles** (différables, à inscrire au backlog) :
- Callback C (ou `tom_node_poll` avec timeout long) pour remplacer le polling 500 ms.
- Compteur de messages droppés quand la queue (1000) déborde, exposé dans `tom_node_status`.

**Critère de sortie** : un dev tiers crée une app iOS vierge, ajoute le package par URL, envoie un message à un nœud ToM. Les apps internes consomment le package.

---

## Phase S3 — Spec protocole publique (≈ 3-5 jours, parallélisable avec S1/S2)

But : honorer « s'approprier le protocole » — un implémenteur tiers (Go, Python…) ne doit pas avoir à lire le source Rust.

| # | Livrable | Contenu |
|---|---|---|
| S3.1 | `docs/spec/tom-wire-v1.md` | Envelope MessagePack champ par champ, `signing_bytes()` (exclusion du `ttl` — ADR-003), types de messages, ACK/read-receipt, TTL 24h, règles de relai stateless |
| S3.2 | `docs/spec/tom-crypto-v1.md` | Ed25519 + X25519 + XChaCha20-Poly1305 + HKDF-SHA256, ordre encrypt-then-sign (ADR-004), dérivation des clés de groupe (sender keys) |
| S3.3 | **Test vectors** | Fichiers JSON/hex générés depuis les tests Rust existants : envelope signée connue, payload chiffré connu, clé→NodeId. C'est ce qui rend la spec vérifiable |
| S3.4 | `docs/spec/tom-discovery-v1.md` (peut glisser en S4) | PeerAnnounce, gossip, rôle des relays, invariants wire iroh (`_iroh`, ALPN, SNI — référencer FORK-GOVERNANCE.md) |

Source de vérité : le code Rust + les ADR de CLAUDE.md + `_bmad-output/planning-artifacts/design-decisions.md` (les 7 décisions verrouillées doivent ouvrir la spec).

**Critère de sortie** : un dev peut valider une implémentation d'envelope contre les test vectors sans exécuter le code Rust.

---

## Phase S4 — Plateformes suivantes (après S1-S3, à séquencer selon la demande)

| Option | Effort estimé | Prérequis | Notes |
|---|---|---|---|
| **Android** (JNI + AAR) | ~2 semaines | S1, S2.1 (cbindgen) | Réutilise le FFI C existant ; vérifier patches netdev/netwatch côté Android |
| **WASM / Web** | ~2-3 semaines | S1 ; étude faisabilité (QUIC/UDP indisponible en browser → nécessite un mode transport relay-only WebSocket/WebTransport) | Remplace le TS legacy à terme |
| **Node.js** (napi-rs) | ~1 semaine | S1 | Cible serveurs/bots ; plus simple que WASM |
| **crates.io** | dépend de dalek | Sortie stable `ed25519-dalek 3.0` / `curve25519-dalek 5.0` | Surveiller les releases ; publier les 8 crates dans l'ordre des dépendances (tom-base → … → tom-protocol) |

### Sort du TypeScript legacy (décision D3 — ✅ tranchée : archiver)
**Archiver** `packages/` + `tools/signaling-server` + `apps/demo` (branche `archive/phase1-typescript` ou dossier `legacy/`), avec un README expliquant la divergence (WebRTC/JSON/XSalsa20 vs QUIC/MessagePack/XChaCha20). Le futur SDK web sera dérivé du cœur Rust (WASM), pas du TS. Exécutable dès S0/S1 (indépendant du chemin critique) ; attention aux jobs CI TypeScript à retirer de `ci.yml` au même moment.

---

## Transverse — à maintenir pendant toutes les phases

- **README racine** : ajouter une section « Integrate ToM » avec les 3 portes d'entrée (Rust crate, Swift Package, spec) dès qu'elles existent.
- **Gate non négociable** (CLAUDE.md) : `cargo clippy --workspace -- -D warnings` + `cargo test --workspace` avant chaque push.
- **Veille forks** : processus léger de surveillance CVE iroh/quinn/rustls (note trimestrielle dans FORK-GOVERNANCE.md).
- **Versioning** : SemVer 0.x assumé ; CHANGELOG.md par release ; tags `vX.Y.Z` (Rust) et `sdk-swift/vX.Y.Z` (Apple).

---

## Récapitulatif effort / valeur

| Phase | Durée estimée | Valeur |
|---|---|---|
| S0 Hygiène | 2-3 j | Base saine, CI fiable |
| S1 API Rust | ~1 sem | SDK Rust consommable par tag git |
| S2 SDK Apple | ~1 sem | **Premier SDK public installable** |
| S3 Spec | 3-5 j (parallèle) | Appropriation du protocole par des tiers |
| S4 Plateformes | à la demande | Android / Web / Node / crates.io |

**Chemin critique vers « un tiers utilise ToM » : S0 → S1.1 → S2 ≈ 2,5 semaines.**

---

## Backlog hors chemin critique (issues à créer, ne pas traiter maintenant)

1. Callback FFI ou long-poll (supprimer la latence 500 ms).
2. Typage fin de `TomProtocolError` (sous-enums Crypto/Serialization).
3. `[features]` Cargo (relay embarqué, DHT optionnels).
4. Builder fluent `RuntimeConfigBuilder`.
5. Compteur de drops queue FFI + exposition dans status.
6. Harness de conformité SDK basé sur tom-stress (valider une implémentation tierce contre un nœud de référence).
7. GOVERNANCE.md + CODE_OF_CONDUCT + process de release documenté.
8. CI : coverage, MSRV check, `cargo test --all-features`, cross-compile iOS/tvOS.
9. Roadmap publique post-R11 (mettre à jour README/docs/plans).
