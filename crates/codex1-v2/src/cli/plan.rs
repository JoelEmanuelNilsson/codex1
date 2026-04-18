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
const GRAPH_SCHEMA: &str = "codex1.plan.graph.v1";
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

pub fn cmd_plan_graph(cli: &Cli, mission: &str) -> i32 {
    match run_graph(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_graph(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;
    let tasks_json: Vec<serde_json::Value> = dag
        .tasks
        .values()
        .map(|spec| {
            json!({
                "id": spec.id,
                "title": spec.title,
                "kind": spec.kind,
                "depends_on": spec.depends_on,
                "read_paths": spec.read_paths,
                "write_paths": spec.write_paths,
                "exclusive_resources": spec.exclusive_resources,
                "unknown_side_effects": spec.unknown_side_effects,
                "supersedes": spec.supersedes,
                "review_profiles": spec.review_profiles,
            })
        })
        .collect();
    Ok(envelope::success(
        GRAPH_SCHEMA,
        &json!({
            "mission_id": mission,
            "graph_revision": dag.graph_revision,
            "tasks": tasks_json,
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
