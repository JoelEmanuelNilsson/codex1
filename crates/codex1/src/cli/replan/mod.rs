//! `codex1 replan` stub — owned by Phase B Unit 8.

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

#[derive(Debug, Subcommand)]
pub enum ReplanCmd {
    /// Check whether a replan is required.
    Check,
    /// Record a replan decision.
    Record {
        #[arg(long, value_name = "CODE")]
        reason: String,
        #[arg(long, value_name = "TASK_ID")]
        supersedes: Option<String>,
    },
}

pub fn dispatch(cmd: ReplanCmd, _ctx: &Ctx) -> CliResult<()> {
    let label = match cmd {
        ReplanCmd::Check => "replan check",
        ReplanCmd::Record { .. } => "replan record",
    };
    Err(CliError::NotImplemented {
        command: label.to_string(),
    })
}
