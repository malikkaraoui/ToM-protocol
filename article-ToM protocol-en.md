# ToM: Design and Empirical Validation of a Peer-to-Peer Messaging Protocol Without Fixed Infrastructure

**Malik** — February 2026

---

## Abstract

This report presents ToM (The Open Messaging), a decentralized transport protocol inspired by biological mechanisms, where each device simultaneously acts as both client and relay. Unlike classical architectures (Signal, WhatsApp) relying on central servers, and existing decentralized protocols (libp2p, Hyperswarm, Nostr, Tor) still requiring some form of fixed infrastructure, ToM proposes a radically autonomous model: the network *is* the infrastructure, roles are imposed and rotating (unpredictable for an attacker), messages replicate like beneficial viruses to survive 24h, and the bootstrap itself is distributed — with no fixed entry point. We describe the cryptographic pipeline (ephemeral X25519 + XChaCha20-Poly1305 + HKDF-SHA256, encrypt-then-sign), analyze resistance to MITM, 51%, Sybil and Eclipse attacks, and show why the absence of consensus makes the 51% attack economically irrational. We present experimental results from 4 stress test campaigns (2,752 pings, 99.85% reliability during highway mobility, 0.98 ms on 5G) and cross-border tests Switzerland↔France (27 ms, verified E2E encryption). We compare ToM to BitTorrent, Tor, Nostr, libp2p and iroh, and show that no existing protocol combines mandatory contribution, unpredictable rotating roles, censorship-free anti-spam, self-replicating messages, and unconditional purge.

---

## 1. Introduction

### 1.1 Problem Statement

Current messaging systems rest on an architectural paradox: they promise confidentiality while centralizing message transit. Signal, despite its end-to-end encryption, maintains central servers that see metadata (who talks to whom, when, how much). WhatsApp, Telegram, iMessage — same pattern. Encryption protects content, but infrastructure betrays the social graph.

Existing decentralized protocols attempt to solve this problem:

- **libp2p** (Protocol Labs): multi-language, mature ecosystem, but relay-first philosophy — relays are an optimization, not the architectural foundation. Initial complexity is high and dependence on bootstrap relays remains structural.
- **Hyperswarm** (Holepunch/Mafintosh): DHT-first philosophy aligned with our vision, but limited to Node.js, no browser support, restricted community.
- **Matrix/Nostr**: server federation (Matrix) or voluntary relays (Nostr) — in both cases, someone must pay for and maintain the infrastructure.

**The fundamental problem remains unsolved**: how to build a messaging network that works without any entity having to maintain a server?

### 1.2 Economic Inversion

Centralized architectures suffer from a linear scaling problem: more users → more servers → more costs. Load and cost grow together.

ToM proposes the inversion: more participants → more available relays → more alternative paths → reduced latency → zero cost. Each device that joins the network *increases* its capacity. This is an emergent property of the architecture, not an optimization. This inversion cannot be replicated by a centralized architecture.

### 1.3 Contributions

This work brings:

1. A **unified node model** where each device runs identical code — the role (client, relay, backup) is dynamically imposed by the topology, never chosen
2. A **viral replication system** for messages destined to offline nodes, with strict 24h TTL and auto-deletion
3. An **experimental validation** with 4 stress test campaigns including highway mobility, carrier CGNAT, and Switzerland↔France border crossing
4. A **functional prototype** in Rust (1,008+ tests) validating QUIC transport, E2E encryption, and gossip discovery

### 1.4 Biological Inspiration: the Network as a Living Organism

ToM is not merely a network protocol. It is a **digital organism** whose architecture draws directly from biological mechanisms. This inspiration is not metaphorical — it is structural.

**The message as a beneficial virus.** In nature, a virus has a single objective: survive long enough to reach a receptive host. It mutates, it switches hosts when the current one weakens, it replicates to maximize its chances. In ToM, a message destined to an offline node behaves exactly this way: it replicates across 3 to 5 backup nodes, it monitors the "health" of each host (bandwidth, uptime, timezone compatible with the recipient), it **proactively migrates** to a better host before the current one fails — because waiting for the host to die is already too late. And like a virus, it dies after a defined time (24h) if its mission fails. The more hosts in the network, the more resilient the virus. **A positive virus: the more hosts, the stronger the organism.**

**The network as an immune system.** The anti-spam mechanism "the sprinkler gets sprinkled" works like an adaptive immune system. It does not destroy the intruder (no banning) — it progressively identifies and increases the immune response (increased workload) until the intruder exhausts itself. No NK cells that kill on contact (no blacklist), but a T-cell response that forces adaptation or abandonment. The network hardens through the attacks it endures.

**The sliding genesis: an organism without a fossil skeleton.** Blockchains accumulate millions of blocks — a fossil skeleton that grows heavier over time. ToM's L1 architecture uses a "sliding genesis": rather than accumulating history, the anchoring layer stays close to a genesis block in perpetual motion. Old states are compacted into cryptographic snapshots then purged. The network does not carry the weight of its past — like an organism that renews its cells, it only retains the present state.

**Ephemeral subnets: temporary organs.** When a communication pattern emerges (a group of nodes exchanging frequently), a subnet forms spontaneously — like an organ that develops in response to a need. When the need disappears, the subnet dissolves. No permanent unnecessary structure, no architectural debt. The network **breathes**: it structures and destructures continuously.

**Proof of Presence: the right to exist through presence.** Neither proof-of-work (right through computation), nor proof-of-stake (right through capital). ToM introduces the concept of **proof of presence**: the right to participate in network decisions is given by active presence. Not by what you own, not by what you invest, but by the simple fact of being there and contributing. It is the most egalitarian model possible — a smartphone on a mobile network has as many rights as a data center, as long as it is present.

### 1.5 Vision: a Protocol for a Free Internet

> *"The objective: a universal, resilient communication layer, with no central point of control, that is self-sufficient. A new protocol for an internet that belongs to no one."*
> — ToM Whitepaper v1, January 2026

ToM is not an application to install. It is intended to become a **protocol building block integrated into everyday tools** — browsers, messaging apps, email clients — so that the user participates in the network without knowing it. Just as TCP/IP transports packets without the user knowing, ToM will transport messages without being visible.

The source code itself will eventually be **self-hosted on the ToM network**. If GitHub goes down, if a government orders the repository removed, if a company applies pressure — the code lives on the network it created. Documentation, issues, the development workflow: all distributed on ToM. The protocol hosts its own evolution. **The umbilical cord is cut when the baby breathes on its own.**

This is the most radical property: once launched with enough nodes, **ToM can no longer be stopped** — only evolved. Like the Bitcoin network cannot be "turned off" (it would require simultaneously turning off tens of thousands of machines in dozens of countries), but without the financial incentive that concentrates power. There are no tokens to steal, no mining pools to target, no foundation to sue. It belongs to no one because it belongs to everyone.

---

## 2. State of the Art

### 2.1 Taxonomy of Existing Approaches

| Criterion | Signal | libp2p | Hyperswarm | Nostr | **ToM** |
|-----------|--------|--------|------------|-------|---------|
| Required infrastructure | Central servers | Bootstrap relays | DHT bootstrap | Voluntary relays | **None (post-bootstrap)** |
| Visible metadata | Server sees social graph | Relays see source/dest | DHT sees lookups | Relays see everything in cleartext | **Relays only see encrypted from/to** |
| NAT traversal | N/A (client-server) | relay + DCUtR | UDX hole punch | N/A (client-server) | **QUIC hole punch + relay fallback** |
| Node contribution | Passive (consumption) | Optional | Optional (seeding) | Voluntary (relays) | **Mandatory and invisible** |
| Message persistence | Indefinite (server) | Depends on app | None | Indefinite (relays) | **24h strict TTL, then purge** |
| E2E encryption | Yes (Signal Protocol) | App's choice | SecretStream | No (by default) | **Yes (XChaCha20-Poly1305)** |

### 2.2 Identified Limitations

**libp2p** solves multi-protocol transport but imposes disproportionate integration complexity for a messaging use case. The relay protocol (Circuit Relay v2) treats relays as a NAT workaround, not as the architectural core. Furthermore, the initial connection phase always depends on pre-known relays.

**Hyperswarm** has the right philosophy (DHT-first, native hole punch) but the Node.js-only implementation limits deployment. Its technical successor, the Holepunch stack (Pear Runtime), remains proprietary in its distribution.

**Nostr** democratizes access to relays but suffers from a free-riding problem: relay operators pay for infrastructure without guaranteed compensation. Messages are stored indefinitely, creating a storage scaling problem.

### 2.3 Choice of iroh as Transport Foundation

After comparative analysis, we chose **iroh** (n0-computer, 7,800+ GitHub stars, Rust, MIT) as the transport layer to study and eventually fork:

| Criterion | iroh | Justification |
|-----------|------|---------------|
| Transport | Native QUIC | Multiplexing, 0-RTT, connection migration |
| Identity | Ed25519 = network address | Exact alignment with the ToM model |
| NAT traversal | Hole punch + relay fallback | ~90% direct connections in production |
| Discovery | Pkarr (DNS-like) + gossip | Decentralized bootstrap |
| Relays | Stateless, pass-through | Philosophy identical to ToM |
| Encryption | Automatic QUIC TLS | E2E at transport level |
| License | MIT | Fork possible without constraints |

**Strategic decision**: iroh is used as a dependency for the PoC, with the explicit objective of forking the necessary modules once the ToM protocol is stabilized. No permanent dependency on n0-computer.

---

## 3. Architecture

### 3.1 Unified Node Model

Each ToM node runs identical code. There is no "client" binary distinct from a "server" binary:

```
┌─────────────────────────────────────────┐
│              Application                 │
├─────────────────────────────────────────┤
│  Layer 5: ProtocolRuntime               │  ← tokio::select! event loop
│           Router, Topology, Tracker     │
├─────────────────────────────────────────┤
│  Layer 4: Protocol                      │  ← Envelopes, Groups, Backup
│           MessagePack, Ed25519,         │
│           XChaCha20-Poly1305            │
├─────────────────────────────────────────┤
│  Layer 3: Discovery & Roles             │  ← Gossip, Ephemeral subnets
├─────────────────────────────────────────┤
│  Layer 2: Transport (tom-transport)     │  ← QUIC pool, Hole punch, Reconnection
├─────────────────────────────────────────┤
│  Layer 1: Network (iroh v0.96.1)       │  ← QUIC, Relay fallback, Pkarr DNS
└─────────────────────────────────────────┘
```

A node's role (relay, backup, simple client) is dynamically determined by the network based on its availability, contribution score, and local topology. A node never *chooses* to be a relay — it becomes one when the network demands it. This is the fundamental difference with BitTorrent (optional seeding) or Nostr (voluntary relays).

### 3.2 Envelope Format

Messages are encapsulated in envelopes serialized in MessagePack (≈60% more compact than JSON):

```rust
pub struct Envelope {
    id: String,              // UUIDv4
    from: NodeId,            // Sender's Ed25519 public key
    to: NodeId,              // Recipient's Ed25519 public key
    via: Vec<NodeId>,        // Relay chain (ordered)
    msg_type: MessageType,   // Chat, Ack, Heartbeat, Group*, Backup*...
    payload: Vec<u8>,        // Cleartext or encrypted
    timestamp: u64,          // Unix milliseconds
    signature: Vec<u8>,      // Ed25519 signature (64 bytes)
    ttl: u32,                // Hop counter (max 4)
    encrypted: bool,         // Payload encryption indicator
}
```

**Critical point**: the signature covers all fields *except* `ttl` and `signature` itself. TTL is excluded because relays decrement it in transit — including TTL would invalidate the signature after each hop. This allows relays to verify the envelope's authenticity without being able to otherwise modify it.

### 3.3 Key Exchange: How Two Nodes Share a Secret Without the Relay Seeing It

The central problem of any E2E system traversing relays is: **how do Alice and Bob establish a common symmetric key without an intermediary relay being able to reconstruct it?**

The classic answer is the Diffie-Hellman (DH) protocol. ToM uses a modern variant: **ephemeral-static X25519** (ECDH on Curve25519).

#### 3.3.1 The Key Exchange Problem in the Presence of Relays

In ToM, each message traverses at least one relay. The relay sees the entire envelope pass through. If Alice encrypted with a key she sends in the message, the relay could extract it. The solution: **never transmit the key itself**, but transmit only the information needed for Bob to *recompute* it — information useless to anyone who isn't Bob.

#### 3.3.2 Cryptographic Identity of Nodes

Each node possesses an Ed25519 key pair generated at first connection:
- **Private key**: 32 bytes (seed), never transmitted
- **Public key**: 32 bytes = **network address** of the node (no central registry, no DNS)

The Ed25519 public key is known to the network — it is the node's identifier. When Alice wants to write to Bob, she already knows his Ed25519 public key (discovered via gossip or prior exchange).

#### 3.3.3 Ed25519 → X25519 Conversion

Ed25519 operates on the Edwards curve (signing). Diffie-Hellman requires X25519, which operates on the Montgomery curve. The two curves are **birationally equivalent** via:

```
x_montgomery = (1 + y_edwards) / (1 - y_edwards)
```

For the public key:
```rust
// Decompress the Edwards point, convert to Montgomery
let edwards = CompressedEdwardsY(ed25519_pk_bytes).decompress()?;
let montgomery = edwards.to_montgomery();  // → X25519 public key
```

For the private key (seed):
```rust
// SHA-512(seed), take the first 32 bytes, apply X25519 clamping
let hash = Sha512::digest(ed25519_seed);
let mut secret = [0u8; 32];
secret.copy_from_slice(&hash[..32]);
secret[0] &= 248;      // Clear the 3 low bits
secret[31] &= 127;     // Clear the high bit
secret[31] |= 64;      // Set bit 6
```

This clamping is standard (RFC 7748): it forces the key into the prime-order subgroup, eliminating small-subgroup attacks. This is exactly the operation performed by `libsodium crypto_sign_ed25519_sk_to_curve25519`.

#### 3.3.4 Exchange Protocol — Ephemeral-Static Diffie-Hellman

For each message, Alice executes:

```
Step 1: Alice generates a fresh ephemeral X25519 key pair
        eph_secret ← X25519Secret::random(OsRng)     // System CSPRNG
        eph_public ← X25519PublicKey::from(eph_secret) // 32 bytes

Step 2: Alice converts Bob's public key (Ed25519 → X25519)
        bob_x25519 ← ed25519_to_x25519_public(bob_ed25519_pk)

Step 3: Diffie-Hellman
        shared_secret ← eph_secret.diffie_hellman(bob_x25519)
        // = eph_secret × bob_x25519 (scalar multiplication on Curve25519)
        // 32 bytes of shared secret

Step 4: Key Derivation (HKDF-SHA256)
        encryption_key ← HKDF-Expand(shared_secret,
                         info="tom-protocol-e2e-xchacha20poly1305-v1")
        // 32 bytes = AE key for XChaCha20-Poly1305

Step 5: AEAD Encryption
        nonce ← 24 random bytes (OsRng)
        ciphertext ∥ tag ← XChaCha20-Poly1305.encrypt(key, nonce, plaintext)
        // tag = 16 bytes Poly1305 (authentication)

Step 6: Encrypted payload construction
        EncryptedPayload = {
            ciphertext:    Vec<u8>,    // len(plaintext) + 16 bytes (tag)
            nonce:         [u8; 24],   // 24 bytes
            ephemeral_pk:  [u8; 32],   // Alice's ephemeral public key
        }
        // Total overhead: 32 + 24 + 16 = 72 bytes per message

Step 7: Encrypt-then-Sign signature
        envelope.payload ← MessagePack(EncryptedPayload)
        envelope.signature ← Ed25519.sign(signing_bytes)
        // The signature covers the ciphertext, not the plaintext
```

**What the relay sees passing through**:
- `from`: Alice's public key (in cleartext, needed for routing)
- `to`: Bob's public key (in cleartext, needed for routing)
- `payload`: `EncryptedPayload` serialized in MessagePack — i.e. `{ciphertext, nonce, ephemeral_pk}`
- `signature`: verifiable by the relay (proof the envelope comes from Alice)

**What the relay CANNOT do**:
- Decrypt `ciphertext` — it would require Bob's private key to compute `eph_secret.diffie_hellman(bob_x25519)`
- Reconstruct `shared_secret` — it sees `eph_public` and `bob_public`, but without `eph_secret` (never transmitted) nor `bob_secret`, the DH is irreversible (elliptic curve discrete logarithm problem)
- Forge a signature — it would require Alice's private key

**Bob's side (decryption)**:
```
bob_x25519_secret ← ed25519_to_x25519_secret(bob_ed25519_seed)
shared_secret     ← bob_x25519_secret.diffie_hellman(eph_public)
// Fundamental DH property:
//   eph_secret × bob_public  ==  bob_secret × eph_public
//   Both sides compute the SAME secret without ever transmitting it
encryption_key    ← HKDF-Expand(shared_secret, info=...)
plaintext         ← XChaCha20-Poly1305.decrypt(key, nonce, ciphertext)
```

#### 3.3.5 Why XChaCha20-Poly1305 and Not AES-GCM

| Criterion | XChaCha20-Poly1305 | AES-256-GCM |
|-----------|--------------------|-------------|
| Nonce size | **192 bits** | 96 bits |
| Random nonce safety | Yes (2^96 messages before collision) | No (birthday bound at ~2^32) |
| Need to coordinate nonces | **No** | Yes (counter or reuse risk) |
| Hardware instructions | None (pure software) | AES-NI (absent on low-end ARM) |
| ARM performance (Cortex-A72) | **Constant** | Variable (without AES-NI: 5-10x slower) |
| Deployed in | WireGuard, Signal, libsodium | TLS 1.3, SSH |

**The 192-bit nonce choice is decisive in a P2P context**: without a central server to manage a nonce counter, each node generates nonces randomly. With AES-GCM (96 bits), the birthday bound imposes a limit at ~2^32 messages per key pair — beyond that, likely nonce reuse → complete destruction of confidentiality (cf. Joux, 2006). With XChaCha20 (192 bits), this limit rises to ~2^96 — physically unreachable.

#### 3.3.6 HKDF and Domain Separation

```rust
const HKDF_INFO: &[u8] = b"tom-protocol-e2e-xchacha20poly1305-v1";
```

The `info` string acts as a **cryptographic domain separator** (Krawczyk, 2010). If the same `shared_secret` were accidentally used in another protocol with an identical HKDF, the derived key would be different thanks to this string. This is defense in depth: even in case of implementation error, keys do not leak to other contexts.

#### 3.3.7 Forward Secrecy

Each message uses a **fresh ephemeral X25519 key pair** (`OsRng`, the system CSPRNG). Consequence:
- Compromising Alice's long-term key (her Ed25519 seed) allows *signing* future messages in her name, but **does not allow decrypting her past messages** — because the ephemeral keys no longer exist in memory
- Compromising an ephemeral key gives access to **a single message** — the others use independent ephemeral pairs
- This is per-message forward secrecy, stronger than the per-session forward secrecy of classic TLS

#### 3.3.8 Encrypt-then-Sign: Order of Operations and Its Implications

ToM applies **Encrypt-then-Sign** (EtS) and not Sign-then-Encrypt (StE):

```rust
pub fn encrypt_and_sign(self, secret_seed, recipient_pk) -> Envelope {
    let mut env = self.build();
    env.encrypt_payload(recipient_pk)?;   // ← Encrypt first
    env.sign(secret_seed);                 // ← Sign second
    Ok(env)
}
```

**Why this order?**

1. **Relays can verify authenticity without decrypting**: The signature covers the ciphertext. A relay verifies `Ed25519.verify(signing_bytes, signature)` — if someone altered the ciphertext in transit, the signature fails. The relay rejects the corrupted envelope *without ever touching the content*.

2. **Double protection for the recipient**: Bob first verifies the signature (envelope intact?), then decrypts. The Poly1305 tag verifies plaintext integrity. Two independent authentication layers.

3. **No "surreptitious forwarding" attack**: In StE, an attacker could take a signed-then-encrypted message, decrypt it (if they are the recipient), re-encrypt it for someone else while keeping the original signature → the new recipient believes the original sender wrote to them. In EtS, the signature covers the ciphertext which includes `ephemeral_pk` (bound to the specific recipient) — redirection is detectable.

### 3.4 Routing

The `Router` is a pure decision engine — it receives an envelope and returns an action:

```rust
pub enum RoutingAction {
    Deliver(Envelope),              // For this node → deliver to application
    Forward(Envelope, NodeId),      // Not for this node → relay
    Reject(Envelope, RejectReason), // Invalid → reject
    Ack(String, NodeId),            // Delivery confirmation
    ReadReceipt(String, NodeId),    // Read confirmation
    Drop(Envelope, DropReason),     // TTL exhausted, duplicate → discard
}
```

The router never touches the network directly — it returns an intent that the `ProtocolRuntime` executes. This command/execution separation facilitates unit testing (237 Rust tests pass in <2s).

### 3.5 Discovery and Ephemeral Subnets

**Gossip (adapted HyParView)**: Each node maintains 2-3 active gossip neighbors and periodically exchanges `PeerAnnounce` messages. Convergence measured at ~3 seconds for a new node in a 15-peer network.

**Ephemeral subnets**: When a communication pattern is detected (2+ nodes exchanging frequently), a subnet forms via BFS clustering. The subnet exists only as long as it serves — automatic dissolution after 5 minutes of inactivity. Implementation note: after dissolution, dissolved nodes are excluded from BFS in the same cycle to prevent reformation-dissolution oscillation.

### 3.6 Viral Message Replication (ADR-009)

For offline recipients, messages behave like organisms seeking to survive:

```
[Message created] → [Recipient unreachable?]
       ↓                    ↓
  [Direct delivery]    [Replication across 3-5 backup nodes]
                              ↓
                    [Continuous host quality monitoring]
                    (timezone, bandwidth, uptime)
                              ↓
                    [Score < threshold X%] → [Proactive replication
                                              to better host]
                              ↓
                    [Self-deletion BEFORE the host dies]
                              ↓
                    [Recipient reconnected] → [Query + delivery]
                              ↓
                    [ACK propagated → global purge of all copies]
                              ↓
                    [24h elapsed] → [Unconditional purge]
```

**Key insight**: the message does not wait for its host to disconnect before migrating — that's already too late. It observes degradation and acts *before* the failure. The replication timestamp uses an absolute `expires_at` (not a relative TTL) to prevent time drift between nodes.

### 3.7 Group Messaging (Hub-and-Spoke)

Groups use a star topology with a deterministically elected hub:

- **Election**: sort members by `NodeId` (lexicographic order), the first online node becomes hub
- **Failover**: zero coordination — the next in the sorted list takes over immediately
- **Fan-out**: the hub receives a `GroupMessage` and distributes it to all members
- **No consensus**: no Raft, no Paxos, no voting — deterministic ordering eliminates the need

### 3.8 Unpredictability as a Fundamental Security Mechanism

The preceding sections describe classical cryptographic primitives (DH, AEAD, signatures). But ToM's true security innovation is **architectural**: the protocol is designed to be unpredictable at every layer. An attacker cannot anticipate the network's behavior, because the network itself does not know it in advance.

#### 3.8.1 Rotating and Random Roles: You Don't Know What You'll Have to Do

In BitTorrent, a node chooses to seed or not. In Nostr, an operator chooses to maintain a relay. In ToM, **no one chooses**. The network dynamically imposes roles:

```
Time T:   Alice = client,  Bob = relay,   Carol = backup
Time T+1: Alice = relay,   Bob = backup,  Carol = client
Time T+2: Alice = backup,  Bob = client,  Carol = relay
```

Assignment depends on factors the attacker does not control:
- **Contribution score**: historical consumption/service ratio (not falsifiable without actually contributing)
- **Local topology**: which nodes are connected to which (constantly changes with mobility)
- **Non-deterministic selection**: randomness in choosing the relay among eligible candidates

**Security consequence**: an attacker who wants to intercept Alice's messages as a relay cannot *position itself* as Alice's relay — it's the network that decides. And even if it obtains this role at time T, it will lose it at T+1. Compare with:
- **Tor**: relays are permanent voluntary servers → an attacker controlling a Tor relay controls it 24/7
- **Nostr**: relays are chosen by the user → an attacker compromising a Nostr relay intercepts all its traffic indefinitely
- **Signal**: servers are fixed → an attacker (state, insider) compromising a Signal server sees all metadata

In ToM, the relay role is **ephemeral, imposed, and rotating**. The attacker aims at a moving target.

#### 3.8.2 The Sprinkler Gets Sprinkled: Anti-Spam Through Economic Exhaustion

Classical systems handle spam through exclusion: blacklists, CAPTCHAs, rate-limiting, banning. These mechanisms are binary (allowed/blocked) and create censorship power — whoever controls the blacklist controls access.

ToM adopts a radically different approach, inspired by game theory:

```
Normal user:
  [send msg] → [network accepts] → [relay for others] → equilibrium

Spammer (detected by abnormal sending pattern):
  [send msg] → [network accepts] → [BUT: obligation to relay 10x more]
       ↓
  [continue spam] → [obligation to relay 100x more]
       ↓
  [continue further] → [obligation to relay 1000x more]
       ↓
  [the spammer consumes more bandwidth relaying
   than they gain from spamming] → rational abandonment
```

**Why this is superior to banning**:

1. **No censorship**: The spammer is never excluded. They can always send. But each message costs them increasingly more relay work. It's the equivalent of an adaptive proof-of-work — except the "work" is useful relay for the network.

2. **No destructive false positives**: A legitimate user who sends a lot (active group) will see a slight increase in their relay duties, not a ban. The gradient is continuous, not binary.

3. **Self-funding**: The relay work imposed on the spammer *benefits* the network. The attack is transformed into forced contribution. The network literally strengthens through the attacks it endures.

4. **No judge**: No entity decides who is a spammer. The mechanism is purely local and emergent — each node independently adjusts the obligations of its neighbors based on the observed consumption/contribution ratio.

#### 3.8.3 Shattered Bootstrap: The Receptionist Who Changes but Leaves Her Notes

Bootstrap is the Achilles' heel of every decentralized network. Bitcoin has its DNS seeds. Tor has its directory authorities. IPFS has its hardcoded bootstrap nodes. **Each of these fixed points is an attack vector**: compromise the bootstrap, and you control network entry.

ToM treats bootstrap as a living organism:

```
Phase 1 (current PoC):
  A fixed WebSocket server — the "umbilical cord"
  Temporary, documented, marked for elimination

Phase 2 (growth):
  Multiple WebSocket seeds — redundancy
  If a seed falls, the others take over

Phase 3 (alpha — 10-15 nodes):
  DHT begins operating between existing nodes
  Seeds become ordinary nodes
  Bootstrap is no longer a server — it's a question asked to the network

Phase 4 (target):
  Zero fixed infrastructure
  The "phone number" (topic hash) stays the same
  But the "receptionist" who answers changes dynamically
  No one knows who will answer the next call
  If she disappears, the network designates another
  She leaves her "notes" (network state) to her replacement via gossip
```

**The receptionist metaphor** is central: in a classic office, the receptionist is a single point of failure. If she is absent, no one answers. If she is corrupted, she can redirect calls. In ToM, the "receptionist" is a rotating role occupied by a different node at each moment. Corrupting the current receptionist is useless — she will be replaced before the attacker can profit from it. And she has no power: she *introduces* peers to the newcomer, she does not *decide* access.

**Comparison with other bootstraps**:

| Protocol | Bootstrap | Fixed point? | Attackable? |
|----------|-----------|-------------|-------------|
| **Bitcoin** | Hardcoded DNS seeds | Yes (6 domains) | Yes (DNS hijack, BGP) |
| **Tor** | 9 Directory Authorities | Yes (9 known servers) | Yes (authority compromise) |
| **IPFS/libp2p** | Hardcoded bootstrap nodes | Yes (~4 PL servers) | Yes (DoS, compromise) |
| **BitTorrent** | Trackers + DHT bootstrap | Partially (trackers) | Trackers yes, DHT resistant |
| **Nostr** | Relay list in client | Yes (chosen relays) | Yes (malicious relay) |
| **iroh** | n0-computer relays + Pkarr | Partially (n0 relays) | n0 relays = potential SPOF |
| **ToM (target)** | Distributed DHT, rotating role | **No** | **No fixed target** |

#### 3.8.4 Compound Unpredictability: Each Layer Reinforces the Others

The mechanisms described above are not isolated features — they form a **compound unpredictability system**:

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 1: Who will relay my message?                          │
│  → Unpredictable (rotating role based on topology + score)   │
├──────────────────────────────────────────────────────────────┤
│  Layer 2: Who holds the backup of my message?                 │
│  → Unpredictable (viral replication, proactive migration)    │
├──────────────────────────────────────────────────────────────┤
│  Layer 3: Who is the network entry point?                     │
│  → Unpredictable (rotating bootstrap, changed "receptionist")│
├──────────────────────────────────────────────────────────────┤
│  Layer 4: Which subnet will form?                             │
│  → Unpredictable (ephemeral, based on comm patterns)         │
├──────────────────────────────────────────────────────────────┤
│  Layer 5: Which hub will manage my group?                     │
│  → Deterministic BUT changing (first alive in sort)          │
├──────────────────────────────────────────────────────────────┤
│  Layer 6: Who will handle anti-spam?                          │
│  → No one — it's emergent (each node adjusts locally)        │
└──────────────────────────────────────────────────────────────┘
```

**An attacker would need to simultaneously**:
1. Position itself as Alice's relay (unpredictable)
2. Control the message's backup nodes (unpredictable)
3. Control bootstrap to prevent Bob from joining (unpredictable)
4. Be in the same ephemeral subnet (impossible to force)
5. Become the group hub (deterministic but changing)
6. Not get "sprinkled" by anti-spam (impossible if active)

Each layer is independently hard to predict. Combined, they create a **moving attack surface** where no static strategy works. The attacker is condemned to play a game whose rules change every turn.

This is the distributed analog of Kerckhoffs's principle: security does not rely on the secrecy of the algorithm (which is open source), but on the unpredictability of the network state at each moment.

---

## 4. Experimental Validation

### 4.1 Test Protocol

**Infrastructure**:
- Sender: MacBook Pro (macOS, x86_64)
- Receiver: Freebox Delta NAS (Debian ARM64, Cortex-A72 Armada 8040)
- Binary: `tom-stress` cross-compiled via `cargo-zigbuild` (target `aarch64-unknown-linux-musl`, static binary)
- Protocol: ping/pong MessagePack signed Ed25519 over QUIC via iroh

### 4.2 Campaign 1 — Connection Eviction (February 12)

| Metric | Value |
|--------|-------|
| Pings sent | 20 |
| Pongs received | 0 |
| Reliability | **0%** |

**Bug #1**: The connection pool does not detect NAT reassignment. QUIC reports the connection as alive (`close_reason().is_none() == true`) while the NAT has changed the mapping address. The pong is sent on a dead path.

**Fix**: Connection eviction on `open_bi()` error — force rediscovery.

### 4.3 Campaign 2 — Zombie Detection (February 13)

| Metric | Value |
|--------|-------|
| Campaigns | 7 |
| Overall reliability | **97%** |

**Bug #2**: Zombie connections — `send()` succeeds (the QUIC buffer accepts data) but the pong never returns. The passive side does not trigger reconnection on silent failure.

**Fix**: Consecutive timeout tracking. After 3 timeouts without response → forced eviction.

### 4.4 Campaign 3 — Highway Mobility (February 16)

| Metric | Session 1 | Session 2 | Total |
|--------|-----------|-----------|-------|
| Duration | 32 min | 22 min | 54 min |
| Pings | 1,640 | 1,112 | **2,752** |
| Pongs | 1,638 | 1,110 | **2,748** |
| Reliability | 99.88% | 99.82% | **99.85%** |
| Avg RTT | 1.26 ms | 9.7 ms | — |
| Max reconnection | — | 52 s | — |

**Conditions**: A40 highway France↔Switzerland, 4G mobile network, tunnel crossings, cell tower handoffs.

**Observation**: The longest reconnection (52 s) corresponds to a tunnel crossing. The system recovers automatically without intervention. The average RTT of 1.26 ms confirms that the majority of exchanges go through direct QUIC connection (not relayed).

### 4.5 Campaign 4 — Keepalive and Pkarr (February 17)

| Metric | Session 7 | Session 9 |
|--------|-----------|-----------|
| Pings | 1,203 | 402 |
| Pongs | 1,198 | 400 |
| Reliability | 99.58% | 99.50% |
| Avg RTT | 1.57 ms | 0.98 ms |
| Network | WiFi → 4G (border crossing) | Urban 5G |

**Bug #4**: The passive listener (the NAS) has no keepalive mechanism. After ~30 minutes, the Pkarr record expires and the iroh relay dereferences it. Incoming connections fail.

**Analysis**: This bug does not require a transport-level fix — the gossip discovery protocol (layer 3) will naturally send heartbeats that maintain the node's presence. This is a problem solved by architecture, not by a patch.

**Notable result**: RTT of 0.98 ms in session 9 (5G) — sub-millisecond latency for cross-NAT P2P messaging.

### 4.6 NAT Traversal — Hole Punching

| Scenario | NAT Topology | Upgrade time | Direct RTT | Direct rate |
|----------|--------------|--------------|-----------|-------------|
| LAN WiFi | Same network | 0.37 s | 49 ms | 100% |
| 4G CGNAT | iPhone hotspot ↔ Home WiFi | 2.9 s | 107 ms | 90% |
| Cross-border | School CH ↔ Freebox FR | 1.4 s | 32 ms | 95% |

**100% hole punching success** across all 3 scenarios. The most constrained scenario (4G carrier CGNAT) achieves 90% direct connections with an upgrade time of 2.9 s. The iroh relay (`euc1-1.relay.n0.iroh-canary.iroh.link`) only serves for the initial discovery phase.

### 4.7 Cross-Border Validation with ProtocolRuntime (February 19)

| Parameter | Value |
|-----------|-------|
| Sender | MacBook, Nomades school (Switzerland), guest WiFi |
| Receiver | Freebox NAS (France, 82.67.95.8) |
| Protocol | Complete ProtocolRuntime (Router + Topology + Tracker) |
| Encryption | E2E XChaCha20-Poly1305 enabled |
| Signature | Ed25519 on each envelope |
| Direct upgrade | **27 ms** |
| Simultaneous clients | 2 TUI → 1 NAS bot, both reached the bot |

This test validates the complete stack: QUIC transport + ToM protocol + E2E encryption + signature + routing, in real cross-border conditions.

---

## 5. Discussion

### 5.1 Comparison with Blockchain Promises

ToM shares properties with blockchains (cryptographic identity, absence of trusted third party, censorship resistance) while avoiding their limitations:

| Property | Blockchain | ToM |
|----------|-----------|-----|
| Identity | Public key = address | Public key = address (**identical**) |
| Consensus | Raft/PBFT/PoW/PoS | **None** — deterministic ordering + TTL |
| Persistence | Immutable, infinite | **24h max then purge** |
| Scaling | Limited by consensus | **Inverted** — more nodes = faster |
| Cost | Gas/fees | **Zero** — contribution in kind |
| Finality | Waiting for confirmation | **Immediate ACK from recipient** |

The elimination of consensus is the most radical choice. In a messaging protocol, consensus is superfluous: a message is either delivered (ACK), or lost after 24h. There is no global state to synchronize, no double-spend to prevent. This simplification eliminates the most expensive complexity of distributed systems.

### 5.2 Security Analysis: Attacks and Countermeasures

#### 5.2.1 Man-in-the-Middle (MITM) Attack at the Relay Level

**Attack scenario**: A malicious relay Mallory positions itself between Alice and Bob. Mallory intercepts envelopes and attempts to:
- (a) read message content, or
- (b) modify messages in transit, or
- (c) impersonate Alice to Bob

**Countermeasure (a) — Confidentiality**: The relay sees `EncryptedPayload = {ciphertext, nonce, ephemeral_pk}`. To decrypt, it would need to compute:

```
shared_secret = bob_x25519_secret × eph_public
```

Mallory possesses `eph_public` (transmitted in cleartext) and `bob_ed25519_pk` (known to the network), but **neither** `eph_secret` (destroyed after DH on Alice's side) **nor** `bob_secret` (never transmitted). Reconstructing the shared secret amounts to solving the discrete logarithm problem on Curve25519 — complexity ~2^128 operations (128 bits of security). Infeasible.

**Countermeasure (b) — Integrity**: Two independent layers protect integrity.

*Layer 1 — Ed25519 Signature (verifiable by all)*: The signature covers `{id, from, to, via, msg_type, payload, timestamp, encrypted}`. If Mallory modifies a single byte of the ciphertext, the signature becomes invalid. The next node (or Bob) rejects the envelope with `TomProtocolError::InvalidSignature`. Mallory cannot re-sign because it would require Alice's private key.

*Layer 2 — Poly1305 Tag (verifiable only by Bob)*: Even if Mallory found a way to circumvent the signature (absurd hypothesis — this would imply breaking Ed25519), the Poly1305 authentication tag (16 bytes) embedded in the ciphertext would fail during decryption. XChaCha20-Poly1305 is an AEAD scheme (*Authenticated Encryption with Associated Data*): any modification of the ciphertext, even by one bit, produces an authentication error.

**Countermeasure (c) — Identity impersonation**: To send a message "on behalf of Alice", the attacker must produce a valid Ed25519 signature with Alice's private key. Without this key, it is impossible — Ed25519 offers 128 bits of security against existential forgery (EUF-CMA).

#### 5.2.2 Active MITM: Interception with Key Substitution

The most dangerous classic MITM attack against Diffie-Hellman is **active interception**: Mallory intercepts Alice's ephemeral key, replaces it with their own, does the same on Bob's side, and ends up with two separate DH sessions — decrypting and re-encrypting each message.

**Why this attack fails in ToM**:

1. **The ephemeral key is inside the signed payload**: The `EncryptedPayload` (containing `ephemeral_pk`) is serialized in MessagePack and placed in `envelope.payload`. This payload is covered by Alice's Ed25519 signature. If Mallory replaces `ephemeral_pk` with their own key, the signature becomes invalid.

2. **The recipient's public key is bound to the DH**: Alice computes `DH(eph_secret, bob_x25519_pk)`. If Mallory substitutes `bob_pk` with `mallory_pk` in the `to` field, they would also need to re-sign → impossible without `alice_secret`.

3. **Trust-on-First-Use (TOFU)**: Ed25519 public keys are network identifiers. When Alice knows Bob's public key (via gossip, QR code, out-of-band exchange), she encrypts specifically for that key. Mallory cannot substitute Bob's public key without Alice noticing — because Bob's `NodeId` *is* his public key.

**Acknowledged limitation**: If Alice has never communicated with Bob and obtains his public key via a network entirely controlled by Mallory from the start, Mallory could provide their own key while impersonating Bob. This is the fundamental problem of initial key distribution — no protocol solves it without an out-of-band channel (QR code, voice verification, centralized PKI). Signal solves this problem with "safety numbers" verifiable in person. ToM could implement a similar mechanism.

#### 5.2.3 The 51% Attack: Is It Relevant for ToM?

In PoW blockchains, controlling >50% of computing power allows rewriting history (double-spend). In PoS blockchains, controlling >50% of stake allows validating fraudulent transactions. **The question: can an attacker controlling >50% of ToM nodes compromise the network?**

**Short answer: the 51% attack makes no sense in ToM, because there is no consensus to corrupt.**

**Detailed analysis** — What could an attacker controlling 51% of nodes do?

| Attacker's objective | Feasibility | Reason |
|---------------------|-------------|--------|
| Read messages in transit | No | E2E XChaCha20 — malicious nodes are blind relays |
| Modify messages | No | Ed25519 signature + Poly1305 tag — any alteration is detected |
| Prevent delivery (censorship) | **Partially** | Can drop messages as relay, but ToM uses alternative paths and viral replication |
| Impersonate an identity | No | Requires the victim's Ed25519 private key |
| Rewrite history | N/A | There is no history — 24h TTL, unconditional purge |
| Double-spend / double-deliver | N/A | ACKs are idempotent — receiving a message twice is harmless |
| Corrupt consensus | N/A | **There is no consensus** — no vote, no quorum |

**The only real vector: selective censorship (message dropping)**. If 51% of relays are malicious, a message has ~50% chance of traversing an honest relay at each hop. With a TTL of 4 hops and viral replication across 3-5 backup nodes:

```
P(delivery) = 1 - P(all paths blocked)
            = 1 - (0.51)^(nb_independent_paths)

With 3 backup replicas and 2 alternative paths per replica:
P(delivery) ≈ 1 - (0.51)^6 ≈ 98.2%
```

Even with 51% malicious nodes, viral replication and multiple paths maintain a high delivery probability. And unlike a blockchain, the attacker gains nothing — there are no tokens to steal, no history to rewrite, no consensus to corrupt. **The attack cost is high (maintaining 51% of nodes) and the gain is near-zero (delaying a few messages by a few seconds).**

**Comparison with blockchains**:

| Property | Blockchain (51%) | ToM (51%) |
|----------|------------------|-----------|
| Motivation | Financial theft (double-spend) | **None** — no value to extract |
| Impact | History rewriting | **Partial and temporary censorship** |
| Attack duration | Permanent if maintained | **Max 24h** — TTL purges everything |
| Defense | Increase hashrate/stake | **Viral replication + alternative paths** |
| Post-attack state | Loss of confidence, fork | **No permanent damage** |

The absence of consensus, global state, and financial value makes the 51% attack economically irrational against ToM. This is a structural advantage of ephemeral messaging protocols over permanent ledgers.

#### 5.2.4 Other Security Properties

**Per-message forward secrecy**: Each message uses a fresh ephemeral X25519 pair. Compromising one message key compromises neither past nor future messages. This is stronger than the per-session forward secrecy of classic TLS (where compromising a session key exposes the entire session).

**Metadata resistance**: Relays see `from` and `to` (needed for routing) but content is encrypted. Ephemeral subnets reduce the number of intermediary relays, decreasing the exposure surface. Limitation: a global network observer could correlate temporal patterns (traffic analysis). Onion routing is not implemented in the current PoC.

**Censorship-free anti-spam**: The "sprinkler gets sprinkled" mechanism progressively increases the workload of abusers without excluding them. No blocking, no ban, no binary threshold — abuse simply becomes economically irrational. This is the analog of Bitcoin's proof-of-work, but applied to spam instead of consensus.

**Structural right to be forgotten**: The 24h TTL and unconditional purge guarantee that no message persists beyond its delivery window. Unlike GDPR (right to be forgotten *on request*), ToM implements a *structural* right to be forgotten — erasure is a protocol mechanism, not an administrative policy.

### 5.3 Current Limitations and Open Attack Vectors

1. **Scale not validated beyond 1:1** — Stress tests cover a sender-receiver topology. Behavior at 15+ simultaneous nodes remains to be validated.
2. **Temporary bootstrap** — The PoC still uses iroh relays for initial discovery. Complete elimination of fixed bootstrap requires DHT implementation.
3. **Traffic analysis** — A passive observer on the network can correlate temporal patterns (message sizes, timing). Onion routing (Tor-style) is not implemented. This is the most realistic attack vector against privacy.
4. **Sybil attack** — An attacker massively creating identities (free: one Ed25519 key pair = one node) could inflate their presence in the network. Planned countermeasure: the contribution score makes freshly created nodes minimally influential (no relay/backup role assigned without history).
5. **Eclipse attack** — An attacker surrounding a target node with their own nodes could isolate it. Countermeasure: HyParView gossip maintains random neighbors in addition to active neighbors, making encirclement difficult.
6. **Dalek version conflict** — The coexistence of `ed25519-dalek 2.x` (ToM) and `3.0.0-pre.1` (iroh) works via byte-level conversion but is fragile. Resolved by the planned strategic fork.

### 5.4 Positioning Against Existing P2P Protocols

Section 2 presented a synthetic state of the art. After experimental results and security analysis, we can now position ToM more precisely against each protocol family.

#### 5.4.1 ToM vs BitTorrent: Mandatory vs Optional Contribution

BitTorrent has proven that P2P works at scale (hundreds of millions of users). But its economic model relies on **goodwill**: seeding is voluntary. Result: seed/leech ratios are often catastrophic (<10% seeders). Incentive mechanisms (tit-for-tat, ratio tracking) are circumventable.

| Aspect | BitTorrent | ToM |
|--------|-----------|-----|
| Contribution | Voluntary (seeding) | **Imposed** (relay/backup assigned by the network) |
| Free-riding | Endemic (leechers) | **Structurally impossible** — no "pure consumer" role |
| Incentive | Ratio tracking (circumventable) | **Contribution score** (based on observed behavior, not declared) |
| Node role | Chosen (seed/leech) | **Assigned** (client/relay/backup — rotating) |
| Shared data | Files (persistent) | **Messages (ephemeral, 24h TTL)** |
| Censorship resistance | Partial (central trackers) | **Strong** (no tracker, no fixed point) |

**ToM's insight**: BitTorrent treats contribution as a social problem (incentivizing people to share). ToM treats it as an architectural problem (making non-sharing impossible).

#### 5.4.2 ToM vs Tor: Anonymity vs Unpredictability

Tor offers anonymity through onion routing: 3 successive relays, each knowing only the previous and next. It is the reference for privacy protection. But Tor has structural weaknesses that ToM avoids.

| Aspect | Tor | ToM |
|--------|-----|-----|
| Primary objective | Anonymity (hiding who talks to whom) | **Decentralization** (removing infrastructure) |
| Relays | Voluntary, permanent, publicly listed | **Imposed, rotating, unpredictable** |
| Directory Authorities | 9 fixed servers (critical SPOF) | **None** — rotating bootstrap |
| Attack on relays | Malicious operator = entry/exit correlation | **Ephemeral role** — no time to correlate |
| Performance | Slow (3 cryptographic hops) | **Fast** (1-2 hops, 27ms cross-border measured) |
| Metadata | Hidden (onion routing) | **Partially visible** (from/to in cleartext for routing) |
| Censorship resistance | DPI circumventable (pluggable transports) | **Native NAT traversal** (QUIC hole punch) |

**What Tor does better**: pure anonymity. ToM does not hide who talks to whom — the `from` and `to` fields are in cleartext (needed for routing). Future onion routing is conceivable but is not the primary objective.

**What ToM does better**: resilience. Tor depends on 9 Directory Authorities — compromising 5 of them compromises the entire network. ToM has no equivalent to compromise. Tor relays are permanent servers operated by identifiable volunteers — ToM "relays" are ordinary devices whose role constantly changes.

#### 5.4.3 ToM vs Nostr: Imposed vs Voluntary Relays

Nostr is the most recent decentralized protocol to have gained traction (2023-2024). Its model is elegantly simple: clients publish signed events (NIP-01) to relays, which store and redistribute them. But this simplicity hides structural problems.

| Aspect | Nostr | ToM |
|--------|-------|-----|
| Relays | Voluntary, chosen by user | **Imposed by the network, rotating** |
| Relay funding | Operator pays (donations, subscriptions) | **No operator** — each node contributes automatically |
| Storage | Indefinite (relay stores everything) | **24h max** then purge — no storage debt |
| Encryption | No (by default, NIP-04 optional and broken) | **Yes** — E2E XChaCha20 by default |
| Censorship | Relay can filter events | **No entity can filter** — rotating roles |
| Identity | nsec/npub (Schnorr/secp256k1) | Ed25519 (same principle, different curve) |
| Scalability | Limited by relay cost | **Inverted** — more nodes = more capacity |
| Resilience | If your relay goes down, your data disappears | **Viral replication** — messages proactively migrate |

**Nostr's fundamental problem**: someone must pay for the relays. It's the same problem as central servers, distributed instead of centralized. A popular Nostr relay costs thousands of euros per month in bandwidth and storage. The protocol provides no compensation mechanism — it's goodwill, like BitTorrent seeding.

**ToM eliminates the problem**: there are no "relays to maintain". Each connected device is automatically a relay when the network demands it. The relay cost is each participant's residual bandwidth — invisible and distributed. This is the difference between an economic system based on volunteerism (fragile) and a system based on mandatory mutualization (antifragile).

#### 5.4.4 ToM vs libp2p / Hyperswarm / iroh: Transport Layer vs Protocol Layer

libp2p, Hyperswarm and iroh are **transport layers** — they solve connectivity between nodes. ToM is a **protocol layer** — it defines what nodes do once connected. The relevant comparison is not "which is better" but "at what level each operates".

```
┌─────────────────────────────────────────────────────────┐
│  Application (Chat, Game, Collaboration...)               │
├─────────────────────────────────────────────────────────┤
│  ToM Protocol                                            │  ← What ToM adds
│  (Roles, Groups, Viral backup, Anti-spam, Routing)       │
├─────────────────────────────────────────────────────────┤
│  Transport (iroh / libp2p / Hyperswarm)                 │  ← What they do
│  (Connectivity, NAT traversal, Multiplexing)            │
├─────────────────────────────────────────────────────────┤
│  Network (QUIC / TCP / UDP / WebRTC)                     │
└─────────────────────────────────────────────────────────┘
```

| Aspect | libp2p | Hyperswarm | iroh | **ToM** |
|--------|--------|-----------|------|---------|
| Level | Transport | Transport | Transport | **Application protocol** |
| Node roles | Undifferentiated | Undifferentiated | Undifferentiated | **Dynamic (client/relay/backup)** |
| Anti-spam policy | None | None | None | **"Sprinkler gets sprinkled"** |
| Group messaging | To implement | To implement | To implement | **Built-in hub-and-spoke** |
| Offline backup | To implement | To implement | To implement | **Built-in viral replication** |
| Contribution scoring | None | None | None | **Consumption/service score** |
| Application E2E | App's choice | SecretStream | QUIC TLS | **XChaCha20-Poly1305 + signatures** |
| Bootstrap | Hardcoded nodes | DHT bootstrap | n0 relays + Pkarr | **Rotating, no fixed point (target)** |

**Why ToM uses iroh and not libp2p**:
- iroh treats relays as stateless pass-through — aligned with the ToM philosophy
- libp2p treats relays as a NAT workaround — opposite philosophy
- iroh has a ~90% direct connection rate in production (efficient hole punch)
- libp2p prioritizes multi-transport compatibility at the expense of NAT performance

**Why ToM is not just "iroh + some code"**:
iroh solves *how to connect* two nodes. ToM solves *what to do* once connected: who relays what, how messages survive the recipient's absence, how to prevent spam without censorship, how to form groups without a server. These are two complementary layers, not competing ones.

#### 5.4.5 Synthesis: What ToM Does That No One Else Does

| Property | BitTorrent | Tor | Nostr | libp2p | iroh | **ToM** |
|----------|-----------|-----|-------|--------|------|---------|
| Mandatory contribution | No | No | No | No | No | **Yes** |
| Unpredictable rotating roles | No | No | No | No | No | **Yes** |
| Censorship-free anti-spam | No | No | No | No | No | **Yes** |
| Bootstrap without fixed point | No | No | No | No | No | **Yes (target)** |
| Self-replicating messages | No | No | No | No | No | **Yes** |
| Unconditional purge (TTL) | No | No | No | No | No | **Yes** |
| Economic scaling inversion | Partial | No | No | No | No | **Yes** |
| E2E with per-message forward secrecy | N/A | Yes | No | App | Transport | **Yes** |

None of these protocols combine these properties. Some possess one or two, none integrate them all into a coherent system. ToM's innovation is not in the primitives (DH, signatures, gossip have existed for decades) but in their **architectural composition**: each mechanism reinforces the others, and the whole creates emergent properties (compound unpredictability, economic inversion, structural right to be forgotten) that no isolated component possesses.

### 5.5 Toward an Unstoppable Network

#### 5.5.1 Source Code Self-Hosting

Most open source projects depend on a centralized platform (GitHub, GitLab) for their source code. Even Bitcoin, "the unstoppable network", has its source code on github.com/bitcoin/bitcoin — a server controlled by Microsoft. A court order, a corporate decision, or a targeted attack could make the code temporarily inaccessible.

ToM plans a mechanism for **radical self-hosting**:

```
Current phase:
  Source code → GitHub (centralized)
  Documentation → GitHub (centralized)
  Issues/PRs → GitHub (centralized)

Target phase:
  Source code → distributed on the ToM network itself
  Documentation → distributed on ToM
  Dev workflow → distributed on ToM
  GitHub → optional mirror, no longer necessary
```

The protocol hosts the code that makes it run. The network distributes the protocol updates that make the network run. This is an **existential bootstrap**: the system becomes its own development infrastructure.

**Comparison**: IPFS hosts files in a distributed manner, but IPFS itself does not self-host (its code is on GitHub). Tor distributes traffic, but the directory authorities and source code are centralized. ToM aims for the next step: **the code IS the network, the network HOSTS the code**.

#### 5.5.2 Progressive Elimination of the Umbilical Cord

The original whitepaper uses the **umbilical cord** metaphor to describe bootstrap:

```
Birth (PoC):
  The network depends on a fixed WebSocket server for signaling
  → It's the umbilical cord: vital, but temporary

Growth:
  Multiple WebSocket seeds — redundancy
  DHT begins operating between existing nodes
  Seeds become ordinary nodes

Autonomy:
  The network discovers its own peers via gossip + DHT
  The "phone number" (topic hash) stays the same
  But the "receptionist" who answers changes at each call
  If she disappears, the network designates another
  She leaves her "notes" to her replacement via gossip

Maturity:
  Zero fixed infrastructure
  The cord is cut
  The baby breathes on its own
```

Each phase eliminates a dependency. The final phase depends on nothing — no server, no DNS domain, no company, no cloud infrastructure. The network IS the infrastructure. The only way to stop it would be to simultaneously turn off all devices of all participants in all countries — that is, in practice, impossible.

#### 5.5.3 Properties of a Network That Cannot Be Killed

| Property | How it makes the network unstoppable |
|----------|--------------------------------------|
| **No central server** | Nothing to seize, nothing to unplug |
| **No DNS domain** | No DNS to block (discovery via Pkarr + gossip) |
| **No company** | No one to sue, no competent jurisdiction |
| **No financial token** | No speculation incentive, no exchange to regulate |
| **Self-hosted code** | If GitHub goes down, the network distributes its own code |
| **Identity = crypto key** | No identity registry to compromise |
| **Rotating bootstrap** | No fixed entry point to attack |
| **E2E encryption** | Even intercepting traffic, content is unreadable |
| **24h TTL** | No persistent data to seize or analyze |
| **Mandatory contribution** | Each participant strengthens the network (no freeloading) |

**What could still kill ToM** (intellectual honesty):
- **Insufficient critical mass**: if the network never reaches enough nodes, bootstrap remains necessary
- **Widespread DPI**: a state blocking all unidentified QUIC traffic could hinder connections (circumventable by obfuscation, like Tor with pluggable transports)
- **Disinterest**: if nobody uses the network, it dies. The 24h TTL guarantees that a dead network leaves no ghosts

### 5.6 Methodology: From Whitepaper to Working Code

#### 5.6.1 From Idea to PoC in 3 Weeks

The ToM project followed a structured methodology (BMAD) assisted by AI, in 4 phases:

```
Phase 1 — Vision (January 2026)
  Whitepaper v1 → Product Brief → PRD → Architecture → Design Decisions (7 locked)
  Result: 45 functional requirements, 14 non-functional, 9 ADRs

Phase 2 — TypeScript Prototype (January-February 2026)
  8 Epics → 20 Stories → 771 passing tests
  WebRTC DataChannel, WebSocket signaling, E2E TweetNaCl.js
  Result: functional multi-node chat in the browser

Phase 3 — Rust Port + iroh (February 2026)
  NAT traversal evaluation → iroh choice → 4 PoCs → 4 stress test campaigns
  tom-transport (QUIC pool) + tom-protocol (envelopes, groups, backup, discovery)
  Result: 237 Rust tests, 99.85% reliability on highway, E2E cross-border validated

Phase 4 — ProtocolRuntime + TUI (February 2026)
  Complete integration → tokio::select! event loop
  tom-chat (ratatui TUI + headless bot mode)
  Result: 2 simultaneous clients Mac ↔ ARM64 NAS cross-border CH↔FR
```

**The rigor of the approach is intentional**: every architectural decision is documented *before* implementation. The 7 locked decisions were defined on day 1 and have never been modified — the code built around them, not the reverse. This is the opposite approach to "move fast and break things": here, foundations are laid slowly and don't move again.

#### 5.6.2 Two Implementations, One Protocol

Having two complete implementations (TypeScript + Rust) of the same protocol is a validation in itself:

| Property | Phase 1 (TypeScript) | Phase 2 (Rust) |
|----------|---------------------|----------------|
| Transport | WebRTC DataChannel | QUIC (iroh) |
| Crypto | TweetNaCl.js (NaCl) | ed25519-dalek + XChaCha20 |
| Serialization | JSON | MessagePack |
| Runtime | Browser + Node.js | Tokio (native) |
| Tests | 771 | 237 |
| Target | Browser proof of concept | Real network validation |

Both implementations respect the same 7 locked decisions, the same envelope format (adapted to the serializer), and the same routing principles. The protocol survives language change — proof that it is well-defined at the conceptual level, not at the code level.

---

## 6. Implementation

### 6.1 Technical Stack

| Component | Technology | Justification |
|-----------|-----------|---------------|
| Language | Rust | Memory safety, performance, ARM cross-compilation |
| Transport | QUIC (via iroh) | Multiplexing, 0-RTT, connection migration |
| Serialization | MessagePack (rmp-serde) | Compact, deterministic, schema-less |
| Signing | Ed25519 (ed25519-dalek 2.x) | Standard, fast, short keys (32 bytes) |
| Key exchange | X25519 (x25519-dalek 2.x) | DH on Curve25519, ephemeral per message |
| AEAD | XChaCha20-Poly1305 | 192-bit nonce, no AES-NI needed |
| KDF | HKDF-SHA256 | Domain separation, extraction+expansion |
| Runtime | Tokio | Async I/O, select! for concurrency without mutex |
| Cross-compile | cargo-zigbuild | Static musl binaries for ARM64 |

### 6.2 Code Metrics

| Metric | Value |
|--------|-------|
| TypeScript tests (Phase 1 — WebRTC) | 771 |
| Rust tests (Phase 2 — native QUIC) | 237 |
| Total | **1,008** |
| Group integration tests | 4 |
| Discovery integration tests | 6 |
| Backup integration tests | 7 |
| E2E transport tests | 2 |
| Supported message types | 24 |

### 6.3 Implementation Lessons

**Never wrap `TomNode` in `Arc<Mutex>`**: `recv_raw(&mut self)` holds the lock across an `.await`, completely blocking the sender. Solution: a single Tokio task with `select!` for send/recv concurrency.

**The signature must exclude TTL**: Relays decrement TTL in transit. Including TTL in the signed bytes invalidates the signature after the first hop.

**Pkarr rediscovery**: On failed reconnection, force a Pkarr rediscovery every 5 attempts. Exponential backoff alone is not sufficient if the DNS record has expired.

---

## 7. Conclusion

### 7.1 Results

ToM demonstrates the feasibility of a peer-to-peer messaging protocol without fixed infrastructure. Experimental results validate each layer:

- **Transport**: 99.85% reliability over 2,752 pings during highway mobility (A40, tunnels, 4G handoffs)
- **NAT traversal**: 100% hole punch success across 3 topologies (LAN, 4G CGNAT, cross-border CH↔FR)
- **Latency**: 27 ms cross-border Switzerland↔France after direct upgrade, 0.98 ms on urban 5G
- **Encryption**: E2E XChaCha20-Poly1305 validated with per-message forward secrecy and Ed25519 signatures
- **Cross-compilation**: single binary x86_64 + ARM64, verified Mac ↔ Freebox NAS

### 7.2 What Is New

ToM's innovation does not lie in its primitives — Diffie-Hellman, gossip, Ed25519 signatures have existed for decades. It lies in their **composition**:

- **Compound unpredictability** (rotating roles + shattered bootstrap + ephemeral subnets) creates a moving attack surface that no static strategy can target
- **Economic inversion** (more nodes = faster = cheaper) is an emergent property of the architecture, not an optimization
- **Structural right to be forgotten** (24h TTL + unconditional purge) eliminates storage debt and makes mass surveillance impractical
- **Mandatory contribution** (imposed roles, not voluntary) solves the free-riding problem that undermines BitTorrent and Nostr
- **Censorship-free anti-spam** ("the sprinkler gets sprinkled") transforms attacks into forced network contribution

No existing protocol — BitTorrent, Tor, Nostr, libp2p, iroh, Matrix — combines these properties. Some possess one or two. ToM integrates them all into a coherent system where each mechanism reinforces the others.

### 7.3 What Remains to Be Done

1. **Scale validation**: stress tests cover a 1:1 topology. Behavior at 15+ simultaneous nodes, with real rotating roles and active viral replication, remains to be measured
2. **Bootstrap elimination**: distributed DHT to replace WebSocket signaling — cut the umbilical cord
3. **Onion routing**: protection against traffic analysis (the `from`/`to` fields are in cleartext)
4. **Self-hosting**: distribute the source code, documentation, and development workflow on ToM itself
5. **Cryptographic audit**: formal validation of the XChaCha20-Poly1305 + HKDF pipeline by an independent third party

### 7.4 Vision

ToM's success is not measured in metrics. It is measured in a **state**:

- A message travels from A to B without a server, without an intermediary, without fees, without a trace
- The code evolves without any entity controlling it
- The network maintains itself without anyone maintaining it
- No one knows they are using ToM — and that's exactly why it works
- Once launched, the only thing that can happen to it is evolution

> *"A network that belongs to no one because it belongs to everyone. A network that depends on nothing because it is self-sufficient. A network that cannot be attacked because there is nothing to steal. A network where you don't know you're participating — and that's exactly why it works."*
> — ToM Whitepaper v1

---

## References

1. Perrin, T., Marlinspike, M. "The Double Ratchet Algorithm." Signal Foundation, 2016.
2. Leitão, J., Pereira, J., Rodrigues, L. "HyParView: a membership protocol for reliable gossip-based broadcast." IEEE/IFIP DSN, 2007.
3. Bernstein, D.J. "Curve25519: new Diffie-Hellman speed records." PKC 2006.
4. Bernstein, D.J. "ChaCha, a variant of Salsa20." 2008.
5. Krawczyk, H. "Cryptographic Extraction and Key Derivation: The HKDF Scheme." CRYPTO 2010.
6. Iyengar, J., Thomson, M. "QUIC: A UDP-Based Multiplexed and Secure Transport." RFC 9000, 2021.
7. iroh documentation, n0-computer, 2025. https://iroh.computer/docs
8. Arcieri, T. et al. "ed25519-dalek: Fast Ed25519 signing in Rust." GitHub, 2024.
9. Ford, B., Srisuresh, P., Kegel, D. "Peer-to-Peer Communication Across Network Address Translators." USENIX ATC, 2005.
10. Joux, A. "Authentication Failures in NIST version of GCM." Comments on NIST Proposal, 2006.
11. Langley, A., Hamburg, M., Turner, S. "Elliptic Curves for Security." RFC 7748, 2016.
12. Douceur, J.R. "The Sybil Attack." IPTPS, 2002.
13. Heilman, E. et al. "Eclipse Attacks on Bitcoin's Peer-to-Peer Network." USENIX Security, 2015.
14. Diffie, W., Hellman, M. "New Directions in Cryptography." IEEE Transactions on Information Theory, 1976.

---

*Source code: https://github.com/malikkaraoui/ToM-protocol/ — Branches: `main` (TypeScript Phase 1), `feat/tom-protocol` (Rust Phase 2)*

*1,008 passing tests. 4 stress test campaigns. 3 validated NAT scenarios. 0 servers required.*
