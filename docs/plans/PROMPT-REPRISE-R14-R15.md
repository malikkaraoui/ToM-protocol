# Prompt de reprise — R14 (convergence de chemin) puis R15-lite

> Réécrit le 2026-07-19 (matin). Remplace la version du 18/07, **périmée** : elle décrivait R14
> comme « IPv6 first-class » et R15 comme un annuaire d'adresses — les deux ont été redéfinis
> par la mesure. Copier le bloc ci-dessous comme premier message d'une session neuve.

---

CONTEXTE — reprise ToM Protocol. Session dédiée à **R14 « convergence de chemin »** puis
**R15-lite « relais habituel »**, avec sous-agents, review, angle mort, red-team, et un
protocole de test chaos sur appareils réels. Le sujet « pic mémoire / réguler le débit » est
**explicitement différé après ces deux chantiers** (décision Malik du 19/07).

## AVANT TOUT — lire ces mémoires (diagnostic DÉJÀ fait, à VÉRIFIER sur pièces, pas à refaire)

- `tom-path-selection-not-converging` — **le cœur de R14**. Le chantier a été renversé par ses
  propres mesures : le problème n'est PAS le manque d'IPv6.
- `tom-r15-reduced-to-relay-only` — R15 réduit de moitié, et pourquoi.
- `tom-memory-retention-class-of-bug` — la classe de bug à 4 occurrences + **les pièges de
  mesure qui m'ont coûté trois faux diagnostics** (lire absolument avant toute mesure mémoire).
- `tom-backup-store-oom-2026-07-19` — le récit de la découverte de l'OOM.
- `observability-must-reflect-ground-truth` — jamais juger sur des proxies.
- `verify-subagent-security-shortcuts` — **appliqué 4 fois cette session avec succès** : sur
  ~20 findings de sous-agents, une bonne moitié n'a pas survécu à la relecture du code.
- `design-doc-before-coding-protocol-features`, `validate-resilience-fixes-with-real-stress-test`,
  `analyze-logs-myself-every-test`, `kill-stress-processes-before-and-after`,
  `versioning-bump-per-push`, `git-author-email-must-be-gmail`, `nas-dynamic-ip-and-device-diagnostics`,
  `tom-test-runbook`, `handoff-complet-protocol`.

## ÉTAT DU CODE (tout poussé sur origin/main)

- Dernier commit : `4a36f69`. **Build 127**.
- ⚠️ **La flotte Apple tourne encore le build 125.** Les builds 126/127 sont **Rust-only** et
  n'ont été déployés QUE sur le NAS. Pour les propager aux apps : `make ffi-xcframework`
  (ou `scripts/build-tom-protocol-ffi-xcframework.sh`) puis `bash scripts/deploy-apps.sh`
  (nouveauté : `ONLY=<device>` pour n'en viser qu'un — valeurs `ipad appletv iphone iphone-laura`).
- NAS : service **`tom-node`** (PAS `tom-chat` — `journalctl -u tom-chat` renvoie « No entries »),
  binaire `/usr/local/bin/tom-chat`, control `:9300`, status `:8085`, IP **192.168.0.83** (DHCP).
- Working tree propre côté protocole. Résiduels NON à moi, ne pas committer sans demander :
  `.claude/settings.json` (modifié hors session), `docs/article-avancement-2026-07-18.md` (untracked).
- `target/` à 18 Go (plafond 20 Go — surveiller).

## CE QUI A ÉTÉ ÉTABLI (ne pas re-débattre, c'est mesuré et prouvé)

### R14 — le chantier a changé de nature
- **Le DIRECT IPv6 est déjà partout** (RTT v6 ≤ v4). Ajouter « plus d'IPv6 » n'a aucun sens.
- **Le vrai problème : le choix de chemin ne converge pas vers le meilleur lien.** Mesuré sur
  3 relevés espacés : presque chaque chemin change de famille en une heure, et `iPad→iPhone` est
  passé de **v4 9 ms à v6 51 ms** et y est resté.
- **Cause prouvée dans le code** : l'ordre des candidats est **aléatoire** — ni v4-d'abord ni
  parallèle. Le `BTreeSet` trie bien v4<v6 (`tom-base/src/endpoint_addr.rs:44-51`) mais l'ordre
  est **perdu à l'insertion** en `FxHashMap` (`tom-connect/src/socket/remote_map/remote_state/path_state.rs:38`,
  `tom-quinn-proto/src/iroh_hp.rs:91`, boucle de probe L152). ⚠️ `iroh_hp.rs:196` indique qu'un
  autre mécanisme **s'appuie** sur cet aléatoire — ne pas casser sans vérifier.
- Puis l'hystérésis fige le tirage : bascule seulement si `current_rtt >= new_rtt + 5 ms`
  (`RTT_SWITCHING_MIN_IP`), avec 3 ms d'avantage à v6 (`IPV6_RTT_ADVANTAGE`), `remote_state.rs:78-81`.
- **PAS de double tunnel** (tranché par `tcpdump`, 1 274 paquets) : une seule connexion QUIC,
  plusieurs chemins candidats, **un seul actif** pour les données (861 paquets sur un chemin,
  0 sur l'autre, retour symétrique). Donc pas de gaspillage à corriger — QUIC fait déjà
  « choisir + garder un secours ». Ce qui manque est la **RE**-décision.
- **Abandonnés, non justifiés par les mesures** : préférence v6 forcée, happy-eyeballs,
  filtrage des adresses temporaires (les privacy extensions marchent et le /64 commun rend le
  filtrage inutile), règle pare-feu Freebox (Malik refuse de toucher à la box — respecter).

### R15-lite — périmètre réduit
- `peers` en base = 4 colonnes (`storage/schema.rs:66-72`), **zéro adresse**, ni sur disque ni
  en RAM (`PeerInfo`, `relay.rs:66-72`). Backup ADR-009 = **RAM pure**.
- **Persister les adresses directes est ÉCARTÉ** : gain marginal (mDNS répond en 1-2 s sur LAN ;
  hors-LAN ces adresses sont les plus volatiles) contre un risque élevé — ça rouvrirait
  l'empoisonnement de topologie du 17-18/07, avec cette fois des adresses **survivant au restart**.
- **Retenu : `preferred_relay_url` seul**, déjà auto-appris en RAM
  (`tom-transport/src/protocol.rs:250-257`, log « Auto-learned relay route »), expirant avec le
  pair via M2.
- `PathEvent` **est authentifié** : `remote` vient de `connection.remote_id()`
  (`tom-transport/src/protocol.rs:241`), donc du handshake QUIC/TLS où le NodeId est la clé
  publique Ed25519. Un red-team a prétendu le contraire « par hypothèse » — c'est faux.

### Bug à corriger en préalable de R15-lite
`storage/mod.rs:463-475` — le filtre d'élagage 24 h est gardé par `status != Online`, or le load
**préserve** `Online` : un pair fantôme sauvegardé `Online` est rechargé quel que soit son âge.
Borné à ~30 s en régime normal (au save suivant il passe Offline), **non borné en crash-loop**.
Le commentaire au-dessus décrit l'intention, pas le code.

## LES TÂCHES (dans l'ordre)

1. **R14 Lot A — observabilité des bascules.** Bonne nouvelle : `AddrFamily` existe déjà
   (`tom-transport/src/protocol.rs:379,421-440`) et un `PathEvent` est justement émis sur
   changement de famille (L401-410). Reste à **historiser les bascules** (compteur + motif :
   chemin mort ? meilleur RTT ? nouvelle adresse ?) et exposer la famille dans `paths_by_peer`.
   Outil déjà livré : `scripts/path-matrix.py` (N relevés, bascules, asymétries de sens).
2. **R14 Lot B — élucider une bascule dégradante EN DIRECT** (le cas v4 9 ms → v6 51 ms) :
   chemin mort remplacé, ou sélection défaillante ? **AUCUN code de sélection avant cette
   réponse.** C'est la discipline qui a évité d'écrire le mauvais correctif jusqu'ici.
3. **R14 Lot C — déterminisme du probe**, conditionné au Lot B (trier les candidats avant probe
   plutôt qu'ajouter une préférence par-dessus le hasard).
4. **Fix préalable** : l'élagage contourné par un `Online` persisté (ci-dessus).
5. **R15-lite** : `preferred_relay_url`, avec le test de non-résurrection des fantômes.

## PROTOCOLE DE TEST

`docs/plans/RUNBOOK-TESTS.md` (routine générale) + `docs/plans/protocole-test-r14-r15.md`
(scénarios chaos spécifiques à ces deux chantiers, sur appareils réels).
Orchestrateur : `scripts/chaos/orchestrator.py`. Mesure des chemins : `scripts/path-matrix.py`.

## GARDE-FOUS (durement acquis — les ignorer coûte des heures)

**Mesure :**
- **Jamais qualifier un régime réseau sur UN relevé.** Les chemins changent de famille en une
  heure ; ma « baseline » initiale était une photo sur-interprétée, et fausse.
- **`MemoryCurrent` de la cgroup inclut le page cache** → seul `RssAnon` (`/proc/PID/status`)
  gouverne l'OOM. Il exagérait de 2 à 5×.
- **Vérifier qu'un compteur d'activité BOUGE** avant d'interpréter : un envoi vers un pair
  absent de la topologie n'est ni tenté ni compté (`envoyes` restait plat pendant que je croyais
  pousser 240 Mo).
- **Lire `uptime` et `NRestarts` à CHAQUE mesure** : une mémoire qui « redescend » toute seule
  est d'abord suspecte d'un redémarrage. `systemctl restart` remet `NRestarts` à zéro → croiser
  avec `ExecMainStartTimestamp` et `dmesg | grep 'Killed process'`.
- **Mesurer sur un nœud dégradé fabrique une fausse théorie** (une capture a été polluée par un
  NAS étouffé, et suggérait un « double tunnel » inexistant).
- Collecteur : fenêtres par **offset de ligne**, jamais par heure.
- Le status du Mac écoute en **IPv6 uniquement** : `curl 127.0.0.1:9091` échoue, `[::1]:9091` marche.

**Livraison :**
- Gate avant push : `cargo clippy --workspace -- -D warnings` + `cargo test --workspace`
  (⚠️ `--all-targets` casse sur un bench manquant préexistant de tom-quinn, ce n'est pas toi).
- `bash scripts/check-ffi.sh` si tom-protocol/FFI touché. Jamais gate + check-ffi en parallèle.
- Bump `TomVersion.build`, email gmail (`%ae`), commits FR **sujet en minuscules**.
- **Relire le code des sous-agents sur pièces.** Cette session : ~20 findings, une bonne moitié
  réfutée — dont un « BLOQUANT » qui reposait sur une erreur de calcul (`0 == 1` jugé vrai) et un
  autre qui avouait « jamais lu, donc hypothèse ». À l'inverse, mes propres vérifications ont
  trouvé deux vrais bugs que les agents avaient manqués.
- `ps aux` avant/après tout test réseau. `target/` ≤ 20 Go.

## ÉTAT DE LA FLOTTE À LA REMISE

NAS build 127, **RssAnon 125 Mo après 78 min, `NRestarts=0`** (contre 688 Mo + OOM le matin).
Mac/iPad/Apple TV en build 125. iPhone de Malik absent (parti avec lui). iPhone Laura variable.
Aucun process de stress résiduel.

COMMENCER PAR : lire `tom-path-selection-not-converging` + `git log -1` + le §2.4 de
`docs/plans/r14-ipv6-first-class.md` ; puis R14 Lot A (observabilité des bascules), en suivant
`docs/plans/protocole-test-r14-r15.md` pour la validation terrain.
