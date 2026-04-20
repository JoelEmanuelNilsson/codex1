//! `codex1 plan` stub — owned by Phase B Units 3/4/5.

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

#[derive(Debug, Subcommand)]
pub enum PlanCmd {
    /// Interactive planning-level selection handshake.
    ChooseLevel {
        /// Explicit level (accepts 1/2/3 or light/medium/hard).
        #[arg(long, value_name = "LEVEL")]
        level: Option<String>,

        /// Reason the effective level is higher than requested.
        #[arg(long, value_name = "REASON")]
        escalate: Option<String>,
    },
    /// Write a PLAN.yaml skeleton for the chosen level.
    Scaffold {
        #[arg(long, value_name = "LEVEL")]
        level: String,
    },
    /// Validate PLAN.yaml structure and DAG.
    Check,
    /// Emit the DAG in a human/tool-friendly format.
    Graph {
        #[arg(long, default_value = "mermaid")]
        format: GraphFormat,
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    /// Derive wave(s) from depends_on + current task state.
    Waves,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GraphFormat {
    Mermaid,
    Dot,
    Json,
}

pub fn dispatch(cmd: PlanCmd, _ctx: &Ctx) -> CliResult<()> {
    let label = match cmd {
        PlanCmd::ChooseLevel { .. } => "plan choose-level",
        PlanCmd::Scaffold { .. } => "plan scaffold",
        PlanCmd::Check => "plan check",
        PlanCmd::Graph { .. } => "plan graph",
        PlanCmd::Waves => "plan waves",
    };
    Err(CliError::NotImplemented {
        command: label.to_string(),
    })
}
