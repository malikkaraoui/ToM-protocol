# Story L1-001 — Attestation de présence (Proof of Presence, primitif)

> Jalon **M1.1** du `docs/plans/TOM-PLAN-GLOBAL.md`. Première brique du swarm L1.
> **Sans argent, sans quorum, sans partition, sans persistance.** Objectif : prouver
> qu'un nœud est vivant MAINTENANT, de façon vérifiable et difficile à falsifier.
> Spec issue de la revue Fable 5 (2026-07-06), à affiner à l'implémentation.

## Objectif

A prouve que B est vivant à l'instant présent : A envoie un défi, B répond avec une
attestation signée **incluant une preuve d'activité récente** (B a réellement relayé
un message récemment). L'attestation est **éphémère (30 s), jamais persistée, jamais
backupée** (aligné décision #2). Plusieurs attestations agrégées serviront de graine
d'entropie en M1.2 — **hors scope ici**.

## 1. Format des messages (MessagePack / rmp-serde, Ed25519, NodeId)

**Défi (A → B)**
```
AttestedPresenceQuery {
  query_id:     [u8; 32],   // nonce aléatoire unique
  timestamp_ms: u64,        // horloge locale de A (NON fiable — informatif)
  requester_id: NodeId,     // clé publique Ed25519 de A
  signature:    [u8; 64],   // Ed25519 sur signing_bytes
}
signing_bytes = b"tom.presence.query" || query_id || timestamp_ms.to_le_bytes()
```

**Réponse (B → A)**
```
AttestedPresenceResponse {
  query_id:              [u8; 32],   // echo du défi
  responder_id:          NodeId,     // clé publique de B
  response_timestamp_ms: u64,        // horloge locale de B, MAINTENANT
  proof:                 ProofOfActivity,
  signature:             [u8; 64],   // Ed25519 sur signing_bytes
}
signing_bytes = b"tom.presence.response" || query_id || response_timestamp_ms.to_le_bytes() || sha256(proof)
```

## 2. Preuve d'activité récente (le point dur — à durcir)

B prouve qu'il a **réellement relayé** récemment :
```
ProofOfActivity {
  last_relay_msg_hash:  [u8; 32],   // sha256 du dernier message relayé par B
  relay_age_ms:         u64,        // âge du relais (doit être < 5000 ms)
  relay_counter:        u64,        // compteur monotone de relais de B
  counter_sig:          [u8; 64],   // Ed25519(counter || relay_timestamp) par B
}
```
- **Hors-ligne → faux** : si B n'a rien relayé depuis > 5 s, il n'a pas de preuve « récente » → l'attestation est rejetée.
- **Anti-rejeu du compteur** : `counter_sig` lie compteur + timestamp → on ne peut pas rejouer une vieille signature (timestamp incohérent avec `now`).

⚠️ **Caveat honnête (résidu à traiter en M1.2/M1.4)** : le cross-check du `last_relay_msg_hash` n'est **vérifiable que par un nœud A qui a lui-même vu passer ce message**. Une **paire de complices** peut fabriquer un « message relayé » entre eux et le présenter comme preuve. La preuve d'activité **augmente le coût** d'un mensonge (il faut relayer pour de vrai) mais **ne l'élimine pas** pour des Sybils coordonnés. C'est précisément pourquoi l'entropie imprévisible (M1.2) + le coût Sybil chiffré (M1.4) restent nécessaires : L1-001 ne prétend PAS résoudre la Sybil, seulement fournir un primitif de présence coûteux à simuler seul.

## 3. Machine à états (avec timeouts)

```
A: IDLE ──send Query──▶ WAITING (timeout 5 s)
        ├─ Response valide (sig ok, proof récente, nonce jamais vu) ─▶ VERIFIED ─▶ store (TTL 30 s)
        ├─ Response invalide ─▶ REJECTED (log)
        └─ timeout 5 s ─▶ TIMEOUT (redial dans 1 s)

B: IDLE ──recv Query──▶ verify sig
        ├─ ok  ─▶ build Response (proof = relay_activity récente ou "stale") ─▶ send
        └─ ko  ─▶ drop silencieux
B: ──relay un message──▶ update RelayActivity (hash + timestamp + counter)
```
Timeouts : défi 5 s ; vérif locale < 1 ms ; **purge attestation 30 s** ; purge RelayActivity 30 s d'inactivité.

## 4. Où ça vit dans le code

Nouveau module `crates/tom-protocol/src/presence/` :
- `attestation.rs` — types `AttestedPresenceQuery` / `Response` / `ProofOfActivity` + `signing_bytes()`.
- `activity.rs` — `RelayActivity { last_relay_msg_hash, relay_timestamp_ms, relay_counter, counter_sig }` + `is_recent(now, 5000)`.
- `verifier.rs` — `verify_query()`, `verify_response()`, tracker anti-rejeu des nonces.
- `mod.rs` — API publique + purge.

Intégration (esquisse, à affiner) :
- `runtime/state.rs` : `RuntimeState` reçoit `relay_activity: HashMap<NodeId, RelayActivity>`, `pending_queries`, `verified_attestations` (avec `expires_at`), `seen_nonces` (anti-rejeu, borné + TTL).
- `runtime/loop.rs` : nouvelles branches `select!` pour les `Query` entrantes (répondre) et les `Response` entrantes (vérifier + stocker), + un `purge_ticker` (5 s) qui `retain` sur `expires_at` / inactivité.
- `router.rs` : sur `RoutingAction::Forward`, mettre à jour `RelayActivity` du nœud local (hash du payload relayé, timestamp, counter++ ; signer le counter par batch).
- **Aucune sérialisation disque, aucun passage par le module `backup/`** (invariant #2).

## 5. Critères d'acceptation + plan de test adverse

| Critère | Attendu | Attaque testée |
|---|---|---|
| Honnête | 100 % succès, médiane < 200 ms | +1 s de latence → timeout propre à 5 s |
| Anti-rejeu | même `query_id` rejoué → 2ᵉ réponse rejetée | rejeu avec nonce falsifié → sig échoue |
| Anti-forge | sig invalide → rejet | falsifier les octets signés → rejet |
| Anti-menteur offline | B sans relais < 5 s → proof `is_recent`=false → rejet | B fabrique un hash jamais vu de A → A le détecte s'il l'a witnessé (caveat §2) |
| Éphémérité | purge à 30 s, jamais sur disque | tenter de persister → interdit par le code |
| Agrégation ordre-indépendante | `hash([a,b]) == hash([b,a])` (tri avant hash) | réordonner → même graine |
| Anti-flood | quota ~10 query/s par `requester_id` | 10k queries → drop au-delà du quota |

## 6. Invariants (tests)

1. **Pas de persistance** : `relay_activity` / `verified_attestations` jamais sérialisés (grep + test).
2. **Éphémérité 30 s** : entrée purgée après 30 s.
3. **Hash d'agrégation ordre-indépendant** (tri déterministe avant hash).
4. **Signature liée au nonce** : changer `query_id` après signature → vérif échoue.
5. **Pas de Sybil sans relais** : un nœud sans activité relais ne produit pas de proof `is_recent`.
6. **Anti-rejeu nonce** : nonce déjà vu → rejeté ; le tracker de nonces est **borné** (LRU + TTL) pour ne pas fuir en mémoire.

## 7. Ce que cette story NE fait PAS (rester petite)

❌ Pas d'agrégation en quorum (→ M1.2/M1.3). ❌ Pas de VDF/entropie (→ M1.2). ❌ Pas de rôles Observateur/Validateur/Gardien (→ plus tard). ❌ Pas de gate Sybil/probation (→ M1.4). ❌ Aucune persistance/backup.

## 8. Story suivante

**M1.2 — Entropie non-biaisable** (recherche, en parallèle) : à partir de N attestations agrégées, produire un aléa vérifiable **indépendant du demandeur** — candidats VDF / beacon passif / signature-seuil du churn — et le tester contre le grinding. C'est le **verrou de recherche** du PoP.
