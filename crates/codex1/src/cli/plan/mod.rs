//! `codex1 plan` — Phase B Unit 3 owns `ChooseLevel` and `Scaffold`.
//!
//! Units 4 and 5 will replace the `Check`, `Graph`, and `Waves` arms with
//! real handlers and add sibling modules; this unit does not touch those.

pub mod choose_level;
pub mod scaffold;

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

pub fn dispatch(cmd: PlanCmd, ctx: &Ctx) -> CliResult<()> {
    match cmd {
        PlanCmd::ChooseLevel { level, escalate } => choose_level::run(level, escalate, ctx),
        PlanCmd::Scaffold { level } => scaffold::run(level, ctx),
        PlanCmd::Check => Err(CliError::NotImplemented {
            command: "plan check".to_string(),
        }),
        PlanCmd::Graph { .. } => Err(CliError::NotImplemented {
            command: "plan graph".to_string(),
        }),
        PlanCmd::Waves => Err(CliError::NotImplemented {
            command: "plan waves".to_string(),
        }),
    }
}
