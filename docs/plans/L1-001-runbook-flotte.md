# L1-001 — Runbook tests flotte réelle (Proof of Presence)

> Objectif : valider l'attestation de présence sur la **vraie flotte**
> (iPhone · iPad · Apple TV · macOS · NAS) en conditions réelles —
> LAN, WiFi, 4G/CGNAT, veille iOS. Prérequis : build ≥ 19 partout.
>
> Déjà prouvé AVANT la flotte (ne pas re-tester ça demain) :
> 9/9 tests adverses runtime + scénario QUIC local 5/5 (médiane 7 ms,
> gate anti-Sybil vérifié en réseau). La flotte teste ce que le local
> ne peut pas : NAT réel, radio réelle, veille réelle, horloges réelles.

---

## Les deux leviers de test (RuntimeConfig)

| Levier | Défaut | Rôle |
|---|---|---|
| `presence_contribution_min` | `2.0` | Gate anti-Sybil (score local exigé de l'attesteur). **`0.0` = mode plomberie** : accepte toute attestation signée bien formée. Les défenses structurelles (signatures, one-shot, nonce, budget) restent armées. |
| `presence_probe_interval` | `None` (off) | Sonde auto : challenge jusqu'à 8 pairs Online à chaque tick. **Zéro UI nécessaire** — les résultats tombent dans le Live Log. Reco flotte : `15s`. |

## Phase 1 — Plomberie (gate = 0.0, sonde 15s)

**Question posée** : le challenge→attestation traverse-t-il la vraie flotte,
et à quelle latence, sur chaque type de lien ?

Config apps (debug) : `presence_contribution_min = 0.0`,
`presence_probe_interval = 15s`.

| # | Test | Lien | Attendu |
|---|---|---|---|
| P1 | macOS ↔ iPhone même WiFi | LAN direct (mDNS) | attestations < 50 ms, régulières (15s) |
| P2 | macOS ↔ Apple TV | LAN direct | idem P1 — attention CPU Apple TV (appareil contraint) |
| P3 | iPhone (4G, WiFi OFF) ↔ maison | relais/hole-punch | attestations < 300 ms ; noter le path (relay vs direct) |
| P4 | iPhone verrouillé 2 min puis retour | veille iOS | pendant la veille : challenges vers l'iPhone expirent (30s) SANS erreur ; au retour foreground : reprise ≤ 1 tick (15s) |
| P5 | NAS (tom-tui ou responder) ↔ macOS | LAN filaire | le nœud headless atteste (il répond automatiquement — le handler est dans le runtime) |
| P6 | Coupure WiFi 30 s puis retour (n'importe quel pair) | recovery | pas de crash, pas d'attestation fantôme (one-shot), reprise après reconnect |

**Métriques à relever (Live Log)** : latence par lien (médiane + pire),
taux de réponse par device, comportement Apple TV (mémoire ~200 Ko max),
silence propre pendant veille iOS.

**Critère de sortie phase 1** : chaque paire de la flotte échange des
attestations, aucune latence LAN > 200 ms, aucun crash, purge silencieuse.

## Phase 2 — Gate réel (gate = 2.0)

**Question posée** : l'évidence de relais s'acquiert-elle naturellement sur
la vraie flotte, et le gate bloque-t-il bien le reste ?

Config : `presence_contribution_min = 2.0` (défaut), sonde 15s.

| # | Test | Attendu |
|---|---|---|
| G1 | État initial (personne n'a relayé pour personne) | **zéro attestation acceptée** — c'est le comportement CORRECT, pas un bug. Les logs `presence: attestation dropped (local score …)` sont la preuve que le gate travaille. |
| G2 | Forcer un chemin relayé : iPhone 4G → maison via la porte publique (NAS/embedded relay), échanger quelques messages | l'ACK RelayForwarded signé crédite le relais → le gate s'ouvre pour CE pair ; attestations acceptées uniquement de lui |
| G3 | Retour LAN (tout direct) | le score decay (5 %/h) referme progressivement le gate — observer la fade, c'est la décision #4 en action |

**Leçon déjà connue (scénario local)** : en LAN, tout passe en direct, personne
ne relaie → gate fermé partout. C'est attendu. L'évidence vient des chemins
relayés (4G/CGNAT, cross-réseau). Si G2 n'ouvre pas le gate, vérifier que le
message est bien passé par le relais pair (pas la porte transport tom-relay :
seul le relayage **protocole** — via chain — émet l'ACK RelayForwarded).

## Phase 3 (bonus si temps) — Adverse sur flotte

- Rejouer une attestation capturée (proxy/log) → drop silencieux.
- Flood de challenges depuis un device vers l'Apple TV → budget répondeur
  (10/fenêtre 30s) + pas de dérive mémoire.
- Horloge d'un device décalée de +5 min (Réglages) → les attestations de CE
  device restent acceptées (fraîcheur = horloge locale du challenger, pas la
  sienne) — c'est LE test du durcissement anti-NTP.

## Ce qui est déjà câblé dans build 19 (rien à coder demain)

- **FFI** : `tom_node_check_presence()` (déclenchement manuel) +
  `tom_node_presence_stats()` (JSON : accepted_total, last_attester,
  last_latency_ms, window_count, seed_prefix) + 2 clés config JSON
  (`presence_contribution_min`, `presence_probe_interval_secs`).
- **TomProtocolKit** : `checkPresence(target:)`, `presenceStats()`,
  `TomPresenceStats`, paramètres `start(presenceContributionMin:presenceProbeIntervalSecs:)`.
- **Apps iOS/tvOS/macOS** : démarrent en **PHASE 1** (gate 0.0, sonde 15s) ;
  chaque attestation acceptée apparaît dans le Live Log :
  `PRESENCE ✓ <id8> atteste en Xms (total N, fenêtre W, seed abcd1234)`.
- **NAS** : `tom-stress responder` répond automatiquement aux challenges
  (handler dans le runtime) et logge `PRESENCE ✓ …` pour ceux qu'il sollicite.

**Passage PHASE 2** : dans `TomNodeService.swift` (les 2 apps), remplacer
`presenceContributionMin: 0.0` par `nil` (→ défaut protocole 2.0), rebuild.

## Rappels build & déploiement

```bash
# 1. Rust prêt : 11/11 presence_integration + scénario QUIC local 5/5
cargo test -p tom-protocol --test presence_integration
cargo run -p tom-stress --bin tom-stress -- presence

# 2. FFI + XCFramework (AVANT d'ouvrir Xcode)
bash scripts/check-ffi.sh
bash scripts/build-tom-protocol-ffi-xcframework.sh      # 6 slices, long
bash scripts/sync-xcframework-to-package.sh             # → TomProtocolKit/Artifacts

# 3. Apps build 19 : rebuild + install iPhone / iPad / Apple TV / macOS
#    (TomVersion.build = 19, vérifier dans Settings de chaque app)

# 4. NAS : responder à jour (cible de challenge headless)
cargo zigbuild -p tom-stress --target aarch64-unknown-linux-musl --release
# (kill l'ancien process avant scp — 'dest open Failure' sinon)
```

## Build 20 — stress autonome + relevés (API ajoutées)

Le build 20 ajoute de quoi *piloter et mesurer* la présence sans UI, pour
enchaîner des scénarios en autonomie :

| Capacité | RuntimeHandle | FFI | Swift |
|---|---|---|---|
| Challenge en lot | `check_presence_many(Vec)` | `tom_node_check_presence_many` | `checkPresenceMany` |
| Challenge tous les Online | `check_presence_all_online()` | (via many) | — |
| Compteurs par issue | `presence_metrics()` | `tom_node_presence_metrics` | `presenceMetrics()` |

**Compteurs** (`PresenceMetrics`, monotones) : `issued`, `accepted`,
`drop_*` (unknown_challenge/stale/wrong_attester/bad_signature/incoherent/gate/store_full),
`challenges_received`, `signed`, `refused_*`, latence min/max/mean.
→ chaque décision de présence incrémente **exactement un** compteur (invariant testé),
donc les drops silencieux redeviennent mesurables **sans rien fuiter sur le fil**.

**Scénario tempête** (moi, en autonomie, déjà vert) :
```bash
cargo run -p tom-stress --bin tom-stress -- presence-storm
# 4 nœuds maillés, 20 rafales, relevé JSONL : ratio d'acceptation,
# latence min/mean/max, budget répondeur qui plafonne, mémoire bornée
```
Relevé mesuré (local) : 400 émis, 120 acceptés (budget plafonne 120/240 signés),
latence chaude ~184 ms mean, 0 drop (gate 0.0). Sur la flotte : rejouer en
faisant varier NODE_COUNT / gate / pace pour cartographier le point de
saturation par type d'appareil (Apple TV = plancher).

Côté apps, un bouton debug peut appeler `presenceMetrics()` et logger la
ligne de compteurs — utile pour lire le comportement du gate/budget en live
sans brancher un debugger.

## Ce que ce runbook ne teste PAS (assumé)

- Grinding par sous-ensemble du seed — ouvert par design jusqu'à **L1-002**.
- Preuve transférable à un tiers (`ObserverSigned`) — **L1-003**.
- Vraie suspension iOS arrière-plan (pas de wake-up sans APNs — R18) : le
  test P4 mesure le comportement *autour* de la veille, pas pendant.
