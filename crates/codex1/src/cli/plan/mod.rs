//! `codex1 plan` dispatcher.
//!
//! Foundation pins the subcommand enum and `dispatch`. Phase B units fill
//! in the handlers: Unit 3 owns `ChooseLevel` and `Scaffold`; Unit 4 owns
//! `Check` (with its `dag`/`parsed` helpers); Unit 5 owns `Graph` and `Waves`.

pub mod check;
pub mod choose_level;
pub mod dag;
pub mod graph;
pub mod parsed;
pub mod scaffold;
pub mod waves;

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

use crate::cli::Ctx;
use crate::core::error::CliResult;

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
        PlanCmd::Check => check::run(ctx),
        PlanCmd::Graph { format, out } => graph::run(ctx, format, out),
        PlanCmd::Waves => waves::run(ctx),
    }
}
