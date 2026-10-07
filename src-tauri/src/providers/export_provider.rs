use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use crate::models::{Account, RawMemberRecord};
use crate::providers::provider_trait::{FetchResult, InstagramProvider, ProfileResult};

pub struct ExportArchiveProvider;

impl ExportArchiveProvider {
    /// Discovers and parses all `followers*.json` shards from an export directory
    pub fn parse_followers_dir(dir_path: &Path) -> Result<FetchResult, String> {
        let mut shard_paths = Vec::new();

        // Search in given directory and also in `followers_and_following` subfolder if present
        let target_dirs = vec![
            dir_path.to_path_buf(),
            dir_path.join("followers_and_following"),
            dir_path.join("connections").join("followers_and_following"),
        ];

        for d in &target_dirs {
            if d.exists() && d.is_dir() {
                if let Ok(entries) = fs::read_dir(d) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            let lower = file_name.to_lowercase();
                            if lower.starts_with("followers") && lower.ends_with(".json") {
                                shard_paths.push(path);
                            }
                        }
                    }
                }
            }
        }

        shard_paths.sort();
        shard_paths.dedup();

        if shard_paths.is_empty() {
            return Err("No followers*.json files found in the selected export directory.".to_string());
        }

        let mut all_members = Vec::new();
        let mut seen_usernames = HashSet::new();
        let mut hasher = Sha256::new();

        for shard in shard_paths {
            let content = fs::read_to_string(&shard)
                .map_err(|e| format!("Failed to read shard {}: {}", shard.display(), e))?;
            hasher.update(content.as_bytes());

            let parsed = Self::parse_export_json_content(&content)?;
            for member in parsed {
                if seen_usernames.insert(member.username.to_lowercase()) {
                    all_members.push(member);
                }
            }
        }

        let source_hash = format!("{:x}", hasher.finalize());
        let count = all_members.len() as i64;

        Ok(FetchResult {
            members: all_members,
            item_count: count,
            is_complete: true,
            source_hash,
            error_message: None,
        })
    }

    /// Discovers and parses all `following*.json` files from an export directory
    pub fn parse_following_dir(dir_path: &Path) -> Result<FetchResult, String> {
        let mut shard_paths = Vec::new();

        let target_dirs = vec![
            dir_path.to_path_buf(),
            dir_path.join("followers_and_following"),
            dir_path.join("connections").join("followers_and_following"),
        ];

        for d in &target_dirs {
            if d.exists() && d.is_dir() {
                if let Ok(entries) = fs::read_dir(d) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            let lower = file_name.to_lowercase();
                            if lower.starts_with("following") && lower.ends_with(".json") {
                                shard_paths.push(path);
                            }
                        }
                    }
                }
            }
        }

        shard_paths.sort();
        shard_paths.dedup();

        if shard_paths.is_empty() {
            return Err("No following*.json files found in the selected export directory.".to_string());
        }

        let mut all_members = Vec::new();
        let mut seen_usernames = HashSet::new();
        let mut hasher = Sha256::new();

        for shard in shard_paths {
            let content = fs::read_to_string(&shard)
                .map_err(|e| format!("Failed to read shard {}: {}", shard.display(), e))?;
            hasher.update(content.as_bytes());

            let parsed = Self::parse_export_json_content(&content)?;
            for member in parsed {
                if seen_usernames.insert(member.username.to_lowercase()) {
                    all_members.push(member);
                }
            }
        }

        let source_hash = format!("{:x}", hasher.finalize());
        let count = all_members.len() as i64;

        Ok(FetchResult {
            members: all_members,
            item_count: count,
            is_complete: true,
            source_hash,
            error_message: None,
        })
    }

    /// Parses raw JSON content from Instagram's official data archive formats
    pub fn parse_export_json_content(content: &str) -> Result<Vec<RawMemberRecord>, String> {
        let root: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("JSON parsing failed: {}", e))?;

        let mut records = Vec::new();

        // Format 1: Direct Array of entries
        // [ { "string_list_data": [ { "value": "username", "href": "..." } ] } ]
        if let Some(arr) = root.as_array() {
            for item in arr {
                Self::extract_record_from_entry(item, &mut records);
            }
        }
        // Format 2: Object with "relationships_following" or "relationships_followers"
        else if let Some(obj) = root.as_object() {
            for key in ["relationships_following", "relationships_followers", "data"] {
                if let Some(arr) = obj.get(key).and_then(|v| v.as_array()) {
                    for item in arr {
                        Self::extract_record_from_entry(item, &mut records);
                    }
                }
            }
        }

        Ok(records)
    }

    fn extract_record_from_entry(entry: &serde_json::Value, records: &mut Vec<RawMemberRecord>) {
        // Look inside string_list_data
        if let Some(string_list) = entry.get("string_list_data").and_then(|s| s.as_array()) {
            for s in string_list {
                if let Some(val) = s.get("value").and_then(|v| v.as_str()) {
                    let clean = val.trim();
                    if !clean.is_empty() {
                        records.push(RawMemberRecord {
                            instagram_user_id: None,
                            username: clean.to_string(),
                            display_name: entry.get("title").and_then(|t| t.as_str()).map(|t| t.to_string()),
                            avatar_url: None,
                            is_verified: false,
                            is_private: false,
                        });
                    }
                }
            }
        } else if let Some(title) = entry.get("title").and_then(|t| t.as_str()) {
            let clean = title.trim();
            if !clean.is_empty() {
                records.push(RawMemberRecord {
                    instagram_user_id: None,
                    username: clean.to_string(),
                    display_name: None,
                    avatar_url: None,
                    is_verified: false,
                    is_private: false,
                });
            }
        }
    }
}

impl InstagramProvider for ExportArchiveProvider {
    async fn validate_session(&self) -> Result<ProfileResult, String> {
        Ok(ProfileResult {
            instagram_user_id: None,
            username: "export_account".to_string(),
            display_name: "Export Account".to_string(),
            avatar_url: None,
            is_private: false,
            is_verified: false,
            followers_count: 0,
            following_count: 0,
        })
    }

    async fn get_profile(&self, username: &str) -> Result<ProfileResult, String> {
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

    async fn check_target_access(&self, target_username: &str) -> Result<crate::providers::provider_trait::TargetAccessCheckResult, String> {
        Ok(crate::providers::provider_trait::TargetAccessCheckResult {
            target_user_id: None,
            target_username: target_username.to_string(),
            avatar_url: None,
            privacy_state: crate::providers::provider_trait::PrivacyState::Unknown,
            access_state: crate::providers::provider_trait::AccessState::Accessible,
            access_reason: "Official data archive export.".to_string(),
            is_following: true,
            is_approved_follower: true,
            followers_count: 0,
            following_count: 0,
        })
    }

    async fn get_followers(&self, account: &Account) -> Result<crate::providers::provider_trait::RelationshipFetchResult, String> {
        let export_dir = PathBuf::from("instagram_export");
        let fetch = Self::parse_followers_dir(&export_dir)?;
        Ok(crate::providers::provider_trait::RelationshipFetchResult {
            status: "ACCESSIBLE".to_string(),
            target_user_id: account.id.clone(),
            privacy_state: crate::providers::provider_trait::PrivacyState::Unknown,
            access_state: crate::providers::provider_trait::AccessState::Accessible,
            item_count: fetch.item_count,
            pagination_complete: fetch.is_complete,
            completeness: if fetch.is_complete { "complete".to_string() } else { "incomplete".to_string() },
            members: fetch.members,
            source_hash: fetch.source_hash,
            provider_error: fetch.error_message,
        })
    }

    async fn get_following(&self, account: &Account) -> Result<crate::providers::provider_trait::RelationshipFetchResult, String> {
        let export_dir = PathBuf::from("instagram_export");
        let fetch = Self::parse_following_dir(&export_dir)?;
        Ok(crate::providers::provider_trait::RelationshipFetchResult {
            status: "ACCESSIBLE".to_string(),
            target_user_id: account.id.clone(),
            privacy_state: crate::providers::provider_trait::PrivacyState::Unknown,
            access_state: crate::providers::provider_trait::AccessState::Accessible,
            item_count: fetch.item_count,
            pagination_complete: fetch.is_complete,
            completeness: if fetch.is_complete { "complete".to_string() } else { "incomplete".to_string() },
            members: fetch.members,
            source_hash: fetch.source_hash,
            provider_error: fetch.error_message,
        })
    }

    fn name(&self) -> &'static str {
        "Official Export Ingestion"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modern_instagram_export_json() {
        let sample = r#"[
            {
                "title": "alice_smith",
                "media_list_data": [],
                "string_list_data": [
                    {
                        "href": "https://www.instagram.com/alice_smith",
                        "value": "alice_smith",
                        "timestamp": 1720000000
                    }
                ]
            },
            {
                "title": "",
                "media_list_data": [],
                "string_list_data": [
                    {
                        "href": "https://www.instagram.com/bob_jones",
                        "value": "bob_jones",
                        "timestamp": 1720000100
                    }
                ]
            }
        ]"#;

        let records = ExportArchiveProvider::parse_export_json_content(sample).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].username, "alice_smith");
        assert_eq!(records[1].username, "bob_jones");
    }

    #[test]
    fn test_parse_nested_following_format() {
        let sample = r#"{
            "relationships_following": [
                {
                    "title": "charlie_brown",
                    "string_list_data": [
                        {
                            "href": "https://www.instagram.com/charlie_brown",
                            "value": "charlie_brown",
                            "timestamp": 1720000200
                        }
                    ]
                }
            ]
        }"#;

        let records = ExportArchiveProvider::parse_export_json_content(sample).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].username, "charlie_brown");
    }
}
