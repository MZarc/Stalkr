use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::Database;
use crate::models::PersonListItem;

#[derive(Debug, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub followers: i64,
    pub following: i64,
    pub mutual: i64,
    pub not_following_back: i64,
    pub fans: i64,
    pub net_delta_7d: i64,
    /// Authoritative Instagram header counts (what the official app shows).
    /// The tracked list (`followers`/`following` above) can legitimately sit
    /// a few percent below these — deactivated/restricted accounts are
    /// counted in the header but served by NO list endpoint. UIs should
    /// headline these and show tracked as the verified subset.
    #[serde(default)]
    pub profile_followers: Option<i64>,
    #[serde(default)]
    pub profile_following: Option<i64>,
}

#[tauri::command]
pub async fn get_people(
    db: State<'_, Arc<Database>>,
    account_id: String,
    filter_type: String,
    search_query: Option<String>,
    sort_by: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<PersonListItem>, String> {
    let account = db.get_account(&account_id).map_err(|e| e.to_string())?;
    let auth_id = account.as_ref().and_then(|a| a.authenticated_by_account_id.as_deref());

    db.get_people_paginated(
        &account_id,
        auth_id,
        &filter_type,
        search_query.as_deref(),
        sort_by.as_deref(),
        limit,
        offset,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_people_count(
    db: State<'_, Arc<Database>>,
    account_id: String,
    filter_type: String,
    search_query: Option<String>,
) -> Result<i64, String> {
    let account = db.get_account(&account_id).map_err(|e| e.to_string())?;
    let auth_id = account.as_ref().and_then(|a| a.authenticated_by_account_id.as_deref());

    db.get_people_count(
        &account_id,
        auth_id,
        &filter_type,
        search_query.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_relationship_summary(
    db: State<'_, Arc<Database>>,
    account_id: String,
) -> Result<RelationshipSummary, String> {
    let account = db.get_account(&account_id).map_err(|e| e.to_string())?;
    let auth_id = account.as_ref().and_then(|a| a.authenticated_by_account_id.as_deref());

    let (followers, following, mutual, not_following_back, fans) = db
        .get_relationship_counts(&account_id, auth_id)
        .map_err(|e| e.to_string())?;

    // Net delta 7 days calculation from relationship_changes
    let feed = db.get_changes_feed(&account_id, auth_id, None, None, 100, 0).unwrap_or_default();
    let seven_days_ago = chrono::Utc::now().timestamp() - (7 * 86400);

    let mut net_delta = 0i64;
    for change in feed {
        if change.detected_at >= seven_days_ago {
            match change.change_type {
                crate::models::ChangeType::FollowedYou => net_delta += 1,
                crate::models::ChangeType::UnfollowedYou => net_delta -= 1,
                _ => {}
            }
        }
    }

    Ok(RelationshipSummary {
        followers,
        following,
        mutual,
        not_following_back,
        fans,
        net_delta_7d: net_delta,
        // accounts.followers_count tracks the Instagram header since the
        // premium sync (WebView header on fresh accounts). Zero means
        // "unknown", not "zero followers" — the UI falls back to tracked.
        profile_followers: account.as_ref().and_then(|a| {
            if a.followers_count > 0 {
                Some(a.followers_count)
            } else {
                None
            }
        }),
        profile_following: account.as_ref().and_then(|a| {
            if a.following_count > 0 {
                Some(a.following_count)
            } else {
                None
            }
        }),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetOverlapReport {
    pub mutual_connections_count: i64,
    pub shared_people: Vec<PersonListItem>,
}

#[tauri::command]
pub async fn get_target_overlap(
    db: State<'_, Arc<Database>>,
    owner_account_id: String,
    target_account_id: String,
    limit: Option<i64>,
) -> Result<TargetOverlapReport, String> {
    let limit = limit.unwrap_or(20);
    let (count, shared) = db.get_target_overlap(&owner_account_id, &target_account_id, limit)
        .map_err(|e| e.to_string())?;

    Ok(TargetOverlapReport {
        mutual_connections_count: count,
        shared_people: shared,
    })
}
