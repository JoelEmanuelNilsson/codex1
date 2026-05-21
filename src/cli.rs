use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "codex1",
    version,
    about = "Repo-local setup and mission scaffold helper"
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
    Setup {
        #[command(subcommand)]
        command: SetupCommand,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum SetupCommand {
    Install(SetupRepoArgs),
    Enable(SetupRepoArgs),
    Disable(SetupRepoArgs),
    Uninstall(SetupRepoArgs),
    Status(SetupStatusArgs),
    Doctor(SetupStatusArgs),
    Backups {
        #[command(subcommand)]
        command: SetupBackupsCommand,
    },
}

#[derive(Clone, Debug, Args)]
pub struct SetupRepoArgs {
    #[arg(long, value_name = "PATH")]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Args)]
pub struct SetupStatusArgs {
    #[arg(long, value_name = "PATH")]
    pub repo: Option<PathBuf>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SetupBackupsCommand {
    List,
    Restore(SetupBackupRestoreArgs),
}

#[derive(Clone, Debug, Args)]
pub struct SetupBackupRestoreArgs {
    pub id: String,
    #[arg(long)]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub dry_run: bool,
}
