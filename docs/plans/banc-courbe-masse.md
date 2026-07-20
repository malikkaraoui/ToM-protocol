# Banc « courbe de masse » — prouver l'anti-cyclicité par la mesure

> Compagnon de `charte-cibles-agressives.md` (§0 deux régimes, §4 gap super-additif).
> Design v1 (2026-07-20), issu de la revue « avocat du diable ». **À valider avec Malik
> avant toute exécution** — zéro code tant que le design n'est pas ratifié.

---

## §0 Objectif et critères PRÉ-ENREGISTRÉS

Produire les premières **courbes falsifiables** de la thèse : à **charge par nœud fixe**,
quand N croît —

1. **Courbe capacité** : le débit *livré par nœud* ne s'effondre pas (plat ou mieux).
2. **Courbe latence** : la latence de livraison 1-à-1 (p50/p95) reste bornée
   (charte §0 : au pire sous-logarithmique pour le broadcast, stable pour le 1-à-1).

**Critères fixés AVANT la mesure** (anti-biais — on ne déplace pas les poteaux après) :

| Verdict | Condition (drafts à ratifier) |
|---|---|
| PASS | débit/nœud à N_max ≥ 80 % de celui à N_min ET latence p95 ≤ 2× celle à N_min |
| FAIL | l'un des deux seuils est franchi sur ≥ 2 runs consécutifs |
| INVALIDE | garde-fou contention déclenché (§3) → le point n'existe pas, on ne conclut RIEN |

Un FAIL n'est pas une honte : c'est un **bug architectural localisé** (charte §0), donc un
chantier. Un point INVALIDE publié comme PASS serait, lui, une faute.

## §1 Les 4 pièges que ce design évite (leçons projet + revue adverse)

1. **Contention du banc** (fatal, déjà vécu : « dégradation mesh = contention de mes
   builds », pool-lock 74 s). 100 nœuds sur 1 Mac mesure l'ordonnanceur, pas le réseau.
   → densité par hôte bornée + garde-fou CPU (§3), sinon point INVALIDE.
2. **Compteurs menteurs** (vécu : indicateurs verts sur des proxies pendant des semaines ;
   `messages_recus=0` silencieux ; NAS « connecte » avec 0 pair). → Phase 0 d'audit
   d'instrumentation, BLOQUANTE.
3. **Métrique tautologique** (revue adverse) : « le débit agrégé monte » ne falsifie rien.
   → toutes les métriques sont **par nœud, à charge par nœud fixe**.
4. **LAN ≠ WAN** (validité externe) : un LAN à RTT 2 ms ne dit rien d'un déploiement réel.
   → Phase 2 en WAN simulé (netem), et le rapport sépare les deux mondes sans extrapoler.

## §2 Phase 0 — Audit d'instrumentation (BLOQUANTE)

Avant tout point de courbe, prouver que ce qu'on compte est vrai :

- **Canari comptage** : injecter K messages connus A→B ; vérifier compteur émetteur = K,
  compteur récepteur = K, chacun compté UNE fois (pas de double-comptage à la reconnexion).
- **Latence sans horloge partagée** : mesurer en **aller-retour applicatif** (A→B→écho→A,
  latence = RTT/2) — jamais de timestamps croisés entre machines (horloges non synchro).
- **Vérité terrain croisée** : compteurs :9091/:8085 vs logs collecteur UDP vs comptage
  côté harnais — les trois doivent converger sur le canari.
- **Charge de fond nulle** : `ps aux` avant chaque run, kill des process de stress résiduels
  (leçon : un cas a tourné 2 jours), aucun build en parallèle pendant les mesures.

Sortie : une note « instrumentation auditée le X, écarts trouvés/corrigés ». Sans elle,
aucune courbe n'est publiable.

## §3 Phase 1 — Courbe LAN multi-machines

- **Hôtes physiques** : Mac, NAS (VM Debian), iPad, iPhone, Apple TV = 5 hôtes.
- **Points de courbe** : N = 5 (1 nœud/hôte) → 10 → 20. Les nœuds supplémentaires sont des
  process headless (tom-tui --bot ou binaire stress) sur Mac + NAS UNIQUEMENT (pas sur les
  devices Apple mobiles).
- **Garde-fou contention (non négociable)** : CPU et loadavg échantillonnés sur chaque hôte
  pendant le run. Si un hôte dépasse **70 % CPU soutenu** (ou loadavg > cœurs), le point est
  **INVALIDE** → réduire la densité, pas la vigilance. C'est le plafond de validité du banc
  local : au-delà de ~20-24 nœuds, il FAUT du matériel distribué (hors scope v1).
- **Charge fixe par nœud** : 1 msg/s, payload 1 Ko, destinataires tirés uniformément parmi
  les N-1 autres. Durée : 15 min/point, 2 runs/point.
- **Mesures par point** : débit livré/nœud, latence écho p50/p95, taux de livraison,
  RssAnon/nœud (l'axe stockage de la charte exige qu'il reste constant), CPU/hôte
  (validité), part du trafic relayé par nœud (→ métrique de **concentration**, charte §4).

## §4 Phase 2 — Courbe WAN-simulée + churn

- **Dégradation réseau** : `tc netem` sur la VM NAS (root dispo) : RTT 40-80 ms, jitter
  10 ms, perte 1 % sur l'interface. Effet : tous les chemins passant par le NAS deviennent
  « WAN ». Piste complémentaire côté Mac : dummynet (`dnctl`/`pfctl`) — à valider avant
  d'en dépendre.
- **Churn contrôlé** : l'orchestrateur étage F (runbook) kill/relance une fraction des
  nœuds headless (10 %/5 min) pendant la mesure — le monde réel n'est pas statique, et la
  charte exige le churn absorbé.
- Mêmes points N, mêmes métriques, mêmes garde-fous que Phase 1. Les courbes LAN et WAN ne
  se mélangent JAMAIS dans un même graphe sans étiquette.

## §5 Phase 3 — Rapport honnête

- Les 2 courbes × 2 mondes (LAN/WAN), avec les points INVALIDES montrés comme tels.
- **Limites d'extrapolation explicites** : N max testé, matériel semi-homogène, un seul
  site physique ; AUCUNE conclusion au-delà de N mesuré, pente ≠ preuve à 10 000.
- La distribution de concentration (part de l'infra par décile) — première photo du
  « nœud-équivalent » réel de la flotte.
- Le rapport devient une annexe de la charte et alimente le tableau de bord juge (§3.2).

## §6 Ce que le banc ne prétend PAS

- Prouver 10 000 nœuds (il mesure une PENTE à petit N, c'est tout).
- Mesurer l'in-censurabilité, la sécurité, ou le hole-punch cross-NAT réel (autres bancs).
- Remplacer le terrain : la flotte réelle (I9/I10) reste le juge de paix des reconnexions.

## §7 Réutilisation de l'existant (pas de réécriture)

- `tom-stress` : campagnes Mac↔NAS existantes (250/250) — base des émetteurs de charge.
- Orchestrateur étage F + runbook (`docs/plans/RUNBOOK-TESTS.md`) : lancement/kill de
  flottes headless, déjà exercé sur un mesh de 8 nœuds.
- Collecteur UDP :9999 + status :9091/:8085 : télémétrie en place (auditée en Phase 0).
- Harnais étage L : inchangé, hors scope ici.

---

> Prochaine étape si ratifié : Phase 0 seule (audit instrumentation), livrable = la note
> d'audit. Aucun point de courbe avant ça.
