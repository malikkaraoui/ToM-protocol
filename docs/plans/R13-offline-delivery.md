# R13 — Offline Delivery

## Probleme

Quand un destinataire est offline :

| Scenario | Aujourd'hui | Impact |
|----------|------------|--------|
| **1-to-1** | Backup virus (24h TTL, replication) | OK |
| **Groupe, membre offline** | Hub broadcast fire-and-forget (`SendEnvelope`) | **Messages perdus** |
| **Groupe, rejoin** | Sync des 100 derniers messages (VecDeque en memoire) | **Insuffisant** |
| **Hub restart** | `message_history: VecDeque::new()` au restore | **Historique perdu** |

Le trou critique : les messages de groupe pour les membres offline sont **silencieusement perdus**. Le hub utilise `SendEnvelope` (pas de backup), et le buffer ne garde que 100 messages en memoire volatile.

## Solution : Numeros de sequence + historique persistant

### Principe

1. Le hub assigne un **numero de sequence monotone** a chaque message de groupe
2. L'historique est **persiste en SQLite** (pas en memoire VecDeque)
3. Chaque membre traque son `last_seq` par groupe
4. Au rejoin, le membre demande les messages depuis son `last_seq`
5. Le hub repond avec les messages manquants (dans la limite du TTL 24h)

### Pourquoi ce design

- **Pas de tracking per-member au hub** : le hub reste leger (pas de "delivery matrix")
- **Le membre sait ce qu'il a manque** : il connait son `last_seq`, il demande le delta
- **TTL 24h respecte** : les messages expires sont purges, pas d'historique infini
- **Compatible avec le backup virus** : les deux systemes sont complementaires (1-to-1 = backup, groupe = seq+persist)

---

## R13.1 — Sequence numbers sur les messages de groupe

### Changements

**`group/hub.rs` — `HubGroup`** :
```rust
struct HubGroup {
    info: GroupInfo,
    message_history: VecDeque<GroupMessage>,  // garde pour compat
    next_seq: u64,                            // NOUVEAU: compteur monotone
    // ... rate_limits, seen_message_ids, etc.
}
```

**`group/types.rs` — `GroupMessage`** :
```rust
pub struct GroupMessage {
    // ... champs existants ...
    #[serde(default)]
    pub seq: u64,     // NOUVEAU: assigne par le hub (0 = pas encore assigne)
}
```

**`group/hub.rs` — `handle_message()`** :
- Avant le broadcast, le hub assigne `msg.seq = hub_group.next_seq; hub_group.next_seq += 1;`
- Le seq est inclus dans le message broadcaste (les membres le recoivent)

**Tests** : 3-4 tests unitaires
- `hub_assigns_monotonic_seq`
- `seq_survives_serialization`
- `seq_increments_across_messages`

---

## R13.2 — Persistance de l'historique hub en SQLite

### Schema V4

```sql
CREATE TABLE IF NOT EXISTS hub_message_history (
    group_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    message_data BLOB NOT NULL,
    stored_at INTEGER NOT NULL,
    PRIMARY KEY (group_id, seq)
);

CREATE INDEX idx_hub_history_expire ON hub_message_history (stored_at);
```

### Changements

**`storage/schema.rs`** — `migrate_v4()` : cree la table + ajoute `next_seq` a `hub_groups`

**`storage/mod.rs`** :
- `save_hub_message()` : insere un message (appele apres chaque `handle_message`)
- `load_hub_messages_since(group_id, since_seq)` : charge les messages > since_seq
- `cleanup_expired_hub_messages(cutoff_ms)` : purge les messages TTL expire
- `save_hub_next_seq(group_id, next_seq)` : persiste le compteur

**`group/hub.rs`** :
- `HubGroup.message_history` reste en memoire (cache rapide pour les joins recents)
- Mais la source de verite est SQLite (pour les gaps longs)

**`runtime/state.rs`** :
- `tick_hub_cleanup()` : toutes les 60s, purge les messages > 24h de SQLite
- Integration avec le save_state() periodique (30s)

### Interaction avec `GroupHubSnapshot`

`GroupHubSnapshot` est enrichi :
```rust
pub struct GroupHubSnapshot {
    pub groups: HashMap<GroupId, GroupInfo>,
    pub invited_sets: HashMap<GroupId, HashSet<NodeId>>,
    pub next_seqs: HashMap<GroupId, u64>,  // NOUVEAU
}
```

Le `message_history` n'est PAS dans le snapshot — il est lu directement depuis SQLite.

**Tests** : 3-4 tests
- `hub_messages_persisted_to_sqlite`
- `hub_messages_survive_restart`
- `expired_messages_cleaned_up`

---

## R13.3 — Gap-fill au rejoin (membre)

### Changements

**`group/manager.rs`** :
```rust
struct GroupState {
    // ... champs existants ...
    last_seq: u64,  // NOUVEAU: dernier seq recu pour ce groupe
}
```

- A chaque `GroupMessage` recu, `last_seq = max(last_seq, msg.seq)`
- Les messages avec `seq <= last_seq` sont ignores (dedup gratuit)

**`group/types.rs` — `GroupPayload`** :
```rust
enum GroupPayload {
    // ... variantes existantes ...

    /// Member requests missed messages since a sequence number.
    SyncRequest {
        group_id: GroupId,
        since_seq: u64,
    },

    /// Hub responds with missed messages.
    SyncResponse {
        group_id: GroupId,
        messages: Vec<GroupMessage>,
        latest_seq: u64,
    },
}
```

**`group/hub.rs`** :
- `handle_sync_request(from, group_id, since_seq)` :
  - Charge les messages depuis SQLite (ou cache memoire si recent)
  - Repond avec `SyncResponse { messages, latest_seq }`
  - Limite : max 500 messages par reponse (pagination si besoin)

**`group/manager.rs` — `rejoin_groups()`** :
- Quand le runtime rejoint un groupe existant, il envoie `SyncRequest { since_seq: last_seq }`
- A la reception de `SyncResponse`, traite les messages manques dans l'ordre du seq

**`runtime/state.rs`** — `handle_join() existant modifie` :
- Le `Sync` existant (rejoin d'un membre) inclut deja `recent_messages`
- On enrichit : si le membre envoie `since_seq`, on charge depuis SQLite au lieu du VecDeque

### Nouveau MessageType

```rust
MessageType::GroupSyncRequest   // membre → hub
MessageType::GroupSyncResponse  // hub → membre
```

**Tests** : 5-6 tests
- `member_tracks_last_seq`
- `rejoin_sends_sync_request_with_last_seq`
- `hub_responds_with_missed_messages`
- `duplicate_seq_ignored`
- `sync_request_for_unknown_group_ignored`
- `sync_response_limit_500`

---

## R13.4 — Persistance du `last_seq` (membre)

### Changements

**`storage/schema.rs`** — ajoute dans `migrate_v4()` :
```sql
ALTER TABLE groups ADD COLUMN last_seq INTEGER NOT NULL DEFAULT 0;
```

**`storage/mod.rs`** :
- `GroupManagerSnapshot.last_seqs: HashMap<GroupId, u64>` ajoute
- Save/load du `last_seq` par groupe

**`group/manager.rs`** — `snapshot()` et `restore()` incluent `last_seq`

**Tests** : 2 tests
- `last_seq_persisted`
- `last_seq_restored_after_restart`

---

## R13.5 — Purge TTL 24h cote hub

### Changements

**`runtime/state.rs`** :
- Nouveau tick `tick_hub_message_cleanup()` — toutes les 60s
- Supprime de SQLite : `DELETE FROM hub_message_history WHERE stored_at < (now - 24h)`
- Supprime du VecDeque memoire les messages expires

**`group/hub.rs`** :
- `cleanup_expired_messages(cutoff_ms)` — purge le VecDeque et retourne les IDs supprimes
- Appele par le tick runtime

**Tests** : 2 tests
- `expired_messages_purged_from_hub`
- `non_expired_messages_kept`

---

## Ordre d'implementation

| Step | Sous-phase | Effort | Dependances |
|------|-----------|--------|-------------|
| 1 | **R13.1** — seq numbers | petit | aucune |
| 2 | **R13.2** — SQLite hub history | moyen | R13.1 |
| 3 | **R13.4** — persist last_seq membre | petit | R13.1 |
| 4 | **R13.3** — gap-fill SyncRequest/Response | moyen | R13.1 + R13.2 + R13.4 |
| 5 | **R13.5** — purge TTL | petit | R13.2 |

Total estime : ~15 tests nouveaux, ~400-500 lignes de code

## Ce qui ne change PAS

- Le backup virus (1-to-1) reste identique
- `SendWithBackupFallback` pour les messages directs — inchange
- Le hub reste "fire-and-forget" pour le broadcast — c'est le **membre** qui demande le rattrapage
- Le hub ne track pas la delivery per-member (pas de matrice de delivery)
- TTL 24h max — aucune exception
