#[cfg(target_os = "android")]
use jni::objects::{JClass, JString};
#[cfg(target_os = "android")]
use jni::sys::jstring;
#[cfg(target_os = "android")]
use jni::JNIEnv;

/// Native JNI entrypoint invoked directly by Kotlin WorkManager CoroutineWorker.
/// Bypasses the Svelte WebView completely, ensuring reliable headless periodic execution on Android.
#[cfg(target_os = "android")]
#[cfg(target_os = "android")]
fn run_headless_sync(
    mut env: JNIEnv,
    j_account_id: JString,
    j_db_path: JString,
) -> jstring {
    let account_id: String = match env.get_string(&j_account_id) {
        Ok(s) => s.into(),
        Err(_) => return env.new_string("{\"success\":false,\"error\":\"Invalid account_id\"}").unwrap().into_raw(),
    };

    let db_path: String = match env.get_string(&j_db_path) {
        Ok(s) => s.into(),
        Err(_) => return env.new_string("{\"success\":false,\"error\":\"Invalid db_path\"}").unwrap().into_raw(),
    };

    let db = match crate::db::Database::new(&db_path) {
        Ok(d) => std::sync::Arc::new(d),
        Err(e) => {
            let err_json = format!("{{\"success\":false,\"error\":\"Database error: {}\"}}", e);
            return env.new_string(err_json).unwrap().into_raw();
        }
    };

    // Execute synchronous or runtime-blocked sync for the account
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    let result_json = match rt {
        Ok(runtime) => {
            runtime.block_on(async {
                let account = match db.get_account(&account_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => return "{\"success\":false,\"error\":\"Account not found\"}".to_string(),
                    Err(e) => return format!("{{\"success\":false,\"error\":\"DB read error: {}\"}}", e),
                };

                match account.provider_type {
                    crate::models::ProviderType::Session => {
                        let auth_account_id = account.authenticated_by_account_id.as_deref().unwrap_or(&account.id);
                        let session_record = match db.get_account_session(auth_account_id) {
                            Ok(Some(s)) => s,
                            Ok(None) => return "{\"success\":false,\"error\":\"No authenticated session found\"}".to_string(),
                            Err(e) => return format!("{{\"success\":false,\"error\":\"Session DB error: {}\"}}", e),
                        };

                        let decrypted = match crate::security::SessionEncryptor::decrypt_session(&session_record.session_data_ciphertext, &session_record.nonce) {
                            Ok(d) => d,
                            Err(e) => return format!("{{\"success\":false,\"error\":\"Decryption error: {}\"}}", e),
                        };

                        let config = crate::providers::AuthenticatedSessionConfig {
                            session_id: decrypted.session_id,
                            ds_user_id: decrypted.ds_user_id,
                            csrftoken: decrypted.csrftoken,
                            cookies: decrypted.cookies,
                        };

                        let provider = crate::providers::AuthenticatedSessionProvider::new(Some(config));
                        match crate::sync_coordinator::SyncCoordinator::execute_sync(&db, &account_id, &provider).await {
                            Ok(report) => serde_json::to_string(&report).unwrap_or_else(|_| "{\"success\":true}".to_string()),
                            Err(e) => format!("{{\"success\":false,\"error\":\"Sync error: {}\"}}", e),
                        }
                    }
                    crate::models::ProviderType::PublicProfile => {
                        let provider = crate::providers::PublicProfileProvider::default();
                        match crate::sync_coordinator::SyncCoordinator::execute_sync(&db, &account_id, &provider).await {
                            Ok(report) => serde_json::to_string(&report).unwrap_or_else(|_| "{\"success\":true}".to_string()),
                            Err(e) => format!("{{\"success\":false,\"error\":\"Sync error: {}\"}}", e),
                        }
                    }
                    crate::models::ProviderType::Export => {
                        let provider = crate::providers::ExportArchiveProvider;
                        match crate::sync_coordinator::SyncCoordinator::execute_sync(&db, &account_id, &provider).await {
                            Ok(report) => serde_json::to_string(&report).unwrap_or_else(|_| "{\"success\":true}".to_string()),
                            Err(e) => format!("{{\"success\":false,\"error\":\"Sync error: {}\"}}", e),
                        }
                    }
                    crate::models::ProviderType::Mock => {
                        let fixture = crate::providers::FixtureProvider::new(
                            account.followers_count.max(25) as usize,
                            account.following_count.max(20) as usize,
                            15,
                        );

                        match crate::sync_coordinator::SyncCoordinator::execute_sync(&db, &account_id, &fixture).await {
                            Ok(report) => serde_json::to_string(&report).unwrap_or_else(|_| "{\"success\":true}".to_string()),
                            Err(e) => format!("{{\"success\":false,\"error\":\"Sync error: {}\"}}", e),
                        }
                    }
                }
            })
        }
        Err(e) => format!("{{\"success\":false,\"error\":\"Tokio runtime init error: {}\"}}", e),
    };

    env.new_string(result_json).unwrap_or_else(|_| env.new_string("{}").unwrap()).into_raw()
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauri_dev_StalkrNative_headlessSync(
    env: JNIEnv,
    _class: JClass,
    j_account_id: JString,
    j_db_path: JString,
) -> jstring {
    run_headless_sync(env, j_account_id, j_db_path)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_stalkr_app_StalkrNative_headlessSync(
    env: JNIEnv,
    _class: JClass,
    j_account_id: JString,
    j_db_path: JString,
) -> jstring {
    run_headless_sync(env, j_account_id, j_db_path)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_meet_stalkr_StalkrNative_headlessSync(
    env: JNIEnv,
    _class: JClass,
    j_account_id: JString,
    j_db_path: JString,
) -> jstring {
    run_headless_sync(env, j_account_id, j_db_path)
}

