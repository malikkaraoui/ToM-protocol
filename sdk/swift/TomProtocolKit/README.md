# TomProtocolKit

SDK Apple (iOS 16+ / tvOS 16+ / macOS 13+) du **protocole ToM** — messagerie P2P décentralisée, chiffrée de bout en bout, relais sans stockage.

Wrapper Swift (`actor TomNodeWrapper`) au-dessus du cœur Rust (`tom-protocol-ffi`), embarqué en XCFramework.

## Installation

### Depuis une release (recommandé, à venir)

Les releases `sdk-swift/vX.Y.Z` publient le XCFramework zippé + checksum SPM. Le `Package.swift` publié pointera `binaryTarget(url:checksum:)`.

### En local (monorepo)

```bash
# 1. Builder le XCFramework (5 slices : iOS/tvOS/macOS, device+sim)
bash scripts/build-tom-protocol-ffi-xcframework.sh
# 2. Le copier dans le package
bash scripts/sync-xcframework-to-package.sh
```

Puis dans Xcode : **File → Add Package Dependencies → Add Local…** → `sdk/swift/TomProtocolKit`.

## Usage

```swift
import TomProtocolKit

let node = TomNodeWrapper()
try await node.create(relayUrl: "http://192.168.0.21:3340", identityPath: nil, n0Discovery: false)
try await node.start(
    username: "alice", encryption: true, enableDht: false,
    relayUrl: "http://192.168.0.21:3340", identityPath: nil,
    n0Discovery: false, localDiscovery: true, dataDir: nil
)

try await node.sendMessage(to: peerId, payload: Data("salut".utf8))

// Modèle polling : drainer périodiquement (~500 ms)
let messages = await node.receiveMessages()
let peers = await node.discoveredPeers()
```

## Notes

- **Modèle polling** : pas de callbacks — drainer `receiveMessages()` depuis une `Task` périodique (latence ≈ l'intervalle choisi).
- Le handle natif est un `OpaquePointer` (header généré par cbindgen — `scripts/generate-ffi-header.sh`).
- Erreurs : `TomError` (LocalizedError) ; le détail FFI vient de `tom_node_last_error`.
- SDK Rust équivalent : crate `tom-sdk` (voir `crates/tom-sdk/README.md`).

Licence MIT.
