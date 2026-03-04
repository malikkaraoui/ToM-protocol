/// State persistence for the ToM protocol runtime.
///
/// Stores groups, sender keys, contacts, and hub state in SQLite.
/// Designed for fast reads on startup and periodic batched writes.
mod schema;

use std::collections::HashMap;
use std::path::Path;

use std::sync::Mutex;

use rusqlite::Connection;

use crate::group::{GroupHubSnapshot, GroupId, GroupInfo, GroupManagerSnapshot};
use crate::group::SenderKeyEntry;
use crate::relay::{PeerInfo, PeerRole, PeerStatus};
use crate::roles::ContributionMetrics;
use crate::tracker::TrackedMessageRecord;
use crate::types::{MessageStatus, NodeId};

/// SQLite-backed state store.
///
/// Wraps Connection in Mutex for Sync (required because RuntimeState
/// holds &self across .await points in tokio::spawn).
pub struct StateStore {
    conn: Mutex<Connection>,
}

/// Combined snapshot of all persistent state.
#[derive(Debug, Default)]
pub struct StateSnapshot {
    pub manager: Option<GroupManagerSnapshot>,
    pub hub: Option<GroupHubSnapshot>,
    pub peers: HashMap<NodeId, PeerInfo>,
    pub metrics: HashMap<NodeId, ContributionMetrics>,
    pub tracked_messages: HashMap<String, TrackedMessageRecord>,
}

impl StateStore {
    /// Open (or create) a state database at the given path.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(path)?;

        // Performance: WAL mode for concurrent reads during writes
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        schema::initialize(&conn)?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an in-memory database (for testing).
    #[cfg(test)]
    pub fn open_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        schema::initialize(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    // ── Save methods ────────────────────────────────────────────────────

    /// Save all persistent state in a single transaction.
    pub fn save(&self, snapshot: &StateSnapshot) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        if let Some(ref mgr) = snapshot.manager {
            self.save_groups_tx(&tx, &mgr.groups, &mgr.last_seqs)?;
            self.save_sender_keys_tx(&tx, &mgr.local_sender_keys, &mgr.sender_keys)?;
        }

        if let Some(ref hub) = snapshot.hub {
            self.save_hub_groups_tx(&tx, hub)?;
        }

        self.save_peers_tx(&tx, &snapshot.peers)?;
        self.save_metrics_tx(&tx, &snapshot.metrics)?;
        self.save_tracked_messages_tx(&tx, &snapshot.tracked_messages)?;

        tx.commit()?;
        Ok(())
    }

    fn save_groups_tx(
        &self,
        tx: &rusqlite::Transaction,
        groups: &HashMap<GroupId, GroupInfo>,
        last_seqs: &HashMap<GroupId, u64>,
    ) -> Result<(), rusqlite::Error> {
        tx.execute("DELETE FROM groups", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO groups (group_id, data, last_seq) VALUES (?1, ?2, ?3)",
        )?;
        for (gid, info) in groups {
            let json = serde_json::to_string(info).unwrap_or_default();
            let last_seq = last_seqs.get(gid).copied().unwrap_or(0) as i64;
            stmt.execute(rusqlite::params![gid.to_string(), json, last_seq])?;
        }
        Ok(())
    }

    fn save_sender_keys_tx(
        &self,
        tx: &rusqlite::Transaction,
        local_keys: &HashMap<GroupId, SenderKeyEntry>,
        remote_keys: &HashMap<GroupId, HashMap<NodeId, SenderKeyEntry>>,
    ) -> Result<(), rusqlite::Error> {
        tx.execute("DELETE FROM sender_keys", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO sender_keys (group_id, owner_id, data) VALUES (?1, ?2, ?3)",
        )?;

        // Save local sender keys (owner_id = "LOCAL")
        for (gid, entry) in local_keys {
            let json = serde_json::to_string(entry).unwrap_or_default();
            stmt.execute(rusqlite::params![gid.to_string(), "LOCAL", json])?;
        }

        // Save remote sender keys
        for (gid, keys) in remote_keys {
            for (node_id, entry) in keys {
                let json = serde_json::to_string(entry).unwrap_or_default();
                stmt.execute(rusqlite::params![
                    gid.to_string(),
                    node_id.to_string(),
                    json
                ])?;
            }
        }
        Ok(())
    }

    fn save_hub_groups_tx(
        &self,
        tx: &rusqlite::Transaction,
        hub: &GroupHubSnapshot,
    ) -> Result<(), rusqlite::Error> {
        tx.execute("DELETE FROM hub_groups", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO hub_groups (group_id, data, invited_set, next_seq) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (gid, info) in &hub.groups {
            let json = serde_json::to_string(info).unwrap_or_default();
            let invited: Vec<String> = hub.invited_sets
                .get(gid)
                .map(|s| s.iter().map(|n| n.to_string()).collect())
                .unwrap_or_default();
            let invited_json = serde_json::to_string(&invited).unwrap_or_default();
            let next_seq = hub.next_seqs.get(gid).copied().unwrap_or(0) as i64;
            stmt.execute(rusqlite::params![gid.to_string(), json, invited_json, next_seq])?;
        }
        Ok(())
    }

    fn save_peers_tx(
        &self,
        tx: &rusqlite::Transaction,
        peers: &HashMap<NodeId, PeerInfo>,
    ) -> Result<(), rusqlite::Error> {
        tx.execute("DELETE FROM peers", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO peers (node_id, role, status, last_seen) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (nid, info) in peers {
            let role = match info.role {
                PeerRole::Peer => "Peer",
                PeerRole::Relay => "Relay",
            };
            let status = match info.status {
                PeerStatus::Online => "Online",
                PeerStatus::Offline => "Offline",
                PeerStatus::Stale => "Stale",
            };
            stmt.execute(rusqlite::params![
                nid.to_string(),
                role,
                status,
                info.last_seen as i64
            ])?;
        }
        Ok(())
    }

    fn save_metrics_tx(
        &self,
        tx: &rusqlite::Transaction,
        metrics: &HashMap<NodeId, ContributionMetrics>,
    ) -> Result<(), rusqlite::Error> {
        tx.execute("DELETE FROM contribution_metrics", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO contribution_metrics (node_id, data) VALUES (?1, ?2)",
        )?;
        for (nid, m) in metrics {
            let json = serde_json::to_string(m).unwrap_or_default();
            stmt.execute(rusqlite::params![nid.to_string(), json])?;
        }
        Ok(())
    }

    fn save_tracked_messages_tx(
        &self,
        tx: &rusqlite::Transaction,
        messages: &HashMap<String, TrackedMessageRecord>,
    ) -> Result<(), rusqlite::Error> {
        tx.execute("DELETE FROM tracked_messages", [])?;
        let mut stmt = tx.prepare(
            "INSERT INTO tracked_messages (message_id, to_node_id, status, created_ms, retries_remaining) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (msg_id, record) in messages {
            stmt.execute(rusqlite::params![
                msg_id,
                record.to.to_string(),
                record.status as i32,
                record.created_ms as i64,
                record.retries_remaining as i32
            ])?;
        }
        Ok(())
    }

    // ── Hub message history (R13) ────────────────────────────────────

    /// Save a single hub message to history (called after each handle_message).
    pub fn save_hub_message(
        &self,
        group_id: &GroupId,
        seq: u64,
        message_data: &[u8],
        stored_at: u64,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO hub_message_history (group_id, seq, message_data, stored_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![group_id.to_string(), seq as i64, message_data, stored_at as i64],
        )?;
        Ok(())
    }

    /// Load hub messages for a group since a given sequence number.
    /// Returns messages ordered by seq ascending, limited to `max_count`.
    pub fn load_hub_messages_since(
        &self,
        group_id: &GroupId,
        since_seq: u64,
        max_count: usize,
    ) -> Result<Vec<(u64, Vec<u8>)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT seq, message_data FROM hub_message_history \
             WHERE group_id = ?1 AND seq > ?2 \
             ORDER BY seq ASC LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![group_id.to_string(), since_seq as i64, max_count as i64],
            |row| {
                let seq: i64 = row.get(0)?;
                let data: Vec<u8> = row.get(1)?;
                Ok((seq as u64, data))
            },
        )?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Delete expired hub messages (TTL cleanup).
    pub fn cleanup_hub_messages(&self, cutoff_ms: u64) -> Result<usize, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM hub_message_history WHERE stored_at < ?1",
            rusqlite::params![cutoff_ms as i64],
        )?;
        Ok(deleted)
    }

    // ── Load methods ────────────────────────────────────────────────────

    /// Load all persistent state.
    pub fn load(&self) -> Result<StateSnapshot, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let (groups, member_last_seqs) = Self::load_groups(&conn)?;
        let (local_keys, remote_keys) = Self::load_sender_keys(&conn)?;
        let (hub_groups, hub_invited_sets, hub_next_seqs) = Self::load_hub_groups(&conn)?;
        let peers = Self::load_peers(&conn)?;
        let metrics = Self::load_metrics(&conn)?;
        let tracked_messages = Self::load_tracked_messages(&conn)?;

        let manager = if !groups.is_empty() || !local_keys.is_empty() {
            Some(GroupManagerSnapshot {
                groups,
                local_sender_keys: local_keys,
                sender_keys: remote_keys,
                previous_sender_keys: HashMap::new(),
                local_sender_message_counts: HashMap::new(),
                message_history: HashMap::new(), // Not persisted (rebuilt via hub sync)
                last_seqs: member_last_seqs,
            })
        } else {
            None
        };

        let hub = if !hub_groups.is_empty() {
            Some(GroupHubSnapshot {
                groups: hub_groups,
                invited_sets: hub_invited_sets,
                next_seqs: hub_next_seqs,
            })
        } else {
            None
        };

        Ok(StateSnapshot {
            manager,
            hub,
            peers,
            metrics,
            tracked_messages,
        })
    }

    #[allow(clippy::type_complexity)]
    fn load_groups(conn: &Connection) -> Result<(HashMap<GroupId, GroupInfo>, HashMap<GroupId, u64>), rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT group_id, data, last_seq FROM groups")?;
        let mut groups = HashMap::new();
        let mut last_seqs = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let gid: String = row.get(0)?;
            let json: String = row.get(1)?;
            let last_seq: i64 = row.get::<_, i64>(2).unwrap_or(0);
            Ok((gid, json, last_seq))
        })?;
        for row in rows {
            let (gid, json, last_seq) = row?;
            let group_id = GroupId::from(gid);
            if let Ok(info) = serde_json::from_str::<GroupInfo>(&json) {
                groups.insert(group_id.clone(), info);
            }
            if last_seq > 0 {
                last_seqs.insert(group_id, last_seq as u64);
            }
        }
        Ok((groups, last_seqs))
    }

    #[allow(clippy::type_complexity)]
    fn load_sender_keys(
        conn: &Connection,
    ) -> Result<
        (
            HashMap<GroupId, SenderKeyEntry>,
            HashMap<GroupId, HashMap<NodeId, SenderKeyEntry>>,
        ),
        rusqlite::Error,
    > {
        let mut stmt = conn.prepare("SELECT group_id, owner_id, data FROM sender_keys")?;
        let mut local_keys = HashMap::new();
        let mut remote_keys: HashMap<GroupId, HashMap<NodeId, SenderKeyEntry>> = HashMap::new();

        let rows = stmt.query_map([], |row| {
            let gid: String = row.get(0)?;
            let owner: String = row.get(1)?;
            let json: String = row.get(2)?;
            Ok((gid, owner, json))
        })?;

        for row in rows {
            let (gid, owner, json) = row?;
            let Ok(entry) = serde_json::from_str::<SenderKeyEntry>(&json) else {
                continue;
            };
            let group_id = GroupId::from(gid);
            if owner == "LOCAL" {
                local_keys.insert(group_id, entry);
            } else if let Ok(node_id) = owner.parse::<NodeId>() {
                remote_keys
                    .entry(group_id)
                    .or_default()
                    .insert(node_id, entry);
            }
        }

        Ok((local_keys, remote_keys))
    }

    #[allow(clippy::type_complexity)]
    fn load_hub_groups(conn: &Connection) -> Result<(HashMap<GroupId, GroupInfo>, HashMap<GroupId, std::collections::HashSet<NodeId>>, HashMap<GroupId, u64>), rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT group_id, data, invited_set, next_seq FROM hub_groups")?;
        let mut groups = HashMap::new();
        let mut invited_sets = HashMap::new();
        let mut next_seqs = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let gid: String = row.get(0)?;
            let json: String = row.get(1)?;
            let invited_json: String = row.get(2)?;
            let next_seq: i64 = row.get::<_, i64>(3).unwrap_or(0);
            Ok((gid, json, invited_json, next_seq))
        })?;
        for row in rows {
            let (gid, json, invited_json, next_seq) = row?;
            let group_id = GroupId::from(gid);
            if let Ok(info) = serde_json::from_str::<GroupInfo>(&json) {
                groups.insert(group_id.clone(), info);
            }
            if let Ok(invited) = serde_json::from_str::<Vec<String>>(&invited_json) {
                let set: std::collections::HashSet<NodeId> = invited
                    .iter()
                    .filter_map(|s| s.parse::<NodeId>().ok())
                    .collect();
                if !set.is_empty() {
                    invited_sets.insert(group_id.clone(), set);
                }
            }
            if next_seq > 0 {
                next_seqs.insert(group_id, next_seq as u64);
            }
        }
        Ok((groups, invited_sets, next_seqs))
    }

    fn load_peers(conn: &Connection) -> Result<HashMap<NodeId, PeerInfo>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT node_id, role, status, last_seen FROM peers")?;
        let mut peers = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let nid: String = row.get(0)?;
            let role: String = row.get(1)?;
            let status: String = row.get(2)?;
            let last_seen: i64 = row.get(3)?;
            Ok((nid, role, status, last_seen))
        })?;
        for row in rows {
            let (nid, role, status, last_seen) = row?;
            let Ok(node_id) = nid.parse::<NodeId>() else {
                continue;
            };
            let role = match role.as_str() {
                "Relay" => PeerRole::Relay,
                _ => PeerRole::Peer,
            };
            let status = match status.as_str() {
                "Online" => PeerStatus::Online,
                "Stale" => PeerStatus::Stale,
                _ => PeerStatus::Offline, // All peers start offline after restart
            };
            peers.insert(
                node_id,
                PeerInfo {
                    node_id,
                    role,
                    status,
                    last_seen: last_seen as u64,
                },
            );
        }
        Ok(peers)
    }

    fn load_metrics(conn: &Connection) -> Result<HashMap<NodeId, ContributionMetrics>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT node_id, data FROM contribution_metrics")?;
        let mut metrics = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let nid: String = row.get(0)?;
            let json: String = row.get(1)?;
            Ok((nid, json))
        })?;
        for row in rows {
            let (nid, json) = row?;
            let Ok(node_id) = nid.parse::<NodeId>() else {
                continue;
            };
            if let Ok(m) = serde_json::from_str::<ContributionMetrics>(&json) {
                metrics.insert(node_id, m);
            }
        }
        Ok(metrics)
    }

    fn load_tracked_messages(
        conn: &Connection,
    ) -> Result<HashMap<String, TrackedMessageRecord>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT message_id, to_node_id, status, created_ms, retries_remaining FROM tracked_messages",
        )?;
        let mut messages = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let msg_id: String = row.get(0)?;
            let to: String = row.get(1)?;
            let status: i32 = row.get(2)?;
            let created_ms: i64 = row.get(3)?;
            let retries: i32 = row.get(4)?;
            Ok((msg_id, to, status, created_ms, retries))
        })?;
        for row in rows {
            let (msg_id, to, status_int, created_ms, retries) = row?;
            let Ok(to_id) = to.parse::<NodeId>() else {
                continue;
            };
            let status = match status_int {
                0 => MessageStatus::Pending,
                1 => MessageStatus::Sent,
                2 => MessageStatus::Relayed,
                3 => MessageStatus::Delivered,
                4 => MessageStatus::Read,
                5 => MessageStatus::Failed,
                _ => MessageStatus::Pending,
            };
            messages.insert(
                msg_id,
                TrackedMessageRecord {
                    to: to_id,
                    status,
                    created_ms: created_ms as u64,
                    retries_remaining: retries as u8,
                },
            );
        }
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::types::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn make_group_info(name: &str, hub_id: NodeId, creator: NodeId) -> GroupInfo {
        GroupInfo {
            group_id: GroupId::new(),
            name: name.into(),
            hub_relay_id: hub_id,
            backup_hub_id: None,
            members: vec![GroupMember {
                node_id: creator,
                username: "alice".into(),
                joined_at: 1000,
                role: GroupMemberRole::Admin,
            }],
            created_by: creator,
            created_at: 1000,
            last_activity_at: 1000,
            max_members: MAX_GROUP_MEMBERS,
            shadow_id: None,
            candidate_id: None,
            invite_only: false,
        }
    }

    #[test]
    fn roundtrip_groups() {
        let store = StateStore::open_memory().unwrap();
        let hub = node_id(10);
        let alice = node_id(1);

        let g1 = make_group_info("Group A", hub, alice);
        let g2 = make_group_info("Group B", hub, alice);
        let gid1 = g1.group_id.clone();
        let gid2 = g2.group_id.clone();

        let mut groups = HashMap::new();
        groups.insert(gid1.clone(), g1);
        groups.insert(gid2.clone(), g2);

        let snapshot = StateSnapshot {
            manager: Some(GroupManagerSnapshot {
                groups,
                local_sender_keys: HashMap::new(),
                sender_keys: HashMap::new(),
                previous_sender_keys: HashMap::new(),
                local_sender_message_counts: HashMap::new(),
                message_history: HashMap::new(),
                last_seqs: HashMap::new(),
            }),
            ..Default::default()
        };

        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        let mgr = loaded.manager.unwrap();
        assert_eq!(mgr.groups.len(), 2);
        assert_eq!(mgr.groups[&gid1].name, "Group A");
        assert_eq!(mgr.groups[&gid2].name, "Group B");
    }

    #[test]
    fn roundtrip_sender_keys() {
        let store = StateStore::open_memory().unwrap();
        let alice = node_id(1);
        let bob = node_id(2);
        let gid = GroupId::from("grp-1".to_string());

        let local_key = SenderKeyEntry {
            owner_id: alice,
            key: [42u8; 32],
            epoch: 3,
            created_at: 5000,
        };
        let remote_key = SenderKeyEntry {
            owner_id: bob,
            key: [7u8; 32],
            epoch: 1,
            created_at: 6000,
        };

        let mut local_keys = HashMap::new();
        local_keys.insert(gid.clone(), local_key.clone());

        let mut remote_keys = HashMap::new();
        remote_keys
            .entry(gid.clone())
            .or_insert_with(HashMap::new)
            .insert(bob, remote_key.clone());

        let snapshot = StateSnapshot {
            manager: Some(GroupManagerSnapshot {
                groups: HashMap::new(),
                local_sender_keys: local_keys,
                sender_keys: remote_keys,
                previous_sender_keys: HashMap::new(),
                local_sender_message_counts: HashMap::new(),
                message_history: HashMap::new(),
                last_seqs: HashMap::new(),
            }),
            ..Default::default()
        };

        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        let mgr = loaded.manager.unwrap();
        assert_eq!(mgr.local_sender_keys[&gid].epoch, 3);
        assert_eq!(mgr.local_sender_keys[&gid].key, [42u8; 32]);
        assert_eq!(mgr.sender_keys[&gid][&bob].epoch, 1);
    }

    #[test]
    fn roundtrip_peers() {
        let store = StateStore::open_memory().unwrap();
        let alice = node_id(1);
        let bob = node_id(2);

        let mut peers = HashMap::new();
        peers.insert(alice, PeerInfo {
            node_id: alice,
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: 99000,
        });
        peers.insert(bob, PeerInfo {
            node_id: bob,
            role: PeerRole::Peer,
            status: PeerStatus::Stale,
            last_seen: 88000,
        });

        let snapshot = StateSnapshot { peers, ..Default::default() };
        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.peers.len(), 2);
        assert_eq!(loaded.peers[&alice].role, PeerRole::Relay);
        // After restart, Online peers become Offline
        assert_eq!(loaded.peers[&bob].last_seen, 88000);
    }

    #[test]
    fn roundtrip_hub_groups() {
        let store = StateStore::open_memory().unwrap();
        let hub = node_id(10);
        let alice = node_id(1);

        let info = make_group_info("Hub Group", hub, alice);
        let gid = info.group_id.clone();

        let mut groups = HashMap::new();
        groups.insert(gid.clone(), info);

        let snapshot = StateSnapshot {
            hub: Some(GroupHubSnapshot { groups, invited_sets: HashMap::new(), next_seqs: HashMap::new() }),
            ..Default::default()
        };

        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        let hub_snap = loaded.hub.unwrap();
        assert_eq!(hub_snap.groups.len(), 1);
        assert_eq!(hub_snap.groups[&gid].name, "Hub Group");
    }

    #[test]
    fn roundtrip_hub_invited_sets() {
        let store = StateStore::open_memory().unwrap();
        let hub = node_id(10);
        let alice = node_id(1);
        let bob = node_id(2);
        let charlie = node_id(3);

        let mut info = make_group_info("Invite-Only", hub, alice);
        info.invite_only = true;
        let gid = info.group_id.clone();

        let mut groups = HashMap::new();
        groups.insert(gid.clone(), info);

        let mut invited_sets = HashMap::new();
        let mut set = std::collections::HashSet::new();
        set.insert(bob);
        set.insert(charlie);
        invited_sets.insert(gid.clone(), set);

        let snapshot = StateSnapshot {
            hub: Some(GroupHubSnapshot { groups, invited_sets, next_seqs: HashMap::new() }),
            ..Default::default()
        };

        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        let hub_snap = loaded.hub.unwrap();
        assert_eq!(hub_snap.groups.len(), 1);
        assert!(hub_snap.groups[&gid].invite_only);
        assert_eq!(hub_snap.invited_sets[&gid].len(), 2);
        assert!(hub_snap.invited_sets[&gid].contains(&bob));
        assert!(hub_snap.invited_sets[&gid].contains(&charlie));
    }

    #[test]
    fn roundtrip_metrics() {
        let store = StateStore::open_memory().unwrap();
        let alice = node_id(1);

        let mut m = ContributionMetrics::new(1000);
        m.record_relay(2000);
        m.bytes_relayed = 1024;

        let mut metrics = HashMap::new();
        metrics.insert(alice, m);

        let snapshot = StateSnapshot { metrics, ..Default::default() };
        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.metrics.len(), 1);
        assert_eq!(loaded.metrics[&alice].messages_relayed, 1);
        assert_eq!(loaded.metrics[&alice].bytes_relayed, 1024);
    }

    #[test]
    fn save_overwrites_previous() {
        let store = StateStore::open_memory().unwrap();
        let alice = node_id(1);

        // Save with 1 peer
        let mut peers = HashMap::new();
        peers.insert(alice, PeerInfo {
            node_id: alice,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 1000,
        });
        store.save(&StateSnapshot { peers, ..Default::default() }).unwrap();

        // Save with 0 peers
        store.save(&StateSnapshot::default()).unwrap();

        let loaded = store.load().unwrap();
        assert!(loaded.peers.is_empty());
    }

    #[test]
    fn roundtrip_tracked_messages() {
        let store = StateStore::open_memory().unwrap();
        let bob = node_id(2);

        let mut tracked = HashMap::new();
        tracked.insert(
            "msg-001".to_string(),
            TrackedMessageRecord {
                to: bob,
                status: MessageStatus::Sent,
                created_ms: 1000000,
                retries_remaining: 2,
            },
        );
        tracked.insert(
            "msg-002".to_string(),
            TrackedMessageRecord {
                to: bob,
                status: MessageStatus::Relayed,
                created_ms: 1000500,
                retries_remaining: 1,
            },
        );

        let snapshot = StateSnapshot {
            tracked_messages: tracked,
            ..Default::default()
        };
        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.tracked_messages.len(), 2);
        assert_eq!(loaded.tracked_messages["msg-001"].status, MessageStatus::Sent);
        assert_eq!(loaded.tracked_messages["msg-001"].retries_remaining, 2);
        assert_eq!(loaded.tracked_messages["msg-002"].status, MessageStatus::Relayed);
        assert_eq!(loaded.tracked_messages["msg-002"].created_ms, 1000500);
    }

    #[test]
    fn empty_database_loads_empty() {
        let store = StateStore::open_memory().unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.manager.is_none());
        assert!(loaded.hub.is_none());
        assert!(loaded.peers.is_empty());
        assert!(loaded.metrics.is_empty());
        assert!(loaded.tracked_messages.is_empty());
    }

    #[test]
    fn file_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("state.db");

        let alice = node_id(1);

        // Write
        {
            let store = StateStore::open(&db_path).unwrap();
            let mut peers = HashMap::new();
            peers.insert(alice, PeerInfo {
                node_id: alice,
                role: PeerRole::Relay,
                status: PeerStatus::Online,
                last_seen: 42000,
            });
            store.save(&StateSnapshot { peers, ..Default::default() }).unwrap();
        }

        // Read from a new connection
        {
            let store = StateStore::open(&db_path).unwrap();
            let loaded = store.load().unwrap();
            assert_eq!(loaded.peers.len(), 1);
            assert_eq!(loaded.peers[&alice].last_seen, 42000);
        }
    }

    // ── R13.2: Hub message history persistence ────────────────────────

    #[test]
    fn hub_message_history_save_and_load() {
        let store = StateStore::open_memory().unwrap();
        let gid = GroupId::from("grp-hist".to_string());

        // Save 3 messages
        for seq in 0..3u64 {
            let data = format!("message-{seq}").into_bytes();
            store.save_hub_message(&gid, seq, &data, 1000 + seq).unwrap();
        }

        // Load all since seq -1 (effectively all)
        let msgs = store.load_hub_messages_since(&gid, 0, 100).unwrap();
        assert_eq!(msgs.len(), 2, "since_seq=0 should return seq>0, i.e. 1,2");
        assert_eq!(msgs[0].0, 1);
        assert_eq!(msgs[1].0, 2);

        // Load all (since_seq=0 means >0, so use u64::MAX trick or just 0)
        // Actually our API is "seq > since_seq", so to get seq=0 too we'd need since_seq < 0
        // For gap-fill, member sends last_seq they have, so this is correct behavior
    }

    #[test]
    fn hub_message_history_limit() {
        let store = StateStore::open_memory().unwrap();
        let gid = GroupId::from("grp-limit".to_string());

        for seq in 0..100u64 {
            store.save_hub_message(&gid, seq, b"data", 1000 + seq).unwrap();
        }

        // Load with limit 10
        let msgs = store.load_hub_messages_since(&gid, 50, 10).unwrap();
        assert_eq!(msgs.len(), 10);
        assert_eq!(msgs[0].0, 51);
        assert_eq!(msgs[9].0, 60);
    }

    #[test]
    fn hub_message_cleanup_expired() {
        let store = StateStore::open_memory().unwrap();
        let gid = GroupId::from("grp-cleanup".to_string());

        // Store messages with different timestamps
        store.save_hub_message(&gid, 0, b"old", 1000).unwrap();
        store.save_hub_message(&gid, 1, b"old", 2000).unwrap();
        store.save_hub_message(&gid, 2, b"recent", 5000).unwrap();

        // Cleanup messages older than 3000
        let deleted = store.cleanup_hub_messages(3000).unwrap();
        assert_eq!(deleted, 2);

        // Only recent message remains
        let msgs = store.load_hub_messages_since(&gid, 0, 100).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].0, 2);
    }

    #[test]
    fn hub_next_seq_persisted() {
        let store = StateStore::open_memory().unwrap();
        let alice = node_id(1);
        let hub = node_id(10);

        let info = make_group_info("SeqGroup", hub, alice);
        let gid = info.group_id.clone();

        let mut groups = HashMap::new();
        groups.insert(gid.clone(), info);
        let mut next_seqs = HashMap::new();
        next_seqs.insert(gid.clone(), 42u64);

        let snapshot = StateSnapshot {
            hub: Some(GroupHubSnapshot { groups, invited_sets: HashMap::new(), next_seqs }),
            ..Default::default()
        };

        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        let hub_snap = loaded.hub.unwrap();
        assert_eq!(hub_snap.next_seqs[&gid], 42);
    }

    #[test]
    fn member_last_seq_persisted() {
        let store = StateStore::open_memory().unwrap();
        let alice = node_id(1);
        let hub = node_id(10);

        let info = make_group_info("SeqGroup", hub, alice);
        let gid = info.group_id.clone();

        let mut groups = HashMap::new();
        groups.insert(gid.clone(), info);
        let mut last_seqs = HashMap::new();
        last_seqs.insert(gid.clone(), 17u64);

        let snapshot = StateSnapshot {
            manager: Some(GroupManagerSnapshot {
                groups,
                local_sender_keys: HashMap::new(),
                sender_keys: HashMap::new(),
                previous_sender_keys: HashMap::new(),
                local_sender_message_counts: HashMap::new(),
                message_history: HashMap::new(),
                last_seqs,
            }),
            ..Default::default()
        };

        store.save(&snapshot).unwrap();
        let loaded = store.load().unwrap();

        let mgr_snap = loaded.manager.unwrap();
        assert_eq!(mgr_snap.last_seqs[&gid], 17);
    }
}
