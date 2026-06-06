# ToM Protocol — Apple Platform Deployment Guide

## Overview

ToM Protocol ships as a **static XCFramework** (`TomProtocolFFI.xcframework`) built from a Rust FFI crate, consumed by SwiftUI apps on all Apple platforms:

| Platform | App | Min OS | Arch |
|----------|-----|--------|------|
| Apple TV | `apps/tom-node-tvos/` | tvOS 16.0 | arm64 |
| iPhone / iPad | `apps/tom-node-ios/` | iOS 16.0 | arm64 |
| MacBook (Apple Silicon) | iOS app via Mac Catalyst | macOS 13.0 | arm64 |
| MacBook (Intel) | iOS app via Mac Catalyst | macOS 13.0 | x86_64 |

All apps share the same Swift source tree. Platform differences are handled via `#if os(tvOS)` / `#if os(iOS)` / `#if os(macOS)` conditional compilation.

---

## Prerequisites

- macOS 13+ with Xcode 15+
- Rust nightly toolchain (`nightly-aarch64-apple-darwin`)
- `rustup`, `cargo`, `lipo` (bundled with Xcode Command Line Tools)

```bash
# Install Rust nightly + Apple targets
rustup toolchain install nightly-aarch64-apple-darwin
rustup target add aarch64-apple-tvos aarch64-apple-tvos-sim \
    aarch64-apple-ios aarch64-apple-ios-sim \
    aarch64-apple-darwin x86_64-apple-darwin \
    --toolchain nightly-aarch64-apple-darwin
```

---

## Step 1 — Build the Rust XCFramework

From the repo root:

```bash
bash scripts/build-tom-protocol-ffi-xcframework.sh
```

This builds and packages **6 slices** into a single XCFramework:

| Slice | Target triple |
|-------|--------------|
| tvOS device | `aarch64-apple-tvos` |
| tvOS simulator | `aarch64-apple-tvos-sim` |
| iOS device | `aarch64-apple-ios` |
| iOS simulator | `aarch64-apple-ios-sim` |
| macOS arm64 | `aarch64-apple-darwin` |
| macOS x86_64 | `x86_64-apple-darwin` |

macOS arm64 + x86_64 are merged into a universal binary via `lipo` before packaging.

**Output:**
```
apps/tom-node-tvos/build/
├── TomProtocolFFI.xcframework/   ← add to all Xcode projects
└── tom_protocol_ffi.h            ← C header (referenced by bridging headers)
```

**Environment flags:**
```bash
PROFILE=debug bash scripts/...              # debug build (faster)
DEVICE_ONLY=1 bash scripts/...             # skip simulator slices
MACOS=0 bash scripts/...                   # skip macOS slices
TVOS_DEPLOYMENT_TARGET=17.0 bash scripts/... # override deployment target
```

---

## Step 2 — tvOS App (apps/tom-node-tvos/)

The tvOS app is the reference implementation. Xcode project already exists.

```bash
cd apps/tom-node-tvos
make tvsim          # build for Apple TV simulator
make tvrun          # build + install + launch on simulator
make tvdevice       # build for physical Apple TV
make doctor         # diagnose setup issues
```

The Xcode project is at `apps/tom-node-tvos/TomNode.xcodeproj`.

---

## Step 3 — iOS/iPadOS App (apps/tom-node-ios/)

Swift sources are in `apps/tom-node-ios/TomNode/`. The Xcode project must be created on a Mac:

### 3.1 Create Xcode Project

1. Open Xcode → **File → New → Project**
2. Choose **iOS → App**
3. Configure:
   - **Product Name**: `TomNode`
   - **Bundle Identifier**: `malik.karaoui.TomNode-iOS`
   - **Interface**: SwiftUI
   - **Language**: Swift
   - **Minimum Deployments**: iOS 16.0
4. Save location: `apps/tom-node-ios/`

### 3.2 Add Source Files

1. Delete the default `ContentView.swift` and `<AppName>App.swift`
2. Right-click the project group → **Add Files to "TomNode"**
3. Navigate to `apps/tom-node-ios/TomNode/`
4. Select all folders (`Models/`, `Services/`, `Views/`) + `TomNodeApp.swift`
5. **Uncheck** "Copy items if needed"
6. Click **Add**

### 3.3 Add XCFramework

1. Target → **General** → **Frameworks, Libraries, and Embedded Content**
2. Click **+** → **Add Other... → Add Files...**
3. Navigate to `apps/tom-node-tvos/build/TomProtocolFFI.xcframework`
4. Select → **Open**
5. Set embed: **Do Not Embed** (static library)

### 3.4 Configure Build Settings

Target → **Build Settings**:

| Setting | Value |
|---------|-------|
| Objective-C Bridging Header | `TomNode/TomNode-Bridging-Header.h` |
| Header Search Paths | `$(PROJECT_DIR)/../tom-node-tvos/build` |

### 3.5 Build and Run

```
⌘+R → select iPhone 16 simulator → Run
```

Or via Makefile (after Xcode project exists):
```bash
cd apps/tom-node-ios
make iossim   # build
make iosrun   # build + launch
```

---

## Step 4 — macOS via Mac Catalyst

The iOS app runs natively on macOS (including Intel Macs) via Mac Catalyst — **no code changes required**.

1. Open the iOS Xcode project
2. Target → **General** → enable **"Mac (Designed for iPad)"**
   - Or enable **"Mac Catalyst"** for a full macOS window UI
3. Build → select **My Mac** as destination
4. **⌘+R**

The XCFramework includes a macOS universal slice (arm64 + x86_64), so both M-series and Intel Macs are covered.

### macOS-specific behavior

| Feature | macOS behavior |
|---------|----------------|
| Anti-sleep | No-op (macOS doesn't auto-sleep like iOS/tvOS) |
| Device name | Returns `"Mac"` |
| `appareil` log tag | `"macos"` |
| UDP log broadcast | Works (POSIX `getifaddrs`/`sendto` available on macOS) |
| AVAudioSession | Skipped (`#if !os(macOS)` guard) |

---

## Architecture: Swift → Rust

```
SwiftUI Views
    ↓
TomNodeService (@MainActor singleton)
    ↓
TomNodeWrapper (Swift actor — thread-safe FFI)
    ↓
TomNode-Bridging-Header.h (C ABI)
    ↓
TomProtocolFFI.xcframework (static lib)
    ↓
tom-protocol (Rust ProtocolRuntime)
    ↓
QUIC / UDP (tom-connect, tom-quinn)
```

### FFI Functions (14 total)

| Function | Purpose |
|----------|---------|
| `tom_node_create(relay_url, username)` | Create node instance |
| `tom_node_start(node)` | Start QUIC + relay connection |
| `tom_node_stop(node)` | Graceful shutdown |
| `tom_node_free(node)` | Free Rust memory |
| `tom_node_send_message(node, to, msg)` | Send 1-1 message |
| `tom_node_create_group(node, name)` | Create group |
| `tom_node_send_group_message(node, group_id, msg)` | Send to group |
| `tom_node_receive_messages(node)` | Poll inbox (JSON) |
| `tom_node_status(node)` | Poll node status (JSON) |
| `tom_node_last_error(node)` | Get last error string |
| `tom_node_add_peer_addr(node, addr)` | Add bootstrap peer |
| `tom_node_connected_peers(node)` | List connected peers (JSON) |
| `tom_node_discovered_peers(node)` | List all known peers (JSON) |
| `tom_node_free_string(s)` | Free C string from Rust |

All strings returned by Rust **must** be freed with `tom_node_free_string()`.

---

## Relay Configuration

The node connects to a relay for bootstrap and NAT traversal. Configure in-app Settings tab or via environment:

| Env var | Default | Description |
|---------|---------|-------------|
| `TOM_RELAY_URL` | `http://192.168.0.83:3340` | Relay URL |

Public relay (NAS, ARM64 Debian VM):
- LAN: `http://192.168.0.21:3340`
- WAN: `http://82.67.95.8:3340`

Run your own relay:
```bash
cargo run -p tom-relay -- --dev
```

---

## Troubleshooting

| Error | Fix |
|-------|-----|
| `tom_node_create` undefined | Check bridging header path + Header Search Paths |
| `libtom_protocol_ffi not found` | Re-run `bash scripts/build-tom-protocol-ffi-xcframework.sh` |
| Build fails on simulator | Ensure XCFramework has `-sim` slices (don't use `DEVICE_ONLY=1`) |
| Build fails on Intel Mac | Ensure XCFramework has `x86_64-apple-darwin` slice |
| Cannot find `TomNodeService` | Check all Swift files are added to the Xcode target |
| `AVAudioSession` error on macOS | Ensure `#if !os(macOS)` guards are in TomNodeService.swift |
| XCFramework validation error | Delete `apps/tom-node-tvos/build/TomProtocolFFI.xcframework` and rebuild |

---

## File Layout Summary

```
tom-protocol/
├── crates/tom-protocol-ffi/          # Rust FFI crate (C ABI)
│   ├── Cargo.toml
│   ├── src/lib.rs                    # 14 exported C functions
│   └── include/tom_protocol_ffi.h   # C header
│
├── scripts/
│   └── build-tom-protocol-ffi-xcframework.sh  # Builds XCFramework (all platforms)
│
├── apps/
│   ├── tom-node-tvos/                # tvOS reference app
│   │   ├── TomNode.xcodeproj/
│   │   ├── TomNode/                  # Swift sources (shared)
│   │   │   ├── TomNodeApp.swift
│   │   │   ├── TomNode-Bridging-Header.h
│   │   │   ├── Models/
│   │   │   ├── Services/
│   │   │   └── Views/
│   │   ├── build/
│   │   │   ├── TomProtocolFFI.xcframework   ← shared by all apps
│   │   │   └── tom_protocol_ffi.h
│   │   └── Makefile
│   │
│   └── tom-node-ios/                 # iOS/iPadOS app (same sources)
│       ├── TomNode/                  # Swift sources (symlinked logic)
│       ├── SETUP.md                  # Xcode project creation guide
│       └── Makefile
│
└── docs/
    └── DEPLOY-APPLE.md               # ← this file
```
