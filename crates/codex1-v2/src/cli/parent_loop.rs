//! `codex1 parent-loop activate | deactivate | pause | resume`.
//!
//! Backs the `$execute`, `$review-loop`, `$autopilot`, and `$close` skills.
//! Ralph reads `codex1 status` and blocks stop when the parent loop is
//! active and not paused (see `status::derive_stop_policy`).

use serde_json::json;

use crate::envelope;
use crate::error::CliError;
use crate::mission::resolve_mission;
use crate::state::{EventDraft, ParentLoopMode, StateStore};

use super::{Cli, emit_error, emit_success, resolve_repo};

const ACTIVATE_SCHEMA: &str = "codex1.parent_loop.activate.v1";
const DEACTIVATE_SCHEMA: &str = "codex1.parent_loop.deactivate.v1";
const PAUSE_SCHEMA: &str = "codex1.parent_loop.pause.v1";
const RESUME_SCHEMA: &str = "codex1.parent_loop.resume.v1";

pub fn cmd_activate(cli: &Cli, mission: &str, mode: &str) -> i32 {
    match run_activate(cli, mission, mode) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_activate(cli: &Cli, mission: &str, mode_str: &str) -> Result<serde_json::Value, CliError> {
    let mode = parse_mode(mode_str)?;
    if mode == ParentLoopMode::None {
        return Err(CliError::Internal {
            message: "cannot activate parent loop with mode `none`; use deactivate".into(),
        });
    }
    let store = mutating_store(cli, mission)?;
    let mode_str_owned = mode_str.to_string();
    let state = store.mutate_checked(cli.expect_revision, move |state| {
        state.parent_loop.mode = mode;
        state.parent_loop.paused = false;
        Ok(EventDraft::new("parent_loop_activated").with("mode", mode_str_owned))
    })?;
    Ok(envelope::success(
        ACTIVATE_SCHEMA,
        &json!({
            "mission_id": mission,
            "mode": mode_str,
            "state_revision": state.state_revision,
            "message": format!("Parent loop activated in mode {mode_str}."),
        }),
    ))
}

pub fn cmd_deactivate(cli: &Cli, mission: &str) -> i32 {
    match run_deactivate(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_deactivate(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let store = mutating_store(cli, mission)?;
    let state = store.mutate_checked(cli.expect_revision, move |state| {
        state.parent_loop.mode = ParentLoopMode::None;
        state.parent_loop.paused = false;
        Ok(EventDraft::new("parent_loop_deactivated"))
    })?;
    Ok(envelope::success(
        DEACTIVATE_SCHEMA,
        &json!({
            "mission_id": mission,
            "state_revision": state.state_revision,
            "message": "Parent loop deactivated.".to_string(),
        }),
    ))
}

pub fn cmd_pause(cli: &Cli, mission: &str) -> i32 {
    match run_pause(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_pause(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let store = mutating_store(cli, mission)?;
    let state = store.mutate_checked(cli.expect_revision, |state| {
        if state.parent_loop.mode == ParentLoopMode::None {
            return Err(CliError::Internal {
                message: "cannot pause: no active parent loop".into(),
            });
        }
        state.parent_loop.paused = true;
        Ok(EventDraft::new("parent_loop_paused"))
    })?;
    Ok(envelope::success(
        PAUSE_SCHEMA,
        &json!({
            "mission_id": mission,
            "state_revision": state.state_revision,
            "message": "Parent loop paused; Ralph will now allow stop.".to_string(),
        }),
    ))
}

pub fn cmd_resume(cli: &Cli, mission: &str) -> i32 {
    match run_resume(cli, mission) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_resume(cli: &Cli, mission: &str) -> Result<serde_json::Value, CliError> {
    let store = mutating_store(cli, mission)?;
    let state = store.mutate_checked(cli.expect_revision, |state| {
        if state.parent_loop.mode == ParentLoopMode::None {
            return Err(CliError::Internal {
                message: "cannot resume: no active parent loop".into(),
            });
        }
        state.parent_loop.paused = false;
        Ok(EventDraft::new("parent_loop_resumed"))
    })?;
    Ok(envelope::success(
        RESUME_SCHEMA,
        &json!({
            "mission_id": mission,
            "state_revision": state.state_revision,
            "message": "Parent loop resumed.".to_string(),
        }),
    ))
}

fn parse_mode(s: &str) -> Result<ParentLoopMode, CliError> {
    match s {
        "execute" => Ok(ParentLoopMode::Execute),
        "review" => Ok(ParentLoopMode::Review),
        "autopilot" => Ok(ParentLoopMode::Autopilot),
        "close" => Ok(ParentLoopMode::Close),
        "none" => Ok(ParentLoopMode::None),
        _ => Err(CliError::Internal {
            message: format!(
                "unknown parent-loop mode {s:?}; expected execute|review|autopilot|close|none"
            ),
        }),
    }
}

fn mutating_store(cli: &Cli, mission: &str) -> Result<StateStore, CliError> {
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    Ok(StateStore::new(paths.mission_dir))
}
