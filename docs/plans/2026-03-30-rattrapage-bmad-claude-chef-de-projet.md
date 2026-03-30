# Rattrapage chef de projet — BMAD + Claude + vision ToM

**Date :** 2026-03-30
**But :** éviter que Malik doive réexpliquer demain la naissance, la vision et la logique de pilotage du projet.

---

## 1. Ce qu'est ToM au fond

ToM n'est pas un produit de messagerie.

ToM est une **couche protocolaire** de transport distribué, pensée comme un équivalent de :

- TCP/IP,
- HTTP,
- SSH,

mais pour une communication où :

- les utilisateurs sont aussi l'infrastructure,
- il n'y a pas de serveur central durable,
- les messages vivent, circulent, puis disparaissent,
- la participation doit être invisible pour l'utilisateur final.

La formule la plus juste est :

> **ToM est une fondation réseau invisible, pas une application.**

---

## 2. ADN fondateur issu des artefacts BMAD

### Source fondatrice principale

Le document le plus important pour comprendre l'intention initiale est :

- `_bmad-output/planning-artifacts/product-brief-tom-protocol-2026-01-30.md`

On y retrouve les idées structurantes suivantes :

- refus de la centralisation cloud / plateformes / intermédiaires,
- refus de la spéculation et des tokens,
- refus d'une dépendance à une fondation, à des serveurs ou à un financeur,
- volonté d'un réseau **auto-soutenu**,
- volonté d'une intégration **invisible** dans les applications et devices.

### Traduction produit

Le document de référence pour la traduction opérationnelle est :

- `_bmad-output/planning-artifacts/prd.md`

On y retrouve :

- le **"Satoshi moment"** de ToM,
- la logique d'adoption par les développeurs,
- la distribution pensée aussi pour les LLMs et assistants de code,
- la montée progressive : PoC → SDK → intégration invisible.

### Traduction architecture

Le document fondamental côté design initial est :

- `_bmad-output/planning-artifacts/architecture.md`

Il montre :

- l'origine browser-first / TypeScript,
- le bootstrap minimal comme compromis transitoire,
- la pensée en itérations progressives,
- l'importance de la purge, du stateless, et de l'absence de séparation client/serveur.

---

## 3. Le vrai cap n'a pas changé

Même si la stack a évolué, la vision profonde est restée cohérente.

### Ce qui n'a pas changé

- ToM doit être **invisible** pour l'utilisateur final.
- ToM ne doit pas reposer sur une infra fixe à long terme.
- ToM doit être **auto-soutenu**.
- Le réseau doit pouvoir survivre au retrait du porteur initial.
- Les rôles doivent être **assignés / portés dynamiquement**, pas choisis librement par les nœuds.
- L'objectif n'est pas un produit brandé, mais un **socle universel**.

### Ce qui a changé en exécution

Le chemin réel a divergé de la narration BMAD d'origine :

- départ TypeScript / browser-first,
- puis pivot fort vers **Rust natif**,
- fork de la stack `iroh` vers `tom-*`,
- priorité à un **socle réseau crédible** avant le polish produit.

Ce changement est assumé dans :

- `CLAUDE.md`
- `README.md`
- `docs/plans/2026-03-21-macro-roadmap-realignment.md`

---

## 4. Le vrai état actuel du projet

### Ce qui est déjà prouvé

Le repo et les validations récentes montrent que plusieurs briques sont désormais réelles :

- transport QUIC natif,
- NAT traversal / hole punching,
- discovery relay-aware,
- relay embarqué,
- publication et consommation de relays,
- groupe / failover / backup / ACK,
- stress réel Mac ↔ NAS,
- Apple TV comme 3e nœud validé en trafic réel.

### Ce qui n'est pas encore prouvé

Ce qui manque encore à la promesse ToM n'est plus un simple "peut-on faire circuler un message ?".

Le verrou principal est maintenant :

> **l'amorçage autonome et la survie du réseau après retrait du seed initial**

Autrement dit :

- le socle fonctionne,
- mais le réseau n'a pas encore démontré sa capacité à se porter lui-même sans béquille de départ.

---

## 5. Le point critique révélé le 30 mars 2026

Les tests Apple TV / Mac / NAS ont confirmé quelque chose d'important :

- le réseau ne peut pas aujourd'hui être honnêtement présenté comme "zéro amorçage".

Le test terrain a montré :

- **strict organique** : pas de convergence utile spontanée,
- **seed handoff minimal** : Mac ↔ NAS convergent avec une amorce runtime minimale,
- Apple TV reste gênée si l'amorce demande une saisie longue.

Conclusion :

- supprimer le hardcoding était nécessaire,
- mais ce n'est **pas suffisant**,
- car une saisie manuelle d'un NodeId long revient à déplacer le hardcoding dans l'humain.

---

## 6. Vision longue sur le bootstrap : ce qu'il faut comprendre

Le mot *bootstrap* est trompeur.

Dans l'industrie, il évoque souvent :

- plusieurs serveurs fixes,
- connus à l'avance,
- persistants,
- codés dans le client.

### Ce n'est PAS la cible ToM.

La cible ToM est mieux décrite par la métaphore suivante, déjà présente dans la documentation :

## la secrétaire qui joue à la chaise musicale

Ce que cela signifie :

- la fonction d'entrée dans le réseau existe bien,
- mais elle n'est pas attachée durablement à une machine fixe,
- elle est portée temporairement,
- puis transmise,
- puis reprise par une autre,
- sans centre de décision,
- sans annuaire figé éternel.

La bonne phrase de synthèse est :

> **le rôle existe, le porteur change.**

Cette vision est explicitement cohérente avec :

- `docs/article-ToM protocol.md`
- `docs/plans/2026-02-26-phase-r7-dht-bootstrap-elimination-design.md`
- `docs/plans/2026-03-30-bootstrap-pump-module.md`

---

## 7. Formulation claire à retenir : "pompe d'amorçage"

Pour éviter les malentendus, il faut parler d'un module clair :

## `pompe d'amorçage`

Pourquoi :

- le mot "bootstrap" mélange souvent compromis transitoire et cible finale,
- alors que ToM a besoin d'une **fonction d'amorçage** réelle,
- mais pas d'un **bootstrap central classique**.

La pompe d'amorçage ToM doit être :

- explicite,
- temporaire,
- non centrale,
- remplaçable,
- transmissible,
- reprise par le réseau.

Document de cadrage créé :

- `docs/plans/2026-03-30-bootstrap-pump-module.md`

---

## 8. La bonne roadmap court terme

Le réalignement du 30 mars fixe l'ordre juste.

### Ordre reclassé

1. **Dé-hardcoder l'amorçage**
2. **Prouver la survie du réseau sans les devices du seed initial**
3. **NAS down / seed down**
4. **Mini-stress Apple TV (10 min)**
5. **Groupes**
6. **MacBook Air 2011**
7. **4G/5G / autre Freebox**

Cette re-priorisation est déjà inscrite dans :

- `docs/plans/2026-03-21-macro-roadmap-realignment.md`

---

## 9. Ce qu'un chef de projet doit surveiller maintenant

### Risque n°1 — se raconter une victoire trop tôt

Le socle réseau est devenu crédible.

Mais la promesse ToM complète n'est pas encore démontrée tant que :

- l'entrée dans le réseau reste pénible,
- le seed initial reste indispensable,
- ou le réseau ne sait pas reprendre le flambeau après disparition du porteur initial.

### Risque n°2 — confusion entre vision et preuve

La vision de Malik est claire.

Ce qui reste à produire, ce n'est pas une nouvelle vision.

C'est :

- une traduction technique nette,
- une preuve terrain,
- une architecture d'amorçage explicite.

### Risque n°3 — dispersion

Le projet a déjà beaucoup de briques.

Le danger n'est plus le manque d'idées.

Le danger, c'est :

- d'ouvrir trop de chantiers en parallèle,
- de faire du stress avant de résoudre l'entrée réseau,
- de courir vers les groupes / WAN alors que l'amorçage n'est pas encore une vraie fonction première classe.

---

## 10. Ce qu'il faut retenir pour demain

### Résumé ultra-court

- ToM = protocole invisible, pas appli.
- Vision intacte : réseau auto-soutenu, sans infra fixe durable.
- Exécution réelle : pivot réussi vers Rust + socle réseau crédible.
- Verrou actuel : **l'amorçage autonome**.
- Le bon terme = **pompe d'amorçage**.
- La bonne image = **secrétaire en chaise musicale**.
- La bonne preuve attendue = **seed handoff puis seed down sans effondrement**.

### Documents qu'il faut garder en tête

- `_bmad-output/planning-artifacts/product-brief-tom-protocol-2026-01-30.md`
- `_bmad-output/planning-artifacts/prd.md`
- `_bmad-output/planning-artifacts/architecture.md`
- `CLAUDE.md`
- `README.md`
- `docs/plans/2026-03-21-macro-roadmap-realignment.md`
- `docs/article-ToM protocol.md`
- `docs/plans/2026-03-30-bootstrap-pump-module.md`
- `docs/plans/2026-03-30-organic-seed-handoff-test.md`

---

## 11. Prochaine brique logique

La prochaine brique logique n'est pas de refaire un énième test avec saisie manuelle.

La prochaine brique logique est :

> **designer et implémenter le module `pompe d'amorçage`**

avec un premier backend vraisemblable et utile :

- **amorçage LAN zero-conf**,
- puis handoff vers réseau plus large,
- puis retrait du porteur initial.

---

## Conclusion finale

Le projet n'a pas un problème de vision.

Le projet a un problème plus précis et plus sain :

> la vision est claire, mais la brique d'amorçage digne de cette vision n'est pas encore une entité explicite et implémentée.

C'est donc cela que le pilotage doit assumer comme priorité numéro un.
