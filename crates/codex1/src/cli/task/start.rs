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
use crate::state::{self, schema::ReviewRecordCategory, schema::ReviewVerdict, schema::TaskStatus};

use super::lifecycle::{deps_satisfied, ensure_task_record, load_plan, now_rfc3339, status_str};

pub fn run(task_id: &str, ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    state::check_expected_revision(ctx.expect_revision, &state)?;
    // Refuse to start tasks while the plan is unlocked (e.g. during a
    // pending replan). See `state::require_plan_locked` for rationale.
    state::require_plan_locked(&state)?;
    let plan = load_plan(&paths)?;

    let Some(plan_task) = plan.get(task_id) else {
        return Err(CliError::TaskNotReady {
            message: format!("Task `{task_id}` not found in PLAN.yaml"),
        });
    };
    if plan_task.kind == "review" {
        return Err(CliError::TaskNotReady {
            message: format!(
                "Task `{task_id}` is a review task; use `codex1 review start {task_id}`"
            ),
        });
    }

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

    let repair_start = matches!(current_status, TaskStatus::AwaitingReview)
        && is_dirty_repair_target(&plan, &state, task_id);

    // Only Pending, Ready, or an advertised dirty-review repair target may transition.
    if !matches!(current_status, TaskStatus::Pending | TaskStatus::Ready) && !repair_start {
        return Err(CliError::TaskNotReady {
            message: format!(
                "Task `{task_id}` has status `{}`; cannot start",
                status_str(&current_status)
            ),
        });
    }

    if ctx.dry_run {
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
        state::mutate_dynamic_maybe(&paths, ctx.expect_revision, move |state| {
            // Re-check `plan.locked` under the exclusive lock to
            // close the TOCTOU between the pre-mutate shared-lock
            // load and this closure: a concurrent `replan record`
            // landing in that window could otherwise produce
            // `!plan.locked && task.status == InProgress`. See
            // round-2 correctness P1-1.
            state::require_plan_locked(state)?;
            let rec = ensure_task_record(state, &task_id);
            if matches!(rec.status, TaskStatus::InProgress) {
                return Ok(None);
            }
            rec.status = TaskStatus::InProgress;
            rec.started_at = Some(started_at);
            Ok(Some((
                "task.started".to_string(),
                json!({ "task_id": task_id, "started_at": rec.started_at }),
            )))
        })?
    };

    let (state_for_env, revision, idempotent) = match mutation {
        state::MaybeMutation::Mutated(m) => (m.state, Some(m.new_revision), false),
        state::MaybeMutation::Unchanged(s) => (s, None, true),
    };
    let started_at = state_for_env
        .tasks
        .get(task_id)
        .and_then(|r| r.started_at.clone())
        .unwrap_or(started_at);
    let env = JsonOk::new(
        Some(state_for_env.mission_id.clone()),
        revision.or(Some(state_for_env.revision)),
        json!({
            "task_id": task_id,
            "status": "in_progress",
            "started_at": started_at,
            "idempotent": idempotent,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

fn is_dirty_repair_target(
    plan: &super::lifecycle::ParsedPlan,
    state: &crate::state::schema::MissionState,
    task_id: &str,
) -> bool {
    if state.replan.triggered {
        return false;
    }
    state.reviews.iter().any(|(review_id, record)| {
        matches!(record.verdict, ReviewVerdict::Dirty)
            && matches!(record.category, ReviewRecordCategory::AcceptedCurrent)
            && plan
                .get(review_id)
                .and_then(|task| task.review_target.as_ref())
                .is_some_and(|target| target.tasks.iter().any(|id| id == task_id))
    })
}
