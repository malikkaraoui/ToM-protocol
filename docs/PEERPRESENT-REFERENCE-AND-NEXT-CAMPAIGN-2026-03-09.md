# PeerPresent — référence de validation et prochaine campagne terrain

Date : 2026-03-09  
Auteur : GitHub Copilot  
Objectif : conserver une trace claire de l’état actuel de `PeerPresent`, de la méthode de validation retenue, et de la prochaine campagne terrain à exécuter.  
Usage : document de référence pour humain + Claude Code.

---

## 1. Résumé exécutif

La feature `PeerPresent` est désormais validée sur la couche code par des tests ciblés, mais la prochaine preuve utile n’est plus un nouveau test unitaire : c’est une campagne terrain simple et contrôlée entre **MacBook Pro** et **NAS Freebox**.

La stratégie retenue est volontairement stricte :

1. **ne pas mélanger** tests unitaires, tests d’intégration et essais physiques ;
2. valider d’abord les **invariants locaux** ;
3. ensuite valider la **chaîne produit réelle** ;
4. puis seulement élargir à **Apple TV**, **MacBook Air** et au **churn**.

---

## 2. Ce qui est déjà validé

### 2.1 Tests écrits et passants

Les tests suivants ont été ajoutés et passent :

| # | Crate | Test | Ce que le test verrouille |
|---|-------|------|---------------------------|
| 2 | `tom-relay` | `register_first_client_no_peer_present` | Premier client seul : aucun hint `PeerPresent` émis |
| 2 | `tom-relay` | `register_broadcasts_peer_present` | Deux pairs : hints croisés, pas d’auto-référence |
| 3 | `tom-connect` | `test_peer_present_received_on_channel` | Un frame `PeerPresent` reçu du relay remonte bien sur le channel `peer_present_rx` |
| 4 | `tom-transport` | `add_peer_addr_feeds_memory_lookup_and_pool` | `add_peer_addr()` nourrit bien `MemoryLookup` puis le `Pool` |

### 2.2 Ce qui était déjà couvert avant

- `PeerPresent` côté proto `tom-relay` était déjà partiellement couvert par les tests de sérialisation / roundtrip existants.
- Le test d’intégration `peer_present_auto_discovery_leads_to_neighbor_up_and_delivery` existe déjà dans `crates/tom-integration-tests/tests/peer_present_auto_discovery.rs` et reste volontairement `#[ignore]` pour éviter le bruit dans les runs workspace généraux.

### 2.3 Validation globale déjà faite

Les validations suivantes ont été rapportées comme vertes :

- `cargo clippy --workspace -- -D warnings` ✅
- `cargo test --workspace` ✅ sur les crates standards concernées

Note importante :

- les blocages observés sur certains tests `multi_node` sont **pré-existants** et **non attribués à PeerPresent** ;
- ils ne doivent pas être utilisés comme signal contre cette feature tant qu’aucun lien causal n’est démontré.

---

## 3. Ce que PeerPresent est censé prouver côté produit

La valeur produit de `PeerPresent` n’est pas seulement “le frame existe”.

La vraie promesse est :

> deux nœuds connectés au même relay peuvent se découvrir automatiquement sans bootstrap manuel explicite.

La chaîne utile à valider est :

$$
Relay \rightarrow PeerPresent \rightarrow add\_peer\_addr \rightarrow gossip\ join \rightarrow NeighborUp \rightarrow delivery
$$

Tant que cette chaîne n’est pas observée au moins une fois en conditions réelles contrôlées, la validation n’est pas terminée au sens produit.

---

## 4. Méthode retenue : ne plus mélanger les couches de validation

Le problème rencontré précédemment venait d’un mélange entre trois types de preuve.

### 4.1 Couche A — tests de code

But :

- prouver les invariants locaux ;
- attraper les erreurs de plomberie ;
- exécuter vite et souvent.

Exemples :

- broadcast de `PeerPresent` dans le relay ;
- remontée de `PeerPresent` vers le channel ;
- injection correcte dans `MemoryLookup`.

### 4.2 Couche B — test d’intégration ciblé

But :

- prouver une chaîne complète sur un setup minimal ;
- éviter qu’un test réseau sensible pollue les runs généraux.

Exemple :

- `peer_present_auto_discovery_leads_to_neighbor_up_and_delivery`.

### 4.3 Couche C — campagne terrain réelle

But :

- observer le comportement réel avec vraie machine, vrai relay, vraie latence et vrais redémarrages ;
- valider la valeur produit, pas seulement la cohérence interne.

Conclusion méthodologique :

> un test de code ne remplace pas un essai terrain, et un essai terrain ne doit pas servir à déboguer des invariants unitaires qui auraient dû être verrouillés avant.

---

## 5. Pourquoi la prochaine campagne doit être MacBook Pro ↔ NAS Freebox

Le meilleur prochain setup est :

- **MacBook Pro** = machine de dev / orchestrateur / visibilité logs
- **NAS Freebox** = relay réel / cible ARM64 / environnement infra proche du vrai usage

Ce duo est prioritaire parce qu’il réduit le nombre de variables :

- pas encore d’Apple TV ;
- pas encore de MacBook Air ;
- pas encore de topologie multi-participants ;
- pas encore de churn complexe.

Autrement dit :

> si `PeerPresent` ne passe pas proprement sur MacBook Pro ↔ NAS Freebox, cela ne sert à rien d’ajouter tout de suite Apple TV ou un quatrième protagoniste.

---

## 6. Prochaine campagne recommandée

### 6.1 Nom de campagne

**Campagne P1 — PeerPresent terrain minimal (MacBook Pro ↔ NAS Freebox)**

### 6.2 Objectif

Valider en conditions réelles contrôlées :

1. un relay NAS accessible ;
2. deux nœuds utilisant ce même relay ;
3. aucune injection manuelle de peer comme bootstrap principal ;
4. apparition d’un `NeighborUp` ;
5. livraison d’un premier message ;
6. reprise après restart relay.

---

## 7. Ordre exact d’exécution recommandé

### Étape 1 — sanity infra NAS

Valider d’abord l’environnement, pas le protocole.

À vérifier :

- NAS accessible en SSH ;
- relay démarrable ;
- endpoint health OK ;
- metrics exposées si activées ;
- MacBook Pro atteint bien le NAS.

**But** : éliminer les faux positifs de type “c’est cassé” alors que c’est juste l’infra Freebox.

### Étape 2 — scénario PeerPresent minimal

Setup :

- 2 nœuds ;
- même relay NAS ;
- zéro bootstrap manuel ;
- attente explicite d’un voisinage ou d’un signe équivalent (`NeighborUp`, event runtime, message réussi juste après découverte).

**Critère principal** : premier échange applicatif livré sans bootstrap manuel explicite.

### Étape 3 — restart relay

Après succès du scénario minimal :

- redémarrer le relay NAS ;
- observer la reconnexion ;
- vérifier que le service repart sans intervention lourde.

**Critère principal** : la reprise fonctionne et la feature n’est pas “one-shot”.

### Étape 4 — endurance courte

Lancer une session courte mais réelle :

- 10 à 20 minutes ;
- message/ping régulier ;
- pas nécessairement une campagne d’1h au premier run.

**But** : vérifier qu’il n’y a pas de dérive évidente ou de perte progressive rapide.

---

## 8. Ce qu’il ne faut pas faire tout de suite

Tant que la campagne P1 n’est pas stable, ne pas ouvrir simultanément :

- Apple TV ;
- MacBook Air ;
- scénarios multi-relay complexes ;
- churn agressif (disconnect/reconnect multiples) ;
- endurance longue 1h+ ;
- analyse de bugs non déterministes sur 4 machines à la fois.

Règle simple :

> une seule variable nouvelle à la fois.

---

## 9. Critères de succès de la campagne P1

### Succès minimum

- relay NAS démarre ;
- health OK ;
- deux nœuds utilisent le même relay ;
- pas de bootstrap manuel explicite ;
- `NeighborUp` observé ou équivalent fonctionnel probant ;
- message livré.

### Succès renforcé

- restart relay sans casse durable ;
- reconnexion automatique ;
- mini endurance OK.

### Succès suffisant pour passer à l’étape suivante

Si plusieurs runs consécutifs sont propres sur MacBook Pro ↔ NAS Freebox, alors seulement ouvrir :

1. **Apple TV** ;
2. puis **MacBook Air** ;
3. puis scénarios de churn ;
4. puis topologies plus riches.

---

## 10. Risques et pièges connus

### 10.1 Ne pas attribuer trop vite un bug à PeerPresent

Des problèmes réseau Freebox/NAS existent déjà dans l’historique du repo, notamment autour du bridge / ARP / reboot Freebox.

Donc après toute perturbation réseau ou reboot :

- revérifier SSH ;
- revérifier reachability IP ;
- revérifier health relay ;
- seulement ensuite juger PeerPresent.

### 10.2 Ne pas utiliser les tests terrain pour déboguer des invariants unitaires

Si un invariant de code est suspect, il doit être fixé et verrouillé par test ciblé avant de retourner sur le terrain.

### 10.3 Ne pas considérer les tests `multi_node` bloqués comme preuve négative automatique

Ces blocages sont signalés comme pré-existants. Ils doivent être traités séparément, sans contaminer l’évaluation de PeerPresent tant qu’aucun lien direct n’est démontré.

---

## 11. Séquence recommandée après la campagne P1

Si P1 réussit, l’ordre recommandé est :

### Phase suivante A — Apple TV

Ajouter l’Apple TV comme nouveau participant, sans encore lancer des scénarios agressifs.

### Phase suivante B — MacBook Air

Ajouter ensuite le MacBook Air pour monter progressivement vers un setup 3 à 4 protagonistes.

### Phase suivante C — churn

Seulement après stabilité du setup étendu :

- déconnexion d’un protagoniste ;
- reconnexion ;
- arrivée tardive ;
- restart relay ;
- observation de la reprise.

---

## 12. Doctrine opérationnelle pour Claude Code

Si Claude Code reprend ce sujet, il doit respecter l’ordre suivant.

### 12.1 Avant tout nouveau patch

- vérifier si le problème vient d’une couche **code**, **intégration**, ou **terrain** ;
- ne pas patcher à l’aveugle après un run terrain flou.

### 12.2 Si le bug est local

- ajouter ou corriger un test ciblé ;
- valider crate touché + downstream ;
- exécuter `clippy` représentatif.

### 12.3 Si le bug est produit / terrain

- garder le code stable ;
- documenter précisément le scénario matériel ;
- réduire le setup au minimum ;
- rejouer la campagne la plus simple avant d’élargir.

### 12.4 Règle d’or

> ne jamais demander à une campagne terrain de jouer le rôle d’un test unitaire, et ne jamais déclarer la feature “prouvée” tant que la chaîne produit n’a pas été observée sur au moins une campagne réelle propre.

---

## 13. Références utiles du repo

- `docs/plans/2026-03-04-c1-1-mac-freebox-test-protocol.md`
- `docs/plans/2026-02-24-stress-campaign-design.md`
- `docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md`
- `docs/REVIEW-PEER-DISCOVERY-V3-2026-03-07.md`
- `docs/REVIEW-PEERPRESENT-IMPLEMENTATION-2026-03-07.md`
- `crates/tom-integration-tests/tests/peer_present_auto_discovery.rs`

---

## 14. Conclusion

L’état actuel est bon :

- les verrous de code utiles ont été ajoutés ;
- la validation locale est solide ;
- la prochaine preuve pertinente est une campagne terrain simple.

La suite recommandée n’est donc **pas** “plus de code tout de suite”, mais :

> **MacBook Pro ↔ NAS Freebox d’abord, Apple TV ensuite, MacBook Air après, churn seulement à la fin.**

Cette séquence minimise le bruit, maximise la lisibilité des résultats, et donne à Claude Code une référence claire pour éviter de repartir dans des boucles de patch/test mal cadrées.