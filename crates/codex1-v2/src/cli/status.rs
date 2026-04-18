//! `codex1 status --mission <id>` — Ralph's sole interface.
//!
//! Loads the blueprint and state, projects via `status::project_status`, and
//! emits the envelope. On blueprint / DAG / state parse failure, emits the
//! specific error code rather than a partial envelope.

use walkdir::WalkDir;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph;
use crate::mission::resolve_mission;
use crate::review::BUNDLES_DIRNAME;
use crate::review::bundle::ReviewBundle;
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
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;
    let state = StateStore::new(paths.mission_dir.clone()).load()?;
    let bundles = load_all_bundles(&paths.mission_dir.join(BUNDLES_DIRNAME))?;

    let envelope_struct = project_status_with_bundles(&state, &dag, &bundles);
    let value = serde_json::to_value(&envelope_struct).map_err(|e| CliError::Internal {
        message: format!("serialize status envelope: {e}"),
    })?;
    Ok(envelope::success(status::SCHEMA, &value))
}

fn load_all_bundles(bundles_dir: &std::path::Path) -> Result<Vec<ReviewBundle>, CliError> {
    if !bundles_dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(bundles_dir).min_depth(1).max_depth(1) {
        let entry = entry.map_err(|e| CliError::Io {
            path: bundles_dir.display().to_string(),
            source: e
                .into_io_error()
                .unwrap_or_else(|| std::io::Error::other("walkdir")),
        })?;
        if entry.file_type().is_file() {
            let bytes = std::fs::read(entry.path()).map_err(|e| CliError::Io {
                path: entry.path().display().to_string(),
                source: e,
            })?;
            if let Ok(b) = serde_json::from_slice::<ReviewBundle>(&bytes) {
                out.push(b);
            }
        }
    }
    Ok(out)
}
