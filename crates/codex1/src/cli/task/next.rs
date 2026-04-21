//! `codex1 task next` — report the next ready task / wave / review / close.

use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::state;

use super::lifecycle::{
    all_tasks_terminal, awaiting_review_targets, effective_tasks, load_plan, next_ready_review,
    ready_wave,
};

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;

    // Short-circuit on unlocked plan and pending replan so `task next`
    // agrees with `status.next_action`. Without these, a skill calling
    // `task next` directly would be handed a wave while `status` is
    // telling it to `$plan` or `$plan replan` — two canonical readiness
    // endpoints disagreeing. See round-2 e2e P2-1 and the round-1 P2-2
    // fix at `cli/status/project.rs::build`.
    if !state.plan.locked {
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "next": {
                    "kind": "plan",
                    "hint": "Draft and lock PLAN.yaml.",
                }
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }
    if state.replan.triggered {
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "next": {
                    "kind": "replan",
                    "reason": state.replan.triggered_reason.clone().unwrap_or_default(),
                }
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let plan = load_plan(&paths)?;
    let effective = effective_tasks(&plan, &state);

    let next = if all_tasks_terminal(&plan, &state) {
        json!({
            "kind": "mission_close_review",
            "reason": "all tasks complete or superseded",
        })
    } else {
        let awaiting = awaiting_review_targets(&effective);
        if let Some((review_task, covered)) = next_ready_review(&plan, &effective, &awaiting) {
            json!({
                "kind": "run_review",
                "task_id": review_task.id,
                "targets": covered,
            })
        } else {
            let ready = ready_wave(&effective);
            match ready.len() {
                0 => json!({
                    "kind": "blocked",
                    "reason": blocker_reason(&state, &effective),
                }),
                1 => {
                    let t = &ready[0];
                    json!({
                        "kind": "run_task",
                        "task_id": t.id,
                        "task_kind": t.kind,
                    })
                }
                _ => {
                    let ids: Vec<String> = ready.iter().map(|t| t.id.clone()).collect();
                    json!({
                        "kind": "run_wave",
                        "wave_id": wave_id_for(&effective, &ready),
                        "tasks": ids,
                        "parallel_safe": true,
                    })
                }
            }
        }
    };

    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        json!({ "next": next }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

/// Assign a deterministic id to the current ready wave.
/// Depth(T) = 1 if `depends_on` is empty, else 1 + max(Depth(d) for d in deps).
/// All tasks in a ready wave share the same depth, so we take the min.
fn wave_id_for(
    effective: &[super::lifecycle::EffectiveTask],
    ready: &[super::lifecycle::EffectiveTask],
) -> String {
    use std::collections::BTreeMap;
    let mut depth_of: BTreeMap<String, usize> = BTreeMap::new();
    for t in effective {
        let d = if t.depends_on.is_empty() {
            1
        } else {
            t.depends_on
                .iter()
                .map(|d| depth_of.get(d).copied().unwrap_or(1))
                .max()
                .unwrap_or(0)
                + 1
        };
        depth_of.insert(t.id.clone(), d);
    }
    let ready_depth = ready
        .iter()
        .map(|t| depth_of.get(&t.id).copied().unwrap_or(1))
        .min()
        .unwrap_or(1);
    format!("W{ready_depth}")
}

/// Human-readable reason when no ready wave and no ready review.
fn blocker_reason(
    state: &crate::state::schema::MissionState,
    effective: &[super::lifecycle::EffectiveTask],
) -> String {
    use crate::state::schema::TaskStatus;
    if !state.outcome.ratified {
        return "OUTCOME.md is not ratified".to_string();
    }
    if !state.plan.locked {
        return "PLAN.yaml is not locked".to_string();
    }
    if effective.is_empty() {
        return "PLAN.yaml has no tasks".to_string();
    }
    let in_progress: Vec<&str> = effective
        .iter()
        .filter(|t| matches!(t.status, TaskStatus::InProgress))
        .map(|t| t.id.as_str())
        .collect();
    if !in_progress.is_empty() {
        return format!("tasks in progress: {}", in_progress.join(", "));
    }
    let awaiting: Vec<&str> = effective
        .iter()
        .filter(|t| matches!(t.status, TaskStatus::AwaitingReview))
        .map(|t| t.id.as_str())
        .collect();
    if !awaiting.is_empty() {
        return format!(
            "tasks awaiting review without a ready review task: {}",
            awaiting.join(", ")
        );
    }
    "no ready tasks and no ready review task".to_string()
}
