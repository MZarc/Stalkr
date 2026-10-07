use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteRecord {
    pub id: String,
    pub account_id: String,
    pub person_id: String,
    pub content_ciphertext: String, // Base64 AES-256-GCM
    pub nonce: String,              // Base64 12-byte nonce
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDto {
    pub id: String,
    pub account_id: String,
    pub person_id: String,
    pub content: String,
    pub updated_at: i64,
}
