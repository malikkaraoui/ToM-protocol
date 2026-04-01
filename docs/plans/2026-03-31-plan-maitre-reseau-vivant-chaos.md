# 2026-03-31 — Plan maître réseau vivant + chaos multi-device

## Intention

Passer de :

- un **amorçage validé**,
- un **build Apple revenu à la vie**,
- un **réglage `peer_present_k = 8` déjà gagnant**,

à :

- un **réseau ToM vivant**, 
- **observable en live**, 
- **soumis à du churn et du chaos**,
- capable de **survivre à la disparition des porteurs initiaux de l’amorçage**,
- et montrant que la **fonction d’entrée devient distribuée, copiée, relayée, puis non-critique** une fois le réseau convergé.

Le but n’est pas de traiter le NAS ou l’Apple TV comme des serveurs spéciaux du protocole.
Leur différence est uniquement pratique : **ils restent connectés plus longtemps** et servent donc de noyau de stabilité pour simuler un réseau où “il y a toujours du monde”.

---

## Vérités déjà figées

Avant toute extension de campagne, on part des faits suivants :

1. **Branche de travail cohérente** : `copilot/bootstrap-pump-skeleton`
2. **Build Xcode revenu** : le chemin de build Apple fonctionne à nouveau et doit être préservé
3. **Chaîne produit validée** : `PeerPresent -> GossipNeighborUp -> delivery`
4. **Référence de réglage bootstrap relay-assisted** : `peer_present_k = 8`
5. **Amorçage déjà mûri** : mèche allumée, pipeline bootstrap/discovery réellement branché

### Changements locaux à préserver explicitement

Ces changements ne sont **pas du bruit** et ne doivent pas être écrasés :

- `apps/tom-node-tvos/TomNode.xcodeproj/project.pbxproj`
	- ajout d’une phase **Build Rust FFI**
- `apps/tom-node-tvos/TomNode/Services/build_ffi.sh`
	- script de build FFI Apple rendu exploitable depuis Xcode
- `apps/tom-node-tvos/TomNode/Services/TomNodeService.swift`
	- auto-start, auto-ping, seed de routes relay, export logs UDP
- `apps/tom-node-tvos/TomNode/TomNodeApp.swift`
	- déclenchement auto-start côté app
- `crates/tom-transport/src/protocol.rs`
	- auto-apprentissage de route relay sur connexion entrante
- `crates/tom-transport/src/node.rs`
	- injection du pool pour supporter cet apprentissage

### Nettoyage déjà effectué

Les artefacts trackés sous `apps/tom-node-tvos/.build/xcode/**` ont été restaurés depuis `HEAD` pour supprimer le faux bruit Git.

---

## Vision cible de la prochaine étape

Le réseau cible à démontrer n’est pas :

- un bootstrap fixe,
- un NAS maître,
- une Apple TV sacrée,
- une campagne propre “de labo” sans churn.

Le réseau cible à démontrer est :

1. un petit noyau de nœuds stables fait naître le réseau,
2. d’autres nœuds entrent automatiquement,
3. la fonction d’amorçage est **recopiée / relayée / distribuée**, 
4. le porteur initial peut tomber,
5. le réseau continue,
6. les rôles utiles se redistribuent selon la topologie et la contribution,
7. le système reste lisible grâce à une tour de contrôle de logs et métriques.

---

## Parc machines cible

## Noyau stable (toujours présent au départ)

- **NAS Freebox**
	- relay principal de terrain
	- discovery
	- metrics / health
- **Apple TV / tvOS**
	- nœud applicatif stable et always-on
- **MacBook Pro**
	- tour de contrôle
	- orchestrateur
	- **4 nœuds simulés locaux** si besoin

## Satellites réels à introduire dans la boucle

- **MacBook Air 2011**
	- nœud ancien / plus fragile / Wi‑Fi potentiellement moins stable
- **iPad**
	- nœud Apple mobile local
- **iPhone 12 Pro**
	- nœud Apple mobile
	- **entrée/sortie via 4G/5G**

## Règle de modélisation

Ces machines ne sont **pas** des rôles protocolaires différents.
Elles ont le **même modèle ToM**.
La différence vient de :

- disponibilité,
- qualité réseau,
- durée de présence,
- capacité à rester branché,
- capacité à relayer / héberger des copies / servir de point de passage.

---

## Rôles et fonctions à tester

La campagne doit couvrir les rôles/fonctions réellement présents dans le projet.

### 1. Peer standard

Rôle de base de tout nœud entrant.

À observer :

- entrée réseau,
- apprentissage de voisins,
- livraison simple,
- disparition / retour sans corruption d’état.

### 2. Relay dynamique

Rôle attribué par le réseau selon contribution / utilité.

À observer :

- promotion,
- démotion,
- continuité de routage,
- comportement quand un relay utile disparaît.

### 3. Fonction de pompe d’amorçage / “secrétaire”

Ce n’est **pas** un serveur central ; c’est une fonction tournante.

À observer :

- naissance du réseau,
- handoff,
- disparition du porteur initial,
- maintien d’une entrée réseau par d’autres porteurs.

### 4. Backup holders

Nœuds qui portent temporairement des copies pour la livraison différée.

À observer :

- réplication,
- suffisance / insuffisance de backup,
- montée en charge du rôle quand le réseau manque de porteurs stables,
- purge après ACK / TTL.

### 5. Chaîne hub de groupe

Rôles de failover explicitement documentés :

- **Primary**
- **Shadow**
- **Candidate**

À observer :

- watchdog,
- promotion,
- restauration de chaîne,
- impact réel sur groupe vivant sous churn.

### 6. Embedded relay / relay local

Fonction utile pour certains nœuds capables d’héberger un relay embarqué.

À observer :

- démarrage,
- santé,
- impact sur la connectivité locale,
- non-confusion entre “healthy” local et “publiable” réseau.

### 7. L1

Rappel de doctrine : **L1 ancre, n’arbitre pas**.

Conséquence pour cette campagne :

- on ne le traite pas comme centre de décision opérationnelle,
- on vérifie seulement que le plan de test ne lui attribue jamais un rôle de chef.

---

## Tour de contrôle live — exigence non négociable

Le plan doit donner un œil partout en temps réel.

## Objectif

Voir en direct :

- qui est vivant,
- qui rejoint,
- qui chute,
- qui reprend,
- quel bootstrap hint a fonctionné,
- quand le réseau n’a plus besoin du porteur initial,
- quels rôles changent,
- quand des backups manquent,
- quand un hub bascule,
- quand un relay restart sans casser durablement le réseau.

## Architecture cible

### A. MacBook Pro = centre de contrôle

Le MacBook Pro héberge :

1. **collecteur de logs central**
2. **scraping relay/discovery**
3. **vue synthétique des nœuds**
4. **orchestrateur de scénario / chaos**

### B. Logs device → collector

Canal minimal recommandé :

- **UDP JSONL** pour les clients Apple et autres clients légers

Point de départ déjà présent :

- `TomNodeService.swift` sait déjà exporter des logs via UDP

À généraliser vers :

- tvOS
- futur iOS/iPadOS target
- `tom-stress`
- `tom-tui` / nœuds desktop si nécessaire

### C. Infra metrics

Réutiliser l’existant :

- Prometheus
- Grafana
- endpoints `/ready`, `/health`, `/healthz`, `/metrics`, `/status`, `/relays`

### D. Vue unifiée de campagne

Le centre de contrôle doit agréger au minimum :

- logs événements structurés par nœud
- santé relay/discovery
- chronologie des coupures / retours
- statut bootstrap/convergence
- changements de rôle détectés

## Données minimales à normaliser dans chaque log

- `timestamp_wall`
- `node_label`
- `device_class`
- `node_id`
- `event_type`
- `phase`
- `peer_id` (si pertinent)
- `group_id` (si pertinent)
- `bootstrap_source` (`mDNS`, `PeerPresent`, `DHT`, `manual`)
- `role_state`
- `message`

## Livrable technique prioritaire associé

Créer un **mini serveur de collecte live** côté MacBook Pro.

Forme minimale acceptable :

- UDP listener
- persistance JSONL par nœud
- vue terminal live ou petite UI web locale

---

## Orchestration et chaos — ce qu’il faut vraiment pouvoir faire

## Chaos global

- coupures aléatoires massives
- retours aléatoires
- temps morts
- rafales de trafic
- phases calmes
- relay actif puis inactif
- disparition/reprise du porteur d’amorçage

## Chaos ciblé

- couper le NAS relay
- couper l’Apple TV
- faire dormir / revenir un iPad
- faire sortir / rentrer l’iPhone en 4G/5G
- tuer 1 ou 2 des 4 nœuds du MacBook Pro
- dégrader le MacBook Air 2011 comme nœud faible

## Trafic automatique

Le réseau doit vivre sans pilotage manuel permanent :

- ping/pong
- burst
- messages aléatoires
- groupes
- invitations auto-acceptées dans certains scénarios
- périodes silencieuses
- reprises après silence

## Condition importante

Pour les appareils Apple réels (tvOS / iPad / iPhone), une automatisation fine des coupures ciblées demandera probablement un **petit agent de contrôle de test** dans l’app.

Sans cela :

- le Mac et le NAS sont automatisables fort,
- les appareils Apple restent partiellement manuels ou semi-automatisés.

### Conséquence produit/dev

Avant la campagne chaos complète, prévoir un **canal de contrôle test** pour l’app Apple, limité au LAN de test :

- `start_runtime`
- `stop_runtime`
- `toggle_udp_log_export`
- `set_relay_url`
- `set_bootstrap_peer`
- `inject_idle_window`
- `simulate_disconnect` (arrêt runtime, pas coupure OS magique)
- `resume`

---

## Phases de campagne recommandées

## Phase 0 — Freeze propre

But : figer l’état avant d’ouvrir un nouveau front.

Sorties attendues :

- artefacts Xcode parasites nettoyés
- changements utiles identifiés et préservés
- doc de campagne maître rédigée

## Phase 1 — Tour de contrôle

But : voir tout le monde avant de faire n’importe quoi.

À mettre en place :

- collector UDP central
- scraping relay/discovery
- stockage JSONL
- dashboard synthétique des nœuds actifs

Critère de sortie :

- chaque nœud de test visible depuis le MacBook Pro en live

## Phase 2 — Noyau stable

But : naissance propre du réseau avec noyau stable.

Topologie minimale :

- NAS
- Apple TV
- 2 nœuds MBP

Critères :

- convergence bootstrap sans saisie longue
- `NeighborUp`
- premier message utile
- visibilité live complète

## Phase 3 — Entrée des satellites

But : faire entrer progressivement :

- MacBook Air 2011
- iPad
- iPhone 12 Pro
- 2 nœuds MBP additionnels

Critères :

- entrée automatique ou quasi-automatique
- identification de la source d’amorçage effective
- pas de dépendance durable au porteur initial

## Phase 4 — Handoff d’amorçage

But : démontrer que le réseau n’a plus besoin du porteur initial.

Scénarios :

- couper le NAS après convergence
- couper l’Apple TV après convergence
- couper le porteur qui a effectivement servi de “secrétaire”

Succès :

- les nouveaux entrants trouvent encore le réseau
- ou le réseau vivant continue sans effondrement même si l’entrée devient plus lente

## Phase 5 — Distribution des rôles

But : observer qui prend le travail utile quand il manque des porteurs.

Sous-tests :

- promotion/démotion relay
- raréfaction des backups
- montée en charge de nœuds plus disponibles
- failover de groupe Primary/Shadow/Candidate

Succès :

- redistribution lisible
- pas de centre fixe codé en dur

## Phase 6 — Chaos dur

But : casser le réseau sans casser la vérité du modèle.

Scénarios :

- churn aléatoire fort
- relay restart
- coupures ciblées
- temps morts + réveils
- phases burst + silence

Succès :

- redémarrages absorbés
- reconvergence automatique
- logs exploitables

## Phase 7 — WAN réel

But : faire entrer le monde non-LAN.

Cas cible :

- iPhone 12 Pro en 4G/5G

Succès :

- entrée relay-assisted réelle
- participation utile au réseau
- sortie/retour sans devoir “reconstruire le monde”

## Phase 8 — Endurance

But : voir si le réseau tient dans la durée.

Durée cible :

- 1h puis 6h puis 24h selon maturité

À observer :

- churn moyen
- besoin résiduel du bootstrap
- stabilité des rôles
- pression backup
- volume de reconnexions

---

## Scénarios de test prioritaires

1. **Cold start sans seed manuel lourd**
2. **Convergence avec `peer_present_k = 8`**
3. **Disparition du premier porteur d’amorçage**
4. **Redémarrage du relay NAS en pleine vie réseau**
5. **Apple TV stable + MacBook Pro 4 nœuds = mini-réseau vivant**
6. **MacBook Air 2011 comme nœud fragile**
7. **iPad qui dort puis revient**
8. **iPhone 12 Pro qui entre/sort via 4G/5G**
9. **Failover de groupe Primary → Shadow → Candidate**
10. **Manque de backup holders puis redistribution du travail**

---

## Backlog préalable avant campagne complète

## P0 — obligatoire

1. **Figer les changements Apple/runtime utiles**
2. **Créer le collector live central**
3. **Brancher tous les nœuds de test possibles sur une sortie logs commune**
4. **Définir un identifiant lisible par nœud/device**
5. **Outiller le lancement de 4 nœuds MBP en parallèle**

## P1 — fortement recommandé

6. **Étendre l’app Apple actuelle vers un target iOS/iPadOS**
7. **Ajouter un canal de contrôle test pour iPhone/iPad/Apple TV**
8. **Ajouter la journalisation explicite des changements de rôle**
9. **Rendre observable la pression backup / manque de backup holders**

## P2 — après premiers runs

10. **Dashboard unifié réseau vivant**
11. **Orchestrateur de chaos reproductible**
12. **Rapport automatique de campagne (timeline + incidents + succès)**

---

## Ce qu’on ne fait pas maintenant

- on ne repart pas dans une nouvelle théorie de bootstrap
- on ne refait pas une matrice mono-Mac `k=8/16/32`
- on ne transforme pas le NAS ou l’Apple TV en rôles protocolaires spéciaux
- on ne fait pas de “mode campagne bricolé” séparé de la cible produit/dev

---

## Critères de réussite de cette prochaine étape

On pourra considérer la prochaine étape atteinte si :

1. le réseau naît à partir du noyau stable,
2. plusieurs devices hétérogènes rejoignent automatiquement,
3. la tour de contrôle voit tout le monde en live,
4. le porteur initial de l’amorçage peut tomber,
5. le réseau continue de fonctionner,
6. les rôles utiles se redistribuent,
7. le WAN (iPhone 4G/5G) entre réellement dans la boucle,
8. les coupures et retours ne remettent pas le réseau à zéro.

---

## Décision opérationnelle

La prochaine marche n’est **pas** “plus de dev au hasard”.

La prochaine marche est :

1. **geler l’état utile actuel**, 
2. **monter la tour de contrôle live**, 
3. **construire la campagne réseau vivant + chaos**,
4. **faire parler tout le monde automatiquement**,
5. **mesurer si l’amorçage devient bien une fonction distribuée puis non-critique**.
