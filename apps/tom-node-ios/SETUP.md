# TomNode iOS/iPadOS — Setup

Le projet Xcode est généré par xcodegen depuis `project.yml`. La FFI Rust est
consommée via le Swift Package local **TomProtocolKit** (`sdk/swift/TomProtocolKit`) —
plus de bridging header ni d'XCFramework à câbler à la main (S2.4).

## Prerequisites

```bash
brew install xcodegen

# From repo root — build the Rust XCFramework + sync into the package
# (needs macOS + nightly toolchain)
bash scripts/build-tom-protocol-ffi-xcframework.sh
bash scripts/sync-xcframework-to-package.sh
# → produces: sdk/swift/TomProtocolKit/Artifacts/TomProtocolFFI.xcframework
```

## Step 1 — Generate the Xcode project

```bash
cd apps/tom-node-ios
make gen          # = xcodegen generate
```

`project.yml` déclare tout : sources, Info.plist, signing, et la dépendance
au package `TomProtocolKit` (chemin local `../../sdk/swift/TomProtocolKit`).

## Step 2 — Build & Run

```bash
make iossim       # build iPhone simulator
make iosrun       # build + launch
# ou dans Xcode : ⌘+R → iPhone 17 simulator
```

The node auto-starts 5 seconds after launch.

## Multi-platform (Mac Catalyst)

To add macOS support to the same project:
1. Target → **General** → enable **Mac (Designed for iPad)**
2. Or enable **Mac Catalyst** for a full macOS window

No code changes needed — all UIKit guards are in place.

## Troubleshooting

| Error | Fix |
|-------|-----|
| `Missing package product 'TomProtocolKit'` | Vérifier `sdk/swift/TomProtocolKit/Artifacts/` — re-run `make ffi-xcframework` |
| `tom_node_create` undefined | Artefact du package absent ou périmé — `make ffi-xcframework` |
| Build fails on simulator | Ensure XCFramework has `-sim` slice (5 slices attendues) |
| Cannot find `TomNodeService` | Re-run `make gen` (project.yml = source de vérité) |
| Project file drift | Ne jamais éditer le `.xcodeproj` à la main — éditer `project.yml` + `make gen` |
