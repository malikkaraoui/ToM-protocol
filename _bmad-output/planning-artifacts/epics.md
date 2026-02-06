---
stepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-create-stories, step-04-final-validation]
status: complete
completedAt: 2026-02-02
inputDocuments:
  - prd.md
  - architecture.md
---

# tom-protocol - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for tom-protocol, decomposing the requirements from the PRD and Architecture into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: A sender can transmit a message to a recipient via a relay node
FR2: A relay node can forward messages between two participants who don't have a direct connection
FR3: A recipient can receive messages from any sender routed through the network
FR4: A sender can receive delivery confirmation when a message reaches the relay
FR5: A recipient can send a direct acknowledgment back to the sender
FR6: Two participants can establish a direct communication path after initial relay introduction
FR7: The network can dynamically select a relay when the sender doesn't know one
FR8: A message can traverse multiple relays to reach its destination
FR9: The network can reroute a message through an alternate relay if the primary relay fails
FR10: A sender can encrypt messages end-to-end so that relays cannot read content
FR11: A user can see a list of all currently connected participants on the network
FR12: A user can select a participant from the list and initiate a conversation
FR13: A user can choose a display username when joining the network
FR14: A user can optionally view message path details (relays used, forks, timers, delivery status)
FR15: A user can see delivery confirmation and read receipts for sent messages
FR16: A node can join the network and be assigned a role dynamically (client, relay, observer, guardian, validator)
FR17: A node can discover other participants through the network's peer discovery mechanism
FR18: A node can function simultaneously as client (sending/receiving) and relay (forwarding for others)
FR19: The network can survive individual node failures without manual intervention (at alpha scale)
FR20: A node can participate in the bootstrap phase through multiple vectors (seed servers, browser tabs, SDK integration)
FR21: A new node can discover the network through a minimal signaling bootstrap mechanism
FR22: The bootstrap mechanism can be isolated, documented, and marked as temporary
FR23: A browser tab can act as a persistent network node contributing to bootstrap
FR24: The network can progressively transition from bootstrap-dependent to autonomous discovery
FR25: A developer can install the raw protocol library via package manager (tom-protocol)
FR26: A developer can install the plug-and-play SDK via package manager (tom-sdk)
FR27: A developer can connect to the network, send, and receive messages with the raw protocol API
FR28: A developer can integrate messaging in 2 lines of code using the SDK abstraction
FR29: Each language implementation can be validated against a protocol compliance test suite
FR30: The SDK can handle relay selection, encryption, and retry transparently
FR31: An LLM can discover ToM's capabilities through structured documentation (llms.txt, CLAUDE.md)
FR32: An LLM can interact with the ToM network programmatically via MCP Server
FR33: A developer can discover and quick-start ToM from a VS Code plugin
FR34: A developer can access a live demo of the protocol on the project website
FR35: An LLM can suggest ToM integration to developers based on structured package registry presence
FR36: A contributor can find LLM-friendly, scoped issues tagged by complexity in the repository
FR37: A contributor can complete a meaningful contribution in a 30-minute micro-session
FR38: A contributor can follow clear contributing guidelines that support the micro-session model
FR39: The project can validate contributions through automated testing gates
FR40: The repository can maintain a permanent backlog of available work across verification, building, analysis, and testing
FR41: A node can view its current contribution/usage balance score
FR42: The network can calculate and maintain a contribution/usage equilibrium score for each participant
FR43: A user can see their currently assigned dynamic role in the interface (client, relay, observer, guardian, validator)
FR44: The network can form ephemeral subnets and autonomously dissolve them when they are no longer useful (sliding genesis, organic lifecycle)
FR45: A node returning online can be reassigned a role and receive any pending messages that were in transit for it

### NonFunctional Requirements

NFR1: End-to-end message delivery (sender → relay → recipient) must complete in under 500ms under normal network conditions
NFR2: Peer discovery and role assignment must complete within 3 seconds of a node joining the network
NFR3: Relay failover and message rerouting must be transparent to the user with no manual intervention
NFR4: Relay nodes must never persist message content — relay role is pass-through only (find target, forward, forget)
NFR5: When a message cannot be delivered (recipient offline), backup nodes can store the message redundantly across multiple locations for a maximum of 24 hours, after which it is deleted regardless of delivery status
NFR6: From iteration 5 onward, all message content must be encrypted end-to-end — relays and backup nodes can only see routing metadata, never content
NFR7: No central authority can access, intercept, or compel disclosure of message content (architectural guarantee, not policy)
NFR8: Security posture grows progressively per iteration — cleartext accepted for iterations 1-4, mandatory E2E from iteration 5
NFR9: PoC target: 10-15 simultaneous nodes with no performance degradation
NFR10: Architecture must embody the inversion property — performance improves as node count increases (more relays = faster routing = lower latency)
NFR11: Specific scalability thresholds beyond PoC will be defined based on empirical testing data from iterations 1-6
NFR12: SDK installation and first message sent must be achievable in under 5 minutes for a developer familiar with npm
NFR13: Protocol API must remain language-agnostic in design — TypeScript is first implementation, not specification
NFR14: MCP Server must respond to LLM tool calls with structured, parseable output

### Additional Requirements

- No starter template — manual monorepo scaffold as first implementation story (Architecture)
- pnpm monorepo: packages/core + packages/sdk + apps/demo + tools/signaling-server (Architecture)
- 9 ADRs to follow strictly, including ADR-009 message survival/virus metaphor (Architecture)
- Event-driven unified node model — all components communicate via typed EventEmitter (Architecture)
- Implementation patterns enforced: kebab-case files, PascalCase classes, TomError for errors, co-located tests, async/await only (Architecture)
- Strict dependency direction: demo → sdk → core, never reversed (Architecture)
- Signaling server is independent, temporary, marked for elimination (Architecture)
- Progressive iteration model: each iteration builds on previous, no skipping (PRD)

### FR Coverage Map

FR1: Epic 2 - Send message via relay
FR2: Epic 2 - Relay forwards between participants
FR3: Epic 2 - Recipient receives routed messages
FR4: Epic 2 - Delivery confirmation from relay
FR5: Epic 2 - Direct acknowledgment from recipient
FR6: Epic 4 - Direct path after relay introduction
FR7: Epic 3 - Dynamic relay selection
FR8: Epic 5 - Multi-relay message traversal
FR9: Epic 5 - Reroute on relay failure
FR10: Epic 6 - E2E encryption
FR11: Epic 2 - See connected participants
FR12: Epic 2 - Select participant to chat
FR13: Epic 2 - Choose display username
FR14: Epic 4 - View message path details
FR15: Epic 4 - Delivery confirmation and read receipts
FR16: Epic 3 - Dynamic role assignment
FR17: Epic 3 - Peer discovery
FR18: Epic 3 - Simultaneous client and relay
FR19: Epic 5 - Survive node failures
FR20: Epic 3 - Bootstrap participation vectors
FR21: Epic 2 - Signaling bootstrap discovery
FR22: Epic 1 - Bootstrap documented as temporary
FR23: Epic 2 - Browser tab as network node
FR24: Epic 7 - Transition to autonomous discovery
FR25: Epic 1 - Install raw protocol library
FR26: Epic 6 - Install SDK package
FR27: Epic 1 - Connect, send, receive with protocol API (partial)
FR28: Epic 6 - 2-line SDK integration
FR29: Epic 6 - Protocol compliance test suite
FR30: Epic 6 - SDK transparent relay/encryption/retry
FR31: Epic 8 - LLM discovery via llms.txt/CLAUDE.md
FR32: Epic 8 - MCP Server for LLM interaction
FR33: Epic 8 - VS Code plugin
FR34: Epic 8 - Live demo on website
FR35: Epic 8 - LLM suggests ToM via package registry
FR36: Epic 8 - LLM-friendly scoped issues
FR37: Epic 8 - 30-minute micro-session contribution
FR38: Epic 8 - Contributing guidelines for micro-sessions
FR39: Epic 8 - Automated testing gates
FR40: Epic 8 - Permanent backlog of available work
FR41: Epic 5 - View contribution/usage score
FR42: Epic 5 - Calculate contribution equilibrium
FR43: Epic 5 - See assigned dynamic role
FR44: Epic 7 - Ephemeral subnets with sliding genesis
FR45: Epic 4 - Reconnection and pending message delivery

## Epic List

### Epic 1: Project Foundation & Node Identity
A developer can clone the repo, build, and run the project. A node can generate and persist its unique identity on the network.
**FRs covered:** FR22, FR25, FR27 (partial)
**NFRs:** NFR12 (partial), NFR13

### Epic 2: First Message Through a Relay (Iteration 1)
A user can send a message to a recipient via a known relay. The relay forwards without storing. The demo shows it visually. The "wow moment" — bytes traversing a relay with no application server.
**FRs covered:** FR1, FR2, FR3, FR4, FR5, FR11, FR12, FR13, FR21, FR23
**NFRs:** NFR1, NFR4

### Epic 3: Dynamic Routing & Network Discovery (Iteration 2)
The sender doesn't know the relay. The network finds the path. Roles are assigned dynamically. A node functions simultaneously as client and relay.
**FRs covered:** FR7, FR16, FR17, FR18, FR20
**NFRs:** NFR2

### Epic 4: Bidirectional Conversation (Iteration 3)
Two users can have a back-and-forth conversation. After relay introduction, a direct path A↔B can be established. A reconnecting node receives pending messages.
**FRs covered:** FR6, FR14, FR15, FR45
**NFRs:** NFR5

### Epic 5: Resilient Multi-Relay Transport (Iteration 4)
A message can traverse multiple relays. If a relay drops, the network reroutes automatically. Contribution tracking begins. Users can see their role and score.
**FRs covered:** FR8, FR9, FR19, FR41, FR42, FR43
**NFRs:** NFR3, NFR9, NFR10

### Epic 6: Private Communication (Iteration 5)
E2E encryption — relays can no longer read content. Only sender and recipient see the message. The SDK handles encryption/decryption transparently. SDK packaging complete.
**FRs covered:** FR10, FR26, FR28, FR29, FR30
**NFRs:** NFR6, NFR7, NFR8

### Epic 7: Self-Sustaining Alpha Network (Iteration 6)
10-15 simultaneous nodes. Ephemeral subnets. The network begins its transition toward autonomy. Bootstrap fades progressively.
**FRs covered:** FR24, FR44
**NFRs:** NFR11

### Epic 8: LLM & Community Ecosystem (Post-MVP)
LLM-first docs, MCP server, VS Code plugin, micro-session contribution model. The protocol spreads through the tools developers already use.
**FRs covered:** FR31, FR32, FR33, FR34, FR35, FR36, FR37, FR38, FR39, FR40
**NFRs:** NFR14

## Epic 1: Project Foundation & Node Identity

A developer can clone the repo, build, and run the project. A node can generate and persist its unique identity on the network.

### Story 1.1: Monorepo Scaffold

As a developer,
I want to clone the repo and have a working monorepo with all packages building and testing,
So that I have a solid foundation to start implementing protocol features.

**Acceptance Criteria:**

**Given** a fresh clone of the repository
**When** I run `pnpm install && pnpm build`
**Then** all packages (core, sdk, demo, signaling-server) build successfully with zero errors
**And** `pnpm test` runs vitest across all packages with a passing placeholder test in each
**And** `pnpm lint` runs biome with zero warnings
**And** the dependency direction is enforced: demo → sdk → core, never reversed
**And** TypeScript strict mode is enabled in all packages
**And** the signaling-server package contains a comment marking it as temporary (ADR-002)

### Story 1.2: Node Identity Generation & Persistence

As a network participant,
I want my node to generate a unique Ed25519 keypair on first launch and persist it,
So that I have a stable cryptographic identity across sessions.

**Acceptance Criteria:**

**Given** a node starts for the first time with no existing identity
**When** the identity module initializes
**Then** an Ed25519 keypair is generated using TweetNaCl.js
**And** the keypair is persisted to the configured storage (localStorage in browser, file in Node.js)
**And** the node's public key serves as its unique network identifier

**Given** a node starts with an existing persisted identity
**When** the identity module initializes
**Then** the existing keypair is loaded from storage without generating a new one
**And** the node's network identifier remains the same as previous sessions

**Given** the identity module is asked to sign data
**When** a valid payload is provided
**Then** it returns a valid Ed25519 signature
**And** the signature can be verified using the node's public key

### Story 1.3: Shared Types & Error Foundation

As a developer,
I want shared TypeScript types for MessageEnvelope, TomError, TomErrorCode, and event definitions,
So that all packages use a consistent type system from day one.

**Acceptance Criteria:**

**Given** the core package is imported by another package
**When** the developer accesses the public API
**Then** MessageEnvelope interface is available with fields: id, from, to, via, type, payload, timestamp, signature
**And** TomError class extends Error with code (TomErrorCode) and optional context
**And** TomErrorCode union type includes: TRANSPORT_FAILED, PEER_UNREACHABLE, SIGNALING_TIMEOUT, INVALID_ENVELOPE, IDENTITY_MISSING, RELAY_REJECTED, CRYPTO_FAILED
**And** event type definitions are available for the typed EventEmitter pattern
**And** all types are exported as ESM and CJS via tsup dual build

## Epic 2: First Message Through a Relay (Iteration 1)

A user can send a message to a recipient via a known relay. The relay forwards without storing. The demo shows it visually. The "wow moment" — bytes traversing a relay with no application server.

### Story 2.1: Signaling Server Bootstrap

As a new node joining the network,
I want to connect to a WebSocket signaling server to discover other peers,
So that I can find participants to communicate with.

**Acceptance Criteria:**

**Given** the signaling server is running
**When** a node connects via WebSocket
**Then** the server registers the node with its public key and chosen username
**And** the server broadcasts the updated participant list to all connected nodes
**And** when a node disconnects, the participant list is updated and broadcast

**Given** a node needs to establish a WebRTC connection with another node
**When** it sends an SDP offer/answer or ICE candidate through the signaling server
**Then** the signaling server relays the message to the target node without inspecting content
**And** the signaling server never stores any signaling data after forwarding

### Story 2.2: WebRTC DataChannel Transport

As a network node,
I want to establish a WebRTC DataChannel connection with another peer via signaling,
So that I have a direct, low-latency, browser-native transport for sending bytes.

**Acceptance Criteria:**

**Given** two nodes are connected to the signaling server
**When** node A initiates a connection to node B
**Then** an SDP offer/answer exchange completes via the signaling server
**And** ICE candidates are exchanged and a peer connection is established
**And** a reliable DataChannel is opened for message transport

**Given** an established DataChannel between two nodes
**When** one node sends a MessageEnvelope
**Then** the other node receives the complete envelope with no corruption
**And** delivery completes in under 500ms under normal network conditions (NFR1)

**Given** a DataChannel connection drops unexpectedly
**When** the transport layer detects the disconnection
**Then** a TRANSPORT_FAILED TomError is emitted via the event system
**And** the node is removed from the local peer list

### Story 2.3: Message Routing Through a Known Relay

As a sender,
I want to send a message to a recipient through a known relay node,
So that my message reaches someone I'm not directly connected to.

**Acceptance Criteria:**

**Given** sender A, relay R, and recipient B are all connected
**When** A sends a MessageEnvelope addressed to B with R in the via field
**Then** R receives the envelope, verifies the destination is B, and forwards it to B
**And** R does not persist the message content at any point (NFR4)
**And** R sends a delivery confirmation back to A indicating the message was forwarded

**Given** a relay receives a message for an unknown recipient
**When** the recipient is not in the relay's peer list
**Then** the relay responds with a PEER_UNREACHABLE error to the sender
**And** the message is discarded (not stored)

**Given** a message envelope has an invalid or missing signature
**When** the relay attempts to verify it
**Then** the relay rejects the envelope with an INVALID_ENVELOPE error
**And** the message is not forwarded

### Story 2.4: Recipient Acknowledgment

As a sender,
I want to receive an acknowledgment from the recipient confirming they received my message,
So that I know my message was delivered end-to-end, not just to the relay.

**Acceptance Criteria:**

**Given** recipient B receives a message from sender A via relay R
**When** B's node processes the incoming message
**Then** B's node automatically sends an ACK envelope back to A (routed via R)
**And** A's node emits a message-acknowledged event with the original message ID

**Given** the ACK envelope fails to reach A (relay disconnected)
**When** the transport error is detected
**Then** A's node does not crash or block — the message remains in "delivered to relay" status
**And** a warning event is emitted for potential UI display

### Story 2.5: Demo Chat UI — Lobby & Messaging

As a user,
I want to open the demo in a browser, choose a username, see connected participants, and send/receive messages through a relay,
So that I can experience the protocol working visually.

**Acceptance Criteria:**

**Given** a user opens the demo app in a browser
**When** the page loads
**Then** a username input is displayed
**And** after entering a username, the node connects to the signaling server and joins the network

**Given** the user has joined the network
**When** other participants are connected
**Then** a participant list is displayed showing all online usernames
**And** the list updates in real-time as participants join or leave

**Given** the user selects a participant from the list
**When** they type a message and send it
**Then** the message is routed through an available relay node
**And** the message appears in both sender and recipient chat windows
**And** delivery status is shown (sent → relayed → delivered)

**Given** the demo is built with vanilla HTML/JS (no framework)
**When** inspecting the source
**Then** it imports from the sdk package only
**And** the UI is functional but minimal — demonstrating protocol, not design

## Epic 3: Dynamic Routing & Network Discovery (Iteration 2)

The sender doesn't know the relay. The network finds the path. Roles are assigned dynamically. A node functions simultaneously as client and relay.

### Story 3.1: Peer Discovery Protocol

As a network node,
I want to discover other participants beyond my direct connections,
So that I can find potential relays and recipients across the network.

**Acceptance Criteria:**

**Given** a node is connected to the network
**When** a new node joins or an existing node updates its presence
**Then** the discovery mechanism propagates presence information to reachable peers
**And** each node maintains an up-to-date network topology map

**Given** a node goes offline
**When** its heartbeat stops or connection drops
**Then** the discovery mechanism propagates the departure within 3 seconds
**And** all nodes remove it from their topology map

**Given** the network has more participants than a single node's direct connections
**When** the node queries the discovery layer
**Then** it receives information about indirect peers (reachable through relays)
**And** the information includes the peer's public key, username, and reachable relay path

### Story 3.2: Dynamic Role Assignment

As a node joining the network,
I want to be automatically assigned a role based on my capabilities and network needs,
So that the network self-organizes without manual configuration.

**Acceptance Criteria:**

**Given** a new node joins the network
**When** role assignment evaluates the node
**Then** a role is assigned (client, relay, observer) within 3 seconds of joining (NFR2)
**And** the role is communicated to the node and broadcast to the network

**Given** the network lacks relay capacity
**When** a capable node is evaluated for role assignment
**Then** the node is assigned the relay role in addition to its client role
**And** the node begins accepting forwarding requests

**Given** network conditions change (nodes join/leave)
**When** the role assignment system re-evaluates
**Then** roles can be reassigned dynamically without node restart
**And** role transitions are seamless — no messages lost during transition

### Story 3.3: Automatic Relay Selection

As a sender,
I want the network to automatically select the best relay for my message,
So that I don't need to know the network topology to send a message.

**Acceptance Criteria:**

**Given** sender A wants to send a message to recipient B
**When** A does not specify a relay in the via field
**Then** the routing layer selects the best available relay based on network topology
**And** the message is sent through the selected relay transparently

**Given** multiple relays are available to route a message
**When** the routing layer selects a relay
**Then** it chooses based on proximity (fewest hops) and availability
**And** the selected relay is populated in the via field before sending

**Given** no relay is available to reach the recipient
**When** the routing layer fails to find a path
**Then** a PEER_UNREACHABLE error is returned to the sender
**And** the error includes context about why no path was found

### Story 3.4: Dual-Role Node (Client + Relay)

As a network participant,
I want my node to simultaneously send/receive my own messages and relay messages for others,
So that the network grows stronger as more participants join.

**Acceptance Criteria:**

**Given** a node is assigned both client and relay roles
**When** it receives a message addressed to another node
**Then** it forwards the message as a relay without interfering with its own messaging
**And** relay forwarding and personal messaging share the transport layer without conflicts

**Given** a dual-role node is sending its own message
**When** a relay request arrives simultaneously
**Then** both operations complete without blocking each other
**And** the event system handles both message flows independently

**Given** a dual-role node's relay responsibilities increase
**When** the node detects performance degradation for its own messages
**Then** it emits a capacity warning event
**And** the role system can redistribute relay duties to other capable nodes

### Story 3.5: Bootstrap Participation Vectors

As a node operator,
I want multiple ways to bootstrap into the network (seed server, browser tab, SDK),
So that joining the network is flexible and not limited to a single entry point.

**Acceptance Criteria:**

**Given** a developer integrates the SDK into their application
**When** the SDK initializes with a signaling server URL
**Then** the node bootstraps into the network via the signaling server
**And** the bootstrap mechanism is abstracted — the developer doesn't manage WebSocket connections

**Given** a user opens the demo in a browser tab
**When** the tab connects to the network
**Then** the tab acts as a persistent network node contributing to bootstrap
**And** the tab can serve as a relay for other nodes while active

**Given** the bootstrap mechanism is implemented
**When** a developer reviews the codebase
**Then** all bootstrap code is clearly isolated in dedicated modules
**And** each bootstrap module contains documentation marking it as temporary (ADR-002)
**And** the architecture supports future replacement without affecting core protocol

## Epic 4: Bidirectional Conversation (Iteration 3)

Two users can have a back-and-forth conversation. After relay introduction, a direct path A↔B can be established while both are online. If either goes offline, the network falls back to relay routing and backup storage. A reconnecting node receives pending messages.

### Story 4.1: Direct Path Establishment (Relay Bypass)

As a user in a conversation,
I want my node to establish a direct WebRTC connection with my conversation partner after relay introduction,
So that our messages travel faster without burdening the relay.

**Acceptance Criteria:**

**Given** user A and user B have exchanged messages through relay R
**When** both A and B are online and reachable
**Then** A's node initiates a direct WebRTC DataChannel to B (bypassing R)
**And** subsequent messages between A and B travel directly without passing through R

**Given** a direct path is established between A and B
**When** one of them goes offline or the direct connection drops
**Then** the transport layer detects the disconnection
**And** messages automatically fall back to relay routing
**And** the fallback is transparent — no user action required, no messages lost during transition

**Given** a direct path was previously active and both nodes come back online
**When** the nodes detect each other's presence via discovery
**Then** the direct path is re-established automatically
**And** relay routing is released once the direct path is confirmed

### Story 4.2: Delivery Confirmation & Read Receipts

As a sender,
I want to see the full lifecycle status of my message (sent → relayed → delivered → read),
So that I know exactly what happened to my message.

**Acceptance Criteria:**

**Given** sender A sends a message to recipient B
**When** the message is sent
**Then** A sees status "sent"
**And** when the relay confirms forwarding, status updates to "relayed"
**And** when B's node receives the message, status updates to "delivered"
**And** when B opens/views the message, status updates to "read"

**Given** B reads a message from A
**When** B's node detects the message was displayed
**Then** a read receipt envelope is sent back to A (via direct path or relay)
**And** A's node emits a message-read event with the original message ID

**Given** a read receipt fails to reach A
**When** the transport encounters an error
**Then** the message remains in "delivered" status — no false "read" status shown
**And** the receipt is not retried (best-effort delivery for receipts)

### Story 4.3: Message Path Visualization

As a user,
I want to optionally view the path my message took through the network,
So that I can understand the protocol's routing in action.

**Acceptance Criteria:**

**Given** a message has been delivered
**When** the user activates path details view (toggle or click)
**Then** the UI shows: relays used (via field), direct vs relayed, delivery timing
**And** the path information is derived from envelope metadata — no extra network requests

**Given** a message traveled through a direct path
**When** the user views path details
**Then** the display shows "Direct" with no relay hops
**And** the timing shows the reduced latency compared to relayed messages

**Given** path visualization is optional
**When** the user has not activated it
**Then** no path information is displayed — the chat remains clean and simple

### Story 4.4: Reconnection & Pending Message Delivery

As a user who went offline temporarily,
I want to receive messages that were sent to me while I was away,
So that I don't miss any communication.

**Acceptance Criteria:**

**Given** recipient B goes offline while sender A sends messages
**When** the network detects B is unreachable
**Then** backup nodes store the messages redundantly across multiple locations (ADR-009)
**And** each message monitors its own viability score and replicates to better hosts proactively
**And** messages self-delete when their score drops below threshold — before the host dies

**Given** recipient B comes back online
**When** B's node reconnects to the network
**Then** B's node queries the network for pending messages
**And** backup nodes deliver the stored messages to B
**And** a "received" signal propagates through the network to clear all backup copies

**Given** a message has been stored for more than 24 hours
**When** the TTL expires
**Then** the message is deleted from all backup nodes regardless of delivery status (NFR5)
**And** no trace of the message content remains on any backup node

### Story 4.5: Demo Snake — Multiplayer P2P Game

As a user in the demo app,
I want to invite a chat participant to a real-time multiplayer Snake game in the same window,
So that the protocol's bidirectional transport is demonstrated with a fun, interactive experience.

**Acceptance Criteria:**

**Given** two users are chatting in the demo app
**When** user A sends a game invitation to user B
**Then** B sees the invitation in the chat window
**And** B can accept or decline the invitation

**Given** both users accept the game
**When** the game starts
**Then** a Snake game canvas renders in the same demo window (alongside or replacing the chat view)
**And** both players control their own snake on the same shared game field
**And** game state updates are transmitted via the direct path (or relay fallback) in real-time

**Given** the game is running
**When** a player's snake collides with the other snake or a wall
**Then** the game ends and both players see the result simultaneously
**And** the game result is sent as a chat message (e.g., "Player A won!")
**And** the view returns to chat mode

**Given** the direct connection drops during a game
**When** the transport falls back to relay
**Then** the game continues with potentially higher latency but no crash
**And** a visual indicator shows the connection quality change

## Epic 5: Resilient Multi-Relay Transport (Iteration 4)

A message can traverse multiple relays. If a relay drops, the network reroutes automatically. Contribution tracking begins. Users can see their role and score.

### Story 5.1: Multi-Relay Message Traversal

As a sender,
I want my message to traverse multiple relays to reach a distant recipient,
So that communication works even when no single relay connects sender and recipient.

**Acceptance Criteria:**

**Given** sender A and recipient B have no common relay
**When** A sends a message to B
**Then** the routing layer computes a multi-hop path through 2+ relays
**And** the via field contains the ordered chain of relay public keys
**And** each relay forwards to the next relay in the chain until B is reached

**Given** a message is traversing a multi-relay path
**When** each relay processes the envelope
**Then** it forwards to the next hop only — not to all known peers
**And** no relay in the chain persists the message content (NFR4)
**And** the sender receives a delivery confirmation once the final relay delivers to B

**Given** the network topology changes during transit
**When** a relay in the chain is no longer optimal
**Then** the message continues on the original path (no mid-transit rerouting in this story)

### Story 5.2: Automatic Rerouting on Relay Failure

As a sender,
I want the network to reroute my message through an alternate relay if the primary one fails,
So that relay failures don't prevent message delivery.

**Acceptance Criteria:**

**Given** a message is in transit through relay R1
**When** R1 goes offline or becomes unreachable
**Then** the sending node detects the failure
**And** the routing layer selects an alternate path avoiding R1
**And** the message is resent through the new path automatically

**Given** rerouting occurs
**When** the message reaches the recipient through the alternate path
**Then** the recipient receives exactly one copy of the message (deduplication by message ID)
**And** the sender receives a delivery confirmation as normal
**And** the rerouting is transparent to the user (NFR3)

**Given** no alternate path exists
**When** all available relays are unreachable
**Then** the message enters the backup system (ADR-009) for later delivery
**And** the sender is notified that the message is queued, not lost

### Story 5.3: Node Failure Resilience

As a network participant,
I want the network to survive individual node failures without manual intervention,
So that the network remains functional at alpha scale (10-15 nodes).

**Acceptance Criteria:**

**Given** a relay node crashes or disconnects
**When** the network detects the failure
**Then** peer discovery propagates the departure to all nodes
**And** the role system reassigns relay duties to other capable nodes
**And** in-flight messages through the failed relay are rerouted (Story 5.2)

**Given** multiple nodes fail simultaneously (up to 30% of network)
**When** the remaining nodes detect the failures
**Then** the network re-stabilizes with remaining nodes
**And** roles are redistributed to maintain relay capacity
**And** no manual intervention is required (NFR9)

**Given** a failed node comes back online
**When** it reconnects to the network
**Then** it is reassigned a role based on current network needs
**And** it rejoins the topology map within 3 seconds

### Story 5.4: Contribution/Usage Equilibrium Score

As a network node,
I want the network to track my contribution (relaying for others) versus my usage (sending my own messages),
So that fair participation is incentivized.

**Acceptance Criteria:**

**Given** a node relays messages for other participants
**When** the contribution system processes the relay event
**Then** the node's contribution score increases proportionally
**And** the score is calculated locally by each node observing its own behavior

**Given** a node sends its own messages consuming network resources
**When** the contribution system processes the send event
**Then** the node's usage score increases proportionally
**And** the equilibrium score is computed as contribution/usage ratio

**Given** a node queries its own score
**When** the score API is called
**Then** it returns the current contribution score, usage score, and equilibrium ratio
**And** the score is visible in the demo UI

### Story 5.5: Dynamic Role Display in UI

As a user in the demo app,
I want to see my currently assigned dynamic role (client, relay, observer, guardian, validator),
So that I understand how my node contributes to the network.

**Acceptance Criteria:**

**Given** a node has been assigned a role
**When** the user views the demo UI
**Then** the current role is displayed prominently (e.g., badge or label)
**And** the contribution/usage score is shown alongside the role

**Given** the node's role changes dynamically
**When** the role system reassigns the node
**Then** the UI updates in real-time to show the new role
**And** a brief notification indicates the role change

**Given** the network demonstrates the inversion property (NFR10)
**When** more nodes join and relay capacity increases
**Then** the UI reflects improved network metrics (lower latency, more available paths)
**And** users can observe that more participants = better performance

## Epic 6: Private Communication (Iteration 5)

E2E encryption — relays can no longer read content. Only sender and recipient see the message. The SDK handles encryption/decryption transparently. SDK packaging complete.

### Story 6.1: End-to-End Encryption

As a user,
I want my messages encrypted end-to-end so that only the recipient can read them,
So that relays, backup nodes, and any intermediary cannot access my message content.

**Acceptance Criteria:**

**Given** sender A wants to send a message to recipient B
**When** A's node prepares the MessageEnvelope
**Then** the payload is encrypted using B's public key via TweetNaCl.js box (x25519-xsalsa20-poly1305)
**And** only routing metadata (from, to, via, type, timestamp) remains in cleartext
**And** the envelope signature covers the encrypted payload

**Given** recipient B receives an encrypted envelope
**When** B's node processes the message
**Then** the payload is decrypted using B's private key and A's public key
**And** the decrypted content is delivered to the application layer via event
**And** if decryption fails, a CRYPTO_FAILED TomError is emitted

**Given** a relay or backup node handles an encrypted message
**When** it inspects the envelope
**Then** only routing metadata is visible — payload is opaque encrypted bytes
**And** no architectural mechanism exists for the relay to decrypt the content (NFR7)

**Given** the network transitions from cleartext (iterations 1-4) to mandatory E2E
**When** encryption is enabled
**Then** all new messages are encrypted by default — no opt-out (NFR8)
**And** backward compatibility with cleartext messages is not required

### Story 6.2: SDK Package & 2-Line Integration

As a developer,
I want to install the SDK via npm and send/receive messages in 2 lines of code,
So that integrating ToM into my application is trivial.

**Acceptance Criteria:**

**Given** a developer installs the SDK package
**When** they run `npm install tom-sdk`
**Then** the package installs with all dependencies (core included)
**And** TypeScript types are available out of the box

**Given** a developer wants to send a message
**When** they write minimal integration code
**Then** 2 lines are sufficient: `const tom = new TomClient(config)` and `tom.send(to, message)`
**And** the SDK handles relay selection, encryption, retry, and transport transparently (FR30)

**Given** a developer wants to receive messages
**When** they register a listener
**Then** `tom.on('message', callback)` delivers decrypted messages
**And** the callback receives the sender identity, payload, and metadata

**Given** the SDK is published
**When** a developer inspects the package
**Then** it exports ESM and CJS builds via tsup
**And** the public API surface is minimal: TomClient, config types, event types
**And** installation + first message achievable in under 5 minutes (NFR12)

### Story 6.3: Protocol Compliance Test Suite

As a protocol implementer,
I want a test suite that validates any implementation against the ToM protocol specification,
So that multiple implementations can be verified for interoperability.

**Acceptance Criteria:**

**Given** the compliance test suite is available
**When** run against the TypeScript implementation
**Then** all protocol behaviors are validated: envelope format, routing, relay forwarding, ACK, encryption, signature verification
**And** each test is independent and documents which protocol rule it validates

**Given** an implementation deviates from the protocol
**When** the compliance suite runs
**Then** it clearly identifies which protocol rule was violated
**And** the error message references the specific requirement (FR/NFR number)

**Given** a new protocol feature is added in a future iteration
**When** the compliance suite is updated
**Then** new tests are added without breaking existing ones
**And** the suite serves as the living specification of the protocol

## Epic 7: Self-Sustaining Alpha Network (Iteration 6)

10-15 simultaneous nodes. Ephemeral subnets. The network begins its transition toward autonomy. Bootstrap fades progressively.

### Story 7.1: Autonomous Peer Discovery (Bootstrap Fade)

As a network node,
I want to discover peers through the network itself rather than depending on the signaling server,
So that the network progressively becomes self-sustaining.

**Acceptance Criteria:**

**Given** a node is already connected to the network via bootstrap
**When** it has established connections with multiple peers
**Then** new peer discovery can occur through existing peers (gossip-based or DHT-like)
**And** the signaling server is used only as initial entry point, not for ongoing discovery

**Given** the signaling server goes offline temporarily
**When** existing nodes are already connected
**Then** the network continues to function — no disruption for connected nodes
**And** only brand-new nodes attempting to join are affected

**Given** the network is transitioning toward autonomy
**When** monitoring bootstrap dependency
**Then** metrics show decreasing reliance on the signaling server over time (FR24)
**And** the signaling server remains available but is progressively less critical

### Story 7.2: Ephemeral Subnets with Sliding Genesis

As a network,
I want to form ephemeral subnets that autonomously dissolve when no longer useful,
So that the network self-organizes at a granular level.

**Acceptance Criteria:**

**Given** a group of nodes frequently communicate with each other
**When** the network detects the communication pattern
**Then** an ephemeral subnet forms, optimizing routing between these nodes
**And** the subnet has its own sliding genesis — it exists only as long as it serves a purpose

**Given** an ephemeral subnet's nodes stop communicating or go offline
**When** the subnet detects inactivity or insufficient membership
**Then** the subnet dissolves autonomously — no manual intervention
**And** nodes return to the general network topology seamlessly

**Given** multiple subnets exist simultaneously
**When** a message needs to cross subnet boundaries
**Then** routing works transparently across subnets
**And** subnet formation/dissolution has no impact on message delivery

### Story 7.3: Alpha Scale Validation (10-15 Nodes)

As a protocol developer,
I want to validate that the network operates correctly with 10-15 simultaneous nodes,
So that the protocol is proven at alpha scale before further expansion.

**Acceptance Criteria:**

**Given** 10-15 nodes are connected to the network simultaneously
**When** all nodes are sending and receiving messages
**Then** message delivery completes within the NFR1 latency target (<500ms)
**And** peer discovery and role assignment remain within NFR2 targets (<3s)
**And** no performance degradation compared to smaller network sizes

**Given** the alpha network is running
**When** nodes join and leave dynamically
**Then** the network adapts without manual intervention
**And** the inversion property is observable — more nodes improve routing options

**Given** the alpha scale test completes
**When** results are analyzed
**Then** empirical data defines scalability thresholds beyond PoC (NFR11)
**And** bottlenecks are identified and documented for future iterations

## Epic 8: LLM & Community Ecosystem (Post-MVP)

LLM-first docs, MCP server, VS Code plugin, micro-session contribution model. The protocol spreads through the tools developers already use.

### Story 8.1: LLM-First Documentation

As an LLM,
I want to discover ToM's capabilities through structured documentation (llms.txt, CLAUDE.md),
So that I can suggest ToM integration to developers.

**Acceptance Criteria:**

**Given** the repository contains llms.txt and CLAUDE.md
**When** an LLM reads these files
**Then** it understands ToM's purpose, API, and integration patterns
**And** the documentation follows structured formats optimized for LLM parsing

**Given** a developer asks an LLM about messaging protocols
**When** the LLM has access to ToM's structured docs
**Then** it can provide accurate code examples using the SDK
**And** suggest ToM as a solution for P2P messaging use cases

### Story 8.2: MCP Server for LLM Interaction

As an LLM,
I want to interact with the ToM network programmatically via MCP Server,
So that I can send/receive messages and query network state as tool calls.

**Acceptance Criteria:**

**Given** the MCP server is running
**When** an LLM makes a tool call to send a message
**Then** the message is sent through the ToM network
**And** the response is structured, parseable output (NFR14)

**Given** an LLM queries network state
**When** it calls the appropriate MCP tool
**Then** it receives current participant list, network topology, and node roles
**And** the response format is consistent and documented

### Story 8.3: VS Code Plugin & Live Demo

As a developer,
I want to discover and quick-start ToM from a VS Code plugin and see a live demo on the project website,
So that I can evaluate and adopt the protocol quickly.

**Acceptance Criteria:**

**Given** a developer installs the VS Code plugin
**When** they activate it
**Then** they can scaffold a ToM integration, browse docs, and see network status
**And** the plugin links to the live demo for immediate hands-on experience

**Given** a visitor accesses the project website
**When** they navigate to the demo page
**Then** a live demo of the protocol runs in the browser
**And** the visitor can join the network, send messages, and optionally play Snake

### Story 8.4: Micro-Session Contribution Model

As a contributor,
I want LLM-friendly scoped issues, clear guidelines, and automated testing gates,
So that I can complete a meaningful contribution in a 30-minute micro-session.

**Acceptance Criteria:**

**Given** the repository has open issues
**When** a contributor browses them
**Then** issues are tagged by complexity (micro, small, medium) and category (verification, building, analysis, testing)
**And** each issue contains enough context for an LLM-assisted contributor to start immediately

**Given** a contributor submits a pull request
**When** the CI pipeline runs
**Then** automated testing gates validate the contribution (build, test, lint, compliance)
**And** the contributor receives clear feedback within the micro-session window

**Given** the repository backlog
**When** maintainers review available work
**Then** a permanent backlog of 20+ available issues exists across all categories (FR40)
**And** contributing guidelines document the micro-session model explicitly (FR38)

---

## Technical Debt Register

This section tracks known technical debt items discovered during implementation that require future attention.

### TD-001: Snake Game Countdown Synchronization (Story 4.5)

**Identified:** 2026-02-06 (GPT-5.2 Code Review)
**Severity:** MEDIUM
**Current State:** Workaround implemented
**Story:** 4.5 - Demo Snake Multiplayer P2P Game

#### Problem Description

The countdown sequence (3, 2, 1, GO!) before game start can desynchronize between P1 (host) and P2 (client) due to network latency. When P1 starts countdown, P2 receives the signal after RTT (Round Trip Time), causing P2 to see the countdown later and effectively start the game before P1 has finished their countdown.

**Root Cause:** The current implementation uses simple message passing without timestamp-based synchronization. P2 starts countdown immediately upon receiving `game-ready-ack`, but this doesn't account for network latency.

#### Current Workaround

Implemented in Story 4.5:
- P2 uses `waiting-game-start` state instead of `countdown`
- P2 transitions directly to `playing` state upon receiving first `game-state` from P1
- P2 does not display countdown - waits for game to appear
- This prevents P2 from playing before P1 but provides degraded UX for P2

**Files affected:**
- `apps/demo/src/game/game-controller.ts` (lines ~300-350)
- `GameSessionState` type includes `waiting-game-start`

#### Full Fix (Future Implementation)

The complete solution requires:

1. **New message type: `game-start`**
   ```typescript
   interface GameStartPayload {
     type: 'game-start';
     gameId: string;
     startTimestamp: number; // Precise timestamp when game begins
   }
   ```

2. **Clock synchronization**
   - P1 sends `game-start` with future timestamp (e.g., now + 3500ms)
   - P2 calculates local start time adjusting for clock drift
   - Both clients display countdown based on remaining time to startTimestamp

3. **Implementation steps:**
   - Add `game-start` to `GameMessageType` union
   - Add `isGameStartPayload` type guard
   - Modify `startCountdown` to calculate and send `startTimestamp`
   - Modify P2 countdown to use received timestamp
   - Handle edge cases: late arrival, clock drift > threshold

4. **Testing requirements:**
   - Unit tests for timestamp-based countdown
   - Integration test with simulated network latency
   - Edge case: P2 receives start message after countdown should have begun

#### Acceptance Criteria for Resolution

**Given** P1 initiates game start
**When** P2 receives the game-start message
**Then** both P1 and P2 display synchronized countdown
**And** both enter playing state within 100ms of each other
**And** P2's countdown matches P1's countdown visually (accounting for RTT)
