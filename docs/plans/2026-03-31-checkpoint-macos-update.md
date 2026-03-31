# 2026-03-31 — Checkpoint avant MAJ macOS

## Branche

- `copilot/bootstrap-pump-skeleton`

## État validé

- Le chemin **`PeerPresent -> GossipNeighborUp -> delivery` fonctionne**.
- Validation faite via le test ciblé :
  - `cargo test -p tom-integration-tests peer_present_auto_discovery_leads_to_neighbor_up_and_delivery -- --ignored --nocapture`
- Résultat observé avant checkpoint :
  - `bootstrap: accepted peer hint ... source=PeerPresent`
  - `NeighborUp` des deux côtés
  - `Alice -> Bob via PeerPresent: OK`

## Travaux code présents dans le workspace

### Bootstrap pump / LAN-first
- `tom-transport` : ajout `local_discovery`, `BootstrapHint`, `bootstrap.rs`, branchement mDNS dans `TomNode`
- `tom-protocol` : ajout `runtime/bootstrap.rs`, unification du pipeline `add_peer_addr() + join_peers()` dans `runtime/loop.rs`
- `tom-stress` : `--no-n0-discovery` active maintenant aussi `local_discovery(true)`
- tvOS / FFI : `local_discovery` propagé jusqu’au runtime

### Relay fanout configurable
- `tom-relay` : ajout `peer_present_k` configurable
- presets créés :
  - `deploy/peerpresent-k/tom-relay-k8.toml`
  - `deploy/peerpresent-k/tom-relay-k16.toml`
  - `deploy/peerpresent-k/tom-relay-k32.toml`
- doc de mesure : `docs/plans/2026-03-30-peerpresent-k-measurement.md`

### Harness de mesure ajouté
- nouveau binaire : `crates/tom-integration-tests/src/bin/peer_present_k_matrix.rs`
- objectif : comparer `k=8/16/32` avec assez de nœuds pour que le fanout ait un effet réel
- paramètres supportés :
  - `PEER_PRESENT_KS`
  - `PEER_PRESENT_NODE_COUNT`
  - `PEER_PRESENT_TRIALS`
  - `PEER_PRESENT_BOOTSTRAP_TIMEOUT_SECS`
  - `PEER_PRESENT_DELIVERY_TIMEOUT_SECS`
  - `PEER_PRESENT_TRIAL_TIMEOUT_SECS`

## Blocage actuel

Le benchmark local multi-nœuds sur **une seule machine macOS** n’a pas encore donné de chiffres exploitables pour `k=8/16/32`.

### Symptôme
Le harness avance jusqu’à :
- `waiting for first NeighborUp events`
- puis `probing delivery ...`

mais la topologie se dégrade / se fige avant de produire une comparaison propre.

### Diagnostic actuel
Le problème semble venir de la **tempête de chemins directs / path opening** sur topologie mono-machine, pas du mécanisme `PeerPresent` lui-même.

Le sample pris du process pointait surtout vers :
- `tom_connect::socket::remote_map::remote_state::RemoteStateActor`
- `select_path`
- `open_path`
- `tom_quinn::connection::Connection::open_path_ensure`

Conclusion provisoire :
- **`PeerPresent` est OK**
- **la mesure locale massive est polluée par le runtime réseau direct**
- il manque un mode simple de mesure **relay-only / no-direct-path** pour isoler proprement `peer_present_k`

## Dernier état Git connu

### Fichiers modifiés / créés au checkpoint

Modifiés :
- `Cargo.lock`
- `apps/tom-node-tvos/TomNode/Models/TomNodeWrapper.swift`
- `apps/tom-node-tvos/TomNode/Services/TomNodeService.swift`
- `crates/tom-connect/Cargo.toml`
- `crates/tom-connect/src/address_lookup/mdns.rs`
- `crates/tom-integration-tests/tests/peer_present_auto_discovery.rs`
- `crates/tom-protocol-ffi/Cargo.lock`
- `crates/tom-protocol-ffi/src/lib.rs`
- `crates/tom-protocol-ffi/src/types.rs`
- `crates/tom-protocol/src/runtime/embedded_relay.rs`
- `crates/tom-protocol/src/runtime/loop.rs`
- `crates/tom-protocol/src/runtime/mod.rs`
- `crates/tom-relay/src/main.rs`
- `crates/tom-relay/src/server.rs`
- `crates/tom-relay/src/server/clients.rs`
- `crates/tom-relay/src/server/http_server.rs`
- `crates/tom-relay/src/server/testing.rs`
- `crates/tom-stress/src/campaign.rs`
- `crates/tom-stress/src/main.rs`
- `crates/tom-stress/src/responder.rs`
- `crates/tom-transport/Cargo.toml`
- `crates/tom-transport/src/config.rs`
- `crates/tom-transport/src/lib.rs`
- `crates/tom-transport/src/node.rs`

Créés :
- `crates/tom-integration-tests/src/bin/peer_present_k_matrix.rs`
- `crates/tom-protocol/src/runtime/bootstrap.rs`
- `crates/tom-transport/src/bootstrap.rs`
- `deploy/peerpresent-k/README.md`
- `deploy/peerpresent-k/tom-relay-k8.toml`
- `deploy/peerpresent-k/tom-relay-k16.toml`
- `deploy/peerpresent-k/tom-relay-k32.toml`
- `docs/plans/2026-03-30-bootstrap-pump-skeleton.md`
- `docs/plans/2026-03-30-claude-bootstrap-pump-mission.md`
- `docs/plans/2026-03-30-parallel-copilot-claude-worksplit.md`
- `docs/plans/2026-03-30-peerpresent-k-measurement.md`

## Process temporaire

- Tous les process de mesure / relay / stress ont été stoppés avant la MAJ macOS.
- Vérification faite : plus de `peer_present_k_matrix`, `tom-relay` ou `tom-stress` actifs.

## Reprise recommandée après MAJ macOS

### 1. Vérifier l’état local
- `git branch --show-current`
- `git status --short`

### 2. Revalider le chemin PeerPresent sain
- `cargo test -p tom-integration-tests peer_present_auto_discovery_leads_to_neighbor_up_and_delivery -- --ignored --nocapture`

### 3. Choisir la suite

#### Option A — la plus utile
Introduire un mode de mesure **relay-only** / sans tentative de chemins directs pour isoler `peer_present_k`, puis relancer la matrice `k=8/16/32`.

#### Option B — mesure terrain
Faire la comparaison `k=8/16/32` sur vraie topologie multi-machine (Mac / NAS / tvOS) si l’objectif est la valeur terrain immédiate.

### 4. Commande de reprise du harness actuel (si on veut retenter)
- `cargo build -p tom-integration-tests --bin peer_present_k_matrix`
- `PEER_PRESENT_KS=8 PEER_PRESENT_NODE_COUNT=34 PEER_PRESENT_TRIALS=1 PEER_PRESENT_BOOTSTRAP_TIMEOUT_SECS=20 PEER_PRESENT_DELIVERY_TIMEOUT_SECS=10 PEER_PRESENT_TRIAL_TIMEOUT_SECS=60 cargo run -p tom-integration-tests --bin peer_present_k_matrix`

## Décision recommandée à la reprise

Ne pas repartir directement dans une matrice multi-nœuds brute sur un seul Mac.

La prochaine action la plus rentable est :
- soit **ajouter un mode relay-only de mesure**,
- soit **basculer la campagne `k=8/16/32` sur une topologie multi-machine réelle**.
