# Journal ToM Protocol

> Registre append-only (MISSION.md §4.3). Une entrée par session/chantier. Ne jamais réécrire une entrée passée — corriger par une nouvelle entrée si besoin.

---

## [2026-07-02 | claude-sonnet-5] — Reprise mission + fermeture des bugs verrous #1/#2 + failover hub réel + hijack + CI + dalek

### Objectif (lien vers §1)
Reprendre `docs/MISSION.md` (cible finale §1) après l'audit `docs/audits/AUDIT-2026-06-26.md` (mergé sur `main` ce matin via `fb2a04c`). Premier pas recommandé par la mission (§7) : fermer les bugs de verrous #1/#2, puis le failover hub réel, puis anti-squat DHT + couverture CI. Chantiers visés : verrou #1 (livraison ⟺ ACK signé), verrou #2 (purge TTL 24h), résilience « virus/indébranchable » (failover hub), qualité (CI/coverage).

### Ce que j'ai fait (décisions + pourquoi)

1. **Vérité terrain avant tout** : au lieu de faire confiance à l'audit tel quel, j'ai relu chaque bug cité `fichier:ligne` dans le code actuel de `main` (HEAD `8458184`) pour confirmer qu'il est toujours présent (l'audit datait du 2026-06-26, `main` avait bougé depuis). Les 5 bugs critiques étaient bien tous encore présents.

2. **Verrou #1 — ACK entrant non vérifié** (`runtime/state.rs`, arm `RoutingAction::Ack` dans `handle_incoming_chat`) : l'arm ignorait totalement le paramètre `signature_valid` déjà calculé par l'appelant — un ACK non signé ou falsifié promouvait `mark_relayed`/`mark_delivered` sans aucune vérification. Fix : gate sur `signature_valid`, sinon `MessageRejected`. **Pourquoi via `signature_valid` et pas une re-vérification locale** : le flag est déjà calculé correctement en amont (`envelope.verify_signature()` sur l'enveloppe ACK complète) — pas besoin de dupliquer.
   - Test adversarial ajouté : `forged_ack_rejected_no_status_change` (ACK non signé → aucun `StatusChange`, `MessageRejected` émis).
   - Test existant `ack_updates_tracker_status` corrigé : il utilisait délibérément un ACK relais **non signé** en argumentant "that's fine, sig_valid=false" — ce commentaire documentait la vulnérabilité elle-même. Signé désormais, comme le fait le code de production (`ack.sign()` à l'émission, `state.rs:818,829,854`).

3. **Verrou #2 — purge SQLite hub jamais déclenchée** (`state.rs::tick_hub_cleanup`) : `store.cleanup_hub_messages(TTL_MS)` passait la **durée** (86 400 000 ms) comme cutoff **absolu**, alors que `cleanup_hub_messages` fait `DELETE ... WHERE stored_at < cutoff`. Avec `stored_at` en epoch-ms (~1.7×10¹²), la condition `< 86 400 000` n'était quasiment jamais vraie → la table ne se purgeait jamais. Fix trivial : `now.saturating_sub(TTL_MS)`.
   - Test ajouté : `tick_hub_cleanup_purges_expired_sqlite_rows` — insère une ligne à 25h et une à 1min, appelle `tick_hub_cleanup()`, vérifie que seule la ligne fraîche survit. C'est un test qui exerce la vraie DB SQLite (`StateStore::open_memory()`) au niveau `RuntimeState`, pas juste `cleanup_hub_messages()` isolément (le test isolé existant `hub_message_cleanup_expired` passait déjà — il ne testait pas l'appelant fautif, d'où le bug invisible).

4. **Failover hub mort au runtime** (`group/manager.rs`) : `record_ping_failure` n'était jamais appelé hors tests. Le watchdog (`tick_shadow_ping`, déjà branché sur un timer 3s) envoie bien un `HubPing` périodique et réinitialise les échecs sur `HubPong` (`reset_ping_failures`), mais **rien ne détectait l'absence de pong** — un hub qui devient silencieux (mort) n'était jamais compté comme un échec.
   - Ajout `ShadowState.last_ping_sent_at: Option<u64>` + `GroupManager::note_ping_sent()` + `GroupManager::check_ping_timeouts(now)` (appelé à chaque tick, avant l'envoi du ping suivant : si un ping précédent n'a pas eu de pong dans `SHADOW_PING_TIMEOUT_MS`, compte comme échec via `record_ping_failure`).
   - `tick_shadow_ping` (state.rs) câblé pour appeler `check_ping_timeouts` puis `note_ping_sent` à chaque envoi.
   - Tests : `check_ping_timeouts_promotes_on_silent_hub` (2 cycles de ping sans pong → auto-promotion), `check_ping_timeouts_cleared_by_pong` (un pong efface le timer, pas de fausse alerte).

5. **Hub hijack / split-brain** (`group/manager.rs::handle_hub_migration` + `state.rs::handle_incoming_group`) : la fonction acceptait n'importe quel `new_hub_id` sans vérifier qui l'envoyait. Fix en deux parties (aucune des deux seule n'est suffisante) :
   - **Partie 1 (manager.rs)** : `from != new_hub_id` rejeté — un membre ne peut annoncer QUE sa propre promotion, jamais rediriger vers un tiers. *Nuance découverte en cours de route* : j'avais d'abord ajouté un check `group.shadow_id == Some(new_hub_id)`, mais les membres ordinaires (non-hub, non-shadow, non-candidat) **n'apprennent jamais** qui est le shadow dans la conception actuelle (`assign_shadow` ne notifie que le shadow et le candidat, jamais les membres) — ce check aurait rejeté TOUTE migration légitime pour un membre normal. Retiré, documenté en commentaire.
   - **Partie 2 (state.rs)** : `signature_valid` calculé **avant** `decrypt_payload` (la mutation invaliderait la signature côte encrypt-then-sign) et l'arm `HubMigration` rejette si non signé.
   - Tests : `handle_hub_migration_rejects_sender_mismatch` (manager.rs), 3 sites d'intégration (`group_integration.rs`) mis à jour pour passer le nouvel argument `from`.
   - **Limite résiduelle assumée** (pas un bug caché, une limite de conception documentée) : un membre malveillant qui n'a JAMAIS été shadow peut toujours s'auto-déclarer hub — les membres ordinaires n'ont aucun moyen de vérifier qu'il l'était réellement, faute de notification de l'assignation du shadow à tous les membres. Fermer complètement ce trou demande un changement de conception plus large (diffuser l'assignation shadow signée à tous les membres), hors scope d'un fix de bug.

6. **`pre-push-gate.sh` ignorait Rust** : `detect_stack()` retournait le **premier** stack trouvé (`package.json` existe à la racine du repo pour le legacy TS) et s'arrêtait là — Rust n'était JAMAIS testé par la gate locale, quel que soit le diff. Fix : `detect_stacks()` renvoie **tous** les stacks présents, lint/build/test bouclent sur chacun. Ajout des commandes `cargo clippy/build/test --workspace` pour le stack `rust`.

7. **CI (`ci.yml`)** : `tom-connect` était build+clippy mais jamais testé (~78 tests morts). `tom-dht` totalement absent de CI (20 tests, dont chaos/blackout réels). `tom-integration-tests` jamais lancé (multi-nœuds réels, dont un test `stability_2min`). Ajout : step test pour tom-connect dans `rust-fork`, nouveau job `rust-dht`, nouveau job `rust-integration` (`timeout-minutes: 10`, mesuré en local à ~2-3 min réel).

8. **Double-version `ed25519-dalek`/`curve25519-dalek`** : `tom-protocol/Cargo.toml` déclarait `ed25519-dalek = "2"` / `curve25519-dalek = "4"` au lieu du pin `=3.0.0-pre.1` / `=5.0.0-pre.1` utilisé par `tom-base`/`tom-connect`/`tom-gossip`. Changement de version seule (aucun changement de code requis — l'API `SigningKey::from_bytes`/`VerifyingKey::from_bytes`/`Signature::from_bytes`/`Signer` est identique entre 2.x et 3.0.0-pre.1). `ed25519-dalek` est désormais **unifié** sur une seule version dans tout le workspace (`cargo tree -i ed25519-dalek` confirme une seule entrée).
   - **Limite honnête** : `curve25519-dalek` reste dupliqué (`4.1.3` **et** `5.0.0-pre.1` coexistent) — mais la 4.1.3 vient de `x25519-dalek "2"` (dépendance transitive figée en amont sur curve25519-dalek 4.x), pas de code à moi. Je ne peux pas forcer ça sans remplacer x25519-dalek entièrement (portée bien plus large, hors scope). `ed25519-dalek` (les clés d'identité, le point le plus sensible relevé par l'audit) est bien unifié ; `curve25519-dalek` reste un residual gap à noter pour la suite.

### Fichiers touchés
- `crates/tom-protocol/src/runtime/state.rs` — gate ACK (verrou #1), cutoff purge (verrou #2), `tick_shadow_ping` (câblage timeout), `handle_incoming_group` (signature_valid pré-decrypt + gate HubMigration), 3 nouveaux tests + 1 test corrigé.
- `crates/tom-protocol/src/group/manager.rs` — `ShadowState.last_ping_sent_at`, `note_ping_sent`, `check_ping_timeouts`, `handle_hub_migration` (+ `from` param, check sender), 4 tests (2 nouveaux, 2 modifiés).
- `crates/tom-protocol/tests/group_integration.rs` — 3 call sites `handle_hub_migration` mis à jour.
- `crates/tom-protocol/Cargo.toml` — pin dalek unifié.
- `crates/tom-dht/Cargo.toml`, `crates/tom-dht/src/lib.rs` — anti-squat DHT (voir entrée séparée ci-dessous, même session).
- `scripts/pre-push-gate.sh` — détection multi-stack.
- `.github/workflows/ci.yml` — job `rust-dht`, job `rust-integration`, step test tom-connect.

### Résultats de tests (chiffres réels)
- `cargo test -p tom-protocol --lib` : **534 passed, 0 failed** (531 avant + 3 nouveaux nets après quelques ajouts/suppressions).
- `cargo test -p tom-protocol` (lib + tests d'intégration + proptest crypto) : tout vert (10 + 4 + 4 tests d'intégration, doctest ignoré comme avant).
- `cargo test -p tom-dht --lib` : **21 passed, 0 failed** (20 avant + 1 nouveau test PoP).
- `cargo test -p tom-integration-tests` : **6 passed, 1 ignored, 0 failed** (dont `stability_2min`, ~121s réel).
- `cargo build --workspace` : vert.
- `cargo clippy --workspace -- -D warnings` : vert.
- `cargo test --workspace` : **1 échec pré-existant, non lié à mon diff** — voir Auto-critique.

### Ce qui reste / prochain [→]
- [→] **DHT anti-squat / augmentation du nombre de slots** : PoP portée dans `tom-dht` (voir entrée séparée), mais le nombre de slots (8) n'a **pas** été augmenté — décision délibérée (impact perf/latence de découverte non mesuré, risque de dégrader l'UX zero-config si mal dimensionné). À reprendre avec un vrai benchmark de latence de découverte avant de choisir un nouveau nombre.
- [ ] Résiduel hub-hijack : diffuser l'assignation shadow signée à tous les membres (fermerait le dernier trou d'auto-déclaration).
- [ ] `curve25519-dalek` reste dupliqué (4.1.3 via x25519-dalek + 5.0.0-pre.1 direct) — pas actionnable sans remplacer x25519-dalek.
- [ ] Formes de livraison §2 (SDK Rust durci, binding WASM, TestFlight) — pas commencé cette session, chantier séparé de grande ampleur.
- [ ] Tests DURS taxonomie complète §4.5 (fuzzing wire format, simulation chaos NAT/CGNAT/partition à grande échelle, coverage mesuré `cargo llvm-cov`) — pas fait cette session.
- [ ] Découverte durant la session : **`crates/tom-transport/src/config.rs` a une modification non committée sur disque** qui remplace `DEFAULT_RELAY_URLS` (liste publique `relay-{eu,us,asia}.tom-protocol.org`) par une IP NAS personnelle unique (`82.67.95.8:3340`) — casse le test `fallback_relays_used_when_discovery_fails` et, si jamais committé, violerait le principe §1.1/§1.8 (aucun point de contrôle unique) en codant en dur le relais personnel de l'auteur comme fallback par défaut pour tous les utilisateurs du protocole. Déjà signalé en mémoire d'une session précédente ("à trier par l'utilisateur"), toujours non résolu. **Je n'ai pas touché ce fichier** — à trancher par l'utilisateur (l'exclure du prochain commit, ou le committer en changeant aussi le test s'il s'agit d'un choix voulu).

### Auto-critique (ce qui est fragile, ce dont je doute)
- Le test `tom-transport::node::tests::fallback_relays_used_when_discovery_fails` échoue en suite complète à cause du fichier non committé ci-dessus — **confirmé pré-existant et sans rapport avec mon diff** via `git stash` (le test passe sur `main` propre en isolation). `cargo test --workspace` ne sera donc PAS vert tant que ce fichier n'est pas réglé par l'utilisateur — ce n'est pas une régression que j'ai introduite, mais je le signale explicitement pour ne pas laisser croire que la gate est 100% verte.
- Le check `handle_hub_migration` (partie 1, manager.rs) protège seulement contre l'usurpation d'un TIERS (`from != new_hub_id`) — il ne protège pas contre l'auto-déclaration frauduleuse par un membre légitime mais malveillant (limite documentée ci-dessus). Ce n'est PAS une fausse sécurité : combiné à la vérification de signature (partie 2), il élimine le vecteur d'attaque concret décrit par l'audit (« n'importe quel envelope hijack le groupe ») — mais ce n'est pas une garantie cryptographique complète de légitimité du hub.
- L'augmentation du nombre de slots DHT recommandée par l'audit n'a pas été faite — décision affichée, pas un oubli, mais la mitigation squat reste donc partielle (poisoning bloqué, squat toujours possible avec un budget d'attaque suffisant).
- Je n'ai pas eu le temps de lancer `bash scripts/check-ffi.sh` (FFI hors workspace) avant de préparer le commit — à faire avant tout push si les changements touchent des types traversant la frontière FFI (aucun de mes changements ne touche directement les signatures FFI publiques à ma connaissance, mais `handle_hub_migration`/tick_shadow_ping sont internes à tom-protocol, pas exposés en C — risque jugé faible mais non vérifié formellement).

---

## [2026-07-02 | claude-sonnet-5] — Anti-squat DHT : PoP portée dans tom-dht

### Objectif (lien vers §1, résilience "indébranchable" + souveraineté)
KL#2 (audit) : la vérification proof-of-possession du rendez-vous DHT n'existait que côté `tom-protocol` (`loop.rs::rendezvous_entry_authentic`) — tout consommateur DIRECT de `tom-dht` (sans passer par tom-protocol) restait vulnérable au poisoning d'identité (un attaquant publie une fausse adresse sous le node_id de quelqu'un d'autre).

### Ce que j'ai fait (décisions + pourquoi)
- Ajouté `tom-base` comme dépendance de `tom-dht` (feature `key`) pour réutiliser `PublicKey`/`Signature` — **pourquoi pas dupliquer le décodage de clé** : `tom-base::PublicKey::from_str` est le format canonique utilisé partout pour `NodeId`, dupliquer cette logique dans tom-dht créerait un second point de vérité à maintenir en synchronisation.
- Porté `rendezvous_entry_authentic` (même logique que `loop.rs`) directement dans `tom-dht::rendezvous_discover` — désormais toute entrée non signée, à signature invalide, ou avec un `node_id` usurpé est rejetée **avant même de sortir de tom-dht**, peu importe le consommateur.
- **N'ai pas retiré** la vérification équivalente dans `tom-protocol/src/runtime/loop.rs` — défense en profondeur légitime (tom-protocol reste indépendant si jamais tom-dht changeait), pas de la duplication nuisible. Ses tests existants restent valides tels quels.
- **N'ai PAS augmenté `RENDEZVOUS_SLOTS`** (reste à 8) — voir Auto-critique de l'entrée précédente : impact perf non mesuré, décision affichée pas un oubli.
- Réécrit les helpers de test (`fresh_addr`/`stale_addr`/`distinct_slot_ids`/`same_slot_pair`) pour générer de **vraies paires de clés** ed25519 déterministes (seed → `tom_base::SecretKey::generate`) au lieu de chaînes arbitraires (`"chaos-0"` etc.) — nécessaire car le nouveau filtre PoP rejette tout node_id qui n'est pas une clé publique valide signée.

### Fichiers touchés
- `crates/tom-dht/Cargo.toml` — dépendance `tom-base` (feature `key`), dev-dépendance `rand`.
- `crates/tom-dht/src/lib.rs` — fonction `rendezvous_entry_authentic`, gate dans `rendezvous_discover`, refonte des helpers de test + tous les tests rendezvous qui en dépendaient (8 tests adaptés), 1 nouveau test dédié (`rendezvous_entry_authentic_accepts_valid_and_rejects_forged`).

### Résultats de tests
`cargo test -p tom-dht --lib` : 21 passed, 0 failed (incluant chaos churn + recovery blackout avec vraies clés signées). `cargo clippy -p tom-dht --lib --tests -- -D warnings` : vert.

### Ce qui reste / prochain [→]
Voir entrée précédente (partagé — même session).

### Auto-critique
Le squat de slot (un attaquant publie des entrées **valides et auto-signées** sous ses propres clés pour occuper les 8 slots) n'est **pas** résolu par ce fix — seul le poisoning (usurpation d'identité d'autrui) l'est. C'est exactement la portée que l'audit décrivait ("PoP n'empêche pas le DoS").
