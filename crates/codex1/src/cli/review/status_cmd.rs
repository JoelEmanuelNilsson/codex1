//! `codex1 review status <id>` — read-only review record projection.
//!
//! Returns the current record if any, plus target states and dirty-streak
//! counters for the main thread to surface in the unified `status` view.

use serde_json::{json, Value};

use crate::cli::review::classify::{category_str, verdict_str};
use crate::cli::review::plan_read::{fetch_review_task, load_tasks, review_targets};
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::state::{self};

pub fn run(ctx: &Ctx, task_id: &str) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let plan_tasks = load_tasks(&paths)?;
    let review_task = fetch_review_task(&plan_tasks, task_id)?;
    let targets = review_targets(&review_task)?;

    let record_value = state.reviews.get(task_id).map_or(Value::Null, |r| {
        json!({
            "verdict": verdict_str(&r.verdict),
            "reviewers": r.reviewers,
            "findings_file": r.findings_file,
            "category": category_str(r.category.clone()),
            "recorded_at": r.recorded_at,
            "boundary_revision": r.boundary_revision,
        })
    });

    let target_states: Vec<Value> = targets
        .iter()
        .map(|tid| {
            let status = state.tasks.get(tid).map(|t| format!("{:?}", t.status));
            let streak = state
                .replan
                .consecutive_dirty_by_target
                .get(tid)
                .copied()
                .unwrap_or(0);
            json!({
                "task_id": tid,
                "status": status,
                "consecutive_dirty": streak,
            })
        })
        .collect();

    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        json!({
            "review_task_id": task_id,
            "record": record_value,
            "targets": target_states,
            "replan_triggered": state.replan.triggered,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}
