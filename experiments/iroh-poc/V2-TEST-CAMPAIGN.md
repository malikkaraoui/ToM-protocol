# V2 Test Campaign — In-Motion NAT Traversal

> Stress-testing iroh's hole punching under real-world mobility conditions
> using `nat-test --continuous`.

## Prerequisites

### Listener setup (Freebox NAS — always-on anchor)

```bash
# On MacBook: cross-compile for ARM64
cargo zigbuild --release --bin nat-test --target aarch64-unknown-linux-musl

# Transfer to NAS
scp target/aarch64-unknown-linux-musl/release/nat-test root@192.168.0.21:~/

# On NAS: start listener (leave running for the entire campaign)
ssh root@192.168.0.21
./nat-test --listen --name NAS
# → Note the EndpointId, e.g. 3448e243cc7c...
```

### Connector setup (MacBook — the mobile node)

```bash
# Build release binary
cargo build --release --bin nat-test

# Alias for convenience
alias nattest='./target/release/nat-test'

# Base command (replace NAS_ID with actual ID)
export NAS_ID="<paste NAS EndpointId here>"
```

---

## Scenarios

### 1. School WiFi (Switzerland)

**What it tests**: Restrictive guest NAT, possible UDP blocking, firewall rules

**Procedure**:
```bash
# Connect to school WiFi
# Run 60 pings (2 minutes) to see if hole punch succeeds at all
nattest --connect $NAS_ID --name "SchoolWiFi" --pings 60 --delay 2000 \
  | tee results/01-school-wifi.jsonl
```

**Watch for**:
- Does hole punch succeed? (some school firewalls block UDP entirely)
- If relay-only: what RTT?
- Time to first direct path (if any)

**Fallback**: If 0% direct after 60 pings, the school firewall likely blocks
UDP. Log that result — it's valuable data for relay-only scenarios.

---

### 2. 4G/5G Swiss operator (CGNAT)

**What it tests**: Operator-level CGNAT, symmetric NAT

**Procedure**:
```bash
# Disconnect WiFi, tether iPhone via USB (4G/5G)
# Run 60 pings
nattest --connect $NAS_ID --name "4G-Swiss" --pings 60 --delay 2000 \
  | tee results/02-4g-swiss.jsonl
```

**Already tested in PoC-4**: 2.9s upgrade, 107ms direct RTT, 90% direct.
V2 re-validates with more pings and captures JSON for comparison.

---

### 3. Moving car (passenger seat)

**What it tests**: Cell tower handoffs, relay handoff, reconnection under
continuous motion

**Procedure**:
```bash
# Start BEFORE getting in car. Use continuous mode.
# iPhone tethered via USB (4G)
nattest --connect $NAS_ID --name "Car" --continuous \
  --delay 1000 --summary-interval 30 --max-reconnects 0 \
  | tee results/03-car-continuous.jsonl
```

**Expected behavior**:
- Disconnections when switching cell towers
- Automatic reconnection (exponential backoff)
- Direct % may fluctuate as NAT mapping changes
- Rolling summaries every 30 pings (~30 seconds)

**Duration**: 15-30 minutes minimum (a real drive, not parking lot circles)

**Watch for**:
- `disconnected` events: how often? how long?
- `reconnected` events: reconnect time?
- Direct % over time (does it degrade or stay stable?)
- Any permanent connection loss (max_reconnects=0 means infinite retries)

---

### 4. Border crossing (CH ↔ FR)

**What it tests**: Network switch from Swiss operator → French operator (roaming),
mid-session continuity

**Procedure**:
```bash
# Start in Switzerland, drive/train towards France
# Use continuous mode with frequent summaries
nattest --connect $NAS_ID --name "Border-CH-FR" --continuous \
  --delay 1000 --summary-interval 20 --max-reconnects 0 \
  | tee results/04-border-crossing.jsonl
```

**Already tested in PoC-4** (static): 0.33s upgrade, 32ms RTT, 95% direct.
V2 adds the dynamic crossing — does the session survive the roaming switch?

**Watch for**:
- The exact moment of border crossing (disconnected event)
- Reconnection time on the new operator
- RTT change (Swiss vs French towers)
- Does the relay change? (EU relay should stay the same)

---

### 5. Weak coverage / tunnel

**What it tests**: Disconnection + reconnection under signal loss

**Procedure**:
```bash
# Find a tunnel, parking garage, or area with known weak coverage
# Start outside, walk/drive through, come out the other side
nattest --connect $NAS_ID --name "Tunnel" --continuous \
  --delay 1000 --summary-interval 20 --max-reconnects 0 \
  | tee results/05-tunnel.jsonl
```

**Watch for**:
- Clean disconnection event when signal drops
- Reconnection time when signal returns
- Hole punch re-establishment time after reconnection
- Total disconnected time (tracked in summary)

---

### 6. Network switch (WiFi → 4G → WiFi)

**What it tests**: Mid-session transport change, MagicSock path switching

**Procedure**:
```bash
# Start on home WiFi
nattest --connect $NAS_ID --name "NetSwitch" --continuous \
  --delay 1000 --summary-interval 20 --max-reconnects 0 \
  | tee results/06-network-switch.jsonl

# After ~30 pings: disconnect WiFi (iPhone tethering takes over)
# After ~30 more pings: reconnect WiFi
# After ~30 more pings: Ctrl+C
```

**Watch for**:
- Does MagicSock handle WiFi→4G transparently (no disconnect)?
- Or does it require reconnection?
- RTT change between WiFi and 4G paths
- Path change events during switch

---

## Results Directory

```bash
# Create before starting
mkdir -p results/
```

Each test produces a `.jsonl` file (one JSON event per line).

## Analysis

After each test, run the analysis script:

```bash
# Quick summary of a single test
python3 scripts/analyze-results.py results/03-car-continuous.jsonl

# Compare all tests
python3 scripts/analyze-results.py results/*.jsonl
```

## Campaign Checklist

| # | Scenario | Duration | Network | Status |
|---|----------|----------|---------|--------|
| 1 | School WiFi (CH) | ~2 min | Guest WiFi | ☐ |
| 2 | 4G/5G Swiss operator | ~2 min | Mobile tethering | ☐ |
| 3 | Moving car | 15-30 min | 4G tethering | ☐ |
| 4 | Border crossing (CH↔FR) | 20-40 min | 4G roaming | ☐ |
| 5 | Tunnel / weak coverage | ~5 min | 4G | ☐ |
| 6 | Network switch (WiFi→4G→WiFi) | ~3 min | WiFi + 4G | ☐ |

## What We're Proving

PoC-4 showed iroh punches through NAT 100% of the time in static scenarios.
V2 answers the harder question: **does it stay punched under mobility?**

Key metrics per scenario:
- **Hole punch success rate** (direct %)
- **Reconnection resilience** (count, speed)
- **RTT stability** (mean, p95, variance)
- **Total downtime** (disconnected duration)

If V2 results hold (>80% direct, <5s reconnection), iroh's connectivity layer
is validated for ToM's target use case: messaging on mobile devices.
