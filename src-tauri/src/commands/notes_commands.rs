use std::sync::Arc;
use tauri::State;
use crate::db::Database;
use crate::models::{NoteDto, NoteRecord};
use crate::security::NoteEncryptor;

// Seed for Key B domain (In production Android, populated via Android Keystore Key B)
const KEY_B_MASTER_SEED: &str = "stalkr_security_domain_key_b_private_notes";

#[tauri::command]
pub async fn get_note(
    db: State<'_, Arc<Database>>,
    account_id: String,
    person_id: String,
) -> Result<Option<NoteDto>, String> {
    let record = db.get_note_encrypted(&account_id, &person_id)
        .map_err(|e| e.to_string())?;

    if let Some(r) = record {
        let key = NoteEncryptor::derive_key(KEY_B_MASTER_SEED);
        let decrypted = NoteEncryptor::decrypt(&key, &r.content_ciphertext, &r.nonce)
            .unwrap_or_else(|_| "[Encrypted Note - Decryption Unavailable]".to_string());

        Ok(Some(NoteDto {
            id: r.id,
            account_id: r.account_id,
            person_id: r.person_id,
            content: decrypted,
            updated_at: r.updated_at,
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn save_note(
    db: State<'_, Arc<Database>>,
    account_id: String,
    person_id: String,
    content: String,
) -> Result<NoteDto, String> {
    let key = NoteEncryptor::derive_key(KEY_B_MASTER_SEED);
    let (ciphertext, nonce) = NoteEncryptor::encrypt(&key, &content)?;
    let now = chrono::Utc::now().timestamp();
    let note_id = uuid::Uuid::new_v4().to_string();

    let record = NoteRecord {
        id: note_id.clone(),
        account_id: account_id.clone(),
        person_id: person_id.clone(),
        content_ciphertext: ciphertext,
        nonce,
        updated_at: now,
    };

    db.upsert_note_encrypted(&record).map_err(|e| e.to_string())?;

    Ok(NoteDto {
        id: note_id,
        account_id,
        person_id,
        content,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn delete_note(
    db: State<'_, Arc<Database>>,
    account_id: String,
    person_id: String,
) -> Result<(), String> {
    db.delete_note(&account_id, &person_id).map_err(|e| e.to_string())
}
