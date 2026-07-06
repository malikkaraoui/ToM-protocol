# L1-001 — Attestation de présence

**Jalon L1-001 : première brique du Proof of Presence**

Voir contexte général : [`TOM-PLAN-GLOBAL.md`](./TOM-PLAN-GLOBAL.md) (Phase 1, jalon M1.1)

> **Révision durcie (Fable 5, 2026-07-06)** — 2e passe adversariale sur la spec elle-même.
> Changements vs V1 : challenge **signé** (anti-réflexion/asymétrie CPU) · gate anti-Sybil =
> **score local observé par A** (jamais le champ auto-déclaré de B) · attestation liée à la
> **cible** du challenge (nonce comparé + challenge one-shot) · seed agrégé sur les
> **signatures** (pas les IDs choisis par A) + grinding par sous-ensemble documenté comme
> OUVERT jusqu'à L1-002 · fraîcheur jugée à l'**horloge locale de A** (pas de NTP) ·
> bornes mémoire globales (pattern anti-DoS 347421b) · `u64` ms partout (pas d'`Instant`
> dans l'état, testabilité effect-pattern).

> Cette spécification couvre la première story atomique du swarm L1 : un nœud A prouve que le nœud B est vivant MAINTENANT en le défiant de signer une attestation d'activité réelle. L'attestation est éphémère (30s), jamais persistée, et agrégeable en seed entropique (input L1-002).

---

## Résumé exécutif

### Objectif

Implémenter le primitif de base du Proof of Presence : un nœud A prouve que le nœud B est vivant MAINTENANT en le défiant de signer une attestation d'activité réelle (relais constaté par un tiers). L'attestation est éphémère (30s), jamais persistée, et agrégeable en seed entropique pour L1-002.

### Livrable attendu

- **Protocol wire** : `PresenceChallenge` (A→B) + `PresenceAttestation` (B→A)
- **Preuve d'activité réelle** : lien entre attestation et relais observé par un pair tiers
- **Anti-replay** : challenge **one-shot** (consommé à l'acceptation) + nonce 24 octets comparé à celui du challenge, purge TTL 30s à l'horloge locale du challenger
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
| | `src/presence/mod.rs` (NEW) | Nouveau module : `PresenceManager` (challenges pending one-shot + attestations acceptées + purge 30s + bornes §4.3) |
| | `src/presence/attestation.rs` (NEW) | Structures : `Challenge`, `Attestation`, `AttestationPayload`, sérialisation |
| | `src/presence/relay_proof.rs` (NEW) | Lien attestation → relais : tracker, selector, filtrage |
| | `src/presence/aggregator.rs` (NEW) | Agrégation N→seed, canonicalisation |
| | `src/runtime/effect.rs` | +3 variants : `SendPresenceChallenge`, `ProcessPresenceAttestation`, `RecordRelayProof` |
| | `src/runtime/state.rs` | Handler `handle_presence_challenge`, `handle_presence_attestation`, tick/purge |
| | `src/runtime/loop.rs` | Intégration executor : envoyer Challenge, timer purge 30s |
**Patch mono-crate** : tout tient dans `tom-protocol` (le score de relais vient du
`RoleManager` existant, déjà alimenté). `sha2 = "0.10"` est **déjà** dans
`tom-protocol/Cargo.toml:22` — zéro dépendance nouvelle. Pas de modification
`tom-transport` en L1-001 (les stats d'observation tierce arrivent avec
`ObserverSigned` en L1-003).

### 1.2 Dépendances transversales (ordre de compilation)

```
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
- `PresenceChallenge` et `PresenceAttestation` sont **tous deux des envelopes SIGNÉS** (voir §2.2 pourquoi le challenge doit l'être)
- Payload sérialisé MessagePack (comme `AckPayload`, `ReadReceiptPayload`)
- Les envelopes ne sont JAMAIS chiffrées (attestation doit être vérifiable par relais/observateurs). **Conséquence assumée** : tout relais sur le chemin voit `challenge_id` + nonce → c'est PRÉCISÉMENT pourquoi l'attestation doit être liée à la **cible** du challenge (§9.3, check `env.from == challenge.target`), sinon n'importe quel relais répond à la place de B.

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
    signature: Ed25519(A's secret key).sign(signing_bytes()),  // SIGNÉ — non négociable
    ttl: 4 (hérité de DEFAULT_TTL),
    encrypted: false,
}
```

**Pourquoi le challenge DOIT être signé** (durcissement V2 — la V1 le laissait unsigned) :
1. **Réflexion** : un attaquant forge des challenges « from A » vers N nœuds B → tous répondent à A en attestations signées (~350 o contre ~200 o, amplification ×1.75 + pollution). `Envelope::from` n'est PAS authentifié de bout en bout quand l'envelope transite par relais — seul le hop QUIC l'est.
2. **Asymétrie CPU** : un challenge non signé coûte ~0 à forger ; B répond par une **signature Ed25519** (coûteuse). Signer le challenge rétablit la symétrie : l'attaquant paie une signature pour en obtenir une.
3. B vérifie `env.verify_signature()` + `payload.challenger_id == env.from` AVANT de signer quoi que ce soit ; sinon drop silencieux (pas de Reject réseau → pas d'oracle).

**Taille wire :** ~270 octets (UUID 36 + nonce 24 + signature 64 + header envelope 50 + overhead MessagePack 20).

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
    
    /// Score de contribution de B SELON B (auto-déclaré, 0–10, optionnel).
    /// ⚠️ OBSERVABILITÉ UNIQUEMENT — champ contrôlé par l'attaquant.
    /// Le gate anti-Sybil (§3.1) lit EXCLUSIVEMENT le score LOCAL que A
    /// a observé sur B (`self.role_manager.score(&env.from, now)`).
    /// Toute validation qui lit ce champ est un bug (leçon V1 : le
    /// pseudocode initial gatait sur ce champ → Sybil ment, gate contourné).
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
- Lutte Sybil : le gate est le **score de relais de B tel qu'OBSERVÉ LOCALEMENT par A** (RoleManager de A, alimenté par le trafic que A a réellement vu transiter par B). Rien de ce que B déclare n'entre dans la décision.

**Limite honnête (assumée, pas cachée)** : `SelfObserved` + score local ne prouve la présence qu'aux yeux d'un challenger qui a DÉJÀ observé B relayer. Un nœud fraîchement arrivé qui challenge un inconnu obtient `score = 0` → rejet légitime (c'est la probation qui veut ça), mais cela signifie aussi que **L1-001 ne produit pas encore une preuve transférable à un tiers**. La preuve transférable (signée par un observateur indépendant) est exactement le livrable `ObserverSigned` de L1-003. Ne pas prétendre le contraire dans les démos.

---

## 3. Preuve d'activité réelle — Murs 3 & 4

### 3.1 Mur 4 (Anti-Sybil) : Coût de la présence

**Problème :** Sans coût, une ferme de Sybils peut tous être présents et tous attester chacun.

**Mécanisme — règle d'or : le gate lit UNIQUEMENT l'état local de A** :
- Le score vient du **RoleManager de A** (relais de B observés par le transport de A) — jamais du payload envoyé par B
- Seuil d'acceptation : `score_local(B) ≥ RELAY_CONTRIBUTION_MIN` (2.0 = `DEMOTION_THRESHOLD` existant)
- `PROMOTION_THRESHOLD = 10.0` → attestation d'un Relais confirmé plus crédible (info d'observabilité, pas un 2e gate)

**Implémentation :**

```rust
const RELAY_CONTRIBUTION_MIN: f64 = 2.0;  // = DEMOTION_THRESHOLD (roles/manager.rs:16)

// Dans handle_presence_attestation() — côté A, état local de A uniquement
let local_score = self.role_manager.score(&env.from, now);  // ce que A a VU, pas ce que B DIT
if local_score < RELAY_CONTRIBUTION_MIN {
    return vec![];  // drop silencieux — pas d'oracle pour le Sybil
}
```

**Références code (vérifiées 2026-07-06) :**
- `crates/tom-protocol/src/roles/manager.rs:13` (PROMOTION_THRESHOLD = 10.0)
- `crates/tom-protocol/src/roles/manager.rs:16` (DEMOTION_THRESHOLD = 2.0)
- `crates/tom-protocol/src/roles/manager.rs:60` (`RoleManager::score(&self, node_id, now: u64)`)

### 3.2 Mur 3 (Entropie) : Agrégation non-biaisable

**Problème :** Sélectionner un quorum à partir d'un set d'attestations. Si A choisit les attesteurs qu'il accepte, il peut choisir ses copains.

**Direction (défer à L1-002) :** Agrégation fait par un tiers hors du demandeur, produit un seed via VDF.

**⚠️ Correction V2 — la V1 était grindable par construction.** La V1 hashait les
`challenge_id`… des UUID **générés par A lui-même** : A contrôlait 100 % des inputs du
seed (re-générer des UUID jusqu'à obtenir le seed voulu = grinding trivial, sans même
toucher aux attestations). Corrigé : le seed hashe les **signatures Ed25519 de B**
(64 octets que A ne peut ni forger ni prédire — il faudrait la clé privée de B).

**Ce que L1-001 garantit / NE garantit PAS (honnêteté contractuelle) :**

| Propriété | L1-001 | Qui la fournit |
|---|---|---|
| Ordre-indépendance (même set → même seed) | ✅ testé (T5) | tri canonique |
| Inputs hors du contrôle unilatéral de A | ✅ testé (A6) | signatures de B |
| **Anti-grinding par SOUS-ENSEMBLE** (A choisit QUELLES attestations inclure/exclure → 2^N seeds candidats) | ❌ **OUVERT — ne pas le prétendre** | L1-002 (VDF / beacon passif / signature-seuil) |

C'est exactement le mur #1 de la map (§11) : L1-001 fournit la matière première
(attestations non-forgeables), L1-002 fournit la non-biaisabilité de la sélection.

**Implémentation :**

```rust
pub struct AttestationAggregator {
    /// clé = challenge_id ; borné par MAX_STORED_ATTESTATIONS (§4.3)
    attestations: HashMap<String, StoredAttestation>,  // StoredAttestation = payload + signature envelope
}

impl AttestationAggregator {
    /// Produit un seed reproductible, ordre-indépendant, sur inputs non-forgeables.
    pub fn aggregate_seed(&self) -> [u8; 32] {
        // Trier par SIGNATURE (bytes B-contrôlés), pas par challenge_id (bytes A-contrôlés)
        let mut sigs: Vec<&[u8]> = self.attestations.values()
            .map(|a| a.envelope_signature.as_slice())
            .collect();
        sigs.sort_unstable();  // canonicalisation → ordre d'arrivée sans effet

        let mut hasher = sha2::Sha256::new();
        for sig in sigs {
            hasher.update(sig);
        }
        hasher.finalize().into()
    }
}
```

---

## 4. Éphémérité & Purge

### 4.1 Fenêtre temporelle

| Opération | TTL | Raison |
|-----------|-----|--------|
| Challenge valide | 30s | Fenêtre de réponse (B doit attester dans les 30s) |
| Challenge consommé | **one-shot** | 1 challenge = 1 attestation max ; consommé à l'acceptation → le replay est structurellement impossible (pas besoin d'un cache de nonces séparé côté A) |
| Attestation au-delà du TTL | Ignorée | Trop vieille, pas de preuve de présence « maintenant » |
| Agrégation seed | 0s (immédiat) | Pas de stockage; L1-002 re-calcule à chaque demande |

**⏱️ Horloges non synchronisées (durcissement V2)** : le réseau est asynchrone, sans NTP
(c'est la prémisse même de M1.2). Donc la fraîcheur d'une attestation est jugée à
l'**horloge locale de A** : `now_A − challenge.issued_at_A ≤ 30s` (deux timestamps pris
par la même horloge). Le `timestamp` déclaré par B est **advisory** (observabilité,
jamais un critère d'acceptation). La V1 validait `now − payload.timestamp ≤ 30s` avec
deux horloges différentes → faux rejets sur toute dérive > quelques s, et B pouvait
mentir dans l'autre sens.

### 4.2 Purge garantie

**Invariant :** Aucune attestation stockée au-delà de 30s, jamais en backup.

**Implémentation** (⚠️ `u64` ms partout — JAMAIS d'`Instant` dans `RuntimeState` :
l'effect-pattern exige un état pur pilotable par `now: u64` injecté, sinon les tests
déterministes de purge sont impossibles ; cf. `PeerInfo.last_seen: u64`, même règle) :

```rust
// src/presence/mod.rs — PresenceManager
pub struct PresenceManager {
    /// Challenges émis par A, en attente : challenge_id → (nonce, target, issued_at_ms)
    pending: HashMap<String, PendingChallenge>,
    /// Attestations acceptées (fenêtre agrégation) : challenge_id → StoredAttestation
    accepted: HashMap<String, StoredAttestation>,
}

impl PresenceManager {
    /// Purge entries > 30s old — horloge injectée, testable.
    pub fn cleanup(&mut self, now: u64) {
        self.pending.retain(|_, c| now.saturating_sub(c.issued_at) < 30_000);
        self.accepted.retain(|_, a| now.saturating_sub(a.accepted_at) < 30_000);
    }
}

// Dans runtime/state.rs tick
fn tick_presence_cleanup(&mut self, now: u64) -> Vec<RuntimeEffect> {
    self.presence_manager.cleanup(now);
    vec![]  // No effects; cleanup is silent
}
```

### 4.3 Bornes mémoire globales (pattern anti-DoS — leçon 347421b)

Les limites par-pair ne suffisent pas : 1000 pairs × 10 challenges = collections non
bornées (classe de DoS déjà corrigée ailleurs dans le protocole). Bornes **globales** :

| Constante | Valeur | Effet au dépassement |
|---|---|---|
| `MAX_CONCURRENT_CHALLENGES_PER_PEER` | 10 | challenge suivant droppé (A7) |
| `MAX_PENDING_CHALLENGES` (global, côté A) | 256 | `initiate_presence_check` refuse (backpressure locale) |
| `MAX_STORED_ATTESTATIONS` (global) | 512 | attestation droppée + compteur métrique |

Mémoire pire cas : 512 × ~400 o ≈ 200 Ko — compatible Apple TV / appareils contraints.

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
| **T3** | Anti-replay (one-shot) | Attestation valide acceptée, puis rejouée | 1re acceptée + challenge consommé ; 2e droppée | `presence/tests.rs` |
| **T4** | Relay proof | Score LOCAL de B vu par A ≥ 2.0 | Attestation acceptée | `presence/relay_proof.rs:tests` |
| **T5** | Agrégation seed | N attestations (mêmes, ordres différents) | Seed identique, ordre-indépendant, SHA256 stable | `presence/aggregator.rs:tests` |
| **T6** | TTL purge | Challenge + 35s (`now` injecté, pas de sleep) | Challenge purgé, nouvelle tentative OK | `presence/tests.rs:cleanup` |

### 5.2 Tests adverses (Adversarial)

| # | Cas | Attaque | Défense testée | File:line |
|---|-----|---------|-----------------|-----------|
| **A1** | Forge signature | X signe avec clé falsifiée | Signature verify échoue → drop | `router.rs:tests` |
| **A2** | Replay attestation | X rejoue une attestation déjà acceptée | Challenge **one-shot** consommé à l'acceptation → 2e attestation droppée | `presence/tests.rs:replay` |
| **A3** | Delay attack | Réponse après 35s | Fraîcheur à l'**horloge de A** : `now_A − issued_at_A > 30s` → drop (le ts de B est ignoré) | `presence/tests.rs:delay_attack` |
| **A4** | Offline attester | A défie B offline, B ne répond jamais | Challenge expire à 30s (purge tick) | `runtime/loop.rs:timer` |
| **A5** | Sybil au score menteur | Sybil déclare `reliability_score = 10.0` dans le payload, score LOCAL vu par A = 0.5 | Gate lit UNIQUEMENT `role_manager.score()` local → drop ; le champ payload n'est jamais lu par la validation | `presence/tests.rs:lying_sybil` |
| **A6** | Seed sous contrôle du challenger | A tente de piloter le seed via les inputs | Inputs = **signatures Ed25519 de B** (non-forgeables par A) ; teste ordre-indépendance + « A ne peut pas produire un seed cible sans nouvelles signatures ». ⚠️ Le grinding par sous-ensemble reste OUVERT (documenté §3.2) → testé/fermé en L1-002 | `aggregator.rs:adversarial_tests` |
| **A7** | Challenge spam ciblé | X envoie mille Challenges à B | Limite 10 concurrent/pair + borne globale 256 (§4.3) | `presence/manager.rs:limits` |
| **A8** | Attestation usurpée | Relais M (qui voit le challenge en clair) répond à la place de B | `env.from == challenge.target` + `payload.attester_id == env.from` + nonce comparé à celui du challenge → drop | `presence/tests.rs:wrong_attester` |
| **A9** | Réflexion | X forge des challenges « from A » vers N nœuds → attestations concentrées sur A | Challenge **signé** : B vérifie la signature de A avant de signer quoi que ce soit | `presence/tests.rs:reflection` |
| **A10** | Épuisement mémoire distribué | 1000 pairs × 10 challenges chacun | Bornes globales `MAX_PENDING_CHALLENGES` / `MAX_STORED_ATTESTATIONS` (§4.3) | `presence/manager.rs:global_caps` |

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
| **2.1** | Lifecycle one-shot des challenges (pending/consume, purge u64) | 2 | 1.3 | consume unique, cleanup TTL vérifié avec `now` injecté |
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
    // A9 : vérifier la signature de A AVANT de dépenser une signature Ed25519.
    // Drops silencieux partout : répondre "invalid" à un attaquant = oracle gratuit.
    if env.verify_signature().is_err() {
        return vec![];
    }

    let payload = match PresenceChallengePayload::from_bytes(&env.payload) {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    // Cohérence identité : le challenger déclaré EST le signataire de l'envelope.
    if payload.challenger_id != env.from {
        return vec![];
    }

    if payload.validate(now).is_err() {
        return vec![];
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
    // Ordre des checks : du moins cher au plus cher ; drop silencieux partout (pas d'oracle).

    // 1. Challenge émis par nous, encore pendant (one-shot, pas encore consommé) ?
    let challenge = match self.presence_manager.pending(&extract_challenge_id(&env)) {
        Some(c) => c,
        None => return vec![],  // inconnu, expiré, ou déjà consommé (A2)
    };

    // 2. Fraîcheur à NOTRE horloge — le timestamp de B est ignoré (A3).
    if now.saturating_sub(challenge.issued_at) > 30_000 {
        return vec![];
    }

    // 3. L'attestation vient du nœud QUE NOUS AVONS DÉFIÉ (A8).
    //    Sans ce check, tout relais du chemin (payload en clair) répond à la place de B.
    if env.from != challenge.target {
        return vec![];
    }

    // 4. Signature Ed25519 de B sur l'envelope.
    if env.verify_signature().is_err() {
        return vec![];
    }

    let payload = match PresenceAttestationPayload::from_bytes(&env.payload) {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    // 5. Cohérences payload ↔ envelope ↔ challenge : identité + LE bon nonce.
    if payload.attester_id != env.from
        || payload.challenger_id != self.local_id
        || payload.nonce != challenge.nonce  // V1 ne comparait JAMAIS le nonce au challenge
    {
        return vec![];
    }

    // 6. Gate anti-Sybil : score LOCAL observé par nous (A5).
    //    JAMAIS payload.relay_proof.reliability_score (champ attaquant-contrôlé).
    if self.role_manager.score(&env.from, now) < RELAY_CONTRIBUTION_MIN {
        return vec![];
    }

    // 7. Accepter = CONSOMMER le challenge (one-shot → anti-replay structurel).
    self.presence_manager.consume_and_store(challenge.id, payload.clone(), &env.signature, now);
    
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
| NONCE_TTL | `crates/tom-protocol/src/router.rs` | 37 | Exemple TTL 24h (LRU borné) ; L1-001 n'a PAS de cache nonce séparé (one-shot §4.1), purge 30s |
| PROMOTION_THRESHOLD | `crates/tom-protocol/src/roles/manager.rs` | 13 | PROMOTION = 10.0 ; MIN = 2.0 (demotion) |
| RoleManager::score | `crates/tom-protocol/src/roles/manager.rs` | 60–65 | Fetch contribution score d'un peer |
| RouterAction | `crates/tom-protocol/src/router.rs` | 45–80 | Pattern ; L1-001 ajoute handler |
| RuntimeEffect | `crates/tom-protocol/src/runtime/effect.rs` | 14–66 | Pattern ; L1-001 ajoute variants |
| envelope.sign() | `crates/tom-protocol/src/envelope.rs` | 134–138 | Ed25519 signing pattern |
| envelope.verify_signature() | `crates/tom-protocol/src/envelope.rs` | 143 | Verification pattern |
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
- [ ] Agrégation seed sur SIGNATURES (jamais sur des bytes A-contrôlés) ; grinding par sous-ensemble documenté OUVERT → L1-002
- [ ] Challenge SIGNÉ + gate = score local uniquement + attestation liée à `challenge.target` + nonce comparé + one-shot
- [ ] Drops silencieux (aucun Reject réseau sur input adverse — pas d'oracle)
- [ ] Bornes mémoire globales (§4.3) en place
- [ ] Tests adverses couverts (10/10 cas)
- [ ] Tom-stress campaign ready (100 nodes pattern)
- [ ] FFI check (no export de PresenceManager to Swift yet) → FFI L1-002

---

## Statut d'implémentation (2026-07-06)

**✅ IMPLÉMENTÉ ET VALIDÉ EN RÉSEAU RÉEL** (même session que la spec V2) :

- Module `crates/tom-protocol/src/presence/` (mod + attestation + aggregator), câblé
  dans types/router-dispatch/state/loop/commands. Patch mono-crate comme prévu.
- **Évidence relais réelle** : le gate lit le score local nourri par l'ACK
  `RelayForwarded` **signé** (`state.rs`, branche `RoutingAction::Ack`) — preuve
  cryptographique locale que le pair a relayé pour nous (anti-replay ACK déjà en place).
- Tests **runtime-level** `tests/presence_integration.rs` : 9/9 (happy path avec évidence
  relais réelle, A1 forge, A2 replay one-shot, A5 Sybil menteur, A7 budget répondeur,
  A8 usurpation on-path, A9 réflexion ×2, A10 caps mémoire, self-challenge no-op).
- Scénario **QUIC vivant** `tom-stress presence` (3 nœuds A↔B↔C, jamais A↔C) : **5/5** —
  évidence relais (score 6.00, gate ouvert), **10/10 attestations, médiane 7 ms**
  (budget M1.1 : 200 ms), gate anti-Sybil vérifié sur le réseau (attestation honnête d'un
  pair sans évidence → drop silencieux), fenêtre d'agrégation 10 + seed non trivial.
- Leçon réseau consignée dans le scénario : le PeerAnnounce gossip (role=Peer) écrase le
  hint Relay de la topologie → le hint est ré-upserté avant chaque send dans le scénario.
  En production c'est le pipeline de rôles (promotion score ≥ 10) qui installe le rôle.
- Fenêtres temporelles déterministes (T6/A3) : couvertes dans les tests du module avec
  horloge injectée (`u64`), conformément à la spec (pas d'`Instant` dans l'état).

## Next

**L1-002 — Entropie non-biaisable (Mur 3 — VDF)**

L1-001 produit un seed agrégé ; L1-002 ajoute une Verifiable Delay Function pour rendre impossible le grinding (re-tirage jusqu'à un quorum commode). Le seed devient entropie de sélection cascade (L1-003).
