//! CLI error taxonomy.
//!
//! Each variant has a stable machine-readable `code` (matching the V2 CLI
//! contract), a `message` for humans, an `exit_code`, a `retryable` flag, a
//! suggested `hint`, and structured `details` for the error envelope.

// The envelope module and the CLI command implementations (T10-T12) will
// consume every method below; until then the non-test call sites live in
// the crate's own unit tests.
#![allow(dead_code)]

use serde_json::{json, Value};

/// Canonical CLI errors. Variants map 1:1 to the `code` field in the
/// `codex1.error.v1` envelope.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(
        "mission id must match ^[a-z0-9](?:[a-z0-9-]{{0,62}}[a-z0-9])?$: got {got:?}"
    )]
    MissionIdInvalid { got: String },

    #[error("mission directory already exists at {path}")]
    MissionExists { path: String },

    #[error("mission directory not found at {path}")]
    MissionNotFound { path: String },

    #[error("OUTCOME-LOCK.md is invalid: {reason}")]
    LockInvalid {
        path: String,
        reason: String,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("PROGRAM-BLUEPRINT.md is invalid: {reason}")]
    BlueprintInvalid {
        path: String,
        reason: String,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("DAG task id {got:?} does not match ^T[0-9]+$")]
    DagBadId { got: String },

    #[error("DAG contains duplicate task id {id}")]
    DagDuplicateId { id: String },

    #[error("DAG contains a cycle: {path}")]
    DagCycle { path: String, cycle: Vec<String> },

    #[error("task {task} depends on missing task {missing}")]
    DagMissingDep { task: String, missing: String },

    #[error("PROGRAM-BLUEPRINT.md is missing the codex1:plan-dag markers")]
    DagNoBlock { path: String },

    #[error("DAG schema error: {reason}")]
    DagBadSchema { reason: String },

    #[error("STATE.json is corrupt: {reason}")]
    StateCorrupt {
        path: String,
        reason: String,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("repo root is invalid: {reason}")]
    RepoRootInvalid { reason: String, path: String },

    #[error("I/O error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl CliError {
    /// Stable, machine-readable error code shown in the envelope.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissionIdInvalid { .. } => "MISSION_ID_INVALID",
            Self::MissionExists { .. } => "MISSION_EXISTS",
            Self::MissionNotFound { .. } => "MISSION_NOT_FOUND",
            Self::LockInvalid { .. } => "LOCK_INVALID",
            Self::BlueprintInvalid { .. } => "BLUEPRINT_INVALID",
            Self::DagBadId { .. } => "DAG_BAD_ID",
            Self::DagDuplicateId { .. } => "DAG_DUPLICATE_ID",
            Self::DagCycle { .. } => "DAG_CYCLE",
            Self::DagMissingDep { .. } => "DAG_MISSING_DEP",
            Self::DagNoBlock { .. } => "DAG_NO_BLOCK",
            Self::DagBadSchema { .. } => "DAG_BAD_SCHEMA",
            Self::StateCorrupt { .. } => "STATE_CORRUPT",
            Self::RepoRootInvalid { .. } => "REPO_ROOT_INVALID",
            Self::Io { .. } => "IO_ERROR",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }

    /// Process exit code emitted when this error is the command result.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::MissionExists { .. } => 3,
            Self::Io { .. } => 5,
            Self::Internal { .. } => 70,
            _ => 2,
        }
    }

    /// Whether a caller can safely retry the same command.
    #[must_use]
    pub fn retryable(&self) -> bool {
        matches!(self, Self::Io { .. })
    }

    /// Optional human-actionable hint.
    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::MissionIdInvalid { .. } => Some(
                "Use lowercase alphanumerics with optional dashes; 1-64 chars.".into(),
            ),
            Self::MissionExists { path } => Some(format!(
                "Remove {path} or choose a different --mission id."
            )),
            Self::MissionNotFound { .. } => {
                Some("Run `codex1 init` first to create the mission.".into())
            }
            Self::LockInvalid { .. } => {
                Some("Re-run $clarify or edit OUTCOME-LOCK.md to fix the issue.".into())
            }
            Self::BlueprintInvalid { .. } | Self::DagBadSchema { .. } => {
                Some("Re-run $plan or edit PROGRAM-BLUEPRINT.md to fix the issue.".into())
            }
            Self::DagNoBlock { .. } => Some(
                "PROGRAM-BLUEPRINT.md must contain a YAML block delimited by \
                 <!-- codex1:plan-dag:start --> and <!-- codex1:plan-dag:end -->."
                    .into(),
            ),
            Self::StateCorrupt { .. } => {
                Some("Inspect STATE.json and events.jsonl; V2 does not auto-repair.".into())
            }
            Self::RepoRootInvalid { .. } => {
                Some("Pass --repo-root <existing-directory> or run from the repo root.".into())
            }
            _ => None,
        }
    }

    /// Structured details attached to the envelope.
    #[must_use]
    pub fn details(&self) -> Value {
        match self {
            Self::MissionIdInvalid { got } | Self::DagBadId { got } => {
                json!({ "got": got })
            }
            Self::MissionExists { path } | Self::MissionNotFound { path } => {
                json!({ "path": path })
            }
            Self::LockInvalid { path, reason, .. }
            | Self::BlueprintInvalid { path, reason, .. }
            | Self::StateCorrupt { path, reason, .. } => {
                json!({ "path": path, "reason": reason })
            }
            Self::DagDuplicateId { id } => json!({ "id": id }),
            Self::DagCycle { path, cycle } => json!({ "path": path, "cycle": cycle }),
            Self::DagMissingDep { task, missing } => json!({ "task": task, "missing": missing }),
            Self::DagNoBlock { path } => json!({ "path": path }),
            Self::DagBadSchema { reason } => json!({ "reason": reason }),
            Self::RepoRootInvalid { reason, path } => json!({ "reason": reason, "path": path }),
            Self::Io { path, source } => json!({ "path": path, "source": source.to_string() }),
            Self::Internal { message } => json!({ "message": message }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CliError;

    #[test]
    fn code_is_stable() {
        assert_eq!(
            CliError::MissionIdInvalid { got: "BAD".into() }.code(),
            "MISSION_ID_INVALID"
        );
        assert_eq!(
            CliError::MissionExists { path: "x".into() }.code(),
            "MISSION_EXISTS"
        );
        assert_eq!(
            CliError::DagCycle { path: "p".into(), cycle: vec!["T1".into(), "T2".into()] }.code(),
            "DAG_CYCLE"
        );
    }

    #[test]
    fn exit_codes_match_contract() {
        assert_eq!(CliError::MissionExists { path: "x".into() }.exit_code(), 3);
        assert_eq!(CliError::MissionIdInvalid { got: "X".into() }.exit_code(), 2);
        assert_eq!(
            CliError::Internal { message: "bug".into() }.exit_code(),
            70
        );
    }

    #[test]
    fn io_is_retryable_others_are_not() {
        let io_err = CliError::Io {
            path: "/tmp/x".into(),
            source: std::io::Error::new(std::io::ErrorKind::Other, "denied"),
        };
        assert!(io_err.retryable());
        assert!(!CliError::MissionExists { path: "x".into() }.retryable());
    }

    #[test]
    fn details_include_context() {
        let err = CliError::DagMissingDep {
            task: "T3".into(),
            missing: "T99".into(),
        };
        let d = err.details();
        assert_eq!(d["task"], "T3");
        assert_eq!(d["missing"], "T99");
    }
}
