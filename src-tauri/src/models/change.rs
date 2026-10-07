use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    FollowedYou,
    UnfollowedYou,
    YouFollowed,
    YouUnfollowed,
    UsernameChanged,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeType::FollowedYou => "followed_you",
            ChangeType::UnfollowedYou => "unfollowed_you",
            ChangeType::YouFollowed => "you_followed",
            ChangeType::YouUnfollowed => "you_unfollowed",
            ChangeType::UsernameChanged => "username_changed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "followed_you" => ChangeType::FollowedYou,
            "unfollowed_you" => ChangeType::UnfollowedYou,
            "you_followed" => ChangeType::YouFollowed,
            "you_unfollowed" => ChangeType::YouUnfollowed,
            _ => ChangeType::UsernameChanged,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    Unconfirmed,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::Confirmed => "confirmed",
            Confidence::Unconfirmed => "unconfirmed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "confirmed" => Confidence::Confirmed,
            _ => Confidence::Unconfirmed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipChange {
    pub id: String,
    pub account_id: String,
    pub person_id: String,
    pub related_username: String,
    pub change_type: ChangeType,
    pub confidence: Confidence,
    pub metadata_json: Option<String>,
    pub detected_at: i64,
    pub before_snapshot_id: Option<String>,
    pub after_snapshot_id: Option<String>,
    pub authenticated_by_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeFeedItem {
    pub id: String,
    pub account_id: String,
    pub person_id: String,
    pub related_username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub change_type: ChangeType,
    pub confidence: Confidence,
    pub metadata_json: Option<String>,
    pub detected_at: i64,
    pub before_snapshot_id: Option<String>,
    pub after_snapshot_id: Option<String>,
}

/// A candidate follow/unfollow awaiting two-cycle confirmation.
/// Single-sync list flaps land here and are deleted on the next sync if
/// contradicted — they never reach the changes feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEvent {
    pub id: String,
    pub account_id: String,
    pub person_id: String,
    pub related_username: String,
    pub change_type: ChangeType,
    pub first_seen_at: i64,
    pub first_snapshot_id: Option<String>,
    pub authenticated_by_account_id: Option<String>,
}
