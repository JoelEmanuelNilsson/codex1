//! `codex1 plan` dispatcher.
//!
//! Foundation pins the subcommand enum and `dispatch`. Phase B unit 4 owns
//! the `Check` arm; units 3 and 5 own the other arms. Each sibling module
//! exposes a `run(ctx)` entry point.

pub mod check;
pub mod dag;
pub mod parsed;

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
    let not_implemented = |label: &str| {
        Err(CliError::NotImplemented {
            command: label.to_string(),
        })
    };
    match cmd {
        PlanCmd::Check => check::run(ctx),
        PlanCmd::ChooseLevel { .. } => not_implemented("plan choose-level"),
        PlanCmd::Scaffold { .. } => not_implemented("plan scaffold"),
        PlanCmd::Graph { .. } => not_implemented("plan graph"),
        PlanCmd::Waves => not_implemented("plan waves"),
    }
}
