---
date: 2026-10-18
mandat: M0016
base: aaeee84
statut: synthèse — ne tranche rien à la place du fondateur
fait_foi: non (le texte qui fait foi reste docs/MISSION.md)
---

# Genèse et vision de ToM — huit mois relus point par point

> **Ce document ne fait pas foi.** Le texte qui fait foi est `docs/MISSION.md` (l.5). Ce document
> raconte comment la vision est née, comment elle a bougé et pourquoi, et où le code s'en écarte.
> Il propose ; il ne décide pas. Les décisions à prendre sont listées à la fin.
>
> **Étiquettes.** [VÉRIFIÉ] = relu par moi à l'endroit cité (fichier:ligne), dans le worktree au
> commit `aaeee84` ou dans les sources du fondateur. [HYPOTHÈSE] = déduction ou lecture probable,
> non prouvée. **proposition** = piste, jamais une décision.
>
> **Abréviations des sources les plus citées.**
> `v0.1` = livre blanc v0.1 du fondateur, `vault/runtime/log/sources-fondateur/whitepaper-v0.1-FR.txt` (hors dépôt, jamais committé) ·
> `Step2` = « Step 2 — The Open Messaging », même dossier, `Step2-TheOpenMessaging.txt` ·
> `wp-v1` = `docs/tom-whitepaper-v1.md` · `brief` = `_bmad-output/planning-artifacts/product-brief-tom-protocol-2026-01-30.md` ·
> `LOCKED` = `_bmad-output/planning-artifacts/design-decisions.md` (les 7 verrous) · `MISSION` = `docs/MISSION.md` ·
> `ADN` = `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md` · `18/10` = paroles du fondateur du 2026-10-18, verbatim dans le mandat M0016 (`vault/echanges/F01.md` l.30-31).
>
> **Règle de priorité.** Quand deux sources se contredisent, la plus récente l'emporte, parce que le
> fondateur l'a posé : « le projet évolue, la dernière version fait foi » (MISSION l.5). L'ordre
> pratique est : paroles du 18/10 > MISSION (17/10) > plans datés > documents de février > sources
> fondatrices de janvier. Le code n'est jamais une source de vision : c'est l'état réel, qu'on compare.
> **Une contrainte de plateforme (iOS, Apple TV, Freebox) n'est jamais une cause de vision** — règle de
> méthode posée par le mandat M0016 à la suite des paroles du 18/10 (`vault/echanges/F01.md` l.32) :
> quand une évolution vient d'elle, elle est marquée ⚠️.

---

## La vision en une page

**ToM est une couche de transport universelle, comme TCP/IP.** Elle porte des octets de A à B sans
serveur, sans péage, sans trace (MISSION l.25 ; wp-v1 l.11 ; brief l.14) [VÉRIFIÉ]. Elle n'est pas
pensée pour 10 ni 1 000 personnes mais « pour plusieurs millions voire milliards » (18/10). Elle ne
vise pas iOS : iOS hébergera au mieux le SDK de centaines d'apps ; ToM vise d'être une couche « au
même titre que TCP/IP » (18/10).

**L'infrastructure, ce sont les appareils des gens.** Chaque appareil est client et serveur
(v0.1 l.63-66 ; Step2 l.48-51) [VÉRIFIÉ]. Il n'y a ni serveur, ni relais bénévole, ni point d'entrée
fixe (MISSION l.30). Utiliser, c'est contribuer : le temps passé dans l'app « travaille pour vous »
(v0.1 l.18-23) [VÉRIFIÉ].

**Le réseau est un organisme à rôles, pas un tuyau.** Livrer un message n'est qu'une de ses
fonctions. Chaque appareil reçoit une part du travail commun — relayer, garder, observer, valider,
mettre en lien — selon le besoin du moment, sa note et son contexte (MISSION l.36-42). Personne ne
choisit son rôle ; personne ne peut l'acheter ni s'y soustraire.

**La vérité naît du voisinage, avant l'émission.** Un nœud ne s'auto-proclame pas : ses ~7 voisins,
« comme les étourneaux », acquiescent à son rôle avant qu'il le diffuse, ou le lui donnent et il ne
fait qu'en reprendre l'empreinte. S'il triche malgré tout, sa note baisse : il sort du jeu ou ne
reçoit plus que des tâches sans importance (18/10). Le même voisinage sert de témoin de présence
(MISSION l.71-73) et, un jour, d'observateur d'un portefeuille (Step2 l.261-262).

**Le réseau oublie.** Il ne garde que le présent : 24 h au plus, puis purge (LOCKED l.38-42)
[VÉRIFIÉ]. Un message pour un absent se recopie chez d'autres comme un « virus positif », puis
s'efface à la livraison ou à 24 h (MISSION l.47). La note s'efface aussi, sans bannissement
(LOCKED l.97-114) [VÉRIFIÉ]. L'abus n'est pas interdit : il devient épuisant pour son auteur
(Step2 l.140-168) [VÉRIFIÉ].

**Le jeton est un grain de riz.** C'est une note interne de participation, pas de l'argent ni un
actif (MISSION l.62-63) [VÉRIFIÉ]. La double dépense reste un chantier ouvert (MISSION l.74).

**La force vient de la masse — sous conditions.** « Plus il y a de monde plus le réseau est fort ;
petit c'est dur voire impossible » (18/10). Les mesures du projet confirment que c'est vrai pour la
capacité, la résilience et l'anti-censure, et contre un attaquant de taille fixe. Contre un attaquant
qui grossit avec le réseau, la masse seule n'aide pas : il faut les témoins, la note qui s'efface et
le quorum (`docs/plans/L1-002-entropie-non-biaisable.md` l.89-98) [VÉRIFIÉ]. La masse est
l'amplificateur ; les règles locales sont la digue.

**Le succès est l'invisibilité.** ToM a réussi quand plus personne ne prononce son nom
(`docs/plans/VISION-CONSOLIDEE-2026-07-16.md` l.37) ; « tant que l'utilité est là, le protocole vit
et quand elle disparaît, il s'efface » (v0.1 l.134-135) [VÉRIFIÉ].

---

## Méthode, en bref

- **Sources lues** : les 2 sources fondatrices (lues en entier), puis 264 fichiers du dépôt
  inventoriés dans `genese/00-inventaire.md`, répartis en 23 lots. Chaque lot a été lu par un
  lecteur Haiku frais, en lecture seule. Sorties brutes agrégées dans `genese/10-extraits.md`
  (3 530 extraits — lignes de tableau —, triés par thème puis par date).
- **Aucun extrait Haiku n'entre ici sans recontrôle** à son fichier:ligne. Les citations marquées
  [VÉRIFIÉ] ont été relues par moi ; la liste est en annexe.
- **Code** : relu au commit `aaeee84`. Le diff `99aa090..aaeee84` ne touche que
  `crates/tom-transport/src/node.rs` [VÉRIFIÉ] : les écarts code de l'ADN du 17/10 restent valides.

## Chronologie des pivots (dates vérifiées dans git ou dans la source)

| Date | Pivot | Source |
|---|---|---|
| 2026-01-30 | Sources fondatrices : livre blanc v0.1 et Step2 ; brief produit (entrée : wp-v1) | `brief` l.1-8 (`inputDocuments: tom-whitepaper-v1.md`, `date: 2026-01-30`) [VÉRIFIÉ] |
| 2026-02-02 | Premier commit ; wp-v1 entre dans git tel quel | `2cbf495` [VÉRIFIÉ] |
| 2026-02-06 | Les 7 verrous (LOCKED) | `9048088` [VÉRIFIÉ] |
| 2026-02-11 | Décision de forker iroh « sans dépendre de la feuille de route de n0 » | `experiments/iroh-poc/FORK-ARCHITECTURE.md` l.11 [VÉRIFIÉ] |
| 2026-02-18 | Bascule TypeScript → Rust : « No — cut the cord » | `_bmad-output/implementation-artifacts/rust-port-kickoff-2026-02-18.md` l.5, l.21 [VÉRIFIÉ] |
| 2026-03-03 | Anti-spam progressif (verrou #5) codé | `b800dbe` [VÉRIFIÉ] |
| 2026-03-05/06 | Infra maison « 0 € » ; option C (relais sur VPS) proposée puis non suivie | `docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` l.6, l.209 ; `docs/Brief Infrastructure Option C for GPT 5.3 CODEX.md` l.110 [VÉRIFIÉ] |
| 2026-03-30 | Amorçage = « pompe » tournante, pas un bootstrap fixe | `docs/plans/2026-03-30-bootstrap-pump-module.md` l.18-27 [VÉRIFIÉ] |
| 2026-06-22 | Rendez-vous DHT partagé, zéro configuration | `c34a3fd` [VÉRIFIÉ] |
| 2026-07-06 | Master map en 3 couches L0/L1/L2 ; exception TTL réservée à L2 « actée » | `docs/plans/TOM-MASTER-MAP.md` l.184 [VÉRIFIÉ] |
| 2026-07-10 | ADR-011 : la présence se prouve par le travail | `docs/plans/POP-PROOF-OF-PRESENCE.md` l.29 [VÉRIFIÉ] |
| 2026-07-12 | Quorum de témoins 2 à 4 ; « Observateur » abandonné | `094ec96`, `e5a9d2f` [VÉRIFIÉ] |
| 2026-07-15/19 | Pas de tâche de fond sur iOS ; économie d'énergie iOS assumée | `docs/plans/FEUILLE-DE-ROUTE-2026-07-16.md` l.17 ; `7291ac5` [VÉRIFIÉ] |
| 2026-07-20 | Charte « la masse comme carburant » ; recadrage « organisme à rôles » | `d54e5d3` ; `docs/plans/prisme-des-roles.md` l.12 [VÉRIFIÉ] |
| 2026-07-21 | « usage ≠ contribution » : le relais ne crédite plus l'origine | `2937330` [VÉRIFIÉ] |
| 2026-07-24 | Carte du rendez-vous tournant : murmuration ~7, aléa = état vivant | `666a234` ; `docs/plans/CARTE-rendezvous-tournant.md` l.37-45 [VÉRIFIÉ] |
| 2026-08-07 | Entropie : la masse seule ne protège pas du re-tirage | `95a804b` ; L1-002 l.84 [VÉRIFIÉ] |
| 2026-10-17 | MISSION réécrite avec les décisions du fondateur | `a623a38`, `987e169` [VÉRIFIÉ] |
| 2026-10-18 | Paroles du fondateur : multi-signature par 7 voisins ; millions voire milliards ; pas iOS | mandat M0016 l.30-31 [VÉRIFIÉ] |

---

## 1. Finalité, échelle, couche universelle

**Au départ (janvier).** Deux élans. Le premier est politique et pratique : « communiquer
librement, sur l'internet existant » (v0.1 l.15-16) ; un smartphone est déjà un petit serveur, sa
puissance doit servir l'utilisateur et non la publicité (v0.1 l.18-23). Le second est structurel :
un « BUS de communication mondial, ultra léger, résilient, gratuit dans l'esprit » (Step2 l.13-15),
une blockchain repensée « comme un organisme vivant » qui ne garde que l'état présent
(Step2 l.42-45). Et déjà l'échelle : « pas un petit réseau de niche », mais l'addition de la
puissance déjà présente aux extrémités (Step2 l.321-325). [VÉRIFIÉ]

**Trajectoire.** Le 30/01, le brief et wp-v1 posent la formule « comme TCP/IP » (wp-v1 l.11 ;
brief l.14) et l'échelle « de 3 nœuds à 3 millions » (brief l.203) [VÉRIFIÉ]. Le 06/02, c'est le
verrou #7 : « une fondation universelle invisible, pas une application » (LOCKED l.193-197)
[VÉRIFIÉ]. En juillet, le sens se précise : la vision « est une asymptote », jamais atteinte par
paliers finis (VISION-CONSOLIDEE l.96) [VÉRIFIÉ] ; le 20/07, le réseau devient « un organisme à
rôles tournants », la livraison n'étant « qu'UNE fonction » (prisme-des-roles l.12-14) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.25 : « une COUCHE DE TRANSPORT universelle — comme TCP/IP — pas une
application » [VÉRIFIÉ]. Le 18/10 ajoute l'échelle cible (« plusieurs millions voire milliards »)
et une exigence esthétique : « fluide, robuste, et naturellement beau ». La parole du 18/10 prime :
elle est la plus récente et elle précise sans contredire.

**Pourquoi ça a bougé.** Peu : la finalité est la partie la plus stable du projet. Le recadrage
« organisme à rôles » du 20/07 est une *décision du fondateur* qui corrige une dérive de méthode
(on testait un tuyau send/ACK).

**Verdict : ✅ tient.** Huit mois, aucune contradiction sur la finalité. Un mot à tracer : Step2
repense « la blockchain » (l.42-45) et la master map parle d'« une blockchain qui voyage léger »
(MASTER-MAP l.14), alors que MISSION l.64 et l.262 disent « pas de blockchain » [VÉRIFIÉ] — même mot,
deux sens (l'idée d'un état partagé sans maître ; le grand livre historique). Seule réserve : la
validation tourne sur « UNE box, UN FAI, UN foyer » (VISION-CONSOLIDEE l.87) [VÉRIFIÉ] ; « petit
c'est dur voire impossible » (18/10) implique que la flotte actuelle **ne peut pas** démontrer la
vision, seulement ne pas la contredire.

**Écart code.** Aucun écart de nature : le code est un transport. Écart d'échelle : rien n'a été
mesuré au-delà de quelques nœuds ; la courbe de masse in-process s'arrête à N=20
(`docs/plans/banc-courbe-masse.md` l.104-106) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Un simulateur à 10 / 100 / 1 000 / 10 000 nœuds logiques (déjà évoqué en
   M0014, `vault/reprise/PASSATION-COWORK-2026-10-18.md` l.59) qui mesure la charge *par nœud*,
   seule métrique qui falsifie « plus de monde = plus fort » (charte l.65-67).
2. **proposition** — Écrire en tête de MISSION l'échelle cible du 18/10, pour que chaque choix soit
   jugé « à un million » et non « sur la flotte ».

---

## 2. Plateformes, SDK, formes de livraison, nœuds 24/24

**Au départ.** Le parcours du brief : une page web, puis une extension de navigateur qui fait du
navigateur un nœud, puis un SDK, puis une couche que les fabricants embarquent dans les box et
les OS (brief l.94-104) [VÉRIFIÉ]. Implémentation d'abord en JavaScript pour le navigateur
(brief l.185), un cœur Rust/C pour l'IoT ensuite (brief l.202) [VÉRIFIÉ].

**Trajectoire.** Le 18/02, le TypeScript est abandonné : « No — cut the cord » (rust-port-kickoff
l.21) [VÉRIFIÉ], parce que le transport QUIC venait d'être validé à 99,5 % en 4G (même fichier
l.5) [VÉRIFIÉ]. À partir de mars, les seuls appareils de test sont Apple (Mac, iPhone, iPad,
Apple TV) et une VM sur Freebox. Le 15/07, la tâche de fond iOS devient un non-objectif : au
premier plan on contribue pleinement, en arrière-plan quelques minutes de sursis, le filet est le
backup 24 h (FEUILLE-DE-ROUTE l.17) [VÉRIFIÉ]. Le 18/10, le fondateur relie cela à une logique :
« temps d'écran agrégé » ; les nœuds 24/24 sont les box, Raspberry Pi, VM (PASSATION l.52)
[VÉRIFIÉ].

**Aujourd'hui.** MISSION l.58-60 : livré comme SDK, « un cœur, des bindings partout »
[VÉRIFIÉ]. 18/10 : « On ne vise pas iOS. » La parole du 18/10 prime et clarifie MISSION : la
plateforme n'est qu'un hôte.

**Pourquoi ça a bougé.** *Technique* pour la bascule Rust (le navigateur ne perce pas les NAT
seul). ⚠️ *Contrainte de plateforme* pour la tâche de fond : la décision vient de la suspension
iOS. Mais elle rejoint un principe fondateur — le temps d'usage travaille pour l'utilisateur
(v0.1 l.18-23) — donc la vision n'est pas déformée, elle est retrouvée. [HYPOTHÈSE sur
l'intention : le lien entre les deux n'est écrit nulle part avant le 18/10.]

**Verdict : ⚠️ tension.** Le principe tient ; la pratique est façonnée par Apple. Les formes de
livraison non-Apple (WASM, Kotlin/JNI, Python) « restent à faire en totalité »
(`docs/journal/JOURNAL.md` l.115) [VÉRIFIÉ]. Le seul SDK réel est le paquet Swift. Risque :
que des choix d'API soient pris pour iOS et présentés comme universels.

**Écart code.** FFI vers Swift seulement (`crates/tom-protocol-ffi`, exclu du workspace) ; aucune
cible WASM ni Android propre au projet : `git ls-files` ne trouve « wasm » et « android » que dans
`patches/netwatch-*` et `patches/netdev-*` (dépendances corrigées) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Avant toute distribution large, un deuxième binding non-Apple (Linux headless
   ou Android) pour casser l'échantillon Apple.
2. **proposition** — Écrire dans MISSION (garde-fou §6) la règle de méthode du mandat M0016, tirée des
   paroles du 18/10 : « une contrainte de plateforme n'est jamais une cause de vision ».

---

## 3. Rôles et leur attribution

**Au départ.** Step2 énumère six rôles : Client, Relais, Observateur, Gardien, Archiveur (« en
suspend »), Validateur (Step2 l.54-60). Ils « tournent » et sont attribués par « assignation
pseudo-aléatoire et Proof of Presence » (Step2 l.61-62). Surtout : ils sont **« annoncés avec
quelques blocs d'avance, ce qui permet aux pairs de vérifier que la rotation et l'aléatoire sont
corrects »**, et des « validateurs de l'aléa » surveillent la rotation (Step2 l.106-110).
[VÉRIFIÉ]

**Trajectoire.** Le même jour (30/01), wp-v1 et le brief disent l'inverse en apparence :
« Personne ne peut prédire qui sera sélectionné. Personne ne peut se positionner à l'avance »
(wp-v1 l.211-217) ; « roles at the last moment, unpredictably » (brief l.65) [VÉRIFIÉ]. wp-v1
retire aussi l'Archiveur et les validateurs de l'aléa ; ce texte est entré tel quel dans git le
02/02 (`2cbf495`) [VÉRIFIÉ]. L'ordre de rédaction entre Step2 et wp-v1 n'est pas daté à l'heure
près [HYPOTHÈSE : wp-v1 réécrit Step2]. Ensuite : ADR-006 « rôle par la topologie, pas par
config » ; ADR-011 (10/07) « la présence n'est pas déclarée, elle est prouvée par le travail »
(POP l.4) [VÉRIFIÉ] ; 12/07, l'Observateur est « abandonné » (`vault/20-decisions.md` l.31) ;
17/10, le fondateur précise que l'abandon ne visait que la « présence passive »
(`vault/20-decisions.md` l.32) [VÉRIFIÉ]. 24/07 : les 4 règles de la CARTE — l'aléa est l'état
vivant du réseau, « vérifiable, partagé » ; la capacité est un ticket d'entrée, pas un classement
(CARTE l.90-97) [VÉRIFIÉ]. 07/08 : « le rôle tourne au rythme de l'activité » (L1-002 l.205-212)
[VÉRIFIÉ].

**Aujourd'hui.** MISSION l.36-43 : rôle reçu du réseau selon besoin, note, contexte ; imposé « au
dernier moment, et de façon imprévisible » ; formule non fixée [VÉRIFIÉ]. **18/10** : la véracité
du rôle doit être « validée avant même son émission », par multi-signature des ~7 voisins — soit
ils donnent le rôle et le nœud n'en reprend que l'empreinte, soit ils valident avant diffusion.
**La parole du 18/10 prime et elle rapproche les deux textes de janvier.** Elle reprend l'idée de
Step2 (le rôle est connu des pairs, qui vérifient) et ses « signatures croisées de multiples rôles »
(Step2 l.118) [VÉRIFIÉ], sans renoncer à l'imprévisibilité de wp-v1.

**Pourquoi ça a bougé.** *Sécurité* : la red team a montré qu'un rôle auto-déclaré est une faille
(ADN l.65) et qu'un aléa contrôlé par le demandeur est « grindable » (L1-002 l.84) [VÉRIFIÉ].
*Décision du fondateur* (18/10) : agir avant, pas punir après.

**Verdict : ⚠️ tension — résoluble.** « Imprévisible » et « annoncé » ne s'opposent que si on les
place au même instant. **Lecture logique [HYPOTHÈSE] :** imprévisible *avant* le tirage (personne
ne peut se placer), annoncé et co-signé *après* (tout le monde peut vérifier). C'est exactement
la forme « engagement avant révélation » (L1-002 H1, l.156-159) et « co-signature quorum q-of-n »
(L1-002 H4, l.166-169) [VÉRIFIÉ]. Datation ambiguë dans MISSION l.41 : « imprévisible » y est dit
venir « du texte du 2026-06-26 » (date du premier jet de MISSION, l.4) ; le mot est en réalité dans
wp-v1 et le brief dès le 2026-01-30 (`2cbf495`, brief l.65) [VÉRIFIÉ]. Un lecteur peut croire à un
ajout de juin.

**Écart code.** ⛔ Deux rôles seulement : `enum PeerRole { Peer, Relay }`
(`crates/tom-protocol/src/relay.rs:43-48`) [VÉRIFIÉ]. La promotion est un seuil fixe du score
local, 10 / 2 (`roles/manager.rs:13`, `:16`) [VÉRIFIÉ]. Un rôle annoncé par un pair, signé par
lui-même, est appliqué tel quel (`runtime/state.rs:922` vérifie la signature, `:943` applique
`announce.new_role`) [VÉRIFIÉ] — c'est l'inverse exact du 18/10. Le hub de groupe est le NodeId le
plus bas, donc prévisible (ADN l.68) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Un rôle diffusé porte la co-signature de q de ses ~7 voisins (format d'annonce
   = empreinte du rôle + signatures) ; un pair n'applique `new_role` que si q signatures valides de
   voisins distincts sont présentes. C'est la variante « validation avant diffusion » du 18/10 ; elle
   réutilise le quorum déjà codé pour la présence (`presence/quorum.rs`).
2. **proposition** — Préciser MISSION l.41 (origine de « imprévisible » : 2026-01-30) et trancher la question 4 du
   §8 par la lecture « imprévisible jusqu'au tirage, co-signé ensuite » (voir question F1).
3. **proposition** — Faire de la co-signature du rôle la même mécanique que celle de la graine
   (H4), pour un seul moteur au lieu de deux (CARTE l.101-106).

---

## 4. Observation, témoins, murmuration ~7, gossip

**Au départ.** L'Observateur « surveille l'état de certains wallets ou subnets » (Step2 l.56) ;
il est au cœur de la double dépense (Step2 l.261-262) [VÉRIFIÉ]. Le consensus repose sur la
« participation au routage et à l'observation » (Step2 l.115-118) [VÉRIFIÉ].

**Trajectoire.** En février, le gossip est pensé comme découverte et preuve de présence implicite
(`results/CAMPAIGN-REPORT.md` l.188) [VÉRIFIÉ]. Le 10/07, ADR-011 : la preuve est toujours
« dyadique (je t'ai vu faire) ou scopé-relais », jamais « tout le monde dit » (POP l.81)
[VÉRIFIÉ]. Le 11-12/07 : un appareil faible s'abonne à la vue signée de ses relais ; un pair n'est
`Online` que si ≥ N témoins distincts concordent, N entre 2 et 4 ; « mieux vaut sous-estimer la
présence que se faire eclipser » (L1-003 l.60) [VÉRIFIÉ]. Le 24/07, la CARTE importe la science
des étourneaux : chaque oiseau suit « un nombre FIXE de voisins (~7), pas une distance », sans
aucune vue globale (CARTE l.37-45) [VÉRIFIÉ]. Le 17/10 : « tout le monde s'observe (jusqu'à 7) »
(MISSION l.72) [VÉRIFIÉ].

**Aujourd'hui.** MISSION §1.9 (17/10) et 18/10 : les ~7 voisins sont à la fois témoins de présence
et co-signataires du rôle. La parole du 18/10 prime et **étend** le rôle des 7 : de témoins (« je
t'ai vu ») à garants (« je confirme ton rôle »).

**Pourquoi ça a bougé.** *Terrain* : 44 à 50 pairs « Online » fantômes en juillet (POP l.8-12) ; *sécurité* : éclipse par témoin unique (kill-shot #3, `vault/20-decisions.md`
l.28) [VÉRIFIÉ]. *Science* (étourneaux) pour le nombre 7.

**Verdict : ⚠️ tension.** La logique tient — un nombre fixe de voisins rend la règle indépendante
de la densité, c'est ce qu'il faut à toute échelle. Deux tensions : (a) « jusqu'à 7 » (MISSION)
lit 7 comme un plafond, la science de la CARTE le lit comme une **cible fixe** ; (b) le code
mélange deux notions d'`Online`, voir ci-dessous.

**Écart code.**
- Témoins 2 à 4, pas 7 (`presence/relay_view.rs:33`, `:38`) ; vue active gossip 5
  (`tom-gossip/src/proto/hyparview.rs:202`) [VÉRIFIÉ].
- Le statut `Online` de la topologie est posé par trois chemins hors quorum : tout message signé
  reçu (`runtime/state.rs:1950-1956`), un `PeerAnnounce` lié à l'émetteur (`:1753-1758`), et
  **une annonce de rôle signée d'un pair encore inconnu** (`:953-957`) [VÉRIFIÉ]. Les deux premiers
  sont des preuves de première main (dyadiques) : conformes à ADR-011. Le troisième ne l'est pas :
  une annonce qui a pu voyager par gossip suffit à créer un pair `Online`, alors qu'ADR-011 dit
  « Jamais : annonce gossip, ré-annonce d'autrui, … auto-déclaration » (POP l.78) [VÉRIFIÉ].
- Aucune notion de « 7 voisins » dans le code (grep « murmuration / étourneau / starling » sur
  `crates/tom-protocol/src` : 0 occurrence) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Trancher « 7 = cible fixe » (question F2) et en dériver : chaque nœud tient
   ~7 voisins topologiques stables ; le quorum de présence et le quorum de rôle sont q-of-7.
2. **proposition** — Une annonce de rôle d'un pair inconnu crée au plus un pair `Known`, jamais
   `Online` (ferme l'écart `state.rs:953-957`).
3. **proposition** — Mesurer en simulation quelle valeur de q (sur 7) tient contre un attaquant à
   10 % — le cas réaliste d'EXP-4 (L1-002 l.92).

---

## 5. Note, réputation, « grain de riz », usage/contribution

**Au départ.** `Score_U = Contribution_U − Usage_U` ; l'idéal est **≈ 0**, le donnant-donnant ;
« les jetons … ne représentent pas un capital à accumuler, mais la mesure de cet équilibre »
(Step2 l.122-139) [VÉRIFIÉ]. Le livre blanc v0.1 parle d'un « rendement d'utilité » et d'une
fondation éventuelle (v0.1 l.99-103) [VÉRIFIÉ].

**Trajectoire.** 06/02, verrou #4 : « le passé existe mais s'efface progressivement », aucun état
binaire, aucun « banni » (LOCKED l.97-107) [VÉRIFIÉ]. Juin : le code ne garde qu'un score de
contribution, décroissant de 5 % par heure (`roles/scoring.rs:8`) [VÉRIFIÉ]. 20/07, la charte :
« tu ne peux pas te servir du réseau si le réseau ne peut pas se servir de toi » ; la contribution
est la condition d'usage (charte l.121-125) [VÉRIFIÉ]. 21/07 : le relais ne crédite plus
l'origine d'un message — « usage ≠ contribution » (`2937330`) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.62-65 : « un jeton, ça peut être un grain de riz » ; une note interne,
pas de l'argent ni un actif ; équilibre contribution/usage [VÉRIFIÉ]. 18/10 : la note est aussi la
sanction de celui qui triche malgré la co-signature (« hors jeu, ou des tâches sans importance »).
Les deux sont cohérents ; le 18/10 prime en ajoutant le lien note → rôle.

**Pourquoi ça a bougé.** *Sécurité* (2937330 : un relais gonflait le score de l'émetteur) ;
*décision du fondateur* (grain de riz). Le retour à « équilibre ≈ 0 » est un **retour aux sources**
de Step2, pas une invention.

**Verdict : ⚠️ tension.** La vision est cohérente. Mais le code mesure « qui m'a aidé », pas
« contribution − usage » : l'usage n'entre nulle part. Et wp-v1 l.315 prévoit qu'un nœud suspect
soit « exclu temporairement » [VÉRIFIÉ], ce que le verrou #4/#5 interdit (« No exclusion »,
LOCKED l.152) [VÉRIFIÉ] : contradiction ⛔ *dans les documents*, jamais passée dans le code.

**Écart code.**
- Score = contribution seule, vue locale de chaque nœud, aucune note partagée (ADN l.64)
  [VÉRIFIÉ].
- `record_bytes_relayed` n'a aucun appelant hors tests (appels seulement dans
  `runtime/state.rs` après la ligne 3956, où commence `#[cfg(test)]`, et dans `roles/mod.rs`, test)
  [VÉRIFIÉ] : le terme « octets relayés » du score est inerte en production.
- Le contexte de l'appareil (puissance, place, connexion fiable) n'entre pas dans la note
  (ADN l.66-67) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Mesurer l'usage (octets émis pour soi) à côté de la contribution, et viser
   l'équilibre ≈ 0 de Step2 plutôt qu'un score qui ne fait que monter.
2. **proposition** — Câbler `record_bytes_relayed` sur le chemin de relais réel, ou retirer ce terme
   (un terme inerte ment sur la formule).
3. **proposition** — Retirer « exclu temporairement » de wp-v1 l.315 (ou l'annoter « révoqué par le
   verrou #5 »), au titre de R4 : une décision révoquée se trace.

---

## 6. Anti-spam « arroseur arrosé »

**Au départ.** « Plus un utilisateur tente de saturer le réseau, plus le protocole lui renvoie
cette charge sous forme de travail local. » Trois outils : micro-PoW croissante, sur-assignation de
relais, tâches de validation non critiques ; « il s'épuise lui-même » (Step2 l.140-168) [VÉRIFIÉ].

**Trajectoire.** 06/02, verrou #5 : réaction progressive, continue, sans seuil dur, sans exclusion
(LOCKED l.123-152) [VÉRIFIÉ]. 03/03 : premier codage par limitation de débit selon le score
(`b800dbe`) [VÉRIFIÉ]. Juillet : plafonds globaux contre les essaims d'identités (red team ;
`vault/20-decisions.md` l.30) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.262 range « ban permanent » parmi les interdits ; 18/10 confie au
tricheur « des tâches sans importance » — c'est mot pour mot l'outil 3 de Step2. Le 18/10 prime
et confirme la version de janvier.

**Pourquoi ça a bougé.** *Technique* : la limitation de débit était le plus simple à coder ;
*sécurité* : la red team a imposé des plafonds globaux.

**Verdict : ⚠️ tension.** Le principe tient (aucun bannissement, rédemption possible). Mais le
code fait *ralentir* le spammeur ; il ne le fait pas *travailler*. Le cœur de l'image — l'abus
« transformé en contribution forcée » (`docs/article-ToM protocol.md` l.425) [VÉRIFIÉ] — n'existe
pas.

**Écart code.** Plancher de 30 messages/s, jamais zéro, plafond 100 (`roles/antispam.rs:68-70`) ;
commentaire « progressive load, not exclusion » (`:3-4`) [VÉRIFIÉ]. Aucune micro-PoW, aucune
sur-assignation : `grep` de « pow », « proof of work », « puzzle » vide dans `roles/` [VÉRIFIÉ].
Le budget est consommé sur l'expéditeur déclaré *avant* la vérification de signature : un pair
peut affamer une victime (CLAUDE.md, limite connue #13) — budget consommé l.1880 (`check_rate` sur `envelope.from`), signature vérifiée seulement l.1906 (`runtime/state.rs`) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Sur-assignation : un nœud dont l'équilibre est très négatif est tiré plus
   souvent comme relais intermédiaire (Step2 l.157-160). Cela réutilise la sélection de relais
   existante et transforme l'abus en travail utile.
2. **proposition** — Imputer le budget au saut de connexion authentifié (piste déjà écrite pour
   #13 : « le porteur du spam paie »).

---

## 7. Amorçage, découverte, rendez-vous, zéro tiers, zéro dépense

**Au départ.** « Aucun serveur propriétaire n'est requis. Pas de serveurs volontaires ni de relais
externes » (v0.1 l.65-66) [VÉRIFIÉ]. Le Gardien aide les nouveaux à se synchroniser — un
« bootstrap dynamique » (Step2 l.57) [VÉRIFIÉ]. Le brief admet un seul compromis : un petit serveur
de signalisation, « the one acceptable temporary compromise » (brief l.164) [VÉRIFIÉ].

**Trajectoire.** Février : serveur WebSocket « cordon ombilical », à couper. Mars : infra maison
« 0 € », « Gratuit (pas de VPS) » (`docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` l.6, l.209) [VÉRIFIÉ] ;
une option C avec relais sur VPS est rédigée (`docs/Brief Infrastructure Option C for GPT 5.3
CODEX.md` l.110) [VÉRIFIÉ] puis non suivie. 30/03 : l'amorçage est une « pompe » portée
temporairement par le réseau, dont le rôle d'entrée est rotatif (bootstrap-pump l.18-27)
[VÉRIFIÉ]. 22/06 : rendez-vous DHT partagé à 8 cases (`c34a3fd` ; `tom-dht/src/lib.rs:40`, `:44`)
[VÉRIFIÉ]. 21/07 : la Freebox, nœud stable 24/24, « devient l'aspirateur » du carnet (787 inconnus)
(`docs/plans/design-carnet-rendezvous-tournant.md` l.22) ; verbatim du fondateur : le carnet « doit
tourner demain et ne doit jamais être chez la même personne » (même fichier l.10-11) [VÉRIFIÉ].
24/07 : le premier jet « HRW + horloge » est abandonné, « un minuteur mécanique trahissait l'esprit
organique » (CARTE l.3-5) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.33 et l.265 : amorçage par la découverte externe actuelle, **zéro
dépense externe** pour l'amorçage et les relais ; sa portée au-delà est ouverte [VÉRIFIÉ]. Le
18/10 (passation) : n0 actif par défaut est « incohérent », à couper après un amorçage interne
prouvé (PASSATION l.53) [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Économie* (zéro dépense : VPS non retenus) ; *terrain* (OOM Freebox) ;
*décision du fondateur* (carnet tournant, zéro dépense). La Freebox n'est pas une cause de vision :
elle a seulement rendu visible une faille d'échelle — tout point fixe diffusif devient un
aspirateur à un million de nœuds. Pas de ⚠️ ici.

**Verdict : ⛔ contradiction partielle, assumée provisoirement.** « Zéro dépense » tient (aucun
service payant). « Zéro tiers » ne tient pas : « sans adresse connue d'avance, aucun mécanisme du
code ne permet une première rencontre depuis Internet sans tiers » ; la DHT Mainline et le DNS de
n0 sont deux tiers gratuits (ADN l.53) ; n0 voit le réseau, en tension avec la souveraineté
(ADN l.58) [VÉRIFIÉ]. C'est une entorse au v0.1 l.65-66, tolérée tant qu'elle reste un amorçage.

**Écart code.** `n0_discovery: true` par défaut (`crates/tom-transport/src/config.rs:172`) ;
`DEFAULT_RELAY_URLS` vide (`:12`) ; 8 cases fixes, pas de détenteurs tournants
(`tom-dht/src/lib.rs:44`) [VÉRIFIÉ]. La rotation du carnet n'est pas codée.

**Pour passer un step.**
1. **proposition** — Exécuter C1 de l'ADN (mesurer l'amorçage sans n0 ni DHT publique sur la flotte
   réelle), puis couper n0 par défaut, comme le fondateur l'a demandé (PASSATION l.53).
2. **proposition** — Le carnet tournant de la CARTE : les ~7 voisins portent aussi le rôle de
   rendez-vous, l'aléa est l'état vivant ; la DHT Mainline reste en secours du seul démarrage à
   froid (charte l.242-247).

---

## 8. Relais et transport

**Au départ.** Pas de relais externes (v0.1 l.65-66) ; pas de Bluetooth ni de Tor (v0.1 l.37-38) ;
un routage qui apprend les chemins courts et fiables « sans devenir dépendant d'un nœud
particulier » (v0.1 l.92-93) [VÉRIFIÉ].

**Trajectoire.** Février, le PoC iroh : « le relay est un tremplin, pas une destination »
(`docs/medium-article-poc.md` l.29) [VÉRIFIÉ]. Puis ADR-001, qui va plus loin : « Relays are not
optional — they ARE the architecture » (`CLAUDE.md` l.129) [VÉRIFIÉ]. 11/02 : fork d'iroh pour ne
plus dépendre de n0 (FORK-ARCHITECTURE l.11) [VÉRIFIÉ]. Juillet : relais embarqué dans chaque
nœud, publié seulement s'il est joignable de l'extérieur ; « on ne touche pas à la box »
(`docs/plans/CHECKPOINT-2026-07-16.md` l.31) [VÉRIFIÉ].

**Aujourd'hui.** Tout nœud peut être relais ; le relais ne stocke rien (MISSION, README). La
dernière source sur le relais *réseau* est la CARTE : les puissants portent « le lourd + les
connexions directes rapides », les faibles des tâches simples (CARTE l.13-16) [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Technique* (NAT, CGNAT : il faut un relais de première rencontre) ;
⚠️ *plateforme* pour « on ne touche pas à la box » et pour `tom-gateway`, outil propre à la
Freebox (README l.360) [VÉRIFIÉ] — utile, mais pas universel.

**Verdict : ✅ tient**, avec une nuance de mots. « Relais externe » (v0.1) et « relais
obligatoire » (ADR-001) ne se contredisent pas : le relais obligatoire est un *nœud du réseau*,
pas un service externe. Le risque réel est ailleurs : le relais *publié* de la flotte serait encore
**un seul** nœud — la source le dit, avec la réserve « statut réel non encore confirmé » pour les
autres appareils, où le relais embarqué tourne aussi (`vault/40-roadmap.md` l.123 ; ADN l.66)
[VÉRIFIÉ].

**Écart code.** Relais = rôle `Relay` par seuil de score, pas par besoin (voir §3). ⛔ Sans relais
configuré et avec `n0_discovery` actif (le défaut de l'app), le nœud retombe sur les relais par
défaut de n0 — des relais tiers, externes, contraires à v0.1 l.65-66
(`crates/tom-connect/src/defaults.rs:22-35` ; ADN l.31, qui les juge « cas résiduel ») [VÉRIFIÉ]. L'en-tête
`.tom.invalid` du TLS est identifiable par une inspection de paquets ; arbitrage ouvert (charte
l.262-265) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Mesurer la concentration (part du trafic relayé par les plus gros nœuds),
   indicateur demandé par la charte (l.39-41) [VÉRIFIÉ], et la publier à chaque build.
2. **proposition** — Garder `tom-gateway` comme accessoire Freebox, jamais comme prérequis.

---

## 9. Purge, TTL, backup « virus », état présent, L1 miroir/ancre

**Au départ.** La L1 ne garde que l'état présent : « purge agressive », « genèse glissante »,
« état présent plutôt qu'historique » (Step2 l.64-81) [VÉRIFIÉ]. « Sans copie d'historiques
globaux » (v0.1 l.87-89) [VÉRIFIÉ].

**Trajectoire.** 06/02, verrou #2 : 24 h maximum puis purge globale, quel que soit l'état de
livraison (LOCKED l.38-42) [VÉRIFIÉ]. 06/07 : le mur n°4 — l'anti-double-dépense exige qu'un
témoin se souvienne au-delà de 24 h — est résolu par couches : #2 gouverne le transport, L2
« assume un état scopé » ; exception « réservée à la valeur », actée par l'auteur (MASTER-MAP
l.184) [VÉRIFIÉ]. 18/07 : la règle des 24 h est étendue à la mémoire topologique — « rien ne survit
24 h », la proposition 7 jours écartée (anti-ravivage l.83-86) [VÉRIFIÉ]. 21/07 : le carnet de
contacts, lui, est « borné ~30, privé, local, durable » (design-carnet l.29) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.262 : « Jamais de … persistance au-delà du TTL » ; aucune mention de
l'exception L2 [VÉRIFIÉ]. MISSION l.47 : les messages pour absents « se répliquent sur des nœuds de
secours » puis s'auto-détruisent [VÉRIFIÉ]. MISSION est la plus récente : l'exception du 06/07
n'y figure plus.

**Pourquoi ça a bougé.** *Sécurité* (mur Fable n°4) pour l'exception ; *terrain* (fantômes
ravivés, 17-18/07) pour l'extension aux métadonnées.

**Verdict : ⚠️ tension.**
- L'exception L2 « actée » le 06/07 n'est ni retirée ni reprise par MISSION : on ne sait pas si
  elle vit encore. Tant qu'il n'y a pas de L2, rien ne casse ; dès qu'on code la double dépense, il
  faudra trancher.
- « Durable » pour le carnet de contacts est compatible avec #2 **si** ce carnet est une donnée de
  l'application (le répertoire de l'utilisateur), pas un état du protocole [HYPOTHÈSE de lecture].

**Écart code.**
- TTL plafonné à 24 h à la création (`crates/tom-protocol/src/backup/types.rs:14`) [VÉRIFIÉ].
- ⛔ **Le « virus » ne se propage pas en production.** La seule fonction qui envoie une copie de
  secours à un autre nœud est `replicate_to` (`backup/coordinator.rs:218`, unique producteur de
  `BackupAction::Replicate`, l.248) ; elle n'est appelée que dans des tests (`coordinator.rs`
  après l.310 `#[cfg(test)]`, `backup/mod.rs` test, `tests/backup_integration.rs`). Les événements
  `ReplicationNeeded` et `SelfDeleteRecommended` sont jetés (`runtime/state.rs:3865-3867`). Le
  runtime sait *recevoir* une réplique (`state.rs:1634-1637`) mais aucun nœud n'en *envoie*.
  [VÉRIFIÉ] En pratique, le gardien est l'émetteur lui-même (backup créé sur échec d'envoi). Le
  README parle d'un message qui « self-replicates across backup nodes » (README l.52) [VÉRIFIÉ] :
  la documentation est en avance sur le code.

**Pour passer un step.**
1. **proposition** — Câbler la réplication sortante (déclencheur : viabilité de l'hôte en baisse ou
   destinataire absent depuis X), bornée par le budget en octets déjà en place, et confirmer la
   purge multi-copies (`ConfirmDelivery` conditionnel, déjà écrit dans CLAUDE.md, limite #14).
2. **proposition** — En attendant, corriger le README et MISSION l.47 : « l'émetteur garde la
   copie » est la vérité actuelle.
3. **proposition** — Réinscrire dans MISSION le sort de l'exception L2 (question F5).

---

## 10. Subnets, cercles réels, vases communicants

**Au départ.** Des sous-réseaux naissent quand des échanges deviennent récurrents — famille,
équipe, événement — puis fusionnent, se divisent, se mettent en veille, s'éteignent ; une même
personne relaie entre plusieurs, « sans copie d'historiques globaux » (v0.1 l.80-89) [VÉRIFIÉ].
Le fork d'un subnet surchargé est une « respiration » (Step2 l.83-94) [VÉRIFIÉ].

**Trajectoire.** Le code construit des grappes BFS sur le graphe des messages : arête ≥ 3
messages, taille 3 à 10, dissolution après 5 minutes (`discovery/subnet.rs:15-27`) [VÉRIFIÉ].
20/07 : « jamais exercé par un banc » (prisme-des-roles l.37-38) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.76-79 (17/10) : pas de règle fixe ; « chacun a un répertoire de
contacts et parle très rarement en dehors. Un subnet = ce cercle réel » ; un petit subnet s'appuie
sur un plus robuste — vases communicants [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Décision du fondateur* (17/10). C'est un **retour aux sources partiel** :
« famille, équipe, événement » (v0.1 l.81-82) est déjà un cercle réel, mais v0.1 le fait naître de la
**récurrence des échanges** et mourir sur inactivité — c'est-à-dire du trafic, ce que fait le code ;
MISSION l.78 le rattache au répertoire de contacts.

**Verdict : ⚠️ tension dans la vision ; ⛔ le code contredit MISSION.** Deux lectures du même cercle :
née de la récurrence (v0.1) ou du répertoire (MISSION). Côté code, les bornes fixes 3-10 et le minuteur de
5 minutes sont l'inverse de « pas de règle fixe » ; et les subnets ne servent à rien : ils
produisent un événement d'observabilité, rien d'autre (`runtime/state.rs:3253-3263`) ; ni le
routage, ni le backup, ni les rôles ne les consomment (ADN l.73) [VÉRIFIÉ].

**Écart code.** Ci-dessus. Risque vie privée signalé par l'ADN : un cercle révèle qui parle à qui
(ADN l.113) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Prendre le cercle de contacts déclaré par l'application (le répertoire de
   l'utilisateur, déjà local) comme graine du subnet, et la récurrence des échanges (v0.1) comme
   signal de vie et d'extinction : les deux lectures ensemble, sans bornes fixes.
2. **proposition** — Premier vase communicant concret : les membres d'un petit cercle choisissent
   leurs gardiens de backup et leurs relais dans un cercle voisin plus robuste.

---

## 11. Chiffrement et identité

**Au départ.** Vie privée par défaut, chiffrement de bout en bout, clés locales, pas de collecte
centrale de métadonnées (v0.1 l.95-97) [VÉRIFIÉ]. Identité = couple appareil + wallet, avec une
probation pour les nouveaux (wp-v1 l.295-301) [VÉRIFIÉ].

**Trajectoire.** Février : Ed25519 + X25519 + XChaCha20-Poly1305 + HKDF ; clé publique = adresse
(ADR-004, ADR-005). L'article de février assume : « ToM ne cache pas qui parle à qui — les champs
`from` et `to` sont en clair » (`docs/article-ToM protocol.md` l.752) [VÉRIFIÉ]. Juin : l'audit
note l'absence de sécurité après compromission (pas de double ratchet), « assumée v1 »
(`docs/audits/AUDIT-2026-06-26.md` l.37) [VÉRIFIÉ].

**Aujourd'hui.** Inchangé ; la charte demande de prouver formellement FS et PCS (charte l.266-268) [VÉRIFIÉ].

**Pourquoi ça a bougé.** Presque pas. La seule évolution est la prise de conscience : le
chiffrement protège le contenu, pas le graphe social.

**Verdict : ⚠️ tension.** « Pas de collecte centrale de métadonnées » (v0.1) tient — il n'y a pas
de centre. Mais chaque relais voit `from`/`to`, et la DHT publique voit qui s'annonce. À un million
de nœuds, un relais curieux voit beaucoup [HYPOTHÈSE d'échelle].

**Écart code.** Les attestations de présence sont signées et **jamais chiffrées**, pour être
vérifiables par des tiers (`presence/attestation.rs:3-5`) [VÉRIFIÉ] : choix assumé.

**Pour passer un step.**
1. **proposition** — Faire relire la pile par un cryptographe externe avant tout engagement de
   valeur (déjà recommandé, VISION-CONSOLIDEE l.78) [VÉRIFIÉ].
2. **proposition** — Chiffrer `to` pour les sauts qui n'en ont pas besoin (relais qui ne fait que
   transmettre vers un relais connu) — piste à évaluer, pas un routage onion complet.

---

## 12. Invisibilité et universalité

**Au départ.** Phase 4 du brief : les fabricants embarquent ToM, « Users never know it exists »
(brief l.103-104) [VÉRIFIÉ]. Verrou #6 : « an architectural layer, not a product. Total invisibility
to end users » (LOCKED l.162-166) [VÉRIFIÉ].

**Trajectoire.** Stable. Juillet : « un protocole a réussi quand plus personne ne prononce son
nom » (VISION-CONSOLIDEE l.37) ; ToM « comme une socket » (FEUILLE-DE-ROUTE l.77) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.58-59 ; 18/10 : iOS n'est qu'un hôte du SDK [VÉRIFIÉ].

**Pourquoi ça a bougé.** Ne bouge pas.

**Verdict : ⚠️ tension.** La vision tient et n'a jamais bougé. Mais le code expose de l'état
protocolaire interne au-delà de la couche : les événements de rôle, de backup et d'anti-spam
traversent le FFI et atteignent l'interface. CLAUDE.md (limite #8, l.468) affirme le contraire ;
le code le dément (ci-dessous). Écart corrigé à la relecture R015.

**Écart code.** ⛔ `RolePromoted`, `RoleDemoted`, `BackupStored`, `SenderThrottled` sont convertis en
événements FFI (`crates/tom-protocol-ffi/src/types.rs:489-560`), empilés dans la file d'événements
(`lib.rs:686-690`) et rendus en JSON par `tom_node_receive_events` (`lib.rs:1305`) ; le SDK Swift
les décode (`sdk/swift/TomProtocolKit/Sources/TomProtocolKit/TomModels.swift:428`) et l'app affiche
`SenderThrottled` dans son fil d'activité « pour un humain »
(`apps/tom-node-tvos/TomNode/Services/TomNodeService.swift:300-306`) [VÉRIFIÉ]. L'app est un outil de
test, mais le SDK, lui, est la forme de livraison : tout intégrateur reçoit ces événements.

**Pour passer un step.**
1. **proposition** — Filtrer à la frontière du SDK : les événements internes restent dans un canal
   de diagnostic explicite, absent des builds de production ; puis un test de contrat qui le vérifie.
2. **proposition** — Corriger la limite #8 de CLAUDE.md (hors périmètre de ce mandat : issue #78).

---

## 13. Valeur, double dépense, wallet

**Au départ.** Step2 décrit en détail un wallet engagé, un mini-subnet d'observateurs et une
règle d'or : « aucun observateur ne signera deux transitions différentes partant du même
from_commit » (Step2 l.261-262) [VÉRIFIÉ]. Et une perspective : « une monnaie ou un jeton
utilitaire natif », sans minage ni rente (Step2 l.369-372) [VÉRIFIÉ].

**Trajectoire.** Le brief du même jour : « No speculative token … no convertible asset »
(brief l.58), « No token to buy, sell, or speculate on » (brief l.67) [VÉRIFIÉ]. Juillet : la
valeur devient la couche L2, « bonus », conditionnelle ; « la double dépense est une propriété de
la couche d'ATTESTATION (L1), pas du transport » (MASTER-MAP l.42) ; sécurité « probabiliste »
(MASTER-MAP l.111) ; « si M1.2/M1.4 ne tiennent pas, L2 est abandonné sans honte »
(VISION-CONSOLIDEE l.35) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.62-65 : pas de frais, pas d'actif convertible ; l.74 : double dépense,
chantier ouvert, pas d'argent dans le périmètre [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Sécurité* : sans entropie non biaisable ni coût de Sybil chiffré, aucune
garantie sur la valeur (murs Fable n°1-2) ; *décision du fondateur* (grain de riz).

**Verdict : ⚠️ tension.** La règle d'or de Step2 est saine et compatible avec le verrou #3 : ce
n'est pas la L1 qui arbitre, c'est chaque observateur qui refuse de signer deux fois. Mais la
perspective « monnaie ou jeton utilitaire natif » (Step2 l.369-372) n'a jamais été révoquée par
écrit ; MISSION l'exclut de fait (« pas d'actif convertible »). Une décision révoquée doit se
tracer (R4).

**Écart code.** Rien : aucune occurrence de wallet, ledger ou double dépense dans
`crates/tom-protocol/src` (grep « wallet / ledger / double spend / double dépense » : 0 occurrence) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Tracer explicitement le sort de « monnaie à terme » (question F4).
2. **proposition** — Quand le chantier rouvrira : les observateurs d'un wallet = ses ~7 voisins
   co-signataires (§3-4). Un seul mécanisme pour rôle, présence et transition.

---

## 14. Consensus, L1, genèse glissante

**Au départ.** Consensus par présence active : quorums tournants pseudo-aléatoires, signatures
croisées de multiples rôles ; « ce sont ceux qui sont là, qui relaient et qui observent, qui
valident » (Step2 l.100-120) [VÉRIFIÉ]. Genèse glissante, snapshots engagés (Step2 l.72-77)
[VÉRIFIÉ].

**Trajectoire.** wp-v1 : « des milliers de participants, sélectionnés en cascade au dernier
moment » (wp-v1 l.15) [VÉRIFIÉ]. Verrou #3 : « L1 observes and anchors — it does not arbitrate »
(LOCKED l.62-66) [VÉRIFIÉ]. L'article de février va jusqu'à « il n'y a pas de consensus à
corrompre » (`docs/article-ToM protocol.md` l.662) et « pas de vote — l'ordre déterministe élimine
le besoin » (l.370, à propos du hub) [VÉRIFIÉ]. Juillet : 3 couches ; L1 = couche d'attestation.
Août : l'entropie ; la masse seule ne protège pas du re-tirage (L1-002 l.84) [VÉRIFIÉ].

**Aujourd'hui.** MISSION ne parle plus de consensus ; la CARTE dit « L1 = miroir jamais chef »
(`vault/30-discoveries.md` l.16) [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Technique* : la messagerie n'a pas besoin de consensus (ACK +
TTL suffisent) ; *sécurité* : le consensus de la valeur bute sur l'entropie.

**Verdict : ⚠️ tension — la logique tient, la preuve manque.** Une fois les couches séparées : L0 (messagerie) n'a pas de consensus et
n'en veut pas ; L1 ne fait qu'ancrer des faits co-signés ; le refus de la double dépense est
local (règle d'or). L'article de février (« pas de consensus ») parle de L0, Step2 de L1 : pas de
contradiction, un vocabulaire à préciser. Le vrai trou est ailleurs : l'entropie non biaisable
reste un problème de recherche ouvert (L1-002 ; H1 « NON prouvée », l.156-159) [VÉRIFIÉ].

**Écart code.** Pas de L1 au sens de Step2. Dans le code, « L1 » désigne les specs de présence
(L1-001/002/003). La graine agrège 30 s d'attestations ; le choix du sous-ensemble par
l'agrégateur reste ouvert (« Do not claim non-biasability », `presence/aggregator.rs:9-12`) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Fixer le vocabulaire : « L0 = transport sans consensus ; L1 = ancrage de faits
   co-signés ; jamais d'arbitrage » dans MISSION.
2. **proposition** — Revue externe d'L1-002 par un cryptographe (garde L1.2bis déjà prévue,
   L1-002 l.185-198) [VÉRIFIÉ].

---

## Thèmes ouverts par les lecteurs (15-19)

Les lecteurs ont ouvert cinq thèmes hors liste ; ils sont rangés à part dans l'agrégat :
livraison et ACK (verrou #1), méthode et gouvernance, adoption et distribution, groupes, architecture
interne. Le seul qui touche la vision est **la gouvernance** : « le réseau finit par héberger son
propre code », mainteneurs en rotation (VISION-CONSOLIDEE l.37 et l.100) [VÉRIFIÉ] ; et le risque
« bus factor = 1 » (VISION-CONSOLIDEE l.86) [VÉRIFIÉ]. Il n'est pas traité ici (hors des 14 thèmes
du mandat).

---

## Cohérence d'ensemble

**Un seul moteur se dessine, et le 18/10 le ferme.** Relue d'un bout à l'autre, la vision converge
vers une seule mécanique : **le travail constaté par un voisinage fixe d'environ sept nœuds.** Ce
voisinage prouve la présence (ADR-011), co-signe le rôle (18/10), nourrit la note qui s'efface
(verrou #4), sur-assigne l'abuseur (arroseur arrosé), porte le carnet de rendez-vous (CARTE) et,
un jour, refuse la double dépense (règle d'or de Step2). La CARTE l'avait pressenti pour le
rendez-vous, les gardiens et l'accueil (« un seul moteur », CARTE l.101-106) ; l'étendre à la
présence, à la note et à la double dépense est **ma lecture** [HYPOTHÈSE]. Le 18/10 en donne la clef : la vérification se fait *avant*
l'émission, par les voisins, pas *après*, par la sanction.

**La vision est plus cohérente que ses documents.** Les vraies contradictions sont entre textes,
pas entre idées : « imprévisible » vs « annoncé » (résoluble, §3) ; « exclu temporairement » vs
« jamais d'exclusion » (§5) ; exception L2 actée puis absente de MISSION (§9) ; monnaie à terme
jamais révoquée (§13) ; datation ambiguë dans MISSION l.41 (§3).

**Le code est en retard sur deux points de nature, pas de degré.**
1. **Les rôles se déclarent encore** (`state.rs:943`) : c'est l'inverse du 18/10.
2. **Le virus ne se propage pas** (`replicate_to` sans appelant) : la résilience par réplication
   annoncée par MISSION §1.3 n'existe qu'en tests.
Les autres écarts (témoins 2-4, subnets à bornes fixes, n0 et ses relais par défaut, événements
internes exposés par le SDK) sont des écarts de degré.

**La tension centrale est la masse.** « Plus il y a de monde plus le réseau est fort » est vrai
pour la capacité, la résilience, l'anti-censure, et contre un attaquant de taille fixe ; faux
contre un attaquant proportionnel (L1-002 l.89-98) [VÉRIFIÉ] ; la charte, elle, le dit « vrai SOUS »
quatre conditions et « faux sinon » (charte l.90-91) [VÉRIFIÉ]. La vision tient
si l'on dit : **la masse amplifie, les règles locales protègent.** Le 18/10 contient déjà cette
réponse (co-signature + note), sans la nommer.

**La plateforme a façonné les pratiques, pas la vision — avec deux exceptions à surveiller.**
Pas de tâche de fond mobile : contrainte iOS au départ, mais rejointe par un principe fondateur
(v0.1 l.18-23). Échantillon Apple + une Freebox : il oriente les SDK et les outils (`tom-gateway`).
« Petit c'est dur voire impossible » : la flotte actuelle ne pourra jamais prouver la vision ; seul
un simulateur à grande échelle le pourra.

---

## Questions que seul le fondateur peut trancher

Chaque question donne les options et une recommandation. Aucune n'est tranchée ici. Les questions
déjà posées dans MISSION §8 ne sont rappelées que si ce document apporte un fait nouveau.

**F1. Rôles : imprévisible, annoncé, ou les deux ?** (MISSION §8 q.4)
- A. Imprévisible pur (wp-v1 l.211-217) : personne ne connaît les rôles à l'avance.
- B. Annoncé à l'avance (Step2 l.106-108) : les rôles sont publiés quelques tours avant.
- C. Imprévisible jusqu'au tirage, puis annoncé et co-signé par les ~7 voisins avant de prendre
  effet (lecture du 18/10).
- **Recommandation : C.** Elle garde l'anti-positionnement de A et la vérifiabilité de B ; elle
  correspond aux hypothèses H1/H4 de L1-002. Corollaire : préciser MISSION l.41 (« imprévisible »
  figure dès le 2026-01-30, pas seulement dans le texte du 2026-06-26).

**F2. Les 7 voisins : cible fixe ou plafond ?** (MISSION §8 q.5)
- A. Plafond (« jusqu'à 7 », MISSION l.72) : de 2 à 7 selon la densité.
- B. Cible fixe (~7 voisins topologiques, CARTE l.39-41) : toujours environ 7, quelle que soit la
  densité.
- **Recommandation : B.** C'est ce qui rend la règle indépendante de l'échelle, ce que vise le
  18/10. Le seuil q (combien des 7 doivent co-signer) se fixe par simulation, pas à l'intuition.

**F3. Comment écrire « plus il y a de monde, plus c'est fort » ?**
- A. Tel quel.
- B. « La masse amplifie ; les règles locales (témoins, note, quorum) protègent. »
- **Recommandation : B.** Les mesures du projet (EXP-4) montrent que A est faux contre un botnet
  qui grandit avec le réseau ; B dit la même ambition sans promesse fausse.

**F4. La « monnaie ou jeton utilitaire natif à terme » de Step2 (l.369-372) est-elle révoquée ?**
- A. Révoquée : ToM ne portera jamais de valeur convertible.
- B. En sommeil : possible en L2, seulement si L1 tient (VISION-CONSOLIDEE l.35).
- C. Maintenue comme objectif.
- **Recommandation : B**, écrite noir sur blanc dans MISSION (règle R4 : une décision révoquée se
  trace, ne s'efface pas).

**F5. L'exception au TTL de 24 h pour L2, « actée » le 2026-07-06 (MASTER-MAP l.184), vit-elle
encore ?** MISSION l.262 dit « jamais de persistance au-delà du TTL » sans exception.
- A. Révoquée : même la valeur obéit à 24 h (la double dépense devra se résoudre autrement).
- B. Maintenue, scopée à L2, inactive tant que L2 n'existe pas.
- **Recommandation : B**, inscrite dans MISSION à côté du verrou #2, pour ne pas la redécouvrir
  plus tard comme une contradiction.

**F6. Le « virus » doit-il vraiment se propager maintenant ?**
- A. Câbler la réplication vers des gardiens tiers (fidèle à MISSION §1.3).
- B. Assumer « le gardien est l'émetteur » pour l'instant et corriger la documentation.
- **Recommandation : A**, bornée par le budget en octets existant ; B en attendant, pour que la
  documentation ne mente pas.

**F7. « Exclu temporairement » (wp-v1 l.315) : à retirer ?**
- A. Retirer (le verrou #5 interdit l'exclusion).
- B. Remplacer par « surveillance renforcée et sur-assignation ».
- **Recommandation : B** : elle garde l'intention (repérer les fermes d'identités) dans la forme
  permise.

**F8. L'équilibre ≈ 0 de Step2 redevient-il la formule de la note ?** (MISSION §8 q.3)
- A. Note = contribution seule (code actuel).
- B. Note = contribution − usage, idéal ≈ 0 (Step2 l.130-133).
- **Recommandation : B** : c'est ce que dit déjà MISSION l.64 (« équilibre contribution/usage ») ;
  le code doit mesurer l'usage.

**F9. Quel est le premier SDK non-Apple ?**
- A. Linux headless (box, Raspberry Pi, VM : les nœuds 24/24 du 18/10).
- B. Android.
- C. WASM / navigateur (parcours du brief).
- **Recommandation : A** : ce sont les nœuds qui portent le réseau la nuit, et ils cassent
  l'échantillon Apple sans dépendre d'un magasin d'applications.

---

## Annexe — ce qui a été recontrôlé, et comment

**Règle appliquée.** Toute affirmation reprise d'un extrait Haiku a été relue par moi à son
fichier:ligne avant d'entrer ici. Aucune citation du texte ne repose sur un extrait non relu.

**Extraits Haiku repris puis recontrôlés : 70** (sur 3 530 lignes d'agrégat), par lecteur :
- A04 — rust-port-kickoff l.5, l.21 (2).
- A06 — article l.9, 370, 425, 662, 752, 889 ; medium-article-poc l.29 (7).
- A08 — INFRA-LOCAL l.6, l.209 (2).
- A15 — MASTER-MAP l.14, 42, 68, 111, 184 ; VISION-CONSOLIDEE l.35, 37, 78, 86-87, 89, 100 ;
  POP l.29, 78, 81 ; L1-003 l.60 ; FEUILLE-DE-ROUTE l.17, 77 ; CHECKPOINT l.31 (18).
- A16 — anti-ravivage l.83-86 (1).
- A17 — design-carnet l.10-13, 22, 29-31 ; charte l.65-67, 121-125, 266-268 ;
  banc-courbe-masse l.104-106 (7).
- A19 — journal l.115 ; AUDIT l.37 (2).
- A20 — R010 l.38 ; PASSATION l.52-53, l.59 ; `vault/20-decisions.md` l.26, 28, 30-33 ;
  `vault/40-roadmap.md` l.123 ; `vault/30-discoveries.md` l.16 ; ADN l.113 (9).
- A21 — FORK-ARCHITECTURE l.11 ; CAMPAIGN-REPORT l.7, l.188 ; CLAUDE.md l.129, limite #8 (l.468),
  limite #13 (l.512) ; README l.52, l.360 (8).
- A23 — `runtime/state.rs:940-957, 1728-1760, 1938-1956, 3248-3266, 3855-3872, 1880/1906` ;
  `backup/coordinator.rs:218, 248, 310` ; `presence/attestation.rs:3-5` ;
  `presence/aggregator.rs:9-12` ; greps « appelants de `replicate_to` / `record_bytes_relayed` »,
  « murmuration », « wallet/ledger » (14).

**Autres citations relues directement (sans passer par un extrait) : 110.**
- Sources du fondateur (22) — v0.1 l.15-16, 18-23, 37-38, 63-66, 80-89, 92-93, 95-97, 99-103,
  134-135 ; Step2 l.13-15, 42-45, 48-62, 64-81, 83-94, 100-120, 122-139, 140-168, 261-262, 321-330,
  355-372, 383-393 ; mandat M0016 l.28-34.
- Fondations du dépôt (40) — wp-v1 l.11, 13, 15, 17, 211-217, 295-301, 311-315 et
  `2cbf495:tom-whitepaper-v1.md` l.13, 15 ; brief l.1-8, 14, 58, 65, 67, 94-104, 164, 185, 202-203 ;
  LOCKED l.17-20, 38-42, 62-66, 97-101, 107, 114, 123-127, 152, 162-166, 193-197 ; MISSION l.5, 25,
  29-34, 36-43, 45-48, 58-60, 62-65, 71-74, 76-79, 81, 262, 265, 281-291.
- Plans (26) — CARTE l.3-5, 13-18, 37-45, 80-81, 90-97, 101-106 ; L1-002 l.84, 89-98, 154-169,
  185-198, 205-212 ; charte l.35-41, 90-91, 242-247, 262-265 ; bootstrap-pump l.18-27 ;
  Option C l.110 ; prisme l.12-17, 24, 33-38 ; POP l.8-12 ; ADN l.53, 58, 64-65, 68-69, 72-73.
- Code (22) — `relay.rs:43-48` ; `roles/manager.rs:13, 16` ; `roles/scoring.rs:8` ;
  `roles/antispam.rs:3-4, 68-70` + grep « pow / puzzle » vide ; `runtime/state.rs:922, 943,
  1634-1637, 3700-3730, 3956` ; `backup/types.rs:14` ; `presence/relay_view.rs:33, 38` ;
  `tom-gossip/src/proto/hyparview.rs:202` ; `tom-dht/src/lib.rs:40, 44` ;
  `tom-transport/src/config.rs:12, 172` ; `group/election.rs:5` ; `git ls-files` wasm/android.
- Git (17 commits) — `2cbf495`, `9048088`, `b800dbe`, `c34a3fd`, `094ec96`, `e5a9d2f`, `7291ac5`,
  `d54e5d3`, `2937330`, `666a234`, `95a804b`, `a623a38`, `987e169`, `02e0e0b`, `3bf7229`,
  `e6037e2`, `9558547`.

**Ajoutées lors de la correction R015 (9)** — `crates/tom-protocol-ffi/src/types.rs:489-560`, `lib.rs:686-690`,
`lib.rs:1305` ; `TomModels.swift:428` ; `TomNodeService.swift:300-306` ; `tom-connect/src/defaults.rs:22-35` ;
ADN l.31 ; MISSION l.4 ; POP l.4. Total des vérifications : **189**.

Les comptes regroupent les plages d'un même fichier citées ensemble ; ils comptent des
vérifications, pas des lignes.
