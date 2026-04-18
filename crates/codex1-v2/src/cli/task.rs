//! `codex1 task next` — the next action the parent should take.
//!
//! Wave 1 emits the same `next_action` field as the status envelope
//! (`start_task | user_decision | complete | invalid_state`) plus supporting
//! context (verdict, ready tasks, blocked tasks).

use serde_json::json;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph;
use crate::mission::resolve_mission;
use crate::state::StateStore;
use crate::status::project_status;

use super::{emit_error, emit_success, resolve_repo, Cli};

const SCHEMA: &str = "codex1.task.next.v1";

pub fn cmd_task_next(cli: &Cli, mission: &str) -> i32 {
    match run(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
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
    let status = project_status(&state, &dag);
    let status_value = serde_json::to_value(&status).map_err(|e| CliError::Internal {
        message: format!("serialize status: {e}"),
    })?;

    Ok(envelope::success(
        SCHEMA,
        &json!({
            "mission_id": mission,
            "verdict": status_value["verdict"],
            "next_action": status_value["next_action"],
            "ready_tasks": status_value["ready_tasks"],
            "blocked": status_value["blocked"],
            "required_user_decision": status_value["required_user_decision"],
        }),
    ))
}
