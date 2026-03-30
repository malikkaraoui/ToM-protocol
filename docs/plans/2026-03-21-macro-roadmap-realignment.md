# Réalignement macro — ToM Protocol après relay discovery (21 mars 2026)

## Pourquoi ce document

Ce document sert à réaligner :

- la **vision produit/protocole** portée par le brief, l'architecture et les artefacts BMAD ;
- la **trajectoire réelle** du code Rust et des chantiers récents ;
- la **prochaine séquence de travail** à challenger collectivement.

L'objectif n'est pas de réécrire toute la roadmap historique, mais de clarifier **où nous en sommes réellement**, **ce qui est déjà acquis**, **ce qui manque encore à la promesse ToM**, et **quel cap garder**.

Ce document est volontairement **plus directif et plus discutable** qu'un simple récap. Il assume une ligne :

- dire ce qui est **réellement prouvé** ;
- séparer ce qui relève du **socle**, de **l'opérabilité**, et du **produit perçu** ;
- proposer un cap qui puisse être **challengé frontalement** par l'équipe.

---

## Thèse centrale

La thèse défendue ici est la suivante :

> ToM a franchi un seuil important côté **socle réseau crédible**, mais il n'a pas encore franchi le seuil où la promesse devient **évidente, opérable et partageable**.

Autrement dit :

- nous ne sommes plus dans le fantasme architectural ;
- nous ne sommes pas encore dans la sensation produit "ça y est, le réseau vit et se suffit à lui-même".

Le bon cap n'est donc ni :

- de repartir dans une nouvelle couche théorique ;
- ni de sur-vendre un niveau d'autonomie qui n'est pas encore démontré.

Le bon cap est :

1. **stabiliser le socle relay-aware déjà livré** ;
2. **le rendre opérable sans friction** ;
3. **le démontrer proprement** ;
4. **le pousser ensuite jusqu'à une validation alpha qui mérite vraiment son nom**.

## Mise à jour de cap — 30 mars 2026

Le run Apple TV comme 3e nœud a validé un point important : le socle fonctionne sur plusieurs devices réels. Mais il a aussi révélé un verrou plus prioritaire que le stress ou les groupes : l'amorçage restait encore partiellement **hardcodé côté tvOS** (relay connu, bootstrap peer connu).

Conclusion : la suite doit être **reclassée**.

### Ordre reclassé immédiatement

1. **Dé-hardcoder l'amorçage**
2. **Prouver la survie du réseau sans les devices du seed initial**
3. **NAS down / seed down**
4. **Mini-stress Apple TV (10 min)**
5. **Groupes**
6. **MacBook Air 2011**
7. **4G/5G / autre Freebox**

### Ce qui doit désormais être prouvé

La vraie preuve n'est pas seulement "3 devices communiquent". La vraie preuve est que le réseau peut porter lui-même le flambeau après amorçage.

- **Test A — amorçage minimal** : un volontaire ou une infra amie amorce le réseau, puis les nœuds se rejoignent.
- **Test B — propagation** : peers, relays et routes utiles sont appris par plusieurs nœuds sans saisie manuelle.
- **Test C — retrait du seed initial** : on coupe le seed initial, le réseau continue à vivre.
- **Test D — retrait de nos devices** : on coupe les devices du porteur initial, le réseau continue grâce aux autres volontaires.
- **Test E — retour plus tard** : un device revient plus tard et retrouve un réseau vivant sans reconfiguration lourde.

Tant que cette séquence n'est pas validée, les scénarios de stress, groupes ou extension multi-sites restent utiles, mais ne constituent pas encore la preuve centrale de la promesse ToM.

---

## 1. Constats de réalignement

### 1.1 Ce qui reste parfaitement aligné avec la vision ToM

Les points suivants sont cohérents avec le brief produit, l'architecture et les ADR :

- ToM reste pensé comme une **couche protocolaire**, pas comme un produit final ;
- le modèle de **nœud unifié** reste central : chaque nœud peut potentiellement participer au réseau ;
- la logique **relay-first puis upgrade direct** reste conforme à la philosophie réseau ;
- la séparation entre **rôle logique** et **capacité réseau réellement disponible** est maintenant mieux tenue ;
- le cap reste bien celui d'un réseau **plus autonome, plus distribué, moins dépendant d'une infra fixe**.

### 1.2 Ce qui a divergé du découpage BMAD initial

Le découpage BMAD historique était fortement structuré autour :

- d'un démarrage browser-first / WebRTC / signaling ;
- d'une progression TypeScript/demo ;
- d'itérations plus "story-driven" côté expérience visible.

La trajectoire réelle récente a plutôt consolidé :

- le **port Rust natif** ;
- le **fork transport/gossip/relay** ;
- le **relay embarqué** ;
- la **publication relay-ready** ;
- la **consommation transport des relays publiés** ;
- la **republication périodique** ;
- une validation plus sérieuse par **tests ciblés + validation terrain**.

Conclusion :

> La vision reste alignée, mais le **chemin d'exécution réel** est désormais plus orienté "socle réseau Rust + relay embarqué + discovery réaliste" que le découpage BMAD initial ne le racontait explicitement.

### 1.3 Ce que cette divergence implique pour les discussions d'équipe

Cette divergence n'est pas un problème en soi.

Elle devient un problème seulement si l'équipe continue à raisonner avec :

- un **récit historique devenu partiellement obsolète** ;
- des stories BMAD qui ne décrivent plus précisément l'ordre réel des dépendances ;
- ou une confusion entre **vision finale**, **socle déjà livré**, et **étapes réellement restantes**.

Le rôle de ce document est précisément d'éviter ce flou.

---

## 2. Socle déjà livré ou fortement consolidé

### 2.1 Socle réseau/protocole

Sont déjà crédibles et testés à des niveaux variés :

- runtime protocolaire structuré ;
- transport QUIC/relay intégré ;
- discovery relay-assisted (`PeerPresent -> injection d'adresse -> gossip join`) ;
- relay embarqué démarrable/supervisable ;
- publication `RelayReadyAnnounce` ;
- `RelayRegistry` local côté discovery ;
- injection dynamique dans le transport des relays découverts ;
- republication périodique des relays healthy pour éviter une expiration artificielle.

### 2.2 Discipline d'implémentation

Les chantiers récents ont aussi consolidé une méthode utile :

- design avant code ;
- séparation nette `RuntimeState` / loop async / transport ;
- petites étapes validées ;
- tests unitaires + intégration + terrain ;
- refus des claims trop ambitieux quand les preuves ne suivaient pas.

Ce point est important : la crédibilité du projet dépend autant du code que de la rigueur avec laquelle on décrit ce qui est réellement prouvé.

### 2.3 Ce que ce socle permet déjà de dire sans exagération

À ce stade, on peut défendre raisonnablement les affirmations suivantes :

- un nœud ToM peut démarrer un vrai relay embarqué dans son propre process ;
- ce relay peut être annoncé au réseau de manière signée et vérifiée ;
- d'autres nœuds peuvent consommer cette annonce sans polluer la topologie logique ;
- le transport peut être enrichi dynamiquement avec ce relay découvert ;
- si le relay reste healthy, sa présence peut être maintenue par republication ;
- si cette republication cesse, l'expiration puis le retrait suivent correctement.

Ce n'est pas encore le relay rotatif complet.

Mais ce n'est déjà plus un simple prototype conceptuel : c'est une **chaîne fonctionnelle réseau**.

---

## 3. Ce qui manque encore à la promesse ToM

### 3.1 Ce qui manque côté infrastructure autonome

Même après les derniers chantiers, ToM n'a pas encore livré une boucle complète de **relay rotatif autonome**.

Il manque encore, au minimum :

- une opérabilité simple des options runtime ;
- une expérience de lancement claire pour les rôles observer/publisher ;
- une boucle plus complète entre rôle réseau, relay local réellement prêt, publication, consommation et maintien ;
- une validation alpha plus large à plusieurs nœuds en conditions variées ;
- une preuve plus forte que la mécanique s'intègre dans une exploitation régulière, pas seulement dans des tests ciblés.

### 3.2 Ce qui manque côté produit perçu

Le plus grand écart avec la promesse ToM n'est probablement plus le cœur technique, mais le **ressenti produit**.

Aujourd'hui, on est encore davantage sur :

- de la plomberie réseau intelligente ;
- des primitives robustes ;
- de la validation technique.

Pour se rapprocher de la promesse perçue, il faut encore :

- rendre les features **faciles à piloter** ;
- rendre les scénarios **faciles à démontrer** ;
- réduire la friction humaine autour du runtime ;
- rapprocher l'infrastructure du vécu : "ça marche, sans y penser".

### 3.3 Ce qui manque encore à la promesse, formulé sans jargon

Si on se place du point de vue d'un observateur extérieur, ce qui manque encore est simple :

- pouvoir lancer facilement les bons rôles sans bricolage ;
- pouvoir montrer le comportement du réseau sans expliquer 20 abstractions internes ;
- pouvoir dire "voilà le scénario, voilà la preuve, voilà ce qui manque encore" ;
- pouvoir faire vivre plusieurs nœuds ensemble de manière convaincante sans dépendre de scripts spéciaux pour tout.

Tant que ce point n'est pas atteint, la promesse ToM reste **forte intellectuellement**, mais **pas encore suffisamment incarnée opérationnellement**.

---

## 4. Cap macro recommandé pour la suite

### Bloc A — Opérabilité immédiate

Objectif : rendre les features déjà livrées facilement activables sans binaire ad hoc.

Priorités :

- exposer la config utile dans `tom-tui` ;
- permettre un lancement simple en mode observer ;
- permettre un lancement simple en mode relay publisher ;
- documenter 2 ou 3 scénarios CLI reproductibles.

### Bloc B — Démonstration claire de la promesse réseau

Objectif : transformer des capacités validées en démonstrations partageables.

Priorités :

- scénarios simples et reproductibles ;
- logs/events lisibles ;
- preuve visible de : publication, découverte, refresh, expiry, fallback.

### Bloc B' — IPv6-first (ajouté 2026-03-27)

Objectif : publier des adresses IPv6 globales pour une joignabilité directe sans NAT ni tunnel.

Contexte : le test terrain du 27 mars a montré que le relay embarqué publie l'IP LAN (192.168.0.83) qui n'est pas routable depuis Internet. IPv6 résout ce problème fondamentalement — chaque nœud a une adresse globale unique, pas de NAT, pas de port forwarding.

Priorités :

- préférer IPv6 dans `detect_outbound_ip()` quand une adresse globale est disponible ;
- valider la joignabilité IPv6 directe Mac → NAS (sans tunnel SSH) ;
- tom-relay --dev : déjà dual-stack `[::]:3340`, vérifier le firewall Freebox IPv6 ;
- valider que MagicSock/Disco hole punch fonctionne en IPv6 natif ;
- fallback IPv4+relay quand IPv6 absent (hotspots, réseaux d'entreprise).

Impact attendu : élimine le tunnel SSH pour les tests terrain, réduit la dépendance au relay bootstrap (T0), rapproche de M2 (3 commandes au lieu de 4).

### Bloc C — Fermeture progressive de la boucle "relay vivant"

Objectif : passer du socle relay-aware à une boucle plus autonome.

Priorités :

- clarifier le passage rôle logique -> relay réellement prêt -> relay publiable ;
- maintenir une séparation stricte entre topologie logique et état opérationnel du relay ;
- éviter toute annonce de faux relay ;
- étendre les validations terrain de manière progressive.

### Bloc D — Validation alpha multi-nœuds

Objectif : approcher le niveau "self-sustaining alpha network" de manière crédible.

Priorités :

- scénarios 10–15 nœuds ;
- churn (join/leave/rejoin) ;
- maintien discovery/relay ;
- résilience observable ;
- métriques réelles de stabilité.

### Bloc E — Surface produit / intégration

Objectif : se rapprocher enfin du ToM invisible et intégrable.

Priorités :

- TUI plus pilotable ;
- outils de validation plus simples ;
- exposition claire des modes réseau ;
- chemin vers SDK / intégration / démo plus évidents.

### Ordre recommandé entre ces blocs

L'ordre important n'est pas seulement "quoi faire", mais **dans quel ordre ne pas se tromper**.

Ordre recommandé :

1. **Bloc A — Opérabilité immédiate** (~80% fait, sprint 1)
2. **Bloc B — Démonstration claire** (~70% fait, scripts S7/S8)
3. **Bloc B' — IPv6-first** (nouveau, accélérateur pour C et D)
4. **Bloc C — Boucle relay vivant plus autonome**
5. **Bloc D — Validation alpha multi-nœuds**
6. **Bloc E — Surface produit / intégration**

Pourquoi cet ordre :

- sans opérabilité, il n'y a pas de validation collective confortable ;
- sans démonstration claire, l'équipe risque de continuer à discuter dans le vide ;
- sans fermeture de la boucle relay vivant, l'alpha multi-nœuds restera fragile ;
- sans alpha crédible, la surface produit risque de devenir cosmétique ;
- et sans surface produit, la promesse restera techniquement vraie mais humainement lointaine.

---

## 5. Priorité recommandée à court terme

### Prochaine marche la plus rentable

La prochaine marche recommandée est :

> **rendre la mécanique relay/discovery/republish pilotable simplement depuis `tom-tui`**

Pourquoi :

- ce n'est pas un nouveau grand chantier d'architecture ;
- cela capitalise directement sur ce qui vient d'être livré ;
- cela rapproche immédiatement le projet d'une utilisation réelle ;
- cela prépare mieux les validations collectives et les challenges BMAD.

Concrètement, cela veut dire :

- exposer les options utiles en CLI ;
- garder les defaults existants ;
- permettre des scénarios reproductibles sans binaire jetable ;
- fournir une base simple pour les validations d'équipe.

### Ce qui ne devrait pas passer devant cette priorité

À court terme, les sujets suivants ne devraient **pas** prendre la priorité sur cette opérabilité :

- une nouvelle abstraction lourde autour des rôles ;
- un chantier de refonte large de la TUI ;
- un nouveau sous-système de découverte non démontré ;
- des optimisations prématurées non liées à un point de friction réel ;
- une extension du périmètre produit qui n'améliore ni la preuve, ni l'usage, ni la validation.

En clair :

> la bonne suite n'est pas d'ajouter une nouvelle profondeur théorique, mais de rendre la profondeur déjà acquise **actionnable**.

---

## 6. Lecture honnête de notre position actuelle

### Ce que l'on peut dire honnêtement

- Le projet n'est plus au stade de l'intention abstraite.
- Le socle réseau Rust est devenu sérieux.
- La chaîne relay embarqué -> publication -> registry -> transport discovery -> refresh est désormais réelle.
- La discipline de validation récente augmente la confiance.

### Ce qu'il ne faut pas encore sur-vendre

- le relay rotatif complet de bout en bout ;
- l'autonomie réseau totale déjà acquise ;
- une expérience utilisateur déjà évidente ou invisible ;
- un alignement parfait entre tous les anciens artefacts BMAD et l'état réel du code.

Formulation plus juste :

> Le projet est désormais solide sur plusieurs briques critiques du socle autonome, mais il reste encore un travail important pour transformer cette base technique en promesse ToM pleinement perceptible et exploitable.

### Risque principal à partir de maintenant

Le risque principal n'est probablement plus un risque de faisabilité brute.

Le risque principal devient un risque de **dispersion stratégique**.

Formes typiques de cette dispersion :

- ouvrir trop de mini-chantiers parallèles ;
- confondre robustesse technique et sensation produit ;
- chercher trop tôt la sophistication d'une autonomie complète ;
- ou, à l'inverse, se contenter trop longtemps d'une base techniquement bonne mais difficile à démontrer.

Le mot d'ordre devrait donc être :

> **moins de branches parallèles, plus de continuité entre socle, opérabilité et preuve.**

---

## 7. Proposition de lecture commune pour l'équipe BMAD

Pour challenger utilement la suite, les bonnes questions ne sont probablement plus :

- "Est-ce que le concept est crédible ?"
- "Est-ce qu'on a quelque chose de réel ?"

Les bonnes questions deviennent plutôt :

1. Quelle est la **plus petite suite de chantiers** qui rapproche le plus ToM de sa promesse visible ?
2. Quels sujets relèvent encore du **socle infra** et lesquels relèvent déjà de **l'opérabilité / produit** ?
3. Où faut-il encore produire de la **preuve technique** et où faut-il surtout réduire la **friction d'usage** ?
4. Qu'est-ce qui constitue, pour nous, un **alpha convaincant** et partageable ?

### Questions de challenge recommandées pour l'équipe

Pour éviter un review mou, voici les questions qui méritent d'être posées franchement :

1. **Quel est aujourd'hui le plus petit chemin démontrable vers un “wow moment” honnête ?**
2. **Quelles parties du discours ToM sont déjà soutenues par des preuves, et lesquelles relèvent encore d'une cible ?**
3. **Le prochain chantier proposé réduit-il réellement la distance à la promesse, ou seulement notre confort d'ingénierie ?**
4. **Quel est le plus gros angle mort de nos validations actuelles ?**
5. **Qu'est-ce qu'un critique externe intelligent trouverait encore “trop vendeur” dans notre récit ?**
6. **Quel seuil nous ferait dire sérieusement : “oui, là on a un alpha réseau crédible” ?**

Ces questions sont plus utiles que des validations générales du type “ça a l'air cohérent”.

---

## 8. Anti-plan : ce que je déconseille

Pour être vraiment force de proposition, il faut aussi dire ce qu'il ne faut pas faire.

Je déconseille explicitement les trajectoires suivantes :

### Anti-plan A — repartir trop haut dans la vision

Risque : relancer des discussions très larges sur la philosophie globale, la tokenomics absente, ou la vision civilisationnelle, alors que le projet a surtout besoin d'une meilleure articulation entre socle et usage.

Effet : beaucoup d'énergie, peu de réduction de risque concret.

### Anti-plan B — ouvrir un gros chantier “relay rotatif complet” tout de suite

Risque : vouloir fermer d'un coup toute la boucle autonome alors que l'opérabilité et la démonstration de base peuvent encore être renforcées à bien moindre coût.

Effet : on augmente la complexité avant d'avoir maximisé la lisibilité.

### Anti-plan C — dériver vers du polish UI trop tôt

Risque : donner l'impression de progrès alors que le vrai sujet est encore le passage de la preuve technique à la preuve exploitable.

Effet : sensation de mouvement, faible gain stratégique.

### Anti-plan D — multiplier les outils ad hoc non intégrés

Risque : accumuler des binaires/scénarios spéciaux qui prouvent le système localement mais n'améliorent pas l'opérabilité réelle.

Effet : dette d'usage et confusion dans l'équipe.

---

## 9. Critères de succès pour les 2 prochaines étapes

### Étape 1 — opérabilité simple

Succès si :

- un membre de l'équipe peut lancer un observer et un publisher sans lire 5 docs ;
- les options utiles sont pilotables simplement ;
- les scénarios de base sont reproductibles sans code ad hoc.

### Étape 2 — démonstration claire

Succès si :

- la publication, la découverte, le refresh et l'expiration peuvent être montrés clairement ;
- l'équipe peut discuter sur une preuve concrète, pas sur une interprétation ;
- un reviewer externe comprend rapidement ce qui est réel et ce qui reste à faire.

### Étape 3 — alpha convaincant

Succès si :

- plusieurs nœuds vivent ensemble de manière crédible ;
- la boucle relay/discovery tient sous churn raisonnable ;
- on peut défendre un récit simple, exact et stable devant un regard critique.

---

## 10. Synthèse finale

En une phrase :

> ToM est aujourd'hui mieux aligné avec sa promesse qu'il y a quelques semaines, non pas parce que tout est fini, mais parce que le socle relay/discovery/transport devient enfin réel, vérifié et exploitable.

Le cap recommandé est donc :

> **consolider l'opérabilité immédiate, démontrer la boucle réseau vivante, puis pousser vers une vraie validation alpha multi-nœuds.**

Et la version la plus directe de ce document, si l'on veut être brutalement honnête, est la suivante :

> Le projet a cessé d'être seulement une bonne idée technique ; il doit maintenant prouver qu'il sait devenir une mécanique réseau utilisable, montrable et contestable sans s'effondrer au premier regard critique.
