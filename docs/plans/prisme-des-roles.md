# Le prisme des rôles — synthèse de relecture intégrale des notes

> Rédigé le 2026-07-20 (soir), après le recadrage fondateur de Malik (verbatim dans
> `PROMPT-REPRISE-ROLES.md` §0). Relecture faite via qmd (index `tom`, collections
> vault + docs + bmad, 187 fichiers indexés + embeddings).
> **But** : que L1, subnets, relais, backup, observateurs, validation et carnet de
> rendez-vous tournant transpirent dans toute analyse ET tout scénario de test.
> Ce doc est la fondation du banc « rôles sous charge » (design à part).

## 0. La thèse en une phrase

Le réseau n'est pas un tuyau send/ACK : c'est un **organisme à rôles tournants**.
La livraison de message est UNE fonction. Le but primaire est que **chaque appareil
porte une part du travail commun** — relayer, garder, observer, valider, mettre en
lien — assignée par le réseau selon contribution et besoin, jamais choisie
(ADR-006, LOCKED #6). Un test qui ne mesure que send/ACK est aveugle à 90 % de
l'organisme (mandat Malik).

## 1. La grille des rôles — vision → implémenté → écart

| Rôle (vision) | Source fondatrice | Ce qui EXISTE dans le code | Écart restant |
|---|---|---|---|
| **Client/Participant** | wp §3.4 ; Plan Maître V2 #1 | `PeerRole::Peer` (`relay.rs:43`) | — |
| **Relais** (le facteur, multi-sauts, pass-through) | wp §3.4 ; LOCKED (stateless) | `PeerRole::Relay`, promotion par score (`roles/manager.rs`), decay 5 %/h (`roles/scoring.rs:8`), relais embarqué + sélection (`relay/`) | Rotation « pseudo-aléatoire au dernier moment » (wp §3.4/§4.3) absente : la promotion est par seuil de score, pas par cascade imprévisible |
| **Backup / Gardien de messages** (virus : réplique, restitue, s'auto-détruit) | ADR-009 ; wp §2.3 ; Plan Maître V2 #3 | module `backup/` complet : survival_score, réplication vers meilleur hôte, purge ACK/24 h, budget 64 Mio | — (validé en endurance 15/15) |
| **Responsable / Remplaçant / Suivant / Admin de groupe** | Plan Maître V2 #4-7 | `GroupHub`, failover Primary→Shadow→Candidate ~6 s, sender keys | — |
| **Carnet de rendez-vous** (mettre deux inconnus en lien, comme DNS **mais le rôle TOURNE**) | Malik verbatim ; ADR-010 | 8 slots DHT **statiques** dérivés d'une constante partagée (`tom-dht`) : *tout le monde* détient le carnet | **La rotation n'existe pas** : slots fixes, pas de détenteurs désignés tournants. Aucun doc de conception dédié — chantier à ouvrir (lié au résiduel « 8 slots » des Known Limitations) |
| **Observateur** (surveille wallet/subnet, co-signe les transitions, règle d'or anti-double-dépense) | wp §3.6/§6.7 ; Master Map §4 | **Primitifs seulement** : attestation de présence L1-001 + vue signée quorum L1-003 (`presence/{attestation,aggregator,quorum}.rs`, promotion quorum-attestée `state.rs`) | Le rôle plein (co-signature de transitions d'état) = 🔨 M1.5/M1.6 |
| **Gardien** (aide les nouveaux à se synchroniser + atteste) | wp §3.4 ; Master Map §4 | Fonction « pompe d'amorçage » diffuse : PeerPresent, mDNS, DHT (Plan Maître V2 : *une fonction, pas un rôle*) | Le rôle attestant (validation croisée) = 🔨 |
| **Validateur** (quorums PoP, sélection cascade) | wp §4 ; ADR-011 | ACK signé obligatoire + PoP « présence = travail constaté » (Online vs Known) — le socle | Cascade + entropie non-biaisable + quorum = 🔨 M1.2-M1.3 (mur #1 Fable : grinding) |
| **L1 (Anchor)** (BUS organique : ancre l'état présent, genèse glissante, **n'arbitre JAMAIS**) | wp §3.2 ; LOCKED #3 | Rien de spécifique (la « vue présente » quorum-attestée en est l'embryon) | 🔨 M1.6 |

**Subnets éphémères** (pas un rôle, un TERRITOIRE de rôles) : wp §3.3 — création à la
volée, auto-purge, auto-régulation, fork contrôlé = « mécanisme de respiration ».
Implémenté : `EphemeralSubnetManager` (`discovery/subnet.rs`) — clustering BFS sur
graphe de communication pondéré, arêtes ≥ 3 messages, taille 3-10, dissolution à
5 min d'inactivité, décroissance des arêtes, évaluation 30 s. **Jamais exercé par un
banc à ce jour.**

## 2. Les mécanismes transverses (le sang de l'organisme)

- **PoP — ADR-011 (le pivot)** : la présence est un fait dérivé d'un travail
  **constaté et vérifiable** (ACK signé, relais utile, backup *restitué*, bootstrap
  qui a mené à une connexion) — jamais une déclaration. Un seul signal unifie
  présence, rôle, réputation, anti-Sybil : « avoir mis sa pierre ». Fondu 30-60 s de
  demi-vie ; `Known` (carnet d'adresses, pour router) ≠ `Online` (PoP, pour compter).
- **Économie de l'équilibre** (wp §5) : score = contribution − usage, l'objectif est
  **zéro**, pas l'accumulation. Proportionnel aux moyens de l'appareil. La
  contribution n'est pas du volontariat : c'est la **condition d'usage**,
  réseau-imposée (charte §1).
- **Arroseur arrosé** (wp §6.5, LOCKED #5) : abus → micro-PoW + sur-assignation de
  tâches, jamais d'exclusion. Implémenté : `roles/antispam.rs` (budgets progressifs).
- **Réputation à fade** (LOCKED #4) : gradient, jamais de ban, rédemption toujours
  possible. Implémenté : decay 5 %/h du score de contribution.
- **Validation croisée** (wp §4.5) : « les validateurs proposent, les observateurs
  vérifient, les relais transmettent, les gardiens attestent — chaque rôle surveille
  les autres ». 🔨 M1.5.
- **Migration de rôle** (Master Map §4) : quand un nœud part/surcharge, **la fonction
  se recopie, pas seulement les données** — un porteur de backup transmet le message
  ET le rôle. Implémenté pour backup (réplication) et hub (failover) ; pas de
  mécanisme générique.

## 3. Conséquence : la grille des scénarios « rôles sous charge »

Tout futur banc doit couvrir ces 8 axes (les 6 premiers sont exerçables **dès
aujourd'hui** avec le code livré ; 7-8 arrivent avec M1.x) :

| # | Axe | Ce qu'on prouve | Mécanisme code |
|---|---|---|---|
| R1 | **Relais multi-hop** | un message A→B transite par R (pas de chemin direct), R ne stocke rien | `RoutingAction::Forward`, topologie contrainte |
| R2 | **Backup absent→retour** | destinataire offline → réplication virus → retour → livraison → auto-purge ≤ 24 h | `backup/`, ADR-009 |
| R3 | **Promotion/rétrogradation** | sous contribution le rôle monte (Peer→Relay), à l'arrêt il fade (5 %/h), jamais de ban | `RoleManager`, `ContributionMetrics` |
| R4 | **Rendez-vous** | deux inconnus (zéro connaissance préalable) se trouvent via les slots DHT | `tom-dht`, ADR-010 |
| R5 | **Subnets** | un pattern de trafic dense forme un subnet ; l'inactivité le dissout | `EphemeralSubnetManager` |
| R6 | **Failover de groupe** | mort du Responsable → Remplaçant promu ~6 s, zéro perte | `GroupHub` watchdog |
| R7 | **PoP sous charge** | fantômes jamais comptés vivants ; Online = travail constaté seulement | `presence/`, L1-001/003 |
| R8 | **Arroseur arrosé** | un spammeur s'épuise (budgets progressifs), les autres ne voient rien | `AntiSpam` |

**Règle d'or des bancs** (incident 20/07) : tout nœud de test coupe le rendez-vous
partagé (`n0_discovery(false)` + `local_discovery(false)` + `enable_dht:false`) —
SAUF le scénario R4 qui doit alors utiliser un **namespace de rendez-vous dédié au
test**, jamais celui de la flotte réelle.

## 4. Écarts vision↔code à garder en tête (chantiers, pas des bugs)

1. **Rotation imprévisible des rôles** (cascade + entropie, murs #1/#2 Fable) —
   M1.2/M1.3, problème de recherche ouvert.
2. **Carnet de rendez-vous tournant** — aucun design ; aujourd'hui slots statiques.
3. **Observateur/Validateur/L1 anchor pleins** — M1.5/M1.6.
4. **Migration de rôle générique** (la fonction se recopie) — seuls backup et hub
   la font.
5. **Auto-déclaration de capacités** (L1-003 §« vision long terme ») — le réseau
   route la charge selon les moyens déclarés+constatés.

## 5. Sources relues (traçabilité)

Intégral ou quasi : `docs/tom-whitepaper-v1.md` (§3-§9), `docs/plans/tom-master-map.md`
(v2 entier), `docs/plans/POP-PROOF-OF-PRESENCE.md` (ADR-011, décision + 5 réponses),
charte §1, MISSION §1, design-decisions D3/D4/D5, Plan Maître Réseau Vivant V2
(rôles + logs). Structure + extraits : TOM-PLAN-GLOBAL (M0-M2), L1-001 (758 l.,
sommaire + frontières), L1-003 (grep ciblés), anti-ravivage, r-name-via-dht.
Code vérifié : `relay.rs:43`, `roles/{manager,scoring,antispam}.rs`,
`discovery/subnet.rs`, `presence/*`, `backup/`. Recherches transverses (BM25 +
vecteur) : observateur, rendez-vous, rotation, L1-003, PoP — la rotation du carnet
n'apparaît QUE dans le verbatim de Malik → écart #2 confirmé comme non-documenté.
