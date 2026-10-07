use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::Database;
use crate::models::{Account, AccountKind, ProviderType};
use crate::providers::*;
use crate::security::SessionEncryptor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTargetResponse {
    pub account: Account,
    pub access_check: TargetAccessCheckResult,
}

#[tauri::command]
pub async fn get_accounts(db: State<'_, Arc<Database>>) -> Result<Vec<Account>, String> {
    db.list_accounts().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_account(db: State<'_, Arc<Database>>, id: String) -> Result<Option<Account>, String> {
    db.get_account(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_account(
    db: State<'_, Arc<Database>>,
    username: String,
    display_name: String,
    account_kind: String,
    provider_type: String,
    instagram_user_id: Option<String>,
) -> Result<Account, String> {
    let now = chrono::Utc::now().timestamp();
    let account = Account {
        id: uuid::Uuid::new_v4().to_string(),
        instagram_user_id,
        username,
        display_name,
        account_kind: AccountKind::from_str(&account_kind),
        provider_type: ProviderType::from_str(&provider_type),
        avatar_url: None,
        is_private: false,
        is_verified: false,
        followers_count: 0,
        following_count: 0,
        monitoring_enabled: true,
        created_at: now,
        updated_at: now,
        last_successful_sync_at: None,
        last_attempted_sync_at: None,
        authenticated_by_account_id: None,
        access_state: "unknown".to_string(),
        access_reason: None,
        target_privacy: "unknown".to_string(),
        last_access_checked_at: None,
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;
    Ok(account)
}

#[tauri::command]
pub async fn delete_account(db: State<'_, Arc<Database>>, id: String) -> Result<(), String> {
    db.delete_account(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_target_account(
    db: State<'_, Arc<Database>>,
    owner_account_id: String,
    target_username: String,
) -> Result<AddTargetResponse, String> {
    let clean_target = target_username.trim().trim_start_matches('@').to_string();
    if clean_target.is_empty() {
        return Err("Target username cannot be empty".to_string());
    }

    let owner = db.get_account(&owner_account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Owner account not found".to_string())?;

    let now = chrono::Utc::now().timestamp();

    // Check access using owner's provider
    let access_check = match owner.provider_type {
        ProviderType::Session => {
            let session = db.get_account_session(&owner.id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Owner session not found. Please re-authenticate owner account.".to_string())?;

            let decrypted = SessionEncryptor::decrypt_session(&session.session_data_ciphertext, &session.nonce)
                .map_err(|e| format!("Decryption failed: {}", e))?;

            let config = AuthenticatedSessionConfig {
                session_id: decrypted.session_id,
                ds_user_id: decrypted.ds_user_id,
                csrftoken: decrypted.csrftoken,
                cookies: decrypted.cookies,
            };

            let provider = AuthenticatedSessionProvider::new(Some(config));
            provider.check_target_access(&clean_target).await?
        }
        ProviderType::Mock => {
            let provider = FixtureProvider::new(30, 20, 15);
            provider.check_target_access(&clean_target).await?
        }
        _ => {
            return Err("Target accounts must be added through an Authenticated Session or Mock owner.".to_string());
        }
    };

    // Upsert target account record
    let existing = db.get_account_by_username(&clean_target).map_err(|e| e.to_string())?;
    let target_id = existing.map(|e| e.id).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Check local database for existing person details (e.g. if target is in owner's circle)
    let local_person = db.get_person_by_username(&clean_target).unwrap_or(None);

    let final_ig_id = access_check.target_user_id.clone()
        .or_else(|| local_person.as_ref().and_then(|p| p.instagram_user_id.clone()));

    let final_avatar = access_check.avatar_url.clone()
        .or_else(|| local_person.as_ref().and_then(|p| p.avatar_url.clone()));

    let target_account = Account {
        id: target_id,
        instagram_user_id: final_ig_id,
        username: clean_target,
        display_name: access_check.target_username.clone(),
        account_kind: AccountKind::Monitored,
        provider_type: owner.provider_type,
        avatar_url: final_avatar,
        is_private: access_check.privacy_state == PrivacyState::Private || local_person.as_ref().map(|p| p.is_private).unwrap_or(false),
        is_verified: access_check.access_state == AccessState::Accessible && local_person.as_ref().map(|p| p.is_verified).unwrap_or(false),
        followers_count: access_check.followers_count,
        following_count: access_check.following_count,
        monitoring_enabled: true,
        created_at: now,
        updated_at: now,
        last_successful_sync_at: None,
        last_attempted_sync_at: None,
        authenticated_by_account_id: Some(owner.id),
        access_state: access_check.access_state.as_str().to_string(),
        access_reason: Some(access_check.access_reason.clone()),
        target_privacy: access_check.privacy_state.as_str().to_string(),
        last_access_checked_at: Some(now),
    };

    db.upsert_account(&target_account).map_err(|e| e.to_string())?;

    Ok(AddTargetResponse {
        account: target_account,
        access_check,
    })
}

#[tauri::command]
pub async fn check_target_access(
    db: State<'_, Arc<Database>>,
    owner_account_id: String,
    target_account_id: String,
) -> Result<TargetAccessCheckResult, String> {
    let owner = db.get_account(&owner_account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Owner account not found".to_string())?;

    let mut target = db.get_account(&target_account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Target account not found".to_string())?;

    let now = chrono::Utc::now().timestamp();

    let access_check = match owner.provider_type {
        ProviderType::Session => {
            let session = db.get_account_session(&owner.id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Owner session not found. Please re-authenticate owner account.".to_string())?;

            let decrypted = SessionEncryptor::decrypt_session(&session.session_data_ciphertext, &session.nonce)
                .map_err(|e| format!("Decryption failed: {}", e))?;

            let config = AuthenticatedSessionConfig {
                session_id: decrypted.session_id,
                ds_user_id: decrypted.ds_user_id,
                csrftoken: decrypted.csrftoken,
                cookies: decrypted.cookies,
            };

            let provider = AuthenticatedSessionProvider::new(Some(config));
            let mut check = provider.check_target_access(&target.username).await?;

            // Fallback: the username path needs a live profile lookup, the
            // flakiest Instagram surface. When it comes back inconclusive but
            // we hold a proven numeric id (sync already uses it), probe the
            // relationship endpoints directly instead of reporting Unknown.
            let inconclusive = check.access_state == AccessState::Unknown
                || check.target_user_id.is_none();
            if inconclusive {
                if let Some(stored) = target.instagram_user_id.clone() {
                    let trimmed = stored.trim().to_string();
                    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(probe) =
                            provider.probe_target_by_id(&trimmed, &target.username).await
                        {
                            if probe.access_state != AccessState::Unknown
                                && probe.access_state != AccessState::ProviderError
                            {
                                check = probe;
                            }
                        }
                    }
                }
            }
            check
        }
        ProviderType::Mock => {
            let provider = FixtureProvider::new(30, 20, 15);
            provider.check_target_access(&target.username).await?
        }
        _ => {
            return Err("Access checking requires Authenticated Session or Mock owner.".to_string());
        }
    };

    // Adopt the fresh result — but never let an inconclusive recheck clobber
    // previously known good values (that is how targets got stuck showing
    // "unknown" forever: each failed recheck overwrote the good state).
    let decisive = access_check.access_state != AccessState::Unknown;
    if decisive || target.access_state == "unknown" {
        target.access_state = access_check.access_state.as_str().to_string();
        target.access_reason = Some(access_check.access_reason.clone());
    }
    if access_check.privacy_state != PrivacyState::Unknown {
        target.target_privacy = access_check.privacy_state.as_str().to_string();
    }
    target.last_access_checked_at = Some(now);
    target.updated_at = now;
    // Header counts go stale fast (every follow/unfollow changes them), so
    // always refresh when Instagram returned a real value — never freeze on
    // the first-ever check. A zero from the provider means "unknown", not
    // "this account has no followers", so zeros never overwrite.
    if access_check.followers_count > 0 {
        target.followers_count = access_check.followers_count;
        target.following_count = access_check.following_count;
    }
    if target.avatar_url.is_none() && access_check.avatar_url.is_some() {
        target.avatar_url = access_check.avatar_url.clone();
    }
    if target.instagram_user_id.is_none() && access_check.target_user_id.is_some() {
        target.instagram_user_id = access_check.target_user_id.clone();
    }

    db.upsert_account(&target).map_err(|e| e.to_string())?;

    Ok(access_check)
}
