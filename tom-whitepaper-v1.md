# THE OPEN MESSAGING

**Un protocole pour un internet libre**

Whitepaper — Version 1.0

---

## Abstract

The Open Messaging (ToM) est un protocole de transport de données décentralisé, conçu pour fonctionner comme une couche de communication fondamentale — au même titre que TCP/IP ou HTTP — au-dessus de l'internet existant.

Contrairement aux blockchains traditionnelles qui accumulent un historique croissant et reposent sur des industries de validation, ToM adopte une architecture organique : chaque appareil connecté devient à la fois client et serveur, les rôles sont attribués dynamiquement et de manière imprévisible, et le réseau ne conserve que l'état présent.

Le consensus repose sur la masse et l'imprévisibilité : des milliers de participants, sélectionnés en cascade au dernier moment, valident collectivement les opérations. Personne ne peut anticiper son rôle, rendant toute attaque économiquement absurde.

Le protocole ne dépend d'aucune ressource externe. Son économie interne équilibre usage et contribution de manière proportionnelle aux capacités de chacun, sans course au profit ni accumulation de capital.

ToM n'est pas une application à installer. Il a vocation à devenir une brique protocolaire intégrée dans les outils du quotidien — navigateurs, messageries, clients mail — de sorte que l'utilisateur participe au réseau sans le savoir.

L'objectif : une couche de communication universelle, résiliente, sans point de contrôle central, qui se suffit à elle-même. Un nouveau protocole pour un internet qui n'appartient à personne.

---

## 1. Problème

### 1.1 La centralisation invisible

L'internet a été conçu comme un réseau décentralisé. En pratique, il est devenu l'inverse.

Quelques entreprises contrôlent les couches critiques : le transport (une poignée de fournisseurs d'infrastructure : cloud, CDN, DNS), les applications (des plateformes qui concentrent les échanges : messageries, réseaux sociaux, email), et l'accès (des FAI qui peuvent filtrer, ralentir, ou bloquer).

L'utilisateur croit communiquer librement. En réalité, chaque message transite par des points de contrôle qui peuvent lire, stocker, filtrer, ou couper.

### 1.2 La dépendance aux intermédiaires

Pour envoyer un message aujourd'hui, tu dépends d'un serveur qui relaie, d'une entreprise qui maintient ce serveur, et d'un modèle économique qui finance cette entreprise (publicité, abonnement, exploitation des données).

Même les solutions dites "chiffrées de bout en bout" reposent sur des serveurs centraux pour le routage, la découverte des contacts, et le stockage des messages en attente.

Le chiffrement protège le contenu. Il ne protège pas contre la dépendance.

### 1.3 Le coût caché de la gratuité

Les messageries gratuites ne sont pas gratuites. Le coût est payé autrement : en données personnelles, en attention (publicité), en dépendance à un écosystème fermé.

L'utilisateur n'est pas le client. Il est le produit.

### 1.4 La fragilité structurelle

Un serveur central est un point unique de défaillance. Il peut tomber (panne), être attaqué (DDoS, intrusion), être contraint (injonction légale, pression politique), ou simplement fermer (faillite, décision stratégique).

Quand WhatsApp tombe, des milliards de conversations s'arrêtent. Quand un gouvernement bloque Telegram, des millions de personnes perdent leur moyen de communication.

Cette fragilité n'est pas un bug. C'est une conséquence directe de l'architecture centralisée.

### 1.5 Le fardeau de l'historique

Les blockchains ont proposé une alternative à la centralisation. Mais elles portent un autre fardeau : l'historique complet.

Bitcoin stocke chaque transaction depuis 2009. Ethereum accumule un état toujours plus lourd. La promesse de décentralisation se heurte à la réalité : synchroniser un nœud complet prend des jours, demande des centaines de gigaoctets, et exclut de fait la plupart des utilisateurs.

L'historique est une force (auditabilité). Il est aussi un poids mort pour un réseau qui veut simplement transporter des messages.

### 1.6 L'industrie du consensus

Dans les blockchains actuelles, valider des transactions est devenu une industrie. Fermes de minage, pools, data centers, validateurs professionnels. L'accès au pouvoir de validation se paye en matériel, en énergie, ou en capital immobilisé.

Cela crée de la rente, de la centralisation de fait, et une barrière d'entrée qui éloigne les utilisateurs ordinaires du cœur du réseau.

Le consensus devait être distribué. Il est devenu un business.

---

## 2. Vision

### 2.1 Un protocole, pas une application

ToM n'est pas une messagerie. C'est un protocole de transport.

Comme TCP/IP transporte des paquets sans que l'utilisateur le sache, ToM transportera des messages sans qu'on le voie. Il a vocation à être intégré dans les outils existants — navigateurs, messageries, clients mail, applications — comme une couche invisible.

L'utilisateur final n'installera pas ToM. Il utilisera des applications qui, sous le capot, communiquent via ToM.

### 2.2 Chaque appareil est le réseau

Dans ToM, il n'y a pas de serveur central. Chaque appareil connecté (téléphone, ordinateur, tablette, objet connecté) est à la fois client et serveur.

Tu envoies des messages. Tu relaies ceux des autres. Tu participes au consensus. Sans le savoir.

Le réseau n'existe pas "quelque part". Il est partout où il y a des participants.

### 2.3 L'état présent, pas l'historique

ToM ne conserve pas l'historique complet des échanges. Il maintient uniquement l'état présent : qui existe, qui est connecté, quels engagements sont en cours.

Les messages passent, sont délivrés, et disparaissent du réseau. Seules les traces nécessaires à la cohérence sont conservées, sous forme d'empreintes cryptographiques.

C'est un BUS de données, pas un grand livre comptable.

### 2.4 La masse comme sécurité

La sécurité de ToM ne repose pas sur la puissance de calcul (comme Bitcoin) ni sur le capital immobilisé (comme Ethereum PoS). Elle repose sur la masse.

Des milliers de participants, sélectionnés de manière imprévisible, valident collectivement les opérations. Personne ne sait à l'avance qui sera choisi. Personne ne peut anticiper son rôle.

Corrompre le réseau nécessiterait de corrompre une fraction significative de tous les participants actifs, au moment précis où ils sont sélectionnés, sans savoir lesquels seront choisis.

C'est économiquement absurde.

### 2.5 Un équilibre, pas un profit

ToM ne crée pas de nouvelle cryptomonnaie à accumuler. Son économie interne est un système d'équilibre : tu utilises le réseau, tu contribues au réseau.

Pas de course au minage. Pas de staking. Pas de spéculation. Juste un équilibre dynamique entre ce que tu consommes et ce que tu apportes.

### 2.6 Invisible et universel

L'objectif final : ToM devient une couche de base de l'internet. Intégré dans les navigateurs. Intégré dans les OS. Intégré dans les box internet. Intégré dans les objets connectés.

Tu ne sais pas que tu l'utilises. Tu ne sais pas que tu y participes. Et c'est exactement pour ça que ça marche.

---

## 3. Architecture

### 3.1 Vue d'ensemble

ToM est structuré en deux couches :

La L1 (Layer 1) est le BUS central. Elle maintient l'état global minimal : les engagements des wallets, les ancres cryptographiques, les paramètres du réseau. Elle ne stocke pas les messages ni l'historique des transactions.

Les subnets sont des sous-réseaux éphémères qui se forment au-dessus de la L1. Chaque subnet correspond à un besoin concret : une conversation, un groupe, un canal, un flux de données. Les subnets naissent, vivent, et meurent selon l'activité.

### 3.2 La L1 comme BUS organique

La L1 de ToM n'est pas une blockchain traditionnelle. C'est un BUS organique qui ne retient que l'état présent.

Purge agressive : les messages et transactions déjà traités sont effacés de la L1. Seul l'état courant est conservé.

Genèse glissante : plutôt que d'accumuler des millions de blocs, la L1 reste proche d'un "bloc genèse" en mouvement. Les anciens blocs sont compactés en snapshots cryptographiques, puis archivés ou supprimés.

État présent : l'objectif n'est pas de raconter toute l'histoire, mais de garantir la cohérence de ce qui est vrai maintenant.

### 3.3 Subnets éphémères

Les subnets sont des sous-réseaux dynamiques qui se forment selon les besoins.

Création à la volée : quand A veut parler à B, un subnet se forme entre eux. Quand un groupe veut échanger, un subnet dédié apparaît.

Éphémérité : quand un subnet devient inactif, il s'auto-purge et disparaît. Pas de trace permanente.

Auto-régulation : si un subnet est surchargé, il peut automatiquement se diviser (fork) ou redistribuer ses rôles.

Fork contrôlé : les forks ne sont pas des catastrophes. Ce sont des mécanismes de respiration du réseau.

### 3.4 Rôles dynamiques

Dans ToM, chaque participant peut jouer plusieurs rôles, attribués dynamiquement :

Client : envoie et reçoit des messages.

Relais : transmet des messages pour d'autres (multi-sauts).

Observateur : surveille l'état de certains wallets ou subnets.

Gardien : aide les nouveaux nœuds à se synchroniser.

Validateur : participe aux quorums de validation pour la L1.

Ces rôles ne sont jamais figés. Ils tournent, se redistribuent, et sont attribués via des mécanismes pseudo-aléatoires.

### 3.5 Engagements par wallet

Chaque wallet dans ToM possède un engagement cryptographique de son état.

La L1 ne stocke pas le détail des transactions d'un wallet. Elle stocke uniquement un enregistrement minimal : l'identifiant du wallet (clé publique), l'engagement cryptographique de son état (hash ou racine Merkle), la signature agrégée des observateurs qui ont validé cet état, et le numéro de version.

L'historique détaillé vit chez le propriétaire du wallet et chez ses observateurs, pas sur la L1.

### 3.6 Observateurs distribués

Autour de chaque wallet existe un ensemble d'observateurs : des nœuds qui surveillent l'état de ce wallet et co-signent ses transitions.

Quand le propriétaire d'un wallet veut effectuer une opération (envoyer des fonds, modifier son état), il propose une transition. Les observateurs vérifient que la transition est légitime (état précédent correct, opération valide) et signent collectivement.

Une fois qu'un quorum d'observateurs a signé, la transition est enregistrée sur la L1.

Règle d'or : aucun observateur ne signe deux transitions différentes partant du même état. C'est cette continuité qui empêche la double dépense.

---

## 4. Consensus : Proof of Presence

### 4.1 Le problème des consensus traditionnels

Proof of Work (Bitcoin) : sécurisé, mais énergivore et centralisé de fait (fermes de minage).

Proof of Stake (Ethereum) : moins énergivore, mais crée une plutocratie (ceux qui ont le plus de capital contrôlent le réseau).

Dans les deux cas, le consensus est capturé par une industrie.

### 4.2 Proof of Presence (PoP)

ToM propose une autre approche : le droit de valider n'est pas donné par la puissance de calcul ni par le capital. Il est donné par la présence active sur le réseau.

Être présent signifie : être connecté, relayer des messages, répondre aux sollicitations, participer au routage.

Le consensus est porté par ceux qui sont là, pas par ceux qui ont les moyens.

### 4.3 Sélection en cascade

Les validateurs ne sont pas connus à l'avance. Ils sont sélectionnés en cascade, au dernier moment.

Le processus fonctionne ainsi : une source d'entropie génère un aléa vérifiable. Cet aléa sélectionne un premier groupe de nœuds. Ce groupe génère un nouvel aléa, qui sélectionne le groupe suivant. Et ainsi de suite, jusqu'au quorum final qui valide l'opération.

Personne ne peut prédire qui sera sélectionné. Personne ne peut se positionner à l'avance.

### 4.4 La masse comme rempart

La sécurité repose sur deux propriétés : l'imprévisibilité (tu ne sais pas qui sera choisi) et la masse (il y a trop de participants pour tous les corrompre).

Pour attaquer le consensus, il faudrait contrôler une fraction significative de tous les nœuds présents, au moment exact de la sélection, sans savoir lesquels seront choisis.

Plus il y a de participants, plus c'est impossible.

### 4.5 Validation croisée

Les décisions ne sont pas prises par un seul groupe. Elles sont validées par croisement de plusieurs rôles : les validateurs proposent, les observateurs vérifient, les relais transmettent, les gardiens attestent.

Chaque rôle surveille les autres. Une anomalie détectée par un rôle est signalée aux autres.

### 4.6 Pas de récompense, pas de course

Dans ToM, valider n'est pas un business. C'est une contribution.

Il n'y a pas de récompense en tokens pour avoir validé un bloc. Il n'y a pas de course pour être sélectionné. La validation est une tâche partagée, pas une source de profit.

Cela élimine l'incitation à industrialiser le consensus.

---

## 5. Économie interne

### 5.1 Principe : usage et contribution

L'économie de ToM repose sur un équilibre simple.

Chaque participant a deux compteurs : ce qu'il utilise (Usage) représentant les messages envoyés, données transmises, opérations demandées ; et ce qu'il contribue (Contribution) représentant les messages relayés, validations effectuées, stockage fourni.

Le score d'un participant est la différence entre sa contribution et son usage.

### 5.2 L'équilibre comme objectif

L'objectif n'est pas d'accumuler un score positif. C'est de rester proche de zéro.

Un score proche de zéro signifie que tu donnes autant que tu reçois. C'est l'état idéal.

Un score très positif signifie que tu contribues beaucoup plus que tu n'utilises. C'est acceptable, mais pas récompensé.

Un score très négatif signifie que tu utilises beaucoup plus que tu ne contribues. C'est un signal d'abus potentiel.

### 5.3 Pas de token spéculatif

ToM n'a pas de cryptomonnaie à acheter, vendre, ou accumuler.

Le "score" n'est pas un actif. C'est une mesure d'équilibre. Tu ne peux pas le vendre. Tu ne peux pas spéculer dessus. Il n'a de valeur que dans le contexte de ta participation au réseau.

### 5.4 Proportionnel aux moyens

La contribution attendue est proportionnelle aux capacités de l'appareil.

Un smartphone contribue moins qu'un serveur. Un appareil sur batterie contribue moins qu'un appareil branché. Un appareil avec une connexion lente contribue moins qu'un appareil en fibre.

Le réseau s'adapte. Il ne demande pas plus que ce que tu peux donner.

### 5.5 Gratuité conditionnelle

ToM est gratuit dans l'esprit. Pas de frais pour envoyer un message. Pas d'abonnement. Pas de barrière financière.

Mais cette gratuité est conditionnelle : tu participes. Tu ne peux pas être un passager clandestin permanent.

Si tu joues le jeu (tu relaies, tu valides, tu stockes temporairement), l'expérience reste fluide. Si tu abuses, le protocole répond.

---

## 6. Sécurité

### 6.1 Anti-Sybil : le problème des fausses identités

Dans un réseau décentralisé, un attaquant peut créer de nombreuses fausses identités pour tenter de dominer le consensus. C'est l'attaque Sybil.

ToM utilise plusieurs mécanismes pour la contrer.

### 6.2 Identité liée à l'appareil

L'unité d'identité dans ToM est le couple appareil + wallet.

Créer une nouvelle identité nécessite un nouvel appareil (ou une nouvelle installation) et un délai de probation pendant lequel le nouveau nœud a des capacités limitées.

Multiplier les identités a un coût réel (appareils) et un coût temporel (probation).

### 6.3 Période de probation

Un nouveau nœud ne devient pas immédiatement un participant à part entière.

Pendant une période de probation, il peut envoyer et recevoir des messages, mais ne peut pas être sélectionné comme validateur ou observateur critique, a un quota limité d'opérations, et est surveillé de plus près pour détecter les comportements suspects.

La durée de probation dépend du comportement observé.

### 6.4 Détection des patterns suspects

Le réseau surveille en continu les comportements anormaux : corrélation d'adresses IP ou d'ASN (beaucoup de nœuds depuis la même source), patterns de connexion similaires (même horaires, même durées), graphe de relais anormal (nœuds qui ne relaient qu'entre eux), latences incohérentes (prétendre être à plusieurs endroits).

Un nœud suspect est mis sous surveillance renforcée ou exclu temporairement.

### 6.5 L'arroseur arrosé

Le traitement du spam et des abus repose sur un principe simple : plus tu abuses, plus tu travailles.

Quand un participant a un score fortement négatif (utilisation >> contribution), le protocole augmente la difficulté de ses opérations : micro-preuve-de-travail locale (chaque message sortant nécessite un calcul de hash plus coûteux), sur-assignation de tâches (il est choisi plus souvent comme relais, il reçoit plus de messages à vérifier), et tâches de validation non critiques (vérifications de preuves, recalculs d'engagements).

Le spam devient auto-destructeur. L'arroseur est arrosé.

### 6.6 Pas de butin à voler

Dans les systèmes centralisés, un attaquant a une cible claire : le serveur, la base de données, le trésor.

Dans ToM, il n'y a pas de butin centralisé. Les messages sont chiffrés de bout en bout et ne transitent que temporairement. Les wallets sont distribués, chacun protégé par ses propres observateurs. Il n'y a pas de token à voler (le score n'est pas transférable).

Attaquer ToM ne rapporte rien.

### 6.7 Double dépense : le défi

Comment empêcher la double dépense sans historique complet ?

Dans Bitcoin, chaque transaction est tracée depuis la genèse. Dans ToM, la L1 ne garde que l'état présent.

La solution repose sur les observateurs. Quand tu veux dépenser, tu proposes une transition de ton état actuel vers un nouvel état. Les observateurs de ton wallet vérifient que l'état de départ correspond à ce qu'ils connaissent, que l'opération est valide, et signent la transition.

La règle d'or : aucun observateur ne signe deux transitions différentes partant du même état. Une fois qu'il a accepté "état A → état B", il refuse toute tentative "état A → état C".

Pour double-dépenser, il faudrait corrompre une majorité des observateurs de ton wallet, au même moment, sans qu'ils communiquent entre eux. C'est impraticable.

---

## 7. Comparaison

### 7.1 ToM vs Bitcoin

Bitcoin est un registre de transactions. ToM est un BUS de messages.

Bitcoin conserve tout l'historique depuis 2009. ToM ne garde que l'état présent.

Bitcoin repose sur le Proof of Work (énergivore). ToM repose sur le Proof of Presence (participation).

Bitcoin a des frais de transaction. ToM est gratuit (contre participation).

Bitcoin est sécurisé par la puissance de calcul. ToM est sécurisé par la masse et l'imprévisibilité.

### 7.2 ToM vs Ethereum

Ethereum est une machine à états programmable. ToM est un protocole de transport.

Ethereum accumule un état global croissant. ToM purge agressivement.

Ethereum repose sur le Proof of Stake (capital). ToM repose sur le Proof of Presence (participation).

Ethereum a des gas fees. ToM est gratuit (contre participation).

Ethereum permet les smart contracts complexes. ToM se concentre sur le transport de messages.

### 7.3 ToM vs messageries centralisées

WhatsApp, Telegram, Signal dépendent de serveurs centraux. ToM n'a pas de serveur.

Les messageries centralisées peuvent être coupées, censurées, piratées. ToM n'a pas de point unique de défaillance.

Les messageries centralisées collectent des métadonnées. ToM ne collecte rien centralement.

Les messageries centralisées ont des coûts d'infrastructure. ToM utilise la puissance des participants.

### 7.4 Ce que ToM n'est pas

ToM n'est pas une blockchain au sens classique (pas d'historique complet).

ToM n'est pas une cryptomonnaie (pas de token spéculatif).

ToM n'est pas une application (c'est un protocole).

ToM n'est pas un réseau anonyme type Tor (c'est un réseau de transport, pas d'anonymisation).

---

## 8. Roadmap

### 8.1 Où nous en sommes

La vision est définie. L'architecture est conçue. Les mécanismes sont décrits.

Ce document (whitepaper v1.0) pose les fondations.

### 8.2 Prochaines étapes

Phase 1 — Spécification formelle : définir précisément les formats de messages, les protocoles de sélection, les algorithmes de consensus. Produire des documents techniques exploitables par des développeurs.

Phase 2 — Proof of Concept : implémenter un prototype minimal. Trois appareils qui s'envoient un message via un relais, sans serveur central. Prouver que ça marche.

Phase 3 — Simulation : simuler le réseau à grande échelle. Tester les mécanismes anti-Sybil, la résistance aux attaques, les performances sous charge.

Phase 4 — SDK : développer un kit de développement pour permettre à d'autres d'intégrer ToM dans leurs applications.

Phase 5 — Intégrations pilotes : travailler avec des applications existantes pour intégrer ToM comme couche de transport.

Phase 6 — Déploiement progressif : étendre le réseau, stabiliser, documenter, ouvrir.

### 8.3 Ce qui reste à définir

Certains paramètres seront affinés pendant le développement : la taille exacte des quorums, les seuils de détection des comportements suspects, la durée de probation, la fréquence de purge.

Ces paramètres seront calibrés empiriquement, en fonction des tests et des retours.

### 8.4 Pas de promesses irréalistes

Ce que cette roadmap ne contient pas : des dates précises (le projet avance selon les ressources disponibles), une valorisation financière (il n'y a pas de token à vendre), des promesses de rendement (il n'y a rien à "gagner").

ToM est un protocole. Il sera prêt quand il sera prêt.

---

## 9. Conclusion

### Un constat simple

L'internet a été conçu pour connecter. Il est devenu un outil pour contrôler.

Quelques entreprises décident qui peut parler, qui peut écouter, et à quel prix. L'utilisateur croit communiquer librement. En réalité, chaque message passe par des points de contrôle qui peuvent lire, stocker, filtrer, ou couper.

Ce n'est pas une fatalité. C'est un choix d'architecture.

### Une autre voie

The Open Messaging propose un choix différent : pas de serveur central (chaque appareil est le réseau), pas d'historique permanent (seul l'état présent compte), pas d'industrie de validation (la masse présente décide), pas de token spéculatif (un équilibre, pas un profit), pas d'application à installer (un protocole invisible).

### Ce que ToM n'est pas

ToM n'est pas une blockchain. Pas une cryptomonnaie. Pas une plateforme. Pas une app.

ToM est une couche de communication. Un protocole. Un BUS de données.

Comme TCP/IP transporte des paquets sans qu'on le sache, ToM transportera des messages sans qu'on le voie.

### Ce que ToM pourrait devenir

Un internet parallèle.

Un réseau qui n'appartient à personne parce qu'il appartient à tout le monde. Un réseau qui ne dépend de rien parce qu'il se suffit à lui-même. Un réseau qu'on ne peut pas attaquer parce qu'il n'y a rien à voler.

Un réseau où tu ne sais pas que tu participes — et c'est exactement pour ça que ça marche.

### Le chemin

La vision est claire. L'architecture est posée. Les mécanismes sont conçus.

Reste à construire. Spécifier. Coder. Tester. Itérer. Intégrer.

Pas de promesses. Pas de hype. Le code parlera.

### Un mot de fin

Communiquer n'a pas à dépendre d'intermédiaires qui filtrent, capturent, ou facturent l'évidence.

Reprenons la main sur nos échanges.

---

THE OPEN MESSAGING

Un protocole pour un internet libre.
