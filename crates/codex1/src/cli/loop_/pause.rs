//! `codex1 loop pause` — pause an active loop (used by $close).

use crate::cli::loop_::{run_transition, Transition};
use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};
use crate::state::schema::LoopState;

pub fn run(ctx: &Ctx) -> CliResult<()> {
    run_transition(ctx, "loop.paused", |current| {
        if !current.active {
            return Transition::Reject(CliError::TaskNotReady {
                message: "No active loop to pause".to_string(),
            });
        }
        if current.paused {
            return Transition::NoOp;
        }
        Transition::Apply(LoopState {
            active: true,
            paused: true,
            mode: current.mode.clone(),
        })
    })
}
