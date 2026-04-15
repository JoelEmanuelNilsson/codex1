use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use codex1_core::{
    ActiveCycleState, ArtifactDocument, CloseoutRecord, ContradictionInput, EffectiveConfigReport,
    ExecutionGraphValidationReport, ExecutionPackageInput, MissionBootstrapReport,
    MissionGateIndex, MissionInitInput, MissionPaths, MissionStateFrontmatter,
    OutcomeLockFrontmatter, PackageValidationReport, PlanningWriteInput,
    ProgramBlueprintFrontmatter, ReplanLogInput, ResolveResumeInput, ResolveResumeReport,
    ReviewBundleInput, ReviewBundleValidationReport, ReviewResultInput,
    SelectionAcknowledgementInput, SelectionConsumptionInput, SelectionResolutionInput,
    SelectionState, SelectionStateInput, WaitingRequestAcknowledgementInput,
    WorkstreamSpecFrontmatter, WriterPacketInput, WriterPacketValidationReport,
    acknowledge_selection_request, acknowledge_waiting_request, append_contradiction,
    append_replan_log, compile_execution_package, compile_review_bundle, consume_selection_wait,
    derive_writer_packet, initialize_mission, load_closeouts, load_state, open_selection_wait,
    rebuild_state_from_files, record_review_result, resolve_resume, resolve_selection_wait,
    resolve_stop_hook_output, validate_execution_graph, validate_execution_package,
    validate_review_bundle, validate_writer_packet, write_closeout, write_planning_artifacts,
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
    ValidateVisibleArtifacts {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ValidateMachineState {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ValidateGates {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    ValidateCloseouts {
        #[arg(long, value_name = "PATH")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        mission_id: String,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    LatestValidCloseout {
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
        InternalCommand::ValidateVisibleArtifacts {
            repo_root,
            mission_id,
            json,
        } => run_validate_visible_artifacts(repo_root, &mission_id, json),
        InternalCommand::ValidateMachineState {
            repo_root,
            mission_id,
            json,
        } => run_validate_machine_state(repo_root, &mission_id, json),
        InternalCommand::ValidateGates {
            repo_root,
            mission_id,
            json,
        } => run_validate_gates(repo_root, &mission_id, json),
        InternalCommand::ValidateCloseouts {
            repo_root,
            mission_id,
            json,
        } => run_validate_closeouts(repo_root, &mission_id, json),
        InternalCommand::LatestValidCloseout {
            repo_root,
            mission_id,
            json,
        } => run_latest_valid_closeout(repo_root, &mission_id, json),
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

#[derive(Debug, Serialize)]
struct GatesValidationReport {
    repo_root: PathBuf,
    mission_id: String,
    gates_path: PathBuf,
    exists: bool,
    valid: bool,
    current_phase: Option<String>,
    gate_count: usize,
    findings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CloseoutsValidationReport {
    repo_root: PathBuf,
    mission_id: String,
    closeouts_path: PathBuf,
    exists: bool,
    valid: bool,
    closeout_count: usize,
    latest_closeout_seq: Option<u64>,
    latest_closeout_id: Option<String>,
    findings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LatestValidCloseoutReport {
    repo_root: PathBuf,
    mission_id: String,
    closeouts_path: PathBuf,
    closeout_count: usize,
    latest_closeout: Option<CloseoutRecord>,
}

#[derive(Debug, Serialize)]
struct VisibleArtifactsValidationReport {
    repo_root: PathBuf,
    mission_id: String,
    success: bool,
    findings: Vec<ArtifactFinding>,
}

#[derive(Debug, Serialize)]
struct MachineStateValidationReport {
    repo_root: PathBuf,
    mission_id: String,
    success: bool,
    findings: Vec<ArtifactFinding>,
}

fn run_stop_hook() -> Result<()> {
    let input: StopHookInput = read_stdin_json()?;
    let repo_root = PathBuf::from(input.cwd);
    let output = resolve_stop_hook_output(&repo_root, &[])?;
    println!(
        "{}",
        serde_json::to_string(&output).context("failed to serialize stop-hook output")?
    );
    Ok(())
}

fn run_rebuild_state(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
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
    let mission_paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
    let report = ArtifactValidationReport::run(&mission_paths)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize artifact validation")?,
    )
}

fn run_validate_visible_artifacts(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
    let report = VisibleArtifactsValidationReport::run(&mission_paths)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize visible artifact validation")?,
    )
}

fn run_validate_machine_state(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
    let report = MachineStateValidationReport::run(&mission_paths)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize machine state validation")?,
    )
}

fn run_validate_gates(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
    let report = evaluate_gates_validation(&mission_paths)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize gates validation")?,
    )
}

fn run_validate_closeouts(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
    let report = evaluate_closeouts_validation(&mission_paths)?;
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize closeouts validation")?,
    )
}

fn run_latest_valid_closeout(
    repo_root: Option<PathBuf>,
    mission_id: &str,
    json_output: bool,
) -> Result<()> {
    let repo_root = resolve_repo_root(repo_root)?;
    let mission_paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
    let closeouts_path = mission_paths.closeouts_ndjson();
    let closeouts = load_closeouts(&closeouts_path)?;
    let report = LatestValidCloseoutReport {
        repo_root,
        mission_id: mission_id.to_string(),
        closeouts_path,
        closeout_count: closeouts.len(),
        latest_closeout: closeouts.into_iter().last(),
    };
    emit(
        json_output,
        &serde_json::to_value(report).context("failed to serialize latest closeout report")?,
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
    let paths = MissionPaths::try_new(&repo_root, mission_id)?;
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
    let paths = MissionPaths::try_new(&repo_root, input.mission_id.clone())?;
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
    let paths = MissionPaths::try_new(&repo_root, input.mission_id.clone())?;
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
    let paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
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
    let paths = MissionPaths::try_new(&repo_root, input.mission_id.clone())?;
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
    let paths = MissionPaths::try_new(&repo_root, input.mission_id.clone())?;
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
    let paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
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
    let paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
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
    let paths = MissionPaths::try_new(&repo_root, input.mission_id.clone())?;
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
    let paths = MissionPaths::try_new(&repo_root, input.mission_id.clone())?;
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
    let paths = MissionPaths::try_new(&repo_root, input.mission_id.clone())?;
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
    let paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
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
    let paths = MissionPaths::try_new(&repo_root, mission_id.to_string())?;
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

fn load_selection_state(path: &Path) -> Result<Option<SelectionState>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .with_context(|| format!("failed to parse {}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
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
    visible_artifacts: VisibleArtifactsValidationReport,
    machine_state: MachineStateValidationReport,
    findings: Vec<ArtifactFinding>,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactFinding {
    level: &'static str,
    path: PathBuf,
    message: String,
}

impl ArtifactValidationReport {
    fn run(paths: &MissionPaths) -> Result<Self> {
        let visible_artifacts = VisibleArtifactsValidationReport::run(paths)?;
        let machine_state = MachineStateValidationReport::run(paths)?;
        let mut findings = visible_artifacts.findings.clone();
        findings.extend(machine_state.findings.clone());

        Ok(Self {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            success: visible_artifacts.success && machine_state.success,
            visible_artifacts,
            machine_state,
            findings,
        })
    }
}

impl VisibleArtifactsValidationReport {
    fn run(paths: &MissionPaths) -> Result<Self> {
        let mut findings = Vec::new();
        validate_text_artifact(
            &paths.readme(),
            &mut findings,
            &[
                "## Snapshot",
                "Mission id:",
                "Current phase:",
                "Current verdict:",
                "Next recommended action:",
                "## Start Here",
                "## Active Frontier",
            ],
        );
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

        if review_history_required(paths)? {
            validate_text_artifact(&paths.review_ledger(), &mut findings, &["# ", "##"]);
        }
        if replan_history_required(paths)? {
            validate_text_artifact(&paths.replan_log(), &mut findings, &["# ", "##"]);
        }

        Ok(Self {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            success: findings.iter().all(|finding| finding.level != "error"),
            findings,
        })
    }
}

impl MachineStateValidationReport {
    fn run(paths: &MissionPaths) -> Result<Self> {
        let mut findings = Vec::new();

        match validate_execution_graph(paths) {
            Ok(report) => append_execution_graph_findings(paths, &report, &mut findings),
            Err(error) => findings.push(ArtifactFinding {
                level: "error",
                path: paths.execution_graph(),
                message: format!("failed to validate execution graph: {error}"),
            }),
        }

        append_gates_validation_findings(&evaluate_gates_validation(paths)?, &mut findings);
        append_closeouts_validation_findings(&evaluate_closeouts_validation(paths)?, &mut findings);
        append_state_validation_findings(paths, &mut findings)?;

        Ok(Self {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            success: findings.iter().all(|finding| finding.level != "error"),
            findings,
        })
    }
}

fn evaluate_gates_validation(paths: &MissionPaths) -> Result<GatesValidationReport> {
    let gates_path = paths.gates_json();
    if !gates_path.is_file() {
        return Ok(GatesValidationReport {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            gates_path,
            exists: false,
            valid: false,
            current_phase: None,
            gate_count: 0,
            findings: vec!["missing gates.json".to_string()],
        });
    }

    match fs::read(&gates_path)
        .with_context(|| format!("failed to read {}", gates_path.display()))
        .and_then(|bytes| {
            serde_json::from_slice::<MissionGateIndex>(&bytes)
                .with_context(|| format!("failed to parse {}", gates_path.display()))
        }) {
        Ok(index) => {
            let mut findings = Vec::new();
            if index.mission_id != paths.mission_id() {
                findings.push(format!(
                    "gates.json mission_id {} does not match requested mission {}",
                    index.mission_id,
                    paths.mission_id()
                ));
            }
            if index.current_phase.trim().is_empty() {
                findings.push("gates.json current_phase must not be empty".to_string());
            }
            let mut gate_ids = std::collections::BTreeSet::new();
            for gate in &index.gates {
                if !gate_ids.insert(gate.gate_id.clone()) {
                    findings.push(format!("duplicate gate_id {}", gate.gate_id));
                }
                if gate.target_ref.trim().is_empty() {
                    findings.push(format!("gate {} has empty target_ref", gate.gate_id));
                }
            }
            Ok(GatesValidationReport {
                repo_root: paths.repo_root().clone(),
                mission_id: paths.mission_id().to_string(),
                gates_path,
                exists: true,
                valid: findings.is_empty(),
                current_phase: Some(index.current_phase),
                gate_count: index.gates.len(),
                findings,
            })
        }
        Err(error) => Ok(GatesValidationReport {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            gates_path,
            exists: true,
            valid: false,
            current_phase: None,
            gate_count: 0,
            findings: vec![error.to_string()],
        }),
    }
}

fn evaluate_closeouts_validation(paths: &MissionPaths) -> Result<CloseoutsValidationReport> {
    let closeouts_path = paths.closeouts_ndjson();
    if !closeouts_path.is_file() {
        return Ok(CloseoutsValidationReport {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            closeouts_path,
            exists: false,
            valid: false,
            closeout_count: 0,
            latest_closeout_seq: None,
            latest_closeout_id: None,
            findings: vec!["missing closeouts.ndjson".to_string()],
        });
    }

    match load_closeouts(&closeouts_path) {
        Ok(closeouts) => Ok(CloseoutsValidationReport {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            closeouts_path,
            exists: true,
            valid: true,
            closeout_count: closeouts.len(),
            latest_closeout_seq: closeouts.last().map(|record| record.closeout_seq),
            latest_closeout_id: closeouts
                .last()
                .and_then(|record| record.closeout_id.clone()),
            findings: Vec::new(),
        }),
        Err(error) => Ok(CloseoutsValidationReport {
            repo_root: paths.repo_root().clone(),
            mission_id: paths.mission_id().to_string(),
            closeouts_path,
            exists: true,
            valid: false,
            closeout_count: 0,
            latest_closeout_seq: None,
            latest_closeout_id: None,
            findings: vec![error.to_string()],
        }),
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

fn append_gates_validation_findings(
    report: &GatesValidationReport,
    findings: &mut Vec<ArtifactFinding>,
) {
    let level = if report.valid { "info" } else { "error" };
    for finding in &report.findings {
        findings.push(ArtifactFinding {
            level,
            path: report.gates_path.clone(),
            message: finding.clone(),
        });
    }
}

fn append_closeouts_validation_findings(
    report: &CloseoutsValidationReport,
    findings: &mut Vec<ArtifactFinding>,
) {
    let level = if report.valid { "info" } else { "error" };
    for finding in &report.findings {
        findings.push(ArtifactFinding {
            level,
            path: report.closeouts_path.clone(),
            message: finding.clone(),
        });
    }
}

fn append_state_validation_findings(
    paths: &MissionPaths,
    findings: &mut Vec<ArtifactFinding>,
) -> Result<()> {
    let state_path = paths.state_json();
    let cached_state = match load_state(&state_path) {
        Ok(state) => state,
        Err(error) => {
            findings.push(ArtifactFinding {
                level: "error",
                path: state_path.clone(),
                message: format!("failed to read cached machine state: {error}"),
            });
            None
        }
    };
    let rebuilt_state = match rebuild_state_from_files(&paths.hidden_mission_root()) {
        Ok(state) => state,
        Err(error) => {
            findings.push(ArtifactFinding {
                level: "error",
                path: paths.hidden_mission_root(),
                message: format!(
                    "failed to rebuild machine state from authoritative files: {error}"
                ),
            });
            None
        }
    };

    if cached_state != rebuilt_state {
        findings.push(ArtifactFinding {
            level: "error",
            path: state_path.clone(),
            message:
                "cached state.json does not match authoritative rebuild from hidden mission files"
                    .to_string(),
        });
    }

    match fs::read(&paths.active_cycle()) {
        Ok(bytes) => {
            if let Err(error) = serde_json::from_slice::<ActiveCycleState>(&bytes) {
                findings.push(ArtifactFinding {
                    level: "error",
                    path: paths.active_cycle(),
                    message: format!("failed to parse active-cycle state: {error}"),
                });
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => findings.push(ArtifactFinding {
            level: "error",
            path: paths.active_cycle(),
            message: format!("failed to read active-cycle state: {error}"),
        }),
    }

    Ok(())
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
                level: "error",
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

fn validate_text_artifact(
    path: &Path,
    findings: &mut Vec<ArtifactFinding>,
    required_markers: &[&str],
) {
    match fs::read_to_string(path) {
        Ok(contents) => {
            if contents.trim().is_empty() {
                findings.push(ArtifactFinding {
                    level: "error",
                    path: path.to_path_buf(),
                    message: "artifact is empty".to_string(),
                });
            } else {
                for marker in required_markers {
                    if !contents.contains(marker) {
                        findings.push(ArtifactFinding {
                            level: "error",
                            path: path.to_path_buf(),
                            message: format!(
                                "artifact is missing required content marker `{marker}`"
                            ),
                        });
                    }
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            findings.push(ArtifactFinding {
                level: "error",
                path: path.to_path_buf(),
                message: "artifact is missing".to_string(),
            });
        }
        Err(error) => findings.push(ArtifactFinding {
            level: "error",
            path: path.to_path_buf(),
            message: format!("failed to read artifact: {error}"),
        }),
    }
}

fn review_history_required(paths: &MissionPaths) -> Result<bool> {
    Ok(load_closeouts(&paths.closeouts_ndjson())?
        .into_iter()
        .any(|closeout| closeout.activity == "review_disposition"))
}

fn replan_history_required(paths: &MissionPaths) -> Result<bool> {
    let path = paths.contradictions_ndjson();
    let Ok(contents) = fs::read_to_string(&path) else {
        return Ok(false);
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(layer) = value
            .get("suggested_reopen_layer")
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        if !matches!(layer, "execution_package" | "blueprint" | "mission_lock") {
            continue;
        }
        let status = value.get("status").and_then(|value| value.as_str());
        if matches!(
            status,
            Some("accepted_for_replan" | "resolved" | "dismissed" | "triaged")
        ) {
            return Ok(true);
        }
    }

    Ok(false)
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
