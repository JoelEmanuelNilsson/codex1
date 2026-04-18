//! Serde types for `STATE.json`.
//!
//! `STATE.json` is the only authoritative operational truth. Every field in
//! this module must be explicit — `#[serde(deny_unknown_fields)]` on each
//! struct means typos in hand-edited STATE files fail loudly rather than
//! being silently ignored.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Top-level state payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)] // state_revision is the contract name; don't rename.
pub struct State {
    pub mission_id: String,
    pub state_revision: u64,
    pub phase: Phase,
    pub parent_loop: ParentLoop,
    pub tasks: BTreeMap<String, TaskState>,
}

/// High-level mission phase. Stored, not derived. Wave 1 only transitions
/// via `init` (writes `clarify`); later waves add transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Clarify,
    Planning,
    Executing,
    Reviewing,
    Repairing,
    Replanning,
    Waiting,
    MissionClose,
    Complete,
}

/// Parent-loop state. Wave 1 sets `mode: none, paused: false`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ParentLoop {
    #[serde(default)]
    pub mode: ParentLoopMode,
    #[serde(default)]
    pub paused: bool,
}

/// Allowed parent-loop modes. Wave 1 uses `None`; Wave 4 introduces the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParentLoopMode {
    #[default]
    None,
    Execute,
    Review,
    Autopilot,
    Close,
}

/// Per-task state record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskState {
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
}

impl TaskState {
    /// Convenience: task in the initial `planned` state with no timestamps.
    #[must_use]
    pub fn planned() -> Self {
        Self {
            status: TaskStatus::Planned,
            started_at: None,
            finished_at: None,
            reviewed_at: None,
        }
    }
}

/// Allowed task statuses. The enum encodes every status reachable across all
/// waves so later waves don't require schema changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Planned,
    Ready,
    InProgress,
    ProofSubmitted,
    ReviewOwed,
    ReviewFailed,
    NeedsRepair,
    ReplanRequired,
    ReviewClean,
    Complete,
    Superseded,
}

impl TaskStatus {
    /// True when the status satisfies a downstream dependency.
    #[must_use]
    pub fn satisfies_dep(self) -> bool {
        matches!(self, Self::ReviewClean | Self::Complete)
    }

    /// True when the task is terminal (cannot be acted on further).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Complete | Self::Superseded | Self::ReviewClean
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ParentLoop, ParentLoopMode, Phase, State, TaskState, TaskStatus,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn state_round_trips_through_json() {
        let original = State {
            mission_id: "example".into(),
            state_revision: 1,
            phase: Phase::Clarify,
            parent_loop: ParentLoop::default(),
            tasks: BTreeMap::new(),
        };
        let json = serde_json::to_value(&original).unwrap();
        assert_eq!(json["mission_id"], "example");
        assert_eq!(json["state_revision"], 1);
        assert_eq!(json["phase"], "clarify");
        assert_eq!(json["parent_loop"]["mode"], "none");
        assert_eq!(json["parent_loop"]["paused"], false);
        let parsed: State = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn deny_unknown_fields_rejects_extras_in_state() {
        let bad = json!({
            "mission_id": "example",
            "state_revision": 1,
            "phase": "clarify",
            "parent_loop": { "mode": "none", "paused": false },
            "tasks": {},
            "unexpected_field": "nope"
        });
        let err = serde_json::from_value::<State>(bad).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("unknown"));
    }

    #[test]
    fn deny_unknown_fields_rejects_extras_in_task_state() {
        let bad = json!({ "status": "ready", "weird": 42 });
        let err = serde_json::from_value::<TaskState>(bad).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("unknown"));
    }

    #[test]
    fn all_phase_variants_serialize_as_snake_case() {
        for (phase, expected) in [
            (Phase::Clarify, "clarify"),
            (Phase::Planning, "planning"),
            (Phase::Executing, "executing"),
            (Phase::Reviewing, "reviewing"),
            (Phase::Repairing, "repairing"),
            (Phase::Replanning, "replanning"),
            (Phase::Waiting, "waiting"),
            (Phase::MissionClose, "mission_close"),
            (Phase::Complete, "complete"),
        ] {
            assert_eq!(serde_json::to_value(phase).unwrap(), expected);
        }
    }

    #[test]
    fn all_task_status_variants_serialize_as_snake_case() {
        for (status, expected) in [
            (TaskStatus::Planned, "planned"),
            (TaskStatus::Ready, "ready"),
            (TaskStatus::InProgress, "in_progress"),
            (TaskStatus::ProofSubmitted, "proof_submitted"),
            (TaskStatus::ReviewOwed, "review_owed"),
            (TaskStatus::ReviewFailed, "review_failed"),
            (TaskStatus::NeedsRepair, "needs_repair"),
            (TaskStatus::ReplanRequired, "replan_required"),
            (TaskStatus::ReviewClean, "review_clean"),
            (TaskStatus::Complete, "complete"),
            (TaskStatus::Superseded, "superseded"),
        ] {
            assert_eq!(serde_json::to_value(status).unwrap(), expected);
        }
    }

    #[test]
    fn satisfies_dep_only_for_review_clean_and_complete() {
        assert!(TaskStatus::ReviewClean.satisfies_dep());
        assert!(TaskStatus::Complete.satisfies_dep());
        for s in [
            TaskStatus::Planned,
            TaskStatus::Ready,
            TaskStatus::InProgress,
            TaskStatus::ProofSubmitted,
            TaskStatus::ReviewOwed,
            TaskStatus::ReviewFailed,
            TaskStatus::NeedsRepair,
            TaskStatus::ReplanRequired,
            TaskStatus::Superseded,
        ] {
            assert!(!s.satisfies_dep(), "{s:?} should not satisfy dep");
        }
    }

    #[test]
    fn parent_loop_mode_defaults_to_none() {
        assert_eq!(ParentLoopMode::default(), ParentLoopMode::None);
    }
}
