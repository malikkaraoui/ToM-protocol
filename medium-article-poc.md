# ToM Protocol perce les murs. Littéralement.

## Comment un protocole P2P a percé deux NAT, deux pays et un réseau WiFi d'école — en 1.4 seconde.

---

Il y a une semaine, ToM Protocol avait 771 tests qui passaient, une architecture P2P propre en TypeScript, et un signaling server WebSocket qui faisait le job.

Il y a trois jours, on s'est posé la vraie question : **est-ce que ça marche sans le serveur ?**

Pas "en théorie". Pas "dans un whitepaper". Pas "sur localhost".

Sur un vrai réseau. Avec un vrai NAT. Entre deux vrais pays.

La réponse est oui. Et on a les JSON pour le prouver.

---

## Le problème que personne ne veut affronter

Chaque protocole P2P finit par buter sur le même mur : le NAT.

Ton appareil n'a pas d'adresse publique. Il est derrière une box, derrière un opérateur, derrière un firewall d'entreprise ou d'école. Et l'appareil d'en face aussi.

La solution classique : un serveur relay permanent. Tous les messages passent par un tiers. C'est ce que font Signal (pour la signalisation), WhatsApp (pour les appels), et à peu près tout le monde.

C'est fonctionnel. Mais ce n'est pas du P2P. C'est du "P2serveur2P".

Le vrai P2P, c'est quand deux machines se trouvent et se parlent directement. Sans intermédiaire permanent. Le relay est un tremplin, pas une destination.

Pour ça, il faut percer le NAT. C'est ce qu'on appelle le **hole punching**.

---

## Pourquoi c'est difficile (et pourquoi la plupart des projets évitent le sujet)

Le hole punching UDP, sur le papier, c'est simple : les deux côtés envoient un paquet vers l'autre en même temps, ce qui "perce" un trou dans le NAT de chaque côté.

En pratique, c'est un cauchemar :

- **NAT résidentiel** (ta box) : généralement percable, mais les comportements varient entre FAI
- **CGNAT opérateur** (4G, fibre partagée) : ton opérateur met des centaines d'utilisateurs derrière la même IP publique. Les mappings sont agressifs, les ports changent, les timeouts sont courts
- **NAT d'entreprise/école** : firewalls restrictifs, ports bloqués, inspection de paquets
- **Double NAT** : quand les deux côtés sont derrière un NAT. C'est le cas le plus courant et le plus difficile

La plupart des projets P2P contournent le problème : ils gardent un serveur relay permanent et appellent ça "décentralisé". Ou ils documentent le hole punching comme un "nice to have" qu'ils implémenteront "plus tard".

ToM ne peut pas se permettre ce luxe. Le protocole est conçu pour fonctionner sans infrastructure fixe. Si le hole punching ne marche pas, le projet ne marche pas.

---

## Le choix d'iroh (et pourquoi pas libp2p)

Avant de coder quoi que ce soit, on a étudié trois options en profondeur.

**libp2p** (Protocol Labs, la fondation derrière IPFS) : le standard de facto du P2P. Multi-langage, écosystème énorme, spécification formelle. Mais libp2p est conçu relay-first : la connexion initiale passe par un relay, et le hole punching est une optimisation optionnelle. Pour un protocole qui vise le zéro-infra, c'est un mauvais point de départ philosophique.

**Hyperswarm** (Holepunch, les créateurs de BitTorrent) : DHT-first, philosophie proche de ToM. Simple, efficace. Mais Node.js uniquement, pas de support browser, et la communauté est plus petite.

**iroh** (n0-computer, Rust, MIT, 7800+ stars) : QUIC natif, hole punching intégré avec relay fallback automatique, ~90% de connexions directes en production. Et surtout : identité = clé Ed25519 (exactement le modèle de ToM), relais stateless (exactement la philosophie de ToM), chiffrement E2E automatique via QUIC TLS.

iroh n'est pas une dépendance permanente. Le plan est en trois phases : PoC avec iroh → fork stratégique des modules nécessaires → indépendance complète. Mais pour valider la faisabilité du hole punching, c'est l'outil parfait.

---

## 3 jours, 4 PoC, du Hello World au cross-border

### Jour 1 — PoC-1 et PoC-2 : les fondations

**PoC-1 : Echo QUIC.** Deux noeuds, même machine. Un envoie "Hello", l'autre répond. Connexion en 289ms, RTT 125ms via le relay iroh européen (auto-assigné, zéro configuration). Le "Hello World" de QUIC. Ça compile, ça tourne, ça chiffre automatiquement.

**PoC-2 : Gossip peer discovery.** Trois noeuds qui se découvrent sans registre central via HyParView/PlumTree — un protocole épidémique où chaque noeud propage l'information de proche en proche. Neighbor détecté en 257ms, broadcast instantané. C'est exactement ce dont ToM a besoin pour remplacer son signaling server.

### Jour 2 — PoC-3 : l'architecture cible

**PoC-3 : Gossip pour la découverte, QUIC direct pour les messages.**

C'est le PoC le plus important conceptuellement. Le gossip sert uniquement à annoncer sa présence (ANNOUNCE). Les messages eux-mêmes transitent par des streams QUIC directs — pas par le gossip. Deux couches, deux rôles :

```
Gossip (HyParView)  →  "Je suis là, je m'appelle Alice"
QUIC direct          →  "Voici mon message chiffré pour toi"
```

C'est l'architecture cible de ToM : découverte décentralisée + payload direct. Pas de mélange des responsabilités.

Résultat : découverte en 3 secondes, livraison du premier message en 4.8 secondes. En chemin, trois bugs subtils découverts et corrigés :

1. **Timing ANNOUNCE** : le premier broadcast partait avant qu'aucun neighbor ne soit connecté. Solution : re-broadcast à chaque `NeighborUp`.
2. **Race condition QUIC** : fermer la connexion côté envoyeur tuait le stream avant que le receveur ne lise la réponse. Solution : handshake au niveau du stream, pas de la connexion.
3. **EOF stdin** : la boucle de lecture ne détectait pas la fermeture du pipe. Corrigé avec un check sur le nombre d'octets lus.

Trois bugs qui n'existent que quand on teste pour de vrai. Pas dans les mocks.

### Jour 3 — PoC-4 : le test de vérité

Tout le reste n'était qu'un échauffement.

---

## PoC-4 : percer les NAT pour de vrai

### Le setup

Un binaire Rust instrumenté. Sortie JSON structurée pour chaque événement (connexion, changement de chemin, ping, hole punch, résumé). Pas de stdin, pas d'interactivité — tout automatique.

Cross-compilé en **binaire statique ARM64** via `cargo-zigbuild` (16 MB, zéro dépendance dynamique), déployé sur un **NAS Freebox Delta** : une VM Debian tournant sur un processeur ARM Cortex-A72, le genre de machine qu'on trouve dans un salon, pas dans un datacenter.

Le MacBook Pro sert de connecteur. Le NAS sert de listener.

Le binaire observe en temps réel le chemin de la connexion via l'API `connection.paths()` d'iroh : relay ou direct, RTT par chemin, adresse distante. Chaque changement de chemin génère un événement JSON horodaté.

### Scénario A — Même réseau WiFi (baseline)

```
MacBook (WiFi maison) ←→ NAS Freebox (même réseau)
```

Hole punch en **0.37 seconde**. 20 pings, **100% directs**. RTT moyen : **49ms**. Chemin : IPv4 publique de la Freebox. Zéro relay après le premier échange.

C'est la baseline. Facile. Les deux machines sont sur le même réseau.

### Scénario B — 4G CGNAT (le vrai défi)

```
MacBook (hotspot USB iPhone 12 Pro, 4G) ←→ NAS Freebox (WiFi maison)
```

Le WiFi du MacBook est coupé. Le trafic passe par le réseau cellulaire de l'opérateur. CGNAT : des milliers d'abonnés partagent la même IP publique. Les mappings NAT sont agressifs.

Hole punch en **2.9 secondes**. 10 pings, **90% directs** (seul le premier passe par le relay). RTT direct moyen : **107ms** (cohérent avec la latence 4G). Chemin direct établi via l'IP publique de la Freebox.

**Le CGNAT opérateur est percable.** 2.9 secondes d'attente, puis connexion directe. Le relay n'a servi que pour le premier ping.

### Scénario C — Cross-border (le moment où tout change)

```
MacBook (WiFi invité, école en Suisse) ←→ NAS Freebox (France)
```

Pas prévu. On était en train de finir les tests quand le MacBook s'est retrouvé connecté au WiFi d'une école. En Suisse. Sur un réseau guest. Derrière un NAT restrictif d'établissement scolaire.

Le NAS tourne toujours en France, derrière la box résidentielle.

On lance le test. Sans rien changer. Sans aucune configuration.

```json
{"event":"started","name":"MacBook-Suisse","mode":"connect"}
{"event":"path_change","selected":"RELAY","rtt_ms":157}
{"event":"ping","seq":1,"rtt_ms":122,"via":"RELAY"}
{"event":"path_change","selected":"DIRECT","rtt_ms":28}
{"event":"ping","seq":2,"rtt_ms":37,"via":"DIRECT"}
{"event":"hole_punch","success":true,"time_to_direct_s":1.36}
{"event":"ping","seq":3,"rtt_ms":28,"via":"DIRECT"}
...
{"event":"summary","direct_pings":19,"relay_pings":1,"direct_pct":95.0,"avg_rtt_direct_ms":32}
```

Hole punch en **1.4 seconde**. 20 pings, **95% directs**. RTT moyen : **32ms**.

32 millisecondes. Suisse → France. À travers le NAT d'une école et celui d'une box résidentielle. Sans port forwarding. Sans STUN. Sans TURN. Sans configuration.

Le relay iroh (`euc1-1.relay.n0.iroh-canary.iroh.link`, auto-assigné en Europe) n'a servi que pour le tout premier ping. Ensuite, le chemin direct est établi — via l'IP publique de la Freebox pour l'IPv4, et l'adresse IPv6 globale pour l'alternative.

---

## Les chiffres, résumés

| Scénario | Topologie | Upgrade relay→direct | RTT direct | Pings directs |
|----------|-----------|---------------------|-----------|---------------|
| LAN WiFi | Même réseau | 0.37s | 49ms | 100% |
| 4G CGNAT | iPhone hotspot ↔ WiFi maison | 2.9s | 107ms | 90% |
| Cross-border | WiFi école Suisse ↔ Freebox France | 1.4s | 32ms | 95% |

**100% de succès de hole punching.** Sur les trois scénarios réels testés. Aucun échec.

---

## Ce que ça change pour ToM Protocol

### 1. Le signaling server a un successeur

Le serveur WebSocket de signalisation — le seul composant centralisé du protocole — a désormais un remplaçant testé. Le gossip (PoC-2/3) remplace la découverte. Le QUIC direct (PoC-4) remplace le transport. Le plan d'élimination n'est plus théorique.

### 2. L'architecture cible est validée

Gossip pour annoncer sa présence. QUIC pour envoyer les messages. Relay comme tremplin, pas comme béquille. C'est exactement ce que les PoC-3 et PoC-4 démontrent, avec des données réelles.

### 3. "Zéro infra" devient mesurable

Pas de port forwarding configuré. Pas de serveur STUN déployé. Pas de serveur TURN provisionné. Le relay iroh est un service public partagé (et peut être auto-hébergé). Le reste est du hole punching pur.

### 4. Le chiffrement est gratuit

QUIC inclut TLS nativement. Chaque connexion est chiffrée de bout en bout sans couche supplémentaire. L'identité est une clé Ed25519 — exactement le modèle que ToM utilise déjà.

---

## Ce qui reste à faire

Le PoC prouve la faisabilité. Il ne prouve pas la production.

**Fork stratégique** : extraire les modules iroh nécessaires (connectivité QUIC, hole punching, relay fallback, gossip) et les adapter au wire format ToM (enveloppes JSON signées, rôles dynamiques, virus backup).

**Intégration** : remplacer le transport WebRTC + signaling WebSocket du SDK TypeScript par le transport QUIC natif. Les deux stacks devront coexister pendant la transition.

**Tests à plus grande échelle** : le PoC valide le 1-à-1. Il faudra tester le comportement avec 10, 50, 100 noeuds simultanés. Le gossip HyParView est conçu pour scaler — mais "conçu pour" et "prouvé à" sont deux choses différentes.

**Résilience** : que se passe-t-il quand le relay est down ? Quand le hole punch échoue (ça arrivera sur certains réseaux d'entreprise très restrictifs) ? Quand un noeud change de réseau en cours de conversation ?

---

## Le vrai résultat

Il y a 10 jours, ToM Protocol était un protocole P2P qui marchait — avec un cordon ombilical.

Aujourd'hui, c'est un protocole P2P qui a prouvé qu'il peut couper le cordon.

Pas avec un whitepaper. Pas avec une simulation. Avec un `cargo run` depuis une école en Suisse, un NAS dans un salon en France, et 32ms de latence directe entre les deux.

---

*Repo : [github.com/malikkaraoui/ToM-protocol](https://github.com/malikkaraoui/ToM-protocol)*
*Branche PoC : `experiment/iroh-poc`*
*Stack : iroh 0.96 (Rust), QUIC, Ed25519, HyParView gossip*
*Tests : 771 passing (TypeScript core) + 4 PoC validés (Rust)*
