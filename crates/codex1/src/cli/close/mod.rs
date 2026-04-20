//! `codex1 close` stub — owned by Phase B Unit 10.

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

#[derive(Debug, Subcommand)]
pub enum CloseCmd {
    /// Check whether terminal close is ready.
    Check,
    /// Write CLOSEOUT.md and mark the mission terminal.
    Complete,
}

pub fn dispatch(cmd: CloseCmd, _ctx: &Ctx) -> CliResult<()> {
    let label = match cmd {
        CloseCmd::Check => "close check",
        CloseCmd::Complete => "close complete",
    };
    Err(CliError::NotImplemented {
        command: label.to_string(),
    })
}
