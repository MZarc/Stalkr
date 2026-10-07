use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT};
use sha2::{Digest, Sha256};
use crate::models::{Account, RawMemberRecord};
use crate::providers::provider_trait::{
    AccessState, InstagramProvider, PrivacyState, ProfileResult, RelationshipFetchResult,
    TargetAccessCheckResult,
};

const DEFAULT_APP_ID: &str = "936619743392459";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36";

#[derive(Clone)]
pub struct AuthenticatedSessionConfig {
    pub session_id: String,
    pub ds_user_id: String,
    pub csrftoken: Option<String>,
    pub cookies: Option<String>,
}

pub struct AuthenticatedSessionProvider {
    client: reqwest::Client,
    config: Option<AuthenticatedSessionConfig>,
}

impl AuthenticatedSessionProvider {
    pub fn new(config: Option<AuthenticatedSessionConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();

        Self { client, config }
    }

    fn build_headers(&self) -> Result<HeaderMap, String> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
        headers.insert("X-IG-App-ID", HeaderValue::from_static(DEFAULT_APP_ID));
        headers.insert("X-Requested-With", HeaderValue::from_static("XMLHttpRequest"));
        headers.insert("Accept", HeaderValue::from_static("*/*"));
        headers.insert("Accept-Language", HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert("Referer", HeaderValue::from_static("https://www.instagram.com/"));
        headers.insert("Origin", HeaderValue::from_static("https://www.instagram.com"));

        if let Some(ref cfg) = self.config {
            if let Some(ref c) = cfg.csrftoken {
                if let Ok(val) = HeaderValue::from_str(c) {
                    headers.insert("X-CSRFToken", val);
                }
            }

            let cookie_str = if let Some(ref raw_cookies) = cfg.cookies {
                if !raw_cookies.trim().is_empty() {
                    raw_cookies.clone()
                } else {
                    format!(
                        "sessionid={}; ds_user_id={}; {}",
                        cfg.session_id,
                        cfg.ds_user_id,
                        cfg.csrftoken.as_deref().map(|c| format!("csrftoken={};", c)).unwrap_or_default()
                    )
                }
            } else {
                format!(
                    "sessionid={}; ds_user_id={}; {}",
                    cfg.session_id,
                    cfg.ds_user_id,
                    cfg.csrftoken.as_deref().map(|c| format!("csrftoken={};", c)).unwrap_or_default()
                )
            };

            let cookie_header = HeaderValue::from_str(&cookie_str)
                .map_err(|e| format!("Invalid cookie header: {:?}", e))?;
            headers.insert(COOKIE, cookie_header);
        }

        Ok(headers)
    }

    async fn fetch_paged_list(
        &self,
        endpoint_type: &str, // "followers" or "following"
        user_id: &str,
        is_private: bool,
    ) -> Result<RelationshipFetchResult, String> {
        let privacy_state = if is_private { PrivacyState::Private } else { PrivacyState::Public };

        if self.config.is_none() {
            return Ok(RelationshipFetchResult {
                status: "AUTH_REQUIRED".to_string(),
                target_user_id: user_id.to_string(),
                privacy_state,
                access_state: AccessState::AuthRequired,
                item_count: 0,
                pagination_complete: false,
                completeness: "incomplete".to_string(),
                members: Vec::new(),
                source_hash: "".to_string(),
                provider_error: Some("INSTAGRAM SESSION EXPIRED: Saved Instagram session is no longer accepted. Reconnect required.".to_string()),
            });
        }

        let headers = self.build_headers()?;
        let mut all_members = Vec::new();
        let mut max_id: Option<String> = None;
        let mut page_count = 0;
        let max_pages = 200; // Safety cap
        let mut hasher = Sha256::new();
        // Consecutive 429s — after several, stop entirely to protect the session.
        let mut rate_limit_streak = 0u32;

        loop {
            page_count += 1;
            if page_count > max_pages {
                let source_hash = format!("{:x}", hasher.finalize());
                let count = all_members.len() as i64;
                return Ok(RelationshipFetchResult {
                    status: "PARTIALLY_ACCESSIBLE".to_string(),
                    target_user_id: user_id.to_string(),
                    privacy_state,
                    access_state: AccessState::PartiallyAccessible,
                    item_count: count,
                    pagination_complete: false,
                    completeness: "partial".to_string(),
                    members: all_members,
                    source_hash,
                    provider_error: Some("Pagination reached safety ceiling (200 pages). Marked partial to preserve data integrity.".to_string()),
                });
            }

            let mut url = format!(
                "https://i.instagram.com/api/v1/friendships/{}/{}/?count=200",
                user_id, endpoint_type
            );
            if let Some(ref cursor) = max_id {
                url.push_str(&format!("&max_id={}", cursor));
            }

            let resp = self.client.get(&url).headers(headers.clone()).send().await
                .map_err(|e| format!("Network request failed: {}", e))?;

            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // Human-paced exponential backoff: 4s, then 9s. After that we
                // stop completely — hammering through a rate limit is how
                // sessions get flagged. Partial data is quarantined upstream.
                rate_limit_streak += 1;
                if rate_limit_streak <= 2 {
                    let backoff = if rate_limit_streak == 1 { 4 } else { 9 };
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    page_count -= 1; // don't count the throttled attempt
                    continue;
                }
                let source_hash = format!("{:x}", hasher.finalize());
                return Ok(RelationshipFetchResult {
                    status: "RATE_LIMITED".to_string(),
                    target_user_id: user_id.to_string(),
                    privacy_state,
                    access_state: AccessState::RateLimited,
                    item_count: all_members.len() as i64,
                    pagination_complete: false,
                    completeness: "incomplete".to_string(),
                    members: all_members,
                    source_hash,
                    provider_error: Some("Instagram returned 429 Too Many Requests repeatedly. Sync stopped safely to protect your session — try again in 15–30 minutes.".to_string()),
                });
            }
            // Any successful page resets the streak.
            rate_limit_streak = 0;

            if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
                let source_hash = format!("{:x}", hasher.finalize());
                return Ok(RelationshipFetchResult {
                    status: "AUTH_REQUIRED".to_string(),
                    target_user_id: user_id.to_string(),
                    privacy_state,
                    access_state: AccessState::AuthRequired,
                    item_count: all_members.len() as i64,
                    pagination_complete: false,
                    completeness: "incomplete".to_string(),
                    members: all_members,
                    source_hash,
                    provider_error: Some("INSTAGRAM SESSION EXPIRED: Saved Instagram session is no longer accepted. Reconnect required.".to_string()),
                });
            }

            if !status.is_success() {
                let source_hash = format!("{:x}", hasher.finalize());
                return Ok(RelationshipFetchResult {
                    status: "PROVIDER_ERROR".to_string(),
                    target_user_id: user_id.to_string(),
                    privacy_state,
                    access_state: AccessState::ProviderError,
                    item_count: all_members.len() as i64,
                    pagination_complete: false,
                    completeness: "incomplete".to_string(),
                    members: all_members,
                    source_hash,
                    provider_error: Some(format!("Instagram returned HTTP {}. Sync quarantined.", status)),
                });
            }

            let body_text = resp.text().await
                .map_err(|e| format!("Failed to read response body: {}", e))?;
            hasher.update(body_text.as_bytes());

            let json: serde_json::Value = serde_json::from_str(&body_text)
                .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

            // Check response status field
            if let Some(status_field) = json.get("status").and_then(|s| s.as_str()) {
                if status_field != "ok" {
                    let msg = json.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown provider status");
                    let source_hash = format!("{:x}", hasher.finalize());
                    return Ok(RelationshipFetchResult {
                        status: "PROVIDER_ERROR".to_string(),
                        target_user_id: user_id.to_string(),
                        privacy_state,
                        access_state: AccessState::ProviderError,
                        item_count: all_members.len() as i64,
                        pagination_complete: false,
                        completeness: "incomplete".to_string(),
                        members: all_members,
                        source_hash,
                        provider_error: Some(format!("Instagram response message: {}", msg)),
                    });
                }
            }

            // Extract users
            if let Some(users) = json.get("users").and_then(|u| u.as_array()) {
                for u in users {
                    let username = u.get("username").and_then(|v| v.as_str()).unwrap_or_default();
                    if username.is_empty() {
                        continue;
                    }

                    let user_id_str = u.get("pk").and_then(|pk| {
                        if pk.is_string() {
                            pk.as_str().map(|s| s.to_string())
                        } else if pk.is_number() {
                            Some(pk.to_string())
                        } else {
                            None
                        }
                    });

                    all_members.push(RawMemberRecord {
                        instagram_user_id: user_id_str,
                        username: username.to_string(),
                        display_name: u.get("full_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        avatar_url: u.get("profile_pic_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        is_verified: u.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false),
                        is_private: u.get("is_private").and_then(|v| v.as_bool()).unwrap_or(false),
                    });
                }
            }

            // Next page cursor check
            let next_cursor = json.get("next_max_id")
                .or_else(|| json.get("cursor"))
                .or_else(|| json.get("page_token"))
                .and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else if v.is_number() {
                        Some(v.to_string())
                    } else {
                        None
                    }
                });

            if let Some(cur) = next_cursor {
                if !cur.is_empty() && Some(&cur) != max_id.as_ref() {
                    max_id = Some(cur);
                    // Human-paced inter-page delay with jitter (~0.9–1.5s).
                    // Fixed machine-like intervals are exactly what Instagram's
                    // anti-automation heuristics flag; jitter keeps us safe.
                    // No extra dependency: nanos-based pseudo-jitter is enough.
                    let jitter = (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| (d.subsec_nanos() % 600) as u64)
                        .unwrap_or(300))
                    .max(100);
                    tokio::time::sleep(Duration::from_millis(900 + jitter)).await;
                    continue;
                }
            }

            // End of pagination reached cleanly
            break;
        }

        let source_hash = format!("{:x}", hasher.finalize());
        let count = all_members.len() as i64;

        Ok(RelationshipFetchResult {
            status: "ACCESSIBLE".to_string(),
            target_user_id: user_id.to_string(),
            privacy_state,
            access_state: AccessState::Accessible,
            item_count: count,
            pagination_complete: true,
            completeness: "complete".to_string(),
            members: all_members,
            source_hash,
            provider_error: None,
        })
    }

    /// Secondary list surface: Instagram's web GraphQL follower/following
    /// edges (the technique popularised by Instaloader). Different serving
    /// stack from the mobile friendships endpoint, so it can surface
    /// accounts the primary pagination missed.
    ///
    /// Strictly best-effort and strictly safe:
    /// - capped pages + the same human-paced jittered delays as primary;
    /// - ANY transport/API anomaly (429, 400 "fail", deprecated query_hash,
    ///   schema drift) returns `Ok(None)` — secondary absence never fails
    ///   or blocks a sync, the primary result simply stands alone.
    async fn fetch_graphql_list(
        &self,
        edge: &str, // "edge_followed_by" (followers) or "edge_follow" (following)
        query_hash: &str,
        user_id: &str,
    ) -> Result<Option<Vec<RawMemberRecord>>, String> {
        const MAX_PAGES: usize = 30; // 30 × 50 = up to 1500 extra chances
        let headers = self.build_headers()?;
        let mut members = Vec::new();
        let mut after: Option<String> = None;

        for _ in 0..MAX_PAGES {
            let variables = match &after {
                Some(cur) => format!(
                    "{{\"id\":\"{}\",\"include_reel\":false,\"fetch_mutual\":false,\"first\":50,\"after\":\"{}\"}}",
                    user_id, cur
                ),
                None => format!(
                    "{{\"id\":\"{}\",\"include_reel\":false,\"fetch_mutual\":false,\"first\":50}}",
                    user_id
                ),
            };
            // Query params must be URL-encoded; variables contain JSON.
            let mut url = format!(
                "https://www.instagram.com/graphql/query/?query_hash={}&variables=",
                query_hash
            );
            for b in variables.bytes() {
                if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                    url.push(b as char);
                } else {
                    url.push_str(&format!("%{:02X}", b));
                }
            }

            let resp = self
                .client
                .get(&url)
                .headers(headers.clone())
                .send()
                .await
                .map_err(|_| "graphql transport".to_string())?;
            if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Ok(None); // throttled: stop quietly, keep primary data
            }
            if !resp.status().is_success() {
                return Ok(None); // e.g. 400 on deprecated hash: ignore surface
            }
            let text = resp.text().await.map_err(|_| "graphql body".to_string())?;
            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(|_| "graphql json".to_string())?;
            if json.get("status").and_then(|s| s.as_str()) == Some("fail") {
                return Ok(None);
            }
            let edge_obj = json
                .pointer(&format!("/data/user/{}", edge))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if edge_obj.is_null() {
                return Ok(None); // schema drift: ignore surface
            }

            if let Some(arr) = edge_obj.get("edges").and_then(|e| e.as_array()) {
                for item in arr {
                    if let Some(node) = item.get("node") {
                        let username = node
                            .get("username")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .trim();
                        if username.is_empty() {
                            continue;
                        }
                        let id = node
                            .get("id")
                            .and_then(|v| {
                                if v.is_string() {
                                    v.as_str().map(|s| s.to_string())
                                } else if v.is_number() {
                                    Some(v.to_string())
                                } else {
                                    None
                                }
                            })
                            .filter(|s| !s.trim().is_empty());
                        members.push(RawMemberRecord {
                            instagram_user_id: id,
                            username: username.to_string(),
                            display_name: node
                                .get("full_name")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            avatar_url: node
                                .get("profile_pic_url")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            is_verified: node
                                .get("is_verified")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            is_private: node
                                .get("is_private")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        });
                    }
                }
            }

            let has_next = edge_obj
                .pointer("/page_info/has_next_page")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let end_cursor = edge_obj
                .pointer("/page_info/end_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            match (has_next, end_cursor) {
                (true, Some(cur)) if !cur.is_empty() && Some(&cur) != after.as_ref() => {
                    after = Some(cur);
                    let jitter = (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| (d.subsec_nanos() % 600) as u64)
                        .unwrap_or(300))
                    .max(100);
                    tokio::time::sleep(Duration::from_millis(900 + jitter)).await;
                }
                _ => break,
            }
        }

        Ok(Some(members))
    }

    /// Direct relationship probe by numeric Instagram user id — no profile
    /// lookup required. Used as the fallback when the username-based access
    /// check comes back Unknown (profile endpoints are the flakiest surface;
    /// the friendships endpoints usually still answer, as sync proves).
    /// Never fails the caller: every outcome maps to an explicit state.
    pub async fn probe_target_by_id(
        &self,
        target_user_id: &str,
        target_username: &str,
    ) -> Result<TargetAccessCheckResult, String> {
        let headers = self.build_headers()?;
        let unknown_privacy = PrivacyState::Unknown;

        // Step 1: friendship status between the session and the target.
        let show_url = format!(
            "https://i.instagram.com/api/v1/friendships/show/{}/",
            target_user_id
        );
        let show_resp = self
            .client
            .get(&show_url)
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| format!("Probe friendship query failed: {}", e))?;

        match show_resp.status() {
            s if s == reqwest::StatusCode::UNAUTHORIZED || s == reqwest::StatusCode::FORBIDDEN => {
                return Ok(TargetAccessCheckResult {
                    target_user_id: Some(target_user_id.to_string()),
                    target_username: target_username.to_string(),
                    avatar_url: None,
                    privacy_state: unknown_privacy,
                    access_state: AccessState::AuthRequired,
                    access_reason: "INSTAGRAM SESSION EXPIRED: Saved Instagram session is no longer accepted.".to_string(),
                    is_following: false,
                    is_approved_follower: false,
                    followers_count: 0,
                    following_count: 0,
                });
            }
            s if s == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                return Ok(TargetAccessCheckResult {
                    target_user_id: Some(target_user_id.to_string()),
                    target_username: target_username.to_string(),
                    avatar_url: None,
                    privacy_state: unknown_privacy,
                    access_state: AccessState::RateLimited,
                    access_reason: "Instagram returned HTTP 429 Too Many Requests during probe.".to_string(),
                    is_following: false,
                    is_approved_follower: false,
                    followers_count: 0,
                    following_count: 0,
                });
            }
            _ => {}
        }

        let is_following = if show_resp.status().is_success() {
            let body = show_resp.text().await.unwrap_or_default();
            serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|j| j.get("following").and_then(|v| v.as_bool()))
                .unwrap_or(false)
        } else {
            return Ok(TargetAccessCheckResult {
                target_user_id: Some(target_user_id.to_string()),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: unknown_privacy,
                access_state: AccessState::ProviderError,
                access_reason: "Instagram did not recognise this profile id.".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: 0,
                following_count: 0,
            });
        };

        // Step 2: one-row list probe — the decisive test.
        let probe_url = format!(
            "https://i.instagram.com/api/v1/friendships/{}/followers/?count=1",
            target_user_id
        );
        let probe_resp = self
            .client
            .get(&probe_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Probe request failed: {}", e))?;

        if probe_resp.status().is_success() {
            Ok(TargetAccessCheckResult {
                target_user_id: Some(target_user_id.to_string()),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: unknown_privacy,
                access_state: AccessState::Accessible,
                access_reason: "Relationship list reachable through the connected session (verified by direct probe).".to_string(),
                is_following,
                is_approved_follower: is_following,
                followers_count: 0,
                following_count: 0,
            })
        } else if probe_resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Ok(TargetAccessCheckResult {
                target_user_id: Some(target_user_id.to_string()),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: unknown_privacy,
                access_state: AccessState::RateLimited,
                access_reason: "Rate limited by Instagram during probe.".to_string(),
                is_following,
                is_approved_follower: is_following,
                followers_count: 0,
                following_count: 0,
            })
        } else if probe_resp.status() == reqwest::StatusCode::UNAUTHORIZED
            || probe_resp.status() == reqwest::StatusCode::FORBIDDEN
        {
            Ok(TargetAccessCheckResult {
                target_user_id: Some(target_user_id.to_string()),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: unknown_privacy,
                access_state: AccessState::NotAccessible,
                access_reason: "This account's relationship lists are not accessible through the connected Instagram account.".to_string(),
                is_following,
                is_approved_follower: is_following,
                followers_count: 0,
                following_count: 0,
            })
        } else {
            Ok(TargetAccessCheckResult {
                target_user_id: Some(target_user_id.to_string()),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: unknown_privacy,
                access_state: AccessState::ProviderError,
                access_reason: format!(
                    "Instagram returned HTTP {} for relationship probe.",
                    probe_resp.status()
                ),
                is_following,
                is_approved_follower: is_following,
                followers_count: 0,
                following_count: 0,
            })
        }
    }

    async fn resolve_numeric_id(&self, account: &Account) -> Result<String, String> {
        if let Some(ref id) = account.instagram_user_id {
            let trimmed = id.trim();
            if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
                return Ok(trimmed.to_string());
            }
        }

        // Self-heal by fetching profile
        let profile = self.get_profile(&account.username).await?;
        if let Some(id) = profile.instagram_user_id {
            let trimmed = id.trim().to_string();
            if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
                return Ok(trimmed);
            }
        }

        // NOTE: we deliberately do NOT fall back to `account.id` here.
        // `account.id` is a local UUID; using it as an Instagram user id would
        // silently fetch the WRONG person's relationship list and generate
        // mass false unfollows. Failing loudly is the safe behaviour.
        Err(format!(
            "Cannot sync relationships for @{}: numeric Instagram user ID could not be resolved. Re-check the target (username may have changed) and try again.",
            account.username
        ))
    }
}

impl InstagramProvider for AuthenticatedSessionProvider {
    async fn validate_session(&self) -> Result<ProfileResult, String> {
        let cfg = self.config.as_ref().ok_or_else(|| "No session configuration provided".to_string())?;
        let headers = self.build_headers()?;

        let urls = [
            format!("https://www.instagram.com/api/v1/users/{}/info/", cfg.ds_user_id),
            format!("https://i.instagram.com/api/v1/users/{}/info/", cfg.ds_user_id),
        ];

        let mut last_error = "Validation failed".to_string();

        for url in &urls {
            let resp_res = self.client.get(url).headers(headers.clone()).send().await;
            let resp = match resp_res {
                Ok(r) => r,
                Err(e) => {
                    last_error = format!("Network request failed: {}", e);
                    continue;
                }
            };

            let status = resp.status();
            if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
                last_error = "INSTAGRAM SESSION EXPIRED: Saved Instagram session is no longer accepted. Please reconnect.".to_string();
                continue;
            }
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_error = "RATE_LIMITED: Instagram temporarily rejected the request (HTTP 429).".to_string();
                continue;
            }
            if !status.is_success() {
                last_error = format!("Instagram returned HTTP {}. Validation failed.", status);
                continue;
            }

            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    last_error = format!("Failed to read body: {}", e);
                    continue;
                }
            };
            let json: serde_json::Value = match serde_json::from_str(&text) {
                Ok(j) => j,
                Err(e) => {
                    last_error = format!("Invalid JSON response: {}", e);
                    continue;
                }
            };

            if let Some(user) = json.get("user") {
                let pk = user.get("pk").and_then(|p| {
                    if p.is_string() {
                        p.as_str().map(|s| s.to_string())
                    } else if p.is_number() {
                        Some(p.to_string())
                    } else {
                        None
                    }
                }).unwrap_or_else(|| cfg.ds_user_id.clone());

                let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("user").to_string();
                let full_name = user.get("full_name").and_then(|v| v.as_str()).unwrap_or(&username).to_string();
                let avatar = user.get("profile_pic_url").and_then(|v| v.as_str()).map(|s| s.to_string());
                let is_private = user.get("is_private").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_verified = user.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false);
                let followers = user.get("follower_count").and_then(|v| v.as_i64()).unwrap_or(0);
                let following = user.get("following_count").and_then(|v| v.as_i64()).unwrap_or(0);

                return Ok(ProfileResult {
                    instagram_user_id: Some(pk),
                    username,
                    display_name: full_name,
                    avatar_url: avatar,
                    is_private,
                    is_verified,
                    followers_count: followers,
                    following_count: following,
                });
            }
        }

        Err(last_error)
    }

    async fn get_profile(&self, username: &str) -> Result<ProfileResult, String> {
        let headers = self.build_headers()?;

        // Primary Tier: Instagram Mobile Private API (matches DEFAULT_USER_AGENT and X-IG-App-ID)
        let mobile_url = format!("https://i.instagram.com/api/v1/users/{}/usernameinfo/", username);
        if let Ok(resp) = self.client.get(&mobile_url).headers(headers.clone()).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(user) = json.get("user") {
                            let pk = user.get("pk").and_then(|p| {
                                if p.is_string() {
                                    p.as_str().map(|s| s.to_string())
                                } else if p.is_number() {
                                    Some(p.to_string())
                                } else {
                                    None
                                }
                            });

                            let u_name = user.get("username").and_then(|v| v.as_str()).unwrap_or(username).to_string();
                            let full_name = user.get("full_name").and_then(|v| v.as_str()).unwrap_or(&u_name).to_string();
                            let avatar = user.get("profile_pic_url_hd")
                                .or_else(|| user.get("profile_pic_url"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let is_private = user.get("is_private").and_then(|v| v.as_bool()).unwrap_or(false);
                            let is_verified = user.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false);
                            let followers = user.get("follower_count").and_then(|v| v.as_i64()).unwrap_or(0);
                            let following = user.get("following_count").and_then(|v| v.as_i64()).unwrap_or(0);

                            return Ok(ProfileResult {
                                instagram_user_id: pk,
                                username: u_name,
                                display_name: full_name,
                                avatar_url: avatar,
                                is_private,
                                is_verified,
                                followers_count: followers,
                                following_count: following,
                            });
                        }
                    }
                }
            }
        }

        // Secondary Tier: Web Profile Info endpoint with explicit Referer
        let mut web_headers = headers.clone();
        if let Ok(ref_val) = HeaderValue::from_str(&format!("https://www.instagram.com/{}/", username)) {
            web_headers.insert("Referer", ref_val);
        }
        let web_url = format!("https://www.instagram.com/api/v1/users/web_profile_info/?username={}", username);

        if let Ok(resp) = self.client.get(&web_url).headers(web_headers.clone()).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(user) = json.pointer("/data/user") {
                            let id = user.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let full_name = user.get("full_name").and_then(|v| v.as_str()).unwrap_or(username).to_string();
                            let avatar = user.get("profile_pic_url_hd").or_else(|| user.get("profile_pic_url")).and_then(|v| v.as_str()).map(|s| s.to_string());
                            let is_private = user.get("is_private").and_then(|v| v.as_bool()).unwrap_or(false);
                            let is_verified = user.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false);
                            let followers = user.pointer("/edge_followed_by/count").and_then(|v| v.as_i64()).unwrap_or(0);
                            let following = user.pointer("/edge_follow/count").and_then(|v| v.as_i64()).unwrap_or(0);

                            return Ok(ProfileResult {
                                instagram_user_id: id,
                                username: username.to_string(),
                                display_name: full_name,
                                avatar_url: avatar,
                                is_private,
                                is_verified,
                                followers_count: followers,
                                following_count: following,
                            });
                        }
                    }
                }
            }
        }

        // Tertiary Tier: Query user timeline/HTML fallback for user ID
        let page_url = format!("https://www.instagram.com/{}/?__a=1&__d=dis", username);
        if let Ok(resp) = self.client.get(&page_url).headers(web_headers).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(user) = json.pointer("/graphql/user").or_else(|| json.get("user")) {
                            let id = user.get("id").or_else(|| user.get("pk")).and_then(|v| {
                                if v.is_string() { v.as_str().map(|s| s.to_string()) }
                                else if v.is_number() { Some(v.to_string()) }
                                else { None }
                            });
                            let full_name = user.get("full_name").and_then(|v| v.as_str()).unwrap_or(username).to_string();
                            let avatar = user.get("profile_pic_url_hd").or_else(|| user.get("profile_pic_url")).and_then(|v| v.as_str()).map(|s| s.to_string());
                            let is_private = user.get("is_private").and_then(|v| v.as_bool()).unwrap_or(false);
                            let is_verified = user.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false);

                            return Ok(ProfileResult {
                                instagram_user_id: id,
                                username: username.to_string(),
                                display_name: full_name,
                                avatar_url: avatar,
                                is_private,
                                is_verified,
                                followers_count: 0,
                                following_count: 0,
                            });
                        }
                    }
                }
            }
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
        // Step 1: Resolve target profile & privacy
        let target_profile = self.get_profile(target_username).await?;
        let target_id = target_profile.instagram_user_id.clone().unwrap_or_default();
        let is_private = target_profile.is_private;
        let privacy_state = if is_private { PrivacyState::Private } else { PrivacyState::Public };

        if target_id.is_empty() {
            return Ok(TargetAccessCheckResult {
                target_user_id: None,
                target_username: target_username.to_string(),
                avatar_url: target_profile.avatar_url.clone(),
                privacy_state: PrivacyState::Unknown,
                access_state: AccessState::Unknown,
                access_reason: "Target Instagram user ID could not be resolved.".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: target_profile.followers_count,
                following_count: target_profile.following_count,
            });
        }

        // Step 2: Query friendship status between authenticated session and target
        let headers = self.build_headers()?;
        let friendship_url = format!("https://i.instagram.com/api/v1/friendships/show/{}/", target_id);
        let resp = self.client.get(&friendship_url).headers(headers.clone()).send().await
            .map_err(|e| format!("Friendship query failed: {}", e))?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Ok(TargetAccessCheckResult {
                target_user_id: Some(target_id),
                target_username: target_username.to_string(),
                avatar_url: target_profile.avatar_url.clone(),
                privacy_state,
                access_state: AccessState::AuthRequired,
                access_reason: "INSTAGRAM SESSION EXPIRED: Saved Instagram session is no longer accepted.".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: target_profile.followers_count,
                following_count: target_profile.following_count,
            });
        }

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Ok(TargetAccessCheckResult {
                target_user_id: Some(target_id),
                target_username: target_username.to_string(),
                avatar_url: target_profile.avatar_url.clone(),
                privacy_state,
                access_state: AccessState::RateLimited,
                access_reason: "Instagram returned HTTP 429 Too Many Requests.".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: target_profile.followers_count,
                following_count: target_profile.following_count,
            });
        }

        let is_following = if resp.status().is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let json: serde_json::Value = serde_json::from_str(&body_text).unwrap_or_default();
            json.get("following").and_then(|v| v.as_bool()).unwrap_or(false)
        } else {
            false
        };

        // Decisive check for private targets:
        if is_private {
            if !is_following {
                // Rule 8: Never attempt to bypass privacy controls
                return Ok(TargetAccessCheckResult {
                    target_user_id: Some(target_id),
                    target_username: target_username.to_string(),
                    avatar_url: target_profile.avatar_url.clone(),
                    privacy_state: PrivacyState::Private,
                    access_state: AccessState::NotAccessible,
                    access_reason: "This private account's relationship lists are not accessible through the connected Instagram account (not an approved follower).".to_string(),
                    is_following: false,
                    is_approved_follower: false,
                    followers_count: target_profile.followers_count,
                    following_count: target_profile.following_count,
                });
            }

            // Target is private and authenticated account IS an approved follower!
            // Decisive test: probe relationship list retrieval
            let probe_url = format!("https://i.instagram.com/api/v1/friendships/{}/followers/?count=1", target_id);
            let probe_resp = self.client.get(&probe_url).headers(headers).send().await
                .map_err(|e| format!("Probe request failed: {}", e))?;

            if probe_resp.status().is_success() {
                Ok(TargetAccessCheckResult {
                    target_user_id: Some(target_id),
                    target_username: target_username.to_string(),
                    avatar_url: target_profile.avatar_url.clone(),
                    privacy_state: PrivacyState::Private,
                    access_state: AccessState::Accessible,
                    access_reason: "Private account accessible through authenticated approved follower.".to_string(),
                    is_following: true,
                    is_approved_follower: true,
                    followers_count: target_profile.followers_count,
                    following_count: target_profile.following_count,
                })
            } else if probe_resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                Ok(TargetAccessCheckResult {
                    target_user_id: Some(target_id),
                    target_username: target_username.to_string(),
                    avatar_url: target_profile.avatar_url.clone(),
                    privacy_state: PrivacyState::Private,
                    access_state: AccessState::RateLimited,
                    access_reason: "Rate limited by Instagram during probe.".to_string(),
                    is_following: true,
                    is_approved_follower: true,
                    followers_count: target_profile.followers_count,
                    following_count: target_profile.following_count,
                })
            } else {
                Ok(TargetAccessCheckResult {
                    target_user_id: Some(target_id),
                    target_username: target_username.to_string(),
                    avatar_url: target_profile.avatar_url.clone(),
                    privacy_state: PrivacyState::Private,
                    access_state: AccessState::ProviderError,
                    access_reason: format!("Instagram returned HTTP {} for relationship probe.", probe_resp.status()),
                    is_following: true,
                    is_approved_follower: true,
                    followers_count: target_profile.followers_count,
                    following_count: target_profile.following_count,
                })
            }
        } else {
            // Public target account
            Ok(TargetAccessCheckResult {
                target_user_id: Some(target_id),
                target_username: target_username.to_string(),
                avatar_url: target_profile.avatar_url.clone(),
                privacy_state: PrivacyState::Public,
                access_state: AccessState::Accessible,
                access_reason: "Public account.".to_string(),
                is_following,
                is_approved_follower: is_following,
                followers_count: target_profile.followers_count,
                following_count: target_profile.following_count,
            })
        }
    }

    async fn get_followers(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
        let target_id = self.resolve_numeric_id(account).await?;
        self.fetch_paged_list("followers", &target_id, account.is_private).await
    }

    async fn get_following(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
        let target_id = self.resolve_numeric_id(account).await?;
        self.fetch_paged_list("following", &target_id, account.is_private).await
    }

    async fn get_followers_secondary(
        &self,
        account: &Account,
    ) -> Result<Option<Vec<RawMemberRecord>>, String> {
        let target_id = match self.resolve_numeric_id(account).await {
            Ok(id) => id,
            Err(_) => return Ok(None),
        };
        // Classic Instaloader followers edge hash.
        self.fetch_graphql_list("edge_followed_by", "c76146de99bb02f6415203be841dd25a", &target_id)
            .await
    }

    async fn get_following_secondary(
        &self,
        account: &Account,
    ) -> Result<Option<Vec<RawMemberRecord>>, String> {
        let target_id = match self.resolve_numeric_id(account).await {
            Ok(id) => id,
            Err(_) => return Ok(None),
        };
        // Classic Instaloader following edge hash.
        self.fetch_graphql_list("edge_follow", "d04b0a864b4b5485e4776c30d917fc9", &target_id)
            .await
    }

    fn name(&self) -> &'static str {
        "Authenticated Session"
    }
}
