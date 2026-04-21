//! `codex1 replan record` — record a replan decision.
//!
//! Effects (non-dry-run):
//! - Each `--supersedes <id>` task is set to `TaskStatus::Superseded` with
//!   `superseded_by: Some("replan-<revision>")`.
//! - `state.replan.consecutive_dirty_by_target` is cleared.
//! - `state.replan.triggered = true` and `triggered_reason` is recorded.
//! - `state.plan.locked = false` so `plan check` must re-run.
//! - `state.phase = Phase::Plan` (unconditional).
//! - One `replan.recorded` event is appended to EVENTS.jsonl.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::cli::replan::triggers;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::{
    self,
    schema::{MissionState, Phase, TaskRecord, TaskStatus},
};

pub fn run(ctx: &Ctx, reason: &str, supersedes: &[String]) -> CliResult<()> {
    if !triggers::ALLOWED_REASONS.contains(&reason) {
        return Err(CliError::PlanInvalid {
            message: format!("Unknown replan reason '{reason}'"),
            hint: Some(format!(
                "Use one of: {}.",
                triggers::ALLOWED_REASONS.join(", ")
            )),
        });
    }

    let paths = resolve_mission(&ctx.selector(), true)?;

    if ctx.dry_run {
        let state = state::load(&paths)?;
        if let Some(expected) = ctx.expect_revision {
            if expected != state.revision {
                return Err(CliError::RevisionConflict {
                    expected,
                    actual: state.revision,
                });
            }
        }
        if let Some(closed_at) = state.close.terminal_at.clone() {
            return Err(CliError::TerminalAlreadyComplete { closed_at });
        }
        validate_supersedes(&state, supersedes)?;
        emit_result(&state.mission_id, state.revision, reason, supersedes, true);
        return Ok(());
    }

    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        "replan.recorded",
        json!({ "reason": reason, "supersedes": supersedes }),
        |state| {
            if let Some(closed_at) = state.close.terminal_at.clone() {
                return Err(CliError::TerminalAlreadyComplete { closed_at });
            }
            validate_supersedes(state, supersedes)?;
            apply_replan(state, reason, supersedes);
            Ok(())
        },
    )?;
    emit_result(
        &mutation.state.mission_id,
        mutation.new_revision,
        reason,
        supersedes,
        false,
    );
    Ok(())
}

fn emit_result(
    mission_id: &str,
    revision: u64,
    reason: &str,
    supersedes: &[String],
    dry_run: bool,
) {
    let mut data = json!({
        "reason": reason,
        "supersedes": supersedes,
        "phase_after": "plan",
        "plan_locked": false,
    });
    if dry_run {
        if let Value::Object(map) = &mut data {
            map.insert("dry_run".to_string(), Value::Bool(true));
        }
    }
    let env = JsonOk::new(Some(mission_id.to_string()), Some(revision), data);
    println!("{}", env.to_pretty());
}

/// Verify every superseded id is present in the locked plan and is
/// currently replaceable (not already `Superseded` or `Complete`).
fn validate_supersedes(state: &MissionState, ids: &[String]) -> CliResult<()> {
    let mut unknown: Vec<String> = Vec::new();
    let mut not_supersedable: BTreeMap<String, String> = BTreeMap::new();

    for id in ids {
        let known_in_plan =
            state.tasks.contains_key(id) || state.plan.task_ids.iter().any(|plan_id| plan_id == id);
        if !known_in_plan {
            unknown.push(id.clone());
            continue;
        }
        match state.tasks.get(id) {
            None => {}
            Some(record) => match record.status {
                TaskStatus::Complete | TaskStatus::Superseded => {
                    not_supersedable.insert(id.clone(), status_label(&record.status));
                }
                _ => {}
            },
        }
    }

    if !unknown.is_empty() {
        return Err(CliError::PlanInvalid {
            message: format!("Unknown --supersedes task ids: {}", unknown.join(", ")),
            hint: Some("Pass only task ids present in the locked PLAN.yaml.".to_string()),
        });
    }

    if !not_supersedable.is_empty() {
        let list: Vec<String> = not_supersedable
            .iter()
            .map(|(id, status)| format!("{id} ({status})"))
            .collect();
        return Err(CliError::PlanInvalid {
            message: format!("Tasks cannot be superseded: {}", list.join(", ")),
            hint: Some(
                "Only Pending / Ready / InProgress / AwaitingReview tasks may be superseded."
                    .to_string(),
            ),
        });
    }

    Ok(())
}

fn apply_replan(state: &mut MissionState, reason: &str, supersedes: &[String]) {
    let marker = format!("replan-{}", state.revision);
    for id in supersedes {
        let record = state.tasks.entry(id.clone()).or_insert_with(|| TaskRecord {
            id: id.clone(),
            status: TaskStatus::Pending,
            started_at: None,
            finished_at: None,
            proof_path: None,
            superseded_by: None,
        });
        record.status = TaskStatus::Superseded;
        record.superseded_by = Some(marker.clone());
    }
    state.replan.consecutive_dirty_by_target.clear();
    state.replan.triggered = true;
    state.replan.triggered_reason = Some(reason.to_string());
    state.plan.locked = false;
    state.phase = Phase::Plan;
    state.close.review_state = crate::state::schema::MissionCloseReviewState::NotStarted;
}

/// Render a `TaskStatus` the same way serde would (snake_case), so error
/// messages match the strings callers see in STATE.json.
fn status_label(status: &TaskStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{status:?}"))
}
