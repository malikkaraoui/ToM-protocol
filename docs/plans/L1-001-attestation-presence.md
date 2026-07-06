# L1-001 — Attestation de présence

**Jalon L1-001 : première brique du Proof of Presence**

Voir contexte général : [`TOM-PLAN-BOUT-EN-BOUT.md`](./TOM-PLAN-BOUT-EN-BOUT.md) (Phase 2, §jalon L1-001)

> Cette spécification couvre la première story atomique du swarm L1 : un nœud A prouve que le nœud B est vivant MAINTENANT en le défiant de signer une attestation d'activité réelle. L'attestation est éphémère (30s), jamais persistée, et agrégeable en seed entropique (input L1-002).

---

## Résumé exécutif

### Objectif

Implémenter le primitif de base du Proof of Presence : un nœud A prouve que le nœud B est vivant MAINTENANT en le défiant de signer une attestation d'activité réelle (relais constaté par un tiers). L'attestation est éphémère (30s), jamais persistée, et agrégeable en seed entropique pour L1-002.

### Livrable attendu

- **Protocol wire** : `PresenceChallenge` (A→B) + `PresenceAttestation` (B→A)
- **Preuve d'activité réelle** : lien entre attestation et relais observé par un pair tiers
- **Anti-replay** : nonce 24 octets + timestamp, purge TTL 30s (plus court que 24h, plus strict que PoP DHT)
- **Agrégation** : N attestations → seed reproductible, ordre-indépendant (input L1-002)
- **Éphémérité** : JAMAIS stockée en backup, purge garantie 30s post-réception
- **Critères test** : nominaux (T1–T6) + adversaires (A1–A7)

---

## 1. Architecture & localisation précise

### 1.1 Crates et fichiers touchés

| Crate | Fichier(s) nouveau(x)/modifié(s) | Raison |
|-------|----------------------------------|--------|
| **tom-protocol** | `src/types.rs` | Ajouter `PresenceChallenge`, `PresenceAttestation` message types |
| | `src/router.rs` | Handler routing pour Challenge/Attestation (pure routing, signature vérif) |
| | `src/presence/mod.rs` (NEW) | Nouveau module : `PresenceManager`, nonce cache, scoring |
| | `src/presence/nonce.rs` (NEW) | Anti-replay cache : `(nonce, ts)` → purge 30s |
| | `src/presence/attestation.rs` (NEW) | Structures : `Challenge`, `Attestation`, `AttestationPayload`, sérialisation |
| | `src/presence/relay_proof.rs` (NEW) | Lien attestation → relais : tracker, selector, filtrage |
| | `src/presence/aggregator.rs` (NEW) | Agrégation N→seed, canonicalisation |
| | `src/runtime/effect.rs` | +3 variants : `SendPresenceChallenge`, `ProcessPresenceAttestation`, `RecordRelayProof` |
| | `src/runtime/state.rs` | Handler `handle_presence_challenge`, `handle_presence_attestation`, tick/purge |
| | `src/runtime/loop.rs` | Intégration executor : envoyer Challenge, timer purge 30s |
| **tom-transport** | `src/node.rs` | Stats relais observés (input pour relay_proof) |
| | `Cargo.toml` | +1 dep : `sha2` pour agrégation hash (si pas déjà là) |

### 1.2 Dépendances transversales (ordre de compilation)

```
tom-protocol/src/presence/nonce.rs
    ↓
tom-protocol/src/presence/attestation.rs (uses serde, crypto, envelope)
    ↓
tom-protocol/src/presence/relay_proof.rs (uses Topology, PeerInfo)
    ↓
tom-protocol/src/presence/aggregator.rs (uses attestation + relay_proof)
    ↓
tom-protocol/src/types.rs (add MessageType variants)
    ↓
tom-protocol/src/router.rs (handle routing)
    ↓
tom-protocol/src/runtime/state.rs (integrate handlers)
    ↓
tom-protocol/src/runtime/effect.rs (add effect variants)
    ↓
tom-protocol/src/runtime/loop.rs (executor)
```

---

## 2. Protocol wire — Format binaire MessagePack

### 2.1 Message types (enum dans `types.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    // ... existing ...
    
    // L1-001 Proof of Presence (New)
    PresenceChallenge,          // A → B : « prouve-moi que tu es vivant »
    PresenceAttestation,        // B → A : « j'atteste avec preuve relais »
}
```

**Notas :**
- `PresenceChallenge` et `PresenceAttestation` sont des envelopes signés (ou non pour Challenge)
- Payload sérialisé MessagePack (comme `AckPayload`, `ReadReceiptPayload`)
- Les envelopes ne sont JAMAIS chiffrées (attestation doit être vérifiable par relais/observateurs)

### 2.2 Challenge — A → B

**Payload (sérialisé MessagePack, unsigned envelope) :**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceChallengePayload {
    /// Identifiant unique du challenge (UUID v4).
    pub challenge_id: String,
    
    /// Nonce 24 octets (cryptographiquement aléatoire, utilisé pour anti-replay).
    /// Généré par A ; B l'inclut dans l'attestation pour prouver qu'il a répondu
    /// à THIS challenge, pas un ancien.
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,  // must be 24 bytes
    
    /// Timestamp de création du challenge (Unix ms).
    pub timestamp: u64,
    
    /// Identité du challenger (A — redondante avec Envelope::from, mais utile
    /// pour la vérification sans déshaller l'Envelope complet).
    pub challenger_id: NodeId,
}

impl PresenceChallengePayload {
    pub fn to_bytes(&self) -> Result<Vec<u8>, TomProtocolError> { /* rmp_serde */ }
    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> { /* rmp_serde */ }
    
    /// Valide la structure de base (nonce length, timestamp recent).
    pub fn validate(&self, now: u64) -> Result<(), TomProtocolError>;
}
```

**Enveloppe complète :**

```
Envelope {
    id: UUID unique pour le challenge,
    from: A (NodeId),
    to: B (NodeId),
    via: [] (pas de relai initial — direct ou via la connaissance de A sur B),
    msg_type: PresenceChallenge,
    payload: PresenceChallengePayload::to_bytes(),  // MessagePack bytes
    timestamp: now_ms(),
    signature: EMPTY (unsigned — le challenge n'a pas besoin d'être signé;
                       A's identity EST l'Envelope::from, vérifié par le transport)
    ttl: 4 (hérité de DEFAULT_TTL),
    encrypted: false,
}
```

**Taille wire :** ~200 octets (UUID 36 + nonce 24 + header envelope 50 + overhead MessagePack 20).

### 2.3 Attestation — B → A

**Payload (sérialisé MessagePack, SIGNED) :**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceAttestationPayload {
    /// Identifiant du challenge auquel on répond.
    pub challenge_id: String,
    
    /// Nonce du challenge — B l'inclut pour prouver qu'il a vu le challenge.
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    
    /// Timestamp de création de l'attestation (Unix ms).
    pub timestamp: u64,
    
    /// B's node_id (redondant, comme dans Challenge, pour vérification rapide).
    pub attester_id: NodeId,
    
    /// Identité du challenger (A) — B rapelle qui l'a défié.
    pub challenger_id: NodeId,
    
    /// Preuve d'activité réelle : lien entre cette attestation et un relais
    /// observé par un pair tiers C. Voir section 2.4.
    pub relay_proof: RelayProof,
}

impl PresenceAttestationPayload {
    pub fn to_bytes(&self) -> Result<Vec<u8>, TomProtocolError> { /* rmp_serde */ }
    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> { /* rmp_serde */ }
    
    /// Valide nonce + timestamp + relay_proof structurales.
    pub fn validate(&self, now: u64) -> Result<(), TomProtocolError>;
}
```

**Enveloppe complète (SIGNED par B) :**

```
Envelope {
    id: UUID unique pour l'attestation,
    from: B (NodeId),
    to: A (NodeId),
    via: [] (direct ou heuristique de B),
    msg_type: PresenceAttestation,
    payload: PresenceAttestationPayload::to_bytes(),
    timestamp: now_ms(),
    signature: Ed25519(B's secret key).sign(signing_bytes()),  // 64 bytes
    ttl: 4,
    encrypted: false,
}
```

**Taille wire :** ~350 octets (Challenge + RelayProof 100–150).

### 2.4 RelayProof — Lien attestation ↔ relais réel observé

**Problème :** Attestation ne prouve pas qu'on a vraiment relayé. Sybil peut dire « je suis vivant » sans rien faire.

**Solution :** Lier la preuve à un relais observé par un pair tiers C (qui pourrait être le challenger A ou un observateur indépendant).

#### Option A : Signature d'observateur (Heaviest)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayProof {
    /// Schéma de preuve : "observer_signature" ou "self_observed"
    pub proof_type: RelayProofType,
    
    /// Observateur qui a vu B relayer (C) — peut être A (self-observer) ou tiers.
    pub observer_id: NodeId,
    
    /// Timestamp du relais observé (Unix ms).
    pub observed_at: u64,
    
    /// Charge relayée observée (bytes, borné pour anti-amplification).
    pub bytes_relayed: u64,
    
    /// Signature de l'observateur attestant « j'ai vu B relayer observé_à »
    /// (VIDE si observer == attester, c'est auto-attestation simple).
    #[serde(with = "serde_bytes")]
    pub observer_signature: Vec<u8>,  // Ed25519, peut être vide
    
    /// Score de contribution de B SELON l'observateur (0–10, optionnel).
    /// Aidé pour debugging/observabilité, pas critique pour la validation.
    pub reliability_score: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayProofType {
    /// Self-observed : B atteste qu'il a relayé lui-même (simplest).
    /// observer_signature vide, observer_id == attester_id.
    SelfObserved,
    
    /// Observer-signed : C (un pair tiers) signe que B a relayé.
    /// observer_signature peuplée, observer_id != attester_id.
    ObserverSigned,
}

impl RelayProof {
    pub fn to_bytes(&self) -> Result<Vec<u8>, TomProtocolError> { /* */ }
    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> { /* */ }
    pub fn validate(&self, now: u64) -> Result<(), TomProtocolError>;
}
```

#### Option B : Relais tracking diffus (Lightweight)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayProof {
    /// Timestamp de la dernière ACTIVITÉ observée de B par n'importe qui.
    pub last_activity_ms: u64,
    
    /// Score de contribution lissé (de RoleManager, non-signé mais observable).
    pub contribution_score: f64,
}
```

**Tranche (proposée) :** **Option A (SelfObserved)** pour L1-001.

**Raisons :**
- Simple : pas de tiers à interroger initialement
- Testable : auto-attestation suffit pour le PoC
- Extensible : `ObserverSigned` en L1-003 (validation croisée)
- Lutte Sybil : à défaut d'observateur, au minimum B expose son score de relais (B's RoleManager score)

---

## 3. Preuve d'activité réelle — Murs 3 & 4

### 3.1 Mur 4 (Anti-Sybil) : Coût de la présence

**Problème :** Sans coût, une ferme de Sybils peut tous être présents et tous attester chacun.

**Mécanisme :**
- `RelayProof.contribution_score` vient du **RoleManager** existant (relais observés par le transport)
- Seuil d'acceptation : `score ≥ RELAY_CONTRIBUTION_MIN` (ex: 2.0, seuil demotion)
- Seuil de promotion relais (PROMOTION_THRESHOLD = 10.0) → attestation from Relais plus crédible

**Implémentation :**

```rust
const RELAY_CONTRIBUTION_MIN: f64 = 2.0;  // Seuil demotion (déjà en roles/manager.rs)

// Dans PresenceManager::accept_attestation()
let score = role_manager.score(&attester_id, now);
if score < RELAY_CONTRIBUTION_MIN {
    return Err(TomProtocolError::InsufficientRelayActivity);
}
```

**Références code :**
- `crates/tom-protocol/src/roles/manager.rs:13` (PROMOTION_THRESHOLD = 10.0)
- `crates/tom-protocol/src/roles/manager.rs:60–65` (RoleManager::score)

### 3.2 Mur 3 (Entropie) : Agrégation non-biaisable

**Problème :** Sélectionner un quorum à partir d'un set d'attestations. Si A choisit les attesteurs qu'il accepte, il peut choisir ses copains.

**Direction (défer à L1-002) :** Agrégation fait par un tiers hors du demandeur, produit un seed via VDF.

**Pour L1-001 :**
- Implémenter l'**agrégation** (N attestations → seed canonique)
- Valider que l'ordre d'arrivée ne biaise pas le seed
- Exemple : `Aggregate = SHA256(sorted([attestation_1.id, attestation_2.id, ...]))`

**Implémentation :**

```rust
pub struct AttestationAggregator {
    attestations: HashMap<String, PresenceAttestationPayload>,
}

impl AttestationAggregator {
    pub fn add(&mut self, att: PresenceAttestationPayload) {
        self.attestations.insert(att.challenge_id.clone(), att);
    }
    
    /// Produit un seed reproductible, ordre-indépendant.
    pub fn aggregate_seed(&self) -> [u8; 32] {
        let mut ids: Vec<_> = self.attestations.keys().collect();
        ids.sort();  // Canonicalize
        
        let mut hasher = sha2::Sha256::new();
        for id in ids {
            hasher.update(id.as_bytes());
        }
        let digest = hasher.finalize();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&digest[..]);
        seed
    }
}
```

---

## 4. Éphémérité & Purge

### 4.1 Fenêtre temporelle

| Opération | TTL | Raison |
|-----------|-----|--------|
| Challenge valide | 30s | Fenêtre de réponse (B doit attester dans les 30s) |
| Nonce anti-replay | 30s | Plus strict que NONCE_TTL 24h (crypto); PoP plus urgent |
| Attestation au-delà du TTL | Ignorée | Trop vieille, pas de preuve de présence "maintenant" |
| Agrégation seed | 0s (immédiat) | Pas de stockage; L1-002 re-calcule à chaque demande |

### 4.2 Purge garantie

**Invariant :** Aucune attestation stockée au-delà de 30s, jamais en backup.

**Implémentation :**

```rust
// src/presence/nonce.rs
pub struct NonceCache {
    cache: HashMap<Vec<u8>, (u64, Instant)>,  // (nonce, (ts, first_seen))
}

impl NonceCache {
    /// Purge entries > 30s old.
    pub fn cleanup(&mut self, now: Instant) {
        self.cache.retain(|_, (_, first_seen)| first_seen.elapsed() < Duration::from_secs(30));
    }
}

// Dans runtime/state.rs tick
fn tick_presence_cleanup(&mut self, now: u64) -> Vec<RuntimeEffect> {
    self.presence_manager.cleanup_stale_challenges(now);  // 30s TTL
    self.presence_manager.cleanup_stale_attestations(now); // 30s TTL
    self.presence_manager.nonce_cache.cleanup(Instant::now());
    
    vec![]  // No effects; cleanup is silent
}
```

**Où :** `runtime/loop.rs` ajoute un timer `tick_presence_cleanup` appelé chaque 5–10s (DÉJÀ un patron dans `loop.rs::reconnect_check` etc.).

**Références code :**
- `crates/tom-protocol/src/router.rs:37` (NONCE_TTL = 24h)
- `crates/tom-protocol/src/runtime/loop.rs:785–802` (pattern cleanup timer)

---

## 5. Critères d'acceptation testables

### 5.1 Tests nominaux (Happy path)

| # | Cas | Input | Expected | File:line |
|---|-----|-------|----------|-----------|
| **T1** | Challenge valid | A envoie Challenge à B, nonce valide | B reçoit, crée Attestation | `presence/tests.rs:100–150` |
| **T2** | Attestation sign+verify | B signe Attestation avec sa clé | A vérifie signature avec B's pubkey | `presence/tests.rs:160–200` |
| **T3** | Anti-replay | A reçoit Attestation, chèque nonce | Nonce accepté une fois, rejeté replay | `presence/nonce.rs:tests` |
| **T4** | Relay proof | B inclut `contribution_score ≥ 2.0` | Attestation acceptée | `presence/relay_proof.rs:tests` |
| **T5** | Agrégation seed | N attestations | Seed ordre-indépendant, SHA256 stable | `presence/aggregator.rs:tests` |
| **T6** | TTL purge | Challenge + 35s | Challenge purgé, nouvelle tentative OK | `presence/nonce.rs:cleanup_tests` |

### 5.2 Tests adverses (Adversarial)

| # | Cas | Attaque | Défense testée | File:line |
|---|-----|---------|-----------------|-----------|
| **A1** | Forge signature | X signe avec clé falsifiée | Signature verify échoue → reject | `router.rs:tests` |
| **A2** | Replay nonce | X rejeu ancien nonce+ts | `NonceCache.is_duplicate()` → reject | `nonce.rs:tests` |
| **A3** | Delay attack | Challenge reçu mais réponse après 35s | Timestamp check : `now - timestamp > 30s` → reject | `presence/tests.rs:delay_attack` |
| **A4** | Offline attester | A défie B offline, B ne répond jamais | A timeout après 30s, le challenge expire | `runtime/loop.rs:timer` |
| **A5** | Low-score Sybil | Sybil B avec `score = 0.5` | RelayProof validation : reject si `score < MIN` | `presence/relay_proof.rs:tests` |
| **A6** | Biased aggregation | Attacker choisit N attestateurs en fonction du seed | Seed canonique (sorted, hash) → pas de re-tirage possible | `aggregator.rs:adversarial_tests` |
| **A7** | Large Challenge spam | A envoie mille Challenges à B | `Presence.accept_challenge` : limit N concurrent par peer (ex: 10) | `presence/manager.rs:limits` |

**Outils test :**
- `cargo test -p tom-protocol --lib presence`
- Stress test (tom-stress) : 100 nœuds, 50 challenges/s, observe TTL + agrégation

---

## 6. Découpage en sous-tâches codeables

**Effort relatif :** 1 = 1 jour junior, 10 = 1 semaine senior.

| # | Sous-tâche | Effort | Dépend de | Critère d'acceptance |
|---|-----------|--------|-----------|----------------------|
| **1.1** | Ajouter MessageType enum variants | 1 | — | `types.rs` compile + msgpack roundtrip test |
| **1.2** | Impl `PresenceChallengePayload` + tests | 2 | 1.1 | serde roundtrip, validate tests |
| **1.3** | Impl `PresenceAttestationPayload` + `RelayProof` | 2 | 1.2 | serde roundtrip, signature field present |
| **2.1** | Create module `src/presence/nonce.rs` | 2 | 1.3 | `NonceCache` tests, cleanup TTL verified |
| **2.2** | Create module `src/presence/attestation.rs` | 1 | 1.3 | Helper struct + serializers |
| **2.3** | Create module `src/presence/relay_proof.rs` | 3 | 1.3, roles/manager | `RelayProof.validate()`, score filter |
| **2.4** | Create module `src/presence/aggregator.rs` | 2 | 1.3, 2.2 | Aggregate seed canonical, order-independent |
| **2.5** | Create `src/presence/mod.rs` + `PresenceManager` | 3 | 2.1–2.4 | Manager struct, challenge/attestation maps, lifecycle |
| **3.1** | Add routing handlers in `router.rs` | 2 | 2.5 | `route()` returns `RoutingAction` for Challenge/Attestation |
| **3.2** | Add `RuntimeEffect` variants | 1 | — | `effect.rs` compiles, pattern matches |
| **4.1** | Impl handlers in `runtime/state.rs` | 5 | 3.1, 3.2, 2.5 | `handle_presence_challenge`, `handle_presence_attestation` |
| **4.2** | Add cleanup timer in `runtime/loop.rs` | 2 | 4.1 | cleanup called every 5s, 30s TTL verified |
| **5.1** | Transport integration (`node.rs`) | 2 | 4.2 | `SendEnvelope` effects to transport |
| **6.1** | Unit tests (all modules) | 4 | All above | `cargo test -p tom-protocol` passes |
| **6.2** | Integration tests (runtime + transport) | 3 | 6.1 | 2 nodes, A challenges B, attestation flows |
| **6.3** | Adverse tests (forge, replay, delay, etc.) | 4 | 6.2 | All adversarial cases in 5.2 pass |
| **6.4** | Stress test (tom-stress) | 3 | 6.3 | 100 nodes, 50 challenges/s, zero crashes |

**Estimated total :** ~40 effort points (~2–3 sprints pour équipe 3 dev).

**Critical path :**
```
1.1 → 1.2 → 1.3 → (2.1, 2.2, 2.3 parallel) 
    → 2.4 → 2.5 → 3.1 → 3.2 → 4.1 → 4.2 → 5.1 
    → 6.1 → 6.2 → 6.3 → 6.4
```

---

## 7. Frontières & Non-livrable

### 7.1 L1-001 couvre

✅ Challenge/Attestation wire format  
✅ Signature + anti-replay (nonce)  
✅ Preuve d'activité : RelayProof + RoleManager integration  
✅ Agrégation → seed (input L1-002)  
✅ Éphémérité (30s, jamais backup)  
✅ Routing pure (router.rs)  
✅ Unit tests + integ tests + adversarial tests

### 7.2 L1-001 NE couvre PAS

❌ VDF (Verifiable Delay Function) → L1-002  
❌ Sélection cascade de quorum → L1-003  
❌ ObserverSigned (validation croisée) → L1-003  
❌ Stockage/persistance d'attestations (JAMAIS) → design invariant  
❌ Quorum consensus → L1-005  
❌ Ledger ou ancrage global → L2

---

## 8. Ce que L1-001 débloque

| Jalon | Dépend de | Raison |
|-------|-----------|--------|
| **L1-002 (Mur 3)** | L1-001 seed + VDF | L1-001 produit seed agrégé, L1-002 le VDFy pour non-biaisable |
| **L1-003 (Cascade)** | L1-001 + L1-002 | Sélect quorum via seed, chaque nœud choisit next level |
| **L1-004 (Sybil)** | L1-001 score filter | L1-001 montre coût : `score ≥ threshold` → L1-004 paramètre Q |
| **L1-005 (Validation croisée)** | L1-001 + L1-003 | Roles attestent avec `ObserverSigned` (variant RelayProof) |
| **L1-006 (Ancrage)** | L1-001 + L1-005 | Engagement Merkle du état présent (nœuds actuels) |

---

## 9. Code example (pseudocode — structure réelle)

### 9.1 Envoyer un Challenge (A vers B)

```rust
// runtime/state.rs
pub fn initiate_presence_check(&mut self, target: NodeId, now: u64) -> Vec<RuntimeEffect> {
    let challenge_id = uuid::Uuid::new_v4().to_string();
    let mut nonce = [0u8; 24];
    use rand::Rng;
    rand::thread_rng().fill(&mut nonce);
    
    let payload = PresenceChallengePayload {
        challenge_id: challenge_id.clone(),
        nonce: nonce.to_vec(),
        timestamp: now,
        challenger_id: self.local_id,
    };
    
    let envelope = Envelope::new(
        self.local_id,
        target,
        MessageType::PresenceChallenge,
        payload.to_bytes().unwrap(),
    );
    
    self.presence_manager.store_challenge(challenge_id, &payload, now);
    
    vec![RuntimeEffect::SendEnvelope(envelope)]
}
```

### 9.2 Traiter un Challenge (B reçoit)

```rust
// runtime/state.rs
pub fn handle_presence_challenge(&mut self, env: Envelope, now: u64) -> Vec<RuntimeEffect> {
    let payload = match PresenceChallengePayload::from_bytes(&env.payload) {
        Ok(p) => p,
        Err(e) => return vec![RuntimeEffect::Reject { reason: format!("invalid challenge: {e}") }],
    };
    
    if let Err(e) = payload.validate(now) {
        return vec![RuntimeEffect::Reject { reason: format!("challenge validation failed: {e}") }];
    }
    
    // Build attestation
    let role_score = self.role_manager.score(&self.local_id, now);
    let attestation_payload = PresenceAttestationPayload {
        challenge_id: payload.challenge_id.clone(),
        nonce: payload.nonce.clone(),
        timestamp: now,
        attester_id: self.local_id,
        challenger_id: payload.challenger_id,
        relay_proof: RelayProof {
            proof_type: RelayProofType::SelfObserved,
            observer_id: self.local_id,
            observed_at: now,
            bytes_relayed: self.role_manager.get_metrics(&self.local_id, &self.topology, now)
                .map(|m| m.bytes_relayed).unwrap_or(0),
            observer_signature: vec![],  // Empty for SelfObserved
            reliability_score: Some(role_score),
        },
    };
    
    let mut envelope = Envelope::new(
        self.local_id,
        payload.challenger_id,
        MessageType::PresenceAttestation,
        attestation_payload.to_bytes().unwrap(),
    );
    envelope.sign(&self.secret_key_bytes);
    
    vec![RuntimeEffect::SendEnvelope(envelope)]
}
```

### 9.3 Traiter une Attestation (A reçoit)

```rust
// runtime/state.rs
pub fn handle_presence_attestation(&mut self, env: Envelope, now: u64) -> Vec<RuntimeEffect> {
    if let Err(e) = env.verify_signature() {
        return vec![RuntimeEffect::Reject { reason: format!("invalid signature: {e}") }];
    }
    
    let payload = match PresenceAttestationPayload::from_bytes(&env.payload) {
        Ok(p) => p,
        Err(e) => return vec![RuntimeEffect::Reject { reason: format!("invalid attestation: {e}") }],
    };
    
    // Check challenge was issued by us
    if !self.presence_manager.has_challenge(&payload.challenge_id) {
        return vec![RuntimeEffect::Reject { reason: "unknown challenge".into() }];
    }
    
    // Verify nonce not replayed
    if self.presence_manager.nonce_cache.is_duplicate(&payload.nonce) {
        return vec![RuntimeEffect::Reject { reason: "replay attack".into() }];
    }
    
    // Validate relay proof
    if payload.relay_proof.contribution_score.unwrap_or(0.0) < RELAY_CONTRIBUTION_MIN {
        return vec![RuntimeEffect::Reject { reason: "insufficient relay activity".into() }];
    }
    
    // Accept
    self.presence_manager.store_attestation(payload.clone(), now);
    
    vec![
        RuntimeEffect::Emit(ProtocolEvent::PresenceAttestationReceived {
            from: env.from,
            challenge_id: payload.challenge_id,
        }),
    ]
}
```

---

## 10. Références code exactes (file:line)

| Réf | Fichier | Ligne | Contexte |
|-----|---------|-------|----------|
| NONCE_TTL | `crates/tom-protocol/src/router.rs` | 37 | Exemple TTL 24h ; L1-001 utilise 30s |
| PROMOTION_THRESHOLD | `crates/tom-protocol/src/roles/manager.rs` | 13 | PROMOTION = 10.0 ; MIN = 2.0 (demotion) |
| RoleManager::score | `crates/tom-protocol/src/roles/manager.rs` | 60–65 | Fetch contribution score d'un peer |
| RouterAction | `crates/tom-protocol/src/router.rs` | 45–80 | Pattern ; L1-001 ajoute handler |
| RuntimeEffect | `crates/tom-protocol/src/runtime/effect.rs` | 14–66 | Pattern ; L1-001 ajoute variants |
| envelope.sign() | `crates/tom-protocol/src/envelope.rs` | 134–138 | Ed25519 signing pattern |
| envelope.verify_signature() | `crates/tom-protocol/src/envelope.rs` | 141–157 | Verification pattern |
| MessageType enum | `crates/tom-protocol/src/types.rs` | 6–51 | Ajouter PresenceChallenge, PresenceAttestation |
| runtime tick | `crates/tom-protocol/src/runtime/loop.rs` | 785–802 | Exemple cleanup timer ; L1-001 ajoute presence cleanup |

---

## 11. Checklist pre-implementation

- [ ] Tous les fichiers approuvés dans ce plan
- [ ] Architecture dépendances confirmée (tom-protocol → tom-transport, pas inverse)
- [ ] MessagePack serdes testés (roundtrip)
- [ ] RoleManager integration path clair (score fetch)
- [ ] 30s TTL vs 24h NONCE_TTL arbitrage accepté
- [ ] RelayProof::SelfObserved suffisant pour L1-001 (ObserverSigned défer L1-003)
- [ ] Agrégation seed canonique (sorted) sans re-tirage possible
- [ ] Tests adverses couverts (7/7 cas)
- [ ] Tom-stress campaign ready (100 nodes pattern)
- [ ] FFI check (no export de PresenceManager to Swift yet) → FFI L1-002

---

## Next

**L1-002 — Entropie non-biaisable (Mur 3 — VDF)**

L1-001 produit un seed agrégé ; L1-002 ajoute une Verifiable Delay Function pour rendre impossible le grinding (re-tirage jusqu'à un quorum commode). Le seed devient entropie de sélection cascade (L1-003).
