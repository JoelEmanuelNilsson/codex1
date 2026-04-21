//! `codex1 review start <id>` — begin a planned review.
//!
//! Preconditions:
//! - Mission not terminal.
//! - PLAN.yaml task `<id>` exists and `kind == review`.
//! - Every target in `review_target.tasks` is `Complete` or `AwaitingReview`.
//!
//! Mutation:
//! - Insert a `Pending` `ReviewRecord` into `state.reviews[<id>]`.
//! - `boundary_revision` = the post-write revision (`state.revision + 1`
//!   inside the closure) so that `review record` run immediately after
//!   `review start` classifies as `accepted_current`.
//! - Append `review.started` event.

use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cli::review::plan_read::{fetch_review_task, load_tasks, review_targets};
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::schema::{ReviewRecord, ReviewRecordCategory, ReviewVerdict, TaskStatus};
use crate::state::{self};

pub fn run(ctx: &Ctx, task_id: &str) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    state::check_expected_revision(ctx.expect_revision, &state)?;
    let plan_tasks = load_tasks(&paths)?;
    let review_task = fetch_review_task(&plan_tasks, task_id)?;
    let targets = review_targets(&review_task)?;
    if let Some(closed_at) = state.close.terminal_at.as_ref() {
        return Err(CliError::TerminalAlreadyComplete {
            closed_at: closed_at.clone(),
        });
    }
    // Refuse to start a review while the plan is unlocked (e.g. during
    // a pending replan). See `state::require_plan_locked` for rationale.
    state::require_plan_locked(&state)?;
    ensure_review_deps_ready(&state, &review_task)?;
    if let Some(existing) = state.reviews.get(task_id) {
        if !matches!(existing.verdict, ReviewVerdict::Pending) {
            ensure_dirty_review_repaired(&state, task_id, &targets)?;
        }
    }
    for tid in &targets {
        let Some(task) = state.tasks.get(tid) else {
            return Err(CliError::TaskNotReady {
                message: format!("Review target {tid} is not tracked in STATE.json"),
            });
        };
        match task.status {
            TaskStatus::Complete | TaskStatus::AwaitingReview | TaskStatus::Superseded => {}
            TaskStatus::Pending | TaskStatus::Ready | TaskStatus::InProgress => {
                return Err(CliError::TaskNotReady {
                    message: format!(
                        "Target {tid} is `{:?}`; review start requires Complete or AwaitingReview",
                        task.status
                    ),
                });
            }
        }
    }

    if ctx.dry_run {
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "dry_run": true,
                "review_task_id": task_id,
                "targets": targets,
                "would": "set state.reviews[<id>] = Pending with boundary_revision",
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let review_task_id = task_id.to_string();
    let targets_for_event = targets.clone();
    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        "review.started",
        json!({
            "review_task_id": review_task_id,
            "targets": targets_for_event,
        }),
        |state| {
            // Re-check `plan.locked` under the exclusive lock to close
            // the TOCTOU between the pre-mutate shared-lock load and
            // this closure. See round-2 correctness P1-1.
            state::require_plan_locked(state)?;
            ensure_review_deps_ready(state, &review_task)?;
            if let Some(existing) = state.reviews.get(&review_task_id) {
                if !matches!(existing.verdict, ReviewVerdict::Pending) {
                    ensure_dirty_review_repaired(state, &review_task_id, &targets_for_event)?;
                }
            }
            // boundary_revision is the revision the state will take AFTER
            // this mutation is persisted (closure runs pre-bump, so +1).
            let boundary_revision = state.revision.saturating_add(1);
            let recorded_at = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
            state.reviews.insert(
                review_task_id.clone(),
                ReviewRecord {
                    task_id: review_task_id.clone(),
                    verdict: ReviewVerdict::Pending,
                    reviewers: Vec::new(),
                    findings_file: None,
                    category: ReviewRecordCategory::AcceptedCurrent,
                    recorded_at,
                    boundary_revision,
                },
            );
            Ok(())
        },
    )?;

    let env = JsonOk::new(
        Some(mutation.state.mission_id.clone()),
        Some(mutation.new_revision),
        json!({
            "review_task_id": task_id,
            "verdict": "pending",
            "targets": targets,
            "boundary_revision": mutation.new_revision,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

pub(crate) fn ensure_review_deps_ready(
    state: &state::MissionState,
    review_task: &crate::cli::review::plan_read::PlanTask,
) -> Result<(), CliError> {
    for dep in &review_task.depends_on {
        let Some(task) = state.tasks.get(dep) else {
            return Err(CliError::TaskNotReady {
                message: format!("Review dependency {dep} is not tracked in STATE.json"),
            });
        };
        if !matches!(
            task.status,
            TaskStatus::Complete | TaskStatus::AwaitingReview
        ) {
            return Err(CliError::TaskNotReady {
                message: format!(
                    "Review dependency {dep} is `{:?}`; review start requires Complete or AwaitingReview",
                    task.status
                ),
            });
        }
    }
    Ok(())
}

fn ensure_dirty_review_repaired(
    state: &state::MissionState,
    review_task_id: &str,
    targets: &[String],
) -> Result<(), CliError> {
    let Some(existing) = state.reviews.get(review_task_id) else {
        return Ok(());
    };
    if !matches!(existing.verdict, ReviewVerdict::Dirty) {
        return Err(CliError::ReviewFindingsBlock {
            message: format!(
                "review {review_task_id} already has `{}` verdict",
                crate::cli::review::classify::verdict_str(&existing.verdict)
            ),
        });
    }
    let repaired = targets.iter().all(|tid| {
        state
            .tasks
            .get(tid)
            .and_then(|task| task.finished_at.as_deref())
            .is_some_and(|finished_at| finished_at > existing.recorded_at.as_str())
    });
    if repaired {
        Ok(())
    } else {
        Err(CliError::ReviewFindingsBlock {
            message: format!(
                "review {review_task_id} has dirty findings; finish a repair for every target before restarting"
            ),
        })
    }
}
