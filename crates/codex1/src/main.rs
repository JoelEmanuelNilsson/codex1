mod commands;
mod internal;
mod support_surface;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "codex1", version, about = "Codex1 Harness support CLI")]
struct Cli {
    #[command(subcommand)]
    command: TopLevelCommand,
}

#[derive(Debug, Subcommand)]
enum TopLevelCommand {
    Setup(commands::SetupArgs),
    Init(commands::InitArgs),
    Doctor(commands::DoctorArgs),
    QualifyCodex(commands::QualifyArgs),
    Restore(commands::RestoreArgs),
    Uninstall(commands::UninstallArgs),
    Internal(internal::InternalArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        TopLevelCommand::Setup(args) => commands::setup::run(args),
        TopLevelCommand::Init(args) => commands::init::run(args),
        TopLevelCommand::Doctor(args) => commands::doctor::run(args),
        TopLevelCommand::QualifyCodex(args) => commands::qualify::run(args),
        TopLevelCommand::Restore(args) => commands::restore::run(args),
        TopLevelCommand::Uninstall(args) => commands::uninstall::run(args),
        TopLevelCommand::Internal(args) => internal::run(args),
    }
}
