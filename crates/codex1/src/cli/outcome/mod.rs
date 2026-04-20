//! `codex1 outcome` stub — owned by Phase B Unit 2 (`cli-outcome`).
//!
//! Foundation pins the subcommand variants so `cli::mod` can reference
//! them by path. Phase B replaces the `dispatch` body with real
//! `check` / `ratify` implementations and adds any needed helpers in
//! sibling files under this directory.

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

#[derive(Debug, Subcommand)]
pub enum OutcomeCmd {
    /// Validate OUTCOME.md mechanical completeness.
    Check,
    /// Ratify OUTCOME.md (only if check passes).
    Ratify,
}

pub fn dispatch(cmd: OutcomeCmd, _ctx: &Ctx) -> CliResult<()> {
    let label = match cmd {
        OutcomeCmd::Check => "outcome check",
        OutcomeCmd::Ratify => "outcome ratify",
    };
    Err(CliError::NotImplemented {
        command: label.to_string(),
    })
}
