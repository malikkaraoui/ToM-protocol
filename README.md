# ToM Protocol

**The Open Messaging** — a decentralized P2P transport protocol where every device is the network.

## Status: Phase 1 Complete

| Epic | Description | Status |
|------|-------------|--------|
| 1 | Project Foundation & Node Identity | ✅ |
| 2 | First Message Through Relay | ✅ |
| 3 | Dynamic Routing & Discovery | ✅ |
| 4 | Bidirectional Conversation | ✅ |
| 5 | Multi-Relay Transport | ✅ |
| 6 | E2E Encryption | ✅ |
| 7 | Self-Sustaining Network | ✅ |
| 8 | LLM & Community Ecosystem | ✅ |

**663+ tests passing** | **10-15 nodes validated** | **E2E encrypted** | **Hub failover automatique**

## TL;DR

ToM is a transport layer protocol (not a blockchain) that transforms every connected device into both client and relay. No data centers, no speculative tokens, no infinite history.

**The idea:** leverage the dormant power of billions of devices to create a global communication BUS that's resilient and virtually free.

## Quick Start

```bash
# Clone and setup
git clone https://github.com/malikkaraoui/ToM-protocol.git
cd tom-protocol
pnpm install && pnpm build

# Run demo (opens browser + signaling server)
./scripts/start-demo.sh
```

Then open multiple browser tabs at `http://localhost:5173` to chat and play Snake!

## Project Structure

```
tom-protocol/
├── packages/
│   ├── core/                 # Protocol primitives (tom-protocol)
│   └── sdk/                  # Developer SDK (tom-sdk)
├── apps/
│   └── demo/                 # Browser demo with multiplayer Snake
├── tools/
│   ├── signaling-server/     # Bootstrap server (temporary)
│   ├── mcp-server/           # MCP server for LLM interaction
│   └── vscode-extension/     # VS Code extension (WIP)
├── llms.txt                  # LLM quick reference
├── CLAUDE.md                 # Detailed LLM guide
└── CONTRIBUTING.md           # Micro-session contribution model
```

## For Developers

### 2-Line Integration

```typescript
import { TomClient } from 'tom-sdk';

const client = new TomClient({ signalingUrl: 'ws://localhost:3001', username: 'alice' });
await client.connect();

// Send E2E encrypted message
await client.sendMessage(recipientNodeId, 'Hello!');

// Receive messages
client.onMessage((envelope) => {
  console.log(`From ${envelope.from}: ${envelope.payload.text}`);
});
```

### For LLMs

- Read [llms.txt](llms.txt) for quick protocol overview
- Read [CLAUDE.md](CLAUDE.md) for detailed implementation guide
- Use the [MCP server](tools/mcp-server/) for programmatic interaction

## Architecture Highlights

| Feature | Implementation |
|---------|----------------|
| **Identity** | Ed25519 keypair (TweetNaCl.js) |
| **Transport** | WebRTC DataChannel |
| **Encryption** | X25519 + XSalsa20-Poly1305 (E2E) |
| **Discovery** | Gossip protocol + ephemeral subnets |
| **Routing** | Dynamic relay selection, multi-hop |

## Tests E2E Automatisés

Tests Playwright avec génération de rapport détaillé :

```bash
# Lancer les tests E2E (headless)
pnpm test:e2e

# Lancer avec navigateur visible
pnpm test:e2e:headed

# Interface graphique Playwright
pnpm test:e2e:ui

# Voir le rapport HTML
pnpm test:e2e:report
```

### Rapport généré

```
╔══════════════════════════════════════════════════════════════════╗
║                  ToM Protocol E2E Test Report                     ║
╠══════════════════════════════════════════════════════════════════╣
│  Messages: 15 envoyés, 14 reçus (93.3%)                          │
│  Invitations: 3/3 acceptées (100%)                               │
│  Latence moyenne: 352ms                                          │
│  Hub failover: automatique si hub offline                        │
╚══════════════════════════════════════════════════════════════════╝
```

Tests progressifs : 2 users → 3 users → groupes → invitations → déconnexion hub.

## Contributing

ToM uses a **micro-session contribution model** — small, focused changes completable in 30-60 minutes.

See [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Issue complexity levels (micro/small/medium)
- How to find and claim work
- Session workflow

## Why ToM Exists

| Current Problem | ToM's Answer |
|-----------------|--------------|
| Centralized infrastructure = censorship points | Pure P2P, no master server |
| Blockchains = infinite history, sync marathon | Ultra-purged L1, sliding genesis |
| Fees/entry barriers | Free = you pay with network contribution |

## Core Concepts

### Proof of Presence (PoP)

No energy-hungry PoW, no capitalist PoS. You validate because you're there and you behave well.

### Dynamic Roles

Every node can be: **Client, Relay, Observer, Guardian, Validator.**
Roles are assigned dynamically based on network needs and contribution.

### Usage vs. Contribution Balance

```
Score = Contribution - Usage
```

Heavy consumers become relays. Spam is self-destructive.

## Documentation

- [CLAUDE.md](CLAUDE.md) - Implementation guide for AI assistants
- [llms.txt](llms.txt) - Protocol quick reference
- [Architecture](/_bmad-output/planning-artifacts/architecture.md) - ADRs and design decisions
- [Epics & Stories](/_bmad-output/planning-artifacts/epics.md) - Full requirements breakdown
- [Tests Added Log](scripts/tests-added-log.md) - Track of all tests added to the project

## License

MIT
