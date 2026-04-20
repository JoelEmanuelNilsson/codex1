//! `codex1 loop activate --mode <mode>` — idempotently activate the loop.
//!
//! Central entry point for other units (outcome ratify, plan check) to
//! turn the loop on without each inventing their own mutation.

use crate::cli::loop_::{parse_mode, run_transition, Transition};
use crate::cli::Ctx;
use crate::core::error::CliResult;
use crate::state::schema::LoopState;

pub fn run(ctx: &Ctx, mode_raw: &str) -> CliResult<()> {
    let mode = parse_mode(mode_raw)?;
    run_transition(ctx, "loop.activated", move |current| {
        let target = LoopState {
            active: true,
            paused: false,
            mode,
        };
        if *current == target {
            Transition::NoOp
        } else {
            Transition::Apply(target)
        }
    })
}
