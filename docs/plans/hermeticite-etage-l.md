# Herméticité étage L — le nœud de banc refuse le monde extérieur

> Investigation + design, 2026-07-21. Prérequis du capstone R4-F
> (`banc-roles-sous-charge.md` §3). Finding d'origine : « fuite d'herméticité
> ENTRANTE du banc » (handoff `session-handoff-2026-07-21`).

## 1. Constat d'investigation (file:line vérifiés)

1. **Le « trio isolated » ne coupe PAS le transport relais.** Chemin réel dans
   `tom-transport/src/node.rs` : `configured_relays` vide → discovery HTTP
   (177) → DNS TXT `_relay._tcp.tom-protocol.org` (206-235) → fallback
   compile-time (237-243). Or `.cargo/config.toml` (machine, hors git) injecte
   `TOM_EXTRA_FALLBACK_RELAY = "http://82.67.95.8:3340"` (le NAS PUBLIC de la
   flotte) dans tous les builds → `fallback_relay_urls()` non-vide
   (`config.rs:31-36`, `option_env!` ligne 24) → branche `(false, false)` →
   `RelayMode::custom(NAS)` (node.rs:246-248). **Tout nœud de banc « isolé »
   se connecte donc au relais de la flotte réelle.** Canal bidirectionnel
   partagé avec le terrain.
2. **`local_discovery(false)` coupe bien mDNS des deux côtés** (annonce ET
   résolution) : `MdnsAddressLookup` n'est jamais construit (node.rs:389-445),
   et la publication swarm-discovery vit uniquement dans ce service
   (`tom-connect/src/address_lookup/mdns.rs:240-257`). L'hypothèse « la flotte
   trouve le banc par mDNS » du handoff est donc **infirmée** : sans annonce,
   pas de découverte mDNS possible.
3. **Maillon restant NON PROUVÉ** : comment un pair de la flotte apprend le
   `node_id` d'un nœud de banc (nécessaire pour dialer via le relais commun —
   en QUIC iroh le dial exige le node_id). Candidats : résidus d'annonces DHT
   pré-P1, contamination inter-scénarios (shutdown timeout 5 s), autre. Le fix
   ne dépend pas de ce maillon : on ferme le canal (A) ET on refuse les
   entrantes (B). `data_dir: None` par défaut → la piste « carnet persisté
   partagé » est écartée (runtime/mod.rs:68, Default:143).

## 2. Fix — deux verrous indépendants

### A. `TomNodeConfig::hermetic()` — étanchéité par construction

Nouveau flag `hermetic: bool` (défaut `false`) + builder `.hermetic()` qui :
- force `n0_discovery = false` et `local_discovery = false` ;
- vide `relay_url` / `relay_urls` / `relay_discovery_url` (neutralise les env
  `TOM_RELAY_URL*` déjà lues par `new()`) ;
- dans `bind()` : saute les TROIS fallbacks (discovery HTTP, DNS TXT,
  compile-time) → `configured_relays` vide garanti → `RelayMode::Disabled` ;
- incompatible avec `relay_only` (erreur de config explicite).

Non exposé FFI (une app de prod ne doit jamais être hermétique). Rappel :
l'herméticité complète = `hermetic()` (transport) + `enable_dht: false` OU
`rendezvous_namespace: Some(test)` (couche protocole, P1).

### B. Étage L — gate d'accept par liste blanche (défense en profondeur)

- `GatedHandler<H: ProtocolHandler>` dans tom-transport : wrappe les DEUX
  handlers ALPN (tom + gossip, node.rs:341-344). Dans `accept()`, lit
  `connection.remote_id()` : si une liste est active et que l'id n'y est pas →
  `connection.close()` immédiat + trace `warn`, AUCUNE délégation. Sinon
  délègue à `H`. Lock `std::sync::RwLock` relâché avant tout await (leçon
  `pool-lock-hostage`).
- Liste partagée `Arc<RwLock<Option<HashSet<EndpointId>>>>` créée au bind,
  setter public `TomNode::set_allowed_peers(Option<HashSet<NodeId>>)`.
  `None` (défaut) = accepte tout → comportement prod strictement inchangé.
- Séquencement banc (déjà compatible `spawn_mesh_wired`) : bind des N nœuds →
  collecte des ids → `set_allowed_peers(Some(ids))` sur chacun → attach
  runtime → câblage. Fenêtre bind→set de quelques ms, acceptable (documenté).

## 3. Red-team (avant code)

- `hermetic()` activé en prod par erreur ? Non exposé FFI ; nom explicite ;
  doc « bench/tests uniquement ». Un nœud hermétique est inutilisable en
  terrain (aucun relais, aucune découverte) → se voit immédiatement.
- `set_allowed_peers` oublié ? Défaut `None` = comportement actuel, aucun
  changement de surface pour la prod.
- Usurpation d'un id câblé ? Impossible sans la clé privée : l'id EST la clé
  du handshake TLS.
- R2 (B revient) : la liste est par node_id et B garde son identité
  (`identity_path`) → aucun impact.
- Findings review-oracle (21/07, verdict MAJEUR non-bloquant, documentés dans
  le code) : (1) fenêtre bind→set — le gate part à `None` pendant quelques ms ;
  contrat : poser la liste AVANT toute connexion (le banc le fait). (2) Pas de
  sweep rétroactif — `set_allowed_peers` ne ferme pas les connexions déjà
  établies ; raffinement futur si un scénario l'exige (fermer via le pool les
  connexions hors liste au moment du set). (3) Asymétrie assumée : seules les
  ENTRANTES sont filtrées, chaque nœud gate sa propre porte.

## 4. Câblage banc + oracles

- Remplacer le trio par `.hermetic()` + liste blanche dans :
  `scenario_roles_charge.rs` (spawn_mesh_wired + R2), `scenario_courbe.rs`,
  `scenario_invariants.rs`, `scenario_chaos_monkey.rs`.
- **Bénéfice mesurable** : les oracles « étrangers » rendus RELATIFS à cause
  du chatter (handoff 21/07) repassent en ABSOLUS (0 étranger toléré).
- Vérité terrain : re-run `roles-charge --scenario all` → 22 oracles PASS et
  0 Known étranger, flotte allumée ou pas.

## 5. Tests

1. `hermetic_bind_has_no_relay` : bind hermetic → aucun relais configuré sur
   l'endpoint (RelayMode::Disabled observable), même avec
   `TOM_EXTRA_FALLBACK_RELAY` injecté à la compilation.
2. `gate_rejects_unlisted_peer` : A avec allowed={B} ; C→A refusé (aucun
   message ne passe), B→A passe.
3. `gate_none_accepts_all` : défaut prod inchangé.
4. Non-régression : 22 oracles roles-charge + clippy/test workspace.
