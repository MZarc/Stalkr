pub mod db;
pub mod models;
pub mod diff;
pub mod providers;
pub mod security;
pub mod sync_coordinator;
pub mod commands;
pub mod jni_bridge;

use std::sync::Arc;
use tauri::Manager;
use crate::db::Database;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            // Locate app data directory for SQLite persistence
            let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&app_data_dir).ok();
            let db_path = app_data_dir.join("stalkr.db");

            log::info!("Stalkr initializing SQLite database at: {:?}", db_path);
            let database = Database::new(&db_path).expect("Failed to initialize SQLite database");
            let db = Arc::new(database);

            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::get_accounts,
            crate::commands::get_account,
            crate::commands::create_account,
            crate::commands::delete_account,
            crate::commands::add_target_account,
            crate::commands::check_target_access,
            crate::commands::connect_instagram,
            crate::commands::connect_mock_owner,
            crate::commands::enter_demo_mode,
            crate::commands::get_session_health,
            crate::commands::revalidate_session,
            crate::commands::disconnect_instagram,
            crate::commands::get_people,
            crate::commands::get_people_count,
            crate::commands::get_target_overlap,
            crate::commands::get_relationship_summary,
            crate::commands::get_changes_feed,
            crate::commands::get_changes_count,
            crate::commands::get_note,
            crate::commands::save_note,
            crate::commands::delete_note,
            crate::commands::get_setting,
            crate::commands::set_setting,
            crate::commands::get_provider_health,
            crate::commands::delete_all_local_data,
            crate::commands::sync_now,
            crate::commands::import_raw_export_content,
        ])
        .run(tauri::generate_context!())
        .expect("error while running stalkr application");
}
