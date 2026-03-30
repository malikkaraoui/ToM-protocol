# Test réel "organic / seed handoff" — 30 mars 2026

## Objectif

Relancer un test **plus naturel**, avec :

- **aucune IP relay personnelle hardcodée dans le code**
- **aucun bootstrap peer hardcodé dans le code**
- un amorçage éventuellement fourni **au runtime seulement**
- puis vérifier que **le flambeau est repris par le réseau**

Ce document formalise le test le plus honnête possible **avec le code actuel**.

---

## Vérité terrain actuelle

### Déjà acquis

- l'app tvOS démarre désormais avec `relayUrl = ""`
- l'app tvOS démarre désormais avec `bootstrapPeerId = ""`
- `tom-chat` n'a pas besoin de `--bootstrap`
- `tom-chat` n'a pas besoin de `TOM_RELAY_URL`

### Implicites encore présents dans la stack

Sans configuration explicite, le transport peut encore utiliser :

- la découverte `n0` / Pkarr / DNS
- le DNS fallback `_relay._tcp.tom-protocol.org`
- les relays publics par défaut dans `tom-transport`

Donc :

> ce test est **sans hardcoding scénario/appareil**, mais **pas encore sans aucune infra par défaut dans le code source transport**.

### Limite importante

Le projet **ne prouve pas encore** une apparition initiale totalement autonome depuis zéro connaissance applicative.

La DHT aide surtout à **résoudre un peer déjà identifié** ; elle ne constitue pas à elle seule une découverte magique de tous les peers inconnus.

Donc on distingue deux sous-tests :

1. **Mode strict organique** — aucun seed saisi, on observe ce qui se passe réellement
2. **Mode seed handoff** — un seed est injecté **au runtime seulement**, puis retiré

---

## Sous-test 1 — mode strict organique

### Règles

- pas de `TOM_RELAY_URL`
- pas de `TOM_BOOTSTRAP_PEER`
- pas de `--bootstrap`
- pas de relay URL saisi dans l'Apple TV
- pas de bootstrap peer saisi dans l'Apple TV
- `n0Discovery = ON`
- `DHT = ON`

### Apple TV

Settings attendus :

- Relay Mode : `Automatic discovery / public fallback`
- Bootstrap : `Organic discovery only`
- Relay URL field : vide
- Bootstrap Peer ID field : vide
- N0 Discovery : ON
- DHT : ON
- Auto-Echo : ON
- UDP log export : ON si on veut observer à distance

### Mac

Exécution :

- `tom-chat --username mac-organic --bot --bot-ping 5 --relay-discovery --relay-ttl 120`
- sans `--bootstrap`
- sans variable `TOM_RELAY_URL`

### Volontaire / NAS / autre seed déjà vivant

Optionnel dans ce sous-test.

Le but ici est simplement de mesurer :

> est-ce qu'un nœud lancé "vierge" rejoint spontanément quelque chose d'utile sans aucune info runtime ?

### Verdict attendu

- **PASS fort** : peers découverts + messages réels sans aucune injection runtime
- **PASS faible** : relays visibles / activité transport, mais pas de peer applicatif utile
- **FAIL honnête** : rien ne converge ; cela confirme qu'il manque encore une vraie stratégie d'apparition initiale applicative

---

## Sous-test 2 — mode "seed handoff" (recommandé)

C'est le test le plus réaliste **aujourd'hui**.

### Principe

- aucune valeur n'est hardcodée dans le code
- un seed initial existe quelque part dans le réseau
- ses coordonnées sont fournies **au runtime seulement**
- une fois le réseau convergé, on coupe ce seed
- on observe si le réseau continue à vivre

### Étape A — lever un seed minimal

Un volontaire ou un de tes devices démarre un seed avec identité stable.

Exemple `tom-chat` :

- `tom-chat --username seed-a --self-relay --relay-discovery --relay-ttl 120 --bot --bot-ping 5`
- sans `--bootstrap` si ce seed est vraiment le premier
- avec identité persistante si possible

But : obtenir un `Node ID` vivant et un relay publié dynamiquement.

### Étape B — rejoindre sans hardcoding source

#### Mac entrant

Démarrage :

- `tom-chat --username mac-join --bot --bot-ping 5 --relay-discovery --relay-ttl 120`
- sans `TOM_RELAY_URL`
- sans `--bootstrap`

Si aucune convergence utile n'apparaît spontanément, injecter **temporairement** le `Node ID` live du seed :

- via `--bootstrap <SEED_NODE_ID>`
- ou via `TOM_BOOTSTRAP_PEER=<SEED_NODE_ID>`

#### Apple TV entrante

Laisser par défaut :

- Relay URL vide
- Bootstrap Peer ID vide

Si aucune convergence utile n'apparaît spontanément, saisir **temporairement** dans l'UI :

- Relay URL : vide dans la première tentative
- Bootstrap Peer ID : `<SEED_NODE_ID>` live seulement si nécessaire

Important :

> cette saisie runtime n'est **pas** du hardcoding. C'est une amorce transitoire, injectée à la main pour le test.

### Étape C — propagation

Succès si on observe ensuite :

- `Peer discovered`
- `Gossip neighbor up`
- `Relay discovered`
- `Transport relay added`
- messages réels échangés
- l'Apple TV qui reçoit puis echo

### Étape D — retrait du seed initial

Quand Mac + Apple TV + au moins un autre volontaire ont convergé :

- couper `seed-a`
- ne toucher à rien d'autre
- observer 5 à 10 minutes

### Étape E — critères de PASS

Le test est **PASS** si après coupure du seed initial :

1. les peers restants continuent à se voir
2. les messages continuent à circuler
3. aucun des entrants n'a besoin qu'on re-saisisse le seed
4. un relay utile reste connu quelque part dans le réseau
5. un nœud redémarré retrouve un réseau vivant avec au plus une amorce runtime minimale

---

## Logs à capturer

### Apple TV

Chercher :

- `Relay mode: Automatic discovery / public fallback`
- `Bootstrap mode: Organic discovery only`
- `Bootstrap peers: none (organic discovery)`
- `PEER DISCOVERED:`
- `MSG from`
- `ECHO #`

### Mac / seed

Chercher :

- `Peer discovered:`
- `Gossip neighbor up:`
- `Relay discovered:`
- `Transport relay added:`
- `[bot] ping #`
- `[bot] replied:`

---

## Ce qu'on veut apprendre exactement

### Si le sous-test 1 passe

Très gros signal : le réseau commence à avoir une vraie capacité d'apparition sans couture applicative locale.

### Si le sous-test 1 échoue mais le sous-test 2 passe

C'est probablement l'état réel actuel :

- l'amorce initiale existe encore
- mais elle n'est plus hardcodée dans le code
- et surtout le réseau peut ensuite reprendre le flambeau

### Si le sous-test 2 échoue après retrait du seed

Le chantier prioritaire reste bien :

- persistance des peers appris
- persistance / circulation des relays appris
- stratégie d'apparition/rejoin plus autonome

---

## Conclusion opérationnelle

Le bon prochain run n'est pas un stress brut.

Le bon prochain run est :

1. **strict organique** (zéro seed saisi)
2. puis **seed handoff** (seed live au runtime, jamais hardcodé)
3. puis **seed down**
4. puis seulement le stress
