//! `codex1 loop` stub — owned by Phase B Unit 9.

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

#[derive(Debug, Subcommand)]
pub enum LoopCmd {
    /// Pause the active loop (used by $close).
    Pause,
    /// Resume the active loop.
    Resume,
    /// Deactivate the loop entirely.
    Deactivate,
}

pub fn dispatch(cmd: LoopCmd, _ctx: &Ctx) -> CliResult<()> {
    let label = match cmd {
        LoopCmd::Pause => "loop pause",
        LoopCmd::Resume => "loop resume",
        LoopCmd::Deactivate => "loop deactivate",
    };
    Err(CliError::NotImplemented {
        command: label.to_string(),
    })
}
