# ToM Protocol — Audit complet (sous toutes les coutures)

> Date : 2026-06-06 · Branche : `claude/repo-status-check-LI97R`
> Méthode : analyse statique fan-out (4 agents) + lecture directe + exécution des tests.
> **Principe : des faits avec `fichier:ligne`, pas des promesses.** Les trous sont nommés, classés par sévérité, et l'état réel (corrigé / restant) est indiqué sans enjoliver.

---

## 1. Couverture de tests — vérité terrain

Comptage réel des fonctions de test par crate (`#[test]` + `#[tokio::test]`) :

| Crate | Tests | Fichiers | Verdict |
|-------|-------|----------|---------|
| tom-protocol | 505 | 33 | Cœur bien couvert ; trous adversariaux (voir §4) |
| tom-quinn-proto | 315 | 27 | Hérité fork iroh, solide |
| tom-connect | 78 | 17 | 6 tests `bind_addr` échouent **avant mes changements** (pré-existant, voir §6) |
| tom-relay | 58 | 11 | OK |
| tom-transport | 31 | 3 | OK |
| tom-quinn | 25 | 2 | OK |
| tom-gateway | 13 | 3 | Léger |
| tom-gossip | 12 | 5 | **Léger** — pas de test gossip malformé (voir §4) |
| tom-base | 9 | 3 | OK |
| tom-dht | 6 | 1 | **Léger** |
| **tom-protocol-ffi** | **20** (était 3) | 1 | **Durci + testé cette session (§3)** |
| tom-relay-ffi | 1 | 1 | **Quasi nul** |

**Constat fort** : `tom-protocol-ffi` (la couche C qui alimente TOUTES les apps Apple) était à 3 tests pour ~1000 LOC = ~0.3 % de surface couverte. C'était le maillon faible critique. Corrigé cette session.

---

## 2. Les 7 décisions LOCKED — appliquées ? testées ?

| # | Décision | Appliquée (`fichier:ligne`) | Testée | Trou |
|---|----------|------------------------------|--------|------|
| 1 | Delivery ⟺ ACK | ✅ `runtime/state.rs:864` (`mark_delivered` uniquement sous `AckType::RecipientReceived`) | ✅ positif `tests/runtime_integration.rs:152` + **négatif ajouté §3** `tracker.rs` | **Comblé** |
| 2 | TTL 24h purge | ✅ `backup/types.rs:14,73` (clamp `MAX_TTL_MS`), purge `backup/store.rs:199` | ✅ borne `backup/mod.rs:131` (just-under gardé / at-24h purgé) | Aucun |
| 3 | L1 ancre, n'arbitre pas | ℹ️ Hors scope crate (L1 = couche externe) | N/A | N/A |
| 4 | Réputation fade, pas de ban | ✅ `roles/scoring.rs:8,80` (décroissance exp. 5 %/h, pas de plancher) | ✅ `scoring.rs:171` (jamais 0) + recovery `scoring.rs:174` | Aucun |
| 5 | Anti-spam progressif | ✅ `roles/antispam.rs:25,162` (courbe S, `min_rate>0`, token bucket) | ✅ `antispam.rs:240` (ne bloque jamais à 0) | ⚠️ **Incohérence** : le hub groupe `group/hub.rs:769` fait un **drop dur** (5 msg/s), pas du back-pressure progressif. Conforme « pas de ban » mais pas « sprinkler sprinkled ». Décision produit requise. |

**ADR-003 (signing_bytes exclut ttl)** : l'invariant wire le plus important. Le `struct SignableEnvelope` (`envelope.rs:271-284`) exclut bien `ttl`, mais **aucun test ne le prouvait au niveau comportement**. → **Comblé cette session (§3)**.

---

## 3. Ce qui a été corrigé/ajouté cette session (faits)

### 3.1 Durcissement FFI (`tom-protocol-ffi/src/lib.rs`)

Bugs **réels et exploitables** (UB) trouvés et corrigés :

| Bug (fait) | Avant | Après |
|------------|-------|-------|
| 6 fns déréférençaient un `char*` NULL via `CStr::from_ptr` | seul `handle` null-checké | `cstr_opt()` : null-check + UTF-8 check sur **tous** les params string |
| `payload` NULL + `len>0` → `from_raw_parts` UB | aucune garde | rejet explicite `-1` |
| 3 × `.lock().unwrap()` sur `std::Mutex` → panic traversant le FFI = UB | `unwrap()` | `lock_recover()` tolérant au poison |

Tests FFI : **3 → 20** (+17). Couvre : null-handle sur les 14 fonctions, params NULL, JSON invalide, non-UTF-8, NodeId invalide, lifecycle non-démarré, contrat JSON Swift↔Rust.

### 3.2 ADR-003 prouvé (`tom-protocol/src/envelope.rs`)

+7 tests : ttl exclu de `signing_bytes`, 8 hops décrémentés avec signature valide, survie au roundtrip wire, exhaustion ttl→erreur, entrées msgpack tronquées/garbage → `Err` (pas panic), ciphertext malformé → `Err` (pas panic).

### 3.3 Invariant #1 négatif (`tom-protocol/src/tracker.rs`)

+3 tests : sans ACK jamais `Delivered` ; ACK relais ≠ livraison ; message épuisé → `Failed`, jamais promu silencieusement.

**Total session : +27 tests, 0 régression, clippy workspace clean.**

---

## 4. Trous restants — classés par sévérité (honnête)

### 🔴 CRITIQUE (à traiter avant « vrai protocole » multi-nœuds hostile)

| Trou | Évidence | Risque |
|------|----------|--------|
| **Mort simultanée Primary+Shadow** du hub non testée | `group/manager.rs` — cascade testée 1 panne à la fois `tests/group_integration.rs:927` | Groupe orphelin si les deux tombent (partition, double crash) |
| **Partition réseau / split-brain** non testé | aucun test | Deux partitions voient des hubs différents → état divergent |
| **Gossip malformé/malveillant** non rejeté en test | `tests/discovery_integration.rs:186` ne teste que les bornes de timestamp | Injection msgpack/sig falsifiée non couverte |
| **Replay nonce** (rejouer la même enveloppe) | absent côté runtime | XChaCha20 : réutilisation nonce = perte confidentialité |

### 🟠 ÉLEVÉ

| Trou | Évidence |
|------|----------|
| Perte de messages sous churn (1-5 % packet loss réel) jamais simulée | `tom-integration-tests/tests/multi_node.rs` suppose 100 % livraison |
| Réplication backup suppose livraison réseau OK (pas de réplication partielle) | `tests/backup_integration.rs:19` |
| Distribution sender-key à un nouveau membre : perte pendant le handshake non testée | `tests/group_integration.rs:677` |
| Vecteurs de régression HKDF (valeurs épinglées) absents | `crypto.rs:78` `derive_key` privé, jamais testé isolément |

### 🟡 MOYEN

| Trou | Évidence |
|------|----------|
| Timings watchdog (3 s ping / ~6 s promote) non validés sous jitter | `group/types.rs:31,34,37` (constantes seulement) |
| `EncryptedPayload` tronqué (champ manquant) partiellement couvert | seul le garbage brut est testé |
| Mutex bloquant (`std::Mutex` `last_error`) en contexte async FFI | `lib.rs:44` — risque de blocage d'exécuteur si appelé depuis tokio |
| Double-free contrat C `stop`/`free` (Swift nil bien le handle, mais contrat C fragile) | `lib.rs:351,371` |
| Hub groupe : back-pressure progressif vs drop dur (cf. Décision #5) | `group/hub.rs:769` |

---

## 5. Plan de comblement recommandé (ordre de valeur)

1. **Test partition/split-brain + double panne hub** — la plus grande faille « vrai protocole ». Nécessite un harness multi-nœuds avec contrôle réseau (kill + isolate).
2. **Replay nonce au niveau runtime** — rejouer une enveloppe → 2e occurrence rejetée (anti-replay R11 à vérifier sous test).
3. **Gossip adversarial** — sig falsifiée, msgpack corrompu, faux relay addr → rejet sans panic.
4. **Churn / packet loss** — injecter 1-5 % de perte dans `multi_node.rs`, vérifier convergence.
5. **Vecteurs HKDF épinglés** — figer (secret partagé → clé dérivée) pour détecter toute régression crypto.
6. **catch_unwind FFI** — isoler tout panic résiduel du runtime au lieu de seulement les locks (résiduel nommé, pas faux « done »).

---

## 6. Note CI — échecs pré-existants (transparence)

6 tests `tom-connect` échouent : `endpoint::tests::test_bind_addr_*`. **Vérifié par `git stash` : ils échouent AVANT mes changements** — pré-existants, sans rapport avec ce travail (liés au binding socket dans l'environnement CI sandbox). À investiguer séparément. Le reste du workspace passe.

---

## 7. Verdict

**Le cœur protocole (crypto, envelope, routing, tracker, roles, backup, TTL) est solidement testé et les invariants fondateurs sont appliqués.** Les trous réels pour atteindre « vrai protocole résilient en environnement hostile » sont concentrés sur : **résilience multi-nœuds adversariale** (partition, double panne, churn, gossip malveillant). Ce sont des tests d'intégration réseau, pas des bugs de logique — la fondation est saine.

La couche FFI Apple, point faible critique au départ, est maintenant durcie et testée.
