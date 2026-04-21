//! Shared close-readiness predicate used by both `status` and `close check`.
//!
//! Keeping the derivation in Foundation guarantees both commands can
//! never disagree about whether a mission is terminal-ready.

use crate::state::schema::{
    MissionCloseReviewState, MissionState, ReviewRecordCategory, ReviewVerdict, TaskStatus,
};

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
    if has_orphan_nonterminal_task(state) {
        return Verdict::Blocked;
    }
    if has_current_dirty_review(state) {
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

#[must_use]
pub fn has_orphan_nonterminal_task(state: &MissionState) -> bool {
    state.tasks.iter().any(|(id, task)| {
        !state.plan.task_ids.iter().any(|plan_id| plan_id == id)
            && !matches!(task.status, TaskStatus::Complete | TaskStatus::Superseded)
    })
}

#[must_use]
pub fn has_current_dirty_review(state: &MissionState) -> bool {
    state.reviews.iter().any(|(review_id, r)| {
        matches!(r.verdict, ReviewVerdict::Dirty)
            && matches!(r.category, ReviewRecordCategory::AcceptedCurrent)
            && dirty_review_record_blocks(state, review_id, r.recorded_at.as_str())
    })
}

#[must_use]
pub fn dirty_review_still_needs_repair(state: &MissionState, recorded_at: &str) -> bool {
    state.tasks.values().any(|task| {
        matches!(task.status, TaskStatus::AwaitingReview)
            && task
                .finished_at
                .as_deref()
                .is_none_or(|finished_at| finished_at <= recorded_at)
    })
}

#[must_use]
pub fn dirty_review_record_blocks(
    state: &MissionState,
    review_id: &str,
    recorded_at: &str,
) -> bool {
    if dirty_review_still_needs_repair(state, recorded_at) {
        return true;
    }
    let review_is_live = state.plan.task_ids.iter().any(|id| id == review_id)
        && state.tasks.get(review_id).is_none_or(|task| {
            !matches!(task.status, TaskStatus::Superseded) && task.superseded_by.is_none()
        });
    review_is_live
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

#[cfg(test)]
mod tests {
    //! Pure unit tests for the verdict/readiness projection. Each test
    //! constructs a `MissionState` via `MissionState::fresh` and mutates
    //! exactly the fields the branch cares about so the assertions don't
    //! depend on the integration layer.

    use super::*;
    use crate::state::schema::{
        CloseState, LoopMode, LoopState, MissionCloseReviewState, MissionState, ReviewRecord,
        ReviewVerdict, TaskRecord, TaskStatus,
    };

    fn base() -> MissionState {
        let mut s = MissionState::fresh("unit");
        s.outcome.ratified = true;
        s.plan.locked = true;
        s.plan.task_ids = vec!["T1".to_string()];
        s
    }

    fn insert_task(s: &mut MissionState, id: &str, status: TaskStatus) {
        s.tasks.insert(
            id.to_string(),
            TaskRecord {
                id: id.to_string(),
                status,
                started_at: None,
                finished_at: None,
                proof_path: None,
                superseded_by: None,
            },
        );
    }

    #[test]
    fn derive_verdict_terminal_complete_wins_over_everything() {
        let mut s = base();
        // Terminal should beat every other precondition.
        s.outcome.ratified = false;
        s.plan.locked = false;
        s.replan.triggered = true;
        s.close.terminal_at = Some("2026-04-21T00:00:00Z".to_string());
        assert_eq!(derive_verdict(&s), Verdict::TerminalComplete);
    }

    #[test]
    fn derive_verdict_unratified_outcome_is_needs_user() {
        let mut s = base();
        s.outcome.ratified = false;
        assert_eq!(derive_verdict(&s), Verdict::NeedsUser);
    }

    #[test]
    fn derive_verdict_unlocked_plan_is_needs_user() {
        let mut s = base();
        s.plan.locked = false;
        assert_eq!(derive_verdict(&s), Verdict::NeedsUser);
    }

    #[test]
    fn derive_verdict_replan_triggered_is_blocked() {
        let mut s = base();
        s.replan.triggered = true;
        assert_eq!(derive_verdict(&s), Verdict::Blocked);
    }

    #[test]
    fn derive_verdict_dirty_review_is_blocked() {
        let mut s = base();
        insert_task(&mut s, "T1", TaskStatus::AwaitingReview);
        s.reviews.insert(
            "T2".to_string(),
            ReviewRecord {
                task_id: "T2".to_string(),
                verdict: ReviewVerdict::Dirty,
                reviewers: vec![],
                findings_file: None,
                category: crate::state::schema::ReviewRecordCategory::AcceptedCurrent,
                recorded_at: "t".to_string(),
                boundary_revision: 0,
            },
        );
        assert_eq!(derive_verdict(&s), Verdict::Blocked);
    }

    #[test]
    fn derive_verdict_tasks_incomplete_is_continue_required() {
        let mut s = base();
        insert_task(&mut s, "T1", TaskStatus::InProgress);
        assert_eq!(derive_verdict(&s), Verdict::ContinueRequired);
    }

    #[test]
    fn derive_verdict_tasks_complete_not_started_is_ready_for_mission_close_review() {
        let mut s = base();
        insert_task(&mut s, "T1", TaskStatus::Complete);
        assert_eq!(derive_verdict(&s), Verdict::ReadyForMissionCloseReview);
    }

    #[test]
    fn derive_verdict_tasks_complete_review_open_is_mission_close_review_open() {
        let mut s = base();
        insert_task(&mut s, "T1", TaskStatus::Complete);
        s.close = CloseState {
            review_state: MissionCloseReviewState::Open,
            terminal_at: None,
        };
        assert_eq!(derive_verdict(&s), Verdict::MissionCloseReviewOpen);
    }

    #[test]
    fn derive_verdict_tasks_complete_review_passed_is_mission_close_review_passed() {
        let mut s = base();
        insert_task(&mut s, "T1", TaskStatus::Complete);
        s.close = CloseState {
            review_state: MissionCloseReviewState::Passed,
            terminal_at: None,
        };
        assert_eq!(derive_verdict(&s), Verdict::MissionCloseReviewPassed);
    }

    #[test]
    fn close_ready_is_only_true_for_passed() {
        let mut s = base();
        insert_task(&mut s, "T1", TaskStatus::Complete);
        s.close.review_state = MissionCloseReviewState::Passed;
        assert!(close_ready(&s));

        s.close.review_state = MissionCloseReviewState::NotStarted;
        assert!(!close_ready(&s));
        s.close.review_state = MissionCloseReviewState::Open;
        assert!(!close_ready(&s));
    }

    #[test]
    fn stop_allowed_when_loop_inactive_or_paused() {
        let mut s = base();
        insert_task(&mut s, "T1", TaskStatus::InProgress);
        // Inactive loop → stop allowed regardless of verdict.
        s.loop_ = LoopState {
            active: false,
            paused: false,
            mode: LoopMode::None,
        };
        assert!(stop_allowed(&s));
        // Active but paused → stop allowed.
        s.loop_ = LoopState {
            active: true,
            paused: true,
            mode: LoopMode::Execute,
        };
        assert!(stop_allowed(&s));
        // Active and unpaused, verdict ContinueRequired → not allowed.
        s.loop_ = LoopState {
            active: true,
            paused: false,
            mode: LoopMode::Execute,
        };
        assert!(!stop_allowed(&s));
        // Active and unpaused, but verdict is NeedsUser (plan not locked) → allowed.
        s.plan.locked = false;
        assert!(stop_allowed(&s));
    }

    #[test]
    fn tasks_complete_requires_dag_and_records() {
        let mut s = base();
        // Empty task_ids → not complete (even if tasks map empty).
        s.plan.task_ids = vec![];
        assert!(!tasks_complete(&s));
        // DAG populated but tasks missing records → not complete.
        s.plan.task_ids = vec!["T1".to_string()];
        assert!(!tasks_complete(&s));
        // Record present and complete → complete.
        insert_task(&mut s, "T1", TaskStatus::Complete);
        assert!(tasks_complete(&s));
        // Superseded counts as complete for readiness.
        insert_task(&mut s, "T1", TaskStatus::Superseded);
        assert!(tasks_complete(&s));
        // In-progress does not.
        insert_task(&mut s, "T1", TaskStatus::InProgress);
        assert!(!tasks_complete(&s));
    }
}
