use crate::models::Account;
use crate::providers::provider_trait::{
    AccessState, InstagramProvider, PrivacyState, ProfileResult, RelationshipFetchResult,
    TargetAccessCheckResult,
};

pub struct PublicProfileProvider {
    client: reqwest::Client,
}

impl Default for PublicProfileProvider {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl InstagramProvider for PublicProfileProvider {
    async fn validate_session(&self) -> Result<ProfileResult, String> {
        Err("Public Profile Inspector does not use authenticated sessions.".to_string())
    }

    async fn get_profile(&self, username: &str) -> Result<ProfileResult, String> {
        let url = format!("https://www.instagram.com/{}/?__a=1&__d=dis", username);
        let resp = self.client.get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .send().await
            .map_err(|e| format!("Public request failed: {}", e))?;

        if !resp.status().is_success() {
            return Ok(ProfileResult {
                instagram_user_id: None,
                username: username.to_string(),
                display_name: username.to_string(),
                avatar_url: None,
                is_private: false,
                is_verified: false,
                followers_count: 0,
                following_count: 0,
            });
        }

        let text = resp.text().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

        if let Some(user) = json.pointer("/graphql/user").or_else(|| json.pointer("/data/user")) {
            return Ok(ProfileResult {
                instagram_user_id: user.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                username: username.to_string(),
                display_name: user.get("full_name").and_then(|v| v.as_str()).unwrap_or(username).to_string(),
                avatar_url: user.get("profile_pic_url_hd").and_then(|v| v.as_str()).map(|s| s.to_string()),
                is_private: user.get("is_private").and_then(|v| v.as_bool()).unwrap_or(false),
                is_verified: user.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false),
                followers_count: user.pointer("/edge_followed_by/count").and_then(|v| v.as_i64()).unwrap_or(0),
                following_count: user.pointer("/edge_follow/count").and_then(|v| v.as_i64()).unwrap_or(0),
            });
        }

        Ok(ProfileResult {
            instagram_user_id: None,
            username: username.to_string(),
            display_name: username.to_string(),
            avatar_url: None,
            is_private: false,
            is_verified: false,
            followers_count: 0,
            following_count: 0,
        })
    }

    async fn check_target_access(&self, target_username: &str) -> Result<TargetAccessCheckResult, String> {
        let profile = self.get_profile(target_username).await?;
        let is_private = profile.is_private;
        let privacy_state = if is_private { PrivacyState::Private } else { PrivacyState::Public };

        if is_private {
            Ok(TargetAccessCheckResult {
                target_user_id: profile.instagram_user_id,
                target_username: target_username.to_string(),
                avatar_url: profile.avatar_url.clone(),
                privacy_state,
                access_state: AccessState::NotAccessible,
                access_reason: "This private account's relationship lists are not accessible without an authorized authenticated session.".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: profile.followers_count,
                following_count: profile.following_count,
            })
        } else {
            Ok(TargetAccessCheckResult {
                target_user_id: profile.instagram_user_id,
                target_username: target_username.to_string(),
                avatar_url: profile.avatar_url.clone(),
                privacy_state,
                access_state: AccessState::PartiallyAccessible,
                access_reason: "Public metadata accessible. Follower lists require an authenticated session.".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: profile.followers_count,
                following_count: profile.following_count,
            })
        }
    }

    async fn get_followers(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
        Ok(RelationshipFetchResult {
            status: "NOT_ACCESSIBLE".to_string(),
            target_user_id: account.id.clone(),
            privacy_state: if account.is_private { PrivacyState::Private } else { PrivacyState::Public },
            access_state: AccessState::NotAccessible,
            item_count: 0,
            pagination_complete: false,
            completeness: "incomplete".to_string(),
            members: Vec::new(),
            source_hash: "".to_string(),
            provider_error: Some("Follower list enumeration requires an authenticated session or an official data export. Unauthenticated public access does not expose relationship identities.".to_string()),
        })
    }

    async fn get_following(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
        Ok(RelationshipFetchResult {
            status: "NOT_ACCESSIBLE".to_string(),
            target_user_id: account.id.clone(),
            privacy_state: if account.is_private { PrivacyState::Private } else { PrivacyState::Public },
            access_state: AccessState::NotAccessible,
            item_count: 0,
            pagination_complete: false,
            completeness: "incomplete".to_string(),
            members: Vec::new(),
            source_hash: "".to_string(),
            provider_error: Some("Following list enumeration requires an authenticated session or an official data export. Unauthenticated public access does not expose relationship identities.".to_string()),
        })
    }

    fn name(&self) -> &'static str {
        "Public Profile Inspector"
    }
}
