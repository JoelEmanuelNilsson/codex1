//! `codex1 close check --json` — read-only mission-close readiness projection.
//!
//! The verdict string comes from `state::readiness::derive_verdict` so that
//! `close check` and `status` can never disagree. The blocker list is
//! derived independently from raw state fields — it enumerates every
//! concrete reason the mission is not terminal-ready regardless of which
//! single verdict `derive_verdict` collapsed to.

use serde::Serialize;
use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::core::paths::ensure_mission_write_safe;
use crate::core::paths::MissionPaths;
use crate::state::readiness::{self, Verdict};
use crate::state::schema::{
    MissionCloseReviewState, MissionState, ReviewRecordCategory, ReviewVerdict, TaskStatus,
};
use crate::state::{self};

use super::serde_variant;

/// One element of `close check`'s `blockers` array.
#[derive(Debug, Clone, Serialize)]
pub struct Blocker {
    pub code: &'static str,
    pub detail: String,
}

impl Blocker {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// Snapshot of the readiness signal used by both `check` and `complete`.
pub struct ReadinessReport {
    pub verdict: Verdict,
    pub blockers: Vec<Blocker>,
    pub ready: bool,
}

impl ReadinessReport {
    pub fn from_state(state: &MissionState) -> Self {
        let verdict = readiness::derive_verdict(state);
        let blockers = derive_blockers(state);
        let ready = matches!(verdict, Verdict::MissionCloseReviewPassed);
        Self {
            verdict,
            blockers,
            ready,
        }
    }

    pub fn from_state_and_paths(state: &MissionState, paths: &MissionPaths) -> Self {
        let verdict = readiness::derive_verdict(state);
        let blockers = derive_blockers_with_paths(state, Some(paths));
        let ready = matches!(verdict, Verdict::MissionCloseReviewPassed) && blockers.is_empty();
        Self {
            verdict,
            blockers,
            ready,
        }
    }

    /// Concatenated blocker message for error envelopes.
    pub fn blocker_summary(&self) -> String {
        if self.blockers.is_empty() {
            "not ready".to_string()
        } else {
            self.blockers
                .iter()
                .map(|b| format!("{}: {}", b.code, b.detail))
                .collect::<Vec<_>>()
                .join("; ")
        }
    }
}

/// Enumerate every concrete reason the mission is not terminal-ready.
/// Independent of `derive_verdict` so a single collapsed verdict does
/// not hide parallel issues from the caller.
#[must_use]
pub fn derive_blockers(state: &MissionState) -> Vec<Blocker> {
    derive_blockers_with_paths(state, None)
}

fn derive_blockers_with_paths(state: &MissionState, paths: Option<&MissionPaths>) -> Vec<Blocker> {
    let mut blockers = Vec::new();

    if !state.outcome.ratified {
        blockers.push(Blocker::new(
            "OUTCOME_NOT_RATIFIED",
            "OUTCOME.md has not been ratified",
        ));
    }
    if !state.plan.locked {
        blockers.push(Blocker::new("PLAN_INVALID", "plan not locked"));
    }
    if state.replan.triggered {
        let detail = state
            .replan
            .triggered_reason
            .clone()
            .unwrap_or_else(|| "replan triggered".to_string());
        blockers.push(Blocker::new("REPLAN_REQUIRED", detail));
    }
    // Enumerate every DAG node that is not yet Complete/Superseded.
    // Walk `state.plan.task_ids` (the authoritative DAG snapshot) so
    // tasks that were never started still show up as `TASK_NOT_READY`,
    // and fall back to `state.tasks` when the plan is not yet locked
    // (task_ids empty) so early-phase users still see per-task detail.
    if state.plan.task_ids.is_empty() {
        for (task_id, record) in &state.tasks {
            if !matches!(record.status, TaskStatus::Complete | TaskStatus::Superseded) {
                blockers.push(Blocker::new(
                    "TASK_NOT_READY",
                    format!("{task_id} is {}", serde_variant(&record.status)),
                ));
            }
        }
    } else {
        for id in &state.plan.task_ids {
            match state.tasks.get(id) {
                Some(t) if matches!(t.status, TaskStatus::Complete | TaskStatus::Superseded) => {
                    if matches!(t.status, TaskStatus::Complete) {
                        if let Some(paths) = paths {
                            match proof_exists(paths, t.proof_path.as_deref()) {
                                Ok(()) => {}
                                Err(detail) => blockers
                                    .push(Blocker::new("PROOF_MISSING", format!("{id}: {detail}"))),
                            }
                        }
                    }
                }
                Some(t) => blockers.push(Blocker::new(
                    "TASK_NOT_READY",
                    format!("{id} is {}", serde_variant(&t.status)),
                )),
                None => blockers.push(Blocker::new(
                    "TASK_NOT_READY",
                    format!("{id} has not started"),
                )),
            }
        }
    }
    for (review_id, record) in &state.reviews {
        if matches!(record.verdict, ReviewVerdict::Dirty) {
            if !matches!(record.category, ReviewRecordCategory::AcceptedCurrent) {
                continue;
            }
            blockers.push(Blocker::new(
                "REVIEW_FINDINGS_BLOCK",
                format!("{review_id} has dirty review"),
            ));
        }
    }
    if readiness::tasks_complete(state) {
        match state.close.review_state {
            MissionCloseReviewState::NotStarted => {
                blockers.push(Blocker::new(
                    "CLOSE_NOT_READY",
                    "mission-close review has not started",
                ));
            }
            MissionCloseReviewState::Open => {
                blockers.push(Blocker::new(
                    "CLOSE_NOT_READY",
                    "mission-close review still open",
                ));
            }
            MissionCloseReviewState::Passed => {}
        }
    }

    if matches!(state.close.review_state, MissionCloseReviewState::Passed) {
        if let Some(paths) = paths {
            if let Err(detail) = closeout_ready(paths) {
                blockers.push(Blocker::new("CLOSE_NOT_READY", detail));
            }
        }
    }

    blockers
}

fn proof_exists(paths: &MissionPaths, proof_path: Option<&str>) -> Result<(), String> {
    let Some(raw) = proof_path else {
        return Ok(());
    };
    let path = std::path::Path::new(raw);
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        paths.mission_dir.join(path)
    };
    if abs.is_file() {
        Ok(())
    } else {
        Err(format!("proof file not found at {}", abs.display()))
    }
}

fn closeout_ready(paths: &MissionPaths) -> Result<(), String> {
    ensure_mission_write_safe(paths).map_err(|err| err.to_string())?;
    let closeout = paths.closeout();
    if let Ok(meta) = std::fs::symlink_metadata(&closeout) {
        if meta.file_type().is_symlink() {
            return Err(format!(
                "CLOSEOUT.md must not be a symlink: {}",
                closeout.display()
            ));
        }
    }
    if closeout.exists() && !closeout.is_file() {
        return Err(format!(
            "CLOSEOUT.md target is not a file: {}",
            closeout.display()
        ));
    }
    Ok(())
}

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let report = ReadinessReport::from_state_and_paths(&state, &paths);
    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        json!({
            "ready": report.ready,
            "verdict": report.verdict.as_str(),
            "blockers": report.blockers,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}
