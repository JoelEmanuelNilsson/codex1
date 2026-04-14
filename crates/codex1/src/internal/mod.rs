use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use codex1_core::{
    ArtifactDocument, CloseoutRecord, ContradictionInput, EffectiveConfigReport,
    ExecutionGraphValidationReport, ExecutionPackageInput, MissionBootstrapReport,
    MissionInitInput, MissionPaths, MissionStateFrontmatter, OutcomeLockFrontmatter,
    PackageValidationReport, PlanningWriteInput, ProgramBlueprintFrontmatter, ReplanLogInput,
    ResolveResumeInput, ResolveResumeReport, ReviewBundleInput, ReviewBundleValidationReport,
    ReviewResultInput, SelectionAcknowledgementInput, SelectionConsumptionInput,
    SelectionResolutionInput, SelectionState, SelectionStateInput, StopHookOutput, Verdict,
    WaitingRequestAcknowledgementInput, WorkstreamSpecFrontmatter, WriterPacketInput,
    WriterPacketValidationReport, acknowledge_selection_request, acknowledge_waiting_request,
    append_contradiction, append_replan_log, compile_execution_package, compile_review_bundle,
    consume_selection_wait, derive_writer_packet, determine_stop_decision, initialize_mission,
    list_non_terminal_missions, load_closeouts, load_state, open_selection_wait,
    rebuild_state_from_files, record_review_result, resolve_resume, resolve_selection_wait,
    validate_execution_graph, validate_execution_package, validate_review_bundle,
    validate_writer_packet, write_closeout, write_planning_artifacts,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Args)]
pub struct InternalArgs {
    #[command(subcommand)]
    command: InternalCommand,
}

#[derive(Debug, Subcommand)]
enum InternalCommand {
    StopHook,
    #[command(name = "repair-state", visible_alias = "rebuild-state")]
    RebuildState {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    #[command(
        name = "validate-mission-artifacts",
        visible_alias = "validate-artifacts"
    )]
    ValidateArtifacts {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    #[command(name = "inspect-effective-config", visible_alias = "effective-config")]
    EffectiveConfig {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    InitMission {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    #[command(name = "materialize-plan", visible_alias = "write-blueprint")]
    WriteBlueprint {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    CompileExecutionPackage {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ValidateExecutionPackage {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long)]
        package_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    CompileReviewBundle {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    DeriveWriterPacket {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ValidateWriterPacket {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long)]
        packet_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ValidateReviewBundle {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long)]
        bundle_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    #[command(name = "record-review-outcome", visible_alias = "record-review-result")]
    RecordReviewResult {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    RecordContradiction {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    #[command(name = "append-replan-log", visible_alias = "write-replan-log")]
    WriteReplanLog {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    OpenSelectionWait {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ResolveResume {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ResolveSelectionWait {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    #[command(name = "clear-selection-wait", visible_alias = "consume-selection")]
    ConsumeSelection {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long)]
        selection_request_id: Option<String>,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    AcknowledgeWaitingRequest {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    AcknowledgeSelectionRequest {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    #[command(name = "append-closeout", visible_alias = "write-closeout")]
    WriteCloseout {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value = "-")]
        input_json: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
}

pub fn run(args: InternalArgs) -> Result<()> {
    match args.command {
        InternalCommand::StopHook => run_stop_hook(),
        InternalCommand::RebuildState {
            repo_root,
            mission_id,
            json,
        } => run_rebuild_state(repo_root, &mission_id, json),
        InternalCommand::ValidateArtifacts {
            repo_root,
            mission_id,
            json,
        } => run_validate_artifacts(repo_root, &mission_id, json),
        InternalCommand::EffectiveConfig { repo_root, json } => {
            run_effective_config(repo_root, json)
        }
        InternalCommand::InitMission {
            repo_root,
            input_json,
            json,
        } => run_init_mission(repo_root, &input_json, json),
        InternalCommand::WriteBlueprint {
            repo_root,
            input_json,
            json,
        } => run_write_blueprint(repo_root, &input_json, json),
        InternalCommand::CompileExecutionPackage {
            repo_root,
            input_json,
            json,
        } => run_compile_execution_package(repo_root, &input_json, json),
        InternalCommand::ValidateExecutionPackage {
            repo_root,
            mission_id,
            package_id,
            json,
        } => run_validate_execution_package(repo_root, &mission_id, &package_id, json),
        InternalCommand::CompileReviewBundle {
            repo_root,
            input_json,
            json,
        } => run_compile_review_bundle(repo_root, &input_json, json),
        InternalCommand::DeriveWriterPacket {
            repo_root,
            input_json,
            json,
        } => run_derive_writer_packet(repo_root, &input_json, json),
        InternalCommand::ValidateWriterPacket {
            repo_root,
            mission_id,
            packet_id,
            json,
        } => run_validate_writer_packet(repo_root, &mission_id, &packet_id, json),
        InternalCommand::ValidateReviewBundle {
            repo_root,
            mission_id,
            bundle_id,
            json,
        } => run_validate_review_bundle(repo_root, &mission_id, &bundle_id, json),
        InternalCommand::RecordReviewResult {
            repo_root,
            input_json,
            json,
        } => run_record_review_result(repo_root, &input_json, json),
        InternalCommand::RecordContradiction {
            repo_root,
            input_json,
            json,
        } => run_record_contradiction(repo_root, &input_json, json),
        InternalCommand::WriteReplanLog {
            repo_root,
            input_json,
            json,
        } => run_write_replan_log(repo_root, &input_json, json),
        InternalCommand::OpenSelectionWait {
            repo_root,
            input_json,
            json,
        } => run_open_selection_wait(repo_root, &input_json, json),
        InternalCommand::ResolveResume {
            repo_root,
            input_json,
            json,
        } => run_resolve_resume(repo_root, &input_json, json),
        InternalCommand::ResolveSelectionWait {
            repo_root,
            input_json,
            json,
        } => run_resolve_selection_wait(repo_root, &input_json, json),
        InternalCommand::ConsumeSelection {
            repo_root,
            mission_id,
            selection_request_id,
            json,
        } => run_consume_selection(
            repo_root,
            &mission_id,
            selection_request_id.as_deref(),
            json,
        ),
        InternalCommand::AcknowledgeWaitingRequest {
            repo_root,
            mission_id,
            input_json,
            json,
        } => run_acknowledge_waiting_request(repo_root, &mission_id, &input_json, json),
        InternalCommand::AcknowledgeSelectionRequest {
            repo_root,
            input_json,
            json,
        } => run_acknowledge_selection_request(repo_root, &input_json, json),
        InternalCommand::WriteCloseout {
            repo_root,
            mission_id,
            input_json,
            json,
        } => run_write_closeout(repo_root, &mission_id, &input_json, json),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopHookInput {
    cwd: String,
}

fn run_stop_hook() -> Result<()> {
    let input: StopHookInput = read_stdin_json()?;
    let repo_root = PathBuf::from(input.cwd);
    let ralph_root = repo_root.join(".ralph");
    let existing_selection_state =
        match load_selection_state(&ralph_root.join("selection-state.json")) {
            Ok(state) => state,
            Err(error) => {
                let output = StopHookOutput {
                    continue_processing: true,
                    decision: Some("block".to_string()),
                    reason: Some(format!(
                        "Repair malformed selection state before continuing: {}",
                        error
                    )),
                    system_message: None,
                };
                println!(
                    "{}",
                    serde_json::to_string(&output)
                        .context("failed to serialize stop-hook output")?
                );
                return Ok(());
            }
        };
    if let Some(selection_state) = existing_selection_state
        && selection_state.cleared_at.is_none()
    {
        let report = match resolve_resume(
            &repo_root,
            &ResolveResumeInput {
                mission_id: None,
                live_child_lanes: Vec::new(),
            },
        ) {
            Ok(report) => report,
            Err(error) => {
                let output =
                    block_stop_output(format!("Repair resume state before continuing: {}", error));
                println!(
                    "{}",
                    serde_json::to_string(&output)
                        .context("failed to serialize stop-hook output")?
                );
                return Ok(());
            }
        };
        let output = match report.resume_status {
            codex1_core::ResumeStatus::Terminal => {
                let mission_id = match report.selected_mission_id.clone() {
                    Some(mission_id) => mission_id,
                    None => {
                        return Ok(println_and_return_output(block_stop_output(
                            "Repair terminal mission state before continuing: resolved selection has no mission id.",
                        ))?);
                    }
                };
                let paths = MissionPaths::new(&repo_root, mission_id);
                if latest_closeout_is_terminal(&paths)? {
                    StopHookOutput {
                        continue_processing: true,
                        decision: None,
                        reason: None,
                        system_message: None,
                    }
                } else {
                    block_stop_output(
                        "Repair terminal mission state before continuing: latest closeout is not terminal.",
                    )
                }
            }
            codex1_core::ResumeStatus::WaitingSelection => {
                let current_selection_state =
                    load_selection_state(&ralph_root.join("selection-state.json"))?
                        .context("selection wait report returned without selection state")?;
                let emitted_selection_state =
                    if current_selection_state.request_emitted_at.is_none() {
                        acknowledge_selection_request(
                            &ralph_root,
                            &SelectionAcknowledgementInput {
                                selection_request_id: current_selection_state
                                    .selection_request_id
                                    .clone(),
                            },
                        )?
                    } else {
                        current_selection_state
                    };
                StopHookOutput::for_selection_wait(&emitted_selection_state)
            }
            codex1_core::ResumeStatus::WaitingNeedsUser => {
                let mission_id = report
                    .selected_mission_id
                    .clone()
                    .context("resolved selection did not bind a mission id")?;
                let paths = MissionPaths::new(&repo_root, mission_id);
                let state = load_state(&paths.state_json())?
                    .context("waiting selection resolved without mission state")?;
                if state.request_emitted_at.is_none()
                    && let Some(waiting_request_id) = state.waiting_request_id.clone()
                    && latest_closeout_supports_waiting_ack(&paths, &waiting_request_id)?
                    && let Err(error) = acknowledge_waiting_request(
                        &paths,
                        &WaitingRequestAcknowledgementInput { waiting_request_id },
                    )
                {
                    let output = StopHookOutput {
                        continue_processing: true,
                        decision: Some("block".to_string()),
                        reason: Some(format!(
                            "Repair waiting emission acknowledgement before continuing: {}",
                            error
                        )),
                        system_message: None,
                    };
                    println!(
                        "{}",
                        serde_json::to_string(&output)
                            .context("failed to serialize stop-hook output")?
                    );
                    return Ok(());
                }
                StopHookOutput {
                    continue_processing: true,
                    decision: None,
                    reason: None,
                    system_message: Some(report.next_action),
                }
            }
            codex1_core::ResumeStatus::ActionableNonTerminal
            | codex1_core::ResumeStatus::InterruptedCycle
            | codex1_core::ResumeStatus::ContradictoryState => StopHookOutput {
                continue_processing: true,
                decision: Some("block".to_string()),
                reason: Some(report.next_action),
                system_message: None,
            },
            _ => StopHookOutput {
                continue_processing: true,
                decision: None,
                reason: None,
                system_message: None,
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("failed to serialize stop-hook output")?
        );
        return Ok(());
    }
    let missions_root = repo_root.join(".ralph").join("missions");
    let active = match list_non_terminal_missions(&missions_root) {
        Ok(active) => active,
        Err(error) => {
            let output = block_stop_output(format!(
                "Repair mission discovery before continuing: {}",
                error
            ));
            println!(
                "{}",
                serde_json::to_string(&output).context("failed to serialize stop-hook output")?
            );
            return Ok(());
        }
    };

    let output = match active.len() {
        0 => StopHookOutput {
            continue_processing: true,
            decision: None,
            reason: None,
            system_message: None,
        },
        1 => {
            let (mission_id, state) = &active[0];
            match resolve_resume(
                &repo_root,
                &ResolveResumeInput {
                    mission_id: Some(mission_id.clone()),
                    live_child_lanes: Vec::new(),
                },
            ) {
                Ok(report)
                    if matches!(
                        report.resume_status,
                        codex1_core::ResumeStatus::ActionableNonTerminal
                            | codex1_core::ResumeStatus::InterruptedCycle
                            | codex1_core::ResumeStatus::ContradictoryState
                    ) =>
                {
                    StopHookOutput {
                        continue_processing: true,
                        decision: Some("block".to_string()),
                        reason: Some(report.next_action),
                        system_message: None,
                    }
                }
                Ok(report)
                    if report.resume_status == codex1_core::ResumeStatus::WaitingNeedsUser
                        && state.request_emitted_at.is_none() =>
                {
                    let paths = MissionPaths::new(&repo_root, mission_id.clone());
                    let refreshed_state =
                        load_state(&paths.state_json())?.unwrap_or_else(|| state.clone());
                    if let Some(waiting_request_id) = refreshed_state.waiting_request_id.clone()
                        && latest_closeout_supports_waiting_ack(&paths, &waiting_request_id)?
                        && let Err(error) = acknowledge_waiting_request(
                            &paths,
                            &WaitingRequestAcknowledgementInput { waiting_request_id },
                        )
                    {
                        StopHookOutput {
                            continue_processing: true,
                            decision: Some("block".to_string()),
                            reason: Some(format!(
                                "Repair waiting emission acknowledgement before continuing: {}",
                                error
                            )),
                            system_message: None,
                        }
                    } else {
                        StopHookOutput {
                            continue_processing: true,
                            decision: None,
                            reason: None,
                            system_message: Some(report.next_action),
                        }
                    }
                }
                Ok(report) => {
                    let paths = MissionPaths::new(&repo_root, mission_id.clone());
                    let refreshed_state =
                        load_state(&paths.state_json())?.unwrap_or_else(|| state.clone());
                    if report.resume_status == codex1_core::ResumeStatus::Terminal
                        && !latest_closeout_is_terminal(&paths)?
                    {
                        block_stop_output(
                            "Repair terminal mission state before continuing: latest closeout is not terminal.",
                        )
                    } else {
                        let decision = determine_stop_decision(&refreshed_state);
                        let mut output = StopHookOutput::from_state(&decision, &refreshed_state);
                        if report.resume_status == codex1_core::ResumeStatus::Terminal {
                            output.reason = None;
                        }
                        output
                    }
                }
                Err(error) => StopHookOutput {
                    continue_processing: true,
                    decision: Some("block".to_string()),
                    reason: Some(format!("Repair resume state before continuing: {}", error)),
                    system_message: None,
                },
            }
        }
        _ => {
            let candidates: Vec<String> = active
                .into_iter()
                .map(|(mission_id, _)| mission_id)
                .collect();
            let canonical_selection_request = "Select the mission to resume.".to_string();
            let selection_state = open_selection_wait(
                &ralph_root,
                &SelectionStateInput {
                    candidate_mission_ids: candidates,
                    canonical_selection_request,
                },
            )?;
            let emitted_selection_state = acknowledge_selection_request(
                &ralph_root,
                &SelectionAcknowledgementInput {
                    selection_request_id: selection_state.selection_request_id.clone(),
                },
            )?;
            StopHookOutput::for_selection_wait(&emitted_selection_state)
        }
    };

    println!(
        "{}",
        serde_json::to_string(&output).context("failed to serialize stop-hook output")?
    );
    Ok(())
}

fn println_and_return_output(output: StopHookOutput) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string(&output).context("failed to serialize stop-hook output")?
    );
    Ok(())
}

fn block_stop_output(reason: impl Into<String>) -> StopHookOutput {
    StopHookOutput {
        continue_processing: true,
        decision: Some("block".to_string()),
        reason: Some(reason.into()),
        system_message: None,
    }
}

fn latest_closeout_supports_waiting_ack(
    paths: &MissionPaths,
    waiting_request_id: &str,
) -> Result<bool> {
    let latest = load_closeouts(&paths.closeouts_ndjson())?
        .into_iter()
        .last();
    Ok(latest.is_some_and(|closeout| {
        closeout.verdict == Verdict::NeedsUser
            && closeout.waiting_request_id.as_deref() == Some(waiting_request_id)
    }))
}

fn latest_closeout_is_terminal(paths: &MissionPaths) -> Result<bool> {
    Ok(load_closeouts(&paths.closeouts_ndjson())?
        .into_iter()
        .last()
        .is_some_and(|closeout| {
            matches!(closeout.verdict, Verdict::Complete | Verdict::HardBlocked)
        }))
}

fn load_selection_state(path: &Path) -> Result<Option<SelectionState>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .with_context(|| format!("failed to parse {}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn run_rebuild_state(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::new(&repo_root, mission_id.to_string());
    let state = rebuild_state_from_files(&mission_paths.hidden_mission_root())?;
    emit(
        json_output,
        &json!({
            "repo_root": repo_root,
            "mission_id": mission_id,
            "state": state,
        }),
    )
}

fn run_validate_artifacts(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::new(&repo_root, mission_id.to_string());
    let report = ArtifactValidationReport::run(&mission_paths)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize artifact validation")?,
    )
}

fn run_effective_config(repo_root: Option<PathBuf>, json_output: bool) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let report = inspect_effective_config(&repo_root)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize config report")?,
    )
}

fn run_init_mission(repo_root: Option<PathBuf>, input_json: &str, json_output: bool) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mut input: MissionInitInput = load_input_json(input_json)?;
    let mission_id = input
        .mission_id
        .clone()
        .unwrap_or_else(|| derive_mission_id(&input.title));
    input.mission_id = Some(mission_id.clone());
    let paths = MissionPaths::new(&repo_root, mission_id);
    let report: MissionBootstrapReport = initialize_mission(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize mission bootstrap report")?,
    )
}

fn run_write_blueprint(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: PlanningWriteInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, input.mission_id.clone());
    let report = write_planning_artifacts(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize planning report")?,
    )
}

fn run_compile_execution_package(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: ExecutionPackageInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, input.mission_id.clone());
    let package = compile_execution_package(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(package).context("failed to serialize execution package")?,
    )
}

fn run_validate_execution_package(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    package_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let paths = MissionPaths::new(&repo_root, mission_id.to_string());
    let report: PackageValidationReport = validate_execution_package(&paths, package_id)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize package validation")?,
    )
}

fn run_compile_review_bundle(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: ReviewBundleInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, input.mission_id.clone());
    let bundle = compile_review_bundle(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(bundle).context("failed to serialize review bundle")?,
    )
}

fn run_derive_writer_packet(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: WriterPacketInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, input.mission_id.clone());
    let packet = derive_writer_packet(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(packet).context("failed to serialize writer packet")?,
    )
}

fn run_validate_writer_packet(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    packet_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let paths = MissionPaths::new(&repo_root, mission_id.to_string());
    let report: WriterPacketValidationReport = validate_writer_packet(&paths, packet_id)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize writer packet validation")?,
    )
}

fn run_validate_review_bundle(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    bundle_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let paths = MissionPaths::new(&repo_root, mission_id.to_string());
    let report: ReviewBundleValidationReport = validate_review_bundle(&paths, bundle_id)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize review bundle validation")?,
    )
}

fn run_record_review_result(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: ReviewResultInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, input.mission_id.clone());
    let report = record_review_result(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize review result")?,
    )
}

fn run_record_contradiction(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: ContradictionInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, input.mission_id.clone());
    let record = append_contradiction(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(record).context("failed to serialize contradiction")?,
    )
}

fn run_write_replan_log(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: ReplanLogInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, input.mission_id.clone());
    let report = append_replan_log(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize replan log report")?,
    )
}

fn run_open_selection_wait(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: SelectionStateInput = load_input_json(input_json)?;
    let state = open_selection_wait(&repo_root.join(".ralph"), &input)?;
    emit(
        json_output,
        &serde_json::to_value(state).context("failed to serialize selection state")?,
    )
}

fn run_resolve_resume(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input = if input_json == "-" {
        let mut stdin = String::new();
        io::stdin()
            .read_to_string(&mut stdin)
            .context("failed to read stdin")?;
        if stdin.trim().is_empty() {
            ResolveResumeInput {
                mission_id: None,
                live_child_lanes: Vec::new(),
            }
        } else {
            serde_json::from_str(&stdin).context("failed to parse stdin JSON")?
        }
    } else {
        load_input_json(input_json)?
    };
    let report: ResolveResumeReport = resolve_resume(&repo_root, &input)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize resume resolution")?,
    )
}

fn run_resolve_selection_wait(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: SelectionResolutionInput = load_input_json(input_json)?;
    let state = resolve_selection_wait(&repo_root.join(".ralph"), &input)?;
    emit(
        json_output,
        &serde_json::to_value(state).context("failed to serialize selection state")?,
    )
}

fn run_consume_selection(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    selection_request_id: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let path = repo_root.join(".ralph/selection-state.json");
    let state = load_selection_state(&path)?
        .context("cannot consume selection because .ralph/selection-state.json is missing")?;
    let selected = state
        .selected_mission_id
        .as_deref()
        .context("cannot consume selection before a mission has been selected")?;
    if selected != mission_id {
        anyhow::bail!(
            "selection state is bound to mission {}, not {}",
            selected,
            mission_id
        );
    }
    let report = consume_selection_wait(
        &repo_root.join(".ralph"),
        &SelectionConsumptionInput {
            selection_request_id: selection_request_id
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| state.selection_request_id.clone()),
        },
    )?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize selection consume report")?,
    )
}

fn run_acknowledge_waiting_request(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: WaitingRequestAcknowledgementInput = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, mission_id.to_string());
    let closeout = acknowledge_waiting_request(&paths, &input)?;
    emit(
        json_output,
        &serde_json::to_value(closeout).context("failed to serialize waiting acknowledgement")?,
    )
}

fn run_acknowledge_selection_request(
    repo_root: Option<PathBuf>,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: SelectionAcknowledgementInput = load_input_json(input_json)?;
    let state = acknowledge_selection_request(&repo_root.join(".ralph"), &input)?;
    emit(
        json_output,
        &serde_json::to_value(state).context("failed to serialize selection acknowledgement")?,
    )
}

fn run_write_closeout(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    input_json: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let input: CloseoutRecord = load_input_json(input_json)?;
    let paths = MissionPaths::new(&repo_root, mission_id.to_string());
    let closeout = write_closeout(&paths, input)?;
    emit(
        json_output,
        &serde_json::to_value(closeout).context("failed to serialize closeout")?,
    )
}

fn emit(json_output: bool, value: &serde_json::Value) -> Result<()> {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(value).context("failed to encode JSON")?
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(value).context("failed to encode JSON")?
        );
    }
    Ok(())
}

fn resolve_repo_root(repo_root: Option<PathBuf>) -> Result<PathBuf> {
    let candidate = match repo_root {
        Some(path) => path,
        None => env::current_dir().context("failed to resolve current working directory")?,
    };
    fs::canonicalize(&candidate)
        .with_context(|| format!("failed to canonicalize {}", candidate.display()))
}

fn read_stdin_json<T>() -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut stdin = String::new();
    io::stdin()
        .read_to_string(&mut stdin)
        .context("failed to read stdin")?;
    serde_json::from_str(&stdin).context("failed to parse stdin JSON")
}

fn load_input_json<T>(path_or_dash: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    if path_or_dash == "-" {
        return read_stdin_json();
    }
    let raw = fs::read_to_string(path_or_dash)
        .with_context(|| format!("failed to read {}", path_or_dash))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {} as JSON", path_or_dash))
}

fn derive_mission_id(title: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    let base = if slug.is_empty() { "mission" } else { slug };
    format!("{}-{}", base, &Uuid::new_v4().simple().to_string()[..8])
}

#[derive(Debug, Serialize)]
struct ArtifactValidationReport {
    repo_root: PathBuf,
    mission_id: String,
    success: bool,
    findings: Vec<ArtifactFinding>,
}

#[derive(Debug, Serialize)]
struct ArtifactFinding {
    level: &'static str,
    path: PathBuf,
    message: String,
}

impl ArtifactValidationReport {
    fn run(paths: &MissionPaths) -> Result<Self> {
        let mut findings = Vec::new();
        validate_markdown::<MissionStateFrontmatter>(&paths.mission_state(), &mut findings);
        validate_markdown::<OutcomeLockFrontmatter>(&paths.outcome_lock(), &mut findings);
        validate_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint(), &mut findings);

        let specs_root = paths.specs_root();
        if specs_root.is_dir() {
            for entry in fs::read_dir(&specs_root)
                .with_context(|| format!("failed to read {}", specs_root.display()))?
            {
                let entry = entry.context("failed to read spec directory entry")?;
                let spec_file = entry.path().join("SPEC.md");
                validate_markdown::<WorkstreamSpecFrontmatter>(&spec_file, &mut findings);
            }
        }

        match validate_execution_graph(paths) {
            Ok(report) => append_execution_graph_findings(paths, &report, &mut findings),
            Err(error) => findings.push(ArtifactFinding {
                level: "error",
                path: paths.execution_graph(),
                message: format!("failed to validate execution graph: {error}"),
            }),
        }

        Ok(Self {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            success: findings.iter().all(|finding| finding.level != "error"),
            findings,
        })
    }
}

fn append_execution_graph_findings(
    paths: &MissionPaths,
    report: &ExecutionGraphValidationReport,
    findings: &mut Vec<ArtifactFinding>,
) {
    let level = if report.valid { "info" } else { "error" };
    for finding in &report.findings {
        findings.push(ArtifactFinding {
            level,
            path: paths.execution_graph(),
            message: finding.clone(),
        });
    }
}

fn validate_markdown<F>(path: &Path, findings: &mut Vec<ArtifactFinding>)
where
    F: codex1_core::TypedArtifactFrontmatter,
{
    match fs::read_to_string(path) {
        Ok(contents) => {
            if let Err(error) = ArtifactDocument::<F>::parse(&contents) {
                findings.push(ArtifactFinding {
                    level: "error",
                    path: path.to_path_buf(),
                    message: format!("failed to parse artifact: {error}"),
                });
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            findings.push(ArtifactFinding {
                level: "warn",
                path: path.to_path_buf(),
                message: "artifact is missing".to_string(),
            });
        }
        Err(error) => {
            findings.push(ArtifactFinding {
                level: "error",
                path: path.to_path_buf(),
                message: format!("failed to read artifact: {error}"),
            });
        }
    }
}

fn inspect_effective_config(repo_root: &Path) -> Result<EffectiveConfigReport> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    let user_config_path = home.join(".codex/config.toml");
    let project_config_path = repo_root.join(".codex/config.toml");
    let user = read_optional_toml(&user_config_path)?;
    let project = read_optional_toml(&project_config_path)?;

    let entries = vec![
        inspect_entry(&user, &project, "features.codex_hooks", json!(true)),
        inspect_entry(&user, &project, "agents.max_threads", json!(16)),
        inspect_entry(&user, &project, "agents.max_depth", json!(1)),
        inspect_entry(
            &user,
            &project,
            "codex1_orchestration.model",
            json!("gpt-5.4"),
        ),
        inspect_entry(
            &user,
            &project,
            "codex1_orchestration.reasoning_effort",
            json!("high"),
        ),
        inspect_entry(
            &user,
            &project,
            "codex1_review.model",
            json!("gpt-5.4-mini"),
        ),
        inspect_entry(
            &user,
            &project,
            "codex1_review.reasoning_effort",
            json!("high"),
        ),
        inspect_entry(
            &user,
            &project,
            "codex1_fast_parallel.model",
            json!("gpt-5.3-codex-spark"),
        ),
        inspect_entry(
            &user,
            &project,
            "codex1_fast_parallel.reasoning_effort",
            json!("high"),
        ),
        inspect_entry(
            &user,
            &project,
            "codex1_hard_coding.model",
            json!("gpt-5.3-codex"),
        ),
        inspect_entry(
            &user,
            &project,
            "codex1_hard_coding.reasoning_effort",
            json!("xhigh"),
        ),
    ];

    Ok(EffectiveConfigReport {
        generated_at: Some(time::OffsetDateTime::now_utc()),
        repo_root: Some(repo_root.to_path_buf()),
        entries,
    })
}

fn read_optional_toml(path: &Path) -> Result<Option<toml::Value>> {
    if !path.is_file() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let parsed = raw
        .parse::<toml::Value>()
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(parsed))
}

fn inspect_entry(
    user: &Option<toml::Value>,
    project: &Option<toml::Value>,
    key: &str,
    required_value: serde_json::Value,
) -> codex1_core::EffectiveConfigEntry {
    let project_value = lookup_toml(project.as_ref(), key).map(toml_to_json);
    let user_value = lookup_toml(user.as_ref(), key).map(toml_to_json);
    let (effective_value, source_layer) = if let Some(value) = project_value.clone() {
        (value, codex1_core::ConfigLayer::Project)
    } else if let Some(value) = user_value.clone() {
        (value, codex1_core::ConfigLayer::User)
    } else {
        (serde_json::Value::Null, codex1_core::ConfigLayer::Default)
    };

    codex1_core::EffectiveConfigEntry {
        key: key.to_string(),
        required_value: required_value.clone(),
        effective_value: effective_value.clone(),
        source_layer,
        status: if effective_value == required_value {
            codex1_core::CheckStatus::Pass
        } else {
            codex1_core::CheckStatus::Fail
        },
        remediation: Some(format!("set {key} in the trusted project config")),
    }
}

fn lookup_toml<'a>(value: Option<&'a toml::Value>, key: &str) -> Option<&'a toml::Value> {
    let mut current = value?;
    for segment in key.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(text) => json!(text),
        toml::Value::Integer(number) => json!(number),
        toml::Value::Float(number) => json!(number),
        toml::Value::Boolean(value) => json!(value),
        toml::Value::Datetime(value) => json!(value.to_string()),
        toml::Value::Array(values) => json!(values.iter().map(toml_to_json).collect::<Vec<_>>()),
        toml::Value::Table(table) => {
            let object = table
                .iter()
                .map(|(key, value)| (key.clone(), toml_to_json(value)))
                .collect();
            serde_json::Value::Object(object)
        }
    }
}
