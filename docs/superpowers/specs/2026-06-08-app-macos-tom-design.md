# Cadrage — App macOS native ToM (portage 1:1 de l'app tvOS/iOS)

> Date : 2026-06-08 · Auteur : session ToM · Statut : **validé, prêt à coder (nouvelle fenêtre)**
> Décision d'approche : **option 1 — cible macOS native dans le projet Xcode existant** (validée par Malik)
> Ambition : **portage 1:1** des 6 vues SwiftUI existantes en fenêtre Mac native (MacBook Pro M1).

## 1. Objectif

Produire une **vraie app macOS native** (pas un nœud headless) sur MacBook Pro M1, équivalente à
l'app tvOS/iOS de `apps/tom-node-tvos/`. Même UI (6 vues), même moteur (FFI Rust), fenêtre Mac
native redimensionnable, **anti-veille activé** (le nœud doit rester joignable quand le Mac est inactif).

Critère de succès : `make macrun` build + lance l'app ; StatusView montre la connexion au nœud NAS
(node_id `11d5bb113bb0b6c24819a7b2ad4396d533d2e5b8c23b622b51eeaa83b3d2d759`) et l'échange de messages.

## 2. État existant (vérifié)

| Élément | État |
| --- | --- |
| Projet Xcode | `apps/tom-node-tvos/TomNode.xcodeproj` — cibles `appletvos` + `iphoneos` uniquement |
| Vues SwiftUI | `ContentView`, `StatusView`, `MessagesView`, `GroupsView`, `LogView`, `SettingsView` — **SwiftUI pur, aucune n'importe UIKit** |
| Entrée `@main` | `TomNodeApp.swift` — utilise déjà `WindowGroup` → **macOS-compatible tel quel** ✅ |
| FFI Swift↔Rust | `TomNodeWrapper.swift` (actor, mappe toute la surface C) + `TomNodeService.swift` (poll 250 ms, auto-echo, anti-veille, export UDP) |
| UIKit | **Confiné à un seul fichier** : `TomNodeService.swift` — `import UIKit` (l.3), anti-veille `UIApplication.shared.isIdleTimerDisabled` (l.184/221), blocs `#if os(iOS)` (l.62/71) |
| xcframework FFI | `apps/tom-node-tvos/build/TomProtocolFFI.xcframework` — slices `ios-arm64`, `ios-arm64-simulator`, `tvos-arm64`, `tvos-arm64-simulator` — **PAS de slice macOS** |
| Script build FFI | `scripts/build-tom-protocol-ffi-xcframework.sh` — bâtit les 4 slices via `cargo +nightly build --target … --manifest-path crates/tom-protocol-ffi/Cargo.toml`, assemble avec `xcodebuild -create-xcframework` |
| Crate FFI | `crates/tom-protocol-ffi/` — staticlib `libtom_protocol_ffi.a`, **exclu du workspace Cargo** (build via `--manifest-path`), header `crates/tom-protocol-ffi/include/tom_protocol_ffi.h` |

## 3. Architecture cible

Ajouter une **3ᵉ cible** `TomNode-macOS` au `.xcodeproj` existant. Les fichiers source sont **partagés
par target-membership** (aucune copie) :

```
TomNode.xcodeproj
├── TomNode            (cible tvOS, existante)
├── TomNode-iOS        (cible iOS, existante — via SUPPORTED_PLATFORMS)
└── TomNode-macOS      (NOUVELLE — SDKROOT=macosx, deployment 14.0)
        └── membership : TomNodeApp, ContentView, 6 Views, Models/*, Services/*, TomNodeWrapper
```

Data flow inchangé : `TomNodeService` (poll 250 ms) ↔ `TomNodeWrapper` (actor FFI) ↔ `libtom_protocol_ffi` (slice `aarch64-apple-darwin`).

## 4. Lot de travail (chantier)

### Lot A — Slice FFI macOS (Rust, le plus simple)

Étendre `scripts/build-tom-protocol-ffi-xcframework.sh` :

1. Ajouter `aarch64-apple-darwin` à la liste `TARGETS` (les deux branches : `DEVICE_ONLY` et complète).
2. Builder la slice avec deployment macOS :
   ```bash
   MAC_A=$(build_slice aarch64-apple-darwin MACOSX_DEPLOYMENT_TARGET "${MACOS_DEPLOYMENT:-14.0}")
   ```
   La cible `aarch64-apple-darwin` compile **nativement sur M1** — aucun cross-toolchain, juste
   `rustup target add aarch64-apple-darwin --toolchain nightly-aarch64-apple-darwin`.
3. Ajouter `-library "${MAC_A}" -headers "${HEADER_DIR}"` à l'appel `xcodebuild -create-xcframework`.
4. Variable `MACOS_DEPLOYMENT` en tête de script (défaut `14.0`), miroir de `TV_DEPLOYMENT`/`IOS_DEPLOYMENT`.

Résultat attendu : `ls TomProtocolFFI.xcframework/` inclut désormais `macos-arm64`.

> ⚠️ Vérifier que `crates/tom-protocol-ffi` compile pour `aarch64-apple-darwin` (dépendances réseau
> tom-connect/quinn : OK sur macOS natif, c'est la plateforme de dev). En cas d'échec de link sur
> symboles système, c'est un souci de feature crate, pas de la slice.

### Lot B — Cible Xcode macOS

> ⚠️ **Apple impose la création de cible via Xcode** (le `.pbxproj` à la main est fragile et risqué).
> Ouvrir Xcode **une fois** : File → New → Target → **macOS App** → nom `TomNode-macOS`,
> interface SwiftUI, langage Swift. Puis tout le reste (build) se fait en terminal (workflow V4).

Config de la cible :
- `SDKROOT = macosx`, `MACOSX_DEPLOYMENT_TARGET = 14.0`
- Bundle id : suffixe `.macos` (ex. `com.atelier.tomnode.macos`)
- **Supprimer** le `ContentView.swift`/`App.swift` générés par le template ; ajouter à la place la
  **membership macOS** sur les fichiers partagés existants (TomNodeApp, ContentView, 6 Views,
  Models, Services, TomNodeWrapper).
- Lier `TomProtocolFFI.xcframework` (General → Frameworks) ; `HEADER_SEARCH_PATHS` →
  `$(SRCROOT)/build` (où le header est copié) ; `LIBRARY_SEARCH_PATHS` si besoin.
- Si le projet a un fichier de réglages partagé (xcconfig), répliquer la config tvOS/iOS.

### Lot C — Portabilité `TomNodeService.swift`

Seul fichier à toucher. Trois éditions ciblées :

1. **Import UIKit conditionnel** (l.3) :
   ```swift
   #if canImport(UIKit)
   import UIKit
   #endif
   ```
2. **Anti-veille cross-plateforme** (remplace l.184/221). iOS garde `isIdleTimerDisabled` ;
   macOS utilise `ProcessInfo.beginActivity` qui retourne un token à conserver tant que le nœud tourne :
   ```swift
   #if os(macOS)
   private var sleepAssertion: NSObjectProtocol?
   #endif

   // au démarrage du nœud (équivalent l.184) :
   #if os(iOS)
   UIApplication.shared.isIdleTimerDisabled = true
   #elseif os(macOS)
   sleepAssertion = ProcessInfo.processInfo.beginActivity(
       options: [.idleSystemSleepDisabled, .suddenTerminationDisabled],
       reason: "ToM node must stay reachable")
   #endif

   // à l'arrêt du nœud (équivalent l.221) :
   #if os(iOS)
   UIApplication.shared.isIdleTimerDisabled = false
   #elseif os(macOS)
   if let a = sleepAssertion { ProcessInfo.processInfo.endActivity(a); sleepAssertion = nil }
   #endif
   ```
   > `.idleSystemSleepDisabled` empêche la veille système quand l'app tourne (équivalent macOS de
   > l'anti-veille iOS). C'est l'API recommandée (pas `IOPMAssertion` bas niveau).
3. **Auditer les blocs `#if os(iOS)`** (l.62/71) : décider si la branche s'applique aussi à macOS
   (souvent `#if os(iOS) || os(macOS)`) ou reste iOS-only. À trancher à la lecture du contexte exact.

### Lot D — Entitlements / App Sandbox

Le nœud ouvre des sockets QUIC **entrants et sortants**. Sans entitlements réseau, le sandbox bloque.
Fichier `TomNode-macOS.entitlements` :
- `com.apple.security.app-sandbox` = `true`
- `com.apple.security.network.client` = `true`
- `com.apple.security.network.server` = `true` (écoute entrante / hole-punch)

> Alternative pragmatique pour débogage : désactiver App Sandbox au début, le réactiver une fois la
> connexion validée. Mais viser sandbox activé pour une vraie app.

### Lot E — Tooling Makefile (workflow V4)

Ajouter à `apps/tom-node-tvos/Makefile` (miroir des cibles `tv*`) :
- `macffi:` → `cd ../.. && ./scripts/build-tom-protocol-ffi-xcframework.sh` (ou variante macOS-only)
- `macrun:` → `xcodebuild -scheme TomNode-macOS -destination 'platform=macOS' build` puis `open` du `.app` produit (chemin DerivedData ou `-derivedDataPath build/`)
- `macdoctor:` (optionnel) → préflight (toolchain, target rustup, slice présente)

## 5. Anti-veille (exigence explicite Malik)

Implémentation = Lot C.2 ci-dessus : `ProcessInfo.processInfo.beginActivity(options: [.idleSystemSleepDisabled, .suddenTerminationDisabled], reason:)`, token conservé dans `TomNodeService`, libéré à l'arrêt du nœud. Empêche la mise en veille macOS tant que le nœud est actif — garantit que le Mac reste un pair joignable.

## 6. Tests / validation

- **Filet Rust** : contrats FFI serde déjà testés côté crate (aucun nouveau test Rust requis pour ce lot).
- **macOS** : pas de cible XCTest (cohérent avec l'absence côté tvOS). Validation manuelle :
  1. `make macffi` → vérifier slice `macos-arm64` dans le xcframework.
  2. `make macrun` → l'app se lance en fenêtre native.
  3. StatusView affiche `phase: connected` + le NAS (`11d5bb11…`) dans les pairs.
  4. Envoyer un message depuis MessagesView → ACK.
  5. Mettre le Mac en idle quelques minutes → le nœud reste connecté (preuve anti-veille).

## 7. Risques & inconnues

| Risque | Mitigation |
| --- | --- |
| Création de cible `.pbxproj` à la main fragile | Passer par Xcode **une fois** (Lot B), build ensuite en terminal |
| Slice `aarch64-apple-darwin` échoue au build crate | Native sur M1 = cas le plus favorable ; si échec, isoler la feature crate fautive |
| Blocs `#if os(iOS)` (l.62/71) au comportement macOS ambigu | Lire le contexte exact avant de décider iOS-only vs macOS |
| StatusView (399L) layout télécommande/iOS | La branche iOS sert de base ; ajustements fenêtre mineurs |
| Sandbox bloque QUIC | Entitlements `network.client` + `network.server` (Lot D) |

## 8. Hors scope (YAGNI)

Écartés (= option 2 « UX desktop » non retenue) : menu bar, multi-fenêtre, notifications natives,
raccourcis clavier, icône barre d'état, nouveau projet séparé. **Strict portage 1:1 + anti-veille.**

## 9. Ordre d'exécution recommandé

1. **Lot A** (slice FFI macOS) — indépendant, validable seul (`ls` du xcframework).
2. **Lot B** (cible Xcode) — nécessite Xcode une fois.
3. **Lot C** (portabilité TomNodeService) — débloque la compilation macOS.
4. **Lot D** (entitlements) — débloque le réseau.
5. **Lot E** (Makefile) — confort, peut se faire en parallèle de B.
6. Validation §6.

---

*Contexte porté depuis la session du 2026-06-08 (même session que la feature `--bind-port`). Le nœud
NAS de test tourne en `tom-node.service` (node_id stable `11d5bb11…`, port UDP fixe 43925).*
