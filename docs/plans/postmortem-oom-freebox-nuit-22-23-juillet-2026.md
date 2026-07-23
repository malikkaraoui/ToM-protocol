# Postmortem — surchauffe Freebox 22/07 soir + validation nocturne 22→23/07

> Statut : incident clos, fix validé en conditions réelles sur 2 cycles complets.
> Portée : NAS Freebox (`tom-node.service`), corrélée au fix `ConnectionInner` livré la veille.

## 1. Ce qui s'est passé (le constat direct)

Vers 21h00 le 22/07, Malik entend les ventilateurs de la Freebox tourner à plein régime et va vérifier le panneau `mafreebox.freebox.fr` → Courbes de température. Le graphique montre un pic net : les CPU (Master/Slave) grimpent à **100°C** entre 21h00 et 21h05, les ventilateurs suivent de ~2000 à ~4500-5000 RPM.

Ce que le graphique seul ne dit pas, et que le diagnostic réseau a révélé : le pic thermique n'est que la moitié de l'histoire.

| Heure (CEST) | Événement |
|---|---|
| ~12h04 | La boucle de monitoring locale (CSV, suivi RSS du process) meurt en silence — personne ne le sait encore |
| ~21h00–21h05 | Pic CPU 100°C sur la courbe Freebox (cause probable : voir §2) |
| ~21h39 | SSH vers le NAS : le handshake TCP aboutit mais la bannière SSH n'arrive jamais (`timed out during banner exchange`) — le process est déjà en détresse |
| ~21h53:54 | Le journal système du NAS s'arrête net (dernière ligne applicative). Coïncide avec la 2ᵉ capture d'écran de Malik (21h53:58) |
| ~21h56 | Ping/ARP échouent aussi (`Host is down`) — mort complète, même la couche réseau la plus basse ne répond plus |
| ~22h05–22h16 | Malik éteint la VM manuellement pour la laisser refroidir, puis la rallume |
| 22h16:43 | Redémarrage propre, début de la surveillance nocturne bornée |

**Le point clé** : un pic thermique qui redescend tout seul (le CPU était déjà en train de refroidir avant la mort complète) n'explique pas à lui seul l'arrêt du réseau. Ce n'est donc pas un simple coup de chaud — quelque chose a rendu la machine incapable de répondre, et la chaleur en est la conséquence visible, pas la cause isolée.

## 2. Pourquoi (l'enquête dans les logs)

Le kernel log (`journalctl -k`) du boot précédent était **vide** — la remontée des messages noyau vers journald semble cassée sur cette VM (gap découvert cette nuit, à corriger séparément). Impossible donc de confirmer ou d'exclure un OOM-killer par cette voie.

Mais le log applicatif de `tom-node` a survécu, et les 70 dernières secondes avant la mort (21h52:47 → 21h53:54) montrent un signal net :

- Rafale de `"Utilizing the connection for poll took 500-950ms"` — un verrou de connexion qui devrait se prendre quasi instantanément
- Flot de `"accepting failed: timed out"` et `"discarding packet with unexpected remote during handshake"`
- Les timestamps applicatifs traînaient de 50-60s derrière l'heure de réception journald — signature classique d'un process saturé qui n'arrive plus à vider sa propre file

C'est la même famille de symptôme que le verrou-pool-otage du 17/07. Le suspect le plus probable : **piste 3** (rafales de churn/handshakes, encore ouverte dans la liste des résiduels du fix `ConnectionInner`) plutôt qu'une pure fuite mémoire — une rafale de handshakes qui sature l'accept expliquerait à la fois le blocage réseau et la chauffe (contention = CPU brûlé).

## 3. Le fix : borner la durée de vie, pas seulement corriger le symptôme

Le vrai problème structurel n'était pas seulement "quelque chose a coincé" — c'est que **rien ne bornait combien de temps ce quelque chose pouvait s'aggraver sans supervision**. Le service tournait depuis des heures sans personne qui regarde (la boucle CSV était déjà morte), et `Restart=always` (déjà en place) ne protège que du crash — pas de la dégradation progressive.

Fix déployé : `deploy/tom-node.service.d/override.conf`

```ini
[Service]
RuntimeMaxSec=14400
```

4 heures. Combiné à `Restart=always` (déjà confirmé actif), le service se recycle tout seul avant d'avoir eu le temps d'accumuler quoi que ce soit d'incontrôlé — indépendamment de toute supervision humaine ou automatisée. Ce n'est pas un fix du root cause (piste 3 reste ouverte), c'est un **filet structurel** : même si le même scénario se reproduit, son rayon d'action est plafonné à 4h maximum, pas 9h30 sans personne qui regarde comme cette nuit.

Principe retenu pour la suite : plus aucun nœud/process long-running sans borne de durée explicite. Voir mémoire `bound-every-node-and-campaign-on-a-timeline`.

## 4. Validation — une nuit de preuve, pas une déclaration de victoire

Chaîne de ~20 check-ins SSH bornés (cadence 25 min, 22h23 → 07h35), chacun comparant mémoire cgroup + compteurs réseau (`conns_quic_live`, `handshakes_accepted`, `relais_accepts_total`, `peers_known`) au check précédent, cherchant une tendance, pas du bruit ponctuel.

**Résultats :**

| Fenêtre | Boot | Durée | Memory max atteinte | `conns_quic_live` |
|---|---|---|---|---|
| 1 | 22h16:43 → 02h16:53 | 4h00 | ~60M | Toujours retombé à 0 entre les pics |
| 2 | 02h16:53 → 06h16:53 | 4h00 | ~31M | Toujours retombé à 0 entre les pics |
| 3 | 06h16:53 → (en cours) | — | ~30M à 07h35 | Stable à 2 (trafic réel du matin) |

Sur budget 920 Mio disponibles, jamais approché une fraction significative. **Deux cycles `RuntimeMaxSec` automatiques observés, pile à l'heure prévue** (02h16:53 et 06h16:53 CEST, exactement 4h00 après chaque boot), `Restart=always` a relancé le service en quelques secondes les deux fois — preuve en conditions réelles, pas juste en théorie.

Deux fausses alertes internes (légère ré-accélération mémoire vers 00h39-01h05, +0.3M→+1.3M→+2.5M sur 3 fenêtres) ont été surveillées puis se sont résolues d'elles-mêmes (retombées à +1.6M) — exactement le comportement attendu d'une surveillance qui cherche une tendance plutôt que de paniquer sur un point isolé.

Le fix `ConnectionInner` de la veille (pool-close détaché + `app_handle_count`, commits `f406a83`/`12144a7`/`cc41036`) tient donc la charge réelle, sans supervision continue d'un humain, sur une nuit complète.

## 5. Trouvaille annexe — staleness Xcode (récidive)

Le matin du 23/07, le Mac affichait encore build 137 alors que la source était à 139 depuis la veille. Cause : troisième chemin de build (`apps/tom-node-tvos/build/`, distinct du `.build/xcode` du Makefile ET du DerivedData global Xcode) — la même classe de bug déjà documentée en mémoire (`xcode-derived-data-staleness-version-drift`), Xcode qui ne recompile pas fiablement un Swift Package local (`TomProtocolKit`) après un simple edit source.

Fix : purge des 3 caches de build (déplacés, pas supprimés — `rm -rf` est bloqué par une règle absolue du projet, `.claude/settings.json`), rebuild à froid, **vérifié cette fois dans le log de compilation lui-même** (`Compiling TomVersion.swift`), pas sur la foi d'un `BUILD SUCCEEDED` — la leçon du soir précédent où cette même vérification avait été trop vite acceptée.

## 6. Suite — propagation fleet-wide (en cours)

Le fix `ConnectionInner` vit dans les crates Rust (`tom-quinn`, `tom-transport`), consommées par les apps Apple via le XCFramework FFI packagé dans `TomProtocolKit`. Tant que ce XCFramework n'est pas rafraîchi et redéployé, iPhone/iPad/tvOS tournent avec l'ancien code de gestion de connexion — seul le relais NAS bénéficiait du fix jusqu'ici.

Action : `make ffi-xcframework` (rebuild toutes plateformes) + bump `TomVersion.build` → **140** + redéploiement Mac/iPhone de Malik/iPad/Apple TV.

## 7. Ce qui reste ouvert

- **Piste 3** (rafales de churn/handshakes) : cause probable de cet incident précis, toujours pas corrigée à la racine — le `RuntimeMaxSec` limite les dégâts, ne l'empêche pas.
- **Piste 4** (keep-alive qui ré-arme l'idle sur pairs morts) : résiduelle, non liée à cet incident.
- **Kernel log forwarding cassé** sur la VM NAS — découvert cette nuit, empêche toute confirmation OOM directe au prochain incident. À réparer (`journald` config) pour ne pas revivre ce trou de visibilité.
- **`journalctl -b -1` a bien survécu au power-off manuel** de Malik (logs applicatifs préservés) — bon réflexe à généraliser : privilégier l'extinction propre à l'arrachage quand c'est possible.
