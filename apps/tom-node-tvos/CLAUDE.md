# TomNode tvOS — Claude Context

## Project
- Name: TomNode
- Platform: tvOS 16.0+
- Language: Swift 5.9+
- UI: SwiftUI (MVVM)
- Architecture: MVVM + Services + Rust FFI

## Core Stack
- **Rust FFI**: `libtom_protocol_ffi.a` (tom-protocol-ffi crate)
- **Bridging Header**: `TomNode/TomNode-Bridging-Header.h`
- **C Header**: `build/tom_protocol_ffi.h`
- **Static library**: `build/libtom_protocol_ffi.a`

## Scheme
- TomNode (tvOS app)

## Simulator
- Apple TV 4K (3rd generation)

## Bundle ID
- malik.karaoui.TomNode

## Dev Team
- K22558HU63

## Architecture
```
Swift UI (Views)
    ↓
ViewModels (ObservableObject)
    ↓
TomNodeService (singleton, @MainActor)
    ↓
TomNodeWrapper (actor, FFI bridge)
    ↓
tom_protocol_ffi.h (C ABI)
    ↓
libtom_protocol_ffi.a (Rust)
    ↓
tom-protocol (ProtocolRuntime)
```

## File Structure
```
TomNode/
├── TomNodeApp.swift          — @main entry
├── TomNode-Bridging-Header.h — FFI bridge
├── Views/
│   ├── ContentView.swift     — TabView (4 tabs)
│   ├── StatusView.swift      — Node status + start/stop
│   ├── MessagesView.swift    — 1-1 messages
│   ├── GroupsView.swift      — Group messaging
│   └── SettingsView.swift    — Config + identity
├── ViewModels/               — (future extraction from Service)
├── Models/
│   ├── TomModels.swift       — TomPeer, TomMessage, TomGroup, TomNodeStatus
│   ├── TomError.swift        — Error enum
│   └── TomNodeWrapper.swift  — FFI actor wrapper
├── Services/
│   └── TomNodeService.swift  — Singleton orchestrator
└── Assets.xcassets/
```

## Commands
- `make tvsim` — build for simulator
- `make tvrun` — build + install + launch
- `make tvtest` — run unit tests
- `make ffi` — rebuild Rust FFI (simulator)
- `make ffi-device` — rebuild Rust FFI (device)
- `make doctor` — check setup
- `make clean` — clean builds

## Rules
- async/await preferred (no Combine legacy)
- TomNodeWrapper is an actor (thread-safe FFI access)
- TomNodeService is @MainActor (UI updates on main thread)
- Message polling: 500ms interval via Task
- All FFI strings freed with tom_node_free_string()
- Preview per View with mock data

## Do Not Modify
- project.pbxproj signing settings
- Bridging header path
- Library/header search paths
- Bundle identifier
