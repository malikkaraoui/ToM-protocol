---
stepsCompleted: [1, 2, 3, 4, 5, 6]
status: complete
inputDocuments:
  - tom-whitepaper-v1.md
date: 2026-01-30
author: Malik
---

# Product Brief: tom-protocol

## Executive Summary

The Open Messaging (ToM) is a decentralized data transport protocol designed to function as a foundational communication layer — like TCP/IP or HTTP — on top of existing internet infrastructure. ToM eliminates all reliance on central servers, trusted third parties, or speculative economics. Every connected device becomes both client and server, roles are assigned dynamically and unpredictably, and the network retains only the present state. The protocol's internal economy automatically balances usage and contribution — no tokens, no fees, no data exploitation. ToM is not an application. It is a protocol layer intended for integration into everyday tools — browsers, messaging apps, routers, IoT devices — so that users participate without knowing it. The goal: a universal, resilient, self-sustaining communication layer that belongs to no one.

---

## Core Vision

### Problem Statement

The internet was built to be decentralized. In practice, a handful of corporations control its critical layers: transport (cloud, CDN, DNS providers), applications (messaging platforms, social networks, email), and access (ISPs that can filter, throttle, or block). Every message passes through control points that can read, store, filter, or cut communication. Users believe they communicate freely. They do not.

Beyond corporate control, governments weaponize this centralization: freezing bank accounts of Canadian truckers, cutting communications during uprisings in Nepal, pressuring Apple into compliance, forcing Telegram to surrender. The centralized architecture is not a bug — it is a tool of control.

The economic model compounds the problem: "free" services extract payment through data exploitation and resale. Users are told data centers require this trade-off. This is false. Alternative architectures exist.

### Problem Impact

- **Individual sovereignty destroyed**: No one truly owns their data, their communications, or their digital identity. Access can be revoked at any time by entities the user never agreed to depend on.
- **Communication as hostage**: Speaking to a loved one — a mother, a spouse, a child — should cost nothing and depend on no one. Speech is a planetary right. Yet every private message transits through servers owned by corporations with their own agendas.
- **Ecological absurdity**: Massive data centers consume enormous energy to store, process, and monetize data that users never consented to share. The Web 4.0 trajectory perpetuates this unsustainable model.
- **Systemic fragility**: When WhatsApp goes down, billions lose communication. When a government blocks a platform, millions are silenced. Single points of failure are single points of control.

### Why Existing Solutions Fall Short

Every existing alternative carries the same structural flaw: **someone must pay for the infrastructure, someone must maintain it, and someone can cut it.**

- **libp2p**: Toolkit, not a protocol. No consensus, no economy, assembly required.
- **BitTorrent**: Designed for files, not real-time communication. Depends on centralized trackers. Seeding is optional — so people don't seed.
- **IPFS**: Storage, not transport. Slow for real-time. No purge mechanism. Filecoin added speculation without solving the core problem.
- **Nostr**: Depends on volunteer relays that can censor and that nobody pays for.
- **Matrix**: Federated servers are still servers. Someone must maintain and fund them.
- **Signal, Telegram, WhatsApp**: All ultimately depend on central infrastructure, corporate decisions, or capitulation to government pressure.

These systems separate users from infrastructure. ToM refuses this separation.

### Proposed Solution

ToM is a transport protocol — not an application — that turns every connected device into the network itself. A smartphone, a router, a payment terminal, an internet box, a connected fridge — anything with a CPU, connectivity, and minimal storage becomes a node.

Core design principles:
- **Use it = contribute to it**: Participation is automatic and invisible. No opt-in, no volunteer relays, no "please seed." The network assigns roles dynamically and unpredictably.
- **No servers, not even federated**: The infrastructure IS the users. No one to maintain, no one to pay, no one to pressure.
- **Purge by default**: Messages delivered are deleted. Past state is compacted then erased. The network stays light regardless of age.
- **Invisible integration**: Delivered as an SDK/protocol layer. Users open their app, it works. Zero configuration. Zero friction.
- **Built for communication from day one**: Not a file system adapted for messaging, not a blockchain repurposed for transport. A data BUS designed for real-time byte transport.
- **Nothing to steal**: No speculative token, no centralized data store, no convertible asset. Attacking ToM yields nothing.

The protocol transports bytes. Period. From A to B, without asking anyone's permission, without paying anyone. Messaging, storage, streaming, services — these are layers above. ToM's job is one byte to another, free, sovereign.

### Key Differentiators

1. **Mandatory contribution**: The network uses you as you use it. Proportional to device capability. No free riders.
2. **Imposed roles, not chosen**: Unlike BitTorrent seeding or Nostr relay volunteering, ToM assigns roles at the last moment, unpredictably. Impossible to game.
3. **Zero infrastructure dependency**: No servers, no relays, no bootstrap nodes controlled by a single entity. The hardware already exists in every pocket, every home, every connected object.
4. **Non-speculative economy**: Internal balance of contribution/usage. No token to buy, sell, or speculate on. The score is not an asset — it is a measure of equilibrium.
5. **Designed around human nature**: Does not ask humans to be altruistic. Makes contribution automatic and invisible. The system works because it does not trust its participants to volunteer.
6. **Radical open source**: No monetization, no foundation seeking funding, no corporate capture. Community-verified, community-evolved. AI tools now make it possible for a single developer to build what previously required teams of fifty.
7. **Timing**: The convergence of ubiquitous connected devices, underutilized computing power (an iPhone 18's processing power is colossal and largely idle), and AI-powered development tools means this protocol can be built now, by anyone, from anywhere. There are no more excuses.

---

## Target Users

### Primary Users

**Persona 1 — "Alex", the App Developer**
Alex is a backend/fullstack developer building a messaging or collaboration tool. Today, Alex rents servers, manages infrastructure, handles scaling, and pays growing cloud bills. Alex is tired of being the middleman between users who just want to talk to each other. With ToM's SDK, Alex plugs in a protocol layer, removes the servers, and lets users communicate directly. Alex's infrastructure cost drops by orders of magnitude. Alex's app becomes truly resilient — no single point of failure, no government pressure point, no data liability.

**Persona 2 — "The Enterprise" — Communication Platforms**
Companies like Slack, email providers, messaging startups. They spend millions on data centers to relay messages between users. ToM offers them a radical proposition: integrate the SDK, eliminate server infrastructure for message transport, and divide data center costs by 100. As a bonus, they can truthfully market themselves as decentralized — no data to hand over to governments, no liability for stored messages. Apple could claim iMessage is truly decentralized, where no country could demand the keys.

### Secondary Users

**Persona 3 — "Leila", the Invisible Beneficiary**
Leila is a mother, a spouse, a child — anyone who opens an app and sends a message. Leila has never heard of ToM. She never will. She opens her messaging app, types "I love you", hits send. It arrives. She doesn't know her message traveled through a mesh of devices with no central server. She doesn't know her phone relayed three other messages while she was typing. She doesn't care. It just works.

**Persona 4 — "Reza", the Citizen Under Constraint**
Reza is a journalist in Tehran, a protester in Yangon, a trucker in Ottawa. When the government shuts down Telegram, blocks Signal, freezes bank accounts — Reza needs a communication channel that no authority can cut. ToM has no server to seize, no company to pressure, no kill switch. For Reza, ToM is not a convenience. It is survival.

### User Journey

**Phase 1 — The Proof of Concept (Discovery)**
A developer or curious user opens a web page. No installation. A simple input field: choose a target, type a message, send. The page visualizes in real-time the message's journey — hopping through nodes, being validated, arriving. Not a simulation. Real bytes, real network. The "aha!" moment: this works, and there's no server anywhere.

**Phase 2 — The Browser Extension (Early Adoption)**
A browser extension turns Chrome or Firefox into a persistent ToM node. Install the extension, your browser joins the network. You browse normally while participating in the background — relaying, validating, contributing. The extension also acts as a trust barometer: a visible indicator on every website showing whether the site uses ToM for its data transport or relies on centralized servers. Like the HTTPS padlock, but for data sovereignty. Users see which services respect their data and which don't. This creates bottom-up pressure: users demand that their favorite hardware store's online chat, their bank's customer service, their kid's school platform adopt ToM. The awareness comes from below — not from marketing, but from visibility.

**Phase 3 — The SDK (Developer Adoption)**
Developers integrate the ToM SDK into their applications. Mobile apps, desktop apps, IoT devices. The protocol becomes a library call. Every app that integrates ToM adds nodes to the network. The network grows with every integration.

**Phase 4 — The Invisible Layer (Mass Adoption)**
Manufacturers embed ToM in routers, internet boxes, connected devices. Operating systems integrate it natively. Users never know it exists. ToM becomes like TCP/IP — invisible, universal, essential. The network is everywhere because everyone is the network.

---

## Success Metrics

### Philosophy

ToM has no business objectives, no revenue targets, no growth KPIs. It is a common good. If it is not used, it dies — and that means humanity was not ready. Success cannot be forced. It can only be observed.

The only meaningful measure of success: the SDK is used across the world, and the source code is in perpetual community-driven evolution.

### Network Health Metrics (Self-Monitoring)

These are not goals to achieve. They are vital signs the protocol uses to self-regulate, fork, or adapt:

- **Message delivery rate**: Percentage of messages successfully delivered. The network monitors its own failure rate and adjusts routing, role assignment, and quorum size accordingly.
- **Delivery latency**: Time from send to receive. Not a target — a pulse. If latency degrades, the network rebalances.
- **Active node count**: How many devices are participating. Not a growth target — a measure of network density and resilience capacity.
- **Contribution/usage equilibrium**: Global distribution of scores. A healthy network has most participants near zero. Skew indicates systemic imbalance.
- **Resilience under node loss**: How the network performs when 10%, 30%, 50% of nodes drop simultaneously. Self-tested, self-reported.
- **Fork events**: Number and nature of subnet forks. Forks are not failures — they are the network breathing. But their frequency and triggers are diagnostic.

### Adoption Signals (Observed, Not Targeted)

- SDK downloads and integrations across ecosystems
- Number of community contributors and commit frequency on core repository
- Browser extension installations and blue padlock visibility across the web
- Diversity of implementations (languages, platforms, device types)

### Resilience Principles

- **Self-hosting**: Once critical mass is reached, ToM's own source code, documentation, and development workflow are distributed on the ToM network itself. If GitHub falls, the code lives on. The protocol hosts its own evolution.
- **Open contribution**: Any developer can submit features, patches, and fixes. Core maintainers rotate. No single entity controls the roadmap. No money means no capture.
- **Unstoppable by design**: Once launched with sufficient nodes, ToM cannot be shut down — only evolved. Like Bitcoin's network, but without the financial incentive that concentrates power. It belongs to everyone and to no one.

### What Success Looks Like

Success is not a number. It is a state:
- A message travels from A to B with no server, no intermediary, no fee, no trace.
- The code evolves without any single entity controlling it.
- The network sustains itself without anyone maintaining it.
- Nobody knows they are using ToM. And that is exactly why it works.
- Once launched, the only thing that can happen to it is evolution.

---

## MVP Scope

### Philosophy

The MVP follows a biological growth model: start as a single cell, prove it lives, then let it evolve. Each iteration adds one capability. No big bang. No feature bloat. Prove the core, then grow.

### Core Features — PoC (Iteration 0)

The absolute minimum: **a real message travels from browser A to browser B with no central server.**

- **Two browsers, one message**: User A opens a web page, types a message, selects a target (User B on another browser). The message arrives. No installation. No extension. Just a web page.
- **WebRTC peer-to-peer transport**: Direct browser-to-browser communication. The message does not transit through any application server.
- **Visual journey tracking**: The UI shows in real-time how the message travels — hops, validation steps, delivery confirmation. Not a simulation. Real data, real path.
- **Minimal signaling bootstrap**: A lightweight signaling mechanism for initial peer discovery. This is the one acceptable temporary compromise — a minimal coordination point that will be eliminated as the network grows.

### Incremental Iterations (Post-PoC, Pre-SDK)

Each iteration adds one layer, proven before moving to the next:

1. **Iteration 1 — Relay nodes**: Message goes from A to B through relay node C. Proof that multi-hop works. A → C → B.
2. **Iteration 2 — Dynamic roles**: Nodes are assigned roles (client, relay) dynamically. Not chosen. The network decides.
3. **Iteration 3 — E2E encryption**: Cryptographic key exchange and encrypted message transport. No node in the chain can read the content.
4. **Iteration 4 — Contribution tracking**: Basic contribution/usage score. The network begins to self-monitor balance.
5. **Iteration 5 — Subnet formation**: Ephemeral subnets form and dissolve based on communication needs.
6. **Iteration 6 — Purge mechanism**: Delivered messages are deleted. State compaction begins. The network forgets.

### Out of Scope for MVP

- **Browser extension**: Phase 2, after the SDK is proven
- **Full Proof of Presence consensus**: Requires network scale; simplified validation for MVP
- **Packaged SDK**: The MVP IS the proto-SDK; packaging comes after protocol validation
- **Mobile/IoT integration**: Desktop browsers first
- **Blue padlock indicator**: Requires extension; post-MVP
- **Self-hosting of source code on ToM**: Requires critical mass; long-term goal
- **Multi-language library support**: Single implementation first (JavaScript/TypeScript for browser compatibility)
- **L1 engine implementation**: Architecture and development phase; not part of the PoC brief

### MVP Success Criteria

The PoC is validated when:
- A message travels from browser A to browser B with zero server infrastructure
- The visual trace shows the real path taken
- Adding a third node as relay works without reconfiguration
- Closing a node does not crash the network — messages reroute
- The demo is reproducible by anyone opening the web page

### Future Vision

Once the PoC lives, the path is:
1. **SDK extraction**: Separate the protocol layer from the demo UI into a standalone SDK
2. **Browser extension**: Persistent node that runs in background with blue padlock trust indicator
3. **Native implementations**: Rust/C core for IoT, routers, embedded devices
4. **Network scale**: From 3 nodes to 30 to 300 to 3 million — each threshold unlocks new protocol capabilities
5. **Self-evolution**: The network hosts its own code, its own governance, its own evolution. Once launched, it can only be evolved, never stopped.
