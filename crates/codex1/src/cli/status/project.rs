//! Build the `codex1 status --json` data projection.
//!
//! Pure function from `MissionState` + PLAN.yaml tasks to the JSON
//! value inside the success envelope's `data` field. `status` and
//! `close check` share readiness via `state::readiness`; this module
//! only arranges the already-derived facts into the published shape.

use serde_json::{json, Value};

use crate::state::readiness::{self, Verdict};
use crate::state::schema::MissionState;

use super::next_action::{
    dirty_repair_targets, next_ready_wave, ready_reviews, PlanTask, ReadyWave,
};

/// Build the full `data` body.
pub fn build(state: &MissionState, tasks: &[PlanTask], close_ready: bool) -> Value {
    let verdict = readiness::derive_verdict(state);
    let stop = stop_projection(state, verdict, close_ready);

    // When the plan is not locked (fresh mission, or post-replan
    // `replan record` that cleared the lock), wave / review derivation
    // is meaningless: the caller is supposed to be planning, not
    // picking tasks. Short-circuit to empty projections so a skill
    // reading `ready_tasks` nonempty + `next_action.kind: plan` never
    // gets mixed signals. Mirrors the guard at
    // `cli/plan/waves.rs:66-79`.
    let (wave, reviews_ready, repair_targets) = if state.plan.locked {
        (
            next_ready_wave(tasks, state),
            ready_reviews(tasks, state),
            dirty_repair_targets(tasks, state),
        )
    } else {
        (None, Vec::new(), Vec::new())
    };

    let next_action = derive_next_action(
        state,
        verdict,
        close_ready,
        wave.as_ref(),
        &reviews_ready,
        &repair_targets,
    );
    let ready_tasks = wave
        .as_ref()
        .map(|w| w.tasks.iter().map(|t| t.id.clone()).collect::<Vec<_>>())
        .unwrap_or_default();
    let parallel_safe = wave.as_ref().is_some_and(|w| w.parallel_safe);
    let parallel_blockers = wave
        .as_ref()
        .map(|w| w.blockers.clone())
        .unwrap_or_default();
    let review_required: Vec<Value> = reviews_ready
        .iter()
        .map(|(id, targets)| json!({ "task_id": id, "targets": targets }))
        .collect();

    json!({
        "phase": state.phase,
        "verdict": verdict.as_str(),
        "loop": state.loop_,
        "next_action": next_action,
        "ready_tasks": ready_tasks,
        "parallel_safe": parallel_safe,
        "parallel_blockers": parallel_blockers,
        "review_required": review_required,
        "replan_required": state.replan.triggered,
        "close_ready": close_ready,
        "outcome_ratified": state.outcome.ratified,
        "plan_locked": state.plan.locked,
        "stop": stop,
    })
}

/// Populate `stop.{allow,reason,message}` consistently with
/// `readiness::stop_allowed`.
///
/// `terminal` beats `paused`/`idle`/`active_loop` so the message stays
/// accurate after mission close. When the loop is active-and-unpaused
/// but the verdict already allows stop (NeedsUser, MissionCloseReviewPassed),
/// we surface `idle` so the message does not contradict `allow=true`.
fn stop_projection(state: &MissionState, verdict: Verdict, close_ready: bool) -> Value {
    let base_allow = readiness::stop_allowed(state);
    let active_close_blocked = state.loop_.active
        && !state.loop_.paused
        && matches!(verdict, Verdict::MissionCloseReviewPassed)
        && !close_ready;
    let allow = base_allow && !active_close_blocked;
    let (reason, message) = if matches!(verdict, Verdict::TerminalComplete) {
        ("terminal", "Mission is terminal; stop is allowed.")
    } else if !state.loop_.active {
        ("idle", "Loop is inactive; stop is allowed.")
    } else if state.loop_.paused {
        (
            "paused",
            "Loop is paused; stop is allowed. Use $execute or loop resume to continue.",
        )
    } else if allow {
        // Loop says active-unpaused, but the verdict already permits
        // stop (needs_user, mission_close_review_passed). Surface it as
        // "idle" so allow=true and the message agree.
        ("idle", "Loop is active but verdict allows stop.")
    } else {
        (
            "active_loop",
            "Active loop in progress. Use $close to pause or finish the next action.",
        )
    };
    json!({
        "allow": allow,
        "reason": reason,
        "message": message,
    })
}

/// First-match-wins next-action derivation. Branches (in order):
/// invalid_state → terminal → outcome → plan → replan → repair →
/// mission-close verdicts → next wave → blocked. The order is the
/// contract — moving a branch changes what skills do.
fn derive_next_action(
    state: &MissionState,
    verdict: Verdict,
    close_ready: bool,
    wave: Option<&ReadyWave>,
    reviews_ready: &[(String, Vec<String>)],
    repair_targets: &[String],
) -> Value {
    if matches!(verdict, Verdict::InvalidState) {
        return json!({
            "kind": "fix_state",
            "message": "STATE.json appears inconsistent. Run `codex1 doctor` and inspect EVENTS.jsonl.",
        });
    }
    if matches!(verdict, Verdict::TerminalComplete) {
        return json!({
            "kind": "closed",
            "hint": "Mission is terminal.",
        });
    }
    if !state.outcome.ratified {
        return json!({
            "kind": "clarify",
            "command": "$clarify",
            "hint": "Ratify OUTCOME.md before planning.",
        });
    }
    if !state.plan.locked {
        return json!({
            "kind": "plan",
            "command": "$plan",
            "hint": "Draft and lock PLAN.yaml.",
        });
    }
    if state.replan.triggered {
        return json!({
            "kind": "replan",
            "command": "$plan replan",
            "reason": state.replan.triggered_reason.clone().unwrap_or_default(),
        });
    }
    if !repair_targets.is_empty() {
        return json!({
            "kind": "repair",
            "task_ids": repair_targets,
            "command": "$execute (repair)",
        });
    }
    match verdict {
        Verdict::ReadyForMissionCloseReview => {
            return json!({
                "kind": "mission_close_review",
                "command": "$review-loop (mission-close)",
                "hint": "All tasks complete; run the mission-close review.",
            });
        }
        Verdict::MissionCloseReviewOpen => {
            return json!({
                "kind": "mission_close_review",
                "command": "$review-loop",
                "hint": "Mission-close review is open.",
            });
        }
        Verdict::MissionCloseReviewPassed => {
            if close_ready {
                return json!({
                    "kind": "close",
                    "command": "codex1 close complete",
                    "hint": "Mission-close review passed; finalize close.",
                });
            }
            return json!({
                "kind": "blocked",
                "reason": "Mission-close review passed, but close check has blockers.",
            });
        }
        _ => {}
    }
    if let Some((review_id, targets)) = reviews_ready.first() {
        return json!({
            "kind": "run_review",
            "review_task_id": review_id,
            "targets": targets,
            "command": "$review-loop",
        });
    }
    if let Some(w) = wave {
        let review_in_wave = reviews_ready
            .iter()
            .find(|(id, _)| w.tasks.iter().any(|t| &t.id == id));
        if let Some((review_id, targets)) = review_in_wave {
            return json!({
                "kind": "run_review",
                "review_task_id": review_id,
                "targets": targets,
                "command": "$review-loop",
            });
        }
        if w.tasks.len() == 1 {
            let only = &w.tasks[0];
            return json!({
                "kind": "run_task",
                "task_id": only.id,
                "task_kind": only.kind.clone().unwrap_or_else(|| "work".to_string()),
                "command": "$execute",
            });
        }
        let ids: Vec<String> = w.tasks.iter().map(|t| t.id.clone()).collect();
        return json!({
            "kind": "run_wave",
            "wave_id": w.wave_id.clone(),
            "tasks": ids,
            "parallel_safe": w.parallel_safe,
            "parallel_blockers": w.blockers.clone(),
            "hint": format!("Run wave {} with $execute.", w.wave_id),
        });
    }
    // Distinguish an empty/missing plan from a plan that deadlocked on
    // work: if any task is `awaiting_review` with no ready review
    // target, surface the concrete block instead of the "plan may be
    // missing" message. Matches the e2e audit remediation at
    // `docs/audits/round-1/e2e-walkthrough.md` P2-1b.
    let awaiting: Vec<&String> = state
        .tasks
        .iter()
        .filter(|(_, r)| matches!(r.status, crate::state::schema::TaskStatus::AwaitingReview))
        .map(|(id, _)| id)
        .collect();
    if !awaiting.is_empty() {
        let ids = awaiting
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return json!({
            "kind": "blocked",
            "reason": format!(
                "tasks {ids} are awaiting review but no review task is ready; check the DAG for review-loop deadlock"
            ),
        });
    }
    json!({
        "kind": "blocked",
        "reason": "No ready wave derivable — PLAN.yaml may be missing, empty, or inconsistent with STATE.json.",
    })
}
