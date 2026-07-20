# ToM — Charte des cibles agressives (North Star)

> « La masse comme carburant, pas comme charge. »
> Créée le 2026-07-20. Document VIVANT — chaque build se mesure à lui.
> Statut : v1 — thèse recadrée après revue « avocat du diable » (20/07) : deux régimes
> (capacité vs latence), unité nœud-équivalent, gap bootstrap Mainline acté. Cibles
> chiffrées à ratifier avec Malik.

---

## §0 La thèse fondatrice — l'inversion anti-cyclique

L'industrie des réseaux vit un **problème cyclique** : la demande croît → l'infra
centralisée sature → il faut ajouter des serveurs, des CDN, des régions → le coût par
utilisateur monte ou stagne, jamais il ne s'effondre → et à chaque palier de croissance,
le mur de scaling se rejoue. Plus d'utilisateurs = plus de charge = plus de coût. La masse
est un **problème** qu'on gère.

**ToM inverse le signe.** La propriété qu'on vise — et qui doit devenir mesurable, pas
métaphorique — est l'**anti-cyclicité** :

> Chaque nœud qui rejoint le réseau doit le rendre PLUS rapide, PLUS résilient, PLUS
> in-censurable, et MOINS cher par tête. La masse n'est pas la charge : c'est le carburant.

### Le mécanisme : le réseau s'héberge lui-même

Cette inversion n'est pas magique — elle a une cause structurelle unique : **le réseau EST
sa propre infrastructure.** Il n'existe aucune couche d'hébergement séparée (serveurs, CDN,
data-centers) à dimensionner à côté des utilisateurs. Chaque device est simultanément
client, relais, nœud de stockage (backup), et point de découverte (rendez-vous DHT). Donc :

> Ajouter un utilisateur, c'est ajouter de l'infrastructure — pas seulement de la charge.
> La demande et l'offre de capacité arrivent **dans le même paquet** : le nouveau nœud.

Précision d'honnêteté (revue 20/07) : l'infra apportée est **proportionnelle aux capacités
du nœud** (whitepaper §5 : la contribution attendue l'est aussi) — un NAS always-on apporte
des ordres de grandeur de plus qu'un mobile en power-save. L'unité de compte de la thèse
est donc le **nœud-équivalent** (pondéré par capacité), pas la tête brute ; et la santé de
l'organisme se surveille par la **concentration** (part de l'infra portée par les plus
gros nœuds, §4) — si l'essentiel vient d'une poignée de gros nœuds, on a recentralisé de
facto sans se l'avouer.

C'est pour ça que le coût centralisé croît avec la demande (il faut acheter l'infra en
face) alors que le coût ToM par tête **s'effondre** (l'infra arrive avec la tête). Le
réseau auto-hébergé est la raison d'être de tout le reste de cette charte : chaque cible
agressive n'est atteignable QUE si l'auto-hébergement reste vrai à toute échelle — d'où le
test anti-cyclique appliqué partout. Si un jour une fonction exige une infra externe
dédiée (un serveur privilégié, un service central), on a rompu le mécanisme, pas juste
ajouté une dépendance.

Ce n'est pas un slogan. C'est un critère de conception falsifiable — à condition de le
mesurer honnêtement. Recadrage (revue adverse 20/07) : l'anti-cyclicité a **deux régimes**
à ne pas confondre :

- **Capacité, résilience, anti-censure** : super-additifs par construction (plus de nœuds
  = plus de débit servable, plus de copies, plus de portes). Là, la courbe doit MONTER
  avec N.
- **Latence** : une connexion 1-à-1 profite de la densité (plus de candidats proches),
  mais une propagation broadcast (gossip) coûte au mieux ~log N — elle ne s'améliore PAS
  avec la masse. L'exigence honnête : **bornée, au pire sous-logarithmique**, jamais
  linéaire.

Et la métrique du juge est TOUJOURS **par nœud, à charge par nœud fixe** — « le débit
agrégé monte » est tautologique (vrai de tout système à plus de tuyaux) et ne falsifie
rien. Le juge de chaque décision devient : **« est-ce que ça se comporte mieux — par
nœud — à 10 000 nœuds qu'à 5 ? »** Si la réponse n'est pas structurellement « oui » (ou
« borné sous-log » pour le broadcast), on n'a pas fini.

C'est ce qui casse les codes : là où le centralisé paie la croissance, ToM la mange.

### La thèse n'est pas une foi — elle a une preuve et des conditions (sourcé)

**La preuve de terrain que l'anti-cyclicité EXISTE : BitTorrent.** Au-delà de ~100
seeders, chaque pair supplémentaire n'ajoute **aucun coût** au système — il n'augmente que
le débit agrégé ; un torrent bien seedé sature la connexion de *chaque* client
indépendamment ([wiki][bt1], [clustering arXiv][bt2]). Côté coût, le CDN pur croît
linéairement avec l'audience quand l'hybride P2P **décharge 60-78 %** du trafic
(données streaming 2026, [Quanteec][cdn]). L'inversion est donc réelle, mesurée, pas
idéologique.

**Mais elle est CONDITIONNELLE — et les conditions dessinent notre architecture :**

| La thèse TIENT si… | …sinon elle S'INVERSE (la masse dégrade) | Statut ToM |
|---|---|---|
| Topologie **sous-linéaire** (gossip/DHT), jamais full-mesh | Full-mesh s'effondre à **50-100 pairs** (O(n²), [Cellular Mesh][mesh]) | ✅ mesh par hubs, pas full-mesh (LOCKED #10) |
| Churn **toléré par design** (réplication) | Churn > capacité de stabilisation écrase même Kademlia tuné | ✅ backup TTL 24h + rejoin ; à chiffrer |
| Contribution **évaluée** (tous les pairs ≠ égaux) | Masse de parasites = charge sans capacité | ✅ équilibre usage/contribution géré par le protocole (whitepaper §5, §6.5) — pas du volontariat ; à surveiller : concentration (§4) |
| Payload **authentifiée** | « plus de masse » devient « plus d'attaquants » (squatting) | ⚠️ signé, mais 8 slots DHT = angle mort connu |

La conclusion de la recherche est nette : *« plus de masse = plus fort » est vrai SOUS
architecture sous-linéaire + churn absorbé + contribution pesée + payload signée ; faux
sinon.* ToM a fait ces quatre paris dans ses décisions LOCKED. **La charte existe pour
transformer ces paris en courbes mesurées.**

---

## §1 L'organisme vivant — la nature comme cahier des charges

La cible n'est pas « un bon protocole de plus ». C'est un **organisme** : il se multiplie,
se répare, se nourrit de son milieu, et la loi du plus fort y règne (tu contribues, tu
avances ; tu parasites, tu fades). Cette biologie n'est pas décorative — elle est déjà
inscrite dans les décisions LOCKED et les ADR. La charte ne fait que l'expliciter comme
loi de conception.

| Propriété du vivant | Traduction ToM (déjà en place) | Ce que la charte exige EN PLUS |
|---|---|---|
| **Se multiplier** | Backup virus (ADR-009) : un message pour un absent se réplique sur des nœuds de sauvegarde, se supprime quand livré ou à 24h | Que la capacité de réplication CROISSE avec N (plus de nœuds = plus de porteurs sains), jamais qu'elle sature |
| **Se réparer** | Isolation recovery (ADR-010), re-sondage des chemins morts (R14 Lot C), hub failover | Temps de cicatrisation borné et **décroissant** avec la densité (plus de voisins = réparation plus vite) |
| **Se nourrir du milieu** | Chaque device = client + relais ; rôles réseau-imposés selon la contribution | Le débit/capacité agrégé du réseau doit être **super-additif** : +1 nœud contributif > +1 charge |
| **Loi du plus fort (fade, pas ban)** | Réputation à fade progressif (LOCKED #4), « sprinkler gets sprinkled » (LOCKED #5) | Le tri par contribution doit rester **sans exclusion binaire** ET converger vers une allocation de rôles efficace à grande échelle |
| **S'auto-héberger** | Chaque device = client + relais + backup + rendez-vous ; zéro infra externe (ADR-002 bootstrap éliminé, ADR-010 zéro-config) | Rester vrai à TOUTE échelle : aucune fonction ne doit exiger un serveur privilégié. C'est LE mécanisme de l'anti-cyclicité (§0) |
| **Invisible / fondation** | Couche invisible (LOCKED #6), fondation universelle type TCP/IP (LOCKED #7) | Zéro état protocolaire visible même sous charge ; l'organisme travaille, l'utilisateur ne le voit pas |
| **Avancer ou crever** | (implicite) | Rendre EXPLICITE : un nœud qui ne contribue pas fade ; un chemin qui meurt est re-sondé puis abandonné ; rien ne stagne indéfiniment (déjà : TTL 24h LOCKED #2, cooldowns réversibles) |

**Garde-fou philosophique** : « loi du plus fort » en biologie ≠ darwinisme d'exclusion.
ToM applique un **fade réversible** (LOCKED #4) — le faible n'est jamais banni, il perd du
rôle et peut le regagner. La force du réseau vient de ce qu'il **n'exclut personne** tout
en récompensant la contribution. C'est un point non négociable : l'in-censurabilité et la
résilience VIENNENT de l'inclusion (plus de portes = moins de points de blocage).

**Et l'équilibre n'est pas du volontariat** : tu ne peux pas te servir du réseau si le
réseau ne peut pas se servir de toi — c'est le **protocole** qui gère l'équilibre
usage/contribution (deux compteurs, score = différence, whitepaper §5.1-5.2 ; sur-coût
progressif pour le déséquilibré, §6.5 « l'arroseur arrosé »), pas un choix de
l'utilisateur. La question « quelle incitation à contribuer ? » est mal posée dans ToM :
la contribution est la **condition d'usage**, réseau-imposée (rôles LOCKED #6),
proportionnelle aux capacités du device.

---

## §2 Le tableau des cibles agressives

> Règle : notre cible doit **égaler ou battre** la meilleure barre d'industrie sur chaque
> axe, ET satisfaire le test anti-cyclique (§0 : ne pas se dégrader quand N croît).
> Colonnes « barre industrie » en cours de sourçage (3 recherches). Les cibles ToM
> proposées sont des DRAFTS à ratifier.

### Axe 1 — Latence (établissement + reconvergence)
Barres sourcées : QUIC 1-RTT = 1× RTT réseau (0-RTT en gagne un) ; WireGuard handshake
< 100 ms/2 paquets ; hole-punch **libp2p DCUtR ~70 %**, **Tailscale/iroh ~90 %** ;
time-to-DIRECT **libp2p ~5 s** ; latence post-punch typique **+30-50 %** du RTT (meilleur
connu : WebRTC LAN ~+5-10 %) ; heartbeat Tailscale 2 s.

| Métrique | Barre industrie | Cible ToM (agressive) | Où on en est | Anti-cyclique ? |
|---|---|---|---|---|
| Connexion à un pair connu (chaud) | QUIC 0-RTT ~1 RTT | **< 1 s**, ↓ quand densité ↑ | I10 : ~1 s LAN (chaud), mécanisme R15 prouvé | Oui : plus de relais = chemin plus court |
| **Time-to-DIRECT (froid, cross-NAT)** | ~5 s (libp2p), ~3-5 s (iroh) | **≤ 1 s** — battre l'industrie | chantier historique (dial parallèle à durcir) | Oui : plus de voisins candidats |
| Taux de connexion DIRECTE | 70 % libp2p / 90 % Tailscale/iroh | **≥ 95 %** | à instrumenter (distinct du taux de LIVRAISON) | Oui : plus de chemins possibles |
| Reconvergence mort de chemin (I9b) | multipath QUIC (peu publié) | **≤ 60 s puis viser bien moins** | re-probe 15 s (build 135), terrain à re-mesurer | Oui : plus de voisins = re-probe plus vite |
| Latence post-DIRECT vs relais | +30-50 % du RTT | **≤ +10 %** | ✅ déjà : 4,5 ms direct v6 vs ~150 ms relais (LAN) | Oui |

> ⚠️ Ne pas confondre deux métriques que l'industrie sépare et que nous devons séparer
> aussi : le **taux de connexion directe** (hole-punch réussi) ≠ le **taux de livraison**
> (message arrivé, quitte à passer par relais + backup). Nos ~98,6 % de campagne sont de
> la LIVRAISON (relais/backup inclus), pas du direct pur. La barre 1 (time-to-DIRECT ≤ 1 s)
> reste le vrai défi face à l'industrie.

### Axe 2 — Rapidité (débit / propagation)
| Métrique | Barre industrie | Cible ToM (draft) | Mesure | Anti-cyclique ? |
|---|---|---|---|---|
| Propagation d'un message à N sauts | _(gossipsub vs N)_ | à fixer | banc multi-nœuds | **Test dur** : la courbe doit rester sous-linéaire |
| Débit **par nœud** à charge/nœud fixe | _(BitTorrent seeders)_ | ne s'effondre pas quand N croît (plat ou mieux) | banc courbe de masse (`banc-courbe-masse.md`) | Oui par construction — à PROUVER (l'agrégat brut serait tautologique) |

### Axe 3 — Résilience
| Métrique | Barre industrie | Cible ToM (draft) | Mesure | Anti-cyclique ? |
|---|---|---|---|---|
| Fraction de churn tolérée sans perte | _(études Kademlia)_ | à fixer (agressif) | chaos churn (tom-dht en a) | Oui : plus de nœuds = churn absolu plus absorbable |
| Temps de cicatrisation post-blackout | _(reconnexion réseaux P2P)_ | borné, **↓ avec densité** | test blackout | Oui |
| Livraison différée (absent 24h) | _(Signal queue, Briar mailbox)_ | 100% dans le TTL, redondance ↑ avec N | endurance ADR-009 | Oui : plus de porteurs sains |

### Axe 4 — Stockage
| Métrique | Barre industrie | Cible ToM (draft) | Mesure | Anti-cyclique ? |
|---|---|---|---|---|
| Empreinte mémoire par nœud sous charge | _(à contraster)_ | bornée en OCTETS, plafond dur | RssAnon, budgets (déjà : backup 64 Mio, pending 32 Mio) | **Doit rester CONSTANTE** quand N croît (pas de fuite par-pair) |
| Redondance du backup | _(IPFS erasure coding)_ | à fixer | — | Oui : plus de nœuds = plus de copies possibles |

### Axe 5 — Sécurité & résistance métadonnées
Distinction sourcée à tenir : **« chiffré » ≠ « résistant aux métadonnées » ≠ « in-censurable »**.
Barres : FS+PCS **formellement prouvés** (Signal/WhatsApp, Tamarin/ProVerif) ; métadonnées
— Briar/Session (fortes, zéro serveur / onion) > Signal sealed sender (cache l'émetteur au
*serveur*, PAS au DPI) > Matrix/WhatsApp/Telegram (fuient le graphe social).

| Propriété | Barre industrie | Cible ToM (agressive) | Où on en est |
|---|---|---|---|
| E2E (contenu) | Signal Double Ratchet | égaler | ✅ Ed25519+X25519+XChaCha20+HKDF |
| Forward Secrecy + Post-Compromise | **prouvé formellement** (Signal) | égaler ET **prouver** (Tamarin/ProVerif) | 🟡 tests unitaires seulement — GAP |
| Résistance métadonnées | Briar/Session (fortes) ; Signal partielle | **battre Signal** : pas de serveur central qui voie le graphe (avantage P2P natif) | 🟡 relais pass-through sans log, MAIS gossip+DHT observables — à durcir |
| Anti-replay / anti-spam | — | fade progressif + nonce TTL | ✅ R11 |

### Axe 6 — In-censurable
Barres : Briar (mesh local, zéro serveur → incensurable réseau) ; Tor+Snowflake (proxies
éphémères + bridges distribués → résiste à l'IP-blocking) ; obfs4 (résiste au DPI mais
fingerprinté/bloqué en Chine/Russie). Faiblesse universelle : **le bootstrap est un SPOF**
(bridges énumérables, seed relais bloquables).

| Propriété | Barre industrie | Cible ToM (agressive) | Où on en est |
|---|---|---|---|
| Zéro point de blocage central | Signal/WhatsApp échouent (1 serveur = 1 pays coupé) | maintenir : pas de serveur central | ✅ en régime établi (ADR-002, ADR-010) ; 🟡 amorçage à froid via DHT Mainline public (§4) |
| Résistance IP/DPI/SNI | Tor+Snowflake (référence) | **la masse = anti-blocage** : chaque device une porte, PAS de liste de bridges à énumérer | 🟡 thèse forte, MAIS SNI `.tom.invalid` = signature DPI identifiable (tension, voir §4) |
| Fonctionne sous coupure | Briar (hors-internet, Bluetooth/WiFi) | égaler à terme | 🟡 LAN/direct existe ; pas de transport hors-IP |

> **L'argument massa décisif sur la censure** : Tor a une *liste de bridges* qu'un État peut
> énumérer et bloquer. Si dans ToM **chaque utilisateur est un relais**, il n'y a pas de
> liste à énumérer — le coût de blocage croît avec la population. C'est l'anti-cyclicité
> appliquée à l'in-censurabilité : *plus de masse = plus de portes = blocage plus cher*.
> C'est peut-être notre différenciateur le plus fort face à Tor lui-même. À condition de
> régler la signature DPI (§4).

---

## §3 Comment la charte devient un JUGE

Elle n'a de valeur que si elle **arbitre**. Trois points d'entrée :

1. **Juge de décision** : toute proposition (feature, tuning, ADR) répond à deux
   questions AVANT le code — (a) « quelle cible de la charte ça sert, et de combien ? »
   (b) « ça passe le test anti-cyclique (§0) ? ». Une décision qui dégrade un axe quand
   N croît doit être justifiée explicitement ou refusée. *(C'est exactement ce qui a
   manqué au tuning re-probe 30 s : « 90-350 s » aurait dû être jugé inacceptable
   d'emblée par la cible latence, pas proposé comme option.)*
2. **Juge de build** : un tableau de bord (à construire) où chaque build reporte sa
   mesure sur les axes instrumentables (I9b, I10, RssAnon, churn). Régression sur un axe
   agressif = signal rouge, comme la gate clippy/test.
3. **Juge de roadmap** : les axes « à fixer » et les gaps (§4) deviennent des chantiers
   priorisés par l'écart à la cible, pas par l'envie.

---

## §4 Ce qui manque AUJOURD'HUI pour tenir la thèse (gaps honnêtes)

La thèse anti-cyclique est un OBJECTIF, pas un acquis. Points où ToM ne la tient pas
encore (à instruire, sans complaisance) :

- **DHT rendez-vous : 8 slots** (`tom-dht`, `RENDEZVOUS_SLOTS=8`). À grande échelle,
  collisions de slots → bruit croissant avec N. Ce n'est PAS anti-cyclique en l'état
  (connu, déjà noté en Known Limitations). Chantier : slots ∝ échelle, ou schéma sans
  collision.
- **Gossip mesh** : la propagation à N sauts doit être PROUVÉE sous-linéaire — pas mesurée
  aujourd'hui à grande échelle. Risque de broadcast storm (la masse qui DÉGRADE).
- **Amorçage à froid : dépendance au DHT Mainline public** (vérifié `tom-dht/src/lib.rs:238`
  « bootstraps from well-known mainline DHT nodes », `:155` : 4 hostnames publics résolus
  en DNS). Le rendez-vous zéro-config s'amorce sur une infra EXTERNE (le DHT BitTorrent) :
  censurable (DNS/IP des bootstrap), hors de notre contrôle. Contredit « le réseau
  s'héberge lui-même » pour le SEUL cold-start (le warm start passe par relais + pairs
  persistés). Atténuation déjà en place : bootstrap custom supporté (`:254`). Piste :
  s'amorcer sur nos propres nœuds (le réseau devient son propre bootstrap), Mainline en
  secours.
- **Concentration de l'infra (nœud-équivalent, §0)** : si le gros de l'infra utile vient
  du top-10 % des nœuds (NAS, always-on), la décentralisation est de façade. Métrique à
  instrumenter au banc : part du trafic relayé/stocké par décile de nœuds. Pas de cible
  chiffrée encore — d'abord MESURER la distribution réelle.
- **Coût mobile** : power-save assumé (décision produit), mais le coût CPU/batterie par
  device sous charge n'est pas budgété. Un organisme ne doit pas épuiser ses cellules.
- **Preuve du super-additif** : « plus de relais = plus de débit » est une hypothèse de
  conception, pas encore un chiffre. À DÉMONTRER avec la métrique honnête (§0 : par nœud,
  à charge/nœud fixe) — design du banc : `docs/plans/banc-courbe-masse.md`.
- **Pic mémoire transitoire** en charge (différé) : borné en rétention mais pas en débit
  d'émission — pourrait ne pas être constant avec N.
- **Tableau de bord juge (§3.2)** : n'existe pas. Sans lui, la charte reste déclarative.
- **Signature DPI `.tom.invalid`** : notre SNI (invariant wire « sacré » côté CLAUDE.md) est
  un **marqueur identifiable** — un DPI peut fingerprinter ToM et bloquer par SNI, exactement
  la faiblesse d'obfs4. Tension réelle entre l'invariant wire figé et l'axe in-censurable.
  À arbitrer : obfuscation de transport (à la Snowflake/obfs4) sans casser le namespace ?
  NE PAS changer unilatéralement (c'est un invariant protocolaire) — question stratégique
  pour Malik.
- **FS/PCS non prouvés formellement** : Signal a une preuve Tamarin/ProVerif ; nous avons des
  tests unitaires. Pour être crédible « mieux que l'industrie » sur la sécurité, il faudra
  une preuve formelle, pas juste des tests.
- **Métadonnées gossip/DHT observables** : notre atout (pas de serveur central qui voie le
  graphe) est réel, mais le trafic gossip et les lookups DHT restent observables par un
  réseau. Battre Signal sur les métadonnées demande de traiter ça.
- **Trade-off latence ↔ censure** : l'obfuscation (Tor) coûte +500 ms-2 s, incompatible avec
  nos cibles latence. Nos relais sont rapides mais bloquables. Ce trade-off est fondamental —
  la sortie est probablement « rapide par défaut, obfusqué à la demande sous blocage ».

> Le premier vrai test de la thèse ne sera pas un chiffre isolé, mais une COURBE :
> mesurer un axe à N croissant et montrer que la pente va dans le bon sens — **par nœud,
> à charge par nœud fixe**. Tant qu'on n'a pas cette courbe sur au moins un axe, « la
> masse est un carburant » reste une intention. Design du banc (phases, garde-fous
> contention, WAN simulé) : `docs/plans/banc-courbe-masse.md`.
