# Runbook des tests ToM — routine reproductible (pour tout LLM)

> But : **n'importe quel LLM** doit pouvoir lancer nos campagnes de test sans reconstruire le
> contexte. Deux étages : **L** (in-process, rapide, CI) et **F** (vraie flotte, terrain).
> Rédigé 2026-07-18 après la boucle nuit (builds 117→125). Concept : `banc-test-chaos.md`.
>
> ⚠️ Ce runbook décrit COMMENT lancer. Le POURQUOI (8 invariants durs, 21 scénarios) est dans
> `docs/plans/banc-test-chaos.md`. Les résultats/incidents sont dans `vault/30-discoveries.md`.

---

## 0. Les 8 invariants (ce qu'un test VÉRIFIE — pas un chrono)

Un test PASSE ssi les invariants tiennent, jamais parce que « le RTT est bas » :
- **I1** 0 perte finale (tout envoi finit livré au BON destinataire, backup rejoué au retour).
- **I2** 0 fantôme (received ⊆ sent — jamais un message qu'on n'a pas envoyé).
- **I3** reconvergence bornée (< 15 s après kill).
- **I4** 0 zombie au restart. **I5** 0 gel de boucle. **I6** 0 crash surprise.
- **I7** mémoire bornée (pas d'OOM). **I8** 0 double-livraison (dédup).

---

## 1. Prérequis (À FAIRE AVANT TOUTE CAMPAGNE)

```bash
# a) Le collecteur UDP :9999 doit tourner (horloge Mac = référentiel unique).
/usr/sbin/lsof -iUDP:9999 | grep -i python    # doit renvoyer un PID
# S'il est ABSENT, le relancer :
#   python3 /tmp/tom_collector.py &            # écrit dans /tmp/tom_collector.log
# (le script bind 0.0.0.0:9999, horodate à la RÉCEPTION, append au log)

# b) Aucun process de stress résiduel (contention = faux bugs).
ps aux | grep -E "[t]om-stress|[i]nvariants" ; pgrep -fl "TEST-load"
# → tuer tout résidu AVANT de lancer (kill-stress-processes-before-and-after).

# c) target/ ≤ 20 Go (sinon cargo clean).  du -sh target/
```

**PIÈGES GRAVÉS (ne pas se faire avoir) :**
- Le collecteur `/tmp/tom_collector.log` est **multi-jours SANS date** (heure seule) → filtrer par
  **numéro de ligne (offset)**, JAMAIS par heure (`awk 'NR>OFFSET'`). Capturer l'offset AVANT.
- Les compteurs `:9091` **ne survivent PAS à un SIGKILL** (throttle de persistance) → pour un
  scénario de kill, **mesurer par SEQ au collecteur**, pas par compteur.
- **Silence du collecteur ≠ mort** (pertes UDP broadcast) → toujours croiser `:9091` / `/metrics`.
- Notifications background « completed » **menteuses** → marqueur `EXIT=$?` en dernière ligne +
  relecture dans le shell principal + `ps` avant tout verdict.

---

## 2. Étage L — invariants in-process (rapide, seedé, CI)

Chaos KILL/REVIVE/SKEW sur 5 nœuds à **identités persistantes** (revive = même node_id) +
trafic chat numéroté tracké. Auto-vérifie I1/I2/I3/I5 (I4/I6/I7 = étage F ; I8 = N/A applicatif).

```bash
# Depuis la racine du repo. Reproductible à seed fixe.
cargo run --bin tom-stress -p tom-stress -- invariants --seed 42
```
- Sortie : `[ ok] I1 — ... delivered=13/13 ...` etc. + une ligne JSON récap (`passed`/`failed`).
- **Exécutable près de la vraie flotte SANS pollution** depuis le build 117 : les nœuds portent
  le préfixe `TEST-invariants-N` → la flotte les badge 🧪 et ne les cible jamais (transparence).
- ⚠️ Une assertion qui « passe » sans rien prouver est PIRE qu'un échec : 3 faux PASS attrapés au
  1er jet (I2 « ≥1 livré », I3 artefact de mesure, I5 fin seulement). Toujours relire les
  assertions sur pièces (`scenario_invariants.rs`), pas juste le « PASS ».

---

## 3. Étage F — campagne sur la VRAIE flotte

Orchestrateur : `scripts/chaos/orchestrator.py`. Pilote la flotte via API et vérifie les
invariants sur les traces réelles.

### 3.1 Inventaire flotte + endpoints (⚠️ IP NAS dynamique — vérifier)
| Nœud | Endpoint status | Notes |
|---|---|---|
| NAS | `http://192.168.0.83:8085/status` + **control `:9300`** | IP DHCP, peut changer ; control = `/send?to=&size=&count=`, `/sendall`, `/metrics`, `/peers`, `/stop`, `/reset?level=` |
| Mac | `http://127.0.0.1:9091/` | app locale |
| iPhone | `http://192.168.0.28:9091/` | |
| iPad | `http://192.168.0.23:9091/` | |
| ~~ATV~~ | `http://192.168.0.76:9091/` | retirée 18/07 soir (télé) — réintégrer dans `NODES` si redispo |

Les apps exposent aussi `/reset?level=network\|factory` (DEBUG only) pour réinitialiser sans écran.

### 3.2 Lancer une campagne
```bash
# Scénarios par défaut #1,#2,#3,#7 (sanity / broadcast / message lourd / kill+retour).
python3 scripts/chaos/orchestrator.py --seed 45 --scenarios 1,2,3,7
# Sous-ensemble : --scenarios 1,2   |   un seul : --scenarios 7
```
- Chaque assertion s'imprime `[PASS]/[FAIL] scénario · invariant — preuve`.
- Récap final + `ORCH_EXIT=0` (0 FAIL) ou `1`. Rapport JSON : `/tmp/chaos_report_<ts>.json`.
- **Fenêtre témoin** : l'orchestrateur mesure le bruit ambiant (bot NAS) avant le trafic.

### 3.3 Scénarios couverts (V1)
| # | Nom | Invariants | Cible |
|---|---|---|---|
| 1 | Sanity unicast (NAS→iPhone, 20×1 Ko) | I1, I2, I5 | ambiant témoin |
| 2 | Broadcast `/sendall` | I1 sur chaque pair | tous |
| 3 | Message lourd (ladder 1→8 Mo) | I1 + réassemblage (refus = plafond assumé) | NAS→Mac |
| 7+8 | Kill brutal + retour + backup rejoué | I1 (par SEQ), I3, **I8 dédup**, retour | Mac (kill/relance locaux) |

Scénario #7 : **deux vagues à tailles distinctes** (1 Ko pré-kill / 2 Ko pendant l'absence) pour
lever l'ambiguïté des seq ; mesuré par comptage de seq au collecteur (offset), pas par compteur.

---

## 4. Après la campagne (VÉRITÉ TERRAIN, pas de proxy)

```bash
# Anti-ravivage : la topologie doit rester BORNÉE (pas des centaines qui grossissent).
for ip in 127.0.0.1 192.168.0.28 192.168.0.23; do
  curl -s "http://$ip:9091/" | python3 -c 'import json,sys;d=json.load(sys.stdin);print(d.get("taille_reseau"),d.get("messages_echoues"))'
done
# taille_reseau STABLE = OK (croissance = fantômes non évincés). echoues doit rester bas.
```
- **Ne JAMAIS conclure sur un seul indicateur** (`phase`/`taille_reseau` sont des proxies) :
  croiser connexions/livraisons RÉELLES (`paths_by_peer` :9091 + lignes du collecteur).
- `taille_reseau` élevé mais STABLE + 0 storm de dials = pollution de découverte (rendez-vous DHT),
  PAS une fuite runtime. Un tom-chat de test doit tourner `--isolated` (build 124 : coupe aussi le
  rendez-vous DHT partagé) pour ne pas polluer.

**FIN DE CAMPAGNE** : tuer tout process de test (`pkill -f TEST-`, `pkill -f tom-stress`) +
`ps aux` de contrôle. Ne jamais laisser un nœud de test tourner (il pollue le rendez-vous 24 h).

---

## 5. Durcir le banc (il grandit avec le code — jamais « fini »)

À chaque nouvelle feature, ajouter le scénario qui la stresse ET monter le curseur des voisins :
- nouveaux scénarios orchestrateur (destinataire offline long, kills multiples, réseau adverse
  mobile via Network Link Conditioner — semi-auto guidé) ;
- brancher I4/I6/I7 (zombie/crash/mémoire) à l'étage F ;
- un `FAIL` doit NOMMER l'invariant violé + la trace (fichier collecteur, ligne, timing) —
  jamais « ça a l'air d'aller ».

## 6. Checklist express (copier-coller mental)
1. Collecteur :9999 up ? · 2. `ps` propre ? · 3. offset collecteur capturé ?
4. Étage L (`invariants --seed`) vert ? · 5. Campagne F (orchestrateur) `ORCH_EXIT=0` ?
6. `taille_reseau` stable + `echoues` bas ? · 7. Kill des process de test + `ps` final ?
