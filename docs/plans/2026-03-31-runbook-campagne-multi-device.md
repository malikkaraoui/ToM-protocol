# 2026-03-31 — Runbook campagne multi-device (NAS + Apple TV + MacBook)

## But

Valider en topologie réelle les travaux du checkpoint `copilot/bootstrap-pump-skeleton` :

- bootstrap **LAN-first** via `local_discovery`
- bootstrap **relay-assisted** via `PeerPresent`
- stabilité du réglage **`peer_present_k = 8`**
- bon comportement de la chaîne complète :
  - découverte
  - `GossipNeighborUp`
  - première livraison
  - continuité après redémarrage relay

## Ce qu’on considère déjà validé avant campagne

Déjà vert en local sur la branche poussée :

- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`
- test ciblé `PeerPresent -> NeighborUp -> delivery`
- benchmark relay-only local montrant que **`k=8` bat `16` et `32`** sur mono-machine

La campagne multi-device ne repart donc **pas de zéro** : elle sert à vérifier le comportement **terrain** sur machines réelles.

---

## Topologie recommandée pour cette campagne

### Machines et rôles

1. **NAS Freebox**
   - rôle principal : **relay + discovery**
   - ports attendus :
     - relay `:3340`
     - metrics `:9090`
     - discovery `:8080`

2. **MacBook**
   - rôle principal : **orchestrateur / client de test / observation logs**
   - peut aussi servir de second nœud utilisateur

3. **Apple TV / tvOS**
   - rôle principal : **nœud applicatif ToM via FFI**
   - objectif pratique de cette campagne : vérifier qu’il rejoint proprement le réseau et échange au moins un message utile

### Décision de campagne

Pour cette session terrain, la séquence la plus rentable est :

- **Phase A** : NAS + MacBook
- **Phase B** : NAS + MacBook + Apple TV
- **Phase C** : restart relay NAS avec clients déjà actifs

On **ne refait pas** une matrice `k=8/16/32` complète tant que la campagne `k=8` n’est pas propre sur les 3 devices.

---

## Réglages à figer pour éviter le bruit

### Relay

- garder **`peer_present_k = 8`**
- utiliser de préférence le preset :
  - `deploy/peerpresent-k/tom-relay-k8.toml`

### Nodes

Pour la campagne multi-device, utiliser :

- `n0_discovery(false)`
- `local_discovery(true)` quand possible sur Mac / tvOS / autres nœuds locaux
- **pas** de `relay_only(true)` pour le terrain normal

Pourquoi :

- `relay_only(true)` a servi à **isoler la mesure locale** ;
- en campagne réelle, on veut observer le comportement normal du système, pas une cage d’essai.

---

## Préflight avant lancement

### 1. Sur MacBook

Vérifier :

- branche : `copilot/bootstrap-pump-skeleton`
- repo propre ou au moins checkpoint connu
- tests déjà verts localement

Contrôle minimum :

- `git branch --show-current`
- `git status --short`

### 2. Sur NAS

Vérifier :

- SSH OK
- relay OK
- discovery OK
- endpoints santé OK

Endpoints attendus :

- `http://<NAS_IP>:3340/health`
- `http://<NAS_IP>:3340/healthz`
- `http://<NAS_IP>:3340/ready`
- `http://<NAS_IP>:9090/metrics`
- `http://<NAS_IP>:8080/health`
- `http://<NAS_IP>:8080/relays`
- `http://<NAS_IP>:8080/status`

### 3. Sur Apple TV / tvOS

Préflight recommandé :

- `scripts/apple-tv-preflight.sh`
- dans `apps/tom-node-tvos/` :
  - `make doctor`
  - `make ffi` pour simulateur si test simulé
  - `make ffi-device` si test device physique

Objectif minimal côté Apple TV :

- l’app démarre
- le runtime démarre
- les logs montrent l’état réseau
- `local_discovery` est bien transmis au runtime

---

## Campagne d’exécution

## Phase A — NAS + MacBook

### Objectif

Valider le socle terrain sur 2 machines avant d’ajouter tvOS.

### Étapes

1. **Démarrer / vérifier le relay NAS**
   - config `k=8`
   - santé OK

2. **Vérifier discovery NAS**
   - `/relays` doit contenir le relay NAS attendu

3. **Lancer un nœud MacBook**
   - `n0_discovery(false)`
   - `local_discovery(true)`

4. **Exécuter un scénario simple Mac ↔ NAS**
   - soit via `tom-stress responder` côté NAS + `campaign` côté Mac
   - soit via TUI / runtime minimal si plus simple pour observer les événements

### Critères de succès

- le Mac voit un bootstrap utile
- au moins un `GossipNeighborUp`
- un message 1-to-1 est livré
- pas de crash relay
- pas de blocage bootstrap

### Logs à capturer

- logs relay NAS
- logs discovery NAS
- logs Mac runtime montrant :
  - `bootstrap: accepted peer hint`
  - source du hint (`mDNS` ou `PeerPresent`)
  - `NeighborUp`
  - livraison utile

---

## Phase B — NAS + MacBook + Apple TV

### Objectif

Valider l’intégration du troisième nœud contraint, celui qui compte le plus produit.

### Étapes

1. **Laisser le relay NAS actif**
2. **Laisser le nœud MacBook actif**
3. **Démarrer l’app tvOS**
4. Sur Apple TV, vérifier dans les logs :
   - runtime créé
   - runtime started
   - `Local discovery: enabled` si affiché par l’app
5. Attendre la convergence bootstrap
6. Tester un échange :
   - MacBook → Apple TV
   - Apple TV → MacBook

### Critères de succès

- Apple TV rejoint le réseau sans saisie manuelle lourde
- Apple TV reçoit au moins un peer exploitable
- Apple TV reçoit un message depuis MacBook
- Apple TV peut répondre
- pas de freeze UI ou crash app

### Ce qu’on veut absolument savoir

- est-ce que l’Apple TV converge plutôt via **mDNS** ou via **PeerPresent** ?
- combien de temps jusqu’au premier voisin utile ?
- y a-t-il une différence visible entre simulateur et device physique ?

---

## Phase C — Continuité après restart relay NAS

### Objectif

Vérifier que le socle reste robuste quand le relay principal redémarre.

### Étapes

1. Laisser MacBook et Apple TV actifs
2. Redémarrer le relay NAS
3. Observer :
   - perte temporaire éventuelle
   - reconnexion
   - reprise de la messagerie

### Critères de succès

- le redémarrage ne laisse pas les clients coincés durablement
- le réseau reprend sans reconfiguration manuelle lourde
- un échange utile redevient possible après reprise

---

## Mesures à relever pendant la campagne

Pour chaque phase, noter :

- heure de début / fin
- machines impliquées
- build/commit utilisé
- relay URL utilisée
- discovery URL utilisée
- premier `NeighborUp` observé
- temps approximatif jusqu’à première livraison
- succès / échec
- logs d’erreur éventuels

## Tableau minimal à remplir

| Phase | Machines | Bootstrap observé | NeighborUp | 1er message | Restart OK | Notes |
|---|---|---|---|---|---|---|
| A | NAS + Mac |  |  |  | n/a |  |
| B | NAS + Mac + tvOS |  |  |  | n/a |  |
| C | NAS + Mac + tvOS |  |  |  |  |  |

---

## Ordre de diagnostic si ça casse

### Si MacBook ne converge pas

1. vérifier relay NAS santé
2. vérifier discovery NAS santé
3. vérifier que le nœud Mac est bien configuré avec `n0_discovery(false)`
4. relancer le test ciblé `PeerPresent` local pour séparer bug terrain / bug code

### Si Apple TV ne converge pas

1. vérifier que l’app démarre vraiment le runtime
2. vérifier que `local_discovery` est bien propagé
3. vérifier que le relay NAS est atteignable depuis le device
4. comparer simulateur vs device
5. regarder si le problème est bootstrap, UI, ou FFI

### Si tout marche à 2 nœuds mais casse à 3

Hypothèses prioritaires :

- problème de bootstrap troisième nœud
- timing / ordering des hints
- spécificité tvOS / FFI / réseau local
- relay restart ou churn mal absorbé

---

## Verdict attendu de cette campagne

À la fin, on doit pouvoir trancher entre 3 cas :

### Cas 1 — GO

- NAS + MacBook OK
- Apple TV rejoint et échange
- restart relay NAS absorbé

=> on peut envisager la PR / fusion GitHub après cette validation terrain

### Cas 2 — GO partiel

- NAS + MacBook OK
- Apple TV démarre mais reste fragile

=> ne pas merger tout de suite ; ouvrir un sous-chantier tvOS ciblé

### Cas 3 — NO GO

- le comportement terrain contredit la validation locale

=> garder la branche de travail, corriger avant toute fusion

---

## Recommandation opérationnelle

Pour la prochaine session, enchaîner exactement comme suit :

1. **NAS relay/discovery sanity**
2. **MacBook ↔ NAS**
3. **ajout Apple TV**
4. **échange bidirectionnel MacBook ↔ Apple TV**
5. **restart NAS relay**
6. **compte-rendu court dans un doc de résultats**

C’est le plus court chemin vers une vraie décision produit :
**est-ce que le bootstrap pump tient sur des machines réelles, oui ou non ?**
