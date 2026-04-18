//! Review bundle schema (`reviews/B<N>.json`).

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

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
