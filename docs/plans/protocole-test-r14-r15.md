# Protocole de test — R14 (convergence chemin) + R15-lite (relais habituel)

> ## ⚠️ VÉRIFICATION FACTUELLE (relecture humaine du 2026-07-19)
>
> Ce protocole a été rédigé par un sous-agent puis **relu et corrigé**. Trois erreurs
> factuelles ont été trouvées ; les corrections sont appliquées, mais lis ceci avant
> d'exécuter quoi que ce soit :
>
> 1. **Le status du Mac écoute en IPv6 UNIQUEMENT.** `curl http://127.0.0.1:9091/` échoue
>    (faux négatif « injoignable »). Utiliser `http://[::1]:9091/` — corrigé partout ci-dessous.
> 2. **Certains endpoints n'existent PAS** : `?raw=remote_candidates` et `/peers?action=drop`
>    ont été inventés. Le serveur de statut (`apps/tom-node-tvos/TomNode/Services/StatusServer.swift`)
>    n'expose que `/` (plus `/reset` et `/group/*`, gated `#if DEBUG`). Les étapes qui les
>    utilisent sont donc **à implémenter d'abord** (elles relèvent du Lot A — observabilité),
>    pas à exécuter telles quelles. Elles sont marquées `[ENDPOINT À CRÉER]`.
> 3. **`192.168.0.28` (iPhone de Malik) est parti avec lui** le 19/07. Flotte réellement
>    disponible : NAS `192.168.0.83`, Mac `[::1]:9091`, iPad `192.168.0.23`, Apple TV
>    (node_id `75145c05`), iPhone Laura `192.168.0.49` (présence variable). Adapter les boucles.
>
> Rappel de méthode : **ne jamais conclure sur un relevé unique**, et vérifier qu'un compteur
> d'activité BOUGE avant d'interpréter une mesure (un envoi vers un pair absent de la topologie
> n'est ni tenté ni compté).


**Livrable décembre 2026 (design-first, validé avant code).** Ce document décrit les scénarios chaos concrets qui exercent R14 et R15-lite sur la vraie flotte, en produisant des verdicts par invariant (pas des impressions).

---

## 0. Invariants — Récap global

Les 8 invariants du banc (RUNBOOK-TESTS.md §1) s'appliquent **toujours**. Deux nouveaux invariants spécifiques à R14 :

- **I9a (convergence stable)** : sur un chemin où deux familles v4/v6 sont disponibles, après que le premier emprunt ait réussi, la famille reste stable (≤ 1 bascule per 300 s d'observation).
- **I9b (converge vers optimum)** : si une adresse morte est remplacée, et que la nouvelle adresse est meilleure (RTT/latence), le système utilise préférentiellement la meilleure. Mesure : chemin actif observé par tcpdump concentre ≥ 95 % des paquets de données sur la meilleure adresse.

**R15-lite ajoute** :
- **I10 (gain connexion)** : un nœud redémarré rejoint un pair connu via le relais mémorisé au moins 2× plus vite qu'une redécouverte froide (DHT off pour l'isoler).
- **I11 (pas de résurrection)** : après restart, `taille_reseau` ne croît pas et aucun dial n'est tenté vers un pair > 24 h (test avec `state.db` pollué volontairement).

---

## 1. Prérequis (avant CHAQUE série de tests)

### 1.1 État de la flotte

```bash
# Lister les nœuds accessibles (status HTTP)
for node in nas mac iphone ipad; do
  echo -n "$node : "
  curl -s -m 2 "http://ENDPOINT:PORT/" | grep -q '"node_id"' && echo "✓" || echo "INJOIGNABLE"
done
```

**État attendu** : ≥ 3 nœuds accessibles. Si NAS injoignable (IP DHCP), mettre à jour `orchestrator.py` `NAS_CONTROL`.

### 1.2 Collecteur UDP prêt

```bash
# Le collecteur UDP :9999 tourne-t-il ?
lsof -iUDP:9999 | grep -i python
# S'il est absent, le démarrer :
python3 /tmp/tom_collector.py &
# Vérifier le log existe
ls -lh /tmp/tom_collector.log  # doit être écrit en temps réel
```

**Pièges d'interprétation** (voir mémoire [[analyze-logs-myself-every-test]]) :
- Le collecteur log SANS date (heure seule) → utiliser OFFSET DE LIGNE (`awk 'NR>OFFSET'`), jamais heure.
- Silence UDP ≠ mort (perte broadcast) → toujours croiser `:9091/metrics`.

### 1.3 Disque & process

```bash
# Pas de résidu de test antérieur
ps aux | grep -E "[t]om-stress|TEST-" | wc -l  # doit être 0
pkill -f "TEST-" ; pkill -f "tom-stress"       # nettoyer si besoin

# target/ ≤ 20 Go (voir mémoire [[rust-target-disk-ceiling-20gb]])
du -sh target/  # si > 18 Go, lancer : bash scripts/clean-cruft.sh --apply --builds
```

### 1.4 Binaires à jour

```bash
# R14 : mesure en étage L d'abord (pas d'injection réseau, rapidité)
cargo build -p tom-stress --release  # Lot A observabilité OK ?

# R15-lite : si code Rust du cache relais existe, valider compilation
cargo build -p tom-protocol --release
cargo clippy --workspace -- -D warnings
cargo test -p tom-protocol --lib   # test du cache de relais isolé
```

---

## 2. R14 — Convergence de chemin : scénarios

### 2.1 Scénario A1 : Observabilité des bascules (prérequis Lot A)

**Objet** : valider que `AddrFamily` est exposée dans `paths_by_peer` et l'historique de bascules est disponible pour la mesure.

**Durée** : 10 min  
**Preconditions** : flotte stable ≥ 10 min (pas de kill en vol).

#### Étapes

1. **Baseline** : capturer `paths_by_peer` + `build` de chaque nœud
   ```bash
   for ip in "[::1]" 192.168.0.28 192.168.0.23 192.168.0.83; do
     echo "=== $ip ==="
     curl -s "http://$ip:9091/" | jq '.paths_by_peer,.app_build' > /tmp/baseline_${ip}.json
   done
   ```

2. **Stabilité observée** : relever 3× à 60 s d'intervalle (`path-matrix.py` par commodité)
   ```bash
   python3 scripts/path-matrix.py --n 3 --interval 60 --json /tmp/paths-r14-a1.json
   ```
   
   Output analysé :
   - Tableau « Bascules détectées » : compter les bascules v4↔v6
   - Tableau « Asymétries de sens » : A→B et B→A utilisent-ils la même famille ?

#### Critères PASS/FAIL par invariant

| Invariant | Critère | Fichier preuve |
|-----------|---------|---|
| **Préalable Lot A** | `paths_by_peer` expose `"addr_family": "v4" | "v6" | "relay"` explicitement (pas parsing) | jq dump de :9091 |
| **Hist bascules** | Compteur ou liste exposé pour chaque pair (« 2 bascules en 180 s, v4→v6 puis v6→v4 ») | même jq dump |
| Bascules < seuil | Sur 3 relevés × 60 s chacun : ≤ 2 bascules v4↔v6 par paire (I9a lite) | `diff(snaps)` output |
| Pas d'asymétrie dégradante | Si A→B v4 et B→A v6, confirmer que A et B **voient la même adresse** de l'autre (pas deux routages différents) | tcpdump 10 s après chaque relevé sur un chemin asymétrique |

#### Pièges d'interprétation

- **Photo instantanée ≠ stabilité** : une seule bascule en 180 s n'est pas une preuve que le système converge. Répéter avec fenêtres de 24-48 h pour estimer MTBF réel.
- **Bascule =\= dégradation** : une bascule v4→v6 où RTT s'améliore (9 ms → 5 ms) est normal. Chercher les dégradations (v4 9 ms → v6 51 ms comme le 18/07).

---

### 2.2 Scénario B1 : Élucider une bascule dégradante EN DIRECT (Lot B)

**Objet** : observer une bascule v4→v6 dégradante et déterminer si c'est un chemin mort remplacé ou une sélection défaillante.

**Conditions** : nécessite que la bascule dégradante se produise (instable par nature).  
**Durée** : observation continue jusqu'à capture de la bascule, típiquement 30-60 min.  
**Préconditions** : flotte diverse (≥ 2 téléphones pour observer les bascules iPhone).

#### Étapes

1. **Trigger opportuniste** : laisser tourner `path-matrix.py` en boucle et attendre une bascule dégradante
   ```bash
   # Loop jusqu'à capture (à lancer dans un tmux/screen)
   while true; do
     python3 scripts/path-matrix.py --n 2 --interval 120 2>&1 | tee -a /tmp/r14-b1-watch.log
     if grep -q "⚠️ DÉGRADATION" /tmp/r14-b1-watch.log; then
       echo "=== BASCULE DÉGRADANTE CAPTURÉE ===" 
       break
     fi
   done
   ```

2. **Capture réseau au moment de la bascule** : dès repérage, lancer tcpdump sur le relais (NAS) et les deux pairs
   ```bash
   # Sur NAS (via SSH)
   ssh root@192.168.0.83 "tcpdump -i en0 -w /tmp/r14-b1-trace.pcap \
     'udp port 43925 or icmpv6' " &
   # Sur Mac (local)
   sudo tcpdump -i en0 -w /tmp/r14-b1-mac.pcap \
     'udp port 43925 or icmpv6' &
   # Sur l'iPhone/iPad (via devicectl — nécessite Xcode)
   devicectl device trace start --output /tmp/r14-b1-device.trace $(devicectl list | grep iPad | cut -d' ' -f1) &
   ```

3. **Historiser les candidats** : juste AVANT la bascule repérée, interroger les slots de route du nœud source
   ```bash
   # Via l'API du nœud (si exposée) ou du collecteur :9999
   # Capture des adresses candidates du pair destination (en RAM côté source)
   # [ENDPOINT À CRÉER — Lot A] curl -s "http://[::1]:9091/?raw=remote_candidates" 2>/dev/null | jq '.'
   ```

4. **Attendre la bascule puis arrêter les traces** (≤ 1 s après)
   ```bash
   sleep 180  # attendre bascule, ou relancer trigger si timeout
   pkill tcpdump
   ```

#### Analyse post-capturage

| Données | Question | Interprétation |
|---------|----------|---|
| **tcpdump** | Nombre de paquets de données SUR chaque famille ?  (count `udp.dstport==43925`) | Si 0 sur l'ancien chemin AVANT bascule → chemin mort remplacé. Si ≥1 sur les deux → sélection/hystérésis défaillante |
| **paths_by_peer avant/après** | L'adresse remplacée est-elle NOUVELLE ou l'ANCIENNE disparue ? | Nouveau candidat probé = plug réseau. Ancien mort + nouveau découvert = nat/interface changé. |
| **RTT avant/après (collecteur)** | Nouvelle adresse rapportait-elle un RTT plus élevé ? | Oui → sélection a mal choisi (le candidat était connu comme mauvais). Non → adresse nouvelle avec RTT inconnu, probe a échoué dessus d'abord (ordre aléatoire). |
| **Historique `addr_fail_count` (Lot C, futur)** | Ancien chemin a-t-il un `fail_count` élevé ? | Oui → était déjà marqué comme mauvais. Non → première perte du chemin. |

#### Critères PASS/FAIL

| Cas | Verdict | Lot à lancer | Preuve |
|-----|---------|---|---|
| Chemin mort remplacé (0 paquets ancien avant bascule) | **PASS I9b** | Lot C : confirmer ordre probe déterministe | tcpdump montre changement d'adresse, pas sélection |
| Sélection défaillante (RTT ancien meilleur, mais bascule quand même) | **FAIL I9b** | Lot C + tuning hystérésis | paths_by_peer RTT avant/après |
| Ordre aléatoire cause bad-first-pick (nouveau candidat probé en premier alors que vieil était bon) | **BLOCAGE Lot C** | Trier candidats avant probe | same tcpdump analyse |

#### Pièges d'interprétation

- **Dégradation RTT ≠ dégradation réelle** : un chemin peut afficher un RTT élevé au probe (première interaction réseau) mais se stabiliser après. Observer ≥ 10 paquets avant de conclure.
- **Le changement réseau n'est jamais local** : si la bascule est v4→v6 et les deux existent, la question n'est pas « le v6 est entré », mais « pourquoi le v4 a disparu ET qui l'a remplacé en premier ». IPv6 privacy ext (RFC 4941) peuvent temporairement interrompre une adresse.

---

### 2.3 Scénario C1 : Déterminisme du probe — tri des candidats (Lot C)

**Objet** : valider que les candidats sont probés dans un ordre déterministe (GUA v6 d'abord, sinon v4), non aléatoire.

**Durée** : 15 min  
**Préconditions** : Lot B résolu (cause de dégradation identifiée).

#### Setup préalable

Instrumenter le code : ajouter un tracing/log au probe
```rust
// tom-quinn-proto/src/iroh_hp.rs:152 approx
tracing::info!(
  "probing candidates in order: {:?}",
  sorted_candidates.iter().map(|(addr, _)| format!("{:?}", addr.family())).collect::<Vec<_>>()
);
```

#### Étapes

1. **Compiler avec instrumentation**
   ```bash
   RUST_LOG=iroh_hp=debug cargo build -p tom-tui --release
   ```

2. **Lancer une connexion neuve entre deux nœuds** (forcer un re-dial)
   ```bash
   # Kill connexion existante (via /peers/drop sur Mac)
   # [ENDPOINT À CRÉER — Lot A] curl -s "http://[::1]:9091/peers?action=drop&peer=IPHONE_ID" 2>/dev/null
   # Attendre 2 s
   sleep 2
   # Relancer app ou trigger un send
   curl -s "http://192.168.0.83:9300/send?to=IPHONE_ID&size=1024&count=1"
   ```

3. **Capturer les logs de probe**
   ```bash
   # Depuis tom-tui ou l'app Mac (si possibilité de redirection stderr)
   # Ou via collecteur si on ajoute le tracing event (type "PROBE_ORDER", champ "candidates_order")
   ```

4. **Répéter 5× avec reset de connexion** pour valider la stabilité de l'ordre

#### Critère PASS/FAIL

| Cas | Verdict | Preuve |
|-----|---------|---|
| Tous les dials utilisent le même ordre : GUA v6 puis v4 (ou fallback) | **PASS I9b** | Logs "candidates in order: [v6, v4, relay]" identiques sur 5 runs |
| Ordre change aléatoirement | **FAIL I9b, bloquer Lot C** | Logs varient : [v4, v6, relay] puis [v6, v4, relay] |
| Ordre v4 d'abord, jamais v6 probé | **FAIL I9b lite** (mais explique la baseline v4-only sur iPhone) | Logs montrent "candidates in order: [v4, relay]" |

---

### 2.4 Scénario D1 : Charge et dégradation réseau simulée (étage F avancé, post-Lot C)

**Objet** : sous charge, vérifier que la convergence n'est pas cassée.

**Durée** : 5 min par profil  
**Préconditions** : Lot A/B/C résolus.

#### Profils de charge

1. **Latence + perte** : sur Mac avec `dnctl` (Network Link Conditioner)
   ```bash
   # Profil LTE dégradé : +50 ms latence, 5% perte
   sudo dnctl pipe 1 config delay 50 loss 5
   sudo pfctl -ef - < <(echo "dummynet-anchor "dummynet-anchor" all in/out")
   # Mesurer les chemins
   python3 scripts/path-matrix.py --n 3 --interval 30
   ```

2. **Coupure d'interface réseau** (NAS via SSH)
   ```bash
   ssh root@192.168.0.83 "ifconfig eth0 down"  # attendre 30 s
   ssh root@192.168.0.83 "ifconfig eth0 up"
   # Observer re-convergence < 15 s (I3)
   ```

#### Critères PASS/FAIL

| Condition | Invariant | Preuve |
|-----------|-----------|--------|
| Sous latence +50 ms, les chemins restent stables (bascules ≤ 1 en 90 s) | **I9a** | path-matrix diff output |
| Après coupure de 30 s, re-convergence < 15 s | **I3** | collecteur timestamps |

---

## 3. R15-lite — Relais habituel : scénarios

### 3.1 Scénario I1 : Persistance du relais préféré (gain connexion)

**Objet** : un nœud redémarré se reconnecte via le relais mémorisé, gain ≥ 2× sur la latence.

**Durée** : 10 min  
**Préconditions** : cache de relais codé ; flotte a une adresse relay_url connue pour ≥ un pair.

#### Étapes

1. **Baseline** : mesurer temps connexion FROID (sans cache)
   ```bash
   # Isole le nœud avec DHT off (décision Malik : --isolated ou équivalent)
   ssh ipad "pkill -9 tom-node"  # tuer proprement via API si possible
   rm -f /private/var/mobile/Containers/Data/Application/*/Library/Private\ Documents/state.db
   # Redémarrer l'app avec DHT off
   devicectl device launch --wait-for-quiescence com.example.TomNode
   # Mesurer temps jusqu'à « pair connecté » (collecteur ou status :9091)
   time_cold=$(date +%s)
   until curl -s "http://192.168.0.23:9091/" | jq '.pairs_connectes | length' | grep -q -E '[1-9]'; do
     sleep 1
   done
   time_reached=$(date +%s)
   echo "Temps froid (DHT off) : $((time_reached - time_cold)) s"
   ```

2. **État chaud** : laisser le cache se remplir, puis redémarrer
   ```bash
   # Garder l'app active 2 min (remplissage du cache : au moins une adresse + relais via connexion)
   sleep 120
   # Capturer contenu du cache (state.db)
   sqlite3 state.db "SELECT COUNT(*) FROM peers WHERE preferred_relay_url IS NOT NULL;" 
   # Redémarrer
   ssh ipad "pkill -9 tom-node"
   devicectl device launch com.example.TomNode
   # Mesurer temps avec cache
   time_warm=$(date +%s)
   until curl -s "http://192.168.0.23:9091/" | jq '.pairs_connectes | length' | grep -q -E '[1-9]'; do
     sleep 1
   done
   time_reached=$(date +%s)
   echo "Temps avec cache : $((time_reached - time_warm)) s"
   ```

3. **Gain** : calculer ratio
   ```bash
   gain=$((time_cold / time_warm))
   echo "Gain : ${gain}x plus rapide avec cache"
   ```

#### Critères PASS/FAIL

| Cas | Verdict | Preuve |
|-----|---------|--------|
| Cache chaud connexion ≥ 2× plus rapide que froid (ex: 8 s → 3 s) | **PASS I10** | Timestamps collecteur ou chrono shell |
| Cache n'aide pas (temps identique) ou ralentit | **FAIL I10** | Même proof, ratio ≤ 1 |
| Nœud ne se reconnecte jamais (timeout) | **FAIL I10, connexion cassée** | time_warm > 60 s |

#### RÉSULTATS 2026-07-19 soir (NAS, build 130) — mécanisme PROUVÉ, ratio NON MESURABLE sur ce banc

Deux runs (11 cycles) : run 1 nominal (3 chauds / 3 froids), run 2 avec blocage
iptables des flux UDP NEW entrants sur le port de bind (isolation de l'assistance).

| Condition | t « premier pair » | Journal |
|---|---|---|
| Chaud (state.db intact) | 0,65 / 1,53 / 4,85 s (run 1) · 0,94 / 0,92 s (run 2) | « Restored 7 preferred relay routes » + « R15: 7 relais semés » → **premier path_change à t+0,4 s = RELAY via la route persistée** (span `connect` = sortant), upgrade DIRECT v6 5 ms 20 ms après |
| Froid (state.db écarté) | 0,93 / 0,93 / 0,62 s (run 1) · 1,23 s+ (run 2) | aucun Restored/R15 ; premières connexions **entrantes** (`router.accept`) ou re-dial appris de l'entrant |

**Verdicts :**
1. **Le mécanisme R15-lite est validé terrain** : au restart chaud, la première
   connexion est SORTANTE via la route relais persistée, avant toute découverte.
2. **Le ratio ≥ 2× est non mesurable sur cette flotte LAN** : la baseline froide est
   écrasée par trois canaux d'assistance immédiate — re-dials UDP entrants de la
   flotte, flux conntrack survivants au restart, et surtout le **relais embarqué du
   NAS** (TCP 3340) par lequel la flotte ré-injecte adresses et dials en < 1 s. Ce
   canal est non blocable : c'est aussi la voie R15 du chaud et l'infra de la flotte.
3. **Pour obtenir le chiffre** : il faut un pair qui ne re-diale pas et hors du relais
   du nœud testé — ex. redémarrer le NAS pendant que l'iPhone est en CELLULAIRE
   (app relancée après le restart), ou banc dédié 2 nœuds isolés. I10 reste « à
   chiffrer », mais le FAIL « cache n'aide pas » est exclu par la preuve mécanisme.

---

### 3.2 Scénario I2 : Non-résurrection des fantômes (anti-ravivage + cache)

**Objet** : le cache ne fait PAS ressusciter les pairs morts (empoisonnement topologie 17/07).

**Durée** : 15 min  
**Préconditions** : cache + filtre M2 (anti-ravivage, builds 121+) en place ; `state.db` modifiable.

#### Étapes — Setup pollué

1. **Créer un state.db pollué** (fixture test)
   ```bash
   # Sur le nœud test, injecter des pairs fantômes (>24 h, status Online, avec adresses en cache)
   sqlite3 state.db << 'EOF'
   INSERT OR REPLACE INTO peers 
     (node_id, role, status, last_seen, direct_addrs_json, preferred_relay_url, addr_fail_count)
   VALUES
     ('cafebabe0000111122223333444455667788abcd', 'Peer', 'Online', 1720000000000, 
      '["192.168.0.100:55555"]', 'http://relay.old.com:3340', 0),
     ('deadbeef0000111122223333444455667788dead', 'Peer', 'Online', 1720000001000, 
      '["[2a01:db8::1]:60000"]', 'http://relay.old.com:3340', 1);
   EOF
   ```

2. **Redémarrer avec DHT off** (isolation)
   ```bash
   # Tuer le nœud
   pkill -9 tom-node  # ou via API
   
   # Relancer avec --isolated (ou équivalent : n0_discovery(false) en Rust)
   # (pas d'accès à la découverte : DHT + mDNS + gossip bootstrap tués)
   devicectl device launch com.example.TomNode  # avec flag isolé si existant
   ```

3. **Vérifier que les fantômes ne sont PAS dialés**
   ```bash
   # Attendre 10 s (le cache pourrait tenter les dials)
   sleep 10
   
   # Vérifier taille_reseau (ne croît pas)
   taille_avant=$(curl -s "http://192.168.0.23:9091/" | jq '.taille_reseau')
   sleep 30
   taille_apres=$(curl -s "http://192.168.0.23:9091/" | jq '.taille_reseau')
   
   # Vérifier aucun dial vers les fantômes (collecteur : grep "DIAL to cafebabe…")
   tail -1000 /tmp/tom_collector.log | grep -E "DIAL.*(cafebabe|deadbeef)" | wc -l
   # doit être 0
   ```

4. **Vérifier le filtrage M2** (aucun pair >24 h n'est chargé)
   ```bash
   now_ms=$(date +%s000)
   ttl_24h=86400000  # ms
   sqlite3 state.db << EOF
   SELECT COUNT(*) 
   FROM peers 
   WHERE status='Online' AND last_seen < ($now_ms - $ttl_24h);
   EOF
   # doit renvoyer 0 (le filtre M2 l'a évincé)
   ```

#### Critères PASS/FAIL

| Cas | Verdict | Preuve |
|-----|---------|--------|
| Taille réseau stable (avant/après identiques) | **PASS I11** | `taille_reseau` JSON |
| Aucun dial vers fantôme (collecteur 0 DIAL) | **PASS I11** | grep output count = 0 |
| Pair >24 h pas chargé par M2 (SQL query = 0) | **PASS I11** | SELECT output |
| Taille réseau croît APRÈS restart (fantômes ravivés) | **FAIL I11, anti-ravivage cassé** | JSON taille_reseau augmente |
| DIALs observés vers addrs en cache des fantômes | **FAIL I11, cache court-circuite M2** | grep count > 0 |

---

### 3.3 Scénario I3 : Cycle de vie du cache lors de TTL pair

**Objet** : quand un pair expire (M2 le supprime), ses adresses et relais en cache disparaissent aussi.

**Durée** : 5 min (dans un test à trame lente, 24 h+ pour valider le réel TTL).

#### Étapes

1. **Injecter un pair avec adresse** (fixture, TTL proche du seuil)
   ```bash
   sqlite3 state.db << EOF
   INSERT OR REPLACE INTO peers
     (node_id, role, status, last_seen, direct_addrs_json, preferred_relay_url)
   VALUES
     ('c0ffee00000111122223333444455667788cafe', 'Peer', 'Offline',
      $(( $(date +%s000) - 86000000 )),  -- 23h 56m 40s ago
      '["192.168.1.50:55555"]', 'http://relay.example.com:3340');
   EOF
   ```

2. **Vérifier que le pair est chargé** (mais `Offline`, donc M2 skip à la lecture)
   ```bash
   sqlite3 state.db "SELECT node_id, status, last_seen FROM peers WHERE node_id LIKE 'c0ffee%';"
   # doit renvoyer la ligne
   ```

3. **Simulation du save() — M2 filtrage**
   ```bash
   # Le runtime appelle save() toutes les 30 s
   # S'il y a une fonction `/debug/save` ou équivalent, l'invoquer ; sinon, arrêter/redémarrer le nœud
   # [ENDPOINT À CRÉER] pas de route force_save aujourd'hui. Sur le NAS, la voie
   # sûre est un redémarrage propre du service (le save tourne toutes les 30 s,
   # donc attendre 35 s est aussi valable et moins invasif) :
   ssh root@192.168.0.83 "systemctl restart tom-node"   # service = tom-node, PAS tom-chat
   # ⚠️ après tout restart : relire uptime ET NRestarts avant d'interpréter la suite.
   sleep 2
   ```

4. **Vérifier que le pair >24 h a été évincé**
   ```bash
   sqlite3 state.db "SELECT COUNT(*) FROM peers WHERE node_id LIKE 'c0ffee%';"
   # doit être 0 (M2 l'a supprimé)
   ```

#### Critères PASS/FAIL

| Cas | Verdict | Preuve |
|-----|---------|--------|
| Pair >24 h supprimé, adresses cascade-delete avec lui | **PASS I11** | SQL count = 0 |
| Adresses orphelines restent (sans pair) | **FAIL I11, schema rupture** | SQL count > 0 sur colonne direct_addrs |

---

## 4. Nouvel invariant : I9 (convergence de chemin)

### Définition formelle

**I9a (stabilité)** : sur un chemin entre deux nœuds où coexistent des adresses v4 et v6, après que le chemin soit actif une première fois (premier ACK reçu), la famille v4↔v6 du chemin actif reste stable pour une fenêtre minimale. Tolérance : ≤ 1 bascule par 300 s d'observation.

**I9b (optimalité)** : si une adresse source devient inaccessible et un nouveau candidat est probé, et que ce nouveau candidat offre un RTT ≤ ancien_rtt - 5ms (seuil `RTT_SWITCHING_MIN_IP`), le système utilise préférentiellement le meilleur. Mesure : tcpdump concentre ≥ 95 % des paquets de données sur l'adresse meilleure, dans un délai < 30 s après le probe réussi.

### Seuils justifiés

- **300 s** : fenêtre observable dans un test de 10 min (3 relevés), reflète un nœud "stable" au sens applicatif.
- **5 ms** : correspond à `RTT_SWITCHING_MIN_IP` du code (`tom-connect/remote_state.rs:78`), épargne les bascules sur des différences de bruit (<1 ms).
- **95 % des paquets** : un multipath QUIC peut sonder des chemins de secours (~5 % probes/contrôle). Écarter le bruit de mesure.
- **30 s** : délai maximal de selection après probe, reflète les cycles de mesure RTT (≤ 10 s) + cycle select (≤ 10 s).

### Mesure

- **I9a** : `path-matrix.py diff()` compte bascules v4↔v6 par paire entre snapshots. Fenêtre = N snapshots × intervalle.
- **I9b** : tcpdump sur le chemin actif, compter paquets par adresse source/destination (SrcIP+SrcPort). RTT d'historique via `paths_by_peer`.

---

## 5. Checklist express (avant/pendant/après)

### Avant la série

- [ ] Collecteur UDP :9999 up (`lsof -iUDP:9999`)
- [ ] Aucun process TEST-* en vol (`ps aux | grep TEST`)
- [ ] NAS IP stable ou mise à jour dans `orchestrator.py`
- [ ] `target/` ≤ 18 Go (`du -sh target/`)
- [ ] Flotte ≥ 3 nœuds accessibles (curl -s status HTTP)
- [ ] Binaires à jour (`cargo build --release`, clippy ✓)
- [ ] Script `path-matrix.py` testable : `python3 scripts/path-matrix.py --n 1 2>&1 | head -20`

### Pendant la série

- [ ] Offset collecteur capturé AVANT chaque scénario (`collector_lines()` dans orchestrateur)
- [ ] Pas de notifications background truquées : finir chaque commande avec `EXIT=$?` + relire en shell principal
- [ ] Mesurer avec des NOEUDS RÉELS, pas des proxies (`:9091/metrics` = vérité, pas `phase` ou `taille_reseau`)
- [ ] Silence UDP/collecteur ≠ mort : toujours croiser deux sources
- [ ] Chemins absolus dans les boucles (`#!/usr/bin/bash`, pas `#!/bin/bash` de macOS ; éviter variables PATH)
- [ ] Tuer TOUS process de stress avant prochain scénario (`pkill -f TEST-`, `ps aux | grep tom-stress`)

### Après la série

- [ ] Collecteur re-testé : `tail -5 /tmp/tom_collector.log` affiche du contenu neuf
- [ ] Flotte revenue stable (pas de relais affichant OFFLINE, `phase` = "connecte" sur ≥ 3 nœuds)
- [ ] Aucun résidu : `ps aux | grep -E "[t]om-stress|TEST-"` = 0
- [ ] Rapport JSON généré et archivé (`/tmp/chaos_report_*.json` copié à un endroit stable)
- [ ] **OBLIGATOIRE : croiser la vérité terrain** (connecter manuellement, envoyer un message, confirmer livraison) avant de conclure PASS
- [ ] Nettoyage disque post-test : si `target/` > 18 Go après session, lancer `bash scripts/clean-cruft.sh --apply --builds`

---

## 6. Limitations & mise en garde

### 6.1 Ce que ce protocole NE couvre PAS

| Gap | Raison | Impact |
|-----|--------|--------|
| **Handover réseau réel** (WiFi↔cellulaire sur iPhone) | Nécessite manipulation physique de l'appareil (mode avion ou Network Link Conditioner profil) | R14 observabilité mesurable, mais test du failover nécessite intervention humaine (scénario #12 du banc, semi-auto) |
| **Stress réseau hostile** (latence + perte combinées) | Faisable sur Mac (`dnctl`), pas sur appareil iOS (require jailbreak) | Mesurer la stabilité sous charge injecte un risque opérationnel : une flotte chargée peut montrer des bugs cachés |
| **Endurance 24-48 h** | Long run → fatigue de la plateforme, dérive des horloges | R15 anti-ravivage + R14 converge ne sont validés QUE sur 10 min. Un test de 24 h montrerait drift de clock, exhaustion mémoire, etc. |
| **Tiers contention / multi-utilisateur** | Flotte test est NÔTRE ; pas d'appareils partagés | Un nœud de test peut polluer le rendez-vous DHT (bruit ambiant) mais ne subit pas de contention vrai. |

### 6.2 Pièges d'interprétation (leçons 18/07)

1. **Un snapshot n'est pas une tendance.** La photo au T1 peut être trompeuse (voir R14 §1 où baseline v4-dominant a été contredite par relevés ultérieurs). **Règle** : toujours ≥ 3 relevés espaoés.

2. **Silence du collecteur ≠ absence d'activité.** UDP broadcast peut perdre des paquets sur un réseau chargé. **Règle** : croiser :9091/metrics + collecteur + tcpdump.

3. **Un test isolé ne valide pas la vraie flotte.** Un nœud test en --isolated (DHT off) fait passer des tests qui échouent en réseau réel (la vraie découverte peut révéler des adresses périmées). **Règle** : toujours valider canari sur la vraie flotte AVANT de crier victoire.

4. **Les métriques de proxy (`taille_reseau`, `phase`) ne reflètent pas la santé du protocole.** Un nœud peut afficher `phase: connecte` et `taille_reseau: 200` tout en étant empoisonné (fantômes du 17/07). **Règle** : lire les chemins RÉELS (`paths_by_peer`) + compteurs d'activité.

5. **Un fix qui adresse I9 ne l'adresse pas forcément bien.** Trier les candidats (Lot C proposé) résout l'aléatoire mais peut cacher un bug de sélection sous-jacent. **Règle** : Lot B (élucider la bascule) DOIT précéder Lot C (le fix).

### 6.3 Risques opérationnels

| Risque | Mitigation |
|--------|-----------|
| Polluter le rendez-vous DHT avec des nœuds TEST-* | Utiliser le préfixe TEST- + ignorer en UI (déjà fait, banc-test-chaos.md §11) |
| Crash d'un appareil dû au test (ex: mémoire épuisée) | Mesurer RSS avant/après ; arrêter si > 80 % ; capper les tailles de message en test |
| Une mesure polluée par un réseau chargé (Malik regarde Netflix) | Mesurer uptime/charge avant test ; reporter si ≥ 50 % CPU utilisé hors test |
| État inconsistent à chaud (kill en vol laissant state.db corrompu) | Toujours stop-via-API avant kill ; si kill d'urgence nécessaire, reset complet après |

---

## 7. Qui lance quoi, dans quel ordre

### Pour R14 seul

```bash
# Phase 1 : Observations
bash scripts/path-matrix.py --n 3 --interval 60 --json /tmp/r14-phase1.json
# → I9a lite : bascules < seuil ?
# → Asymétries de sens ?

# Phase 2 : Élucider dégradation (si capturée lors phase 1)
# (orchestre tcpdump + trace device)
# → I9b : chemin mort ou sélection bad ?

# Phase 3 : Si Lot B pointe ordre aléatoire
# (ajouter instrumentation au probe, recompile)
# → I9b : ordre sort stable ?
```

### Pour R15-lite seul

```bash
# Phase 1 : Setup cache
# (app normale 2 min pour remplir cache)

# Phase 2 : Gain mesurable
# (restart froid vs chaud, chrono)
# → I10 : gain ≥ 2× ?

# Phase 3 : Pas de résurrection
# (fixture pollué, restart isolé)
# → I11 : `taille_reseau` stable + 0 DIAL fantôme ?
```

### Pour les deux (R14 + R15-lite ensemble)

```bash
# Phase 1 : Étage L d'abord (assurance qualité, CI-ready)
cargo run -p tom-stress -- invariants --seed 42
# → I1-I8 : tous PASS in-process ?

# Phase 2 : Étage F R14 (observabilité + élucider, si applicable)
python3 scripts/path-matrix.py --n 3 --interval 60

# Phase 3 : Étage F R15-lite (gain + anti-ravivage)
# (mesures locales comme ci-dessus)

# Phase 4 : Étage F orchestrateur existant (régression I1-I8)
python3 scripts/chaos/orchestrator.py --seed 45 --scenarios 1,2,3,7
# → Pas de régression des scénarios de base ?

# Phase 5 : Canari sur un nœud (ex Mac, maint 48 h)
# (pas de changement code, juste valider observabilité de I9 en temps long)

# Phase 6 : Full flotte (ex: 6 nœuds, 24 h continu)
# (après canari PASS)
```

---

## 8. Output attendu : format rapport

### Rapport JSON per scénario

```json
{
  "seed": 42,
  "timestamp": "2026-07-20T15:30:00Z",
  "phase": "R14-A1",
  "results": [
    {
      "scenario": "A1-observabilité",
      "invariant": "I9a",
      "verdict": "PASS",
      "preuve": "Bascules v4↔v6 : 1 en 180 s (seuil 2) — paths.json",
      "severity": "info"
    },
    {
      "scenario": "B1-dégradante",
      "invariant": "I9b",
      "verdict": "FAIL",
      "preuve": "iPad→iPhone v4 9ms → v6 51ms, tcpdump montre 0 paquets sur ancien avant basculer → chemin mort remplacé, sélection a probé v6 en premier (aléatoire)",
      "severity": "major",
      "recommendation": "Lot C : trier candidats avant probe"
    }
  ],
  "summary": {
    "passed": 15,
    "failed": 1,
    "exit": 1
  }
}
```

### Checklist d'interprétation après rapport

- [ ] Chaque FAIL nomme l'invariant violé + la preuve (fichier, ligne collecteur, timing)
- [ ] Aucun PASS ne « semble passer » — vérifier que le critère numérique était réellement satisfait
- [ ] Mesures de vraie flotte priment sur simulées (tcpdump > path-matrix, :9091 > collecteur)
- [ ] Les findings B/C/etc. sont indépendants (fix de B n'invalide pas A)

---

## Appendix : Commandes reference

```bash
# Mesure R14 A1
python3 scripts/path-matrix.py --n 3 --interval 60 --json /tmp/r14.json

# Mesure R15 I1 gain
time_cold=$( (pkill tom-node; rm state.db; launch_app; wait_connected) 2>&1 | date +%s)
# ... 120s pause ...
time_warm=$( (pkill tom-node; launch_app; wait_connected) 2>&1 | date +%s)

# Mesure R15 I2 anti-résurrection
sqlite3 state.db "INSERT INTO peers VALUES ('GHOST', 'Peer', 'Online', 1000, ...)"
# restart --isolated
sqlite3 state.db "SELECT COUNT(*) FROM peers WHERE node_id = 'GHOST';"  # must be 0

# Orchestrateur base
python3 scripts/chaos/orchestrator.py --seed 45 --scenarios 1,2,3,7

# État flotte
for ip in "[::1]" 192.168.0.28 192.168.0.23; do
  curl -s "http://$ip:9091/" | jq '.node_id, .phase, .taille_reseau, .pairs_connectes'
done
```
