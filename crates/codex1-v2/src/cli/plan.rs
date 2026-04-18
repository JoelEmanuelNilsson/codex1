//! `codex1 plan check | waves` — DAG-only checks and wave derivation.

use serde_json::json;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph::{self, waves::derive_waves};
use crate::mission::resolve_mission;
use crate::state::StateStore;

use super::{emit_error, emit_success, resolve_repo, Cli};

const CHECK_SCHEMA: &str = "codex1.plan.check.v1";
const WAVES_SCHEMA: &str = "codex1.plan.waves.v1";

pub fn cmd_plan_check(cli: &Cli, mission: &str) -> i32 {
    match run_check(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_check(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;
    Ok(envelope::success(
        CHECK_SCHEMA,
        &json!({
            "mission_id": mission,
            "graph_revision": dag.graph_revision,
            "task_count": dag.len(),
            "task_ids": dag.ids(),
            "message": format!(
                "Plan check passed for mission {mission} ({} tasks).",
                dag.len()
            ),
        }),
    ))
}

pub fn cmd_plan_waves(cli: &Cli, mission: &str) -> i32 {
    match run_waves(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_waves(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
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
    let waves = derive_waves(&dag, &state);
    let value = serde_json::to_value(&waves).map_err(|e| CliError::Internal {
        message: format!("serialize waves: {e}"),
    })?;
    // Flatten waves + blocked into the envelope at top level.
    let mut payload = json!({
        "mission_id": mission,
        "graph_revision": dag.graph_revision,
    });
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("waves".into(), value["waves"].clone());
        obj.insert("blocked".into(), value["blocked"].clone());
    }
    Ok(envelope::success(WAVES_SCHEMA, &payload))
}
