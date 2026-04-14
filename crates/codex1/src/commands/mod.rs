use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

pub mod doctor;
pub mod qualify;
pub mod restore;
pub mod setup;
pub mod uninstall;

#[derive(Debug, Args, Clone)]
pub struct CommonArgs {
    #[arg(long)]
    pub json: bool,

    #[arg(long, value_name = "PATH")]
    pub repo_root: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct SetupArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(long, value_name = "PATH")]
    pub backup_root: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Args, Clone)]
pub struct DoctorArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(long = "runtime-override", value_name = "KEY=VALUE")]
    pub runtime_overrides: Vec<String>,
}

#[derive(Debug, Args, Clone)]
pub struct RestoreArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(long, value_name = "PATH")]
    pub backup_root: Option<PathBuf>,

    #[arg(long)]
    pub backup_id: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct UninstallArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(long, value_name = "PATH")]
    pub backup_root: Option<PathBuf>,

    #[arg(long)]
    pub backup_id: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct QualifyArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub live: bool,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub self_hosting: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    Setup(SetupArgs),
    Doctor(DoctorArgs),
    QualifyCodex(QualifyArgs),
    Restore(RestoreArgs),
    Uninstall(UninstallArgs),
}

pub fn resolve_repo_root(explicit: Option<&std::path::Path>) -> Result<PathBuf> {
    let start = match explicit {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().context("resolve current working directory")?,
    };
    let start = std::fs::canonicalize(&start)
        .with_context(|| format!("canonicalize repo root {}", start.display()))?;

    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Ok(ancestor.to_path_buf());
        }
    }

    Ok(start)
}
