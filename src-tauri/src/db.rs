use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
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

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).with_context(|| format!("opening {:?}", path))?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

pub fn deck_name(conn: &Connection, fallback: &str) -> String {
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'name'",
        [],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| fallback.to_string())
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
    match row {
        Some((Some(bytes), mime)) => Ok(Some((
            bytes,
            mime.unwrap_or_else(|| "audio/mpeg".to_string()),
        ))),
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
