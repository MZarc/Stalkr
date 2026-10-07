use std::sync::Arc;
use tauri::State;
use crate::db::Database;
use crate::providers::ProviderHealthStatus;

#[tauri::command]
pub async fn get_setting(db: State<'_, Arc<Database>>, key: String) -> Result<Option<String>, String> {
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(db: State<'_, Arc<Database>>, key: String, value: String) -> Result<(), String> {
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_health(
    db: State<'_, Arc<Database>>,
    account_id: Option<String>,
) -> Result<ProviderHealthStatus, String> {
    let mut health = ProviderHealthStatus::default();

    if let Some(ref aid) = account_id {
        if let Ok(Some(account)) = db.get_account(aid) {
            health.last_successful_sync = account.last_successful_sync_at;
            health.provider_name = account.provider_type.as_str().to_string();

            if account.last_successful_sync_at.is_some() {
                health.is_connected = true;
                health.status_text = "Verified and operational".to_string();
            } else {
                health.is_connected = false;
                health.status_text = "Awaiting initial sync".to_string();
            }
        }
    }

    Ok(health)
}

#[tauri::command]
pub async fn delete_all_local_data(db: State<'_, Arc<Database>>) -> Result<(), String> {
    let sql = r#"
        DELETE FROM pending_events;
        DELETE FROM relationship_changes;
        DELETE FROM snapshot_members;
        DELETE FROM snapshots;
        DELETE FROM relationship_state;
        DELETE FROM notes;
        DELETE FROM person_tags;
        DELETE FROM tags;
        DELETE FROM people;
        DELETE FROM account_sessions;
        DELETE FROM account_aliases;
        DELETE FROM accounts;
        DELETE FROM settings;
        VACUUM;
    "#;
    db.execute_batch(sql).map_err(|e| format!("Nuclear purge failed: {}", e))
}
