//! `codex1 replan record | check`.

use std::path::PathBuf;

use serde_json::json;
use walkdir::WalkDir;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::graph;
use crate::mission::resolve_mission;
use crate::replan::triggers::detect;
use crate::replan::{REPLAN_LOG_FILENAME, ReplanEvent, append_to_log};
use crate::review::BUNDLES_DIRNAME;
use crate::review::bundle::ReviewBundle;
use crate::state::{EventDraft, StateStore, TaskStatus};

use super::{Cli, emit_error, emit_success, now_rfc3339, resolve_repo};

const RECORD_SCHEMA: &str = "codex1.replan.record.v1";
const CHECK_SCHEMA: &str = "codex1.replan.check.v1";

pub fn cmd_replan_record(
    cli: &Cli,
    mission: &str,
    reason: &str,
    supersedes_csv: Option<&str>,
) -> i32 {
    match run_record(cli, mission, reason, supersedes_csv) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_record(
    cli: &Cli,
    mission: &str,
    reason: &str,
    supersedes_csv: Option<&str>,
) -> Result<serde_json::Value, CliError> {
    if reason.trim().is_empty() {
        return Err(CliError::Internal {
            message: "--reason must be non-empty".into(),
        });
    }
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;

    let supersedes: Vec<String> = supersedes_csv
        .map(|csv| {
            csv.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    // Every superseded id must exist in the DAG.
    for id in &supersedes {
        if !dag.tasks.contains_key(id) {
            return Err(CliError::DagMissingDep {
                task: "replan".into(),
                missing: id.clone(),
            });
        }
    }

    let reason_owned = reason.to_string();
    let supersedes_for_closure = supersedes.clone();
    let graph_rev = dag.graph_revision;
    let now = now_rfc3339();

    let store = StateStore::new(paths.mission_dir.clone());
    let state = store.mutate_checked(cli.expect_revision, cli.dry_run, move |state| {
        for id in &supersedes_for_closure {
            if let Some(entry) = state.tasks.get_mut(id) {
                entry.status = TaskStatus::Superseded;
            }
        }
        state.phase = crate::state::Phase::Replanning;
        Ok(EventDraft::new("replan_recorded")
            .with("reason", reason_owned.clone())
            .with(
                "superseded",
                serde_json::to_value(&supersedes_for_closure).unwrap_or_default(),
            )
            .with("graph_revision", graph_rev))
    })?;

    // Append to REPLAN-LOG.md.
    let event = ReplanEvent {
        reason: reason.into(),
        superseded_task_ids: supersedes.clone(),
        new_task_ids: vec![],
        graph_revision: dag.graph_revision,
        state_revision: state.state_revision,
        recorded_at: now,
    };
    let log_path = paths.mission_dir.join(REPLAN_LOG_FILENAME);
    append_to_log(&log_path, &event)?;

    Ok(envelope::success(
        RECORD_SCHEMA,
        &json!({
            "mission_id": mission,
            "reason": reason,
            "superseded_task_ids": supersedes,
            "state_revision": state.state_revision,
            "log_path": log_path.display().to_string(),
            "message": format!("Recorded replan event for mission {mission} (reason: {reason})."),
        }),
    ))
}

pub fn cmd_replan_check(cli: &Cli, mission: &str) -> i32 {
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
    let bundles = load_all_bundles(&paths.mission_dir.join(BUNDLES_DIRNAME))?;
    let report = detect(&bundles);
    Ok(envelope::success(
        CHECK_SCHEMA,
        &json!({
            "mission_id": mission,
            "mandatory_triggers": report.mandatory,
            "warnings": report.warnings,
            "bundle_count": bundles.len(),
            "message": if report.mandatory.is_empty() {
                "No mandatory replan triggers detected.".to_string()
            } else {
                format!(
                    "{} mandatory replan trigger(s) detected.",
                    report.mandatory.len()
                )
            },
        }),
    ))
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
                .unwrap_or_else(|| std::io::Error::other("walkdir error")),
        })?;
        if entry.file_type().is_file() {
            let _ = PathBuf::from(entry.path()); // used below
            let bytes = std::fs::read(entry.path()).map_err(|e| CliError::Io {
                path: entry.path().display().to_string(),
                source: e,
            })?;
            if let Ok(b) = serde_json::from_slice::<ReviewBundle>(&bytes) {
                out.push(b);
            }
            // Tolerate non-bundle JSON files in the directory.
        }
    }
    Ok(out)
}
