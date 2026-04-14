# Quand le réseau devient la loi — le protocole ToM comme infrastructure de la liberté

*Essai pour le concours de la liberté — Pavel Durov, 2026*

---

## I. Le paradoxe d'internet

Il y a trente ans, des ingénieurs visionnaires ont construit un réseau censé incarner la liberté absolue : pas de centre, pas de hiérarchie, une architecture distribuée où chaque nœud valait chaque autre nœud. L'internet était une promesse politique autant que technique. Une infrastructure où aucun gouvernement, aucune corporation, aucune autorité ne pourrait couper le signal.

Aujourd'hui, une poignée d'entreprises — cinq, peut-être six — contrôlent la quasi-totalité des communications numériques mondiales. Des milliards de messages transitent chaque jour par des serveurs centralisés, lisibles en théorie par leurs opérateurs, exposés aux injonctions légales de n'importe quel État, vulnérables à une panne unique qui peut rendre muet un continent entier. L'architecture qui devait défier le contrôle est devenue son vecteur le plus efficace.

Ce n'est pas une trahison accidentelle. C'est le résultat d'une logique économique : centraliser est moins cher, plus rapide, plus facile à monétiser. Chaque service qui centralise devient un point de contrôle. Et les points de contrôle attirent le pouvoir comme les aimants attirent le métal.

Pavel Durov le sait mieux que quiconque. Il a construit VKontakte, vu son réseau lui être arraché par des intérêts politiques, puis construit Telegram — un acte de résistance architecturale autant qu'entrepreneuriale. Être arrêté à Paris en 2024 pour avoir construit une messagerie refusant de se plier aux injonctions des États : c'est l'illustration parfaite de ce que signifie construire une infrastructure de liberté dans un monde qui préfère le contrôle.

Le protocole ToM — *The Open Messaging* — part d'une question simple et radicale : et si l'architecture elle-même rendait le contrôle structurellement impossible ?

---

## II. L'architecture comme acte politique

La liberté numérique n'est pas une feature. Elle ne s'ajoute pas après coup, comme un cadenas sur une porte déjà percée. Elle doit être la forme même de la structure.

ToM est un protocole de transport décentralisé où chaque appareil — téléphone, ordinateur, tablette, télévision connectée — est simultanément client et relais. Pas de serveur central. Pas d'infrastructure hébergée. Pas d'entité juridique à qui adresser une injonction. Le réseau *est* les appareils des utilisateurs, et rien d'autre.

Cette formule semble simple. Ses implications sont profondes.

Pour qu'un État censure ToM, il lui faudrait éteindre tous les appareils de ses citoyens simultanément. Pour qu'une entreprise monétise les données, il lui faudrait casser le chiffrement de bout en bout — qui est mathématiquement non-négociable, intégré dans le protocole lui-même, pas dans une politique de confidentialité révisable à la prochaine mise à jour. Pour qu'un acteur malveillant coupe le réseau, il lui faudrait éliminer chaque nœud, un par un, pendant que les autres comblent les vides.

Ce n'est pas de la résilience par redondance. C'est de la liberté par architecture.

---

## III. Chaque nœud est égal — et c'est tout

Le modèle de nœud unifié de ToM est peut-être sa décision politique la plus forte. Chaque appareil qui rejoint le réseau exécute exactement le même code. Il n'y a pas de nœuds "premium" avec plus de droits. Pas de serveurs "maîtres" dont l'opinion compte plus. Pas d'administrateurs capables de bannir, de réduire au silence, de throttler une voix particulière.

Le rôle de relais — forwarding des messages pour d'autres participants — est assigné dynamiquement par le réseau lui-même, en fonction de la contribution de chaque nœud. Ceux qui transmettent plus reçoivent plus. C'est une économie de la réciprocité, pas de la rente. Et personne ne *choisit* d'être relais : le réseau décide, selon des règles transparentes encodées dans le protocole, visibles par tous.

Cette égalité structurelle n'est pas une utopie. Elle est une conséquence directe de l'architecture. Quand il n'y a pas de centre, il n'y a pas de point depuis lequel exercer un pouvoir asymétrique.

ToM emprunte ici une tradition intellectuelle vieille comme la cryptographie moderne : la confiance dans les mathématiques plutôt que dans les institutions. Ed25519 pour les signatures. X25519 pour l'échange de clés. XChaCha20-Poly1305 pour le chiffrement. Ces algorithmes ne respectent aucune frontière nationale, aucune injonction judiciaire, aucun intérêt commercial. Un message chiffré avec ces primitives est illisible pour tout acteur qui ne détient pas la clé privée du destinataire — y compris les opérateurs du réseau, y compris les développeurs du protocole, y compris les forces de l'ordre de n'importe quel État.

La clé privée est l'identité du nœud. Elle ne réside que sur l'appareil de son propriétaire. Elle n'est jamais transmise, jamais sauvegardée sur un serveur tiers, jamais accessible à qui que ce soit d'autre. Votre identité numérique vous appartient parce que vous êtes le seul à pouvoir la prouver.

---

## IV. Le message comme organisme vivant

ToM introduit une métaphore remarquable pour la persistance des messages : le message se comporte comme un organisme vivant.

Quand Alice envoie un message à Bob qui est hors-ligne, le message ne reste pas "en attente" sur un serveur central qu'Alice ou le service devrait maintenir. Il se *réplique* à travers le réseau, porté par les nœuds voisins qui acceptent de le stocker temporairement — comme un virus bénin qui se propage d'hôte en hôte jusqu'à trouver son destinataire. Dès que Bob revient en ligne et reçoit le message, toutes les copies se détruisent. Si personne ne reçoit le message dans les 24 heures, toutes les copies se détruisent quand même.

Pas de stockage permanent. Pas d'archives centralisées. Pas de base de données que l'on pourrait saisir, copier, analyser. Les messages existent le temps d'être transmis, et disparaissent.

Cette décision de design n'est pas seulement technique. Elle exprime une philosophie : la communication est un acte, pas un dossier. Ce que vous dites à quelqu'un lui appartient, à lui seul, au moment où vous le dites. Le réseau n'a pas à en garder trace.

---

## V. La liberté ne devrait pas demander d'effort

Il y a une tentation, dans les projets de liberté numérique, de faire supporter à l'utilisateur le coût de cette liberté. Installez ce client spécial. Apprenez ces commandes. Gérez vos clés manuellement. Acceptez cette friction comme preuve de votre engagement.

ToM refuse cette logique. L'un de ses principes fondateurs — inscrit comme règle non-négociable dans l'architecture — est que le protocole doit être *invisible* à l'utilisateur final. La liberté ne devrait pas requérir une formation. Le chiffrement ne devrait pas demander une décision consciente. Le réseau décentralisé devrait fonctionner exactement comme n'importe quelle application de messagerie, avec la même fluidité, la même réactivité, la même facilité.

C'est une ambition difficile. Elle exige que la complexité technique — la négociation des relais, le hole punching NAT pour les connexions directes, la rotation des clés de groupe, la réplication des messages hors-ligne — soit absorbée entièrement par le protocole, jamais exposée à l'utilisateur.

ToM s'appuie pour cela sur QUIC, le protocole de transport développé par Google et standardisé par l'IETF, qui réduit la latence de connexion, multiplex les flux de données, et intègre nativement le chiffrement TLS 1.3. Par-dessus QUIC, un mécanisme de *hole punching* — le traçage de chemins directs entre deux appareils même derrière des NAT différents — permet aux nœuds de communiquer sans passer par aucun serveur dès que les conditions réseau le permettent. Le relais n'est un intermédiaire que lorsque c'est strictement nécessaire, pas par design.

La liberté maximale avec la friction minimale. C'est le contrat de ToM avec ses utilisateurs.

---

## VI. TCP/IP pour la messagerie — et non pas un produit

ToM ne se pense pas comme une application. Il se pense comme une infrastructure.

TCP/IP n'appartient à personne. Il ne monétise rien. Il ne cherche pas à capter l'attention de ses utilisateurs. Il est une fondation sur laquelle des milliers de services ont été construits — certains libres, certains fermés, certains commerciaux, certains associatifs. Cette neutralité est sa force : personne ne peut le "fermer" parce que personne ne le possède.

ToM vise la même position pour la messagerie. Un protocole ouvert, documenté, implémentable par n'importe qui, sur lequel des applications diverses pourront être construites avec des modèles économiques variés, sans que l'infrastructure elle-même ne soit jamais un vecteur de contrôle, de surveillance ou de dépendance.

C'est une distinction qui semble subtile mais qui est fondamentale. Signal est une application — excellente, respectueuse de la vie privée, mais centralisée sur les serveurs de la Signal Foundation. Fermer Signal est juridiquement possible. Fermer ToM exigerait d'éteindre chaque appareil qui exécute son code, partout dans le monde, simultanément.

Il n'y a pas d'entité à poursuivre. Pas de serveur à saisir. Pas de CEO à arrêter.

---

## VII. La preuve par l'expérience

ToM n'est pas une idée abstraite. C'est un protocole en cours de développement et de test dans des conditions réelles.

Des expériences récentes ont mesuré sa résilience de manière concrète : trois appareils Apple — un iPad, un iPhone, une Apple TV — connectés au même réseau Wi-Fi local, échangeant des milliers de messages via le protocole. Le relais privé utilisé comme infrastructure de coordination a été délibérément éteint, sans prévenir les appareils.

Résultat : zéro interruption. Zéro déconnexion. Les messages ont continué à s'échanger sans la moindre intervention humaine, le réseau basculant automatiquement sur une infrastructure de relais publique de secours. Les appareils ne savaient pas que leur relais primaire avait disparu. Ils ont simplement continué.

C'est ce que la résilience architecturale signifie en pratique. Le réseau ne tombe pas quand un composant tombe. Il s'adapte, se reconfigure, trouve un autre chemin. Pas parce qu'un administrateur est intervenu. Parce que le protocole l'a prévu.

Cette résilience n'est pas un luxe. Dans les situations où la communication libre est le plus nécessaire — crises politiques, catastrophes naturelles, régimes autoritaires — c'est une nécessité absolue.

---

## VIII. Ce que ToM ne fait pas — et pourquoi c'est important

La liberté véritable inclut la liberté de ne pas être contrôlé par son propre outil de liberté.

ToM n'a pas de système de réputation permanente. Un nœud qui se comporte mal voit sa contribution pénalisée progressivement, mais n'est jamais banni de manière définitive. Les punitions s'effacent avec le temps. Personne — ni les développeurs du protocole, ni les opérateurs de relais, ni les utilisateurs avancés — ne peut exclure définitivement quelqu'un du réseau.

C'est une décision contestable. Elle accepte un risque de spam, d'abus, de comportements malveillants. Mais elle refuse un risque plus grand : celui du réseau-outil, où les mêmes mécanismes créés pour protéger peuvent être retournés pour exclure. Les listes noires permanentes sont des outils de censure autant que de protection. ToM choisit de ne pas en avoir.

Il n'y a pas non plus d'état de protocole visible pour l'utilisateur. Le réseau ne vous dit pas si vous êtes en mode relais ou client. Il ne vous demande pas de décider. Il ne vous expose pas à des questions dont vous n'avez pas à vous préoccuper. La couche protocole est invisible — non pas parce qu'on vous cache quelque chose, mais parce qu'un protocole qui se préoccupe de son propre fonctionnement n'est pas encore mature.

---

## IX. L'héritage de Durov

Pavel Durov a construit Telegram parce qu'il croyait que les gens avaient droit à une communication privée, et que cette croyance valait d'être défendue même contre les États, même contre les pressions économiques, même au prix personnel d'une arrestation.

ToM est une tentative d'aller plus loin. Pas seulement une application privée mais un protocole libre. Pas seulement un service respectueux des données mais une infrastructure qui rend la collecte de données structurellement impossible. Pas seulement une entreprise avec de bonnes intentions mais un bien commun que personne ne peut s'approprier.

L'internet a été construit par des gens qui croyaient que l'information voulait être libre. Ces gens avaient raison sur le fond, mais ils ont sous-estimé la puissance des forces économiques et politiques qui allaient centraliser cette infrastructure. ToM est une correction architecturale à cette sous-estimation.

La liberté numérique ne se décrète pas. Elle ne se promet pas dans des conditions d'utilisation. Elle se construit, protocole par protocole, décision d'architecture par décision d'architecture, dans chaque choix qui rend le contrôle structurellement plus difficile et la communication libre structurellement plus robuste.

ToM est ce choix, rendu code.

---

## Conclusion : la liberté comme propriété émergente

La liberté n'est pas une fonctionnalité que l'on peut ajouter à un système conçu pour le contrôle. Elle est une propriété émergente de systèmes conçus pour être libres.

Le protocole ToM propose que la liberté de communication ne devrait dépendre d'aucune bonne volonté institutionnelle, d'aucune promesse contractuelle, d'aucun courage individuel sous pression. Elle devrait être la conséquence naturelle d'un réseau dont chaque nœud est égal, dont chaque message est chiffré, dont chaque architecture refuse le point de contrôle.

C'est une ambition technique. C'est aussi un acte politique.

Dans un monde où les gouvernements criminalisent le chiffrement, où les entreprises monétisent la surveillance, où les plateformes censurent sous couvert de modération, construire une infrastructure qui rend tout cela structurellement impossible n'est pas un projet marginal. C'est peut-être le projet le plus urgent de notre temps.

Le réseau ToM ne demande pas la permission d'être libre. Il l'est par construction.

---

*ToM Protocol — The Open Messaging — est un projet open source de transport P2P décentralisé. Code disponible, architecture documentée, protocole ouvert.*
