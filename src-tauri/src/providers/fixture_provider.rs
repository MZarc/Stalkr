use sha2::{Digest, Sha256};
use crate::models::{Account, RawMemberRecord};
use crate::providers::provider_trait::{
    AccessState, InstagramProvider, PrivacyState, ProfileResult, RelationshipFetchResult,
    TargetAccessCheckResult,
};

pub struct FixtureProvider {
    pub follower_count: usize,
    pub following_count: usize,
    pub mutual_count: usize,
}

impl Default for FixtureProvider {
    fn default() -> Self {
        Self {
            follower_count: 50,
            following_count: 35,
            mutual_count: 20,
        }
    }
}

const REALISTIC_PROFILES: &[(&str, &str, &str, bool, bool)] = &[
    ("elena.rostova", "Elena Rostova", "https://images.unsplash.com/photo-1534528741775-53994a69daeb?auto=format&fit=crop&w=120&q=80", true, false),
    ("alex_vance", "Alex Vance", "https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?auto=format&fit=crop&w=120&q=80", false, true),
    ("sora_k", "Sora Kuroki", "https://images.unsplash.com/photo-1517841905240-472988babdf9?auto=format&fit=crop&w=120&q=80", false, false),
    ("marcus_dev", "Marcus Thorne", "https://images.unsplash.com/photo-1500648767791-00dcc994a43e?auto=format&fit=crop&w=120&q=80", true, false),
    ("luna_art", "Luna Sterling", "https://images.unsplash.com/photo-1494790108377-be9c29b29330?auto=format&fit=crop&w=120&q=80", false, true),
    ("clara.zhao", "Clara Zhao", "https://images.unsplash.com/photo-1524504388940-b1c1722653e1?auto=format&fit=crop&w=120&q=80", false, false),
    ("hugo_nordic", "Hugo Lindqvist", "https://images.unsplash.com/photo-1519085360753-af0119f7cbe7?auto=format&fit=crop&w=120&q=80", true, false),
    ("maya_design", "Maya Patel", "https://images.unsplash.com/photo-1573496359142-b8d87734a5a2?auto=format&fit=crop&w=120&q=80", false, false),
    ("zane.ai", "Zane Robertson", "https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?auto=format&fit=crop&w=120&q=80", false, true),
    ("chloe_paris", "Chloé Laurent", "https://images.unsplash.com/photo-1544005313-94ddf0286df2?auto=format&fit=crop&w=120&q=80", true, false),
    ("david_lens", "David Kim", "https://images.unsplash.com/photo-1522075469751-3a6694fb2f61?auto=format&fit=crop&w=120&q=80", false, false),
    ("sophie_art", "Sophie Dubois", "https://images.unsplash.com/photo-1529626455594-4ff0802cfb7e?auto=format&fit=crop&w=120&q=80", true, false),
    ("kai_urban", "Kai Tanaka", "https://images.unsplash.com/photo-1539571696357-5a69c17a67c6?auto=format&fit=crop&w=120&q=80", false, true),
    ("olivia_travel", "Olivia Bennett", "https://images.unsplash.com/photo-1488426862026-3ee34a7d66df?auto=format&fit=crop&w=120&q=80", false, false),
    ("liam_creates", "Liam O'Connor", "https://images.unsplash.com/photo-1492562080023-ab3db95bfbce?auto=format&fit=crop&w=120&q=80", true, false),
    ("isabella_style", "Isabella Rossi", "https://images.unsplash.com/photo-1517841905240-472988babdf9?auto=format&fit=crop&w=120&q=80", false, false),
    ("noah_visuals", "Noah Miller", "https://images.unsplash.com/photo-1501196354995-cbb51c65aaea?auto=format&fit=crop&w=120&q=80", false, true),
    ("mia_aesthetic", "Mia Chen", "https://images.unsplash.com/photo-1534528741775-53994a69daeb?auto=format&fit=crop&w=120&q=80", true, false),
    ("ethan_sound", "Ethan Walker", "https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?auto=format&fit=crop&w=120&q=80", false, false),
    ("ava_ventures", "Ava Morales", "https://images.unsplash.com/photo-1544005313-94ddf0286df2?auto=format&fit=crop&w=120&q=80", false, true),
    ("lucas_lens", "Lucas Silva", "https://images.unsplash.com/photo-1500648767791-00dcc994a43e?auto=format&fit=crop&w=120&q=80", true, false),
    ("harper_daily", "Harper Evans", "https://images.unsplash.com/photo-1524504388940-b1c1722653e1?auto=format&fit=crop&w=120&q=80", false, false),
    ("mason_craft", "Mason Reed", "https://images.unsplash.com/photo-1519085360753-af0119f7cbe7?auto=format&fit=crop&w=120&q=80", false, true),
    ("evelyn_wild", "Evelyn Brooks", "https://images.unsplash.com/photo-1573496359142-b8d87734a5a2?auto=format&fit=crop&w=120&q=80", true, false),
    ("julian_apex", "Julian Wright", "https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?auto=format&fit=crop&w=120&q=80", false, false),
    ("grace_motion", "Grace Taylor", "https://images.unsplash.com/photo-1529626455594-4ff0802cfb7e?auto=format&fit=crop&w=120&q=80", false, false),
];

impl FixtureProvider {
    pub fn new(followers: usize, following: usize, mutuals: usize) -> Self {
        Self {
            follower_count: followers,
            following_count: following,
            mutual_count: mutuals,
        }
    }

    fn generate_members(count: usize, prefix: &str) -> Vec<RawMemberRecord> {
        let mut members = Vec::with_capacity(count);
        let pool_len = REALISTIC_PROFILES.len();
        
        // Offset start index by prefix to avoid identical lists for mutual vs fan
        let offset = match prefix {
            "mutual" => 0,
            "fan" => 8,
            "non_mutual" => 16,
            _ => 0,
        };

        for i in 0..count {
            let idx = (offset + i) % pool_len;
            let cycle = (offset + i) / pool_len;
            let (base_user, base_name, _avatar, is_ver, is_priv) = REALISTIC_PROFILES[idx];

            let username = if cycle == 0 {
                base_user.to_string()
            } else {
                format!("{}_{}", base_user, cycle + 1)
            };

            let display_name = if cycle == 0 {
                base_name.to_string()
            } else {
                format!("{} {}", base_name, cycle + 1)
            };

            members.push(RawMemberRecord {
                instagram_user_id: Some(format!("ig_fixture_{}_{}", prefix, i + 1)),
                username,
                display_name: Some(display_name),
                avatar_url: None,
                is_verified: is_ver,
                is_private: is_priv,
            });
        }
        members
    }
}

impl InstagramProvider for FixtureProvider {
    async fn validate_session(&self) -> Result<ProfileResult, String> {
        Ok(ProfileResult {
            instagram_user_id: Some("ig_meetzarc".to_string()),
            username: "meetzarc".to_string(),
            display_name: "Meet Zarc".to_string(),
            avatar_url: Some("https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?auto=format&fit=crop&w=160&q=80".to_string()),
            is_private: false,
            is_verified: true,
            followers_count: self.follower_count as i64,
            following_count: self.following_count as i64,
        })
    }

    async fn get_profile(&self, username: &str) -> Result<ProfileResult, String> {
        let is_private = username.contains("private");
        let (display, avatar) = match username {
            "meetzarc" => ("Meet Zarc", Some("https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?auto=format&fit=crop&w=160&q=80")),
            "private_friend" => ("Private Friend", Some("https://images.unsplash.com/photo-1544005313-94ddf0286df2?auto=format&fit=crop&w=120&q=80")),
            _ => (username, None),
        };
        Ok(ProfileResult {
            instagram_user_id: Some(format!("ig_{}", username)),
            username: username.to_string(),
            display_name: display.to_string(),
            avatar_url: avatar.map(|s| s.to_string()),
            is_private,
            is_verified: true,
            followers_count: self.follower_count as i64,
            following_count: self.following_count as i64,
        })
    }

    async fn check_target_access(&self, target_username: &str) -> Result<TargetAccessCheckResult, String> {
        let is_private = target_username.contains("private");
        let is_unauthorized = target_username.contains("unfollowed") || target_username.contains("unknown") || target_username.contains("inaccessible");

        if is_private && is_unauthorized {
            Ok(TargetAccessCheckResult {
                target_user_id: Some(format!("ig_{}", target_username)),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: PrivacyState::Private,
                access_state: AccessState::NotAccessible,
                access_reason: "This private account's relationship lists are not accessible through the connected Instagram account (not an approved follower).".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: self.follower_count as i64,
                following_count: self.following_count as i64,
            })
        } else if is_private {
            let followers = if target_username == "private_friend" { 18 } else { self.follower_count as i64 };
            let following = if target_username == "private_friend" { 14 } else { self.following_count as i64 };
            Ok(TargetAccessCheckResult {
                target_user_id: Some(format!("ig_{}", target_username)),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: PrivacyState::Private,
                access_state: AccessState::Accessible,
                access_reason: "Private account accessible through authenticated approved follower (@meetzarc).".to_string(),
                is_following: true,
                is_approved_follower: true,
                followers_count: followers,
                following_count: following,
            })
        } else {
            Ok(TargetAccessCheckResult {
                target_user_id: Some(format!("ig_{}", target_username)),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: PrivacyState::Public,
                access_state: AccessState::Accessible,
                access_reason: "Public account.".to_string(),
                is_following: true,
                is_approved_follower: true,
                followers_count: self.follower_count as i64,
                following_count: self.following_count as i64,
            })
        }
    }

    async fn get_followers(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
        if account.access_state == "not_accessible" {
            return Ok(RelationshipFetchResult {
                status: "NOT_ACCESSIBLE".to_string(),
                target_user_id: account.id.clone(),
                privacy_state: PrivacyState::Private,
                access_state: AccessState::NotAccessible,
                item_count: 0,
                pagination_complete: false,
                completeness: "incomplete".to_string(),
                members: Vec::new(),
                source_hash: "".to_string(),
                provider_error: Some("This private account's relationship lists are not accessible through the connected Instagram account.".to_string()),
            });
        }

        let mut members = Vec::new();
        // Mutual members
        members.extend(Self::generate_members(self.mutual_count, "mutual"));
        // Fans (followers only)
        let fans_count = self.follower_count.saturating_sub(self.mutual_count);
        members.extend(Self::generate_members(fans_count, "fan"));

        let mut hasher = Sha256::new();
        hasher.update(b"fixture_followers_hash");

        Ok(RelationshipFetchResult {
            status: "ACCESSIBLE".to_string(),
            target_user_id: account.id.clone(),
            privacy_state: if account.is_private { PrivacyState::Private } else { PrivacyState::Public },
            access_state: AccessState::Accessible,
            item_count: members.len() as i64,
            pagination_complete: true,
            completeness: "complete".to_string(),
            members,
            source_hash: format!("{:x}", hasher.finalize()),
            provider_error: None,
        })
    }

    async fn get_following(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
        if account.access_state == "not_accessible" {
            return Ok(RelationshipFetchResult {
                status: "NOT_ACCESSIBLE".to_string(),
                target_user_id: account.id.clone(),
                privacy_state: PrivacyState::Private,
                access_state: AccessState::NotAccessible,
                item_count: 0,
                pagination_complete: false,
                completeness: "incomplete".to_string(),
                members: Vec::new(),
                source_hash: "".to_string(),
                provider_error: Some("This private account's relationship lists are not accessible through the connected Instagram account.".to_string()),
            });
        }

        let mut members = Vec::new();
        // Mutual members
        members.extend(Self::generate_members(self.mutual_count, "mutual"));
        // Non-mutuals (following only)
        let non_mutual_count = self.following_count.saturating_sub(self.mutual_count);
        members.extend(Self::generate_members(non_mutual_count, "non_mutual"));

        let mut hasher = Sha256::new();
        hasher.update(b"fixture_following_hash");

        Ok(RelationshipFetchResult {
            status: "ACCESSIBLE".to_string(),
            target_user_id: account.id.clone(),
            privacy_state: if account.is_private { PrivacyState::Private } else { PrivacyState::Public },
            access_state: AccessState::Accessible,
            item_count: members.len() as i64,
            pagination_complete: true,
            completeness: "complete".to_string(),
            members,
            source_hash: format!("{:x}", hasher.finalize()),
            provider_error: None,
        })
    }

    fn name(&self) -> &'static str {
        "Fixture / Test Provider (DEMO DATA)"
    }
}
