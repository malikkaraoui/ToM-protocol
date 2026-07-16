# ToM — Feuille de route ancrée réel + vision ultra long terme

> 2026-07-16 · build 74 · compagnon de `TOM-PLAN-GLOBAL.md` (stratégie L0→L1→L2) et de `vault/40-roadmap.md` (journal détaillé).
> Ce document fait trois choses : (1) **ancrer l'état réel** mesuré aujourd'hui, pas supposé ; (2) **réordonner R13→R18** selon ce que le terrain a tranché ; (3) poser la **vision ultra long terme** sur quatre axes.
> Règle d'or (mémoire `observability-must-reflect-ground-truth`) : un jalon n'est « fait » que **mesuré sur la flotte réelle**, jamais sur un proxy vert.

---

## 1. Ce que le terrain a tranché (mesuré au 2026-07-16)

Trois semaines de doute réseau se sont levées cette semaine. Les faits, avec preuve :

| Ce qu'on croyait (≤ 07-13) | Ce que le terrain dit (07-16, mesuré) |
|---|---|
| « DIRECT same-WiFi ne tient jamais, 100 % RELAY » | **Faux — c'était un mensonge d'affichage.** `last_path` FFI était un singleton global écrasé par n'importe quel pair. Corrigé (#33). Vérité terrain : **DIRECT partout**, y compris **iPad↔Apple TV en IPv6 global bilatéral** (11 ms), NAS↔iPad IPv6 (4,5 ms), Mac↔pairs DIRECT. Zéro flapping chronique sur 16 min d'observation. |
| « Il faut ouvrir l'IPv6 entrante sur la Freebox à la main » | **Faux — débloqué en autonomie.** Le vrai verrou était `Connection::is_ipv6()` qui testait les paths existantes au lieu de la capacité socket (#32). Le hole-punch IPv6 global marche **sans toucher la box**. |
| « iOS suspension = APNs obligatoire (R18) » | **Requalifié en non-objectif assumé** (décision produit 07-15). Le pattern voulu : premier plan = contribue plein ; arrière-plan = quelques minutes de sursis ; filet = backup 24h. Pas de daemon, pas de push. Le **watchdog anti-blocage** (#9, build 73) sort le nœud d'un démarrage figé après handoff réseau — la vraie résilience mobile. |
| « On n'a pas de vue fiable de l'état réseau » | **Résolu.** Observabilité **path par pair** (#33) : `paths_by_peer` exposé sur `:9091` (apps) / `:8085` (NAS), badge par pair dans l'UI, événement `path_change` au collecteur. On voit enfin qui est DIRECT/RELAY/v4/v6 vers qui. |

**Effets de bord durcis cette semaine** (builds 68→74) : skew d'horloge honnête (#25), nom d'usage propagé en cellulaire via DHT (#27), anti-orage des timers de maintenance (#28), version par device (#30), antispam grâce + dédup alerte (#31 t1), **file de redelivery backup pacée** (#31 t2 — plus de burst antispam au join, 5 msg/s/pair, ADR-009 préservé), filtrage du bruit d'events path.

**Ce qui reste vraiment ouvert** (honnêteté) : herméticité des tests réseau (des nœuds de test se font découvrir par la flotte pendant `cargo test`) ; stabilité DIRECT sur la **durée longue** (mesurable maintenant, pas encore mesurée > 1 h) ; le stall NAS historique (deadlock mutex tom-quinn) à re-confirmer clos sur endurance.

---

## 2. Roadmap R13→R18 — réordonnée selon le réel

> Principe inchangé : **une seule porte publique = SPOF**. Ces chantiers élargissent le vivier de portes éligibles, sans point central. Chaque step validé en `tom-stress` + campagne multi-device réelle.

### R13 — Porte d'entrée automatique · 🟢 avancé
- Étape 1 (mapping UPnP obtenu sur Freebox réelle) ✅ · Étape 2 (mapping du port relais embarqué) ✅.
- **Reste** : étape 3 = test d'acceptation **iPhone data ↔ maison SANS le NAS** (le Mac/iPad devient la porte seul). C'est LE test qui prouve la fin du SPOF.
- **Rappel dette** (vault 07-13) : UPnP IGD est **désactivé par défaut** sur Freebox → la parade robuste est `tom-gateway` (API native Freebox, pairing `app_token` une fois sur l'écran de la box), pas l'UPnP générique. **À construire.**

### R14 — IPv6 first-class · 🟢 largement débloqué (ré-évalué 07-16)
- Le DIRECT IPv6 global **est prouvé** en autonomie (voir §1). Ce qui était un « chantier bloqué sur action box » est devenu un **acquis mesuré**.
- **Reste** : (1) confirmer la **stabilité longue durée** du DIRECT v6 (endurance > 1 h, tous liens) ; (2) préférence v6 au dial généralisée + PCP pinhole auto quand la box le permet (zéro friction v6) ; (3) fallback v4/relais propre quand le FAI n'a pas d'IPv6.
- **Note** : R14 est passé devant R13 en maturité — l'IPv6 hole-punch est plus fiable que le NAT v4 et court-circuite le besoin d'ouvrir des ports. **Investir v6 en priorité** est le meilleur levier « zéro friction ».

### R15 — Annuaire local des pairs · 🟡 amorcé
- Brique déjà là : `paths_by_peer` (#33) donne `node_id → path_kind + addr` en mémoire.
- **Reste** : persister `node_id → relais habituel + dernières addrs (LAN/publique/v6) + path_kind` ; dial parallèle cache + lookup frais ; expiration douce (décision #4). Gain : rejoin < 2 s famille/amis, moins de pression DHT.

### R16 — Nœud léger multi-plateforme · 🔨 le vecteur d'adoption
- Binaire statique musl déjà cross-compilé (NAS ARM64 tourne). Canaux : Raspberry Pi Imager (« ToM Node OS »), Pi-Apps, Docker (NAS/home-server), VM Freebox qcow2 (nœud + `tom-gateway` préinstallés, pairing natif box). One-liner curl (pattern Pi-hole).
- **Prérequis** : R13 (porte auto) — « brancher et oublier » d'abord, distribuer ensuite.

### R17 — Seeds optionnels + rotation observée · 🔨
- 1-2 VPS `relay-eu/us.tom-protocol.org` (URLs déjà en défaut committé), **explicitement retirables**. Valider en réel la rotation (publication/dé-publication du gate, répartition RelaySelector). Confort d'amorçage, **pas d'infra sacrée**.

### R18 — Wake-up mobile · 🟢 requalifié NON-OBJECTIF
- **N'est plus un jalon.** APNs/FCM/daemon background = contraire à la vision (couche invisible, pas service énergivore toujours-actif). Stratégie mobile assumée = watchdog résilient + scenePhase restart + backup 24h. Un hook « sonnette » neutre côté SDK **peut** exister pour une app tierce qui, elle, choisit APNs — mais le cœur ToM n'en dépend jamais.

**Ordre de bataille recommandé** : R14 stabilité longue (mesure) → R13 étape 3 + `tom-gateway` (fin du SPOF) → R15 (annuaire) → R16 (packaging) → R17 (seeds). L1 (Proof of Presence) reste le grand saut d'après, cadré dans `TOM-PLAN-GLOBAL.md` §Phase 1.

---

## 3. Vision ultra long terme — quatre axes

> L'étoile polaire (décision #7) : **une fondation universelle de transport, neutre, gratuite, chiffrée, sans blockchain ni frais — comme TCP/IP, invisible.** Les quatre axes ci-dessous sont les dimensions de cette étoile, pas des produits.

### Axe A — Adoption / distribution : la contagion par le bas
- **Thèse** : ToM ne se « lance » pas, il **se propage** — métaphore virus positif (mémoire `tom-vision-cible`). Le premier hôte est le geek souverainiste (Pi/Freebox/self-hoster) qui veut couper ses chaînes. Il porte la vraie parole ; le deal BitTorrent est le pitch (héberger un nœud = messagerie chiffrée gratuite, zéro serveur à payer).
- **Court terme (R13-R16)** : rendre l'installation littéralement sans terminal (Imager, VM box, Docker). Un nœud installé = une porte publique complète, recrutée par le gate sans manip.
- **Moyen terme** : **le LLM comme canal de distribution** — docs LLM-first, SDK, MCP, plugin. Un assistant qui sait déployer ToM en 3 messages est un vecteur d'adoption que les concurrents centralisés n'ont pas.
- **Ultra long terme** : le point de bascule où **héberger un nœud devient le défaut invisible** — livré avec la box, l'OS, le terminal de paiement. L'utilisateur ne « choisit » plus ToM ; il l'a déjà, comme il a TCP/IP. La messagerie n'est qu'la première app ; le bus transporte tout.

### Axe B — Souveraineté technique : effacer les derniers maîtres
- **État** : le fork iroh est complet (namespace `tom-*`), le protocole est autonome. Résidus : hostnames n0/iroh (`dns.iroh.link`, canary) actifs **seulement** avec le preset `n0_discovery`, remplaçables.
- **Court/moyen terme** : (1) retirer/remplacer les derniers résidus n0 → **découverte 100 % souveraine** (DHT rendezvous + relais communautaires) ; (2) relais **tournants** (rôles réseau ADR-006 + gate ADR-010) — aucun nœud privilégié, jamais ; (3) IPv6 first-class = chaque appareil son adresse directe, moins de dépendance aux relais.
- **Ultra long terme** : **le réseau héberge son propre code.** Si GitHub tombe, le code vit dans le swarm (distribution du binaire + specs normatives via le protocole lui-même). Gouvernance en rotation, pas de capture. La souveraineté n'est pas un slogan : c'est l'absence testable de tout point unique dont la disparition tue le réseau — infra, autorité, ou juridiction.

### Axe C — Plateforme / OS natif : de l'app à la couche système
- **Thèse** : aujourd'hui ToM est une app (`TomNode`) qui parle au runtime Rust via FFI. Demain c'est une **couche** que les apps tierces appellent.
- **Court terme** : durcir le SDK (Rust `tom-sdk`, Swift `TomProtocolKit`) + les specs wire/crypto normatives (déjà publiées) → n'importe qui implémente ToM dans n'importe quel langage, byte-for-byte vérifiable.
- **Moyen terme** : **wake-up natif là où c'est légitime** — pas un daemon énergivore, mais l'intégration propre à chaque OS (Android foreground service raisonné, Linux/desktop service système, home-server toujours-allumé). L'API « une app tierce envoie un message ToM » sans réimplémenter le transport.
- **Ultra long terme** : ToM comme **primitive système invisible** — le terminal de paiement, l'objet IoT, la box, le desktop l'utilisent sans que l'utilisateur le sache, exactement comme une socket. La messagerie devient un cas d'usage parmi mille (sync, présence, transfert), tous portés par le même bus neutre.

### Axe D — Économie / gouvernance : vivre sans capturer
- **Principe LOCKED** : réputation à fade progressif, **jamais de ban permanent** (#4) ; antispam « sprinkler gets sprinkled », charge progressive **jamais exclusion** (#5) ; L1 ancre mais **n'arbitre jamais** (#3) ; **pas de token spéculatif**.
- **Le mur de recherche (L1, Proof of Presence)** : prouver que le consensus par **présence** est réel — entropie non-biaisable (M1.2, problème de recherche), anti-Sybil quantifié (M1.4, coût de relais non-amortissable). C'est le verrou qui conditionne toute notion de « valeur » (L2). Tant qu'il n'est pas prouvé, L2 reste un bonus différé, pas une promesse.
- **Incitation sans monnaie** : le deal est **utilitaire, pas financier** — tu relaies, donc tu profites d'un réseau gratuit et chiffré ; tu contribues, ton rôle monte ; tu abuses, ta réputation fade. L'incitation est l'accès, pas un jeton.
- **Ultra long terme** : une gouvernance de protocole **minimale et en rotation** (maintainers tournants, specs normatives comme constitution), et — **si et seulement si** L1 tient — une couche de valeur L2 (portefeuille scellé auto-custodié, dépense témoignée, cohérence > disponibilité sous partition). L2 n'est pas le but ; c'est la preuve ultime que le PoP est réel. Le but reste le **bus neutre**.

---

## 4. Les murs nommés (rien de caché sous le tapis)

| Mur | Où | Statut |
|---|---|---|
| Entropie PoP biaisable (grinding) | L1 / M1.2 | 🔴 problème de **recherche** — à lancer, pas de l'ingénierie |
| Sybil patient rafle le quorum | L1 / M1.4 | 🔴 à quantifier (coût de relais non-amortissable) |
| Partition → double-spend | L2 / M2.0 | 🔴 porte = choix CAP (cohérence > dispo) à acter |
| Stabilité DIRECT longue durée | R14 | 🟡 mesurable maintenant (télémétrie #33), pas encore mesurée > 1 h |
| Herméticité tests réseau | infra | 🟡 nœuds de test visibles par la flotte pendant `cargo test` |
| SPOF porte publique | R13 | 🟡 vivier élargi (self-relay), test « sans le NAS » à faire |

---

*L0 est réel et mesuré — cette semaine l'a prouvé jusqu'à l'IPv6 direct. L1 est le grand saut testable (présence prouvée sans argent). L2 est le sommet, conditionné à L1. Chaque mur est nommé ; chaque porte a son prix ; aucune promesse n'est vendue comme un fait. La feuille de route n'est pas gravée — elle est vivante, et elle vient de gagner en réalité.*
