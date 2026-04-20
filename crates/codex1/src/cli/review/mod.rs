//! `codex1 review` — reviewer packet emission, start/record/status.
//!
//! The CLI does not check caller identity; the main thread records review
//! outcomes. Reviewer subagents only return findings (findings-file copied
//! to `PLANS/<mission>/reviews/<id>.md`). See
//! `docs/cli-contract-schemas.md` § Review record freshness for
//! classification rules.

use std::path::PathBuf;

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::CliResult;

mod classify;
mod packet;
mod plan_read;
mod record;
mod start;
mod status_cmd;

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
        #[arg(
            long,
            conflicts_with = "findings_file",
            required_unless_present = "findings_file"
        )]
        clean: bool,
        /// Path to a markdown file with P0/P1/P2 findings.
        #[arg(long, value_name = "PATH", required_unless_present = "clean")]
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

pub fn dispatch(cmd: ReviewCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        ReviewCmd::Start { task } => start::run(ctx, &task),
        ReviewCmd::Packet { task } => packet::run(ctx, &task),
        ReviewCmd::Record {
            task,
            clean,
            findings_file,
            reviewers,
        } => record::run(
            ctx,
            &record::RecordInputs {
                task_id: &task,
                clean,
                findings_file,
                reviewers_csv: reviewers,
            },
        ),
        ReviewCmd::Status { task } => status_cmd::run(ctx, &task),
    }
}
