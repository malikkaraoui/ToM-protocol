# 2026-03-30 — Protocole de mesure `PeerPresent k`

## But

Mesurer l'impact réel de `peer_present_k` sur la convergence bootstrap relay-assisted.

Valeurs à comparer :

- `k = 8` (baseline actuelle)
- `k = 16`
- `k = 32`

## Précondition technique

Le relay accepte maintenant :

```toml
peer_present_k = 16
```

dans la config `tom-relay`.

Le harness local supporte désormais aussi un mode **relay-only** côté transport
(`TomNodeConfig::relay_only(true)`), utile pour éviter que les tentatives de
chemins directs ne polluent les mesures mono-machine.

## Ce qu'on veut observer

### Métriques principales

1. **temps jusqu'au premier `GossipNeighborUp`**
2. **temps jusqu'au premier message livré**
3. **taux de succès bootstrap sans seed manuel**
4. **volume de hints `PeerPresent` émis** (via logs relay)

### Métriques secondaires

1. charge CPU relay
2. pression sur queues `peer_present` (drops `channel full`)
3. stabilité sur arrivée en rafale de plusieurs nodes

## Scénarios minimaux

### Scénario A — LAN local simple

- 3 nodes sur le même LAN
- même relay
- `n0_discovery = false`
- `local_discovery = false` si on veut isoler uniquement `PeerPresent`

But : mesurer la convergence relay-assisted pure.

### Scénario B — LAN mixte avec mDNS actif

- 3 nodes sur le même LAN
- même relay
- `n0_discovery = false`
- `local_discovery = true`

But : vérifier si augmenter `k` apporte encore quelque chose quand le LAN-first existe déjà.

### Scénario C — WAN / seed handoff réel

- Mac + NAS + tvOS (si dispo)
- relay NAS ou relay dev dédié
- pas de bootstrap manuel après démarrage initial du scénario

But : voir si `k > 8` améliore la convergence réelle ou ajoute juste du bruit.

## Méthode recommandée

Pour chaque valeur de `k` :

1. lancer **au moins 10 runs** du même scénario
2. noter :
   - succès / échec bootstrap
   - temps jusqu'au premier voisin gossip
   - temps jusqu'au premier message utile
3. comparer médiane + pire cas

## Décision attendue

### Garder `k = 8` si

- succès déjà proche de 100%
- gains marginaux avec `16`/`32`
- coût relay/hints augmente visiblement

### Passer à `k = 16` si

- amélioration nette du pire cas
- baisse visible des échecs bootstrap
- pas de saturation observable sur les queues

### Passer à `k = 32` seulement si

- `16` améliore encore sensiblement mais reste insuffisant
- la topologie réelle justifie ce fanout

## Résultat préliminaire — 2026-03-31 (mono-Mac, relay-only, 40 nodes, 1 run / valeur)

Commande utilisée : harness `peer_present_k_matrix` avec

- `PEER_PRESENT_KS=8,16,32`
- `PEER_PRESENT_NODE_COUNT=40`
- `PEER_PRESENT_TRIALS=1`
- `PEER_PRESENT_BOOTSTRAP_TIMEOUT_SECS=15`
- `PEER_PRESENT_DELIVERY_TIMEOUT_SECS=5`

Résumé observé :

| k | succès bootstrap | médiane 1er NeighborUp | pire 1er NeighborUp | livraison |
|---|---:|---:|---:|---:|
| 8  | 40/40 | 3374.0 ms | 3778.5 ms | 1167.4 ms |
| 16 | 40/40 | 3671.1 ms | 4268.1 ms | 1460.0 ms |
| 32 | 40/40 | 3777.5 ms | 4978.0 ms | 3310.1 ms |

Lecture rapide : sur ce premier run local relay-only, `k = 8` reste le meilleur
point de fonctionnement parmi `8/16/32`, avec convergence plus rapide et une
latence de livraison plus basse. À confirmer avec plusieurs runs et sur une
topologie multi-machine, mais `32` n'apporte clairement aucun gain ici.

## Résultat consolidé — 2026-03-31 (mono-Mac, relay-only, 40 nodes, 10 runs / valeur)

Commande utilisée : harness `peer_present_k_matrix` avec

- `PEER_PRESENT_KS=8,16,32`
- `PEER_PRESENT_NODE_COUNT=40`
- `PEER_PRESENT_TRIALS=10`
- `PEER_PRESENT_BOOTSTRAP_TIMEOUT_SECS=15`
- `PEER_PRESENT_DELIVERY_TIMEOUT_SECS=5`
- `PEER_PRESENT_TRIAL_TIMEOUT_SECS=45`

Résumé observé :

| k | trials ok | succès bootstrap | médiane 1er NeighborUp | pire 1er NeighborUp | médiane livraison |
|---|---:|---:|---:|---:|---:|
| 8  | 10/10 | 100% | 3340.7 ms | 3936.7 ms | 803.7 ms |
| 16 | 10/10 | 100% | 3606.2 ms | 4428.7 ms | 1447.0 ms |
| 32 | 10/10 | 100% | 3817.3 ms | 4958.1 ms | 2572.0 ms |

Conclusion locale nette : sur cette topologie mono-machine relay-only, augmenter
`peer_present_k` au-delà de `8` dégrade à la fois la convergence gossip et la
latence de première livraison, sans gain de taux de succès puisque `8/16/32`
atteignent déjà tous `100%` de bootstrap.

## `recently_seen`

Ne l'introduire **qu'après** les mesures ci-dessus.

Ordre recommandé :

1. mesurer `k`
2. choisir la meilleure valeur simple
3. seulement ensuite évaluer `recently_seen` si les échecs restants viennent clairement d'un mauvais échantillonnage

## Verdict de méthode

`peer_present_k` est maintenant un bon **knob expérimental**.

Le prochain travail n'est plus d'architecture mais de **mesure terrain**.