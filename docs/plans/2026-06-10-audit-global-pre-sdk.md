# Audit global pré-SDK — ToM Protocol

> Date : 2026-06-10 · Périmètre : intégralité du repo (crates Rust, FFI, apps natives, TS legacy, docs, CI)
> Méthode : 4 audits parallèles en lecture seule. Tous les constats sont sourcés (fichier:ligne). Aucune modification de code.
> Objectif : poser une base saine avant les étapes à venir, en particulier la sortie d'un SDK public.

---

## Synthèse exécutive

Le cœur du projet est **sain et mature** : le crate `tom-protocol` (v0.2.0) a une API channel-based cohérente, ~90 tests d'intégration, zéro TODO/FIXME, des forks iroh bien gouvernés (`docs/FORK-GOVERNANCE.md`), et une couche FFI fonctionnelle déjà consommée par 3 apps Apple (iOS/tvOS/macOS).

En revanche, **rien n'est aujourd'hui consommable par un tiers** :

| Canal | État |
|---|---|
| crates.io (Rust) | ❌ Bloqué — path dependencies + pins pré-release dalek |
| Swift Package (Apple) | ❌ Inexistant — XCFramework buildé localement, jamais versionné/distribué |
| npm (TS) | ❌ `tom-sdk` est `private: true`, et divergent du protocole Rust |
| Android | ❌ Aucun support FFI |
| Spec protocole (implémenteur tiers) | ❌ Pas de spec wire format indépendante du code |

**Conclusion** : la voie la plus rapide vers un SDK n'est PAS crates.io (bloqué structurellement par les pins `ed25519-dalek =3.0.0-pre.1`), mais un **Swift Package binaire versionné** (l'infra FFI existe à 80 %) + une **distribution Rust par tag git**. Détail dans la roadmap jointe (`2026-06-10-roadmap-sdk.md`).

---

## 1. API publique `tom-protocol` (cœur du futur SDK Rust)

### Forces
- Surface minimale et cohérente : ~50 items publics (`lib.rs:23-50`), pattern `ProtocolRuntime::spawn() → RuntimeChannels {handle, messages, events}`.
- `RuntimeConfig`, `RuntimeCommand`, `ProtocolEvent` très bien documentés (rustdoc dense, `runtime/mod.rs`).
- `tom-tui` est un excellent consommateur de référence (usage propre, channel-based).
- Tests d'intégration solides (`tests/runtime_integration.rs`), zéro TODO/FIXME dans le code.

### Lacunes
| # | Constat | Source | Gravité |
|---|---|---|---|
| 1 | **Types forkés fuient dans l'API publique** : `EndpointAddr`, `RelayUrl` (tom-connect), `PathEvent` (tom-transport) exposés dans `RuntimeCommand`/`ProtocolEvent` | `runtime/mod.rs:134, 196, 252, 363, 371` | 🔴 Bloquant SDK |
| 2 | `TomProtocolError` quasi pas documentée, variantes à `String` génériques (`Crypto(String)`, `Serialization(String)`) | `error.rs:1-31` | 🟠 |
| 3 | Pas de `#![deny(missing_docs)]`, pas de doc-tests, pas de dossier `examples/`, pas de README de crate | `lib.rs` | 🟠 |
| 4 | Métadonnées Cargo.toml incomplètes : pas de `license`, `repository`, `keywords`, `authors` | `crates/tom-protocol/Cargo.toml:1-5` | 🟠 |
| 5 | Pas de `[features]` — API monolithique (relay embarqué, DHT, etc. toujours compilés) | Cargo.toml | 🟡 |
| 6 | Pas d'enum `#[non_exhaustive]` sur les types publics → tout ajout de variante est un breaking change | `error.rs`, `runtime/mod.rs` | 🟡 |
| 7 | `unwrap()` sur sérialisation metrics (panic théorique) | `runtime/metrics.rs:205, 229` | 🟡 |

---

## 2. Couche FFI et apps natives

### Forces
- `tom-protocol-ffi` : 15 fonctions C ABI propres (`crates/tom-protocol-ffi/src/lib.rs:121-953`), modèle erreur `i32 + tom_node_last_error()`, strings null-safe, mutex poison-safe.
- Script XCFramework complet : iOS + tvOS + macOS (device/sim, lipo universal) — `scripts/build-tom-protocol-ffi-xcframework.sh`.
- Wrapper Swift de qualité : `actor TomNodeWrapper` (281 lignes), `TomError` typé, `@MainActor TomNodeService`.

### Lacunes
| # | Constat | Source | Gravité |
|---|---|---|---|
| 1 | **Aucune distribution** : pas de `Package.swift`, XCFramework jamais zippé/release, chemins hardcodés inter-apps (`../tom-node-tvos/build/...`) | `apps/tom-node-ios/project.yml` | 🔴 Bloquant SDK |
| 2 | **Code Swift dupliqué au byte près** entre tom-node-ios et tom-node-tvos (TomNodeWrapper, TomModels, TomError, TomNodeService = ~1200 lignes ×2) | `apps/*/TomNode/Models/` | 🟠 |
| 3 | Header C `tom_protocol_ffi.h` **écrit à la main** — risque de dérive vs l'impl Rust (pas de cbindgen) | `crates/tom-protocol-ffi/include/` | 🟠 |
| 4 | Architecture **polling 500 ms** (pas de callback) → latence perçue + boucle Swift de 753 lignes | `TomNodeService.swift:432-590` | 🟡 |
| 5 | `tom-protocol-ffi` **exclu du workspace** (raison historique « ne compile pas » — obsolète, il compile) → pas couvert par CI ni clippy workspace | `Cargo.toml:3` | 🟠 |
| 6 | Artefacts `.build/` tvOS commités par erreur (21 fichiers, en cours de nettoyage), `.gitignore` ignore `build/ios/` mais pas `.build/` | `apps/tom-node-tvos/.gitignore:1` | 🟡 |
| 7 | `block_on()` ×11 dans le FFI → appels séquentiels, pas de parallélisme côté caller | `lib.rs:254, 389, …` | 🟡 |
| 8 | Queue messages bornée à 1000 → perte silencieuse possible en burst | `lib.rs:300-302` | 🟡 |
| 9 | Android : zéro support (pas de JNI, pas d'AAR) | — | 🟡 (selon cible) |

---

## 3. Workspace, dépendances, CI

### Forces
- Gouvernance des forks exemplaire (`docs/FORK-GOVERNANCE.md`, invariants wire documentés, historique tenu).
- `unsafe` minimal et localisé (~150 LOC, quasi exclusivement FFI + sockets UDP — légitime).
- CI couvre build/test/clippy par groupe de crates + smoke tests relay/stress.
- CHANGELOG.md, CONTRIBUTING.md (modèle micro-sessions), LICENSE MIT présents.

### Lacunes
| # | Constat | Source | Gravité |
|---|---|---|---|
| 1 | **Pins pré-release** : `curve25519-dalek =5.0.0-pre.1`, `ed25519-dalek =3.0.0-pre.1` → publication crates.io impossible tant que dalek stable ne sort pas (pin imposé par compat quinn, cf. CLAUDE.md) | `tom-base/Cargo.toml:34-35`, `tom-connect:49`, `tom-gossip:39` | 🔴 Bloquant crates.io |
| 2 | Pas de `cargo audit` ni `cargo deny` en CI → aucun scan CVE alors que le projet embarque un fork QUIC de 41K LOC figé sur iroh 0.96 | `.github/workflows/ci.yml` | 🟠 Sécurité |
| 3 | Pas de `[workspace.lints]` ni `rustfmt.toml` → lints incohérents entre crates | `Cargo.toml` racine | 🟡 |
| 4 | MSRV déclarée sur les forks (1.88/1.89) mais **absente des crates originaux** (tom-protocol, tom-transport, tom-dht, tom-stress, tom-tui) | Cargo.toml des crates | 🟡 |
| 5 | CI ubuntu-only — aucun build macOS/iOS alors que la cible SDK n°1 est Apple | `ci.yml` | 🟠 |
| 6 | `release.yml` = release-please Node.js uniquement, aucune automation release Rust/XCFramework | `.github/workflows/release.yml` | 🟠 |
| 7 | `Cargo.lock` commités pour crates exclus — ⚠️ CORRIGÉ en S0.2 : seuls les locks `patches/` étaient à supprimer ; ceux de tom-protocol-ffi et iroh-poc sont versionnés à raison (sans lock, les pins dalek pre-release dérivent → E0432 constaté) | — | ~~🟡~~ traité |
| 8 | Éditions hétérogènes : forks en 2024, crates originaux en 2021 | Cargo.toml | 🟡 |
| 9 | Fork iroh 0.96 : pas de processus de veille CVE upstream (QUIC/rustls) | FORK-GOVERNANCE.md | 🟠 |

---

## 4. Vision, documentation, TypeScript legacy

### Cible documentée (PRD)
Le PRD (`_bmad-output/planning-artifacts/prd.md`) définit 3 cibles : **développeurs intégrant ToM via SDK**, contributeurs micro-sessions, et **LLMs comme canal de distribution** (llms.txt, MCP server, plugin VS Code). La promesse explicite : *intégration en 2 lignes de SDK*. → **La demande SDK est donc déjà au cœur de la vision produit ; rien n'a encore été livré sur ce front.**

### Lacunes
| # | Constat | Source | Gravité |
|---|---|---|---|
| 1 | **Pas de plan SDK formalisé** nulle part (ni docs/plans/, ni _bmad-output) | — | 🔴 |
| 2 | **Pas de spec wire format indépendante du code** (type RFC) — un implémenteur tiers doit lire le source ; pas de test vectors | — | 🔴 pour adoption protocole |
| 3 | **TS legacy divergent du Rust** : WebRTC vs QUIC, JSON vs MessagePack, XSalsa20 vs XChaCha20+HKDF, pas de DHT. `tom-sdk` npm `private: true`, jamais publié | `packages/sdk/src/tom-client.ts` | 🟠 Décision à prendre |
| 4 | README : bon sur la vision et les résultats NAT, mais **aucun quickstart d'intégration** (ni Rust ni TS), pas d'instructions d'installation | `README.md` | 🟠 |
| 5 | `tools/signaling-server` confirmé déprécié (ADR-002) mais toujours présent, apps/demo en dépend encore | `tools/signaling-server/` | 🟡 |
| 6 | Pas de GOVERNANCE.md, CODE_OF_CONDUCT, process de release documenté | — | 🟡 |
| 7 | Roadmap publique s'arrête à R11 — pas de phases documentées au-delà | docs/plans/ | 🟡 |

---

## 5. Classement consolidé des risques

### 🔴 Bloquants pour un SDK
1. Types forkés (`EndpointAddr`, `RelayUrl`, `PathEvent`) dans l'API publique de tom-protocol.
2. Aucune distribution : ni Swift Package, ni release XCFramework, ni publication Rust possible (pins dalek pre-release).
3. Pas de spec protocole ni de plan SDK écrit.

### 🟠 Importants (à traiter pendant la phase SDK)
4. `tom-protocol-ffi` hors workspace → hors CI/clippy.
5. Header C manuel (dérive), duplication Swift ×2 apps.
6. Pas de cargo-audit/deny, CI ubuntu-only, pas de release automation.
7. Documentation d'erreurs et d'API incomplète (TomProtocolError, examples/).
8. Sort du TS legacy non tranché (archive vs base SDK web).

### 🟡 Hygiène (rapides, faible risque)
9. `.build/` tvOS commité, Cargo.lock des crates exclus, .gitignore incomplet.
10. MSRV manquantes, workspace.lints, rustfmt.toml, éditions hétérogènes.
11. unwrap() metrics, queue FFI bornée silencieuse, polling 500 ms.

---

## 6. Décisions — TRANCHÉES le 2026-06-10 (Malik)

| # | Décision | Choix validé |
|---|---|---|
| D1 | Canal de distribution Rust | ✅ **Tag git maintenant** ; crates.io quand ed25519-dalek 3.0 stable sortira |
| D2 | Première plateforme SDK | ✅ **Apple (Swift Package / SPM)** — l'infra FFI existe à 80 % |
| D3 | Sort du TS legacy (`packages/`) | ✅ **Archiver** (branche/dossier legacy + README divergence) ; SDK web futur = WASM sur le cœur Rust |
| D4 | Spec protocole publique | ✅ **Oui, dès S1** — RFC-style + test vectors, parallélisable |
| D5 | Nom et périmètre du SDK | ✅ **Façade `tom-sdk` fine** au-dessus de tom-protocol (contrat public stable, moteur libre d'évoluer) |

---

*Audit réalisé en lecture seule. Roadmap d'exécution : `docs/plans/2026-06-10-roadmap-sdk.md`.*
