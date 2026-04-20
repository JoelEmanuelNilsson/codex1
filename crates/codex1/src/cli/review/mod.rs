//! `codex1 review` stub — owned by Phase B Unit 7.

use std::path::PathBuf;

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

#[derive(Debug, Subcommand)]
pub enum ReviewCmd {
    /// Begin a planned review for a task.
    Start {
        #[arg(value_name = "TASK_ID")]
        task: String,
    },
    /// Emit a reviewer packet (target files, diffs, proofs, profile).
    Packet {
        #[arg(value_name = "TASK_ID")]
        task: String,
    },
    /// Record the review outcome.
    Record {
        #[arg(value_name = "TASK_ID")]
        task: String,
        /// Mark review clean (no P0/P1/P2 findings).
        #[arg(long, conflicts_with = "findings_file")]
        clean: bool,
        /// Path to a markdown file with P0/P1/P2 findings.
        #[arg(long, value_name = "PATH")]
        findings_file: Option<PathBuf>,
        /// Comma-separated reviewer actor ids.
        #[arg(long, value_name = "LIST")]
        reviewers: Option<String>,
    },
    /// Show review status for a task.
    Status {
        #[arg(value_name = "TASK_ID")]
        task: String,
    },
}

pub fn dispatch(cmd: ReviewCmd, _ctx: &Ctx) -> CliResult<()> {
    let label = match cmd {
        ReviewCmd::Start { .. } => "review start",
        ReviewCmd::Packet { .. } => "review packet",
        ReviewCmd::Record { .. } => "review record",
        ReviewCmd::Status { .. } => "review status",
    };
    Err(CliError::NotImplemented {
        command: label.to_string(),
    })
}
