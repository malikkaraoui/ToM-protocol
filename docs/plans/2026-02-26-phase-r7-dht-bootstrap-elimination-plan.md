# Phase R7: DHT + Bootstrap Elimination — Implementation Plan (REVISED)

> **For Claude:** Execute sub-phases in order. Each sub-phase = testable deliverable + go/no-go decision.

**Goal:** Eliminate WebSocket signaling by implementing DHT-first discovery.

**Prerequisite:** Phase R6 complete (✅ 352 tests passing)

**Revision rationale:** Original plan was too big (11 tasks, 7-9 days, Task 4 alone = 2-3 days). Breaking into 4 incremental sub-phases with validation checkpoints.

---

## Phase R7.1: DHT Proof of Concept (1-2 days)

**Goal:** Prove DHT discovery works BEFORE committing to the fork. Low risk, fast validation.

**Success criteria:**

- [ ] DHT integration functional (publish + lookup)
- [ ] 2 nodes discover each other via DHT
- [ ] Encrypted + signed messages delivered
- [ ] No changes to tom-transport yet (DHT added alongside current discovery)

**Deliverable:** Working DHT on current stack. If this fails, we abort before fork investment.

---

### R7.1 — Task 1: Create tom-dht wrapper crate

**Files:**

- Create: `crates/tom-dht/`
- Create: `crates/tom-dht/Cargo.toml`
- Create: `crates/tom-dht/src/lib.rs`
- Modify: `Cargo.toml` (workspace member)

**Actions:**

1. **Create crate:**
```bash
cargo new --lib crates/tom-dht
```

2. **Setup Cargo.toml:**
```toml
[package]
name = "tom-dht"
version = "0.1.0"
edition = "2021"
description = "DHT discovery for ToM Protocol"
license = "MIT"

[dependencies]
mainline = "2.0"  # Mainline DHT (BEP-0044)
sha1 = "0.10"     # NodeId → DHT key hashing
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

3. **Create lib.rs:**
```rust
//! DHT-based peer discovery for ToM Protocol.
//! Uses Mainline DHT (BEP-0044) for distributed peer discovery.

use anyhow::Result;
use mainline::Dht;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::time::Duration;

/// Node address for DHT storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtNodeAddr {
    pub node_id: String,  // iroh::PublicKey string repr
    pub relay_urls: Vec<String>,
    pub direct_addrs: Vec<String>,
    pub timestamp: u64,
}

/// DHT discovery service.
pub struct DhtDiscovery {
    dht: Dht,
}

impl DhtDiscovery {
    /// Create and bootstrap DHT.
    pub async fn new() -> Result<Self> {
        let mut dht = Dht::client()?;
        dht.bootstrap().await?;
        tracing::info!("DHT bootstrapped");
        Ok(Self { dht })
    }

    /// Publish node address to DHT.
    pub async fn publish(&self, addr: DhtNodeAddr) -> Result<()> {
        let key = dht_key(&addr.node_id);
        let value = serde_json::to_vec(&addr)?;

        // BEP-0044: mutable storage (24h TTL)
        self.dht.put_mutable(key, &value).await?;
        tracing::info!("Published to DHT: {}", addr.node_id);
        Ok(())
    }

    /// Lookup peer by node ID.
    pub async fn lookup(&self, node_id: &str) -> Result<Option<DhtNodeAddr>> {
        let key = dht_key(node_id);
        let timeout = Duration::from_secs(5);

        match tokio::time::timeout(timeout, self.dht.get(&key)).await {
            Ok(Ok(Some(value))) => {
                let addr: DhtNodeAddr = serde_json::from_slice(&value)?;
                tracing::info!("DHT lookup success: {}", node_id);
                Ok(Some(addr))
            }
            Ok(Ok(None)) => {
                tracing::warn!("DHT lookup not found: {}", node_id);
                Ok(None)
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                tracing::warn!("DHT lookup timeout: {}", node_id);
                Ok(None)
            }
        }
    }
}

/// Hash node ID to 20-byte DHT key (SHA1).
fn dht_key(node_id: &str) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(node_id.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_key_deterministic() {
        let id = "test-node-id";
        let key1 = dht_key(id);
        let key2 = dht_key(id);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_dht_key_different() {
        let key1 = dht_key("node-a");
        let key2 = dht_key("node-b");
        assert_ne!(key1, key2);
    }
}
```

4. **Add to workspace:**
```toml
# Cargo.toml (root)
[workspace]
members = [
    "crates/tom-transport",
    "crates/tom-protocol",
    "crates/tom-tui",
    "crates/tom-stress",
    "crates/tom-dht",  # NEW
]
```

5. **Build:**
```bash
cargo build -p tom-dht
cargo test -p tom-dht
cargo clippy -p tom-dht
```

**Acceptance:**

- [ ] tom-dht crate compiles
- [ ] Unit tests pass
- [ ] Clippy clean

**Commit:**
```bash
git add crates/tom-dht Cargo.toml
git commit -m "feat(dht): create tom-dht wrapper crate

- Mainline DHT (BEP-0044) integration
- DhtDiscovery::publish() + lookup()
- 5s timeout with fallback
- Part of Phase R7.1 (DHT PoC)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.1 — Task 2: Integrate DHT into tom-protocol (alongside current discovery)

**Files:**

- `crates/tom-protocol/Cargo.toml`
- `crates/tom-protocol/src/runtime/mod.rs` or `runtime.rs`
- `crates/tom-protocol/src/runtime/config.rs`

**Actions:**

1. **Add tom-dht dependency:**
```toml
# crates/tom-protocol/Cargo.toml
[dependencies]
tom-dht = { path = "../tom-dht" }
```

2. **Add DHT to RuntimeConfig:**
```rust
pub struct RuntimeConfig {
    pub username: String,
    pub encryption: bool,
    pub enable_dht: bool,  // NEW: optional DHT discovery
    // ... existing fields
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            enable_dht: true,  // Enable by default in R7.1
            // ... existing defaults
        }
    }
}
```

3. **Add DHT to ProtocolRuntime:**
```rust
use tom_dht::{DhtDiscovery, DhtNodeAddr};

pub struct ProtocolRuntime {
    local_id: NodeId,
    // ... existing fields
    dht: Option<DhtDiscovery>,  // NEW
}
```

4. **Initialize DHT in spawn():**
```rust
pub fn spawn(node: TomNode, config: RuntimeConfig) -> RuntimeChannels {
    // ... existing setup

    // Initialize DHT if enabled
    let dht = if config.enable_dht {
        match DhtDiscovery::new().await {
            Ok(d) => {
                // Publish our address
                let our_addr = DhtNodeAddr {
                    node_id: local_id.to_string(),
                    relay_urls: vec![], // Add from node.addr()
                    direct_addrs: vec![], // Add from MagicSock
                    timestamp: current_timestamp_ms(),
                };
                if let Err(e) = d.publish(our_addr).await {
                    tracing::warn!("DHT publish failed: {e}");
                }
                Some(d)
            }
            Err(e) => {
                tracing::warn!("DHT init failed: {e}");
                None
            }
        }
    } else {
        None
    };

    // ... existing spawn logic
}
```

5. **Add DHT lookup before send:**
```rust
// In handle_send_message() or equivalent
async fn try_dht_lookup(&self, target_id: &NodeId) -> Option<NodeAddr> {
    if let Some(ref dht) = self.dht {
        if let Ok(Some(addr)) = dht.lookup(&target_id.to_string()).await {
            tracing::info!("DHT resolved peer: {target_id}");
            // Convert DhtNodeAddr → iroh::EndpointAddr
            // Add peer to topology
            return Some(addr);
        }
    }
    None
}
```

**Acceptance:**

- [ ] DHT integrated into runtime
- [ ] Optional (enable_dht flag)
- [ ] Publishes on startup
- [ ] Lookups before connect
- [ ] All existing tests still pass

**Commit:**
```bash
git add crates/tom-protocol
git commit -m "feat(protocol): integrate DHT discovery (optional)

- RuntimeConfig.enable_dht flag (default: true)
- Publish address to DHT on startup
- Lookup peers via DHT before connect
- Fallback to existing discovery if DHT fails
- All 352 tests pass

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.1 — Task 3: E2E test — DHT-only discovery

**Files:**

- Create: `crates/tom-protocol/tests/dht_e2e.rs`

**Actions:**

1. **Write integration test:**
```rust
use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tokio::time::Duration;

#[tokio::test]
#[ignore] // Requires DHT network
async fn test_dht_only_discovery() {
    tracing_subscriber::fmt::init();

    // Node A
    let node_a = tom_transport::TomNode::bind_random().await.unwrap();
    let id_a = node_a.id();
    let config_a = RuntimeConfig {
        username: "alice".into(),
        encryption: true,
        enable_dht: true,
        ..Default::default()
    };
    let channels_a = ProtocolRuntime::spawn(node_a, config_a);

    // Node B
    let node_b = tom_transport::TomNode::bind_random().await.unwrap();
    let id_b = node_b.id();
    let config_b = RuntimeConfig {
        username: "bob".into(),
        encryption: true,
        enable_dht: true,
        ..Default::default()
    };
    let mut channels_b = ProtocolRuntime::spawn(node_b, config_b);

    // Wait for DHT publish (both nodes)
    tokio::time::sleep(Duration::from_secs(10)).await;

    // A sends message to B (DHT lookup happens internally)
    channels_a
        .handle
        .send_message(id_b, b"hello DHT".to_vec())
        .await
        .unwrap();

    // B receives message
    let msg = tokio::time::timeout(Duration::from_secs(20), channels_b.messages.recv())
        .await
        .expect("timeout waiting for message")
        .expect("channel closed");

    assert_eq!(msg.payload, b"hello DHT");
    assert!(msg.was_encrypted, "message should be encrypted");
    assert!(msg.signature_valid, "signature should be valid");

    println!("✅ DHT-only discovery successful!");
}
```

2. **Run test:**
```bash
cargo test -p tom-protocol --test dht_e2e -- --ignored --nocapture
```

**Acceptance:**

- [ ] Test passes (2 nodes discover via DHT)
- [ ] No WebSocket signaling used
- [ ] Message encrypted + signed
- [ ] Total time < 30s (DHT is slow cold-start)

**Commit:**
```bash
git add crates/tom-protocol/tests/dht_e2e.rs
git commit -m "test(protocol): E2E DHT-only peer discovery

- 2 nodes discover each other via Mainline DHT
- No signaling server required
- Encrypted + signed message delivery
- --ignored test (requires network)
- Validates Phase R7.1 success

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.1 Checkpoint: Go/No-Go Decision

**If test passes:**
- ✅ DHT works on current stack
- ✅ Proceed to R7.2 (fork preparation)

**If test fails:**
- ❌ DHT issues (network, bootstrap, lookup timeout)
- ❌ Investigate before investing in fork
- Options:
  - Debug DHT integration
  - Try alternative DHT library
  - Fallback to Pkarr-only (no fork needed)

---

## Phase R7.2: Fork Preparation (1-2 days)

**Goal:** Understand fork boundaries, create crate skeletons, upgrade iroh. No breaking changes yet.

**Success criteria:**

- [ ] iroh upgraded to 0.97/1.0-rc (or skip documented)
- [ ] tom-connect skeleton exists (types only)
- [ ] tom-relay skeleton exists (structure only)
- [ ] API boundaries documented
- [ ] All tests still pass (no functionality moved yet)

**Deliverable:** Fork structure ready. If we abort here, we only have empty crates to clean up.

---

### R7.2 — Task 1: Upgrade iroh to 0.97/1.0-rc

**Files:**

- `crates/tom-transport/Cargo.toml`

**Actions:**

1. **Check crates.io:**
```bash
cargo search iroh --limit 1
```

2. **If 0.97+ available:**
```bash
cd crates/tom-transport
cargo update iroh
cargo test
```

3. **If NOT available:**
- Document decision to stay on 0.96
- Update MEMORY.md with reasoning

**Acceptance:**

- [ ] iroh upgraded OR skip documented
- [ ] All transport tests pass

**Commit:**
```bash
git commit -m "chore(transport): upgrade iroh to 0.97

(OR: chore(transport): document stay on 0.96 - 0.97 not stable yet)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### R7.2 — Task 2: Create tom-connect skeleton

**Files:**

- Create: `crates/tom-connect/`

**Actions:**

1. **Create basic crate:**
```bash
cargo new --lib crates/tom-connect
```

2. **Minimal Cargo.toml:**
```toml
[package]
name = "tom-connect"
version = "0.1.0"
edition = "2021"
description = "ToM transport layer — forked from iroh 0.96"
license = "MIT"

[dependencies]
quinn = "0.11"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
serde = { version = "1", features = ["derive"] }

# Will add more during R7.3 (MagicSock copy)
```

3. **Type placeholders only:**
```rust
//! tom-connect — ToM transport layer (forked from iroh)
//!
//! Phase R7.2: Skeleton only. MagicSock copy happens in R7.3.

/// Node identity (Ed25519 public key).
/// Will replace iroh::PublicKey in R7.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId([u8; 32]);

impl NodeId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Node address (ID + relay URLs + direct addresses).
/// Will replace iroh::NodeAddr in R7.3.
#[derive(Debug, Clone)]
pub struct NodeAddr {
    pub node_id: NodeId,
    pub relay_urls: Vec<String>,
    pub direct_addrs: Vec<std::net::SocketAddr>,
}

/// Endpoint placeholder.
/// Will contain MagicSock + Quinn in R7.3.
pub struct Endpoint;
```

4. **Add to workspace:**
```toml
# Cargo.toml (root)
members = [
    # ... existing
    "crates/tom-connect",
]
```

5. **Build:**
```bash
cargo build -p tom-connect
cargo clippy -p tom-connect
```

**Acceptance:**

- [ ] Compiles clean
- [ ] Types defined (no functionality yet)

**Commit:**
```bash
git add crates/tom-connect Cargo.toml
git commit -m "feat(connect): create tom-connect skeleton

- NodeId + NodeAddr types (placeholders)
- Endpoint stub (no MagicSock yet)
- Phase R7.2: structure only, functionality in R7.3

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.2 — Task 3: Create tom-relay skeleton

**Files:**

- Create: `crates/tom-relay/`

**Actions:**

1. **Create crate:**
```bash
cargo new --lib crates/tom-relay
```

2. **Minimal Cargo.toml:**
```toml
[package]
name = "tom-relay"
version = "0.1.0"
edition = "2021"
description = "ToM relay server — forked from iroh-relay"
license = "MIT"

[[bin]]
name = "tom-relay"
path = "src/bin/server.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
anyhow = "1"

# Will add more during R7.3
```

3. **Placeholder lib.rs:**
```rust
//! tom-relay — ToM relay server (forked from iroh-relay)
//!
//! Phase R7.2: Skeleton only. Full copy happens in R7.3.

pub struct RelayServer;
```

4. **Placeholder binary:**
```rust
// src/bin/server.rs
fn main() {
    println!("tom-relay server (placeholder - R7.3 will implement)");
}
```

5. **Add to workspace, build:**
```bash
cargo build -p tom-relay
cargo run -p tom-relay
```

**Acceptance:**

- [ ] Compiles
- [ ] Binary runs (placeholder message)

**Commit:**
```bash
git add crates/tom-relay Cargo.toml
git commit -m "feat(relay): create tom-relay skeleton

- RelayServer stub
- Binary placeholder
- Phase R7.2: structure only, functionality in R7.3

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.2 — Task 4: Document API boundaries

**Files:**

- Create: `docs/plans/2026-02-26-fork-api-boundaries.md`

**Actions:**

1. **Document what we call from iroh:**
```markdown
# Fork API Boundaries

## What we USE from iroh (will fork)

### MagicSock
- `MagicSock::spawn()`
- Path multiplexing (relay + direct)
- Hole punching (Disco protocol)

### Relay Client
- `relay::Client::connect()`
- Send/receive datagrams

### Types
- `PublicKey` → `NodeId`
- `NodeAddr` → `NodeAddr`

## What we DON'T use (won't fork)

- iroh-blobs
- iroh-docs
- iroh-gossip (we have our own)
- iroh Router (we have tom-protocol)

## Dependencies to keep upstream

- Quinn (QUIC)
- rustls (TLS)
- netwatcher
- portmapper
```

**Commit:**
```bash
git add docs/plans/2026-02-26-fork-api-boundaries.md
git commit -m "docs: document fork API boundaries

Phase R7.2: preparation for R7.3 MagicSock copy

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.2 Checkpoint: Ready for Fork?

- ✅ iroh upgraded (or skip justified)
- ✅ Crate skeletons exist
- ✅ API boundaries clear
- ✅ All tests pass

**Next:** R7.3 (the actual fork — 2-3 days)

---

## Phase R7.3: The Fork (2-3 days)

**Goal:** Copy MagicSock + relay to tom-connect/tom-relay. Replace tom-transport. All tests pass.

**Success criteria:**

- [ ] MagicSock code copied to tom-connect
- [ ] Relay code copied to tom-relay
- [ ] tom-protocol uses tom-connect (not tom-transport)
- [ ] All 352 tests pass
- [ ] DHT E2E test still works

**Deliverable:** Fork complete, fully functional. This is the big work.

---

### R7.3 — Task 1: Copy MagicSock to tom-connect

**Context:** This is the largest single task (2-3 days). MagicSock is ~2000 lines across multiple modules.

**Files:**

- Copy from `/tmp/iroh/iroh/src/magicsock/` → `crates/tom-connect/src/magic_sock/`

**Actions:**

1. **Download iroh source:**
```bash
cd /tmp
git clone https://github.com/n0-computer/iroh.git --branch v0.96.1 --depth 1
```

2. **Copy MagicSock modules:**
```bash
cp -r /tmp/iroh/iroh/src/magicsock crates/tom-connect/src/magic_sock
cp /tmp/iroh/iroh/src/disco.rs crates/tom-connect/src/disco.rs
```

3. **Adapt imports:**
- Replace `iroh::` → `tom_connect::`
- Replace `PublicKey` → `NodeId`
- Replace `NodeAddr` → `NodeAddr` (our type)
- Remove iroh-blobs/docs integration points

4. **Create endpoint.rs:**
```rust
use crate::magic_sock::MagicSock;
use quinn::Endpoint as QuinnEndpoint;

pub struct Endpoint {
    magic_sock: MagicSock,
    quinn: QuinnEndpoint,
}

impl Endpoint {
    pub async fn bind(config: Config) -> Result<Self> {
        // Initialize MagicSock
        let magic_sock = MagicSock::spawn(config).await?;

        // Setup Quinn with MagicSock transport
        let quinn = QuinnEndpoint::new_with_abstract_socket(
            // ... Quinn config
            Box::new(magic_sock.clone()),
        )?;

        Ok(Self { magic_sock, quinn })
    }
}
```

5. **Build incrementally:**
```bash
cargo build -p tom-connect 2>&1 | head -100
# Fix errors iteratively
```

**Note:** Budget 2-3 days. Many compiler errors initially. Fix systematically.

**Acceptance:**

- [ ] MagicSock compiles in tom-connect
- [ ] Endpoint API functional
- [ ] Localhost connection test passes

**Commit:**
```bash
git add crates/tom-connect
git commit -m "feat(connect): copy iroh MagicSock + Disco

- MagicSock path multiplexer
- Disco hole punching protocol
- Endpoint wrapper (bind + connect)
- Forked from iroh v0.96.1 (MIT license)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.3 — Task 2: Copy relay to tom-relay

**Files:**

- Copy from `/tmp/iroh/iroh-relay/` → `crates/tom-relay/`

**Actions:**

1. **Copy relay source:**
```bash
cp -r /tmp/iroh/iroh-relay/src/* crates/tom-relay/src/
cp /tmp/iroh/iroh-relay/Cargo.toml crates/tom-relay/Cargo.toml.iroh
```

2. **Adapt Cargo.toml:**
- Keep dependencies
- Rename package to `tom-relay`

3. **Adapt code:**
- `iroh_relay` → `tom_relay` namespace
- Keep wire protocol compatible

4. **Build:**
```bash
cargo build -p tom-relay
cargo run -p tom-relay -- --help
```

**Acceptance:**

- [ ] Relay server compiles
- [ ] Binary runs
- [ ] Wire protocol compatible with iroh relays

**Commit:**
```bash
git add crates/tom-relay
git commit -m "feat(relay): copy iroh-relay server

- Stateless relay server (WebSocket)
- Wire protocol compatible with iroh v0.96
- Self-hostable (Docker + fly.io)
- Forked from iroh-relay v0.96.1 (MIT license)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.3 — Task 3: Replace tom-transport with tom-connect

**Files:**

- `crates/tom-protocol/Cargo.toml`
- All `tom-protocol` source files
- `crates/tom-tui/Cargo.toml`
- `crates/tom-stress/Cargo.toml`

**Actions:**

1. **Update dependencies:**
```toml
# crates/tom-protocol/Cargo.toml
tom-connect = { path = "../tom-connect" }  # NEW
# tom-transport = { path = "../tom-transport" }  # REMOVE (comment out)
```

2. **Find/replace in tom-protocol:**
```bash
cd crates/tom-protocol
rg "tom_transport" --files-with-matches | xargs sed -i '' 's/tom_transport/tom_connect/g'
```

3. **Manual API fixes:**
- `TomNode::bind()` → `Endpoint::bind()`
- `node.id()` → `endpoint.node_id()`
- Update any transport-specific logic

4. **Update binaries:**
```toml
# tom-tui and tom-stress: same replacement
tom-connect = { path = "../tom-connect" }
```

5. **Test:**
```bash
cargo test --workspace
```

**Acceptance:**

- [ ] All 352 tests pass
- [ ] DHT E2E test still works
- [ ] Binaries compile (tom-tui, tom-stress)

**Commit:**
```bash
git add crates/tom-protocol crates/tom-tui crates/tom-stress
git commit -m "refactor(protocol): replace tom-transport with tom-connect

- Swap dependency: tom-transport → tom-connect
- Update imports and API calls
- All 352 tests passing
- DHT E2E test passing

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.3 Checkpoint: Fork Complete?

- ✅ MagicSock in tom-connect
- ✅ Relay in tom-relay
- ✅ tom-protocol uses tom-connect
- ✅ All tests pass

**Next:** R7.4 (make DHT primary, deprecate signaling)

---

## Phase R7.4: Integration & Deprecation (1 day)

**Goal:** Make DHT primary discovery, deprecate signaling server, update docs.

**Success criteria:**

- [ ] DHT integrated into tom-connect Endpoint
- [ ] Signaling server marked DEPRECATED
- [ ] CLAUDE.md updated
- [ ] sprint-status.yaml updated (Phase R7 done)

**Deliverable:** Bootstrap eliminated. Documentation updated.

---

### R7.4 — Task 1: Integrate DHT into tom-connect Endpoint

**Files:**

- `crates/tom-connect/Cargo.toml`
- `crates/tom-connect/src/endpoint.rs`

**Actions:**

1. **Add tom-dht dependency:**
```toml
tom-dht = { path = "../tom-dht" }
```

2. **Add DHT to Endpoint:**
```rust
use tom_dht::{DhtDiscovery, DhtNodeAddr};

pub struct Endpoint {
    magic_sock: MagicSock,
    quinn: QuinnEndpoint,
    dht: DhtDiscovery,  // NEW
}

impl Endpoint {
    pub async fn bind(config: Config) -> Result<Self> {
        // ... MagicSock + Quinn setup

        // Bootstrap DHT
        let dht = DhtDiscovery::new().await?;

        // Publish our address
        let our_addr = DhtNodeAddr {
            node_id: config.node_id.to_string(),
            relay_urls: config.relay_urls.clone(),
            direct_addrs: magic_sock.local_addrs().iter().map(|a| a.to_string()).collect(),
            timestamp: current_timestamp_ms(),
        };
        dht.publish(our_addr).await?;

        // Spawn re-publish task (every 12h)
        tokio::spawn(republish_task(dht.clone(), our_addr));

        Ok(Self { magic_sock, quinn, dht })
    }

    /// Connect to peer (DHT lookup first).
    pub async fn connect(&self, target_id: NodeId) -> Result<Connection> {
        // Try DHT lookup
        if let Some(addr) = self.dht.lookup(&target_id.to_string()).await? {
            tracing::info!("DHT resolved {target_id}");
            // Convert DhtNodeAddr → NodeAddr, add to MagicSock
            return self.connect_addr(addr).await;
        }

        // Fallback: relay (if configured)
        Err(anyhow!("peer not found in DHT"))
    }
}
```

**Acceptance:**

- [ ] Endpoint publishes to DHT on bind
- [ ] Endpoint::connect() uses DHT lookup
- [ ] All tests pass

**Commit:**
```bash
git add crates/tom-connect
git commit -m "feat(connect): DHT-first peer discovery in Endpoint

- Publish NodeAddr to DHT on bind
- Lookup peer via DHT before connect
- Auto-republish every 12h
- DHT is primary, relay is fallback

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.4 — Task 2: Deprecate signaling server

**Files:**

- `tools/signaling-server/README.md`
- `tools/signaling-server/package.json`

**Actions:**

1. **Update README:**
```markdown
# Signaling Server (DEPRECATED)

> **Status:** DEPRECATED as of Phase R7 (2026-02-26).
> Use DHT-first discovery instead. See `crates/tom-connect`.

This WebSocket signaling server was used for bootstrap during Phases R1-R6.
It is no longer required for ToM operation.

## Migration to DHT

Replace signaling with tom-connect Rust crate or WASM bindings.

## Removal Timeline

- **Phase R7** (now): Signaling optional, DHT primary
- **Phase R8** (Q2 2026): Signaling removed entirely

## Running (for legacy testing only)

\`\`\`bash
npm install && npm start
\`\`\`
```

2. **Mark deprecated in package.json:**
```json
{
  "name": "tom-signaling-server",
  "version": "1.0.0-deprecated",
  "description": "DEPRECATED: Use DHT discovery (Phase R7)",
  "deprecated": true
}
```

**Commit:**
```bash
git add tools/signaling-server
git commit -m "chore(signaling): deprecate WebSocket signaling server

Phase R7 complete — DHT-first discovery is primary.
Signaling will be removed in Phase R8.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.4 — Task 3: Update CLAUDE.md

**Files:**

- `CLAUDE.md`

**Actions:**

1. **Add DHT section:**
```markdown
## DHT-First Discovery (Phase R7)

ToM uses Mainline DHT (BEP-0044) for peer discovery. No signaling server required.

### Discovery Flow

1. Node starts → joins Mainline DHT
2. Node publishes: `put(hash(NodeId), NodeAddr)` → 24h TTL
3. Peer lookup: `get(hash(target_NodeId))` → returns NodeAddr
4. Connect via MagicSock (relay or direct)

### Crates

- `tom-connect` — Transport layer (forked from iroh 0.96)
- `tom-relay` — Relay server (forked from iroh-relay)
- `tom-dht` — DHT discovery wrapper
- `tom-protocol` — Protocol layer (unchanged)

### WebSocket Signaling

**Status:** DEPRECATED (Phase R7). Will be removed in Phase R8.
```

2. **Update Repository Structure:**
```markdown
tom-protocol/
├── crates/
│   ├── tom-connect/       # NEW: Transport (forked from iroh)
│   ├── tom-relay/         # NEW: Relay server (forked from iroh-relay)
│   ├── tom-dht/           # NEW: DHT discovery
│   ├── tom-protocol/      # Protocol layer
│   ├── tom-tui/           # TUI demo
│   └── tom-stress/        # Stress testing
```

**Commit:**
```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for Phase R7

- DHT-first discovery architecture
- Fork crates documented
- Signaling marked deprecated

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

### R7.4 — Task 4: Update sprint-status.yaml

**Files:**

- `_bmad-output/implementation-artifacts/sprint-status.yaml`

**Actions:**

```yaml
  # Phase R7: DHT + Bootstrap Elimination
  rust-phase-7: done
  r7-1-dht-poc: done                   # DHT proof-of-concept on existing stack
  r7-2-fork-preparation: done          # Crate skeletons + API boundaries
  r7-3-the-fork: done                  # Copy MagicSock + relay, replace tom-transport
  r7-4-integration: done               # DHT primary, signaling deprecated
```

**Commit:**
```bash
git add _bmad-output/implementation-artifacts/sprint-status.yaml
git commit -m "chore: mark Phase R7 complete — DHT + bootstrap elimination

- R7.1: DHT PoC (1-2 days)
- R7.2: Fork preparation (1-2 days)
- R7.3: The fork (2-3 days)
- R7.4: Integration (1 day)
Total: 5-8 days across 4 sub-phases

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
git push
```

---

## Phase R7 Final Checklist

- [ ] **R7.1 (DHT PoC):** DHT works on current stack, E2E test passes
- [ ] **R7.2 (Prep):** Crate skeletons, iroh upgraded, API boundaries documented
- [ ] **R7.3 (Fork):** MagicSock + relay copied, tom-connect replaces tom-transport, all tests pass
- [ ] **R7.4 (Integration):** DHT primary in Endpoint, signaling deprecated, docs updated

**Total estimated time:** 5-8 days (vs original 7-9 days, but with 4 checkpoints instead of 1)

---

## Revised Timeline Summary

| Sub-Phase | Duration | Risk | Deliverable |
|-----------|----------|------|-------------|
| R7.1 | 1-2 days | Low | DHT works (abort if fails) |
| R7.2 | 1-2 days | Low | Fork structure ready |
| R7.3 | 2-3 days | High | Fork complete, tests pass |
| R7.4 | 1 day | Low | Bootstrap eliminated |

**Key improvement:** Each sub-phase has a go/no-go checkpoint. If R7.1 fails, we abort before fork investment. If R7.3 is too hard, we still have DHT working on the old stack.

---

## Next Phase Preview (R8)

After R7 complete:

- **Phase R8:** Production hardening
  - Rate limiting + anti-spam (DHT pollution prevention)
  - Relay contribution incentives
  - WASM bindings (TypeScript SDK)
  - Signaling server removal (breaking change)
  - Performance tuning
  - Security audit
