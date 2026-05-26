mod commands;
mod db;
mod scheduler;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::open_deck,
            commands::close_deck,
            commands::current_deck,
            commands::deck_stats,
            commands::next_card,
            commands::rate_card,
            commands::card_audio,
            commands::list_decks,
            commands::deck_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
