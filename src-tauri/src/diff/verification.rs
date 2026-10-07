use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationDecision {
    Accepted,
    RequiresVerificationFetch {
        reason: String,
        previous_count: i64,
        observed_count: i64,
    },
    VerifiedLegitimateDrop {
        previous_count: i64,
        confirmed_count: i64,
    },
    Incomplete {
        reason: String,
    },
}

pub struct CardinalityVerifier;

impl CardinalityVerifier {
    /// Tolerance for list-length vs Instagram header-count reconciliation.
    /// Instagram header counts legitimately include deactivated/spam/restricted
    /// accounts that never appear in the relationship list, so a small gap is
    /// normal. Anything beyond this is treated as silent truncation.
    pub const HEADER_TOLERANCE_PCT: i64 = 8;
    pub const HEADER_TOLERANCE_ABS: i64 = 15;
    /// Drops at or above this size always trigger the verification ladder
    /// (legacy threshold was 25% — too permissive, caused false unfollows).
    pub const SMALL_DROP_PCT: i64 = 5;
    pub const SMALL_DROP_ABS: i64 = 10;

    /// Evaluates whether a snapshot should be accepted, requires a verification fetch, or is rejected as incomplete.
    pub fn evaluate(
        previous_count: i64,
        observed_count: i64,
        pagination_exhausted: bool,
        is_verification_run: bool,
        previous_verification_count: Option<i64>,
    ) -> VerificationDecision {
        if !pagination_exhausted {
            return VerificationDecision::Incomplete {
                reason: "Pagination did not exhaust cleanly or was interrupted.".to_string(),
            };
        }

        // If there was no previous count (e.g. baseline initial sync), accept cleanly
        if previous_count <= 0 {
            return VerificationDecision::Accepted;
        }

        // Check for sudden suspicious cardinality drop (drop greater than 25%)
        let drop_threshold = (previous_count * 75) / 100;
        let is_suspicious_drop = observed_count < drop_threshold;

        if is_suspicious_drop {
            if is_verification_run {
                // If this is the second verification fetch and the counts match (within 2%), accept as legitimate drop
                if let Some(prev_verify) = previous_verification_count {
                    let diff = (observed_count - prev_verify).abs();
                    if diff <= 5 || diff * 100 / observed_count.max(1) <= 2 {
                        return VerificationDecision::VerifiedLegitimateDrop {
                            previous_count,
                            confirmed_count: observed_count,
                        };
                    }
                }
                return VerificationDecision::Incomplete {
                    reason: format!(
                        "Verification fetch yielded inconsistent counts ({} vs {:?}). Data quarantined to prevent false unfollows.",
                        observed_count, previous_verification_count
                    ),
                };
            } else {
                return VerificationDecision::RequiresVerificationFetch {
                    reason: format!(
                        "Follower count dropped from {} to {} (>25% decrease). Triggering verification ladder.",
                        previous_count, observed_count
                    ),
                    previous_count,
                    observed_count,
                };
            }
        }

        VerificationDecision::Accepted
    }

    /// Premium gate: compare paginated list length against Instagram's
    /// authoritative profile header count.
    /// Returns `None` when the list is plausibly complete, or `Some(reason)`
    /// when it must be quarantined (no diff may be emitted).
    pub fn check_list_vs_header(list_count: i64, header_count: i64) -> Option<String> {
        if header_count <= 0 {
            return None; // header unavailable — fall back to legacy checks
        }
        if list_count == 0 && header_count > 0 {
            return Some(format!(
                "Relationship list came back empty but Instagram reports {} (access likely revoked or throttled). Data quarantined to prevent false unfollows.",
                header_count
            ));
        }
        let gap = (header_count - list_count).abs();
        let pct = gap * 100 / header_count.max(1);
        if list_count > header_count {
            // Lists can legitimately exceed a slightly stale header after a
            // follow burst; only quarantine large overshoots.
            if pct > Self::HEADER_TOLERANCE_PCT && gap > Self::HEADER_TOLERANCE_ABS {
                return Some(format!(
                    "Follower list ({}) exceeds Instagram header count ({}) by {}% — pagination anomaly. Data quarantined.",
                    list_count, header_count, pct
                ));
            }
            return None;
        }
        if pct > Self::HEADER_TOLERANCE_PCT && gap > Self::HEADER_TOLERANCE_ABS {
            return Some(format!(
                "Follower list ({} tracked) covers only {}% of Instagram header count {} — likely truncated/throttled. Data quarantined to prevent false unfollows.",
                list_count,
                100 - pct,
                header_count
            ));
        }
        None
    }

    /// HARD gate: returns `Some(reason)` only for certain corruption that
    /// must never be diffed (empty list vs known history, or a catastrophic
    /// shortfall). Moderate gaps return `None` here — they are handled with
    /// `Unconfirmed` confidence instead of blocking the sync, because a
    /// hard block on a systematic-but-benign gap bricks the app forever
    /// (no snapshot can ever be stored, counts stay 0).
    pub fn check_list_vs_header_hard(
        list_count: i64,
        header_count: i64,
        previous_count: i64,
    ) -> Option<String> {
        if header_count <= 0 {
            // Header unavailable — fall back to history comparison.
            if list_count == 0 && previous_count > 10 {
                return Some(format!(
                    "Relationship list came back empty but the last verified snapshot had {} members. Data quarantined to prevent false unfollows.",
                    previous_count
                ));
            }
            return None;
        }
        if list_count == 0 {
            return Some(format!(
                "Relationship list came back empty but Instagram reports {} (access likely revoked or throttled). Data quarantined to prevent false unfollows.",
                header_count
            ));
        }
        // Catastrophic shortfall only (>50% AND >50 accounts). Anything less
        // flows through as Unconfirmed rather than bricking the sync.
        const HARD_PCT: i64 = 50;
        const HARD_ABS: i64 = 50;
        let gap = (header_count - list_count).abs();
        let pct = gap * 100 / header_count.max(1);
        if pct > HARD_PCT && gap > HARD_ABS {
            if list_count < header_count {
                return Some(format!(
                    "Follower list ({} tracked) covers only {}% of Instagram header count {} — severe truncation. Data quarantined to prevent false unfollows.",
                    list_count,
                    100 - pct,
                    header_count
                ));
            }
            return Some(format!(
                "Follower list ({}) exceeds Instagram header count ({}) by {}% — pagination anomaly. Data quarantined.",
                list_count, header_count, pct
            ));
        }
        None
    }

    /// Returns true when a drop is big enough to warrant a verification fetch.
    /// Catches the 5–25% silent truncations the legacy 25% check missed.
    pub fn needs_verification_fetch(previous_count: i64, observed_count: i64) -> bool {
        if previous_count <= 0 || observed_count >= previous_count {
            return false;
        }
        let drop = previous_count - observed_count;
        let pct = drop * 100 / previous_count.max(1);
        pct >= Self::SMALL_DROP_PCT && drop >= Self::SMALL_DROP_ABS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_sync_accepted() {
        let decision = CardinalityVerifier::evaluate(1000, 995, true, false, None);
        assert_eq!(decision, VerificationDecision::Accepted);
    }

    #[test]
    fn test_incomplete_pagination_rejected() {
        let decision = CardinalityVerifier::evaluate(1000, 995, false, false, None);
        match decision {
            VerificationDecision::Incomplete { .. } => {}
            _ => panic!("Expected Incomplete decision"),
        }
    }

    #[test]
    fn test_suspicious_drop_triggers_verification() {
        let decision = CardinalityVerifier::evaluate(1000, 600, true, false, None);
        match decision {
            VerificationDecision::RequiresVerificationFetch { previous_count, observed_count, .. } => {
                assert_eq!(previous_count, 1000);
                assert_eq!(observed_count, 600);
            }
            _ => panic!("Expected RequiresVerificationFetch"),
        }
    }

    #[test]
    fn test_verification_run_confirms_legitimate_drop() {
        let decision = CardinalityVerifier::evaluate(1000, 600, true, true, Some(600));
        match decision {
            VerificationDecision::VerifiedLegitimateDrop { previous_count, confirmed_count } => {
                assert_eq!(previous_count, 1000);
                assert_eq!(confirmed_count, 600);
            }
            _ => panic!("Expected VerifiedLegitimateDrop"),
        }
    }

    #[test]
    fn test_header_reconciliation_accepts_small_gap() {
        // 3% gap (spam/deactivated accounts) is normal.
        assert!(CardinalityVerifier::check_list_vs_header(970, 1000).is_none());
    }

    #[test]
    fn test_header_reconciliation_quarantines_truncation() {
        // 15% shortfall must be quarantined, not diffed.
        assert!(CardinalityVerifier::check_list_vs_header(850, 1000).is_some());
    }

    #[test]
    fn test_header_reconciliation_quarantines_empty_list() {
        assert!(CardinalityVerifier::check_list_vs_header(0, 500).is_some());
    }

    #[test]
    fn test_small_drop_needs_verification() {
        assert!(CardinalityVerifier::needs_verification_fetch(1000, 900));
        assert!(!CardinalityVerifier::needs_verification_fetch(1000, 995));
        assert!(!CardinalityVerifier::needs_verification_fetch(1000, 1010));
    }

    #[test]
    fn test_hard_gate_quarantines_empty_and_catastrophic() {
        // Empty list vs known history/header: always blocked.
        assert!(CardinalityVerifier::check_list_vs_header_hard(0, 500, 0).is_some());
        assert!(CardinalityVerifier::check_list_vs_header_hard(0, 0, 500).is_some());
        // Catastrophic 60% shortfall: blocked.
        assert!(CardinalityVerifier::check_list_vs_header_hard(400, 1000, 1000).is_some());
    }

    #[test]
    fn test_hard_gate_passes_moderate_gap_and_bootstrap() {
        // Moderate 15% gap: must NOT block (flows as Unconfirmed).
        assert!(CardinalityVerifier::check_list_vs_header_hard(850, 1000, 1000).is_none());
        // Bootstrap (no history, no header): never blocked.
        assert!(CardinalityVerifier::check_list_vs_header_hard(850, 0, 0).is_none());
        assert!(CardinalityVerifier::check_list_vs_header_hard(0, 0, 0).is_none());
        // Small account, empty list, tiny history: not blocked.
        assert!(CardinalityVerifier::check_list_vs_header_hard(0, 0, 5).is_none());
    }
}
