use crate::db::{self, Card, DeckMeta, DeckStats};
use crate::scheduler;
use chrono::Utc;
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

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
    let mut guard = state.conn.lock().map_err(|e| e.to_string())?;
    *guard = Some((conn, path));
    Ok(meta)
}

#[tauri::command]
pub fn close_deck(state: State<AppState>) -> Result<(), String> {
    let mut guard = state.conn.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn current_deck(state: State<AppState>) -> Result<Option<DeckMeta>, String> {
    let guard = state.conn.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some((conn, path)) => Ok(Some(db::meta(conn, path).map_err(map_err)?)),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn deck_stats(state: State<AppState>) -> Result<DeckStats, String> {
    let guard = state.conn.lock().map_err(|e| e.to_string())?;
    let (conn, _) = guard.as_ref().ok_or("no deck open")?;
    db::stats(conn, Utc::now()).map_err(map_err)
}

#[tauri::command]
pub fn next_card(state: State<AppState>) -> Result<Option<Card>, String> {
    let guard = state.conn.lock().map_err(|e| e.to_string())?;
    let (conn, _) = guard.as_ref().ok_or("no deck open")?;
    db::next_due_card(conn, Utc::now()).map_err(map_err)
}

#[tauri::command]
pub fn rate_card(id: i64, rating: u8, state: State<AppState>) -> Result<Option<Card>, String> {
    let mut guard = state.conn.lock().map_err(|e| e.to_string())?;
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
pub fn card_audio(id: i64, state: State<AppState>) -> Result<Option<AudioBlob>, String> {
    let guard = state.conn.lock().map_err(|e| e.to_string())?;
    let (conn, _) = guard.as_ref().ok_or("no deck open")?;
    Ok(db::get_audio(conn, id)
        .map_err(map_err)?
        .map(|(data, mime)| AudioBlob { data, mime }))
}
