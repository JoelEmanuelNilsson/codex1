use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};
use std::time::Instant;

use chrono::Utc;
use clap::error::ErrorKind;
use clap::Parser;
use serde::Serialize;
use serde_json::{json, Value};

use crate::cli::{
    Cli, Commands, InterviewArgs, LoopCommand, RalphCommand, ReceiptCommand, SubplanCommand,
    TemplateCommand,
};
use crate::envelope;
use crate::error::{Codex1Error, IoContext, Result};
use crate::event;
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
    let started = Instant::now();
    let layout = explicit_layout(cli)?;
    layout.create_dirs()?;
    let warnings: Vec<_> = event::append_best_effort(
        &layout,
        &event::EventRecord::mission_initialized(&layout, started.elapsed()),
    )
    .into_iter()
    .collect();
    let data = json!({
        "mission_id": layout.mission_id,
        "repo_root": layout.repo_root,
        "mission_dir": layout.mission_dir,
        "artifacts": descriptors(&layout),
    });
    if cli.json {
        print_json(&envelope::success_with_warnings(data, warnings));
    } else {
        print_event_warnings(&warnings);
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
    let started = Instant::now();
    let layout = resolve_layout(cli)?;
    if cli.json && args.answers.is_none() {
        return Err(Codex1Error::Argument(
            "interactive interviews in --json mode require --answers".into(),
        ));
    }
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
    let path = match artifact_write_path(&layout, kind, &answers, args.overwrite) {
        Ok(path) => path,
        Err(error) => {
            log_artifact_write_failed(
                &layout,
                kind,
                template.version,
                args.overwrite,
                &error,
                started.elapsed(),
            );
            return Err(error);
        }
    };
    if let Err(error) = write_new_or_overwrite(
        &layout,
        &path,
        &content,
        args.overwrite || !kind.is_singleton(),
    ) {
        log_artifact_write_failed(
            &layout,
            kind,
            template.version,
            args.overwrite,
            &error,
            started.elapsed(),
        );
        return Err(error);
    }
    let mut warnings = Vec::new();
    if let Ok(record) = event::EventRecord::artifact_written(
        &layout,
        kind,
        template.version,
        args.overwrite,
        &path,
        started.elapsed(),
    ) {
        if let Some(warning) = event::append_best_effort(&layout, &record) {
            warnings.push(warning);
        }
    }
    let data = json!({
        "kind": kind,
        "path": path,
    });
    if cli.json {
        print_json(&envelope::success_with_warnings(data, warnings));
    } else {
        print_event_warnings(&warnings);
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
        println!("  events: {}", inventory.artifacts.events);
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
    let started = Instant::now();
    let layout = resolve_layout(cli)?;
    let mut warnings = Vec::new();
    match command {
        SubplanCommand::Move { id, to } => {
            let target_state: SubplanState = to.into();
            let source = match find_subplan(&layout, &id) {
                Ok(source) => source,
                Err(error) => {
                    log_subplan_move_failed(&layout, target_state, &error, started.elapsed());
                    return Err(error);
                }
            };
            let source_state = match subplan_lifecycle_for_path(&source) {
                Ok(state) => state,
                Err(error) => {
                    log_subplan_move_failed(&layout, target_state, &error, started.elapsed());
                    return Err(error);
                }
            };
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
                let error = Codex1Error::ArtifactValidation(format!(
                    "target already exists: {}",
                    target.display()
                ));
                log_subplan_move_failed(&layout, target_state, &error, started.elapsed());
                return Err(error);
            }
            if let Err(error) = ensure_existing_contained(&layout.mission_dir, &source) {
                log_subplan_move_failed(&layout, target_state, &error, started.elapsed());
                return Err(error);
            }
            if let Err(error) = create_dir_all_contained(
                &layout.mission_dir,
                Path::new("SUBPLANS").join(target_state.as_str()),
            ) {
                log_subplan_move_failed(&layout, target_state, &error, started.elapsed());
                return Err(error);
            }
            if let Err(error) = fs::rename(&source, &target).io_context(format!(
                "failed to move {} to {}",
                source.display(),
                target.display()
            )) {
                log_subplan_move_failed(&layout, target_state, &error, started.elapsed());
                return Err(error);
            }
            if let Ok(record) = event::EventRecord::subplan_moved(
                &layout,
                &source,
                &target,
                source_state,
                target_state,
                started.elapsed(),
            ) {
                if let Some(warning) = event::append_best_effort(&layout, &record) {
                    warnings.push(warning);
                }
            }
            let data = json!({
                "from": source,
                "to": target,
            });
            if cli.json {
                print_json(&envelope::success_with_warnings(data, warnings));
            } else {
                print_event_warnings(&warnings);
                println!("Moved subplan to {}", target.display());
            }
        }
    }
    Ok(())
}

fn cmd_receipt(cli: &Cli, command: ReceiptCommand) -> Result<()> {
    let started = Instant::now();
    let layout = resolve_layout(cli)?;
    layout.create_dirs()?;
    let mut warnings = Vec::new();
    match command {
        ReceiptCommand::Append { message } => {
            let path = layout.receipts_dir().join("receipts.jsonl");
            ensure_contained_for_write(&layout.mission_dir, &path)?;
            let record = json!({
                "version": 1,
                "timestamp": Utc::now(),
                "message": message,
            });
            let mut file = match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .io_context(format!("failed to open {}", path.display()))
            {
                Ok(file) => file,
                Err(error) => {
                    log_receipt_append_failed(&layout, &error, started.elapsed());
                    return Err(error);
                }
            };
            if let Err(error) = writeln!(file, "{record}")
                .io_context(format!("failed to append {}", path.display()))
            {
                log_receipt_append_failed(&layout, &error, started.elapsed());
                return Err(error);
            }
            if let Ok(record) =
                event::EventRecord::receipt_appended(&layout, &path, started.elapsed())
            {
                if let Some(warning) = event::append_best_effort(&layout, &record) {
                    warnings.push(warning);
                }
            }
            if cli.json {
                print_json(&envelope::success_with_warnings(
                    json!({ "path": path }),
                    warnings,
                ));
            } else {
                print_event_warnings(&warnings);
                println!("Appended optional receipt to {}", path.display());
            }
        }
    }
    Ok(())
}

fn cmd_loop(cli: &Cli, command: LoopCommand) -> Result<()> {
    let started = Instant::now();
    let layout = resolve_layout(cli)?;
    let mut warnings = Vec::new();
    let state = match command {
        LoopCommand::Start { mode, message } => {
            layout.create_dirs()?;
            let message_present = !message.trim().is_empty();
            let mode_for_event = mode.clone();
            let state = match LoopState::start(mode, message, &layout) {
                Ok(state) => state,
                Err(error) => {
                    let record = event::EventRecord::loop_start_failed(
                        &layout,
                        &mode_for_event,
                        message_present,
                        error.code().as_str(),
                        started.elapsed(),
                    );
                    let _ = event::append_best_effort(&layout, &record);
                    return Err(error);
                }
            };
            if let Err(error) = loop_state::write(&layout, &state) {
                let record = event::EventRecord::loop_start_failed(
                    &layout,
                    &state.mode,
                    message_present,
                    error.code().as_str(),
                    started.elapsed(),
                );
                let _ = event::append_best_effort(&layout, &record);
                return Err(error);
            }
            let record = event::EventRecord::loop_started(
                &layout,
                &state.mode,
                message_present,
                started.elapsed(),
            );
            if let Some(warning) = event::append_best_effort(&layout, &record) {
                warnings.push(warning);
            }
            state
        }
        LoopCommand::Pause { reason } => {
            let reason_present = reason
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty());
            let state = match loop_state::pause(&layout, reason) {
                Ok(state) => state,
                Err(error) => {
                    let record = event::EventRecord::loop_pause_failed(
                        &layout,
                        reason_present,
                        error.code().as_str(),
                        started.elapsed(),
                    );
                    let _ = event::append_best_effort(&layout, &record);
                    return Err(error);
                }
            };
            let record =
                event::EventRecord::loop_paused(&layout, reason_present, started.elapsed());
            if let Some(warning) = event::append_best_effort(&layout, &record) {
                warnings.push(warning);
            }
            state
        }
        LoopCommand::Resume => {
            let state = match loop_state::resume(&layout) {
                Ok(state) => state,
                Err(error) => {
                    let record = event::EventRecord::loop_resume_failed(
                        &layout,
                        error.code().as_str(),
                        started.elapsed(),
                    );
                    let _ = event::append_best_effort(&layout, &record);
                    return Err(error);
                }
            };
            let record = event::EventRecord::loop_resumed(&layout, started.elapsed());
            if let Some(warning) = event::append_best_effort(&layout, &record) {
                warnings.push(warning);
            }
            state
        }
        LoopCommand::Stop { reason } => {
            let reason_present = reason
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty());
            let state = match loop_state::stop(&layout, reason) {
                Ok(state) => state,
                Err(error) => {
                    let record = event::EventRecord::loop_stop_failed(
                        &layout,
                        reason_present,
                        error.code().as_str(),
                        started.elapsed(),
                    );
                    let _ = event::append_best_effort(&layout, &record);
                    return Err(error);
                }
            };
            let record =
                event::EventRecord::loop_stopped(&layout, reason_present, started.elapsed());
            if let Some(warning) = event::append_best_effort(&layout, &record) {
                warnings.push(warning);
            }
            state
        }
        LoopCommand::Status => loop_state::read(&layout)?,
    };
    if cli.json {
        print_json(&envelope::success_with_warnings(state, warnings));
    } else {
        print_event_warnings(&warnings);
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
    let exe = env::current_exe().io_context("failed to resolve current executable")?;
    let installed_command = run_installed_command_check(&exe)?;
    let loop_ralph_smoke = run_loop_ralph_smoke(&exe)?;
    let data = json!({
        "binary": exe,
        "templates_registered": template::all().len(),
        "mission_id_validation": "ok",
        "loop_schema_version": 1,
        "installed_command": installed_command,
        "loop_ralph_smoke": loop_ralph_smoke,
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
    if kind == ArtifactKind::Subplan {
        return allocate_subplan_path(layout, &base);
    }
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

fn allocate_subplan_path(layout: &MissionLayout, base: &str) -> Result<PathBuf> {
    let mut max_index = 0;
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
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some((prefix, suffix)) = file_name.split_once('-') else {
                continue;
            };
            if suffix == format!("{base}.md") {
                if let Ok(index) = prefix.parse::<usize>() {
                    max_index = max_index.max(index);
                }
            }
        }
    }

    let index = max_index + 1;
    if index >= 10_000 {
        return Err(Codex1Error::ArtifactValidation(format!(
            "could not allocate unique filename for subplan {}",
            base
        )));
    }
    let name = format!("{index:04}-{base}.md");
    safe_join(
        &layout.mission_dir,
        Path::new("SUBPLANS")
            .join(SubplanState::Ready.as_str())
            .join(name),
    )
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

fn subplan_lifecycle_for_path(path: &Path) -> Result<SubplanState> {
    let lifecycle = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| Codex1Error::MissionPath("subplan path has no lifecycle folder".into()))?;
    match lifecycle {
        "ready" => Ok(SubplanState::Ready),
        "active" => Ok(SubplanState::Active),
        "done" => Ok(SubplanState::Done),
        "paused" => Ok(SubplanState::Paused),
        "superseded" => Ok(SubplanState::Superseded),
        _ => Err(Codex1Error::MissionPath(format!(
            "unknown subplan lifecycle folder: {lifecycle}"
        ))),
    }
}

fn log_artifact_write_failed(
    layout: &MissionLayout,
    kind: ArtifactKind,
    template_version: u32,
    overwrite: bool,
    error: &Codex1Error,
    duration: std::time::Duration,
) {
    let record = event::EventRecord::artifact_write_failed(
        layout,
        kind,
        template_version,
        overwrite,
        error.code().as_str(),
        duration,
    );
    let _ = event::append_best_effort(layout, &record);
}

fn log_subplan_move_failed(
    layout: &MissionLayout,
    target_state: SubplanState,
    error: &Codex1Error,
    duration: std::time::Duration,
) {
    let record = event::EventRecord::subplan_move_failed(
        layout,
        target_state,
        error.code().as_str(),
        duration,
    );
    let _ = event::append_best_effort(layout, &record);
}

fn log_receipt_append_failed(
    layout: &MissionLayout,
    error: &Codex1Error,
    duration: std::time::Duration,
) {
    let record = event::EventRecord::receipt_append_failed(layout, error.code().as_str(), duration);
    let _ = event::append_best_effort(layout, &record);
}

fn run_installed_command_check(current_exe: &Path) -> Result<Value> {
    let (binary, source, on_path) = match find_on_path("codex1") {
        Some(path) => (path, "path", true),
        None => (current_exe.to_path_buf(), "current-exe", false),
    };
    let output = run_command(Command::new(&binary).current_dir(env::temp_dir()).args([
        "--json",
        "--mission",
        "../bad",
        "init",
    ]))?;
    let value = parse_json_output(&output, "installed command JSON-envelope smoke")?;
    let json_error_envelope = value.get("ok") == Some(&Value::Bool(false))
        && value
            .get("error")
            .and_then(Value::as_object)
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
            .is_some();
    if !json_error_envelope {
        return Err(Codex1Error::Argument(
            "installed command did not emit a JSON error envelope".into(),
        ));
    }
    Ok(json!({
        "binary": binary,
        "source": source,
        "on_path": on_path,
        "json_error_envelope": true,
    }))
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let executable_name = format!("{name}{}", env::consts::EXE_SUFFIX);
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(&executable_name))
        .find(|candidate| candidate.is_file())
}

fn run_loop_ralph_smoke(current_exe: &Path) -> Result<Value> {
    let root = env::temp_dir().join(format!(
        "codex1-doctor-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    fs::create_dir(&root).io_context(format!("failed to create {}", root.display()))?;
    let result = run_loop_ralph_smoke_in(current_exe, &root);
    let _ = fs::remove_dir_all(&root);
    result
}

fn run_loop_ralph_smoke_in(current_exe: &Path, root: &Path) -> Result<Value> {
    fs::create_dir(root.join(".git"))
        .io_context(format!("failed to create {}", root.join(".git").display()))?;

    let init = run_command(
        Command::new(current_exe)
            .current_dir(root)
            .args(["--json", "--repo-root"])
            .arg(root)
            .args(["--mission", "smoke", "init"]),
    )?;
    ensure_json_success(&init, "doctor init smoke")?;

    let start = run_command(
        Command::new(current_exe)
            .current_dir(root)
            .args(["--json", "--repo-root"])
            .arg(root)
            .args([
                "--mission",
                "smoke",
                "loop",
                "start",
                "--mode",
                "doctor",
                "--message",
                "Doctor continuation smoke.",
            ]),
    )?;
    ensure_json_success(&start, "doctor loop smoke")?;

    let ralph = run_command(
        Command::new(current_exe)
            .current_dir(root)
            .args(["--repo-root"])
            .arg(root)
            .args(["--mission", "smoke", "ralph", "stop-hook"]),
    )?;
    let value = parse_json_output(&ralph, "doctor Ralph smoke")?;
    if value.get("decision").and_then(Value::as_str) != Some("block") {
        return Err(Codex1Error::Loop(
            "Ralph smoke did not block an active loop".into(),
        ));
    }
    Ok(json!({ "blocked": true }))
}

fn ensure_json_success(output: &Output, label: &str) -> Result<Value> {
    let value = parse_json_output(output, label)?;
    if value.get("ok") != Some(&Value::Bool(true)) {
        return Err(Codex1Error::Argument(format!(
            "{label} did not emit a success envelope"
        )));
    }
    Ok(value)
}

fn parse_json_output(output: &Output, label: &str) -> Result<Value> {
    serde_json::from_slice(&output.stdout).map_err(|source| {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Codex1Error::Argument(format!(
            "{label} emitted invalid JSON: {source}; stdout={stdout:?}; stderr={stderr:?}"
        ))
    })
}

fn run_command(command: &mut Command) -> Result<Output> {
    command
        .output()
        .io_context(format!("failed to run diagnostic command: {command:?}"))
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(_) => println!(
            r#"{{"ok":false,"error":{{"code":"IO_ERROR","message":"failed to serialize output"}}}}"#
        ),
    }
}

fn print_event_warnings(warnings: &[event::EventWarning]) {
    for warning in warnings {
        eprintln!("{}: {}", warning.code, warning.message);
    }
}
