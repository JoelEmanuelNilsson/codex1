//! Mandatory-replan trigger detection.
//!
//! Wave 3 implements the six-consecutive-non-clean-review counter (per
//! task). Other mandatory triggers (write scope expansion, interface
//! contract change, etc.) are not fully machine-checkable in Wave 3; the
//! CLI exposes `codex1 replan check` which today returns the mechanical
//! triggers and leaves narrative triggers as a warning list.

#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Serialize;

use crate::review::bundle::{ReviewBundle, ReviewStatus, ReviewTarget};

/// Number of non-clean review closures on the same task that triggers a
/// mandatory replan (per V2 contract).
pub const REPLAN_AFTER_N_FAILURES: usize = 6;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TriggerReport {
    pub mandatory: Vec<MandatoryTrigger>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MandatoryTrigger {
    pub task_id: String,
    pub reason: String,
    pub consecutive_failures: usize,
}

/// Detect mandatory replan triggers from the bundles collected so far.
///
/// Round 12 P1: the trigger is "six *consecutive* dirty reviews on the
/// same task," not "six total dirty reviews ever." A Clean review
/// resets the streak for that task. The previous implementation
/// incremented a monotonic counter and fired whenever total failures
/// ≥ 6, which could force replanning after an already-successful
/// repair path (e.g., fail×3, clean, fail×3 incorrectly triggered).
#[must_use]
pub fn detect(bundles: &[ReviewBundle]) -> TriggerReport {
    // Group Clean + Failed bundles per task; Open bundles are still
    // in-flight and neither extend nor reset the streak.
    let mut per_task: BTreeMap<String, Vec<&ReviewBundle>> = BTreeMap::new();
    for bundle in bundles {
        if let ReviewTarget::Task { task_id, .. } = &bundle.target
            && matches!(bundle.status, ReviewStatus::Clean | ReviewStatus::Failed)
        {
            per_task.entry(task_id.clone()).or_default().push(bundle);
        }
    }

    let mut mandatory: Vec<MandatoryTrigger> = Vec::new();
    for (task_id, mut bundles) in per_task {
        // Order by closed_at so the streak reflects review timeline,
        // not bundle-id order or filesystem enumeration order.
        // closed_at is an RFC-3339 string; None sorts first but should
        // not occur for Clean/Failed bundles in practice.
        bundles.sort_by(|a, b| a.closed_at.cmp(&b.closed_at));
        let mut streak: usize = 0;
        let mut max_streak: usize = 0;
        for b in bundles {
            match b.status {
                ReviewStatus::Failed => {
                    streak += 1;
                    if streak > max_streak {
                        max_streak = streak;
                    }
                }
                ReviewStatus::Clean => {
                    streak = 0;
                }
                ReviewStatus::Open => {}
            }
        }
        if max_streak >= REPLAN_AFTER_N_FAILURES {
            mandatory.push(MandatoryTrigger {
                task_id,
                reason: "six_consecutive_non_clean_reviews".into(),
                consecutive_failures: max_streak,
            });
        }
    }
    TriggerReport {
        mandatory,
        warnings: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::{REPLAN_AFTER_N_FAILURES, detect};
    use crate::review::bundle::{ReviewBundle, ReviewStatus, ReviewTarget};

    /// Build a closed bundle with an explicit `closed_at` so the
    /// streak math can order reviews along the real timeline.
    fn bundle_at(task_id: &str, status: ReviewStatus, closed_at: &str) -> ReviewBundle {
        ReviewBundle {
            bundle_id: format!("B-{closed_at}"),
            mission_id: "m".into(),
            graph_revision: 1,
            state_revision: 1,
            target: ReviewTarget::Task {
                task_id: task_id.into(),
                task_run_id: "r".into(),
            },
            requirements: vec![],
            evidence_refs: vec![],
            evidence_snapshot_hash: "x".into(),
            status,
            opened_at: closed_at.into(),
            closed_at: Some(closed_at.into()),
            opener_role: "parent".into(),
        }
    }

    fn ts(i: usize) -> String {
        // RFC-3339-like sortable timestamps; seconds precision is enough.
        format!("2026-04-19T00:00:{i:02}Z")
    }

    #[test]
    fn no_failures_no_triggers() {
        let r = detect(&[bundle_at("T1", ReviewStatus::Clean, &ts(1))]);
        assert!(r.mandatory.is_empty());
    }

    #[test]
    fn fewer_than_threshold_no_triggers() {
        let bundles: Vec<_> = (0..(REPLAN_AFTER_N_FAILURES - 1))
            .map(|i| bundle_at("T1", ReviewStatus::Failed, &ts(i)))
            .collect();
        let r = detect(&bundles);
        assert!(r.mandatory.is_empty());
    }

    #[test]
    fn exactly_threshold_triggers() {
        let bundles: Vec<_> = (0..REPLAN_AFTER_N_FAILURES)
            .map(|i| bundle_at("T1", ReviewStatus::Failed, &ts(i)))
            .collect();
        let r = detect(&bundles);
        assert_eq!(r.mandatory.len(), 1);
        assert_eq!(r.mandatory[0].task_id, "T1");
        assert_eq!(r.mandatory[0].consecutive_failures, REPLAN_AFTER_N_FAILURES);
    }

    #[test]
    fn per_task_independent() {
        let mut bundles: Vec<_> = (0..REPLAN_AFTER_N_FAILURES)
            .map(|i| bundle_at("T1", ReviewStatus::Failed, &ts(i)))
            .collect();
        bundles.push(bundle_at("T2", ReviewStatus::Failed, &ts(99)));
        let r = detect(&bundles);
        assert_eq!(r.mandatory.len(), 1);
        assert_eq!(r.mandatory[0].task_id, "T1");
    }

    #[test]
    fn clean_review_resets_streak() {
        // Round 12 P1: fail×3, clean, fail×3 must NOT trigger.
        // The previous counter-based detector treated this as 6 total
        // failures and triggered mandatory replan after a repair path
        // had already succeeded once.
        let mut bundles = Vec::new();
        for i in 0..3 {
            bundles.push(bundle_at("T1", ReviewStatus::Failed, &ts(i)));
        }
        bundles.push(bundle_at("T1", ReviewStatus::Clean, &ts(3)));
        for i in 4..7 {
            bundles.push(bundle_at("T1", ReviewStatus::Failed, &ts(i)));
        }
        let r = detect(&bundles);
        assert!(
            r.mandatory.is_empty(),
            "fail×3, clean, fail×3 must not trigger replan; got {:?}",
            r.mandatory
        );
    }

    #[test]
    fn streak_spans_clean_interruption_only_on_tail() {
        // fail×6 after a long history of earlier cleans still triggers
        // — the tail streak reaches six.
        let mut bundles = Vec::new();
        bundles.push(bundle_at("T1", ReviewStatus::Failed, &ts(0)));
        bundles.push(bundle_at("T1", ReviewStatus::Clean, &ts(1)));
        for i in 2..8 {
            bundles.push(bundle_at("T1", ReviewStatus::Failed, &ts(i)));
        }
        let r = detect(&bundles);
        assert_eq!(r.mandatory.len(), 1);
        assert_eq!(r.mandatory[0].consecutive_failures, 6);
    }

    #[test]
    fn closed_at_ordering_drives_streak() {
        // Array order can be arbitrary (filesystem enumeration,
        // re-hydration). Streak math must sort by closed_at so the
        // timeline is authoritative, not vec insertion order.
        //
        // Scenario: timeline is fail, fail, fail, fail, fail, fail, clean
        // (max streak = 6, triggers). Vec order scrambles one Failed
        // to the front so an unsorted walk would see:
        //   fail(streak=1), clean(streak=0), fail×5(streak=5) → max=5
        // which would NOT trigger. With timestamp sort the walk matches
        // the timeline and does trigger.
        let bundles = vec![
            bundle_at("T1", ReviewStatus::Failed, &ts(6)),
            bundle_at("T1", ReviewStatus::Clean, &ts(7)),
            bundle_at("T1", ReviewStatus::Failed, &ts(1)),
            bundle_at("T1", ReviewStatus::Failed, &ts(2)),
            bundle_at("T1", ReviewStatus::Failed, &ts(3)),
            bundle_at("T1", ReviewStatus::Failed, &ts(4)),
            bundle_at("T1", ReviewStatus::Failed, &ts(5)),
        ];
        let r = detect(&bundles);
        assert_eq!(
            r.mandatory.len(),
            1,
            "timestamp sort should reveal fail×6 streak even though vec order scrambles them"
        );
        assert_eq!(r.mandatory[0].consecutive_failures, 6);
    }
}
