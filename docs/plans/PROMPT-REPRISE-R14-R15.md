# Prompt de reprise — roadmap R14 (IPv6 first-class) / R15 (annuaire local)

> Rédigé le 2026-07-18 (nuit) à la fin d'une boucle de 9 chantiers (builds 117→125).
> Copier le bloc ci-dessous comme premier message d'une nouvelle session.

---

CONTEXTE — reprise ToM Protocol. Nouvelle session dédiée à la **roadmap R14 (IPv6
first-class) puis R15 (annuaire local / mémoire des pairs)**. Suite de la boucle
nuit du 18/07 (9 chantiers livrés, builds 117→125, tout poussé gate+CI vertes).

AVANT TOUT — lire ces mémoires (diagnostic DÉJÀ fait, à VÉRIFIER sur pièces, pas refaire) :
- tom-night-loop-2026-07-18 — les 9 chantiers de la nuit (transparence TEST-*, orchestrateur
  étage F, P0-1 binding PeerAnnounce, M1 re-dial, anti-ravivage M1/wart/M2/M3, sanitize username,
  P3, reset cache). Ce qui est livré, ce qui reste, l'état exact de la flotte.
- observability-must-reflect-ground-truth — leçon-clé : ne jamais juger un fix réseau sur des
  proxies (phase/taille_reseau) mais sur connexions/livraisons RÉELLES. R14 mesure des chemins :
  croiser :9091 `paths_by_peer` + collecteur, jamais un seul indicateur.
- design-doc-before-coding-protocol-features — R14/R15 touchent la découverte/dial (proche du
  LOCKED) → doc de conception AVANT de coder, red-team, canari avant flotte.
- validate-resilience-fixes-with-real-stress-test — unit tests insuffisants pour du réseau réel ;
  passer par la flotte (orchestrateur `scripts/chaos/orchestrator.py`) + collecteur.
- nas-dynamic-ip-and-device-diagnostics — IP NAS dynamique (192.168.0.83 ce soir), `ping`/`xctrace`
  mentent ; tester le VRAI service + `devicectl`.
- handoff-complet-protocol — quand Malik demande un prompt de reprise = exécuter TOUT le protocole.
- kill-stress-processes-before-and-after, analyze-logs-myself-every-test, versioning-bump-per-push,
  git-author-email-must-be-gmail, verify-subagent-security-shortcuts.

ÉTAT DU CODE (à date, tout poussé sur origin/main) :
- Dernier commit docs : `4607974`. Dernier build : **125** (`5a4c16a`), déployé sur TOUTE la
  flotte : Mac/iPhone/iPad en 125, NAS `tom-chat` en 125 (md5 82d683f6, /usr/local/bin, control
  :9300). **Apple TV RETIRÉE** ce soir (Malik regarde la télé) — réintégrer si redispo.
- Vault à jour : `vault/30-discoveries.md` (entrée « boucle nuit »), `vault/40-roadmap.md`
  (section R14/R15 lignes ~209-216).
- Working tree : propre côté protocole. Résiduels NON à moi (ne pas committer sans demander) :
  `.claude/settings.json` (modifié hors session), `docs/article-avancement-2026-07-18.md`
  (untracked, rapport rédigé hors session).
- Anti-ravivage bouclé : `relay::TOPOLOGY_TTL_MS=24h`, filtre rejoin 15 s, `Topology::evict_stale`.
  ⚠️ P0-1/M1/anti-ravivage sont Rust-only ; la flotte tourne le XCFramework du 122 (ils
  s'appliqueront au prochain `make ffi-xcframework` + redeploy — défensifs, pas urgents).

RÉFÉRENCES ROADMAP (vault/40-roadmap.md) :
- **R14 — IPv6 first-class** : (1) règle pare-feu Freebox port 43925 (déjà identifiée, ACTION
  MANUELLE utilisateur — pas exécutable en autonomie) + MESURER le DIRECT v6 sur la flotte ;
  (2) publier les GUA IPv6 au rendez-vous DHT, préférence v6 au dial, hole-punch v6 (quasi 100%
  vs NAT v4) ; (3) pinhole automatique via PCP quand la box le permet (zéro friction v6).
  ⚠️ Déjà observé : des liens DIRECT IPv6 téléphone-à-téléphone existent (jalon 17/07, Malik+Laura
  en cellulaire). R14 = généraliser/mesurer/préférer v6, pas partir de zéro.
- **R15 — Annuaire local (mémoire des pairs)** : persister `node_id → relais habituel + dernières
  addrs (LAN/publique/v6) + path_kind` ; dial parallèle (cache + lookup frais) ; expiration douce
  (décision #4, cohérent avec le TTL 24 h anti-ravivage qu'on vient de poser). Gain : reconnexion
  quasi instantanée famille/amis, moins de pression DHT. Zéro config (décision #6). ⚠️ Recouvre
  partiellement l'anti-ravivage (le state.db persiste déjà node_id+status+last_seen) : commencer
  par CARTOGRAPHIER ce qui est déjà persisté avant d'ajouter un annuaire — ne pas dupliquer.

LES TÂCHES (dans l'ordre, design-first) :
1. **R14 étape 1 — MESURE d'abord** : instrumenter/lire `paths_by_peer` (:9091) + collecteur sur
   la flotte 125 pour établir la BASELINE v4/v6 réelle par pair (qui est en DIRECT v6, qui en v4,
   qui en RELAY). Pas de code protocole avant d'avoir la photo. Croiser 2 sources (leçon
   observabilité). La règle pare-feu Freebox 43925 = à demander à Malik (manuel).
2. **R14 étape 2 — doc de conception** : préférence v6 au dial + publication GUA v6 au rendez-vous.
   Red-team (un GUA v6 est routable — surface d'attaque vs v4 NATé). Puis code + canari.
3. **R15 — doc de conception APRÈS avoir cartographié le state.db existant** (ne pas dupliquer
   l'anti-ravivage). Expiration douce alignée sur TOPOLOGY_TTL_MS.

GARDE-FOUS (leçons de la boucle nuit) :
- Design-doc AVANT de coder une feature de découverte/dial ; red-team ; canari avant flotte.
- Gate avant push (`cargo clippy --workspace -- -D warnings` + `cargo test --workspace`), email
  gmail (%ae/%ce), commits FR **sujet minuscule** (commitlint refuse Maj/PascalCase dans le sujet),
  bump `TomVersion.build`, check-ffi.sh si tom-protocol/FFI touché, JAMAIS gate+check-ffi en
  parallèle (flake mDNS). `/review-oracle` (ou 2 sous-agents ciblés) OBLIGATOIRE sur tout hot-path
  réseau — et RELIRE leur code sur pièces (3 faux PASS + 1 bug SQLite attrapés cette nuit ainsi).
- Mesurer sur connexions/livraisons RÉELLES, jamais sur des proxies. Analyser les logs soi-même
  (collecteur `/tmp/tom_collector.py` :9999, :9091, journalctl NAS en UTC).
- `ps aux` avant/après tout test réseau ; tuer les process de stress. target/ ≤ 20 Go.
- Orchestrateur : `scripts/chaos/orchestrator.py` (flotte Mac/iPhone/iPad/NAS, ATV retirée).
- Protocole de handoff complet à chaque demande de prompt de reprise (mémoires + vault + code
  propre + prompt).

COMMENCER PAR : lire tom-night-loop-2026-07-18 + `git log -1` + `vault/40-roadmap.md` (R14/R15) ;
puis R14 étape 1 (baseline v4/v6 mesurée sur la flotte 125) ; puis doc de conception R14.
