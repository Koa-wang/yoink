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

/// Open (creating if necessary) the SQLite database.
pub fn init(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
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

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) -> Result<()> {
    conn.execute(
        "UPDATE entries SET pinned = ?1 WHERE id = ?2",
        params![pinned as i64, id],
    )?;
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
