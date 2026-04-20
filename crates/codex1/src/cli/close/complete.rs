//! `codex1 close complete --json` — terminal mission close.
//!
//! Preconditions:
//!
//! 1. The shared readiness predicate (`close check`) must return
//!    `ready: true`. Otherwise → `CliError::CloseNotReady`.
//! 2. The mission must not already be terminal. Otherwise →
//!    `CliError::TerminalAlreadyComplete`.
//!
//! The state mutation is authoritative: `terminal_at` is set through
//! `state::mutate`, and CLOSEOUT.md is written afterwards. On a crash
//! between the two, idempotency is preserved — the next `complete` call
//! hits `TerminalAlreadyComplete` before doing any work.

use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::MissionPaths;
use crate::state::fs_atomic::atomic_write;
use crate::state::schema::{LoopMode, LoopState, Phase};
use crate::state::{self};

use super::check::ReadinessReport;
use super::closeout;

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let current = state::load(&paths)?;

    if let Some(closed_at) = &current.close.terminal_at {
        return Err(CliError::TerminalAlreadyComplete {
            closed_at: closed_at.clone(),
        });
    }

    let report = ReadinessReport::from_state(&current);
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

    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        "close.complete",
        json!({ "terminal_at": now.clone() }),
        |state| {
            state.close.terminal_at = Some(now.clone());
            state.phase = Phase::Terminal;
            state.loop_ = LoopState {
                active: false,
                paused: false,
                mode: LoopMode::None,
            };
            Ok(())
        },
    )?;

    let closeout_body = closeout::render(&mutation.state, &paths);
    atomic_write(&paths.closeout(), closeout_body.as_bytes())?;

    emit_success(
        &mutation.state.mission_id,
        Some(mutation.new_revision),
        &paths,
        mutation.state.close.terminal_at.as_deref().unwrap_or(&now),
        /*dry_run=*/ false,
    );
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
