# Handoff — PR #39 : Apple distribution + audit protocole + durcissement FFI

> Date : 2026-06-09
> Type : review
> Priorité : haute (bugs UB exploitables corrigés, 20 commits, ~3000 lignes)
> reviewedRange: 9487ccd..21b0055
> PR : https://github.com/malikkaraoui/ToM-protocol/pull/39

---

## De : Claude (Sonnet 4.6)

### Contexte

PR #39 représente une session complète de travail avec 20 commits couvrant 4 domaines :

**1. Durcissement FFI (bugs UB critiques)**
- 6 fonctions segfaultaient sur param NULL via `CStr::from_ptr(NULL)` → remplacées par `cstr_opt()` null+UTF-8 safe
- `payload NULL + len>0` → `from_raw_parts` UB → rejet explicite `-1`
- 3× `.lock().unwrap()` sur `std::Mutex` → `lock_recover()` tolérant au poison
- Suite tests FFI : 3 → 20 tests (null-handle sur 14 fonctions, params NULL, JSON invalide, non-UTF-8)

**2. Nouveaux tests protocol (+41)**
- Gossip adversarial (7 tests) : sig forgée, URL trafiquée, wrong_signer, score trafiqué, timestamps invalides
- Double panne hub (3 tests) : Primary+Shadow morts avant HubMigration → invariant "pas de mise à jour silencieuse"
- HKDF vecteurs épinglés (4 tests) : déterminisme, vecteur `IKM=[0x42;32] → [0xcb,0x3f,...]`
- ADR-003 TTL (7 tests) : signing_bytes exclut TTL, mutation + wire roundtrip
- Invariant #1 Delivery⟺ACK (3 tests) : cas négatifs

**3. Déviation B corrigée — bootstrap hardcodé supprimé**
- `tom_node_status()` retourne `relay_url_active` (relay configurée > gossip > vide)
- `tom_get_discovered_relay()` nouvelle fonction FFI
- `TomNodeService.swift` : IP Freebox hardcodée `"http://192.168.0.83:3340"` → `""` (découverte auto)
- `project.yml` (xcodegen) pour iOS : `make gen` → `TomNode.xcodeproj` sans Xcode manuel

**4. Harness réseau nouveaux scénarios**
- `scenario_partition.rs` : split-brain 4 nœuds, guérison et livraison confirmée
- `scenario_churn.rs` : arrivée/départ mid-stream, résilience Alice↔Charlie

### Question précise

**Focus 1 — FFI sécurité (critique) :**
Les corrections `cstr_opt()` et `lock_recover()` couvrent-elles tous les vecteurs d'UB exploitables depuis Swift/ObjC ? Y a-t-il des cas de double-free, use-after-free ou lifetime violation dans `lib.rs` qui auraient été manqués ?

**Focus 2 — relay_url_active logique :**
La logique de priorité `configured > gossip-discovered > ""` dans `tom_node_status()` est-elle correcte ? Risque de race condition entre le thread gossip et le thread FFI lors de la lecture du relay découvert ?

**Focus 3 — Tests double panne hub :**
Les 3 tests `double_hub_failure` testent le cas où Primary+Shadow crashent AVANT réception de `HubMigration`. Le cas inverse (crash APRÈS migration partielle) est-il couvert ? Quel est le comportement attendu ?

### Fichiers à lire

1. `crates/tom-protocol-ffi/src/lib.rs` — cstr_opt(), lock_recover(), tom_node_status(), tom_get_discovered_relay()
2. `crates/tom-protocol/src/group/hub.rs` — double panne hub tests
3. `crates/tom-protocol/src/crypto.rs` — HKDF vecteurs épinglés
4. `crates/tom-protocol/src/envelope.rs` — ADR-003 TTL tests
5. `apps/tom-node-tvos/TomNode/Services/TomNodeService.swift` — suppression IP hardcodée
6. `apps/tom-node-ios/project.yml` — xcodegen iOS

### Contraintes / hors scope

- Ne pas proposer de réécrire ce qui fonctionne
- Se concentrer sur : UB résiduel FFI, race conditions relay discovery, couverture trous hub
- Les 6 échecs `test_bind_addr_*` dans tom-connect sont pré-existants (non liés à cette PR)

---

## Réponse de : Copilot/GPT

**INSTRUCTION POUR LE LLM QUI RÉPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans docs/handoffs/2026-06-09-pr39-apple-audit-ffi.md"

---

## Intégration
<!-- Claude remplit après lecture de la réponse -->
