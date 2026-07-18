# Reset de cache in-app — « sortir du laboratoire »

> Statut : **PROPOSITION — en attente de validation avant code.** Chantier app + outillage
> (pas de protocole LOCKED touché). Priorité arbitrée (Malik 18/07) : **après P0-1**, avant
> le re-dial M1. Migration d'identité Caches → Application Support = **chantier séparé, juste
> après** (ne PAS mélanger ici).

## 1. Intention (mot de Malik)

> « Vider le cache de l'app pour certains tests et campagnes. Le but est de tendre à sortir
> du laboratoire progressivement, et d'avoir de vrais effets de la vraie vie… vouloir entrer
> un nouveau contact et lui écrire sans s'être déjà écrit. »

Le scénario cible n'est PAS « nettoyer un disque » : c'est **reproduire la première prise de
contact**. Deux nœuds qui ne partagent aucun état antérieur, l'un saisit le `node_id` de
l'autre et lui écrit. Aujourd'hui nos nœuds se « connaissent » toujours (state.db accumulé,
BG-cache, noms) — on teste en permanence le régime établi, jamais le cold-start réel.

**Exigence d'autonomie (Malik, 18/07) — NON NÉGOCIABLE** : le reset doit être **pilotable par
API sur CHAQUE nœud, y compris les apps** (pas seulement le NAS), pour que l'agent puisse
réinitialiser toute la flotte dans ses boucles de test/campagne **sans toucher un écran**.
Un bouton UI seul ne suffit pas : il faut la commande scriptable partout, avec un traqueur
qui distingue un reset humain d'un reset API.

## 2. Deux niveaux (validé : les deux)

| Niveau | Efface | GARDE | Vu des autres nœuds |
|---|---|---|---|
| **Oublier le réseau** | topologie + groupes + backup + BG-cache + noms connus | **identité** + compteurs cumulés | le MÊME nœud, amnésique — « je ne connais plus personne, mais je reste moi » |
| **Réinitialisation usine** | tout ce qui précède **+ identité + compteurs** | rien | un **nouveau nœud** (node_id neuf) — le vrai « nouveau contact » jamais vu |

- **Oublier le réseau** → teste la re-découverte à froid en gardant l'observabilité
  (compteurs continus entre campagnes) et l'adresse réseau stable.
- **Usine** → teste « un inconnu rejoint le réseau » : c'est le node_id neuf que l'autre doit
  entrer à la main pour la toute première fois.

## 3. Ce qui est effacé — inventaire vérifié (sur pièces)

**État réseau (les deux niveaux)** :
- `state.db` — topologie persistée (`storage/mod.rs:842`).
- `hub_history.db`, `member_seq.db` — groupes (`storage/mod.rs:991,1040`).
- base de backup (messages ADR-009).
- UserDefaults Swift : `cachedPeerKey` (BG-cache de re-seed, `TomNodeService.swift:676/977`),
  `knownNamesKey` (map id→nom, `:312-329`).

**Identité + compteurs (usine seulement)** :
- `tom_identity.key` (`TomNodeService.swift:1294-1297`, ⚠️ dans `Caches/` — d'où le chantier
  migration séparé qui suit).
- compteurs cumulés UserDefaults (`:333-343`).

Le `data_dir` Rust entier vit sous `Caches/tom_data` (`:1299-1302`) : « réseau » supprime son
CONTENU (les .db) en gardant le dossier ; « usine » supprime aussi la clé d'identité voisine.

## 4. Invariant de sûreté — reset uniquement nœud ARRÊTÉ

Les .db SQLite sont ouvertes tant que le runtime tourne, et le teardown FFI est détaché
(ADR-010). **Effacer sous un nœud vivant = corruption / course avec le teardown.**

Règle : la commande de reset n'est acceptée que si `state == .stopped`. Si le nœud tourne :
1. `stop()` (arrêt propre, attendre l'état `.stopped`), 2. puis effacer, 3. le prochain
`start()` repart sur un état vierge. Côté NAS (process unique + systemd) : `shutdown` →
effacer → `exit(0)` → systemd `Restart=always` relance vierge (usine ⇒ nouveau node_id).

## 5. Surfaces (validé : apps + control NAS) — **API partout, bouton en plus**

Chaque nœud expose la MÊME commande logique par deux entrées : un bouton UI (humain) et une
route HTTP (agent autonome). Les deux convergent vers une seule fonction `resetNode(level:,
source:)` — `source` ∈ {`bouton`, `api`} n'affecte QUE le traqueur, jamais l'effet.

### Apps (iPhone/iPad/ATV/Mac) — bouton Settings **+ route :9091**
- **Bouton** : deux entrées dans `SettingsView`, **actives uniquement quand
  `state ∈ {stopped, error}`** (grisées sinon), chacune derrière une confirmation (usine =
  destructif, change l'identité). Traqueur : `appendLog(.warning, "👆 BOUTON Oublier-réseau
  pressé (utilisateur)")` / `"👆 BOUTON Reset-usine pressé (utilisateur)"`.
- **API (l'autonomie)** : nouvelle route `/reset?level=network|factory` sur le `StatusServer`
  :9091, qui est DÉJÀ un routeur `(méthode, path, query) → JSON` conçu pour piloter le nœud à
  distance (`StatusServer.swift:5-11`, il crée/envoie/accepte déjà). La route **arrête le
  runtime d'abord** (le nœud tourne forcément quand :9091 répond), puis efface, puis répond
  JSON `{ok, level, new_node_id?}`. Traqueur distinct : `"🤖 API reset (\(level)) — source=api"`.
- ⚠️ Le traqueur n'atteint le collecteur UDP que si le nœud **redémarre** ensuite (export
  coupé à l'arrêt) : l'émettre AVANT l'effacement, tant que l'ancien nœud loggue encore ;
  sinon il reste on-device (`:9091`/os.log en juge).

### NAS — control :9300 (déjà scriptable)
- Nouvelle route `"/reset"` dans le serveur control (`tom-tui/src/main.rs`, à côté de
  `/stop:494`), paramètre `level=network|factory`. Séquence : `handle.save_now()` (cohérence)
  → `shutdown()` → effacer les fichiers du `data_dir` (+ clé si `factory`) → réponse JSON →
  `exit(0)` → systemd relance vierge. **Défaut sûr** : `level` absent ⇒ `network`.

### Surface d'attaque — gating `#if DEBUG` OBLIGATOIRE dès le code (durci par review-oracle)
`:9091` est **broadcast sur le LAN** : une commande destructrice y est un DoS destructif (tout
appareil du LAN peut wiper un nœud, et l'usine change son node_id).

**Le pattern de défense existe DÉJÀ et `/reset` DOIT s'y conformer sans exception** : les
routes write actuelles (`/group/create|send|accept`) sont gées `#if DEBUG` — release répond
`"contrôle write désactivé en release"` (`TomNodeService.swift:1744-1752`, commentaire « cf
review sécurité »). **Donc `/reset` s'ajoute à CE `case` et passe par `handleControlWrite` →
DEBUG-only par construction, release inerte, dès la PREMIÈRE ligne de code.** Ce n'est pas
« à barrer avant hors-labo » (formulation initiale trop molle, rejetée à la review) : c'est
gaté d'emblée, comme ses voisines.

**Tension autonomie ↔ release (à assumer explicitement)** : mon autonomie de campagne repose
sur des builds **DEBUG** (où `/reset` répond) — c'est le régime labo actuel et il suffit.
Le jour où l'on distribue des builds **release**, `/reset` sera inerte : rétablir l'autonomie
hors-DEBUG demandera alors un **token partagé** dans la query (chantier de durcissement
distinct, à l'échéance « vraie distribution »). Cohérent avec « sortir du labo
PROGRESSIVEMENT » : DEBUG tant qu'on teste, token quand on livre.

## 6. Recette — le scénario « première prise de contact » EST le test

1. **Unit/app** (dont exigences durcies par la review) :
   - reset **refusé** sur tout état ≠ `.stopped` (`.running`/`.starting`/`.error`) — sinon
     .db SQLite ouverte corrompue (finding sécurité #4) ;
   - « réseau » garde `tom_identity.key` + compteurs, efface les .db + les deux clés
     UserDefaults ; « usine » produit un `node_id` **différent** au redémarrage (assertion) ;
   - route `/reset` **absente en release** (assertion : release répond « désactivé »), présente
     en DEBUG — même gate que `/group/*`.
   - **convergence post-usine** (finding sécurité #3) : un nœud reset usine puis relancé
     (node_id neuf) re-mesh la flotte en < 60 s ; les caches de l'ancien node_id chez les
     voisins pourrissent sans bloquer (croiser avec le chantier anti-ravivage : l'ancien
     node_id est un fantôme, évincé au TTL 24 h).
2. **Recette terrain (le cœur)** — deux appareils, AUCUN état partagé :
   - A et B font « Oublier le réseau » (ou usine pour du node_id neuf),
   - A saisit le `node_id` de B dans le champ existant `MessagesView.swift:182`
     (« Peer Node ID (hex) ») et écrit **sans qu'ils se soient jamais écrit**,
   - **Succès attendu** : livraison via re-découverte seule (rendezvous ADR-010 / relay),
     ACK reçu — cible de délai à mesurer au canari, PAS à inventer ici.
   - Métrique : temps saisie-node_id → premier ACK, et par quel chemin (DIRECT vs RELAY).
3. Croiser collecteur `:9999` + `:9091` (un « 0 pair » local peut mentir — leçon 17/07).

## 7. Non-buts / limites assumées

- **Pas de migration d'identité ici** : elle reste en `Caches/` pour ce chantier ; la
  migration Caches → Application Support est le chantier séparé qui suit (une identité ne doit
  pas disparaître sous pression disque avant de vrais utilisateurs — mais ça mérite son canari).
- **Usine sur le NAS change son node_id** : les autres nœuds devront ré-apprendre l'adresse du
  NAS (re-seed bootstrap / rendezvous). Assumé — c'est justement le scénario « nouveau nœud ».
- Pas de reset partiel « groupes seulement » / « backup seulement » : deux niveaux nets,
  pas un panneau de cases à cocher (invisibilité #6 : outil de test, pas une fonction produit).

## 8. Questions ouvertes

1. Confirmation UI : simple `confirmationDialog`, ou double-tap (ATV télécommande — pas de
   clavier facile pour un dialogue) ?
2. Le reset « réseau » doit-il aussi purger le **tracker de messages** (statuts en cours) ou
   seulement l'état réseau ? (proposition : le purger — un statut « en cours » vers un pair
   oublié n'a plus de sens.)
