# ToM Protocol — rapport d'avancement technique

## 765 commits, 117 builds, un réseau sans maître qui tient debout

> 18 juillet 2026 · Rédigé depuis l'atelier, avec Claude Fable 5.
> Règle d'écriture héritée du projet : un jalon n'est « fait » que **mesuré sur la flotte réelle**, jamais sur un proxy vert. Tout chiffre de cet article a une source : un commit, un log, un fichier de code, une campagne instrumentée.

---

## TL;DR — les chiffres du jour

| Métrique | Valeur | Source |
|---|---|---|
| Naissance du repo | 2 février 2026 | `git log --reverse` |
| Commits | **765** | `git log` (18/07) |
| Builds versionnés | **117** (flotte réelle en 107) | `TomVersion.swift` |
| Code Rust du workspace | **~150 000 lignes** | comptage `crates/` |
| Tests | ~990 Rust (comptage 22/06, en croissance) + 771 TypeScript legacy | CLAUDE.md |
| Flotte réelle | 6 nœuds : Mac, iPad, iPhone ×2, Apple TV, NAS ARM64 | campagne 18/07 |
| Campagne du 18/07 | **1809 messages livrés, 0 perte finale, 0 crash** | rapport instrumenté |
| Gros transferts | 64 Mo livrés à ~6 Mo/s, E2E chiffré | mesure flotte |
| Re-mesh après SIGKILL | **8 secondes** (Apple TV tuée en plein trafic) | campagne 18/07 |
| Reconnexion à froid | nœud < 1 s, flotte complète 18 s | mesures 16/07 nuit |
| Red-team | 5 classes de DoS fermées · 6/6 kill-shots PoP fermés | vault + commits |

La roadmap Rust (R1→R12) est **complète**. La phase actuelle n'est plus « faire marcher », c'est « rendre incassable, puis invisible ».

---

## 1. La thèse de départ, pour ceux qui prennent le train en marche

ToM (*The Open Messaging*) part d'une inversion simple : **chaque appareil est à la fois client et relais**. Pas de serveur central, pas de compte, pas de token. Les messages routent par des pairs, chiffrés de bout en bout (Ed25519 + X25519 + XChaCha20-Poly1305), et le réseau ne garde que **le présent** : TTL de 24 heures, puis purge globale, sans exception.

Sept décisions ont été gravées dès le début (« LOCKED ») et n'ont jamais été démenties depuis :

1. **Livraison** : un message est livré ⟺ le destinataire émet un ACK.
2. **TTL** : 24 h de vie maximum, puis purge. Le réseau n'archive rien.
3. **L1 ancre l'état, n'arbitre jamais.**
4. **Réputation** : fade progressif, jamais de ban permanent.
5. **Anti-spam** : charge progressive (« l'arroseur arrosé »), jamais d'exclusion.
6. **Invisibilité** : la couche protocolaire est invisible pour l'utilisateur final.
7. **Portée** : une fondation universelle, comme TCP/IP — pas un produit.

La métaphore fondatrice est le **virus positif** : le réseau se nourrit du travail qu'on lui confie et meurt seulement si plus personne ne s'en sert. Plus d'hôtes = un organisme plus fort.

Cinq mois et demi plus tard, la question n'est plus « est-ce que c'est possible ? ». C'est mesuré.

---

## 2. La roadmap, tenue — et le point où le terrain l'a dépassée

### Phase 1 (TypeScript) puis Phase 2 (Rust) : R1 → R12, 100 %

| Phase | Contenu | Statut |
|---|---|---|
| TS 1-8 | Stack protocolaire complète, 771 tests | ✅ |
| R1-R2 | Envelope MessagePack, crypto, Router, ProtocolRuntime | ✅ |
| R3-R4 | Discovery gossip, keepalive, backup « virus », rôles dynamiques | ✅ |
| R5 | Groupes : hub, failover Primary→Shadow→Candidate, sender keys | ✅ |
| R6 | TUI, intégration, campagnes de stress | ✅ |
| **R7** | **Fork complet d'iroh + élimination du bootstrap** | ✅ |
| R8-R9 | Durcissement production, DHT, fiabilité de livraison | ✅ |
| R10-R11 | Récupération de groupe, antispam, nonce anti-replay, admin | ✅ |
| R12 | **Rendez-vous DHT zéro-config**, isolation recovery, anti-veille | ✅ |

La suite (R13→R18) a été **réordonnée par le réel** — et c'est le passage le plus instructif du trimestre. Le 16/07, la feuille de route acte noir sur blanc :

> « DIRECT same-WiFi ne tient jamais, 100 % RELAY » → **Faux — c'était un mensonge d'affichage.** `last_path` FFI était un singleton global écrasé par n'importe quel pair. Vérité terrain : **DIRECT partout**, y compris iPad↔Apple TV en IPv6 global bilatéral (11 ms), NAS↔iPad en IPv6 (4,5 ms).

Trois semaines de doute réseau se sont levées d'un coup : **le réseau était meilleur que ses instruments de mesure**. L'IPv6 global en hole-punch marche *sans toucher à la box* (le vrai verrou était `Connection::is_ipv6()` qui testait les chemins existants au lieu de la capacité du socket). Conséquence stratégique : R14 (IPv6 first-class) est passé **devant** R13 (porte automatique) en maturité — le hole-punch v6 est plus fiable que le NAT v4 et court-circuite le besoin d'ouvrir des ports.

Autre dépassement du plan par le terrain : le plan global du 06/07 notait l'attestation de présence (M1.1, première brique de la couche L1) « prête à coder ». **Dix jours plus tard elle tournait sur la flotte** — challenges signés bidirectionnels, éphémères 30 s, puis vue signée du relais avec quorum de témoins distincts (L1-003). Citation de la vision consolidée du 16/07 : *« la vitesse réelle du projet est supérieure à la vitesse planifiée »*.

---

## 3. L'acte de souveraineté : le fork

En février, ToM démarrait au-dessus d'iroh 0.96. En phase R7, **tout le chemin critique a été forké et assimilé** sous namespace `tom-*` (MIT) : `tom-connect` (~15K LOC, MagicSock + hole punching), `tom-relay` (~8K), `tom-gossip` (~5K), `tom-quinn` (6,5K), `tom-quinn-proto` (41K), `tom-base`, `tom-metrics`. Environ **75 000 lignes assimilées**, sur les ~150 000 du workspace.

Ce n'est pas un fork décoratif : les identifiants protocolaires sont désormais souverains et **volontairement incompatibles** avec le réseau iroh public —

| Invariant wire | Valeur |
|---|---|
| Préfixe DNS | `_tom` |
| SNI TLS | `.tom.invalid` |
| En-têtes relay | `X-Tom-NodeId`, `X-Tom-Challenge`, `X-Tom-Response` |
| ALPN transport | `tom-protocol/transport/0` |
| ALPN gossip | `/tom-gossip/1` |

iroh est le point de départ historique, pas une dépendance réseau. Le protocole n'a plus de maître amont.

---

## 4. Le cœur technique, en quatre extraits de code réels

### 4.1 Le routeur est un moteur de décision *pur* — et la décision #1 est dans le type

Le Router ne fait pas d'I/O : il retourne une `RoutingAction`, le runtime exécute. Extrait de `crates/tom-protocol/src/router.rs` (variante `ReadReceipt` coupée pour la lecture) :

```rust
/// What to do with an incoming envelope.
#[derive(Debug)]
pub enum RoutingAction {
    /// Regular message for us — deliver to application.
    /// `response` is an unsigned delivery ACK to send back to the sender.
    Deliver { envelope: Envelope, response: Envelope },
    /// ACK for us — update message status tracker.
    Ack { original_message_id: String, ack_type: AckType, from: NodeId },
    /// Forward to next hop. `relay_ack` is an unsigned ACK for the original sender.
    Forward { envelope: Envelope, next_hop: NodeId, relay_ack: Envelope },
    /// Rejected (TTL exhausted, chain too deep, malformed, etc.)
    Reject { reason: String },
    /// Duplicate of an already-delivered message: do NOT re-deliver locally, but
    /// RE-SEND the (unsigned) delivery ACK. Without this, a lost ACK would make
    /// the sender retry forever — the recipient already has the message but never
    /// re-confirms (breaks LOCKED decision #1: delivered ⟺ ACK).
    ReAck { response: Envelope },
    /// Duplicate or expired — silently ignore.
    Drop,
}
```

La variante `ReAck` est un bon exemple de ce que « décision LOCKED » veut dire en pratique : le cas pathologique (ACK perdu → réémission → doublon) est traité **dans le type**, avec la décision fondatrice citée en commentaire. Et l'ACK n'est pas une politesse : il est **signé à l'émission et vérifié à la réception** — un ACK forgé est rejeté, seules les opérations à l'intérieur de ce gate accordent la confiance :

```rust
let mut ack = response;
ack.sign(&self.secret_seed);
effects.push(RuntimeEffect::SendEnvelope(ack));
```

### 4.2 Zéro-config : le rendez-vous DHT partagé, sans nœud privilégié

Le problème classique du P2P : comment deux inconnus se trouvent-ils **sans bootstrap server** ? Réponse ToM (ADR-010) : une constante publique (`tom-protocol-rendezvous-v1`) dérive 8 keypairs Ed25519 « slots » partagés par tout le monde. Chaque nœud publie `{node_id, addrs}` dans le slot `hash(node_id) % 8` de la DHT Mainline (BEP-0044 mutable) ; n'importe qui lit les 8 slots et découvre les pairs vivants **avec zéro connaissance préalable**. Aucun nœud privilégié, aucune infra à payer.

Un rendez-vous public est squattable par construction — donc chaque entrée découverte doit **prouver la possession** de son identité. `crates/tom-protocol/src/runtime/loop.rs:1166`, verbatim :

```rust
fn rendezvous_entry_authentic(addr: &tom_dht::DhtNodeAddr) -> bool {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let Ok(node_id) = addr.node_id.parse::<NodeId>() else {
        return false;
    };
    let Ok(vk) = VerifyingKey::from_bytes(&node_id.as_bytes()) else {
        return false;
    };
    if addr.sig.len() != 64 {
        return false;
    }
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&addr.sig);
    vk.verify(&addr.signing_bytes(), &Signature::from_bytes(&sig_bytes))
        .is_ok()
}
```

Un attaquant peut écrire dans le slot ; il ne peut pas y mettre un pair qu'il ne possède pas.

### 4.3 Le détecteur de zombies : « connecté » ne veut rien dire, « vivant » se mesure

Découverte de terrain : QUIC peut maintenir une connexion *ouverte* dont l'autre bout est mort (veille, cellulaire, suspension iOS). Le runtime ne fait donc plus confiance à `connected_peers()` seul — il exige du trafic entrant récent :

```rust
/// Silence window after which "connected" peers are treated as zombies.
/// Healthy peers emit gossip announces (~10s) + heartbeats, so 45s of total
/// inbound silence means the links are dead even if QUIC still reports them.
const LIVENESS_STALE_MS: u64 = 45_000;

/// Whether inbound traffic has gone stale (zombie-connection detector).
/// Saturating so a `last_inbound` slightly in the future never underflows to stale.
fn liveness_is_stale(last_inbound: u64, now: u64, threshold_ms: u64) -> bool {
    now.saturating_sub(last_inbound) > threshold_ms
}
```

```rust
let zombie = !connected.is_empty()
    && liveness_is_stale(last_inbound_at, now_ms(), LIVENESS_STALE_MS);
if connected.is_empty() || zombie {
    // → retour en phase d'amorçage : reprobe relais, republish DHT,
    //   rendez-vous, rejoin gossip. Le nœud se ré-amorce seul.
}
```

C'est ce mécanisme qui donne l'auto-guérison mesurée en campagne : un nœud arrêté brutalement est détecté, contourné, et ré-intégré en secondes à son retour.

### 4.4 Une décision LOCKED, c'est une ligne qui rend la violation impossible

Le backup « virus » (ADR-009) réplique les messages des destinataires hors-ligne sur des nœuds de garde. La décision #2 (24 h max) n'est pas un commentaire ni une review — c'est un clamp **à la construction** de l'entrée, `crates/tom-protocol/src/backup/types.rs` :

```rust
pub const MAX_TTL_MS: u64 = 24 * 60 * 60 * 1000;
// …dans le constructeur :
let ttl = ttl_ms.unwrap_or(DEFAULT_TTL_MS).min(MAX_TTL_MS);
```

Le stockage SQLite **ne peut pas** contenir une entrée qui dépasse 24 h, quel que soit l'appelant. Purge câblée dans le tick du coordinateur. L'ADR-009 a ensuite été verrouillé par endurance réelle (15/15 en livraison différée) et par tests déterministes.

---

## 5. Les batailles — ce que le réseau réel a appris au code

C'est la partie qu'aucun benchmark de labo ne raconte. Tous ces bugs ne vivaient **qu'en réel**, sur de vrais appareils, de vrais réseaux, de vraies mises en veille.

**Le verrou-otage du pool (17/07).** `get_or_connect` dialait — 10 à 20 secondes de timeout réseau — *en tenant le mutex du pool de connexions*, et le poll de statut passait dans la même boucle. Record mesuré : **74 secondes de boucle tenue**. Fix : dial hors verrou + statut répondu hors boucle, et surtout un legs durable : le profileur `timed()` qui **nomme tout await inline > 300 ms** dans la boucle du runtime. Règle gravée : jamais d'await réseau sous un mutex partagé.

**Le gel transport iOS (17/07).** Un iPad prouvé gelé 5 min 21 s par un wedge tokio dans la couche transport — le timer FFI lui-même était mort, donc aucun timeout applicatif ne pouvait sauver. Cure : port de deux fixes upstream d'iroh (#4314, ref_count atomique), budgets FFI bornés, et au passage une régression de surdité UDP attrapée et hotfixée le jour même. Leçon : quand le runtime async est wedgé, la supervision doit vivre *en dessous* de lui.

**La topologie empoisonnée (17/07).** 1286 pairs persistés dans l'état local, des fantômes ravivés par gossip → des ticks de boucle à 84–218 secondes. Diagnostic tranché par un **canari A/B** (deux nœuds identiques, un seul purgé). La réponse de fond est arrivée cette semaine : anti-ravivage avec **TTL 24 h strict** sur les identités disparues, validé design-first (build 112) — cohérent avec la décision #2 : le réseau ne garde que le présent, y compris dans sa mémoire topologique.

**Le mensonge d'affichage « 100 % RELAY » (16/07).** Déjà raconté en §2 — le singleton `last_path` écrasé par n'importe quel pair. La leçon a été promue règle de projet : **l'observabilité doit refléter la vérité terrain**, par pair, par chemin (`paths_by_peer` exposé en HTTP sur chaque nœud), jamais un agrégat qui peut mentir.

**Le décodage silencieux (17/07).** Une régression Swift : un champ non-Optional ajouté à un `Codable` sans `init(from:)` explicite → **toute la réception jetée en silence** côté app pendant que le transport livrait parfaitement. Fix + test de contrat sur le JSON wire exact. Leçon : une frontière FFI mérite des tests de contrat, pas de la confiance.

**Le flake CI 1/256 (18/07).** Un test de falsification crypto forçait le dernier octet à `0xff`… no-op si l'octet valait déjà `0xff` — le message « falsifié » restait intact et déchiffrait, test rouge une fois sur 256. Fix : un flip garanti différent. Un flake probabiliste ne se reproduit pas en 23 runs ; il se **calcule**.

**L'orage de timers (16/07).** 18 `tokio::interval` en mode Burst : après un stall (résume iOS, appareil faible), tous les ticks manqués rejouaient d'un coup — tempête de republish/rejoin au pire moment. Fix : `MissedTickBehavior::Skip` partout — un tick de rattrapage, pas le backlog.

La méta-leçon de ces batailles tient en une phrase, déjà payée deux fois : **les indicateurs verts sur des proxies laissent des régressions survivre des semaines**. C'est elle qui a engendré le chantier suivant.

---

## 6. Le hardtest : d'un banc de chronos à un banc d'invariants

Constat du 18/07 : « nos campagnes valident l'envoi/réception plus quelques chronos — c'est trop peu ». Un test qui passe parce que « ça avait l'air d'aller » ne prouve rien. Le renversement, écrit dans la conception du banc chaos (commit `682887d`, build 115) :

> Sous cette faute précise, est-ce que les **GARANTIES** du protocole tiennent encore ? Un scénario qui livre 100 % des messages mais laisse une connexion zombie, ou gèle une boucle 45 s, ou double une livraison : **ÉCHOUE**.

### Les 8 invariants durs

| # | Invariant | Seuil |
|---|---|---|
| I1 | Zéro perte finale (livré OU explicitement expiré TTL) | 0 perte silencieuse |
| I2 | ACK ⟺ livraison à l'app (pas d'ACK fantôme) | 0 |
| I3 | Reconvergence bornée après perte de pair | < 15 s (shadow hub ~8 s) |
| I4 | Zéro connexion zombie | jamais > 45 s |
| I5 | Zéro gel de boucle (ticks mesurés par `timed()`) | jamais > 2× la période |
| I6 | Zéro crash non compté | 0 surprise |
| I7 | Mémoire bornée en endurance | pas de croissance monotone |
| I8 | Zéro double-livraison (rejeu ⇒ idempotence) | strict |

Et **21 scénarios gradués** en 6 niveaux — du sanity unicast jusqu'à « #20 cascade combinatoire » (kill d'un pair + message lourd en vol + handover cellulaire d'un autre + clock-skew d'un troisième, *simultanés*) et « #21 endurance 6→24 h sous chaos ». Le scénario-signature, #18 : A fait un reset usine, saisit le node_id d'un pair **jamais rencontré et hors ligne**, écrit — le message doit arriver quand même, via backup et découverte, sans qu'ils se soient jamais parlé.

### Étage L : livré (build 116, aujourd'hui)

Le premier étage tourne : N nœuds in-process, chaos-monkey **seedé donc reproductible** (KILL/REVIVE/SKEW), et les invariants I1/I2/I3/I5 assertés avec un tracking réel des livraisons. Un détail de conception qui montre le niveau d'exigence — le tracker de réception compte, il n'ensemble pas :

```rust
/// node_idx → (payload → number of times that exact payload was delivered).
/// A COUNT, not a set: a set would silently dedup double-deliveries and make
/// I8 impossible to observe (the exact trap the first two rounds fell into).
type ReceivedMap = HashMap<usize, HashMap<Vec<u8>, u32>>;
```

(Deux premières itérations du banc, écrites par sous-agents, étaient tombées exactement dans ce piège : des assertions creuses qui ne pouvaient pas observer une double-livraison. Reprises à la main. Vérifier le travail des agents reste une discipline, pas une option.)

### Étage F : la flotte réelle, avec l'outillage pointu

Le deuxième étage pilote les **vrais appareils** — là où vivaient le verrou-otage, le wedge transport et la surdité UDP. L'arsenal, déjà en place ou en construction :

- **Injection de fautes OS** : `pkill`/SIGKILL, `systemctl stop` (NAS), `devicectl process terminate` (iOS/tvOS), `pmset sleepnow` (veille Mac réelle).
- **Réseau adverse** : Network Link Conditioner d'Apple sur iPhone/iPad (profils 3G/LTE dégradé/100 % perte), `pfctl` + `dnctl` côté Mac/NAS (partition réelle, +200 ms, perte).
- **API de contrôle par nœud** : status HTTP (`:9091` apps, `:8085` NAS), control `:9300` (`/send?size=`, `/sendall`, `/stop`, `/metrics`), API `/reset` en chantier (« sortir du labo »).
- **Télémétrie de vérité terrain** : collecteur UDP broadcast (`:9999`) horodaté à la réception sur un référentiel unique, événements `path_change` par pair, traqueurs d'actions bouton (qui ont permis d'invalider deux « findings » — cf. §8).
- **Property-based testing** : proptest sur le QUIC multipath a exhibé un **vrai bug protocolaire** — un `PATH_ABANDON` non réciproqué tuait la connexion — corrigé en portant le patch upstream #436 (build 105).

Le chantier ouvert et assumé de ce banc : l'**herméticité** — un nœud de test ne doit *jamais* voir la flotte de production pendant `cargo test`. C'est listé dans les murs de la feuille de route, pas caché sous le tapis.

---

## 7. L'arrivée de Fable 5 : le juge adverse à un million de tokens

Début juillet, l'atelier a intégré **Claude Fable 5** — premier modèle de la famille Claude 5 d'Anthropic, d'une classe au-dessus d'Opus. Son apport au projet n'est pas « écrire du code plus vite ». C'est un changement de *méthode* : le modèle qui développe est aussi celui qu'on retourne **contre** le projet.

Le cas d'école : la revue « démolis-le » de l'ADR Proof of Presence, menée à 1M de tokens de contexte — assez pour tenir *tout le protocole* en tête d'un coup, code et docs. Verdict du 11/07 : « PoP survit comme direction, pas comme état actuel », avec **6 kill-shots ancrés file:line** — pas des opinions, des attaques : heartbeat déclaratif non gaté sur signature (une enveloppe forgée reconstruisait des pairs-fantômes), coût d'identité « KNOWN » quasi-nul amorti en 14 h, témoin unique éclipsable, budget par-identité sans cap global agrégé…

Quarante-huit heures plus tard : **6/6 fermés**, vérifiés commit par commit — présence gatée sur `signature_valid`, séparation `Known` (adresse découverte) / `Online` (travail soutenu prouvé), quorum de témoins distincts (promotion seulement si ≥ N témoins concordent, dédup par témoin, TTL 30 s), cap global symétrique. Détail savoureux et instructif : deux des six kill-shots étaient *déjà* fermés par du code existant que la doc n'avait pas rattrapé — l'audit a aussi débusqué la dérive documentaire, et des tests ont été ajoutés pour figer ce que personne n'avait figé.

Ce pattern est désormais institutionnalisé dans la vision consolidée : des **checkpoints adversariaux périodiques** dont le seul but est de démonter l'état présent. La revue Fable du 06/07 avait déjà réordonné le plan global ; la boucle red-team de la même semaine avait produit 6 brèches et identifié une *classe* récurrente de vulnérabilité (« budget par-identité sans cap global ») réutilisée depuis comme grille d'audit. C'est ça, le dev LLM-first version 2026 : pas un autocomplete — un partenaire qui code la journée, red-team le soir, et écrit ce rapport.

Et ce n'est pas un hasard si ça marche sur *ce* projet : ToM est conçu depuis le début pour être **LLM-first** aussi côté adoption (docs structurées pour agents, SDK, MCP). Un protocole qu'un agent peut comprendre en entier est un protocole qu'un agent peut auditer en entier — et déployer chez un utilisateur en trois messages.

---

## 8. Le jalon qui compte : sortir du labo

Le 17/07 au soir, le jalon **produit** : deux iPhones réels, sur réseaux cellulaires, qui conversent via ToM — dont un en LTE monté en **DIRECT IPv6 cellulaire**, sans serveur, sans compte, sans configuration. La première conversation qui ne doit rien au labo.

Le 18/07 au matin, la campagne totale instrumentée sur les 6 nœuds : **1809 messages livrés, 26 échecs transitoires tous rattrapés, 0 perte finale, 0 crash, 0 gel de boucle**. SIGKILL de l'Apple TV en plein trafic : re-mesh en 8 s. Coupure data d'un iPhone : continuité sur 3 pairs sur 4, backup rejoué au retour.

Et peut-être le plus révélateur : l'après-midi, une **contre-enquête** a invalidé 2 des 3 « findings » de la campagne. Le « gel de 45 s » et la « cascade de veille » ? Quatre arrêts *manuels* au bouton, prouvés par les traqueurs d'UI et `pmset -g log`. Le protocole s'était auto-guéri en moins de 10 s à chaque fois. Un projet qui instruit à charge **et à décharge** contre ses propres résultats — c'est cette discipline-là qui rend les 1809/0 crédibles.

---

## 9. Ce qui reste dur — les murs, nommés

Rien sous le tapis. Les murs actuels, tels que consignés :

- **M1.2 — entropie non-biaisable** (L1) : LE verrou de recherche. Sans source d'aléa que personne ne peut griller, la sélection de quorums est attaquable. Budget d'étude borné (VDF, beacons, signatures-seuil), revue externe par un cryptographe prévue — c'est le seul point où l'auto-évaluation ne suffit structurellement pas.
- **M1.4 — anti-Sybil quantifié** : donner un *chiffre* à P(quorum Sybil), avec un modèle qui inclut l'attaquant patient qui attend 6 mois.
- **Échantillon de 1** : toute la validation tourne sur une box, un FAI, un foyer. Avant distribution : 3-5 foyers testeurs, autres FAI, autres pays.
- **Bus factor = 1** : le piège n°1, devant tous les murs techniques. Mitigations : specs normatives, mémoire d'agent, docs LLM-first comme assurance-vie du projet.
- **Herméticité des tests réseau** ; **stabilité DIRECT > 1 h** (mesurable depuis #33, à mesurer) ; **SPOF de la porte publique** (le test « iPhone data ↔ maison sans le NAS » reste à passer).
- **iOS en arrière-plan** : non-objectif *assumé* (décision produit du 15/07). Pas de daemon, pas de push APNs — premier plan = contribution pleine, arrière-plan = sursis de quelques minutes, filet = backup 24 h. C'est un choix de vision, pas un trou.

---

## 10. La vision : crédible ? Le terrain dit mieux que ça

Reprenons la promesse du whitepaper : *« un nouveau protocole pour un internet qui n'appartient à personne »*. En février c'était une thèse. En juillet :

- **Zéro-config atteint** : deux inconnus se trouvent par le rendez-vous DHT partagé, sans nœud privilégié, avec preuve de possession. Le serveur de signaling est mort et enterré.
- **Souveraineté du fil atteinte** : plus un octet du chemin critique qui dépende d'un tiers ; les derniers hostnames externes sont optionnels et en sursis.
- **Résilience mesurée, pas promise** : auto-guérison en secondes sous SIGKILL, zombies détectés en 45 s, zéro perte sur 1809 messages avec chaos humain réel dedans.
- **La couche L1 a démarré en avance sur son propre plan** — et a déjà survécu à un red-team à 1M de tokens.

La vision de base n'est plus « potentiellement crédible ». Sur l'étage 0, **elle est en retard sur son propre terrain** — le réseau faisait déjà du DIRECT IPv6 global pendant qu'on le croyait coincé en relay.

La suite est écrite en trois étages, avec leurs conditions :

**Étage 0 — le tuyau irréprochable** (maintenant → fin 2026). DIRECT stable des jours entiers, porte publique automatique sur toute box (`tom-gateway`), rejoin < 2 s pour les pairs connus, un binaire qui s'installe **sans terminal** : image Raspberry Pi, Docker, VM Freebox, one-liner. Un protocole d'infrastructure n'a droit qu'à une première impression.

**Étage 1 — le réseau qui se prouve lui-même** (2027). Le Proof of Presence transforme la présence en intégrité : quorums imprévisibles, validation croisée, ancrage du présent **sans ledger**. Utile même sans argent — une couche d'immunité (anti-Sybil, anti-eclipse) qui rend le réseau digne de confiance pour des usages tiers. La sécurité par la masse × l'imprévisibilité, pas par le capital ni le calcul.

**Étage 2 — la valeur comme preuve ultime** (2028+, conditionnel). Le portefeuille scellé auto-custodié n'est pas le but : c'est le **test de vérité** du PoP. Si un réseau de présence pure peut empêcher une double dépense sans grand livre ni frais, la thèse fondatrice est démontrée. Et si M1.2/M1.4 ne tiennent pas, L2 sera abandonné sans honte — la messagerie souveraine et le swarm d'intégrité justifient le projet à eux seuls.

Et au-dessus des étages, l'horizon qui donne son sens à chaque build : **la disparition comme succès**. Le nœud livré par défaut — dans la box, l'OS, le terminal de paiement. Le réseau qui finit par héberger son propre code (si GitHub tombe, le code vit dans le swarm). ToM comme primitive système que les apps appellent sans le savoir, comme une socket. L'incitation par l'accès, jamais par le jeton. *Un protocole a réussi quand plus personne ne prononce son nom.*

Il y a une raison plus profonde de tenir cette ligne, et elle est dans la genèse du projet : les trois péages — la surveillance, la rente, le point central — **reviennent toujours**. L'email fut décentralisé ; puis vint Gmail. Un protocole souverain n'est jamais « terminé » : il est *maintenu libre*, activement, contre une gravité constante. Et quelque part il y a une Leila qui ne saura jamais qu'elle utilise ToM, et un Reza pour qui ce réseau sera la différence entre parler et se taire. Chaque heure investie dans l'ennuyeuse fiabilité du tuyau est pour quelqu'un qu'on ne rencontrera jamais.

---

## Épilogue

Le 2 février, ce repo s'ouvrait sur un scaffold vide. Le 18 juillet, un message part d'un iPhone en LTE, se chiffre, trouve son destinataire sans qu'aucun serveur au monde sache qu'il existe, traverse en direct IPv6, revient acquitté et signé — et si le destinataire dort, le réseau le porte 24 heures puis l'oublie, exactement comme promis, parce qu'une ligne de code rend l'inverse impossible.

765 commits. 117 builds. Zéro serveur. La suite : rendre tout ça ennuyeux de fiabilité — puis invisible.

---

*Sources : `git log` (765 commits, 02/02→18/07), `docs/plans/FEUILLE-DE-ROUTE-2026-07-16.md`, `docs/plans/VISION-CONSOLIDEE-2026-07-16.md`, `docs/plans/banc-test-chaos.md`, campagne du 18/07 (rapport instrumenté), code cité verbatim aux emplacements indiqués. Rédigé avec Claude Fable 5.*
