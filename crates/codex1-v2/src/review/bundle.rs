//! Review bundle schema (`reviews/B<N>.json`).

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::graph::Dag;
use crate::state::{State, TaskStatus};

/// Compute the canonical mission-close evidence snapshot hash from
/// `(graph_revision, non-superseded task terminal truth)`. This is what
/// a mission-close reviewer is certifying: the graph shape plus every
/// non-superseded task's final status and proof. If any of those change
/// after the bundle closes, the recomputed hash will differ, and
/// `mission_close::check_readiness` marks the clean bundle stale.
///
/// `state_revision` is deliberately excluded: it bumps for reasons that
/// don't affect terminal truth (replan events, `parent_loop` toggles),
/// and including it would make the hash too fragile. The sorted triple
/// `(id, status, proof_hash)` is exactly the terminal truth.
#[must_use]
pub fn mission_close_evidence_hash(state: &State, dag: &Dag) -> String {
    use sha2::{Digest, Sha256};
    let mut ids: Vec<String> = dag.ids();
    ids.sort();
    let mut h = Sha256::new();
    h.update(format!("graph_revision={}\n", dag.graph_revision).as_bytes());
    h.update(b"tasks:\n");
    for id in ids {
        let Some(t) = state.tasks.get(&id) else {
            h.update(format!("  {id}|pre_terminal|none\n").as_bytes());
            continue;
        };
        // Superseded tasks do not contribute — the mission-close review
        // certifies the terminal surface, not the replaced history.
        if t.status == TaskStatus::Superseded {
            continue;
        }
        // ReviewClean and Complete are both "terminal" from the
        // reviewer's perspective: the latter just means `mission-close
        // complete` ran and flipped the bucket. Coalesce them in the
        // hash so idempotent completion doesn't invalidate the bundle.
        let status_str = if matches!(t.status, TaskStatus::ReviewClean | TaskStatus::Complete) {
            "terminal"
        } else {
            "pre_terminal"
        };
        let proof_str = t.proof_hash.as_deref().unwrap_or("none");
        h.update(format!("  {id}|{status_str}|{proof_str}\n").as_bytes());
    }
    format!("sha256:{:x}", h.finalize())
}

/// A single review bundle: target + requirements + evidence binding.
///
/// Wave 3 scope: task-targeted bundles only. Wave 5 adds mission-close
/// targets; Wave 3 accepts the enum shape so the schema does not break
/// when later waves extend it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewBundle {
    pub bundle_id: String,
    pub mission_id: String,
    pub graph_revision: u64,
    pub state_revision: u64,
    pub target: ReviewTarget,
    pub requirements: Vec<ReviewRequirement>,
    pub evidence_refs: Vec<String>,
    /// `sha256:<hex>` of the evidence snapshot (for task targets: the
    /// proof file hash).
    pub evidence_snapshot_hash: String,
    pub status: ReviewStatus,
    pub opened_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    /// Opener's role identifier. `review submit` refuses outputs whose
    /// `reviewer_role` equals this value (parent self-review guard).
    pub opener_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReviewTarget {
    Task {
        task_id: String,
        task_run_id: String,
    },
    Wave {
        wave_id: String,
        task_ids: Vec<String>,
    },
    MissionClose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewRequirement {
    pub id: String,
    pub profile: String,
    pub min_outputs: u32,
    pub allowed_roles: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Open,
    Clean,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::{ReviewBundle, ReviewRequirement, ReviewStatus, ReviewTarget};
    use serde_json::json;

    fn sample() -> ReviewBundle {
        ReviewBundle {
            bundle_id: "B1".into(),
            mission_id: "m".into(),
            graph_revision: 1,
            state_revision: 5,
            target: ReviewTarget::Task {
                task_id: "T1".into(),
                task_run_id: "run-abc".into(),
            },
            requirements: vec![ReviewRequirement {
                id: "B1-code".into(),
                profile: "code_bug_correctness".into(),
                min_outputs: 1,
                allowed_roles: vec!["reviewer".into()],
            }],
            evidence_refs: vec!["specs/T1/PROOF.md".into()],
            evidence_snapshot_hash: "sha256:abc".into(),
            status: ReviewStatus::Open,
            opened_at: "2026-04-18T10:00:00Z".into(),
            closed_at: None,
            opener_role: "parent".into(),
        }
    }

    #[test]
    fn bundle_round_trips() {
        let b = sample();
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(v["bundle_id"], "B1");
        assert_eq!(v["target"]["kind"], "task");
        assert_eq!(v["target"]["task_id"], "T1");
        assert_eq!(v["status"], "open");
        let parsed: ReviewBundle = serde_json::from_value(v).unwrap();
        assert_eq!(parsed, b);
    }

    #[test]
    fn mission_close_target_parses() {
        let v = json!({
            "bundle_id": "B9",
            "mission_id": "m",
            "graph_revision": 1,
            "state_revision": 10,
            "target": { "kind": "mission_close" },
            "requirements": [],
            "evidence_refs": [],
            "evidence_snapshot_hash": "sha256:x",
            "status": "open",
            "opened_at": "2026-04-18T10:00:00Z",
            "opener_role": "parent"
        });
        let b: ReviewBundle = serde_json::from_value(v).unwrap();
        assert!(matches!(b.target, ReviewTarget::MissionClose));
    }

    #[test]
    fn unknown_field_rejected() {
        let mut v = serde_json::to_value(sample()).unwrap();
        v["surprise"] = json!("no");
        assert!(serde_json::from_value::<ReviewBundle>(v).is_err());
    }

    #[test]
    fn status_values_are_snake_case() {
        for (status, expected) in [
            (ReviewStatus::Open, "open"),
            (ReviewStatus::Clean, "clean"),
            (ReviewStatus::Failed, "failed"),
        ] {
            assert_eq!(serde_json::to_value(status).unwrap(), expected);
        }
    }
}
