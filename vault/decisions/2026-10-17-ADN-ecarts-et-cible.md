---
date: 2026-10-17
tags: [adn, mission, ecarts, cible, amorcage]
mandat: M0012
source_qui_fait_foi: docs/MISSION.md (mise à jour 2026-10-17)
base: origin/main 99aa090
---

# ADN : écarts entre `docs/MISSION.md` et le code, et cible proposée

> Lecture seule du code au commit `99aa090`. Aucun nœud lancé, aucun chantier lancé.
> `[VÉRIFIÉ]` = lu dans le code ou constaté par commande ; `[HYPOTHÈSE]` = non prouvé, avec le moyen de le vérifier.
> Le texte qui fait foi est `docs/MISSION.md` mis à jour le 2026-10-17 (décisions du fondateur, §1.1, §1.2, §1.7, §1.9, §1.10, §8).

## 1. Amorçage : réponse à la question du fondateur

Question : peut-on se passer de tout service externe quand seules la VM Freebox et l'Apple TV tournent ?

### 1.1 Mécanismes de découverte présents dans le code

| # | Mécanisme | Preuve | Externe (tiers) ? | Coûte de l'argent ? | Actif par défaut ? |
|---|---|---|---|---|---|
| 1 | **mDNS LAN** (`swarm-discovery`, service `tomv1`) | `tom-connect/src/address_lookup/mdns.rs:59,73` ; branché `tom-transport/src/node.rs:411-430` | Non (multicast local) | Non | **Oui** : `local_discovery: true` (`tom-transport/src/config.rs:173`) ; l'app Apple le force à `true` (`apps/tom-node-tvos/TomNode/Services/TomNodeService.swift:671`) `[VÉRIFIÉ]` |
| 2 | **Gossip** (HyParView, vue active 5) + redial des pairs persistés vus il y a moins de 5 min (16 au plus) | `tom-gossip/src/proto/hyparview.rs:202` ; `tom-protocol/src/runtime/mod.rs:982-1001` | Non | Non | Oui, mais il faut un premier contact `[VÉRIFIÉ]` |
| 3 | **Relais embarqué** dans chaque nœud + publication par gossip + port UPnP | FFI : `enable_embedded_relay` et `_publication` à `true` si absents (`tom-protocol-ffi/src/lib.rs:368-369`), et le wrapper Swift ne les envoie pas (`sdk/swift/TomProtocolKit/Sources/TomProtocolKit/TomNodeWrapper.swift:78-108`). Runtime Rust seul : `false` (`tom-protocol/src/runtime/mod.rs:148-149`). Publication globale seulement si l'adresse est publique (`runtime/state.rs:865`, `:3888`). UPnP : `runtime/embedded_relay.rs:174-180` → `state.rs:3133` | Non (nos appareils) | Non | Oui sur les apps Apple ; sur le NAS via `tom-chat --self-relay` (`tom-tui/src/main.rs:661-664,875-877`) `[VÉRIFIÉ]` |
| 4 | **Relais injecté à la compilation** (`TOM_EXTRA_FALLBACK_RELAY`) | `tom-transport/src/config.rs:24`, utilisé `node.rs:250-252` ; fixé par `.cargo/config.toml` local, hors git (`.gitignore:67`), qui pointe sur l'adresse publique de la Freebox, port 3340 | Non (infra du fondateur) | Non | Dans les builds de la machine du fondateur seulement, pas dans le code publié `[VÉRIFIÉ]` |
| 5 | **Liste de relais publics par défaut** | `DEFAULT_RELAY_URLS = &[]` (`config.rs:12`) | — | — | Vide. La roadmap R17 (« URLs déjà en défaut committé », `vault/40-roadmap.md:293`) est périmée `[VÉRIFIÉ]` |
| 6 | **Repli DNS TXT** `_relay._tcp.tom-protocol.org` | `config.rs:29`, `node.rs:219-246` | Oui (résolveur DNS + domaine) | Domaine à acheter | Tenté si aucun relais n'est configuré. **`tom-protocol.org` est NXDOMAIN le 2026-10-17** (`dig SOA` → `status: NXDOMAIN`) : ce repli ne donne rien `[VÉRIFIÉ]` |
| 7 | **Découverte HTTP de relais** (`TOM_RELAY_DISCOVERY_URL`) | `config.rs:157`, `node.rs:190-216` | Selon l'URL | Selon l'hébergement | Non (variable d'environnement absente par défaut) `[VÉRIFIÉ]` |
| 8 | **Pkarr/DNS n0** (preset `N0`) | `n0_discovery: true` (`config.rs:172`) ; app : `n0Discovery = true` (`TomNodeService.swift:164`) ; preset : `PkarrPublisher::n0_dns()` + `DnsAddressLookup::n0_dns()` (`tom-connect/src/endpoint/presets.rs:51-67`) → `dns.iroh.link` | **Oui (société n0)** | Pas de compte ni de clé dans le code. Gratuité et durée du service : `[HYPOTHÈSE]` (vérifier les conditions publiées par n0) | **Oui** `[VÉRIFIÉ]` |
| 9 | **Relais n0 par défaut** (`*.relay.n0.iroh-canary.iroh.link`) | `tom-connect/src/defaults.rs:26-32`, `endpoint.rs:1428` ; choisis seulement si aucun relais n'est configuré ET `n0_discovery = true` (`node.rs:259-276`) | Oui (n0) | Non | Cas résiduel. ToM n'est pas wire-compatible iroh (`X-Tom-*`, ALPN `tom-*`) : ces relais sont très probablement inutilisables `[HYPOTHÈSE]` (vérifier par un nœud sans relais configuré, logs de connexion relais) |
| 10 | **DHT Mainline publique** + rendez-vous partagé (8 slots, entrées signées) | `enable_dht: true` (`runtime/mod.rs:145`, FFI `lib.rs:364`) ; `Dht::client()` (`tom-dht/src/lib.rs:218`) ; amorces de `mainline 6.1.1` : `router.bittorrent.com`, `dht.transmissionbt.com`, `dht.libtorrent.org`, `relay.pkarr.org` (`mainline-6.1.1/src/rpc.rs:42-46`) ; rendez-vous `tom-dht/src/lib.rs:40-47`, `runtime/loop.rs:1354,1427-1430` | **Oui** (réseau BitTorrent public, 4 hôtes d'amorce tenus par des tiers, résolution DNS) | Non | **Oui** `[VÉRIFIÉ]` |
| 11 | **Pkarr via DHT** | `tom-connect/src/address_lookup/pkarr/dht.rs` | — | — | Présent dans `tom-connect`, non branché dans `tom-transport` ni `tom-protocol` (grep `DhtAddressLookup`) `[VÉRIFIÉ]` |
| 12 | **Ajout manuel d'un pair** | `tom_node_add_peer_addr` (`tom-protocol-ffi/src/lib.rs:1721`) | Non | Non | À la demande `[VÉRIFIÉ]` |
| 13 | **Ouverture de port par l'API Freebox** (`tom-gateway`, R16c) | `vault/40-roadmap.md:122` | Non | Non | Pas construit `[VÉRIFIÉ]` |

**Aucun mécanisme ne coûte de l'argent aujourd'hui.** Le seul qui en coûterait est R17 (VPS + domaine), qui contredit la règle « zéro dépense externe » de MISSION §1.1 et §6.7.

### 1.2 Scénario : seules la VM Freebox et l'Apple TV tournent, aucun service externe

**En LAN — oui en principe, sans aucun tiers.**
- mDNS est actif des deux côtés (n°1). Le NAS l'a s'il ne tourne pas en `--isolated` (`tom-tui/src/main.rs:889-893`). La ligne `ExecStart` du NAS n'est pas dans le dépôt : `[HYPOTHÈSE]` d'après `CLAUDE.md` (« `tom-chat --self-relay` »). Vérifier avec `systemctl cat tom-node` sur le NAS.
- Deux réserves `[HYPOTHÈSE]` :
  - le multicast doit passer entre la VM (pont réseau Freebox) et l'Apple TV ;
  - tvOS doit laisser passer le multicast brut de `swarm-discovery` : le dépôt n'a aucun entitlement multicast, et le code note que le broadcast est filtré sur iOS (`TomNodeService.swift:167`).
  Vérification : couper l'accès WAN de la box, lancer les deux nœuds avec `n0_discovery=false` et `enable_dht=false`, chercher les traces `MdnsDiscovered` (`tom-transport/src/node.rs:426-429`) dans les logs des deux côtés.
- Repli LAN : le relais embarqué du NAS, si l'Apple TV en connaît l'adresse. L'adresse injectée au build est l'adresse **publique** de la Freebox : depuis le LAN, cela suppose que la box renvoie le trafic vers elle-même (NAT hairpin) `[HYPOTHÈSE]`.
- Après un redémarrage simultané, le redial des pairs persistés (n°2) ne vaut que pour les pairs vus dans les 5 dernières minutes.

**Depuis Internet (par exemple un iPhone en 4G qui rejoint) — seulement par un point d'entrée connu d'avance.**
- Sans tiers, le seul point de rencontre est le relais du NAS à l'adresse publique de la Freebox (port 3340 ouvert), injecté au build (n°4). C'est l'infra du fondateur, pas un tiers, et elle ne coûte rien. Elle est fixe : c'est le « niveau 1 — naissance » de la pompe d'amorçage (`docs/plans/2026-03-30-bootstrap-pump-module.md:79-81`), pas encore une fonction tournante.
- Le relais embarqué de l'Apple TV peut obtenir un port public par UPnP et se publier (n°3), mais par gossip seulement, donc après un premier contact.
- **Sans adresse connue d'avance, aucun mécanisme du code ne permet une première rencontre depuis Internet sans tiers.** La découverte à connaissance nulle repose sur la DHT Mainline (n°10) et sur Pkarr/DNS n0 (n°8), deux tiers gratuits. `[VÉRIFIÉ]` par l'inventaire ci-dessus.
- Une adresse IP littérale évite toute résolution DNS. Que l'adresse publique de la Freebox soit fixe : `[HYPOTHÈSE]` (à vérifier dans l'interface Freebox).

**Verdict.** Zéro dépense : oui, déjà le cas. Zéro tiers : oui en LAN (sous réserve du multicast) ; non pour une première rencontre depuis Internet, sauf via l'adresse publique de la Freebox connue d'avance. La question reste ouverte (MISSION §8, question 2).

Effet de bord noté : `n0_discovery` étant à `true` par défaut (app comprise), chaque appareil publie son identifiant et ses adresses chez n0 (`dns.iroh.link`). C'est un tiers qui voit le réseau, en tension avec §1.8 (souverain).

## 2. Carte des écarts

| Principe (MISSION 2026-10-17) | État du code | Preuve `fichier:ligne` | Écart | Lot proposé |
|---|---|---|---|---|
| §1.7 Jeton = note interne (« grain de riz ») sur les personnes qui participent | Chaque nœud calcule **sa propre vue** du score des pairs qu'il voit relayer (contribution seule, décroissance 5 %/h). Aucune note partagée par le réseau. Le score diffusé dans les annonces de rôle est **auto-déclaré** | `crates/tom-protocol/src/roles/scoring.rs:87-120` ; `roles/manager.rs:44-60` ; `discovery/role_sync.rs:9-17` | Pas de composante « usage » ; pas de note commune ; un nœud peut annoncer le score qu'il veut | C3 |
| §1.2 Rôle reçu **du réseau** (besoin + note + contexte) | Deux rôles seulement (`Peer`, `Relay`). Bascule sur seuils fixes 10 / 2 du score local. L'annonce de rôle d'un pair, signée par lui-même, est **acceptée telle quelle** | `relay.rs:43-48` ; `roles/manager.rs:13-16,161-210` ; `runtime/state.rs:907-943` (l.943 `updated_peer.role = announce.new_role`) | Un nœud peut se déclarer `Relay` : contredit « impossible à truquer » (§1.2) | C2 |
| §1.2 Contexte : modèle d'appareil (puissance), place disponible | Rien dans l'assignation. Le relais embarqué tourne sur **tous** les appareils Apple, quel que soit le besoin. Les facteurs d'hôte du backup ne sont alimentés que par les tests (valeurs par défaut 50/50/50 en production) | `tom-protocol-ffi/src/lib.rs:368-369` ; `backup/store.rs:373-380` ; `backup/types.rs:199-215` ; seuls appelants : `backup/coordinator.rs:513` (sous `#[cfg(test)]`, l.310), `tests/backup_integration.rs:160,186` | Contexte absent de toute décision de rôle | C3 |
| §1.2 Note de connexion (fiable / pas fiable) | Liveness et taux de succès existent (`success_rate` dans le score, détection zombie), mais pas de note de fiabilité qui pèse sur le rôle | `roles/scoring.rs:88-93` ; `CLAUDE.md` Known Limitations n°1 (`runtime/loop.rs`, zombie) | Mesures partielles, pas d'entrée d'assignation | C3 |
| §1.2 Besoin du moment | Aucune mesure du besoin. Le hub de groupe est élu par le NodeId le plus bas (prévisible) | `group/election.rs:37-40,82` | Ni besoin, ni imprévisibilité | C2 (constat) / §8 q.3-4 |
| §1.9 Tout le monde s'observe (jusqu'à 7) | Témoins de présence : 2 à 4 selon densité et activité, pour les appareils faibles. Vue active gossip : 5 voisins | `presence/relay_view.rs:33,38,56-68` ; `tom-gossip/src/proto/hyparview.rs:202` | Plafond 4 (et non 7) ; pas « tout le monde » | C4 |
| §1.9 Observateur passif abandonné, témoins non | Pas de rôle observateur (conforme). Décision du 12/07 désormais limitée à la « présence passive » | `relay.rs:43-48` ; `vault/20-decisions.md` (ligne du 2026-07-12) | Aucun (portée de la décision précisée le 2026-10-17) | — |
| §1.9 Double dépense : chantier ouvert | Absente (aucun wallet, aucune signature agrégée) | état des lieux `vault/reprise/2026-10-17-ETAT-DES-LIEUX.md:131` | Conforme au périmètre (pas d'argent) | §8 q.1 |
| §1.10 Subnet = cercle réel, sans règle fixe | Clusters BFS sur le graphe des messages (arête ≥ 3 messages), taille **fixe** 3 à 10, dissolution après 5 min. Chaque nœud n'enregistre que les messages `Chat` **reçus par lui** : sa vue = ses propres correspondants | `discovery/subnet.rs:15-30,284-300` ; `runtime/state.rs:1965-1969` | Bornes fixes (contraire à « pas de règle fixe ») ; vue locale seulement | C5 |
| §1.10 Vases communicants (un petit subnet s'appuie sur un plus robuste) | Les subnets ne produisent qu'un événement d'observabilité. Ni routage, ni backup, ni rôle ne les consomment | `runtime/state.rs:3255-3259` ; état des lieux `:123` | Absent | C5 |
| §1.1 Amorçage, zéro dépense externe | Zéro dépense : tenu. Amorçage Internet = adresse fixe de la Freebox (build local) ou tiers gratuits (DHT, n0) | §1 ci-dessus | Pas de fonction d'amorçage tournante ; n0 actif par défaut | C1 |
| §6.7 Zéro dépense externe | R17 prévoit des VPS et un domaine | `vault/40-roadmap.md:292-295` | R17 à revoir à la lumière de §6.7 | C1 (constat) |

Hors des 5 décisions, documents restés périmés (signalés, non corrigés ici) : `vault/00-brief.md` (phase, IP du NAS, signaling) ; `vault/40-roadmap.md:293` (R17 « URLs déjà en défaut committé ») ; `CLAUDE.md` ADR-002 « Bootstrap Elimination DONE » alors que `n0_discovery` est à `true` par défaut ; critère MISSION §5 « découverte via DHT rendezvous + Pkarr uniquement », qui repose sur deux tiers (lié à §8 q.2).

## 3. Cible proposée (aucun chantier lancé)

Ordonnée du plus petit pas au plus grand. Chaque chantier protocolaire commence par un document de conception validé par le fondateur.

### C1 — Prouver l'amorçage maison sans tiers, sur le réseau actuel
- **But** : répondre par la mesure à MISSION §8 q.2, sans écrire de code protocole. Les interrupteurs existent déjà (`n0_discovery`, `enable_dht`, `local_discovery`).
- **Critère mesurable** : avec `n0_discovery=false` et `enable_dht=false` sur la VM Freebox et l'Apple TV :
  1. en LAN, découverte par mDNS (trace `MdnsDiscovered` des deux côtés) puis message chiffré acquitté dans les deux sens, en moins de 60 s après un démarrage simultané ;
  2. un iPhone en 4G joint les deux via le relais du NAS (adresse connue d'avance) ;
  3. **zéro** requête DNS vers `dns.iroh.link` ni vers les amorces DHT, prouvé par capture (`tcpdump` sur la VM, journal DNS de la box).
- **Risques** : multicast filtré par tvOS ou par le pont de la VM ; hairpin NAT absent. L'app ne persiste pas `n0Discovery` (`TomNodeService.swift:164`) : il faut un build de test ou un réglage.
- **Irréversibles** : aucun (réglages d'exécution, retour arrière immédiat).

### C2 — Le rôle ne se déclare plus : il se constate
- **But** : fermer l'écart « impossible à truquer ». Le rôle `Relay` d'un pair vient de ce que le nœud local observe, pas de l'annonce du pair (`runtime/state.rs:943`).
- **Critère mesurable** : test qui échoue avant et passe après — un pair qui s'annonce `Relay` avec un score fort, sans relais observé, reste `Peer` dans la topologie locale ; non-régression des promotions réelles (`roles/manager.rs` tests).
- **Risques** : promotions plus lentes sur un réseau de 5 appareils (peu d'observations) ; effet sur `RelaySelector`.
- **Irréversibles** : aucun (pas de changement de format sur le fil si l'annonce garde son schéma).

### C3 — Le contexte de l'appareil entre dans la note et le rôle
- **But** : brancher MISSION §1.2 (puissance, place disponible, fiabilité de connexion) et §1.7 (note contribution / usage). Alimenter `HostFactors` en production, mesurer un profil d'appareil sans saisie utilisateur, et réserver le relais embarqué aux appareils que le contexte désigne.
- **Critère mesurable** : sur la flotte réelle, la VM Freebox (secteur, toujours en ligne) et l'iPhone (batterie, souvent suspendu) reçoivent des rôles différents, justifiés par des valeurs mesurées et visibles dans les métriques internes (jamais à l'écran, verrou #6). Test unitaire de la pondération.
- **Risques** : la formule est une question ouverte (§8 q.3) : il faut l'arbitrage du fondateur avant de coder ; sur iOS, mesurer la place libre et la puissance passe par des API système à vérifier.
- **Irréversibles** : changer le schéma de `RoleChangeAnnounce` ou du profil sur le fil touche le protocole (version à gérer entre builds de la flotte).

### C4 — Les témoins « jusqu'à 7 »
- **But** : passer de 2-4 témoins de présence à un principe « tout le monde s'observe, jusqu'à 7 voisins », dans la ligne de `docs/plans/CARTE-rendezvous-tournant.md`.
- **Critère mesurable** : sur un banc de N nœuds (N = 5, 10, 20), chaque nœud a min(N-1, 7) observateurs, sans vue globale ; le faux `Online` par un témoin Sybil unique reste rejeté (non-régression de `presence/quorum.rs`).
- **Risques** : charge sur l'Apple TV et les mobiles ; amplification Sybil (cap global existant, `presence/mod.rs::allow_response()`).
- **Irréversibles** : messages de présence sur le fil (compatibilité entre builds).

### C5 — Le subnet devient le cercle réel, et les vases communiquent
- **But** : faire du cercle de contacts l'unité d'organisation (§1.10) : bornes adaptatives au lieu de 3-10, subnet consommé par le backup et le choix des relais, appui d'un petit subnet sur un subnet plus robuste.
- **Critère mesurable** : sur un banc, les copies de backup d'un message vont d'abord à des membres du cercle de l'émetteur ou du destinataire ; un cercle de 2 appareils trouve un relais ou un gardien dans un cercle plus robuste, sans état visible par l'utilisateur.
- **Risques** : vie privée (un cercle révèle qui parle à qui : `from`/`to` déjà en clair pour les relais, état des lieux `:125`) ; interaction avec le backup TTL 24 h.
- **Irréversibles** : aucun tant que le subnet reste une vue locale ; un échange de subnets sur le fil en serait un.

Ordre recommandé : **C1 d'abord** (aucun code, il tranche par des faits la question la plus concrète du fondateur), puis C2 (petit correctif), puis C3 après arbitrage de §8 q.3, puis C4 et C5 après documents de conception.
