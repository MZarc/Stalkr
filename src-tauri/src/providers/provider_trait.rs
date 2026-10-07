use serde::{Deserialize, Serialize};
use crate::models::{Account, RawMemberRecord};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyState {
    Public,
    Private,
    Unknown,
}

impl PrivacyState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrivacyState::Public => "public",
            PrivacyState::Private => "private",
            PrivacyState::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "public" => PrivacyState::Public,
            "private" => PrivacyState::Private,
            _ => PrivacyState::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessState {
    Accessible,
    PartiallyAccessible,
    NotAccessible,
    AuthRequired,
    RateLimited,
    ProviderError,
    Unknown,
}

impl AccessState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessState::Accessible => "accessible",
            AccessState::PartiallyAccessible => "partially_accessible",
            AccessState::NotAccessible => "not_accessible",
            AccessState::AuthRequired => "auth_required",
            AccessState::RateLimited => "rate_limited",
            AccessState::ProviderError => "provider_error",
            AccessState::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "accessible" => AccessState::Accessible,
            "partially_accessible" => AccessState::PartiallyAccessible,
            "not_accessible" => AccessState::NotAccessible,
            "auth_required" => AccessState::AuthRequired,
            "rate_limited" => AccessState::RateLimited,
            "provider_error" => AccessState::ProviderError,
            _ => AccessState::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResult {
    pub instagram_user_id: Option<String>,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_private: bool,
    pub is_verified: bool,
    pub followers_count: i64,
    pub following_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAccessCheckResult {
    pub target_user_id: Option<String>,
    pub target_username: String,
    pub avatar_url: Option<String>,
    pub privacy_state: PrivacyState,
    pub access_state: AccessState,
    pub access_reason: String,
    pub is_following: bool,
    pub is_approved_follower: bool,
    pub followers_count: i64,
    pub following_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipFetchResult {
    pub status: String,
    pub target_user_id: String,
    pub privacy_state: PrivacyState,
    pub access_state: AccessState,
    pub item_count: i64,
    pub pagination_complete: bool,
    pub completeness: String,
    pub members: Vec<RawMemberRecord>,
    pub source_hash: String,
    pub provider_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub members: Vec<RawMemberRecord>,
    pub item_count: i64,
    pub is_complete: bool,
    pub source_hash: String,
    pub error_message: Option<String>,
}

pub trait InstagramProvider: Send + Sync {
    fn validate_session(&self) -> impl std::future::Future<Output = Result<ProfileResult, String>> + Send;
    fn get_profile(&self, username: &str) -> impl std::future::Future<Output = Result<ProfileResult, String>> + Send;
    fn check_target_access(&self, target_username: &str) -> impl std::future::Future<Output = Result<TargetAccessCheckResult, String>> + Send;
    fn get_followers(&self, account: &Account) -> impl std::future::Future<Output = Result<RelationshipFetchResult, String>> + Send;
    fn get_following(&self, account: &Account) -> impl std::future::Future<Output = Result<RelationshipFetchResult, String>> + Send;
    fn name(&self) -> &'static str;

    /// Secondary list surface (web GraphQL, Instaloader-style). Best-effort:
    /// `Ok(None)` means "no second source" — the sync proceeds on primary
    /// data alone. Only the live session provider overrides these.
    fn get_followers_secondary(
        &self,
        _account: &Account,
    ) -> impl std::future::Future<Output = Result<Option<Vec<RawMemberRecord>>, String>> + Send {
        async { Ok(None) }
    }
    fn get_following_secondary(
        &self,
        _account: &Account,
    ) -> impl std::future::Future<Output = Result<Option<Vec<RawMemberRecord>>, String>> + Send {
        async { Ok(None) }
    }
}

/// Union two relationship-list surfaces into one de-duplicated list.
/// Identity key: stable numeric Instagram id when present, otherwise
/// case-insensitive username. The secondary surface can only ADD members
/// (recovering accounts the primary surface missed); it can never remove,
/// so a union cannot manufacture false unfollows.
pub fn union_members(
    mut primary: Vec<RawMemberRecord>,
    secondary: Vec<RawMemberRecord>,
) -> Vec<RawMemberRecord> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = primary
        .iter()
        .map(|m| member_key(m))
        .collect();
    for m in secondary {
        if seen.insert(member_key(&m)) {
            primary.push(m);
        }
    }
    primary
}

fn member_key(m: &RawMemberRecord) -> String {
    if let Some(ref id) = m.instagram_user_id {
        let t = id.trim();
        if !t.is_empty() {
            return format!("id:{}", t);
        }
    }
    format!("un:{}", m.username.trim().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: Option<&str>, username: &str) -> RawMemberRecord {
        RawMemberRecord {
            instagram_user_id: id.map(|s| s.to_string()),
            username: username.to_string(),
            display_name: None,
            avatar_url: None,
            is_verified: false,
            is_private: false,
        }
    }

    #[test]
    fn test_union_adds_missing_only() {
        let primary = vec![rec(Some("1"), "alice"), rec(None, "Bob")];
        let secondary = vec![
            rec(Some("1"), "alice"),          // dup by id
            rec(Some("2"), "carol"),          // genuinely missing
            rec(None, "BOB"),                // dup by username, case-insensitive
            rec(None, "dave"),               // missing id-less
        ];
        let out = union_members(primary, secondary);
        let names: Vec<&str> = out.iter().map(|m| m.username.as_str()).collect();
        assert_eq!(names, vec!["alice", "Bob", "carol", "dave"]);
    }

    #[test]
    fn test_union_recycled_username_not_merged() {
        // Same username, different numeric ids = different humans (Instagram
        // recycles usernames). Keyed by id, both survive.
        let primary = vec![rec(Some("100"), "sam")];
        let secondary = vec![rec(Some("200"), "sam")];
        let out = union_members(primary, secondary);
        assert_eq!(out.len(), 2);
    }
}
