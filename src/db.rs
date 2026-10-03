use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

use crate::config::Config;
use crate::models::{fuzzy_match, Entry};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL DEFAULT 'unknown',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_entries_updated ON entries(updated_at DESC);
";

/// Open (creating if necessary) the SQLite database with WAL enabled.
pub fn init(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)?;
    Ok(())
}

/// Insert new content, or bump `updated_at` if the content already exists.
pub fn insert_or_update(conn: &Connection, content: &str, source: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO entries (content, source, created_at, updated_at, pinned)
         VALUES (?1, ?2, ?3, ?3, 0)
         ON CONFLICT(content) DO UPDATE SET updated_at = ?3, source = ?2",
        params![content, source, now],
    )?;
    Ok(())
}

/// List entries in reverse chronological order.
pub fn list(conn: &Connection, limit: usize) -> Result<Vec<Entry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, source, created_at, updated_at, pinned
         FROM entries ORDER BY updated_at DESC, id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], row_to_entry)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

/// All entries in reverse chronological order.
pub fn all(conn: &Connection) -> Result<Vec<Entry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, source, created_at, updated_at, pinned
         FROM entries ORDER BY updated_at DESC, id DESC",
    )?;
    let rows = stmt.query_map(params![], row_to_entry)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

/// Fuzzy-search history, returning the best matches first.
pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Entry>> {
    let mut matches: Vec<(i64, Entry)> = all(conn)?
        .into_iter()
        .filter_map(|e| fuzzy_match(query, &e.content).map(|s| (s, e)))
        .collect();
    matches.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.updated_at.cmp(&a.1.updated_at))
    });
    Ok(matches
        .into_iter()
        .take(limit)
        .map(|(_, e)| e)
        .collect())
}

pub fn delete(conn: &Connection, id: i64) -> Result<usize> {
    Ok(conn.execute("DELETE FROM entries WHERE id = ?1", params![id])?)
}

pub fn clear(conn: &Connection) -> Result<usize> {
    Ok(conn.execute("DELETE FROM entries", params![])?)
}

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE entries SET pinned = ?1 WHERE id = ?2",
        params![pinned as i64, id],
    )?)
}

/// Reclaim space. Call after large deletions (e.g. `clear`).
pub fn vacuum(conn: &Connection) -> Result<()> {
    conn.execute_batch("VACUUM;")?;
    Ok(())
}

/// Enforce `max_entries` and `history_days`. Pinned entries are always kept.
pub fn cleanup(conn: &Connection, cfg: &Config) -> Result<()> {
    let cutoff = chrono::Utc::now().timestamp() - cfg.history_days * 86_400;
    conn.execute(
        "DELETE FROM entries WHERE pinned = 0 AND updated_at < ?1",
        params![cutoff],
    )?;

    let pinned_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE pinned = 1",
        params![],
        |r| r.get(0),
    )?;
    let keep = (cfg.max_entries as i64 - pinned_count).max(0);
    conn.execute(
        "DELETE FROM entries WHERE pinned = 0 AND id IN (
            SELECT id FROM entries WHERE pinned = 0
            ORDER BY updated_at DESC, id DESC LIMIT -1 OFFSET ?1
        )",
        params![keep],
    )?;
    Ok(())
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        content: row.get(1)?,
        source: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        pinned: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn insert_updates_timestamp_instead_of_duplicating() {
        let conn = mem();
        insert_or_update(&conn, "hello", "app").unwrap();
        let first = list(&conn, 10).unwrap();
        assert_eq!(first.len(), 1);
        let id = first[0].id;
        let ts = first[0].updated_at;

        insert_or_update(&conn, "hello", "app2").unwrap();
        let second = list(&conn, 10).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].id, id);
        assert!(second[0].updated_at >= ts);
    }

    #[test]
    fn cleanup_removes_expired_but_keeps_pinned() {
        let conn = mem();
        let mut cfg = Config::default();
        cfg.history_days = 1;

        conn.execute(
            "INSERT INTO entries (content, source, created_at, updated_at, pinned)
             VALUES ('old', 's', 100, 100, 0)",
            params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entries (content, source, created_at, updated_at, pinned)
             VALUES ('old-pinned', 's', 100, 100, 1)",
            params![],
        )
        .unwrap();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO entries (content, source, created_at, updated_at, pinned)
             VALUES ('new', 's', ?1, ?1, 0)",
            params![now],
        )
        .unwrap();

        cleanup(&conn, &cfg).unwrap();

        let contents: Vec<String> = all(&conn).unwrap().into_iter().map(|e| e.content).collect();
        assert!(contents.contains(&"old-pinned".to_string()));
        assert!(contents.contains(&"new".to_string()));
        assert!(!contents.contains(&"old".to_string()));
    }

    #[test]
    fn cleanup_trims_to_max_entries() {
        let conn = mem();
        let mut cfg = Config::default();
        cfg.max_entries = 2;
        cfg.history_days = 3650;

        let now = chrono::Utc::now().timestamp();
        for i in 0..5 {
            conn.execute(
                "INSERT INTO entries (content, source, created_at, updated_at, pinned)
                 VALUES (?1, 's', ?2, ?2, 0)",
                params![format!("c{i}"), now - i],
            )
            .unwrap();
        }

        cleanup(&conn, &cfg).unwrap();
        assert_eq!(all(&conn).unwrap().len(), 2);
    }

    #[test]
    fn search_matches_subsequence() {
        let conn = mem();
        insert_or_update(&conn, "banana", "s").unwrap();
        insert_or_update(&conn, "apple", "s").unwrap();

        let hits = search(&conn, "ban", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].content, "banana");
    }

    #[test]
    fn delete_and_pin() {
        let conn = mem();
        insert_or_update(&conn, "x", "s").unwrap();
        let id = list(&conn, 10).unwrap()[0].id;

        set_pinned(&conn, id, true).unwrap();
        assert!(list(&conn, 10).unwrap()[0].pinned);

        delete(&conn, id).unwrap();
        assert!(list(&conn, 10).unwrap().is_empty());
    }
}
