//! `codex1 loop resume` — resume a paused loop.

use crate::cli::loop_::{run_transition, Transition};
use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};
use crate::state::schema::LoopState;

pub fn run(ctx: &Ctx) -> CliResult<()> {
    run_transition(ctx, "loop.resumed", |current| {
        // Idempotent only when already active and unpaused; otherwise the
        // caller must go through activate/deactivate for the intended
        // semantic (resume presumes there is a loop to resume).
        if current.active && !current.paused {
            return Transition::NoOp;
        }
        if !current.active {
            return Transition::Reject(CliError::TaskNotReady {
                message: "Loop is not active; cannot resume".to_string(),
            });
        }
        // active && paused → unpause.
        Transition::Apply(LoopState {
            active: true,
            paused: false,
            mode: current.mode.clone(),
        })
    })
}
