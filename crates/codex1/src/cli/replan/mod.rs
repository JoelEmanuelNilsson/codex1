//! `codex1 replan` — detect and record replan decisions.
//!
//! `check` inspects `state.replan.consecutive_dirty_by_target` and
//! reports whether the "six consecutive dirty reviews on the same
//! active target" threshold has been reached. `record` mutates
//! `state.replan`, supersedes referenced tasks, clears the plan lock,
//! and transitions the mission back to `Phase::Plan`.

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::CliResult;

pub mod check;
pub mod record;
pub mod triggers;

#[derive(Debug, Subcommand)]
pub enum ReplanCmd {
    /// Check whether a replan is required.
    Check,
    /// Record a replan decision.
    Record {
        #[arg(long, value_name = "CODE")]
        reason: String,
        /// Task id to mark `Superseded` (may be passed multiple times).
        #[arg(long, value_name = "TASK_ID")]
        supersedes: Vec<String>,
    },
}

pub fn dispatch(cmd: ReplanCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        ReplanCmd::Check => check::run(ctx),
        ReplanCmd::Record { reason, supersedes } => record::run(ctx, &reason, &supersedes),
    }
}
