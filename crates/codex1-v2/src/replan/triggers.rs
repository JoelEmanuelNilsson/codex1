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
#[must_use]
pub fn detect(bundles: &[ReviewBundle]) -> TriggerReport {
    let mut per_task: BTreeMap<String, usize> = BTreeMap::new();
    for bundle in bundles {
        if let ReviewTarget::Task { task_id, .. } = &bundle.target
            && bundle.status == ReviewStatus::Failed
        {
            *per_task.entry(task_id.clone()).or_insert(0) += 1;
        }
    }
    let mandatory: Vec<MandatoryTrigger> = per_task
        .into_iter()
        .filter_map(|(task_id, count)| {
            if count >= REPLAN_AFTER_N_FAILURES {
                Some(MandatoryTrigger {
                    task_id,
                    reason: "six_consecutive_non_clean_reviews".into(),
                    consecutive_failures: count,
                })
            } else {
                None
            }
        })
        .collect();
    TriggerReport {
        mandatory,
        warnings: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::{detect, REPLAN_AFTER_N_FAILURES};
    use crate::review::bundle::{ReviewBundle, ReviewStatus, ReviewTarget};

    fn bundle(task_id: &str, status: ReviewStatus) -> ReviewBundle {
        ReviewBundle {
            bundle_id: "B".into(),
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
            opened_at: "t".into(),
            closed_at: None,
            opener_role: "parent".into(),
        }
    }

    #[test]
    fn no_failures_no_triggers() {
        let r = detect(&[bundle("T1", ReviewStatus::Clean)]);
        assert!(r.mandatory.is_empty());
    }

    #[test]
    fn fewer_than_threshold_no_triggers() {
        let mut bundles = Vec::new();
        for _ in 0..(REPLAN_AFTER_N_FAILURES - 1) {
            bundles.push(bundle("T1", ReviewStatus::Failed));
        }
        let r = detect(&bundles);
        assert!(r.mandatory.is_empty());
    }

    #[test]
    fn exactly_threshold_triggers() {
        let bundles: Vec<_> = (0..REPLAN_AFTER_N_FAILURES)
            .map(|_| bundle("T1", ReviewStatus::Failed))
            .collect();
        let r = detect(&bundles);
        assert_eq!(r.mandatory.len(), 1);
        assert_eq!(r.mandatory[0].task_id, "T1");
        assert_eq!(r.mandatory[0].consecutive_failures, REPLAN_AFTER_N_FAILURES);
    }

    #[test]
    fn per_task_independent() {
        let mut bundles = Vec::new();
        for _ in 0..REPLAN_AFTER_N_FAILURES {
            bundles.push(bundle("T1", ReviewStatus::Failed));
        }
        bundles.push(bundle("T2", ReviewStatus::Failed));
        let r = detect(&bundles);
        assert_eq!(r.mandatory.len(), 1);
        assert_eq!(r.mandatory[0].task_id, "T1");
    }
}
