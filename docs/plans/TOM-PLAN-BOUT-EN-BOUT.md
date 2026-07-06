# ToM — Plan d'exécution bout-en-bout

> Vision & architecture : `TOM-MASTER-MAP.md` (v2). Ce doc = **le chemin**, jalon par jalon.
> Structure par jalon : **Objectif · Livrable attendu · Critère de succès (mesurable) · Risque · Mitigation**.
> Principe : on avance **couche par couche**, on **prouve** avant de promettre. 2026-07-06.

---

## Vue d'ensemble — 4 phases

| Phase | Couche | But | État |
|---|---|---|---|
| **P0** | L0 socle | réseau P2P + transport + backup | ✅ **livré** (build 18) |
| **P1** | L0 robuste | zéro-friction + distribution multi-plateforme | 🔨 en cours (R13→R16) |
| **P2** | L1 swarm | intégrité réseau : Proof of Presence + rôles | ⏳ prochaine grande étape |
| **P3** | L2 valeur | portefeuille scellé (bonus, différé) | 🎁 après P2 prouvée |

Règle d'or du plan : **P3 ne démarre pas tant que P2 (surtout les murs 3 & 4) n'est pas empiriquement validée.**

---

## PHASE 0 — L0 socle ✅ LIVRÉ

| Jalon | Objectif | Livrable | Succès | Statut |
|---|---|---|---|---|
| **P0.1** | Réseau P2P décentralisé | 7 crates `tom-*` forkées, transport QUIC | Nœuds se parlent sans serveur | ✅ |
| **P0.2** | Messagerie E2E + groupes | crypto, envelopes, hub/failover | 771 tests TS + ~600 tests Rust | ✅ |
| **P0.3** | Backup « virus » | module `backup/` (survival_score, self-delete, 24h) | Livraison différée validée endurance | ✅ |
| **P0.4** | Gros paquets + durcissement | chunking 64 Mo + 5 fixes DoS + fixes CPU/watchdog | Flotte build 18, 64 Mo @ ~6 Mo/s LAN | ✅ |

**Acquis mesuré** : iPhone/iPad/Apple TV/macOS/NAS se parlent sans serveur central, encaissent 64 Mo, se réparent seuls. **C'est le socle réel de tout le reste.**

---

## PHASE 1 — L0 robuste & sans friction (R13→R16)

**Objectif de phase** : que « M. et Mme Tout-le-monde » puissent héberger un nœud **sans ouvrir un port à la main**, et que le nœud soit distribuable partout.

| Jalon | Objectif | Livrable attendu | Critère de succès | Risque | Mitigation |
|---|---|---|---|---|---|
| **R13** | Porte d'entrée auto (LE multiplicateur) | UPnP/NAT-PMP/PCP dans MagicSock + self-relay ouvre son port seul | iPhone en data ↔ maison **sans le NAS** | Box/routeurs qui bloquent UPnP | fallback relais + IPv6 (R14) + diagnostic |
| **R14** | IPv6 first-class | publier GUA v6 au rendez-vous + préférence v6 au dial + PCP pinhole | Connexion DIRECT QUIC entrante v6 réussie | FAI sans v6 / pare-feu box | garder v4+relais en secours |
| **R15** | Annuaire local des pairs | persister `node_id → relais/addrs/path_kind` + dial parallèle cache | Reconnexion < seuil, moins de round-trips | cache périmé → dial mort | expiration douce + lookup frais parallèle |
| **R16** | Nœud léger multi-plateforme | binaire musl statique + paquets : Pi Imager · Pi-Apps · Docker · VM Freebox | 1 install en < 5 min sans terminal | soumissions stores lentes | commencer par Docker + Pi-Apps (communautaire) |

**Livrable de phase** : un nœud « grand public » installable en quelques clics, avec porte d'entrée automatique. **Débloque l'adoption par le bas.**

---

## PHASE 2 — L1 swarm : Proof of Presence (intégrité réseau)

**Objectif de phase** : implémenter le **swarm de rôles** (Observateur/Validateur/Gardien) et le **Proof of Presence**, comme couche d'intégrité **RÉSEAU** — utile **sans** l'argent. C'est ici qu'on prouve (ou casse) les murs 3 (entropie) et 4 (Sybil).

| Jalon | Objectif | Livrable attendu | Critère de succès | Risque | Mitigation |
|---|---|---|---|---|---|
| **L1-001** | Attestation de présence (primitive) | A défie B → B signe une attestation avec **preuve de relais récent**, éphémère 30s | Attestation vérifiable, anti-replay, purgée 30s ; agrégat → entropie | l'attestation ne prouve pas une activité RÉELLE (Sybil « vivant » sans relayer) | lier la preuve à un relais réel constaté par un tiers |
| **L1-002** | Entropie non-biaisable (mur 3) | seed = agrégat d'attestations d'AUTRES nœuds + **VDF** | Impossible de re-tirer la sélection (grinding échoue en test adverse) | VDF coûteuse/complexe sur mobile | paramétrer le délai VDF ; benchmarker sur Pi/phone |
| **L1-003** | Sélection en cascade | choisir un quorum imprévisible à partir du seed L1-002 | Attaquant ne peut pas prédire ni se positionner | source d'entropie insuffisamment aléatoire | test statistique + red-team |
| **L1-004** | Anti-Sybil chiffré (mur 4) | probation **avec durée + coût** = preuve de relais cumulée ; Q paramétrable | P(quorum majority-Sybil) < seuil pour Q donné et % Sybil donné | ferme patiente qui accumule de la présence | coût de présence croissant + Q élevé pour opérations critiques |
| **L1-005** | Validation croisée | « validateurs proposent, observateurs vérifient, relais transmettent, gardiens attestent » | Une anomalie détectée par un rôle est signalée et bloque | collusion inter-rôles | diversité de sélection + rotation |
| **L1-006** | Ancrage L1 (présent) | engagement Merkle + version, **sans ledger**, signé par quorum | État présent vérifiable, purge agressive respectée | dérive vers un ledger (viole #2/#3) | invariant testé : L1 ne stocke que commitment+version |

**Livrable de phase** : un réseau qui **sait qui est présent et honnête**, avec un aléa de sélection prouvé non-biaisable et une résistance Sybil quantifiée. **Vaut le coup seul** (anti-spam, anti-abus, confiance réseau) — indépendamment de l'argent.

**Go/No-Go P3** : on ne passe à L2 **que si** L1-002 (entropie) et L1-004 (Sybil) sont **empiriquement validés en red-team**.

---

## PHASE 3 — L2 valeur : le portefeuille scellé (BONUS, différé)

**Objectif de phase** : transfert de valeur sans double dépense, sans frais, sans grand livre — **posé sur le swarm L1**. Ne démarre qu'après le Go P3.

| Jalon | Objectif | Livrable attendu | Critère de succès | Risque | Mitigation |
|---|---|---|---|---|---|
| **L2-001** | Wallet scellé auto-custodié | état = montant + sceau (hash+version+sigs), **stocké dans le wallet** | Ouvrir/dépenser/re-sceller devant témoins frais | perte d'appareil = perte de fonds | (L2-004) récupération |
| **L2-002** | Continuité d'attestation scopée (mur 1) | témoins d'un wallet **se relaient** + mémoire de version transmise | Impossible de re-dépenser depuis un sceau consommé | churn des témoins perd la mémoire | quorum + redondance + handoff obligatoire |
| **L2-003** | Dépense sûre sous partition (mur 2) | **blocage** de la dépense si quorum partitionné (cohérence > dispo) | Aucun double-spend en test de partition Est/Ouest | UX : paiement refusé hors-ligne | messagerie reste dispo ; annoncer clairement le trade-off |
| **L2-004** | Récupération de wallet | multi-device et/ou social recovery via gardiens (à trancher) | Récupérer sans autorité centrale ni historique global | toute piste plie un principe | décision produit explicite au moment venu |
| **L2-005** | Frontière L0/L2 formelle | invariant : le transport n'ouvre jamais la sémantique valeur | Test prouvant que L0 reste content-agnostic | fuite de sémantique dans le routage | tests d'invariance + audit |

**Livrable de phase** : « dépenser en public devant des témoins, re-sceller, sans péage ni ledger ». **Le sommet — livré en dernier, honnêtement, avec ses trade-offs assumés.**

---

## TRANSVERSE (tout le long)

| Chantier | Objectif | Livrable | Risque | Mitigation |
|---|---|---|---|---|
| **GTM bottom-up** | adoption par les geeks souverainistes | Pi/Docker/Freebox packages + doc LLM-first + SDK npm + MCP | pas de traction | canal LLM (assistant recommande ToM) |
| **Sécurité** | durcissement continu | red-team régulier (Fable), pre-push gate, review interne agent | régression | tests non-régression par classe de bug |
| **Observabilité** | mesurer le réseau réel | campagnes stress + métriques flotte | angles morts | scénarios chaos + monitoring |
| **Invisibilité (#6)** | l'utilisateur ne voit rien | SDK intégrable, zéro config | complexité exposée | frontière stricte protocole/produit |

---

## Chemin critique & plus gros risque

**Chemin critique** : R13 (porte auto) → L1-001/002 (attestation + entropie) → L1-004 (Sybil) → **Go P3** → L2-002/003 (continuité + partition).

**Plus gros risque non résolu** : la **faisabilité de l'entropie PoP non-biaisable** (mur 3) et le **coût réel de la résistance Sybil** (mur 4). **Tout L2 en dépend.** → C'est **exactement** ce que L1-001 puis L1-002/L1-004 vont trancher empiriquement, **avant** d'investir dans la valeur.

**Décision de séquencement** : L1 d'abord (utile seul), L2 après preuve. Pas l'inverse.
