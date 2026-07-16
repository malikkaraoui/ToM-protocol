# CHECKPOINT FORT — 2026-07-16 (build 71)

> Point de sauvegarde clair. HEAD = `84f164c`, **build 71**, tout poussé, **CI verte**.
> Rien à perdre : tout est ci-dessous + dans la mémoire agent (`session-handoff-2026-07-16`).

## 🟢 L'état est SUPERBE — ce qui a été livré (CI verte, déployé build 71)

| # | Chantier | Commit | Effet |
|---|----------|--------|-------|
| **#25** | Skew horloge honnête | `5e2008e` | N'échantillonne plus les messages relayés (`via.is_empty()`) → fini le « +8min » trompeur |
| **#27** | Nom via DHT | `9f47495` | Le username se propage en cellulaire (signé dans `DhtNodeAddr`) — le pair s'affiche par son nom |
| **#28** | Anti-orage | `6f4a196` | 18 intervalles en `MissedTickBehavior::Skip` → plus de rafale de ticks (republish/rejoin) après un stall / résume iOS |
| **#30** | Version par device | `42a074a`→`3c4d989` | « Mac 71 » en tête de Status + « iPad 71 » dans les Pairs ; `app_build` propagé signé ; **exposé sur :9091** (vérif build à distance) |
| **#31-t1** | Antispam grâce + dédup | `f2abdf2` | Pair connu qui se reconnecte n'est plus throttlé en « étranger 30/s » (burst 4x) ; alerte « Pair ralenti » dédupliquée (1/pair/10s) |
| **#32** | Fix hole-punch IPv6 | `84f164c` | `Connection::is_ipv6()` reflète la vraie capacité socket → le hole-punch peut enfin tenter les IPv6 globales même après un bootstrap relais IPv4 |

**Le réseau MARCHE** (flotte Mac/iPad/AppleTV en 71, connectée, messages livrés) — via relais.

**Bonus outillage** : `curl http://<ip>:9091` d'un nœud renvoie son `app_build` → on vérifie le build qui tourne RÉELLEMENT (fini les déploiements à l'aveugle).

## 🟡 CE QUI PEUT ÊTRE ARRANGÉ (le gros sujet ouvert)

**Connexion DIRECTE same-WiFi bloquée en RELAY** (#26/#32). C'est ce qui rend le réseau « mou » (chaque message fait un détour relais).

**Prouvé :**
- Découverte mDNS : OK, rapide.
- Collecte + annonce des **IPv6 globales** (`2a01:...`) : **OK** (vérifié empiriquement, le nœud annonce ses 4 IPv6).
- Bug `is_ipv6()` : réel, **corrigé** (#32).
- **MAIS** : 3 appareils tous en 71 sur le WiFi → **100% RELAY, zéro DIRECT stable**. Le fix seul ne suffit pas.

**Hypothèse #1 (non prouvée)** : le pare-feu Freebox bloque l'**IPv6 entrant** → l'IPv6 hole-punch tenté échoue faute de pinhole coordonné. Décision produit : on NE touche PAS les réglages box, le protocole doit trouver la parade.

**Prochaine étape (chantier transport dédié, à froid) :**
1. **Instrumenter le NAT-traversal** (tom-quinn-proto `iroh_hp` / `connection`) : logger si l'IPv6 est maintenant *tentée* (`initiate_nat_traversal_round ipv6=?`), quels candidats sont essayés/rejetés, le résultat du hole-punch IPv6. ⚠️ Ces logs Rust n'atteignent PAS le collecteur UDP (Swift only) → les remonter en `ProtocolEvent`, OU observer via un run `tom-chat` avec `RUST_LOG`.
2. **Trancher** : IPv6 tentée-mais-échoue (→ pare-feu, besoin de hole-punch coordonné simultané) vs pas-tentée (→ autre verrou).
3. **Itérer** le fix (hole-punch IPv6 coordonné, ou fallback relais optimisé). Zone délicate : fork QUIC 41K LOC — comprendre avant de changer, ne pas casser l'IPv4, ne pas affamer la découverte.

**Autre chantier ouvert** : **#31 tranche 2** — file de sortie maîtrisée côté expéditeur (dédup queue / drop périmés / pacing / backup→attendre direct). Doc : `docs/plans/R-sender-queue-antispam-grace.md`.

## Infra / rappels
- Flotte : Mac, iPad (1DC13ED3), Apple TV Séjour (2C36638F) en **71**. iPhone Malik (C3CFC878) sorti (build 70, à repasser en 71). NAS 192.168.0.83 (systemd `tom-node`, binaire `/usr/local/bin/tom-chat`) = Rust build 69 → **à recompiler+redéployer en 71** (n'a pas le fix IPv6).
- Signature : team **UPES5479T5** (payante). Collecteur : `/tmp/tom_collector.py` (:9999), log `/tmp/tom_collector.log`. Status/nœud : `:9091` (JSON avec `app_build`).
- Build Apple : **purger le derived-data** (`.build/xcode|ios-device|tvos-device`) au bump `TomVersion`, vérifier `BUILD SUCCEEDED` + exécutable réel (pas juste `__preview.dylib`), `devicectl launch --terminate-existing`, confirmer via `:9091`.
