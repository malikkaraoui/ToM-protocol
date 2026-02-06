# ToM Protocol — Design Decisions (Locked)

> **Status**: LOCKED — These decisions are final and non-negotiable.  
> **Date**: February 2026  
> **Version**: 1.0

---

## Overview

This document defines the 7 foundational architectural decisions of The Open Messaging protocol. These are not suggestions or options — they are **locked design choices** that define ToM's character.

Any implementation, contribution, or extension must respect these decisions without exception.

---

## Decision 1: Message Delivery Definition

### Rule
**A message is delivered if and only if the final recipient emits an ACK.**

### Specification

| Aspect | Behavior |
|--------|----------|
| ACK type | Public network message, propagated like any other message |
| ACK visibility | Observable by any node still holding a copy |
| Pre-ACK state | Message is alive: can duplicate, change host, exploit network presence |
| Post-ACK state | Nodes that learn of the ACK purge their local copy immediately |
| Sender dependency | None — message survives even if sender disappears |
| Synchronization | Not required — each node purges independently upon ACK discovery |

### Rationale
The network's job is to maximize delivery probability within a time window, not to guarantee eternal delivery. ACK-based purging keeps the network light and self-cleaning.

---

## Decision 2: Acceptable Loss & TTL

### Rule
**Messages have a 24-hour maximum lifespan. After TTL expiration, global purge occurs regardless of delivery status.**

### Specification

| Phase | Behavior |
|-------|----------|
| 0–24h | Maximum effort: duplication, relay switching, network exploitation |
| >24h | Global purge — no exceptions, no recovery, no debt |
| Failure handling | Not an anomaly — the network fulfilled its contract |

### Consequences
- ToM promises **bounded maximum effort**, not infinite guarantee
- Reliability is **temporal**, not absolute
- Simplicity, lightness, and resilience take precedence over conservation
- **No retroactive recovery, no residual state, no network blame**

### Rationale
Aligned with aggressive purge philosophy, present-state L1, autonomous messages, and absence of servers/archives.

---

## Decision 3: L1 Role

### Rule
**L1 observes and anchors — it does not arbitrate.**

### Specification

| L1 Does | L1 Does Not |
|---------|-------------|
| Record subnet existence | Make delivery decisions |
| Anchor cryptographic commitments | Judge disputes |
| Maintain global present state | Override subnet-level operations |
| Provide proof of existence | Store message history |

### Architecture Implication
```
┌─────────────────────────────────────┐
│            L1 (Anchor)              │  ← Records state, doesn't decide
├─────────────────────────────────────┤
│     Subnets (Operational Layer)     │  ← Makes real-time decisions
├─────────────────────────────────────┤
│         Nodes (Execution)           │  ← Delivers, duplicates, purges
└─────────────────────────────────────┘
```

### Consequences
- L1 is a **present-state registry**, not a judge
- In case of local divergence, the network evolves — it doesn't "ask permission"
- Each layer has responsibilities; none overloads L1 with decisions it shouldn't make

### Rationale
Aligned with BUS concept, aggressive purge, and absence of implicit central governance.

---

## Decision 4: Reputation & Right to Be Forgotten

### Rule
**The past exists but fades progressively. No permanent condemnation.**

### Specification

| Aspect | Behavior |
|--------|----------|
| Node scoring | Based on past reliability, real contribution, observed behavior |
| Binary states | **None** — no "good" or "banned" status |
| Gradient | Nodes are more or less reliable, not categorized |
| Network strategy | Rely primarily on reliable nodes, tolerate small fraction of "bad" nodes |
| Redemption | A malicious actor can naturally climb back by becoming useful |
| Degradation | A persistently bad actor becomes ineffective without drama |

### Consequences
- No spectacular bans
- No central judge
- Just **progressive inefficiency** for bad actors

### Rationale
Aligned with non-judgmental L1, network adaptation philosophy, and absence of central authority.

---

## Decision 5: Anti-Spam ("The Sprinkler Gets Sprinkled")

### Rule
**Progressive, continuous reaction without hard thresholds.**

### Specification

| Trigger | Network Response |
|---------|------------------|
| Excessive consumption | Rebalance usage |
| Spam pattern detected | Assign more useful tasks |
| Continued abuse | Increase effort required before allowing consumption |

### Mechanism
```
Normal user:     [consume] ←→ [contribute]  (balanced)
Abuser:          [consume] → [forced work] → [consume] → [more work] → ...
```

### Consequences
- Spammer stays in network but:
  - Loses advantage
  - Becomes ineffective
  - Exhausts themselves
- Normal users see nothing
- Attackers reveal themselves through their effort

### What Does NOT Happen
- No blocking
- No exclusion
- No censorship
- No magic threshold
- No moral judgment

### Rationale
The sprinkler gets sprinkled: abuse becomes naturally irrational, not forbidden.

---

## Decision 6: User Invisibility

### Rule
**ToM is an architectural layer, not a product. Total invisibility to end users.**

### Specification

| What Users See | What Users Don't See |
|----------------|---------------------|
| Application UI | ToM protocol |
| App-level messages | Network rebalancing |
| App-level errors (if any) | Spam absorption |
| Nothing about transport | Message death after TTL |

### Comparison
ToM is like:
- **HTTP** — users don't "see" HTTP
- **TCP/IP** — users don't "configure" TCP
- **SSH** — users don't "choose" SSH encryption

### Consequences
- No ToM branding visible
- No ToM configuration exposed
- No ToM metrics shown to users
- Applications **may** choose to expose some info, but ToM itself never does

### Rationale
Protocol layers are invisible. That's what makes them universal.

---

## Decision 7: ToM Scope

### Rule
**ToM is an invisible universal foundation, not an application.**

### Specification

| ToM Is | ToM Is Not |
|--------|------------|
| A protocol layer | An app |
| Comparable to TCP/IP, HTTP, SSH | A messaging product |
| Used by applications transparently | Something users interact with |
| A universal foundation | A branded service |

### Vision Statement
> Applications use ToM without knowing it.  
> Users never see it.  
> It just works.

---

## Summary: The 7 Locks

| # | Decision | One-Line Summary |
|---|----------|------------------|
| 1 | Delivery | ACK from final recipient = delivered |
| 2 | Loss | 24h TTL, then purge — no drama |
| 3 | L1 Role | Anchors state, doesn't judge |
| 4 | Reputation | Fades progressively, no permanent ban |
| 5 | Anti-spam | Progressive load, not exclusion |
| 6 | Visibility | Totally invisible to users |
| 7 | Scope | Universal protocol layer, not product |

---

## Internal Consistency Check

These 7 decisions form a **closed, coherent system**:

```
[Invisible protocol] ← needs → [No user-facing state]
        ↓
[24h TTL + purge] ← enables → [Aggressive lightness]
        ↓
[L1 as anchor only] ← requires → [Subnet autonomy]
        ↓
[Progressive reputation] ← feeds → [Progressive anti-spam]
        ↓
[ACK-based delivery] ← triggers → [Autonomous purge]
        ↓
[Universal foundation] ← justifies → [All of the above]
```

**No internal contradictions. No escape hatches. No exceptions.**

---

## For Contributors

Before writing any code, ask yourself:

1. Does this respect the 7 locked decisions?
2. Does this introduce user-visible protocol state? (If yes → reject)
3. Does this require L1 to make operational decisions? (If yes → reject)
4. Does this create permanent bans or binary states? (If yes → reject)
5. Does this assume message persistence beyond TTL? (If yes → reject)

If all answers are compliant, proceed.

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Feb 2026 | Initial locked decisions |

---

*This document is the canonical reference for ToM architectural decisions.*
