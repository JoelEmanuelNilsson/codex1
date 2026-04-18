//! Worker/reviewer output binding schema + staleness check.
//!
//! Every worker or reviewer output carries the five binding fields below.
//! [`check_staleness`] takes a binding and the current snapshot and either
//! returns `Ok(())` (output is fresh) or [`CliError::StaleOutput`] (output
//! must be quarantined and not counted).
//!
//! The retrospective named stale outputs as a primary V1 pain source:
//! late reviewer outputs counting as evidence, receipts whose hashes no
//! longer matched the current proof, bundles confused by superseded
//! tasks. V2 makes that drift mechanically detectable.

// Call sites for [`check_staleness`] land in T15/T16/T21.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::error::CliError;

/// Full binding attached to any worker or reviewer output. Optional fields
/// cover both worker (`task_run_id`) and reviewer (`bundle_id`) variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutputBinding {
    /// Task id the output is scoped to. Present for worker outputs; reviewer
    /// outputs may also set it when the bundle targets a single task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Worker run identifier minted by `task start`. Present on worker
    /// outputs; None on reviewer outputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_run_id: Option<String>,
    /// Review bundle id. Present on reviewer outputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    /// Graph revision the output was produced against.
    pub graph_revision: u64,
    /// STATE.json `state_revision` the output was produced against.
    pub state_revision: u64,
    /// `sha256:<hex>` hash of the evidence the output reasoned over (proof
    /// file for workers, evidence snapshot for reviewers).
    pub evidence_snapshot_hash: String,
    /// Parent-issued packet id (idempotency key).
    pub packet_id: String,
}

/// Current-truth snapshot the caller compares against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentSnapshot<'a> {
    pub task_id: Option<&'a str>,
    pub task_run_id: Option<&'a str>,
    pub bundle_id: Option<&'a str>,
    pub graph_revision: u64,
    pub state_revision: u64,
    pub evidence_snapshot_hash: Option<&'a str>,
}

/// Validate that `binding` matches the current snapshot. On mismatch,
/// returns [`CliError::StaleOutput`] whose `reason` names the specific
/// field that drifted — caller can surface the binding in the error's
/// `details` payload.
pub fn check_staleness(
    binding: &OutputBinding,
    current: &CurrentSnapshot<'_>,
) -> Result<(), CliError> {
    if let Some(t) = current.task_id
        && binding.task_id.as_deref() != Some(t)
    {
        return Err(stale(
            binding,
            format!(
                "task_id mismatch: binding {:?} vs current {t:?}",
                binding.task_id
            ),
        ));
    }
    if let Some(run) = current.task_run_id
        && binding.task_run_id.as_deref() != Some(run)
    {
        return Err(stale(
            binding,
            format!(
                "task_run_id mismatch: binding {:?} vs current {run:?}",
                binding.task_run_id
            ),
        ));
    }
    if let Some(b) = current.bundle_id
        && binding.bundle_id.as_deref() != Some(b)
    {
        return Err(stale(
            binding,
            format!(
                "bundle_id mismatch: binding {:?} vs current {b:?}",
                binding.bundle_id
            ),
        ));
    }
    if binding.graph_revision != current.graph_revision {
        return Err(stale(
            binding,
            format!(
                "graph_revision mismatch: binding {} vs current {}",
                binding.graph_revision, current.graph_revision
            ),
        ));
    }
    if binding.state_revision > current.state_revision {
        return Err(stale(
            binding,
            format!(
                "state_revision ahead of current: binding {} vs current {}",
                binding.state_revision, current.state_revision
            ),
        ));
    }
    if let Some(h) = current.evidence_snapshot_hash
        && binding.evidence_snapshot_hash != h
    {
        return Err(stale(
            binding,
            format!(
                "evidence_snapshot_hash mismatch: binding {:?} vs current {h:?}",
                binding.evidence_snapshot_hash
            ),
        ));
    }
    Ok(())
}

fn stale(binding: &OutputBinding, reason: String) -> CliError {
    CliError::StaleOutput {
        task_id: binding.task_id.clone(),
        bundle_id: binding.bundle_id.clone(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::{CurrentSnapshot, OutputBinding, check_staleness};

    fn binding() -> OutputBinding {
        OutputBinding {
            task_id: Some("T1".into()),
            task_run_id: Some("run-a".into()),
            bundle_id: None,
            graph_revision: 1,
            state_revision: 5,
            evidence_snapshot_hash: "sha256:abc".into(),
            packet_id: "pkt-1".into(),
        }
    }

    fn snapshot<'a>() -> CurrentSnapshot<'a> {
        CurrentSnapshot {
            task_id: Some("T1"),
            task_run_id: Some("run-a"),
            bundle_id: None,
            graph_revision: 1,
            state_revision: 5,
            evidence_snapshot_hash: Some("sha256:abc"),
        }
    }

    #[test]
    fn matching_binding_passes() {
        assert!(check_staleness(&binding(), &snapshot()).is_ok());
    }

    #[test]
    fn task_run_id_drift_quarantines() {
        let mut snap = snapshot();
        snap.task_run_id = Some("run-b");
        let err = check_staleness(&binding(), &snap).unwrap_err();
        assert_eq!(err.code(), "STALE_OUTPUT");
        assert!(err.to_string().contains("task_run_id"));
    }

    #[test]
    fn graph_revision_drift_quarantines() {
        let mut snap = snapshot();
        snap.graph_revision = 2;
        let err = check_staleness(&binding(), &snap).unwrap_err();
        assert_eq!(err.code(), "STALE_OUTPUT");
        assert!(err.to_string().contains("graph_revision"));
    }

    #[test]
    fn evidence_hash_drift_quarantines() {
        let mut snap = snapshot();
        snap.evidence_snapshot_hash = Some("sha256:changed");
        let err = check_staleness(&binding(), &snap).unwrap_err();
        assert_eq!(err.code(), "STALE_OUTPUT");
        assert!(err.to_string().contains("evidence_snapshot_hash"));
    }

    #[test]
    fn state_revision_ahead_quarantines() {
        let mut b = binding();
        b.state_revision = 10;
        let err = check_staleness(&b, &snapshot()).unwrap_err();
        assert_eq!(err.code(), "STALE_OUTPUT");
        assert!(err.to_string().contains("ahead"));
    }

    #[test]
    fn state_revision_behind_is_allowed_for_read_snapshots() {
        // Workers may bind to an older state_revision than the current
        // state — that's the normal case if state has advanced since the
        // worker started. Only "ahead" is drift.
        let mut b = binding();
        b.state_revision = 3;
        assert!(check_staleness(&b, &snapshot()).is_ok());
    }

    #[test]
    fn task_id_mismatch_quarantines() {
        let mut snap = snapshot();
        snap.task_id = Some("T99");
        let err = check_staleness(&binding(), &snap).unwrap_err();
        assert_eq!(err.code(), "STALE_OUTPUT");
        assert!(err.to_string().contains("task_id"));
    }

    #[test]
    fn bundle_id_mismatch_quarantines() {
        let mut b = binding();
        b.bundle_id = Some("B1".into());
        let mut snap = snapshot();
        snap.bundle_id = Some("B2");
        let err = check_staleness(&b, &snap).unwrap_err();
        assert_eq!(err.code(), "STALE_OUTPUT");
        assert!(err.to_string().contains("bundle_id"));
    }

    #[test]
    fn binding_schema_round_trips_json() {
        let b = binding();
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(v["task_id"], "T1");
        assert_eq!(v["task_run_id"], "run-a");
        assert_eq!(v["graph_revision"], 1);
        let parsed: OutputBinding = serde_json::from_value(v).unwrap();
        assert_eq!(parsed, b);
    }

    #[test]
    fn binding_rejects_unknown_fields() {
        let v = serde_json::json!({
            "task_id": "T1",
            "graph_revision": 1,
            "state_revision": 1,
            "evidence_snapshot_hash": "x",
            "packet_id": "p",
            "extra": "surprise"
        });
        let err = serde_json::from_value::<OutputBinding>(v).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }
}
