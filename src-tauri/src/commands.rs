use crate::db::{self, Card, DeckMeta, DeckStats, DeckSummary};
use crate::scheduler;
use chrono::Utc;
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub conn: Mutex<Option<(Connection, PathBuf)>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            conn: Mutex::new(None),
        }
    }
}

fn map_err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[derive(Serialize)]
pub struct AudioBlob {
    pub data: Vec<u8>,
    pub mime: String,
}

#[tauri::command]
pub fn open_deck(path: String, state: State<AppState>) -> Result<DeckMeta, String> {
    let path = PathBuf::from(path);
    let conn = db::open(&path).map_err(map_err)?;
    let meta = db::meta(&conn, &path).map_err(map_err)?;
    let mut guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some((conn, path));
    Ok(meta)
}

#[tauri::command]
pub fn close_deck(state: State<AppState>) -> Result<(), String> {
    let mut guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn current_deck(state: State<AppState>) -> Result<Option<DeckMeta>, String> {
    let guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some((conn, path)) => Ok(Some(db::meta(conn, path).map_err(map_err)?)),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn deck_stats(state: State<AppState>) -> Result<DeckStats, String> {
    let guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let (conn, _) = guard.as_ref().ok_or("no deck open")?;
    db::stats(conn, Utc::now()).map_err(map_err)
}

#[tauri::command]
pub fn next_card(state: State<AppState>) -> Result<Option<Card>, String> {
    let guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let (conn, _) = guard.as_ref().ok_or("no deck open")?;
    db::next_due_card(conn, Utc::now()).map_err(map_err)
}

#[tauri::command]
pub fn rate_card(id: i64, rating: u8, state: State<AppState>) -> Result<Option<Card>, String> {
    let mut guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let (conn, _) = guard.as_mut().ok_or("no deck open")?;

    let card = db::get_card(conn, id)
        .map_err(map_err)?
        .ok_or("card not found")?;

    let now = Utc::now();
    let result = scheduler::schedule(&card, rating, now).map_err(map_err)?;

    db::apply_review(
        conn,
        id,
        &result.updated,
        rating as i64,
        result.elapsed_days,
        result.scheduled_days,
        result.state_before,
    )
    .map_err(map_err)?;

    db::next_due_card(conn, now).map_err(map_err)
}

#[tauri::command]
pub fn undo_rating(card: Card, state: State<AppState>) -> Result<(), String> {
    let guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let (conn, _) = guard.as_ref().ok_or("no deck open")?;
    // Restore the card row to the snapshot the frontend held before the
    // last rate_card call.
    conn.execute(
        "UPDATE cards SET state = ?1, due = ?2, stability = ?3, difficulty = ?4,
                          reps = ?5, lapses = ?6, last_review = ?7
         WHERE id = ?8",
        rusqlite::params![
            card.state,
            card.due,
            card.stability,
            card.difficulty,
            card.reps,
            card.lapses,
            card.last_review,
            card.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    // Drop the most recent review_log entry for this card.
    conn.execute(
        "DELETE FROM review_log
         WHERE id = (
             SELECT id FROM review_log
             WHERE card_id = ?1
             ORDER BY reviewed_at DESC, id DESC
             LIMIT 1
         )",
        rusqlite::params![card.id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn card_audio(id: i64, state: State<AppState>) -> Result<Option<AudioBlob>, String> {
    let guard = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let (conn, _) = guard.as_ref().ok_or("no deck open")?;
    Ok(db::get_audio(conn, id)
        .map_err(map_err)?
        .map(|(data, mime)| AudioBlob { data, mime }))
}

fn resolve_deck_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("decks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
pub fn deck_dir(app: AppHandle) -> Result<String, String> {
    Ok(resolve_deck_dir(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn list_decks(app: AppHandle) -> Result<Vec<DeckSummary>, String> {
    let dir = resolve_deck_dir(&app)?;
    let mut out: Vec<DeckSummary> = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        // Accept both the canonical .lapse extension and the legacy .db
        // files from before the rename (so existing decks still appear).
        let ext = path.extension().and_then(|s| s.to_str());
        if ext != Some("lapse") && ext != Some("db") {
            continue;
        }
        // Open briefly to summarize. Files that aren't valid lapse decks are
        // skipped (logged for visibility in dev) so a stray .db doesn't kill
        // the whole listing.
        match db::open(&path).and_then(|c| db::summarize(&c, &path, Utc::now())) {
            Ok(summary) => out.push(summary),
            Err(e) => {
                eprintln!("[lapse] skipping {}: {e:#}", path.display());
                continue;
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}
