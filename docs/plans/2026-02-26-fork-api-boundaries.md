# Phase R7: Fork API Boundaries

**Status**: Phase R7.2 - Preparation
**Date**: 2026-02-26
**Author**: AI pair programming session

## Overview

This document defines the precise boundaries between ToM-owned code and upstream dependencies after the iroh fork in Phase R7.3.

## Fork Scope: Socket Layer Only

### What We Fork (tom-connect + tom-relay)

**From iroh-net (→ tom-connect):**
- `MagicSock` - UDP hole punching + relay fallback logic
- `Endpoint` - QUIC endpoint management
- `NodeId` / `NodeAddr` - Identity and addressing types
- `Discovery` traits - DNS + Pkarr discovery interfaces

**From iroh-relay (→ tom-relay):**
- DERP-like relay server implementation
- Client-relay protocol (WebSocket upgrade)
- Relay mesh coordination

**Size estimate**: ~8,000-10,000 lines of Rust code

### What We Keep as Upstream Dependencies

**QUIC layer (Quinn):**
- `quinn` - QUIC protocol implementation
- `rustls` - TLS 1.3 encryption
- `tokio` - Async runtime

**Cryptography:**
- `ed25519-dalek` - Signing (we use separately)
- `x25519-dalek` - Key exchange (we use separately)
- `chacha20poly1305` - Encryption (we use separately)

**Discovery helpers:**
- `pkarr` - DNS-over-QUIC records
- `mainline` - DHT client (our tom-dht wrapper)

## API Boundaries

### 1. Transport Layer (tom-connect)

**Public API surface:**

```rust
// Identity (replaces iroh::PublicKey)
pub struct NodeId([u8; 32]);
impl NodeId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self;
    pub fn as_bytes(&self) -> &[u8; 32];
    pub fn to_string(&self) -> String;  // Base32 encoding
}

// Addressing (replaces iroh::NodeAddr)
pub struct NodeAddr {
    pub node_id: NodeId,
    pub relay_urls: Vec<String>,
    pub direct_addrs: Vec<std::net::SocketAddr>,
}

// Endpoint (replaces iroh::Endpoint)
pub struct Endpoint { /* internals hidden */ }
impl Endpoint {
    pub async fn bind() -> Result<Self>;
    pub async fn connect(&self, addr: NodeAddr) -> Result<Connection>;
    pub fn id(&self) -> NodeId;
    pub fn addr(&self) -> NodeAddr;
}

// Connection (wraps quinn::Connection)
pub struct Connection { /* internals hidden */ }
impl Connection {
    pub async fn open_bi(&self) -> Result<(SendStream, RecvStream)>;
    pub async fn accept_bi(&self) -> Result<(SendStream, RecvStream)>;
    pub fn remote_id(&self) -> NodeId;
}
```

**Dependencies:**
- `quinn::Endpoint` (internal)
- `quinn::Connection` (internal)
- `tokio::net::UdpSocket` (internal)

**NOT copied from iroh:**
- Blob transfer (iroh-blobs)
- Document sync (iroh-docs)
- Gossip (we already use iroh-gossip separately)
- RPC framework (iroh-rpc)

### 2. Relay Server (tom-relay)

**Public API surface:**

```rust
// Server (replaces iroh-relay server)
pub struct RelayServer {
    config: RelayConfig,
}

impl RelayServer {
    pub fn new(config: RelayConfig) -> Result<Self>;
    pub async fn serve(self, addr: SocketAddr) -> Result<()>;
}

pub struct RelayConfig {
    pub stun_addr: Option<SocketAddr>,
    pub rate_limit: Option<RateLimit>,
    pub mesh_peers: Vec<String>,  // Other relay URLs for mesh
}
```

**Binary entry point:**
```bash
tom-relay --addr 0.0.0.0:3478 --stun --mesh-peer https://relay2.tom.network
```

**Dependencies:**
- `axum` - HTTP/WebSocket server
- `tokio` - Async runtime

**NOT copied from iroh:**
- iroh-relay metrics/monitoring (we'll add our own)
- iroh-relay auth/billing (not needed for open protocol)

### 3. Protocol Layer (tom-protocol)

**What changes in R7.3:**

```rust
// BEFORE (R7.2):
use tom_transport::TomNode;  // wraps iroh::Endpoint
let node = TomNode::new().await?;

// AFTER (R7.3):
use tom_connect::Endpoint;  // our fork, not iroh
let endpoint = Endpoint::bind().await?;
```

**What stays the same:**
- All tom-protocol modules (Router, GroupHub, Backup, etc.)
- Crypto stack (signing, encryption)
- Wire format (MessagePack)
- iroh-gossip (still upstream dependency)

### 4. DHT Discovery (tom-dht)

**No changes in R7.3** - tom-dht already wraps `mainline` independently.

In R7.4, we add:
```rust
// Publish to DHT (BEP-0044 mutable)
pub async fn publish(&self, addr: DhtNodeAddr) -> Result<()>;

// Lookup from DHT
pub async fn lookup(&self, node_id: &str) -> Result<Option<DhtNodeAddr>>;
```

## Version Management

### iroh Dependency

**Current (R7.2):**
```toml
# tom-transport/Cargo.toml
iroh = "0.96.0"  # Staying on 0.96.0 until 0.97/1.0-rc

# tom-protocol/Cargo.toml
iroh = "0.96.0"
iroh-gossip = "0.96.0"
```

**After fork (R7.3):**
```toml
# tom-transport/Cargo.toml - DELETED (replaced by tom-connect)

# tom-protocol/Cargo.toml
tom-connect = { path = "../tom-connect" }  # Our fork
iroh-gossip = "0.96.0"  # Still upstream (unchanged)
```

### Maintenance Strategy

**Upstream tracking:**
- Monitor iroh releases for security fixes
- Cherry-pick relevant fixes to tom-connect/tom-relay
- No automatic version bumps

**Divergence points:**
- We remove blob/docs modules (not needed)
- We keep naming aligned with ToM conventions (NodeId not PublicKey)
- We may optimize relay protocol for ToM patterns

## Migration Path (R7.3)

### Step 1: Copy iroh-net → tom-connect
```bash
# Copy MagicSock + discovery
cp -r iroh/iroh-net/src/magicsock crates/tom-connect/src/
cp -r iroh/iroh-net/src/discovery crates/tom-connect/src/
cp -r iroh/iroh-net/src/endpoint.rs crates/tom-connect/src/

# Remove unneeded modules
rm -rf crates/tom-connect/src/blobs
rm -rf crates/tom-connect/src/docs
```

### Step 2: Copy iroh-relay → tom-relay
```bash
# Copy relay server
cp -r iroh/iroh-relay/src/* crates/tom-relay/src/

# Remove unneeded features
rm -rf crates/tom-relay/src/metrics
rm -rf crates/tom-relay/src/billing
```

### Step 3: Update tom-protocol imports
```rust
// Find/replace across tom-protocol:
// - use tom_transport::TomNode → use tom_connect::Endpoint
// - NodeId type stays tom_protocol::types::NodeId (our type)
// - But internally wraps tom_connect::NodeId
```

### Step 4: Delete tom-transport crate
```bash
rm -rf crates/tom-transport
```

### Step 5: Update Cargo.toml dependencies
```toml
# All crates that used tom-transport now use tom-connect directly
tom-connect = { path = "../tom-connect" }
```

### Step 6: Verify all tests pass
```bash
cargo test --workspace  # Must be 352 tests passing
```

## Test Requirements

**No regressions allowed:**
- All 352 existing tests must pass unchanged
- No performance degradation (hole punch success rate, latency)
- No API breaks for tom-protocol consumers

**New tests (optional in R7.3):**
- tom-connect unit tests (if we modify logic)
- tom-relay unit tests (if we modify protocol)

## Success Criteria for R7.3

- [ ] tom-connect compiles
- [ ] tom-relay compiles and runs
- [ ] tom-protocol migrated from tom-transport to tom-connect
- [ ] All 352 tests passing
- [ ] No iroh dependency in tom-transport/tom-protocol (except iroh-gossip)
- [ ] Binary size ≤ 15MB (release build)
- [ ] Memory footprint ≤ 50MB per node
- [ ] Hole punch success rate ≥ 90% (same as current)

## Open Questions

None at this stage. All boundaries clear.

## References

- [Phase R7 Design](./2026-02-26-phase-r7-dht-bootstrap-elimination-design.md)
- [Phase R7 Plan](./2026-02-26-phase-r7-dht-bootstrap-elimination-plan.md)
- [iroh repository](https://github.com/n0-computer/iroh) - upstream source
- [iroh-net crate](https://docs.rs/iroh-net/0.96.0) - fork source
- [iroh-relay crate](https://docs.rs/iroh-relay/0.96.0) - relay source
