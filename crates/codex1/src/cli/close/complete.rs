//! `codex1 close complete --json` — terminal mission close.
//!
//! Preconditions:
//!
//! 1. The shared readiness predicate (`close check`) must return
//!    `ready: true`. Otherwise → `CliError::CloseNotReady`.
//! 2. The mission must not already be terminal. Otherwise →
//!    `CliError::TerminalAlreadyComplete`.
//!
//! The state mutation is authoritative: `terminal_at` is set under the
//! state lock, and CLOSEOUT.md is rendered from the same post-mutation
//! state before the event/state are persisted.

use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::{ensure_artifact_file_write_safe, MissionPaths};
use crate::state::fs_atomic::atomic_write;
use crate::state::schema::{LoopMode, LoopState, Phase};
use crate::state::{self};

use super::check::ReadinessReport;
use super::closeout;

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let current = state::load(&paths)?;
    state::check_expected_revision(ctx.expect_revision, &current)?;

    if let Some(closed_at) = &current.close.terminal_at {
        if !paths.closeout().is_file() && !ctx.dry_run {
            let closeout_body = closeout::render(&current, &paths);
            ensure_closeout_writable(&paths)?;
            atomic_write(&paths.closeout(), closeout_body.as_bytes())?;
            emit_success(
                &current.mission_id,
                Some(current.revision),
                &paths,
                closed_at,
                /*dry_run=*/ false,
            );
            return Ok(());
        }
        return Err(CliError::TerminalAlreadyComplete {
            closed_at: closed_at.clone(),
        });
    }

    let report = ReadinessReport::from_state_and_paths(&current, &paths);
    if !report.ready {
        return Err(CliError::CloseNotReady {
            message: report.blocker_summary(),
        });
    }

    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    if ctx.dry_run {
        emit_success(
            &current.mission_id,
            Some(current.revision),
            &paths,
            &now,
            /*dry_run=*/ true,
        );
        return Ok(());
    }

    ensure_closeout_writable(&paths)?;

    let mutation = state::mutate_dynamic_with_precommit(
        &paths,
        ctx.expect_revision,
        |state| {
            if let Some(closed_at) = state.close.terminal_at.as_ref() {
                return Err(CliError::TerminalAlreadyComplete {
                    closed_at: closed_at.clone(),
                });
            }
            let report = ReadinessReport::from_state_and_paths(state, &paths);
            if !report.ready {
                return Err(CliError::CloseNotReady {
                    message: report.blocker_summary(),
                });
            }
            state.close.terminal_at = Some(now.clone());
            state.phase = Phase::Terminal;
            state.loop_ = LoopState {
                active: false,
                paused: false,
                mode: LoopMode::None,
            };
            Ok((
                "close.complete".to_string(),
                json!({ "terminal_at": now.clone() }),
            ))
        },
        |state, _event| {
            let closeout_body = closeout::render(state, &paths);
            ensure_closeout_writable(&paths)?;
            atomic_write(&paths.closeout(), closeout_body.as_bytes())?;
            Ok(())
        },
    )?;

    emit_success(
        &mutation.state.mission_id,
        Some(mutation.new_revision),
        &paths,
        mutation.state.close.terminal_at.as_deref().unwrap_or(&now),
        /*dry_run=*/ false,
    );
    Ok(())
}

fn ensure_closeout_writable(paths: &MissionPaths) -> CliResult<()> {
    let closeout = paths.closeout();
    if closeout.exists() && !closeout.is_file() {
        return Err(CliError::ParseError {
            message: format!("CLOSEOUT.md target is not a file: {}", closeout.display()),
        });
    }
    ensure_artifact_file_write_safe(paths, &closeout, "CLOSEOUT.md")?;
    Ok(())
}

fn emit_success(
    mission_id: &str,
    revision: Option<u64>,
    paths: &MissionPaths,
    terminal_at: &str,
    dry_run: bool,
) {
    let env = JsonOk::new(
        Some(mission_id.to_string()),
        revision,
        json!({
            "closeout_path": paths.closeout(),
            "terminal_at": terminal_at,
            "mission_id": mission_id,
            "dry_run": dry_run,
        }),
    );
    println!("{}", env.to_pretty());
}
