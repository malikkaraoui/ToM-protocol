# S1 — Runbook smoke test 4 process

## Architecture

```
T0: tom-relay --dev (bootstrap relay, port 3340)
T1: tom-tui publisher (embedded relay + publication gossip + bot-ping)
T2: tom-tui observer1 (relay-discovery + bot auto-reply)
T3: tom-tui observer2 (relay-discovery + bot auto-reply)
```

- T0 assure le bootstrap deterministique initial (meme relay = PeerPresent garanti).
- Le relay embarque de T1 ne bootstrap PAS la rencontre. Il prouve la publication/decouverte/enrichissement transport.
- T1 envoie un ping periodique au premier peer decouvert. T2/T3 auto-repondent.

## Prerequis

```bash
cargo build -p tom-relay -p tom-tui
```

## Lancement (4 terminaux)

### T0 — Relay local (bootstrap)

```bash
cargo run -p tom-relay -- --dev
```

Attendre : `listening on http://127.0.0.1:3340`

### T1 — Publisher

```bash
TOM_RELAY_URL=http://127.0.0.1:3340 cargo run -p tom-tui -- \
  --username publisher --bot --bot-ping 10 \
  --embedded-relay --embedded-relay-publish \
  --relay-ttl 60 --relay-publish-interval 25
```

### T2 — Observer 1

```bash
TOM_RELAY_URL=http://127.0.0.1:3340 cargo run -p tom-tui -- \
  --username obs1 --bot \
  --relay-discovery --relay-ttl 60
```

### T3 — Observer 2

```bash
TOM_RELAY_URL=http://127.0.0.1:3340 cargo run -p tom-tui -- \
  --username obs2 --bot \
  --relay-discovery --relay-ttl 60
```

## Intervention humaine

S'arrete apres le lancement des 4 commandes. Tout le reste est automatique.

## Criteres binaires de succes

### Discovery prouvee

| # | Critere | Terminal | Deterministe |
|---|---------|----------|-------------|
| 1 | `Embedded relay started: http://...` | T1 | oui |
| 2 | `Gossip neighbor up` (entre noeuds) | T1, T2, T3 | oui (meme relay) |
| 3 | `Peer discovered` | T1, T2, T3 | oui (gossip PeerAnnounce) |
| 4 | `Relay discovered: ... -> http://...` | T2, T3 | oui (gossip RelayReadyAnnounce) |
| 5 | `Transport relay added: http://...` | T2, T3 | oui (relay_discovery=true) |

### Message prouve

| # | Critere | Terminal | Deterministe |
|---|---------|----------|-------------|
| 6 | `ping #1 -> ...` (T1 envoie) | T1 | oui (bot-ping) |
| 7 | `#1 from ... "ping #1..."` (T2 ou T3 recoit) | T2 ou T3 | oui |
| 8 | `replied: "recu 5/5 malik (msg #1)"` | T2 ou T3 | oui |

## Point ou l'automatique prend le relais

Quand `Gossip neighbor up` apparait — PeerPresent a fonctionne, gossip join est fait.

## Premier symptome si ca casse

- Pas de `Gossip neighbor up` apres 30s : les noeuds ne se voient pas.
  - Verifier que T0 est demarre et port 3340 libre.
  - Verifier `TOM_RELAY_URL` correctement passe.

- `Gossip neighbor up` OK mais pas de `Relay discovered` :
  - Le publisher ne publie pas. Verifier `--embedded-relay --embedded-relay-publish`.
  - Ou l'embedded relay n'a pas demarre (port deja pris).

- `Relay discovered` OK mais pas de `Transport relay added` :
  - L'observer n'a pas `--relay-discovery`.

- `ping #1` emis mais pas de reception cote observer :
  - Le premier peer decouvert n'est pas joignable (adresse non resolue).
  - Verifier `PeerDiscovered` dans les logs de T1.

## Ce que ce scenario ne prouve PAS

- La convergence n0 (on force TOM_RELAY_URL).
- La resilience (pas de churn, pas de kill/restart).
- La stabilite longue duree (pas de run 10 min ici).
- Le relay rotatif (l'embedded relay est statique).
