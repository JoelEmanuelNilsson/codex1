//! `codex1 task next` — report the next ready task / wave / review / close.

use serde_json::json;

use crate::cli::close::check::ReadinessReport;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::state::{self, schema::ReviewRecordCategory, schema::ReviewVerdict, schema::TaskStatus};

use super::lifecycle::{
    all_tasks_terminal, awaiting_review_targets, effective_tasks, load_plan, next_ready_review,
    ready_wave,
};

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;

    // Short-circuit on missing clarified outcome, unlocked plan, and
    // pending replan so `task next`
    // agrees with `status.next_action`. Without these, a skill calling
    // `task next` directly would be handed a wave while `status` is
    // telling it to `$plan` or `$plan replan` — two canonical readiness
    // endpoints disagreeing. See round-2 e2e P2-1 and the round-1 P2-2
    // fix at `cli/status/project.rs::build`.
    if !state.outcome.ratified {
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "next": {
                    "kind": "clarify",
                    "hint": "Ratify OUTCOME.md before planning.",
                }
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }
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

    let plan = load_plan(&paths, &state)?;
    let effective = effective_tasks(&plan, &state);

    let next = if all_tasks_terminal(&plan, &state) {
        if state.close.terminal_at.is_some() {
            json!({
                "kind": "closed",
                "hint": "Mission is terminal.",
            })
        } else if matches!(
            state.close.review_state,
            crate::state::schema::MissionCloseReviewState::Passed
        ) {
            let report = ReadinessReport::from_state_and_paths(&state, &paths);
            if report.blockers.is_empty() {
                json!({
                    "kind": "close",
                    "hint": "Mission-close review passed; finalize close.",
                })
            } else {
                json!({
                    "kind": "blocked",
                    "reason": "Mission-close review passed, but close check has blockers.",
                })
            }
        } else {
            json!({
                "kind": "mission_close_review",
                "reason": "all tasks complete or superseded",
            })
        }
    } else {
        if let Some(task_id) = dirty_repair_target(&plan, &state) {
            let env = JsonOk::new(
                Some(state.mission_id.clone()),
                Some(state.revision),
                json!({
                    "next": {
                        "kind": "repair",
                        "task_id": task_id,
                        "reason": "accepted-current review findings must be repaired before rerunning review",
                    }
                }),
            );
            println!("{}", env.to_pretty());
            return Ok(());
        }
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
                    let (parallel_safe, blockers) = parallel_safety(&ready);
                    json!({
                        "kind": "run_wave",
                        "wave_id": wave_id_for(&effective, &ready),
                        "tasks": ids,
                        "parallel_safe": parallel_safe,
                        "parallel_blockers": blockers,
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

fn dirty_repair_target(
    plan: &super::lifecycle::ParsedPlan,
    state: &crate::state::schema::MissionState,
) -> Option<String> {
    for (review_id, record) in &state.reviews {
        if !matches!(record.verdict, ReviewVerdict::Dirty)
            || !matches!(record.category, ReviewRecordCategory::AcceptedCurrent)
        {
            continue;
        }
        let Some(target) = plan
            .get(review_id)
            .and_then(|task| task.review_target.as_ref())
        else {
            continue;
        };
        for task_id in &target.tasks {
            if state.tasks.get(task_id).is_some_and(|task| {
                matches!(task.status, TaskStatus::AwaitingReview)
                    && task
                        .finished_at
                        .as_deref()
                        .is_none_or(|finished_at| finished_at <= record.recorded_at.as_str())
            }) {
                return Some(task_id.clone());
            }
        }
    }
    None
}

/// Assign a deterministic id to the current ready wave.
/// Depth(T) = 1 if `depends_on` is empty, else 1 + max(Depth(d) for d in deps).
/// All tasks in a ready wave share the same depth, so we take the min.
fn wave_id_for(
    effective: &[super::lifecycle::EffectiveTask],
    ready: &[super::lifecycle::EffectiveTask],
) -> String {
    use std::collections::{BTreeMap, BTreeSet};
    let ids: BTreeSet<&str> = effective.iter().map(|t| t.id.as_str()).collect();
    let mut depth_of: BTreeMap<String, usize> = BTreeMap::new();
    for root in effective.iter().filter(|t| t.depends_on.is_empty()) {
        depth_of.insert(root.id.clone(), 1);
    }
    let max_iters = effective
        .len()
        .saturating_mul(effective.len())
        .saturating_add(1);
    for _ in 0..max_iters {
        let mut changed = false;
        for t in effective {
            if t.depends_on.is_empty() || !t.depends_on.iter().all(|d| ids.contains(d.as_str())) {
                continue;
            }
            let Some(parent_depth) = t
                .depends_on
                .iter()
                .map(|d| depth_of.get(d).copied())
                .collect::<Option<Vec<_>>>()
                .and_then(|ds| ds.into_iter().max())
            else {
                continue;
            };
            let next_depth = parent_depth.saturating_add(1);
            if depth_of.get(&t.id).copied().unwrap_or(0) < next_depth {
                depth_of.insert(t.id.clone(), next_depth);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if depth_of.len() != effective.len() {
        return "W1".to_string();
    }
    let mut depths: Vec<usize> = effective
        .iter()
        .filter_map(|t| depth_of.get(&t.id).copied())
        .collect();
    depths.sort_unstable();
    depths.dedup();
    let ready_depth = ready
        .iter()
        .filter_map(|t| depth_of.get(&t.id).copied())
        .min()
        .unwrap_or(1);
    let wave_index = depths
        .iter()
        .position(|d| *d == ready_depth)
        .map_or(ready_depth, |idx| idx + 1);
    format!("W{wave_index}")
}

fn parallel_safety(ready: &[super::lifecycle::EffectiveTask]) -> (bool, Vec<String>) {
    use std::collections::{BTreeMap, BTreeSet};
    let mut blockers = Vec::new();
    let mut seen = BTreeSet::new();
    let mut owners: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for task in ready {
        for resource in &task.exclusive_resources {
            owners
                .entry(resource.as_str())
                .or_default()
                .push(task.id.as_str());
        }
    }
    for (resource, task_ids) in owners {
        if task_ids.len() > 1 {
            let key = format!("exclusive_resource:{resource}");
            if seen.insert(key.clone()) {
                blockers.push(key);
            }
        }
    }
    for task in ready {
        if task.unknown_side_effects {
            let key = format!("unknown_side_effects:{}", task.id);
            if seen.insert(key.clone()) {
                blockers.push(key);
            }
        }
    }
    (blockers.is_empty(), blockers)
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
