The Open Messaging : 10 jours qui ont transformé une idée en protocole — du code TypeScript au hole punch Suisse↔France

Il y a des dépôts GitHub qui cherchent à rassurer.

Et il y a ceux qui cherchent à prouver.

Le repo ToM Protocol (The Open Messaging) fait partie de la deuxième catégorie : pas un "produit", pas une "plateforme", pas un token en attente de narration. Un protocole de transport P2P qui veut devenir aussi banal qu'un tuyau — et qui le montre déjà, code à l'appui.

Ce qui suit n'est pas une histoire racontée "de l'extérieur". C'est un état des lieux construit à partir des fichiers .md du dépôt (vision, PRD, décisions verrouillées, rétro, backlog, changelog), et recoupé avec l'activité Git des 10 derniers jours.

Repo GitHub : https://github.com/malikkaraoui/ToM-protocol

Documents de référence dans le repo :
- Whitepaper v1 : tom-whitepaper-v1.md
- PRD : _bmad-output/planning-artifacts/prd.md
- Architecture / ADRs : _bmad-output/planning-artifacts/architecture.md
- Décisions verrouillées ("7 locks") : _bmad-output/planning-artifacts/design-decisions.md
- Rétro + consolidation : _bmad-output/implementation-artifacts/epic-4-8-retro-2026-02-07.md
- Backlog d'issues (source) : .github/ISSUE_BACKLOG.md
- Changelog : CHANGELOG.md
- Docs LLM-first : llms.txt, CLAUDE.md
- PoC iroh / NAT traversal : experiments/iroh-poc/ (branche experiment/iroh-poc)

⸻

Ce qui a été fait ces 10 derniers jours (01 → 11 février 2026)

1) “Phase 1 Complete” n’est pas une phrase, c’est un périmètre livré

Le README annonce Phase 1 Complete, et le sprint-status confirme : Epics 1 → 8 = done.
Ça couvre : fondations, transport WebRTC, routage/relai, découverte (gossip), multi-relay, E2E, réseau “alpha” auto-organisé (subnets éphémères), tooling LLM, extension VS Code, et modèle de contribution micro-session.

Le CHANGELOG ancre un jalon : release 1.0.0 (2026-02-05) avec une liste claire des fonctionnalités principales.

2) Une semaine de consolidation post-épics, documentée (pas cachée)

Le document pivot : epic-4-8-retro-2026-02-07.md.

Il raconte ce que les bons projets font rarement publiquement :
- “Oui, ça marche”
- “Voilà où ça casse”
- “Voilà l’ordre d’exécution pour rendre ça robuste”

Les 4 actions annoncées (hub failover, invitations robustes, UI réactive, framework E2E) sont marquées DONE, avec commits.
Le fichier scripts/tests-added-log.md chiffre l'effort : 577 → 626 tests (+49) sur la session du 07/02. Le compteur total est désormais à 771 tests.

3) Le “LLM-first” est un système complet, pas un slogan

Ce n’est pas juste llms.txt.
C’est un ensemble cohérent :
- CONTRIBUTING.md pose le modèle micro-session (30–60 min, petites PRs)
- .github/ISSUE_BACKLOG.md maintient un stock d’issues “copier-coller”
- CLAUDE.md + architecture.md imposent des règles de cohérence (dépendances, naming, invariants)

ToM ne livre pas seulement un protocole : il livre une manière de le construire à plusieurs, vite, et proprement.

4) Les tests E2E ne servent pas à “faire joli” : ils trouvent des défauts réels

Sur la branche actuelle, invitation-flow E2E est à 4 tests OK / 1 test KO.
Le test KO est un scénario “multiple pending invitations” qui tente de cliquer un bouton d’acceptation désactivé (déjà “Rejoint”).
C’est un bon échec : pas un crash. Une friction de scénario / sélecteur / état UI à corriger.

5) Le vrai tournant : ToM perce les NAT — pour de vrai (09 → 11 février)

Jusqu'ici, tout le transport de ToM reposait sur WebRTC et un signaling server WebSocket. C'est fonctionnel, mais c'est une corde ombilicale. Un vrai protocole P2P doit percer les NAT tout seul.

La question de la semaine : peut-on remplacer ce WebSocket par du vrai hole punching QUIC ? La réponse est venue en 3 jours, avec un PoC progressif basé sur iroh (n0-computer, Rust, MIT, 7800+ stars).

Pourquoi iroh et pas libp2p ou Hyperswarm ?

Trois alternatives ont été étudiées en profondeur :

| Solution | Forces | Pourquoi pas (pour ToM) |
|----------|--------|------------------------|
| Hyperswarm | DHT-first (philosophie proche de ToM), simple | Node.js uniquement, pas de browser, communauté plus petite |
| libp2p | Multi-langage, écosystème large, spec formelle | Relay-heavy dans la phase initiale, complexité d'intégration, mauvais alignement avec la vision "zéro infra" |
| iroh | Rust, QUIC natif, hole punching + relay fallback, ~90% direct en production | Dépendance à n0-computer (gérée par un plan de fork stratégique) |

iroh aligne avec ToM sur les fondamentaux : identité = clé Ed25519, relais stateless, transport chiffré E2E automatique, pas de serveur central requis.

Les 4 PoC — du "Hello World" au hole punch cross-border

PoC-1 — Echo QUIC (deux noeuds, même machine)
Résultat : connexion en 289ms, RTT 125ms. Le hello world de QUIC. Ça marche, le relay iroh est automatiquement assigné (Europe).

PoC-2 — Gossip peer discovery (HyParView/PlumTree)
Résultat : neighbor détecté en 257ms, broadcast instantané. Les noeuds se découvrent sans registre central, via un protocole épidémique. Exactement ce dont ToM a besoin pour remplacer le signaling server.

PoC-3 — Architecture cible ToM : gossip pour la découverte, QUIC direct pour les messages
Le gossip sert uniquement à annoncer sa présence. Les messages transitent par des streams QUIC directs — pas par le gossip. C'est l'architecture cible de ToM : découverte décentralisée + payload direct.
Résultat : découverte en 3s, livraison du premier message en 4.8s. Trois bugs subtils découverts et corrigés (timing ANNOUNCE, race condition QUIC, EOF stdin).

PoC-4 — Le test de vérité : NAT traversal sur de vrais réseaux

C'est le test qui valide ou invalide toute la stratégie.

Setup : un binaire Rust instrumenté (sortie JSON structurée), cross-compilé en statique ARM64 via cargo-zigbuild, déployé sur un NAS Freebox Delta (VM Debian, Cortex-A72). Le MacBook Pro sert de connecteur.

4 scénarios, 4 succès :

| Scénario | Topologie | Upgrade relay→direct | RTT direct | Pings directs |
|----------|-----------|---------------------|-----------|---------------|
| LAN WiFi | MacBook ↔ NAS, même réseau | 0.37s | 49ms | 100% |
| 4G CGNAT | MacBook sur hotspot iPhone ↔ NAS sur WiFi maison | 2.9s | 107ms | 90% |
| Cross-border (CH↔FR) | MacBook sur WiFi d'une école en Suisse ↔ NAS en France | 1.4s | 32ms | 95% |
| Localhost | Même machine (baseline) | 1.4s | 2.6ms | 80% |

Le résultat le plus parlant : depuis un réseau WiFi invité d'une école en Suisse (derrière un NAT restrictif), le hole punch vers un NAS derrière une box résidentielle en France prend 1.4 seconde. Ensuite, tous les pings passent en direct à 32ms. Le relay n'est qu'un tremplin.

Ce que ça prouve pour ToM :
- Le CGNAT opérateur (4G) est perçable — 90% de pings directs même à travers le NAT le plus agressif
- Le relay iroh (euc1-1.relay.n0.iroh-canary.iroh.link, EU) est un fallback, pas une béquille
- Zéro configuration réseau nécessaire : pas de port forwarding, pas de STUN/TURN custom
- Le chiffrement E2E est automatique (QUIC TLS), pas une couche ajoutée après coup

La suite pour iroh dans ToM : fork stratégique des modules de connectivité, puis remplacement du signaling WebSocket par le transport QUIC natif. Le signaling server n'est plus un plan théorique à éliminer — il a désormais un successeur testé.

⸻

L'état actuel (ce qui est vrai maintenant)

Architecture repo (vue simple) :
- packages/core (tom-protocol) : primitives du protocole (identity, transport, routing, discovery, groups, roles, crypto, backup, types, errors)
- packages/sdk (tom-sdk) : TomClient, l’API “plug-and-play”
- apps/demo : démo web (chat + snake multi)
- tools/signaling-server : bootstrap WebSocket temporaire (ADR-002)
- tools/mcp-server : outillage pour interaction LLM
- tools/vscode-extension : extension VS Code (encore en partie “WIP”/mock selon la rétro)

Invariants (“7 locks”) qui cadrent le projet :
- Un message est “délivré” si et seulement si le destinataire final émet un ACK
- TTL max 24h puis purge globale (pas d’exception)
- La L1 observe/ancre, n’arbitre pas
- Réputation progressive, pas de bannissement permanent
- Anti-spam par charge progressive (“sprinkler gets sprinkled”), pas par exclusion
- Invisibilité côté utilisateur final (ToM = couche, pas produit)
- Scope : fondation universelle (type TCP/IP), pas application

Ces règles sont un filtre à contributions : tout ce qui les contredit est hors-scope, même si c’est “cool”.

⸻

Ce qu’il reste à faire (la suite logique)

Même si “Phase 1” est cochée, le projet est à un moment classique : ça marche, donc il faut le rendre inévitable.

1) Stabiliser les scénarios multi-participants (au-delà du happy path)
- Corriger le test E2E encore rouge (invitation-flow : multi pending invites)
- Continuer à transformer les comportements “fragiles mais fonctionnels” en comportements “robustes et prévisibles”

2) Intégrer le transport QUIC/iroh dans le coeur du protocole
- Le PoC-4 a validé que le hole punching fonctionne sur tous les types de NAT testés
- Prochaine étape : fork stratégique des modules iroh, remplacement du signaling WebSocket
- Le plan est en trois phases : fork → adaptation (wire format ToM, rôles dynamiques) → intégration dans le SDK TypeScript

3) Qualité de prod (CI) : coverage, audit deps, size tracking
- Plusieurs issues ouvertes vont exactement dans ce sens (coverage report, pnpm audit, build size)

4) Whitepaper v2
- Aujourd’hui, il n’y a qu’un whitepaper v1 dans le repo
- Le v2 doit être plus “preuve-driven” : ce que le code prouve déjà, ce qui reste volontairement ouvert, et ce qui est verrouillé

⸻

Validation de certains choix tech (solide vs à challenger)

Solide et cohérent avec la phase actuelle :
- Monorepo pnpm + TypeScript strict + tsup + vitest + biome : excellent setup pour itérer vite sans perdre le contrôle
- WebRTC DataChannel : incontournable browser-first (Phase 1)
- Stack crypto TweetNaCl : portable (browser + node), surface petite
- iroh (Rust, QUIC) pour le transport natif : validé en conditions réelles, 100% hole punch success sur 4 scénarios réseau (LAN, 4G CGNAT, cross-border CH↔FR)

À challenger / clarifier (utile pour le whitepaper v2) :
- Transition WebRTC → QUIC natif : quel calendrier, quelle coexistence ?
- Alignement exact du wording crypto dans la doc (certaines parties parlent de ChaCha20-Poly1305 vs XSalsa20-Poly1305 ; à unifier avec l'implémentation)
- Safari / "secure context" sur LAN (HTTP + WS) : c'est une contrainte réelle de démo/adoption dev, à documenter proprement (workarounds, HTTPS local, tunnel, etc.)
- iroh fork vs dépendance : le plan de fork stratégique doit être documenté (quels modules garder, quels modules adapter, quels modules réécrire)

⸻

Le GitHub ouvert : issues dispo (extraits)

Au 08/02/2026 : 25 issues ouvertes.
Exemples immédiats :
- #27 [refactor] Simplify EphemeralSubnetManager API (small, core/discovery)
- #26 [refactor] Extract message validation to separate module (medium, core/routing)
- #23–#25 CI : coverage report, build size tracking, dependency audit
- #20 [verification] Review signaling server for security issues
- #18 [verification] Audit TomError usage consistency

Source de backlog permanent : .github/ISSUE_BACKLOG.md (20+ micro/small/medium tasks prêtes à créer en issues).

⸻

Whitepaper v2 (où on en est, et comment le faire mieux)

Aujourd’hui : v1 existe (tom-whitepaper-v1.md). Le v2 n’est pas encore un document dans le repo.

Ce qui rend possible un v2 “meilleur” dès maintenant :
- le code prouve déjà des choses concrètes (E2E, failover, discovery, subnets, tests)
- le projet a des invariants verrouillés (design-decisions.md) qui forment une “constitution courte”
- la rétro décrit explicitement les points faibles et leur résolution, avec tests

Le v2 ne doit pas figer trop tôt.
Il doit faire trois choses :
1) Dire ce qui est désormais vrai (preuves)
2) Dire ce qui est volontairement ouvert (zones d’exploration)
3) Dire ce qui est non négociable (les locks)

⸻

Conclusion

Il y a 10 jours, ToM Protocol avait 771 tests qui passaient, une architecture propre, et une dépendance assumée à un signaling server WebSocket.

Aujourd'hui, il a tout ça — plus la preuve que deux machines séparées par deux NAT, deux pays et un réseau WiFi invité restrictif peuvent s'envoyer des pings directs à 32ms, sans aucune configuration réseau, sans port forwarding, sans serveur central.

Ce n'est pas un whitepaper qui promet. C'est un `cargo run` qui livre.

Le Whitepaper v2 ne devrait pas être un "document de plus".
Il devrait être l'endroit où le projet dit clairement :
- voilà ce qui est vrai maintenant (et les JSON de PoC-4 le prouvent)
- voilà ce qu'on refuse de figer trop tôt
- voilà comment venir casser (et donc renforcer) le système

Repo GitHub : https://github.com/malikkaraoui/ToM-protocol
Branche PoC iroh : experiment/iroh-poc