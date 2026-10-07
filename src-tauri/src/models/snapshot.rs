use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotType {
    Followers,
    Following,
    Combined,
}

impl SnapshotType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SnapshotType::Followers => "followers",
            SnapshotType::Following => "following",
            SnapshotType::Combined => "combined",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "followers" => SnapshotType::Followers,
            "following" => SnapshotType::Following,
            _ => SnapshotType::Combined,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStatus {
    Complete,
    Incomplete,
    Failed,
}

impl SnapshotStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SnapshotStatus::Complete => "complete",
            SnapshotStatus::Incomplete => "incomplete",
            SnapshotStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "complete" => SnapshotStatus::Complete,
            "incomplete" => SnapshotStatus::Incomplete,
            _ => SnapshotStatus::Failed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub account_id: String,
    pub snapshot_type: SnapshotType,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub status: SnapshotStatus,
    pub item_count: i64,
    pub source_hash: String,
    pub error_message: Option<String>,
    pub authenticated_by_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMember {
    pub snapshot_id: String,
    pub person_id: String,
    pub username_at_snapshot: String,
    pub display_name_at_snapshot: Option<String>,
    pub avatar_url_at_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMemberRecord {
    pub instagram_user_id: Option<String>,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_verified: bool,
    pub is_private: bool,
}
