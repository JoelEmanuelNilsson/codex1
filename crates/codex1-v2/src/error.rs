//! CLI error taxonomy.
//!
//! Each variant has a stable machine-readable `code` (matching the V2 CLI
//! contract), a `message` for humans, an `exit_code`, a `retryable` flag, a
//! suggested `hint`, and structured `details` for the error envelope.

// The envelope module and the CLI command implementations (T10-T12) will
// consume every method below; until then the non-test call sites live in
// the crate's own unit tests.
#![allow(dead_code)]

use serde_json::{Value, json};

/// Canonical CLI errors. Variants map 1:1 to the `code` field in the
/// `codex1.error.v1` envelope.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("mission id must match ^[a-z0-9](?:[a-z0-9-]{{0,62}}[a-z0-9])?$: got {got:?}")]
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

    #[error("plan for mission {mission} has zero tasks")]
    DagEmpty { mission: String },

    #[error("DAG schema error: {reason}")]
    DagBadSchema { reason: String },

    #[error(
        "task {task_id} in blueprint is under-specified; missing required fields: {missing}",
        missing = missing.join(", ")
    )]
    DagTaskUnderspecified {
        task_id: String,
        missing: Vec<String>,
    },

    #[error(
        "plan declares review_boundaries but V2 does not yet enforce them; \
         remove the `review_boundaries:` block from PROGRAM-BLUEPRINT.md"
    )]
    DagBoundariesNotSupported { count: usize },

    #[error(
        "plan check for mission {mission} refuses to certify route truth \
         while OUTCOME-LOCK.md is still draft; run $clarify first"
    )]
    PlanCheckLockDraft { mission: String, path: String },

    #[error(
        "task {task_id} (kind={kind}) is missing required review profile(s) for this kind: {missing}",
        missing = missing.join(", ")
    )]
    DagKindReviewProfileMissing {
        task_id: String,
        kind: String,
        missing: Vec<String>,
        required: Vec<String>,
    },

    #[error(
        "review open for task {task_id} omits required profile(s): {missing}",
        missing = missing.join(", ")
    )]
    ReviewProfileMissing {
        task_id: String,
        missing: Vec<String>,
        blueprint: Vec<String>,
        provided: Vec<String>,
    },

    #[error("STATE.json is corrupt: {reason}")]
    StateCorrupt {
        path: String,
        reason: String,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("review bundle at {path} is corrupt: {reason}")]
    ReviewBundleCorrupt { path: String, reason: String },

    #[error("repo root is invalid: {reason}")]
    RepoRootInvalid { reason: String, path: String },

    #[error("I/O error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("proof is invalid: {reason}")]
    ProofInvalid { path: String, reason: String },

    #[error("task cannot {attempted} from status {current:?}")]
    TaskStateTransitionInvalid {
        task_id: String,
        current: String,
        attempted: String,
    },

    #[error("worker/reviewer output is stale: {reason}")]
    StaleOutput {
        task_id: Option<String>,
        bundle_id: Option<String>,
        reason: String,
    },

    #[error(
        "reviewer output declares profile {output_profile:?} but requirement {requirement_id} expects profile {requirement_profile:?}; \
         the output and requirement profiles must match"
    )]
    ReviewProfileMismatched {
        bundle_id: String,
        requirement_id: String,
        requirement_profile: String,
        output_profile: String,
        packet_id: String,
    },

    #[error(
        "mission-close review cannot open while {non_terminal_count} task(s) are not terminal: {task_ids}",
        task_ids = task_ids.join(", ")
    )]
    MissionCloseNotReady {
        task_ids: Vec<String>,
        non_terminal_count: usize,
    },

    #[error(
        "an open review bundle {bundle_id} already exists for task {task_id} (run {task_run_id}); \
         close it before opening another"
    )]
    ReviewBundleAlreadyOpen {
        task_id: String,
        task_run_id: String,
        bundle_id: String,
    },

    #[error("revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict { expected: u64, actual: u64 },

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
            Self::DagEmpty { .. } => "DAG_EMPTY",
            Self::DagBadSchema { .. } => "DAG_BAD_SCHEMA",
            Self::DagTaskUnderspecified { .. } => "DAG_TASK_UNDERSPECIFIED",
            Self::DagBoundariesNotSupported { .. } => "DAG_BOUNDARIES_NOT_SUPPORTED",
            Self::PlanCheckLockDraft { .. } => "PLAN_CHECK_LOCK_DRAFT",
            Self::DagKindReviewProfileMissing { .. } => "DAG_KIND_REVIEW_PROFILE_MISSING",
            Self::ReviewProfileMissing { .. } => "REVIEW_PROFILE_MISSING",
            Self::StateCorrupt { .. } => "STATE_CORRUPT",
            Self::ReviewBundleCorrupt { .. } => "REVIEW_BUNDLE_CORRUPT",
            Self::RepoRootInvalid { .. } => "REPO_ROOT_INVALID",
            Self::Io { .. } => "IO_ERROR",
            Self::ProofInvalid { .. } => "PROOF_INVALID",
            Self::TaskStateTransitionInvalid { .. } => "TASK_STATE_INVALID",
            Self::StaleOutput { .. } => "STALE_OUTPUT",
            Self::ReviewProfileMismatched { .. } => "REVIEW_PROFILE_MISMATCHED",
            Self::MissionCloseNotReady { .. } => "MISSION_CLOSE_NOT_READY",
            Self::ReviewBundleAlreadyOpen { .. } => "REVIEW_BUNDLE_ALREADY_OPEN",
            Self::RevisionConflict { .. } => "REVISION_CONFLICT",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }

    /// Process exit code emitted when this error is the command result.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::MissionExists { .. } => 3,
            Self::RevisionConflict { .. } => 4,
            Self::Io { .. } => 5,
            Self::Internal { .. } => 70,
            _ => 2,
        }
    }

    /// Whether a caller can safely retry the same command.
    #[must_use]
    pub fn retryable(&self) -> bool {
        matches!(self, Self::Io { .. } | Self::RevisionConflict { .. })
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
            Self::DagEmpty { .. } => Some(
                "Author tasks under the codex1:plan-dag:start/end markers and \
                 re-run codex1 plan check. An empty plan is not executable."
                    .into(),
            ),
            Self::DagTaskUnderspecified { .. } => Some(
                "Every task must declare spec_ref, write_paths (non-empty), \
                 proof (non-empty), and review_profiles (non-empty) so the \
                 executability, proof, and review contracts are all defined."
                    .into(),
            ),
            Self::DagBoundariesNotSupported { .. } => Some(
                "Remove the `review_boundaries:` block until V2 implements the \
                 integration-review flow. Task-level review_profiles still work."
                    .into(),
            ),
            Self::PlanCheckLockDraft { .. } => Some(
                "Run $clarify to ratify OUTCOME-LOCK.md (flip `lock_status: draft` \
                 to `ratified`). Route truth cannot be certified before destination \
                 truth is locked in."
                    .into(),
            ),
            Self::DagKindReviewProfileMissing { required, .. } => Some(format!(
                "Tasks of this kind must include every profile in {required:?} so \
                 the review contract covers the kind's risk surface. Add \
                 the missing profile(s) to this task's review_profiles."
            )),
            Self::ReviewProfileMissing { .. } => Some(
                "Pass every profile listed in the task's blueprint review_profiles \
                 via --profiles; additional profiles may be added, but none of \
                 the blueprint-required ones may be dropped."
                    .into(),
            ),
            Self::StateCorrupt { .. } => {
                Some("Inspect STATE.json and events.jsonl; V2 does not auto-repair.".into())
            }
            Self::ReviewBundleCorrupt { .. } => Some(
                "A malformed bundle file is treated as mission-close truth loss; \
                 inspect reviews/B*.json manually rather than letting V2 silently skip it."
                    .into(),
            ),
            Self::RepoRootInvalid { .. } => {
                Some("Pass --repo-root <existing-directory> or run from the repo root.".into())
            }
            Self::ProofInvalid { .. } => {
                Some("Ensure specs/T<N>/PROOF.md exists and is reachable from --proof.".into())
            }
            Self::TaskStateTransitionInvalid { .. } => {
                Some("Inspect STATE.json; only Ready → InProgress and InProgress → ProofSubmitted are allowed in Wave 2.".into())
            }
            Self::StaleOutput { .. } => Some(
                "Re-run task start to mint a fresh task_run_id or re-open the review bundle.".into(),
            ),
            Self::ReviewProfileMismatched { .. } => Some(
                "Set the output's `profile` to match the requirement's declared `profile`, \
                 or re-target the output at the requirement whose profile it was actually produced for."
                    .into(),
            ),
            Self::MissionCloseNotReady { .. } => Some(
                "Run every task through review and mark it review_clean (or supersede it) \
                 before opening the mission-close bundle. The bundle binds to the terminal \
                 state that exists at open time."
                    .into(),
            ),
            Self::ReviewBundleAlreadyOpen { bundle_id, .. } => Some(format!(
                "Close {bundle_id} first with `codex1 review close --mission <id> \
                 --bundle {bundle_id}`, or submit the missing reviewer outputs \
                 and close that bundle clean."
            )),
            Self::RevisionConflict { .. } => Some(
                "State changed under you; re-read STATE.json and retry.".into(),
            ),
            _ => None,
        }
    }

    /// Structured details attached to the envelope.
    #[must_use]
    #[allow(clippy::too_many_lines)] // One arm per variant; splitting obscures.
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
            | Self::StateCorrupt { path, reason, .. }
            | Self::ReviewBundleCorrupt { path, reason } => {
                json!({ "path": path, "reason": reason })
            }
            Self::DagDuplicateId { id } => json!({ "id": id }),
            Self::DagCycle { path, cycle } => json!({ "path": path, "cycle": cycle }),
            Self::DagMissingDep { task, missing } => json!({ "task": task, "missing": missing }),
            Self::DagNoBlock { path } => json!({ "path": path }),
            Self::DagEmpty { mission } => json!({ "mission": mission }),
            Self::DagBadSchema { reason } => json!({ "reason": reason }),
            Self::DagTaskUnderspecified { task_id, missing } => {
                json!({ "task_id": task_id, "missing": missing })
            }
            Self::DagBoundariesNotSupported { count } => {
                json!({ "count": count })
            }
            Self::PlanCheckLockDraft { mission, path } => {
                json!({ "mission": mission, "path": path })
            }
            Self::DagKindReviewProfileMissing {
                task_id,
                kind,
                missing,
                required,
            } => {
                json!({
                    "task_id": task_id,
                    "kind": kind,
                    "missing": missing,
                    "required": required,
                })
            }
            Self::ReviewProfileMissing {
                task_id,
                missing,
                blueprint,
                provided,
            } => {
                json!({
                    "task_id": task_id,
                    "missing": missing,
                    "blueprint": blueprint,
                    "provided": provided,
                })
            }
            Self::RepoRootInvalid { reason, path } => json!({ "reason": reason, "path": path }),
            Self::Io { path, source } => json!({ "path": path, "source": source.to_string() }),
            Self::ProofInvalid { path, reason } => json!({ "path": path, "reason": reason }),
            Self::TaskStateTransitionInvalid {
                task_id,
                current,
                attempted,
            } => {
                json!({ "task_id": task_id, "current": current, "attempted": attempted })
            }
            Self::StaleOutput {
                task_id,
                bundle_id,
                reason,
            } => {
                json!({ "task_id": task_id, "bundle_id": bundle_id, "reason": reason })
            }
            Self::ReviewProfileMismatched {
                bundle_id,
                requirement_id,
                requirement_profile,
                output_profile,
                packet_id,
            } => {
                json!({
                    "bundle_id": bundle_id,
                    "requirement_id": requirement_id,
                    "requirement_profile": requirement_profile,
                    "output_profile": output_profile,
                    "packet_id": packet_id,
                })
            }
            Self::MissionCloseNotReady {
                task_ids,
                non_terminal_count,
            } => {
                json!({
                    "task_ids": task_ids,
                    "non_terminal_count": non_terminal_count,
                })
            }
            Self::ReviewBundleAlreadyOpen {
                task_id,
                task_run_id,
                bundle_id,
            } => {
                json!({
                    "task_id": task_id,
                    "task_run_id": task_run_id,
                    "bundle_id": bundle_id,
                })
            }
            Self::RevisionConflict { expected, actual } => {
                json!({ "expected": expected, "actual": actual })
            }
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
            CliError::DagCycle {
                path: "p".into(),
                cycle: vec!["T1".into(), "T2".into()]
            }
            .code(),
            "DAG_CYCLE"
        );
    }

    #[test]
    fn exit_codes_match_contract() {
        assert_eq!(CliError::MissionExists { path: "x".into() }.exit_code(), 3);
        assert_eq!(
            CliError::MissionIdInvalid { got: "X".into() }.exit_code(),
            2
        );
        assert_eq!(
            CliError::Internal {
                message: "bug".into()
            }
            .exit_code(),
            70
        );
    }

    #[test]
    fn io_is_retryable_others_are_not() {
        let io_err = CliError::Io {
            path: "/tmp/x".into(),
            source: std::io::Error::other("denied"),
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
