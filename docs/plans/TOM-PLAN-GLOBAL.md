# ToM — Plan global de bout en bout (L0 → L1 → L2)

> Version 2026-07-06. Compagnon exécutable de `docs/plans/TOM-MASTER-MAP.md` (la vision).
> Approche **en couches actée** : L0 transport livré → L1 swarm réseau (Proof of Presence)
> → L2 valeur (différé, bonus). On ne monte à L2 qu'après que L1 ait fait ses preuves.
> Légende statut : ✅ livré · 🔨 à faire · 🟢/🟠/🔴 sévérité du risque.

---

## Vue d'ensemble — les 3 phases

| Phase | Couche | But | Argent ? | Statut |
|---|---|---|---|---|
| **0** | L0 Transport | Rendre le tuyau déployable **sans friction, partout, résilient** | non | ✅ socle livré, reste le durcissement déploiement |
| **1** | L1 Swarm | Bâtir les rôles + **Proof of Presence** comme couche d'**intégrité réseau** | **non** | 🔨 prochaine grande étape |
| **2** | L2 Valeur | Transfert de valeur sans double-dépense/frais/grand livre | oui | 🔨 différé (bonus final) |

**Chemin critique** : M1.1 (attestation) **∥ M1.2 (entropie — recherche, en parallèle)** → M1.3 (cascade) → M1.4 (anti-Sybil quantifié). Si M1.2 ne tient pas, **le PoP n'est pas réel** et L2 est caduque → M1.2 est le **verrou de recherche**, à lancer immédiatement, pas après M1.1. Tout est testable **sans argent**.

---

## PHASE 0 — L0 : socle transport robuste

**Objectif de phase** : « si mon beau-père doit ouvrir un port à la main, c'est mort ». Tout nœud installé = porte publique complète, multi-plateforme, auto-réparant.

### M0.1 — Porte d'entrée automatique (R13)
- **Objectif** : zéro-config d'ouverture réseau — tout nœud devient une porte publique (self-relay + mapping auto).
- **Livrable** : UPnP-IGD + NAT-PMP + PCP câblés dans MagicSock (`portmapper`), instrumentés sur Freebox ; test iPhone data ↔ maison **sans le NAS**.
- **Critère de succès** : un pair externe joint un nœud domestique sans intervention manuelle.
- **Risque** 🟠 : box qui refusent UPnP → *mitigation* : IPv6 (M0.2) + relais de secours.
- **Statut** : ✅ vérifiée 2026-07-03.

### M0.2 — IPv6 first-class (R14)
- **Objectif** : chaque appareil = adresse directe, plus de NAT.
- **Livrable** : publier le GUA v6 au rendez-vous + préférence v6 au dial + PCP pinhole auto ; règle Freebox 43925.
- **Critère** : connexion DIRECT QUIC v6 établie entre deux foyers.
- **Risque** 🟠 : FAI/box sans IPv6 → fallback v4/relais.

### M0.3 — Annuaire local des pairs (R15)
- **Objectif** : reconnexion instantanée aux pairs habituels.
- **Livrable** : persister `node_id → relais + addrs + path_kind` ; dial parallèle cache + lookup frais ; expiration douce.
- **Critère** : rejoin < 2s pour un pair connu.
- **Risque** 🟢 : annuaire périmé → expiration + revalidation.

### M0.4 — Nœud léger multi-plateforme (R16)
- **Objectif** : distribution par le bas, friction nulle.
- **Livrable** : binaire statique musl + **Raspberry Pi Imager** (« ToM Node OS ») + **Pi-Apps** + **Docker** (NAS/home-server) + **VM Freebox** (qcow2 ARM64, autostart, pairing natif).
- **Critère** : un geek flashe/installe et rejoint le réseau en < 5 min, sans terminal.
- **Risque** 🟠 : soumissions stores + signing → anticiper.

### M0.5 — Wake-up / suspension iOS (R18)
- **Objectif** : lever la seule vraie limite du transport mobile.
- **Livrable** : hook « sonnette » neutre dans le SDK + adaptateur **APNs** (puis FCM), rien sur headless.
- **Critère** : un iPhone en fond est réveillé pour recevoir, sans hack audio.
- **Risque** 🔴 : dépend d'un service push (APNs) → n'est pas 100% souverain ; *mitigation* : le backup 24h couvre l'intervalle, APNs = confort pas dépendance dure.

---

## PHASE 1 — L1 : swarm Proof of Presence (intégrité réseau, SANS argent)

**Objectif de phase** : prouver que le consensus par présence est **réel** (entropie non-biaisable + anti-Sybil chiffré), et livrer les rôles Observateur/Validateur/Gardien comme couche de sécurité **réseau** — utile même sans valeur.

### ⚙️ Paramètres à VERROUILLER avant M1.1 (sinon M1.1 est un leurre — Fable)
- **Durée de probation** : proposé **7 jours** (non négociable) avant qu'un nœud puisse être témoin critique.
- **Taille de quorum Q** : proposé **Q ≥ 50** (à calibrer en M1.4 sur `P(quorum Sybil)`).
- **Format « preuve d'activité récente »** : hash signé du **dernier message réellement relayé** + **compteur de relais signé** (timestamp lié) + fenêtre 5 s. Détail dans la story L1-001.
- Ces trois paramètres sont des **entrées** de M1.1/M1.4, pas des détails d'implémentation.

### M1.1 — Attestation de présence *(première story — endossée)*
- **Objectif** : primitif du PoP — A prouve que B est vivant MAINTENANT, B co-signe.
- **Livrable** : protocole challenge→réponse signé, avec **preuve d'activité récente** (B a relayé ≥1 msg dans les 5 s) ; attestation **éphémère 30s, jamais persistée, jamais backupée** (aligné #2).
- **Critères d'acceptation** :
  - Honnête : 100 % succès, latence médiane < 200 ms.
  - Anti-replay : nonce rejoué → rejeté.
  - Anti-forge : signature falsifiée → rejetée.
  - Anti-menteur : B hors-ligne mais « atteste » → rejeté (preuve d'activité fausse).
  - Éphémérité : purge après 30 s, jamais sur disque ni dans le backup.
  - Agrégation : hash de N attestations = graine reproductible **indépendante de l'ordre**.
- **Risque** 🟠 : « preuve d'activité » falsifiable → à durcir (lien cryptographique au dernier relais réel).
- **Dépend de** : L0 (DHT rendez-vous, QUIC). **Sans dépendance argent/partition/quorum.**

### M1.2 — Entropie non-biaisable *(le mur #1 — PROBLÈME DE RECHERCHE, à mener EN PARALLÈLE de M1.1)*
- **Objectif** : produire un aléa **vérifiable, indépendant du demandeur**, sans horloge ni bloc global.
- **⚠️ Nature (Fable)** : ce n'est **pas** de l'ingénierie, c'est de la **recherche**. La VDF (fonction à délai vérifiable) exige un **temps absolu mesuré** — dur à garantir sur un P2P **asynchrone sans NTP** (un nœud peut simuler « j'ai attendu 10s » en local). La littérature VDF sur réseau async est mince.
- **Livrable** : étude comparative + choix, parmi 3 candidats :
  1. **VDF** (Wesolowski/Sloth) — si la vérification du délai tient sur async ; sinon éliminée.
  2. **Beacon passif** — aléa = hash des **N derniers messages du réseau** (personne ne les contrôle tous).
  3. **Signature-seuil du churn** — `t-of-n` des relais présents co-signent un aléa.
- **Critère** : un demandeur qui **re-tire 10⁶ fois** ne gagne aucun avantage de sélection (test de grinding), sur le candidat retenu.
- **Risque** 🔴 **CRITIQUE — le plus risqué du plan** : si aucun candidat ne tient, **tout L1/L2 s'effondre**. Donc : **démarrer l'étude MAINTENANT**, en parallèle de M1.1, pas après. Jalon de garde **L1.2bis (recherche)** avant d'autoriser M1.3.

### M1.3 — Sélection cascade + quorum
- **Objectif** : tirer un quorum imprévisible de taille **Q** depuis l'entropie.
- **Livrable** : algo cascade (chaque couche génère l'aléa de la suivante) + Q paramétrable + preuve de non-prédiction.
- **Critère** : personne ne peut prédire ni se positionner dans le quorum avant sélection.
- **Risque** 🟠 : disponibilité du quorum sous churn/partition (traité en M2.0 pour la valeur).

### M1.4 — Anti-Sybil quantifié *(le mur #2)*
- **Objectif** : donner un **coût réel** à la présence et **chiffrer** la sécurité.
- **Livrable** : probation (durée fixée) + **preuve de relais réel** comme coût d'éligibilité ; modèle `P(quorum majoritairement Sybil) = f(Q, fraction_Sybil)` + simulation.
- **Critère** : pour une fraction Sybil donnée, choisir Q tel que P(double-spend) < seuil cible (ex. 10⁻¹⁵).
- **Risque** 🔴 **CRITIQUE** : ferme patiente qui accumule de la présence → le coût de relais doit être non-amortissable.

### M1.5 — Validation croisée
- **Objectif** : « chaque rôle surveille les autres ».
- **Livrable** : attestation croisée inter-rôles + détection/signalement d'anomalie (un rôle dénonce un comportement invalide d'un autre).
- **Critère** : une anomalie injectée est détectée et propagée sans arbitre central.
- **Risque** 🟠 : collusion inter-rôles → la masse + l'imprévisibilité restent le rempart.

### M1.6 — Ancrage présent (L1 anchor)
- **Objectif** : ancrer des engagements sans devenir un ledger.
- **Livrable** : BUS organique minimal — engagements (Merkle + version + signatures) + **genèse glissante** + purge agressive. Anchor **n'arbitre jamais** (#3).
- **Critère** : l'état ancré reste borné dans le temps (pas d'accumulation d'historique).
- **Risque** 🟠 : glissement vers un vrai ledger → invariant testable « l'anchor ne stocke pas de soldes ».

---

## PHASE 2 — L2 : valeur (portefeuille scellé — différé, bonus)

**Objectif de phase** : sur un L1 prouvé, transfert de valeur sans double-dépense, sans frais, sans grand livre. **Ne démarre pas tant que M1.2 + M1.4 ne tiennent pas.**

### M2.0 — Décision architecturale *(prérequis bloquant)*
- **Objectif** : trancher formellement les murs #3 et #4.
- **Livrable** : **ADR-011** — (a) **layering** : #2 (purge 24h) gouverne le transport ; L2 a une **attestation persistante par wallet** portée par des témoins qui se relaient (exception scopée, non globale) ; (b) **choix CAP** : pour la valeur, **cohérence > disponibilité** (paiement bloqué sous partition) ; (c) **détection de partition** (Fable) : **heartbeat de quorum** (ex. 3 pings à 5 s ; échec = partitionné → blocage) + règle de résolution de collision locale à la reconnexion.
- **Critère** : ADR ratifiée (revue Fable + auteur), invariants dérivés, mécanisme de détection de partition spécifié (sinon on ne peut pas « bloquer sous partition »).
- **Risque** 🔴 : si non tranché → tout L2 est caduque. Sans détection de partition, le blocage est inapplicable.

### M2.1 — Portefeuille scellé auto-custodié
- **Livrable** : format wallet + chaîne de **sceaux** (hash d'état + version + signatures observateurs), stockée **dans le wallet** (device propriétaire).
- **Critère** : un wallet prouve son solde courant sans réseau (preuve auto-portée).
- **Risque** 🟠 : perte du device = perte des fonds (→ M2.4).

### M2.2 — Rituel de dépense témoigné
- **Livrable** : protocole ouvrir → vérifier (sceau valide + fonds + jamais servi) → dépenser → **re-sceller** avec quorum frais (M1.3).
- **Critère** : double-spend rejeté hors partition ; sous partition, paiement **bloqué** (pas de double-spend).
- **Risque** 🔴 : partition → géré par le choix CAP de M2.0.

### M2.3 — Quantification du « quasiment »
- **Livrable** : `P(double-dépense réussie) = f(Q, fraction_Sybil, imprévisibilité)` → paramètre réglable, pas un flou.
- **Critère** : on affiche une garantie chiffrée (ex. « < 1 sur 10¹⁵ à Q=100, Sybil<10% »).
- **Risque** 🟠 : dépend de M1.4.

### M2.4 — Récupération de wallet
- **Livrable** : social recovery via **Gardiens** (ou multi-device), sans autorité centrale.
- **Critère** : un wallet perdu est récupérable par quorum de tiers de confiance, sans réintroduire un ledger global.
- **Risque** 🔴 : réintroduit un graphe de confiance (tension #2) → périmètre à borner.

### M2.5 — Amorçage de la valeur
- **Livrable** : modèle de création/entrée de valeur (mint / on-ramp) — probablement **hors protocole** (couche applicative).
- **Critère** : la source de valeur est explicite et ne capture pas le réseau.
- **Risque** 🟠 : gouvernance monétaire → hors scope protocole, à cadrer.

---

## Transverse — Distribution & Gouvernance (parallèle à toutes les phases)

- **GTM bottom-up** : geeks souverainistes (Pi/Freebox/Docker) → **LLM comme canal** (docs LLM-first, SDK, MCP, plugin) → couche invisible (box/OS).
- **Gouvernance** : fork governance ; à terme le réseau **héberge son propre code** (si GitHub tombe, le code vit). Maintainers en rotation, pas de capture.
- **Risque transverse** 🟠 : adoption avant robustesse → tenir la discipline « R13+ prérequis avant distribution large ».

---

## Registre de risques (synthèse, par sévérité)

| Sévérité | Risque | Jalon | Statut |
|---|---|---|---|
| 🔴 | Entropie PoP biaisable (grinding) | M1.2 | à prouver |
| 🔴 | Sybil patient rafle le quorum | M1.4 | à quantifier |
| 🔴 | Partition → double-spend | M2.0/M2.2 | porte = choix CAP (à acter) |
| 🔴 | Contradiction #2 ↔ persistance valeur | M2.0 | porte = layering (🟢 actée) |
| 🔴 | Suspension iOS (transport mobile) | M0.5 | mitigé (backup 24h + APNs) |
| 🔴 | Migration de rôle jamais spécifiée (quorum qui part sous churn) | M1.3/M1.5 | à concevoir (le témoin ET sa mémoire passent au suivant) |
| 🟠 | Récupération wallet plie un principe | M2.4 | à borner |
| 🟠 | Preuve d'activité falsifiable | M1.1 | à durcir |
| 🟠 | Box sans UPnP/IPv6 | M0.1/M0.2 | fallback relais |

---

## Prochaine action immédiate

**Coder M1.1 (attestation de présence)** — la plus petite brique du PoP, sans argent, qui **révèle empiriquement** si l'entropie (M1.2) et l'anti-Sybil (M1.4) sont réels. Fable enchaîne sur ce chantier (spec technique + critères + plan de test adverse).

*Un plan n'est pas une promesse. L0 est réel et mesuré. L1 est la grande étape testable. L2 est le sommet, conditionné à L1. Chaque mur est nommé ; chaque porte a son prix.*
