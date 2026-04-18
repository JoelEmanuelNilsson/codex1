//! Review cleanliness computation + staleness quarantine +
//! parent-self-review refusal.
//!
//! Given a bundle, its reviewer outputs, and a snapshot of current truth,
//! [`compute_cleanliness`] classifies the bundle as clean, dirty (with the
//! reasons), or structurally invalid.

#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Serialize;

use crate::binding::{check_staleness, CurrentSnapshot, OutputBinding};

use super::bundle::ReviewBundle;
use super::output::{FindingSeverity, ReviewerOutput, ReviewerResultKind};

/// Cleanliness verdict for a bundle.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CleanlinessVerdict {
    pub clean: bool,
    pub missing_profiles: Vec<String>,
    pub blocking_findings: u32,
    pub stale_outputs: Vec<String>,
    pub self_review_refused: Vec<String>,
    pub accepted_outputs: Vec<String>,
}

/// Current-truth snapshot for cleanliness checks.
#[derive(Debug, Clone)]
pub struct CurrentTruth<'a> {
    pub graph_revision: u64,
    pub state_revision: u64,
    /// Proof (or general evidence) hash the bundle is reviewing.
    pub evidence_snapshot_hash: &'a str,
    /// Task run id if the bundle targets a task.
    pub task_run_id: Option<&'a str>,
    /// Task id if the bundle targets a task.
    pub task_id: Option<&'a str>,
}

/// Classify the bundle.
#[must_use]
pub fn compute_cleanliness(
    bundle: &ReviewBundle,
    outputs: &[ReviewerOutput],
    current: &CurrentTruth<'_>,
) -> CleanlinessVerdict {
    let mut accepted: Vec<&ReviewerOutput> = Vec::with_capacity(outputs.len());
    let mut stale_outputs: Vec<String> = Vec::new();
    let mut self_review_refused: Vec<String> = Vec::new();

    for out in outputs {
        if out.bundle_id != bundle.bundle_id {
            // A caller error at a prior layer; skip silently.
            continue;
        }
        // Parent self-review guard.
        if out.reviewer_role == bundle.opener_role {
            self_review_refused.push(out.packet_id.clone());
            continue;
        }
        // Requirement must exist in bundle.
        let req = bundle
            .requirements
            .iter()
            .find(|r| r.id == out.requirement_id);
        let Some(req) = req else {
            stale_outputs.push(out.packet_id.clone());
            continue;
        };
        // Role must be allowed.
        if !req.allowed_roles.contains(&out.reviewer_role) {
            self_review_refused.push(out.packet_id.clone());
            continue;
        }
        // Staleness check.
        let binding = OutputBinding {
            task_id: out.task_id.clone(),
            task_run_id: out.task_run_id.clone(),
            bundle_id: Some(out.bundle_id.clone()),
            graph_revision: out.graph_revision,
            state_revision: out.state_revision,
            evidence_snapshot_hash: out.evidence_snapshot_hash.clone(),
            packet_id: out.packet_id.clone(),
        };
        let snapshot = CurrentSnapshot {
            task_id: current.task_id,
            task_run_id: current.task_run_id,
            bundle_id: Some(bundle.bundle_id.as_str()),
            graph_revision: current.graph_revision,
            state_revision: current.state_revision,
            evidence_snapshot_hash: Some(current.evidence_snapshot_hash),
        };
        if check_staleness(&binding, &snapshot).is_err() {
            stale_outputs.push(out.packet_id.clone());
            continue;
        }
        accepted.push(out);
    }

    // Group accepted by requirement and count.
    let mut per_req: BTreeMap<String, u32> = BTreeMap::new();
    let mut blocking = 0_u32;
    for out in &accepted {
        *per_req.entry(out.requirement_id.clone()).or_insert(0) += 1;
        if out.result == ReviewerResultKind::Findings {
            for f in &out.findings {
                if f.severity.blocks_clean() {
                    blocking += 1;
                }
            }
        }
    }

    let missing: Vec<String> = bundle
        .requirements
        .iter()
        .filter_map(|r| {
            let count = per_req.get(&r.id).copied().unwrap_or(0);
            if count < r.min_outputs {
                Some(r.profile.clone())
            } else {
                None
            }
        })
        .collect();

    let clean = missing.is_empty() && blocking == 0;

    let mut accepted_ids: Vec<String> =
        accepted.iter().map(|o| o.packet_id.clone()).collect();
    accepted_ids.sort();
    stale_outputs.sort();
    self_review_refused.sort();

    CleanlinessVerdict {
        clean,
        missing_profiles: missing,
        blocking_findings: blocking,
        stale_outputs,
        self_review_refused,
        accepted_outputs: accepted_ids,
    }
}

/// Count blocking findings across a slice — helpful for callers that
/// compute cleanliness piecewise.
#[must_use]
pub fn count_blocking_findings(out: &ReviewerOutput) -> u32 {
    if out.result != ReviewerResultKind::Findings {
        return 0;
    }
    u32::try_from(
        out.findings
            .iter()
            .filter(|f| f.severity.blocks_clean())
            .count(),
    )
    .unwrap_or(u32::MAX)
}

/// Convenience: does this severity set contain at least one blocker?
#[must_use]
pub fn any_blocking(severities: &[FindingSeverity]) -> bool {
    severities.iter().any(|s| s.blocks_clean())
}

#[cfg(test)]
mod tests {
    use super::{compute_cleanliness, CurrentTruth};
    use crate::review::bundle::{ReviewBundle, ReviewRequirement, ReviewStatus, ReviewTarget};
    use crate::review::output::{Finding, FindingSeverity, ReviewerOutput, ReviewerResultKind};

    fn bundle() -> ReviewBundle {
        ReviewBundle {
            bundle_id: "B1".into(),
            mission_id: "m".into(),
            graph_revision: 1,
            state_revision: 5,
            target: ReviewTarget::Task {
                task_id: "T1".into(),
                task_run_id: "run-a".into(),
            },
            requirements: vec![ReviewRequirement {
                id: "B1-code".into(),
                profile: "code_bug_correctness".into(),
                min_outputs: 1,
                allowed_roles: vec!["reviewer".into()],
            }],
            evidence_refs: vec![],
            evidence_snapshot_hash: "sha256:abc".into(),
            status: ReviewStatus::Open,
            opened_at: "t".into(),
            closed_at: None,
            opener_role: "parent".into(),
        }
    }

    fn truth<'a>() -> CurrentTruth<'a> {
        CurrentTruth {
            graph_revision: 1,
            state_revision: 5,
            evidence_snapshot_hash: "sha256:abc",
            task_run_id: Some("run-a"),
            task_id: Some("T1"),
        }
    }

    fn clean_output() -> ReviewerOutput {
        ReviewerOutput {
            bundle_id: "B1".into(),
            reviewer_id: "R1".into(),
            reviewer_role: "reviewer".into(),
            requirement_id: "B1-code".into(),
            profile: "code_bug_correctness".into(),
            task_id: Some("T1".into()),
            task_run_id: Some("run-a".into()),
            graph_revision: 1,
            state_revision: 5,
            evidence_snapshot_hash: "sha256:abc".into(),
            packet_id: "pkt-1".into(),
            result: ReviewerResultKind::None,
            findings: vec![],
            produced_at: "t".into(),
        }
    }

    #[test]
    fn single_clean_output_is_clean() {
        let v = compute_cleanliness(&bundle(), &[clean_output()], &truth());
        assert!(v.clean);
        assert!(v.missing_profiles.is_empty());
        assert_eq!(v.blocking_findings, 0);
        assert!(v.stale_outputs.is_empty());
        assert!(v.self_review_refused.is_empty());
        assert_eq!(v.accepted_outputs, vec!["pkt-1".to_string()]);
    }

    #[test]
    fn missing_required_output_is_not_clean() {
        let v = compute_cleanliness(&bundle(), &[], &truth());
        assert!(!v.clean);
        assert!(v
            .missing_profiles
            .iter()
            .any(|p| p == "code_bug_correctness"));
    }

    #[test]
    fn p1_finding_blocks_clean() {
        let mut out = clean_output();
        out.result = ReviewerResultKind::Findings;
        out.findings = vec![Finding {
            severity: FindingSeverity::P1,
            title: "bug".into(),
            evidence_refs: vec![],
            rationale: "r".into(),
            suggested_next_action: None,
        }];
        let v = compute_cleanliness(&bundle(), &[out], &truth());
        assert!(!v.clean);
        assert_eq!(v.blocking_findings, 1);
    }

    #[test]
    fn p3_finding_does_not_block() {
        let mut out = clean_output();
        out.result = ReviewerResultKind::Findings;
        out.findings = vec![Finding {
            severity: FindingSeverity::P3,
            title: "nit".into(),
            evidence_refs: vec![],
            rationale: "r".into(),
            suggested_next_action: None,
        }];
        let v = compute_cleanliness(&bundle(), &[out], &truth());
        assert!(v.clean);
        assert_eq!(v.blocking_findings, 0);
    }

    #[test]
    fn stale_task_run_id_is_quarantined() {
        let mut out = clean_output();
        out.task_run_id = Some("run-STALE".into());
        let v = compute_cleanliness(&bundle(), &[out], &truth());
        assert!(!v.clean);
        assert_eq!(v.stale_outputs, vec!["pkt-1".to_string()]);
    }

    #[test]
    fn stale_evidence_hash_is_quarantined() {
        let mut out = clean_output();
        out.evidence_snapshot_hash = "sha256:OLD".into();
        let v = compute_cleanliness(&bundle(), &[out], &truth());
        assert!(!v.clean);
        assert_eq!(v.stale_outputs, vec!["pkt-1".to_string()]);
    }

    #[test]
    fn parent_self_review_is_refused() {
        let mut out = clean_output();
        out.reviewer_role = "parent".into(); // same as bundle.opener_role
        let v = compute_cleanliness(&bundle(), &[out], &truth());
        assert!(!v.clean);
        assert_eq!(v.self_review_refused, vec!["pkt-1".to_string()]);
        assert!(v.accepted_outputs.is_empty());
    }

    #[test]
    fn disallowed_role_refused() {
        let mut out = clean_output();
        out.reviewer_role = "advisor".into(); // not in allowed_roles
        let v = compute_cleanliness(&bundle(), &[out], &truth());
        assert!(!v.clean);
        assert_eq!(v.self_review_refused, vec!["pkt-1".to_string()]);
    }

    #[test]
    fn multiple_outputs_satisfy_min_outputs() {
        let mut b = bundle();
        b.requirements[0].min_outputs = 2;
        let out_a = clean_output();
        let mut out_b = clean_output();
        out_b.reviewer_id = "R2".into();
        out_b.packet_id = "pkt-2".into();
        let v = compute_cleanliness(&b, &[out_a.clone(), out_b.clone()], &truth());
        assert!(v.clean);
        let _ = (out_a, out_b);
    }

    #[test]
    fn insufficient_outputs_not_clean() {
        let mut b = bundle();
        b.requirements[0].min_outputs = 2;
        let v = compute_cleanliness(&b, &[clean_output()], &truth());
        assert!(!v.clean);
    }
}
