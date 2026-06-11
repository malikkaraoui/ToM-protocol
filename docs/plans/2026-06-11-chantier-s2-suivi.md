# Chantier S2 — SDK Apple distribué (Swift Package) · Suivi d'exécution

> Démarré : 2026-06-11 06:41 · Référence : `2026-06-10-roadmap-sdk.md` (Phase S2) · Précédents : S0 + S1 clôturés
> Décision D2 : Apple d'abord. Objectif : un package ajoutable dans Xcode en 30 secondes.
> Préexistant : tag `v0.3.0` posé (SDK Rust consommable par git). ⚠️ un tag `v1.0.0` hérité de la phase TS existe — la série SDK Apple utilisera `sdk-swift/vX.Y.Z`.

## Tableau de bord

| Tâche | Description | Statut | Commit |
|---|---|---|---|
| S2.1 | Header C généré par cbindgen + check drift CI | ✅ | `02eea51` |
| S2.2 | Swift Package `TomProtocolKit` (wrappers dé-dupliqués + binaryTarget) | ✅ | `3194ed2` |
| S2.3 | Workflow release : XCFramework → zip → checksum → GitHub Release | ✅ | `7a66ea2` |
| S2.4 | Migration des apps iOS/tvOS vers le package | 🔁 REPORTÉ (chantier dédié, voir journal) | — |
| S2.V | Validation (swift build + gates) + clôture | ✅ | (docs) |

## Journal de chantier

### 2026-06-11 06:41 — Ouverture · tag v0.3.0 posé au préalable

### S2.1 ✅ — commit `02eea51`

- `cbindgen.toml` + `scripts/generate-ffi-header.sh [--check]`. Header généré **identique en surface** au manuel (15/15 fonctions, diff vérifié) mais mieux typé : handle = struct opaque (→ `OpaquePointer` Swift au lieu de `void*`).
- Step CI « Check FFI header drift » dans le job rust-ffi (cbindgen via taiki-e/install-action).
- ⚠️ Changement d'ABI de surface : `size_t → uintptr_t` (Swift `Int` → `UInt`) — un cast adapté dans le wrapper du package. **Les apps qui buildent encore contre l'ancien header devront s'aligner à leur migration (S2.4).**

### S2.2 ✅ — commit `3194ed2`

- `sdk/swift/TomProtocolKit` : Package.swift (iOS 16/tvOS 16/macOS 13), sources publiques `TomNodeWrapper` (actor) + `TomModels` + `TomError` — **versions uniques** des fichiers dupliqués byte-à-byte dans les 2 apps.
- `module.modulemap` ajouté à `crates/tom-protocol-ffi/include/` → emballé automatiquement dans chaque slice par `-headers` (script existant inchangé) ; artefact local patché manuellement pour validation immédiate.
- `binaryTarget` local `Artifacts/` (gitignoré) alimenté par `scripts/sync-xcframework-to-package.sh`. Les releases publiées basculeront sur `binaryTarget(url:checksum:)`.
- **`swift build` vert** (0,53 s, slice macOS). Incident mineur : biome scannait `.build/` du package → exclusions ajoutées à biome.json.

### S2.3 ✅ — commit `7a66ea2`

- `.github/workflows/release-sdk-swift.yml` : tag `sdk-swift/v*` → build 5 slices (nightly + rust-src, tvOS tier 3) → `ditto` zip → `swift package compute-checksum` → validation `swift build` contre l'artefact frais → GitHub Release avec binaires. Aucun input non fiable dans les `run:` (revue sécurité hook OK).

### 🔁 S2.4 REPORTÉ — migration des apps (chantier dédié)

Trop risqué en fin de session (surgery pbxproj tvOS à la main + suppression des bridging headers + ajout d'`import TomProtocolKit` dans tous les fichiers consommateurs ×2 apps + validation xcodebuild). Étapes précises pour le chantier suivant :
1. iOS (`project.yml`, xcodegen) : ajouter `packages: { TomProtocolKit: { path: ../../sdk/swift/TomProtocolKit } }`, retirer le framework dep + bridging header, supprimer `Models/TomNodeWrapper.swift`, `TomModels.swift`, `TomError.swift`, ajouter les imports, `xcodegen && xcodebuild -scheme TomNode -destination 'platform=iOS Simulator,…' build`.
2. tvOS (`TomNode.xcodeproj` brut) : idem via Xcode (Add Local Package) — préférer générer un project.yml d'abord pour sortir du pbxproj manuel.
3. Adapter les casts `Int/UInt` (header cbindgen) côté apps si du code FFI direct subsiste (il ne devrait plus).
4. Supprimer la copie `build/tom_protocol_ffi.h` du HEADER_SEARCH_PATHS quand plus aucun bridging header ne la référence.

### S2.V ✅ — clôture 2026-06-11

- `swift build` package : ✅ · `./scripts/generate-ffi-header.sh --check` : ✅ · gates Rust : clippy + tests workspace (hors multi_node, exception documentée S0).
- **Livrable S2 atteint** : un dev tiers peut builder le XCFramework (1 script), l'injecter dans le package (1 script) et ajouter `TomProtocolKit` en local dans Xcode ; la release publique s'automatise au premier tag `sdk-swift/v0.1.0`.

### Backlog généré
1. **Chantier S2.4** (migration apps — étapes ci-dessus).
2. Premier tag `sdk-swift/v0.1.0` pour étrenner le workflow release (décision Malik : quand l'infra NAS sera revenue pour un test E2E complet).
3. Publier un repo/manifest SPM consommable par URL (Package.swift avec url+checksum) après la première release.
4. Durcissement StatusServer (findings sécurité, cf. journal S1) avant tout build de distribution.
