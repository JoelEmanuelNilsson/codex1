//! `codex1 validate --mission <id>` — structural validation of mission files.
//!
//! This is the superset of `lock check` + `plan check` + STATE/events
//! consistency. Runs the narrower checks in order and short-circuits on the
//! first failure with a specific error code. A one-line event lag
//! (`last_seq == state_revision - 1`) is a warning recorded in the envelope,
//! not an error.

use serde_json::json;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::events;
use crate::graph;
use crate::mission::{lock::parse_and_validate, resolve_mission};
use crate::review::{BUNDLES_DIRNAME, load_all_bundles};
use crate::state::StateStore;

use super::{Cli, emit_error, emit_success, resolve_repo};

const SCHEMA: &str = "codex1.validate.v1";

pub fn cmd_validate(cli: &Cli, mission: &str) -> i32 {
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

    // 1. OUTCOME-LOCK.md structure
    let lock = parse_and_validate(&paths.outcome_lock())?;

    // 2. PROGRAM-BLUEPRINT.md + DAG
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;

    // 3. STATE.json
    let store = StateStore::new(paths.mission_dir.clone());
    let state = store.load()?;
    if state.state_revision == 0 {
        return Err(CliError::StateCorrupt {
            path: store.state_path().display().to_string(),
            reason: "state_revision must be >= 1".into(),
            source: None,
        });
    }
    if state.mission_id != mission {
        return Err(CliError::StateCorrupt {
            path: store.state_path().display().to_string(),
            reason: format!(
                "mission_id mismatch: state says {:?}, command says {:?}",
                state.mission_id, mission
            ),
            source: None,
        });
    }
    // STATE tasks must reference blueprint tasks.
    for task_id in state.tasks.keys() {
        if !dag.tasks.contains_key(task_id) {
            return Err(CliError::StateCorrupt {
                path: store.state_path().display().to_string(),
                reason: format!(
                    "STATE.json references task {task_id:?} that is not in the blueprint"
                ),
                source: None,
            });
        }
    }

    // 4. events.jsonl — last seq must not exceed state_revision.
    let last_seq = events::last_seq(&store.events_path()).map_err(|e| CliError::Io {
        path: store.events_path().display().to_string(),
        source: e,
    })?;
    let mut warnings: Vec<String> = Vec::new();
    match last_seq {
        Some(seq) if seq > state.state_revision => {
            return Err(CliError::StateCorrupt {
                path: store.events_path().display().to_string(),
                reason: format!(
                    "last event seq {seq} > state_revision {}",
                    state.state_revision
                ),
                source: None,
            });
        }
        Some(seq) if seq < state.state_revision => {
            warnings.push(format!(
                "events.jsonl trails state_revision by {} (last seq {seq}, state_revision {})",
                state.state_revision - seq,
                state.state_revision
            ));
        }
        Some(_) | None => {
            if last_seq.is_none() {
                warnings.push("events.jsonl is missing or empty".into());
            }
        }
    }

    // 5. Review bundles parse. Round 8 follow-up: `validate` must fail
    // closed when any `reviews/B*.json` is corrupt, matching the
    // fail-closed behaviour `status` and `mission-close check` already
    // exhibit. Without this, a corrupted mission-close bundle could
    // sit invisibly in reviews/ while validate reports ok, which
    // contradicts validate's "structural superset" claim and lets
    // review truth loss hide from both humans and the qualification
    // verifier.
    let bundles = load_all_bundles(&paths.mission_dir.join(BUNDLES_DIRNAME))?;

    Ok(envelope::success(
        SCHEMA,
        &json!({
            "mission_id": state.mission_id,
            "mission_dir": paths.mission_dir.display().to_string(),
            "lock": { "lock_status": lock.frontmatter.lock_status, "title": lock.frontmatter.title },
            "graph_revision": dag.graph_revision,
            "task_count": dag.len(),
            "state_revision": state.state_revision,
            "last_event_seq": last_seq,
            "review_bundle_count": bundles.len(),
            "warnings": warnings,
            "message": format!(
                "Validation passed for mission {} (tasks: {}, state_revision: {}).",
                state.mission_id, dag.len(), state.state_revision
            ),
        }),
    ))
}
