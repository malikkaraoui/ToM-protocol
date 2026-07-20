# Audit d'instrumentation — Phase 0 du banc « courbe de masse »

> Exécuté le 2026-07-20 (GO Malik). Prérequis BLOQUANT de toute courbe
> (`banc-courbe-masse.md` §2). Méthode : 2 canaris de comptage (20×1 Ko NAS→Mac,
> croisement 3 sources) + scénario 7 orchestrateur (kill/retour/dédup) + lecture du code
> des compteurs. Flotte : build 134, bruit ambiant réel (bot NAS + campagne iPad).

## Verdict

**Phase 0 FAITE — le comptage applicatif exact est possible, mais PAS avec les compteurs
natifs.** Source de vérité : le collecteur UDP (apps, par seq) et `/inbox` (headless).
6 findings, 2 corrections appliquées, 1 question ouverte hors-banc.

## Findings

1. **Les compteurs `:9300`/`:9091` comptent des ENVELOPES protocolaires, pas des messages
   applicatifs.** Preuve code : `executor.rs:178` (`inc_messages_sent` à CHAQUE envelope
   sortante — ACKs signés inclus, décision #1 oblige), `loop.rs:344`
   (`inc_messages_received` à chaque datagramme entrant, avant routage). Preuve terrain
   (canari B) : ΔNAS_envoyes = +37 pour 20 injectés = 20 CTRL + 3 réponses bot
   (journalctl) + ~14 ACKs/contrôle. → **Interdit de lire un « débit applicatif » dans ces
   compteurs.** Ils restent utiles comme compteurs d'activité I/O.
2. **Le compteur `:9091` (apps) est RETARDÉ** : +12/20 à T+10 s, rattrapage complet
   < 2 min (throttle de persistance/refresh — cohérent avec le piège « ne survit pas au
   SIGKILL » du runbook). → Lire à STABILISATION (2 lectures identiques espacées) ou
   passer par le collecteur.
3. **Un nœud headless Linux est INVISIBLE du collecteur** : 0 ligne « nas » sur les 3000
   dernières (Mac/iPad/ATV/iPhone seulement). L'observabilité de flotte actuelle repose
   entièrement sur les apps Apple. → Pour la Phase 1 (nœuds headless en nombre) : brancher
   l'export UDP des headless vers le collecteur, OU compter par `/inbox`
   (`?contains=CTRL:`, existe déjà) + journalctl (⚠️ UTC).
4. **L'écho applicatif pour la latence EXISTE déjà** : le bot répond « recu 5/5 (msg #N) »
   à tout message non-écho, anti-boucle intégré (`tom-tui/src/main.rs:1478-1485`). MAIS la
   réponse ne cite PAS le seq du message d'origine → appariement requête/réponse impossible
   sous concurrence. → Phase 1 : sérialiser les pings de latence (1 en vol à la fois), ou
   patch mineur « la réponse cite le seq reçu ».
5. **Deux bundles app Mac coexistaient à md5 DIFFÉRENTS** (`build/…` = process live vérifié
   134 via :9091 ; `.build/xcode/…` = binaire non vérifié) et l'orchestrateur relançait le
   second après kill. **CORRIGÉ** (`orchestrator.py`, chemin du process observé) — piège
   « vieux binaire ressuscité » (cousin du « vieux 51 »).
6. **Fenêtre grise autour d'un SIGKILL** (scénario 7) : vague pré-kill envoyée 2 s avant le
   kill → 1/10 seq seulement au collecteur, et JAMAIS rejoués par le backup (alors que la
   vague pendant-absence est rejouée 20/20, dédup parfait). Lecture la plus cohérente :
   messages ACKés au protocole, app tuée avant le log/la remise → la TRACE est perdue,
   le backup ne rejoue pas (déjà « livrés »). → Banc : **fenêtre d'exclusion de comptage
   ±5 s autour de tout kill** (Phase 2 churn).

> **QUESTION OUVERTE (produit/protocole, HORS banc — à instruire séparément)** : dans la
> fenêtre du finding 6, si l'app ne persiste pas le message AVANT que le runtime émette
> l'ACK, un crash peut perdre côté utilisateur un message que le réseau considère livré
> (décision #1 : delivered ⟺ ACK). À vérifier sur pièces (ordre persistance/ACK côté
> app + FFI) avant d'en faire un chantier.

## Validations positives (ce qui est fiable)

- **Comptage exact par seq au collecteur** : 2 runs, 20/20 seq distincts, 0 doublon.
- **Dédup sous kill/retour (I8)** : aucun doublon ; **backup ADR-009** : 20/20 rejoués
  après l'absence ; retour app en 16,1 s ; NAS re-voit le pair (I3).
- **Pas d'auto-reply côté apps Swift** (Mac : +2 envois pendant le canari) — le bruit
  émetteur vient du bot headless et du protocole, pas des apps.
- **Bruit ambiant chiffré** : NAS ~26 envelopes/min, Mac ~24 reçues/min → fenêtre témoin
  OBLIGATOIRE avant tout comptage par compteurs (déjà dans l'orchestrateur).
- `[::1]:9091` ET `127.0.0.1:9091` répondent tous les deux (la note « IPv6 only » est
  périmée).

## Conséquences gravées pour la Phase 1 (courbe LAN)

1. Débit livré/nœud : **par seq au collecteur** (apps) et **/inbox** (headless) — jamais
   par `:9300`/`:9091`.
2. Latence : écho bot **sérialisé** (ou patch seq-dans-la-réponse d'abord).
3. Les headless doivent devenir visibles du collecteur (export UDP) ou comptés par
   inbox/journalctl — à trancher avant d'écrire le harnais Phase 1.
4. Churn : fenêtre d'exclusion ±5 s autour des kills pour le comptage par traces.

## Reproduire

Canaris : ~40 lignes chacun sur les briques de `scripts/chaos/orchestrator.py`
(`ambient_window`, `collector_since`, `seq_counts`) — méthode décrite ci-dessus ;
scénario kill/dédup : `python3 scripts/chaos/orchestrator.py --scenarios 7`.
