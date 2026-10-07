use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::prelude::*;
use sha2::{Digest, Sha256};

pub struct NoteEncryptor;

impl NoteEncryptor {
    pub fn derive_key(seed: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }

    pub fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<(String, String), String> {
        let cipher = Aes256Gcm::new(key.into());
        let mut nonce_bytes = [0u8; 12];
        let random_part = uuid::Uuid::new_v4();
        nonce_bytes.copy_from_slice(&random_part.as_bytes()[..12]);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption error: {:?}", e))?;

        let ciphertext_b64 = BASE64_STANDARD.encode(&ciphertext);
        let nonce_b64 = BASE64_STANDARD.encode(&nonce_bytes);

        Ok((ciphertext_b64, nonce_b64))
    }

    pub fn decrypt(key: &[u8; 32], ciphertext_b64: &str, nonce_b64: &str) -> Result<String, String> {
        let cipher = Aes256Gcm::new(key.into());
        let ciphertext = BASE64_STANDARD
            .decode(ciphertext_b64)
            .map_err(|e| format!("Base64 ciphertext decode error: {:?}", e))?;
        let nonce_bytes = BASE64_STANDARD
            .decode(nonce_b64)
            .map_err(|e| format!("Base64 nonce decode error: {:?}", e))?;

        if nonce_bytes.len() != 12 {
            return Err("Invalid nonce length".to_string());
        }

        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| format!("Decryption error: {:?}", e))?;

        String::from_utf8(plaintext_bytes).map_err(|e| format!("UTF-8 decode error: {:?}", e))
    }
}

pub struct SessionEncryptor;

impl SessionEncryptor {
    pub fn get_key_a() -> [u8; 32] {
        NoteEncryptor::derive_key("stalkr_keystore_domain_a_session_v1")
    }

    pub fn encrypt_session(session: &crate::models::DecryptedSessionData) -> Result<(String, String), String> {
        let json = serde_json::to_string(session).map_err(|e| e.to_string())?;
        NoteEncryptor::encrypt(&Self::get_key_a(), &json)
    }

    pub fn decrypt_session(ciphertext_b64: &str, nonce_b64: &str) -> Result<crate::models::DecryptedSessionData, String> {
        let json = NoteEncryptor::decrypt(&Self::get_key_a(), ciphertext_b64, nonce_b64)?;
        serde_json::from_str(&json).map_err(|e| format!("Invalid session data: {}", e))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_encryption_roundtrip() {
        let key = NoteEncryptor::derive_key("stalkr_test_master_key_b");
        let note = "Met Alex at Tokyo design conference. Likes typography and horology.";
        let (cipher_b64, nonce_b64) = NoteEncryptor::encrypt(&key, note).unwrap();
        assert_ne!(note, &cipher_b64);
        let decrypted = NoteEncryptor::decrypt(&key, &cipher_b64, &nonce_b64).unwrap();
        assert_eq!(note, decrypted);
    }

    #[test]
    fn test_session_encryption_roundtrip() {
        let session = crate::models::DecryptedSessionData {
            session_id: "test_session_id_xyz123".to_string(),
            ds_user_id: "123456789".to_string(),
            csrftoken: Some("csrf_token_abc".to_string()),
            cookies: None,
        };

        let (cipher_b64, nonce_b64) = SessionEncryptor::encrypt_session(&session).unwrap();
        assert_ne!(cipher_b64, session.session_id);

        let decrypted = SessionEncryptor::decrypt_session(&cipher_b64, &nonce_b64).unwrap();
        assert_eq!(decrypted.session_id, session.session_id);
        assert_eq!(decrypted.ds_user_id, session.ds_user_id);
        assert_eq!(decrypted.csrftoken, session.csrftoken);
    }
}

