//! `codex1 task finish` — transition InProgress task to Complete or
//! AwaitingReview (depending on whether a review task targets it).

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::{self, schema::TaskStatus};

use super::lifecycle::{ensure_task_record, load_plan, now_rfc3339, status_str};

pub fn run(task_id: &str, proof: &Path, ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    // Refuse to finish tasks while the plan is unlocked (e.g. during a
    // pending replan). See `state::require_plan_locked` for rationale.
    state::require_plan_locked(&state)?;
    let plan = load_plan(&paths)?;

    // Validate the task exists in PLAN.yaml.
    if plan.get(task_id).is_none() {
        return Err(CliError::TaskNotReady {
            message: format!("Task `{task_id}` not found in PLAN.yaml"),
        });
    }

    // Resolve proof path. Absolute paths are taken verbatim; relative
    // paths are joined against mission_dir.
    let proof_abs: PathBuf = if proof.is_absolute() {
        proof.to_path_buf()
    } else {
        paths.mission_dir.join(proof)
    };
    if !proof_abs.is_file() {
        return Err(CliError::ProofMissing {
            path: proof_abs.clone(),
        });
    }

    let current_status = state
        .tasks
        .get(task_id)
        .map_or(TaskStatus::Pending, |r| r.status.clone());

    if !matches!(current_status, TaskStatus::InProgress) {
        return Err(CliError::TaskNotReady {
            message: format!(
                "Task `{task_id}` has status `{}`; only in_progress tasks can be finished",
                status_str(&current_status)
            ),
        });
    }

    // Decide next status: AwaitingReview iff a review task targets this task.
    let has_review = plan.review_task_targeting(task_id).is_some();
    let next_status = if has_review {
        TaskStatus::AwaitingReview
    } else {
        TaskStatus::Complete
    };
    let next_status_str = status_str(&next_status);

    // Store proof path relative to mission dir when possible.
    let proof_display = match proof_abs.strip_prefix(&paths.mission_dir) {
        Ok(rel) => rel.display().to_string(),
        Err(_) => proof_abs.display().to_string(),
    };

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
                    "to": next_status_str,
                },
                "proof_path": proof_display,
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let finished_at = now_rfc3339();
    let mutation = {
        let task_id = task_id.to_string();
        let finished_at = finished_at.clone();
        let proof_display = proof_display.clone();
        let next_status = next_status.clone();
        state::mutate(
            &paths,
            ctx.expect_revision,
            "task.finished",
            json!({
                "task_id": task_id,
                "finished_at": finished_at,
                "proof_path": proof_display,
                "next_status": next_status_str,
            }),
            move |state| {
                // Re-check `plan.locked` under the exclusive lock to
                // close the TOCTOU between the pre-mutate shared-lock
                // load and this closure. See round-2 correctness P1-1.
                state::require_plan_locked(state)?;
                let rec = ensure_task_record(state, &task_id);
                rec.status = next_status;
                rec.finished_at = Some(finished_at);
                rec.proof_path = Some(proof_display);
                Ok(())
            },
        )?
    };

    let env = JsonOk::new(
        Some(mutation.state.mission_id.clone()),
        Some(mutation.new_revision),
        json!({
            "task_id": task_id,
            "status": next_status_str,
            "finished_at": finished_at,
            "proof_path": proof_display,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}
