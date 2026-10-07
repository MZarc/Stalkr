use std::collections::{HashMap, HashSet};
use crate::models::*;

pub struct DiffEngine;

pub struct DiffResult {
    pub changes: Vec<RelationshipChange>,
    pub updated_states: Vec<RelationshipState>,
}

impl DiffEngine {
    /// Compares two snapshots of the same type (e.g. Followers or Following) and computes changes.
    pub fn diff_snapshots(
        account_id: &str,
        snapshot_type: SnapshotType,
        before_snapshot_id: Option<&str>,
        after_snapshot_id: &str,
        old_members: &[SnapshotMember],
        new_members: &[SnapshotMember],
        detected_at: i64,
        confidence: Confidence,
    ) -> Vec<RelationshipChange> {
        let mut changes = Vec::new();

        // If this is the initial baseline (no before_snapshot), no unfollow/follow changes are produced
        if before_snapshot_id.is_none() {
            return changes;
        }

        let old_map: HashMap<&str, &SnapshotMember> = old_members
            .iter()
            .map(|m| (m.person_id.as_str(), m))
            .collect();

        let new_map: HashMap<&str, &SnapshotMember> = new_members
            .iter()
            .map(|m| (m.person_id.as_str(), m))
            .collect();

        let old_ids: HashSet<&str> = old_map.keys().copied().collect();
        let new_ids: HashSet<&str> = new_map.keys().copied().collect();

        // New members (in new, not in old)
        for added_id in new_ids.difference(&old_ids) {
            let member = new_map[*added_id];
            let change_type = match snapshot_type {
                SnapshotType::Followers => ChangeType::FollowedYou,
                SnapshotType::Following => ChangeType::YouFollowed,
                SnapshotType::Combined => ChangeType::FollowedYou,
            };

            changes.push(RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: (*added_id).to_string(),
                related_username: member.username_at_snapshot.clone(),
                change_type,
                confidence: confidence.clone(),
                metadata_json: None,
                detected_at,
                before_snapshot_id: before_snapshot_id.map(|s| s.to_string()),
                after_snapshot_id: Some(after_snapshot_id.to_string()),
                authenticated_by_account_id: None,
            });
        }

        // Lost members (in old, not in new)
        for removed_id in old_ids.difference(&new_ids) {
            let member = old_map[*removed_id];
            let change_type = match snapshot_type {
                SnapshotType::Followers => ChangeType::UnfollowedYou,
                SnapshotType::Following => ChangeType::YouUnfollowed,
                SnapshotType::Combined => ChangeType::UnfollowedYou,
            };

            changes.push(RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.to_string(),
                person_id: (*removed_id).to_string(),
                related_username: member.username_at_snapshot.clone(),
                change_type,
                confidence: confidence.clone(),
                metadata_json: None,
                detected_at,
                before_snapshot_id: before_snapshot_id.map(|s| s.to_string()),
                after_snapshot_id: Some(after_snapshot_id.to_string()),
                authenticated_by_account_id: None,
            });
        }

        // Check for username changes among persisting members
        for common_id in old_ids.intersection(&new_ids) {
            let old_m = old_map[*common_id];
            let new_m = new_map[*common_id];

            if old_m.username_at_snapshot.to_lowercase() != new_m.username_at_snapshot.to_lowercase() {
                let metadata = serde_json::json!({
                    "old_username": old_m.username_at_snapshot,
                    "new_username": new_m.username_at_snapshot
                });

                changes.push(RelationshipChange {
                    id: uuid::Uuid::new_v4().to_string(),
                    account_id: account_id.to_string(),
                    person_id: (*common_id).to_string(),
                    related_username: new_m.username_at_snapshot.clone(),
                    change_type: ChangeType::UsernameChanged,
                    confidence: Confidence::Confirmed,
                    metadata_json: Some(metadata.to_string()),
                    detected_at,
                    before_snapshot_id: before_snapshot_id.map(|s| s.to_string()),
                    after_snapshot_id: Some(after_snapshot_id.to_string()),
                    authenticated_by_account_id: None,
                });
            }
        }

        changes
    }

    /// Recomputes the entire materialized relationship state from the current followers and following sets.
    pub fn compute_relationship_states(
        account_id: &str,
        current_followers: &[SnapshotMember],
        current_following: &[SnapshotMember],
        observed_at: i64,
    ) -> Vec<RelationshipState> {
        let follower_set: HashSet<&str> = current_followers.iter().map(|m| m.person_id.as_str()).collect();
        let following_set: HashSet<&str> = current_following.iter().map(|m| m.person_id.as_str()).collect();

        let all_person_ids: HashSet<&str> = follower_set.union(&following_set).copied().collect();
        let mut states = Vec::with_capacity(all_person_ids.len());

        for pid in all_person_ids {
            let is_follower = follower_set.contains(pid);
            let is_following = following_set.contains(pid);

            states.push(RelationshipState {
                account_id: account_id.to_string(),
                person_id: pid.to_string(),
                is_follower,
                is_following,
                first_observed_at: observed_at,
                last_observed_at: observed_at,
                authenticated_by_account_id: None,
            });
        }

        states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_member(id: &str, username: &str) -> SnapshotMember {
        SnapshotMember {
            snapshot_id: "snap_1".to_string(),
            person_id: id.to_string(),
            username_at_snapshot: username.to_string(),
            display_name_at_snapshot: None,
            avatar_url_at_snapshot: None,
        }
    }

    #[test]
    fn test_diff_snapshots_addition_and_removal() {
        let old = vec![
            mock_member("p1", "alice"),
            mock_member("p2", "bob"),
            mock_member("p3", "charlie"),
        ];

        let new = vec![
            mock_member("p1", "alice"),
            mock_member("p3", "charlie"),
            mock_member("p4", "david"),
        ];

        let changes = DiffEngine::diff_snapshots(
            "acc_1",
            SnapshotType::Followers,
            Some("snap_old"),
            "snap_new",
            &old,
            &new,
            1700000000,
            Confidence::Confirmed,
        );

        assert_eq!(changes.len(), 2);
        let followed = changes.iter().find(|c| c.change_type == ChangeType::FollowedYou).unwrap();
        assert_eq!(followed.person_id, "p4");
        assert_eq!(followed.related_username, "david");

        let unfollowed = changes.iter().find(|c| c.change_type == ChangeType::UnfollowedYou).unwrap();
        assert_eq!(unfollowed.person_id, "p2");
        assert_eq!(unfollowed.related_username, "bob");
    }

    #[test]
    fn test_diff_snapshots_username_change() {
        let old = vec![
            mock_member("p1", "alice_old"),
            mock_member("p2", "bob"),
        ];

        let new = vec![
            mock_member("p1", "alice_new"),
            mock_member("p2", "bob"),
        ];

        let changes = DiffEngine::diff_snapshots(
            "acc_1",
            SnapshotType::Followers,
            Some("snap_old"),
            "snap_new",
            &old,
            &new,
            1700000000,
            Confidence::Confirmed,
        );

        assert_eq!(changes.len(), 1);
        let rename = &changes[0];
        assert_eq!(rename.change_type, ChangeType::UsernameChanged);
        assert_eq!(rename.person_id, "p1");
        assert!(rename.metadata_json.as_ref().unwrap().contains("alice_old"));
        assert!(rename.metadata_json.as_ref().unwrap().contains("alice_new"));
    }

    #[test]
    fn test_diff_following_snapshots() {
        let old = vec![
            mock_member("p1", "alice"),
            mock_member("p2", "bob"),
        ];

        let new = vec![
            mock_member("p1", "alice"),
            mock_member("p3", "claire"),
        ];

        let changes = DiffEngine::diff_snapshots(
            "acc_1",
            SnapshotType::Following,
            Some("snap_old"),
            "snap_new",
            &old,
            &new,
            1700000000,
            Confidence::Confirmed,
        );

        assert_eq!(changes.len(), 2);
        let followed = changes.iter().find(|c| c.change_type == ChangeType::YouFollowed).unwrap();
        assert_eq!(followed.person_id, "p3");
        assert_eq!(followed.related_username, "claire");

        let unfollowed = changes.iter().find(|c| c.change_type == ChangeType::YouUnfollowed).unwrap();
        assert_eq!(unfollowed.person_id, "p2");
        assert_eq!(unfollowed.related_username, "bob");
    }

    #[test]
    fn test_diff_identical_snapshots_produces_no_changes() {
        let old = vec![
            mock_member("p1", "alice"),
            mock_member("p2", "bob"),
        ];

        let new = vec![
            mock_member("p1", "alice"),
            mock_member("p2", "bob"),
        ];

        let changes = DiffEngine::diff_snapshots(
            "acc_1",
            SnapshotType::Followers,
            Some("snap_old"),
            "snap_new",
            &old,
            &new,
            1700000000,
            Confidence::Confirmed,
        );

        assert_eq!(changes.len(), 0);
    }
}
