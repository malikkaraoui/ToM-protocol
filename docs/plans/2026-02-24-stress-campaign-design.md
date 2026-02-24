# Stress Campaign V5 — Design Document

**Objectif :** Sortir du labo. Tester le protocole complet (transport + crypto + groupes chiffrés + failover) en conditions réelles sur Mac ↔ Freebox NAS, avec endurance 1h+.

**Setup :** MacBook Pro x86_64 (orchestrateur) ↔ Freebox NAS ARM64 (répondeur). iPhone = hotspot 4G pour scénarios mobiles. Future extension : iPhone comme 3ème noeud.

---

## Architecture

### Deux rôles

**Responder** (`tom-stress responder`) — NAS :
- ProtocolRuntime complet (crypto, groupes, discovery, backup)
- Auto-echo : renvoie chaque message chat reçu
- Auto-accept : accepte toutes les invitations groupe
- Auto-reply groupe : echo dans les groupes
- Tourne indéfiniment, log sur stdout

**Campaign** (`tom-stress campaign --connect <ID> --duration 3600`) — MacBook :
- ProtocolRuntime complet
- Drive 6 phases en séquence
- Collecte métriques JSONL + rapport markdown final
- Chaque phase indépendante (`--phase ping` pour une seule)

### Phases

| # | Phase | Durée | Volume | Ce qu'on valide |
|---|-------|-------|--------|-----------------|
| 1 | Ping | 30s | 20 msgs | RTT baseline, path direct/relay |
| 2 | Burst | 60s | 300 msgs (3×100) | Throughput, perte sous charge |
| 3 | Protocol E2E | 60s | 50 msgs | Chiffrement+signature bout-en-bout |
| 4 | Group Encrypted | 90s | 100 msgs | Sender Keys, rotation, forward secrecy |
| 5 | Failover | 60s | 20 msgs | Shadow promotion, continuité de service |
| 6 | Endurance | configurable (défaut 1h) | 1 msg/s | Stabilité longue durée, reconnection |

### Communication Protocol (Responder ↔ Campaign)

Les messages utilisent le protocole ToM existant (Envelope, signed, encrypted). Le responder identifie les commandes par le contenu du payload :

- `PING:<seq>` → répond `PONG:<seq>`
- `BURST:<seq>` → répond `BURST-ACK:<seq>`
- Tout autre texte → echo `ECHO:<original>`
- GroupMessage → echo `GROUP-ECHO:<text>`
- GroupInvite → auto-accept
- GroupPayload::SenderKeyDistribution → handled by ProtocolRuntime

### Output

**JSONL streaming** (stdout + fichier) :
```json
{"phase":"ping","event":"ping","seq":1,"rtt_ms":45.2,"path":"direct","elapsed_s":1.5}
{"phase":"burst","event":"burst_round","round":1,"sent":100,"acked":98,"lost":2,"avg_rtt_ms":23.5}
{"phase":"e2e","event":"message","seq":1,"encrypted":true,"signed":true,"rtt_ms":67.3}
{"phase":"group","event":"group_message","seq":1,"encrypted":true,"key_epoch":1,"rtt_ms":89.1}
{"phase":"failover","event":"shadow_promoted","elapsed_ms":5800}
{"phase":"endurance","event":"rolling_stats","minute":15,"sent":900,"received":897,"loss_pct":0.33,"avg_rtt_ms":12.4}
```

**Rapport final** (`campaign-report.md`) :
```markdown
# Campaign V5 — 2026-02-24
## Setup: MacBook (WiFi) ↔ NAS (LAN)
## Results
| Phase | Status | Messages | Loss | Avg RTT |
|-------|--------|----------|------|---------|
| Ping | PASS | 20/20 | 0% | 45ms |
...
```

## Non-Scope

- ❌ iPhone comme noeud (phase 2, après résultats)
- ❌ Multi-hop relay (pas de 3ème noeud)
- ❌ UI web pour visualiser les résultats
- ❌ Tests de sécurité (fuzzing, injection)
