//! `codex1 loop` — Phase B Unit 9.
//!
//! Drives `STATE.json.loop`. Ralph's Stop-allow decision keys off of
//! `loop.active` and `loop.paused`; `$close` pauses, `$close` resume or
//! deactivates. Other units activate the loop via `loop activate` rather
//! than inventing the mutation themselves.

use clap::Subcommand;
use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::MissionPaths;
use crate::state;
use crate::state::schema::{LoopMode, LoopState, MissionState};

pub mod activate;
pub mod deactivate;
pub mod pause;
pub mod resume;

#[derive(Debug, Subcommand)]
pub enum LoopCmd {
    /// Activate the loop. Used by other units (outcome ratify, plan check) via shell-out.
    Activate {
        /// clarify | plan | execute | review_loop | mission_close
        #[arg(long, default_value = "execute")]
        mode: String,
    },
    /// Pause the active loop (used by $close).
    Pause,
    /// Resume a paused loop.
    Resume,
    /// Deactivate the loop entirely.
    Deactivate,
}

pub fn dispatch(cmd: LoopCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        LoopCmd::Activate { mode } => activate::run(ctx, &mode),
        LoopCmd::Pause => pause::run(ctx),
        LoopCmd::Resume => resume::run(ctx),
        LoopCmd::Deactivate => deactivate::run(ctx),
    }
}

/// Outcome of evaluating a loop transition against the current state.
pub(crate) enum Transition {
    /// State already matches the target; emit success without writing.
    NoOp,
    /// State needs to change to `target`.
    Apply(LoopState),
    /// Transition is rejected with a canonical error.
    Reject(CliError),
}

/// Run a loop transition: resolve the mission, load state, classify the
/// transition, and either emit a no-op success or perform the mutation
/// (and append the event). Honors `--dry-run` and `--expect-revision`.
pub(crate) fn run_transition(
    ctx: &Ctx,
    event_kind: &'static str,
    classify: impl FnOnce(&LoopState) -> Transition,
) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let current = state::load(&paths)?;

    match classify(&current.loop_) {
        Transition::Reject(err) => {
            // Enforce --expect-revision first: callers that pinned the
            // revision want to know the state moved out from under them,
            // even if the transition is also semantically invalid.
            check_expected_revision(ctx, &current)?;
            Err(err)
        }
        Transition::NoOp => {
            check_expected_revision(ctx, &current)?;
            emit(
                &paths,
                current.revision,
                &current.loop_,
                None,
                ctx.dry_run,
                true,
            );
            Ok(())
        }
        Transition::Apply(target) => {
            if ctx.dry_run {
                check_expected_revision(ctx, &current)?;
                emit(
                    &paths,
                    current.revision,
                    &target,
                    Some(&current.loop_),
                    true,
                    false,
                );
                return Ok(());
            }
            let payload = json!(&target);
            let mutation = state::mutate(&paths, ctx.expect_revision, event_kind, payload, |s| {
                s.loop_ = target;
                Ok(())
            })?;
            emit(
                &paths,
                mutation.state.revision,
                &mutation.state.loop_,
                None,
                false,
                false,
            );
            Ok(())
        }
    }
}

fn check_expected_revision(ctx: &Ctx, state: &MissionState) -> CliResult<()> {
    if let Some(expected) = ctx.expect_revision {
        if expected != state.revision {
            return Err(CliError::RevisionConflict {
                expected,
                actual: state.revision,
            });
        }
    }
    Ok(())
}

fn emit(
    paths: &MissionPaths,
    revision: u64,
    loop_: &LoopState,
    before: Option<&LoopState>,
    dry_run: bool,
    noop: bool,
) {
    // Mirror LoopState at the top of `data` (matching the task contract
    // `{ active, paused, mode }`) and add `noop` / `dry_run` / optional
    // `before` as extras callers can rely on without schema contention.
    let mut data = json!(loop_);
    data["noop"] = json!(noop);
    data["dry_run"] = json!(dry_run);
    if let Some(prev) = before {
        data["before"] = json!(prev);
    }
    let env = JsonOk::new(Some(paths.mission_id.clone()), Some(revision), data);
    println!("{}", env.to_pretty());
}

/// Parse a mode string from the CLI into `LoopMode`. `none` is rejected —
/// that is what `deactivate` is for.
pub(crate) fn parse_mode(raw: &str) -> CliResult<LoopMode> {
    match raw {
        "clarify" => Ok(LoopMode::Clarify),
        "plan" => Ok(LoopMode::Plan),
        "execute" => Ok(LoopMode::Execute),
        "review_loop" => Ok(LoopMode::ReviewLoop),
        "mission_close" => Ok(LoopMode::MissionClose),
        "none" => Err(CliError::PlanInvalid {
            message: "mode `none` is not a valid activation target".to_string(),
            hint: Some("Use `codex1 loop deactivate` instead.".to_string()),
        }),
        other => Err(CliError::PlanInvalid {
            message: format!("unknown loop mode `{other}`"),
            hint: Some(
                "Valid modes: clarify, plan, execute, review_loop, mission_close.".to_string(),
            ),
        }),
    }
}
