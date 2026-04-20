//! `codex1 task` stub — owned by Phase B Unit 6.

use std::path::PathBuf;

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

#[derive(Debug, Subcommand)]
pub enum TaskCmd {
    /// Report the next ready task or ready wave.
    Next,
    /// Transition a task to in-progress.
    Start {
        #[arg(value_name = "TASK_ID")]
        task: String,
    },
    /// Mark a task complete (after proof).
    Finish {
        #[arg(value_name = "TASK_ID")]
        task: String,
        #[arg(long, value_name = "PATH")]
        proof: PathBuf,
    },
    /// Show status for a single task.
    Status {
        #[arg(value_name = "TASK_ID")]
        task: String,
    },
    /// Emit a worker packet (SPEC excerpt, write_paths, proof commands).
    Packet {
        #[arg(value_name = "TASK_ID")]
        task: String,
    },
}

pub fn dispatch(cmd: TaskCmd, _ctx: &Ctx) -> CliResult<()> {
    let label = match cmd {
        TaskCmd::Next => "task next",
        TaskCmd::Start { .. } => "task start",
        TaskCmd::Finish { .. } => "task finish",
        TaskCmd::Status { .. } => "task status",
        TaskCmd::Packet { .. } => "task packet",
    };
    Err(CliError::NotImplemented {
        command: label.to_string(),
    })
}
