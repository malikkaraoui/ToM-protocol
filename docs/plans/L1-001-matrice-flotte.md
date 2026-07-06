# L1-001 — Matrice de test flotte (exploitation API tous azimuts)

> Objectif : dès que les devices sont branchés, exercer **toute la surface
> d'API présence** (build 20) dans tous les sens, et relever automatiquement.
> Pilote autonome : `tom-stress fleet-probe` sur le MacBook (ou le NAS).
> Prérequis : apps build 20 sur iOS/tvOS/macOS, responder build 20 sur NAS.

---

## Flotte cible

| Device | Rôle dans le test | Ce qu'il exécute | Chemin d'API exercé |
|---|---|---|---|
| **iPhone A** | pair mobile + veille | app iOS build 20, sonde auto 15s | challenge émis + attesté ; test veille (P4) |
| **iPhone B** | 2ᵉ mobile (4G) | app iOS, WiFi OFF → 4G/CGNAT | chemin relayé → évidence relais (phase 2) |
| **iPad** | pair LAN | app iOS (universelle) | challenge/attestation LAN direct |
| **MacBook** | **pilote + pair** | app macOS **ET** `fleet-probe` | pilote toute l'API : `check_presence_all_online`, `presence_metrics`, reachability |
| **Apple TV** | pair contraint | app tvOS build 20 | budget répondeur + **mémoire bornée** (plancher perf) |
| **NAS (VM Freebox)** | cible headless + relais | `tom-stress responder` + `tom-relay` | répond aux challenges auto ; porte publique 4G |

---

## Le pilote autonome — `fleet-probe`

Un nœud contrôlable qui rejoint le vrai réseau et, à mesure que les devices
se branchent, les découvre, les challenge tous en boucle, ping la
reachability, et sort un tableau live + JSONL.

```bash
# Sur le MacBook, pointé sur le relais NAS (LAN)
cargo run -p tom-stress --bin tom-stress -- fleet-probe \
    --relay-url http://192.168.0.21:3340 \
    --probe-interval 10 --report-interval 5

# Depuis l'extérieur (4G), via la porte publique
cargo run -p tom-stress --bin tom-stress -- fleet-probe \
    --relay-url http://82.67.95.8:3340 --duration-secs 600
```

Sortie (toutes les 5s) — tableau par device + ligne JSONL de compteurs :
```
── Fleet report @ 35s — 5 discovered, 4 connected ──
  id         username         source   attest  lat(ms)  echo  conn
  a1b2c3d4   iphone-a         Mdns         12       31     6   yes
  e5f6a7b8   apple-tv         Relay         8      142     5   yes
  ...
{"event":"fleet-probe-relevé","elapsed_s":35,"peers_connected":4,"presence":{...}}
```

**Ce que le pilote exerce, dans tous les sens** :
- `check_presence_all_online()` chaque round → challenge chaque device découvert
- attestations reçues comptées **par attesteur** + latence (le device est-il vivant, à quelle vitesse ?)
- `send_message` ping → écho de l'app (auto-echo « recu 5/5 ») → **reachability deux sens** par device
- `presence_metrics()` → compteurs globaux par issue (accept/drop/budget) en JSONL

---

## Séquence de branchement (ordre conseillé)

1. **NAS d'abord** : `tom-relay --dev` (porte) + `tom-stress responder` (cible headless). Note son NodeId.
2. **MacBook** : lance `fleet-probe --relay-url <NAS>`. Il doit voir le NAS en quelques s.
3. **iPad + iPhone A (LAN WiFi)** : ouvre l'app, Start. Ils apparaissent dans le tableau, `attest` monte.
4. **Apple TV** : ouvre l'app. Surveiller `attest` **et** la stabilité (pas de reboot app : mémoire).
5. **iPhone B en 4G** (WiFi OFF) : doit apparaître via le relais NAS (`source=Relay`), latence plus haute.
6. Laisser tourner ≥ 5 min : les tableaux se remplissent, le JSONL s'accumule.

---

## Cas de figure à couvrir (cases à cocher)

### Découverte & reachability
- [ ] Chaque device apparaît dans le tableau (`discovered`)
- [ ] Chaque device passe `connected=yes`
- [ ] Chaque device renvoie ≥ 1 écho (reachability deux sens)
- [ ] iPhone B (4G) découvert via `source=Relay` (porte publique OK)

### Présence (le nouveau mécanisme)
- [ ] Chaque device accumule des `attest` (il atteste sa présence)
- [ ] Latence LAN (iPad, iPhone A, Mac) < 200 ms
- [ ] Latence relayée (iPhone B 4G) relevée (attendu plus haut, noter la valeur)
- [ ] Apple TV atteste sans planter ni gonfler la mémoire
- [ ] Budget répondeur visible sous rafale (`challenges_received > signed` dans le JSONL)

### Robustesse
- [ ] **iPhone A veille** : verrouiller 2 min → `attest` gèle proprement, pas d'erreur ; retour foreground → reprise ≤ 1 round
- [ ] **Coupure WiFi 30 s** (un device) → disparaît puis revient, pas de crash pilote
- [ ] **NAS relais tué puis relancé** → les 4G re-basculent, découverte reprend

### Gate anti-Sybil (phase 2 — repasser gate à `nil`)
- [ ] Rebuild apps avec `presenceContributionMin: nil` (défaut 2.0)
- [ ] `fleet-probe` : les `drop_gate` montent (aucune évidence relais initiale)
- [ ] Après trafic relayé iPhone B ↔ NAS, `accepted` remonte pour ce pair uniquement

---

## Pièges par device (connus)

| Device | Piège | Signe | Parade |
|---|---|---|---|
| Apple TV | RAM faible → jetsam si UI décode trop | app tuée sous charge | budget mémoire présence borné (~200 Ko) ; ne pas logger le payload brut |
| iPhone (fond) | vraie suspension → process gelé | `attest` s'arrête app en fond | attendu (R18/APNs futur) ; backup 24h couvre |
| iPhone 4G/CGNAT | pas de direct → tout relayé | `source=Relay`, latence ↑ | c'est le cas nominal 4G, pas un bug |
| NAS ARM64 | binaire mauvaise arch | responder ne démarre pas | `cargo zigbuild --target aarch64-unknown-linux-musl` |
| Tous | horloges désynchro | — | fraîcheur = horloge locale du challenger (anti-NTP durci) |

---

## Relevés à conserver

Rediriger le JSONL vers un fichier pour analyse :
```bash
cargo run -p tom-stress --bin tom-stress -- fleet-probe \
    --relay-url http://192.168.0.21:3340 > releves-flotte-$(date +%H%M).jsonl
# stderr = tableaux humains, stdout = JSONL machine
```
Par device et global : latence min/mean/max, ratio d'acceptation, répartition
des drops, comportement du budget. Ces chiffres alimentent la décision
« L1-001 tient sur vraie flotte → ouvrir L1-002 ».
