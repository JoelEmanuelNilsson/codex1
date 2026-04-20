//! `codex1 plan` stub — owned by Phase B Units 3/4/5.

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

use crate::cli::Ctx;
use crate::core::error::{CliError, CliResult};

pub mod graph;
pub mod waves;

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

pub fn dispatch(cmd: PlanCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        PlanCmd::Waves => waves::run(ctx),
        PlanCmd::Graph { format, out } => graph::run(ctx, format, out),
        PlanCmd::ChooseLevel { .. } => Err(CliError::NotImplemented {
            command: "plan choose-level".to_string(),
        }),
        PlanCmd::Scaffold { .. } => Err(CliError::NotImplemented {
            command: "plan scaffold".to_string(),
        }),
        PlanCmd::Check => Err(CliError::NotImplemented {
            command: "plan check".to_string(),
        }),
    }
}
