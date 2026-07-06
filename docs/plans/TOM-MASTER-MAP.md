# ToM — Master Map (vision + architecture consolidée)

> **Version 2 (2026-07-06)** — intègre la revue adversariale Fable 5 (§11) et la
> décision de layering (L1 réseau d'abord, L2 valeur différée). Voir le plan
> d'exécution de bout en bout : `docs/plans/TOM-PLAN-GLOBAL.md`.
> Chaque section marquée 🎯 contient une surface d'attaque explicite.
> Source de vérité amont : `docs/tom-whitepaper-v1.md` (genèse §3-4),
> `_bmad-output/planning-artifacts/design-decisions.md` (7 décisions LOCKED).

---

## 0. Thèse en une phrase

ToM est **la couche de transport souveraine** qui rend obsolètes trois péages : la surveillance déguisée en messagerie gratuite, la rente par transaction des réseaux de paiement, et le serveur central lui-même. **Une blockchain qui voyage léger** : elle garde les deux cadeaux (pas d'autorité centrale, pas de double dépense) et jette les trois boulets (histoire éternelle, consensus par PoW/PoS, ordre global sériel).

---

## 1. Les 7 décisions LOCKED (non négociables)

Elles gouvernent tout le reste. Toute proposition qui les viole est rejetée.

1. **Livraison** — un message est livré **si et seulement si** le destinataire émet un ACK.
2. **TTL** — durée de vie 24h max, puis **purge globale**, sans exception. Pas d'historique.
3. **L1** — ancre l'état présent, **n'arbitre jamais** (ne décide pas la livraison, ne juge pas).
4. **Réputation** — fade progressif, **jamais de ban permanent**, pas d'états binaires (gradient).
5. **Anti-spam** — « the sprinkler gets sprinkled » : plus d'abus → plus de travail exigé, jamais d'exclusion.
6. **Invisibilité** — ToM est une couche, pas un produit. Invisible à l'utilisateur final.
7. **Scope** — fondation universelle (comme TCP/IP), pas une application.

**Conséquence structurante** : #2 (pas d'historique) interdit tout grand livre global. Toute la conception du paiement en découle.

---

## 2. Architecture en 3 couches

| Couche | Nom | Rôle | Connaît la sémantique ? | État |
|---|---|---|---|---|
| **L0** | Transport | le tuyau : livraison distribuée + backup + présence | **NON** (dollar = photo = octet) | ✅ **livré** |
| **L1** | Swarm | rôles Observateur/Validateur/Gardien : intégrité **réseau** (présence, anti-Sybil, validation croisée) + ancrage présent | non | 🔨 prochaine grande étape |
| **L2** | Valeur | transfert de valeur + anti-double-dépense (**bonus**, dernières étapes) | oui | 🎁 sommet |

Principe cardinal : **la double dépense est une propriété de la couche d'ATTESTATION (L1), pas du transport (L0).** Le tuyau reste bête, rapide, content-agnostic. La valeur (L2) est un service que le swarm offre, jamais une cargaison que le tuyau ouvre.

---

## 3. L0 — Le transport (socle livré)

Ce qui tourne **aujourd'hui, sur du vrai matériel** (pas une simulation) :

- Réseau P2P décentralisé : chaque appareil = client + relais. Aucun serveur central obligatoire.
- Découverte zéro-config (DHT rendez-vous partagé, gossip, mDNS LAN, relais de secours).
- Chiffrement E2E (Ed25519 + X25519 + XChaCha20-Poly1305 + HKDF).
- **Backup « virus »** : un message pour un hors-ligne se réplique sur plusieurs nœuds backup, surveille son `survival_score` (fuseau du destinataire, historique, bande passante), **réplique vers un meilleur hôte en parallèle** quand le score baisse, **s'auto-supprime avant que l'hôte meure**, et se purge à l'ACK ou à 24h.
- Segmentation transport : messages jusqu'à 64 Mo (chunking transparent, budget mémoire borné, anti-DoS).
- Résilience : recovery d'isolement 15s (reprobe relais + republish DHT + rejoin).

**Preuve mesurée (2026-07-05)** : flotte réelle (iPhone, iPad, Apple TV, macOS, NAS) — **64 Mo livrés à ~6 Mo/s en LAN**, ~5 Mo/s via relais WiFi, 100 % de livraison, self-healing après coupure. Durci contre 5 classes de DoS (amplification réassemblage, budget mémoire, spam de transferts, collections non bornées d'un pair malveillant).

🎯 **Surface d'attaque L0** : suspension iOS/tvOS en arrière-plan (contrainte OS, mitigée par backup + wake-up APNs futur) ; une seule « porte publique » par foyer aujourd'hui (SPOF résolu par R13 UPnP/NAT-PMP) ; analyse de trafic par un observateur global (routage onion non implémenté).

---

## 4. Le vivant — les rôles comme micro-agents

Chaque rôle = un **micro-processus autonome, hyper-optimisé** (conso quasi nulle). **C'est leur MULTIPLICATION mondiale qui sécurise le réseau** — « la masse comme rempart » (wp 4.4). Les rôles sont :

- **Réseau-imposés** : personne ne choisit son rôle (contre BitTorrent seed / Nostr relais).
- **Imprévisibles, sélectionnés au dernier moment en cascade** (wp 4.3) : une source d'entropie vérifiable sélectionne un premier groupe, qui génère l'aléa du suivant, jusqu'au quorum. **Impossible de se positionner à l'avance.**
- **Migrants** : quand un nœud part ou surcharge, **le subnet redistribue ses rôles** (wp 3.3-3.4). La **fonction** se recopie, pas seulement les données — un nœud qui portait un backup transmet et le message ET le rôle.

### Les rôles

| Rôle | Job (micro-agent) | Statut code |
|---|---|---|
| **Client** | envoie/reçoit ses propres messages | ✅ `PeerRole::Peer` |
| **Relais** | transmet pour les autres (multi-sauts), pass-through | ✅ `PeerRole::Relay` |
| **Backup** | garde les messages des hors-ligne (virus, survival_score, self-delete) | ✅ module `backup/` |
| **Observateur** | surveille l'état d'un wallet/subnet, **co-signe les transitions**, **empêche la double dépense** (règle d'or : jamais 2 transitions du même état) | 🔨 à construire (L1) |
| **Gardien** | **aide les nouveaux nœuds à se synchroniser** + **atteste** en validation croisée | 🔨 à construire (L1) |
| **Validateur** | participe aux **quorums de validation** (Proof of Presence) | 🔨 à construire (L1) |
| **L1 (Anchor)** | BUS organique : ancre l'état **présent** (engagements Merkle + version + signatures), genèse glissante, purge agressive — **pas de ledger** | 🔨 à construire |

### Proof of Presence (PoP)

Le droit de valider vient de **la présence** (être connecté, relayer, répondre), pas du capital (PoS) ni du calcul (PoW). Consensus **gratuit**. Sécurité = **imprévisibilité × masse** : pour attaquer, il faudrait contrôler une fraction significative de tous les nœuds présents, au moment exact de la sélection, sans savoir lesquels seront choisis.

### Validation croisée (wp 4.5)

« Les validateurs proposent, les observateurs vérifient, les relais transmettent, les gardiens attestent. **Chaque rôle surveille les autres.** » Un swarm de contre-pouvoirs, pas une autorité.

🎯 **Surface d'attaque L1** : la source d'entropie de la sélection cascade doit être **indépendante du demandeur** (sinon grinding : re-tirer jusqu'à obtenir un quorum complice) ; disponibilité d'un quorum sous partition réseau ; anti-Sybil (voir §6) ; coût réel de « conso quasi nulle » à quantifier sur des millions de nœuds mobiles.

---

## 5. L2 — La valeur : le portefeuille scellé à la cire

> Modèle affiné avec l'auteur. Reformulation concrète de « les observateurs co-signent les transitions ».

**Le wallet est un coffre scellé, auto-custodié.** Son état courant = un **montant + un sceau de cire** = le dernier engagement co-signé (hash d'état + version + signatures d'observateurs). **Ce sceau est stocké DANS le wallet lui-même, sur l'appareil du propriétaire — pas sur le réseau.**

### Le rituel de dépense (public, témoigné)

1. **On prouve qu'on a de quoi tirer** : le propriétaire présente le dernier sceau valide (montant scellé ≥ dépense).
2. **On ouvre devant tout le monde** : la transition est diffusée à un ensemble d'observateurs **frais, imprévisibles** (sélection cascade PoP). Ils vérifient : sceau valide + version courante + fonds suffisants + **ce sceau n'a jamais servi** (règle d'or : un observateur qui a témoigné « sceau N → N+1 » refuse toute autre dépense depuis N).
3. **On re-scelle** : après la dépense (montant → montant − dépensé ; le destinataire crédite SON wallet par le même rituel), un **nouveau sceau de cire** est apposé — nouveau montant, nouvelle version, co-signé par les témoins — et **rangé dans le wallet du propriétaire**.

### Pourquoi c'est léger et cohérent avec les décisions LOCKED

- **Pas de grand livre global** : le wallet **porte sa propre preuve** (chaîne de sceaux auto-custodiée). Le réseau ne fournit que les **témoins vivants** au moment de la dépense. → respecte #2 (état présent, pas d'historique) et #3 (L1 ancre, n'arbitre pas).
- **Double dépense impossible** sans corrompre un ensemble de témoins **frais, imprévisibles et massif** à l'instant exact de la dépense.
- **« On en garantit quasiment la véracité »** — honnête : sécurité **probabiliste** (masse + imprévisibilité + continuité d'attestation), pas une vérité mathématique absolue façon chaîne globale. Le « quasiment » est le prix du « sans lourdeur ».

🎯 **Surface d'attaque L2 (à démonter en priorité)** :
- **Disponibilité des témoins** : si le quorum frais ne peut pas être assemblé (partition, offline), la dépense **stalle**. Modèle offline / pré-autorisations signées ?
- **Grinding de témoins** : le demandeur peut-il re-tirer la sélection jusqu'à un quorum complice ? → l'entropie doit être hors de son contrôle.
- **Collusion / Sybil** : coût réel pour devenir « le quorum » d'un wallet ciblé (voir §6).
- **Perte du wallet** : si l'appareil (donc le sceau auto-custodié) est perdu → fonds perdus, comme une seed de hardware wallet. Modèle de récupération (multi-device, social recovery) ?
- **Amorçage de la valeur** : d'où vient le montant initial (mint / on-ramp) ? Hors protocole, mais à répondre.
- **Destinataire de mauvaise foi** : il doit aussi témoigner/vérifier le sceau entrant ; que se passe-t-il s'il refuse d'accuser réception après avoir vu la valeur ?

---

## 6. Sécurité transversale

- **Anti-Sybil** : identité liée à l'appareil ; **période de probation** (un nouveau nœud peut envoyer/recevoir mais ne peut pas être témoin critique, quota limité, surveillé) ; coût de fabriquer des milliers d'identités présentes simultanément.
- **Réputation** (#4) : fade progressif (decay ~5%/h), rédemption toujours possible, dégradation sans drame.
- **Anti-spam** (#5) : l'abus devient irrationnel (plus de travail exigé), jamais interdit.

🎯 **Surface d'attaque sécurité** : « identité liée à l'appareil » sur du matériel virtualisé/émulé ; coût amorti d'une ferme de Sybils patients qui accumulent de la présence pendant la probation ; corrélation de métadonnées (from/to visibles au relais).

---

## 7. Pourquoi « blockchain, mais léger »

| Boulet blockchain | Réponse ToM |
|---|---|
| Histoire éternelle (600 Go → To) | **Genèse glissante** : seul le présent, le passé oublié (#2) |
| Consensus PoW/PoS (énergie/capital) | **Proof of Presence** : être là suffit, consensus gratuit |
| Ordre global sériel (7 tx/s) | **Subnets éphémères** : accord local par wallet, parallèle → rapide |
| Frais (block space rare) | Contribution imposée, pas de récompense, pas d'enchère |
| Plutocratie / industrialisation | Rôles imposés au dernier moment, imprévisibles, ingamables |

**Ce qu'on perd, assumé** : l'audit éternel. ToM choisit **la vérité du présent** contre **l'archive infinie**. Pour la messagerie, c'est un cadeau. Pour la valeur, c'est le pari à sécuriser (§5).

---

## 8. Go-to-market — par le bas

1. **Geeks souverainistes d'abord** : Raspberry Pi (Imager + Pi-Apps), Freebox (VM + `tom-gateway`), Docker/NAS, self-hosters. Message : « héberger un nœud = messagerie chiffrée gratuite, sans serveur à payer ».
2. **LLM comme canal primaire** : docs LLM-first + SDK npm + MCP server + plugin VS Code → un assistant recommande ToM en 2 lignes. « The LLM is the new distribution channel. »
3. **Couche invisible** : intégré dans box, routeurs, OS. Leila ne saura jamais. Reza en dépendra pour survivre.

---

## 9. Roadmap alignée sur les couches

- **L0 — Socle livré** : R1-R12 + apps build 18 + durcissement DoS. ✅
- **Court terme (finir L0 robuste)** : R13 porte d'entrée auto (UPnP/NAT-PMP — LE multiplicateur, vérifiée) · R14 IPv6 first-class · R15 annuaire local des pairs · nœud léger multi-plateforme (R16).
- **Moyen terme — L1 Swarm** : implémenter Observateur/Validateur/Gardien + Proof of Presence + sélection cascade + validation croisée + ancrage présent L1. **Sécurise le réseau, indépendamment de l'argent.**
- **Long terme — L2 Valeur (bonus)** : le portefeuille scellé (§5) sur le swarm L1. Transfert de valeur sans double dépense, sans frais, sans grand livre.

---

## 10. Les questions ouvertes (pour Fable : porte-les au step suivant)

1. **L1 en premier** : quelle est la **première story concrète** du swarm ? (proposer : « attestation de présence » — un nœud prouve/co-signe la présence d'un autre, sans argent — le primitif de base du PoP.)
2. **Entropie de la sélection cascade** : quelle source vérifiable, indépendante du demandeur, sur un réseau sans horloge globale ni bloc ?
3. **Disponibilité du quorum d'observateurs** d'un wallet quand ils tournent/partent : seuils, redondance, re-scellement paresseux ?
4. **Récupération d'un wallet perdu** sans autorité centrale (multi-device ? social recovery via gardiens ?).
5. **Le « quasiment »** de §5 : peut-on le **quantifier** (probabilité de double dépense réussie = f(taille du quorum, taux de Sybil, imprévisibilité)) pour en faire un paramètre réglable plutôt qu'un flou ?
6. **Frontière L0/L2** : garantir formellement que le transport ne connaît jamais la sémantique valeur (invariant testable).

---

## 11. Revue adversariale Fable 5 — les 4 murs et leurs portes (2026-07-06)

Fable 5 a démonté la V1. Verdict : **L0 tient (solide, mesuré) ; les 7 décisions LOCKED sont cohérentes TANT QU'ON RESTE messagerie. C'est L2 (valeur) qui déchire le contrat.** Quatre murs, chacun avec une porte — mais la porte a un prix. Portes = **propositions à valider (Fable + auteur)**, pas des vérités.

| # | Mur (ce qui casse) | Porte proposée (avec son prix) | Statut |
|---|---|---|---|
| 1 | **Entropie PoP introuvable** : sans horloge/bloc global, le demandeur peut *grinder* la sélection cascade jusqu'à un quorum complice. | Entropie issue des **attestations agrégées d'AUTRES nœuds vivants** (hors contrôle du demandeur) + **VDF** (fonction à délai vérifiable, aléa non-biaisable sans horloge). Prix : complexité crypto, latence VDF. | 🔴 à prouver (story L1-001 puis L1-002) |
| 2 | **Sybil sans coût** : probation sans durée ni prix → ferme patiente de N nœuds qui rafle le quorum. | Coût = **preuve de relais réel** (un témoin doit avoir vraiment relayé, pas juste exister) + taille de quorum **Q fixée** + fraction Sybil tolérée paramétrée. Prix : exclut le matériel purement passif ; Q élevé = plus de validations/op. | 🔴 à quantifier |
| 3 | **Partition = double-spend** : le même sceau présenté à deux quorums disjoints simultanément → deux dépenses valides, et #3 interdit à L1 d'arbitrer. | **Choix CAP assumé pour la VALEUR : cohérence > disponibilité.** Un paiement **se bloque** si le quorum du wallet est partitionné (comme une carte hors-ligne). Pas de double-spend car pas de dépense du tout quand les témoins sont coupés. La messagerie, elle, reste dispo. Prix : paiement indisponible sous partition. | 🟠 décision de conception à acter |
| 4 | **🔴 Contradiction #2 (purge 24h) ↔ persistance d'attestation** : l'anti-double-spend exige que le témoin se souvienne « j'ai signé N→N+1 » ; #2 purge tout à 24h → il oublie → re-spend. **Architectural.** | **Layering** : #2 gouverne le **transport/messagerie** (reste pur, sans historique). La **valeur (L2) est une couche séparée** avec sa propre règle : **attestation persistante PAR WALLET**, portée par un ensemble de témoins qui **se relaie** (migration de rôle : le témoin ET sa mémoire de version passent au suivant). Prix : L2 assume un état scopé (non global, non 24h) — exception explicite à #2, réservée à la valeur. | 🟢 **ACTÉ (auteur 2026-07-06)** : approche en couches acceptée |

**Ouvertures secondaires** (Fable) : récupération de wallet perdu (toutes les options plient un principe → à trancher en L2), taille de quorum jamais fixée (bloque le calcul de P(double-spend) → paramètre à définir).

**Le plus grand trou non résolu reste #1+#2 (entropie + Sybil du PoP)** — c'est **exactement** ce que la première story L1 va tester empiriquement, sans toucher à l'argent.

---

## 12. Le premier pas — story L1-001 (endossée)

**Attestation de présence** : A défie B ; B répond par une attestation signée incluant une **preuve d'activité récente** (B a relayé ≥1 message dans les 5 dernières s) ; l'attestation est **éphémère (30s), jamais persistée, jamais backupée** (aligné #2). Plusieurs attestations agrégées → **graine d'entropie** pour la sélection cascade (story suivante). C'est le **primitif** du Proof of Presence : petit, sans argent, sans partition, sans quorum — et il **révèle** si l'entropie/anti-Sybil sont réels. Critères d'acceptation détaillés dans `docs/plans/TOM-PLAN-GLOBAL.md` (jalon M1.1).

---

*Fin de la map V2. L0 est réel et mesuré. L1/L2 sont la vision à porter — les murs sont nommés, les portes proposées, le premier pas est petit et testable.*
