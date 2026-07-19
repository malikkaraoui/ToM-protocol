# Prompt de reprise — Valider R15-lite (I10) + coder R14 Lot C (re-sondage transport)

> Écrit le 2026-07-19 (soir). Copier le bloc ci-dessous comme premier message d'une
> session neuve. **Deux chantiers DISTINCTS** (souvent confondus) — voir « Ce que sont
> vraiment les deux chantiers » plus bas.

---

CONTEXTE — reprise ToM Protocol. Deux chantiers, dans cet ordre :
1. **Valider R15-lite en terrain** (mesure I10 — gain de reconnexion). R15-lite est DÉJÀ
   LIVRÉ et en prod (build 130) ; il ne reste qu'à MESURER son gain. Zéro code transport.
2. **Coder R14 Lot C** (re-sondage des chemins morts) dans le TRANSPORT QUIC forké. Le
   design est écrit ET révisé par un red-team ; il faut le feu vert de Malik sur le design
   AVANT d'écrire une ligne (c'est la zone de nos deadlocks historiques).

## AVANT TOUT — lire ces mémoires (dans l'ordre)
- `session-handoff-2026-07-19-apresmidi` — le récit complet de la session qui a livré 128→130.
- `tom-lotb-verdict-failover-sans-resondage` — POURQUOI le Lot C existe (le mécanisme exact).
- `relaymode-disabled-pas-de-transport-relais` — piège CI qui a coûté 3 rounds sur R15-lite.
- `ios-power-save-churn-assume` — décision produit : on ne se bat PAS contre le power save iOS.
- `tom-r15-reduced-to-relay-only`, `tom-path-selection-not-converging` — l'historique du chantier.
- `tom-memory-retention-class-of-bug` — les pièges de MESURE (RssAnon, compteur qui bouge…),
  à relire AVANT toute mesure I10.
- `validate-resilience-fixes-with-real-stress-test`, `analyze-logs-myself-every-test`,
  `kill-stress-processes-before-and-after`, `design-doc-before-coding-protocol-features`,
  `versioning-bump-per-push`, `git-author-email-must-be-gmail`, `verify-subagent-security-shortcuts`.

## ÉTAT DU CODE (tout poussé sur origin/main, CI verte)
- Dernier commit : `e3c5f47`. **Build 130.** Flotte alignée en 130 (le durcissement 130
  a d'abord été déployé NAS-only puis propagé aux apps Apple — leçon : un fix Rust-only
  n'atteint les apps qu'après `make ffi-xcframework` + `deploy-apps.sh` + `macbuild`).
- NAS : service **`tom-node`** (PAS tom-chat), binaire `/usr/local/bin/tom-chat`, IP DHCP
  **192.168.0.83**, status `:8085`, control `:9300`. **Persistance ACTIVE** :
  drop-in systemd `/etc/systemd/system/tom-node.service.d/datadir.conf` ajoute
  `--data-dir /root/tom-data` (state.db réel, routes relais persistées). Retirer ce
  fichier pour revenir à l'éphémère.
- Status Mac : IPv6 SEUL → `http://[::1]:9091/` (jamais 127.0.0.1).
- Flotte dispo : iPhone Malik (WiFi), Freebox/NAS, MacBook Pro, iPad. Apple TV NON dispo.
- Working tree propre côté protocole. Résiduels NON à moi : `.claude/settings.json` (hors
  session), `docs/article-avancement-2026-07-18.md` (untracked). Ne pas committer sans demander.
- Un logger passif tourne peut-être encore (`/tmp/lotb-paths.jsonl`, snapshot/60 s) — le
  tuer s'il traîne (`pkill -f lotb-logger`).

## CE QUI EST ÉTABLI (ne pas re-débattre)
- **R14 Lot A LIVRÉ** : `paths_by_peer` expose `family/switches/last_switch/last_switch_at_ms`.
  Outil `scripts/path-matrix.py` (compteurs vue-nœud, bascules entre relevés).
- **R15-lite LIVRÉ** : `preferred_relay_url` (schéma V5), appris des PathEvent RELAY
  authentifiés, non-résurrection par construction (load M2-filtré), semis du pool au
  démarrage. ⚠️ **RelayMode::Disabled n'installe pas le transport relais** : un nœud sans
  relais configuré ne peut pas dialer via relais (vrai partout en prod, les apps/NAS ont
  toujours un relais). Test : `crates/tom-integration-tests/tests/r15_relay_cache.rs`.
- **Lot B TRANCHÉ (mesuré)** : la non-convergence n'est PAS une sélection défaillante. Ce
  sont des failovers (mort de chemin) + le chemin perdu n'est plus jamais re-sondé. Le fix
  est le Lot C.
- **Power save iOS = assumé** (décision Malik). Ne PAS proposer de keepalive/anti-power-save
  ni « élucider pourquoi les chemins v6 iOS meurent ». Le Lot C vise les nœuds ACTIFS.

## LES DEUX CHANTIERS (dans l'ordre)

### Chantier 1 — Valider R15-lite (mesure I10, terrain, PAS de code)
Objectif : prouver le gain de R15-lite = un nœud redémarré rejoint un pair connu via le
relais mémorisé **≥ 2× plus vite** qu'une redécouverte froide. Protocole détaillé :
`docs/plans/protocole-test-r14-r15.md` §3.1 (scénario I1) + §3.2 (I11 non-résurrection avec
`state.db` pollué en fixture).
- Le NAS persiste déjà (`--data-dir`) → candidat naturel. Mesurer : arrêt propre → restart
  FROID (state.db vidé, DHT à froid) vs restart CHAUD (route en cache) → chrono jusqu'à
  « pair connecté ».
- ⚠️ Pièges de mesure (mémoire `tom-memory-retention-class-of-bug`) : vérifier qu'un
  compteur d'activité BOUGE, lire `NRestarts`/`uptime` à chaque relevé, ne jamais conclure
  sur un relevé unique.
- Bonus terrain déjà prouvé : cycle complet observé sur le NAS (« Restored 1 preferred
  relay routes » → « R15: 1 relais semés »). Il manque juste le CHIFFRE du gain.

### Chantier 2 — Coder R14 Lot C (transport QUIC forké — FEU VERT REQUIS)
Design RÉVISÉ par red-team : `docs/plans/r14-lot-c-resondage.md` (lire §2bis corrections +
§3 proposition révisée). Résumé :
- **Déclencheur** = `PathEvent::Abandoned` d'un chemin qui était ACTIF (`remote_state.rs:958`,
  `connection/mod.rs:750`), PAS une comparaison `new_rtt > old_rtt` (tautologique).
- **Action** = re-probe via `open_path(addr_mort)` (`remote_state.rs:832`, émet un
  PATH_CHALLENGE — brique existante, PAS de sonde neuve), câblé sur le pattern de backoff
  tokio du holepunch (`scheduled_holepunch`, `remote_state.rs:280`). Backoff 30s→5min, 6
  essais, abandon si le chemin revit.
- **La sélection existante décide** au retour du RTT (`select_v4_v6`, `remote_state.rs:1160`)
  — AUCUN nouveau seuil.
- **Garde-fous obligatoires** (attaques red-team confirmées) : cooldown post-failover
  (ne pas re-basculer vers une adresse quittée < 30 s sans gain > 10 ms) + ≤ 1 re-probe en
  vol par adresse-cible (anti-amplification). Ce sont des FADE réversibles (LOCKED #4 OK),
  pas des bans.
- Candidats morts déjà horodatés `Inactive(Instant)` (`path_state.rs:51`), bornés par
  `MAX_INACTIVE_IP_PATHS=10` + `prune_ip_paths` (`path_state.rs:244`).
- ⚠️ `iroh_hp.rs:196` : un mécanisme s'appuie sur l'aléatoire d'itération — ne pas casser
  (vérifier `continue_nat_traversal_round` avant de toucher l'ordre).

**Validation exigée** (avant de déclarer le Lot C fait) :
- Étage L hermétique (modèle r15_relay_cache, bind loopback + asserts filtrés) : tuer
  artificiellement un chemin, vérifier failover PUIS retour ≤ 60 s quand il revit.
- Étage F : rejouer Mac↔iPad avec `path-matrix.py` + logger : I9b (retour au meilleur ≤ 60 s),
  I9a (pas d'oscillation accrue). ⚠️ Séparer les mobiles inactifs (power save) de la flotte
  active — sinon on mesure Apple, pas le protocole.

## GARDE-FOUS (durement acquis)
- Gate avant push : `cargo clippy --workspace -- -D warnings` + `cargo test --workspace`.
  `bash scripts/check-ffi.sh` si tom-protocol/FFI touché (le FFI est HORS workspace).
- Le hook commit-msg (commitlint) : sujet ≤ ~72 car, type conventionnel (`feat/fix/docs/
  test/refactor…` — PAS `hardening`), français, PAS de signature, minuscules.
- Bump `TomVersion.build` à chaque push. Email gmail (`git config user.email`).
- Déploiement : `make ffi-xcframework` puis `bash scripts/deploy-apps.sh` (`ONLY=<device>`
  pour un seul : `ipad appletv iphone iphone-laura`). NAS : `cargo zigbuild -p tom-tui
  --target aarch64-unknown-linux-musl --release` puis `bash scripts/deploy-nas-node.sh`.
  App Mac : `make macbuild` DANS `apps/tom-node-tvos/` puis vérifier `BUILD SUCCEEDED`
  (le Makefile imprime « ✅ done » même en échec).
- **Relire le code des sous-agents sur pièces** : ce red-team a rendu 3 findings « bloquants »
  tous RÉFUTÉS à la relecture. La moitié ne survit jamais. Vérifier avant de corriger.
- `ps aux` avant/après tout test réseau. `target/` ≤ 20 Go (`du -sh target/`, `cargo clean`
  si besoin — il regonfle vite).
- `/review-oracle` (4 agents) avant chaque push de code (hook bloque le push sinon).

COMMENCER PAR : lire `session-handoff-2026-07-19-apresmidi` + `tom-lotb-verdict-failover-sans-resondage`
+ `git log -1`, PUIS proposer à Malik : (1) mesurer I10 tout de suite (rapide, terrain), et
(2) obtenir son feu vert sur le design Lot C (`r14-lot-c-resondage.md` §2bis/§3) avant de
toucher au transport QUIC forké.
