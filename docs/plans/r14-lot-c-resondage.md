# R14 Lot C — Re-sondage des candidats inactifs (design, AVANT code)

> Créé le 2026-07-19 (soir), à partir du verdict Lot B (`r14-ipv6-first-class.md` §2.4,
> mémoire `tom-lotb-verdict-failover-sans-resondage`). **Aucun code écrit.**
> À red-teamer + arbitrage Malik avant implémentation.

## §1 Le problème exact (mesuré, pas supposé)

Après un failover (mort du chemin actif), le système reste sur le chemin survivant même
s'il est nettement plus lent que l'ancien chemin redevenu disponible : iPad→Mac resté sur
v6 18 ms pendant ≥ 25 min avec un v4 à 7 ms vivant (gain 11 ms >> hystérésis 5 ms).
Cause : **un chemin non-actif n'est plus sondé** → pas de RTT frais → `select_v4_v6`
(`remote_state.rs:1160-1223`) n'a aucun candidat à comparer. L'hystérésis et l'avantage
v6 fonctionnent — ils n'ont juste plus de matière.

## §2 Ce que le Lot C n'est PAS

- PAS le tri déterministe du probe initial (`iroh_hp.rs`, FxHashMap) : l'aléatoire du
  premier choix est réel mais ne cause pas la non-convergence mesurée — et
  `iroh_hp.rs:196` documente qu'un mécanisme s'appuie sur cet aléatoire. On n'y touche pas.
- PAS un « ping toutes les X ms sur tous les candidats » : le multipath QUIC sonde déjà
  les chemins de l'ensemble ACTIF (§2.3bis : sondes 8 octets vues au tcpdump). Le trou ne
  concerne que les candidats SORTIS de l'ensemble actif (morts, jamais validés).

## §3 Proposition (à challenger)

**Re-probe paresseux et borné des candidats connus non-actifs**, dans tom-connect
(`remote_state`), déclenché par une condition de « suspicion de sous-optimalité » plutôt
qu'un timer aveugle :

1. **Déclencheur** : à chaque bascule ENREGISTRÉE comme failover (le nouveau chemin est
   plus lent que l'ancien : `new_rtt > old_rtt`), armer un re-probe du chemin perdu à
   T+30 s, puis backoff ×2 (30 s → 1 min → 2 min → 5 min, cap 5 min), abandon après ~6
   tentatives OU dès que le chemin redevient actif.
2. **Portée** : uniquement les adresses déjà CONNUES du remote (candidats existants de la
   connexion) — jamais de nouvelle découverte, jamais de dial de pair (c'est un
   path-probe QUIC intra-connexion, pas un `get_or_connect`).
3. **Décision** : le re-probe réussi réinjecte un RTT frais → la logique EXISTANTE
   (`select_v4_v6` + `RTT_SWITCHING_MIN_IP` 5 ms + `IPV6_RTT_ADVANTAGE` 3 ms) décide.
   Aucun nouveau seuil de sélection.
4. **Budget** : ≤ 1 re-probe en vol par connexion, sondes = trames de validation QUIC
   (~8-40 octets). Coût plafonné : 6 sondes / chemin perdu / cycle de backoff.

## §4 Risques à red-teamer

- **Oscillation** : chemin qui meurt/revit vite (le cas iPad, cycle ~5-15 min) → le
  re-probe pourrait re-basculer vers un chemin instable. Mitigation candidate : mémoire
  courte de fiabilité par candidat (n morts récentes → exiger un gain > hystérésis
  majorée, p.ex. 2×5 ms) — attention décision LOCKED #4 (fade, pas de ban).
- **Interférence avec le NAT traversal** (`continue_nat_traversal_round`,
  `iroh_hp.rs:196`) : le re-probe ne doit pas relancer de round complet de hole punch.
- **Batterie/veille iOS** : des sondes périodiques peuvent réveiller la radio — le
  backoff cap 5 min et l'abandon après 6 essais bornent ça ; à mesurer sur appareil.
- **Amplification** : sondes vers une adresse morte (peer parti) = trafic vers un tiers
  potentiel (DHCP réattribué). Borné par l'abandon + intra-connexion uniquement (la
  connexion meurt avec le pair).

## §5 Question amont — TRANCHÉE par décision produit (Malik, 19/07 soir)

**On ne creuse pas, on assume.** iOS restreint le réseau même en foreground sans
activité tactile (surtout iPad — la batterie est un point fort du produit). Décision,
extension du non-objectif APNs/background du 15/07 : **ToM n'exploite pas une machine
mobile au-delà de ce que son utilisateur utilise réellement du réseau.** En prod, l'app
mobile est ouverte pendant l'usage réel (envoi/réception/lecture) — pas laissée en
foreground des heures comme au labo.

Conséquences pour ce design :
- Le churn de chemins d'un mobile inactif = comportement ATTENDU, plus une anomalie.
  Aucun keepalive/anti-power-save ne sera ajouté pour le contrer.
- Le Lot C est RECADRÉ, pas invalidé : sa valeur est la **re-convergence des nœuds
  actifs** (NAS, desktop, mobile en usage réel) après un failover — dont le retour au
  meilleur chemin quand l'utilisateur REVIENT dans l'app et que ses chemins revivent.
- Les mesures I9a/I9b doivent séparer les mobiles inactifs de la flotte active, sinon
  elles mesurent le power save d'Apple, pas notre protocole.

## §6 Validation prévue

- Étage L : test hermétique loopback (modèle r15_relay_cache) : tuer artificiellement un
  chemin (drop socket v6), vérifier failover PUIS retour ≤ 60 s quand le chemin revit.
- Étage F : rejouer la fenêtre Mac↔iPad avec `path-matrix.py` + logger : I9b (retour au
  meilleur ≤ 60 s après renaissance), I9a (pas d'oscillation accrue).
- Invariants : aucune bascule ne doit plus durer > backoff-cap quand un chemin 5 ms
  meilleur est vivant.
