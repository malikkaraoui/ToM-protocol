# Chantier S2.4 — Migration des apps iOS/tvOS vers TomProtocolKit · Suivi d'exécution

> Démarré : 2026-06-12 14:15 · Référence : journal S2 (`2026-06-11-chantier-s2-suivi.md`, § S2.4 reporté)
> Objectif : les 2 apps consomment le Swift Package `TomProtocolKit` — fin des
> wrappers dupliqués, du bridging header et de l'XCFramework câblé à la main.

## Tableau de bord

| Tâche | Description | Statut |
|---|---|---|
| S2.4.a | Package.swift : linkerSettings (z, resolv, frameworks système) | ✅ |
| S2.4.b | iOS : project.yml → package, purge Models/bridging header, imports | ✅ |
| S2.4.c | tvOS : project.yml créé (sortie du pbxproj manuel), 2 targets migrés | ✅ |
| S2.4.d | Hygiène : dé-tracking .build/ (999 fichiers), purge header copié | ✅ |
| S2.4.e | Docs : SETUP.md iOS, CLAUDE.md tvOS, Makefiles | ✅ |
| S2.4.V | Validation builds (iOS sim, tvOS sim, macOS) + PR | ✅ builds / PR ⏳ |

## Journal de chantier

### Phase 0 — Cartographie

- Doublons à supprimer : `Models/{TomError,TomModels,TomNodeWrapper}.swift` ×2 apps + bridging headers ×2.
- Aucun appel FFI direct hors `Models/` (grep `tom_node_|tom_protocol_`) — la surface à migrer est close.
- Surface API package vs apps : identique modulo `public` + init explicites + casts `UInt` (cbindgen).
  Divergence trouvée : `getDiscoveredRelay()` présent côté iOS, absent côté tvOS — **le package l'a déjà**
  (et le XCFramework des Artifacts contient le symbole, vérifié par `nm` : 34 occurrences).
- XCFramework du package : 5 slices confirmées via Info.plist (ios, ios-sim, tvos, tvos-sim, macos).
- Outillage : xcodegen 2.x + Xcode 26.4.1 — chantier menable de bout en bout en CLI.

### S2.4.a — linkerSettings dans le package

Le pbxproj tvOS portait `-lz -lresolv -framework Network/Security/SystemConfiguration`
(exigences de link de la lib statique Rust). Un binaryTarget SPM ne peut pas déclarer
de linkerSettings → posés sur le target source `TomProtocolKit`. Les apps n'ont plus
aucun `OTHER_LDFLAGS` à connaître.

### S2.4.b — iOS (xcodegen existant)

- `project.yml` : `packages: TomProtocolKit (path ../../sdk/swift/TomProtocolKit)` +
  `dependencies: package`. Retirés : `SWIFT_OBJC_BRIDGING_HEADER`, `HEADER_SEARCH_PATHS`,
  `OTHER_LDFLAGS -ObjC`, framework dep, preBuild script XCFW (remplacé par un check Artifacts).
- `import TomProtocolKit` ajouté aux 3 consommateurs (TomNodeService, GroupsView, MessagesView).
- Build : `iPhone 17` simulateur — **BUILD SUCCEEDED** (2 fois, dont revalidation post-linkerSettings).
- Piège : le simulateur `iPhone 16` du Makefile n'existe plus sous Xcode 26.4 (xcodebuild
  partait chercher l'iPad physique de Laura) — Makefile mis à jour (iPhone 17 / iPad Pro M5).

### S2.4.c — tvOS (sortie du pbxproj manuel)

- `project.yml` créé : 2 targets (TomNode tvOS multi-plateforme + TomNode-macOS, sources
  Swift partagées avec `excludes: Assets.xcassets` + assets macOS dédiés), schemes générés,
  signing préservé (team K22558HU63, style Automatic), entitlements macOS conservés.
- Réglage mort non reporté : `CODE_SIGN_ENTITLEMENTS = TomNode.entitlements` (fichier inexistant).
- Reliquats supprimés : `Services/build_ffi.sh`, `BUILD_FFI_TROUBLESHOOTING.md`, phase
  « Build Rust FFI », XCFramework embarqué.
- **Incident — MEMBER_IMPORT_VISIBILITY** : le pbxproj activait
  `SWIFT_UPCOMING_FEATURE_MEMBER_IMPORT_VISIBILITY` → tout fichier utilisant un *membre*
  ToM (case `.running`, propriété `displayName`) exige l'import explicite. Le grep par noms
  de types avait raté SettingsView & co. Fix : `import TomProtocolKit` dans les 9 fichiers
  consommateurs tvOS (3 + 6).
- Builds : tvOS simulateur (Apple TV 4K 3rd gen) **SUCCEEDED** · macOS arm64 **SUCCEEDED**.

### S2.4.d — Hygiène git

- `apps/tom-node-tvos/.build/` : 999 artefacts xcodebuild encore trackés (la règle
  `**/.build/` du .gitignore n'agit pas sur les fichiers déjà suivis) → `git rm -r --cached`.
- `apps/tom-node-tvos/build/tom_protocol_ffi.h` : copie versionnée du header, plus
  référencée par aucun bridging header (étape 4 du plan) → supprimée. Le dossier `build/`
  reste l'output intermédiaire du script XCFramework (non versionné).

### S2.4.e — Docs et Makefiles

- `SETUP.md` iOS réécrit : xcodegen + package (l'ancien décrivait la création manuelle
  du projet + câblage bridging header en 5 étapes).
- `CLAUDE.md` tvOS réécrit : nouvelle stack, règle MEMBER_IMPORT_VISIBILITY, « Do Not
  Modify » réorienté vers project.yml (plus de pbxproj manuel).
- Makefiles ×2 : `ffi-xcframework` = build + sync package, `doctor` pointe les Artifacts
  du package et le header cbindgen, cibles mortes retirées (`ffi`, `ffi-device`, `macffi`),
  cible `gen` ajoutée côté tvOS.

## Validation

- iOS : xcodebuild iPhone 17 simulateur ✅ (×2)
- tvOS : xcodebuild Apple TV 4K simulateur ✅
- macOS : xcodebuild arm64 ✅
- Gates Rust workspace : aucune crate Rust touchée — gate pre-push standard avant push.

## Review de substitution (Copilot HS jusqu'au 2026-07-01)

Review adversariale par subagent local — 6 findings, traitement :

| # | Sévérité | Finding | Traitement |
|---|---|---|---|
| 1 | CRITIQUE | Views iOS (LogView, SettingsView, StatusView) utilisent des membres ToM sans import — casserait si MEMBER_IMPORT_VISIBILITY était activé côté iOS | ✅ imports ajoutés (ContentView : 0 usage vérifié, pas d'import) |
| 2 | CRITIQUE | Clone frais : package non buildable sans Artifacts/ | Acté sans changement — design assumé depuis S2.2 (artefact local gitignoré, bascule `binaryTarget(url:checksum:)` prévue à la 1re release `sdk-swift/v*`), documenté SETUP.md + DEPLOY-APPLE.md |
| 3 | MOYEN | docs/DEPLOY-APPLE.md décrivait l'ancien monde (bridging header, câblage manuel) | ✅ réécrit : flux package (steps 1-3, architecture, troubleshooting, layout) |
| 4 | MOYEN | scripts/build-tom-protocol-ffi-tvos.sh mort (plus aucune cible ne l'appelle) | ✅ supprimé |
| 5 | MINEUR | BUNDLE_ID_MACOS du Makefile tvOS ≠ project.yml | ✅ aligné (malik.karaoui.TomNode-macOS) |
| 6 | MINEUR | .xcodeproj généré versionné (source de vérité ambiguë) | Réfuté — convention du repo (l'iOS le versionnait déjà), project.yml documenté comme source de vérité dans CLAUDE.md/SETUP.md |

## Notes

- Backlog : test sur device physique (Apple TV / iPhone) avec signing — à faire par Malik
  dans Xcode à l'occasion ; dé-dup éventuelle de TomNodeService/StatusServer (hors scope S2.4).
