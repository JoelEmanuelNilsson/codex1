//! `codex1 task next | start | finish | status`.
//!
//! * `next` — mirrors the `next_action` field of the status envelope.
//! * `start` — `Ready`/`NeedsRepair` → `InProgress`. Mints a fresh
//!   `task_run_id` (UUID v4) and sets `started_at`. Transitions stored
//!   phase to `executing`.
//! * `finish` — `InProgress` → `ProofSubmitted`. Reads the proof file,
//!   computes a `sha256:<hex>` hash, stores `proof_ref` + `proof_hash`,
//!   sets `finished_at`.
//! * `status` — single-task read; returns the full `TaskState` plus the
//!   blueprint `TaskSpec` so callers see spec + state in one payload.

use std::path::Path;

use serde_json::json;
use uuid::Uuid;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph::{self, validate::validate_id_format};
use crate::mission::lock::parse_and_validate as parse_lock;
use crate::mission::resolve_mission;
use crate::proof::{default_proof_ref, read_and_hash};
use crate::review::{BUNDLES_DIRNAME, load_all_bundles};
use crate::state::{EventDraft, Phase, StateStore, TaskState, TaskStatus};
use crate::status::project_status_with_bundles;

use super::{Cli, emit_error, emit_success, now_rfc3339, resolve_repo};

const NEXT_SCHEMA: &str = "codex1.task.next.v1";
const READY_SCHEMA: &str = "codex1.task.ready.v1";
const START_SCHEMA: &str = "codex1.task.start.v1";
const FINISH_SCHEMA: &str = "codex1.task.finish.v1";
const STATUS_SCHEMA: &str = "codex1.task.status.v1";

pub fn cmd_task_next(cli: &Cli, mission: &str) -> i32 {
    match run_next(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_next(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let (paths, dag, state) = load_mission(cli, mission)?;
    // Round 15 P1: route `task next` through the same lock-aware
    // projection `codex1 status` uses. The bare `project_status`
    // synthesizes a ratified lock and passes empty bundles, which lets
    // a draft-lock mission emit `start_task` even though `codex1
    // status` correctly routes to `user_decision/lock_not_ratified`.
    let lock = parse_lock(&paths.outcome_lock())?;
    let bundles = load_all_bundles(&paths.mission_dir.join(BUNDLES_DIRNAME))?;
    let status = project_status_with_bundles(&lock, &state, &dag, &bundles);
    let v = serde_json::to_value(&status).map_err(|e| CliError::Internal {
        message: format!("serialize status: {e}"),
    })?;
    Ok(envelope::success(
        NEXT_SCHEMA,
        &json!({
            "mission_id": mission,
            "verdict": v["verdict"],
            "next_action": v["next_action"],
            "ready_tasks": v["ready_tasks"],
            // Round 4: surface the Round 3 wave-parallel hint here too, since
            // $execute calls `task next` before `status` and would otherwise
            // never see it.
            "ready_wave_parallel_safe": v["ready_wave_parallel_safe"],
            "blocked": v["blocked"],
            "running_tasks": v["running_tasks"],
            "required_user_decision": v["required_user_decision"],
        }),
    ))
}

pub fn cmd_task_ready(cli: &Cli, mission: &str, task_id: &str) -> i32 {
    match run_ready(cli, mission, task_id) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

/// Transition `Planned` → `Ready` via `StateStore::mutate`. Round 6 Fix
/// #1: `$plan` previously hand-edited STATE.json to set task statuses;
/// that bypassed the lock + revision + events path. This command is the
/// authoritative way to mark a task ready once its spec file exists.
/// Round 10 P1: also runs the full plan-acceptance suite and requires a
/// ratified outcome lock so an unaccepted plan can't become executable
/// repo truth by going through this back door.
fn run_ready(cli: &Cli, mission: &str, task_id: &str) -> Result<serde_json::Value, CliError> {
    validate_id_format(task_id)?;
    let repo_root = super::resolve_repo(cli)?;
    let paths = crate::mission::resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    // Ratified lock is a precondition — `$clarify` must have finished
    // before any task becomes executable.
    let lock = crate::mission::lock::parse_and_validate(&paths.outcome_lock())?;
    if lock.frontmatter.lock_status != crate::mission::lock::LockStatus::Ratified {
        return Err(CliError::Internal {
            message: "cannot ready a task while OUTCOME-LOCK.md is still draft; run $clarify first"
                .into(),
        });
    }
    let blueprint = crate::blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = crate::graph::build_dag(&blueprint)?;
    // Same plan-acceptance checks `plan check` runs, so a rejected
    // plan cannot still mint a ready task.
    super::plan::enforce_plan_acceptance(&paths.program_blueprint(), &blueprint, &dag)?;
    if !dag.tasks.contains_key(task_id) {
        return Err(CliError::TaskStateTransitionInvalid {
            task_id: task_id.into(),
            current: "not_in_blueprint".into(),
            attempted: "ready".into(),
        });
    }
    let store = StateStore::new(paths.mission_dir.clone());
    let task_id_owned = task_id.to_string();
    let state = store.mutate_checked(cli.expect_revision, cli.dry_run, move |state| {
        let current_status = state
            .tasks
            .get(&task_id_owned)
            .map_or(TaskStatus::Planned, |t| t.status);
        if !matches!(current_status, TaskStatus::Planned) {
            return Err(CliError::TaskStateTransitionInvalid {
                task_id: task_id_owned,
                current: task_status_str(current_status),
                attempted: "ready".into(),
            });
        }
        let entry = state
            .tasks
            .entry(task_id_owned.clone())
            .or_insert_with(TaskState::planned);
        entry.status = TaskStatus::Ready;
        Ok(EventDraft::new("task_marked_ready").with("task_id", task_id_owned.as_str()))
    })?;
    Ok(envelope::success(
        READY_SCHEMA,
        &json!({
            "mission_id": mission,
            "task_id": task_id,
            "status": "ready",
            "state_revision": state.state_revision,
            "message": format!("Task {task_id} marked ready."),
        }),
    ))
}

pub fn cmd_task_start(cli: &Cli, mission: &str, task_id: &str) -> i32 {
    match run_start(cli, mission, task_id) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_start(cli: &Cli, mission: &str, task_id: &str) -> Result<serde_json::Value, CliError> {
    validate_id_format(task_id)?;
    let (paths, dag, _state) = load_mission(cli, mission)?;
    if !dag.tasks.contains_key(task_id) {
        return Err(CliError::TaskStateTransitionInvalid {
            task_id: task_id.into(),
            current: "not_in_blueprint".into(),
            attempted: "start".into(),
        });
    }
    let store = StateStore::new(paths.mission_dir.clone());
    let graph_revision = dag.graph_revision;
    let new_run_id = format!("run-{}", Uuid::new_v4());
    let now = now_rfc3339();
    let dag_clone = dag; // closure takes ownership

    let run_id_for_closure = new_run_id.clone();
    let now_for_closure = now.clone();
    let task_id_owned = task_id.to_string();
    let state = store.mutate_checked(cli.expect_revision, cli.dry_run, move |state| {
        let current_status = state
            .tasks
            .get(&task_id_owned)
            .map_or(TaskStatus::Planned, |t| t.status);
        if !matches!(current_status, TaskStatus::Ready | TaskStatus::NeedsRepair) {
            return Err(CliError::TaskStateTransitionInvalid {
                task_id: task_id_owned,
                current: task_status_str(current_status),
                attempted: "start".into(),
            });
        }
        let spec = dag_clone.get(&task_id_owned).expect("dag contains id");
        for dep in &spec.depends_on {
            let dep_status = state
                .tasks
                .get(dep)
                .map_or(TaskStatus::Planned, |t| t.status);
            if !dep_status.satisfies_dep() {
                return Err(CliError::TaskStateTransitionInvalid {
                    task_id: task_id_owned,
                    current: format!("dep_{}_is_{}", dep, task_status_str(dep_status)),
                    attempted: "start".into(),
                });
            }
        }
        let entry = state
            .tasks
            .entry(task_id_owned.clone())
            .or_insert_with(TaskState::planned);
        entry.status = TaskStatus::InProgress;
        entry.started_at = Some(now_for_closure.clone());
        entry.finished_at = None;
        entry.reviewed_at = None;
        entry.task_run_id = Some(run_id_for_closure.clone());
        entry.proof_ref = None;
        entry.proof_hash = None;
        state.phase = Phase::Executing;
        Ok(EventDraft::new("task_started")
            .with("task_id", task_id_owned.as_str())
            .with("task_run_id", run_id_for_closure.as_str())
            .with("graph_revision", graph_revision))
    })?;

    let ts = state.tasks.get(task_id).expect("task present");
    Ok(envelope::success(
        START_SCHEMA,
        &json!({
            "mission_id": mission,
            "task_id": task_id,
            "task_run_id": ts.task_run_id,
            "status": ts.status,
            "started_at": ts.started_at,
            "state_revision": state.state_revision,
            "graph_revision": graph_revision,
            "message": format!("Started task {task_id} (run {})", ts.task_run_id.as_deref().unwrap_or("")),
        }),
    ))
}

pub fn cmd_task_finish(cli: &Cli, mission: &str, task_id: &str, proof: Option<&Path>) -> i32 {
    match run_finish(cli, mission, task_id, proof) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_finish(
    cli: &Cli,
    mission: &str,
    task_id: &str,
    proof_override: Option<&Path>,
) -> Result<serde_json::Value, CliError> {
    validate_id_format(task_id)?;
    let (paths, dag, _state) = load_mission(cli, mission)?;
    if !dag.tasks.contains_key(task_id) {
        return Err(CliError::TaskStateTransitionInvalid {
            task_id: task_id.into(),
            current: "not_in_blueprint".into(),
            attempted: "finish".into(),
        });
    }
    let proof_rel = proof_override.map_or_else(|| default_proof_ref(task_id), Path::to_path_buf);
    let now = now_rfc3339();
    let receipt = read_and_hash(&paths.mission_dir, &proof_rel, &now)?;
    let store = StateStore::new(paths.mission_dir.clone());
    let graph_revision = dag.graph_revision;
    let task_id_owned = task_id.to_string();
    let receipt_for_closure = receipt.clone();
    let now_for_closure = now.clone();

    let state = store.mutate_checked(cli.expect_revision, cli.dry_run, move |state| {
        let current_status = state
            .tasks
            .get(&task_id_owned)
            .map_or(TaskStatus::Planned, |t| t.status);
        if current_status != TaskStatus::InProgress {
            return Err(CliError::TaskStateTransitionInvalid {
                task_id: task_id_owned,
                current: task_status_str(current_status),
                attempted: "finish".into(),
            });
        }
        let entry = state
            .tasks
            .get_mut(&task_id_owned)
            .expect("task in state since status was in_progress");
        entry.status = TaskStatus::ProofSubmitted;
        entry.finished_at = Some(now_for_closure.clone());
        entry.proof_ref = Some(receipt_for_closure.proof_ref.clone());
        entry.proof_hash = Some(receipt_for_closure.proof_hash.clone());
        Ok(EventDraft::new("task_finished")
            .with("task_id", task_id_owned.as_str())
            .with("task_run_id", entry.task_run_id.clone().unwrap_or_default())
            .with("proof_ref", receipt_for_closure.proof_ref.clone())
            .with("proof_hash", receipt_for_closure.proof_hash.clone())
            .with("graph_revision", graph_revision))
    })?;

    let ts = state.tasks.get(task_id).expect("task present");
    Ok(envelope::success(
        FINISH_SCHEMA,
        &json!({
            "mission_id": mission,
            "task_id": task_id,
            "task_run_id": ts.task_run_id,
            "status": ts.status,
            "finished_at": ts.finished_at,
            "proof_ref": ts.proof_ref,
            "proof_hash": ts.proof_hash,
            "state_revision": state.state_revision,
            "graph_revision": graph_revision,
            "message": format!("Finished task {task_id}; review now owed."),
        }),
    ))
}

pub fn cmd_task_status(cli: &Cli, mission: &str, task_id: &str) -> i32 {
    match run_status(cli, mission, task_id) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_status(cli: &Cli, mission: &str, task_id: &str) -> Result<serde_json::Value, CliError> {
    validate_id_format(task_id)?;
    let (_paths, dag, state) = load_mission(cli, mission)?;
    let spec = dag
        .get(task_id)
        .ok_or_else(|| CliError::TaskStateTransitionInvalid {
            task_id: task_id.into(),
            current: "not_in_blueprint".into(),
            attempted: "status".into(),
        })?;
    let task_state = state
        .tasks
        .get(task_id)
        .cloned()
        .unwrap_or_else(TaskState::planned);
    Ok(envelope::success(
        STATUS_SCHEMA,
        &json!({
            "mission_id": mission,
            "task_id": task_id,
            "spec": {
                "title": spec.title,
                "kind": spec.kind,
                "depends_on": spec.depends_on,
            },
            "state": task_state,
            "state_revision": state.state_revision,
            "graph_revision": dag.graph_revision,
        }),
    ))
}

fn load_mission(
    cli: &Cli,
    mission: &str,
) -> Result<
    (
        crate::mission::MissionPaths,
        graph::Dag,
        crate::state::State,
    ),
    CliError,
> {
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;
    let state = StateStore::new(paths.mission_dir.clone()).load()?;
    Ok((paths, dag, state))
}

fn task_status_str(status: TaskStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(std::string::ToString::to_string))
        .unwrap_or_else(|| format!("{status:?}"))
}
