# ToM — Master Map v2 (vision + architecture consolidée)

> **v2 (2026-07-06)** : intègre la démolition adversariale Fable 5 (voir §11) et les
> **décisions actées** — approche en couches, résolution de la contradiction #2↔valeur,
> choix CAP pour la valeur. Le plan d'exécution bout-en-bout vit dans
> `docs/plans/TOM-PLAN-BOUT-EN-BOUT.md`.
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

## 11. Verdict de la revue adversariale Fable v1 + décisions actées

La revue a **confirmé L0** (solide, mesuré) et les 7 décisions **cohérentes tant qu'on reste messagerie**. Elle a trouvé **4 murs** sur la couche valeur. Chacun a désormais une **porte assumée** (avec son prix).

### Mur 1 — Contradiction #2 (pas d'historique) ↔ valeur (persistance d'attestation) 🔴 architectural
Anti-double-dépense exige qu'un témoin se souvienne « j'ai signé N→N+1 » ; #2 purge tout à 24h → oubli → re-dépense.
**Décision actée (layering)** : **#2 gouverne L0 (messagerie) — la mission reste pure, purge 24h sans exception.** La **valeur (L2) est une couche séparée** avec sa propre règle : une **attestation persistante PAR WALLET**, portée par un **ensemble de témoins qui se relaie** (migration de rôle : le témoin ET sa mémoire de version passent au suivant lors du churn). Ce n'est PAS un grand livre global ; c'est un état **scopé au wallet**, borné, vivant tant que le wallet est actif. **Prix assumé** : L2 n'est plus « zéro état » — elle accepte le minimum d'état que l'argent exige. La messagerie, elle, ne le paie pas.

### Mur 2 — Partition réseau = double-dépense 🔴 (le tueur du sharding sans ordre global)
Même sceau N présenté aux quorums Est ET Ouest pendant une coupure → 2 dépenses valides ; #3 interdit à L1 d'arbitrer.
**Décision actée (choix CAP)** : **pour la valeur, cohérence > disponibilité.** Un paiement **se BLOQUE** si le quorum de témoins du wallet est partitionné/injoignable (comme une carte qui refuse hors-ligne). On ne double-dépense pas parce qu'on **ne dépense pas du tout** quand les témoins sont coupés. **La messagerie (L0) reste disponible** (elle tolère la perte, #2). **Prix assumé** : pas de paiement en zone coupée/edge instable — acceptable pour de l'argent, inacceptable pour un message (d'où la séparation des couches).

### Mur 3 — Entropie du PoP / grinding 🔴
Sur un réseau sans horloge ni bloc, d'où vient l'aléa non-biaisable de la sélection cascade ?
**Direction (à prouver par L1-001)** : l'aléa vient des **attestations agrégées d'AUTRES nœuds vivants** (hors du choix du demandeur) + une **VDF** (Verifiable Delay Function — l'outil standard pour un aléa non-biaisable sans horloge : impossible à re-tirer car chaque essai coûte un délai incompressible). **Statut** : hypothèse à valider empiriquement avant tout L2.

### Mur 4 — Sybil sans coût 🔴
Probation sans durée ni prix → ferme patiente qui rafle le quorum.
**Direction** : le coût d'être un témoin valide = **preuve de relais RÉEL** (un nœud doit avoir effectivement relayé du trafic récemment, pas juste « exister ») + **taille de quorum Q paramétrable** (Q petit = attaquable, Q=100 → P(double-dépense) ≈ 10⁻³⁰ mais 100 validations/tx). **Statut** : Q et le seuil de Sybil toléré deviennent des **paramètres de sécurité explicites**, pas un flou.

### Ce qui reste ouvert (défer jusqu'à L1 prouvé)
- **Récupération de wallet perdu** sans autorité ni historique de confiance (toutes les pistes plient un principe → à trancher au moment de L2, pas avant).
- **Amorçage de la valeur** (mint / on-ramp) — hors protocole cœur.
- **Quantification fine** du « quasiment » une fois Q et le modèle de témoins fixés.

### Décision stratégique actée
**On construit L1 (swarm + PoP) comme couche d'INTÉGRITÉ RÉSEAU d'abord — elle vaut le coup SANS l'argent.** **L2 (valeur) est différée** jusqu'à ce que L1 fasse ses preuves. **Premier pas : story L1-001 (attestation de présence)** — la plus petite brique qui teste empiriquement les murs 3 et 4, sans toucher à la valeur.

---

*Fin de la map v2. L0 est réel et mesuré. Les murs de L2 sont nommés et ont des portes assumées. On avance couche par couche, en prouvant, pas en promettant.*
