# CARTE — Le rendez-vous tournant (et l'organisme qui le porte)

> Vision cadrée avec Malik, 2026-07-24. Référence d'une page. **Pas de code avant validation.**
> Remplace la logique « HRW + horloge » du 1er jet (`design-carnet-rendezvous-tournant.md`) —
> abandonnée : un minuteur mécanique trahissait l'esprit organique.

---

## 0. LE PILIER (au-dessus de tout)

- **Rien n'est commandé d'en haut.** La **L1 tient le miroir** (l'ancre de l'état présent), elle
  **n'orchestre JAMAIS** (décret LOCKED #3). Tout s'**auto-organise**.
- **Le réseau se durcit ET se démultiplie avec la masse.** Comme Bitcoin ajuste sa difficulté au
  nombre de mineurs — **mais SANS mur qui exclut** : ToM **répartit** (les puissants portent le lourd
  + les connexions directes rapides ; les faibles reçoivent des **tâches simples** ; personne n'est
  débranché).
- **Plus il y a de monde, plus c'est fort.** Chaque nouveau **renforce** le réseau. Un serveur
  central se fragilise sous la charge ; ToM se **renforce**. C'est **la locomotive**.

---

## 0bis. LE BUT D'UN NŒUD (et le rythme)

**Le but de chaque nœud** : apporter sa **réelle contribution** — le **rôle dont le réseau a besoin**
(pas celui qu'il choisit) — et **appliquer les règles** (le tournus des rôles). **Le tout au RYTHME
du réseau.** *(Comme un musicien sans chef : sa partie + la partition tournante + le tempo.)*

**Le rythme = la cadence** — l'autre info que les étourneaux échangent, avec la direction. Un tempo
commun **synchronise l'organisme sans chef** (comme une nuée bat des ailes ensemble, comme les
lucioles clignotent en phase). Il est **soutenable** (chacun à sa mesure : un mobile bat plus
lentement qu'une Freebox, mais dans le même tempo), **ni frénétique** (ça épuise / vide la batterie)
**ni endormi** (ça fige). Ancrage : les **ticks** existants (rotation, publication, rafraîchissement
du front) **SONT** ce rythme — désormais pensés comme le **pouls vital**, pas des détails techniques.

---

## 1. LE PRINCIPE LOCAL — la murmuration (fondement scientifique)

Science des nuées d'étourneaux (Rome, 2008 ; projet STARFLAG, 4000 oiseaux filmés en 3D) :
- Chaque oiseau ne suit qu'un **nombre FIXE de voisins (~7)**, **pas une distance** → **robuste que la
  nuée soit dense ou clairsemée** (= s'adapte à la masse, exactement notre pilier).
- Ces 7 suffisent à une **corrélation « scale-free »** : le mouvement d'**un seul** se propage en
  **vague** à toute la nuée, quasi-instantanément. **Aucune vue globale — l'ordre émerge du local.**
- **7 = l'optimum** entre rester soudé et ne pas se surcharger, **sous incertitude**.

→ **Pour ToM** : chaque nœud regarde un **petit nombre fixe de voisins** (son subnet), en déduit la
**direction** et le **pouls** du réseau (santé, qui porte quoi). L'info (rôles, présence, alerte) se
propage en **vagues** de proche en proche. **Jamais de vue d'ensemble** — elle serait le serveur
central interdit. *(Ce que le red-team appelait « faille : pas de consensus global » est en fait
LA fondation.)*

**Deux échelles :**

| Échelle | Ce qu'il faut | Pourquoi |
|---|---|---|
| **Vivre** (organiser les rôles, mon subnet) | la **vue locale** de mes ~voisins | l'ordre émerge de proche en proche, aucun central |
| **Se rencontrer** (deux inconnus, sans voisin commun) | un **ancrage mince et tournant** (le gardien/secrétaire du moment) | il faut un point commun *minimal* pour calculer un lieu de rendez-vous |

---

## 1bis. LE FRONT QUI PARLE (le titre = tableau de bord ambiant)

La murmuration **rendue concrète** : l'étourneau ne *communique* pas sa direction, il la **porte**
(position des ailes) et le voisin la **lit** rien qu'en l'observant. Pareil ici — chaque nœud
**porte son état sur son titre diffusé**, les voisins le **lisent sans même se connecter**. Info
**gratuite, ambiante, zéro protocole**, qui nourrit le pouls local (règle #1).

**Format** (compact — canal serré : username ToM = 32 octets, advertising BLE ~31) :
`Malik-iPhone · 140 · RG · 4p · ~`

| Champ | Dit |
|---|---|
| nom | qui je suis (lisible) |
| build | `140` (la demande de départ) |
| rôle(s) | `RG` = Relais+Gardien (multi-rôles = lettres collées : P/R/G/S/B/H) |
| pairs | voisins **en ligne avec moi maintenant** |
| activité | `·` calme · `~` actif · `▲` suractif/saturé · `zzz` veille |

**Règles d'usage :**
- **HINT, pas vérité** : non signé (comme le username, cf. `R-name-via-dht`) → coup d'œil indicatif,
  **jamais** une base de décision/sécurité. L'ancre reste **node_id + PoP réel** (un menteur peut
  afficher un faux front, on s'en fiche pour un aperçu ambiant).
- **Sur le label ToM** (mDNS + gossip + record DHT + liste de pairs), **pas** le nom système iOS (verrouillé).
- **Rafraîchi par paliers** (calme/actif/saturé), pas du continu → batterie économisée.
- **Rien à inventer** : le `/status` calcule déjà rôle/pairs/activité, le `node-label` est déjà
  diffusé, `R-name-via-dht` le prévoit déjà — juste à **condenser** dans le titre.

---

## 2. LES 4 RÈGLES (tranchées)

| # | Règle | En clair |
|---|---|---|
| **1 — l'aléa qui fait tourner** | La graine = **l'état vivant** du réseau (la vue de présence, signée par quorum — déjà à moitié construite : L1-003 + entropy-seed L1-001). **Ça tourne parce que l'état change, pas l'horloge.** Vérifiable, partagé. *(Cascade d'entropie §4.3 en durcissement plus tard.)* |
| **2 — anti-monopole** | La capacité est un **TICKET D'ENTRÉE** (être fiable te rend *éligible*), **PAS un classement**. Tirage **égal** entre éligibles → le stable n'est plus « le » porteur, juste *un* parmi d'autres. *(Tour de garde / cooldown en filet pour les petits réseaux peu mouvants.)* |
| **3 — la porte d'entrée** | **Deux étages.** ① l'**humain** : bouche-à-oreille, la reco d'un proche « vaut mille pubs ». ② les **gardiens tournants** : pour l'**éclaireur** (« chef de meute ») venu de nulle part, sans invitation. Croissance **en éventail** : l'éclaireur entre par les gardiens → devient ensuite **la porte des siens**. |
| **4 — le délestage** | Réflexe **SENTIR + DÉLÉGUER** : le nœud connaît sa charge **globale** (pas rôle par rôle) et **passe la main** quand il sature (réplique/recopie le rôle — comme backup & hub failover, déjà éprouvés). *« Se retenir » tombe gratuitement* : qui délègue cesse d'accumuler. Le pilier absorbe (plus de bras avec la masse). |

---

## 3. ÉCONOMIE DE RÔLES

**Un même geste sert plusieurs rôles** : accueillir un nouveau = le présenter à son voisinage = mettre
deux inconnus en relation. **Un seul moteur** — un rôle tournant, sélectionné par la vue vivante,
avec ticket d'entrée — anime le **rendez-vous**, les **gardiens** et l'**accueil**. On **ne multiplie
pas** les rôles ; on réutilise le geste.

---

## 4. CE QUI EST DÉJÀ SOLIDE (éprouvé au red-team, 21 agents)

La vision **encaisse les attaques individuelles** (faux nœud, spammeur, hôte qui ment ou tombe) grâce
au filet des décrets LOCKED : **PoP** (présence = travail constaté), **réputation qui fade**,
**arroseur-arrosé**, **TTL**, **redondance K**. Les seuls points durs étaient **systémiques**
(l'aléa, le point fixe, le cold-start) — et le **principe local (murmuration)** + l'**ancre mince
tournante** en referment l'essentiel : le cold-start disparaît (on entre par un humain, pas dans le
vide), la divergence de vue devient le principe, le point fixe cède au ticket d'entrée.

---

## Sources (murmuration)
- [The Science Of Starling Murmurations — Bird Spot](https://www.birdspot.co.uk/bird-behaviour/the-science-of-starling-murmurations)
- [Fluctuation-Driven Flocking & Scale-Free Correlation — PMC](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3360731/)
- [Starling Flock Networks Manage Uncertainty in Consensus at Low Cost — PLOS Comp. Biol.](https://journals.plos.org/ploscompbiol/article?id=10.1371%2Fjournal.pcbi.1002894)
