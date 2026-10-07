use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub instagram_user_id: Option<String>,
    pub current_username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_verified: bool,
    pub is_private: bool,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipState {
    pub account_id: String,
    pub person_id: String,
    pub is_follower: bool,
    pub is_following: bool,
    pub first_observed_at: i64,
    pub last_observed_at: i64,
    pub authenticated_by_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonListItem {
    pub id: String,
    pub instagram_user_id: Option<String>,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_verified: bool,
    pub is_private: bool,
    pub is_follower: bool,
    pub is_following: bool,
    pub is_mutual: bool,
    pub last_seen_at: i64,
    pub tags: Vec<Tag>,
    pub has_note: bool,
    pub last_change_type: Option<String>,
    pub last_change_at: Option<i64>,
}
