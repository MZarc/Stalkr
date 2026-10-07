use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::Database;
use crate::models::{Account, AccountKind, AccountSession, ChangeType, Confidence, DecryptedSessionData, NoteRecord, ProviderType, RelationshipChange};
use crate::providers::{AuthenticatedSessionConfig, AuthenticatedSessionProvider, FixtureProvider, InstagramProvider, ProfileResult};
use crate::sync_coordinator::SyncCoordinator;
use crate::security::{NoteEncryptor, SessionEncryptor};

const KEY_B_MASTER_SEED: &str = "stalkr_security_domain_key_b_private_notes";

/// Simple pseudo-random integer in [min, max] using system nanos as entropy seed.
pub fn rand_in(min: usize, max: usize) -> usize {
    if max <= min { return min; }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    // Mix with a call-site-unique hash to vary across rapid calls
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let c = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let r = (nanos ^ (c.wrapping_mul(6364136223846793005))) % (max - min + 1);
    min + r
}

fn seed_note(db: &Database, account_id: &str, person_id: &str, content: &str, now: i64) {
    let key = NoteEncryptor::derive_key(KEY_B_MASTER_SEED);
    if let Ok((ciphertext, nonce)) = NoteEncryptor::encrypt(&key, content) {
        let record = NoteRecord {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: account_id.to_string(),
            person_id: person_id.to_string(),
            content_ciphertext: ciphertext,
            nonce,
            updated_at: now,
        };
        let _ = db.upsert_note_encrypted(&record);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHealthResponse {
    pub is_healthy: bool,
    pub last_validated_at: i64,
    pub error_message: Option<String>,
}

#[tauri::command]
pub async fn connect_instagram(
    db: State<'_, Arc<Database>>,
    session_id: String,
    ds_user_id: String,
    csrftoken: Option<String>,
    cookies: Option<String>,
    username: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    followers_count: Option<i64>,
    following_count: Option<i64>,
    is_private: Option<bool>,
    is_verified: Option<bool>,
) -> Result<Account, String> {
    if session_id.trim().is_empty() || ds_user_id.trim().is_empty() {
        return Err("Session ID and User ID (ds_user_id) are required.".to_string());
    }

    let config = AuthenticatedSessionConfig {
        session_id: session_id.trim().to_string(),
        ds_user_id: ds_user_id.trim().to_string(),
        csrftoken: csrftoken.clone(),
        cookies: cookies.clone(),
    };

    // 1. Determine profile: use verified data from native WebView when available
    //    When cookies are provided, the session came from an authentic in-app WebView login —
    //    NEVER call validate_session() over HTTP in this case. Instagram blocks Rust HTTP clients
    //    with 401/403 even for valid sessions. Trust the WebView-captured data directly.
    let has_native_cookies = cookies.as_ref().map(|c| !c.trim().is_empty()).unwrap_or(false);
    let has_native_username = username.as_ref().map(|u| !u.trim().is_empty()).unwrap_or(false);

    let profile = if has_native_cookies {
        // Native WebView login path: use whatever profile data we have from the WebView.
        // Even if profile JSON extraction failed (username empty), we still have valid
        // cookies + ds_user_id. Create a minimal profile — sync will refresh real metrics.
        let clean_uname = username
            .as_ref()
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| format!("user_{}", &ds_user_id.trim()[..ds_user_id.trim().len().min(8)]));

        ProfileResult {
            instagram_user_id: Some(ds_user_id.trim().to_string()),
            username: clean_uname.clone(),
            display_name: display_name
                .as_ref()
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| clean_uname.clone()),
            avatar_url: avatar_url.clone(),
            is_private: is_private.unwrap_or(false),
            is_verified: is_verified.unwrap_or(false),
            followers_count: followers_count.unwrap_or(0),
            following_count: following_count.unwrap_or(0),
        }
    } else if has_native_username {
        // Username provided but no cookies — trust it directly
        let clean_uname = username.unwrap().trim().to_string();
        ProfileResult {
            instagram_user_id: Some(ds_user_id.trim().to_string()),
            username: clean_uname.clone(),
            display_name: display_name.unwrap_or_else(|| clean_uname.clone()),
            avatar_url: avatar_url.clone(),
            is_private: is_private.unwrap_or(false),
            is_verified: is_verified.unwrap_or(false),
            followers_count: followers_count.unwrap_or(0),
            following_count: following_count.unwrap_or(0),
        }
    } else {
        // Manual session entry (no WebView cookies, no username): validate over HTTP
        let provider = AuthenticatedSessionProvider::new(Some(config.clone()));
        provider.validate_session().await.map_err(|e| format!("Authentication failed: {}", e))?
    };

    let now = chrono::Utc::now().timestamp();

    // 2. Check if account already exists by instagram_user_id or username
    let existing_account = if let Some(ref ig_id) = profile.instagram_user_id {
        db.get_account_by_instagram_id(ig_id).unwrap_or(None)
    } else {
        db.get_account_by_username(&profile.username).unwrap_or(None)
    };

    let account_id = existing_account.map(|a| a.id).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let account = Account {
        id: account_id.clone(),
        instagram_user_id: profile.instagram_user_id.clone(),
        username: profile.username.clone(),
        display_name: profile.display_name.clone(),
        account_kind: AccountKind::Owner,
        provider_type: ProviderType::Session,
        avatar_url: profile.avatar_url.clone(),
        is_private: profile.is_private,
        is_verified: profile.is_verified,
        followers_count: profile.followers_count,
        following_count: profile.following_count,
        monitoring_enabled: true,
        created_at: now,
        updated_at: now,
        last_successful_sync_at: None,
        last_attempted_sync_at: Some(now),
        authenticated_by_account_id: None, // Owner account is root
        access_state: "accessible".to_string(),
        access_reason: Some("Authenticated owner session".to_string()),
        target_privacy: if profile.is_private { "private".to_string() } else { "public".to_string() },
        last_access_checked_at: Some(now),
    };

    db.upsert_account(&account).map_err(|e| format!("Failed to save account: {}", e))?;

    // 3. Encrypt session credentials using Key A (Keystore domain)
    let session_data = DecryptedSessionData {
        session_id: config.session_id,
        ds_user_id: config.ds_user_id,
        csrftoken: config.csrftoken,
        cookies: config.cookies,
    };

    let (ciphertext, nonce) = SessionEncryptor::encrypt_session(&session_data)
        .map_err(|e| format!("Encryption error: {}", e))?;

    let account_session = AccountSession {
        account_id: account_id.clone(),
        session_data_ciphertext: ciphertext,
        nonce,
        last_validated_at: now,
        is_healthy: true,
        error_message: None,
    };

    db.save_account_session(&account_session)
        .map_err(|e| format!("Failed to persist session: {}", e))?;

    Ok(account)
}

#[tauri::command]
pub async fn connect_mock_owner(
    db: State<'_, Arc<Database>>,
    username: String,
    display_name: String,
    followers: i64,
    following: i64,
) -> Result<Account, String> {
    let now = chrono::Utc::now().timestamp();
    let account_id = uuid::Uuid::new_v4().to_string();

    let account = Account {
        id: account_id.clone(),
        instagram_user_id: Some(format!("mock_id_{}", username)),
        username: username.clone(),
        display_name: if display_name.is_empty() { username.clone() } else { display_name },
        account_kind: AccountKind::Owner,
        provider_type: ProviderType::Mock,
        avatar_url: None,
        is_private: false,
        is_verified: false,
        followers_count: followers,
        following_count: following,
        monitoring_enabled: true,
        created_at: now,
        updated_at: now,
        last_successful_sync_at: None,
        last_attempted_sync_at: Some(now),
        authenticated_by_account_id: None,
        access_state: "accessible".to_string(),
        access_reason: Some("Test owner account (Fixture)".to_string()),
        target_privacy: "public".to_string(),
        last_access_checked_at: Some(now),
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;

    // Also create dummy encrypted session
    let dummy_session = DecryptedSessionData {
        session_id: "mock_session".to_string(),
        ds_user_id: format!("mock_{}", username),
        csrftoken: None,
        cookies: None,
    };
    let (ciphertext, nonce) = SessionEncryptor::encrypt_session(&dummy_session)
        .map_err(|e| e.to_string())?;

    let session = AccountSession {
        account_id: account_id.clone(),
        session_data_ciphertext: ciphertext,
        nonce,
        last_validated_at: now,
        is_healthy: true,
        error_message: None,
    };
    db.save_account_session(&session).map_err(|e| e.to_string())?;

    Ok(account)
}

#[tauri::command]
pub async fn enter_demo_mode(
    db: State<'_, Arc<Database>>,
) -> Result<Account, String> {
    let username = "meetzarc".to_string();
    let display_name = "Meet Mistry".to_string();
    let followers = 1842;
    let following = 936;
    let now = chrono::Utc::now().timestamp();

    let existing = db.get_account_by_username(&username).map_err(|e| e.to_string())?;
    let account_id = existing.as_ref().map(|a| a.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let account = Account {
        id: account_id.clone(),
        instagram_user_id: Some("ig_meetzarc".to_string()),
        username: username.clone(),
        display_name: display_name.clone(),
        account_kind: AccountKind::Owner,
        provider_type: ProviderType::Mock,
        avatar_url: Some("/profile.png".to_string()),
        is_private: false,
        is_verified: true,
        followers_count: followers,
        following_count: following,
        monitoring_enabled: true,
        created_at: now - 86400 * 30,
        updated_at: now,
        last_successful_sync_at: Some(now - 120),
        last_attempted_sync_at: Some(now),
        authenticated_by_account_id: None,
        access_state: "accessible".to_string(),
        access_reason: Some("Authenticated owner session (Demo Mode)".to_string()),
        target_privacy: "public".to_string(),
        last_access_checked_at: Some(now),
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;

    // Create dummy session
    let dummy_session = DecryptedSessionData {
        session_id: "demo_session_meetzarc".to_string(),
        ds_user_id: "ig_meetzarc".to_string(),
        csrftoken: Some("demo_csrftoken".to_string()),
        cookies: None,
    };
    let (ciphertext, nonce) = SessionEncryptor::encrypt_session(&dummy_session)
        .map_err(|e| e.to_string())?;

    let session = AccountSession {
        account_id: account_id.clone(),
        session_data_ciphertext: ciphertext,
        nonce,
        last_validated_at: now,
        is_healthy: true,
        error_message: None,
    };
    db.save_account_session(&session).map_err(|e| e.to_string())?;

    // Force-regenerate fixture data (fresh random) every demo entry
    let owner_followers = rand_in(40, 85);
    let owner_following = rand_in(28, 65);
    let max_mutual = (owner_following.saturating_sub(6)).min(owner_followers.saturating_sub(6)).max(5);
    let owner_mutual = rand_in(5, max_mutual);
    let fixture = FixtureProvider::new(owner_followers, owner_following, owner_mutual);
    let _ = SyncCoordinator::execute_sync(&db, &account_id, &fixture).await;

    // Always re-seed changes (delete old then re-insert)
    seed_demo_changes_for_account(&db, &account_id);

    // Create a demo monitored private target: @private_friend
    let target_username = "private_friend".to_string();
    let existing_target = db.get_account_by_username(&target_username).map_err(|e| e.to_string())?;
    let target_id = existing_target.as_ref().map(|a| a.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let target_account = Account {
        id: target_id.clone(),
        instagram_user_id: Some("ig_private_friend".to_string()),
        username: target_username.clone(),
        display_name: "Private Friend".to_string(),
        account_kind: AccountKind::Monitored,
        provider_type: ProviderType::Mock,
        avatar_url: Some("https://images.unsplash.com/photo-1544005313-94ddf0286df2?auto=format&fit=crop&w=120&q=80".to_string()),
        is_private: true,
        is_verified: false,
        followers_count: 18,
        following_count: 14,
        monitoring_enabled: true,
        created_at: now - 86400 * 7,
        updated_at: now,
        last_successful_sync_at: Some(now - 180),
        last_attempted_sync_at: Some(now),
        authenticated_by_account_id: Some(account_id.clone()),
        access_state: "accessible".to_string(),
        access_reason: Some("Private account accessible through authenticated approved follower (@meetzarc).".to_string()),
        target_privacy: "private".to_string(),
        last_access_checked_at: Some(now),
    };
    db.upsert_account(&target_account).map_err(|e| e.to_string())?;

    let target_followers = rand_in(22, 45);
    let target_following = rand_in(15, 35);
    let max_target_mutual = (target_following.saturating_sub(5)).min(target_followers.saturating_sub(5)).max(3);
    let target_mutual = rand_in(3, max_target_mutual);
    let target_fixture = FixtureProvider::new(target_followers, target_following, target_mutual);
    let _ = SyncCoordinator::execute_sync(&db, &target_id, &target_fixture).await;

    // Always re-seed changes for target
    seed_demo_changes_for_account(&db, &target_id);

    Ok(account)
}

pub fn seed_demo_changes_if_needed(db: &Database, account_id: &str) {
    let Ok(account_opt) = db.get_account(account_id) else { return; };
    let Some(account) = account_opt else { return; };
    let auth_id = account.authenticated_by_account_id.as_deref();
    let current_count = db.get_changes_count(account_id, auth_id, None, None).unwrap_or(0);
    if current_count > 0 {
        return;
    }

    let now = chrono::Utc::now().timestamp();
    if account.username == "meetzarc" {
        let people = db.get_people_paginated(account_id, None, "all", None, None, 50, 0).unwrap_or_default();
        let find_pid = |handle: &str| {
            people.iter().find(|p| p.username == handle).map(|p| p.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        };

        let owner_changes = vec![
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_pid("alex_vance"),
                related_username: "alex_vance".to_string(),
                change_type: ChangeType::UnfollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 5,
                before_snapshot_id: Some("snap_prev_01".to_string()),
                after_snapshot_id: Some("snap_curr_01".to_string()),
                authenticated_by_account_id: None,
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_pid("elena.rostova"),
                related_username: "elena.rostova".to_string(),
                change_type: ChangeType::FollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 18,
                before_snapshot_id: Some("snap_prev_01".to_string()),
                after_snapshot_id: Some("snap_curr_01".to_string()),
                authenticated_by_account_id: None,
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_pid("marcus_dev"),
                related_username: "marcus_dev".to_string(),
                change_type: ChangeType::UsernameChanged,
                confidence: Confidence::Confirmed,
                metadata_json: Some("{\"old_username\":\"marcus_old\",\"new_username\":\"marcus_dev\"}".to_string()),
                detected_at: now - 3600 * 48,
                before_snapshot_id: Some("snap_prev_02".to_string()),
                after_snapshot_id: Some("snap_curr_02".to_string()),
                authenticated_by_account_id: None,
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_pid("luna_art"),
                related_username: "luna_art".to_string(),
                change_type: ChangeType::UnfollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 72,
                before_snapshot_id: Some("snap_prev_03".to_string()),
                after_snapshot_id: Some("snap_curr_03".to_string()),
                authenticated_by_account_id: None,
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_pid("david.co"),
                related_username: "david.co".to_string(),
                change_type: ChangeType::YouFollowed,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 96,
                before_snapshot_id: Some("snap_prev_04".to_string()),
                after_snapshot_id: Some("snap_curr_04".to_string()),
                authenticated_by_account_id: None,
            },
        ];
        let _ = db.insert_relationship_changes(&owner_changes);
        seed_note(db, account_id, &find_pid("elena.rostova"), "Creative Director at Studio Berlin. Shared connection from monograph.", now - 3600 * 12);
    } else if account.username == "private_friend" {
        let target_people = db.get_people_paginated(account_id, auth_id, "all", None, None, 50, 0).unwrap_or_default();
        let find_t_pid = |handle: &str| {
            target_people.iter().find(|p| p.username == handle).map(|p| p.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        };

        let target_changes = vec![
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_t_pid("chloe_paris"),
                related_username: "chloe_paris".to_string(),
                change_type: ChangeType::FollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 14,
                before_snapshot_id: Some("target_snap_01".to_string()),
                after_snapshot_id: Some("target_snap_02".to_string()),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_t_pid("zane.ai"),
                related_username: "zane.ai".to_string(),
                change_type: ChangeType::FollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 36,
                before_snapshot_id: Some("target_snap_01".to_string()),
                after_snapshot_id: Some("target_snap_02".to_string()),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_t_pid("marcus_dev"),
                related_username: "marcus_dev".to_string(),
                change_type: ChangeType::UsernameChanged,
                confidence: Confidence::Confirmed,
                metadata_json: Some("{\"old_username\":\"marcus_old\",\"new_username\":\"marcus_dev\"}".to_string()),
                detected_at: now - 3600 * 48,
                before_snapshot_id: Some("target_snap_02".to_string()),
                after_snapshot_id: Some("target_snap_03".to_string()),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_t_pid("olivia_travel"),
                related_username: "olivia_travel".to_string(),
                change_type: ChangeType::UnfollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 70,
                before_snapshot_id: Some("target_snap_03".to_string()),
                after_snapshot_id: Some("target_snap_04".to_string()),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            },
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_t_pid("kai_zenith"),
                related_username: "kai_zenith".to_string(),
                change_type: ChangeType::YouFollowed,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 90,
                before_snapshot_id: Some("target_snap_04".to_string()),
                after_snapshot_id: Some("target_snap_05".to_string()),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            },
        ];
        let _ = db.insert_relationship_changes(&target_changes);
        seed_note(db, account_id, &find_t_pid("marcus_dev"), "Close colleague of Private Friend from design circle.", now - 3600 * 20);
    }
}

/// Always-regenerate version: deletes existing changes then re-seeds with random timing.
pub fn seed_demo_changes_for_account(db: &Database, account_id: &str) {
    let Ok(account_opt) = db.get_account(account_id) else { return; };
    let Some(account) = account_opt else { return; };

    // Delete existing changes for this account first
    let sql = format!("DELETE FROM relationship_changes WHERE account_id = '{}';", account_id);
    let _ = db.execute_batch(&sql);

    let auth_id = account.authenticated_by_account_id.as_deref();
    let now = chrono::Utc::now().timestamp();

    if account.username == "meetzarc" {
        let people = db.get_people_paginated(account_id, None, "all", None, None, 50, 0).unwrap_or_default();
        let find_pid = |handle: &str| {
            people.iter().find(|p| p.username == handle).map(|p| p.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        };
        let picks: Vec<(&str, ChangeType, i64, Option<String>)> = vec![
            ("alex_vance", ChangeType::UnfollowedYou, 3, None),
            ("elena.rostova", ChangeType::FollowedYou, 14, None),
            ("marcus_dev", ChangeType::UsernameChanged, 28, Some("{\"old_username\":\"marcus_old\",\"new_username\":\"marcus_dev\"}".to_string())),
            ("luna_art", ChangeType::UnfollowedYou, 52, None),
            ("david.co", ChangeType::YouFollowed, 96, None),
            ("sophia.design", ChangeType::FollowedYou, 168, None), // 7 days
            ("kai_visuals", ChangeType::UnfollowedYou, 240, None),  // 10 days
            ("nina.travels", ChangeType::FollowedYou, 360, None),   // 15 days
            ("leo_sound", ChangeType::YouUnfollowed, 500, None),    // 20 days
            ("chloe.paris", ChangeType::FollowedYou, 720, None),    // 30 days
        ];

        let mut owner_changes = Vec::new();
        for (idx, (handle, ct, base_hours, meta)) in picks.into_iter().enumerate() {
            let jitter = rand_in(1, 6) as i64;
            let detected = now - 3600 * (base_hours + jitter);
            owner_changes.push(RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_pid(handle),
                related_username: handle.to_string(),
                change_type: ct,
                confidence: Confidence::Confirmed,
                metadata_json: meta,
                detected_at: detected,
                before_snapshot_id: Some(format!("snap_prev_{:02}", idx)),
                after_snapshot_id: Some(format!("snap_curr_{:02}", idx)),
                authenticated_by_account_id: None,
            });
        }
        let _ = db.insert_relationship_changes(&owner_changes);
        seed_note(db, account_id, &find_pid("elena.rostova"), "Creative Director at Studio Berlin. Shared connection from monograph.", now - 3600 * 12);
    } else if account.username == "private_friend" {
        let target_people = db.get_people_paginated(account_id, auth_id, "all", None, None, 50, 0).unwrap_or_default();
        let find_t_pid = |handle: &str| {
            target_people.iter().find(|p| p.username == handle).map(|p| p.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        };

        let t_picks: Vec<(&str, ChangeType, i64)> = vec![
            ("chloe_paris", ChangeType::FollowedYou, 5),
            ("zane.ai", ChangeType::FollowedYou, 18),
            ("olivia_travel", ChangeType::UnfollowedYou, 44),
            ("maya_arch", ChangeType::FollowedYou, 110),
            ("lucas.lens", ChangeType::UnfollowedYou, 260),
            ("clara_atelier", ChangeType::FollowedYou, 580),
        ];

        let mut target_changes = Vec::new();
        for (idx, (handle, ct, base_hours)) in t_picks.into_iter().enumerate() {
            let jitter = rand_in(1, 5) as i64;
            let detected = now - 3600 * (base_hours + jitter);
            target_changes.push(RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: find_t_pid(handle),
                related_username: handle.to_string(),
                change_type: ct,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: detected,
                before_snapshot_id: Some(format!("target_snap_{:02}", idx)),
                after_snapshot_id: Some(format!("target_snap_{:02}", idx + 1)),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            });
        }
        let _ = db.insert_relationship_changes(&target_changes);
        seed_note(db, account_id, &find_t_pid("chloe_paris"), "Close colleague of Private Friend from design circle.", now - 3600 * 20);
    }
}

#[tauri::command]
pub async fn get_session_health(
    db: State<'_, Arc<Database>>,
    account_id: String,
) -> Result<SessionHealthResponse, String> {
    let session = db.get_account_session(&account_id).map_err(|e| e.to_string())?;
    match session {
        Some(s) => Ok(SessionHealthResponse {
            is_healthy: s.is_healthy,
            last_validated_at: s.last_validated_at,
            error_message: s.error_message,
        }),
        None => Err("No session configured for this account".to_string()),
    }
}

#[tauri::command]
pub async fn revalidate_session(
    db: State<'_, Arc<Database>>,
    account_id: String,
) -> Result<Account, String> {
    let session_rec = db.get_account_session(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No session found for this account".to_string())?;

    let decrypted = SessionEncryptor::decrypt_session(&session_rec.session_data_ciphertext, &session_rec.nonce)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    let has_native_cookies = decrypted.cookies.as_ref().map(|c| !c.trim().is_empty()).unwrap_or(false);

    let config = AuthenticatedSessionConfig {
        session_id: decrypted.session_id,
        ds_user_id: decrypted.ds_user_id,
        csrftoken: decrypted.csrftoken,
        cookies: decrypted.cookies,
    };

    let provider = AuthenticatedSessionProvider::new(Some(config));
    let now = chrono::Utc::now().timestamp();

    // When the session came from the native WebView (has cookies), Instagram's private
    // API endpoints reject direct HTTP clients. Instead, just mark the session as healthy
    // based on the existing stored account data, and let sync_now do the real refresh.
    if has_native_cookies {
        let _ = db.update_session_health(&account_id, true, None, now);
        if let Some(mut acc) = db.get_account(&account_id).map_err(|e| e.to_string())? {
            acc.access_state = "accessible".to_string();
            acc.access_reason = Some("Native WebView session active".to_string());
            acc.last_access_checked_at = Some(now);
            acc.updated_at = now;
            db.upsert_account(&acc).map_err(|e| e.to_string())?;
            return Ok(acc);
        }
        return Err("Account not found".to_string());
    }

    match provider.validate_session().await {
        Ok(profile) => {
            let _ = db.update_session_health(&account_id, true, None, now);
            if let Some(mut acc) = db.get_account(&account_id).map_err(|e| e.to_string())? {
                acc.followers_count = profile.followers_count;
                acc.following_count = profile.following_count;
                acc.display_name = profile.display_name;
                acc.avatar_url = profile.avatar_url;
                acc.access_state = "accessible".to_string();
                acc.access_reason = Some("Session healthy and verified".to_string());
                acc.last_access_checked_at = Some(now);
                acc.updated_at = now;
                db.upsert_account(&acc).map_err(|e| e.to_string())?;
                Ok(acc)
            } else {
                Err("Account not found".to_string())
            }
        }
        Err(err) => {
            let _ = db.update_session_health(&account_id, false, Some(&err), now);
            if let Some(mut acc) = db.get_account(&account_id).map_err(|e| e.to_string())? {
                acc.access_state = "auth_required".to_string();
                acc.access_reason = Some(format!("Session rejected: {}", err));
                acc.last_access_checked_at = Some(now);
                db.upsert_account(&acc).map_err(|e| e.to_string())?;
                Ok(acc)
            } else {
                Err(format!("Validation failed: {}", err))
            }
        }
    }
}

#[tauri::command]
pub async fn disconnect_instagram(
    db: State<'_, Arc<Database>>,
    account_id: String,
) -> Result<(), String> {
    db.delete_account_session(&account_id).map_err(|e| e.to_string())?;
    db.delete_account(&account_id).map_err(|e| e.to_string())?;
    Ok(())
}
