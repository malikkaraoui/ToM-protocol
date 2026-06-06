# TomNode iOS/iPadOS — Xcode Setup

Sources are ready in `TomNode/`. Create the Xcode project in 10 minutes.

## Prerequisites

```bash
# From repo root — build the Rust XCFramework (needs macOS + nightly toolchain)
bash scripts/build-tom-protocol-ffi-xcframework.sh
# → produces: apps/tom-node-tvos/build/TomProtocolFFI.xcframework
```

## Step 1 — Create Xcode Project

1. Open Xcode → **File → New → Project**
2. Choose **iOS → App**
3. Configure:
   - **Product Name**: TomNode
   - **Bundle Identifier**: `malik.karaoui.TomNode-iOS`
   - **Interface**: SwiftUI
   - **Language**: Swift
   - **Minimum Deployments**: iOS 16.0
4. Save into this directory (`apps/tom-node-ios/`)

## Step 2 — Add Source Files

1. Delete the default `ContentView.swift` and `<AppName>App.swift` that Xcode creates
2. Right-click the project group → **Add Files to "TomNode"**
3. Navigate to `apps/tom-node-ios/TomNode/`
4. Select **all folders** (Models, Services, Views) + `TomNodeApp.swift`
5. Ensure "Copy items if needed" is **unchecked** (files are already in the right place)
6. Click **Add**

## Step 3 — Add XCFramework

1. In Xcode project navigator, select the **TomNode** project
2. Select the **TomNode** target → **General** tab
3. Scroll to **Frameworks, Libraries, and Embedded Content**
4. Click **+** → **Add Other... → Add Files...**
5. Navigate to `apps/tom-node-tvos/build/TomProtocolFFI.xcframework`
6. Select it and click **Open**
7. Set embed to **Do Not Embed** (static library)

## Step 4 — Configure Bridging Header

1. Target → **Build Settings** → search "bridging"
2. Set **Objective-C Bridging Header** to: `TomNode/TomNode-Bridging-Header.h`
3. Set **Header Search Paths** to: `$(PROJECT_DIR)/../tom-node-tvos/build`

## Step 5 — Build & Run

```
⌘+R  →  select iPhone 16 simulator  →  Run
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
| `tom_node_create` undefined | Check bridging header path + header search path |
| `libtom_protocol_ffi not found` | Re-run `make ffi-xcframework` |
| Build fails on simulator | Ensure XCFramework has `-sim` slice |
| Cannot find `TomNodeService` | Check all Swift files are added to the target |
