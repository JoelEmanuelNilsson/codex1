use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::Parser;
use serde::Serialize;
use serde_json::json;

use crate::cli::{Cli, Commands};
use crate::envelope;
use crate::error::{Codex1Error, Result};
use crate::layout::{descriptors, MissionLayout};
use crate::paths::discover_repo_root;
use crate::setup;

pub fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|arg| arg == "--json");
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(err) => {
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = err.print();
                return ExitCode::SUCCESS;
            }
            if json_mode {
                let wrapped = Codex1Error::Argument(err.to_string());
                print_json(&envelope::error(&wrapped));
            } else {
                let _ = err.print();
            }
            return ExitCode::from(2);
        }
    };

    match run_cli(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if json_mode {
                print_json(&envelope::error(&error));
            } else {
                eprintln!("{}: {}", error.code().as_str(), error);
            }
            ExitCode::from(1)
        }
    }
}

fn run_cli(cli: Cli) -> Result<()> {
    match cli.command.clone() {
        Commands::Init => cmd_init(&cli),
        Commands::Setup { command } => setup::run(cli.json, cli.repo_root.clone(), command),
    }
}

fn cmd_init(cli: &Cli) -> Result<()> {
    let layout = explicit_layout(cli)?;
    layout.create_dirs()?;
    let data = json!({
        "mission_id": layout.mission_id,
        "repo_root": layout.repo_root,
        "mission_dir": layout.mission_dir,
        "artifacts": descriptors(&layout),
    });
    if cli.json {
        print_json(&envelope::success(data));
    } else {
        println!("Initialized mission {}", layout.mission_id);
        println!("{}", layout.mission_dir.display());
    }
    Ok(())
}

fn explicit_layout(cli: &Cli) -> Result<MissionLayout> {
    let repo_root = discover_repo_root(cli.repo_root.clone())?;
    let mission_id = cli
        .mission
        .clone()
        .ok_or_else(|| Codex1Error::Argument("--mission is required".into()))?;
    MissionLayout::new(repo_root, mission_id)
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(_) => println!(
            r#"{{"ok":false,"error":{{"code":"IO_ERROR","message":"failed to serialize output"}}}}"#
        ),
    }
}
