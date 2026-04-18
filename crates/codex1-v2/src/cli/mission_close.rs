//! `codex1 mission-close check | complete`.

use serde_json::json;
use walkdir::WalkDir;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph;
use crate::mission::resolve_mission;
use crate::mission_close::{ReadinessReport, check_readiness};
use crate::review::BUNDLES_DIRNAME;
use crate::review::bundle::ReviewBundle;
use crate::state::{EventDraft, Phase, StateStore};

use super::{Cli, emit_error, emit_success, resolve_repo};

const CHECK_SCHEMA: &str = "codex1.mission_close.check.v1";
const COMPLETE_SCHEMA: &str = "codex1.mission_close.complete.v1";

pub fn cmd_mission_close_check(cli: &Cli, mission: &str) -> i32 {
    match run_check(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_check(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let (_paths, report) = readiness(cli, mission)?;
    Ok(envelope::success(
        CHECK_SCHEMA,
        &json!({
            "mission_id": mission,
            "can_close": report.can_close,
            "can_complete": report.can_complete,
            "mission_close_bundle": report.mission_close_bundle,
            "mission_close_clean": report.mission_close_clean,
            "blocking_reasons": report.blocking_reasons,
            "message": if report.can_close {
                "Mission is ready for terminal close.".to_string()
            } else {
                format!(
                    "Mission not ready to close ({} blocking reason(s)).",
                    report.blocking_reasons.len()
                )
            },
        }),
    ))
}

pub fn cmd_mission_close_complete(cli: &Cli, mission: &str) -> i32 {
    match run_complete(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_complete(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let (paths, report) = readiness(cli, mission)?;
    if !report.can_close {
        return Err(CliError::Internal {
            message: format!(
                "refuse to complete mission {mission}: {} blocking reason(s); run codex1 mission-close check",
                report.blocking_reasons.len()
            ),
        });
    }
    let store = StateStore::new(paths.mission_dir.clone());
    let state = store.mutate_checked(cli.expect_revision, |state| {
        if state.phase == Phase::Complete {
            return Err(CliError::Internal {
                message: "mission already complete".into(),
            });
        }
        state.phase = Phase::Complete;
        state.parent_loop.mode = crate::state::ParentLoopMode::None;
        state.parent_loop.paused = false;
        // Mark all non-superseded review_clean tasks as complete so the
        // status envelope reaches verdict: complete.
        for task in state.tasks.values_mut() {
            if task.status == crate::state::TaskStatus::ReviewClean {
                task.status = crate::state::TaskStatus::Complete;
            }
        }
        Ok(EventDraft::new("mission_closed").with("closed_at", super::now_rfc3339()))
    })?;
    Ok(envelope::success(
        COMPLETE_SCHEMA,
        &json!({
            "mission_id": mission,
            "phase": state.phase,
            "state_revision": state.state_revision,
            "message": format!("Mission {mission} closed — terminal."),
        }),
    ))
}

fn readiness(
    cli: &Cli,
    mission: &str,
) -> Result<(crate::mission::MissionPaths, ReadinessReport), CliError> {
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
    let report = check_readiness(&state, &dag, &bundles);
    Ok((paths, report))
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
