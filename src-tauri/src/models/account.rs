use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Owner,
    Monitored,
}

impl AccountKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountKind::Owner => "owner",
            AccountKind::Monitored => "monitored",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "owner" => AccountKind::Owner,
            _ => AccountKind::Monitored,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Session,
    Export,
    PublicProfile,
    Mock,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::Session => "session",
            ProviderType::Export => "export",
            ProviderType::PublicProfile => "public_profile",
            ProviderType::Mock => "mock",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "session" => ProviderType::Session,
            "export" => ProviderType::Export,
            "public_profile" => ProviderType::PublicProfile,
            _ => ProviderType::Mock,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub instagram_user_id: Option<String>,
    pub username: String,
    pub display_name: String,
    pub account_kind: AccountKind,
    pub provider_type: ProviderType,
    pub avatar_url: Option<String>,
    pub is_private: bool,
    pub is_verified: bool,
    pub followers_count: i64,
    pub following_count: i64,
    pub monitoring_enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_successful_sync_at: Option<i64>,
    pub last_attempted_sync_at: Option<i64>,

    // Access context fields
    pub authenticated_by_account_id: Option<String>,
    pub access_state: String, // "accessible", "partially_accessible", "not_accessible", "auth_required", "rate_limited", "provider_error", "unknown"
    pub access_reason: Option<String>,
    pub target_privacy: String, // "public", "private", "unknown"
    pub last_access_checked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSession {
    pub account_id: String,
    pub session_data_ciphertext: String,
    pub nonce: String,
    pub last_validated_at: i64,
    pub is_healthy: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptedSessionData {
    pub session_id: String,
    pub ds_user_id: String,
    pub csrftoken: Option<String>,
    #[serde(default)]
    pub cookies: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountAlias {
    pub account_id: String,
    pub username: String,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
}
