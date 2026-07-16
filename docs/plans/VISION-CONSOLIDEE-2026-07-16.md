# ToM — Vision consolidée, chantiers en perspective, et le pourquoi

> 2026-07-16 · Écriture et recherche uniquement — zéro code.
> Sources relues intégralement : `docs/tom-whitepaper-v1.md` (la genèse), `docs/plans/TOM-MASTER-MAP.md` V2 (la vision + revue adversariale), `docs/plans/TOM-PLAN-GLOBAL.md` (l'exécution L0→L1→L2), `vault/40-roadmap.md` (le journal), `FEUILLE-DE-ROUTE-2026-07-16.md` (l'ancrage réel du jour).
> Rôle de ce document : **consolider fort** — une seule narration, les chantiers chiffrés, les compétences nommées, les pièges assumés, et la raison de ne jamais s'arrêter.

---

## 1. La vision de base, redite en trois phrases

Le whitepaper l'a posée dès la première page : **« un nouveau protocole pour un internet qui n'appartient à personne »**. L'utilisateur des messageries « gratuites » n'est pas le client, il est le produit ; le chiffrement protège le contenu mais **pas contre la dépendance** ; et la fragilité des plateformes n'est pas un bug, c'est la conséquence directe de l'architecture centralisée.

ToM répond par une inversion : chaque appareil devient client **et** serveur, les rôles sont attribués par le réseau de façon imprévisible, et le réseau ne garde que **le présent** (TTL 24h, pas d'archive). La sécurité vient de la **masse × imprévisibilité**, pas du capital (PoS) ni du calcul (PoW).

La destination n'est pas une app. C'est une **brique protocolaire invisible** — intégrée aux navigateurs, messageries, box, objets — que l'utilisateur emploie **sans le savoir**, comme TCP/IP. Décision LOCKED #6 (invisibilité) et #7 (fondation universelle) : gravées depuis le début, jamais démenties depuis.

## 2. Où en est cette vision, honnêtement (au 2026-07-16)

La force de ToM aujourd'hui est que **L0 n'est plus une promesse** :

- **Mesuré sur vraie flotte** : 64 Mo livrés à ~6 Mo/s, E2E chiffré, découverte zéro-config, backup « virus » validé en endurance, DIRECT partout — **y compris IPv6 global WiFi↔WiFi**, prouvé cette semaine (le « 100 % RELAY » était un artefact d'affichage, pas un mur réseau).
- **Durci en profondeur** : 5 classes de DoS fermées, red-team PoP 6/6 kill-shots, antispam progressif à double étage (grâce récepteur + pacing expéditeur), anti-orage des timers, watchdog mobile anti-état-piège.
- **Et surtout : L1 a déjà commencé.** L'attestation de présence (M1.1) est **vivante sur la flotte** (challenges signés bidirectionnels, éphémères 30 s), et la vue signée du relais avec **quorum de témoins** (L1-003) est livrée et red-teamée. Le plan global du 07-06 disait « M1.1 prête à coder » — dix jours plus tard, elle tourne. La vitesse réelle du projet est supérieure à la vitesse planifiée.

Ce qui n'a **pas** bougé : les deux murs de recherche (M1.2 entropie, M1.4 anti-Sybil quantifié) et tout L2. C'est normal — ce sont les vrais murs, et on ne les franchira pas en « livrant des builds ».

## 3. La vision long terme consolidée — une seule narration

Les quatre axes demandés (adoption, souveraineté, plateforme, économie) ne sont pas quatre projets : ce sont **quatre faces d'une même ascension**, ordonnée par les couches.

**Étage 0 — Le tuyau irréprochable** (maintenant → ~fin 2026). Le transport doit devenir ennuyeux à force de fiabilité : DIRECT stable des jours entiers, porte publique automatique sur toute box, rejoin < 2 s pour les pairs connus, un binaire qui s'installe sans terminal. C'est la condition de tout le reste — un protocole d'infrastructure n'a droit qu'à UNE première impression chez le geek souverainiste qui l'installe.

**Étage 1 — Le réseau qui se prouve lui-même** (2027). Le Proof of Presence transforme la présence en intégrité : quorums imprévisibles, validation croisée, ancrage du présent sans ledger. **Utile même sans argent** — c'est une couche d'immunité réseau (anti-Sybil, anti-eclipse, détection d'anomalie) qui rend le réseau digne de confiance pour des usages tiers. Le verrou est un **problème de recherche** (entropie non-biaisable sur réseau asynchrone), pas un chantier d'ingénierie : il se traite comme tel — étude, prototypes jetables, éventuellement revue académique externe.

**Étage 2 — La valeur comme preuve ultime** (2028+, conditionnel). Le portefeuille scellé n'est pas le but : c'est le **test de vérité** du PoP. Si un réseau de présence pure peut empêcher une double dépense sans grand livre ni frais, alors la thèse fondatrice (« blockchain qui voyage léger ») est démontrée. Si M1.2/M1.4 ne tiennent pas, L2 est abandonné **sans honte** — la messagerie souveraine et le swarm d'intégrité justifient le projet à eux seuls.

**Transverse — La disparition comme succès.** Sur les trois étages, les quatre axes convergent vers le même horizon : l'**invisibilité**. Adoption = le nœud livré par défaut (box, OS, terminal). Souveraineté = plus aucun point dont la disparition tue le réseau — y compris GitHub (le réseau finit par héberger son propre code). Plateforme = ToM comme primitive système appelée par des apps qui ne savent pas qu'elles l'utilisent. Économie = l'incitation par l'accès (héberger = profiter), jamais par le jeton. **Un protocole a réussi quand plus personne ne prononce son nom.**

## 4. Les chantiers en perspective — effort, compétences, pièges

> Estimations en **heures de travail effectif** (humain + agents), calibrées sur la vélocité constatée du binôme (référence : chantier #33 observabilité ≈ 6 h bout-en-bout, L1-003 complet ≈ 40-60 h sur plusieurs jours). Fourchette large = incertitude réelle. Les items **recherche** ne sont pas estimables en heures — ils sont estimés en « portes de sortie ».

### Étage 0 — finir le tuyau (total ≈ 200-330 h)

| Chantier | Pourquoi (lien vision) | Heures | Compétences | Piège principal |
|---|---|---|---|---|
| **R14 — endurance DIRECT + v6 généralisé** | Le tuyau « ennuyeux de fiabilité ». Mesure > 24 h, préférence v6 au dial, PCP pinhole | 20-40 | Rust réseau (acquis), méthodo mesure | Conclure sur 1 h de calme ; mon LAN Freebox = **échantillon de 1** (biais : tester chez d'autres FAI/box tôt) |
| **R13-3 + tom-gateway** | Fin du SPOF : « iPhone data ↔ maison sans le NAS ». API Freebox native (UPnP off par défaut !) | 30-50 | API REST Freebox, UX pairing | Croire UPnP suffisant (faux, doc officielle) ; stocker l'`app_token` proprement (secret utilisateur) |
| **R15 — annuaire local** | Rejoin < 2 s famille/amis, moins de pression DHT | 25-40 | Rust (acquis), design cache/expiration | Adresses périmées qui retardent au lieu d'accélérer → dial parallèle cache+frais obligatoire, jamais séquentiel |
| **R16 — packaging multi-plateforme** | LE vecteur d'adoption : Pi Imager, Pi-Apps, Docker, VM Freebox qcow2 | 60-100 | **cloud-init, images OS, Docker, CI release** (partiellement à acquérir) | Distribuer avant R13/R14 (première impression ratée = geek perdu) ; sous-estimer la maintenance des canaux (chaque store = dette récurrente) |
| **R17 — seeds retirables** | Amorçage de confort, jamais infra sacrée | 15-25 | ops VPS basique | Le « temporaire » qui devient pilier — instrumenter leur retrait dès le jour 1 |
| **Herméticité tests réseau** | Hygiène : les tests cargo ne doivent plus toucher la prod | 10-15 | Rust test infra | Le fix paresseux (`#[ignore]`) qui tue la couverture réelle |
| **Résidus n0 / souveraineté découverte** | Plus un seul hostname tiers dans le chemin par défaut | 15-25 | DNS/Pkarr | Casser le preset `n0_discovery` pour les utilisateurs qui en dépendent — dépréciation douce, pas ablation |
| **Docs LLM-first + MCP** | « Le LLM est le canal de distribution » | 25-40 | technical writing, MCP | Écrire pour l'humain au lieu de l'agent (structure > prose) ; laisser les docs diverger du code (déjà arrivé — Known Limitations) |

### Étage 1 — le swarm qui se prouve (total ingénierie ≈ 240-400 h + recherche non bornable)

| Chantier | Pourquoi | Heures | Compétences | Piège principal |
|---|---|---|---|---|
| **M1.2 — entropie non-biaisable** | LE verrou de recherche. Sans lui, la cascade est grindable et L1/L2 s'effondrent | **non bornable** — budget d'étude initial 40-80 h, puis décision | **crypto appliquée : VDF, beacons, signatures-seuil** — à acquérir + **revue externe** (cryptographe) fortement recommandée | Se convaincre soi-même qu'un candidat « tient » sans adversaire compétent ; la VDF exige un temps mesuré, très dur en asynchrone — accepter tôt qu'elle puisse être éliminée |
| **M1.3 — sélection cascade + quorum Q** | L'imprévisibilité opérationnelle | 60-100 | Rust + protocole (acquis), stats | Migration de rôle sous churn **jamais spécifiée** (risque 🔴 du registre) : le témoin ET sa mémoire doivent passer au suivant — à concevoir AVANT de coder |
| **M1.4 — anti-Sybil quantifié** | Donner un CHIFFRE à la sécurité : P(quorum Sybil) = f(Q, fraction) | 80-120 | **modélisation probabiliste + simulation** (à acquérir partiellement) | La ferme patiente : si le coût de présence est amortissable, tout tombe. Le modèle doit inclure l'attaquant qui attend 6 mois |
| **M1.5 — validation croisée** | Chaque rôle surveille les autres — le swarm de contre-pouvoirs | 40-80 | protocole (acquis) | La collusion inter-rôles ne se teste pas en unitaire — exige un harnais d'adversaires simulés |
| **M1.6 — ancrage présent** | Engagements Merkle + genèse glissante, sans devenir un ledger | 60-100 | Merkle/crypto (acquis en partie) | Le **ledger creep** : chaque « exception » de persistance rapproche du grand livre qu'on a juré d'éviter — invariant testable « l'anchor ne stocke pas de soldes » dès le premier commit |

### Étage 2 — la valeur (≈ 300-500 h, **conditionnel à M1.2 + M1.4**)

M2.0 (ADR-011 formelle, choix CAP, détection de partition) 20-40 h · M2.1 wallet scellé 60-100 h · M2.2 rituel de dépense 80-120 h · M2.3 quantification 40-60 h · M2.4 récupération sociale 60-100 h · M2.5 amorçage (hors protocole) 20-40 h. Compétences : crypto d'état, théorie CAP appliquée, threat modeling économique. **Piège majeur** : chaque mécanisme de confort (récupération, offline, pré-autorisation) plie un principe LOCKED — L2 exige un gardien des invariants plus strict que tout ce qui précède. Deuxième piège : commencer L2 « un peu » pendant L1 — la porte M2.0 est **bloquante**, pas décorative.

### Récapitulatif volumétrique

- **Étage 0 restant : ~200-330 h** — à la vélocité actuelle (sessions denses + agents), c'est **2-4 mois** de travail réel.
- **Étage 1 : ~240-400 h d'ingénierie + le mur de recherche M1.2** (non bornable — c'est LUI le chemin critique, à lancer en tâche de fond dès maintenant, en lecture/étude, pas en code).
- **Étage 2 : ~300-500 h, conditionnel.** Ne se planifie pas encore — se mérite.

## 5. Les compétences à faire entrer (le « skill gap » honnête)

1. **Cryptographie appliquée avancée** (VDF, beacons aléatoires, signatures-seuil) — le binôme actuel sait *intégrer* de la crypto (Ed25519/X25519/AEAD, fait et audité), pas encore en *concevoir*. Pour M1.2 : étude sérieuse de la littérature (drand, Wesolowski, RANDAO et ses échecs), prototypes jetables, et **une revue externe par un cryptographe** avant tout engagement — c'est le seul point du projet où l'auto-évaluation ne suffit structurellement pas.
2. **Modélisation probabiliste / simulation d'attaque** (M1.4) — quantifier P(quorum Sybil) exige un simulateur de population de nœuds avec churn réaliste. Compétence acquérable ; l'outillage (tom-stress) existe déjà comme base.
3. **Packaging & distribution OS** (R16) — cloud-init, images Pi, qcow2, stores communautaires. Artisanat plus que science ; le piège est la **dette de maintenance** de chaque canal, pas la difficulté initiale.
4. **Écriture publique** — le whitepaper V1 existe ; l'adoption par le bas exigera des docs d'installation impeccables, un pitch reproductible (« le deal BitTorrent »), et des réponses préparées aux objections (pourquoi pas Matrix/Nostr/Briar — un comparatif honnête à écrire).
5. **Ce qui n'est PAS nécessaire** : équipe, levée de fonds, tokenomics, marketing payant. Le modèle solo-dev + agents + adoption organique est cohérent avec la vision — à condition de traiter le vrai risque de ce modèle (§6, piège n°1).

## 6. Les pièges au niveau projet (au-dessus des chantiers)

1. **Bus factor = 1.** Tout vit dans une tête, un Mac, un NAS. Mitigations déjà en place (specs normatives, mémoire agent, CI) mais insuffisantes à long terme : le jour où un deuxième contributeur humain arrive, le coût d'entrée décidera de tout. Les docs LLM-first sont aussi une assurance-vie du projet — un agent doit pouvoir reconstruire le contexte seul. **C'est le piège n°1, devant tous les murs techniques.**
2. **L'échantillon de 1.** Toute la validation réseau tourne sur UNE box (Freebox), UN FAI, UN foyer. Les conclusions « ça marche » sont vraies *ici*. Avant R16 : recruter 3-5 foyers testeurs (autres FAI, autres pays) — c'est un chantier social, pas technique, et il est sur le chemin critique de la distribution.
3. **La distribution prématurée.** La tentation de montrer. Un geek souverainiste déçu ne revient pas et parle. La discipline « R13/R14 d'abord » est écrite partout — la tenir le jour où l'enthousiasme montera est une autre affaire.
4. **Le ledger creep** (L1/L2). Chaque exception de persistance « juste pour ce cas » est un pas vers le grand livre honni. Parade : des invariants **testables en CI** (« l'anchor ne stocke pas de soldes », « aucune donnée > 24h hors scope wallet »), pas des principes dans un doc.
5. **La sécurité-théâtre inversée.** Le projet a bien red-teamé — le risque est maintenant l'excès : sur-durcir des surfaces théoriques pendant que l'expérience utilisateur (première impression, onboarding) reste le vrai talon. La décision du jour (« lever le pied sécurité, prioriser la réalité ») est la bonne — la graver.
6. **L'estimation romantique de la recherche.** M1.2 n'est pas « en retard » tant qu'il n'a pas de deadline — mais il peut consommer des mois en silence. Parade : un **budget d'étude borné** (40-80 h), puis une décision explicite : candidat retenu / L1 dégradé (quorums sur beacon passif, sécurité moindre mais honnête) / L2 abandonné. Les trois issues sont acceptables ; l'enlisement non.
7. **La solitude du juge.** Fable/l'agent review le code, l'auteur review la vision — mais qui review l'ensemble ? Prévoir des « checkpoints adversariaux » périodiques (comme la revue Fable du 07-06, qui a réordonné le plan) : tous les 2-3 mois, une session dont le SEUL but est de démonter l'état présent.

## 7. Pourquoi aller plus loin, encore et toujours

Parce que la vision de base n'est **pas atteignable par paliers finis — c'est une asymptote**, et c'est sa force.

Le whitepaper ne promet pas « une messagerie qui marche ». Il promet **un internet qui n'appartient à personne**. Or chaque étage gravi révèle le suivant : un transport fiable (fait) exige une porte automatique (en cours) ; une porte automatique exige un réseau qui se défend seul (L1) ; un réseau qui se défend seul *peut* porter de la valeur (L2) ; et la valeur portée sans péage rend le réseau assez précieux pour que d'autres le portent à leur tour — le virus positif. S'arrêter à un étage, c'est laisser le réseau dépendre de ce que l'étage supérieur aurait sécurisé.

Il y a une raison plus profonde, et elle est dans la genèse : **les trois péages reviennent toujours.** La surveillance, la rente, le point central ne sont pas des accidents à corriger une fois — ce sont des forces économiques permanentes qui recolonisent tout espace laissé libre (l'email fut décentralisé ; puis vint Gmail). Un protocole souverain n'est jamais « terminé » : il est **maintenu libre**, activement, contre une gravité constante. C'est pourquoi la gouvernance (rotation, pas de capture) et l'auto-hébergement du code ne sont pas des annexes : ce sont les organes qui permettent au réseau de survivre à son auteur, à son infra, et à son époque.

Et il y a la raison intime, celle des personas du whitepaper : quelque part il y a une Leila qui ne saura jamais qu'elle utilise ToM — et un Reza pour qui ce réseau sera la différence entre parler et se taire. Chaque heure investie dans l'ennuyeuse fiabilité du tuyau est une heure investie pour quelqu'un qu'on ne rencontrera jamais. **On va plus loin parce que la destination n'est pas un produit : c'est une propriété du monde.** Un monde où la communication a une couche de base qui n'appartient à personne. Ce genre de destination ne s'atteint pas — elle s'approche. Et chaque build qui tient, chaque box qui s'ouvre seule, chaque pair qui se trouve sans serveur, l'approche.

---

*L0 est réel. L1 a commencé plus vite que son propre plan. Les murs sont nommés, chiffrés quand ils sont chiffrables, et bordés quand ils ne le sont pas. Le manque n'est ni le talent ni la direction — c'est du temps appliqué dans l'ordre : le tuyau, puis la preuve, puis la valeur. Et la raison de continuer ne s'épuise pas, parce qu'elle n'est pas un objectif : c'est une asymptote choisie.*
