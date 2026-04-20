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
pub fn build(state: &MissionState, tasks: &[PlanTask]) -> Value {
    let verdict = readiness::derive_verdict(state);
    let close_ready = readiness::close_ready(state);
    let stop = stop_projection(state, verdict);

    let wave = next_ready_wave(tasks, state);
    let reviews_ready = ready_reviews(tasks, state);
    let repair_targets = dirty_repair_targets(tasks, state);

    let next_action = derive_next_action(
        state,
        verdict,
        wave.as_ref(),
        &reviews_ready,
        &repair_targets,
    );
    let ready_tasks = wave
        .as_ref()
        .map(|w| w.tasks.iter().map(|t| t.id.clone()).collect::<Vec<_>>())
        .unwrap_or_default();
    // A wave with any ready tasks is considered parallel-safe until a
    // future unit adds resource-conflict analysis (exclusive_resources,
    // unknown_side_effects). `parallel_blockers` will carry any
    // detected conflicts once that logic lands.
    let parallel_safe = wave.as_ref().is_some_and(|w| !w.tasks.is_empty());
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
        "parallel_blockers": Vec::<String>::new(),
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
fn stop_projection(state: &MissionState, verdict: Verdict) -> Value {
    let allow = readiness::stop_allowed(state);
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
            return json!({
                "kind": "close",
                "command": "codex1 close complete",
                "hint": "Mission-close review passed; finalize close.",
            });
        }
        _ => {}
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
            "parallel_safe": true,
            "hint": format!("Run wave {} with $execute.", w.wave_id),
        });
    }
    json!({
        "kind": "blocked",
        "reason": "No ready wave derivable — PLAN.yaml may be missing, empty, or inconsistent with STATE.json.",
    })
}
