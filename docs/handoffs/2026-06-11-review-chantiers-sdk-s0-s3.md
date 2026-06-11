# Handoff — Review chantiers SDK S0→S3 (hygiène, tom-sdk, TomProtocolKit, specs)

> Date : 2026-06-11
> Type : review
> Priorité : haute (le SDK va être promu publiquement)
> reviewedRange: 2903fcf..c16c6b6

---

## De : Claude (Fable 5)

### Contexte

Quatre chantiers livrés en 48 h pour ouvrir le protocole ToM aux tiers (journaux détaillés dans `docs/plans/2026-06-10-chantier-s0-suivi.md` → `2026-06-11-chantier-s3-suivi.md`) :

1. **S0 Hygiène** : purge artefacts `.build/`, MSRV+lints workspace, cargo-deny en CI (10 advisories RustSec relevées et ignorées avec justification — triage à venir), job CI macOS, rustfmt.toml.
2. **S1 SDK Rust** : nouveau crate `tom-sdk` — façade au-dessus de tom-protocol. TomClientBuilder→TomClient, **fusion des 3 canaux runtime en un seul flux `Event`** (tâche tokio `spawn_event_merger`), connectivité par **tickets opaques** (JSON d'EndpointAddr, jamais exposé), `deny(missing_docs)`, `#[non_exhaustive]`. Test d'intégration : 2 nœuds échangent un message chiffré via tickets. Tag `v0.3.0`.
3. **S2 SDK Apple** : header C **généré par cbindgen** (remplace le manuel ; handle `void*` → struct opaque = `OpaquePointer` Swift, `size_t` → `uintptr_t`/UInt) + check drift CI. **Swift Package `TomProtocolKit`** (wrappers dé-dupliqués des 2 apps, binaryTarget local). Workflow release `sdk-swift/v*`.
4. **S3 Specs** : `docs/spec/tom-wire-v1.md` + `tom-crypto-v1.md` + 7 test vectors auto-vérifiés (`examples/gen_test_vectors.rs`). Constats wire documentés : envelope = array MessagePack positionnel, `Vec<u8>` = array d'entiers (pas `bin`), enums = string de variante.

### Question précise

Review adversariale sur 4 axes — cherche ce qui est **fragile ou manquant**, pas ce qui est joli :

1. **API tom-sdk** : la façade fuit-elle quelque part ? Le merger d'événements peut-il perdre des événements (ordre, backpressure, canal plein à 4096, drop silencieux des variantes non mappées) ? Le pattern ticket (JSON EndpointAddr brut) est-il un risque (déserialisation d'entrée non fiable, versionnage du format) ?
2. **FFI/cbindgen** : le passage `void*`→struct opaque et `size_t`→`uintptr_t` peut-il casser les apps existantes qui buildent encore contre l'ancien header (elles n'ont pas migré — S2.4 reporté) ?
3. **Specs vs implémentation** : les specs affirment que la v1 est figée — vois-tu des champs/encodages qui risquent de bouger (rmp-serde upgrade changerait-il l'encodage array-positionnel ?) ? Le générateur de vectors couvre-t-il les cas limites (via vide, payload vide, ttl=0) ?
4. **CI/sécurité** : les 10 ignores de deny.toml sont-ils correctement bornés ? Le workflow release-sdk-swift a-t-il des failles (supply chain, checksum) ?

### Fichiers à lire

1. `crates/tom-sdk/src/builder.rs` (merger d'événements — le point le plus sensible)
2. `crates/tom-sdk/src/client.rs` (tickets, API publique)
3. `crates/tom-sdk/src/event.rs` (mapping + drops silencieux)
4. `crates/tom-protocol-ffi/cbindgen.toml` + `include/tom_protocol_ffi.h`
5. `sdk/swift/TomProtocolKit/Sources/TomProtocolKit/TomNodeWrapper.swift`
6. `docs/spec/tom-wire-v1.md`
7. `crates/tom-protocol/examples/gen_test_vectors.rs`
8. `deny.toml`
9. `.github/workflows/release-sdk-swift.yml`
10. `docs/plans/2026-06-10-chantier-s0-suivi.md` (§ advisories)

### Contraintes / hors scope

- Ne pas proposer de réécrire ce qui fonctionne.
- Hors scope : migration des apps (S2.4, chantier planifié), spec discovery (S3.4), StatusServer.swift (findings sécurité déjà actés au journal S1), dérive rustfmt (~700 sites, traitement planifié).
- Se concentrer sur ce qui manque ou est fragile avant promotion publique du SDK.

---

## Réponse de : Copilot/GPT

**INSTRUCTION POUR LE LLM QUI REPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans docs/handoffs/2026-06-11-review-chantiers-sdk-s0-s3.md."

---

## Intégration
<!-- Claude remplit après lecture de la réponse -->
