use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

const SCHEMA_SQL: &str = include_str!("schema.sql");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: i64,
    pub front: String,
    pub back: String,
    pub has_audio: bool,
    pub tags: Vec<String>,
    pub state: i64,
    pub due: i64,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    pub last_review: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckStats {
    pub total: i64,
    pub due: i64,
    pub new: i64,
    pub learning: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckMeta {
    pub name: String,
    pub schema_version: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSummary {
    pub path: String,
    pub name: String,
    pub due: i64,
    pub new: i64,
    pub total: i64,
}

const EXPECTED_TABLES: &[&str] = &["meta", "cards", "review_log"];

// Probe query: validates that `cards` has every column we read elsewhere.
// LIMIT 0 means no rows are scanned — purely a schema check.
const COLUMN_PROBE: &str = "SELECT id, front, back, audio, audio_mime, tags, \
                            state, due, stability, difficulty, reps, lapses, last_review \
                            FROM cards LIMIT 0";

pub fn open(path: &Path) -> Result<Connection> {
    // Validate the file read-only first so a rejected foreign DB is left
    // untouched on disk (no journal artifacts written next to it).
    let exists = path.exists();
    if exists {
        let probe = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("opening {path:?} for validation"))?;
        validate_schema(&probe)?;
        drop(probe);
    }

    let conn = Connection::open(path).with_context(|| format!("opening {path:?}"))?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

fn validate_schema(conn: &Connection) -> Result<()> {
    let existing: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    // Empty file: nothing to validate, we'll create the schema afterwards.
    if existing.is_empty() {
        return Ok(());
    }

    let unexpected: Vec<&String> = existing
        .iter()
        .filter(|n| !EXPECTED_TABLES.contains(&n.as_str()))
        .collect();
    if !unexpected.is_empty() {
        anyhow::bail!(
            "not a lapse deck: file contains unexpected tables ({})",
            unexpected
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Tables exist; verify the columns we depend on are actually there.
    // If `cards` is missing any column the probe references, prepare() errors.
    if existing.iter().any(|n| n == "cards") {
        conn.prepare(COLUMN_PROBE)
            .context("not a lapse deck: cards table has unexpected schema")?;
    }

    Ok(())
}

pub fn deck_name(conn: &Connection, fallback: &str) -> String {
    let stored: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'name'", [], |row| {
            row.get::<_, String>(0)
        })
        .ok();
    match stored {
        Some(v) if !v.trim().is_empty() => v,
        _ => fallback.to_string(),
    }
}

pub fn summarize(conn: &Connection, path: &Path, now: DateTime<Utc>) -> Result<DeckSummary> {
    let fallback = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("deck")
        .to_string();
    let name = deck_name(conn, &fallback);
    let s = stats(conn, now)?;
    Ok(DeckSummary {
        path: path.to_string_lossy().into_owned(),
        name,
        due: s.due,
        new: s.new,
        total: s.total,
    })
}

pub fn meta(conn: &Connection, path: &Path) -> Result<DeckMeta> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("deck")
        .to_string();

    let schema_version = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "1".to_string());

    Ok(DeckMeta {
        name,
        schema_version,
        path: path.to_string_lossy().into_owned(),
    })
}

pub fn stats(conn: &Connection, now: DateTime<Utc>) -> Result<DeckStats> {
    let now_ms = now.timestamp_millis();
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))?;
    let new: i64 = conn.query_row("SELECT COUNT(*) FROM cards WHERE state = 0", [], |r| r.get(0))?;
    let learning: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE state IN (1, 3)",
        [],
        |r| r.get(0),
    )?;
    let due: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE state = 0 OR (state != 0 AND due <= ?1)",
        params![now_ms],
        |r| r.get(0),
    )?;
    Ok(DeckStats { total, due, new, learning })
}

pub fn next_due_card(conn: &Connection, now: DateTime<Utc>) -> Result<Option<Card>> {
    let now_ms = now.timestamp_millis();
    // Priority: learning/relearning due now > review due now > new (limited elsewhere by daily cap)
    let row = conn
        .query_row(
            "SELECT id, front, back, audio IS NOT NULL, tags,
                    state, due, stability, difficulty, reps, lapses, last_review
             FROM cards
             WHERE (state = 0) OR (state != 0 AND due <= ?1)
             ORDER BY
                 CASE state WHEN 1 THEN 0 WHEN 3 THEN 0 WHEN 2 THEN 1 ELSE 2 END,
                 due ASC,
                 id ASC
             LIMIT 1",
            params![now_ms],
            row_to_card,
        )
        .optional()?;
    Ok(row)
}

fn row_to_card(row: &rusqlite::Row) -> rusqlite::Result<Card> {
    let tags: String = row.get(4)?;
    Ok(Card {
        id: row.get(0)?,
        front: row.get(1)?,
        back: row.get(2)?,
        has_audio: row.get::<_, bool>(3)?,
        tags: tags
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        state: row.get(5)?,
        due: row.get(6)?,
        stability: row.get(7)?,
        difficulty: row.get(8)?,
        reps: row.get(9)?,
        lapses: row.get(10)?,
        last_review: row.get(11)?,
    })
}

pub fn get_card(conn: &Connection, id: i64) -> Result<Option<Card>> {
    let row = conn
        .query_row(
            "SELECT id, front, back, audio IS NOT NULL, tags,
                    state, due, stability, difficulty, reps, lapses, last_review
             FROM cards WHERE id = ?1",
            params![id],
            row_to_card,
        )
        .optional()?;
    Ok(row)
}

pub fn get_audio(conn: &Connection, id: i64) -> Result<Option<(Vec<u8>, String)>> {
    let row = conn
        .query_row(
            "SELECT audio, audio_mime FROM cards WHERE id = ?1",
            params![id],
            |r| {
                let bytes: Option<Vec<u8>> = r.get(0)?;
                let mime: Option<String> = r.get(1)?;
                Ok((bytes, mime))
            },
        )
        .optional()?;
    // Require both audio + audio_mime. We don't guess the format — a deck
    // that stores BLOBs without setting the mime is malformed.
    match row {
        Some((Some(bytes), Some(mime))) => Ok(Some((bytes, mime))),
        _ => Ok(None),
    }
}

#[derive(Debug, Clone)]
pub struct UpdatedCard {
    pub state: i64,
    pub due: i64,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    pub last_review: i64,
}

pub fn apply_review(
    conn: &mut Connection,
    card_id: i64,
    updated: &UpdatedCard,
    rating: i64,
    elapsed_days: f64,
    scheduled_days: f64,
    state_before: i64,
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE cards SET state = ?1, due = ?2, stability = ?3, difficulty = ?4,
                          reps = ?5, lapses = ?6, last_review = ?7
         WHERE id = ?8",
        params![
            updated.state,
            updated.due,
            updated.stability,
            updated.difficulty,
            updated.reps,
            updated.lapses,
            updated.last_review,
            card_id,
        ],
    )?;
    tx.execute(
        "INSERT INTO review_log
             (card_id, reviewed_at, rating, elapsed_days, scheduled_days, state_before)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            card_id,
            updated.last_review,
            rating,
            elapsed_days,
            scheduled_days,
            state_before
        ],
    )?;
    tx.commit()?;
    Ok(())
}
