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
use crate::state::readiness::{self, Verdict};
use crate::state::schema::{MissionCloseReviewState, MissionState, ReviewVerdict, TaskStatus};
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
    for (task_id, record) in &state.tasks {
        if !matches!(record.status, TaskStatus::Complete | TaskStatus::Superseded) {
            blockers.push(Blocker::new(
                "TASK_NOT_READY",
                format!("{task_id} is {}", serde_variant(&record.status)),
            ));
        }
    }
    for (review_id, record) in &state.reviews {
        if matches!(record.verdict, ReviewVerdict::Dirty) {
            blockers.push(Blocker::new(
                "REVIEW_FINDINGS_BLOCK",
                format!("{review_id} has dirty review"),
            ));
        }
    }
    if tasks_all_complete(state) {
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

    blockers
}

fn tasks_all_complete(state: &MissionState) -> bool {
    if state.tasks.is_empty() {
        return false;
    }
    state
        .tasks
        .values()
        .all(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Superseded))
}

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let report = ReadinessReport::from_state(&state);
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
