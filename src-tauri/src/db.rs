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
    /// Which side the audio's content belongs to: "front", "back", "both",
    /// or None when undecided (older decks predating schema v2). The
    /// review screen uses this to choose autoplay timing.
    pub audio_side: Option<String>,
    pub tags: Vec<String>,
    pub state: i64,
    pub due: i64,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    pub last_review: Option<i64>,
    /// Index into the scheduler's LEARN_STEPS_MS ladder. None once the
    /// card has graduated to Review (FSRS-managed).
    pub learn_step: Option<i64>,
    /// Optional usage example shown on the back of the card. Multi-line:
    /// canonical format is <sentence>\n<transliteration>\n<gloss>, but
    /// builders can use any text — the UI renders it as a single block.
    pub example: Option<String>,
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

// Probe query: validates that `cards` has the columns we ALWAYS read.
// audio_side (added in v2) is deliberately omitted — older decks will
// pass validation and then be migrated below.
const COLUMN_PROBE: &str = "SELECT id, front, back, audio, audio_mime, tags, \
                            state, due, stability, difficulty, reps, lapses, last_review \
                            FROM cards LIMIT 0";

pub fn open(path: &Path) -> Result<Connection> {
    // Validate the file read-only first so a rejected foreign DB is left
    // untouched on disk (no journal artifacts written next to it). Skip the
    // probe for missing or zero-byte files — SQLite refuses to open them
    // read-only, but the write-mode open below will initialise them cleanly.
    let needs_probe = std::fs::metadata(path)
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false);
    if needs_probe {
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
    migrate(&conn)?;
    Ok(conn)
}

/// In-place migrations for older deck versions. Idempotent.
fn migrate(conn: &Connection) -> Result<()> {
    let has_col = |name: &str| -> Result<bool> {
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('cards') WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    };

    // v1 -> v2: cards.audio_side
    if !has_col("audio_side")? {
        conn.execute("ALTER TABLE cards ADD COLUMN audio_side TEXT", [])?;
    }
    // v2 -> v3: cards.learn_step
    if !has_col("learn_step")? {
        conn.execute("ALTER TABLE cards ADD COLUMN learn_step INTEGER", [])?;
    }
    // v3 -> v4: cards.example
    if !has_col("example")? {
        conn.execute("ALTER TABLE cards ADD COLUMN example TEXT", [])?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES ('schema_version', '4')",
        [],
    )?;
    Ok(())
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
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
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

const CARD_SELECT: &str =
    "SELECT id, front, back, audio IS NOT NULL, audio_side, tags, \
            state, due, stability, difficulty, reps, lapses, last_review, learn_step, \
            example \
     FROM cards";

pub fn next_due_card(conn: &Connection, now: DateTime<Utc>) -> Result<Option<Card>> {
    let now_ms = now.timestamp_millis();
    let sql = format!(
        "{CARD_SELECT}
         WHERE (state = 0) OR (state != 0 AND due <= ?1)
         ORDER BY
             CASE state WHEN 1 THEN 0 WHEN 3 THEN 0 WHEN 2 THEN 1 ELSE 2 END,
             CASE WHEN state = 0 THEN random() ELSE due END,
             id ASC
         LIMIT 1"
    );
    let row = conn
        .query_row(&sql, params![now_ms], row_to_card)
        .optional()?;
    Ok(row)
}

fn row_to_card(row: &rusqlite::Row) -> rusqlite::Result<Card> {
    let tags: String = row.get(5)?;
    Ok(Card {
        id: row.get(0)?,
        front: row.get(1)?,
        back: row.get(2)?,
        has_audio: row.get::<_, bool>(3)?,
        audio_side: row.get(4)?,
        tags: tags.split_whitespace().map(|s| s.to_string()).collect(),
        state: row.get(6)?,
        due: row.get(7)?,
        stability: row.get(8)?,
        difficulty: row.get(9)?,
        reps: row.get(10)?,
        lapses: row.get(11)?,
        last_review: row.get(12)?,
        learn_step: row.get(13)?,
        example: row.get(14)?,
    })
}

pub fn get_card(conn: &Connection, id: i64) -> Result<Option<Card>> {
    let sql = format!("{CARD_SELECT} WHERE id = ?1");
    let row = conn
        .query_row(&sql, params![id], row_to_card)
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
    pub learn_step: Option<i64>,
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
                          reps = ?5, lapses = ?6, last_review = ?7, learn_step = ?8
         WHERE id = ?9",
        params![
            updated.state,
            updated.due,
            updated.stability,
            updated.difficulty,
            updated.reps,
            updated.lapses,
            updated.last_review,
            updated.learn_step,
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
