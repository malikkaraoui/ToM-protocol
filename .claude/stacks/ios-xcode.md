---
kind: stack
name: ios-xcode
loads_from: src/fr/CLAUDE.md §10
triggers: xcode, ios, tvos, ipados, swift, swiftui, xcodebuild, simctl, xcrun, testflight, app store
---

# Stack — iOS / tvOS / iPadOS + Xcode

> **Agent : Steve** 🍎
>
> L'atelier ouvre un chantier Apple. Steve entre en scène — il connaît
> le workflow, les pièges Xcode, le FFI, les simulateurs. Il code dans
> VS Code, build depuis le terminal, et n'ouvre Xcode que quand Apple
> l'impose. C'est le workflow V4.
>
> *« Stay hungry, stay foolish — mais build depuis le Makefile. »*

## Philosophie

80–95 % du dev se fait dans **VS Code + Claude Code**.
Xcode intervient uniquement là où Apple reste incontournable.

La cible change (iPhone, iPad, Apple TV) — le workflow reste le même.
Seul le scheme et le simulateur changent.

```text
VS Code (Claude édite Swift / Rust / C)
    ↓
Makefile + scripts  (ou Cmd+Shift+B)
    ↓
xcodebuild + simctl  (build + install + launch)
    ↓
Simulateur (iPhone / iPad / Apple TV)
    ↓
Xcode (seulement : signing, device, LLDB, Instruments, Archive)
```

## Conventions Swift (6.x — avril 2026)

- `async/await` — **Observation** framework pour l'état (remplace Combine pour les cas simples)
- Architecture **MVVM + Services** + local Swift Packages
- Une Preview par View SwiftUI
- Logique métier dans `Packages/CoreKit` (ou équivalent)
- Tests unitaires dans `*Tests/`
- `async let` pour la concurrence, `@MainActor` pour les mutations UI
- `actor` pour les wrappers thread-safe (ex: FFI handles)
- **Swift 6 strict concurrency** par défaut : `Sendable` protocol obligatoire
- **Swift Testing** framework (nouveau) pour les tests neufs ; XCTest pour legacy
- **SwiftData** préféré à Core Data pour les nouveaux projets
- Combine réservé aux cas avancés (publishers custom, chaînes réactives complexes)

## Commandes Makefile standard

Le Makefile est le point d'entrée unique. Adapter les targets à la plateforme :

```bash
# iOS
make run            # build + install + launch simulateur iPhone
make sim            # build iOS simulateur seulement
make device DEVICE_ID=<udid>  # deploy device physique

# tvOS
make tvrun          # build + install + launch simulateur Apple TV
make tvsim          # build tvOS simulateur seulement
make tvdevice DEVICE_ID=<udid>  # deploy Apple TV physique

# Commun
make test           # tests unitaires sur simulateur
make doctor         # diagnostic complet (Xcode, Swift, simulateurs, dépendances)
make devices        # liste devices disponibles
make archive        # archive App Store / TestFlight
make clean          # nettoyage build/
make logs           # tail logs de build
```

### VS Code : `Cmd+Shift+B`

Configurer `.vscode/tasks.json` pour lancer le build sans quitter l'éditeur :

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Build & Run (Simulator)",
      "type": "shell",
      "command": "make run",
      "group": { "kind": "build", "isDefault": true }
    }
  ]
}
```

## FFI natif (Rust / C → Swift)

Quand le projet embarque du code natif (Rust, C, C++) compilé en `.a` :

### Pipeline FFI

```text
Rust/C source → cargo build --target aarch64-apple-{ios,tvos}[-sim]
    ↓
libyourlib.a  (static library)
    ↓
Bridging Header (YourApp-Bridging-Header.h)
    ↓
Swift actor wrapper (async/await safe)
```

### Script build_ffi.sh

Le script de build FFI doit :
- Détecter la plateforme cible via `EFFECTIVE_PLATFORM_NAME` (sim vs device)
- Trouver le workspace Rust automatiquement
- Compiler avec le bon target (`aarch64-apple-ios`, `aarch64-apple-tvos`, etc.)
- Copier le `.a` dans `build/`

Ce script peut être :
- Appelé manuellement via `make ffi`
- Intégré dans Xcode Build Phases (Run Script)

### Targets Rust courants

| Plateforme | Target | Usage |
| --- | --- | --- |
| iOS device | `aarch64-apple-ios` | App Store / TestFlight |
| iOS simulateur | `aarch64-apple-ios-sim` | Dev local |
| tvOS device | `aarch64-apple-tvos` | Apple TV physique |
| tvOS simulateur | `aarch64-apple-tvos-sim` | Dev local |

Installation : `rustup target add <target>`

### Wrapper Swift pour FFI

```swift
actor NativeNode: Sendable {
    private var handle: NativeHandle?

    func start(config: Config) async throws { /* tom_node_start(handle) */ }
    func stop() async throws { /* tom_node_stop(handle) */ }
    // JSON serialization pour les types complexes
}
```

## Quand ouvrir Xcode

| Cas | Outil |
| --- | --- |
| Coder / refactorer / générer | VS Code + Claude Code |
| Build + simulateur | `make run` / `Cmd+Shift+B` |
| Tests unitaires | `make test` |
| Signing / capabilities | **Xcode obligatoire** |
| Device physique | **Xcode obligatoire** |
| LLDB / Instruments / profiling | **Xcode obligatoire** |
| Archive + TestFlight + App Store | **Xcode obligatoire** |
| Build Phases / Run Script config | **Xcode obligatoire** |

## Pré-requis (`make doctor`)

```bash
xcodebuild -version              # Xcode installé
swift --version                  # Swift CLI dispo
xcpretty --version               # gem install xcpretty (optionnel)
claude --version                 # Claude Code CLI
xcrun simctl list devices available  # simulateurs dispo
# Si FFI Rust :
cargo --version                  # Rust installé
rustup target list --installed   # targets Apple installés
```

## Troubleshooting

| Problème | Cause | Fix |
| --- | --- | --- |
| `xcpretty: command not found` | non installé | `gem install xcpretty` |
| app ne se lance pas après build OK | mauvais bundle id | corriger `BUNDLE_ID` dans Makefile |
| simulateur ne boot pas | état incohérent | `xcrun simctl shutdown all` |
| provisioning error | signing Apple | ouvrir Xcode → corriger Team/Signing |
| LSP Swift lent | indexation en cours | attendre ou relancer extension Swift |
| `PhaseScriptExecution failed` | erreur masquée par Xcode | lancer le script manuellement (`./build_ffi.sh`) pour voir l'erreur réelle |
| Rust not found par Xcode | PATH manquant | ajouter `export PATH="$HOME/.cargo/bin:$PATH"` en tête du script |
| target Rust manquant | non installé | `rustup target add aarch64-apple-ios-sim` |
| workspace Cargo introuvable | chemin dur dans le script | vérifier `SEARCH_DIRS` dans `build_ffi.sh` |
| `.a` non généré malgré build OK | `crate-type` manquant | vérifier `[lib] crate-type = ["staticlib"]` dans `Cargo.toml` |
| permission denied sur script | pas exécutable | `chmod +x build_ffi.sh` |

### Debug rapide FFI

```bash
# 1. Lancer le script hors Xcode pour voir l'erreur
./build_ffi.sh

# 2. Tester le build Rust manuellement
cargo build --release --target aarch64-apple-ios-sim

# 3. Vérifier la lib générée
ls -la target/aarch64-apple-ios-sim/release/lib*.a

# 4. Vérifier les symboles exportés
nm target/aarch64-apple-ios-sim/release/lib*.a | grep your_func
```

## Extensions VS Code recommandées

| Extension | ID |
| --- | --- |
| Swift | `sswg.swift-lang` |
| Error Lens | `usernamehakki.error-lens` |
| GitLens | `eamodio.gitlens` |

## Contraintes par plateforme

| Contrainte | iOS | tvOS | Impact |
| --- | --- | --- | --- |
| File system | Sandbox app | Sandbox app | UserDefaults / CoreData |
| Background execution | Limité | Très limité | Background Modes capability |
| Network discovery | Bonjour OK | Bonjour OK | Relay/DHT pour WAN |
| Chip ancien (A8, etc.) | Rare | Apple TV HD | Optimiser, build Release |

## Prompts utiles pour Claude

```text
# Refactor SwiftUI
Refactor this SwiftUI screen into smaller reusable components.
Keep previews. Preserve current behavior.

# MVVM
Reorganize this feature into MVVM.
Move side effects to Services, keep the View simple.

# FFI wrapper
Create an async Swift actor wrapping this C/Rust FFI.
Handle memory (free), errors (JSON), and thread safety.

# Makefile
Generate a Makefile with run/sim/test/doctor/clean targets
for [iOS/tvOS]. Use xcodebuild + simctl.

# Tests
Generate unit tests for this service. Cover success + failure cases.
```

## Bonnes pratiques systématiques — App Store

Ces patterns sont incontournables. Steve les propose systématiquement sur toute app iOS.

### Dark Mode automatique

```swift
// ✅ Correct — semantic colors
Text("Titre").foregroundStyle(.primary)
Color("Surface") // Asset Catalog avec variant Any/Dark

// ❌ Interdit — hardcodé
Text("Titre").foregroundColor(.black)
```

Ne pas forcer `UIUserInterfaceStyle` dans `Info.plist`. Créer chaque couleur custom en Asset Catalog avec appearance Any + Dark. Tester : simulateur → Features → Toggle Appearance.

### Internationalisation (i18n)

Dès le premier écran. Minimum FR + EN.

```text
App/Resources/
├── fr.lproj/Localizable.strings
└── en.lproj/Localizable.strings
```

```swift
Text(String(localized: "onboarding.title"))
```

Règle : aucun texte visible en string littérale directe dans une View.

### Notation App Store

```swift
import StoreKit

func requestReviewIfAppropriate(after actionsCount: Int) {
    guard actionsCount >= 3 else { return }
    guard let scene = UIApplication.shared.connectedScenes
        .first(where: { $0.activationState == .foregroundActive }) as? UIWindowScene
    else { return }
    SKStoreReviewController.requestReview(in: scene)
}
```

Règle : jamais au lancement, jamais après un échec. Apple limite à 3 demandes/an automatiquement.

### Share Sheet

```swift
ShareLink(
    item: URL(string: "https://apps.apple.com/app/idXXXXXXXXX")!,
    subject: Text("Découvre cette app"),
    message: Text("Je pense que ça pourrait t'intéresser !")
) {
    Label("Partager l'app", systemImage: "square.and.arrow.up")
}
```

### Écran Réglages / À propos

Structure minimale obligatoire dans toute app :

```swift
struct SettingsView: View {
    private var appVersion: String {
        let v = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "?"
        let b = Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "?"
        return "v\(v) (\(b))"
    }

    var body: some View {
        List {
            Section(String(localized: "settings.appearance")) {
                Text(String(localized: "settings.appearance.auto"))
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section(String(localized: "settings.language")) {
                Button(String(localized: "settings.language.change")) {
                    UIApplication.shared.open(URL(string: UIApplication.openSettingsURLString)!)
                }
            }
            Section {
                ShareLink(item: URL(string: "https://apps.apple.com/app/idXXXXXXXXX")!) {
                    Label(String(localized: "settings.share"), systemImage: "square.and.arrow.up")
                }
                Link(destination: URL(string: "https://apps.apple.com/app/idXXXXXXXXX?action=write-review")!) {
                    Label(String(localized: "settings.rate"), systemImage: "star")
                }
            }
            Section(String(localized: "settings.about")) {
                LabeledContent(String(localized: "settings.version"), value: appVersion)
                Link(String(localized: "settings.privacy"),
                     destination: URL(string: "https://example.com/privacy")!)
                Link(String(localized: "settings.terms"),
                     destination: URL(string: "https://example.com/terms")!)
            }
        }
        .navigationTitle(String(localized: "settings.title"))
    }
}
```

Remplacer `idXXXXXXXXX` par l'Apple ID réel dès qu'il est connu. Le noter dans `CLAUDE.md` projet.

### Checklist pré-soumission App Store

- [ ] Dark mode testé sur tous les écrans (Features → Toggle Appearance)
- [ ] Toutes les strings visibles dans `Localizable.strings` (FR + EN)
- [ ] `SKStoreReviewController` déclenché après action positive (pas au lancement)
- [ ] `ShareLink` vers l'App Store présent dans `SettingsView`
- [ ] `SettingsView` : version, privacy URL, terms URL, notation, partage, langue
- [ ] `CFBundleShortVersionString` et `CFBundleVersion` incrémentés
- [ ] Screenshots préparés pour toutes les tailles (6.7", 6.5", 5.5", iPad si applicable)
- [ ] Privacy manifest (`PrivacyInfo.xcprivacy`) si APIs sensibles
- [ ] App Review Information remplie (notes + URL démo si compte requis)

## CLAUDE.md recommandé pour un projet Apple

```markdown
## Projet [iOS/tvOS]
- Plateformes : iOS 17+ / tvOS 17+ (adapter)
- Langage : Swift 5.9+
- UI : SwiftUI
- Architecture : MVVM + Services + local Swift Packages
- Schemes : [nom-iOS] / [nom-tvOS]
- Simulateurs : iPhone 15 Pro / Apple TV 4K (3rd gen)
- FFI : [Rust/C] → staticlib → Bridging Header (si applicable)
- Ne pas modifier : structure .xcodeproj, signing, Info.plist
- Build : make run (iOS) / make tvrun (tvOS)
```
