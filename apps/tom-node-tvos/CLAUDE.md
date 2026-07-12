# TomNode tvOS — Claude Context

## Project
- Name: TomNode
- Platform: tvOS 16.3+ (target multi-plateforme : Apple TV + iPhone/iPad) + target macOS dédié
- Language: Swift 5.9+
- UI: SwiftUI (MVVM)
- Architecture: MVVM + Services + Swift Package TomProtocolKit

## Core Stack (S2.4 — migration package)
- **SDK**: Swift Package local `TomProtocolKit` (`sdk/swift/TomProtocolKit`)
- **FFI Rust**: portée par le package (binaryTarget `TomProtocolFFI.xcframework` + linkerSettings)
- **Projet Xcode**: généré par xcodegen depuis `project.yml` — ne jamais éditer le `.xcodeproj` à la main
- Plus de bridging header, plus de `build/TomProtocolFFI.xcframework` embarqué

## Schemes
- TomNode (tvOS app)
- TomNode-macOS (macOS app, mêmes sources Swift + assets dédiés)

## Simulator
- Apple TV 4K (3rd generation)

## Bundle IDs
- malik.karaoui.TomNode (tvOS)
- malik.karaoui.TomNode-macOS (macOS)

## Dev Team
- `UPES5479T5` (équipe payante, Apple Developer Program — corrigé 2026-07-12,
  `K22558HU63` était l'ancienne Personal Team GRATUITE, plafonnée à 3 apps/7
  jours, cause d'un échec d'install réel constaté ce jour)

## Architecture
```
Swift UI (Views)
    ↓
ViewModels (ObservableObject)
    ↓
TomNodeService (singleton, @MainActor)
    ↓
TomProtocolKit (Swift Package : TomNodeWrapper actor, TomModels, TomError)
    ↓
TomProtocolFFI.xcframework (C ABI, binaryTarget du package)
    ↓
tom-protocol (ProtocolRuntime, Rust)
```

## File Structure
```
project.yml                   — source de vérité du projet (xcodegen)
TomNode/
├── TomNodeApp.swift          — @main entry
├── Views/
│   ├── ContentView.swift     — TabView (4 tabs)
│   ├── StatusView.swift      — Node status + start/stop
│   ├── MessagesView.swift    — 1-1 messages
│   ├── GroupsView.swift      — Group messaging
│   ├── LogView.swift         — Logs
│   └── SettingsView.swift    — Config + identity
├── ViewModels/               — (future extraction from Service)
├── Services/
│   ├── TomNodeService.swift  — Singleton orchestrator
│   └── StatusServer.swift    — HTTP status endpoint
└── Assets.xcassets/
TomNode-macOS/
└── Assets.xcassets/          — assets macOS (AppIcon dédié)
```
Les anciens `Models/` (TomModels, TomError, TomNodeWrapper) vivent dans le
package : `sdk/swift/TomProtocolKit/Sources/TomProtocolKit/`.

## Commands
- `make gen` — régénérer le projet Xcode depuis project.yml
- `make tvsim` — build for simulator
- `make tvrun` — build + install + launch
- `make macbuild` / `make macrun` — target macOS
- `make ffi-xcframework` — rebuild Rust XCFramework + sync vers le package
- `make doctor` — check setup
- `make clean` — clean builds

## Rules
- async/await preferred (no Combine legacy)
- TomNodeWrapper is an actor (thread-safe FFI access) — fourni par TomProtocolKit
- TomNodeService is @MainActor (UI updates on main thread)
- Message polling: 500ms interval via Task
- `SWIFT_UPCOMING_FEATURE_MEMBER_IMPORT_VISIBILITY` est actif : tout fichier
  utilisant des membres ToM (même un case d'enum comme `.running`) doit faire
  `import TomProtocolKit`
- Preview per View with mock data

## Do Not Modify
- `.xcodeproj` à la main → toujours passer par `project.yml` + `make gen`
- Signing settings (DEVELOPMENT_TEAM UPES5479T5, CODE_SIGN_STYLE Automatic dans project.yml)
- Bundle identifiers
- Sources du package TomProtocolKit sans passer par le chantier SDK
