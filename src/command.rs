use std::fs::{self, OpenOptions};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Utc;
use clap::error::ErrorKind;
use clap::Parser;
use serde::Serialize;
use serde_json::json;

use crate::cli::{
    Cli, Commands, InterviewArgs, LoopCommand, RalphCommand, ReceiptCommand, SubplanCommand,
    TemplateCommand,
};
use crate::envelope;
use crate::error::{Codex1Error, IoContext, Result};
use crate::inspect;
use crate::interview;
use crate::layout::{descriptors, ArtifactKind, MissionLayout, SubplanState};
use crate::loop_state::{self, LoopState};
use crate::paths::{
    create_dir_all_contained, discover_repo_root, ensure_contained_for_write,
    ensure_existing_contained, safe_join, slug, validate_mission_id,
};
use crate::ralph;
use crate::render::{render_markdown, render_template_outline, AnswerValue, Answers};
use crate::template;

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
        Commands::Template { command } => cmd_template(&cli, command),
        Commands::Interview(args) => cmd_interview(&cli, args),
        Commands::Inspect => cmd_inspect(&cli),
        Commands::Subplan { command } => cmd_subplan(&cli, command),
        Commands::Receipt { command } => cmd_receipt(&cli, command),
        Commands::Loop { command } => cmd_loop(&cli, command),
        Commands::Ralph { command } => cmd_ralph(&cli, command),
        Commands::Doctor => cmd_doctor(&cli),
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

fn cmd_template(cli: &Cli, command: TemplateCommand) -> Result<()> {
    template::validate_registry()?;
    match command {
        TemplateCommand::List => {
            let templates = template::all();
            if cli.json {
                let data: Vec<_> = templates
                    .into_iter()
                    .map(|template| {
                        json!({
                            "kind": template.kind,
                            "version": template.version,
                            "name": template.name,
                            "sections": template.sections.len(),
                        })
                    })
                    .collect();
                print_json(&envelope::success(data));
            } else {
                for template in templates {
                    println!(
                        "{} v{} - {}",
                        template.kind, template.version, template.name
                    );
                }
            }
        }
        TemplateCommand::Show { kind } => {
            let template = template::get(kind.into());
            if cli.json {
                print_json(&envelope::success(template));
            } else {
                print!("{}", render_template_outline(&template));
            }
        }
    }
    Ok(())
}

fn cmd_interview(cli: &Cli, args: InterviewArgs) -> Result<()> {
    let layout = resolve_layout(cli)?;
    layout.create_dirs()?;
    let kind: ArtifactKind = args.kind.into();
    let template = template::get(kind);
    let answers = match args.answers {
        Some(path) => interview::read_answers_file(&path)?,
        None => {
            let stdin = io::stdin();
            let stdout = io::stdout();
            interview::run_interactive(&template, BufReader::new(stdin.lock()), stdout.lock())?
        }
    };
    let content = render_markdown(&template, &answers)?;
    let path = artifact_write_path(&layout, kind, &answers, args.overwrite)?;
    write_new_or_overwrite(
        &layout,
        &path,
        &content,
        args.overwrite || !kind.is_singleton(),
    )?;
    if cli.json {
        print_json(&envelope::success(json!({
            "kind": kind,
            "path": path,
        })));
    } else {
        println!("Wrote {} artifact to {}", kind, path.display());
    }
    Ok(())
}

fn cmd_inspect(cli: &Cli) -> Result<()> {
    let layout = resolve_layout(cli)?;
    let inventory = inspect::inspect(&layout)?;
    if cli.json {
        print_json(&envelope::success(inventory));
    } else {
        println!("Mission: {}", inventory.mission_id);
        println!("Directory: {}", inventory.mission_dir);
        println!("Artifacts:");
        println!("  prd: {}", inventory.artifacts.prd);
        println!("  plan: {}", inventory.artifacts.plan);
        println!("  research_plan: {}", inventory.artifacts.research_plan);
        println!("  research: {}", inventory.artifacts.research);
        println!("  specs: {}", inventory.artifacts.specs);
        println!("  subplans: {}", inventory.artifacts.subplans);
        println!("  adrs: {}", inventory.artifacts.adrs);
        println!("  reviews: {}", inventory.artifacts.reviews);
        println!("  triage: {}", inventory.artifacts.triage);
        println!("  proofs: {}", inventory.artifacts.proofs);
        println!("  closeout: {}", inventory.artifacts.closeout);
        println!(
            "  optional_receipts: {}",
            inventory.artifacts.optional_receipts
        );
        if !inventory.mechanical_warnings.is_empty() {
            println!("Mechanical warnings:");
            for warning in inventory.mechanical_warnings {
                println!("  {} {}", warning.code, warning.detail);
            }
        }
    }
    Ok(())
}

fn cmd_subplan(cli: &Cli, command: SubplanCommand) -> Result<()> {
    let layout = resolve_layout(cli)?;
    match command {
        SubplanCommand::Move { id, to } => {
            let target_state: SubplanState = to.into();
            let source = find_subplan(&layout, &id)?;
            let file_name = source.file_name().ok_or_else(|| {
                Codex1Error::MissionPath("subplan source has no file name".into())
            })?;
            let target = safe_join(
                &layout.mission_dir,
                Path::new("SUBPLANS")
                    .join(target_state.as_str())
                    .join(file_name),
            )?;
            if target.exists() {
                return Err(Codex1Error::ArtifactValidation(format!(
                    "target already exists: {}",
                    target.display()
                )));
            }
            ensure_existing_contained(&layout.mission_dir, &source)?;
            create_dir_all_contained(
                &layout.mission_dir,
                Path::new("SUBPLANS").join(target_state.as_str()),
            )?;
            fs::rename(&source, &target).io_context(format!(
                "failed to move {} to {}",
                source.display(),
                target.display()
            ))?;
            if cli.json {
                print_json(&envelope::success(json!({
                    "from": source,
                    "to": target,
                })));
            } else {
                println!("Moved subplan to {}", target.display());
            }
        }
    }
    Ok(())
}

fn cmd_receipt(cli: &Cli, command: ReceiptCommand) -> Result<()> {
    let layout = resolve_layout(cli)?;
    layout.create_dirs()?;
    match command {
        ReceiptCommand::Append { message } => {
            let path = layout.receipts_dir().join("receipts.jsonl");
            ensure_contained_for_write(&layout.mission_dir, &path)?;
            let record = json!({
                "version": 1,
                "timestamp": Utc::now(),
                "message": message,
            });
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .io_context(format!("failed to open {}", path.display()))?;
            writeln!(file, "{record}")
                .io_context(format!("failed to append {}", path.display()))?;
            if cli.json {
                print_json(&envelope::success(json!({ "path": path })));
            } else {
                println!("Appended optional receipt to {}", path.display());
            }
        }
    }
    Ok(())
}

fn cmd_loop(cli: &Cli, command: LoopCommand) -> Result<()> {
    let layout = resolve_layout(cli)?;
    layout.create_dirs()?;
    let state = match command {
        LoopCommand::Start { mode, message } => {
            let state = LoopState::start(mode, message, &layout)?;
            loop_state::write(&layout, &state)?;
            state
        }
        LoopCommand::Pause { reason } => loop_state::pause(&layout, reason)?,
        LoopCommand::Resume => loop_state::resume(&layout)?,
        LoopCommand::Stop { reason } => loop_state::stop(&layout, reason)?,
        LoopCommand::Status => loop_state::read(&layout)?,
    };
    if cli.json {
        print_json(&envelope::success(state));
    } else {
        println!(
            "Loop active={} paused={} mode={}",
            state.active, state.paused, state.mode
        );
    }
    Ok(())
}

fn cmd_ralph(cli: &Cli, command: RalphCommand) -> Result<()> {
    match command {
        RalphCommand::StopHook => {
            let output = ralph::stop_hook(cli.repo_root.clone(), cli.mission.clone());
            print_json(&output);
        }
    }
    Ok(())
}

fn cmd_doctor(cli: &Cli) -> Result<()> {
    template::validate_registry()?;
    validate_mission_id("doctor-smoke")?;
    let exe = std::env::current_exe().io_context("failed to resolve current executable")?;
    let data = json!({
        "binary": exe,
        "templates_registered": template::all().len(),
        "mission_id_validation": "ok",
        "loop_schema_version": 1,
        "anti_oracle": "inspect reports inventory and mechanical warnings only",
    });
    if cli.json {
        print_json(&envelope::success(data));
    } else {
        println!("codex1 doctor ok");
        println!("templates_registered={}", template::all().len());
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

fn resolve_layout(cli: &Cli) -> Result<MissionLayout> {
    if cli.mission.is_some() {
        return explicit_layout(cli);
    }
    let repo_root = discover_repo_root(cli.repo_root.clone())?;
    let cwd = std::env::current_dir().io_context("failed to read current directory")?;
    MissionLayout::from_cwd(repo_root, &cwd).ok_or_else(|| {
        Codex1Error::Argument("--mission is required when cwd is not inside a mission".into())
    })
}

fn artifact_write_path(
    layout: &MissionLayout,
    kind: ArtifactKind,
    answers: &Answers,
    overwrite: bool,
) -> Result<PathBuf> {
    if kind.is_singleton() {
        let path = layout.singleton_path(kind)?;
        if path.exists() && !overwrite {
            return Err(Codex1Error::ArtifactValidation(format!(
                "artifact already exists: {}",
                path.display()
            )));
        }
        return Ok(path);
    }

    let dir = layout.collection_dir(kind)?;
    let relative_dir = dir.strip_prefix(&layout.mission_dir).map_err(|_| {
        Codex1Error::MissionPath(format!(
            "artifact directory escapes mission: {}",
            dir.display()
        ))
    })?;
    create_dir_all_contained(&layout.mission_dir, relative_dir)?;
    let title = match answers.get("title") {
        Some(AnswerValue::Text(text)) => text.as_str(),
        _ => kind.title(),
    };
    let base = slug(title);
    for index in 1..10_000 {
        let name = format!("{index:04}-{base}.md");
        let candidate = dir.join(&name);
        let relative = candidate.strip_prefix(&layout.mission_dir).unwrap();
        let path = safe_join(&layout.mission_dir, relative)?;
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(Codex1Error::ArtifactValidation(format!(
        "could not allocate unique filename for {}",
        kind
    )))
}

fn write_new_or_overwrite(
    layout: &MissionLayout,
    path: &Path,
    content: &str,
    allow_overwrite: bool,
) -> Result<()> {
    crate::paths::ensure_contained_for_write(&layout.mission_dir, path)?;
    if path.exists() && !allow_overwrite {
        return Err(Codex1Error::ArtifactValidation(format!(
            "artifact already exists: {}",
            path.display()
        )));
    }
    fs::write(path, content).io_context(format!("failed to write {}", path.display()))
}

fn find_subplan(layout: &MissionLayout, id: &str) -> Result<PathBuf> {
    if id.contains('/') || id.contains('\\') || id.contains('\0') || id == "." || id == ".." {
        return Err(Codex1Error::MissionPath("unsafe subplan id".into()));
    }
    let mut matches = Vec::new();
    for state in SubplanState::ALL {
        let dir = layout.subplans_dir().join(state.as_str());
        let Ok(metadata) = fs::symlink_metadata(&dir) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            return Err(Codex1Error::MissionPath(format!(
                "subplan lifecycle directory must not be a symlink: {}",
                dir.display()
            )));
        }
        if !metadata.is_dir() {
            continue;
        }
        ensure_existing_contained(&layout.mission_dir, &dir)?;
        for entry in fs::read_dir(&dir).io_context(format!("failed to read {}", dir.display()))? {
            let entry = entry.io_context(format!("failed to read entry in {}", dir.display()))?;
            let file_type = entry
                .file_type()
                .io_context(format!("failed to inspect entry in {}", dir.display()))?;
            if file_type.is_symlink() {
                return Err(Codex1Error::MissionPath(format!(
                    "subplan file must not be a symlink: {}",
                    entry.path().display()
                )));
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            ensure_existing_contained(&layout.mission_dir, &path)?;
            if path.extension().and_then(|value| value.to_str()) != Some("md") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            let file = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if stem == id || file == id || file == format!("{id}.md") {
                matches.push(path);
            }
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(Codex1Error::ArtifactValidation(format!(
            "no subplan found for id {id}"
        ))),
        _ => Err(Codex1Error::ArtifactValidation(format!(
            "multiple subplans matched id {id}"
        ))),
    }
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(_) => println!(
            r#"{{"ok":false,"error":{{"code":"IO_ERROR","message":"failed to serialize output"}}}}"#
        ),
    }
}
