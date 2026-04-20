//! Canonical `CliError` enum and error codes.
//!
//! Every variant serializes into the stable `JsonErr` shape. The string
//! `code` values here are part of the CLI contract — never rename or
//! remove a variant without a coordinated bump.

use std::path::PathBuf;

use serde_json::{json, Value};
use thiserror::Error;

use super::envelope::JsonErr;

/// Whether an error is a handled user/input error (exit 1) or a harness bug (exit 2).
#[derive(Debug, Clone, Copy)]
pub enum ExitKind {
    HandledError,
    Bug,
}

/// Canonical CLI error set. The serialized `code` matches the variant's
/// canonical name (see `Self::code`).
#[derive(Debug, Error)]
pub enum CliError {
    #[error("OUTCOME.md is incomplete: {message}")]
    OutcomeIncomplete {
        message: String,
        hint: Option<String>,
    },
    #[error("OUTCOME.md is not ratified")]
    OutcomeNotRatified,
    #[error("PLAN.yaml is invalid: {message}")]
    PlanInvalid {
        message: String,
        hint: Option<String>,
    },
    #[error("DAG contains a cycle: {message}")]
    DagCycle { message: String },
    #[error("DAG has a missing dependency: {message}")]
    DagMissingDep { message: String },
    #[error("Task is not ready: {message}")]
    TaskNotReady { message: String },
    #[error("Proof file missing: {path}")]
    ProofMissing { path: PathBuf },
    #[error("Review findings block progress: {message}")]
    ReviewFindingsBlock { message: String },
    #[error("Replan required: {message}")]
    ReplanRequired { message: String },
    #[error("Mission is not ready for close: {message}")]
    CloseNotReady { message: String },
    #[error("STATE.json is corrupt: {message}")]
    StateCorrupt { message: String },
    #[error("Revision conflict (expected {expected}, actual {actual})")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error("Review record is stale: {message}")]
    StaleReviewRecord { message: String },
    #[error("Mission is already terminal (closed at {closed_at})")]
    TerminalAlreadyComplete { closed_at: String },
    #[error("Configuration is missing: {message}")]
    ConfigMissing { message: String },
    #[error("Mission directory not found: {message}")]
    MissionNotFound {
        message: String,
        hint: Option<String>,
    },
    #[error("Parse error: {message}")]
    ParseError { message: String },
    #[error("Not implemented: {command}")]
    NotImplemented { command: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl CliError {
    /// Canonical error code string (stable contract).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::OutcomeIncomplete { .. } => "OUTCOME_INCOMPLETE",
            Self::OutcomeNotRatified => "OUTCOME_NOT_RATIFIED",
            Self::PlanInvalid { .. } => "PLAN_INVALID",
            Self::DagCycle { .. } => "DAG_CYCLE",
            Self::DagMissingDep { .. } => "DAG_MISSING_DEP",
            Self::TaskNotReady { .. } => "TASK_NOT_READY",
            Self::ProofMissing { .. } => "PROOF_MISSING",
            Self::ReviewFindingsBlock { .. } => "REVIEW_FINDINGS_BLOCK",
            Self::ReplanRequired { .. } => "REPLAN_REQUIRED",
            Self::CloseNotReady { .. } => "CLOSE_NOT_READY",
            Self::StateCorrupt { .. } => "STATE_CORRUPT",
            Self::RevisionConflict { .. } => "REVISION_CONFLICT",
            Self::StaleReviewRecord { .. } => "STALE_REVIEW_RECORD",
            Self::TerminalAlreadyComplete { .. } => "TERMINAL_ALREADY_COMPLETE",
            Self::ConfigMissing { .. } => "CONFIG_MISSING",
            Self::MissionNotFound { .. } => "MISSION_NOT_FOUND",
            Self::ParseError { .. } => "PARSE_ERROR",
            Self::NotImplemented { .. } => "NOT_IMPLEMENTED",
            Self::Io(_) | Self::Json(_) | Self::Yaml(_) => "PARSE_ERROR",
            Self::Other(_) => "INTERNAL",
        }
    }

    #[must_use]
    pub fn retryable(&self) -> bool {
        matches!(self, Self::RevisionConflict { .. })
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::OutcomeIncomplete { hint, .. }
            | Self::PlanInvalid { hint, .. }
            | Self::MissionNotFound { hint, .. } => hint.clone(),
            Self::DagCycle { .. } => Some(
                "Break the cycle by removing or redirecting one of the depends_on edges."
                    .to_string(),
            ),
            Self::DagMissingDep { .. } => Some(
                "Ensure every depends_on entry references an existing task id (e.g. T1, T2)."
                    .to_string(),
            ),
            Self::ProofMissing { .. } => {
                Some("Write the proof file before calling `task finish`.".to_string())
            }
            Self::RevisionConflict { expected, actual } => Some(format!(
                "Re-read STATE.json and retry with --expect-revision {actual} (you sent {expected})."
            )),
            Self::TerminalAlreadyComplete { .. } => Some(
                "Start a new mission; a terminal mission cannot be reopened.".to_string(),
            ),
            Self::ConfigMissing { .. } => Some(
                "Codex1 does not require auth by default; only set config if you rely on it."
                    .to_string(),
            ),
            Self::NotImplemented { .. } => Some(
                "This command surface is reserved by Foundation but not yet wired up."
                    .to_string(),
            ),
            _ => None,
        }
    }

    #[must_use]
    pub fn context(&self) -> Value {
        match self {
            Self::RevisionConflict { expected, actual } => json!({
                "expected": expected,
                "actual": actual,
            }),
            Self::ProofMissing { path } => json!({ "path": path.display().to_string() }),
            Self::TerminalAlreadyComplete { closed_at } => json!({ "closed_at": closed_at }),
            Self::NotImplemented { command } => json!({ "command": command }),
            _ => Value::Null,
        }
    }

    #[must_use]
    pub fn kind(&self) -> ExitKind {
        match self {
            Self::Io(_) | Self::Json(_) | Self::Yaml(_) | Self::Other(_) => ExitKind::Bug,
            _ => ExitKind::HandledError,
        }
    }

    #[must_use]
    pub fn to_envelope(&self) -> JsonErr {
        JsonErr::new(
            self.code().to_string(),
            self.to_string(),
            self.hint(),
            self.retryable(),
            self.context(),
        )
    }
}

pub type CliResult<T> = Result<T, CliError>;
