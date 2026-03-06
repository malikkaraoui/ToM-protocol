# iOS/tvOS Perfect Dev Setup — V4

## VS Code + Claude + Xcode minimal + `make run`

> Objectif : coder dans **VS Code avec Claude**, lancer l’app rapidement en **simulateur iOS/tvOS**, garder **Xcode** pour le **debug avancé**, le **signing**, les **tests sur device** et la **soumission App Store / TestFlight**.

---

# 1. Positionnement du workflow

Ce setup vise à faire :

- **80 à 95 % du développement hors Xcode**
- **build et lancement depuis VS Code / terminal**
- **Xcode uniquement quand Apple l’impose ou quand LLDB / Instruments est nécessaire**

Pipeline cible :

```text
Claude / Continue / Claude Code
        ↓
VS Code
        ↓
Makefile / scripts
        ↓
xcodebuild + simctl
        ↓
Simulator / Apple TV Simulator
        ↓
Xcode (debug device, archive, upload)
```

---

# 2. Pré-requis

## 2.1 Outils Apple

```bash
xcode-select --install
xcodebuild -version
swift --version
xcrun simctl list devices
```

## 2.2 Node + Claude Code

```bash
node -v   # recommandé : >= 18
npm install -g @anthropic-ai/claude-code
claude --version
```

## 2.3 Outils de confort

```bash
gem install xcpretty
brew install jq
```

`jq` est utile pour parser la sortie JSON de `simctl`.

---

# 3. Extensions VS Code recommandées

| Extension | ID | Usage |
|---|---|---|
| Swift | `sswg.swift-lang` | support Swift |
| GitLens | `eamodio.gitlens` | historique Git |
| Continue | `continue.continue` | interface IA dans l’éditeur |
| Error Lens | `usernamehakki.error-lens` | erreurs inline |
| Swift Macro | `onevcat.swiftmacro` | macros Swift |
| Markdown All in One | `yzhang.markdown-all-in-one` | doc projet |

---

# 4. Arborescence recommandée

```text
MyApp/
├── MyApp.xcodeproj
├── MyApp/
│   ├── App/
│   │   ├── MyAppApp.swift
│   │   └── ContentView.swift
│   ├── Views/
│   ├── ViewModels/
│   ├── Models/
│   ├── Services/
│   ├── Components/
│   └── Resources/
├── MyAppTests/
├── Packages/
│   └── CoreKit/
├── Scripts/
│   ├── build_ios_sim.sh
│   ├── build_tvos_sim.sh
│   ├── run_ios_sim.sh
│   ├── run_tvos_sim.sh
│   └── doctor.sh
├── .vscode/
│   ├── tasks.json
│   └── settings.json
├── Makefile
├── CLAUDE.md
├── .gitignore
└── README.md
```

---

# 5. `CLAUDE.md` recommandé

Créer un fichier `CLAUDE.md` à la racine :

```markdown
# CLAUDE.md

## Projet
- Nom : MyApp
- Plateformes : iOS 17+ / tvOS 17+
- Langage : Swift 5.9+
- UI : SwiftUI
- Architecture : MVVM + Services + local Swift Packages

## Schemes
- iOS : MyApp
- tvOS : MyTVApp

## Simulateurs
- iPhone 15 Pro
- Apple TV 4K (3rd generation)

## Règles
- async/await prioritaire
- pas de Combine legacy sauf besoin explicite
- une Preview par View
- logique métier dans Packages/CoreKit si possible
- tests unitaires dans MyAppTests/

## Commandes standard
- make sim
- make tvsim
- make run
- make tvrun
- make test
- make clean
- make devices

## Ne pas modifier sans demande explicite
- structure interne du .xcodeproj
- signing settings
- Info.plist
```

Ce fichier améliore fortement la qualité des réponses de Claude Code sur un repo Swift.

---

# 6. Makefile V4 complet

Ce Makefile ajoute :

- `make run` : build + install + launch iOS simulator
- `make tvrun` : build + install + launch tvOS simulator
- `make doctor` : diagnostic rapide
- `make logs` : lecture des logs de build
- sélection centralisée des simulateurs

Créer `Makefile` :

```makefile
# ─────────────────────────────────────────────
# Configuration
# ─────────────────────────────────────────────
PROJECT         = MyApp.xcodeproj
SCHEME_IOS      = MyApp
SCHEME_TVOS     = MyTVApp
BUNDLE_ID_IOS   = com.example.MyApp
BUNDLE_ID_TVOS  = com.example.MyTVApp

IOS_SIM_NAME    = iPhone 15 Pro
TVOS_SIM_NAME   = Apple TV 4K (3rd generation)

IOS_DEST        = platform=iOS Simulator,name=$(IOS_SIM_NAME)
TVOS_DEST       = platform=tvOS Simulator,name=$(TVOS_SIM_NAME)

BUILD_DIR       = .build/xcode
LOG_DIR         = .build/logs
ARCHIVE_DIR     = .build/archive

APP_IOS         = $(shell find $(BUILD_DIR) -name "$(SCHEME_IOS).app" 2>/dev/null | head -1)
APP_TVOS        = $(shell find $(BUILD_DIR) -name "$(SCHEME_TVOS).app" 2>/dev/null | head -1)

# ─────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────
.PHONY: help
help:
	@echo "Commandes disponibles :"
	@echo "  make sim        → build iOS simulator"
	@echo "  make run        → build + install + launch iOS simulator"
	@echo "  make tvsim      → build tvOS simulator"
	@echo "  make tvrun      → build + install + launch tvOS simulator"
	@echo "  make test       → tests unitaires"
	@echo "  make devices    → liste devices"
	@echo "  make doctor     → diagnostic setup"
	@echo "  make logs       → lire les logs de build"
	@echo "  make clean      → nettoyage"
	@echo "  make archive    → archive App Store"
	@echo "  make device DEVICE_ID=<udid> → build device"

# ─────────────────────────────────────────────
# Build iOS Simulator
# ─────────────────────────────────────────────
.PHONY: sim
sim:
	@mkdir -p $(LOG_DIR)
	@echo "▶ Build iOS Simulator..."
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME_IOS) \
		-destination '$(IOS_DEST)' \
		-configuration Debug \
		-derivedDataPath $(BUILD_DIR) \
		CODE_SIGNING_ALLOWED=NO \
		| tee $(LOG_DIR)/ios_build.log \
		| xcpretty --color
	@echo "✅ Build iOS terminé"

# ─────────────────────────────────────────────
# Run iOS Simulator
# ─────────────────────────────────────────────
.PHONY: run
run: sim
	@echo "▶ Boot simulator $(IOS_SIM_NAME)..."
	@open -a Simulator
	@xcrun simctl boot "$(IOS_SIM_NAME)" 2>/dev/null || true
	@sleep 2
	@APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME_IOS).app" | head -1); \
	if [ -z "$$APP_PATH" ]; then echo "❌ App iOS introuvable"; exit 1; fi; \
	echo "▶ Install $$APP_PATH"; \
	xcrun simctl install booted "$$APP_PATH"; \
	echo "▶ Launch $(BUNDLE_ID_IOS)"; \
	xcrun simctl launch booted $(BUNDLE_ID_IOS) || true
	@echo "✅ App iOS lancée"

# ─────────────────────────────────────────────
# Build tvOS Simulator
# ─────────────────────────────────────────────
.PHONY: tvsim
tvsim:
	@mkdir -p $(LOG_DIR)
	@echo "▶ Build tvOS Simulator..."
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME_TVOS) \
		-destination '$(TVOS_DEST)' \
		-configuration Debug \
		-derivedDataPath $(BUILD_DIR) \
		CODE_SIGNING_ALLOWED=NO \
		| tee $(LOG_DIR)/tvos_build.log \
		| xcpretty --color
	@echo "✅ Build tvOS terminé"

# ─────────────────────────────────────────────
# Run tvOS Simulator
# ─────────────────────────────────────────────
.PHONY: tvrun
tvrun: tvsim
	@echo "▶ Boot tvOS simulator $(TVOS_SIM_NAME)..."
	@open -a Simulator
	@xcrun simctl boot "$(TVOS_SIM_NAME)" 2>/dev/null || true
	@sleep 2
	@APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME_TVOS).app" | head -1); \
	if [ -z "$$APP_PATH" ]; then echo "❌ App tvOS introuvable"; exit 1; fi; \
	echo "▶ Install $$APP_PATH"; \
	xcrun simctl install booted "$$APP_PATH"; \
	echo "▶ Launch $(BUNDLE_ID_TVOS)"; \
	xcrun simctl launch booted $(BUNDLE_ID_TVOS) || true
	@echo "✅ App tvOS lancée"

# ─────────────────────────────────────────────
# Tests
# ─────────────────────────────────────────────
.PHONY: test
test:
	@mkdir -p $(LOG_DIR)
	xcodebuild test \
		-project $(PROJECT) \
		-scheme $(SCHEME_IOS) \
		-destination '$(IOS_DEST)' \
		-derivedDataPath $(BUILD_DIR) \
		| tee $(LOG_DIR)/tests.log \
		| xcpretty --color

# ─────────────────────────────────────────────
# Device réel
# ─────────────────────────────────────────────
.PHONY: device
device:
	@if [ -z "$(DEVICE_ID)" ]; then echo "❌ DEVICE_ID manquant"; exit 1; fi
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME_IOS) \
		-destination 'id=$(DEVICE_ID)' \
		-configuration Debug \
		-allowProvisioningUpdates \
		| xcpretty --color

# ─────────────────────────────────────────────
# Devices list
# ─────────────────────────────────────────────
.PHONY: devices
devices:
	@echo "📱 Devices disponibles :"
	@xcrun devicectl list devices 2>/dev/null || xcrun instruments -s devices

# ─────────────────────────────────────────────
# Logs
# ─────────────────────────────────────────────
.PHONY: logs
logs:
	@echo "--- ios_build.log ---"
	@cat $(LOG_DIR)/ios_build.log 2>/dev/null || true
	@echo ""
	@echo "--- tvos_build.log ---"
	@cat $(LOG_DIR)/tvos_build.log 2>/dev/null || true
	@echo ""
	@echo "--- tests.log ---"
	@cat $(LOG_DIR)/tests.log 2>/dev/null || true

# ─────────────────────────────────────────────
# Doctor
# ─────────────────────────────────────────────
.PHONY: doctor
doctor:
	@echo "▶ Xcode"
	@xcodebuild -version || true
	@echo ""
	@echo "▶ Swift"
	@swift --version || true
	@echo ""
	@echo "▶ Claude"
	@claude --version || true
	@echo ""
	@echo "▶ xcpretty"
	@xcpretty --version || true
	@echo ""
	@echo "▶ Simulators"
	@xcrun simctl list devices available | head -40 || true

# ─────────────────────────────────────────────
# Clean
# ─────────────────────────────────────────────
.PHONY: clean
clean:
	@rm -rf $(BUILD_DIR) $(LOG_DIR) $(ARCHIVE_DIR)
	@echo "🧹 Clean terminé"

# ─────────────────────────────────────────────
# Archive
# ─────────────────────────────────────────────
.PHONY: archive
archive:
	@mkdir -p $(ARCHIVE_DIR)
	xcodebuild archive \
		-project $(PROJECT) \
		-scheme $(SCHEME_IOS) \
		-archivePath $(ARCHIVE_DIR)/$(SCHEME_IOS).xcarchive \
		-allowProvisioningUpdates \
		| xcpretty --color
```

## Remarque importante

Pour que `make run` et `make tvrun` fonctionnent, il faut renseigner correctement :

- `BUNDLE_ID_IOS`
- `BUNDLE_ID_TVOS`

Sinon `simctl launch` ne pourra pas lancer l’app.

---

# 7. Scripts shell utiles

## 7.1 `Scripts/doctor.sh`

```bash
#!/usr/bin/env bash
set -e

echo "== Xcode =="
xcodebuild -version || true
echo

echo "== Swift =="
swift --version || true
echo

echo "== Claude =="
claude --version || true
echo

echo "== Simulators =="
xcrun simctl list devices available | head -50 || true
```

## 7.2 `Scripts/run_ios_sim.sh`

```bash
#!/usr/bin/env bash
set -e
make run
```

## 7.3 `Scripts/run_tvos_sim.sh`

```bash
#!/usr/bin/env bash
set -e
make tvrun
```

Pense à rendre les scripts exécutables :

```bash
chmod +x Scripts/*.sh
```

---

# 8. VS Code tasks

Créer `.vscode/tasks.json` :

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Run iOS Simulator",
      "type": "shell",
      "command": "make run",
      "group": {
        "kind": "build",
        "isDefault": true
      },
      "presentation": {
        "reveal": "always",
        "panel": "shared"
      },
      "problemMatcher": []
    },
    {
      "label": "Run tvOS Simulator",
      "type": "shell",
      "command": "make tvrun",
      "presentation": {
        "reveal": "always",
        "panel": "shared"
      },
      "problemMatcher": []
    },
    {
      "label": "Run Tests",
      "type": "shell",
      "command": "make test",
      "group": "test",
      "presentation": {
        "reveal": "always",
        "panel": "shared"
      },
      "problemMatcher": []
    },
    {
      "label": "Doctor",
      "type": "shell",
      "command": "make doctor"
    },
    {
      "label": "Clean",
      "type": "shell",
      "command": "make clean"
    },
    {
      "label": "List Devices",
      "type": "shell",
      "command": "make devices"
    }
  ]
}
```

Usage :

- `⌘⇧B` → **Run iOS Simulator**
- `⌘⇧P` → **Tasks: Run Task**
- raccourci facile pour lancer l’app sans ouvrir Xcode

---

# 9. VS Code settings projet

Créer `.vscode/settings.json` :

```json
{
  "editor.formatOnSave": true,
  "editor.tabSize": 4,
  "files.exclude": {
    "**/.git": true,
    "**/.build": true,
    "**/DerivedData": true,
    "**/*.xcuserstate": true
  },
  "search.exclude": {
    "**/.build": true,
    "**/DerivedData": true
  },
  "git.autofetch": true,
  "continue.enableTabAutocomplete": true,
  "swift.path": "/usr/bin/swift"
}
```

---

# 10. Continue.dev — configuration type

Exemple de config `~/.continue/config.json` :

```json
{
  "models": [
    {
      "title": "Claude Sonnet",
      "provider": "anthropic",
      "model": "claude-sonnet-4-20250514",
      "apiKey": "YOUR_ANTHROPIC_API_KEY"
    }
  ],
  "tabAutocompleteModel": {
    "title": "Claude Haiku",
    "provider": "anthropic",
    "model": "claude-haiku-20240307",
    "apiKey": "YOUR_ANTHROPIC_API_KEY"
  },
  "contextProviders": [
    { "name": "code" },
    { "name": "docs" },
    { "name": "terminal" },
    { "name": "open" }
  ]
}
```

## Point d’attention

Les noms exacts de modèles peuvent évoluer. Il faut vérifier la nomenclature effectivement disponible dans ton environnement Anthropic / Continue.

---

# 11. `.gitignore` recommandé

```gitignore
# Xcode
*.xcuserstate
xcuserdata/
DerivedData/
*.xcscmblueprint

# Build
.build/

# Logs
*.log

# VS Code
.vscode/launch.json

# Claude
.claude_cache/

# macOS
.DS_Store
```

---

# 12. Workflow quotidien conseillé

## Matin

```bash
cd MyApp
code .
claude   # optionnel
```

## Développement

- coder dans VS Code
- utiliser Continue pour les suggestions inline
- utiliser Claude Code pour les grosses refactorisations
- lancer `⌘⇧B` ou `make run`

## Test rapide iOS

```bash
make run
```

## Test rapide tvOS

```bash
make tvrun
```

## Tests unitaires

```bash
make test
```

## Device réel

```bash
make devices
make device DEVICE_ID=<udid>
```

## Livraison

```bash
make archive
```

Puis dans Xcode :

- Organizer
- Upload vers TestFlight / App Store Connect

---

# 13. Quand ouvrir Xcode

Xcode reste recommandé pour :

- régler le **signing**
- gérer les **capabilities**
- exécuter sur **iPhone / Apple TV physique**
- utiliser **LLDB**
- lancer **Instruments**
- archiver et soumettre
- corriger un problème de configuration projet Apple

En pratique :

- **VS Code** = édition / IA / automatisation
- **Xcode** = obligations Apple + debug avancé

---

# 14. Prompts utiles pour Claude

## Refactor SwiftUI

```text
Refactor this SwiftUI screen into smaller reusable components.
Keep previews. Preserve current behavior.
```

## MVVM

```text
Reorganize this feature into MVVM.
Move side effects to Services and keep the View simple.
```

## Tests

```text
Generate unit tests for this service and cover success + failure cases.
```

## Performance

```text
Review this SwiftUI view for performance issues and unnecessary re-renders.
```

## Package extraction

```text
Move the business logic of this feature into a local Swift Package named CoreKit.
```

---

# 15. Problèmes fréquents

| Problème | Cause probable | Correctif |
|---|---|---|
| `xcpretty: command not found` | non installé | `gem install xcpretty` |
| `make run` build OK mais app ne se lance pas | mauvais bundle id | corriger `BUNDLE_ID_IOS` |
| simulateur ne boot pas | état incohérent | `xcrun simctl shutdown all` puis `make run` |
| app introuvable après build | chemin `.app` non trouvé | vérifier scheme / target / nom du produit |
| provisioning error sur device | signing Apple | ouvrir Xcode et corriger Team / Signing |
| `devicectl` absent | version Xcode | fallback `xcrun instruments -s devices` |
| LSP Swift lent | indexation en cours | attendre, relancer VS Code ou extension |

---

# 16. Vérification finale

```bash
xcodebuild -version
swift --version
claude --version
xcpretty --version
make doctor
make help
```

Si tout sort sans erreur bloquante, le setup est prêt.

---

# 17. Conclusion

Le setup V4 apporte le niveau pratique qui manque souvent aux guides trop théoriques :

- **Claude** pour raisonner, générer, refactoriser
- **VS Code** pour coder vite
- **Makefile** pour normaliser les commandes
- **`make run` / `make tvrun`** pour lancer l’app sans friction
- **Xcode** uniquement là où Apple reste incontournable

C’est probablement l’un des meilleurs compromis actuels pour développer une app **iOS / tvOS** avec un workflow moderne orienté IA.
