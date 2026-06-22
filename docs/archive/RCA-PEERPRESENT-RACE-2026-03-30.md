# RCA — PeerAnnounce 12/13 (race condition self-relay)

Date: 2026-03-30
Status: **CORRIGE** (commit `5c38cdf`)

## Bug

Le publisher ne se connecte pas comme client à son propre relay embarqué. Fenêtre muette de ~20-26s, discovery et messages bloqués.

## Cause

Race condition : le probe relay initial de l'Endpoint fire avant que le relay embarqué démarre. Probe échoue, `preferred_relay=None`, retry dans 20-26s.

## Fix

`reprobe_relays()` exposé via `Endpoint` → `TomNode`, appelé juste après `embedded_relay.start()` dans `loop.rs`. Force un re-probe immédiat.

## Preuve locale

Scénario 3 commandes (publisher + obs1 + obs2, sans relay externe) :
- Embedded relay started OK
- PeerDiscovered via Announce pour obs1 et obs2
- 315+ messages, 0 erreur, path upgrade direct 0.65ms RTT
- Commit: `5c38cdf`

## Validation terrain (2026-03-30)

Mac (observer) ↔ NAS (publisher + self-relay), WAN via `http://82.67.95.8:3340` :

- Discovery auto : PASS (PeerDiscovered via Announce, ~10s)
- Path upgrade : Direct, 4-5ms RTT
- Endurance 11 min : **17 543 msgs Mac / 17 570 msgs NAS, 0 erreur**
- Fenêtre morte au démarrage : aucune

**Bug clos.** Fix validé local + terrain.
