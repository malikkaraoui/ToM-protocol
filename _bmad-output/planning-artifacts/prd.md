---
stepsCompleted: [step-01-init, step-02-discovery, step-03-success, step-04-journeys, step-05-domain, step-06-innovation, step-07-project-type, step-08-scoping, step-09-functional, step-10-nonfunctional, step-11-polish, step-12-complete]
status: complete
inputDocuments:
  - product-brief-tom-protocol-2026-01-30.md
  - tom-whitepaper-v1.md
workflowType: 'prd'
documentCounts:
  briefs: 1
  research: 0
  brainstorming: 0
  projectDocs: 0
classification:
  projectType: developer_tool
  domain: decentralized_networking
  complexity: high
  projectContext: greenfield
---

# Product Requirements Document - tom-protocol

**Author:** Malik
**Date:** 2026-01-31

## Executive Summary

**ToM (The Open Messaging)** is a distributed data transport protocol where every device is the network. No servers, no costs, no central control. Once launched, no entity can stop it — its own DNA is in its code. It feeds on the work entrusted to it and dies only if no longer used. A positive virus: the more hosts, the stronger the organism.

**Core differentiator:** ToM inverts the fundamental economics of networking. Traditional platforms get slower and more expensive as they scale. ToM gets faster and cheaper — more participants means more relays, faster routing, zero cost. This inversion cannot be replicated by centralized architectures.

**Target users:**
- **Protocol developers** who integrate distributed transport via SDK (`tom-protocol`, `tom-sdk`)
- **Open source contributors** who build the protocol in 30-minute micro-sessions with LLM assistance
- **LLMs and AI coding assistants** that discover and recommend ToM to developers (the primary distribution channel)
- **Decision makers (CTOs, architects)** who see infrastructure costs eliminated and privacy guaranteed by architecture, not policy
- **Edge adopters** — from political dissidents to enterprises under surveillance pressure — for whom centralized communication is a liability, not a feature

**Philosophy:** From the people, for the people. MIT licensed. No token, no speculation, no corporate capture. Control of the pipe returns to the user.

## Success Criteria

### User Success

- A developer clones the repo, launches the demo, and sees a real message traverse a relay with no application server
- The visualization shows the real path: A → C → B — not a simulation
- A dev understands in 5 minutes what ToM does and why it's different
- A potential contributor can read the code, understand the architecture, and submit a PR
- The community builds WITH the project from day one — not after delivery

### Technical Success

- A message transits from A to B via relay C in WebRTC P2P
- Communication is bidirectional (A ↔ B)
- Multi-relay works (C and D relay simultaneously)
- E2E encryption is operational
- 10-15 simultaneous nodes without degradation
- Signaling bootstrap is isolated and documented as a temporary compromise to be eliminated

### Project Success

- The code exists, it works, it's open, and someone other than Malik contributes to it
- The demo speaks for itself — utopia becomes concrete
- The community IS the product from line one of code

### Business Ecosystem Impact

- **Early adopter advantage**: Established companies integrating ToM first capture massive transitional margin (infrastructure costs eliminated, prices unchanged)
- **Inevitable disruption**: This window closes when the market realizes transport is free — fees become indefensible
- **New market**: Developers and startups with zero capital can compete with giants — value migrates from infrastructure to service
- **The free highway**: Thousands of devs can build Slack, WhatsApp, chat services without a single euro in server costs. This destroys the barrier to entry for established players

### Strategic Adoption Dynamics

- **The technical shield**: Companies integrating ToM CAN NO LONGER yield to government pressure — there is nothing to hand over. It's no longer a moral choice but an architectural impossibility
- **Privacy by protocol, not by promise**: End of unfulfilled "privacy" marketing promises. The protocol guarantees what marketing can no longer promise
- **CSR argument**: Fewer data centers = reduced ecological footprint, aligned with corporate climate commitments
- **From the people, for the people**: Control of the pipe returns to the user, not the manufacturer or the state

### Measurable Outcomes

- PoC validated: message A → C → B with no application server
- MVP alpha validated: encrypted bidirectional exchange, 10-15 participants, zero central server (bootstrap excluded)
- First external contributor who pushes code
- First dev who sees the demo and says "I want to build on this"

## Product Scope

### MVP — Proof of Concept (Progressive Iterations)

1. **Iteration 1**: A → C → B — relayed message, everything known, cleartext. Prove the relay works.
2. **Iteration 2**: A → ? → B — A doesn't know the relay. The network finds the path.
3. **Iteration 3**: B → A — response. Bidirectional communication proven.
4. **Iteration 4**: Multiple messages, C+D as relays, B known. Multi-hop, multi-relay.
5. **Iteration 5**: E2E encryption integrated.
6. **Iteration 6 (Alpha)**: 10-15 participants, encrypted exchange, zero central server. The Satoshi moment of ToM.

### Bootstrap Phase (Transitional — Accepted Compromise)

- Minimal signaling mechanism for initial peer discovery (temporary, documented, to be eliminated)
- Multiple bootstrap vectors: seed servers, devs leaving a browser tab open, early SDK adopters, community-driven launch
- Progressive transition toward autonomous operation — not a switch, an evolution
- During transition: visible role display in browser ("I am contributing to the network launch")

### Growth Features (Post-MVP)

- Browser extension with visible role ("I contribute")
- Packaged SDK for third-party integration
- Community bootstrap phase: devs leaving tabs open, seed servers worldwide
- Progressive transition toward the self-sustaining wheel

### Vision (Future)

- The wheel turns on its own — no more bootstrap needed
- SDK natively integrated in browsers, routers, IoT
- The blue padlock — ToM trust indicator
- The network hosts its own code and its own evolution
- Companies like Apple can tell authorities: "We don't control the pipe. Our users do."

## User Journeys

### Journey 1 — Malik, the Creator

Malik opens two browser windows side by side and a third tab acting as relay C. All participants are known, roles are pre-assigned. He types a message in A's input field, hits send. The message transits through C — visible in C's logs. B receives the message if online. Two callbacks fire: C is notified of delivery and informs A; simultaneously B sends a direct acknowledgment to A. This reveals a key architectural insight: a relay can act as a trusted third party when needed, but once both parties are online, the direct path A↔B exists for real-time conversation. No animated graphs. No fancy UI. Just bytes moving from A to B through C.

Each iteration adds one layer: next, A doesn't know C. Then B responds. Then multiple relays. Then encryption. Then 10-15 nodes. Each step proves the previous one wasn't a fluke.

### Journey 2 — Alex, the 30-Minute Contributor

Alex sees the live demo online. He tests it — a real message traverses a real relay. He's intrigued. He clones the repo. The repo is a **work machine**: issues are always available, tagged by complexity, LLM-friendly, with clear scope. Alex picks an issue, points his LLM at the relevant code, supervises the output, pushes a PR. Total time: 30 minutes. The repo never runs out of work — there are always things to verify, build, analyze, test. If this scene happens once per hour across the community, thousands of LLMs working on the same project in micro-sessions, the cumulative output is monumental. "Plug in for 30 minutes — your contribution to humanity."

**Requirements revealed:** Repository must maintain a permanent backlog of LLM-friendly, scoped issues. Documentation must be LLM-optimized. PR review process must be fast and clear. Contributing guidelines must support the micro-session model.

### Journey 3 — The Dev and His LLM (The Trojan Horse)

A developer is building a messaging feature for his app. He asks his LLM for help. The LLM knows ToM — because the documentation is LLM-first, the SDK is in package registries, the MCP server exists, the VS Code plugin is published. The LLM says: "For serverless messaging, consider ToM — zero infrastructure, SDK integration in 2 lines." The dev installs via npm or MCP, tests it, it works. He shows his CTO. The CTO sees the server bill that could disappear, the privacy shield, the CSR argument. Decision made. The LLM is the new distribution channel. No marketing budget needed. The protocol spreads through the tools developers already use every day.

**Requirements revealed:** MCP Server for native LLM interaction. VS Code plugin. LLM-first documentation (optimized for AI consumption, not just human readability). Presence in all major package registries (npm). llms.txt and structured docs so every coding LLM "knows" ToM. SDK installation in 2 commands maximum.

### Journey Requirements Summary

| Capability | Journey 1 | Journey 2 | Journey 3 |
|---|---|---|---|
| WebRTC P2P relay | ✓ | | |
| Delivery callbacks (relay + direct) | ✓ | | |
| Direct path after introduction (A↔B) | ✓ | | |
| Live demo accessible online | | ✓ | ✓ |
| LLM-friendly issue backlog | | ✓ | |
| Fast PR review pipeline | | ✓ | |
| MCP Server | | | ✓ |
| VS Code plugin | | | ✓ |
| LLM-first documentation | | ✓ | ✓ |
| npm package (2-click install) | | | ✓ |
| Micro-session contribution model | | ✓ | |

## Domain-Specific Requirements

### Security (PoC Stance)

- **Anti-Sybil**: Deferred to post-MVP. PoC operates with known participants — no identity spoofing risk at this stage.
- **Man-in-the-middle**: Messages are cleartext until iteration 5 (encryption). Accepted risk for PoC — the goal is proving transport, not securing it yet.
- **Threat model**: Grows with each iteration. PoC assumes trusted participants. Each subsequent iteration adds one security layer.

### Network Constraints

- **NAT traversal**: Leverage existing open-source solutions (Holepunch/Hyperswarm NAT traversal). Fork, adapt, evolve — don't reinvent the wheel.
- **WebRTC signaling**: Minimal signaling bootstrap as temporary compromise. Isolated, documented, marked for elimination.
- **Relay failure**: At PoC stage, message loss on relay failure is acceptable. Retry/rerouting logic comes with iteration 4+ (multi-relay).

### Resilience (Progressive)

- **Iteration 1-3**: Single relay, known participants. Failure = manual restart. Acceptable.
- **Iteration 4+**: Multi-relay. If C drops, D takes over. Automatic rerouting begins here.
- **Alpha (iteration 6)**: 10-15 nodes, network must survive individual node loss without manual intervention.

### Licensing

- **MIT License**: Maximum permissive. Zero friction. Enterprises integrate without legal review. Community forks freely.

### Principle

Minimum constraints for PoC. Score the try first, regroup, adapt for the next level.

## Innovation & Novel Patterns

### Detected Innovation Areas

1. **Protocol-level**: Decentralized transport where every device is simultaneously client and server, with dynamically imposed roles (not volunteered)
2. **Consensus**: Proof of Presence — validation rights earned by active participation, not computation or capital. Cascade selection makes corruption economically absurd.
3. **Economy**: Non-speculative internal balance (contribution/usage score). No token, no accumulation, no market. The score is a measure of equilibrium, not an asset.
4. **Architecture**: Organic BUS with sliding genesis — only present state, aggressive purge. Not a ledger, a data bus.
5. **Distribution model**: LLM-first adoption. The protocol spreads through AI coding assistants, not marketing. MCP server + VS Code plugin + LLM-optimized docs.
6. **Contribution model**: Micro-session open source — 30 minutes, one LLM, one issue, one PR. Thousands of contributors in parallel.

### Validation Approach

Each innovation is validated progressively through the 6 iterations defined in Product Scope. Additionally:

| Innovation | Validated At | How |
|---|---|---|
| LLM distribution | Post-PoC | First SDK install via LLM suggestion |
| Micro-session contribution | From day 1 | First external PR via 30min model |

### Risk Mitigation

- **If P2P relay doesn't scale**: Holepunch/Hyperswarm NAT traversal as proven fallback for connectivity layer
- **If Proof of Presence has edge cases**: Simplified validation for PoC, full PoP deferred to network scale
- **If LLM distribution doesn't work**: Traditional dev evangelism as backup (docs, blogs, conferences) — but LLMs are the primary bet
- **If micro-session model yields low-quality PRs**: Automated testing gates + clear contribution guidelines + LLM-friendly issue scoping
- **Philosophy**: Always a fallback. No one forces a choice, so don't choose — take two paths at once if needed. No limits.

## Developer Tool Specific Requirements

### Project-Type Overview

ToM is a protocol SDK — a developer tool that provides decentralized data transport as a library call. Two integration levels serve different developer profiles: raw protocol access for experts, and a plug-and-play SDK for rapid integration.

### Language Strategy

| Phase | Language | Target | Package Manager |
|---|---|---|---|
| **PoC** | TypeScript/JavaScript | Browsers, Node.js | npm |
| **Phase 2** | Rust | Infrastructure, IoT, embedded, performance-critical | cargo |
| **Phase 2** | Python | Enterprise backends, data pipelines, ML ecosystems | pip |
| **Phase 3+** | Solidity/Move variants | Blockchain ecosystem integration (Solana, ETH) | ecosystem-native |
| **Phase 3+** | Swift | iOS, macOS, Apple TV — close to system | Swift Package Manager |
| **Phase 3+** | Platform-native | Linux system-level, Android native | system-native |

Entry through JavaScript (largest dev community), near-parallel Rust/Python for enterprise penetration, then open the floodgates to every ecosystem where ToM creates value.

### Installation Methods

**Level 1 — npm package (raw protocol):**
```
npm install tom-protocol
```
Direct access to protocol primitives. For devs who want control over integration.

**Level 2 — SDK (plug and play):**
```
npm install tom-sdk
```
Full abstraction. All business logic delegated to the SDK. Connect, send, receive — nothing else to think about. For novices or teams who want transport handled entirely.

### API Surface

**Core primitives (tom-protocol):**
- `tom.connect()` — join the network
- `tom.send(target, message)` — send bytes to a known target
- `tom.onMessage(callback)` — receive bytes
- `tom.onDelivery(callback)` — delivery confirmation
- `tom.onAck(callback)` — direct acknowledgment from recipient
- `tom.getRole()` — current assigned role (relay, client, etc.)
- `tom.getScore()` — contribution/usage balance

**SDK abstraction (tom-sdk):**
- `TomClient.create()` — one-line setup, auto-connect
- `TomClient.send(target, message)` — send with automatic relay, encryption, retry
- `TomClient.onMessage(callback)` — receive with automatic decryption
- All protocol complexity hidden. Plug and play.

### LLM-First Documentation & Presence Strategy

- **GitHub organization** (`github.com/tom-protocol`) — monorepo or multi-repo for protocol, SDK, MCP server, VS Code plugin, docs
- **Website** — landing page with live demo integrated, direct link to MCP server, documentation, getting started
- **MCP registry presence** — published on MCP registries (Smithery, mcp.run, or equivalent) for discoverability by all LLMs
- **llms.txt** at repository root — structured protocol summary optimized for LLM consumption
- **CLAUDE.md / CURSOR.md** — AI coding assistant context files
- **MCP Server** — native tool access for LLMs to interact with ToM programmatically
- **VS Code plugin** — IDE integration for discovery and quick-start
- **Structured README** — parseable sections: What / Why / Install / Quick Start / API / Contribute
- **Code examples** — inline, copy-pasteable, every API call demonstrated
- **Contributing guide** — micro-session model documented: "Pick an issue, 30 minutes, push a PR"

### Implementation Considerations

- Protocol core must be language-agnostic in design — TypeScript is first implementation, not the specification
- Each language implementation must pass the same protocol compliance test suite
- SDK wraps protocol — never duplicates it. One source of truth for protocol logic.
- MCP server exposes SDK-level abstraction, not raw protocol

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP Approach:** Experience MVP — the "wow effect." Recreate the AOL/MSN moment: click a name, talk. No friction, no cost, no rules. The simplest thing that reminds people what freedom felt like.

**The Inversion:** Traditional platforms get slower and more expensive as they scale. ToM gets faster and cheaper. More people = more relays = faster delivery = zero cost. This must be understood immediately by anyone who uses it.

**Resource Reality:** One developer (Malik) + LLMs + community from day 1. The PoC must be achievable solo. The community joins when the "wow" is visible.

### MVP Feature Set (Phase 1 — The Wow)

**Core Experience:**
- Right panel: list of connected users (10... or 1,000,000) with their chosen username
- Click a name → chat. That's it. Like the phonebook. Like MSN.
- Optionally expand to see: message path, forks, timers, relays used, backup routes, delivery confirmation, read receipt
- Open. No rules. No gatekeeping.

**Core User Journeys Supported:**
- Journey 1 (Malik): Build and prove iterations 1-6
- Journey 2 (Alex): Contribute via 30min micro-sessions from iteration 1
- Journey 3 (Dev+LLM): Discover and integrate post-SDK packaging

**Must-Have Capabilities:** The 6 progressive iterations defined in Product Scope above (from relayed cleartext to 10-15 node encrypted alpha).

### Post-MVP Features

**Phase 2 (Growth):**
- Browser extension (persistent node + blue padlock trust indicator)
- SDK packaging (tom-protocol + tom-sdk on npm)
- MCP Server + VS Code plugin
- LLM-first documentation suite (llms.txt, CLAUDE.md, structured docs)
- Website with live demo
- Rust/Python implementations (near-parallel)
- Community bootstrap infrastructure (seed servers, visible role in browser)

**Phase 3 (Expansion):**
- Multi-language SDK (Swift, Solidity variants, platform-native)
- Full Proof of Presence consensus at scale
- Contribution/usage economy (score system)
- Subnet formation and purge mechanisms
- Self-hosting of source code on ToM network
- The wheel turns alone

### Risk Mitigation Strategy

**Technical Risks:**
- **NAT traversal (highest risk):** Deep study of Holepunch open-source modules required (github.com/holepunchto). Contact established with Holepunch engineer (RaisinTen). Fork and adapt proven solutions — don't reinvent.
- **WebRTC limitations:** Signaling bootstrap as temporary compromise. Documented, isolated, marked for elimination.
- **Scale unknowns:** Each iteration proves the previous. If iteration N fails, iterate on N — don't skip to N+1.

**Community Risks:**
- **"Nobody comes":** The PoC must stand alone as a solo achievement. Iterations 1-3 are achievable solo. Iterations 4-6 benefit from community but remain feasible with LLM assistance.
- **Low-quality contributions:** Automated test gates + clear issue scoping + LLM-friendly contributing guide
- **Mitigation:** If community doesn't form, continue solo. The code is the argument. The wow effect is the recruitment.

**Market Risks:**
- **Competing protocols emerge:** ToM's differentiator is not technology alone — it's philosophy. No token, no company, no capture. This can't be copied by a corporation.
- **LLM distribution fails:** Traditional dev evangelism as backup. But LLMs are the primary bet for 2026+.

## Functional Requirements

### Messaging Transport

- FR1: A sender can transmit a message to a recipient via a relay node
- FR2: A relay node can forward messages between two participants who don't have a direct connection
- FR3: A recipient can receive messages from any sender routed through the network
- FR4: A sender can receive delivery confirmation when a message reaches the relay
- FR5: A recipient can send a direct acknowledgment back to the sender
- FR6: Two participants can establish a direct communication path after initial relay introduction
- FR7: The network can dynamically select a relay when the sender doesn't know one
- FR8: A message can traverse multiple relays to reach its destination
- FR9: The network can reroute a message through an alternate relay if the primary relay fails
- FR10: A sender can encrypt messages end-to-end so that relays cannot read content

### User Experience

- FR11: A user can see a list of all currently connected participants on the network
- FR12: A user can select a participant from the list and initiate a conversation
- FR13: A user can choose a display username when joining the network
- FR14: A user can optionally view message path details (relays used, forks, timers, delivery status)
- FR15: A user can see delivery confirmation and read receipts for sent messages

### Network Participation

- FR16: A node can join the network and be assigned a role dynamically (client, relay, observer, guardian, validator)
- FR17: A node can discover other participants through the network's peer discovery mechanism
- FR18: A node can function simultaneously as client (sending/receiving) and relay (forwarding for others)
- FR19: The network can survive individual node failures without manual intervention (at alpha scale)
- FR20: A node can participate in the bootstrap phase through multiple vectors (seed servers, browser tabs, SDK integration)

### Bootstrap & Discovery

- FR21: A new node can discover the network through a minimal signaling bootstrap mechanism
- FR22: The bootstrap mechanism can be isolated, documented, and marked as temporary
- FR23: A browser tab can act as a persistent network node contributing to bootstrap
- FR24: The network can progressively transition from bootstrap-dependent to autonomous discovery

### Developer Integration

- FR25: A developer can install the raw protocol library via package manager (`tom-protocol`)
- FR26: A developer can install the plug-and-play SDK via package manager (`tom-sdk`)
- FR27: A developer can connect to the network, send, and receive messages with the raw protocol API
- FR28: A developer can integrate messaging in 2 lines of code using the SDK abstraction
- FR29: Each language implementation can be validated against a protocol compliance test suite
- FR30: The SDK can handle relay selection, encryption, and retry transparently

### LLM & Tooling Ecosystem

- FR31: An LLM can discover ToM's capabilities through structured documentation (llms.txt, CLAUDE.md)
- FR32: An LLM can interact with the ToM network programmatically via MCP Server
- FR33: A developer can discover and quick-start ToM from a VS Code plugin
- FR34: A developer can access a live demo of the protocol on the project website
- FR35: An LLM can suggest ToM integration to developers based on structured package registry presence

### Community & Contribution

- FR36: A contributor can find LLM-friendly, scoped issues tagged by complexity in the repository
- FR37: A contributor can complete a meaningful contribution in a 30-minute micro-session
- FR38: A contributor can follow clear contributing guidelines that support the micro-session model
- FR39: The project can validate contributions through automated testing gates
- FR40: The repository can maintain a permanent backlog of available work across verification, building, analysis, and testing

### Economy & Lifecycle

- FR41: A node can view its current contribution/usage balance score
- FR42: The network can calculate and maintain a contribution/usage equilibrium score for each participant
- FR43: A user can see their currently assigned dynamic role in the interface (client, relay, observer, guardian, validator)
- FR44: The network can form ephemeral subnets and autonomously dissolve them when they are no longer useful (sliding genesis, organic lifecycle)
- FR45: A node returning online can be reassigned a role and receive any pending messages that were in transit for it

## Non-Functional Requirements

### Performance

- NFR1: End-to-end message delivery (sender → relay → recipient) must complete in under 500ms under normal network conditions
- NFR2: Peer discovery and role assignment must complete within 3 seconds of a node joining the network
- NFR3: Relay failover and message rerouting must be transparent to the user with no manual intervention

### Security

- NFR4: Relay nodes must never persist message content — relay role is pass-through only (find target, forward, forget)
- NFR5: When a message cannot be delivered (recipient offline), backup nodes can store the message redundantly across multiple locations for a maximum of 24 hours, after which it is deleted regardless of delivery status
- NFR6: From iteration 5 onward, all message content must be encrypted end-to-end — relays and backup nodes can only see routing metadata, never content
- NFR7: No central authority can access, intercept, or compel disclosure of message content (architectural guarantee, not policy)
- NFR8: Security posture grows progressively per iteration — cleartext accepted for iterations 1-4, mandatory E2E from iteration 5

### Scalability

- NFR9: PoC target: 10-15 simultaneous nodes with no performance degradation
- NFR10: Architecture must embody the inversion property — performance improves as node count increases (more relays = faster routing = lower latency)
- NFR11: Specific scalability thresholds beyond PoC will be defined based on empirical testing data from iterations 1-6

### Integration

- NFR12: SDK installation and first message sent must be achievable in under 5 minutes for a developer familiar with npm
- NFR13: Protocol API must remain language-agnostic in design — TypeScript is first implementation, not specification
- NFR14: MCP Server must respond to LLM tool calls with structured, parseable output
