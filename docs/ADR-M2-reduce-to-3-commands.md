# ADR-M2 : Réduire de 4 à 3 commandes

**Date** : 2026-03-30
**Statut** : PROPOSÉ
**Contexte** : Sprint 1, M2 = FAIL (4 commandes, cible ≤ 3)

## Problème

La démo nécessite 4 commandes :

```bash
T0  tom-relay --dev                          # relay bootstrap externe
T1  tom-chat --username pub --embedded-relay  # publisher
T2  tom-chat --username obs1                  # observer 1
T3  tom-chat --username obs2                  # observer 2
```

T0 existe uniquement parce que les 3 nodes ont besoin d'un point de rendez-vous initial pour former le gossip mesh. Sans T0, `n0_discovery` seul ne garantit pas la convergence.

## Pourquoi T0 est nécessaire aujourd'hui

1. **Gossip bootstrap** : les nodes rejoignent un topic gossip via `join_peers()`. Il faut au moins un peer connu pour amorcer.
2. **Relay comme rendez-vous** : le relay (`tom-relay --dev`) fait office de point de rendez-vous. Les nodes se connectent au relay, découvrent les autres via PeerPresent, puis forment le gossip mesh.
3. **n0_discovery (Pkarr/DNS)** : fonctionne mais avec une latence variable (5-30s) et pas de garantie sur un LAN isolé sans accès Internet.
4. **DHT (Mainline)** : même problème — dépend d'un réseau DHT externe.

## Options

| Option | Statut | Commentaire |
|--------|--------|-------------|
| 4 commandes | **Baseline actuelle** | Fonctionne, démontrée (S9 PASS) |
| A1 — relay embarqué, port fixe | **Bloquée** | Gossip/relay OK, PeerDiscovered KO |
| A2 — auto-discovery mDNS | Hors scope | Changement significatif |
| B — script wrapper | Rejetée | Masque la complexité, ne la réduit pas |
| C — n0_discovery seul | Rejetée | Convergence non garantie (<80%) |

### Option A : Relay embarqué dans le premier node (direction cible)

Le premier node (`tom-chat --username pub`) démarre un relay embarqué ET publie son URL. Les observers pointent vers le relay du publisher au lieu d'un relay externe.

**Hypothèse initiale** : A1 semblait faisable sans changement code (`--embedded-relay` existe déjà).
**Résultat du test** : faux — un fix technique est nécessaire pour rendre A1 opérationnelle (voir section "Résultat du test").

**Deux variantes :**

- **(A1-local)** Même machine. T1 bind sur port fixe (`--embedded-relay-bind 127.0.0.1:3340`) → T2/T3 utilisent `TOM_RELAY_URL=http://127.0.0.1:3340`.
- **(A1-cross-machine)** Machines différentes. T1 bind sur `0.0.0.0:3340` ou `[::]:3340` → T2/T3 utilisent `TOM_RELAY_URL=http://<IP_PUB>:3340`. Nécessite que l'IP du publisher soit connue ou annoncée.

### Option B : Script wrapper

Un `./demo.sh` qui lance les 4 process. Ne réduit pas la complexité réelle, la masque. M2 mesurait la complexité sous-jacente.

### Option C : Supprimer le relay bootstrap, forcer n0_discovery

Convergence non garantie (M3 = 90% avec relay, probablement <80% sans). Régression.

## Scénario A1-local testé (3 commandes)

```bash
# T1 — Publisher + relay intégré (port fixe)
TOM_RELAY_URL=http://127.0.0.1:3340 tom-chat \
  --username pub --bot --bot-ping 5 \
  --embedded-relay --embedded-relay-bind 127.0.0.1:3340 \
  --embedded-relay-publish --relay-ttl 60

# T2 — Observer 1
TOM_RELAY_URL=http://127.0.0.1:3340 tom-chat \
  --username obs1 --bot --relay-discovery --relay-ttl 60 <PUB_NODE_ID>

# T3 — Observer 2
TOM_RELAY_URL=http://127.0.0.1:3340 tom-chat \
  --username obs2 --bot --relay-discovery --relay-ttl 60 <PUB_NODE_ID>
```

## Résultat du test (2026-03-30)

### Scénario 4 commandes (baseline) : PASS

- Discovery : 3/3 nodes se trouvent en ~8s
- Messages : 352+ msgs, 0 erreur
- Path upgrade : relay → direct (0.85ms RTT)

### Scénario 3 commandes (Option A1-local) : FAIL partiel

- Gossip mesh : OK (neighbors up des 2 côtés)
- Relay discovery : OK (obs1/obs2 découvrent le relay de pub)
- PeerAnnounce : FAIL — aucun `PeerDiscovered` en 25s
- Messages : 0 (pas de target pour --bot-ping)

### Hypothèses de diagnostic

Le bug n'a pas été investigué en profondeur. Hypothèses principales, à confirmer :

1. Le publisher n'apparaît pas comme client de son propre relay embarqué (pas de PeerPresent frame du point de vue des observers)
2. Les PeerAnnounce gossip n'aboutissent pas à `PeerDiscovered` dans cette topologie spécifique
3. Il existe peut-être une dépendance implicite à PeerPresent pour finaliser la découverte au niveau transport

Le gossip fonctionne (bootstrap via node ID positionnel, neighbors up), mais la chaîne PeerAnnounce → PeerDiscovered ne se complète pas dans le délai testé.

## Décision

**Direction cible : A1** (relay embarqué dans le premier node, port fixe).

Mais A1 est **bloquée techniquement** à ce stade. Le scénario **4 commandes reste la baseline officielle** jusqu'à résolution du bug PeerAnnounce → PeerDiscovered.

### Compromis accepté

T2/T3 doivent toujours connaître le Node ID de T1 (argument positionnel). C'est inhérent au modèle P2P — il faut un point d'entrée. Ce n'est pas un problème, c'est le design.

### Prochaine étape

Investiguer pourquoi PeerAnnounce via gossip ne produit pas de PeerDiscovered dans le scénario 3 commandes. Chantier technique séparé, scope strict.
