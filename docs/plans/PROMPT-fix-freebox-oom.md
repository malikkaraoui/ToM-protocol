# PROMPT DE REPRISE — chasse au bug OOM Freebox (gros moyens, multi-agent)

> Rédigé 2026-07-21 soir. **But : EN FINIR avec ce bug.** Une chasse, pas une
> exploration. Boucle qui resserre : trancher → fixer → tester → itérer jusqu'à
> ce que la Freebox ne fuie plus. Employer les gros moyens (multi-agent, workflow).

## 0. PREMIÈRE ACTION (avant tout)
Lire, avec QMD (`mcp__qmd__status` d'abord — témoin cache) :
- mémoire `tom-freebox-oom-carnet-rendezvous` (l'autopsie complète)
- mémoire `session-handoff-2026-07-21-soir` (l'état)
- `vault/30-discoveries.md` entrée « AUTOPSIE OOM Freebox »
Vérifier l'état Freebox : `ssh root@192.168.0.83` (arp si injoignable, IP dynamique),
`systemctl status tom-node`, RSS via `/proc/$(systemctl show -p MainPID --value tom-node)/status`.

## 1. CE QUI EST DÉJÀ ÉTABLI (ne pas refaire)
- **Fuite RÉELLE** (4 OOM à 760 Mo), permanente, ∝ à l'interaction avec des nœuds.
  Test 8-nœuds : 38→208 Mo en 2,5 min, **ne redescend jamais** (229 à +20 min).
- **Transport INNOCENTÉ** : repro locale `tom-stress listen` + N connexions
  directes = +3 Mo/40, +1,4 Mo/12 longues+kill. Négligeable. Ce n'est PAS le
  MagicSock/RemoteStateActor. `prune_inactive` commenté (remote_map.rs:38-41) ne
  borne que des mappings ~50o.
- **Éliminés** (code + ordre de grandeur) : scores, backup_redelivery_queue,
  mainline (bornée), pending/backup (bornés KL#11).
- **COUPABLE RESSERRÉ** : self-relay (serveur relais embarqué tom-relay) OU
  DHT/rendez-vous. Seuls composants absents de LEAKHUB, fuient QUE sous
  interaction (Freebox à vide = stable, tests A/D).

## 2. PHASE 1 — TRANCHER relais vs DHT (le call-site, en premier)
Deux voies, choisir la plus rapide qui MARCHE (la repro relais locale a échoué
os err 6 ; l'instrumentation est plus fiable) :
- **Voie instrumentation (préférée — « lire la vérité »)** : exposer dans le
  `/status` du nœud DEUX compteurs : (a) clients relais actifs
  (`Clients` DashMap de `tom-relay/src/server/clients.rs` — `.len()`), (b)
  connexions QUIC vivantes (`SocketMetrics.connections_opened - connections_closed`,
  via `endpoint.metrics()`). Câblage : tom-connect expose → tom-transport remonte
  au snapshot → tom-protocol `MetricsSnapshot` (metrics.rs) → tom-tui status
  (main.rs:186). Rebuild ARM (`cargo zigbuild -p tom-tui --bin tom-chat --target
  aarch64-unknown-linux-musl --release`), redéployer, refaire le test 8-nœuds,
  **LIRE quel compteur suit le RSS**. Le compteur qui monte avec le RSS et ne
  redescend pas = le coupable. Bonus : observabilité durable (« plus de boîte noire »).
- **Voie repro relais locale** (si l'instrumentation traîne) : fiabiliser un
  `tom-chat --embedded-relay --embedded-relay-bind 127.0.0.1:PORT` (l'os err 6
  venait peut-être de `--isolated` + bind loopback — essayer sans isolated mais
  data-dir éphémère, ou bind sur l'IP LAN) + des clients `TOM_RELAY_URL=... tom-stress`
  qui relaient. Mesurer le RSS du relais sous churn de clients.

## 3. PHASE 2 — LE FIX (deux volets, le 2ᵉ est l'intuition de Malik)
### Volet A — colmater la fuite localisée (Phase 1 dit où)
Selon le verdict : borner/nettoyer l'état par client relais (tom-relay) OU l'état
DHT. Principe du projet (classe de bug retention, [[tom-memory-retention-class-of-bug]])
: budget en OCTETS + un point de mutation unique + éviction. Réactiver une borne
existante (comme prune_inactive) si elle s'avère pertinente APRÈS localisation.

### Volet B — LE CARNET A UN MAXIMUM (intuition Malik, à incorporer au fix)
**Verbatim Malik** : « un carnet détenu a un maximum, on n'a pas besoin d'avoir
l'adresse des relay, backup, ou autre rôle. Le but on le sait : se concentrer sur
les personnes qui comptent (perso j'écris à 20 personnes, et les autres c'est une
fois dans l'année, max en tout 30). »

- **Distinction fondamentale à introduire** : le CARNET DE CONTACTS (les gens à
  qui j'écris — borné ~30) ≠ l'INFRA (relais/backup/hôtes de rendez-vous — des
  ROLES, éphémères, gérés séparément, JAMAIS gardés comme contacts).
- Aujourd'hui tout est mélangé dans la topology (789 entrées). Le fix borne le
  carnet de CONTACTS et n'y met PAS les node_ids d'infra (un relais que je croise
  n'est pas un contact ; je n'ai pas besoin de son adresse dans mon carnet).
- Effet attendu : le carnet de la Freebox chute de 789 à quelques dizaines →
  supprime la source du gonflement à la racine, indépendamment du call-site.
- ⚠️ Design-first léger : c'est protocolaire (touche la topology/PoP). Vérifier
  qu'un relais/backup a bien un canal SÉPARÉ (RelayRegistry existe déjà,
  `discovery/`) et que borner les contacts ne casse pas le routing multi-hop
  (le relais reste joignable via son registre, pas via le carnet de contacts).

## 4. MÉTHODE — gros moyens, boucle qui resserre
- **Multi-agent / Workflow** : fan-out pour (a) localiser (instrumentation +
  repro en parallèle), (b) concevoir le fix des 2 volets, (c) vérifier
  adversarialement (le fix tient-il sous le test 8-nœuds ? le carnet borné
  casse-t-il le routing ?).
- **Boucle** : trancher → fixer un volet → tester (test 8-nœuds répétable +
  observation RSS) → mesurer si ça resserre → itérer. Ne PAS empiler les fixes
  sans mesurer entre chaque (systematic-debugging Phase 4).
- loop-master + review-oracle + gate avant push. Commits FR sujet minuscule.

## 5. CRITÈRES DE SORTIE (en finir)
1. Call-site tranché et prouvé (le compteur instrumenté suit le RSS).
2. Sous le test 8-nœuds répété : le RSS **plafonne et redescend** après le départ
   des nœuds (plus de +21 Mo/nœud permanent).
3. Carnet de contacts borné (~30) : la Freebox tourne 24 h+ sans OOM, carnet
   stable à quelques dizaines, l'infra (relais/backup) gérée hors carnet.
4. Non-régression : la flotte se parle toujours (multi-hop via relais OK), tests
   workspace verts, FFI vérifié (`scripts/check-ffi.sh`).

## Garde-fous
- Anti-pollution rendez-vous : tout nœud de test = `hermetic()`/`--isolated` ou
  namespace test (incident 20/07). Le test 8-nœuds sur le VRAI rendez-vous ajoute
  des fantômes 24 h — assumé/documenté, kill + `/reset` après.
- Ne jamais conclure « fuite » sur 8 min (trop lent) ni « OK » sur le seul RSS
  (allocateur) — croiser avec les compteurs instrumentés et les OOM.
- NAS actuellement propre (30 Mo, carnet archivé `/root/tom-data.ghosts-*`).
