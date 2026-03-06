# TOM Protocol — Full Node on tvOS (Apple TV)

## Phase 0 : Kickoff Planning
**Status**: READY FOR NEW SESSION
**Objective**: Execute OPTION B — Complete tom-protocol ProtocolRuntime node via FFI on Apple TV HD
**Scope**: Full P2P node (send/recv messages, groups, discovery) + native tvOS UI
**Timeline**: ~10-15 days (parallel track: VSCode + Xcode)

---

## Architecture (VSCode ↔ Xcode Loop)

```
VSCode (Claude Code) ← → Xcode (Signing, LLDB, device)
        ↓
   Swift Packages
   (CoreKit: tom-protocol FFI bindings)
        ↓
   TomNode.app (tvOS)
        ↓
   Apple TV HD (A8 ARM64)
```

**Workflow**:
- `80-95%` dev in VSCode + Claude
- `make tvrun` → simulator in seconds
- Xcode only for: signing, device, Archive

---

## Phase 1 : Project Bootstrap (2-3 days)

### 1.1 Create tvOS Xcode Project
- [ ] Create `apps/tom-node-tvos/` structure per ios_vscode_claude_workflow_v4.md
- [ ] Create `TomNode.xcodeproj` with schemes:
  - `TomNode-Simulator` (Debug)
  - `TomNode-Device` (Release)
- [ ] Info.plist: network entitlements (local network + bonjour)
- [ ] Capabilities: Background Modes (Audio, Network fetch)

### 1.2 Makefile (tvOS focused)
- [ ] Create root `Makefile` with targets:
  - `make tvbuild` — build tvOS simulator
  - `make tvrun` — build + install + launch simulator
  - `make tvdevice` — build for physical Apple TV
  - `make tvtest` — unit tests on simulator
  - `make tvlogs` — tail console logs
  - `make clean`
- [ ] Reference: ios_vscode_claude_workflow_v4.md section 6

### 1.3 CLAUDE.md for tvOS
- [ ] Document project:
  - Platform: tvOS 16.3+
  - UI: SwiftUI
  - Architecture: MVVM + Services + local Packages
  - Core package: `Packages/TomCoreKit` (Rust FFI + protocol bindings)
  - Scheme: `TomNode`
  - Simulator: `Apple TV 4K (3rd generation)` or `Apple TV HD`

### 1.4 VS Code Setup
- [ ] `.vscode/tasks.json` with `make tvrun`, `make tvtest`
- [ ] `.vscode/settings.json` per workflow doc
- [ ] `.gitignore` for iOS/tvOS builds

---

## Phase 2 : tom-protocol FFI Library (4-5 days)

### 2.1 tom-protocol-ffi Crate (Rust)
**File**: `crates/tom-protocol-ffi/src/lib.rs`

Exports:
- `tom_node_create(config_json: *const c_char) → TomNodeHandle`
- `tom_node_start(handle) → i32`
- `tom_node_stop(handle) → i32`
- `tom_node_send_message(handle, target_id, payload) → i32`
- `tom_node_create_group(handle, members_json) → *const c_char` (group_id)
- `tom_node_send_group_message(handle, group_id, payload) → i32`
- `tom_node_receive_messages(handle) → *const c_char` (JSON batch)
- `tom_node_status(handle) → *const c_char` (JSON status)
- `tom_node_free(handle) → void`
- `tom_node_free_string(s) → void`

**Internal**:
- Wraps `ProtocolRuntime::spawn()` (from tom-protocol)
- Manages tokio runtime lifetime
- Serializes Rust errors → JSON for Swift

**Tests**:
- [ ] `test_node_lifecycle()` — create/start/stop
- [ ] `test_send_receive_1v1()`
- [ ] `test_group_create_send()`
- [ ] `test_status_updates()`

**Build**:
- [ ] `cargo build -p tom-protocol-ffi --target aarch64-apple-tvos --release`
- [ ] Output: `target/aarch64-apple-tvos/release/libtom_protocol_ffi.a` (~80-100 MB)

### 2.2 Build Script
**File**: `scripts/build-tom-protocol-ffi-tvos.sh`

- [ ] Parallel build for simulator + device
- [ ] Stage artifacts in `apps/tom-node-tvos/build/`
- [ ] Generate C header + Swift module map

### 2.3 Validate Compilation
- [ ] Build for aarch64-apple-tvos
- [ ] Build for aarch64-apple-tvos-sim (simulator)
- [ ] Verify no missing symbols (cross-compile dependencies)

---

## Phase 3 : Swift FFI Bindings (3-4 days)

### 3.1 Local Swift Package: TomCoreKit
**Path**: `Packages/TomCoreKit/Sources/TomCore/`

**Files**:
- [ ] `TomNode.swift` — wrapper around FFI handle
- [ ] `TomNodeConfig.swift` — config struct (Codable)
- [ ] `Models.swift` — Peer, Message, Group (Swift types)
- [ ] `TomError.swift` — error enum (from Rust JSON)
- [ ] `TomDelegate.swift` — async message receiver

**Example Structure**:
```swift
actor TomNode: Sendable {
    private var handle: TomNodeHandle?

    func start(config: TomNodeConfig) async throws
    func stop() async throws
    func sendMessage(to: NodeId, payload: Data) async throws -> MessageId
    func createGroup(members: [NodeId]) async throws -> GroupId
    func sendGroupMessage(group: GroupId, payload: Data) async throws -> MessageId
    func receiveMessages() async -> [DeliveredMessage]
    func status() async -> TomNodeStatus
}
```

### 3.2 Bridging Header
**File**: `apps/tom-node-tvos/TomNode/TomNode-Bridging-Header.h`

```objc
#ifndef TomNode_Bridging_Header_h
#define TomNode_Bridging_Header_h

#import "tom_protocol_ffi.h"

#endif
```

### 3.3 Test (Swift)
- [ ] `TomNodeTests.swift` — actor lifecycle + message flow
- [ ] Mock FFI for CI (optional)

---

## Phase 4 : tvOS UI (4-5 days)

### 4.1 Views (SwiftUI)
**File**: `apps/tom-node-tvos/TomNode/Views/`

- [ ] `ContentView.swift` — root tab view
  - **Tab 1: Status**
    - Node ID (QR code)
    - Connection status (online/offline)
    - Peers discovered (count)
    - Groups joined (list)
    - Start/Stop button

  - **Tab 2: Messages**
    - Recent 1-1 messages (table)
    - Tap to reply (modal)
    - Search by peer

  - **Tab 3: Groups**
    - Groups joined (list)
    - Tap → group detail (members, messages)
    - Create group (add peers modal)

  - **Tab 4: Settings**
    - Node identity display (copy)
    - Relay URL
    - DHT toggle (if compiled)
    - Logs (tail last 100 lines)

### 4.2 ViewModels (MVVM)
**File**: `apps/tom-node-tvos/TomNode/ViewModels/`

- [ ] `NodeViewModel.swift` — start/stop, status polling
- [ ] `MessagesViewModel.swift` — message list, send
- [ ] `GroupsViewModel.swift` — group list, detail
- [ ] `SettingsViewModel.swift` — identity, relay config

### 4.3 Services
**File**: `apps/tom-node-tvos/TomNode/Services/`

- [ ] `TomNodeService.swift` — singleton wrapper (TomNode actor)
- [ ] `MessagePoller.swift` — async task polling `receiveMessages()` every 500ms
- [ ] `PersistenceService.swift` — save identity, config, seen peers (UserDefaults/CoreData)

### 4.4 Reusable Components
- [ ] `QRCodeView.swift` — display node ID as QR
- [ ] `PeerBadge.swift` — online/offline indicator
- [ ] `MessageCell.swift` — message timestamp + author + text

### 4.5 Preview Mocks
- [ ] Mock TomNode (for Xcode canvas)
- [ ] Sample data (peers, messages, groups)

---

## Phase 5 : Integration & Build (2-3 days)

### 5.1 Link FFI Library in Xcode
- [ ] **Build Phases** → **Link Binary With Libraries**
  - Add `build/libtom_protocol_ffi.a`
  - Add `build/libtom_core_kit.a` (if pre-built)

### 5.2 Header Search Paths
- [ ] **Build Settings** → **Header Search Paths**
  - Add: `$(PROJECT_DIR)/../../apps/tom-node-tvos/build`

### 5.3 Bridging Header Config
- [ ] **Build Settings** → **Objective-C Bridging Header**
  - Set: `TomNode/TomNode-Bridging-Header.h`

### 5.4 Compile & Test
- [ ] `make tvbuild` — should link cleanly
- [ ] `make tvrun` — should start simulator + install + launch
- [ ] **UI smoke test**:
  - Tap **Start**
  - Verify logs show `tom_node_create` + `tom_node_start` success
  - Status updates to "Running"
  - Tap **Stop** → "Stopped"

### 5.5 Device Build
- [ ] Provisioning profile (or automatic signing)
- [ ] `make tvdevice DEVICE_ID=<udid>`
- [ ] Test on physical Apple TV HD

---

## Phase 6 : E2E Testing (2-3 days)

### 6.1 Local Network Tests
- [ ] **On Simulator**:
  - Start node on tvOS simulator
  - Create group + send 10 messages
  - Verify delivery + local persistence

- [ ] **On Device** (Apple TV HD):
  - Same test sequence
  - Measure: startup time, memory, CPU
  - Verify network: check logs for DHT/relay connectivity

### 6.2 Multi-Node Tests
- [ ] **Mac (tom-tui --username alice) ↔ Apple TV (tom-node)**
  - Send 1-1 message: Mac → TV
  - Verify: TV receives + displays
  - Send reply: TV → Mac
  - Verify: Mac receives + ACK displayed

### 6.3 Stress Test (if time)
- [ ] Send 100 messages in loop (5 sec)
- [ ] Monitor: memory, CPU, heat
- [ ] Verify: no crashes, all delivered

---

## Phase 7 : Polish & Documentation (1-2 days)

### 7.1 Logging
- [ ] Rust FFI: structured logs (tracing) → JSON
- [ ] Swift: capture FFI logs → on-device log viewer
- [ ] Tail endpoint in UI (Settings tab)

### 7.2 Error Handling
- [ ] Parse Rust JSON errors → Swift exceptions
- [ ] Show user-friendly alerts (network down, bind failed, etc.)

### 7.3 README
- [ ] `apps/tom-node-tvos/README.md`:
  - Setup instructions (Makefile, Xcode)
  - Architecture diagram
  - Known limitations (tvOS sandbox, network)
  - Testing on simulator vs device

### 7.4 CLAUDE.md Update
- [ ] Reflect final architecture
- [ ] Document FFI bindings strategy

---

## Dependencies & Critical Path

```
FFI Library (Phase 2)
    ↓ (blocks all other phases)
Swift Bindings (Phase 3)
    ↓
UI Views (Phase 4)
    ↓
Build & Integration (Phase 5)
    ↓
Testing (Phase 6)
    ↓
Polish (Phase 7)
```

**Critical Dependencies**:
- ✅ tom-protocol-ffi compiles
- ✅ FFI functions link cleanly in tvOS
- ✅ Swift actor model handles async properly
- ✅ Message polling doesn't block UI

---

## Known Constraints (tvOS)

| Constraint | Impact | Workaround |
|---|---|---|
| No file system (app sandbox) | State persistence limited | UserDefaults + in-memory cache |
| No background execution (unless enabled) | Node stops when app backgrounded | Background Modes capability (Audio/Network) |
| Network discovery (Bonjour) | Peer discovery on LAN only | Rely on DHT + custom relay |
| 4 KB stack limit per thread (rumor) | Deep call stacks may crash | Monitor stack usage in Rust FFI |
| A8 chip (older models) | Performance limited | Optimize async spawning; use release build |

---

## Success Criteria

✅ **Phase 1**: Xcode project boots, Makefile works, VSCode setup complete
✅ **Phase 2**: tom-protocol-ffi compiles for tvOS (device + sim), tests pass
✅ **Phase 3**: Swift bindings link cleanly, no undefined symbols
✅ **Phase 4**: UI renders on simulator, Start/Stop buttons functional
✅ **Phase 5**: `make tvrun` launches app, node starts without crashing
✅ **Phase 6**: Send 1-1 message (Mac ↔ TV), group message (TV → group)
✅ **Phase 7**: 100 messages delivered, no memory leak, clean shutdown

**Final**: `curl http://<APPLE_TV_IP>:3343/health` → HTTP 200 (relay on node) ✓

---

## Next Session Agenda

1. Create `apps/tom-node-tvos/` project structure
2. Implement `tom-protocol-ffi` (Phase 2.1)
3. Build + validate for tvOS
4. Begin Swift bindings (Phase 3)
5. Checkpoint: FFI ↔ Swift handshake works

---

## Files to Track

- `crates/tom-protocol-ffi/` — FFI library (new)
- `scripts/build-tom-protocol-ffi-tvos.sh` — build script (new)
- `apps/tom-node-tvos/` — Xcode project (new)
- `Packages/TomCoreKit/` — Swift package (new)
- `Makefile` — root (tvOS targets added)
- `CLAUDE.md` — updated with tvOS info

---

**Prepared by**: Claude Code
**Status**: Ready for next session
**Approach**: VSCode + Xcode workflow per ios_vscode_claude_workflow_v4.md
