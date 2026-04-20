//! Shared close-readiness predicate used by both `status` and `close check`.
//!
//! Keeping the derivation in Foundation guarantees both commands can
//! never disagree about whether a mission is terminal-ready.

use crate::state::schema::{MissionCloseReviewState, MissionState, ReviewVerdict, TaskStatus};

/// Derived verdict (stable string values).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    ContinueRequired,
    NeedsUser,
    Blocked,
    ReadyForMissionCloseReview,
    MissionCloseReviewOpen,
    MissionCloseReviewPassed,
    TerminalComplete,
    InvalidState,
}

impl Verdict {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContinueRequired => "continue_required",
            Self::NeedsUser => "needs_user",
            Self::Blocked => "blocked",
            Self::ReadyForMissionCloseReview => "ready_for_mission_close_review",
            Self::MissionCloseReviewOpen => "mission_close_review_open",
            Self::MissionCloseReviewPassed => "mission_close_review_passed",
            Self::TerminalComplete => "terminal_complete",
            Self::InvalidState => "invalid_state",
        }
    }
}

/// Derive the verdict from the given `MissionState`. Follows the order
/// documented in the plan's "Pinned semantics — Verdict derivation".
#[must_use]
pub fn derive_verdict(state: &MissionState) -> Verdict {
    if state.close.terminal_at.is_some() {
        return Verdict::TerminalComplete;
    }
    if !state.outcome.ratified {
        return Verdict::NeedsUser;
    }
    if !state.plan.locked {
        return Verdict::NeedsUser;
    }
    if state.replan.triggered {
        return Verdict::Blocked;
    }
    if has_blocking_dirty(state) {
        return Verdict::Blocked;
    }

    if tasks_complete(state) {
        match state.close.review_state {
            MissionCloseReviewState::NotStarted => Verdict::ReadyForMissionCloseReview,
            MissionCloseReviewState::Open => Verdict::MissionCloseReviewOpen,
            MissionCloseReviewState::Passed => Verdict::MissionCloseReviewPassed,
        }
    } else {
        Verdict::ContinueRequired
    }
}

/// True iff `close complete` may proceed.
#[must_use]
pub fn close_ready(state: &MissionState) -> bool {
    matches!(derive_verdict(state), Verdict::MissionCloseReviewPassed)
}

/// True iff every DAG task (`state.plan.task_ids`) has a terminal
/// record (`Complete`/`Superseded`) in `state.tasks`. Shared between
/// `status`, `close check`, and any other caller that needs one
/// definition of "done" — iterating `state.tasks.values()` alone
/// would silently ignore DAG nodes that have never been started.
#[must_use]
pub fn tasks_complete(state: &MissionState) -> bool {
    let dag = &state.plan.task_ids;
    if dag.is_empty() {
        // Plan isn't locked (or the plan has zero tasks). Either way,
        // mission-close isn't applicable: treat as "not done".
        return false;
    }
    dag.iter().all(|id| {
        state
            .tasks
            .get(id)
            .is_some_and(|t| matches!(t.status, TaskStatus::Complete | TaskStatus::Superseded))
    })
}

fn has_blocking_dirty(state: &MissionState) -> bool {
    state
        .reviews
        .values()
        .any(|r| matches!(r.verdict, ReviewVerdict::Dirty))
}

/// Whether a Stop request from Ralph should be allowed.
#[must_use]
pub fn stop_allowed(state: &MissionState) -> bool {
    if !state.loop_.active || state.loop_.paused {
        return true;
    }
    matches!(
        derive_verdict(state),
        Verdict::TerminalComplete | Verdict::MissionCloseReviewPassed | Verdict::NeedsUser
    )
}
