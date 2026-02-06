
# The Open Messaging (ToM)

**A decentralized transport protocol for the free internet**

## TL;DR

ToM is a transport layer protocol (not a blockchain) that transforms every connected device into both client and server. No data centers, no speculative tokens, no infinite history to drag around.

**The idea:** leverage the dormant power of billions of devices to create a global communication BUS that's resilient and virtually free.

## Why ToM Exists

| Current Problem | ToM's Answer |
|-----------------|--------------|
| Centralized infrastructure = censorship points | Pure P2P, no master server |
| Blockchains = infinite history, sync marathon | Ultra-purged L1, sliding genesis |
| Consensus = industry (mining, capital staking) | Proof of Presence (PoP): you participate, you validate |
| Fees/entry barriers | "Free" = you pay with network contribution |
| Double-spend without full history? | Per-wallet state commitments + distributed observers |

## Architecture in 30 Seconds

```
┌─────────────────────────────────────────────────────────────┐
│                      L1 (Organic BUS)                       │
│  • Present state only (no history)                          │
│  • Sliding genesis: a few blocks max                        │
│  • Periodic cryptographic snapshots                         │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
   ┌─────────┐          ┌─────────┐          ┌─────────┐
   │ Subnet  │          │ Subnet  │          │ Subnet  │
   │    A    │          │    B    │          │    C    │
   │(ephemer)│          │(ephemer)│          │(ephemer)│
   └─────────┘          └─────────┘          └─────────┘
        │                     │                     │
   On-demand creation — Auto-purge if inactive — Fork if overloaded
```

Every node can be: **Client, Relay, Observer, Guardian, Archiver, Validator.**
Dynamic roles, assigned via PoP.

## Core Concepts

### Proof of Presence (PoP)

No energy-hungry PoW, no capitalist PoS. You validate because you're there and you behave well.

- Rotating quorums selected pseudo-randomly among present nodes
- Roles announced a few blocks ahead (verifiable rotation)
- Dedicated nodes monitor randomness quality and rotation dynamics

### Per-Wallet State Commitments (Double-Spend Solution)

```
State_W = {
    wallet_id:  PK_W,           // Public key
    commit:     Commit_W,       // Cryptographic commitment (Merkle/Pedersen)
    net_sig:    AggSig_quorum,  // Aggregated observer signatures
    height:     h               // State version number
}
```

**How it works:**
1. Wallet owner proposes a transition: `Commit_old → Commit_new`
2. Observers verify `from_commit` matches their last known state
3. Quorum signs only if valid
4. L1 records new state — old state is gone

**Golden rule:** No observer signs two different transitions from the same `from_commit`.

### Dynamic Economy: Usage vs. Contribution

```
Score_U = Contribution_U − Usage_U
```

| Score | Status |
|-------|--------|
| ≈ 0 | Ideal: give-and-take balance |
| >> 0 | Heavy contributor (potential fork trigger) |
| << 0 | Heavy consumer (potential spam profile) |

Tokens aren't capital — they're balance trackers. No rent-seeking.

### Anti-Spam: The Sprinkler Gets Sprinkled

When `Score_U` goes deeply negative:

1. **Local micro-PoW:** outgoing messages require increasingly costly hash puzzles
2. **Relay over-assignment:** spammer becomes network relay, burning their own resources
3. **Non-critical validation tasks:** proof verifications, commitment recalculations

Spam isn't just forbidden — it's self-destructive.

## What ToM Is NOT

| ❌ NOT | ✅ IS |
|--------|-------|
| A blockchain | A transport layer protocol |
| A cryptocurrency | A utility-balanced token system |
| Permanent storage | Aggressive purge, present-state only |
| Mining/staking industry | Participation-based consensus |
| Dependent on external infra | Self-sufficient P2P mesh |

## Technical Challenges (Open Questions)

These are acknowledged design gaps requiring further formalization:

| Challenge | Current Status |
|-----------|----------------|
| PoP mathematical formalization | Conceptual — needs formal security proofs |
| Observer selection protocols | Outlined — attack surface analysis pending |
| Cryptographic commitment details | Direction chosen (Merkle/Pedersen) — spec incomplete |
| Network partition handling | Subnet fork mechanism described — edge cases TBD |
| Bootstrap without seed nodes | Guardian role defined — bootstrap protocol incomplete |
| Sybil resistance in PoP | Relies on behavior scoring — formal analysis needed |

## Project Structure

```
tom/
├── packages/
│   ├── core/                     # Protocol primitives (transport, routing, identity, groups)
│   └── sdk/                      # Developer-friendly API (TomClient)
├── apps/
│   └── demo/                     # Demo app with multiplayer Snake game
├── tools/
│   └── signaling-server/         # Bootstrap WebSocket server (temporary)
├── docs/
│   ├── whitepaper-v0.1.pdf       # Initial whitepaper (FR)
│   └── step-2-architecture.pdf   # Extended architecture doc
└── specs/                        # Protocol specifications (WIP)
```

## Quick Start

```bash
pnpm install
pnpm build
pnpm test
```

## Roadmap

| Phase | Focus | Status |
|-------|-------|--------|
| **0** | Conceptual foundation, whitepaper | ✅ Done |
| **1** | Protocol spec formalization | 🔄 In progress |
| **2** | Reference implementation (SDK) | 📋 Planned |
| **3** | Testnet with ephemeral subnets | 📋 Planned |
| **4** | Security audits, attack simulations | 📋 Planned |
| **5** | Mainnet bootstrap | 📋 Planned |

## Philosophy

> *"A network where the power comes from the sum of everyone's contribution, not the concentration of a few."*

ToM is designed for:

- **Messaging first** — payments later, if ever
- **Environmental sanity** — reuse existing compute, no ASIC arms race
- **True decentralization** — no validators-as-a-service industry
- **Universal access** — no capital barrier, just participation

## Contributing

Project is in early conceptual phase. Contributions welcome on:

- Protocol formalization
- Attack scenario analysis
- SDK architecture proposals

## License

TBD — Open source intent confirmed, license selection pending.

---

<p align="center">
  <i>"Stop selling your data for a service that's become essential."</i>
</p>


## License

MIT
