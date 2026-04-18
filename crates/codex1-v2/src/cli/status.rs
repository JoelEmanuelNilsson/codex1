//! `codex1 status --mission <id>` — Ralph's sole interface.
//!
//! Loads the blueprint and state, projects via `status::project_status`, and
//! emits the envelope. On blueprint / DAG / state parse failure, emits the
//! specific error code rather than a partial envelope.

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph;
use crate::mission::resolve_mission;
use crate::state::StateStore;
use crate::status::{self, project_status};

use super::{emit_error, emit_success, resolve_repo, Cli};

pub fn cmd_status(cli: &Cli, mission: &str) -> i32 {
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

    let envelope_struct = project_status(&state, &dag);
    let value = serde_json::to_value(&envelope_struct).map_err(|e| CliError::Internal {
        message: format!("serialize status envelope: {e}"),
    })?;
    Ok(envelope::success(status::SCHEMA, &value))
}
