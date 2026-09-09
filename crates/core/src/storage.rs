//! SQLite connection handling: open with safe pragmas, `user_version` access.

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;

/// Open (or create) the application database with production pragmas.
///
/// - `journal_mode = WAL` — crash-safe, concurrent readers.
/// - `foreign_keys = ON` — relational integrity enforced.
/// - `synchronous = NORMAL` — safe with WAL.
/// - `busy_timeout = 5000` — tolerate brief lock contention.
pub fn open_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    Ok(conn)
}

/// Open an in-memory database with the same pragmas.
/// Intended for tests (unit + integration); harmless in production builds.
pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "journal_mode", "MEMORY")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

/// Read `PRAGMA user_version`.
pub fn user_version(conn: &Connection) -> Result<u32> {
    Ok(conn.query_row("PRAGMA user_version", rusqlite::params![], |r| r.get(0))?)
}

/// Set `PRAGMA user_version`. Only the migration runner should call this.
pub fn set_user_version(conn: &Connection, version: u32) -> Result<()> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_db_enforces_foreign_keys() {
        let conn = open_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE p(id TEXT PRIMARY KEY); \
             CREATE TABLE c(id TEXT PRIMARY KEY, p TEXT REFERENCES p(id));",
        )
        .unwrap();
        assert!(conn.execute("INSERT INTO c VALUES ('x','missing')", rusqlite::params![]).is_err());
    }

    #[test]
    fn user_version_roundtrip() {
        let conn = open_memory().unwrap();
        assert_eq!(user_version(&conn).unwrap(), 0);
        set_user_version(&conn, 6).unwrap();
        assert_eq!(user_version(&conn).unwrap(), 6);
    }
}
