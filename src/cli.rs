use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::layout::{ArtifactKind, SubplanState};

#[derive(Debug, Parser)]
#[command(
    name = "codex1",
    version,
    about = "Deterministic artifact workflow helper"
)]
pub struct Cli {
    #[arg(long, global = true, help = "Emit a stable JSON envelope")]
    pub json: bool,
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Repository root to use"
    )]
    pub repo_root: Option<PathBuf>,
    #[arg(long, global = true, value_name = "ID", help = "Mission id")]
    pub mission: Option<String>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    Init,
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
    Interview(InterviewArgs),
    Inspect,
    Subplan {
        #[command(subcommand)]
        command: SubplanCommand,
    },
    Receipt {
        #[command(subcommand)]
        command: ReceiptCommand,
    },
    Loop {
        #[command(subcommand)]
        command: LoopCommand,
    },
    Ralph {
        #[command(subcommand)]
        command: RalphCommand,
    },
    Doctor,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TemplateCommand {
    List,
    Show { kind: ArtifactKindArg },
}

#[derive(Clone, Debug, Args)]
pub struct InterviewArgs {
    pub kind: ArtifactKindArg,
    #[arg(long, value_name = "FILE", help = "JSON answers file")]
    pub answers: Option<PathBuf>,
    #[arg(long, help = "Overwrite singleton artifacts")]
    pub overwrite: bool,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SubplanCommand {
    Move {
        #[arg(long)]
        id: String,
        #[arg(long)]
        to: SubplanStateArg,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum ReceiptCommand {
    Append {
        #[arg(long)]
        message: String,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum LoopCommand {
    Start {
        #[arg(long)]
        mode: String,
        #[arg(long)]
        message: String,
    },
    Pause {
        #[arg(long)]
        reason: Option<String>,
    },
    Resume,
    Stop {
        #[arg(long)]
        reason: Option<String>,
    },
    Status,
}

#[derive(Clone, Debug, Subcommand)]
pub enum RalphCommand {
    StopHook,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ArtifactKindArg {
    Prd,
    Plan,
    ResearchPlan,
    Research,
    Spec,
    Subplan,
    Adr,
    Review,
    Triage,
    Proof,
    Closeout,
}

impl From<ArtifactKindArg> for ArtifactKind {
    fn from(value: ArtifactKindArg) -> Self {
        match value {
            ArtifactKindArg::Prd => Self::Prd,
            ArtifactKindArg::Plan => Self::Plan,
            ArtifactKindArg::ResearchPlan => Self::ResearchPlan,
            ArtifactKindArg::Research => Self::Research,
            ArtifactKindArg::Spec => Self::Spec,
            ArtifactKindArg::Subplan => Self::Subplan,
            ArtifactKindArg::Adr => Self::Adr,
            ArtifactKindArg::Review => Self::Review,
            ArtifactKindArg::Triage => Self::Triage,
            ArtifactKindArg::Proof => Self::Proof,
            ArtifactKindArg::Closeout => Self::Closeout,
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum SubplanStateArg {
    Ready,
    Active,
    Done,
    Paused,
    Superseded,
}

impl From<SubplanStateArg> for SubplanState {
    fn from(value: SubplanStateArg) -> Self {
        match value {
            SubplanStateArg::Ready => Self::Ready,
            SubplanStateArg::Active => Self::Active,
            SubplanStateArg::Done => Self::Done,
            SubplanStateArg::Paused => Self::Paused,
            SubplanStateArg::Superseded => Self::Superseded,
        }
    }
}
