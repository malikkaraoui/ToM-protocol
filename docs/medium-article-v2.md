# De 0% à 99.85% sur autoroute : comment on a rendu un protocole P2P fiable sur 4G

## Trois bugs, 54 minutes d'autoroute, et 2752 pings entre la France et la Suisse.

---

La semaine dernière, on a prouvé que ToM Protocol pouvait percer les NAT. Un NAS dans un salon en France, un MacBook sur le WiFi d'une école en Suisse, 32ms de latence directe. Hole punch réussi à 100%.

C'était un PoC. Propre, contrôlé, 20 pings par test.

Cette semaine, on a lancé le protocole sur une autoroute. En 4G. Pendant 54 minutes. En traversant des tunnels, en changeant d'antenne-relais, en perdant le réseau.

Et on a cassé. Puis réparé. Puis re-cassé. Puis re-réparé.

Voici ce qu'on a appris.

---

## Le problème : un PoC n'est pas un protocole

Le PoC-4 avait un secret : chaque test créait une connexion fraîche. 20 pings, on ferme, on recommence. Pas de connexion persistante, pas de cache, pas de pool.

En conditions réelles, c'est l'inverse. Une application de messagerie maintient une connexion ouverte pendant des heures. Le réseau change (WiFi → 4G → tunnel → 4G). Le NAT rebind les ports. L'opérateur coupe silencieusement les mappings UDP après 30 secondes d'inactivité.

Pour tester ça, on a construit deux choses :

**tom-transport** : une API Rust stable qui wrappe iroh avec un pool de connexions. `send()`, `recv()`, `disconnect()`. Trois méthodes. Le pool met en cache les connexions QUIC pour éviter de refaire le hole punch à chaque message.

**tom-stress** : un binaire de stress test avec 4 modes — ping continu, burst (débit), ladder (tailles croissantes), et un mode campagne automatisé. Sortie JSON structurée, cross-compilé en ARM64 statique pour le NAS.

Setup identique au PoC : NAS Freebox Delta (ARM Cortex-A72, Debian) en listener, MacBook Pro en émetteur. Sauf que cette fois, le MacBook est sur le siège passager d'une voiture sur l'A40, en 4G, entre la France et la Suisse.

---

## Nuit du 12 février : 0%

Premier test en 4G réel. Pas sur un hotspot USB contrôlé. Sur le réseau mobile, en mouvement.

Résultat : **0/20 pings**. Zéro. Rien ne passe.

```
ping #1 recv failed: pong timeout (10s)
ping #2 recv failed: pong timeout (10s)
...
Done: 0/20 pings OK, avg RTT: 0.0ms
```

Le NAS reçoit les pings et renvoie les pongs. Mais le MacBook ne les voit jamais.

### Bug #1 : les connexions mortes restent dans le pool

Le diagnostic prend 30 minutes. Le pool de connexions met en cache la connexion QUIC. Quand le NAT 4G rebind les ports (ce qu'il fait toutes les 30 secondes environ), la connexion en cache est morte. Mais `close_reason().is_none()` retourne `true` — QUIC pense que la connexion est vivante parce qu'aucun RST n'a été reçu.

Résultat : chaque `send()` réutilise une connexion zombie. Le `open_bi()` échoue, mais l'erreur n'évince pas la connexion du pool. Le prochain `send()` réutilise la même connexion morte. Boucle infinie.

**Fix** : 3 lignes dans `node.rs`. Sur erreur de `open_bi()`, évincer la connexion du pool avant de retourner l'erreur. Le prochain `send()` crée une connexion fraîche, refait le hole punch, et tout repart.

```rust
Err(e) => {
    self.pool.remove(&to).await;
    return Err(TomTransportError::Send { ... });
}
```

---

## 13 février, 7h42 : 97%

Nouveau build, nouveau départ. Cette fois en voiture, trajet quotidien France → Suisse. 7 campagnes de test lancées pendant le trajet.

**Résultat : 97% de pings réussis, 100% de bursts réussis.** Le pool eviction fonctionne. Quand le réseau change, la connexion est recréée et le hole punch refait automatiquement.

Mais.

Campagne #7 et le test continu échouent : 0/160 pings. Le MacBook entre dans une zone sans couverture (tunnel ? zone blanche ?), et le programme ne s'en remet jamais.

### Bug #2 : les connexions zombie ne déclenchent pas la reconnexion

Le `send()` réussit (la connexion QUIC semble vivante). Le pong ne revient jamais (timeout 10s). Le programme logge "pong timeout" mais ne fait rien. Il continue à envoyer sur une connexion morte, indéfiniment.

La reconnexion (`try_reconnect()`) n'est appelée que sur erreur de `send()`. Mais quand le réseau meurt graduellement — les paquets partent mais ne reviennent pas — `send()` ne voit pas le problème.

**Fix** : compteur de timeouts consécutifs. Après 3 timeouts d'affilée, on force l'éviction de la connexion. Le prochain `send()` déclenche une nouvelle connexion.

```rust
if state.consecutive_timeouts >= 3 {
    node.disconnect(target).await;
    state.consecutive_timeouts = 0;
    eprintln!("  evicted zombie connection, will reconnect on next send");
}
```

Même logique ajoutée aux modes burst (0 pongs reçus → eviction) et ladder (tous les reps échouent → eviction).

---

## 16 février, 7h18 : 99.85%

Nouveau build. Autoroute A40, direction Suisse. Test continu : un ping par seconde, indéfiniment.

### Session 1 : 1638/1640 — 99.88%

```
RTT moyen : 1.26ms
Durée : 32 minutes
Reconnexions : 0
```

1.26 millisecondes. En 4G. C'est de la latence de LAN. Iroh a trouvé un chemin direct via hole punch UDP, et il tient. 1580 pings consécutifs sans une seule perte.

Les 20 derniers pings montrent la dégradation réseau progressive : 900ms, 1396ms, 570ms. Puis déconnexion totale — un tunnel.

Le programme tente 10 reconnexions. Toutes échouent. Il abandonne.

### Bug #3 : la reconnexion a une limite arbitraire

En mode `--continuous`, le programme est censé tourner indéfiniment. Mais `try_reconnect()` s'arrête après 10 tentatives. Si le tunnel dure plus de 2 minutes (1+2+4+8+16+32+32+32+32+32 secondes de backoff), c'est fini.

**Fix** : en mode continu, la boucle de reconnexion tourne indéfiniment. Le backoff plafonne à 32 secondes. Toutes les 5 tentatives ratées, on force une redécouverte réseau complète via Pkarr (le DNS décentralisé d'iroh). Ctrl+C reste le seul moyen d'arrêter.

### Session 2 : 1110/1112 — 99.82%

Relancé manuellement après le tunnel. Le programme met 52 secondes à se reconnecter (6 tentatives de backoff exponentiel). Ensuite : 1110 pings consécutifs, RTT moyen 9.7ms.

Le RTT plus élevé (9.7ms vs 1.26ms) suggère un passage par relay plutôt que direct — probablement un segment réseau différent au retour.

---

## Les chiffres, résumés

| Date | Test | Résultat | Bug trouvé |
|------|------|----------|------------|
| 12 fév, soir | 4G statique | **0/20 (0%)** | Pool ne vide pas les connexions mortes |
| 13 fév, matin | 4G autoroute, 7 campagnes | **97%** | Zombie detection manquante |
| 16 fév, matin | 4G autoroute, 54 min | **2748/2752 (99.85%)** | Limite de reconnexion arbitraire |

De 0% à 99.85% en 4 jours. Trois bugs. Trois fixes. Chacun trouvé en conditions réelles, pas en test unitaire.

---

## Ce que ça prouve (et ce que ça ne prouve pas)

### Ce que ça prouve

**Le P2P fonctionne sur 4G CGNAT en mouvement.** Pas en théorie, pas en laboratoire. Sur une autoroute, en changeant d'antenne-relais, en traversant des tunnels. 99.85% de fiabilité sur 2752 pings.

**Le hole punching tient dans le temps.** Session 1 : 1580 pings consécutifs sans perte, 26 minutes, RTT sub-milliseconde. Le chemin direct UDP survit aux changements d'antenne 4G.

**La reconnexion automatique fonctionne.** Quand le réseau revient après une coupure, iroh refait le hole punch et la connexion reprend. 52 secondes de reconnexion, c'est long pour un humain mais acceptable pour un protocole qui vise la résilience totale.

**Les bugs sont dans le pool, pas dans le réseau.** Les trois bugs trouvés sont des erreurs de gestion de cache côté applicatif, pas des limitations du hole punching QUIC. La couche réseau d'iroh fait son travail — c'est notre couche au-dessus qui ne gérait pas correctement les transitions.

### Ce que ça ne prouve pas encore

**Le multi-noeud.** Tous les tests sont en 1-à-1 (MacBook ↔ NAS). Le comportement avec 10, 50, 100 noeuds simultanés reste à valider.

**La résilience aux firewalls d'entreprise.** Le CGNAT opérateur est percé. Le NAT d'école est percé. Mais certains firewalls d'entreprise bloquent UDP complètement. Dans ce cas, le relay iroh reste le seul chemin — et c'est par design.

**La charge applicative.** On envoie des pings de 100 octets. Pas des messages de 100 Ko avec du chiffrement E2E applicatif par-dessus. Les tests ladder (jusqu'à 64 KB) fonctionnent, mais le cas réel avec routage et encryption ToM reste à tester.

---

## L'outillage qui a rendu ça possible

On n'aurait pas trouvé ces bugs sans un outillage adapté. Trois choix qui ont fait la différence :

**Sortie JSON structurée.** Chaque événement (ping, path_change, disconnected, reconnecting, reconnected, summary) est une ligne JSON. Pas de parsing de logs. Un `jq` ou un script Python suffit pour analyser un test de 1640 pings.

**Cross-compilation statique.** `cargo zigbuild --target aarch64-unknown-linux-musl` produit un binaire de 17 MB, zéro dépendance. `wget` + `chmod +x` sur le NAS. Pas de Docker, pas de runtime, pas de dépendance.

**Archivage automatique.** Après avoir perdu les données d'une session (écrasement de fichier), on a ajouté `--output-dir` : les fichiers JSONL et logs sont automatiquement créés avec un timestamp unique. Plus jamais de `2> file | tee file` dans le terminal.

---

## La suite

Le transport P2P est validé. L'API est stable : `send()`, `recv()`, `disconnect()`. Les trois fixes de résilience transforment un PoC fragile en couche de transport fiable.

La prochaine étape : porter la couche protocole ToM (routing, relay selection, enveloppes chiffrées, groupes) sur ce transport Rust. Le signaling server WebSocket a officiellement un successeur.

---

*Repo : [github.com/malikkaraoui/ToM-protocol](https://github.com/malikkaraoui/ToM-protocol)*
*Branche : `experiment/iroh-poc`*
*Stack : iroh (Rust, QUIC), Ed25519, cargo-zigbuild, ARM64 musl*
*Données brutes : `results/` dans le repo*
