use std::sync::Arc;
use sha2::{Digest, Sha256};
use tauri::State;
use crate::db::Database;
use crate::diff::DiffEngine;
use crate::models::{AccountKind, Confidence, ProviderType, Snapshot, SnapshotStatus, SnapshotType};
use crate::providers::*;
use crate::security::SessionEncryptor;
use crate::sync_coordinator::{SyncCoordinator, SyncReport};

#[tauri::command]
pub async fn sync_now(
    db: State<'_, Arc<Database>>,
    account_id: String,
) -> Result<SyncReport, String> {
    let account = db.get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account {} not found", account_id))?;

    // Guardrail: For monitored targets, verify access state before attempting sync
    if account.account_kind == AccountKind::Monitored && account.access_state == "not_accessible" {
        return Ok(SyncReport {
            success: false,
            status: "not_accessible".to_string(),
            followers_count: account.followers_count,
            following_count: account.following_count,
            changes_detected: 0,
            message: account.access_reason.unwrap_or_else(|| {
                "This private account's relationship lists are not accessible through the connected Instagram account.".to_string()
            }),
            profile_followers_count: None,
            profile_following_count: None,
            tracked_followers_count: None,
            tracked_following_count: None,
            verification: Some("not_accessible".to_string()),
        });
    }

    match account.provider_type {
        ProviderType::Mock => {
            let (f, fg, m) = if account.username == "private_friend" {
                let f = crate::commands::auth_commands::rand_in(20, 50);
                let fg = crate::commands::auth_commands::rand_in(15, 38);
                let max_m = (fg.saturating_sub(5)).min(f.saturating_sub(5)).max(3);
                let m = crate::commands::auth_commands::rand_in(3, max_m);
                (f, fg, m)
            } else {
                let f = crate::commands::auth_commands::rand_in(35, 85);
                let fg = crate::commands::auth_commands::rand_in(25, 65);
                let max_m = (fg.saturating_sub(6)).min(f.saturating_sub(6)).max(5);
                let m = crate::commands::auth_commands::rand_in(5, max_m);
                (f, fg, m)
            };

            let now = chrono::Utc::now().timestamp();
            let mut updated_account = account.clone();
            updated_account.followers_count = f as i64;
            updated_account.following_count = fg as i64;
            updated_account.updated_at = now;
            updated_account.last_successful_sync_at = Some(now);
            let _ = db.upsert_account(&updated_account);

            let fixture = FixtureProvider::new(f, fg, m);
            let res = SyncCoordinator::execute_sync(&db, &account_id, &fixture).await?;

            // Re-seed demo changes with fresh timestamps
            crate::commands::auth_commands::seed_demo_changes_for_account(&db, &account_id);

            Ok(res)
        }
        ProviderType::Session => {
            // Find owner account ID that authenticates this session
            let auth_account_id = account.authenticated_by_account_id.as_deref().unwrap_or(&account.id);
            let session_record = db.get_account_session(auth_account_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "No authenticated session found. Please connect or re-authenticate your Instagram account.".to_string())?;

            let decrypted = SessionEncryptor::decrypt_session(&session_record.session_data_ciphertext, &session_record.nonce)
                .map_err(|e| format!("Session decryption failed: {}", e))?;

            let config = AuthenticatedSessionConfig {
                session_id: decrypted.session_id,
                ds_user_id: decrypted.ds_user_id.clone(),
                csrftoken: decrypted.csrftoken,
                cookies: decrypted.cookies,
            };

            let provider = AuthenticatedSessionProvider::new(Some(config));

            // For owner accounts: try to refresh real profile data (username, avatar, metrics)
            // before the main follower/following sync. This corrects any placeholder username
            // that was set when the WebView JS profile fetch failed.
            if account.account_kind == crate::models::AccountKind::Owner {
                if let Ok(profile) = provider.validate_session().await {
                    let now = chrono::Utc::now().timestamp();
                    if let Ok(Some(mut fresh)) = db.get_account(&account_id) {
                        let mut changed = false;
                        // Update username only if it was a placeholder (starts with "user_")
                        if fresh.username.starts_with("user_") && !profile.username.is_empty() {
                            fresh.username = profile.username;
                            changed = true;
                        }
                        if profile.display_name != fresh.display_name {
                            fresh.display_name = profile.display_name;
                            changed = true;
                        }
                        if profile.avatar_url.is_some() {
                            fresh.avatar_url = profile.avatar_url;
                            changed = true;
                        }
                        if changed {
                            fresh.updated_at = now;
                            let _ = db.upsert_account(&fresh);
                        }
                    }
                }
            }

            SyncCoordinator::execute_sync(&db, &account_id, &provider).await
        }
        ProviderType::PublicProfile => {
            let provider = PublicProfileProvider::default();
            SyncCoordinator::execute_sync(&db, &account_id, &provider).await
        }
        ProviderType::Export => {
            let provider = ExportArchiveProvider;
            SyncCoordinator::execute_sync(&db, &account_id, &provider).await
        }
    }
}

#[tauri::command]
pub async fn import_raw_export_content(
    db: State<'_, Arc<Database>>,
    account_id: String,
    followers_json: String,
    following_json: String,
) -> Result<SyncReport, String> {
    let account = db.get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account {} not found", account_id))?;

    let now = chrono::Utc::now().timestamp();

    // 1. Parse followers JSON
    let raw_followers = ExportArchiveProvider::parse_export_json_content(&followers_json)
        .map_err(|e| format!("Followers JSON parsing failed: {}", e))?;
    if raw_followers.is_empty() {
        return Err("No followers found in the uploaded followers JSON.".to_string());
    }

    // 2. Parse following JSON
    let raw_following = ExportArchiveProvider::parse_export_json_content(&following_json)
        .map_err(|e| format!("Following JSON parsing failed: {}", e))?;
    if raw_following.is_empty() {
        return Err("No following users found in the uploaded following JSON.".to_string());
    }

    let mut f_hasher = Sha256::new();
    f_hasher.update(followers_json.as_bytes());
    let followers_hash = format!("{:x}", f_hasher.finalize());

    let mut g_hasher = Sha256::new();
    g_hasher.update(following_json.as_bytes());
    let following_hash = format!("{:x}", g_hasher.finalize());

    // 3. Resolve into canonical people
    let follower_people = db.upsert_people_batch(&raw_followers, now)
        .map_err(|e| format!("Failed to upsert followers: {}", e))?;
    let following_people = db.upsert_people_batch(&raw_following, now)
        .map_err(|e| format!("Failed to upsert following: {}", e))?;

    // 4. Check for previous complete snapshot
    let prev_snap = db.get_latest_complete_snapshot(
        &account.id,
        SnapshotType::Followers,
        account.authenticated_by_account_id.as_deref(),
    ).map_err(|e| e.to_string())?;

    // 5. Store new snapshots
    let followers_snap_id = uuid::Uuid::new_v4().to_string();
    let followers_snap = Snapshot {
        id: followers_snap_id.clone(),
        account_id: account.id.clone(),
        snapshot_type: SnapshotType::Followers,
        started_at: now,
        completed_at: Some(now),
        status: SnapshotStatus::Complete,
        item_count: follower_people.len() as i64,
        source_hash: followers_hash,
        error_message: None,
        authenticated_by_account_id: account.authenticated_by_account_id.clone(),
    };
    db.insert_snapshot(&followers_snap).map_err(|e| e.to_string())?;
    db.insert_snapshot_members(&followers_snap_id, &follower_people).map_err(|e| e.to_string())?;

    let following_snap_id = uuid::Uuid::new_v4().to_string();
    let following_snap = Snapshot {
        id: following_snap_id.clone(),
        account_id: account.id.clone(),
        snapshot_type: SnapshotType::Following,
        started_at: now,
        completed_at: Some(now),
        status: SnapshotStatus::Complete,
        item_count: following_people.len() as i64,
        source_hash: following_hash,
        error_message: None,
        authenticated_by_account_id: account.authenticated_by_account_id.clone(),
    };
    db.insert_snapshot(&following_snap).map_err(|e| e.to_string())?;
    db.insert_snapshot_members(&following_snap_id, &following_people).map_err(|e| e.to_string())?;

    // 6. Diff with previous complete snapshot
    let mut total_changes = Vec::new();
    if let Some(prev) = prev_snap {
        let old_members = db.get_snapshot_members(&prev.id).map_err(|e| e.to_string())?;
        let current_members = db.get_snapshot_members(&followers_snap_id).map_err(|e| e.to_string())?;

        let diffs = DiffEngine::diff_snapshots(
            &account.id,
            SnapshotType::Followers,
            Some(&prev.id),
            &followers_snap_id,
            &old_members,
            &current_members,
            now,
            Confidence::Confirmed,
        );
        total_changes.extend(diffs);
    }

    for c in &mut total_changes {
        c.authenticated_by_account_id = account.authenticated_by_account_id.clone();
    }

    if !total_changes.is_empty() {
        db.insert_relationship_changes(&total_changes).map_err(|e| e.to_string())?;
    }

    // 7. Update relationship state table
    let cur_followers = db.get_snapshot_members(&followers_snap_id).map_err(|e| e.to_string())?;
    let cur_following = db.get_snapshot_members(&following_snap_id).map_err(|e| e.to_string())?;
    let mut rel_states = DiffEngine::compute_relationship_states(
        &account.id,
        &cur_followers,
        &cur_following,
        now,
    );
    for s in &mut rel_states {
        s.authenticated_by_account_id = account.authenticated_by_account_id.clone();
    }
    db.upsert_relationship_state_batch(&account.id, &rel_states).map_err(|e| e.to_string())?;
    {
        let keep: Vec<String> = rel_states.iter().map(|s| s.person_id.clone()).collect();
        let _ = db.prune_relationship_state(&account.id, &keep);
    }

    // 8. Update account record
    let mut updated_account = account.clone();
    updated_account.followers_count = follower_people.len() as i64;
    updated_account.following_count = following_people.len() as i64;
    updated_account.last_successful_sync_at = Some(now);
    updated_account.updated_at = now;
    db.upsert_account(&updated_account).map_err(|e| e.to_string())?;

    let changes_detected = total_changes.len();

    Ok(SyncReport {
        success: true,
        status: "complete".to_string(),
        followers_count: follower_people.len() as i64,
        following_count: following_people.len() as i64,
        changes_detected,
        message: format!(
            "Successfully imported {} followers and {} following. {} relationship changes recorded.",
            follower_people.len(),
            following_people.len(),
            changes_detected
        ),
        // Export archives are user-provided ground truth: header == list.
        profile_followers_count: Some(follower_people.len() as i64),
        profile_following_count: Some(following_people.len() as i64),
        tracked_followers_count: Some(follower_people.len() as i64),
        tracked_following_count: Some(following_people.len() as i64),
        verification: Some("export_ground_truth".to_string()),
    })
}
