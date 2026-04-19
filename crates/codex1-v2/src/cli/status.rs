//! `codex1 status --mission <id>` — Ralph's sole interface.
//!
//! Loads the blueprint and state, projects via `status::project_status`, and
//! emits the envelope. On blueprint / DAG / state parse failure, emits the
//! specific error code rather than a partial envelope.

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph;
use crate::mission::lock::parse_and_validate as parse_lock;
use crate::mission::resolve_mission;
use crate::review::{BUNDLES_DIRNAME, load_all_bundles};
use crate::state::StateStore;
use crate::status::{self, project_status_with_bundles};

use super::{Cli, emit_error, emit_success, resolve_repo};

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
    let lock = parse_lock(&paths.outcome_lock())?;
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;
    let state = StateStore::new(paths.mission_dir.clone()).load()?;
    let bundles = load_all_bundles(&paths.mission_dir.join(BUNDLES_DIRNAME))?;

    let envelope_struct = project_status_with_bundles(&lock, &state, &dag, &bundles);
    let value = serde_json::to_value(&envelope_struct).map_err(|e| CliError::Internal {
        message: format!("serialize status envelope: {e}"),
    })?;
    Ok(envelope::success(status::SCHEMA, &value))
}
