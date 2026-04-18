//! Reviewer output schema (`reviews/outputs/R<N>.json`).

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewerOutput {
    pub bundle_id: String,
    pub reviewer_id: String,
    /// Identifier of the reviewer role (e.g. `reviewer`, `advisor`). The
    /// review bundle refuses submission when this equals the bundle's
    /// `opener_role` (parent self-review guard).
    pub reviewer_role: String,
    pub requirement_id: String,
    pub profile: String,

    // Binding fields (mirror `binding::OutputBinding`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_run_id: Option<String>,
    pub graph_revision: u64,
    pub state_revision: u64,
    pub evidence_snapshot_hash: String,
    pub packet_id: String,

    pub result: ReviewerResultKind,
    #[serde(default)]
    pub findings: Vec<Finding>,

    /// RFC-3339 timestamp when the reviewer wrote the output.
    pub produced_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerResultKind {
    None,
    Findings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub severity: FindingSeverity,
    pub title: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_next_action: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum FindingSeverity {
    P0,
    P1,
    P2,
    P3,
}

impl FindingSeverity {
    /// P0/P1/P2 block a clean review. P3 is recorded without blocking.
    #[must_use]
    pub fn blocks_clean(self) -> bool {
        matches!(self, Self::P0 | Self::P1 | Self::P2)
    }
}

#[cfg(test)]
mod tests {
    use super::{Finding, FindingSeverity, ReviewerOutput, ReviewerResultKind};
    use serde_json::json;

    #[test]
    fn clean_output_round_trips() {
        let out = ReviewerOutput {
            bundle_id: "B1".into(),
            reviewer_id: "R1".into(),
            reviewer_role: "reviewer".into(),
            requirement_id: "B1-code".into(),
            profile: "code_bug_correctness".into(),
            task_id: Some("T1".into()),
            task_run_id: Some("run-abc".into()),
            graph_revision: 1,
            state_revision: 5,
            evidence_snapshot_hash: "sha256:abc".into(),
            packet_id: "pkt-1".into(),
            result: ReviewerResultKind::None,
            findings: vec![],
            produced_at: "2026-04-18T10:00:00Z".into(),
        };
        let v = serde_json::to_value(&out).unwrap();
        assert_eq!(v["result"], "none");
        let parsed: ReviewerOutput = serde_json::from_value(v).unwrap();
        assert_eq!(parsed, out);
    }

    #[test]
    fn findings_output_round_trips() {
        let out = ReviewerOutput {
            bundle_id: "B1".into(),
            reviewer_id: "R2".into(),
            reviewer_role: "reviewer".into(),
            requirement_id: "B1-intent".into(),
            profile: "local_spec_intent".into(),
            task_id: Some("T1".into()),
            task_run_id: Some("run-abc".into()),
            graph_revision: 1,
            state_revision: 5,
            evidence_snapshot_hash: "sha256:abc".into(),
            packet_id: "pkt-2".into(),
            result: ReviewerResultKind::Findings,
            findings: vec![Finding {
                severity: FindingSeverity::P1,
                title: "Proof gap".into(),
                evidence_refs: vec!["specs/T1/PROOF.md:12".into()],
                rationale: "Checks help but not behaviour.".into(),
                suggested_next_action: Some("Add a behaviour test.".into()),
            }],
            produced_at: "2026-04-18T10:00:00Z".into(),
        };
        let v = serde_json::to_value(&out).unwrap();
        assert_eq!(v["findings"][0]["severity"], "P1");
        let parsed: ReviewerOutput = serde_json::from_value(v).unwrap();
        assert_eq!(parsed, out);
    }

    #[test]
    fn severity_blocks_clean_only_p0_p1_p2() {
        assert!(FindingSeverity::P0.blocks_clean());
        assert!(FindingSeverity::P1.blocks_clean());
        assert!(FindingSeverity::P2.blocks_clean());
        assert!(!FindingSeverity::P3.blocks_clean());
    }

    #[test]
    fn unknown_fields_rejected() {
        let v = json!({
            "bundle_id": "B1",
            "reviewer_id": "R1",
            "reviewer_role": "reviewer",
            "requirement_id": "r",
            "profile": "p",
            "graph_revision": 1,
            "state_revision": 1,
            "evidence_snapshot_hash": "x",
            "packet_id": "pk",
            "result": "none",
            "produced_at": "2026-04-18T10:00:00Z",
            "extra_field": "bad"
        });
        assert!(serde_json::from_value::<ReviewerOutput>(v).is_err());
    }
}
