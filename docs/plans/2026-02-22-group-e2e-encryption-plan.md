# Group E2E Encryption (Sender Keys) — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make group messages unreadable by the hub relay using Sender Keys (each member encrypts with their own symmetric key).

**Architecture:** Each member generates a Sender Key (XChaCha20-Poly1305 symmetric). The key is distributed to other members via the existing 1-to-1 encryption (X25519 DH). Messages contain ciphertext the hub cannot read. Rotation on member departure.

**Tech Stack:** XChaCha20-Poly1305 (chacha20poly1305 crate), existing `crypto::encrypt`/`crypto::decrypt` for key distribution, MessagePack serialization.

**Design doc:** `docs/plans/2026-02-22-group-e2e-encryption-design.md`

---

### Task 1: Symmetric group encryption primitives in `crypto.rs`

**Files:**
- Modify: `crates/tom-protocol/src/crypto.rs`

**Context:** The crypto module already has 1-to-1 encryption (ephemeral X25519 DH + XChaCha20-Poly1305). We need simpler symmetric-only helpers for group messages (no DH — the key is pre-shared).

**Step 1: Write the failing tests**

Add these tests at the bottom of the `#[cfg(test)] mod tests` block in `crypto.rs`:

```rust
#[test]
fn group_encrypt_decrypt_roundtrip() {
    let key = generate_sender_key();
    let plaintext = b"Hello group!";
    let (ciphertext, nonce) = encrypt_group_message(plaintext, &key);
    let decrypted = decrypt_group_message(&ciphertext, &nonce, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn group_encrypt_wrong_key_fails() {
    let key1 = generate_sender_key();
    let key2 = generate_sender_key();
    let (ciphertext, nonce) = encrypt_group_message(b"secret", &key1);
    let result = decrypt_group_message(&ciphertext, &nonce, &key2);
    assert!(result.is_err());
}

#[test]
fn group_encrypt_tampered_ciphertext_fails() {
    let key = generate_sender_key();
    let (mut ciphertext, nonce) = encrypt_group_message(b"secret", &key);
    ciphertext[0] ^= 0xFF;
    let result = decrypt_group_message(&ciphertext, &nonce, &key);
    assert!(result.is_err());
}

#[test]
fn generate_sender_key_is_random() {
    let k1 = generate_sender_key();
    let k2 = generate_sender_key();
    assert_ne!(k1, k2);
}

#[test]
fn group_encrypt_empty_payload() {
    let key = generate_sender_key();
    let (ciphertext, nonce) = encrypt_group_message(b"", &key);
    let decrypted = decrypt_group_message(&ciphertext, &nonce, &key).unwrap();
    assert_eq!(decrypted, b"");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p tom-protocol crypto::tests::group_encrypt -- --no-capture 2>&1 | head -20`
Expected: compilation errors (functions don't exist yet)

**Step 3: Implement the functions**

Add these public functions above the `#[cfg(test)]` block in `crypto.rs`:

```rust
/// Generate a random 32-byte Sender Key for group encryption.
pub fn generate_sender_key() -> [u8; 32] {
    use chacha20poly1305::aead::rand_core::{OsRng, RngCore};
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

/// Encrypt plaintext with a symmetric Sender Key (XChaCha20-Poly1305).
///
/// Returns (ciphertext, nonce). The ciphertext includes a 16-byte auth tag.
/// Uses a random 24-byte nonce (safe with XChaCha20).
pub fn encrypt_group_message(plaintext: &[u8], key: &[u8; 32]) -> (Vec<u8>, [u8; 24]) {
    use chacha20poly1305::aead::rand_core::{OsRng, RngCore};

    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .expect("XChaCha20-Poly1305 encryption with valid key never fails");

    (ciphertext, nonce_bytes)
}

/// Decrypt ciphertext with a symmetric Sender Key (XChaCha20-Poly1305).
pub fn decrypt_group_message(
    ciphertext: &[u8],
    nonce: &[u8; 24],
    key: &[u8; 32],
) -> Result<Vec<u8>, TomProtocolError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let xnonce = XNonce::from(*nonce);
    cipher
        .decrypt(&xnonce, ciphertext)
        .map_err(|_| TomProtocolError::Crypto("group message decryption failed".into()))
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p tom-protocol crypto::tests::group_encrypt`
Expected: 5 tests pass

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/crypto.rs
git commit -m "feat(crypto): add symmetric group encryption primitives"
```

---

### Task 2: New data structures in `group/types.rs`

**Files:**
- Modify: `crates/tom-protocol/src/group/types.rs`
- Modify: `crates/tom-protocol/src/group/mod.rs` (re-exports)
- Modify: `crates/tom-protocol/src/lib.rs` (re-exports)

**Context:** We need `SenderKeyEntry`, `EncryptedSenderKey`, `GroupMessageContent`, and a new `GroupPayload::SenderKeyDistribution` variant. We also need to modify `GroupMessage` to support both plaintext (backward compat) and encrypted modes.

**Step 1: Add new structures**

In `group/types.rs`, after the `LeaveReason` enum and before `GroupAction`, add:

```rust
// ── Sender Key Encryption ─────────────────────────────────────────────

/// A member's Sender Key — used to encrypt their outgoing group messages.
/// Other members store a copy to decrypt incoming messages from this sender.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SenderKeyEntry {
    /// Who generated this key.
    pub owner_id: NodeId,
    /// 32-byte XChaCha20-Poly1305 symmetric key.
    pub key: [u8; 32],
    /// Key version (incremented on rotation).
    pub epoch: u32,
    /// When this key was created (ms since epoch).
    pub created_at: u64,
}

/// A Sender Key encrypted for a specific recipient (1-to-1 encryption).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncryptedSenderKey {
    /// Who this encrypted key is for.
    pub recipient_id: NodeId,
    /// The Sender Key encrypted with recipient's public key (existing 1-to-1 crypto).
    pub encrypted_key: crate::crypto::EncryptedPayload,
}

/// Plaintext content inside an encrypted group message.
/// Serialized to MessagePack before encryption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupMessageContent {
    pub username: String,
    pub text: String,
}
```

**Step 2: Add `SenderKeyDistribution` to `GroupPayload`**

In the `GroupPayload` enum, add a new variant after `HubHeartbeat`:

```rust
    /// Distribution of a member's Sender Key to other members.
    /// Sent after joining a group or when rotating keys (member departure).
    SenderKeyDistribution {
        group_id: GroupId,
        from: NodeId,
        epoch: u32,
        encrypted_keys: Vec<EncryptedSenderKey>,
    },
```

**Step 3: Modify `GroupMessage` to support encryption**

Replace the `GroupMessage` struct with:

```rust
/// A single message in a group conversation.
///
/// Supports both plaintext (legacy) and encrypted (Sender Key) modes.
/// When `encrypted == true`, `ciphertext`/`nonce`/`key_epoch` are used.
/// When `encrypted == false`, `text` is used (backward compat).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupMessage {
    pub group_id: GroupId,
    pub message_id: String,
    pub sender_id: NodeId,
    /// Plaintext (only set when encrypted == false).
    #[serde(default)]
    pub sender_username: String,
    /// Plaintext (only set when encrypted == false).
    #[serde(default)]
    pub text: String,
    /// Ciphertext: encrypted {username, text} (only set when encrypted == true).
    #[serde(default)]
    pub ciphertext: Vec<u8>,
    /// Nonce for ciphertext decryption.
    #[serde(default)]
    pub nonce: [u8; 24],
    /// Which epoch of the sender's key was used.
    #[serde(default)]
    pub key_epoch: u32,
    /// Whether this message is encrypted with a Sender Key.
    #[serde(default)]
    pub encrypted: bool,
    pub sent_at: u64,
    /// Ed25519 signature over `signing_bytes()`. Empty if unsigned.
    #[serde(default)]
    pub sender_signature: Vec<u8>,
}
```

**Step 4: Update `GroupMessage::new()` (plaintext, backward compat)**

```rust
impl GroupMessage {
    /// Create a new plaintext group message (unsigned, unencrypted).
    pub fn new(
        group_id: GroupId,
        sender_id: NodeId,
        sender_username: String,
        text: String,
    ) -> Self {
        Self {
            group_id,
            message_id: uuid::Uuid::new_v4().to_string(),
            sender_id,
            sender_username,
            text,
            ciphertext: Vec::new(),
            nonce: [0u8; 24],
            key_epoch: 0,
            encrypted: false,
            sent_at: now_ms(),
            sender_signature: Vec::new(),
        }
    }

    /// Create a new encrypted group message.
    ///
    /// Encrypts `{username, text}` with the sender's group key.
    /// The `sender_username` and `text` fields are left empty (data is in ciphertext).
    pub fn new_encrypted(
        group_id: GroupId,
        sender_id: NodeId,
        username: String,
        text: String,
        sender_key: &[u8; 32],
        key_epoch: u32,
    ) -> Self {
        let content = GroupMessageContent { username, text };
        let content_bytes = rmp_serde::to_vec(&content).expect("content serialization");
        let (ciphertext, nonce) = crate::crypto::encrypt_group_message(&content_bytes, sender_key);

        Self {
            group_id,
            message_id: uuid::Uuid::new_v4().to_string(),
            sender_id,
            sender_username: String::new(),
            text: String::new(),
            ciphertext,
            nonce,
            key_epoch,
            encrypted: true,
            sent_at: now_ms(),
            sender_signature: Vec::new(),
        }
    }

    /// Decrypt this message's ciphertext using the sender's key.
    /// Returns the decrypted content (username + text).
    /// Fails if the key is wrong or ciphertext is tampered.
    pub fn decrypt(&self, sender_key: &[u8; 32]) -> Result<GroupMessageContent, crate::TomProtocolError> {
        if !self.encrypted {
            return Ok(GroupMessageContent {
                username: self.sender_username.clone(),
                text: self.text.clone(),
            });
        }
        let plaintext_bytes = crate::crypto::decrypt_group_message(&self.ciphertext, &self.nonce, sender_key)?;
        rmp_serde::from_slice(&plaintext_bytes).map_err(|e| {
            crate::TomProtocolError::Deserialization(format!("group message content: {e}"))
        })
    }
```

**Step 5: Update `signing_bytes()` to support both modes**

Replace the existing `signing_bytes` method:

```rust
    /// Deterministic bytes for signing.
    ///
    /// For encrypted messages: signs over ciphertext (not plaintext).
    /// For plaintext messages: signs over text (backward compat).
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.group_id.0.as_bytes());
        buf.extend_from_slice(self.message_id.as_bytes());
        buf.extend_from_slice(&self.sender_id.as_bytes());
        if self.encrypted {
            buf.extend_from_slice(&self.ciphertext);
            buf.extend_from_slice(&self.nonce);
            buf.extend_from_slice(&self.key_epoch.to_le_bytes());
        } else {
            buf.extend_from_slice(self.text.as_bytes());
        }
        buf.extend_from_slice(&self.sent_at.to_le_bytes());
        buf
    }
```

The `sign()`, `verify_signature()`, and `is_signed()` methods remain unchanged (they use `signing_bytes()`).

**Step 6: Add re-exports**

In `group/mod.rs`, add to the re-exports:

```rust
pub use types::{
    // ... existing ...
    EncryptedSenderKey, GroupMessageContent, SenderKeyEntry,
};
```

In `lib.rs`, update the `pub use group::` block to include the new types:

```rust
pub use group::{
    elect_hub, ElectionReason, ElectionResult, EncryptedSenderKey, GroupAction, GroupEvent,
    GroupHub, GroupId, GroupInfo, GroupInvite, GroupMember, GroupManager, GroupMemberRole,
    GroupMessage, GroupMessageContent, GroupPayload, LeaveReason, SenderKeyEntry,
};
```

**Step 7: Run all tests**

Run: `cargo test -p tom-protocol`

Expected: All existing tests pass. The `GroupMessage::new()` constructor is backward-compatible. The `text` field in existing tests still works because `encrypted == false` by default.

**Critical check:** The `GroupPayload` roundtrip test in `types.rs` must still pass — `serde(default)` on the new fields ensures old serialized messages deserialize correctly.

**Step 8: Add tests for new structures**

Add to the `#[cfg(test)] mod tests` block in `group/types.rs`:

```rust
#[test]
fn sender_key_entry_roundtrip() {
    let entry = SenderKeyEntry {
        owner_id: node_id(1),
        key: [42u8; 32],
        epoch: 1,
        created_at: 1000,
    };
    let bytes = rmp_serde::to_vec(&entry).expect("serialize");
    let decoded: SenderKeyEntry = rmp_serde::from_slice(&bytes).expect("deserialize");
    assert_eq!(entry, decoded);
}

#[test]
fn group_message_content_roundtrip() {
    let content = GroupMessageContent {
        username: "alice".into(),
        text: "Hello!".into(),
    };
    let bytes = rmp_serde::to_vec(&content).expect("serialize");
    let decoded: GroupMessageContent = rmp_serde::from_slice(&bytes).expect("deserialize");
    assert_eq!(content, decoded);
}

#[test]
fn encrypted_group_message_roundtrip() {
    let seed = secret_seed(1);
    let sender = node_id(1);
    let key = [7u8; 32];

    let mut msg = GroupMessage::new_encrypted(
        GroupId::from("grp-enc".to_string()),
        sender,
        "alice".into(),
        "Secret message".into(),
        &key,
        1,
    );
    assert!(msg.encrypted);
    assert!(msg.text.is_empty(), "plaintext should be empty");
    assert!(!msg.ciphertext.is_empty(), "ciphertext should be set");

    // Sign
    msg.sign(&seed);
    assert!(msg.verify_signature());

    // MessagePack roundtrip
    let bytes = rmp_serde::to_vec(&msg).expect("serialize");
    let decoded: GroupMessage = rmp_serde::from_slice(&bytes).expect("deserialize");
    assert!(decoded.verify_signature());
    assert!(decoded.encrypted);

    // Decrypt
    let content = decoded.decrypt(&key).unwrap();
    assert_eq!(content.username, "alice");
    assert_eq!(content.text, "Secret message");
}

#[test]
fn encrypted_group_message_wrong_key_fails() {
    let key1 = [7u8; 32];
    let key2 = [8u8; 32];
    let msg = GroupMessage::new_encrypted(
        GroupId::from("grp-enc".to_string()),
        node_id(1),
        "alice".into(),
        "Secret".into(),
        &key1,
        1,
    );
    let result = msg.decrypt(&key2);
    assert!(result.is_err());
}

#[test]
fn plaintext_message_decrypt_returns_content() {
    let msg = GroupMessage::new(
        GroupId::from("grp-1".to_string()),
        node_id(1),
        "alice".into(),
        "Plain text".into(),
    );
    let content = msg.decrypt(&[0u8; 32]).unwrap();
    assert_eq!(content.username, "alice");
    assert_eq!(content.text, "Plain text");
}

#[test]
fn sender_key_distribution_roundtrip() {
    let payload = GroupPayload::SenderKeyDistribution {
        group_id: GroupId::from("grp-1".to_string()),
        from: node_id(1),
        epoch: 1,
        encrypted_keys: vec![],
    };
    let bytes = rmp_serde::to_vec(&payload).expect("serialize");
    let decoded: GroupPayload = rmp_serde::from_slice(&bytes).expect("deserialize");
    assert_eq!(payload, decoded);
}
```

**Step 9: Run all tests again**

Run: `cargo test -p tom-protocol`
Expected: All tests pass (existing + new).

**Step 10: Commit**

```bash
git add crates/tom-protocol/src/group/types.rs crates/tom-protocol/src/group/mod.rs crates/tom-protocol/src/lib.rs
git commit -m "feat(group): add Sender Key types and encrypted GroupMessage"
```

---

### Task 3: Sender Key management in `GroupManager`

**Files:**
- Modify: `crates/tom-protocol/src/group/manager.rs`

**Context:** The GroupManager needs to:
1. Store sender keys for each group (ours + other members')
2. Generate and distribute our sender key when we join/create
3. Handle incoming sender key distributions
4. Buffer messages when we don't have the sender's key yet
5. Rotate keys when a member leaves

**Step 1: Add sender key storage fields**

Add to the `GroupManager` struct:

```rust
pub struct GroupManager {
    local_id: NodeId,
    local_username: String,
    groups: HashMap<GroupId, GroupInfo>,
    pending_invites: HashMap<GroupId, GroupInvite>,
    message_history: HashMap<GroupId, Vec<GroupMessage>>,
    max_history_per_group: usize,
    /// Our own sender keys per group (what we use to encrypt outgoing messages).
    local_sender_keys: HashMap<GroupId, SenderKeyEntry>,
    /// Other members' sender keys per group (group_id → (sender_id → entry)).
    sender_keys: HashMap<GroupId, HashMap<NodeId, SenderKeyEntry>>,
    /// Messages waiting for a sender key to decrypt.
    pending_decrypt: HashMap<GroupId, Vec<GroupMessage>>,
}
```

Update `new()` to initialize the new fields:

```rust
pub fn new(local_id: NodeId, local_username: String) -> Self {
    Self {
        local_id,
        local_username,
        groups: HashMap::new(),
        pending_invites: HashMap::new(),
        message_history: HashMap::new(),
        max_history_per_group: MAX_SYNC_MESSAGES,
        local_sender_keys: HashMap::new(),
        sender_keys: HashMap::new(),
        pending_decrypt: HashMap::new(),
    }
}
```

**Step 2: Add sender key generation and distribution methods**

```rust
// ── Sender Key Management ─────────────────────────────────────────

/// Get our current sender key for a group (if we have one).
pub fn local_sender_key(&self, group_id: &GroupId) -> Option<&SenderKeyEntry> {
    self.local_sender_keys.get(group_id)
}

/// Get a member's sender key for a group.
pub fn get_sender_key(&self, group_id: &GroupId, sender_id: &NodeId) -> Option<&SenderKeyEntry> {
    self.sender_keys.get(group_id)?.get(sender_id)
}

/// Generate a new sender key for ourselves in this group.
/// Returns the SenderKeyEntry (caller is responsible for distribution).
fn generate_local_sender_key(&mut self, group_id: &GroupId) -> SenderKeyEntry {
    let old_epoch = self.local_sender_keys
        .get(group_id)
        .map(|e| e.epoch)
        .unwrap_or(0);

    let entry = SenderKeyEntry {
        owner_id: self.local_id,
        key: crate::crypto::generate_sender_key(),
        epoch: old_epoch + 1,
        created_at: now_ms(),
    };

    self.local_sender_keys.insert(group_id.clone(), entry.clone());
    entry
}

/// Build SenderKeyDistribution actions to send our key to all members.
///
/// Encrypts the 32-byte key individually for each member using 1-to-1 crypto.
pub fn build_sender_key_distribution(
    &self,
    group_id: &GroupId,
) -> Vec<GroupAction> {
    let Some(group) = self.groups.get(group_id) else {
        return vec![];
    };
    let Some(local_key) = self.local_sender_keys.get(group_id) else {
        return vec![];
    };

    let mut encrypted_keys = Vec::new();
    for member in &group.members {
        if member.node_id == self.local_id {
            continue;
        }
        let recipient_pk = member.node_id.as_bytes();
        match crate::crypto::encrypt(&local_key.key, &recipient_pk) {
            Ok(encrypted) => {
                encrypted_keys.push(EncryptedSenderKey {
                    recipient_id: member.node_id,
                    encrypted_key: encrypted,
                });
            }
            Err(e) => {
                tracing::warn!("sender key encrypt for {} failed: {e}", member.node_id);
            }
        }
    }

    if encrypted_keys.is_empty() {
        return vec![];
    }

    // Send to hub for fan-out
    vec![GroupAction::Send {
        to: group.hub_relay_id,
        payload: GroupPayload::SenderKeyDistribution {
            group_id: group_id.clone(),
            from: self.local_id,
            epoch: local_key.epoch,
            encrypted_keys,
        },
    }]
}

/// Handle an incoming SenderKeyDistribution — decrypt and store the sender's key.
pub fn handle_sender_key_distribution(
    &mut self,
    group_id: &GroupId,
    from: NodeId,
    epoch: u32,
    encrypted_keys: &[EncryptedSenderKey],
    local_secret_seed: &[u8; 32],
) -> Vec<GroupAction> {
    // Find our encrypted key in the bundle
    let Some(our_key) = encrypted_keys.iter().find(|ek| ek.recipient_id == self.local_id) else {
        return vec![]; // Not for us
    };

    // Decrypt the sender key using our Ed25519 secret
    let key_bytes = match crate::crypto::decrypt(&our_key.encrypted_key, local_secret_seed) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("sender key decrypt from {from} failed: {e}");
            return vec![];
        }
    };

    if key_bytes.len() != 32 {
        tracing::warn!("sender key from {from} has wrong length: {}", key_bytes.len());
        return vec![];
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);

    let entry = SenderKeyEntry {
        owner_id: from,
        key,
        epoch,
        created_at: now_ms(),
    };

    // Store the sender key
    self.sender_keys
        .entry(group_id.clone())
        .or_default()
        .insert(from, entry);

    // Try to decrypt any pending messages from this sender
    let mut actions = Vec::new();
    if let Some(pending) = self.pending_decrypt.get_mut(group_id) {
        let mut still_pending = Vec::new();
        for msg in pending.drain(..) {
            if msg.sender_id == from {
                // We now have the key — deliver
                actions.extend(self.deliver_decrypted_message(msg, &key));
            } else {
                still_pending.push(msg);
            }
        }
        *pending = still_pending;
    }

    actions
}

/// Try to decrypt and deliver a group message. If we don't have the sender's
/// key yet, buffer it for later.
fn try_decrypt_and_deliver(&mut self, message: GroupMessage) -> Vec<GroupAction> {
    if !message.encrypted {
        // Plaintext (legacy) — deliver directly
        return self.deliver_message(message);
    }

    let group_id = &message.group_id;
    let sender_id = &message.sender_id;

    // Look up sender's key
    if let Some(sender_key) = self.sender_keys
        .get(group_id)
        .and_then(|keys| keys.get(sender_id))
    {
        let key = sender_key.key;
        self.deliver_decrypted_message(message, &key)
    } else {
        // Buffer — we'll decrypt when the key arrives
        self.pending_decrypt
            .entry(group_id.clone())
            .or_default()
            .push(message);
        vec![]
    }
}

/// Decrypt an encrypted message and deliver it (internal helper).
fn deliver_decrypted_message(&mut self, mut message: GroupMessage, key: &[u8; 32]) -> Vec<GroupAction> {
    match message.decrypt(key) {
        Ok(content) => {
            // Populate plaintext fields for display/storage
            message.sender_username = content.username;
            message.text = content.text;
            self.deliver_message(message)
        }
        Err(e) => {
            tracing::warn!("group message decrypt failed: {e}");
            vec![]
        }
    }
}

/// Deliver a message (store in history + emit event). Internal helper.
fn deliver_message(&mut self, message: GroupMessage) -> Vec<GroupAction> {
    let group_id = &message.group_id;
    if !self.groups.contains_key(group_id) {
        return vec![];
    }

    if let Some(group) = self.groups.get_mut(group_id) {
        group.last_activity_at = now_ms();
    }

    let history = self.message_history.entry(group_id.clone()).or_default();
    history.push(message.clone());

    if history.len() > self.max_history_per_group {
        let excess = history.len() - self.max_history_per_group;
        history.drain(..excess);
    }

    vec![GroupAction::Event(GroupEvent::MessageReceived(message))]
}

/// Rotate our sender key (called when a member leaves the group).
/// Returns distribution actions.
pub fn rotate_sender_key(&mut self, group_id: &GroupId) -> Vec<GroupAction> {
    if !self.groups.contains_key(group_id) {
        return vec![];
    }
    self.generate_local_sender_key(group_id);
    self.build_sender_key_distribution(group_id)
}

/// Clean up sender keys and pending messages when leaving a group.
fn cleanup_group_keys(&mut self, group_id: &GroupId) {
    self.local_sender_keys.remove(group_id);
    self.sender_keys.remove(group_id);
    self.pending_decrypt.remove(group_id);
}
```

**Step 3: Modify existing methods to integrate sender keys**

3a. **`handle_group_created`** — generate our sender key after group creation:

After `self.groups.insert(group_id, group.clone());`, add:
```rust
self.generate_local_sender_key(&group.group_id);
```

3b. **`handle_group_sync`** — generate our sender key after joining, return distribution actions:

After storing the group and messages, add sender key generation and return distribution:
```rust
let mut actions = vec![GroupAction::Event(GroupEvent::Joined {
    group_id: group_id.clone(),
    group_name,
})];
// Generate and distribute our sender key to existing members
self.generate_local_sender_key(&group_id);
actions.extend(self.build_sender_key_distribution(&group_id));
actions
```

3c. **`handle_message`** — replace direct delivery with `try_decrypt_and_deliver`:

Replace the entire body of `handle_message` with:
```rust
pub fn handle_message(&mut self, message: GroupMessage) -> Vec<GroupAction> {
    let group_id = &message.group_id;
    if !self.groups.contains_key(group_id) {
        return vec![];
    }
    self.try_decrypt_and_deliver(message)
}
```

3d. **`handle_member_joined`** — distribute our sender key to the new member:

After adding the member and before returning, add:
```rust
// Send our sender key to the new member
actions.extend(self.build_sender_key_distribution(group_id));
```

3e. **`handle_member_left`** — rotate our sender key:

After removing the member, add key rotation:
```rust
// Remove the departed member's sender key
if let Some(keys) = self.sender_keys.get_mut(group_id) {
    keys.remove(node_id);
}
// Rotate our key (forward secrecy)
actions.extend(self.rotate_sender_key(group_id));
```

3f. **`leave_group`** — clean up sender keys:

After removing the group, add:
```rust
self.cleanup_group_keys(group_id);
```

**Step 4: Add `handle_payload` dispatch for SenderKeyDistribution**

The `GroupManager` doesn't currently have a `handle_payload` — the dispatch is done in `RuntimeState`. We'll add handling in Task 5 (runtime integration). For now, we just ensure the methods are available.

**Step 5: Write unit tests**

Add at the bottom of the `#[cfg(test)] mod tests` block in `manager.rs`:

```rust
fn secret_seed(seed: u8) -> [u8; 32] {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    secret.to_bytes()
}

#[test]
fn sender_key_generated_on_group_create() {
    let mut mgr = make_manager();
    let hub = node_id(10);
    let group = make_test_group(node_id(1), hub);
    let gid = group.group_id.clone();

    mgr.handle_group_created(group);
    assert!(mgr.local_sender_key(&gid).is_some());
    assert_eq!(mgr.local_sender_key(&gid).unwrap().epoch, 1);
}

#[test]
fn sender_key_distribution_encrypts_for_members() {
    let mut mgr = make_manager();
    let hub = node_id(10);
    let mut group = make_test_group(node_id(1), hub);
    let bob = node_id(2);
    group.members.push(GroupMember {
        node_id: bob,
        username: "bob".into(),
        joined_at: 1000,
        role: GroupMemberRole::Member,
    });
    let gid = group.group_id.clone();
    mgr.handle_group_created(group);

    let actions = mgr.build_sender_key_distribution(&gid);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        GroupAction::Send { to, payload } => {
            assert_eq!(*to, hub);
            match payload {
                GroupPayload::SenderKeyDistribution { encrypted_keys, .. } => {
                    assert_eq!(encrypted_keys.len(), 1);
                    assert_eq!(encrypted_keys[0].recipient_id, bob);
                }
                _ => panic!("expected SenderKeyDistribution"),
            }
        }
        _ => panic!("expected Send"),
    }
}

#[test]
fn sender_key_decrypt_and_store() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let bob_seed = secret_seed(2);
    let hub = node_id(10);

    // Alice setup
    let mut alice_mgr = GroupManager::new(alice_id, "alice".into());
    let mut group = make_test_group(alice_id, hub);
    group.members.push(GroupMember {
        node_id: bob_id,
        username: "bob".into(),
        joined_at: 1000,
        role: GroupMemberRole::Member,
    });
    let gid = group.group_id.clone();
    alice_mgr.handle_group_created(group.clone());

    // Get Alice's distribution
    let dist_actions = alice_mgr.build_sender_key_distribution(&gid);
    let GroupAction::Send { payload: GroupPayload::SenderKeyDistribution { from, epoch, encrypted_keys, .. }, .. } = &dist_actions[0]
    else { panic!("expected distribution") };

    // Bob receives Alice's key
    let mut bob_mgr = GroupManager::new(bob_id, "bob".into());
    bob_mgr.handle_group_created(group);
    bob_mgr.handle_sender_key_distribution(&gid, *from, *epoch, encrypted_keys, &bob_seed);

    // Bob should now have Alice's sender key
    let alice_key = bob_mgr.get_sender_key(&gid, &alice_id);
    assert!(alice_key.is_some());
    assert_eq!(alice_key.unwrap().key, alice_mgr.local_sender_key(&gid).unwrap().key);
}

#[test]
fn encrypted_message_delivered_after_key_arrives() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let bob_seed = secret_seed(2);
    let hub = node_id(10);

    let mut bob_mgr = GroupManager::new(bob_id, "bob".into());
    let group = make_test_group(alice_id, hub);
    let gid = group.group_id.clone();
    bob_mgr.handle_group_created(group);

    // Alice sends encrypted message, but Bob doesn't have the key yet
    let alice_key = [42u8; 32];
    let msg = GroupMessage::new_encrypted(
        gid.clone(),
        alice_id,
        "alice".into(),
        "Secret!".into(),
        &alice_key,
        1,
    );

    let actions = bob_mgr.handle_message(msg);
    assert!(actions.is_empty(), "should buffer without key");
    assert_eq!(bob_mgr.message_history(&gid).len(), 0);

    // Now Alice's key arrives
    let encrypted_key = crate::crypto::encrypt(&alice_key, &bob_id.as_bytes()).unwrap();
    let actions = bob_mgr.handle_sender_key_distribution(
        &gid,
        alice_id,
        1,
        &[EncryptedSenderKey {
            recipient_id: bob_id,
            encrypted_key,
        }],
        &bob_seed,
    );

    // Buffered message should now be delivered
    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], GroupAction::Event(GroupEvent::MessageReceived(_))));
    assert_eq!(bob_mgr.message_history(&gid).len(), 1);
    assert_eq!(bob_mgr.message_history(&gid)[0].text, "Secret!");
}

#[test]
fn key_rotation_on_member_leave() {
    let mut mgr = make_manager();
    let hub = node_id(10);
    let bob = node_id(2);
    let mut group = make_test_group(node_id(1), hub);
    group.members.push(GroupMember {
        node_id: bob,
        username: "bob".into(),
        joined_at: 1000,
        role: GroupMemberRole::Member,
    });
    let gid = group.group_id.clone();
    mgr.handle_group_created(group);

    let old_key = mgr.local_sender_key(&gid).unwrap().key;

    // Bob leaves → key should rotate
    mgr.handle_member_left(&gid, &bob, "bob".into(), LeaveReason::Voluntary);

    let new_key = mgr.local_sender_key(&gid).unwrap().key;
    assert_ne!(old_key, new_key, "key should have rotated");
    assert_eq!(mgr.local_sender_key(&gid).unwrap().epoch, 2);
}
```

**Step 6: Run tests**

Run: `cargo test -p tom-protocol group::manager`
Expected: All tests pass (existing + new).

**Step 7: Commit**

```bash
git add crates/tom-protocol/src/group/manager.rs
git commit -m "feat(group): add Sender Key management to GroupManager"
```

---

### Task 4: Hub handles `SenderKeyDistribution` fan-out

**Files:**
- Modify: `crates/tom-protocol/src/group/hub.rs`

**Context:** The hub needs to:
1. Fan out `SenderKeyDistribution` payloads to members (opaque — hub can't decrypt)
2. Stop reading message content (hub-blind for encrypted messages)

The hub's `handle_payload` already has a match on `GroupPayload` variants. We add the new variant.

**Step 1: Add `SenderKeyDistribution` to `handle_payload`**

In the `handle_payload` method match, add a new arm before the hub-outbound catch-all:

```rust
GroupPayload::SenderKeyDistribution { ref group_id, from, epoch: _, ref encrypted_keys } => {
    self.handle_sender_key_distribution(from, group_id, encrypted_keys)
}
```

**Step 2: Implement `handle_sender_key_distribution`**

```rust
fn handle_sender_key_distribution(
    &self,
    from: NodeId,
    group_id: &GroupId,
    encrypted_keys: &[EncryptedSenderKey],
) -> Vec<GroupAction> {
    let Some(hub_group) = self.groups.get(group_id) else {
        return vec![];
    };

    // Verify sender is a member
    if !hub_group.info.is_member(&from) {
        return vec![GroupAction::Event(GroupEvent::SecurityViolation {
            group_id: group_id.clone(),
            node_id: from,
            reason: "non-member sent sender key distribution".into(),
        })];
    }

    // Fan out to each recipient (only if they're actually members)
    let mut actions = Vec::new();
    for ek in encrypted_keys {
        if hub_group.info.is_member(&ek.recipient_id) {
            actions.push(GroupAction::Send {
                to: ek.recipient_id,
                payload: GroupPayload::SenderKeyDistribution {
                    group_id: group_id.clone(),
                    from,
                    epoch: 0, // Hub doesn't know the epoch — it's in the encrypted key
                    encrypted_keys: vec![ek.clone()],
                },
            });
        }
    }

    actions
}
```

**Note:** The hub sends each recipient only their own `EncryptedSenderKey` (not the full bundle). This is more efficient and avoids leaking the member list's key-exchange patterns.

**Step 3: Write tests**

Add to the `#[cfg(test)] mod tests` block in `hub.rs`:

```rust
#[test]
fn sender_key_distribution_fanout() {
    let mut hub = make_hub();
    let alice = node_id(1);
    let bob = node_id(2);
    let charlie = node_id(3);

    // Create group
    hub.handle_payload(
        GroupPayload::Create {
            group_name: "E2E".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice,
    );
    let gid = hub.groups.keys().next().unwrap().clone();
    hub.handle_join(bob, &gid, "bob".into());
    hub.handle_join(charlie, &gid, "charlie".into());

    // Alice distributes her sender key
    let actions = hub.handle_payload(
        GroupPayload::SenderKeyDistribution {
            group_id: gid.clone(),
            from: alice,
            epoch: 1,
            encrypted_keys: vec![
                EncryptedSenderKey {
                    recipient_id: bob,
                    encrypted_key: crate::crypto::EncryptedPayload {
                        ciphertext: vec![1, 2, 3],
                        nonce: [0u8; 24],
                        ephemeral_pk: [0u8; 32],
                    },
                },
                EncryptedSenderKey {
                    recipient_id: charlie,
                    encrypted_key: crate::crypto::EncryptedPayload {
                        ciphertext: vec![4, 5, 6],
                        nonce: [0u8; 24],
                        ephemeral_pk: [0u8; 32],
                    },
                },
            ],
        },
        alice,
    );

    // Should fan out to bob and charlie (one Send each)
    assert_eq!(actions.len(), 2);
    assert!(matches!(&actions[0], GroupAction::Send { to, .. } if *to == bob));
    assert!(matches!(&actions[1], GroupAction::Send { to, .. } if *to == charlie));
}

#[test]
fn sender_key_from_nonmember_rejected() {
    let mut hub = make_hub();
    let alice = node_id(1);
    let stranger = node_id(99);

    hub.handle_payload(
        GroupPayload::Create {
            group_name: "Secure".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice,
    );
    let gid = hub.groups.keys().next().unwrap().clone();

    let actions = hub.handle_payload(
        GroupPayload::SenderKeyDistribution {
            group_id: gid,
            from: stranger,
            epoch: 1,
            encrypted_keys: vec![],
        },
        stranger,
    );

    assert_eq!(actions.len(), 1);
    assert!(matches!(&actions[0], GroupAction::Event(GroupEvent::SecurityViolation { .. })));
}
```

**Step 4: Run tests**

Run: `cargo test -p tom-protocol group::hub`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add crates/tom-protocol/src/group/hub.rs
git commit -m "feat(group): hub fans out SenderKeyDistribution (blind relay)"
```

---

### Task 5: Runtime integration — encrypt on send, decrypt on receive, dispatch key distribution

**Files:**
- Modify: `crates/tom-protocol/src/runtime/state.rs`

**Context:** The RuntimeState needs to:
1. `handle_send_group_message` — encrypt with our sender key before sending to hub
2. `handle_incoming_group` — dispatch `SenderKeyDistribution` to GroupManager, pass secret seed
3. `handle_incoming_group` — when receiving a Message, GroupManager now handles decrypt internally
4. `group_actions_to_effects` — handle `SenderKeyDistribution` in action-to-effect conversion

**Step 1: Modify `handle_send_group_message` to use encryption**

Replace lines 690-724 of `state.rs` with:

```rust
pub fn handle_send_group_message(
    &mut self,
    group_id: crate::group::GroupId,
    text: String,
) -> Vec<RuntimeEffect> {
    let Some(group) = self.group_manager.get_group(&group_id) else {
        return vec![RuntimeEffect::Emit(ProtocolEvent::Error {
            description: format!("not a member of group {group_id}"),
        })];
    };

    let hub_id = group.hub_relay_id;

    // Build message — encrypted if we have a sender key, plaintext otherwise
    let mut msg = if let Some(sender_key) = self.group_manager.local_sender_key(&group_id) {
        let key = sender_key.key;
        let epoch = sender_key.epoch;
        GroupMessage::new_encrypted(
            group_id,
            self.local_id,
            self.config.username.clone(),
            text,
            &key,
            epoch,
        )
    } else {
        GroupMessage::new(
            group_id,
            self.local_id,
            self.config.username.clone(),
            text,
        )
    };

    msg.sign(&self.secret_seed);
    let payload = GroupPayload::Message(msg);
    let payload_bytes = rmp_serde::to_vec(&payload).expect("group msg serialization");

    let via = self.relay_selector.select_path(hub_id, &self.topology);
    let envelope = EnvelopeBuilder::new(
        self.local_id,
        hub_id,
        MessageType::GroupMessage,
        payload_bytes,
    )
    .via(via)
    .sign(&self.secret_seed);

    vec![RuntimeEffect::SendEnvelope(envelope)]
}
```

**Step 2: Add `SenderKeyDistribution` dispatch in `handle_incoming_group`**

In the `match group_payload` block, add a new arm for `SenderKeyDistribution`:

```rust
GroupPayload::SenderKeyDistribution {
    ref group_id,
    from,
    epoch,
    ref encrypted_keys,
} => {
    self.group_manager.handle_sender_key_distribution(
        group_id,
        from,
        epoch,
        encrypted_keys,
        &self.secret_seed,
    )
}
```

The hub also needs to process `SenderKeyDistribution` — already handled in the `Message` style dispatch. But `SenderKeyDistribution` is always hub-bound (from sender to hub) or member-bound (from hub to recipients). The hub handles it in `handle_payload`. When it reaches a non-hub node, it should go to GroupManager. So the dispatch is:

```rust
GroupPayload::SenderKeyDistribution {
    ref group_id,
    from,
    epoch,
    ref encrypted_keys,
} => {
    if self.group_hub.get_group(group_id).is_some() {
        // We're the hub — fan out to members
        self.group_hub.handle_payload(group_payload, envelope.from)
    } else {
        // We're a member — store the sender key
        self.group_manager.handle_sender_key_distribution(
            group_id,
            from,
            epoch,
            encrypted_keys,
            &self.secret_seed,
        )
    }
}
```

**Step 3: Write RuntimeState tests**

Add to the `#[cfg(test)] mod tests` block in `state.rs`:

```rust
#[test]
fn send_group_message_encrypts_when_key_exists() {
    let (mut state, _seed) = make_test_state();
    let hub_id = setup_group_in_state(&mut state);

    // State should have a sender key after group creation
    let gid = state.group_manager.all_groups()[0].group_id.clone();
    assert!(state.group_manager.local_sender_key(&gid).is_some());

    let effects = state.handle_send_group_message(gid, "Hello encrypted!".into());
    assert_eq!(effects.len(), 1);

    // Verify the envelope contains an encrypted GroupMessage
    match &effects[0] {
        RuntimeEffect::SendEnvelope(env) => {
            let payload: GroupPayload = rmp_serde::from_slice(&env.payload).unwrap();
            match payload {
                GroupPayload::Message(msg) => {
                    assert!(msg.encrypted, "message should be encrypted");
                    assert!(msg.text.is_empty(), "plaintext should be empty");
                    assert!(!msg.ciphertext.is_empty(), "ciphertext should be set");
                }
                _ => panic!("expected Message"),
            }
        }
        _ => panic!("expected SendEnvelope"),
    }
}
```

**Note:** The test helper `make_test_state` and `setup_group_in_state` may need to be created or adapted. The implementer should check what test helpers already exist in `state.rs` and adapt.

**Step 4: Run all tests**

Run: `cargo test -p tom-protocol`
Expected: All tests pass.

**Step 5: Run clippy**

Run: `cargo clippy -p tom-protocol -- -D warnings`
Expected: Zero warnings.

**Step 6: Commit**

```bash
git add crates/tom-protocol/src/runtime/state.rs
git commit -m "feat(runtime): integrate group E2E encryption in state handlers"
```

---

### Task 6: Integration test — full encrypted group lifecycle

**Files:**
- Modify: `crates/tom-protocol/tests/group_integration.rs`

**Context:** Test the entire encrypted flow: create → join → key exchange → send encrypted → receive decrypted. This test simulates 3 members communicating through a hub, with the hub never seeing plaintext.

**Step 1: Write the integration test**

Add to `group_integration.rs`:

```rust
/// Test E2E encrypted group messaging with Sender Keys.
///
/// Scenario: Alice creates group, Bob joins. They exchange sender keys.
/// Alice sends encrypted message. Bob decrypts. Hub never sees plaintext.
#[test]
fn encrypted_group_messaging_e2e() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let hub_id = node_id(10);
    let alice_seed = secret_seed(1);
    let bob_seed = secret_seed(2);

    let mut alice = GroupManager::new(alice_id, "alice".into());
    let mut bob = GroupManager::new(bob_id, "bob".into());
    let mut hub = GroupHub::new(hub_id);

    // ── Step 1: Alice creates group ──────────────────────
    let create_actions = alice.create_group("E2E Test".into(), hub_id, vec![bob_id]);
    let GroupAction::Send { payload, .. } = &create_actions[0] else { panic!() };
    let hub_actions = hub.handle_payload(payload.clone(), alice_id);

    // Deliver Created to Alice
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0] else { panic!() };
    let gid = group.group_id.clone();
    alice.handle_group_created(group.clone());

    // Alice should have a sender key
    assert!(alice.local_sender_key(&gid).is_some());

    // ── Step 2: Bob joins via invite ─────────────────────
    let GroupAction::Send { payload: GroupPayload::Invite { group_id, group_name, inviter_id, inviter_username }, .. } = &hub_actions[1] else { panic!() };
    bob.handle_invite(group_id.clone(), group_name.clone(), *inviter_id, inviter_username.clone(), hub_id);

    let join_actions = bob.accept_invite(&gid);
    let GroupAction::Send { payload: join_payload, .. } = &join_actions[0] else { panic!() };
    let hub_join_actions = hub.handle_payload(join_payload.clone(), bob_id);

    // Deliver Sync to Bob
    let GroupAction::Send { payload: GroupPayload::Sync { group: sync_group, recent_messages }, .. } = &hub_join_actions[0] else { panic!() };
    let bob_join_effects = bob.handle_group_sync(sync_group.clone(), recent_messages.clone());

    // Bob should have generated a sender key and want to distribute it
    assert!(bob.local_sender_key(&gid).is_some());
    // handle_group_sync returns [Joined event, SenderKeyDistribution Send]
    assert!(bob_join_effects.len() >= 2, "should have Joined + SenderKeyDistribution");

    // Deliver MemberJoined to Alice (also triggers Alice sending her key to Bob)
    let GroupAction::Broadcast { payload: GroupPayload::MemberJoined { member, .. }, .. } = &hub_join_actions[1] else { panic!() };
    let alice_member_actions = alice.handle_member_joined(&gid, member.clone());
    // Alice should distribute her key to Bob
    assert!(!alice_member_actions.is_empty(), "alice should distribute key to bob");

    // ── Step 3: Key exchange ─────────────────────────────

    // Alice → Bob: Alice's sender key
    let alice_dist = alice.build_sender_key_distribution(&gid);
    let GroupAction::Send { payload: GroupPayload::SenderKeyDistribution { from: a_from, epoch: a_epoch, encrypted_keys: a_keys, .. }, .. } = &alice_dist[0] else { panic!() };

    // Deliver Alice's key to Bob via hub
    let hub_dist = hub.handle_payload(
        GroupPayload::SenderKeyDistribution { group_id: gid.clone(), from: *a_from, epoch: *a_epoch, encrypted_keys: a_keys.clone() },
        alice_id,
    );
    // Hub fans out to Bob
    for action in &hub_dist {
        if let GroupAction::Send { to, payload: GroupPayload::SenderKeyDistribution { from, epoch, encrypted_keys, .. } } = action {
            if *to == bob_id {
                bob.handle_sender_key_distribution(&gid, *from, *epoch, encrypted_keys, &bob_seed);
            }
        }
    }

    // Bob → Alice: Bob's sender key
    let bob_dist = bob.build_sender_key_distribution(&gid);
    if !bob_dist.is_empty() {
        let GroupAction::Send { payload: GroupPayload::SenderKeyDistribution { from: b_from, epoch: b_epoch, encrypted_keys: b_keys, .. }, .. } = &bob_dist[0] else { panic!() };

        let hub_dist2 = hub.handle_payload(
            GroupPayload::SenderKeyDistribution { group_id: gid.clone(), from: *b_from, epoch: *b_epoch, encrypted_keys: b_keys.clone() },
            bob_id,
        );
        for action in &hub_dist2 {
            if let GroupAction::Send { to, payload: GroupPayload::SenderKeyDistribution { from, epoch, encrypted_keys, .. } } = action {
                if *to == alice_id {
                    alice.handle_sender_key_distribution(&gid, *from, *epoch, encrypted_keys, &alice_seed);
                }
            }
        }
    }

    // Verify both have each other's keys
    assert!(bob.get_sender_key(&gid, &alice_id).is_some(), "Bob should have Alice's key");
    assert!(alice.get_sender_key(&gid, &bob_id).is_some(), "Alice should have Bob's key");

    // ── Step 4: Alice sends encrypted message ────────────
    let alice_key = alice.local_sender_key(&gid).unwrap();
    let mut msg = GroupMessage::new_encrypted(
        gid.clone(),
        alice_id,
        "alice".into(),
        "Top secret message!".into(),
        &alice_key.key,
        alice_key.epoch,
    );
    msg.sign(&alice_seed);

    // Hub processes — should not be able to read content
    assert!(msg.text.is_empty(), "hub should not see plaintext");
    assert!(msg.encrypted, "message should be marked encrypted");

    let fanout = hub.handle_payload(GroupPayload::Message(msg.clone()), alice_id);
    let GroupAction::Broadcast { to, payload: GroupPayload::Message(fanned_msg) } = &fanout[0] else { panic!() };
    assert!(to.contains(&bob_id));

    // Bob receives and decrypts
    let bob_actions = bob.handle_message(fanned_msg.clone());
    assert_eq!(bob_actions.len(), 1);

    let GroupAction::Event(GroupEvent::MessageReceived(delivered)) = &bob_actions[0] else { panic!() };
    assert_eq!(delivered.text, "Top secret message!");
    assert_eq!(delivered.sender_username, "alice");
}

/// Test that an ex-member cannot decrypt messages after key rotation.
#[test]
fn key_rotation_forward_secrecy() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let eve_id = node_id(3);
    let hub_id = node_id(10);

    let mut alice = GroupManager::new(alice_id, "alice".into());
    let mut hub = GroupHub::new(hub_id);

    // Create group with Alice + Eve
    let create_actions = alice.create_group("Rotation Test".into(), hub_id, vec![]);
    let GroupAction::Send { payload, .. } = &create_actions[0] else { panic!() };
    let hub_actions = hub.handle_payload(payload.clone(), alice_id);
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0] else { panic!() };
    let gid = group.group_id.clone();
    alice.handle_group_created(group.clone());

    // Eve joins
    hub.handle_payload(GroupPayload::Join { group_id: gid.clone(), username: "eve".into() }, eve_id);
    alice.handle_member_joined(&gid, tom_protocol::GroupMember {
        node_id: eve_id,
        username: "eve".into(),
        joined_at: 1000,
        role: tom_protocol::GroupMemberRole::Member,
    });

    // Record Alice's key before Eve leaves
    let old_key = alice.local_sender_key(&gid).unwrap().key;

    // Eve leaves → Alice rotates her key
    hub.handle_leave(eve_id, &gid);
    alice.handle_member_left(&gid, &eve_id, "eve".into(), LeaveReason::Voluntary);

    // Alice's key should have rotated
    let new_key = alice.local_sender_key(&gid).unwrap().key;
    assert_ne!(old_key, new_key, "key should have rotated after member leave");

    // New encrypted message with rotated key
    let msg = GroupMessage::new_encrypted(
        gid.clone(),
        alice_id,
        "alice".into(),
        "Post-rotation secret".into(),
        &new_key,
        alice.local_sender_key(&gid).unwrap().epoch,
    );

    // Eve tries to decrypt with old key — should fail
    let result = msg.decrypt(&old_key);
    assert!(result.is_err(), "old key should not decrypt new message");
}
```

**Step 2: Run integration tests**

Run: `cargo test -p tom-protocol --test group_integration`
Expected: All tests pass (existing + new).

**Step 3: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests pass.

**Step 4: Run clippy on workspace**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

**Step 5: Commit**

```bash
git add crates/tom-protocol/tests/group_integration.rs
git commit -m "test(group): add E2E encrypted group messaging integration tests"
```

---

### Task 7: Final verification and cleanup

**Files:**
- All modified files (review)

**Step 1: Full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests pass, count should be ~300+ (was 289 before this feature).

**Step 2: Clippy clean**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings.

**Step 3: Verify backward compatibility**

Check that the existing `full_group_lifecycle` integration test still passes — it uses plaintext `GroupMessage::new()` which should still work (the `encrypted: false` default path).

**Step 4: Push to GitHub**

```bash
git push
```

---

## Summary of Changes

| File | Change |
|------|--------|
| `crypto.rs` | +3 functions: `generate_sender_key`, `encrypt_group_message`, `decrypt_group_message` |
| `group/types.rs` | +3 structs: `SenderKeyEntry`, `EncryptedSenderKey`, `GroupMessageContent`. Modified `GroupMessage` (encrypted fields). New `GroupPayload::SenderKeyDistribution` variant |
| `group/manager.rs` | +3 fields (sender_keys, local_sender_keys, pending_decrypt). +8 methods (key gen, distribution, decrypt, rotation, cleanup) |
| `group/hub.rs` | +1 method: `handle_sender_key_distribution` (blind fan-out) |
| `runtime/state.rs` | Modified `handle_send_group_message` (encrypt). Added `SenderKeyDistribution` dispatch |
| `group/mod.rs` | Updated re-exports |
| `lib.rs` | Updated re-exports |
| `tests/group_integration.rs` | +2 integration tests (E2E encrypted lifecycle, key rotation forward secrecy) |

## Test Coverage

| Test | What it verifies |
|------|-----------------|
| `group_encrypt_decrypt_roundtrip` | Symmetric crypto primitives |
| `group_encrypt_wrong_key_fails` | Wrong key rejected |
| `group_encrypt_tampered_ciphertext_fails` | Tamper detection |
| `sender_key_entry_roundtrip` | Serialization |
| `encrypted_group_message_roundtrip` | Encrypt → serialize → deserialize → decrypt |
| `encrypted_group_message_wrong_key_fails` | Key isolation |
| `plaintext_message_decrypt_returns_content` | Backward compat |
| `sender_key_generated_on_group_create` | Auto-generation |
| `sender_key_distribution_encrypts_for_members` | Distribution |
| `sender_key_decrypt_and_store` | Key exchange |
| `encrypted_message_delivered_after_key_arrives` | Buffering |
| `key_rotation_on_member_leave` | Rotation |
| `sender_key_distribution_fanout` | Hub blind relay |
| `sender_key_from_nonmember_rejected` | Security |
| `send_group_message_encrypts_when_key_exists` | Runtime integration |
| `encrypted_group_messaging_e2e` | Full lifecycle |
| `key_rotation_forward_secrecy` | Forward secrecy |
