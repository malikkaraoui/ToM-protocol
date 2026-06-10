---
name: ios-setup
description: "Configure le workflow iOS/tvOS optimal : VS Code + Claude Code + Xcode minimal + Makefile. Génère la structure, le Makefile V4, les tasks VS Code, le CLAUDE.md projet. Déclenché automatiquement si Xcode ou Swift est mentionné."
figure: Maître d'atelier
triggers: xcode, ios, tvos, swift, swiftui, app store, testflight, simulateur
---

# iOS Setup

> L'atelier s'ouvre sur un nouveau chantier. Pas une app ordinaire —
> une app Apple. Les règles changent. Les outils aussi.
> Le Maître d'atelier sort le plan spécial.

Setup complet du workflow iOS/tvOS dans l'atelier.

## Quand se déclencher

- Mention de Xcode, iOS, tvOS, Swift, SwiftUI, App Store, TestFlight
- Projet mobile en cours de setup
- `/ios-setup` explicite

## Le principe

Xcode est incontournable chez Apple. Mais il ne doit pas être le centre.

```text
80-95% du dev → VS Code + Claude Code
5-20% → Xcode (signing, device, LLDB, archive)
```

Le Makefile est le pont. `make run` remplace "ouvrir Xcode, sélectionner le scheme, cliquer Run".

## Procédure

### Étape 1 — Vérifier les pré-requis

```bash
xcodebuild -version   # Xcode 15+ recommandé
swift --version
gem install xcpretty
brew install jq
```

Si Xcode absent → "Télécharger Xcode depuis l'App Store avant de continuer. C'est le seul prérequis non négociable."

### Étape 2 — Générer la structure projet

```text
MyApp/
├── MyApp.xcodeproj
├── MyApp/
│   ├── App/          (MyAppApp.swift, ContentView.swift)
│   ├── Views/
│   ├── ViewModels/
│   ├── Models/
│   ├── Services/
│   ├── Components/
│   └── Resources/
├── MyAppTests/
├── Packages/CoreKit/
├── Scripts/
├── .vscode/          (tasks.json, settings.json)
├── Makefile
└── CLAUDE.md
```

### Étape 3 — Demander les informations projet

Avant de générer le Makefile, récupérer :

1. Nom du projet (ex: `MyApp`)
2. Bundle ID iOS (ex: `com.example.myapp`)
3. Bundle ID tvOS si applicable
4. Scheme iOS (souvent = nom du projet)
5. Scheme tvOS si applicable
6. Simulateur cible iOS (défaut: `iPhone 15 Pro`)
7. Simulateur cible tvOS (défaut: `Apple TV 4K (3rd generation)`)

### Étape 4 — Générer le Makefile V4

Avec les infos récupérées, générer le Makefile complet incluant :
- `make run` → build + boot simulator + install + launch
- `make tvrun` → idem tvOS
- `make test` → tests unitaires
- `make doctor` → diagnostic setup
- `make devices` → liste devices
- `make device DEVICE_ID=<udid>` → device réel
- `make archive` → archive App Store
- `make clean` → nettoyage
- `make logs` → logs build
- `make help` → aide

### Étape 5 — Générer les tasks VS Code

Fichier `.vscode/tasks.json` avec :
- `⌘⇧B` → Run iOS Simulator
- Run tvOS Simulator
- Run Tests
- Doctor / Clean / List Devices

### Étape 6 — Générer le CLAUDE.md projet iOS

```markdown
## Projet iOS
- Nom : [nom]
- Plateformes : iOS 17+ / tvOS 17+
- Langage : Swift 5.9+
- UI : SwiftUI
- Architecture : MVVM + Services + local Swift Packages
- Scheme iOS : [scheme]  Bundle : [bundle_id]
- Simulateur : iPhone 15 Pro
- Règles : async/await, une Preview par View, logique → CoreKit
- Ne pas modifier : .xcodeproj interne, signing, Info.plist
```

### Étape 7 — Générer le .gitignore iOS

```gitignore
# Xcode
*.xcuserstate
xcuserdata/
DerivedData/
*.xcscmblueprint

# Build
.build/

# macOS
.DS_Store
```

### Étape 8 — Vérification finale

```bash
make doctor
make help
```

Si tout est vert → "L'atelier iOS est prêt. `make run` pour lancer."

## Règles

- Jamais forcer l'ouverture de Xcode si une commande CLI suffit
- Toujours demander bundle ID avant de générer le Makefile (sans lui, `simctl launch` échoue)
- Si Swift Package Manager est utilisé : documenter les packages dans §0
- Tests SwiftUI : XCTest + `.testable import`
- §5 prime : ne pas inventer de flags `xcodebuild` non vérifiés

## Prompts utiles à proposer après setup

```text
# Refactor SwiftUI
Refactor this SwiftUI screen into smaller reusable components.
Keep previews. Preserve current behavior.

# MVVM
Reorganize this feature into MVVM.
Move side effects to Services, keep the View simple.

# Tests
Generate unit tests for this service. Cover success + failure cases.

# Performance
Review this SwiftUI view for unnecessary re-renders.
```
