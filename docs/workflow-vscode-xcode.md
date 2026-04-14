# Workflow VS Code ↔ Xcode — TomNode tvOS

> Coder dans VS Code avec Claude, builder/lancer depuis le terminal, Xcode uniquement quand Apple l'impose.

---

## Pipeline

```
VS Code (Claude edite le Swift/Rust)
    ↓
make tvrun  (ou Cmd+Shift+B)
    ↓
xcodebuild + simctl  (build + install + launch)
    ↓
Simulateur Apple TV
    ↓
Xcode (seulement pour signing, LLDB, Instruments)
```

---

## Scripts en place

### 1. Makefile (`apps/tom-node-tvos/Makefile`)

| Commande | Action |
|----------|--------|
| `make tvrun` | Build + install + lance le simulateur Apple TV |
| `make tvsim` | Build seul (sans lancer) |
| `make ffi` | Recompile le Rust FFI → `.a` (simulateur) |
| `make ffi-device` | Recompile le Rust FFI (device physique) |
| `make tvdevice DEVICE_ID=xxx` | Deploy sur Apple TV physique |
| `make tvtest` | Tests unitaires sur simulateur |
| `make doctor` | Diagnostic complet (Xcode, Rust, FFI, simulateurs) |
| `make clean` | Nettoyage |

### 2. build_ffi.sh (`apps/tom-node-tvos/TomNode/Services/build_ffi.sh`)

Script appele par Xcode Build Phases automatiquement :
- Detecte la plateforme cible (simulateur vs device) via `EFFECTIVE_PLATFORM_NAME`
- Trouve le workspace Rust automatiquement
- Compile avec `cargo +nightly` pour tvOS
- Copie le `.a` dans `build/`

### 3. VS Code tasks.json

`Cmd+Shift+B` lance directement le build sans ouvrir Xcode.

Ref : `docs/ios-vscode-claude-workflow-v4.md` (setup complet V4)

---

## Quand ouvrir Xcode

- Signing / capabilities
- Device physique (Apple TV)
- LLDB / Instruments
- Archive + soumission TestFlight / App Store

---

## Documents de reference

| Document | Contenu |
|----------|---------|
| `docs/ios-vscode-claude-workflow-v4.md` | Setup complet V4 (Makefile, tasks.json, scripts, .gitignore) |
| `docs/tom-tvos-node-plan.md` | Plan implementation node tvOS (7 phases) |
| `apps/tom-node-tvos/CLAUDE.md` | Contexte projet (architecture, fichiers, commandes) |
| `apps/tom-node-tvos/TomNode/Services/BUILD_FFI_TROUBLESHOOTING.md` | Troubleshooting build FFI Rust→tvOS |
