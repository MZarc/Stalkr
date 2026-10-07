use std::sync::Arc;
use tauri::State;
use crate::commands::auth_commands::seed_demo_changes_if_needed;
use crate::db::Database;
use crate::models::ChangeFeedItem;

#[tauri::command]
pub async fn get_changes_feed(
    db: State<'_, Arc<Database>>,
    account_id: String,
    filter_type: Option<String>,
    search_query: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ChangeFeedItem>, String> {
    seed_demo_changes_if_needed(&db, &account_id);
    let account = db.get_account(&account_id).map_err(|e| e.to_string())?;
    let auth_id = account.as_ref().and_then(|a| a.authenticated_by_account_id.as_deref());
    db.get_changes_feed(
        &account_id,
        auth_id,
        filter_type.as_deref(),
        search_query.as_deref(),
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_changes_count(
    db: State<'_, Arc<Database>>,
    account_id: String,
    filter_type: Option<String>,
    search_query: Option<String>,
) -> Result<i64, String> {
    seed_demo_changes_if_needed(&db, &account_id);
    let account = db.get_account(&account_id).map_err(|e| e.to_string())?;
    let auth_id = account.as_ref().and_then(|a| a.authenticated_by_account_id.as_deref());
    db.get_changes_count(
        &account_id,
        auth_id,
        filter_type.as_deref(),
        search_query.as_deref(),
    )
    .map_err(|e| e.to_string())
}
