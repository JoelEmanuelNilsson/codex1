//! `codex1 task start` — transition a task to `InProgress`.
//!
//! - Idempotent on re-entry (no second event emitted).
//! - Refuses if dependencies are not complete/superseded.
//! - Respects `--dry-run` and `--expect-revision`.

use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::{self, schema::TaskStatus};

use super::lifecycle::{deps_satisfied, ensure_task_record, load_plan, now_rfc3339, status_str};

pub fn run(task_id: &str, ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    // Refuse to start tasks while the plan is unlocked (e.g. during a
    // pending replan). See `state::require_plan_locked` for rationale.
    state::require_plan_locked(&state)?;
    let plan = load_plan(&paths)?;

    let Some(plan_task) = plan.get(task_id) else {
        return Err(CliError::TaskNotReady {
            message: format!("Task `{task_id}` not found in PLAN.yaml"),
        });
    };

    if !deps_satisfied(plan_task, &state) {
        return Err(CliError::TaskNotReady {
            message: format!(
                "Task `{task_id}` has incomplete dependencies: {}",
                plan_task.depends_on.join(", ")
            ),
        });
    }

    let record = state.tasks.get(task_id);
    let current_status = record.map_or(TaskStatus::Pending, |r| r.status.clone());

    // Idempotent: already in progress → no mutation.
    if matches!(current_status, TaskStatus::InProgress) {
        // Stale-writer protection applies even when the call does no
        // work — otherwise `--expect-revision` silently succeeds
        // against a state that has moved on. See
        // `docs/cli-contract-schemas.md:74` (strict equality).
        state::check_expected_revision(ctx.expect_revision, &state)?;
        let started_at = record
            .and_then(|r| r.started_at.clone())
            .unwrap_or_default();
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "task_id": task_id,
                "status": "in_progress",
                "started_at": started_at,
                "idempotent": true,
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    // Only Pending or Ready may transition.
    if !matches!(current_status, TaskStatus::Pending | TaskStatus::Ready) {
        return Err(CliError::TaskNotReady {
            message: format!(
                "Task `{task_id}` has status `{}`; cannot start",
                status_str(&current_status)
            ),
        });
    }

    if ctx.dry_run {
        state::check_expected_revision(ctx.expect_revision, &state)?;
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "dry_run": true,
                "task_id": task_id,
                "would_transition": {
                    "from": status_str(&current_status),
                    "to": "in_progress",
                }
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let started_at = now_rfc3339();
    let mutation = {
        let task_id = task_id.to_string();
        let started_at = started_at.clone();
        state::mutate(
            &paths,
            ctx.expect_revision,
            "task.started",
            json!({ "task_id": task_id, "started_at": started_at }),
            move |state| {
                // Re-check `plan.locked` under the exclusive lock to
                // close the TOCTOU between the pre-mutate shared-lock
                // load and this closure: a concurrent `replan record`
                // landing in that window could otherwise produce
                // `!plan.locked && task.status == InProgress`. See
                // round-2 correctness P1-1.
                state::require_plan_locked(state)?;
                let rec = ensure_task_record(state, &task_id);
                rec.status = TaskStatus::InProgress;
                rec.started_at = Some(started_at);
                Ok(())
            },
        )?
    };

    let env = JsonOk::new(
        Some(mutation.state.mission_id.clone()),
        Some(mutation.new_revision),
        json!({
            "task_id": task_id,
            "status": "in_progress",
            "started_at": started_at,
            "idempotent": false,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}
