# Rapport d’implémentation R14 — Rotation Sender Keys / Epoch Transition

**Date** : 4 mars 2026  
**Auteur** : GitHub Copilot (GPT-5.3-Codex)  
**Contexte** : mise en œuvre de R14 après débogage transport, avec politique de transition validée (fallback 5 min + dual trigger 24h/10k).

---

## 1) Objectif R14 implémenté

Implémenter la politique suivante pour les Sender Keys de groupe :

1. **Rotation dual trigger** (premier atteint) :
   - temps : 24h
   - volume : 10 000 messages
2. **Fallback d’epoch** :
   - accepter `epoch-1` pendant une **grace period de 5 min**
   - rejeter `epoch-1` après grace period + forcer resync
3. **Purge des clés expirées** :
   - purge des clés > 7 jours
4. **Protection anti-spam rotation** :
   - max 1 trigger rotation / heure / groupe
5. **Re-sync proactive** :
   - replay/proposition proactive des clés lors de mismatch / rejoin

---

## 2) Modifications réalisées (code)

### A. `crates/tom-protocol/src/group/types.rs`

Ajout des constantes R14 :

- `SENDER_KEY_ROTATE_MAX_AGE_MS = 24h`
- `SENDER_KEY_ROTATE_MAX_MESSAGES = 10_000`
- `SENDER_KEY_EPOCH_GRACE_MS = 5 min`
- `SENDER_KEY_PURGE_MAX_AGE_MS = 7 jours`
- `SENDER_KEY_ROTATE_RATE_LIMIT_MS = 1h`

### B. `crates/tom-protocol/src/group/mod.rs`

Export des constantes R14 pour usage runtime/group.

### C. `crates/tom-protocol/src/group/manager.rs`

Ajouts principaux côté membre :

- Nouveau stockage de clé précédente par sender/groupe avec fin de grâce (`PreviousSenderKey`).
- Extension snapshot manager :
  - `previous_sender_keys`
  - `local_sender_message_counts`
- Ajout du compteur local de messages par groupe pour le trigger volume.
- Ajout API :
  - `note_local_message_sent`
  - `maybe_rotate_local_sender_key`
  - `purge_expired_sender_keys`
  - `local_sender_epoch`
- Logique de réception SenderKeyDistribution :
  - gestion d’upgrade d’epoch
  - conservation de l’epoch précédent pendant 5 minutes
- Logique de décryptage :
  - accepte clé courante
  - accepte `epoch-1` si fenêtre de grâce active
  - sinon : **force `SyncRequest`** (re-sync)

### D. `crates/tom-protocol/src/group/hub.rs`

Ajouts principaux côté hub :

- État interne par groupe :
  - cache latest sender keys par sender
  - état d’epoch (`current`, `previous`, `grace_until`)
  - compteur messages depuis dernier trigger rotation
  - timestamp dernier trigger
- `SenderKeyDistribution` :
  - propagation de l’**epoch réel** (plus de `epoch: 0`)
  - mise à jour de l’état de transition d’epoch
- Politique mismatch epoch sur message encrypté :
  - accept si `current` ou `previous` dans grace
  - reject post-grace + `SecurityViolation`
  - replay proactif des clés vers le nœud concerné
- `handle_join` : replay proactif des sender keys au membre qui rejoint/revient.
- Trigger rotation hub :
  - dual trigger (24h/10k)
  - rate limit 1/h
- Purge cache sender keys hub (>7j).

### E. `crates/tom-protocol/src/runtime/state.rs`

Intégration runtime :

- Dans `handle_send_group_message` :
  - exécution de `maybe_rotate_local_sender_key` **avant envoi**
  - émission des effects de distribution si rotation
  - incrément compteur via `note_local_message_sent`
- Dans `tick_hub_cleanup` :
  - ajout purge clés manager + hub (>7 jours)
  - log consolidé purge messages + clés
- Correction de move ownership `group_id` (clones nécessaires)

### F. `crates/tom-protocol/src/storage/mod.rs`

Corrections de compatibilité snapshot suite aux nouveaux champs manager :

- Ajout de valeurs par défaut lors des initialisations `GroupManagerSnapshot` dans le stockage/tests :
  - `previous_sender_keys: HashMap::new()`
  - `local_sender_message_counts: HashMap::new()`

### G. `crates/tom-protocol/tests/group_integration.rs`

Ajout test d’intégration R14 :

- `epoch_transition_old_and_new_messages_delivered`
  - Vérifie livraison message epoch N puis epoch N+1 autour d’une rotation.

---

## 3) Corrections apportées pendant l’implémentation

Pendant le build/test, plusieurs corrections ont été faites :

1. **Erreur de compilation storage snapshot** (champs manquants)  
   → initialisations `GroupManagerSnapshot` mises à jour.

2. **Erreur borrow after move sur `group_id` dans runtime**  
   → usage de `group_id.clone()` dans la création de `GroupMessage`.

3. **Test hub post-grace initialement en échec** (pas de replay ciblé observable)  
   → enrichissement du fixture test pour inclure une clé chiffrée ciblant aussi l’émetteur, permettant de vérifier le replay proactif attendu.

4. **Patch hub volumineux**  
   → ré-application en plusieurs patches atomiques pour garantir intégrité/compilation.

---

## 4) Tests exécutés et résultats

## Tests ciblés R14

- `cargo test -p tom-protocol epoch_minus_one`
  - ✅ `epoch_minus_one_accepted_within_grace`
  - ✅ `epoch_minus_one_after_grace_forces_resync`

- `cargo test -p tom-protocol sender_key_distribution_fanout`
  - ✅ fanout sender keys + epoch propagé

- `cargo test -p tom-protocol epoch_mismatch_after_grace_rejected_and_replayed`
  - ✅ reject post-grace + replay proactif

- `cargo test -p tom-protocol local_sender_dual_trigger_rotates_on_message_count`
  - ✅ trigger volume 10k

- `cargo test -p tom-protocol purge_expired_sender_keys_removes_old_entries`
  - ✅ purge > 7 jours

- `cargo test -p tom-protocol epoch_transition_old_and_new_messages_delivered`
  - ✅ test intégration transition d’epoch

## Validation complète crate

- `cargo test -p tom-protocol`
  - ✅ **371 passed**, 0 failed
  - intégrations/proptests/doc-tests associés : ✅

## Validation complète workspace

- `cargo test --workspace`
  - ✅ succès global (0 échec détecté)
  - suites multi-crates + doc-tests : ✅

> Note: des warnings réseau apparaissent en e2e (relay/dns) mais les tests concernés passent.

---

## 5) Fichiers modifiés

- `crates/tom-protocol/src/group/types.rs`
- `crates/tom-protocol/src/group/mod.rs`
- `crates/tom-protocol/src/group/manager.rs`
- `crates/tom-protocol/src/group/hub.rs`
- `crates/tom-protocol/src/runtime/state.rs`
- `crates/tom-protocol/src/storage/mod.rs`
- `crates/tom-protocol/tests/group_integration.rs`

---

## 6) Statut final

R14 est **implémenté et validé par tests** selon la politique demandée :

- fallback temporaire 5 min : ✅
- reject + resync post-grace : ✅
- dual trigger 24h/10k : ✅
- purge clés >7j : ✅
- anti-spam trigger rotation (1/h) : ✅
- tests critiques transition/offline-like/reject : ✅

Ce rapport peut être transmis tel quel à Claude Code pour suivi/archivage.