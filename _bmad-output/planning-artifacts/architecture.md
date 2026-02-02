---
stepsCompleted: [step-01-init, step-02-context, step-03-starter, step-04-decisions, step-05-patterns, step-06-structure, step-07-validation, step-08-complete]
status: 'complete'
completedAt: '2026-02-02'
inputDocuments:
  - prd.md
  - product-brief-tom-protocol-2026-01-30.md
  - tom-whitepaper-v1.md
  - prd-validation-report.md
workflowType: 'architecture'
project_name: 'tom-protocol'
user_name: 'Malik'
date: '2026-02-02'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
45 FRs across 8 capability areas. Core protocol (FR1-10) defines relay-based message transport with progressive features: known relay → dynamic routing → bidirectional → multi-relay → E2E encryption. UX layer (FR11-15) provides demo interface. Network participation (FR16-20) establishes dual client/relay node model. Bootstrap (FR21-24) handles temporary signaling. SDK (FR25-30) exposes two integration levels. LLM tooling (FR31-35) enables AI-first distribution. Community (FR36-40) supports micro-session contribution. Economy (FR41-45) tracks contribution/usage equilibrium.

**Non-Functional Requirements:**
14 NFRs drive architectural decisions: <500ms message delivery and <3s peer discovery set performance floor. Relay statelessness (pass-through only) eliminates server-side storage. 24h max backup with multi-node redundancy then delete defines temporary persistence model. Mandatory E2E from iteration 5 requires crypto pipeline from day one in design. Inversion property (more nodes = faster) is the core architectural differentiator — architecture must enable this, not merely tolerate it.

**Scale & Complexity:**
- Primary domain: distributed networking / protocol SDK
- Complexity level: high — novel protocol with P2P transport, dynamic role assignment, cryptographic pipeline, contribution economy
- Estimated architectural components: ~8 (transport layer, signaling/bootstrap, routing engine, crypto pipeline, node manager/role assignment, contribution tracker, SDK abstraction, demo UI)

### Technical Constraints & Dependencies

- **Browser-first**: WebRTC as primary transport — no native UDP sockets in browsers
- **Holepunch stack**: HyperDHT + Hyperswarm for NAT traversal (native only). WebRTC ↔ UDX bridge required for browser nodes.
- **No central infrastructure**: Architecture must function with zero servers post-bootstrap. Bootstrap is temporary, isolated, documented.
- **TypeScript/JavaScript first**: PoC targets browsers and Node.js. Protocol design must remain language-agnostic.
- **Progressive iteration model**: Each layer builds on the previous. Architecture must support incremental capability addition without refactoring core.
- **MIT License**: No dependency on restrictively-licensed libraries.

### Cross-Cutting Concerns Identified

- **NAT traversal**: Affects transport, discovery, bootstrap, and relay selection. Holepunch modules as foundation, WebRTC bridge as browser adaptation.
- **Encryption pipeline**: Touches transport, relay logic (must not read content), backup storage (encrypted at rest), and SDK abstraction (transparent to developer).
- **Node identity & role assignment**: Spans network participation, contribution tracking, relay selection, and reconnection handling.
- **Statelessness & purge**: Affects relay design (no persistence), backup design (24h TTL), subnet lifecycle (sliding genesis), and overall memory management.
- **Dual client/relay nature**: Every node must handle both sending/receiving and forwarding. No architectural separation between "client nodes" and "server nodes."

## Starter Template Evaluation

### Primary Technology Domain

Protocol SDK — not a web application. No starter template applies. Manual scaffold, LLM-friendly.

### Technical Preferences

- **Language:** TypeScript 5.x (strict mode)
- **Runtime:** Browser (WebRTC) + Node.js
- **Framework:** None — vanilla HTML/JS for demo. Zero framework dependency.
- **Build:** tsup (library bundling, ESM + CJS output)
- **Tests:** vitest (fast, TypeScript-native)
- **Lint/Format:** biome (single tool, fast, replaces ESLint + Prettier)
- **Package manager:** pnpm (workspaces support, strict dependency resolution)
- **License:** MIT

### Selected Approach: pnpm Monorepo

**Rationale:** Three packages share protocol types and need coordinated versioning. Monorepo from day one prevents painful extraction later. pnpm workspaces are minimal — no Nx/Turborepo overhead.

**Structure:**

```
tom-protocol/
├── packages/
│   ├── core/             # Raw protocol primitives (connect, send, receive, roles)
│   └── sdk/              # Plug-and-play abstraction (TomClient.create, auto-relay, auto-encrypt)
├── apps/
│   └── demo/             # Vanilla HTML/JS — click, start, chat. Zero friction.
├── pnpm-workspace.yaml
├── tsconfig.base.json
├── biome.json
└── package.json
```

**Initialization:** No CLI command. First implementation story scaffolds this structure manually.

**Architectural Decisions Provided by This Choice:**
- ESM-first with CJS fallback (tsup dual output)
- Strict TypeScript across all packages
- Shared tsconfig.base.json, per-package tsconfig.json
- vitest with workspace config for cross-package testing
- biome for consistent formatting and linting
- Demo app uses Vite dev server (vanilla mode, no framework) for hot reload during development

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
1. Transport mechanism (WebRTC DataChannel via relay)
2. Signaling bootstrap (WebSocket, temporary)
3. Node identity (Ed25519 keypair, persisted)
4. Internal node architecture (event-driven, unified client/relay)

**Important Decisions (Shape Architecture):**
5. Wire format (JSON, upgradeable)
6. Encryption stack (TweetNaCl.js, iteration 5+)
7. Role model (send/receive = user choice, relay/backup = network-imposed)

**Deferred Decisions (Post-MVP):**
8. DHT implementation for autonomous discovery
9. Contribution scoring formula
10. Subnet formation algorithm

### Transport & Communication

**ADR-001: WebRTC DataChannel via Relay**
- All messages transit through at least one relay node, even in iteration 1
- No direct A→B in the protocol — the relay IS the architecture
- WebRTC DataChannels carry the data (binary-capable, browser-native, P2P after signaling)
- Rationale: Relays are not an optimization — they are the mechanism by which nodes contribute. Sending and receiving is a user action. Relaying is a network-imposed role.

**ADR-002: Signaling Bootstrap**
- **Demo/PoC (iterations 1-3):** Single WebSocket server (Node.js, ~50 lines). Fixed endpoint. The umbilical cord.
- **Growth (iterations 4-5):** Multiple signaling seeds (still WebSocket, several VPS). Resilience through redundancy.
- **Alpha (iteration 6):** Distributed hash table begins operating among 10-15 nodes. Seeds become regular nodes.
- **Target:** The signaling "phone number" (topic hash) stays the same. The "secretary" who answers changes dynamically. No one knows who will answer next. No one can corrupt her in advance. If she disappears, the network appoints another.
- **Final state:** Zero fixed infrastructure. The network IS the infrastructure. The VPS is the umbilical cord — cut when the baby breathes alone.

**ADR-003: Wire Format**
- JSON for iterations 1-4 (human-readable, debug-friendly, native JS)
- Envelope structure: `{from, to, via, type, payload, timestamp, signature}`
- Protocol layer abstracts the format — swappable to MessagePack or binary without breaking SDK
- Migration to compact format when performance requires it (iteration 5+)

### Security & Cryptography

**ADR-004: Encryption Stack**
- **Library:** TweetNaCl.js — minimal, audited, browser + Node.js compatible
- **Key exchange:** X25519 (Diffie-Hellman on Curve25519)
- **Symmetric encryption:** ChaCha20-Poly1305 (authenticated encryption)
- **Handshake:** Custom protocol inspired by Noise XX pattern
- **Timeline:** Crypto pipeline designed from iteration 1, activated at iteration 5
- Rationale: No heavy dependencies. Same primitives as Holepunch's SecretStream. Battle-tested algorithms.

**ADR-005: Node Identity**
- Ed25519 keypair generated on first join
- Public key = node identity on the network (no central registry)
- **Browser:** Persisted in localStorage
- **Node.js:** Persisted in file (~/.tom/identity.json)
- Reconnection (FR45): same keypair = same identity = pending messages delivered
- No accounts, no registration, no identity server. Your key is your address.

### Node Internal Architecture

**ADR-006: Unified Node Model (Event-Driven)**
- Every node runs the same code — no "client binary" vs "relay binary"
- Internal components:
  - `TransportLayer` — manages WebRTC DataChannels (connections to peers)
  - `Router` — receives all messages, decides: for me → deliver, not for me → forward
  - `RoleManager` — tracks assigned role (client, relay, backup, guardian, validator). Role is imposed by network, not chosen.
  - `MessageHandler` — processes messages destined for this node
  - `ContributionTracker` — maintains contribution/usage score
- Message flow: Transport → Router → (MessageHandler | forward via Transport)
- The topology creates the role, not the code. No if/else "am I a relay?"

**ADR-007: Role Model**
- **User-controlled:** Send messages, receive messages, choose username, view path details
- **Network-imposed:** Relay (forward others' messages), backup (temporarily store undelivered messages), guardian, validator
- A node does not volunteer to relay. The network assigns relay duty based on availability, score, and unpredictable selection.
- Rationale: This is ToM's core differentiator from BitTorrent (optional seeding) and Nostr (volunteer relays). Contribution is mandatory and invisible.

### Infrastructure & Deployment

**ADR-008: Demo Hosting (Temporary)**
- **Demo UI:** GitHub Pages (static, free, reliable) — vanilla HTML/JS
- **Signaling server:** Minimal VPS (~5€/month, Hetzner/OVH) running Node.js WebSocket
- Both are transitional. The demo page will eventually be served by the ToM network itself (self-hosting). The signaling VPS will be replaced by distributed DHT.
- This is the accepted compromise documented in the PRD. Isolated, temporary, marked for elimination.

### Decision Impact Analysis

**Implementation Sequence:**
1. Monorepo scaffold (pnpm workspace, tsconfig, biome, vitest)
2. Node identity generation (Ed25519 keypair)
3. Transport layer (WebRTC DataChannel establishment via signaling)
4. Router + MessageHandler (receive, decide, forward)
5. Signaling server (WebSocket, Node.js)
6. Demo UI (vanilla HTML/JS, Vite)
7. RoleManager (iteration 2+)
8. ContributionTracker (iteration 4+)
9. Crypto pipeline (iteration 5)

**Cross-Component Dependencies:**
- Transport depends on Signaling (for WebRTC SDP/ICE exchange)
- Router depends on Transport (for message ingestion)
- RoleManager depends on Router (to know what traffic flows through)
- ContributionTracker depends on RoleManager (to measure contribution)
- Crypto pipeline wraps Transport (encrypt before send, decrypt after receive)
- SDK wraps all of the above into `TomClient.create()` / `.send()` / `.onMessage()`

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:**
23 areas where AI agents could make different choices across naming, structure, format, communication, and process categories.

### Naming Patterns

**File & Directory Naming:**
- All files: `kebab-case.ts` (e.g., `transport-layer.ts`, `role-manager.ts`)
- Test files: `kebab-case.test.ts` co-located next to source file
- Index files: `index.ts` for public API re-exports only
- No `utils.ts` or `helpers.ts` — name by purpose (e.g., `message-validation.ts`)

**Code Naming:**
- Classes/Interfaces/Types: `PascalCase` (e.g., `TransportLayer`, `MessageEnvelope`)
- Functions/Methods: `camelCase` (e.g., `forwardMessage`, `assignRole`)
- Constants: `UPPER_SNAKE_CASE` (e.g., `MAX_RELAY_HOPS`, `DEFAULT_TTL`)
- Variables: `camelCase` (e.g., `peerConnection`, `relayNode`)
- Private members: no underscore prefix — use TypeScript `private` keyword
- Boolean variables: prefix with `is`, `has`, `can`, `should` (e.g., `isRelaying`, `hasIdentity`)
- Event names: `kebab-case` strings (e.g., `message-received`, `peer-connected`, `role-assigned`)

**No abbreviations** except universally understood ones: `id`, `msg`, `config`, `ctx`. Spell everything else out.

### Structure Patterns

**Module Organization:**
- Each component is a single directory with `index.ts` public API
- Internal files are private — never import from another component's internals
- Dependency direction: Transport ← Router ← MessageHandler (never reverse)
- Circular dependencies are forbidden — use events to decouple

**Test Organization:**
- Co-located: `transport-layer.ts` → `transport-layer.test.ts` (same directory)
- Test fixtures: `__fixtures__/` directory within component directory
- Integration tests: `packages/<pkg>/tests/integration/`

**Export Rules:**
- Each package has a single `src/index.ts` entry point
- Only export what consumers need — internal types stay internal
- Re-export types explicitly: `export type { MessageEnvelope }` (not `export *`)

### Format Patterns

**Message Envelope (Wire Format):**
```typescript
interface MessageEnvelope {
  id: string;           // UUIDv4, generated by sender
  from: string;         // Sender public key (hex)
  to: string;           // Recipient public key (hex)
  via: string[];        // Ordered relay path (public keys)
  type: string;         // Message type (e.g., "chat", "signal", "role-assign")
  payload: unknown;     // Type-specific content
  timestamp: number;    // Unix epoch milliseconds
  signature: string;    // Ed25519 signature of envelope (hex)
}
```

**Internal Event Payloads:**
```typescript
// All internal events follow this shape
interface InternalEvent<T = unknown> {
  source: string;       // Component name that emitted
  timestamp: number;    // Unix epoch ms
  data: T;              // Typed payload
}
```

**Error Format:**
```typescript
class TomError extends Error {
  constructor(
    message: string,
    public readonly code: TomErrorCode,
    public readonly context?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'TomError';
  }
}

type TomErrorCode =
  | 'TRANSPORT_FAILED'
  | 'PEER_UNREACHABLE'
  | 'SIGNALING_TIMEOUT'
  | 'INVALID_ENVELOPE'
  | 'IDENTITY_MISSING'
  | 'RELAY_REJECTED'
  | 'CRYPTO_FAILED';
```

**Date/Time:** Always Unix epoch milliseconds (`Date.now()`). Never ISO strings in protocol messages. ISO only for human-facing logs.

**JSON Fields:** `camelCase` everywhere — wire format, internal state, API responses.

### Communication Patterns

**Inter-Component Communication:**
- Components communicate via typed `EventEmitter` — never direct method calls between sibling components
- Router does not import MessageHandler. Router emits `message-for-local`, MessageHandler listens.
- TransportLayer emits `data-received`, Router listens.
- RoleManager emits `role-changed`, any interested component listens.

**Event Naming Convention:**
- Format: `noun-verb-past` (e.g., `peer-connected`, `message-forwarded`, `role-assigned`)
- Never future tense (`will-connect`) or imperative (`connect-peer`)

**State Management:**
- Immutable updates only — never mutate state objects in place
- Each component owns its state — no shared mutable state between components
- State reads via getter methods, state changes via events

### Process Patterns

**Error Handling:**
- Always throw `TomError` with appropriate code — never raw `Error`
- Components catch errors at their boundary and emit error events
- Transport errors → retry with exponential backoff (max 3 attempts, 1s/2s/4s)
- Unrecoverable errors → emit `fatal-error` event, let parent handle
- Never swallow errors silently — at minimum, log and re-emit

**Logging:**
- Use structured logging with component prefix: `[TransportLayer]`, `[Router]`, `[RoleManager]`
- Log levels: `debug` (development only), `info` (state changes), `warn` (recoverable issues), `error` (failures)
- Log message received/forwarded/delivered at `debug` level
- Log peer connect/disconnect at `info` level
- Log role changes at `info` level
- Never log private keys or full message payloads at any level

**Async Patterns:**
- All async operations return `Promise` — no callbacks
- Use `async/await` exclusively — no `.then()` chains
- Cleanup: every component implements `destroy(): Promise<void>` for graceful shutdown
- Cancel in-flight operations on destroy — no dangling promises

### Enforcement Guidelines

**All AI Agents MUST:**
- Follow naming patterns exactly — no variations, no "creative" naming
- Use `TomError` for all error cases — never throw plain strings or generic Error
- Communicate between components via events only — no direct imports of sibling internals
- Co-locate tests with source files
- Export only through package `index.ts`
- Use `async/await` for all async code
- Include component prefix in all log statements

**Pattern Verification:**
- Biome config enforces naming conventions (configured in `biome.json`)
- TypeScript strict mode catches type violations
- Package `index.ts` is the only allowed import path for external consumers
- Code review checklist: naming, error handling, event patterns, no circular deps

### Pattern Examples

**Good Examples:**
```typescript
// Correct: event-driven, typed, proper naming
class Router {
  private readonly events: TypedEventEmitter;

  async handleIncoming(envelope: MessageEnvelope): Promise<void> {
    if (envelope.to === this.localIdentity) {
      this.events.emit('message-for-local', {
        source: 'Router',
        timestamp: Date.now(),
        data: envelope,
      });
    } else {
      await this.forward(envelope);
    }
  }
}
```

**Anti-Patterns:**
```typescript
// WRONG: direct import of sibling internal
import { processMessage } from '../message-handler/process';

// WRONG: raw Error
throw new Error('connection failed');

// WRONG: callback
transport.onData((data) => { ... });

// WRONG: mutable shared state
export const globalState = { peers: [] };

// WRONG: any type
function handle(msg: any) { ... }

// WRONG: console.log
console.log('message received');
```

## Project Structure & Boundaries

### Complete Project Directory Structure

```
tom-protocol/
├── package.json                    # Root workspace config
├── pnpm-workspace.yaml             # Workspace: packages/*, apps/*
├── tsconfig.base.json              # Shared TS config (strict, ESM)
├── biome.json                      # Lint + format (all packages)
├── vitest.workspace.ts             # Cross-package test config
├── .gitignore
├── LICENSE                         # MIT
├── README.md
│
├── packages/
│   ├── core/                       # Raw protocol primitives
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── tsup.config.ts          # ESM + CJS dual output
│   │   └── src/
│   │       ├── index.ts            # Public API: TransportLayer, Router, etc.
│   │       ├── identity/
│   │       │   ├── index.ts
│   │       │   ├── keypair.ts              # Ed25519 generation & persistence
│   │       │   └── keypair.test.ts
│   │       ├── transport/
│   │       │   ├── index.ts
│   │       │   ├── transport-layer.ts      # WebRTC DataChannel management
│   │       │   ├── transport-layer.test.ts
│   │       │   ├── signaling-client.ts     # WebSocket signaling connection
│   │       │   └── signaling-client.test.ts
│   │       ├── router/
│   │       │   ├── index.ts
│   │       │   ├── router.ts               # Message routing: local vs forward
│   │       │   └── router.test.ts
│   │       ├── message/
│   │       │   ├── index.ts
│   │       │   ├── message-handler.ts      # Process messages destined for this node
│   │       │   ├── message-handler.test.ts
│   │       │   ├── envelope.ts             # MessageEnvelope type & validation
│   │       │   └── envelope.test.ts
│   │       ├── roles/
│   │       │   ├── index.ts
│   │       │   ├── role-manager.ts         # Track & respond to role assignments
│   │       │   └── role-manager.test.ts
│   │       ├── contribution/
│   │       │   ├── index.ts
│   │       │   ├── contribution-tracker.ts # Usage/contribution score
│   │       │   └── contribution-tracker.test.ts
│   │       ├── crypto/                     # Iteration 5+ (designed now, activated later)
│   │       │   ├── index.ts
│   │       │   ├── handshake.ts            # X25519 key exchange
│   │       │   ├── encryption.ts           # ChaCha20-Poly1305 encrypt/decrypt
│   │       │   └── handshake.test.ts
│   │       ├── errors/
│   │       │   ├── index.ts
│   │       │   └── tom-error.ts            # TomError class + TomErrorCode
│   │       ├── events/
│   │       │   ├── index.ts
│   │       │   └── typed-emitter.ts        # Typed EventEmitter wrapper
│   │       └── types/
│   │           └── index.ts                # Shared types (MessageEnvelope, etc.)
│   │
│   └── sdk/                        # Plug-and-play abstraction
│       ├── package.json
│       ├── tsconfig.json
│       ├── tsup.config.ts
│       └── src/
│           ├── index.ts            # Public API: TomClient
│           ├── tom-client.ts               # TomClient.create(), .send(), .onMessage()
│           ├── tom-client.test.ts
│           ├── auto-relay.ts               # Automatic relay assignment
│           ├── auto-relay.test.ts
│           ├── auto-encrypt.ts             # Transparent E2E (iteration 5+)
│           └── types/
│               └── index.ts                # SDK-specific public types
│
├── apps/
│   └── demo/                       # Vanilla HTML/JS — click, start, chat
│       ├── package.json
│       ├── vite.config.ts          # Vanilla mode, no framework
│       ├── index.html
│       ├── src/
│       │   ├── main.ts             # Entry: TomClient.create() + UI wiring
│       │   ├── ui.ts               # DOM manipulation (connect, send, display)
│       │   └── style.css           # Minimal styling
│       └── public/
│
└── tools/
    └── signaling-server/           # Bootstrap WebSocket server (~50 lines)
        ├── package.json
        ├── tsconfig.json
        ├── src/
        │   ├── index.ts            # Entry: start WS server
        │   ├── server.ts           # WebSocket SDP/ICE relay
        │   └── server.test.ts
        └── Dockerfile              # Optional: deploy to VPS
```

### Architectural Boundaries

**Package Boundaries:**
- **core** → zero external dependency (except tweetnacl). Does not know sdk exists. Does not know demo exists.
- **sdk** → depends on core only. Exposes `TomClient` as sole public API. Hides all protocol complexity.
- **demo** → depends on sdk only. Never imports from core directly.
- **signaling-server** → independent. Zero dependency on any package. Pure WebSocket SDP/ICE relay.

**Dependency Direction (strict, never reversed):**
```
demo → sdk → core
                ↑
signaling-server (independent, no dependency on packages)
```

**Component Boundaries (within core):**
- Components communicate via typed EventEmitter only — no direct imports between siblings
- Transport → emits `data-received` → Router listens
- Router → emits `message-for-local` → MessageHandler listens
- RoleManager → emits `role-changed` → any component listens
- Each component directory has `index.ts` as sole public entry point

### Requirements to Structure Mapping

| FR Category | Package | Directory |
|---|---|---|
| FR1-10 (Core protocol) | core | transport/, router/, message/, roles/ |
| FR11-15 (UX/Demo) | demo | apps/demo/src/ |
| FR16-20 (Network participation) | core | roles/, contribution/ |
| FR21-24 (Bootstrap) | signaling-server + core | tools/signaling-server/, transport/signaling-client |
| FR25-30 (SDK) | sdk | tom-client, auto-relay, auto-encrypt |
| FR31-35 (LLM tooling) | sdk | types/ (well-documented for AI consumption) |
| FR36-40 (Community) | core | contribution/ |
| FR41-45 (Economy) | core | contribution/ |

### Data Flow

```
Demo UI → TomClient.send() → TransportLayer → [WebRTC] → Relay Node → [WebRTC] → Router → MessageHandler → TomClient.onMessage() → Demo UI
                                    ↑
                         SignalingClient ← [WebSocket] ← SignalingServer (bootstrap only)
```

### File Organization Patterns

**Configuration Files:**
- Root: workspace-level config (pnpm, tsconfig base, biome, vitest workspace)
- Per-package: package.json, tsconfig.json, tsup.config.ts
- No `.env` files — signaling server URL is a constructor parameter, not environment config

**Test Organization:**
- Unit tests: co-located (`*.test.ts` next to source)
- Integration tests: `packages/<pkg>/tests/integration/`
- Fixtures: `__fixtures__/` within component directory
- No e2e test directory yet — added when demo reaches iteration 3+

**Build Output:**
- Each package builds to `dist/` (gitignored)
- tsup produces ESM (`dist/index.mjs`) + CJS (`dist/index.cjs`) + types (`dist/index.d.ts`)
- Demo builds to `apps/demo/dist/` (static files for GitHub Pages)

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**
All technology choices (TypeScript 5.x strict, tsup, vitest, biome, WebRTC, TweetNaCl.js) are mutually compatible. All run natively in browser + Node.js. No version conflicts. JSON wire format is native to both runtimes.

**Pattern Consistency:**
Implementation patterns (event-driven communication, typed EventEmitter, kebab-case files, PascalCase classes) directly support the unified node model (ADR-006). No contradictions between patterns and decisions.

**Structure Alignment:**
Project structure (packages/core, packages/sdk, apps/demo, tools/signaling-server) maps cleanly to dependency direction. Each ADR has a clear home in the structure. No orphan decisions.

### Requirements Coverage Validation ✅

**Functional Requirements Coverage:**

| FR Category | Coverage | Notes |
|---|---|---|
| FR1-10 (Core protocol) | ✅ Full | transport/, router/, message/, roles/ |
| FR11-15 (UX/Demo) | ✅ Full | apps/demo/ via TomClient |
| FR16-20 (Network participation) | ✅ Full | roles/, contribution/ |
| FR21-24 (Bootstrap) | ✅ Full | signaling-server + signaling-client |
| FR25-30 (SDK) | ✅ Full | sdk/tom-client, auto-relay, auto-encrypt |
| FR31-35 (LLM tooling) | ✅ Full | sdk/types/ well-documented |
| FR36-40 (Community) | ✅ Full | contribution/ |
| FR41-45 (Economy) | ✅ Full | contribution/ |

**Non-Functional Requirements Coverage:**

| NFR | Coverage | Mechanism |
|---|---|---|
| <500ms message delivery | ✅ | WebRTC DataChannel (sub-100ms native) |
| <3s peer discovery | ✅ | WebSocket signaling, future DHT |
| Relay statelessness | ✅ | ADR-006 unified node, pass-through only |
| 24h backup TTL | ✅ | Backup node mechanism (see ADR-009 below) |
| E2E encryption | ✅ | crypto/ directory, TweetNaCl.js (iteration 5) |
| Inversion property | ✅ | ADR-007 mandatory relay, more nodes = more capacity |

### Architectural Clarification: Backup Node Mechanism (ADR-009)

**ADR-009: Message Backup & Survival Strategy**

Clarified during validation — the backup mechanism follows the "virus" metaphor from the whitepaper:

- **Role assignment:** Backup is a network-imposed role, like relay. The network decides who becomes backup based on available storage, time online, bandwidth, and contribution score. Cascading — never a single backup node.
- **Multi-node replication:** Messages are replicated across multiple backup nodes for resilience. If one backup disconnects, others still hold the message.
- **Message scoring & host-hopping:** Each buffered message has a survival score based on host quality (timezone alignment with recipient, connection history, bandwidth). The message is proactive, not reactive — it monitors its own score continuously. When the score drops, the message replicates to a better host **in parallel** (doesn't wait for completion before acting). When the score drops below a threshold X%, the message **self-deletes from the current host before the host dies** — because waiting for the host to disconnect is already too late. The message does everything in its power to survive 24h.
- **TTL enforcement:** 24h maximum. After TTL, all backup copies are purged. No permanent storage anywhere.
- **Recipient reconnection:** When the recipient comes back online, it queries the network for pending messages. Upon successful delivery, a "received" signal propagates through the network, triggering deletion of all backup copies.
- **Iteration:** This mechanism is designed now but implemented at iteration 3+. The architecture supports it via contribution/ and roles/ in core.

### Gap Analysis Results

**Critical Gaps: 0**

**Important Gaps (all acceptable for PoC):**
1. **Holepunch bridge** — not yet in structure. Will be `transport/holepunch-bridge.ts` at iteration 6+. Architecture supports it.
2. **CI/CD pipeline** — no `.github/workflows/` defined. Added during scaffold story.

**Nice-to-Have Gaps:**
- Monitoring/observability patterns — acceptable for PoC
- Rate limiting on signaling server — acceptable for demo scale

### Architecture Completeness Checklist

**✅ Requirements Analysis**
- [x] Project context thoroughly analyzed (45 FRs, 14 NFRs)
- [x] Scale and complexity assessed (high — novel protocol)
- [x] Technical constraints identified (browser-first, no central infra, MIT)
- [x] Cross-cutting concerns mapped (NAT, encryption, identity, statelessness, dual nature)

**✅ Architectural Decisions**
- [x] 9 ADRs documented with rationale
- [x] Technology stack fully specified with versions
- [x] Integration patterns defined (event-driven, typed EventEmitter)
- [x] Performance considerations addressed (WebRTC latency, JSON parse speed)

**✅ Implementation Patterns**
- [x] Naming conventions established (23 conflict points resolved)
- [x] Structure patterns defined (module organization, exports, tests)
- [x] Communication patterns specified (event-driven, noun-verb-past)
- [x] Process patterns documented (TomError, logging, async/await)

**✅ Project Structure**
- [x] Complete directory structure defined (every file listed)
- [x] Component boundaries established (strict dependency direction)
- [x] Integration points mapped (event flow between components)
- [x] Requirements to structure mapping complete (all 45 FRs mapped)

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION

**Confidence Level:** High

**Key Strengths:**
- Progressive iteration model — each component can be built and tested independently
- Event-driven architecture eliminates coupling between components
- Virus-like message survival strategy is architecturally novel and well-defined
- Zero framework dependency keeps the project lean and portable
- Unified node model means no code duplication between client and relay

**Areas for Future Enhancement:**
- CI/CD pipeline (GitHub Actions) — scaffold story
- Holepunch bridge for native transport — iteration 6
- Monitoring & observability — when network exceeds 10 nodes
- Message scoring algorithm refinement — iteration 3+

### Implementation Handoff

**AI Agent Guidelines:**
- Follow all 9 ADRs exactly as documented
- Use implementation patterns consistently across all components
- Respect package boundaries: demo → sdk → core (never reversed)
- Communicate between components via typed EventEmitter only
- Refer to this document for all architectural questions

**First Implementation Priority:**
1. Monorepo scaffold (pnpm workspace, tsconfig, biome, vitest)
2. Node identity generation (Ed25519 keypair)
3. Transport layer + signaling client
4. Router + MessageHandler
5. Signaling server
6. Demo UI
