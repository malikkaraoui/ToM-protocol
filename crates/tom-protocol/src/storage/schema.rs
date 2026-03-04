/// Database schema for ToM protocol state persistence.
///
/// Schema versioning from day 1 — migrations added as needed.
use rusqlite::Connection;

#[cfg(test)]
const CURRENT_VERSION: i64 = 3;

/// Initialize the database schema (create tables if not exist, run migrations).
pub fn initialize(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Schema version tracking
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        )",
    )?;

    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        migrate_v1(conn)?;
    }
    if version < 2 {
        migrate_v2(conn)?;
    }
    if version < 3 {
        migrate_v3(conn)?;
    }

    Ok(())
}

/// V1: Initial schema.
fn migrate_v1(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        -- Groups we are a member of
        CREATE TABLE IF NOT EXISTS groups (
            group_id TEXT PRIMARY KEY,
            data TEXT NOT NULL
        );

        -- Sender keys (local + remote)
        CREATE TABLE IF NOT EXISTS sender_keys (
            group_id TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            data TEXT NOT NULL,
            PRIMARY KEY (group_id, owner_id)
        );

        -- Groups we are hub for
        CREATE TABLE IF NOT EXISTS hub_groups (
            group_id TEXT PRIMARY KEY,
            data TEXT NOT NULL
        );

        -- Known peers (topology)
        CREATE TABLE IF NOT EXISTS peers (
            node_id TEXT PRIMARY KEY,
            role TEXT NOT NULL,
            status TEXT NOT NULL,
            last_seen INTEGER NOT NULL
        );

        -- Contribution metrics per peer
        CREATE TABLE IF NOT EXISTS contribution_metrics (
            node_id TEXT PRIMARY KEY,
            data TEXT NOT NULL
        );

        INSERT OR REPLACE INTO schema_version (version) VALUES (1);
        ",
    )?;
    Ok(())
}

/// V2: Message tracker persistence (R10.2).
fn migrate_v2(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        -- Tracked messages (delivery status persistence)
        CREATE TABLE IF NOT EXISTS tracked_messages (
            message_id TEXT PRIMARY KEY,
            to_node_id TEXT NOT NULL,
            status INTEGER NOT NULL,
            created_ms INTEGER NOT NULL,
            retries_remaining INTEGER NOT NULL
        );

        INSERT OR REPLACE INTO schema_version (version) VALUES (2);
        ",
    )?;
    Ok(())
}

/// V3: Hub invited_set persistence (R11.3 invite-only groups).
fn migrate_v3(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        ALTER TABLE hub_groups ADD COLUMN invited_set TEXT NOT NULL DEFAULT '[]';

        INSERT OR REPLACE INTO schema_version (version) VALUES (3);
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"groups".to_string()));
        assert!(tables.contains(&"sender_keys".to_string()));
        assert!(tables.contains(&"hub_groups".to_string()));
        assert!(tables.contains(&"peers".to_string()));
        assert!(tables.contains(&"contribution_metrics".to_string()));
        assert!(tables.contains(&"tracked_messages".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
    }

    #[test]
    fn initialize_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        initialize(&conn).unwrap(); // Should not error

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }
}
