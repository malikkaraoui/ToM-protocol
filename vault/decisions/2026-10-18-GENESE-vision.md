---
date: 2026-10-18
version: 2
mandat: M0017 (V1 : M0016)
base: be8e18e
statut: synthèse — ne tranche rien à la place du fondateur
fait_foi: non (le texte qui fait foi reste docs/MISSION.md)
---

# Genèse et vision de ToM — relue point par point (V2)

> **Ce document ne fait pas foi.** Le texte qui fait foi est `docs/MISSION.md` (l.5). Ce document
> raconte comment la vision est née, comment elle a bougé, ce que les sources disent des raisons, et
> où le code s'en écarte. Il propose ; il ne décide pas. Les décisions à prendre sont listées à la fin.
>
> **Étiquettes (règles durcies en V2).** [VÉRIFIÉ] = la source citée dit **littéralement** ce qui
> est écrit (relu à fichier:ligne, dans le worktree au commit `be8e18e` ou dans les sources du
> fondateur). [HYPOTHÈSE] = déduction, rapprochement ou lecture de l'auteur. **Une cause sans phrase
> de source qui l'énonce est toujours [HYPOTHÈSE].** Si aucune source n'écrit la cause : « cause non
> écrite dans les sources ». Une date de jalon vient de git ou de la source ; sinon « non daté ».
> **proposition** = piste, jamais une décision.
>
> **Abréviations des sources les plus citées.**
> `v0.1` = livre blanc v0.1 du fondateur, `vault/runtime/log/sources-fondateur/whitepaper-v0.1-FR.txt` (hors dépôt, jamais committé) ·
> `Step2` = « Step 2 — The Open Messaging », même dossier, `Step2-TheOpenMessaging.txt` ·
> `wp-v1` = `docs/tom-whitepaper-v1.md` · `brief` = `_bmad-output/planning-artifacts/product-brief-tom-protocol-2026-01-30.md` ·
> `LOCKED` = `_bmad-output/planning-artifacts/design-decisions.md` (les 7 verrous) · `MISSION` = `docs/MISSION.md` ·
> `ADN` = `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md` · `CARTE` = `docs/plans/CARTE-rendezvous-tournant.md` ·
> `L1-002` = `docs/plans/L1-002-entropie-non-biaisable.md` · `charte` = `docs/plans/charte-cibles-agressives.md` ·
> `18/10` = paroles du fondateur du 2026-10-18, verbatim dans le mandat M0016 archivé
> (`vault/echanges/archive/2026-10-18-F01-M0016-genese-et-vision-de-tom.md` l.30-31) ·
> `M0017` = mandat de cette V2 (`vault/echanges/F01.md`, archivé ensuite sous `vault/echanges/archive/`).
>
> **Dates des deux sources fondatrices (corrigé en V2).** Le PDF du livre blanc v0.1 porte une date
> de création **2025-09-28** et un titre interne « whitepaper-v0.2-FR.pdf » (métadonnées XMP du
> PDF) [VÉRIFIÉ]. Le PDF de Step2 est un export iOS daté du **2026-01-30 07:06 UTC** [VÉRIFIÉ]. Ce
> sont des dates d'export de PDF, pas des dates de rédaction : le texte existait au plus tard à ces
> dates. wp-v1 n'est pas daté ; sa première trace est le premier commit, le 2026-02-02 (`2cbf495`).
> L'ordre de rédaction entre Step2 et wp-v1 n'est pas établi.
>
> **Règle de priorité (règle d'auteur, pas un fait des sources).** Quand deux sources se
> contredisent, ce document suit la plus récente. Ce n'est pas une conclusion de la lecture : c'est
> une **consigne du mandat**, venue du fondateur : « priorise la plus récente (en stipulant pourquoi
> ce choix ou cette évolution…) » (archive M0016 l.26), et MISSION l.5 : « la dernière version fait
> foi ». L'ordre pratique est donc : paroles du 18/10 > MISSION (17/10) > plans datés > documents de
> février > sources fondatrices. **Partout où une parole du 18/10 change une position, le texte le
> dit (« change »), au lieu de « précise ».** Le code n'est jamais une source de vision : c'est
> l'état réel, qu'on compare.
> **Une contrainte de plateforme (iOS, Apple TV, Freebox) n'est jamais une cause de vision** — règle de
> méthode posée par le mandat M0016 à la suite des paroles du 18/10 (archive M0016 l.32) :
> quand une évolution vient d'elle, elle est marquée ⚠️.

---

## Ce qui a changé depuis la V1

La V1 (M0016, `91b0c33`) avait des citations fiables mais des causes, des dates et des
interprétations fausses étiquetées [VÉRIFIÉ]. Corrections, une ligne chacune :

1. Étiquettes durcies : une cause sans phrase de source est [HYPOTHÈSE] ; « cause non écrite dans les sources » quand rien ne l'énonce.
2. Dates fondatrices : le PDF v0.1 date du 2025-09-28 (pas de janvier) ; Step2 = export PDF du 30/01 ; wp-v1 non daté (git 02/02).
3. Règle « la plus récente prime » présentée comme consigne du mandat ; « change » au lieu de « précise » quand le 18/10 change une position.
4. §1 : « aucune contradiction sur la finalité » était faux — le volet valeur/paiement de Step2 est abandonné ou en sommeil, sans révocation écrite.
5. §1 : simulateur « 10/100/1000/10000 déjà évoqué » faux — M0014 proposait 10/100/500.
6. §2 : cause TS→Rust corrigée — le kickoff n'écrit pas pourquoi Rust (« No need for JSON wire format compatibility » vise le refus de coexistence) ; raison écrite après coup dans l'article l.954 ; « 99,5 % » et « navigateur/NAT » ne sont plus des causes.
7. §2 : la tâche de fond iOS est justifiée par écrit par la vision (15/07) ; le lien avec v0.1 est une tension (« connecté en permanence »), pas une confirmation.
8. §2/F9 : « le seul SDK réel est Swift » faux — `tom-sdk` (Rust) et `tom-tui` existent, compilation Linux aarch64 documentée, et une interface C publique (en-tête cbindgen) existe déjà ; il manque sa stabilisation et un consommateur non-Swift.
9. §3 : « la red team a montré » (ADN l.65) faux — c'est un constat d'écart de l'ADN ; « grindable » absent de L1-002.
10. §3 : « exactement H1/H4 » remplacé par « analogue » — H1/H4 portent sur la graine, H1 « NON prouvée », H4 « à spécifier ».
11. §3 : « même jour (30/01) » pour wp-v1 retiré (non daté) ; « wp-v1 retire » remplacé par « n'a pas ».
12. §3 : la CARTE l.101-106 ne parle pas de la graine (H4) ; la proposition « un seul moteur » devient une idée de ce document.
13. §4 : témoins = garants du rôle, et « étend », passés en [HYPOTHÈSE] ; cause du passage 2-4 → 7 non écrite.
14. §5 : décroissance 5 %/h datée du 22/02 (`f9dd21d`), pas de juin ; « retour aux sources » retiré ; « hors jeu » (18/10) signalé contre « No exclusion ».
15. §6 : « mot pour mot l'outil 3 » faux ; `vault/20-decisions.md` l.30 est un plafond anti-Sybil de présence ; « le plus simple à coder » sans source ; 2-50 → 30-100 msg/s (`f9ba76c`, terrain).
16. §7 : abandon de l'option C (VPS) sans source — R17 garde des VPS « retirables » ; le point d'entrée Freebox est un serveur fixe ; « Zéro tiers : oui en LAN » rétabli.
17. §8 : « Puis ADR-001 » faux (ADR-001 date du 06/02, avant le PoC) ; relais embarqué daté du 20/03 ; citation CHECKPOINT rétablie ; « un seul relais » pas un présent.
18. §8 : ajout de `n0_discovery = true` par défaut (lib + app) et du README l.361 inexact, avec un correctif à coût zéro.
19. §9 : l'exception L2 n'a jamais été dans MISSION ; MASTER-MAP se contredit (l.23 vs l.184) ; cause « fantômes ravivés » fausse (« cohérence stricte »).
20. §10 : « jamais exercé par un banc » périmé (banc R5 du 21/07) ; « retour aux sources partiel » passé en [HYPOTHÈSE].
21. §11/§14 : cryptographe externe signalé comme dépense externe, voies gratuites ajoutées ; ADR-004 de février = TweetNaCl ; conciliation L0/L1 en [HYPOTHÈSE].
22. §13 : observateurs d'un wallet ≠ ~7 voisins (deux objets) ; « grain de riz » n'est pas la cause du passage en L2 (décision du 17/10, passage en L2 le 05/07).
23. Thèmes ouverts : « mainteneurs en rotation » faux — la source dit « gouvernance (rotation, pas de capture) ».
24. Cohérence : « un seul moteur » devient une [HYPOTHÈSE] avec quatre tensions (vivacité, Sybil/éclipse, petite échelle, mémoire) ; la masse n'est pas mesurée (simulations).
25. Questions : F1 option D (certificat porté, sans annonce) ; F3 option C (garder la phrase du fondateur) ; F4 option D (jeton = note), sans fermer la valeur ; F5 option C (hypothèse « Snap à Hash » du fondateur) ; F6 conditions (gardiens, borne globale, anti-concentration) ; F8 deux défauts et option C, signalée comme changement de vision ; F9 reformulée (stabiliser l'interface C existante).
26. Après la revue R016 (tour 1, RESERVE) : cause « Rust » non écrite au kickoff ; date du v0.1 dans le §1 ; passage en L2 daté du 05/07, avant la revue Fable ; MISSION l.47 traitée comme une cible, pas un état ; interface C existante ; F4 ne ferme plus la valeur.

---

## La vision en une page

**ToM est une couche de transport universelle, comme TCP/IP.** Elle porte des octets de A à B sans
serveur, sans péage, sans trace (MISSION l.25 ; wp-v1 l.11 ; brief l.14) [VÉRIFIÉ]. Elle n'est pas
pensée pour 10 ni 1 000 personnes mais « pour plusieurs millions voire milliards » (18/10). Sur les
plateformes, le fondateur dit à la fois « On ne vise pas iOS » et « On ne vise pas **seulement**
iOS » : iOS « risque juste d'héberger le SDK » de centaines d'apps ; ToM vise d'être une couche « au
même titre que TCP/IP » (18/10) [VÉRIFIÉ].

**L'infrastructure, ce sont les appareils des gens.** Chaque appareil est client et serveur
(v0.1 l.63-64 ; Step2 l.48-49) [VÉRIFIÉ]. « Aucun relais bénévole, aucun nœud de bootstrap contrôlé
par une entité unique » (MISSION l.30) [VÉRIFIÉ]. Le temps passé à utiliser le protocole « travaille
pour vous » (v0.1 l.22-23) [VÉRIFIÉ]. ⚠️ Tension non résolue : le même paragraphe de v0.1 (l.18)
décrit le smartphone comme « connecté en permanence », alors que le projet a fait de l'arrière-plan
mobile un non-objectif (§2).

**Le réseau est un organisme à rôles, pas un tuyau.** « La livraison de message est UNE fonction » ;
chaque appareil porte une part du travail commun — « relayer, garder, observer, valider, mettre en
lien » (`docs/plans/prisme-des-roles.md` l.12-15, 20/07) [VÉRIFIÉ]. Le rôle est reçu du réseau selon
le besoin du moment, la note et le contexte ; on ne peut « pas décider d'être relais ni d'y
échapper » (MISSION l.36-42) [VÉRIFIÉ].

**La vérité naît du voisinage, avant l'émission (18/10 — change la position).** Un nœud ne
s'auto-proclame pas : ses 7 pairs, « comme les étourneaux », acquiescent à son rôle avant qu'il le
diffuse, ou le lui donnent et il ne fait qu'en reprendre l'empreinte (« le hash »). S'il agit mal
malgré cela, sa note se dégrade : « hors jeu, ou des tâches sans importance » (18/10) [VÉRIFIÉ].
Avant le 18/10, aucune source ne demandait une validation des rôles *avant* émission (§3). Que ce
même voisinage soit aussi le témoin de présence (MISSION l.72 : « tout le monde s'observe (jusqu'à
7) ») et, un jour, l'observateur d'un portefeuille (Step2 l.216 : un « mini-subnet d'observateurs »
par wallet) est une **lecture de l'auteur** [HYPOTHÈSE] ; voir les tensions de la « Cohérence
d'ensemble ».

**Le réseau oublie.** 24 h au plus, puis purge globale « regardless of delivery status » (LOCKED
l.38-41) [VÉRIFIÉ]. Un message pour un absent « se réplique sur des nœuds de secours », puis
s'auto-détruit à la livraison ou après le TTL (MISSION l.47, « métaphore virus ») [VÉRIFIÉ] — en
production, le code ne réplique pas encore (§9). La note s'efface aussi, sans bannissement
(LOCKED l.100, l.107) [VÉRIFIÉ]. Le spam « n'est donc pas seulement interdit, il est
auto-destructeur pour celui qui le pratique » (Step2 l.165) [VÉRIFIÉ].

**Le jeton est un grain de riz.** C'est une note interne de participation, pas de l'argent ni un
actif (MISSION l.62-63) [VÉRIFIÉ]. La double dépense reste un chantier ouvert ; il n'y a « pas
d'argent dans le périmètre actuel » (MISSION l.74) [VÉRIFIÉ]. C'est un **changement** par rapport à
Step2, qui visait une L1 avec validateurs et « une architecture capable d'évoluer vers le paiement »
(Step2 l.40, l.60) [VÉRIFIÉ] (§1, §13).

**La force vient de la masse — sous conditions.** « Petit c'est dur voire impossible ; une fois le
réseau mature et étendu, c'est magiquement redoutable » et « plus il y a de monde plus le réseau est
fort » (18/10, deux phrases distinctes) [VÉRIFIÉ]. **Rien n'a encore été mesuré à grande échelle** :
« "plus de relais = plus de débit" est une hypothèse de conception, pas encore un chiffre. À
DÉMONTRER » (charte l.254-255) [VÉRIFIÉ] ; le seul banc de masse du protocole réel s'arrête à N=20
(`docs/plans/banc-courbe-masse.md` l.99-106) [VÉRIFIÉ]. Des **simulations** (pas des mesures)
montrent que contre un attaquant de taille fixe la masse écrase son avantage, et que contre un
attaquant qui grossit avec le réseau « la masse n'aide pas » : la digue est le témoignage, le fade et
le quorum (L1-002 l.83-98, EXP-3 et EXP-4) [VÉRIFIÉ]. La charte le dit « vrai SOUS » quatre
conditions, « faux sinon » (charte l.90-91) [VÉRIFIÉ].

**Le succès est l'invisibilité.** « Un protocole a réussi quand plus personne ne prononce son nom »
(`docs/plans/VISION-CONSOLIDEE-2026-07-16.md` l.37) [VÉRIFIÉ]. v0.1 ajoute une idée voisine mais
distincte : « tant que l'utilité est là, le protocole vit et quand elle disparaît, il s'efface »
(v0.1 l.134-135) [VÉRIFIÉ].

---

## Méthode, en bref

- **Sources lues** : les 2 sources fondatrices (lues en entier), puis 264 fichiers du dépôt
  inventoriés dans `genese/00-inventaire.md`, répartis en 23 lots. Chaque lot a été lu par un
  lecteur Haiku frais, en lecture seule. Sorties brutes agrégées dans `genese/10-extraits.md`
  (3 530 extraits — lignes de tableau —, triés par thème puis par date).
- **Aucun extrait Haiku n'entre ici sans recontrôle** à son fichier:ligne. Les citations marquées
  [VÉRIFIÉ] ont été relues par moi ; la liste est en annexe.
- **Code** : relu au commit `aaeee84` pour la V1. Le diff `99aa090..aaeee84` touche trois
  fichiers de code (`tom-transport/src/node.rs`, et une ligne dans `tom-protocol/src/runtime/state.rs`
  et `tom-stress/src/fleet_probe.rs`) [VÉRIFIÉ] ; le diff `aaeee84..be8e18e` ne touche que le vault
  [VÉRIFIÉ]. Les numéros de ligne de code cités restent valides au commit `be8e18e`.
- **V2 (M0017)** : 8 auditeurs Sonnet frais, un par groupe de sections, ont ouvert à la source chaque
  phrase de cause, chaque date, chaque attribution forte et chaque [VÉRIFIÉ] de la V1 (341 points).
  J'ai recontrôlé moi-même chaque point repris avant de corriger. Bilan en fin de document.

## Chronologie des pivots (dates vérifiées dans git ou dans la source)

Les dates de commit sont des dates d'**entrée dans git**, pas toujours de la décision elle-même.

| Date | Pivot | Source |
|---|---|---|
| 2025-09-28 | PDF du livre blanc v0.1 produit (titre interne « whitepaper-v0.2-FR.pdf ») | métadonnées XMP du PDF fondateur [VÉRIFIÉ] |
| 2026-01-30 | PDF de Step2 exporté (iOS) ; brief produit daté, qui prend wp-v1 en entrée | métadonnées du PDF Step2 ; `brief` l.1-8 (`inputDocuments: tom-whitepaper-v1.md`, `date: 2026-01-30`) [VÉRIFIÉ] |
| 2026-02-02 | Premier commit ; wp-v1 (non daté) et le brief entrent dans git | `2cbf495` [VÉRIFIÉ] |
| 2026-02-06 | Les 7 verrous (LOCKED) entrent dans git ; « No blockchain : This is a transport protocol, not a ledger » ; ADR-001 « Relays are not optional — they ARE the architecture » (ère WebRTC) | `9048088` ; `fbf4ba2:CLAUDE.md` l.54 [VÉRIFIÉ] |
| 2026-02-09/11 | PoC iroh (QUIC, hole punching) ; plan du fork « never depending on n0-computer's roadmap again » (le fork lui-même : squelettes de crates le 27/02) | `c0128d8` ; `experiments/iroh-poc/FORK-ARCHITECTURE.md` l.8-11 (`bfc994a`) ; `294ea5c` [VÉRIFIÉ] |
| 2026-02-18 | Bascule TypeScript → Rust : coexistence TS « No — cut the cord », raison écrite « No need for JSON wire format compatibility » | `_bmad-output/implementation-artifacts/rust-port-kickoff-2026-02-18.md` l.21 [VÉRIFIÉ] |
| 2026-02-22 | Score de contribution, décroissance 5 %/h, « no permanent bans » | `f9dd21d` (`roles/scoring.rs`) [VÉRIFIÉ] |
| 2026-03-03 | Anti-spam progressif codé (2 à 50 msg/s) ; relevé à 30-100 le 20/03 après une campagne qui perdait 30 % des messages en rafale | `b800dbe` ; `f9ba76c` [VÉRIFIÉ] |
| 2026-03-05/06 | Infra maison « 0€ », « Pourquoi local d'abord ? … Gratuit (pas de VPS) », « avant migration prod » ; option C (relais sur VPS) rédigée | `docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` l.3-6, l.207-209 ; `docs/Brief Infrastructure Option C for GPT 5.3 CODEX.md` l.110 [VÉRIFIÉ] |
| 2026-03-20 | Relais embarqué dans chaque nœud (Phase R16) | `bd6c442` [VÉRIFIÉ] |
| 2026-03-30 | Amorçage = « pompe » portée par le réseau, rôle d'entrée rotatif, pas un bootstrap fixe (statut : cadrage) | `docs/plans/2026-03-30-bootstrap-pump-module.md` l.18-27 [VÉRIFIÉ] |
| 2026-06-22 | Rendez-vous DHT partagé, zéro configuration ; relais publié au gossip seulement s'il est joignable de l'extérieur | `c34a3fd` ; `9d80e1b` [VÉRIFIÉ] |
| 2026-07-05/06 | Master map en 3 couches (05/07) ; exception TTL « réservée à la valeur », « ACTÉ (auteur 2026-07-06) » (06/07) | `bcd8978` ; `docs/plans/TOM-MASTER-MAP.md` l.184 (`90d5369`) [VÉRIFIÉ] |
| 2026-07-10 | ADR-011 : la présence se prouve par le travail | `docs/plans/POP-PROOF-OF-PRESENCE.md` l.3-4, l.29 [VÉRIFIÉ] |
| 2026-07-12 | Quorum de témoins 2 à 4 ; « Observer abandonné définitivement » | `094ec96`, `e5a9d2f` ; `vault/20-decisions.md` l.28, l.31 [VÉRIFIÉ] |
| 2026-07-15/19 | ⚠️ Pas de tâche de fond sur iOS (« c'est un choix », « contraire à la vision ») ; économie d'énergie iOS assumée | `dd8c99c` (CLAUDE.md, limite #4) ; `docs/plans/FEUILLE-DE-ROUTE-2026-07-16.md` l.17 ; `7291ac5` [VÉRIFIÉ] |
| 2026-07-18 | « Rien ne survit 24 h » étendu à la mémoire des pairs ; 7 jours écartés | `docs/plans/anti-ravivage-fantomes-topologie.md` l.83-86 (`d32bce7`) [VÉRIFIÉ] |
| 2026-07-20 | Charte « la masse comme carburant » ; recadrage « organisme à rôles » | `d54e5d3` ; `docs/plans/prisme-des-roles.md` l.3-4, l.12 [VÉRIFIÉ] |
| 2026-07-21 | « usage ≠ contribution » : le relais ne crédite plus l'origine | `2937330` [VÉRIFIÉ] |
| 2026-07-24 | Carte du rendez-vous tournant : murmuration ~7 (l.37-45), graine = état vivant (l.94) | `666a234` ; CARTE [VÉRIFIÉ] |
| 2026-08-07 | Entropie : la masse seule ne protège pas du re-tirage (simulation) | `95a804b` ; L1-002 l.84 [VÉRIFIÉ] |
| 2026-10-17 | MISSION réécrite avec les décisions du fondateur (grain de riz, zéro dépense, subnets, témoins) | `a623a38`, `987e169` [VÉRIFIÉ] |
| 2026-10-18 | Paroles du fondateur : multi-signature par 7 pairs avant émission ; millions voire milliards ; pas (seulement) iOS | archive M0016 l.30-31 [VÉRIFIÉ] |

---

## 1. Finalité, échelle, couche universelle

**Au départ (v0.1 au plus tard le 28/09/2025 ; Step2 au plus tard le 30/01/2026).** Deux élans. Le premier est politique et pratique : « communiquer
librement, sur l'internet existant » (v0.1 l.15-16) ; un smartphone est déjà un petit serveur, sa
puissance doit servir l'utilisateur et non la publicité (v0.1 l.18-23). Le second est structurel :
un « BUS de communication mondial, ultra léger, résilient, gratuit dans l'esprit » (Step2 l.13-15),
une blockchain repensée « comme un organisme vivant » qui ne garde que l'état présent
(Step2 l.42-45). Et déjà l'échelle : « pas un petit réseau de niche », mais l'addition de la
puissance déjà présente aux extrémités (Step2 l.321-325). [VÉRIFIÉ]

**Trajectoire.** Le brief (daté du 30/01) et wp-v1 (non daté, dans git le 02/02) posent la formule
« comme TCP/IP » (wp-v1 l.11 ; brief l.14) et l'échelle « From 3 nodes to 30 to 300 to 3 million »
(brief l.203) [VÉRIFIÉ]. Le brief exclut déjà la valeur spéculative ou convertible : « No
speculative token … no convertible asset » (brief l.58) [VÉRIFIÉ] ; wp-v1 garde une L1 avec wallets et observateurs (wp-v1 l.131,
l.173-187) mais « pas de cryptomonnaie à acheter, vendre, ou accumuler » (wp-v1 l.263-265)
[VÉRIFIÉ]. Le 06/02 entrent dans git le verrou #7 — « ToM is an invisible universal foundation, not
an application » (LOCKED l.196) — et la consigne « No blockchain: This is a transport protocol, not a
ledger » (`fbf4ba2:CLAUDE.md`) [VÉRIFIÉ]. En juillet, la vision « n'est pas atteignable par paliers
finis — c'est une asymptote » (VISION-CONSOLIDEE l.96) [VÉRIFIÉ]. Le 20/07, un « recadrage
fondateur » de **méthode de test** : le réseau « n'est pas un tuyau send/ACK : c'est un organisme à
rôles tournants. La livraison de message est UNE fonction » (prisme-des-roles l.4, l.12-13)
[VÉRIFIÉ] ; il vaut pour « toute analyse ET tout scénario de test » (prisme-des-roles l.6-7). L'idée
de rôles tournants n'est pas neuve (Step2 l.46-62) ; que le 20/07 change surtout la façon
d'analyser et de tester, pas la finalité, est ma lecture [HYPOTHÈSE].

**Aujourd'hui.** MISSION l.25 : « une COUCHE DE TRANSPORT universelle — comme TCP/IP — pas une
application » [VÉRIFIÉ] ; « pas de blockchain, pas d'actif convertible » (MISSION l.64) ; « pas
d'argent dans le périmètre actuel » (MISSION l.74) [VÉRIFIÉ]. Le 18/10 **élargit** l'échelle visée
(« plusieurs millions voire milliards ») ; le brief donnait une suite de paliers jusqu'à « 3
million », pas un plafond [VÉRIFIÉ]. La phrase
« fluide, robuste, et naturellement beau » porte, dans le verbatim, sur « le mécanisme global »
(la multi-signature des rôles), pas sur la finalité.

**Pourquoi ça a bougé.** Le cœur — couche de transport universelle — n'a pas bougé. **Un volet a été
abandonné ou mis en sommeil : la valeur.** Step2 vise une L1 avec « Validateur : il participe à des
quorums de validation pour la L1 » (l.60), « une architecture capable d'évoluer vers le paiement »
(l.40), des jetons qu'« on en dépense en utilisant le réseau » (l.138), un chapitre entier sur la
double dépense (§3, l.169-319) et « une voie vers une monnaie ou un jeton utilitaire natif »
(l.369-370) [VÉRIFIÉ]. MISSION (17/10) dit l'inverse : pas d'argent, pas d'actif, pas de blockchain,
double dépense « hors périmètre tant qu'il n'y a pas d'argent » (MISSION l.62-74, l.285) [VÉRIFIÉ].
Étapes écrites : 30/01 le brief exclut tout actif convertible ; 06/02 « No blockchain » ; 05/07
la valeur devient une couche L2 « bonus, dernières étapes » (`bcd8978`) ; 16/07 elle devient
conditionnelle : « si M1.2/M1.4 ne tiennent pas, L2 est abandonné sans honte » (VISION-CONSOLIDEE
l.35) ; 17/10 décision du fondateur « ce n'est pas de
l'argent, ce n'est pas un actif » (MISSION l.63) [VÉRIFIÉ]. Par qui : le brief et la décision du
17/10 sont signés du fondateur (`author: Malik` ; « décision 2026-10-17, fondateur ») [VÉRIFIÉ].
**Pourquoi : cause non écrite dans les sources.** [HYPOTHÈSE] le recentrage sur un PoC de transport
(le brief range « L1 engine implementation » hors du PoC, l.186) puis les murs de sécurité de la
valeur (entropie, Sybil : MASTER-MAP l.175-188) ont poussé la valeur à plus tard.

**Verdict : ⚠️ tension.** La finalité « couche de transport universelle » tient du début à la fin.
Mais **ce n'est pas vrai qu'il n'y a « aucune contradiction sur la finalité »** (V1) : le volet
valeur/paiement de Step2 contredit MISSION, et aucune source ne révoque explicitement « monnaie ou
jeton utilitaire natif » — je n'ai trouvé aucune révocation écrite (grep « monnaie » dans MISSION,
ADN, VISION-CONSOLIDEE, MASTER-MAP : 0) [VÉRIFIÉ]. Un mot à tracer : Step2 repense « la
blockchain » (l.42-45) et la master map parle d'« une blockchain qui voyage léger » (MASTER-MAP
l.14), alors que MISSION l.64 et l.262 disent « pas de blockchain » [VÉRIFIÉ] ; que ce soit le même
mot en deux sens est ma lecture [HYPOTHÈSE]. Réserve d'échelle : la validation de juillet tournait
sur « UNE box (Freebox), UN FAI, UN foyer » (VISION-CONSOLIDEE l.87, 16/07) [VÉRIFIÉ] ; un usage
nomade (iPhone en 4G loin du domicile) a eu lieu depuis, non instrumenté. Que « petit c'est dur
voire impossible » empêche la flotte actuelle de démontrer la vision est ma lecture [HYPOTHÈSE] :
le fondateur parlait du mécanisme des rôles.

**Écart code.** Aucun écart de nature : le code est un transport. Écart d'échelle : aucun banc du
protocole réel au-delà de N=20 (`docs/plans/banc-courbe-masse.md` l.99-106) [VÉRIFIÉ] ; les
simulations d'entropie montent à 50 000 nœuds, mais ce sont des modèles (L1-002 l.83-84) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Un simulateur à 10 / 100 / 1 000 / 10 000 nœuds logiques qui mesure la charge
   *par nœud*, la métrique qui peut falsifier « plus de monde = plus fort » (charte l.63-67). M0014
   avait proposé 10/100/500 (`vault/reprise/PASSATION-COWORK-2026-10-18.md` l.59, « pas encore de
   feu vert ») [VÉRIFIÉ] : les paliers 1 000 et 10 000 sont une extension de ce document. Limite
   écrite : « 100 nœuds sur 1 Mac mesure l'ordonnanceur, pas le réseau » (banc-courbe-masse l.31-32).
2. **proposition** — Écrire en tête de MISSION l'échelle cible du 18/10, pour que chaque choix soit
   jugé « à un million » et non « sur la flotte ».

---

## 2. Plateformes, SDK, formes de livraison, nœuds 24/24

**Au départ.** Le parcours du brief : une page web, puis une extension de navigateur qui fait du
navigateur un nœud, puis un SDK, puis une couche que les fabricants embarquent dans les box et
les OS (brief l.94-104) [VÉRIFIÉ]. Implémentation d'abord en JavaScript pour le navigateur
(brief l.185), un cœur Rust/C pour l'IoT ensuite (brief l.202) [VÉRIFIÉ].

**Trajectoire.** Le 09-11/02, un PoC iroh teste le remplacement du transport WebRTC + serveur de
signalisation (le remplacement effectif suit le portage Rust) :
« Jusqu'ici, tout le transport de ToM reposait sur WebRTC et un signaling server WebSocket. C'est
fonctionnel, mais c'est une corde ombilicale. Un vrai protocole P2P doit percer les NAT tout seul »
(`docs/medium-article.md` l.63-65, article dans git le 24/02) [VÉRIFIÉ]. Le 18/02, la coexistence
avec le TypeScript est refusée : « TypeScript coexistence | No — cut the cord | No need for JSON
wire format compatibility » (rust-port-kickoff l.21) [VÉRIFIÉ]. Le kickoff se place « on top of the
validated `tom-transport` crate (QUIC via iroh, 99.5% reliability on 4G CGNAT) » (l.5) [VÉRIFIÉ] :
c'est le contexte, pas la raison écrite. Le TypeScript n'est pas supprimé : il reste en « legacy »
et MISSION prévoit même de le moderniser pour le WASM (MISSION l.93). Le 15/07, la tâche de fond iOS
devient un non-objectif : « On ne fait PAS de background daemon » ; « Pas de push APNs/VoIP, pas de
wakeup permanent : ce serait contraire à la vision (couche invisible, pas un service toujours-actif
énergivore) » ; « Ce n'est donc pas un trou à combler, c'est un choix »
(`CLAUDE.md`, limite #4, entrée par `dd8c99c` le 15/07) [VÉRIFIÉ] ; au premier plan on contribue
pleinement, en arrière-plan quelques minutes de sursis, le filet est le backup 24 h
(FEUILLE-DE-ROUTE l.17) [VÉRIFIÉ]. La passation du 18/10 au matin, résumé de l'orchestrateur sur des
décisions du 17/10 (pas une parole verbatim), ajoute la logique « temps d'écran agrégé » ; les nœuds
24/24 sont les box, Raspberry Pi, VM (PASSATION l.4, l.52) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.58-60 : livré comme SDK, « un cœur, des bindings partout »
[VÉRIFIÉ]. 18/10 : « On ne vise pas iOS. … On ne vise pas seulement iOS : on vise à être une couche
protocolaire au même titre que TCP/IP. » Le verbatim contient les deux formules ; il ne contredit
pas MISSION, il dit que la plateforme n'est qu'un hôte.

**Pourquoi ça a bougé.** Bascule Rust : le kickoff du 18/02 n'écrit **pas** pourquoi Rust ; sa
phrase « No need for JSON wire format compatibility » (l.21) justifie une autre décision, le refus
de garder le TypeScript **à côté** [VÉRIFIÉ]. Une raison du choix de Rust est écrite après coup dans
l'article daté « Février 2026 » (dans git le 24/02) : « Langage | Rust | Sécurité mémoire,
performances, cross-compilation ARM » (`docs/article-ToM protocol.md` l.954) ; le même article
place le choix d'iroh, bibliothèque Rust, comme suite de l'étude des NAT (l.918) [VÉRIFIÉ]. Le
besoin de percer les NAT sans serveur de signalisation est écrit comme raison du PoC iroh
(medium-article l.65) [VÉRIFIÉ]. Que le 99,5 % en 4G ait *causé* la bascule est [HYPOTHÈSE] (la V1
l'écrivait comme un fait). Tâche de fond : le déclencheur est la
suspension iOS ⚠️ (contrainte de plateforme ; l'entrée s'intitule « Suspension iOS/tvOS —
NON-OBJECTIF ASSUMÉ »), mais la décision écrite du 15/07 justifie le refus du push et du réveil
permanent par la vision (« ce serait contraire à la vision ») [VÉRIFIÉ]. Le lien avec v0.1 (« le temps passé à l'utiliser
travaille pour vous », l.22-23) est ma lecture [HYPOTHÈSE] ; v0.1 dit aussi, quatre lignes plus
haut, que le smartphone est « connecté en permanence » (l.18) : c'est une **tension**, pas une
confirmation.

**Verdict : ⚠️ tension.** Le principe tient ; la pratique est façonnée par Apple. Les bindings
non-Apple (WASM, Kotlin/JNI, Python) « reste à faire en totalité — pas commencé »
(`docs/journal/JOURNAL.md` l.115, entrée du 02/07) [VÉRIFIÉ]. Mais il existe déjà un SDK cœur Rust,
`crates/tom-sdk` (membre du workspace, « SDK cœur Rust … P0 », MISSION l.91), et le client
`tom-tui` se compile pour Linux aarch64 musl (README l.358) [VÉRIFIÉ]. Et le FFI expose déjà une
**interface C publique** : en-tête généré `crates/tom-protocol-ffi/include/tom_protocol_ffi.h`
(« génération du header C public … check de drift en CI », `cbindgen.toml` l.1-3), 24 fonctions
`extern "C"` dans `src/lib.rs` [VÉRIFIÉ]. Ce qui manque : la stabiliser et la versionner, la faire
entrer dans le workspace, et un premier consommateur non-Swift. Risque : que des choix d'API soient
pris pour iOS et présentés comme universels.

**Écart code.** Interface C consommée par Swift seulement (`crates/tom-protocol-ffi`, posée sur
`tom-protocol`, exclue du workspace) ; aucune
cible WASM ni Android construite par le projet : `git ls-files` ne trouve « wasm » et « android »
dans les noms de fichiers que sous `patches/netwatch-*` et `patches/netdev-*` [VÉRIFIÉ]. Le code
hérité du fork contient des `cfg` WASM (par ex. `tom-quinn-proto/Cargo.toml`), sans cible
construite.

**Pour passer un step.**
1. **proposition** — Avant toute distribution large, un premier consommateur non-Swift de
   l'interface C existante (stabilisée et versionnée), puis Kotlin/JNI ou WASM, pour casser
   l'échantillon Apple.
2. **proposition** — Écrire dans MISSION (garde-fou §6) la règle de méthode du mandat M0016, tirée des
   paroles du 18/10 : « une contrainte de plateforme n'est jamais une cause de vision ».

---

## 3. Rôles et leur attribution

**Au départ.** Step2 énumère six rôles : Client, Relais, Observateur, Gardien, Archiveur (« en
suspend »), Validateur (Step2 l.54-60). Ils « tournent » et sont attribués par « assignation
pseudo-aléatoire et Proof of Presence » (Step2 l.61-62). Quatre d'entre eux (« validateur,
observateur, gardien, relais critique ») sont « annoncés avec quelques blocs d'avance, ce qui
permet aux pairs de vérifier que la rotation et l'aléatoire sont corrects », et des « validateurs
de l'aléa » surveillent la rotation (Step2 l.106-110). [VÉRIFIÉ]

**Trajectoire.** wp-v1 (non daté, dans git le 02/02) et le brief (daté du 30/01) disent l'inverse
en apparence : « Personne ne peut prédire qui sera sélectionné. Personne ne peut se positionner à
l'avance » (wp-v1 l.211-217) ; « roles at the last moment, unpredictably » (brief l.65) [VÉRIFIÉ].
wp-v1 n'a ni Archiveur ni validateurs de l'aléa (wp-v1 l.160-171) [VÉRIFIÉ] ; parler de « retrait »
supposerait que wp-v1 vient après Step2, ce qui n'est pas établi (ordre non daté). Ensuite :
ADR-006 « Role is determined by network topology, not configuration » (CLAUDE.md) ; ADR-011 (10/07)
« la présence n'est pas déclarée, elle est prouvée par le travail » (POP l.4) [VÉRIFIÉ] ; 12/07,
« Observer abandonné définitivement » (`vault/20-decisions.md` l.31) ; 17/10, le fondateur précise
que l'abandon ne visait que la « présence passive » (`vault/20-decisions.md` l.32) [VÉRIFIÉ]. 24/07 :
les 4 règles de la CARTE — « la graine = l'état vivant du réseau », « Vérifiable, partagé » ; la
capacité est un « TICKET D'ENTRÉE », « PAS un classement » (CARTE l.90-97) [VÉRIFIÉ]. 07/08 :
« TRANCHÉ (décision Malik, 07/08) : on ASSUME "le rôle tourne au rythme de l'activité" » (L1-002
l.205-206) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.36-43 : rôle reçu du réseau selon besoin, note, contexte ; imposé « au
dernier moment, et de façon imprévisible » ; formule non fixée [VÉRIFIÉ]. **18/10 — change la
position** : la véracité du rôle doit être « validé[e] avant même son émission », par « un
mécanisme de multi signature » des pairs « au nombre de 7 » — soit « le rôle est donné par
plusieurs et donc juste repris (le hash) », soit « avant la diffusion, une validation par les
noeuds qui entourent » [VÉRIFIÉ]. **Aucune source antérieure ne demande une validation par les
voisins avant émission** : c'est neuf. Le 18/10 garde aussi la sanction après coup : « Si un noeud
agit mal malgré cela, alors il a sa note qui se dégrade, et donc hors jeu, ou des tâches sans
importance » [VÉRIFIÉ]. Que le 18/10 « rapproche » Step2 (rôles annoncés, vérifiés par les pairs) et
wp-v1 (imprévisibles) est ma lecture [HYPOTHÈSE] : le verbatim ne cite ni l'un ni l'autre et ne
parle pas d'imprévisibilité. Les « signatures croisées de multiples rôles » de Step2 (l.118) sont un
critère de sécurité de la L1, pas une co-signature de rôle par les voisins [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Constat* : l'ADN du 17/10, lecture du code, écrit qu'« un nœud peut se
déclarer `Relay` : contredit "impossible à truquer" » (ADN l.65) [VÉRIFIÉ] — c'est un constat
d'écart, pas un résultat de red team. Les travaux d'entropie (L1-002, dans git le 07/08) montrent, en simulation,
qu'un tirage que chacun peut recalculer seul est re-tirable : « la masse seule ne protège PAS du
re-tirage » (L1-002 l.84) ; le mot « grindable » est dans L1-001 (l.327) et dans
`design-carnet-rendezvous-tournant.md` (l.91), pas dans L1-002, qui dit « grinding » [VÉRIFIÉ]. Ces
travaux portent sur la **graine**, pas sur l'annonce d'un rôle. Que l'un ou l'autre ait motivé les
paroles du 18/10 : **cause non écrite dans les sources** [HYPOTHÈSE]. *Décision du fondateur*
(18/10) : « Il faut agir avant » [VÉRIFIÉ].

**Verdict : ⚠️ tension — résoluble.** « Imprévisible » et « annoncé » ne s'opposent que si on les
place au même instant. **Lecture logique [HYPOTHÈSE] :** imprévisible *avant* le tirage (personne
ne peut se placer), co-signé par les voisins au moment où le rôle sert. Cette forme est
**analogue** — pas identique — à deux hypothèses de L1-002 qui portent sur la **graine**, pas sur
les rôles : H1 « Engagement avant révélation » (contribution « témoignée par ≥q de ses ~7 voisins
AVANT » que la graine soit calculable), « **NON prouvée** » ; H4 « co-signature quorum q-of-n » de
l'ancrage, « **À spécifier et prouver** » (L1-002 l.156-169) [VÉRIFIÉ]. Datation ambiguë dans
MISSION l.41 : « imprévisible » y est dit venir « du texte du 2026-06-26 » (date du premier jet de
MISSION, l.4) ; le mot est dans le brief daté du 2026-01-30 (l.65) et dans wp-v1, dans git dès le
02/02 (`2cbf495`) [VÉRIFIÉ]. Un lecteur peut croire à un ajout de juin.

**Écart code.** ⛔ Deux rôles seulement : `enum PeerRole { Peer, Relay }`
(`crates/tom-protocol/src/relay.rs:43-48`) [VÉRIFIÉ]. La promotion est un seuil fixe du score
local, 10 / 2 (`roles/manager.rs:13`, `:16`) [VÉRIFIÉ]. Un rôle annoncé par un pair, signé par
lui-même, est appliqué tel quel (`runtime/state.rs:922` vérifie la signature, `:943` applique
`announce.new_role` ; pour un pair inconnu, `:953-957`) [VÉRIFIÉ]. Le 18/10 ne refuse pas qu'un nœud
dise « je suis relais » ; il refuse que cette parole compte **sans** l'acquiescement des pairs. Le
code applique la parole seule : c'est l'écart. Le hub de groupe est le NodeId le plus bas, donc
prévisible (ADN l.68) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Un rôle diffusé porte la co-signature de q de ses ~7 voisins (format d'annonce
   = empreinte du rôle + signatures) ; un pair n'applique `new_role` que si q signatures valides de
   voisins distincts sont présentes. C'est la variante « validation avant diffusion » du 18/10 ; elle
   pourrait réutiliser le quorum déjà codé pour la présence (`presence/quorum.rs`) [HYPOTHÈSE de
   faisabilité]. Variante « certificat porté » : voir F1, option D.
2. **proposition** — Préciser MISSION l.41 (le mot « imprévisible » est dans le brief daté du
   2026-01-30) et trancher la question 4 du §8 (voir question F1).
3. **proposition** — Étudier si la co-signature du rôle et la co-signature d'ancrage de la graine
   (H4 de L1-002) peuvent être un seul mécanisme. C'est une idée de ce document [HYPOTHÈSE] ; la
   CARTE parle d'« un seul moteur » pour autre chose : un rôle tournant qui anime « le rendez-vous,
   les gardiens et l'accueil » (CARTE l.103-106) [VÉRIFIÉ].

---

## 4. Observation, témoins, murmuration ~7, gossip

**Au départ.** L'Observateur « surveille l'état de certains wallets ou subnets » (Step2 l.56) ;
il est au cœur de la double dépense (Step2 l.261-262) [VÉRIFIÉ]. Le consensus repose sur la
« participation au routage et à l'observation » (Step2 l.115-118) [VÉRIFIÉ].

**Trajectoire.** En février, le gossip est pensé comme découverte : « Les noeuds qui gossipent
maintiennent leur présence implicitement » (`results/CAMPAIGN-REPORT.md` l.188, rapport des 12-17/02)
[VÉRIFIÉ] — maintien de présence, pas preuve. Le 10/07, ADR-011 : la preuve est toujours
« dyadique (je t'ai vu faire) ou scopé-relais », jamais « tout le monde dit » (POP l.81)
[VÉRIFIÉ]. Le 11-12/07 : un appareil faible s'abonne à la vue signée de ses relais ; un pair n'est
`Online` que si ≥ N témoins distincts concordent, N entre 2 et 4 ; « mieux vaut sous-estimer la
présence que se faire eclipser » (L1-003 l.60) [VÉRIFIÉ]. Le 24/07, la CARTE importe la science
des étourneaux : chaque oiseau suit « un nombre FIXE de voisins (~7), pas une distance », sans
aucune vue globale (CARTE l.37-45) [VÉRIFIÉ]. Le 17/10 : « tout le monde s'observe (jusqu'à 7) »
(MISSION l.72) [VÉRIFIÉ].

**Aujourd'hui.** MISSION §1.9 (17/10) : les témoins — « tout le monde s'observe (jusqu'à 7) »
(MISSION l.72) [VÉRIFIÉ]. 18/10 : des pairs « au nombre de 7 » valident les rôles [VÉRIFIÉ]. Que ce
soient **les mêmes** 7 (témoins de présence = garants du rôle) est ma lecture [HYPOTHÈSE] ; aucune
source ne le dit. Si on l'adopte, le 18/10 **change** le rôle des 7 : de témoins (« je t'ai vu ») à
garants (« je confirme ton rôle »).

**Pourquoi ça a bougé.** ADR-011 (la présence par le travail) et le quorum de 2 à 4 ont deux causes
écrites — la première déclenche ADR-011 en entier, la seconde fonde le quorum lui-même (L1-003
l.54-58) : *terrain* — 44 à 50 pairs
« Online » alors que `connected_peers()` ≈ 0, sur une flotte de ~5 appareils Apple, à cause d'un bug
générique (tout pair découvert marqué `Online` sans preuve) (POP l.8-14) ; *sécurité* — « Ferme
l'eclipse par relais unique (kill-shot #3 red-team Fable) » (`vault/20-decisions.md` l.28)
[VÉRIFIÉ]. Le nombre 7 a une autre origine : la science des étourneaux (« 7 = l'optimum », CARTE
l.40-44) et la décision du fondateur du 17/10 (MISSION l.72) [VÉRIFIÉ]. Pourquoi passer de 2-4 à 7 :
**cause non écrite dans les sources**, en dehors de cette référence aux étourneaux.

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
  sont des contacts de première main ; les juger « conformes à ADR-011 » est ma lecture [HYPOTHÈSE]
  (POP l.70-78 définit le crédit de présence par le travail, pas la règle du statut `Online`, et
  un `PeerAnnounce` reste une déclaration). Le troisième s'en écarte nettement : une annonce reçue
  par gossip (`state.rs:904`) suffit à créer un pair `Online`, alors qu'ADR-011 dit « Jamais :
  annonce gossip, ré-annonce d'autrui, … auto-déclaration » (POP l.78) [VÉRIFIÉ].
- Aucune notion de « 7 voisins » dans le code (grep « murmuration / étourneau / starling » sur
  `crates/tom-protocol/src` : 0 occurrence) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Trancher « 7 = cible fixe » (question F2) et en dériver : chaque nœud tient
   ~7 voisins topologiques. Que le quorum de présence et le quorum de rôle soient le même q-of-7
   est une option [HYPOTHÈSE], à peser contre les tensions de vivacité, d'éclipse et de petite
   échelle (« Cohérence d'ensemble »).
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

**Trajectoire.** 06/02 (entrée dans git), verrou #4 : « The past exists but fades progressively »,
« Binary states: None — no "good" or "banned" status » (LOCKED l.100, l.107) [VÉRIFIÉ]. 22/02 : le
code introduit un score de contribution seule, décroissant de 5 % par heure, « no permanent bans »
(`f9dd21d` ; `roles/scoring.rs:1-8`) [VÉRIFIÉ] (la V1 disait « juin » : faux). 20/07, la charte :
« tu ne peux pas te servir du réseau si le réseau ne peut pas se servir de toi » ; la contribution
est la condition d'usage (charte l.121-125) [VÉRIFIÉ]. 21/07 : le relais ne crédite plus
l'origine d'un message — « usage ≠ contribution » (`2937330`) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.62-65 : « un jeton, ça peut être un grain de riz » ; une note interne,
pas de l'argent ni un actif ; équilibre contribution/usage [VÉRIFIÉ]. MISSION faisait déjà de la
note une entrée du rôle (« besoin, note et contexte », l.39-43) [VÉRIFIÉ]. 18/10 **ajoute** la note
comme sanction de celui qui agit mal malgré la validation : « hors jeu, ou des tâches sans
importance » [VÉRIFIÉ]. ⚠️ « Hors jeu » ressemble à une exclusion, que les verrous #4/#5 refusent
(« No exclusion », LOCKED l.152) ; si « hors jeu » veut dire « plus de rôle tant que la note est
basse » (le nœud reste client), il n'y a pas de conflit — c'est au fondateur de le dire.

**Pourquoi ça a bougé.** *Sécurité* : « L'ancien crédit ici gonflait le score du sender chez le
relais » (commentaire du diff `2937330`, 21/07) [VÉRIFIÉ] ; *décision du
fondateur* (grain de riz, « décision 2026-10-17, fondateur », MISSION l.63) [VÉRIFIÉ].
L'équilibre contribution/usage n'a jamais quitté les **textes** : Step2 l.130-139, wp-v1 §5.1-5.2,
brief l.67, charte l.121-126, MISSION l.64 [VÉRIFIÉ]. Il n'y a donc pas de « retour aux sources »
(V1) : seul le **code** ne l'a jamais appliqué.

**Verdict : ⚠️ tension.** La vision est cohérente. Mais le code mesure « qui m'a aidé », pas
« contribution − usage » : aucun terme d'usage émis (`bytes_received` entre dans un ratio de bande
passante, rendu inerte par l'absence de `bytes_relayed`) [VÉRIFIÉ]. Et wp-v1 l.315 prévoit qu'un
nœud suspect soit « mis sous surveillance renforcée ou exclu temporairement » [VÉRIFIÉ], ce que le
verrou #5 interdit (« No exclusion », LOCKED l.152) [VÉRIFIÉ] : contradiction ⛔ *dans les
documents*, jamais passée dans le code. Nuance : wp-v1 l.313-315 vise les fermes d'identités
(Sybil), le verrou #5 l'anti-spam [VÉRIFIÉ].

**Écart code.**
- Score = contribution seule, vue locale de chaque nœud, aucune note partagée (ADN l.64)
  [VÉRIFIÉ].
- `record_bytes_relayed` n'a aucun appelant hors tests (appels seulement dans
  `runtime/state.rs` après la ligne 3956, où commence `#[cfg(test)]`, et dans `roles/mod.rs`, test)
  [VÉRIFIÉ] : le terme « octets relayés » du score est inerte en production. Il avait un appelant de
  production jusqu'au 21/07 ; `2937330` l'a retiré parce qu'il créditait l'émetteur [VÉRIFIÉ].
- Le contexte de l'appareil (puissance, place, connexion fiable) n'entre pas dans la note
  (ADN l.66-67) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Mesurer localement ce que chaque pair **émet** (ce qu'il fait porter aux
   autres) à côté de sa contribution, selon la réponse à F8 ; ne pas compter la réception comme un
   usage à pénaliser (F8, défauts de l'option B).
2. **proposition** — Câbler `record_bytes_relayed` sur le chemin de relais réel, ou retirer ce terme
   (un terme inerte ment sur la formule).
3. **proposition** — Retirer « exclu temporairement » de wp-v1 l.315 (ou l'annoter « révoqué par le
   verrou #5 »), au titre de R4 : une décision révoquée se trace.

---

## 6. Anti-spam « arroseur arrosé »

**Au départ.** « Plus un utilisateur tente de saturer le réseau, plus le protocole lui renvoie
cette charge sous forme de travail local. » Trois outils : micro-PoW croissante, sur-assignation de
relais, tâches de validation non critiques ; « il s'épuise lui-même » (Step2 l.140-168) [VÉRIFIÉ].

**Trajectoire.** 06/02, verrou #5 : « Progressive, continuous reaction without hard thresholds »,
« No exclusion » (LOCKED l.123-152) [VÉRIFIÉ]. 03/03 : premier codage par limitation de débit selon
le score, de 2 à 50 messages/s (`b800dbe`) [VÉRIFIÉ]. 20/03 : relevé à 30-100 messages/s parce que
« Burst phase lost 30% messages due to antispam token bucket » (campagne Mac↔NAS, `f9ba76c`)
[VÉRIFIÉ] — cause *terrain* écrite. 06/07 : plafond global contre les essaims d'identités fraîches,
« finding #8 … un swarm d identites fraiches inonde en agregat » (`effc192`, texte sans accents)
[VÉRIFIÉ]. (La V1 citait
`vault/20-decisions.md` l.30 : c'est un autre plafond, anti-Sybil sur les réponses de présence des
identités `Known`.)

**Aujourd'hui.** MISSION l.262 range « ban permanent » parmi les interdits [VÉRIFIÉ]. Le 18/10
confie à celui qui agit mal « des tâches sans importance » ; Step2 disait « tâches de validation
non critiques » (l.161) [VÉRIFIÉ]. C'est proche, pas mot pour mot ; et le 18/10 en parle pour un
rôle usurpé, Step2 pour le spam. Le 18/10 ajoute « hors jeu », absent de Step2 (voir §5).

**Pourquoi ça a bougé.** Choix de la limitation de débit plutôt que la micro-PoW ou la
sur-assignation : **cause non écrite dans les sources** (le message de `b800dbe` dit seulement
« Design Decision #5 … progressive load, not exclusion ») ; « le plus simple à coder » (V1) est
[HYPOTHÈSE]. Changements de valeurs : *terrain* (`f9ba76c`). Plafond global : *sécurité*, un
« finding » de revue (`effc192`) [VÉRIFIÉ].

**Verdict : ⚠️ tension.** Le principe tient (aucun bannissement, rédemption possible). Mais le
code fait *ralentir* le spammeur ; il ne le fait pas *travailler*. Le cœur de l'image — « L'attaque
est transformée en contribution forcée » (`docs/article-ToM protocol.md` l.425) [VÉRIFIÉ] —
n'existe pas.

**Écart code.** Plancher de 30 messages/s, jamais zéro, plafond 100 (`roles/antispam.rs:68-70`) ;
commentaire « progressive load, not exclusion » (`:3-4`) [VÉRIFIÉ]. Aucune micro-PoW, aucune
sur-assignation : `grep` de « pow », « proof of work », « puzzle » vide dans `roles/` [VÉRIFIÉ].
Le budget est consommé sur l'expéditeur déclaré *avant* la vérification de signature : un pair
peut affamer une victime (CLAUDE.md, limite connue #13) — budget consommé l.1880 (`check_rate` sur `envelope.from`), signature vérifiée seulement l.1906 (`runtime/state.rs`) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Sur-assignation : un nœud dont l'équilibre est très négatif est tiré plus
   souvent comme relais intermédiaire (Step2 l.156-160). Cela pourrait réutiliser la sélection de
   relais existante [HYPOTHÈSE de faisabilité] et transforme l'abus en travail utile.
2. **proposition** — Imputer le budget au saut de connexion authentifié (piste déjà écrite pour
   #13 : « le porteur du spam paie »).

---

## 7. Amorçage, découverte, rendez-vous, zéro tiers, zéro dépense

**Au départ.** « Aucun serveur propriétaire n'est requis. Pas de serveurs volontaires ni de relais
externes » (v0.1 l.65-66) [VÉRIFIÉ]. Le Gardien aide les nouveaux à se synchroniser — un
« bootstrap dynamique » (Step2 l.57) [VÉRIFIÉ]. Le brief admet un seul compromis : un petit serveur
de signalisation, « the one acceptable temporary compromise » (brief l.164) [VÉRIFIÉ].

**Trajectoire.** Février : le serveur WebSocket de la démo est « The umbilical cord » ; état final
visé : « Zero fixed infrastructure. The network IS the infrastructure. The VPS is the umbilical cord
— cut when the baby breathes alone » (`_bmad-output/planning-artifacts/architecture.md` l.127, l.131,
dans git le 02/02) [VÉRIFIÉ]. Mars : infra maison, « Budget : 0€ (infrastructure maison) »,
« Pourquoi local d'abord ? … Gratuit (pas de VPS) », avec l'objectif de valider « en local (LAN)
avant migration prod » (`docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` l.4-6, l.207-209) [VÉRIFIÉ] : le
local est une **étape avant la prod**, pas un rejet du VPS. Une option C avec relais sur VPS est
rédigée (`docs/Brief Infrastructure Option C for GPT 5.3 CODEX.md` l.110) [VÉRIFIÉ] ; elle n'est pas
abandonnée par écrit : la roadmap garde « R17 — Seeds optionnels » sur « 1-2 VPS »
(`vault/40-roadmap.md` l.292), « explicitement retirables » (FEUILLE-DE-ROUTE l.49) [VÉRIFIÉ].
30/03 : l'amorçage est une « pompe » portée temporairement par le réseau, dont le rôle d'entrée est
rotatif (bootstrap-pump l.18-27, statut « cadrage ») [VÉRIFIÉ]. 22/06 : rendez-vous DHT partagé à 8
cases (`c34a3fd` ; `tom-dht/src/lib.rs:40`, `:44`) [VÉRIFIÉ]. 21/07 : le nœud stable 24/7 (Freebox)
« devient l'aspirateur » du carnet (787 inconnus) (`docs/plans/design-carnet-rendezvous-tournant.md`
l.21-22) ; verbatim du fondateur : le carnet « doit tourner demain et ne doit jamais être chez la
même personne » (même fichier l.10-11) [VÉRIFIÉ] ; ce document est une « PROPOSITION — à valider par
Malik AVANT tout code » (l.4). 24/07 : le premier jet « HRW + horloge » est abandonné, « un minuteur
mécanique trahissait l'esprit organique » (CARTE l.3-5) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.33 et l.265 : amorçage par la découverte externe actuelle, **zéro
dépense externe** pour l'amorçage et les relais ; étendre la règle à tout le fonctionnement, y
compris « les seeds VPS de R17 », est une question ouverte (MISSION l.291) [VÉRIFIÉ]. La passation
du 18/10 (résumé de l'orchestrateur) : « n0 actif par défaut dans l'app = incohérent pour Malik ; à
couper après un amorçage interne prouvé (sinon la 4G casse) » (PASSATION l.53) [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Décision du fondateur* : zéro dépense (17/10, MISSION l.33) ; carnet
tournant (verbatim, design-carnet l.10-11) [VÉRIFIÉ]. *Terrain* : le carnet concentré sur le nœud
stable relié à l'autopsie OOM (« cf. autopsie OOM », design-carnet l.22) [VÉRIFIÉ]. Que les VPS
aient été écartés pour raison d'économie : **cause non écrite dans les sources** — aucune ligne ne
dit que l'option C est abandonnée, ni pourquoi [HYPOTHÈSE : la règle « zéro dépense » du 17/10 la
met en suspens, la question restant ouverte en MISSION l.291]. Que tout point fixe diffusif
devienne un aspirateur « à un million de nœuds » est ma généralisation [HYPOTHÈSE].

**Verdict : ⛔ contradiction partielle, assumée provisoirement.** « Zéro dépense » tient (aucun
service payant). « Zéro tiers » : l'ADN dit « oui en LAN (sous réserve du multicast) » et, depuis
Internet, « seulement par un point d'entrée connu d'avance » (ADN l.50, l.56) [VÉRIFIÉ]. Ce point
d'entrée est « le relais du NAS à l'adresse publique de la Freebox », « l'infra du fondateur, pas
un tiers », mais « fixe » (ADN l.51) [VÉRIFIÉ]. ⚠️ C'est un **serveur fixe**, en tension avec v0.1
« Pas de serveurs volontaires ni de relais externes » (l.65-66) et avec le brief, qui n'admettait
qu'un point de coordination comme « the one acceptable temporary compromise » (brief l.164)
[VÉRIFIÉ]. Sans ce point connu d'avance, « aucun mécanisme du code ne permet une première rencontre
depuis Internet sans tiers » ; la DHT Mainline et le DNS de n0 sont « deux tiers gratuits » (ADN
l.53 ; la gratuité et la durée de n0 sont elles-mêmes en [HYPOTHÈSE] dans l'ADN, l.30) ; n0 voit le
réseau, en tension avec la souveraineté (ADN l.58) [VÉRIFIÉ]. Qualifier ces tiers d'« entorse »
au v0.1 est ma lecture [HYPOTHÈSE] : v0.1 vise « serveur propriétaire » et « relais externes ».

**Écart code.** `n0_discovery: true` par défaut (`crates/tom-transport/src/config.rs:172`) ;
`DEFAULT_RELAY_URLS` vide (`:12`) ; 8 cases fixes, pas de détenteurs tournants
(`tom-dht/src/lib.rs:44`) [VÉRIFIÉ]. La rotation du carnet n'est pas codée. Voir aussi §8 (n0 par
défaut dans l'app, README inexact).

**Pour passer un step.**
1. **proposition** — Exécuter C1 de l'ADN (mesurer l'amorçage sans n0 ni DHT publique sur la flotte
   réelle), puis couper n0 par défaut, ce que la passation rapporte comme souhait du fondateur
   (PASSATION l.53).
2. **proposition** — Le carnet tournant de la CARTE, porté par un « ancrage mince et tournant (le
   gardien/secrétaire du moment) », puisque deux inconnus « sans voisin commun » ne peuvent pas
   s'appuyer sur la seule vue locale (CARTE l.55-57) [VÉRIFIÉ] ; la DHT Mainline reste en secours du
   seul démarrage à froid (charte l.240-246) [VÉRIFIÉ].

---

## 8. Relais et transport

**Au départ.** Pas de relais externes (v0.1 l.65-66) ; pas de Bluetooth ni de Tor (v0.1 l.37-38) ;
un routage qui apprend les chemins courts et fiables « sans devenir dépendant d'un nœud
particulier » (v0.1 l.92-93) [VÉRIFIÉ].

**Trajectoire.** 06/02, ère WebRTC : ADR-001 « All messages transit through at least one relay.
Relays are not optional - they ARE the architecture » (`fbf4ba2:CLAUDE.md` l.54) [VÉRIFIÉ] ; la
formule est conservée lors du passage à QUIC (`CLAUDE.md` l.129). Février, le PoC iroh (09/02) :
« Le relay est un tremplin, pas une destination » (`docs/medium-article-poc.md` l.29, dans git le
24/02) [VÉRIFIÉ]. 11/02 : *plan* du fork d'iroh, « never depending on n0-computer's roadmap again »
(FORK-ARCHITECTURE l.8-11) ; le fork lui-même commence le 27/02 (`294ea5c`, tom-quinn) [VÉRIFIÉ]. 20/03 : relais
embarqué dans chaque nœud (`bd6c442`) [VÉRIFIÉ]. 22/06 : un relais n'est publié au gossip que s'il
est joignable de l'extérieur (`9d80e1b`, ADR-010) [VÉRIFIÉ]. 13/07 : mapping UPnP automatique
(`865a85c`) [VÉRIFIÉ]. 16/07, à propos d'une hypothèse de pare-feu IPv6 sur la box : « Décision
produit : on NE touche PAS les réglages box, le protocole doit trouver la parade »
(`docs/plans/CHECKPOINT-2026-07-16.md` l.31) [VÉRIFIÉ].

**Aujourd'hui.** Tout nœud joignable peut être relais ; « relays forward, they never store »
(README l.39, l.49 ; MISSION l.114) [VÉRIFIÉ]. La dernière source sur la répartition du travail est
la CARTE : « les puissants portent le lourd + les connexions directes rapides ; les faibles
reçoivent des tâches simples » (CARTE l.14-15) [VÉRIFIÉ] ; l'appliquer au relais est ma lecture.

**Pourquoi ça a bougé.** Qu'un relais de première rencontre soit nécessaire à cause des NAT et du
CGNAT : la source dit « First contact goes through a relay » (README l.49) mais n'écrit pas la cause
comme un changement de vision [HYPOTHÈSE]. « On NE touche PAS les réglages box » est une *décision
produit* [VÉRIFIÉ] ; qu'elle protège l'universalité (ne rien exiger de la box) est ma lecture
[HYPOTHÈSE] ; ⚠️ `tom-gateway`, outil
qui « configures a Freebox router (port forwarding) » (README l.360), est lui une réponse propre à
une plateforme [VÉRIFIÉ] — utile, mais pas universel.

**Verdict : ✅ tient**, avec une nuance de mots. Que « relais externe » (v0.1) et « relais
obligatoire » (ADR-001) ne se contredisent pas — le relais obligatoire étant un *nœud du réseau* —
est ma lecture [HYPOTHÈSE] ; ADR-001 ne le précise pas. Concentration : le 14/07, la roadmap notait
« SPOF réseau — seul le NAS tourne `--self-relay` », avec la réserve « statut réel non encore
confirmé » (`vault/40-roadmap.md` l.123, `5edf1cf`) [VÉRIFIÉ] ; le 17/10, l'ADN écrit que « le
relais embarqué tourne sur **tous** les appareils Apple » (ADN l.66) [VÉRIFIÉ]. Combien de relais
sont *publiés* et utilisés aujourd'hui : **non mesuré** — rien ne permet d'écrire au présent « un
seul nœud ».

**Écart code.**
- Relais = rôle `Relay` par seuil de score, pas par besoin (voir §3).
- ⛔ **n0 actif par défaut, partout.** `n0_discovery: true` dans la bibliothèque
  (`crates/tom-transport/src/config.rs:172`) **et** dans l'app tvOS/iOS
  (`apps/tom-node-tvos/TomNode/Services/TomNodeService.swift:164`, `n0Discovery: Bool = true`) ; le
  FFI ne l'applique que si l'app le fournit (`tom-protocol-ffi/src/lib.rs:345-346`) [VÉRIFIÉ]. Avec
  n0 actif et sans relais configuré, le nœud prend les presets n0 (Pkarr/DNS **et relais par
  défaut** de n0, `tom-connect/src/defaults.rs:22-35`) [VÉRIFIÉ] — des relais tiers, contraires à
  « pas de relais externes » (v0.1 l.65-66). L'ADN les juge « cas résiduel », « très probablement
  inutilisables » [HYPOTHÈSE dans l'ADN, l.31], et les builds du fondateur ajoutent un relais de
  secours qui évite ce cas (ADN l.26).
- ⛔ **Le README dit le contraire du défaut réel** : « Pkarr/DNS discovery (`n0_discovery`) is an
  optional preset, off by default in sovereign deployments » (README l.361) [VÉRIFIÉ]. Aucun
  « déploiement souverain » ne le désactive par défaut dans le code.
- L'en-tête `.tom.invalid` du TLS (SNI) est identifiable par une inspection de paquets ; arbitrage
  ouvert (charte l.260-265) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition (coût zéro)** — Corriger tout de suite la phrase du README l.361 pour dire le
   défaut réel (« activé par défaut ; à désactiver pour un déploiement souverain »), et ajouter dans
   l'app un réglage visible `n0Discovery = false` pour les tests d'amorçage interne (C1). Le défaut
   lui-même ne bascule qu'après C1 (sinon la 4G casse, PASSATION l.53).
2. **proposition** — Mesurer la concentration (part du trafic relayé par les plus gros nœuds),
   indicateur demandé par la charte (l.39-41) [VÉRIFIÉ], et la publier à chaque build.
3. **proposition** — Garder `tom-gateway` comme accessoire Freebox, jamais comme prérequis.

---

## 9. Purge, TTL, backup « virus », état présent, L1 miroir/ancre

**Au départ.** La L1 ne garde que l'état présent : « purge agressive », « genèse glissante »,
« état présent plutôt qu'historique » (Step2 l.64-81) [VÉRIFIÉ]. « Sans copie d'historiques
globaux » (v0.1 l.87-89) [VÉRIFIÉ].

**Trajectoire.** 06/02 (entrée dans git), verrou #2 : 24 h maximum puis « global purge occurs
regardless of delivery status » (LOCKED l.38-41) ; « >24h | Global purge — no exceptions, no
recovery, no debt » (LOCKED l.48) [VÉRIFIÉ]. 01/07 : MISSION écrit « TTL 24h max, purge globale sans
exception » (MISSION l.55, `fbf14cd`) [VÉRIFIÉ]. 06/07 : le mur n°4 — « l'anti-double-spend exige
que le témoin se souvienne "j'ai signé N→N+1" ; #2 purge tout à 24h → il oublie → re-spend » —
reçoit une porte par couches : « #2 gouverne le transport/messagerie », la valeur (L2) a « sa propre
règle : attestation persistante PAR WALLET », « exception explicite à #2, réservée à la valeur » ;
la même cellule porte « 🟢 ACTÉ (auteur 2026-07-06) : approche en couches acceptée » (MASTER-MAP
l.184, `90d5369`) [VÉRIFIÉ]. Le § 11 qualifie ses portes de « propositions à valider (Fable +
auteur), pas des vérités » (MASTER-MAP l.177, fin de ligne) [VÉRIFIÉ]. ⛔ **La MASTER-MAP se contredit en
interne** : l.23 (05/07, inchangée) dit « TTL — durée de vie 24h max, puis purge globale, sans
exception » ; l.184 (06/07) ouvre l'« exception explicite à #2 » [VÉRIFIÉ]. 18/07 : la règle des
24 h est étendue à la mémoire des pairs — « Tranché (Malik, 18/07) : 24 heures — cohérence stricte
"rien ne survit 24 h" », la proposition 7 jours écartée (anti-ravivage l.83-86) [VÉRIFIÉ]. 21/07 :
le carnet de contacts est proposé « borné ~30 » et « privé, local, durable » (design-carnet l.29-30),
dans un document au statut « PROPOSITION — à valider par Malik AVANT tout code » (l.4) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.262 : « Jamais de … persistance au-delà du TTL » ; l.55 « purge globale
sans exception » ; aucune mention de L2 [VÉRIFIÉ]. MISSION l.47 : les messages pour absents « se
répliquent sur des nœuds de secours » puis s'auto-détruisent [VÉRIFIÉ]. L'exception du 06/07 n'a
**jamais** figuré dans MISSION : la ligne l.55 date du 01/07, avant elle, et la réécriture du 17/10
ne l'a pas touchée [VÉRIFIÉ]. La V1 écrivait « n'y figure plus » : faux, rien n'a été retiré.

**Pourquoi ça a bougé.** Exception L2 : *sécurité*, le mur n°4 de la revue Fable (MASTER-MAP l.184
l'énonce) [VÉRIFIÉ]. Extension aux pairs : *terrain* — « 1286 pairs », « 951 peers queued » au
démarrage (anti-ravivage l.9-10), une « mémoire sans oubli » (l.40) ; le même document conclut que « le
"ravivage" de `last_seen` par gossip/DHT n'existe plus » (l.35) [VÉRIFIÉ]. La cause écrite de la
règle est la « cohérence stricte » (l.84), pas des fantômes ravivés (V1 : faux).

**Verdict : ⚠️ tension.**
- L'exception L2 « actée » le 06/07 n'a jamais été reprise par MISSION, et la MASTER-MAP elle-même
  se contredit : on ne sait pas si elle vit encore. Tant qu'il n'y a pas de L2, rien ne casse ; dès
  qu'on code la double dépense, il faudra trancher (F5). Garder une exception durable reviendrait à
  **affaiblir un verrou déclaré « final and non-negotiable »** (LOCKED l.3) ; le fondateur a une
  autre piste qui l'évite peut-être (F5, option C).
- « Durable » pour le carnet de contacts est compatible avec #2 **si** ce carnet est une donnée de
  l'application (le répertoire de l'utilisateur), pas un état du protocole [HYPOTHÈSE de lecture].

**Écart code.**
- TTL plafonné à 24 h à la création (`crates/tom-protocol/src/backup/types.rs:14` constante,
  `:86` plafonnement) [VÉRIFIÉ].
- ⛔ **Le « virus » ne se propage pas en production.** La seule fonction qui envoie une copie de
  secours à un autre nœud est `replicate_to` (`backup/coordinator.rs:218`, unique producteur de
  `BackupAction::Replicate`, l.248) ; elle n'est appelée que dans des tests (`coordinator.rs`
  après l.310 `#[cfg(test)]`, `backup/mod.rs` test, `tests/backup_integration.rs`). Les événements
  `ReplicationNeeded`, `SelfDeleteRecommended` et `MessageReplicated` sont jetés (`runtime/state.rs:3865-3867`). Le
  runtime sait *recevoir* une réplique (`state.rs:1634-1637`) mais aucun nœud n'en *envoie*.
  [VÉRIFIÉ] En pratique, le gardien est l'émetteur lui-même (backup créé sur échec d'envoi). Le
  README parle d'un message qui « self-replicates across backup nodes » (README l.52) [VÉRIFIÉ] :
  la documentation est en avance sur le code. Le `CLAUDE.md` du projet (limite #14) affirme aussi
  que « l'émetteur réplique aussi vers un gardien TIERS G » : le code ne le fait pas [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Câbler la réplication sortante (déclencheur : viabilité de l'hôte en baisse ou
   destinataire absent depuis X) avec les trois conditions de F6 : gardiens 24/24 dont le rôle est
   validé, nombre de copies fixe par message (borne globale, pas seulement le budget local de
   64 Mio), purge des copies tierces à la livraison (`ConfirmDelivery` conditionnel, CLAUDE.md,
   limite #14).
2. **proposition** — En attendant, corriger le README (l.52) et CLAUDE.md limite #14 : « l'émetteur
   garde la copie » est la vérité actuelle. MISSION l.47 est une **cible** (§1, « Non négociable »,
   l.13) : ne pas la réécrire, au plus ajouter une note d'état hors du §1.
3. **proposition** — Inscrire dans MISSION le sort de l'exception L2 (question F5) — elle n'y a
   jamais figuré.

---

## 10. Subnets, cercles réels, vases communicants

**Au départ.** Des sous-réseaux naissent quand des échanges deviennent récurrents — famille,
équipe, événement — puis fusionnent, se divisent, se mettent en veille, s'éteignent ; une même
personne relaie entre plusieurs, « sans copie d'historiques globaux » (v0.1 l.80-89) [VÉRIFIÉ].
Le fork d'un subnet surchargé est une « respiration » (Step2 l.83-94) [VÉRIFIÉ].

**Trajectoire.** Le code construit des grappes BFS sur le graphe des messages : arête ≥ 3
messages, taille 3 à 10, dissolution après 5 minutes (`discovery/subnet.rs:13-27`) [VÉRIFIÉ].
20/07 : « Jamais exercé par un banc à ce jour » (prisme-des-roles l.37-38) [VÉRIFIÉ] ; dès le 21/07,
un banc « R5 subnets » l'exerce (`6a55f89` ; `crates/tom-stress/src/scenario_roles_charge.rs:11`)
[VÉRIFIÉ].

**Aujourd'hui.** MISSION l.76-79 (17/10) : pas de règle fixe ; « chacun a un répertoire de
contacts et parle très rarement en dehors. Un subnet = ce cercle réel » ; un petit subnet s'appuie
sur un plus robuste — vases communicants [VÉRIFIÉ].

**Pourquoi ça a bougé.** *Décision du fondateur* (« décision 2026-10-17, fondateur », MISSION l.77)
[VÉRIFIÉ] ; le pourquoi : **cause non écrite dans les sources**. Y voir un « retour aux sources
partiel » est ma lecture [HYPOTHÈSE] : « famille, équipe, événement » (v0.1 l.81-82) est déjà un
cercle réel, mais v0.1 le fait naître de la **récurrence des échanges** et mourir sur inactivité —
c'est-à-dire du trafic, ce que fait le code ; MISSION l.78 le rattache au répertoire de contacts.

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

**Trajectoire.** 06/02 : la première pile écrite est TweetNaCl.js, « X25519 key exchange,
XSalsa20-Poly1305 » (`fbf4ba2:CLAUDE.md` l.65) [VÉRIFIÉ]. 18/02 : la pile Rust Ed25519 + X25519 +
XChaCha20-Poly1305 est décidée (rust-port-kickoff, tableau « Locked Decisions ») [VÉRIFIÉ] ; clé
publique = adresse (ADR-005). L'article daté « Février 2026 » assume : « ToM ne cache pas qui parle
à qui — les champs `from` et `to` sont en clair (nécessaire au routage) » (`docs/article-ToM
protocol.md` l.752) [VÉRIFIÉ]. 26/06 : l'audit note « pas de PCS/ratchet (assumé v1) »
(`docs/audits/AUDIT-2026-06-26.md` l.37) [VÉRIFIÉ].

**Aujourd'hui.** Inchangé ; la charte demande de prouver formellement FS et PCS (charte l.266-268) [VÉRIFIÉ].

**Pourquoi ça a bougé.** Le passage TweetNaCl → pile Rust suit la bascule Rust (§2). Pour le
reste, presque pas : le caractère en clair de `from`/`to` est écrit dès février. Y voir une « prise
de conscience » est ma lecture [HYPOTHÈSE].

**Verdict : ⚠️ tension.** « Pas de collecte centrale de métadonnées » (v0.1) tient — il n'y a pas
de centre. Mais chaque relais voit `from`/`to`, et la DHT publique voit qui s'annonce. À un million
de nœuds, un relais curieux voit beaucoup [HYPOTHÈSE d'échelle].

**Écart code.** Les attestations de présence sont signées et **jamais chiffrées**, pour être
vérifiables par des tiers (`presence/attestation.rs:3-5`) [VÉRIFIÉ] : choix assumé.

**Pour passer un step.**
1. **proposition** — Une relecture cryptographique extérieure avant tout engagement. VISION-CONSOLIDEE
   l.78 la demande pour la cryptographie *avancée* (VDF, beacons, signatures-seuil : « une revue
   externe par un cryptographe avant tout engagement ») [VÉRIFIÉ]. ⚠️ Un cryptographe payé est une
   **dépense externe** et une dépendance, en tension avec « zéro dépense externe » (MISSION l.33,
   l.265 — dont la portée au-delà de l'amorçage et des relais est ouverte, l.291). Voies gratuites,
   à essayer d'abord : publier les specs (L1-001/002/003) et les simulations ; demander une relecture
   académique ouverte (laboratoires, séminaires, prépublication) ; un « bug bounty » non monétaire
   (crédit public, nom dans le dépôt).
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
traversent le FFI et arrivent au SDK ; `SenderThrottled` atteint même le fil d'activité de l'app. CLAUDE.md (limite #8, l.468) affirme le contraire ;
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
from_commit » (Step2 l.261-262) [VÉRIFIÉ]. Les observateurs forment « un mini-subnet d'observateurs
(Obs_W = {O1, O2, …, Ok}) » propre à chaque wallet, qui « gardent localement le dernier engagement
connu Commit_W » (Step2 l.216-219) ; « chaque observateur garde en mémoire son dernier Commit_W pour
W » (l.272) [VÉRIFIÉ]. L'historique « quitte la L1 » et vit chez le propriétaire et ses observateurs
(Step2 l.317-318 ; wp-v1 l.179) [VÉRIFIÉ]. Et une perspective : « une voie vers une monnaie ou un
jeton utilitaire natif », sans « minage industriel » ni « rente » (Step2 l.369-372) [VÉRIFIÉ].

**Trajectoire.** Le brief (daté du 30/01) : « No speculative token … no convertible asset » (brief
l.58), « No token to buy, sell, or speculate on » (brief l.67) [VÉRIFIÉ]. Juillet : la valeur
devient la couche L2, « bonus, dernières étapes » (MASTER-MAP l.40) ; « la double dépense est une
propriété de la couche d'ATTESTATION (L1), pas du transport » (MASTER-MAP l.42) ; sécurité
« probabiliste » (MASTER-MAP l.111) ; « Étage 2 — La valeur … (2028+, conditionnel) … Si M1.2/M1.4
ne tiennent pas, L2 est abandonné sans honte » (VISION-CONSOLIDEE l.35) [VÉRIFIÉ].

**Aujourd'hui.** MISSION l.62-65 : pas de frais, pas d'actif convertible ; l.74 : double dépense,
chantier ouvert, pas d'argent dans le périmètre [VÉRIFIÉ].

**Pourquoi ça a bougé.** Deux temps à séparer. (1) **05/07** : la valeur est rangée en couche L2,
« bonus, dernières étapes », dès la première version de la master map (`bcd8978`, l.38) [VÉRIFIÉ] —
**avant** la revue Fable du 06/07. Pourquoi : **cause non écrite dans les sources** [HYPOTHÈSE : la
séparation en couches, posée le même jour (« la double dépense est une propriété de la couche
d'ATTESTATION (L1), pas du transport (L0) », l.40)]. (2) **06-16/07** : la L2 devient
**conditionnelle**, avec une cause écrite, *sécurité* : la revue Fable du 06/07 place l'entropie en
mur n°1 et le Sybil en mur n°2, « Le plus grand trou non résolu reste #1+#2 » (MASTER-MAP l.188) ;
« C'est le verrou qui conditionne toute notion de « valeur » (L2) » (FEUILLE-DE-ROUTE l.81, 16/07) ;
« (2028+, conditionnel) » (VISION-CONSOLIDEE l.35, 16/07) [VÉRIFIÉ]. « Pas d'argent » : *décision
du fondateur* du **17/10** (MISSION l.63, l.74) [VÉRIFIÉ] — trois mois après le passage en L2, ce
n'est donc pas la cause de celui-ci. Pourquoi le fondateur a sorti l'argent du périmètre : **cause
non écrite dans les sources**.

**Verdict : ⚠️ tension.** La règle d'or de Step2 est saine ; la juger compatible avec le verrou #3
(« L1 observes and anchors — it does not arbitrate ») est ma lecture [HYPOTHÈSE] : ce n'est pas la
L1 qui arbitre, c'est chaque observateur qui refuse de signer deux fois. Mais elle exige que chaque
observateur **se souvienne** du dernier état du wallet (Step2 l.272) — c'est le mur n°4 lui-même
(oubli à 24 h, §9). Et la perspective « monnaie ou jeton utilitaire natif » (Step2 l.369-372) :
je n'ai trouvé **aucune révocation explicite** (grep « monnaie » dans MISSION, ADN,
VISION-CONSOLIDEE, MASTER-MAP : 0) ; le brief (« no convertible asset ») et MISSION (« pas d'actif
convertible », « Jamais de … jeton-actif », l.64, l.262) l'excluent par leurs termes [VÉRIFIÉ]. Une
décision révoquée doit se tracer (R4) : il manque une phrase qui dise « abandonné » ou « en
sommeil ».

**Écart code.** Rien : aucune occurrence de wallet, ledger ou double dépense dans
`crates/tom-protocol/src` (grep « wallet / ledger / double spend / double dépense » : 0 occurrence) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Tracer explicitement le sort de « monnaie à terme » (question F4).
2. **proposition** — Quand le chantier rouvrira, ne pas confondre deux objets [HYPOTHÈSE de
   conception à trancher] : les **observateurs d'un wallet** (Step2 : un groupe *par wallet*, dont
   la mémoire du dernier état doit durer tant que le wallet vit) et les **~7 voisins topologiques**
   d'un nœud (CARTE, 18/10 : ils changent avec le churn, au gré des connexions). Faire des seconds
   les premiers suppose de résoudre ce que devient la mémoire du wallet quand le voisinage change,
   et le mur n°4 (oubli à 24 h). Voir F5.

---

## 14. Consensus, L1, genèse glissante

**Au départ.** Consensus par présence active : quorums tournants pseudo-aléatoires, signatures
croisées de multiples rôles ; « ce sont ceux qui sont là, qui relaient et qui observent, qui
valident » (Step2 l.100-120) [VÉRIFIÉ]. Genèse glissante, snapshots engagés (Step2 l.72-77)
[VÉRIFIÉ].

**Trajectoire.** wp-v1 : « des milliers de participants, sélectionnés en cascade au dernier
moment » (wp-v1 l.15) [VÉRIFIÉ]. Verrou #3 : « L1 observes and anchors — it does not arbitrate »
(LOCKED l.62-66) [VÉRIFIÉ]. L'article daté « Février 2026 » va jusqu'à « il n'y a pas de consensus à
corrompre » (`docs/article-ToM protocol.md` l.662) et « pas de vote — l'ordre déterministe élimine
le besoin » (l.370, à propos du hub) [VÉRIFIÉ]. Juillet : 3 couches ; L1 = couche d'attestation
(MASTER-MAP l.40-42). Août : l'entropie ; « la masse seule ne protège PAS du re-tirage » (L1-002
l.84, simulation) [VÉRIFIÉ].

**Aujourd'hui.** MISSION ne parle pas de consensus (grep : 0) [VÉRIFIÉ]. La CARTE : « La L1 tient
le miroir (l'ancre de l'état présent), elle n'orchestre JAMAIS » (CARTE l.11) ; la formule « L1 =
miroir jamais chef » est celle du journal `vault/30-discoveries.md` l.16 [VÉRIFIÉ].

**Pourquoi ça a bougé.** Que la messagerie n'ait pas besoin de consensus parce que l'ACK et le TTL
suffisent : **cause non écrite dans les sources** [HYPOTHÈSE]. *Sécurité* : le consensus de la
valeur bute sur l'entropie (MASTER-MAP l.188 ; FEUILLE-DE-ROUTE l.81 ; L1-002 l.84) [VÉRIFIÉ].

**Verdict : ⚠️ tension — la logique tient peut-être, la preuve manque.** Lecture proposée
[HYPOTHÈSE] : une fois les couches séparées, L0 (messagerie) n'a pas de consensus et n'en veut pas ;
L1 ne fait qu'ancrer des faits co-signés ; le refus de la double dépense est local (règle d'or).
Mais en février les textes se contredisent vraiment : wp-v1 dit que le consensus « repose sur la
masse » de « milliers de participants » (l.15), l'article qu'« il n'y a pas de consensus à
corrompre » (l.662, en réponse à un attaquant qui contrôle > 50 % des nœuds) [VÉRIFIÉ] ; aucune
source ne les réconcilie par L0/L1 — c'est ma lecture. Le vrai trou est ailleurs : l'entropie non
biaisable reste ouverte (L1-002 ; H1 « NON prouvée », l.156-159 ; « problème de recherche »,
FEUILLE-DE-ROUTE l.81) [VÉRIFIÉ].

**Écart code.** Pas de L1 au sens de Step2. Dans le code, « L1 » désigne les specs de présence
(L1-001/002/003). Une attestation vit 30 s (« ephemeral: 30s lifetime », `presence/attestation.rs:6`) ;
la graine en agrège, et le choix du sous-ensemble par l'agrégateur reste ouvert (« subset-selection
grinding is OPEN … Do not claim non-biasability here », `presence/aggregator.rs:9-12`) [VÉRIFIÉ].

**Pour passer un step.**
1. **proposition** — Fixer le vocabulaire : « L0 = transport sans consensus ; L1 = ancrage de faits
   co-signés ; jamais d'arbitrage » dans MISSION.
2. **proposition** — Revue externe d'L1-002 par un cryptographe (garde L1.2bis déjà prévue,
   L1-002 l.185-198) [VÉRIFIÉ] — par les voies gratuites d'abord (§11, proposition 1).

---

## Thèmes ouverts par les lecteurs (15-19)

Les lecteurs ont ouvert cinq thèmes hors liste ; ils sont rangés à part dans l'agrégat :
livraison et ACK (verrou #1), méthode et gouvernance, adoption et distribution, groupes, architecture
interne. Le seul qui touche la vision est **la gouvernance** : « le réseau finit par héberger son
propre code » (VISION-CONSOLIDEE l.37) et « la gouvernance (rotation, pas de capture) »
(VISION-CONSOLIDEE l.100) [VÉRIFIÉ] — la source ne dit pas *qui* tourne (la V1 écrivait
« mainteneurs en rotation » : glose) ; et le risque
« bus factor = 1 » (VISION-CONSOLIDEE l.86) [VÉRIFIÉ]. Il n'est pas traité ici (hors des 14 thèmes
du mandat).

---

## Cohérence d'ensemble

**Une hypothèse de lecture : un seul moteur [HYPOTHÈSE — c'est ma lecture, pas le 18/10].** On
peut lire la vision comme convergeant vers une seule mécanique : **le travail constaté par un
voisinage d'environ sept nœuds**, qui prouverait la présence, co-signerait le rôle, nourrirait la
note, sur-assignerait l'abuseur et, un jour, refuserait la double dépense. Ce que les sources disent
vraiment : le 18/10 confie aux 7 pairs la **validation des rôles**, et rien d'autre [VÉRIFIÉ] ;
ADR-011 fonde la présence sur un travail constaté « dyadique … ou scopé-relais », sans voisinage
de 7 (POP l.81) [VÉRIFIÉ] ; la CARTE parle d'« un seul moteur » pour **un rôle tournant** qui anime
« le rendez-vous, les gardiens et l'accueil » (CARTE l.103-106), et dit que **se rencontrer** entre
inconnus « sans voisin commun » exige « un ancrage mince et tournant », pas la vue locale (CARTE
l.55-57) [VÉRIFIÉ] ; Step2 confie la double dépense à un groupe d'observateurs **par wallet**
(l.216) [VÉRIFIÉ]. La fusion présence + rôle + note + double dépense reste donc une piste à
éprouver, pas un acquis. Elle a quatre tensions, que ce document ne résout pas à la place du
fondateur :
1. **Vivacité.** Une co-signature q-sur-7 exige que q voisins soient **en ligne** au bon moment. Or
   le mobile n'a pas de tâche de fond, seulement des sessions au premier plan (FEUILLE-DE-ROUTE
   l.17 ; CLAUDE.md limite #4) [VÉRIFIÉ]. Avec des sessions courtes, combien des 7 sont là quand il
   faut signer ? Non mesuré.
2. **Sybil / éclipse.** Qui choisit les 7 ? Si l'attaquant se place dans le voisinage d'une
   victime, il tient sa vue entière : « un relais Sybil unique = toute la vue de présence d'une
   victime faible (eclipse) » (L1-003 l.56) ; « ferme patiente de N nœuds qui rafle le quorum »
   (MASTER-MAP l.182) [VÉRIFIÉ]. **Un moteur unique devient un point de défaillance unique** : qui
   capture un voisinage capture à la fois la présence, le rôle, la note et (un jour) la valeur.
3. **Petite échelle.** Sous 8 nœuds, 7 voisins n'existent pas ; la CARTE prévoit un « Tour de garde
   / cooldown en filet pour les petits réseaux peu mouvants » (CARTE l.95) et le fondateur assume
   « petit c'est dur voire impossible » (18/10) [VÉRIFIÉ]. Comment le réseau naît de 2 à 7 nœuds
   reste à écrire.
4. **Mémoire.** Les observateurs d'un wallet doivent se souvenir de son dernier état (Step2 l.272) ;
   les voisins topologiques changent avec le churn, et le réseau oublie à 24 h (verrou #2, mur n°4
   de MASTER-MAP l.184) [VÉRIFIÉ]. « Observateurs d'un wallet = ses ~7 voisins » (V1) confondait
   deux objets.

**La vision est plus cohérente que ses documents — mais pas sans contradiction de fond.**
Contradictions entre textes : « imprévisible » vs « annoncé » (résoluble, §3) ; « exclu
temporairement » vs « jamais d'exclusion » (§5) ; « hors jeu » (18/10) vs « No exclusion » (§5) ;
MASTER-MAP l.23 « sans exception » vs l.184 « exception explicite à #2 » (§9) ; exception L2
jamais reprise par MISSION (§9) ; README « off by default » vs `n0_discovery: true` (§8) ;
datation ambiguë dans MISSION l.41 (§3). **Une contradiction touche le fond** : le volet valeur
de Step2 (paiement, monnaie à terme) contre « pas d'argent » de MISSION (§1, §13), sans révocation
écrite.

**Le code est en retard sur deux points de nature, pas de degré.**
1. **Les rôles comptent sans validation** (`state.rs:943`) : un nœud qui dit « je suis relais » est
   cru sur parole ; le 18/10 exige l'acquiescement des pairs avant.
2. **Le virus ne se propage pas** (`replicate_to` sans appelant) : la résilience par réplication
   annoncée par MISSION §1.3 n'existe qu'en tests.
Les autres écarts (témoins 2-4, subnets à bornes fixes, n0 et ses relais par défaut, événements
internes exposés par le SDK) sont, à mon avis, des écarts de degré [HYPOTHÈSE de classement].

**La tension centrale est la masse.** « Plus il y a de monde plus le réseau est fort » (18/10)
n'est **ni démontré ni réfuté par une mesure** : la charte dit que « "plus de relais = plus de
débit" est une hypothèse de conception, pas encore un chiffre » (charte l.254-255) [VÉRIFIÉ]. Les
**simulations** d'entropie disent que contre un attaquant de taille fixe la masse écrase son
avantage, et que contre un attaquant proportionnel « la masse n'aide pas » (L1-002 l.89-98)
[VÉRIFIÉ] ; la charte le dit « vrai SOUS » quatre conditions d'architecture et « faux sinon »
(charte l.90-91) [VÉRIFIÉ]. Ce sont deux conditionnements différents de la même thèse. La vision
tient si l'on dit : **la masse amplifie, les règles locales protègent** — formule de ce document
[HYPOTHÈSE], qui ne figure pas dans le 18/10.

**La plateforme a façonné les pratiques ; la vision, peut-être pas — deux points à surveiller.**
Pas de tâche de fond mobile : déclenchée par iOS ⚠️, et le refus du push et du réveil permanent est
justifié par écrit par la vision (« ce serait contraire à la vision », CLAUDE.md limite #4) [VÉRIFIÉ], mais en **tension** avec v0.1 : « le temps passé à
l'utiliser travaille pour vous » (l.22-23) va dans le sens ; « connecté en permanence » (l.18) va
contre. Échantillon Apple + une Freebox : qu'il oriente les SDK et les outils (`tom-gateway`) est
ma lecture [HYPOTHÈSE]. Pour éprouver la vision à grande échelle, L1-002 demande « simulation
étendue ET banc réseau réel » (L1-002 l.185-187) [VÉRIFIÉ] : ni l'un ni l'autre seul.

---

## Questions que seul le fondateur peut trancher

Chaque question donne les options et une recommandation. Aucune n'est tranchée ici. Les questions
déjà posées dans MISSION §8 ne sont rappelées que si ce document apporte un fait nouveau.

**F1. Rôles : imprévisible, annoncé, ou validé avant émission ?** (MISSION §8 q.4)
- A. Imprévisible pur (wp-v1 l.211-217) : personne ne connaît les rôles à l'avance.
- B. Annoncé à l'avance (Step2 l.106-108) : quatre rôles publiés « quelques blocs » avant.
- C. Imprévisible jusqu'au tirage, puis annoncé et co-signé par les ~7 voisins avant de prendre
  effet. Défaut : pendant la fenêtre entre l'annonce et la prise d'effet, le porteur est **connu
  et ciblable** — c'est ce que wp-v1 veut éviter (« Personne ne peut se positionner à l'avance »,
  l.217) [VÉRIFIÉ]. Si on garde C, la fenêtre doit être bornée à la durée d'un tour de
  co-signature (quelques allers-retours avec les voisins, de l'ordre de la seconde), jamais
  « quelques blocs » [HYPOTHÈSE de dimensionnement, à mesurer].
- D. **Certificat de rôle porté dans le message.** Les voisins co-signent le rôle (« le rôle est
  donné par plusieurs et donc juste repris (le hash) par l'acteur », 18/10) ; le nœud joint ce
  certificat **au message qu'il émet** en exerçant le rôle. Pas d'annonce préalable : le rôle
  n'est visible qu'au moment où il sert, et il est déjà vérifiable. Cela colle au mot du
  fondateur : « validé avant même son émission » — pas « annoncé quelques tours avant » (mandat
  M0017, point 20) [VÉRIFIÉ pour le verbatim ; le mécanisme est une proposition].
- **Recommandation : D** pour les rôles exercés au contact (garder une copie, témoigner, relayer
  un message pour un pair déjà en lien). Limite : un rôle qu'il faut **trouver** avant usage (un
  relais de première rencontre, un gardien de rendez-vous) doit être annoncé ; pour ceux-là, C avec
  une fenêtre courte. ⚠️ Trou commun à C et D : un tiers qui reçoit le certificat ne sait pas si les
  signataires sont **vraiment** les voisins du nœud — sept identités Sybil peuvent signer un
  certificat valide. D ne vaut que si la règle « qui devient voisin » (F2, réserve 3 ; « Cohérence
  d'ensemble », tension 2) est résolue. Les deux variantes du
  18/10 correspondent à D (« donné par plusieurs … repris ») et à C (« avant la diffusion, une
  validation ») [HYPOTHÈSE de lecture]. Ni D ni C ne sont « exactement » H1/H4 de L1-002, qui
  portent sur la graine et sont non prouvées (§3). Corollaire : préciser MISSION l.41
  (« imprévisible » figure dans le brief daté du 2026-01-30).

**F2. Les 7 voisins : cible fixe ou plafond ?** (MISSION §8 q.5)
- A. Plafond (« jusqu'à 7 », MISSION l.72) : de 2 à 7 selon la densité.
- B. Cible fixe (~7 voisins topologiques, CARTE l.40-41 : « robuste que la nuée soit dense ou
  clairsemée ») : toujours environ 7, quelle que soit la densité.
- **Recommandation : B**, avec trois réserves écrites dans la règle : (1) sous 8 nœuds, filet
  « tour de garde » (CARTE l.95) ; (2) le seuil q se fixe par simulation **et** par la vivacité
  mesurée (combien des 7 sont en ligne quand on en a besoin, mobiles compris) ; (3) une règle sur
  **qui** devient voisin, pour qu'un attaquant ne puisse pas s'installer autour d'une victime
  (voir « Cohérence d'ensemble », tensions 1 à 3). Attention à la règle de priorité : MISSION
  (17/10, « jusqu'à 7 ») est plus récente que la CARTE (24/07, « nombre FIXE ») ; B ne s'appuie que
  sur le « au nombre de 7 » du 18/10, qui donne un nombre, pas « fixe ». Que B soit « ce que vise le
  18/10 » est donc ma lecture [HYPOTHÈSE] — au fondateur de dire si 7 est un plafond ou une cible.

**F3. Comment écrire « plus il y a de monde, plus c'est fort » ?**
- A. Tel quel (c'est la phrase du fondateur, 18/10).
- B. « La masse amplifie ; les règles locales (témoins, note, quorum) protègent. »
- C. Garder la phrase du fondateur comme **but**, et écrire dessous la condition : « … à condition
  que les règles locales tiennent ; sans elles, un attaquant qui grandit avec le réseau garde son
  avantage ».
- **Recommandation : C.** Elle ne réécrit pas la parole du fondateur, et elle ne promet pas plus
  que ce qui est établi. Ce qui est établi n'est pas une mesure : ce sont des **simulations**
  (EXP-3, EXP-4 de L1-002) qui supposent elles-mêmes H1 (« ne valide pas H1, il la SUPPOSE »,
  L1-002 l.72-79) [VÉRIFIÉ], et qui ne portent que sur le **tirage de la graine** : elles ne
  disent rien de la capacité ni de la résilience. La condition de C vaut donc pour l'attaque du
  tirage ; pour le reste, la charte pose ses quatre conditions (charte l.90-91).

**F4. La « monnaie ou jeton utilitaire natif à terme » de Step2 (l.369-372) est-elle révoquée ?**
- A. Révoquée : ToM ne portera jamais de valeur convertible.
- B. En sommeil : une L2 de valeur possible plus tard, seulement si L1 tient (VISION-CONSOLIDEE
  l.35). Attention : cette source parle d'un « portefeuille scellé », pas d'une monnaie, et dit
  « incitation par l'accès (héberger = profiter), jamais par le jeton » (l.37) [VÉRIFIÉ].
- C. Maintenue comme objectif.
- D. **Le « jeton utilitaire » de Step2 = la note / grain de riz de MISSION l.62** (non
  convertible, pas un actif). La contradiction disparaît sans rouvrir une monnaie. Différence à
  dire : dans Step2, les jetons « On en gagne en contribuant, on en dépense en utilisant le
  réseau » (l.138) — ils se dépensent ; la note de MISSION est « une mesure d'équilibre » (l.64),
  elle ne se dépense pas [VÉRIFIÉ].
- **Recommandation : D**, plus « **pas de jeton-actif natif** » (MISSION l.262), écrits noir sur
  blanc dans MISSION (R4 : une décision révoquée se trace). Pas plus loin : MISSION laisse ouverte
  la question de l'argent — double dépense « hors périmètre **tant qu'il n'y a pas** d'argent », « à
  quel moment il redevient un sujet » (MISSION l.74, l.285) [VÉRIFIÉ] — et l'hypothèse du fondateur
  citée en F5 parle de « dépenser ou encaisser de l'argent » avec l'aval du réseau. Fermer toute
  valeur (option A) contredirait donc MISSION et F5. Transporter ou garder de la valeur reste
  ouvert (option B, et F5).

**F5. L'exception au TTL de 24 h pour L2, « actée » le 2026-07-06 (MASTER-MAP l.184), vit-elle
encore ?** MISSION l.55 dit « purge globale sans exception », l.262 « jamais de persistance au-delà
du TTL » ; la MASTER-MAP se contredit elle-même (l.23 contre l.184).
- A. Révoquée : même la valeur obéit à 24 h. Aucune source ne dit comment la double dépense se
  résoudrait alors : en l'état, A revient à abandonner la règle d'or de Step2, qui exige que
  chaque observateur garde « son dernier Commit_W » (Step2 l.272) [VÉRIFIÉ].
- B. Maintenue, scopée à L2, inactive tant que L2 n'existe pas. ⚠️ C'est **affaiblir un verrou
  déclaré « final and non-negotiable »** (LOCKED l.3) [VÉRIFIÉ] ; à ne faire qu'en le disant.
- C. **L'hypothèse du fondateur** (verbatim, description de son projet, 2026 ; cité par le mandat
  M0017, point 22) : « une hypothèse d'architecture où chaque utilisateur sauvegarde lui-même ses
  sorties et ses entrées et le réseau garderait un Snap à Hash de son portefeuille, il ne pourrait
  dépenser ou encaisser de l'argent seulement avec l'aval du réseau ». L'historique reste chez
  l'utilisateur (comme Step2 l.317-318 : « l'historique quitte la L1 ») ; le réseau ne tient
  qu'une empreinte de l'état présent. Elle **vise** à éviter toute persistance réseau au-delà de
  24 h. Deux points à éclaircir [HYPOTHÈSE] : (1) « le réseau garderait » une empreinte ; pour
  rester dans les 24 h, elle doit être **ré-ancrée** à chaque fenêtre, sinon un wallet inactif plus
  de 24 h perd son ancre — mais un ré-ancrage sans fin est une persistance par renouvellement,
  le « ledger creep » que VISION-CONSOLIDEE redoute (« Chaque exception de persistance "juste pour
  ce cas" est un pas vers le grand livre honni », l.89) [VÉRIFIÉ] ; (2) pour refuser un retour à un
  état ancien, il faut que quelqu'un se souvienne du dernier état au-delà de 24 h (Step2 l.272) :
  le mur n°4 peut revenir par cette porte.
- **Recommandation : explorer C d'abord**, parce qu'elle est la seule piste qui *pourrait* garder
  le verrou #2 intact [HYPOTHÈSE, non établi — voir les deux points ci-dessus] ; B seulement en
  dernier recours, et alors écrite dans MISSION comme un affaiblissement assumé du verrou, pas
  comme une précision. Ne rien inscrire tant qu'il n'y a pas d'argent dans le périmètre
  (MISSION l.74).

**F6. Le « virus » doit-il vraiment se propager maintenant ?**
- A. Câbler la réplication vers des gardiens tiers (fidèle à MISSION §1.3). Pour être cohérente,
  A doit dire : (1) **qui** sont ces gardiens — de préférence des voisins dont le rôle de gardien a
  été validé (F1), pas n'importe quel pair ; (2) une **borne d'amplification globale** — le budget
  actuel est **local** (64 Mio par nœud, `backup/store.rs:29`) : il borne ce que chaque gardien
  garde, pas le nombre total de copies dans le réseau ; il faut un nombre de copies fixe par
  message (k petit) et la purge des copies tierces à la livraison (`ConfirmDelivery` conditionnel,
  CLAUDE.md limite #14) ; (3) un mobile sans tâche de fond **ne peut pas** être gardien
  (FEUILLE-DE-ROUTE l.17) : les gardiens sont des nœuds 24/24. ⚠️ Dans un réseau de millions de
  téléphones et de rares box, cela concentre le backup sur peu de nœuds — la leçon de la Freebox
  devenue « l'aspirateur » (design-carnet l.21-22). Il faut donc, en plus, l'éligibilité par
  « TICKET D'ENTRÉE » et le « Tirage égal entre éligibles » de la CARTE (l.95), et l'indicateur de
  concentration de la charte (l.39-41) [VÉRIFIÉ].
- B. Assumer « le gardien est l'émetteur » pour l'instant et corriger la documentation **d'état**
  (README l.52, CLAUDE.md limite #14). MISSION l.47 est dans le §1, « LA CIBLE. Non négociable »
  (MISSION l.13) : c'est une cible, pas un état ; on ne la réécrit pas d'après le code, au plus une
  note d'état hors du §1 (« pas encore codé ») [VÉRIFIÉ].
- **Recommandation : B tout de suite** (la documentation d'état ne doit pas mentir), **puis A** avec
  les conditions ci-dessus. Câbler A sans borne globale ni purge rouvrirait la classe de bug
  « rétention sans budget » déjà vue quatre fois (CLAUDE.md limite #11) [VÉRIFIÉ].

**F7. « Exclu temporairement » (wp-v1 l.315) : à retirer ?**
- A. Retirer (le verrou #5 interdit l'exclusion).
- B. Remplacer par « surveillance renforcée et sur-assignation ».
- **Recommandation : B** : elle garde l'intention (repérer les fermes d'identités, wp-v1 l.313-315)
  dans la forme permise. ⚠️ La « surveillance » de wp-v1 l.313 repose sur la « corrélation
  d'adresses IP ou d'ASN » et des « patterns de connexion » [VÉRIFIÉ] : des métadonnées, contre
  « Pas de collecte centrale de métadonnées » (v0.1 l.96-97). B ne garde que la sur-assignation et
  des signaux locaux, jamais un fichage centralisé. À trancher avec le « hors jeu » du 18/10 (§5) : si « hors jeu » veut dire
  « plus de rôle tant que la note est basse », B et le 18/10 disent la même chose.

**F8. L'équilibre ≈ 0 de Step2 redevient-il la formule de la note ?** (MISSION §8 q.3)
- A. Note = contribution seule (code actuel).
- B. Note = contribution − usage, idéal ≈ 0 (Step2 l.130-133).
- C. Note = contribution (avec fade) ; l'usage ne sert qu'à **doser l'anti-spam**, et seulement
  pour ce qu'on **émet** (ce qu'on fait porter aux autres), mesuré localement par chaque voisin,
  jamais agrégé [HYPOTHÈSE de conception].
- Deux défauts de B à peser : (1) Step2 compte dans l'usage « ce qu'il consomme du réseau
  (messages, octets …) » (l.125-126) : un utilisateur **surtout récepteur** (un mobile) aurait un
  solde négatif par construction, et un nœud 24/24 qui donne beaucoup serait « Score_U >> 0 …
  profil potentiellement abusé » (l.132-133) [VÉRIFIÉ] ; (2) mesurer l'usage **par personne** crée
  des métadonnées (qui reçoit combien), contre « Pas de collecte centrale de métadonnées » (v0.1
  l.96-97) [VÉRIFIÉ].
- **Recommandation : C**, qui garde l'esprit « l'arroseur arrosé » sans pénaliser le récepteur ni
  ficher l'usage. ⚠️ **C est un changement de vision**, pas une précision : tous les textes, MISSION
  l.64 compris (le plus récent), posent l'« équilibre contribution/usage » dans la note
  [VÉRIFIÉ] ; MISSION l.43 (« n'est pas fixée ») vise la formule, pas le principe. Si le fondateur
  tient à l'équilibre dans la note, la variante est B avec l'usage réduit à l'**émission**.

**F9. Quelle forme pour la couche universelle au-delà de Swift ?** (La V1 demandait « quel est le
premier SDK non-Apple ? » : prémisse fausse. `crates/tom-sdk`, SDK cœur Rust, est membre du
workspace (MISSION l.91), `tom-tui` existe, et la compilation Linux aarch64 musl est documentée,
README l.358 [VÉRIFIÉ].)
Une interface C publique existe déjà : en-tête généré `crates/tom-protocol-ffi/include/tom_protocol_ffi.h`
(cbindgen, « check de drift en CI »), 24 fonctions `extern "C"`, posée sur `tom-protocol` ; seul
Swift la consomme et le crate est exclu du workspace [VÉRIFIÉ].
- A. **Stabiliser cette interface C** (versionnage, contrat de compatibilité, entrée dans le
  workspace) et lui donner un premier consommateur non-Swift : tout langage, tout OS, l'embarqué
  et l'IoT (box, routeurs, objets branchés) s'y branchent.
- B. Kotlin/JNI pour Android.
- C. WASM pour le navigateur (le parcours du brief, l.94-104).
- **Recommandation : A**, parce que c'est elle qui fait de ToM une couche « au même titre que
  TCP/IP » (18/10) : une interface C stable est ce que les systèmes embarquent. B et C se posent
  ensuite dessus. Le travail n'est pas de créer l'interface, mais de la rendre stable et utilisée
  hors Apple.

---

## Annexe — ce qui a été recontrôlé, et comment

**V2 (M0017).** 8 auditeurs Sonnet frais, en lecture seule, un par groupe de sections, ont ouvert à
la source chaque phrase de cause, date, attribution forte et étiquette [VÉRIFIÉ] de la V1 : 341
points. Leurs verdicts bruts : 187 OK, 90 imprécis, 37 « hypothèse seulement », 22 faux (le reste
non classé). J'ai recontrôlé moi-même, à la source, chaque point repris dans la V2, et vérifié un
par un les 25 défauts listés par le mandat M0017 (tous confirmés ; le point 22 nuancé : l'hypothèse
« Snap à Hash » *vise* à éviter la persistance au-delà de 24 h, sous condition de ré-ancrage). Les
listes ci-dessous sont celles de la V1 ; les sources ajoutées en V2 sont citées dans le texte.

**Règle appliquée (V1).** Toute affirmation reprise d'un extrait Haiku a été relue par moi à son
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
