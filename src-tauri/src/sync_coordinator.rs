use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::db::Database;
use crate::diff::{CardinalityVerifier, DiffEngine, VerificationDecision};
use crate::models::*;
use crate::providers::InstagramProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub success: bool,
    pub status: String, // "complete", "incomplete", "failed", "rate_limited", "not_accessible", "throttled_local"
    pub followers_count: i64,
    pub following_count: i64,
    pub changes_detected: usize,
    pub message: String,
    // ── Premium accuracy fields (additive; defaults keep old clients working) ──
    /// Authoritative Instagram profile header counts (what the official app shows).
    #[serde(default)]
    pub profile_followers_count: Option<i64>,
    #[serde(default)]
    pub profile_following_count: Option<i64>,
    /// How many relationships were actually verified page-by-page this sync.
    #[serde(default)]
    pub tracked_followers_count: Option<i64>,
    #[serde(default)]
    pub tracked_following_count: Option<i64>,
    /// Human-readable verification outcome, e.g. "header_matched", "header_quarantined".
    #[serde(default)]
    pub verification: Option<String>,
}

impl SyncReport {
    fn base(
        account: &Account,
        success: bool,
        status: &str,
        changes_detected: usize,
        message: String,
    ) -> Self {
        Self {
            success,
            status: status.to_string(),
            followers_count: account.followers_count,
            following_count: account.following_count,
            changes_detected,
            message,
            profile_followers_count: None,
            profile_following_count: None,
            tracked_followers_count: None,
            tracked_following_count: None,
            verification: None,
        }
    }
}

pub struct SyncCoordinator;

/// Minimum gap between two live Instagram syncs for the same account.
/// Prevents accidental hammering (the #1 ban cause) when the UI retries,
/// background workers overlap, or the user spams refresh.
/// Only applies to the real network provider — mocks/tests bypass it.
const MIN_SYNC_INTERVAL_SECS: i64 = 30;

impl SyncCoordinator {
    pub async fn execute_sync<P: InstagramProvider>(
        db: &Arc<Database>,
        account_id: &str,
        provider: &P,
    ) -> Result<SyncReport, String> {
        let account = db.get_account(account_id)
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Account {} not found", account_id))?;

        // Guardrail: Explicit access check for monitored targets
        if account.account_kind == AccountKind::Monitored && account.access_state == "not_accessible" {
            let reason = account.access_reason.clone().unwrap_or_else(|| "This private account's relationship lists are not accessible through the connected Instagram account.".to_string());
            return Ok(SyncReport::base(
                &account,
                false,
                "not_accessible",
                0,
                reason,
            ));
        }

        let now = chrono::Utc::now().timestamp();

        // Safety: local frequency guard for the live network provider.
        // Returns a soft "throttled_local" instead of hitting Instagram again.
        if provider.name() == "Authenticated Session" {
            if let Some(last) = account.last_attempted_sync_at {
                if now - last < MIN_SYNC_INTERVAL_SECS {
                    let wait = MIN_SYNC_INTERVAL_SECS - (now - last);
                    return Ok(SyncReport::base(
                        &account,
                        false,
                        "throttled_local",
                        0,
                        format!(
                            "Sync throttled locally to protect your Instagram session ({}s cooldown). Showing last verified state — no data changed.",
                            wait
                        ),
                    ));
                }
            }
        }

        // Record attempt early so overlapping workers/back-to-back taps
        // cannot double-fire paginated fetches against Instagram.
        {
            let mut attempted = account.clone();
            attempted.last_attempted_sync_at = Some(now);
            attempted.updated_at = now;
            let _ = db.upsert_account(&attempted);
        }

        // 0. Authoritative header counts (what official Instagram shows).
        // Best-effort: any provider may return 0s (mock/export/public);
        // in that case we fall back to legacy list-length behaviour.
        let (header_followers, header_following) = match provider.get_profile(&account.username).await {
            Ok(p) if p.followers_count > 0 || p.following_count > 0 => {
                (Some(p.followers_count), Some(p.following_count))
            }
            _ => (None, None),
        };

        // 1. Fetch Followers
        let mut followers_res = provider.get_followers(&account).await?;
        if !followers_res.pagination_complete {
            let snap_id = uuid::Uuid::new_v4().to_string();
            let incomplete_snap = Snapshot {
                id: snap_id,
                account_id: account.id.clone(),
                snapshot_type: SnapshotType::Followers,
                started_at: now,
                completed_at: Some(now),
                status: SnapshotStatus::Incomplete,
                item_count: followers_res.item_count,
                source_hash: followers_res.source_hash,
                error_message: followers_res.provider_error.clone(),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            };
            let _ = db.insert_snapshot(&incomplete_snap);

            let mut r = SyncReport::base(
                &account,
                false,
                "incomplete",
                0,
                followers_res.provider_error.unwrap_or_else(|| "Follower list was partial. Existing state preserved.".to_string()),
            );
            r.profile_followers_count = header_followers;
            r.profile_following_count = header_following;
            r.tracked_followers_count = Some(followers_res.item_count);
            r.verification = Some("pagination_incomplete".to_string());
            return Ok(r);
        }

        // 2. Fetch Following
        let mut following_res = provider.get_following(&account).await?;
        if !following_res.pagination_complete {
            let snap_id = uuid::Uuid::new_v4().to_string();
            let incomplete_snap = Snapshot {
                id: snap_id,
                account_id: account.id.clone(),
                snapshot_type: SnapshotType::Following,
                started_at: now,
                completed_at: Some(now),
                status: SnapshotStatus::Incomplete,
                item_count: following_res.item_count,
                source_hash: following_res.source_hash,
                error_message: following_res.provider_error.clone(),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            };
            let _ = db.insert_snapshot(&incomplete_snap);

            let mut r = SyncReport::base(
                &account,
                false,
                "incomplete",
                0,
                following_res.provider_error.unwrap_or_else(|| "Following list was partial. Existing state preserved.".to_string()),
            );
            r.profile_followers_count = header_followers;
            r.profile_following_count = header_following;
            r.tracked_followers_count = Some(followers_res.item_count);
            r.tracked_following_count = Some(following_res.item_count);
            r.verification = Some("pagination_incomplete".to_string());
            return Ok(r);
        }

        // 2b. Secondary-surface union (web GraphQL edges, Instaloader-style).
        // Runs ONLY when the header says the primary surface came up short
        // (deficit of 3+ members and coverage below 98%), so a healthy sync
        // costs zero extra requests. The union can only ADD members the
        // primary missed — it can never manufacture false unfollows — and any
        // secondary failure is silently ignored (primary stands alone).
        let mut secondary_added_followers: usize = 0;
        let mut secondary_added_following: usize = 0;
        if let Some(hf) = header_followers {
            let deficit = hf.saturating_sub(followers_res.item_count);
            let coverage = followers_res.item_count * 100 / hf.max(1);
            if deficit >= 3 && coverage < 98 {
                match provider.get_followers_secondary(&account).await {
                    Ok(Some(extra)) => {
                        let before = followers_res.members.len();
                        followers_res.members =
                            crate::providers::union_members(followers_res.members, extra);
                        secondary_added_followers =
                            followers_res.members.len().saturating_sub(before);
                        followers_res.item_count = followers_res.members.len() as i64;
                    }
                    _ => {}
                }
            }
        }
        if let Some(hg) = header_following {
            let deficit = hg.saturating_sub(following_res.item_count);
            let coverage = following_res.item_count * 100 / hg.max(1);
            if deficit >= 3 && coverage < 98 {
                match provider.get_following_secondary(&account).await {
                    Ok(Some(extra)) => {
                        let before = following_res.members.len();
                        following_res.members =
                            crate::providers::union_members(following_res.members, extra);
                        secondary_added_following =
                            following_res.members.len().saturating_sub(before);
                        following_res.item_count = following_res.members.len() as i64;
                    }
                    _ => {}
                }
            }
        }

        // 3. Previous complete snapshots (history baseline).
        // NOTE: looked up BEFORE the header gate on purpose — a first-ever
        // sync (no history) must always be accepted. With no baseline, no
        // false unfollow can possibly be produced, so blocking bootstrap on
        // a header mismatch would brick the app at all-zeros forever.
        let prev_followers_snap = db.get_latest_complete_snapshot(
            &account.id,
            SnapshotType::Followers,
            account.authenticated_by_account_id.as_deref(),
        ).map_err(|e| format!("DB error: {}", e))?;
        let prev_count = prev_followers_snap.as_ref().map(|s| s.item_count).unwrap_or(0);

        let prev_following_snap = db.get_latest_complete_snapshot(
            &account.id,
            SnapshotType::Following,
            account.authenticated_by_account_id.as_deref(),
        ).map_err(|e| format!("DB error: {}", e))?;
        let prev_following_count = prev_following_snap.as_ref().map(|s| s.item_count).unwrap_or(0);

        // 2b. Header reconciliation HARD gate (premium accuracy).
        // Blocks only certain corruption (empty list vs known history, or a
        // >50% catastrophic gap) and only when history exists to protect.
        // Bootstrap syncs always pass: there is nothing to corrupt yet.
        // Moderate gaps flow through as Unconfirmed instead of bricking.
        //
        // Two exceptions that block EVEN in bootstrap (an empty baseline
        // would otherwise poison every screen with fake success-zeros and
        // turn the next real sync into hundreds of false "new followers"):
        //  - an empty list against a positive header signal;
        //  - empty lists with zero signal at all (profile unreachable too —
        //    the session is almost certainly not returning real data).
        let bootstrapping =
            prev_followers_snap.is_none() && prev_following_snap.is_none();
        if bootstrapping
            && followers_res.item_count == 0
            && following_res.item_count == 0
            && header_followers.unwrap_or(0) <= 0
            && header_following.unwrap_or(0) <= 0
        {
            let mut r = SyncReport::base(
                &account,
                false,
                "incomplete",
                0,
                "Instagram returned empty follower AND following lists and profile counts are unreachable, so there is nothing honest to baseline yet. Your session is probably stale — reconnect Instagram and sync again. Nothing was stored.".to_string(),
            );
            r.verification = Some("empty_bootstrap_no_signal".to_string());
            return Ok(r);
        }
        if prev_followers_snap.is_some() || followers_res.item_count == 0 {
            if let Some(reason) = CardinalityVerifier::check_list_vs_header_hard(
                followers_res.item_count,
                header_followers.unwrap_or(0),
                prev_count,
            ) {
                let _ = db.insert_snapshot(&Snapshot {
                    id: uuid::Uuid::new_v4().to_string(),
                    account_id: account.id.clone(),
                    snapshot_type: SnapshotType::Followers,
                    started_at: now,
                    completed_at: Some(now),
                    status: SnapshotStatus::Incomplete,
                    item_count: followers_res.item_count,
                    source_hash: followers_res.source_hash.clone(),
                    error_message: Some(reason.clone()),
                    authenticated_by_account_id: account.authenticated_by_account_id.clone(),
                });
                let mut r = SyncReport::base(&account, false, "incomplete", 0, reason);
                r.profile_followers_count = header_followers;
                r.profile_following_count = header_following;
                r.tracked_followers_count = Some(followers_res.item_count);
                r.tracked_following_count = Some(following_res.item_count);
                r.verification = Some("header_quarantined_followers".to_string());
                return Ok(r);
            }
        }
        if prev_following_snap.is_some() || following_res.item_count == 0 {
            if let Some(reason) = CardinalityVerifier::check_list_vs_header_hard(
                following_res.item_count,
                header_following.unwrap_or(0),
                prev_following_count,
            ) {
                let _ = db.insert_snapshot(&Snapshot {
                    id: uuid::Uuid::new_v4().to_string(),
                    account_id: account.id.clone(),
                    snapshot_type: SnapshotType::Following,
                    started_at: now,
                    completed_at: Some(now),
                    status: SnapshotStatus::Incomplete,
                    item_count: following_res.item_count,
                    source_hash: following_res.source_hash.clone(),
                    error_message: Some(reason.clone()),
                    authenticated_by_account_id: account.authenticated_by_account_id.clone(),
                });
                let mut r = SyncReport::base(&account, false, "incomplete", 0, reason);
                r.profile_followers_count = header_followers;
                r.profile_following_count = header_following;
                r.tracked_followers_count = Some(followers_res.item_count);
                r.tracked_following_count = Some(following_res.item_count);
                r.verification = Some("header_quarantined_following".to_string());
                return Ok(r);
            }
        }

        // Legacy 25%-drop ladder (kept for compatibility) …
        let decision = CardinalityVerifier::evaluate(
            prev_count,
            followers_res.item_count,
            followers_res.pagination_complete,
            false,
            None,
        );

        match decision {
            VerificationDecision::Incomplete { reason } => {
                let mut r = SyncReport::base(&account, false, "incomplete", 0, reason);
                r.profile_followers_count = header_followers;
                r.profile_following_count = header_following;
                r.tracked_followers_count = Some(followers_res.item_count);
                r.tracked_following_count = Some(following_res.item_count);
                r.verification = Some("cardinality_incomplete".to_string());
                return Ok(r);
            }
            VerificationDecision::RequiresVerificationFetch { .. } => {
                // Secondary verification fetch
                let second_fetch = provider.get_followers(&account).await?;
                let verify_decision = CardinalityVerifier::evaluate(
                    prev_count,
                    second_fetch.item_count,
                    second_fetch.pagination_complete,
                    true,
                    Some(followers_res.item_count),
                );

                if let VerificationDecision::Incomplete { reason: r } = verify_decision {
                    let mut rep = SyncReport::base(
                        &account,
                        false,
                        "incomplete",
                        0,
                        format!("Verification aborted: {}", r),
                    );
                    rep.profile_followers_count = header_followers;
                    rep.profile_following_count = header_following;
                    rep.tracked_followers_count = Some(followers_res.item_count);
                    rep.tracked_following_count = Some(following_res.item_count);
                    rep.verification = Some("verification_fetch_inconsistent".to_string());
                    return Ok(rep);
                }
            }
            _ => {}
        }

        // … plus the tighter premium ladder that also catches 5–25%
        // silent truncations and following-list drops the legacy check missed.
        // This ladder NEVER blocks the sync (blocking here is what bricked
        // fresh logins at all-zeros): suspicious-but-plausible data flows
        // through as Unconfirmed so counts keep working and the UI can show
        // "needs review" instead of a false alarm or a dead screen.
        // Only evaluated when history exists — bootstrap has no baseline.
        let mut soft_notes: Vec<String> = Vec::new();
        if secondary_added_followers > 0 || secondary_added_following > 0 {
            soft_notes.push(format!(
                "web_union+{}+{}",
                secondary_added_followers, secondary_added_following
            ));
        }
        let has_history = prev_followers_snap.is_some() || prev_following_snap.is_some();
        if has_history
            && (CardinalityVerifier::needs_verification_fetch(prev_count, followers_res.item_count)
                || CardinalityVerifier::needs_verification_fetch(
                    prev_following_count,
                    following_res.item_count,
                ))
        {
            let second_followers = provider.get_followers(&account).await?;
            let second_following = provider.get_following(&account).await?;
            let tolerance = 5.max(followers_res.item_count / 50).max(following_res.item_count / 50);
            let stable_followers = second_followers.pagination_complete
                && (second_followers.item_count - followers_res.item_count).abs() <= tolerance;
            let stable_following = second_following.pagination_complete
                && (second_following.item_count - following_res.item_count).abs() <= tolerance;
            if stable_followers && stable_following {
                soft_notes.push("drop_confirmed_stable".to_string());
            } else {
                soft_notes.push("lists_unstable_unconfirmed".to_string());
            }
        }

        // Confidence: Confirmed only when the list reconciles with
        // Instagram's header AND no ladder raised a flag. Everything else is
        // Unconfirmed so the UI renders "needs review" instead of a false
        // alarm — while counts and history keep flowing.
        let header_matched = {
            let f_ok = match header_followers {
                Some(hf) if hf > 0 => CardinalityVerifier::check_list_vs_header(
                    followers_res.item_count,
                    hf,
                )
                .is_none(),
                _ => true,
            };
            let g_ok = match header_following {
                Some(hg) if hg > 0 => CardinalityVerifier::check_list_vs_header(
                    following_res.item_count,
                    hg,
                )
                .is_none(),
                _ => true,
            };
            f_ok && g_ok
        };
        if !header_matched {
            soft_notes.push("header_gap_unconfirmed".to_string());
        }
        let diff_confidence = if header_matched && soft_notes.is_empty() {
            Confidence::Confirmed
        } else {
            Confidence::Unconfirmed
        };
        let verification_label = if diff_confidence == Confidence::Confirmed {
            if has_history {
                "header_matched".to_string()
            } else {
                "bootstrap".to_string()
            }
        } else {
            soft_notes.join("+")
        };

        // 4. Resolve people into canonical people table
        let follower_people = db.upsert_people_batch(&followers_res.members, now)
            .map_err(|e| format!("Failed to upsert follower people: {}", e))?;
        let following_people = db.upsert_people_batch(&following_res.members, now)
            .map_err(|e| format!("Failed to upsert following people: {}", e))?;

        // 5. Store snapshot records
        let followers_snap_id = uuid::Uuid::new_v4().to_string();
        let followers_snap = Snapshot {
            id: followers_snap_id.clone(),
            account_id: account.id.clone(),
            snapshot_type: SnapshotType::Followers,
            started_at: now,
            completed_at: Some(now),
            status: SnapshotStatus::Complete,
            item_count: followers_res.item_count,
            source_hash: followers_res.source_hash,
            error_message: None,
            authenticated_by_account_id: account.authenticated_by_account_id.clone(),
        };
        db.insert_snapshot(&followers_snap).map_err(|e| format!("DB error: {}", e))?;
        db.insert_snapshot_members(&followers_snap_id, &follower_people).map_err(|e| format!("DB error: {}", e))?;

        let following_snap_id = uuid::Uuid::new_v4().to_string();
        let following_snap = Snapshot {
            id: following_snap_id.clone(),
            account_id: account.id.clone(),
            snapshot_type: SnapshotType::Following,
            started_at: now,
            completed_at: Some(now),
            status: SnapshotStatus::Complete,
            item_count: following_res.item_count,
            source_hash: following_res.source_hash,
            error_message: None,
            authenticated_by_account_id: account.authenticated_by_account_id.clone(),
        };
        db.insert_snapshot(&following_snap).map_err(|e| format!("DB error: {}", e))?;
        db.insert_snapshot_members(&following_snap_id, &following_people).map_err(|e| format!("DB error: {}", e))?;

        // 6. Diff with two-cycle confirmation (both Followers and Following).
        // A disappearance becomes an "unfollow" only if still gone on the
        // NEXT sync; an appearance becomes a "follow" only if still present
        // on the next sync. Single-sync list flaps (the fake-12-unfollows
        // class of bug) die in pending_events and never reach the feed.
        // Username changes are same-id facts, safe to record immediately.
        let mut total_changes = Vec::new();
        let mut pending_created: usize = 0;

        // 6a. Followers diff (FollowedYou, UnfollowedYou, UsernameChanged)
        if prev_followers_snap.is_some() {
            Self::confirm_diff(
                db,
                &account,
                SnapshotType::Followers,
                &followers_snap_id,
                now,
                &diff_confidence,
                &mut total_changes,
                &mut pending_created,
            )?;
        }

        // 6b. Following diff (YouFollowed, YouUnfollowed)
        if prev_following_snap.is_some() {
            Self::confirm_diff(
                db,
                &account,
                SnapshotType::Following,
                &following_snap_id,
                now,
                &diff_confidence,
                &mut total_changes,
                &mut pending_created,
            )?;
        }

        // Pendings older than 14 days can never confirm meaningfully; drop.
        let _ = db.purge_old_pending_events(now - 14 * 86400);

        for change in &mut total_changes {
            change.authenticated_by_account_id = account.authenticated_by_account_id.clone();
        }

        if !total_changes.is_empty() {
            db.insert_relationship_changes(&total_changes).map_err(|e| format!("DB error: {}", e))?;
        }

        // 7. Update materialized relationship_state table (upsert + prune
        // stale rows so counts can never drift upward with ghosts).
        let cur_follower_members = db.get_snapshot_members(&followers_snap_id).map_err(|e| format!("DB error: {}", e))?;
        let cur_following_members = db.get_snapshot_members(&following_snap_id).map_err(|e| format!("DB error: {}", e))?;
        let mut rel_states = DiffEngine::compute_relationship_states(
            &account.id,
            &cur_follower_members,
            &cur_following_members,
            now,
        );
        for s in &mut rel_states {
            s.authenticated_by_account_id = account.authenticated_by_account_id.clone();
        }
        db.upsert_relationship_state_batch(&account.id, &rel_states).map_err(|e| format!("DB error: {}", e))?;
        {
            let keep: Vec<String> = rel_states.iter().map(|s| s.person_id.clone()).collect();
            let _ = db.prune_relationship_state(&account.id, &keep);
        }

        // 8. Update account record stats.
        // followers_count now tracks the authoritative Instagram header
        // (what the official app shows) whenever we have one; otherwise it
        // falls back to the verified list length (legacy behaviour).
        let mut updated_account = account.clone();
        updated_account.followers_count = header_followers.unwrap_or(followers_res.item_count);
        updated_account.following_count = header_following.unwrap_or(following_res.item_count);
        updated_account.last_successful_sync_at = Some(now);
        updated_account.last_attempted_sync_at = Some(now);
        updated_account.updated_at = now;
        db.upsert_account(&updated_account).map_err(|e| format!("DB error: {}", e))?;

        let changes_count = total_changes.len();

        let message = if changes_count == 0 {
            if pending_created > 0 {
                format!("Sync complete. Verified {} followers and {} following. No confirmed changes — {} candidate(s) held for verification on the next sync.", updated_account.followers_count, updated_account.following_count, pending_created)
            } else {
                format!("Sync complete. Verified {} followers and {} following. No new changes detected.", updated_account.followers_count, updated_account.following_count)
            }
        } else if pending_created > 0 {
            format!("Sync complete. {} relationship change(s) detected and recorded. {} more awaiting confirmation.", changes_count, pending_created)
        } else {
            format!("Sync complete. {} relationship change(s) detected and recorded.", changes_count)
        };

        Ok(SyncReport {
            success: true,
            status: "complete".to_string(),
            followers_count: updated_account.followers_count,
            following_count: updated_account.following_count,
            changes_detected: changes_count,
            message,
            profile_followers_count: header_followers,
            profile_following_count: header_following,
            tracked_followers_count: Some(followers_res.item_count),
            tracked_following_count: Some(following_res.item_count),
            verification: Some(verification_label),
        })
    }

    /// Diff one snapshot type with two-cycle event confirmation.
    ///
    /// - Username changes (same id, new handle): recorded immediately.
    /// - Appearances: skipped outright if the person was present in the
    ///   snapshot before last (reappearance after a single miss, not new);
    ///   otherwise held as pending, promoted on the next sync if still
    ///   present, deleted if gone (flap resolved).
    /// - Disappearances: held as pending, promoted if still gone on the
    ///   next sync, deleted if reappeared.
    /// Promoted events carry `Confirmed` confidence: surviving two
    /// consecutive observations is the strongest signal available.
    #[allow(clippy::too_many_arguments)]
    fn confirm_diff(
        db: &Arc<Database>,
        account: &Account,
        snapshot_type: SnapshotType,
        current_snap_id: &str,
        now: i64,
        diff_confidence: &Confidence,
        out_changes: &mut Vec<RelationshipChange>,
        pending_created: &mut usize,
    ) -> Result<(), String> {
        let auth = account.authenticated_by_account_id.as_deref();
        let history = db
            .get_complete_snapshots(&account.id, snapshot_type.clone(), auth, 3)
            .map_err(|e| format!("DB error: {}", e))?;
        if history.len() < 2 {
            return Ok(()); // baseline just stored; nothing to compare yet
        }
        // history[0] is the snapshot stored this sync; [1] is previous.
        let prev = &history[1];
        let older_ids: HashSet<String> = match history.get(2) {
            Some(older) => db
                .get_snapshot_members(&older.id)
                .map_err(|e| format!("DB error: {}", e))?
                .into_iter()
                .map(|m| m.person_id)
                .collect(),
            None => HashSet::new(),
        };
        let has_older = history.len() >= 3;

        let old_members = db
            .get_snapshot_members(&prev.id)
            .map_err(|e| format!("DB error: {}", e))?;
        let current_members = db
            .get_snapshot_members(current_snap_id)
            .map_err(|e| format!("DB error: {}", e))?;

        let (follow_kind, unfollow_kind) = match snapshot_type {
            SnapshotType::Following => (ChangeType::YouFollowed, ChangeType::YouUnfollowed),
            _ => (ChangeType::FollowedYou, ChangeType::UnfollowedYou),
        };

        let old_map: HashMap<&str, &SnapshotMember> = old_members
            .iter()
            .map(|m| (m.person_id.as_str(), m))
            .collect();
        let new_map: HashMap<&str, &SnapshotMember> = current_members
            .iter()
            .map(|m| (m.person_id.as_str(), m))
            .collect();
        let old_ids: HashSet<&str> = old_map.keys().copied().collect();
        let new_ids: HashSet<&str> = new_map.keys().copied().collect();

        let mk_pending = |person_id: &str, username: String, kind: &ChangeType| PendingEvent {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: account.id.clone(),
            person_id: person_id.to_string(),
            related_username: username,
            change_type: kind.clone(),
            first_seen_at: now,
            first_snapshot_id: Some(current_snap_id.to_string()),
            authenticated_by_account_id: account.authenticated_by_account_id.clone(),
        };
        let mk_change = |pending: PendingEvent,
                         username: String,
                         kind: ChangeType|
         -> RelationshipChange {
            RelationshipChange {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account.id.clone(),
                person_id: pending.person_id,
                related_username: username,
                change_type: kind,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: pending.first_seen_at,
                before_snapshot_id: pending.first_snapshot_id,
                after_snapshot_id: Some(current_snap_id.to_string()),
                authenticated_by_account_id: account.authenticated_by_account_id.clone(),
            }
        };

        // 1. Cross-cleanup: present now => any unfollow-pending was a flap;
        //    absent now => any follow-pending was a flap.
        for pid in &new_ids {
            db.delete_pending_for_person(&account.id, pid, &unfollow_kind)
                .map_err(|e| format!("DB error: {}", e))?;
        }
        for pid in old_ids.difference(&new_ids) {
            db.delete_pending_for_person(&account.id, pid, &follow_kind)
                .map_err(|e| format!("DB error: {}", e))?;
        }

        // 1b. Promote survivors: a candidate consistent across two
        // consecutive syncs is confirmed and enters the feed exactly once.
        // (Cleanup above already removed every contradicted pending, and raw
        // transitions below can no longer collide with these.)
        for pending in db
            .get_pending_events(&account.id)
            .map_err(|e| format!("DB error: {}", e))?
        {
            if pending.change_type == follow_kind {
                if !new_ids.contains(pending.person_id.as_str()) {
                    continue;
                }
                let username = new_map
                    .get(pending.person_id.as_str())
                    .map(|m| m.username_at_snapshot.clone())
                    .unwrap_or_else(|| pending.related_username.clone());
                let promoted = mk_change(pending.clone(), username, follow_kind.clone());
                out_changes.push(promoted);
                db.delete_pending_for_person(&account.id, &pending.person_id, &follow_kind)
                    .map_err(|e| format!("DB error: {}", e))?;
            } else if pending.change_type == unfollow_kind {
                if new_ids.contains(pending.person_id.as_str()) {
                    continue;
                }
                let username = pending.related_username.clone();
                let promoted = mk_change(pending.clone(), username, unfollow_kind.clone());
                out_changes.push(promoted);
                db.delete_pending_for_person(&account.id, &pending.person_id, &unfollow_kind)
                    .map_err(|e| format!("DB error: {}", e))?;
            }
        }

        // 2. Username changes among persisting members: immediate facts.
        for common_id in old_ids.intersection(&new_ids) {
            let old_m = old_map[*common_id];
            let new_m = new_map[*common_id];
            if old_m.username_at_snapshot.to_lowercase()
                != new_m.username_at_snapshot.to_lowercase()
            {
                let metadata = serde_json::json!({
                    "old_username": old_m.username_at_snapshot,
                    "new_username": new_m.username_at_snapshot
                });
                out_changes.push(RelationshipChange {
                    id: uuid::Uuid::new_v4().to_string(),
                    account_id: account.id.clone(),
                    person_id: (*common_id).to_string(),
                    related_username: new_m.username_at_snapshot.clone(),
                    change_type: ChangeType::UsernameChanged,
                    confidence: diff_confidence.clone(),
                    metadata_json: Some(metadata.to_string()),
                    detected_at: now,
                    before_snapshot_id: Some(prev.id.clone()),
                    after_snapshot_id: Some(current_snap_id.to_string()),
                    authenticated_by_account_id: account.authenticated_by_account_id.clone(),
                });
            }
        }

        // 3. Appearances.
        for added_id in new_ids.difference(&old_ids) {
            if has_older && older_ids.contains(*added_id) {
                continue; // reappearance after a single miss, not a new follow
            }
            match db
                .take_pending_for_person(&account.id, added_id, &follow_kind)
                .map_err(|e| format!("DB error: {}", e))?
            {
                Some(pending) => {
                    let username = new_map[*added_id].username_at_snapshot.clone();
                    out_changes.push(mk_change(pending, username, follow_kind.clone()));
                }
                None => {
                    let username = new_map[*added_id].username_at_snapshot.clone();
                    db.upsert_pending_event(&mk_pending(added_id, username, &follow_kind))
                        .map_err(|e| format!("DB error: {}", e))?;
                    *pending_created += 1;
                }
            }
        }

        // 4. Disappearances.
        for removed_id in old_ids.difference(&new_ids) {
            match db
                .take_pending_for_person(&account.id, removed_id, &unfollow_kind)
                .map_err(|e| format!("DB error: {}", e))?
            {
                Some(pending) => {
                    let username = pending.related_username.clone();
                    out_changes.push(mk_change(pending, username, unfollow_kind.clone()));
                }
                None => {
                    let username = old_map[*removed_id].username_at_snapshot.clone();
                    db.upsert_pending_event(&mk_pending(removed_id, username, &unfollow_kind))
                        .map_err(|e| format!("DB error: {}", e))?;
                    *pending_created += 1;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::provider_trait::{
        AccessState, PrivacyState, ProfileResult, RelationshipFetchResult,
        TargetAccessCheckResult,
    };
    use crate::providers::FixtureProvider;

    fn test_owner(id: &str, username: &str) -> Account {
        let now = chrono::Utc::now().timestamp();
        Account {
            id: id.to_string(),
            instagram_user_id: Some("999999999".to_string()),
            username: username.to_string(),
            display_name: "Test User".to_string(),
            account_kind: AccountKind::Owner,
            provider_type: ProviderType::Session,
            avatar_url: None,
            is_private: false,
            is_verified: false,
            followers_count: 0,
            following_count: 0,
            monitoring_enabled: true,
            created_at: now,
            updated_at: now,
            last_successful_sync_at: None,
            last_attempted_sync_at: None,
            authenticated_by_account_id: None,
            access_state: "accessible".to_string(),
            access_reason: None,
            target_privacy: "public".to_string(),
            last_access_checked_at: None,
        }
    }

    /// Regression test for the reported "sync says 701/186 but dashboard
    /// shows 0" bug: a full sync must leave relationship_state populated so
    /// get_relationship_counts (what the dashboard reads) is non-zero.
    #[tokio::test]
    async fn test_sync_populates_dashboard_counts() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let acc = test_owner("acc_repro", "testuser");
        db.upsert_account(&acc).unwrap();

        let fixture = FixtureProvider::new(701, 186, 100);
        let report = SyncCoordinator::execute_sync(&db, &acc.id, &fixture)
            .await
            .unwrap();
        assert!(report.success, "sync failed: {:?}", report);
        assert_eq!(report.status, "complete");

        let (f, fg, m, _nfb, _fans) = db.get_relationship_counts(&acc.id, None).unwrap();
        assert_eq!(f, 701, "dashboard followers must be 701, got {}", f);
        assert_eq!(fg, 186, "dashboard following must be 186, got {}", fg);
        assert_eq!(m, 100, "dashboard mutual must be 100, got {}", m);

        // Account header counts must also be updated (fallback UI path).
        let fresh = db.get_account(&acc.id).unwrap().unwrap();
        assert_eq!(fresh.followers_count, 701);
        assert_eq!(fresh.following_count, 186);
        assert!(fresh.last_successful_sync_at.is_some());
    }

    /// A second sync against identical data must produce zero changes and
    /// keep counts stable (no false new/lost, no pruning wipeout).
    #[tokio::test]
    async fn test_second_identical_sync_stable_no_changes() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let acc = test_owner("acc_stable", "stableuser");
        db.upsert_account(&acc).unwrap();

        let fixture = FixtureProvider::new(701, 186, 100);
        let first = SyncCoordinator::execute_sync(&db, &acc.id, &fixture)
            .await
            .unwrap();
        assert!(first.success);

        // Reset the local throttle stamp so the immediate re-sync is allowed
        // (mirrors a user syncing again after the cooldown).
        {
            let mut a = db.get_account(&acc.id).unwrap().unwrap();
            a.last_attempted_sync_at = Some(chrono::Utc::now().timestamp() - 120);
            db.upsert_account(&a).unwrap();
        }

        let second = SyncCoordinator::execute_sync(&db, &acc.id, &fixture)
            .await
            .unwrap();
        assert!(second.success, "second sync failed: {:?}", second);
        assert_eq!(second.changes_detected, 0);

        let (f, fg, _, _, _) = db.get_relationship_counts(&acc.id, None).unwrap();
        assert_eq!(f, 701);
        assert_eq!(fg, 186);
    }

    /// Secondary-surface union: primary misses 8 members the header says
    /// exist (the 138-vs-130 target case); the web surface recovers them.
    struct GapProvider {
        header: (i64, i64),
    }

    fn gap_members(prefix: &str, from: usize, to: usize) -> Vec<RawMemberRecord> {
        (from..to)
            .map(|i| RawMemberRecord {
                instagram_user_id: Some(format!("gap_{}_{}", prefix, i)),
                username: format!("gapuser{}_{}", prefix, i),
                display_name: None,
                avatar_url: None,
                is_verified: false,
                is_private: false,
            })
            .collect()
    }

    impl InstagramProvider for GapProvider {
        async fn validate_session(&self) -> Result<ProfileResult, String> {
            self.get_profile("gapuser").await
        }
        async fn get_profile(&self, username: &str) -> Result<ProfileResult, String> {
            Ok(ProfileResult {
                instagram_user_id: Some("424242".to_string()),
                username: username.to_string(),
                display_name: "Gap User".to_string(),
                avatar_url: None,
                is_private: false,
                is_verified: false,
                followers_count: self.header.0,
                following_count: self.header.1,
            })
        }
        async fn check_target_access(
            &self,
            target_username: &str,
        ) -> Result<TargetAccessCheckResult, String> {
            Ok(TargetAccessCheckResult {
                target_user_id: Some("424242".to_string()),
                target_username: target_username.to_string(),
                avatar_url: None,
                privacy_state: PrivacyState::Public,
                access_state: AccessState::Accessible,
                access_reason: "test".to_string(),
                is_following: false,
                is_approved_follower: false,
                followers_count: self.header.0,
                following_count: self.header.1,
            })
        }
        async fn get_followers(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
            Ok(complete_result(account, gap_members("f", 0, 130)))
        }
        async fn get_following(&self, account: &Account) -> Result<RelationshipFetchResult, String> {
            Ok(complete_result(account, gap_members("g", 0, 20)))
        }
        fn name(&self) -> &'static str {
            "Test Gap Stub"
        }
        async fn get_followers_secondary(
            &self,
            _account: &Account,
        ) -> Result<Option<Vec<RawMemberRecord>>, String> {
            // Overlaps 120 of primary's 130 + 8 the primary missed.
            let mut extra = gap_members("f", 120, 138);
            // Same numeric id, different-case username: still a dup.
            extra.push(RawMemberRecord {
                instagram_user_id: Some("gap_f_5".to_string()),
                username: "GAPUSERF_5".to_string(),
                display_name: None,
                avatar_url: None,
                is_verified: false,
                is_private: false,
            });
            Ok(Some(extra))
        }
    }

    fn complete_result(account: &Account, members: Vec<RawMemberRecord>) -> RelationshipFetchResult {
        let n = members.len() as i64;
        RelationshipFetchResult {
            status: "ACCESSIBLE".to_string(),
            target_user_id: account.id.clone(),
            privacy_state: PrivacyState::Public,
            access_state: AccessState::Accessible,
            item_count: n,
            pagination_complete: true,
            completeness: "complete".to_string(),
            members,
            source_hash: "testhash".to_string(),
            provider_error: None,
        }
    }

    #[tokio::test]
    async fn test_secondary_union_closes_header_gap() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let acc = test_owner("acc_gap", "gapuser");
        db.upsert_account(&acc).unwrap();

        let provider = GapProvider { header: (138, 20) };
        let report = SyncCoordinator::execute_sync(&db, &acc.id, &provider)
            .await
            .unwrap();
        assert!(report.success, "sync failed: {:?}", report);
        assert_eq!(report.tracked_followers_count, Some(138));

        let (f, fg, _, _, _) = db.get_relationship_counts(&acc.id, None).unwrap();
        assert_eq!(f, 138, "union must recover all 138, got {}", f);
        assert_eq!(fg, 20);
        assert!(
            report.verification.as_deref().unwrap_or("").contains("web_union"),
            "verification should note the union, got {:?}",
            report.verification
        );
    }
    /// Unfollow confirmation: a shrink is held as pending on the first sync
    /// with zero feed events; still gone on the next sync it promotes to
    /// real UnfollowedYou events and ghosts are pruned from counts.
    #[tokio::test]
    async fn test_unfollow_prunes_ghosts() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let acc = test_owner("acc_prune", "pruneuser");
        db.upsert_account(&acc).unwrap();

        let big = FixtureProvider::new(701, 186, 100);
        let first = SyncCoordinator::execute_sync(&db, &acc.id, &big)
            .await
            .unwrap();
        assert!(first.success);
        assert_eq!(first.changes_detected, 0);

        {
            let mut a = db.get_account(&acc.id).unwrap().unwrap();
            a.last_attempted_sync_at = Some(chrono::Utc::now().timestamp() - 120);
            db.upsert_account(&a).unwrap();
        }

        // 101 fans gone (subset of the previous list): held, not emitted.
        let small = FixtureProvider::new(600, 186, 100);
        let second = SyncCoordinator::execute_sync(&db, &acc.id, &small)
            .await
            .unwrap();
        assert!(second.success, "second sync failed: {:?}", second);
        assert_eq!(second.changes_detected, 0);

        {
            let mut a = db.get_account(&acc.id).unwrap().unwrap();
            a.last_attempted_sync_at = Some(chrono::Utc::now().timestamp() - 120);
            db.upsert_account(&a).unwrap();
        }

        // Still gone on the next sync: promoted to 101 real events.
        let third = SyncCoordinator::execute_sync(&db, &acc.id, &small)
            .await
            .unwrap();
        assert!(third.success, "third sync failed: {:?}", third);
        assert_eq!(third.changes_detected, 101);

        let (f, fg, _, _, _) = db.get_relationship_counts(&acc.id, None).unwrap();
        assert_eq!(f, 600, "ghosts lingered: followers={}", f);
        assert_eq!(fg, 186);
    }

    /// The reported fake-events bug: a single-sync miss (present, absent,
    /// present) must NEVER reach the feed in either direction.
    #[tokio::test]
    async fn test_single_miss_flap_emits_nothing() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let acc = test_owner("acc_flap", "flapuser");
        db.upsert_account(&acc).unwrap();

        // small is a strict subset of big (same 50 mutuals, fewer fans).
        let big = FixtureProvider::new(100, 60, 50);
        let small = FixtureProvider::new(88, 60, 50);

        let s1 = SyncCoordinator::execute_sync(&db, &acc.id, &big).await.unwrap();
        assert!(s1.success);
        assert_eq!(s1.changes_detected, 0);

        // 12 missed by one flaky fetch: held silently, zero feed events.
        let s2 = SyncCoordinator::execute_sync(&db, &acc.id, &small).await.unwrap();
        assert!(s2.success, "s2 failed: {:?}", s2);
        assert_eq!(s2.changes_detected, 0);

        // They reappear: reappearance is skipped, pending flap deleted.
        let s3 = SyncCoordinator::execute_sync(&db, &acc.id, &big).await.unwrap();
        assert!(s3.success, "s3 failed: {:?}", s3);
        assert_eq!(s3.changes_detected, 0);

        // Steady state: still nothing.
        let s4 = SyncCoordinator::execute_sync(&db, &acc.id, &big).await.unwrap();
        assert!(s4.success, "s4 failed: {:?}", s4);
        assert_eq!(s4.changes_detected, 0);

        let (f, fg, _, _, _) = db.get_relationship_counts(&acc.id, None).unwrap();
        assert_eq!(f, 100);
        assert_eq!(fg, 60);
        assert!(
            db.get_pending_events(&acc.id).unwrap().is_empty(),
            "flap pendings must resolve, none may leak"
        );
    }
}
