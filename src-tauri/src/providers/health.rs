use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthStatus {
    pub provider_name: String,
    pub is_connected: bool,
    pub status_text: String,
    pub last_successful_sync: Option<i64>,
    pub follower_retrieval_ok: bool,
    pub following_retrieval_ok: bool,
    pub pagination_ok: bool,
    pub completeness_check_ok: bool,
    pub provider_version: String,
    pub last_error: Option<String>,
}

impl Default for ProviderHealthStatus {
    fn default() -> Self {
        Self {
            provider_name: "Authenticated Session".to_string(),
            is_connected: true,
            status_text: "Connected & Verified".to_string(),
            last_successful_sync: None,
            follower_retrieval_ok: true,
            following_retrieval_ok: true,
            pagination_ok: true,
            completeness_check_ok: true,
            provider_version: "Session Provider 1.0 (Oct 2026)".to_string(),
            last_error: None,
        }
    }
}
