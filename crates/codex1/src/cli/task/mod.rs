//! `codex1 task` — Phase B Unit 6 (task lifecycle + worker packet).
//!
//! Each subcommand is a thin wrapper over helpers in `lifecycle.rs` and
//! `worker_packet.rs`. The `TaskCmd` variants are foundation-pinned so
//! `cli::mod` compiles regardless of this unit's merge order.

use std::path::PathBuf;

use clap::Subcommand;

use crate::cli::Ctx;
use crate::core::error::CliResult;

mod finish;
mod lifecycle;
mod next;
mod packet;
mod start;
mod status_cmd;
mod worker_packet;

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

pub fn dispatch(cmd: TaskCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        TaskCmd::Next => next::run(ctx),
        TaskCmd::Start { task } => start::run(&task, ctx),
        TaskCmd::Finish { task, proof } => finish::run(&task, &proof, ctx),
        TaskCmd::Status { task } => status_cmd::run(&task, ctx),
        TaskCmd::Packet { task } => packet::run(&task, ctx),
    }
}
