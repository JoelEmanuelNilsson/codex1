//! `codex1 loop deactivate` — turn the loop off entirely.

use crate::cli::loop_::{run_transition, Transition};
use crate::cli::Ctx;
use crate::core::error::CliResult;
use crate::state::schema::{LoopMode, LoopState};

pub fn run(ctx: &Ctx) -> CliResult<()> {
    run_transition(ctx, "loop.deactivated", |current| {
        let target = LoopState {
            active: false,
            paused: false,
            mode: LoopMode::None,
        };
        if *current == target {
            Transition::NoOp
        } else {
            Transition::Apply(target)
        }
    })
}
