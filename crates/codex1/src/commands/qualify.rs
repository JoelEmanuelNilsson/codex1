use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
};

use anyhow::{Context, Result, bail};
use codex1_core::{
    ArtifactDocument, BundleKind, ExecutionPackage, GateKind, MissionGateIndex, MissionGateStatus,
    MissionPaths, OutcomeLockFrontmatter, ProgramBlueprintFrontmatter, ReviewBundle,
    WorkstreamSpecFrontmatter, WriterPacket,
};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use time::{OffsetDateTime, macros::format_description};
use uuid::Uuid;

use crate::commands::{QualifyArgs, resolve_repo_root};
use crate::support_surface::{
    AgentsCommandStatus, AgentsScaffoldStatus, SkillSurfaceStatus,
    compute_support_surface_signature, detect_toml_bool, extract_managed_agents_block,
    inspect_agents_scaffold_details, inspect_skill_surface, lookup_toml_value,
    summarize_stop_authority, summarize_stop_authority_with_observational, toml_repo_is_trusted,
};

const REPORT_SCHEMA_VERSION: &str = "codex1.qualify.v1";
const REPORTS_DIR: &str = ".codex1/qualification/reports";
const LATEST_REPORT: &str = ".codex1/qualification/latest.json";
const PRD_MARKER: &str = "docs/codex1-prd.md";
const CONFIG_MODEL: &str = "gpt-5.4";
const CONFIG_REVIEW_MODEL: &str = "gpt-5.4-mini";
const CONFIG_REASONING_EFFORT: &str = "high";
const CONFIG_FAST_PARALLEL_MODEL: &str = "gpt-5.3-codex-spark";
const CONFIG_FAST_PARALLEL_REASONING_EFFORT: &str = "high";
const CONFIG_HARD_CODING_MODEL: &str = "gpt-5.3-codex";
const CONFIG_HARD_CODING_REASONING_EFFORT: &str = "xhigh";
const TRUSTED_CODEX_BUILD: &str = "0.120.0";
const INTERNAL_CONTRACT_PARITY_GATE: &str = "manual_internal_contract_parity";
const REVIEW_LOOP_DECISION_GATE: &str = "review_loop_decision_contract";
const REVIEWER_CAPABILITY_BOUNDARY_GATE: &str = "reviewer_capability_boundary";
const DELEGATED_REVIEW_AUTHORITY_GATE: &str = "delegated_review_authority";
const CONTROL_LOOP_BOUNDARY_GATE: &str = "control_loop_boundary";
const QUALIFY_CHILD_TASK_NAME: &str = "qualify_child";

pub fn run(args: QualifyArgs) -> Result<()> {
    let repo_root = resolve_repo_root(args.common.repo_root.as_deref())?;
    let qualified_at = OffsetDateTime::now_utc();
    let qualification_id = Uuid::new_v4().to_string();
    let user_config = read_optional_string(&codex_home()?.join("config.toml"))?;
    let user_hooks = read_optional_string(&codex_home()?.join("hooks.json"))?;
    let project_config = read_optional_string(&repo_root.join(".codex/config.toml"))?;
    let hooks_config = read_optional_string(&repo_root.join(".codex/hooks.json"))?;
    let agents_doc = read_optional_string(&repo_root.join("AGENTS.md"))?;
    let skill_inspection = inspect_skill_surface(&repo_root)?;
    let managed_agents_block = agents_doc.as_deref().and_then(extract_managed_agents_block);
    let support_surface_signature = compute_support_surface_signature(
        project_config.as_deref(),
        hooks_config.as_deref(),
        user_hooks.as_deref(),
        managed_agents_block.as_deref(),
        &skill_inspection.discovery_root,
    )?;

    let (build_gate, codex_build) = build_probe_gate(args.live);
    let mut gates = vec![supported_platform_gate(), build_gate];
    gates.push(trusted_repo_gate(&repo_root, user_config.as_deref()));
    gates.extend(effective_config_gates(
        &repo_root,
        user_config.as_deref(),
        project_config.as_deref(),
    ));
    gates.extend(project_config_gates(&repo_root));
    gates.push(run_isolated_helper_flow()?);
    gates.push(run_helper_force_normalization_flow()?);
    gates.push(run_helper_partial_install_repair_flow()?);
    gates.push(run_helper_drift_detection_flow()?);
    gates.push(run_runtime_backend_flow()?);
    gates.push(run_waiting_stop_hook_flow()?);
    gates.push(run_control_loop_boundary_flow()?);
    gates.push(run_native_stop_hook_live_flow(args.live)?);
    gates.push(run_native_exec_resume_flow(args.live)?);
    gates.push(run_native_multi_agent_resume_flow(args.live)?);
    gates.push(review_loop_decision_contract_gate());
    gates.push(run_reviewer_capability_boundary_flow()?);
    gates.push(run_delegated_review_authority_flow(&repo_root)?);
    gates.push(manual_autopilot_parity_gate()?);
    gates.push(self_hosting_gate(&repo_root, args.self_hosting));

    let summary = QualificationSummary::from_gates(&gates);
    let evidence = evidence_paths(
        &repo_root,
        qualified_at,
        codex_build.as_ref(),
        &qualification_id,
    );

    let mut report = QualificationReport {
        schema_version: REPORT_SCHEMA_VERSION,
        qualification_id,
        repo_root: repo_root.clone(),
        requested: RequestedQualification {
            live: args.live,
            self_hosting: args.self_hosting,
        },
        codex_build: codex_build
            .as_ref()
            .map(|build| build.raw_version.clone())
            .unwrap_or_else(|| {
                if args.live {
                    "unavailable".to_owned()
                } else {
                    "disabled".to_owned()
                }
            }),
        codex_build_details: codex_build,
        qualified_at,
        tested_at: Some(qualified_at),
        support_surface_signature,
        summary,
        gates,
        evidence_root: repo_root.join(".codex1/qualification"),
        evidence,
    };

    persist_gate_evidence(&mut report)?;
    write_report(&report)?;
    emit_report(&report, args.common.json)?;

    if report.summary.failed > 0 {
        bail!(
            "qualification failed with {} failing gate(s); inspect {}",
            report.summary.failed,
            report.evidence.report_path.display()
        );
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct QualificationReport {
    schema_version: &'static str,
    qualification_id: String,
    repo_root: PathBuf,
    requested: RequestedQualification,
    codex_build: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    codex_build_details: Option<CodexBuildInfo>,
    #[serde(with = "time::serde::rfc3339")]
    qualified_at: OffsetDateTime,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    tested_at: Option<OffsetDateTime>,
    support_surface_signature: String,
    summary: QualificationSummary,
    gates: Vec<QualificationGate>,
    evidence_root: PathBuf,
    evidence: EvidencePaths,
}

#[derive(Debug, Serialize)]
struct RequestedQualification {
    live: bool,
    self_hosting: bool,
}

#[derive(Debug, Serialize, Clone)]
struct CodexBuildInfo {
    command: &'static str,
    raw_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct QualificationSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
    passed_all_required_gates: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    qualification_scope: Option<&'static str>,
    supported_build_qualified: bool,
}

impl QualificationSummary {
    fn from_gates(gates: &[QualificationGate]) -> Self {
        let mut passed = 0_usize;
        let mut failed = 0_usize;
        let mut skipped = 0_usize;

        for gate in gates {
            match gate.status {
                GateStatus::Pass => passed += 1,
                GateStatus::Fail => failed += 1,
                GateStatus::Skipped => skipped += 1,
            }
        }

        let native_stop_proven = gates
            .iter()
            .any(|gate| gate.gate == "waiting_stop_hook_flow" && gate.status == GateStatus::Pass);
        let native_resume_proven = gates
            .iter()
            .any(|gate| gate.gate == "native_exec_resume_flow" && gate.status == GateStatus::Pass);
        let native_multi_agent_proven = gates.iter().any(|gate| {
            gate.gate == "native_multi_agent_resume_flow" && gate.status == GateStatus::Pass
        });
        let passed_all_required_gates = gates
            .iter()
            .filter(|gate| gate_is_required_for_prd(gate.gate))
            .all(|gate| gate.status == GateStatus::Pass);

        Self {
            passed,
            failed,
            skipped,
            passed_all_required_gates,
            qualification_scope: if failed == 0
                && native_stop_proven
                && native_resume_proven
                && native_multi_agent_proven
            {
                Some("scoped_supported_subset")
            } else {
                None
            },
            supported_build_qualified: failed == 0 && passed_all_required_gates,
        }
    }
}

fn gate_is_required_for_prd(gate: &str) -> bool {
    gate != "self_hosting_source_repo"
}

fn project_agents_scaffold_gate(agents_path: &Path, raw: Option<&str>) -> QualificationGate {
    let inspection = inspect_agents_scaffold_details(raw);
    let details = Some(json!({
        "path": agents_path.display().to_string(),
        "agents_state": inspection.status,
        "command_status": inspection.command_status,
        "build_command": inspection.build_command,
        "test_command": inspection.test_command,
        "lint_or_format_command": inspection.lint_or_format_command,
    }));

    match inspection.status {
        AgentsScaffoldStatus::Present
            if inspection.command_status == AgentsCommandStatus::Concrete =>
        {
            QualificationGate::pass(
                "project_agents_scaffold_present",
                "Repo support surfaces include the Codex1-managed AGENTS.md scaffold block with concrete repo commands.",
                format!(
                    "AGENTS.md contains the Codex1-managed scaffold block with concrete Build/Test/Lint commands in {}.",
                    agents_path.display()
                ),
                details,
            )
        }
        AgentsScaffoldStatus::Present => QualificationGate::fail(
            "project_agents_scaffold_present",
            "Repo support surfaces include the Codex1-managed AGENTS.md scaffold block with concrete repo commands.",
            "AGENTS.md contains the managed scaffold block, but the Build/Test/Lint command lines are still placeholders or missing.",
            details,
        ),
        AgentsScaffoldStatus::MissingFile => QualificationGate::fail(
            "project_agents_scaffold_present",
            "Repo support surfaces include the Codex1-managed AGENTS.md scaffold block with concrete repo commands.",
            "Missing AGENTS.md.",
            details,
        ),
        AgentsScaffoldStatus::MissingBlock => QualificationGate::fail(
            "project_agents_scaffold_present",
            "Repo support surfaces include the Codex1-managed AGENTS.md scaffold block with concrete repo commands.",
            "AGENTS.md exists, but the Codex1-managed scaffold block is missing.",
            details,
        ),
        AgentsScaffoldStatus::DriftedBlock => QualificationGate::fail(
            "project_agents_scaffold_present",
            "Repo support surfaces include the Codex1-managed AGENTS.md scaffold block with concrete repo commands.",
            "AGENTS.md contains Codex1 markers, but the managed scaffold block has drifted.",
            details,
        ),
        AgentsScaffoldStatus::MalformedMarkers => QualificationGate::fail(
            "project_agents_scaffold_present",
            "Repo support surfaces include the Codex1-managed AGENTS.md scaffold block with concrete repo commands.",
            "AGENTS.md has malformed Codex1 markers.",
            details,
        ),
    }
}

#[derive(Debug, Serialize)]
struct QualificationGate {
    gate: &'static str,
    description: &'static str,
    status: GateStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl QualificationGate {
    fn pass(
        gate: &'static str,
        description: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self {
            gate,
            description,
            status: GateStatus::Pass,
            message: message.into(),
            evidence_path: None,
            skipped_reason: None,
            details,
        }
    }

    fn fail(
        gate: &'static str,
        description: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self {
            gate,
            description,
            status: GateStatus::Fail,
            message: message.into(),
            evidence_path: None,
            skipped_reason: None,
            details,
        }
    }

    fn skipped(
        gate: &'static str,
        description: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        let message = message.into();
        Self {
            gate,
            description,
            status: GateStatus::Skipped,
            skipped_reason: Some(message.clone()),
            message,
            evidence_path: None,
            details,
        }
    }
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GateStatus {
    Pass,
    Fail,
    Skipped,
}

#[derive(Debug, Serialize)]
struct EvidencePaths {
    report_path: PathBuf,
    latest_path: PathBuf,
    gate_evidence_dir: PathBuf,
}

#[derive(Debug, Serialize)]
struct SmokeStep {
    step: &'static str,
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SnapshotEntry {
    Directory,
    File(Vec<u8>),
    Symlink(String),
}

const PARITY_MISSION_ID: &str = "qualify-parity";
const PARITY_SPEC_ID: &str = "parity_core";

#[derive(Debug, Serialize)]
struct ParityFlowOutcome {
    steps: Vec<SmokeStep>,
    summary: Option<ParityArtifactSummary>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ParityArtifactSummary {
    validate_success: bool,
    execution_graph_present: bool,
    visible_artifacts: BTreeMap<String, bool>,
    hidden_artifact_counts: BTreeMap<String, usize>,
    state: ParityStateSummary,
    gate_phase: String,
    gates: Vec<ParityGateSummary>,
    specs: Vec<ParitySpecSummary>,
    packages: Vec<ParityPackageSummary>,
    packets: Vec<ParityPacketSummary>,
    bundles: Vec<ParityBundleSummary>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ParityStateSummary {
    phase: Option<String>,
    activity: Option<String>,
    verdict: Option<String>,
    terminality: Option<String>,
    resume_mode: Option<String>,
    next_phase: Option<String>,
    target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ParityGateSummary {
    gate_kind: String,
    target_ref: String,
    status: String,
    blocking: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ParitySpecSummary {
    spec_id: String,
    spec_revision: u64,
    blueprint_revision: u64,
    artifact_status: String,
    packetization_status: String,
    execution_status: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ParityPackageSummary {
    target_type: String,
    target_id: String,
    status: String,
    included_specs: Vec<String>,
    proof_obligations: Vec<String>,
    review_obligations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ParityPacketSummary {
    target_spec_id: String,
    required_checks: Vec<String>,
    review_lenses: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ParityBundleSummary {
    bundle_kind: String,
    target_spec_id: Option<String>,
    mandatory_review_lenses: Vec<String>,
    proof_rows_under_review: Vec<String>,
    mission_level_proof_rows: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReviewLoopSeverity {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReviewLoopNextBranch {
    Continue,
    Repair,
    Replan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReviewLoopFinding {
    severity: ReviewLoopSeverity,
    root_cause_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReviewLoopDecision {
    next_branch: ReviewLoopNextBranch,
    blocking_findings: usize,
    unique_blocking_root_causes: usize,
    consecutive_non_clean_loops: u8,
}

fn review_loop_decision(
    findings: &[ReviewLoopFinding],
    prior_consecutive_non_clean_loops: u8,
) -> ReviewLoopDecision {
    let blocking_root_causes = findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.severity,
                ReviewLoopSeverity::P0 | ReviewLoopSeverity::P1 | ReviewLoopSeverity::P2
            )
        })
        .map(|finding| finding.root_cause_key.clone())
        .collect::<BTreeSet<_>>();
    let blocking_findings = findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.severity,
                ReviewLoopSeverity::P0 | ReviewLoopSeverity::P1 | ReviewLoopSeverity::P2
            )
        })
        .count();

    if blocking_findings == 0 {
        return ReviewLoopDecision {
            next_branch: ReviewLoopNextBranch::Continue,
            blocking_findings,
            unique_blocking_root_causes: 0,
            consecutive_non_clean_loops: 0,
        };
    }

    let consecutive_non_clean_loops = prior_consecutive_non_clean_loops.saturating_add(1);
    ReviewLoopDecision {
        next_branch: if consecutive_non_clean_loops >= 6 {
            ReviewLoopNextBranch::Replan
        } else {
            ReviewLoopNextBranch::Repair
        },
        blocking_findings,
        unique_blocking_root_causes: blocking_root_causes.len(),
        consecutive_non_clean_loops,
    }
}

fn review_loop_decision_contract_gate() -> QualificationGate {
    const DESCRIPTION: &str =
        "Prove review-loop clean, repair, and six-loop replan branch decisions.";
    let clean = review_loop_decision(
        &[ReviewLoopFinding {
            severity: ReviewLoopSeverity::P3,
            root_cause_key: "docs-polish".to_string(),
        }],
        5,
    );
    let repair = review_loop_decision(
        &[
            ReviewLoopFinding {
                severity: ReviewLoopSeverity::P1,
                root_cause_key: "stale-gate".to_string(),
            },
            ReviewLoopFinding {
                severity: ReviewLoopSeverity::P2,
                root_cause_key: "stale-gate".to_string(),
            },
        ],
        4,
    );
    let replan = review_loop_decision(
        &[ReviewLoopFinding {
            severity: ReviewLoopSeverity::P0,
            root_cause_key: "architecture-contract".to_string(),
        }],
        5,
    );
    let success = clean.next_branch == ReviewLoopNextBranch::Continue
        && clean.consecutive_non_clean_loops == 0
        && repair.next_branch == ReviewLoopNextBranch::Repair
        && repair.blocking_findings == 2
        && repair.unique_blocking_root_causes == 1
        && repair.consecutive_non_clean_loops == 5
        && replan.next_branch == ReviewLoopNextBranch::Replan
        && replan.consecutive_non_clean_loops == 6;
    let details = json!({
        "clean": clean,
        "repair": repair,
        "replan": replan,
    });

    if success {
        QualificationGate::pass(
            REVIEW_LOOP_DECISION_GATE,
            DESCRIPTION,
            "review-loop branch decisions route clean, repair, and capped replan correctly",
            Some(details),
        )
    } else {
        QualificationGate::fail(
            REVIEW_LOOP_DECISION_GATE,
            DESCRIPTION,
            "review-loop branch decision contract failed",
            Some(details),
        )
    }
}

fn run_reviewer_capability_boundary_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create reviewer-boundary sandbox")?;
    let repo_root = sandbox.path().join("repo");
    fs::create_dir_all(&repo_root).context("failed to create reviewer-boundary repo root")?;

    let (mut contaminated_steps, contaminated_bundle_id) =
        open_reviewer_boundary_bundle(&binary, &repo_root, "reviewer-boundary-contaminated")?;
    let contaminated_snapshot = capture_review_truth_snapshot_payload(
        &binary,
        &repo_root,
        "reviewer-boundary-contaminated",
        &contaminated_bundle_id,
    )?;
    let contaminated_reviewer_output_ref = record_none_reviewer_output_ref(
        &binary,
        &repo_root,
        "reviewer-boundary-contaminated",
        &contaminated_bundle_id,
        "qualification-reviewer-boundary-contaminated-spec",
        &mut contaminated_steps,
    )?;
    let contaminated_code_output_ref = record_none_reviewer_output_ref_after_snapshot(
        &binary,
        &repo_root,
        "reviewer-boundary-contaminated",
        &contaminated_bundle_id,
        "qualification-reviewer-boundary-contaminated-code",
        &mut contaminated_steps,
    )?;
    fs::write(
        repo_root.join("PLANS/reviewer-boundary-contaminated/REVIEW-LEDGER.md"),
        "# contaminated by child reviewer\n",
    )
    .context("failed to contaminate review ledger")?;
    let contaminated_record = run_json_smoke_step(
        "contaminated_record_review_outcome",
        &binary,
        &repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &reviewer_boundary_review_result_payload(
            "reviewer-boundary-contaminated",
            &contaminated_bundle_id,
            &[
                contaminated_reviewer_output_ref,
                contaminated_code_output_ref,
            ],
            contaminated_snapshot,
        ),
    )?;
    let contaminated_rejected = !contaminated_record.success
        && format!(
            "{}{}",
            contaminated_record.stdout, contaminated_record.stderr
        )
        .contains("reviewer_lane_truth_mutation_detected");
    contaminated_steps.push(contaminated_record);

    let (mut clean_steps, clean_bundle_id) =
        open_reviewer_boundary_bundle(&binary, &repo_root, "reviewer-boundary-clean")?;
    let clean_snapshot = capture_review_truth_snapshot_payload(
        &binary,
        &repo_root,
        "reviewer-boundary-clean",
        &clean_bundle_id,
    )?;
    clean_steps.push(run_json_smoke_step(
        "capture_review_evidence_snapshot",
        &binary,
        &repo_root,
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--mission-id",
            "reviewer-boundary-clean",
            "--bundle-id",
            &clean_bundle_id,
            "--json",
        ],
        &json!({}),
    )?);
    let evidence_validation = run_json_smoke_step(
        "validate_review_evidence_snapshot",
        &binary,
        &repo_root,
        &[
            "internal",
            "validate-review-evidence-snapshot",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--mission-id",
            "reviewer-boundary-clean",
            "--bundle-id",
            &clean_bundle_id,
            "--json",
        ],
        &json!({}),
    )?;
    let evidence_valid = evidence_validation.success
        && serde_json::from_str::<Value>(&evidence_validation.stdout)
            .ok()
            .and_then(|value| value.get("valid").and_then(Value::as_bool))
            .unwrap_or(false);
    clean_steps.push(evidence_validation);
    let clean_reviewer_output_ref = record_none_reviewer_output_ref(
        &binary,
        &repo_root,
        "reviewer-boundary-clean",
        &clean_bundle_id,
        "qualification-reviewer-boundary-clean-spec",
        &mut clean_steps,
    )?;
    let clean_code_output_ref = record_none_reviewer_output_ref_after_snapshot(
        &binary,
        &repo_root,
        "reviewer-boundary-clean",
        &clean_bundle_id,
        "qualification-reviewer-boundary-clean-code",
        &mut clean_steps,
    )?;
    let clean_record = run_json_smoke_step(
        "clean_record_review_outcome",
        &binary,
        &repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &reviewer_boundary_review_result_payload(
            "reviewer-boundary-clean",
            &clean_bundle_id,
            &[clean_reviewer_output_ref, clean_code_output_ref],
            clean_snapshot,
        ),
    )?;
    let clean_recorded = clean_record.success
        && serde_json::from_str::<Value>(&clean_record.stdout)
            .ok()
            .and_then(|value| value.get("blocking_findings").and_then(Value::as_u64))
            == Some(0);
    clean_steps.push(clean_record);

    let details = Some(json!({
        "sandbox_root": sandbox.path().display().to_string(),
        "contaminated_bundle_id": contaminated_bundle_id,
        "clean_bundle_id": clean_bundle_id,
        "contaminated_rejected": contaminated_rejected,
        "evidence_valid": evidence_valid,
        "clean_recorded": clean_recorded,
        "contaminated_steps": contaminated_steps,
        "clean_steps": clean_steps,
    }));

    if contaminated_rejected && evidence_valid && clean_recorded {
        Ok(QualificationGate::pass(
            REVIEWER_CAPABILITY_BOUNDARY_GATE,
            "Reviewer-lane capability qualification proves child-review mutation cannot silently clear mission truth.",
            "Contaminated review writeback was rejected, the frozen review evidence snapshot validated, and clean parent-owned review writeback passed.",
            details,
        ))
    } else {
        Ok(QualificationGate::fail(
            REVIEWER_CAPABILITY_BOUNDARY_GATE,
            "Reviewer-lane capability qualification proves child-review mutation cannot silently clear mission truth.",
            "Reviewer capability boundary proof did not satisfy contaminated rejection, evidence snapshot validation, and clean parent writeback.",
            details,
        ))
    }
}

fn run_delegated_review_authority_flow(repo_root: &Path) -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create delegated-review sandbox")?;
    let sandbox_repo_root = sandbox.path().join("repo");
    fs::create_dir_all(&sandbox_repo_root)
        .context("failed to create delegated-review repo root")?;

    let (mut steps, bundle_id) =
        open_reviewer_boundary_bundle(&binary, &sandbox_repo_root, "delegated-review-authority")?;
    let review_truth_snapshot = capture_review_truth_snapshot_payload(
        &binary,
        &sandbox_repo_root,
        "delegated-review-authority",
        &bundle_id,
    )?;

    let missing_reviewer_output = run_json_smoke_step(
        "reject_missing_reviewer_output",
        &binary,
        &sandbox_repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            sandbox_repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "delegated-review-authority",
            "bundle_id": bundle_id,
            "reviewer": "qualify-codex",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["RECEIPTS/reviewer-boundary.txt"],
            "findings": [],
            "disposition_notes": ["Parent-only evidence should not qualify."],
            "next_required_branch": "execution",
            "review_truth_snapshot": review_truth_snapshot.clone()
        }),
    )?;
    let missing_reviewer_output_rejected = !missing_reviewer_output.success
        && format!(
            "{}{}",
            missing_reviewer_output.stdout, missing_reviewer_output.stderr
        )
        .contains("requires reviewer-agent output evidence");
    steps.push(missing_reviewer_output);

    let remint_snapshot = run_json_smoke_step(
        "reject_review_truth_snapshot_remint",
        &binary,
        &sandbox_repo_root,
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--repo-root",
            sandbox_repo_root.to_str().unwrap(),
            "--mission-id",
            "delegated-review-authority",
            "--bundle-id",
            &bundle_id,
            "--json",
        ],
        &json!({}),
    )?;
    let remint_rejected = !remint_snapshot.success
        && format!("{}{}", remint_snapshot.stdout, remint_snapshot.stderr)
            .contains("refusing to remint parent writeback authority");
    steps.push(remint_snapshot);

    let spec_reviewer_output_ref = record_none_reviewer_output_ref(
        &binary,
        &sandbox_repo_root,
        "delegated-review-authority",
        &bundle_id,
        "qualification-delegated-review-spec",
        &mut steps,
    )?;
    let code_reviewer_output_ref = record_none_reviewer_output_ref_after_snapshot(
        &binary,
        &sandbox_repo_root,
        "delegated-review-authority",
        &bundle_id,
        "qualification-delegated-review-code",
        &mut steps,
    )?;
    let mut post_output_snapshot = review_truth_snapshot.clone();
    post_output_snapshot["captured_at"] = json!("2099-01-01T00:00:00Z");
    let mut persisted_post_output_snapshot = post_output_snapshot.clone();
    if let Some(object) = persisted_post_output_snapshot.as_object_mut() {
        object.remove("writeback_authority_token");
    }
    let persisted_snapshot_path = sandbox_repo_root.join(format!(
        ".ralph/missions/delegated-review-authority/review-truth-snapshots/{bundle_id}.json"
    ));
    fs::write(
        &persisted_snapshot_path,
        serde_json::to_vec_pretty(&persisted_post_output_snapshot)
            .context("failed to encode future review truth snapshot")?,
    )
    .with_context(|| {
        format!(
            "failed to write future review truth snapshot {}",
            persisted_snapshot_path.display()
        )
    })?;
    let post_output_capture = run_json_smoke_step(
        "reject_post_output_review_truth_snapshot",
        &binary,
        &sandbox_repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            sandbox_repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "delegated-review-authority",
            "bundle_id": bundle_id,
            "reviewer": "qualify-codex",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                spec_reviewer_output_ref,
                code_reviewer_output_ref,
                "RECEIPTS/reviewer-boundary.txt"
            ],
            "findings": [],
            "disposition_notes": ["Reviewer outputs must be downstream of parent truth capture."],
            "next_required_branch": "execution",
            "review_truth_snapshot": post_output_snapshot
        }),
    )?;
    let post_output_capture_rejected = !post_output_capture.success
        && format!(
            "{}{}",
            post_output_capture.stdout, post_output_capture.stderr
        )
        .contains("was recorded before parent review_truth_snapshot capture");
    steps.push(post_output_capture);

    let (mut no_snapshot_steps, no_snapshot_bundle_id) =
        open_reviewer_boundary_bundle(&binary, &sandbox_repo_root, "delegated-review-no-snapshot")?;
    let missing_snapshot = run_json_smoke_step(
        "reject_missing_review_truth_snapshot",
        &binary,
        &sandbox_repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            sandbox_repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "delegated-review-no-snapshot",
            "bundle_id": no_snapshot_bundle_id,
            "reviewer": "qualify-codex",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                "reviewer-output:delegated-review-authority",
                "RECEIPTS/reviewer-boundary.txt"
            ],
            "findings": [],
            "disposition_notes": ["Missing snapshot should not qualify."],
            "next_required_branch": "execution"
        }),
    )?;
    let missing_snapshot_rejected = !missing_snapshot.success
        && format!("{}{}", missing_snapshot.stdout, missing_snapshot.stderr)
            .contains("requires review_truth_snapshot");
    no_snapshot_steps.push(missing_snapshot);
    steps.extend(no_snapshot_steps);

    let (mut parent_authority_steps, parent_authority_bundle_id) = open_reviewer_boundary_bundle(
        &binary,
        &sandbox_repo_root,
        "delegated-review-loop-authority",
    )?;
    let begin_parent_authority = run_json_smoke_step(
        "begin_parent_review_loop_authority",
        &binary,
        &sandbox_repo_root,
        &[
            "internal",
            "begin-loop-lease",
            "--repo-root",
            sandbox_repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "delegated-review-loop-authority",
            "mode": "review_loop",
            "owner": "parent-review-loop",
            "reason": "Qualification should protect parent-only review authority."
        }),
    )?;
    let parent_authority_token = serde_json::from_str::<Value>(&begin_parent_authority.stdout)
        .ok()
        .and_then(|value| {
            value
                .get("parent_authority_token")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let parent_authority_token_present = parent_authority_token.is_some();
    parent_authority_steps.push(begin_parent_authority);

    let capture_without_parent_authority = run_json_smoke_step_without_parent_authority(
        "reject_capture_without_parent_loop_authority",
        &binary,
        &sandbox_repo_root,
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--repo-root",
            sandbox_repo_root.to_str().unwrap(),
            "--mission-id",
            "delegated-review-loop-authority",
            "--bundle-id",
            &parent_authority_bundle_id,
            "--json",
        ],
        &json!({}),
    )?;
    let capture_without_parent_authority_rejected = !capture_without_parent_authority.success
        && format!(
            "{}{}",
            capture_without_parent_authority.stdout, capture_without_parent_authority.stderr
        )
        .contains("requires parent loop authority");
    parent_authority_steps.push(capture_without_parent_authority);

    let capture_with_parent_authority = if let Some(token) = parent_authority_token.as_deref() {
        run_json_smoke_step_with_env(
            "capture_with_parent_loop_authority",
            &binary,
            &sandbox_repo_root,
            &[
                "internal",
                "capture-review-truth-snapshot",
                "--repo-root",
                sandbox_repo_root.to_str().unwrap(),
                "--mission-id",
                "delegated-review-loop-authority",
                "--bundle-id",
                &parent_authority_bundle_id,
                "--json",
            ],
            &json!({}),
            &[("CODEX1_PARENT_LOOP_AUTHORITY_TOKEN", token)],
        )?
    } else {
        SmokeStep {
            step: "capture_with_parent_loop_authority",
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "missing parent authority token".to_string(),
        }
    };
    let capture_with_parent_authority_passed = capture_with_parent_authority.success;
    parent_authority_steps.push(capture_with_parent_authority);
    steps.extend(parent_authority_steps);

    let skill_surface_root = inspect_skill_surface(repo_root)
        .ok()
        .map(|inspection| inspection.discovery_root);
    let review_loop_text =
        read_optional_string(&repo_root.join(".codex/skills/review-loop/SKILL.md"))?
            .or_else(|| {
                skill_surface_root.as_ref().and_then(|root| {
                    read_optional_string(&root.join("review-loop/SKILL.md"))
                        .ok()
                        .flatten()
                })
            })
            .unwrap_or_default();
    let internal_orchestration_text =
        read_optional_string(&repo_root.join(".codex/skills/internal-orchestration/SKILL.md"))?
            .or_else(|| {
                skill_surface_root.as_ref().and_then(|root| {
                    read_optional_string(&root.join("internal-orchestration/SKILL.md"))
                        .ok()
                        .flatten()
                })
            })
            .unwrap_or_default();
    let runtime_backend_text = read_optional_string(&repo_root.join("docs/runtime-backend.md"))?
        .or_else(|| {
            let source_runtime_backend =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/runtime-backend.md");
            read_optional_string(&source_runtime_backend).ok().flatten()
        });
    let normalize_doc = |contents: &str| contents.split_whitespace().collect::<Vec<_>>().join(" ");
    let review_loop_normalized = normalize_doc(&review_loop_text);
    let internal_orchestration_normalized = normalize_doc(&internal_orchestration_text);
    let runtime_backend_requirement = runtime_backend_text.as_deref().is_some_and(|contents| {
        let normalized = contents.split_whitespace().collect::<Vec<_>>().join(" ");
        normalized.contains("Parent-owned review writeback is not parent-owned review judgment")
            && normalized.contains("reviewer-agent output evidence")
            && normalized.contains("Parent-only review judgment cannot clear")
            && normalized.contains("one transient parent writeback token per review bundle")
            && normalized.contains("parent loop authority")
    });
    let docs_requirements = vec![
        (
            "review_loop_forbids_parent_self_review",
            review_loop_normalized.contains("must not substitute its own code, spec, intent")
                && review_loop_normalized.contains("parent-only judgment is not"),
        ),
        (
            "review_loop_requires_reviewer_output",
            review_loop_normalized.contains("reviewer-agent output"),
        ),
        (
            "internal_orchestration_delegates_judgment",
            internal_orchestration_normalized.contains("not review judgment authority")
                && internal_orchestration_normalized.contains("reviewer-agent outputs"),
        ),
        (
            "runtime_backend_forbids_parent_only_gate_result",
            runtime_backend_requirement,
        ),
    ];
    let docs_ok = docs_requirements.iter().all(|(_, passed)| *passed);

    let details = Some(json!({
        "sandbox_root": sandbox.path().display().to_string(),
        "missing_reviewer_output_rejected": missing_reviewer_output_rejected,
        "missing_snapshot_rejected": missing_snapshot_rejected,
        "remint_rejected": remint_rejected,
        "post_output_capture_rejected": post_output_capture_rejected,
        "parent_authority_token_present": parent_authority_token_present,
        "capture_without_parent_authority_rejected": capture_without_parent_authority_rejected,
        "capture_with_parent_authority_passed": capture_with_parent_authority_passed,
        "docs_requirements": docs_requirements
            .iter()
            .map(|(name, passed)| json!({ "check": name, "passed": passed }))
            .collect::<Vec<_>>(),
        "steps": steps,
    }));

    if missing_reviewer_output_rejected
        && missing_snapshot_rejected
        && remint_rejected
        && post_output_capture_rejected
        && parent_authority_token_present
        && capture_without_parent_authority_rejected
        && capture_with_parent_authority_passed
        && docs_ok
    {
        Ok(QualificationGate::pass(
            DELEGATED_REVIEW_AUTHORITY_GATE,
            "Delegated review authority qualification proves parent self-review cannot clear durable review truth.",
            "Docs forbid parent self-review, missing reviewer-agent evidence is rejected, missing review truth snapshots are rejected, parent writeback authority cannot be reminted, and post-output truth capture cannot clear review.",
            details,
        ))
    } else {
        Ok(QualificationGate::fail(
            DELEGATED_REVIEW_AUTHORITY_GATE,
            "Delegated review authority qualification proves parent self-review cannot clear durable review truth.",
            "Delegated review authority proof failed; inspect wording checks and runtime rejection steps.",
            details,
        ))
    }
}

fn open_reviewer_boundary_bundle(
    binary: &Path,
    repo_root: &Path,
    mission_id: &'static str,
) -> Result<(Vec<SmokeStep>, String)> {
    let mut steps = Vec::new();
    steps.push(run_json_smoke_step(
        "init_mission",
        binary,
        repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": mission_id,
            "title": "Reviewer Boundary Qualification",
            "objective": "Prove reviewer-lane capability boundary.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    )?);
    steps.push(run_json_smoke_step(
        "materialize_plan",
        binary,
        repo_root,
        &[
            "internal",
            "materialize-plan",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use one reviewer-boundary runtime slice.", &["runtime_core"]),
            "plan_level": 4,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{
                "claim_ref": "claim:reviewer-boundary",
                "statement": "Reviewer capability boundary is explicit.",
                "required_evidence": ["RECEIPTS/reviewer-boundary.txt"],
                "review_lenses": ["correctness", "evidence_adequacy"],
                "governing_contract_refs": ["blueprint"]
            }],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Create reviewer boundary proof slice.",
                "body_markdown": qualification_spec_body(
                    "Create reviewer boundary proof slice.",
                    &["src"],
                    &["src"],
                    &["cargo test"]
                ),
                "artifact_status": "active",
                "packetization_status": "runnable",
                "execution_status": "packaged"
            }]
        }),
    )?);
    steps.push(run_json_smoke_step(
        "compile_execution_package",
        binary,
        repo_root,
        &[
            "internal",
            "compile-execution-package",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [{
                "name": "lock_current",
                "satisfied": true,
                "detail": "Outcome Lock revision is current."
            }],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["spec review"]
        }),
    )?);
    let package_id = parse_required_id(steps.last().expect("package step"), "package_id")?;
    steps.push(run_json_smoke_step(
        "compile_review_bundle",
        binary,
        repo_root,
        &[
            "internal",
            "compile-review-bundle",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": mission_id,
            "source_package_id": package_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness", "evidence_adequacy"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test"],
            "receipts": ["RECEIPTS/reviewer-boundary.txt"],
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["reviewer boundary"]
        }),
    )?);
    let bundle_id = parse_required_id(steps.last().expect("bundle step"), "bundle_id")?;
    Ok((steps, bundle_id))
}

fn reviewer_boundary_review_result_payload(
    mission_id: &str,
    bundle_id: &str,
    reviewer_output_refs: &[String],
    review_truth_snapshot: Value,
) -> Value {
    let evidence_refs = reviewer_output_refs
        .iter()
        .cloned()
        .chain(std::iter::once(
            "RECEIPTS/reviewer-boundary.txt".to_string(),
        ))
        .collect::<Vec<_>>();
    json!({
        "mission_id": mission_id,
        "bundle_id": bundle_id,
        "reviewer": "qualify-codex",
        "verdict": "clean",
        "target_spec_id": "runtime_core",
        "governing_refs": ["bundle"],
        "evidence_refs": evidence_refs,
        "findings": [],
        "disposition_notes": ["Reviewer boundary review path is clean."],
        "next_required_branch": "execution",
        "review_truth_snapshot": review_truth_snapshot
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParityAutopilotStep {
    WriteBlueprint,
    CompileExecutionPackage,
    DeriveWriterPacket,
    CompileSpecReviewBundle,
    RecordSpecReviewResult,
    CompileMissionCloseBundle,
    RecordMissionCloseReview,
    Done,
}

fn codex_home() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(explicit));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn trusted_repo_gate(repo_root: &Path, user_config: Option<&str>) -> QualificationGate {
    if is_repo_trusted(repo_root, user_config) {
        QualificationGate::pass(
            "trusted_repo",
            "The target repo is trusted by Codex.",
            "Codex will honor project-scoped support-surface configuration for this repo.",
            Some(json!({ "repo_root": repo_root.display().to_string() })),
        )
    } else {
        QualificationGate::fail(
            "trusted_repo",
            "The target repo is trusted by Codex.",
            "Codex will ignore project-scoped support-surface configuration until the repo is trusted.",
            Some(json!({ "repo_root": repo_root.display().to_string() })),
        )
    }
}

fn is_repo_trusted(repo_root: &Path, user_config: Option<&str>) -> bool {
    user_config.is_some_and(|raw| toml_repo_is_trusted(raw, repo_root))
}

fn effective_config_gates(
    repo_root: &Path,
    user_config: Option<&str>,
    project_config: Option<&str>,
) -> Vec<QualificationGate> {
    let trusted_repo = is_repo_trusted(repo_root, user_config);
    [
        (None, "model", CONFIG_MODEL),
        (None, "review_model", CONFIG_REVIEW_MODEL),
        (None, "model_reasoning_effort", CONFIG_REASONING_EFFORT),
        (Some("features"), "codex_hooks", "true"),
        (Some("agents"), "max_threads", "16"),
        (Some("agents"), "max_depth", "1"),
        (Some("codex1_orchestration"), "model", CONFIG_MODEL),
        (
            Some("codex1_orchestration"),
            "reasoning_effort",
            CONFIG_REASONING_EFFORT,
        ),
        (Some("codex1_review"), "model", CONFIG_REVIEW_MODEL),
        (
            Some("codex1_review"),
            "reasoning_effort",
            CONFIG_REASONING_EFFORT,
        ),
        (
            Some("codex1_fast_parallel"),
            "model",
            CONFIG_FAST_PARALLEL_MODEL,
        ),
        (
            Some("codex1_fast_parallel"),
            "reasoning_effort",
            CONFIG_FAST_PARALLEL_REASONING_EFFORT,
        ),
        (Some("codex1_hard_coding"), "model", CONFIG_HARD_CODING_MODEL),
        (
            Some("codex1_hard_coding"),
            "reasoning_effort",
            CONFIG_HARD_CODING_REASONING_EFFORT,
        ),
    ]
    .into_iter()
    .map(|(section, key, required)| {
        let entry = inspect_effective_config_key(
            trusted_repo,
            user_config,
            project_config,
            section,
            key,
            required,
        );
        match entry {
            (true, "project", effective) => QualificationGate::pass(
                "effective_config_baseline",
                "The trusted effective Codex config resolves the required Codex1 support baseline.",
                format!(
                    "{key} resolves to {} from the trusted project config.",
                    effective
                        .clone()
                        .unwrap_or_else(|| "unset".to_string())
                ),
                Some(json!({ "key": key, "effective_value": effective, "source_layer": "project" })),
            ),
            (true, source_layer, effective) => QualificationGate::fail(
                "effective_config_baseline",
                "The trusted effective Codex config resolves the required Codex1 support baseline.",
                format!(
                    "{key} resolves to {} from {source_layer} instead of the trusted project config.",
                    effective
                        .clone()
                        .unwrap_or_else(|| "unset".to_string())
                ),
                Some(json!({ "key": key, "effective_value": effective, "source_layer": source_layer })),
            ),
            (false, source_layer, effective) => QualificationGate::fail(
                "effective_config_baseline",
                "The trusted effective Codex config resolves the required Codex1 support baseline.",
                format!("{key} resolves to {effective:?} from {source_layer} instead of {required}."),
                Some(json!({ "key": key, "effective_value": effective, "source_layer": source_layer, "required": required })),
            ),
        }
    })
    .collect()
}

fn inspect_effective_config_key(
    trusted_repo: bool,
    user_config: Option<&str>,
    project_config: Option<&str>,
    section: Option<&str>,
    key: &str,
    required_value: &str,
) -> (bool, &'static str, Option<String>) {
    let user_value = user_config.and_then(|raw| lookup_toml_value(raw, section, key));
    let project_value = if trusted_repo {
        project_config.and_then(|raw| lookup_toml_value(raw, section, key))
    } else {
        None
    };

    let (effective_value, source_layer) = if let Some(project_value) = project_value {
        (Some(project_value), "project")
    } else if let Some(user_value) = user_value {
        (Some(user_value), "user")
    } else {
        (None, "default")
    };

    (
        effective_value.as_deref() == Some(required_value),
        source_layer,
        effective_value,
    )
}

fn read_optional_string(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn build_probe_gate(live: bool) -> (QualificationGate, Option<CodexBuildInfo>) {
    if !live {
        return (
            QualificationGate::skipped(
                "codex_build_probe",
                "Capture the exact live Codex CLI build under test.",
                "Live Codex version probing was disabled for this run.",
                None,
            ),
            None,
        );
    }

    match probe_codex_build() {
        Ok(build) => {
            let trusted_build = build
                .normalized_version
                .as_deref()
                .is_some_and(|version| version == TRUSTED_CODEX_BUILD);
            (
                if trusted_build {
                    QualificationGate::pass(
                        "codex_build_probe",
                        "Capture the exact live Codex CLI build under test.",
                        format!("Detected trusted Codex build {}.", build.raw_version),
                        Some(json!({
                            "command": build.command,
                            "normalized_version": build.normalized_version,
                            "trusted_build": TRUSTED_CODEX_BUILD,
                        })),
                    )
                } else {
                    QualificationGate::fail(
                        "codex_build_probe",
                        "Capture the exact live Codex CLI build under test.",
                        format!(
                            "Detected {}, but the trusted Codex build baseline is {}.",
                            build.raw_version, TRUSTED_CODEX_BUILD
                        ),
                        Some(json!({
                            "command": build.command,
                            "normalized_version": build.normalized_version,
                            "trusted_build": TRUSTED_CODEX_BUILD,
                        })),
                    )
                },
                Some(build),
            )
        }
        Err(error) => (
            QualificationGate::fail(
                "codex_build_probe",
                "Capture the exact live Codex CLI build under test.",
                "Unable to capture the live Codex version.",
                Some(json!({
                    "error": error.to_string(),
                })),
            ),
            None,
        ),
    }
}

fn supported_platform_gate() -> QualificationGate {
    let platform = std::env::consts::OS;
    if platform == "macos" {
        QualificationGate::pass(
            "supported_platform",
            "Qualification is running on the PRD-supported macOS environment.",
            "Detected macOS.",
            Some(json!({ "platform": platform })),
        )
    } else {
        QualificationGate::fail(
            "supported_platform",
            "Qualification is running on the PRD-supported macOS environment.",
            format!("Detected unsupported platform {platform}."),
            Some(json!({ "platform": platform })),
        )
    }
}

fn probe_codex_build() -> Result<CodexBuildInfo> {
    let output = Command::new("codex")
        .arg("--version")
        .output()
        .context("failed to execute `codex --version`")?;

    if !output.status.success() {
        bail!(
            "`codex --version` exited with {}",
            output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |code| code.to_string())
        );
    }

    let raw_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let normalized_version = raw_version
        .split_whitespace()
        .find(|token| token.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        .map(ToOwned::to_owned);

    Ok(CodexBuildInfo {
        command: "codex --version",
        raw_version,
        normalized_version,
    })
}

fn project_config_gates(repo_root: &Path) -> Vec<QualificationGate> {
    let config_path = repo_root.join(".codex/config.toml");
    let hooks_path = repo_root.join(".codex/hooks.json");
    let agents_path = repo_root.join("AGENTS.md");
    let user_hooks_path = codex_home()
        .map(|home| home.join("hooks.json"))
        .unwrap_or_else(|_| PathBuf::from("~/.codex/hooks.json"));

    let mut gates = Vec::new();

    let config_contents = match fs::read_to_string(&config_path) {
        Ok(contents) => {
            gates.push(QualificationGate::pass(
                "project_config_present",
                "Project-scoped `.codex/config.toml` exists for harness-required overrides.",
                format!("Found {}", config_path.display()),
                Some(json!({ "path": config_path.display().to_string() })),
            ));
            Some(contents)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            gates.push(QualificationGate::fail(
                "project_config_present",
                "Project-scoped `.codex/config.toml` exists for harness-required overrides.",
                "Missing project `.codex/config.toml`.",
                Some(json!({ "path": config_path.display().to_string() })),
            ));
            None
        }
        Err(error) => {
            gates.push(QualificationGate::fail(
                "project_config_present",
                "Project-scoped `.codex/config.toml` exists for harness-required overrides.",
                "Could not read project `.codex/config.toml`.",
                Some(json!({
                    "path": config_path.display().to_string(),
                    "error": error.to_string(),
                })),
            ));
            None
        }
    };

    match config_contents.as_deref() {
        Some(contents) => match detect_codex_hooks_setting(contents) {
            Some(true) => gates.push(QualificationGate::pass(
                "project_codex_hooks_enabled",
                "Project config enables `features.codex_hooks = true`.",
                "Project config enables Codex hooks.",
                Some(json!({ "path": config_path.display().to_string() })),
            )),
            Some(false) => gates.push(QualificationGate::fail(
                "project_codex_hooks_enabled",
                "Project config enables `features.codex_hooks = true`.",
                "Project config explicitly disables Codex hooks.",
                Some(json!({ "path": config_path.display().to_string() })),
            )),
            None => gates.push(QualificationGate::fail(
                "project_codex_hooks_enabled",
                "Project config enables `features.codex_hooks = true`.",
                "Project config does not declare `features.codex_hooks = true`.",
                Some(json!({ "path": config_path.display().to_string() })),
            )),
        },
        None => gates.push(QualificationGate::skipped(
            "project_codex_hooks_enabled",
            "Project config enables `features.codex_hooks = true`.",
            "Skipped because the project config file is missing or unreadable.",
            Some(json!({ "path": config_path.display().to_string() })),
        )),
    }

    let project_stop_hook_authority = match fs::read_to_string(&hooks_path) {
        Ok(contents) => match serde_json::from_str::<Value>(&contents) {
            Ok(parsed) => {
                gates.push(QualificationGate::pass(
                    "project_hooks_file_present",
                    "Project-scoped `.codex/hooks.json` exists.",
                    format!("Found {}", hooks_path.display()),
                    Some(json!({ "path": hooks_path.display().to_string() })),
                ));
                Some(stop_hook_authority(&parsed))
            }
            Err(error) => {
                gates.push(QualificationGate::fail(
                    "project_hooks_file_present",
                    "Project-scoped `.codex/hooks.json` exists.",
                    "Project hooks file is not valid JSON.",
                    Some(json!({
                        "path": hooks_path.display().to_string(),
                        "error": error.to_string(),
                    })),
                ));
                None
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            gates.push(QualificationGate::fail(
                "project_hooks_file_present",
                "Project-scoped `.codex/hooks.json` exists.",
                "Missing project `.codex/hooks.json`.",
                Some(json!({ "path": hooks_path.display().to_string() })),
            ));
            None
        }
        Err(error) => {
            gates.push(QualificationGate::fail(
                "project_hooks_file_present",
                "Project-scoped `.codex/hooks.json` exists.",
                "Could not read project `.codex/hooks.json`.",
                Some(json!({
                    "path": hooks_path.display().to_string(),
                    "error": error.to_string(),
                })),
            ));
            None
        }
    };

    match project_stop_hook_authority {
        Some((count, managed)) if count == 1 => gates.push(QualificationGate::pass(
            "project_stop_hook_authority",
            "Project hook config exposes one authoritative Stop hook pipeline.",
            if managed == 1 {
                "Exactly one managed Codex1 Stop hook registration was detected.".to_string()
            } else {
                "Exactly one authoritative Stop hook pipeline was detected, implemented outside the direct Codex1 hook command.".to_string()
            },
            Some(json!({
                "path": hooks_path.display().to_string(),
                "authoritative_stop_hook_count": count,
                "managed_stop_hook_count": managed,
            })),
        )),
        Some((0, 0)) => gates.push(QualificationGate::fail(
            "project_stop_hook_authority",
            "Project hook config exposes one authoritative Stop hook pipeline.",
            "No Stop hook registration was detected.",
            Some(json!({
                "path": hooks_path.display().to_string(),
                "authoritative_stop_hook_count": 0,
                "managed_stop_hook_count": 0,
            })),
        )),
        Some((count, managed)) => gates.push(QualificationGate::fail(
            "project_stop_hook_authority",
            "Project hook config exposes one authoritative Stop hook pipeline.",
            format!(
                "Detected {count} authoritative Stop hook registrations with {managed} managed Codex1 handlers; expected exactly one authoritative pipeline."
            ),
            Some(json!({
                "path": hooks_path.display().to_string(),
                "authoritative_stop_hook_count": count,
                "managed_stop_hook_count": managed,
            })),
        )),
        None => gates.push(QualificationGate::skipped(
            "project_stop_hook_authority",
            "Project hook config exposes one authoritative Stop hook pipeline.",
            "Skipped because the project hooks file is missing or unreadable.",
            Some(json!({ "path": hooks_path.display().to_string() })),
        )),
    }

    let user_stop_hook_authority = match fs::read_to_string(&user_hooks_path) {
        Ok(contents) => match serde_json::from_str::<Value>(&contents) {
            Ok(parsed) => Some(stop_hook_authority(&parsed)),
            Err(error) => {
                gates.push(QualificationGate::fail(
                    "user_hooks_file_valid",
                    "User-scoped hook config, when present, must be valid JSON.",
                    "User hooks file is not valid JSON.",
                    Some(json!({
                        "path": user_hooks_path.display().to_string(),
                        "error": error.to_string(),
                    })),
                ));
                None
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some((0, 0)),
        Err(error) => {
            gates.push(QualificationGate::fail(
                "user_hooks_file_valid",
                "User-scoped hook config, when present, must be valid JSON.",
                "Could not read user `.codex/hooks.json`.",
                Some(json!({
                    "path": user_hooks_path.display().to_string(),
                    "error": error.to_string(),
                })),
            ));
            None
        }
    };
    match user_stop_hook_authority {
        Some((0, 0)) => gates.push(QualificationGate::pass(
            "cross_layer_stop_hook_authority",
            "No additional user-level Stop authority conflicts with the repo-local Codex1 pipeline.",
            "No user-level Stop handlers were detected.",
            Some(json!({
                "path": user_hooks_path.display().to_string(),
                "stop_hook_count": 0,
            })),
        )),
        Some((count, managed)) => {
            let parsed = fs::read_to_string(&user_hooks_path)
                .ok()
                .and_then(|contents| serde_json::from_str::<Value>(&contents).ok());
            let counts = parsed
                .as_ref()
                .map(summarize_stop_authority_with_observational)
                .unwrap_or(crate::support_surface::StopAuthorityCounts {
                    total: count,
                    managed,
                    observational: 0,
                });
            if counts.authoritative() == 0 {
                gates.push(QualificationGate::pass(
                    "cross_layer_stop_hook_authority",
                    "No additional user-level Stop authority conflicts with the repo-local Codex1 pipeline.",
                    "All detected user-level Stop hooks are explicitly marked observational.",
                    Some(json!({
                        "path": user_hooks_path.display().to_string(),
                        "stop_hook_count": counts.total,
                        "managed_stop_hook_count": counts.managed,
                        "observational_stop_hook_count": counts.observational,
                    })),
                ));
            } else {
                gates.push(QualificationGate::fail(
                    "cross_layer_stop_hook_authority",
                    "No additional user-level Stop authority conflicts with the repo-local Codex1 pipeline.",
                    format!(
                        "Detected {} user-level authoritative Stop hook registrations across config layers; supported Codex1 environments require one authoritative Stop pipeline.",
                        counts.authoritative()
                    ),
                    Some(json!({
                        "path": user_hooks_path.display().to_string(),
                        "stop_hook_count": counts.total,
                        "managed_stop_hook_count": counts.managed,
                        "observational_stop_hook_count": counts.observational,
                    })),
                ));
            }
        }
        None => {}
    }

    match read_optional_string(&agents_path) {
        Ok(raw) => gates.push(project_agents_scaffold_gate(&agents_path, raw.as_deref())),
        Err(error) => gates.push(QualificationGate::fail(
            "project_agents_scaffold_present",
            "Repo support surfaces include the Codex1-managed AGENTS.md scaffold block.",
            "Could not read AGENTS.md.",
            Some(json!({
                "path": agents_path.display().to_string(),
                "error": error.to_string(),
            })),
        )),
    }

    match inspect_skill_surface(repo_root) {
        Ok(inspection) if inspection.status == SkillSurfaceStatus::ValidExisting => {
            gates.push(QualificationGate::pass(
                "project_skill_surface_valid",
                "Project support surfaces include a discoverable public skill surface.",
                format!(
                    "Found a valid {} skill surface under {}.",
                    inspection
                        .install_mode
                        .map(|mode| mode.as_str())
                        .unwrap_or("discoverable"),
                    inspection.discovery_root.display()
                ),
                Some(json!({
                    "path": inspection.discovery_root.display().to_string(),
                    "install_mode": inspection.install_mode.map(|mode| mode.as_str()),
                    "matched_managed_files": inspection.matched_managed_files,
                })),
            ));
        }
        Ok(inspection) if inspection.status == SkillSurfaceStatus::InvalidBridge => {
            gates.push(QualificationGate::fail(
                "project_skill_surface_valid",
                "Project support surfaces include a discoverable public skill surface.",
                format!(
                    "The configured `[[skills.config]]` bridge at {} is invalid: {}.",
                    inspection.discovery_root.display(),
                    inspection
                        .bridge_error
                        .as_deref()
                        .unwrap_or("invalid bridge target")
                ),
                Some(json!({
                    "path": inspection.discovery_root.display().to_string(),
                    "install_mode": inspection.install_mode.map(|mode| mode.as_str()),
                    "bridge_error": inspection.bridge_error,
                })),
            ))
        }
        Ok(inspection) => gates.push(QualificationGate::fail(
            "project_skill_surface_valid",
            "Project support surfaces include a discoverable public skill surface.",
            format!(
                "Skills under {} are not support-ready (missing: {}; drifted: {}).",
                inspection.discovery_root.display(),
                if inspection.missing_required_public_skills.is_empty() {
                    "none".to_string()
                } else {
                    inspection.missing_required_public_skills.join(", ")
                },
                if inspection.drifted_managed_files.is_empty() {
                    "none".to_string()
                } else {
                    inspection.drifted_managed_files.join(", ")
                }
            ),
            Some(json!({
                "path": inspection.discovery_root.display().to_string(),
                "install_mode": inspection.install_mode.map(|mode| mode.as_str()),
                "missing_required_public_skills": inspection.missing_required_public_skills,
                "drifted_managed_files": inspection.drifted_managed_files,
                "bridge_error": inspection.bridge_error,
            })),
        )),
        Err(error) => gates.push(QualificationGate::fail(
            "project_skill_surface_valid",
            "Project support surfaces include a discoverable public skill surface.",
            "Could not inspect the discoverable skill surface.",
            Some(json!({ "error": error.to_string() })),
        )),
    }

    gates
}

fn detect_codex_hooks_setting(contents: &str) -> Option<bool> {
    detect_toml_bool(contents, Some("features"), "codex_hooks")
}

#[cfg(test)]
fn count_stop_hooks(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.iter().map(count_stop_hooks).sum(),
        Value::Object(map) => {
            if matches!(map.get("event").and_then(Value::as_str), Some("Stop")) {
                return 1;
            }

            if map
                .get("events")
                .and_then(Value::as_array)
                .is_some_and(|events| events.iter().any(|event| event.as_str() == Some("Stop")))
            {
                return 1;
            }

            map.iter()
                .map(|(key, nested)| {
                    if key == "Stop" {
                        stop_bucket_count(nested)
                    } else {
                        count_stop_hooks(nested)
                    }
                })
                .sum()
        }
        _ => 0,
    }
}

fn stop_hook_authority(value: &Value) -> (usize, usize) {
    let counts = summarize_stop_authority_with_observational(value);
    (counts.authoritative(), counts.managed)
}

#[cfg(test)]
fn stop_bucket_count(value: &Value) -> usize {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                0
            } else {
                items
                    .iter()
                    .map(|item| count_stop_hooks(item).max(1))
                    .sum::<usize>()
            }
        }
        Value::Object(_) => count_stop_hooks(value).max(1),
        _ => 1,
    }
}

fn run_isolated_helper_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create qualification sandbox")?;
    let sandbox_root = sandbox.path();
    let repo_root = sandbox_root.join("repo");
    let home_root = sandbox_root.join("home");
    let baseline_home = home_root.join(".codex");

    fs::create_dir_all(&repo_root).context("failed to create temporary repo root")?;
    fs::create_dir_all(&baseline_home).context("failed to create temporary home root")?;
    let canonical_repo_root = fs::canonicalize(&repo_root)
        .with_context(|| format!("failed to canonicalize {}", repo_root.display()))?;

    fs::write(
        repo_root.join("README.md"),
        "# Qualification Sandbox\n\nTemporary repo for codex1 qualification smoke flows.\n",
    )
    .context("failed to seed temporary repo baseline")?;
    fs::write(
        baseline_home.join("config.toml"),
        format!(
            "# user baseline\ntelemetry = false\n\n[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
            canonical_repo_root.display()
        ),
    )
    .context("failed to seed temporary user config baseline")?;

    let repo_before = snapshot_tree(&repo_root)?;
    let home_before = snapshot_tree(&baseline_home)?;

    let mut steps = Vec::new();
    let setup_global = run_smoke_step(
        "setup_global",
        &binary,
        &repo_root,
        &home_root,
        &["setup", "--json"],
    )?;
    let setup_global_backup_id = smoke_step_backup_id(&setup_global);
    let mut should_continue = setup_global.success;
    steps.push(setup_global);

    if should_continue {
        let init_initial = run_smoke_step(
            "init_initial",
            &binary,
            &repo_root,
            &home_root,
            &["init", "--repo-root", repo_root.to_str().unwrap(), "--json"],
        )?;
        should_continue = init_initial.success;
        steps.push(init_initial);
    }
    if should_continue {
        let doctor_after_setup = run_smoke_step(
            "doctor_after_setup",
            &binary,
            &repo_root,
            &home_root,
            &[
                "doctor",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--json",
            ],
        )?;
        should_continue = doctor_after_setup.success;
        steps.push(doctor_after_setup);
    }
    if should_continue {
        let restore_latest = run_smoke_step(
            "restore_project_init",
            &binary,
            &repo_root,
            &home_root,
            &[
                "restore",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--json",
            ],
        )?;
        should_continue = restore_latest.success;
        steps.push(restore_latest);
    }
    if should_continue {
        let setup_after_restore = run_smoke_step(
            "init_after_restore",
            &binary,
            &repo_root,
            &home_root,
            &["init", "--repo-root", repo_root.to_str().unwrap(), "--json"],
        )?;
        should_continue = setup_after_restore.success;
        steps.push(setup_after_restore);
    }
    if should_continue {
        let uninstall_after_setup = run_smoke_step(
            "uninstall_project_init",
            &binary,
            &repo_root,
            &home_root,
            &[
                "uninstall",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--json",
            ],
        )?;
        should_continue = uninstall_after_setup.success;
        steps.push(uninstall_after_setup);
    }
    if should_continue {
        if let Some(backup_id) = setup_global_backup_id.as_deref() {
            let uninstall_global_args = vec![
                "uninstall".to_string(),
                "--repo-root".to_string(),
                repo_root.display().to_string(),
                "--backup-id".to_string(),
                backup_id.to_string(),
                "--json".to_string(),
            ];
            let uninstall_global = run_smoke_step_strings(
                "uninstall_global_setup",
                &binary,
                &repo_root,
                &home_root,
                &uninstall_global_args,
            )?;
            steps.push(uninstall_global);
        } else {
            steps.push(SmokeStep {
                step: "uninstall_global_setup",
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "global setup did not report a backup_id".to_string(),
            });
        }
    }

    let repo_after = snapshot_tree(&repo_root)?;
    let home_after = snapshot_tree(&baseline_home)?;
    let repo_diff = snapshot_diff(&repo_before, &repo_after);
    let home_diff = snapshot_diff(&home_before, &home_after);
    let all_steps_passed = steps.iter().all(|step| step.success);
    let doctor_supported = steps
        .iter()
        .find(|step| step.step == "doctor_after_setup")
        .and_then(|step| serde_json::from_str::<Value>(&step.stdout).ok())
        .map(|value| {
            let supported = value
                .get("supported")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let qualification_only_block = value
                .get("findings")
                .and_then(Value::as_array)
                .is_some_and(|findings| {
                    findings.iter().all(|finding| {
                        let check = finding.get("check").and_then(Value::as_str);
                        let status = finding.get("status").and_then(Value::as_str);
                        status != Some("fail") || check == Some("qualification")
                    })
                });
            supported || qualification_only_block
        })
        .unwrap_or(false);
    let reverted_cleanly = repo_diff.is_empty() && home_diff.is_empty();

    let details = Some(json!({
        "binary": binary.display().to_string(),
        "sandbox_root": sandbox_root.display().to_string(),
        "steps": steps,
        "doctor_after_setup_supported": doctor_supported,
        "repo_diff_after_restore_and_uninstall": repo_diff,
        "home_codex_diff_after_restore_and_uninstall": home_diff,
    }));

    let gate = if all_steps_passed && doctor_supported && reverted_cleanly {
        QualificationGate::pass(
            "isolated_helper_flow",
            "Setup, doctor, restore, and uninstall complete in an isolated temp repo and return the repo to baseline.",
            "The helper command flow succeeded in an isolated temp repo and restored managed state cleanly.",
            details,
        )
    } else if !all_steps_passed {
        QualificationGate::fail(
            "isolated_helper_flow",
            "Setup, doctor, restore, and uninstall complete in an isolated temp repo and return the repo to baseline.",
            "At least one helper command failed in the isolated temp repo flow.",
            details,
        )
    } else if !doctor_supported {
        QualificationGate::fail(
            "isolated_helper_flow",
            "Setup, doctor, restore, and uninstall complete in an isolated temp repo and return the repo to baseline.",
            "Setup ran, but doctor still did not report the isolated repo as support-surface ready.",
            details,
        )
    } else {
        QualificationGate::fail(
            "isolated_helper_flow",
            "Setup, doctor, restore, and uninstall complete in an isolated temp repo and return the repo to baseline.",
            "Helper commands ran, but restore/uninstall did not return the sandbox repo and user surfaces to baseline.",
            details,
        )
    };

    Ok(gate)
}

fn run_helper_force_normalization_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create helper normalization sandbox")?;
    let repo_root = sandbox.path().join("repo");
    let home_root = sandbox.path().join("home");
    seed_helper_sandbox(&repo_root, &home_root)?;

    fs::create_dir_all(repo_root.join(".codex")).context("create repo .codex dir")?;
    fs::write(
        repo_root.join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "codex1 internal stop-hook"
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 user_stop.py"
          }
        ]
      }
    ]
  }
}"#,
    )
    .context("seed multi-stop hooks")?;

    let setup_global = run_smoke_step(
        "setup_global",
        &binary,
        &repo_root,
        &home_root,
        &["setup", "--json"],
    )?;
    let setup_without_force = run_smoke_step(
        "setup_without_force",
        &binary,
        &repo_root,
        &home_root,
        &["init", "--repo-root", repo_root.to_str().unwrap(), "--json"],
    )?;
    let setup_with_force = run_smoke_step(
        "setup_with_force",
        &binary,
        &repo_root,
        &home_root,
        &[
            "init",
            "--force",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;
    let doctor_after_force = run_smoke_step(
        "doctor_after_force",
        &binary,
        &repo_root,
        &home_root,
        &[
            "doctor",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;
    let normalized_counts = fs::read_to_string(repo_root.join(".codex/hooks.json"))
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
        .map(|value| summarize_stop_authority(&value))
        .unwrap_or((0, 0));
    let doctor_supported = doctor_report_support_ready(&doctor_after_force);
    let setup_force_failed_as_expected = !setup_without_force.success
        && format!(
            "{}{}",
            setup_without_force.stdout, setup_without_force.stderr
        )
        .contains("project hooks.json contains authoritative Stop handlers");

    let details = Some(json!({
        "binary": binary.display().to_string(),
        "sandbox_root": sandbox.path().display().to_string(),
        "setup_global": setup_global,
        "setup_without_force": setup_without_force,
        "setup_with_force": setup_with_force,
        "doctor_after_force": doctor_after_force,
        "normalized_project_stop_hook_count": normalized_counts.0,
        "normalized_project_managed_stop_hook_count": normalized_counts.1,
    }));

    Ok(
        if setup_global.success
            && setup_force_failed_as_expected
            && setup_with_force.success
            && doctor_supported
            && normalized_counts == (0, 0)
        {
            QualificationGate::pass(
                "helper_force_normalization_flow",
                "Helper qualification proves multi-Stop conflict detection and force-based normalization back to exactly one authoritative Codex1 Stop pipeline across user and project layers.",
                "The helper flow rejected duplicate project Stop handlers without --force, then removed project Stop authority so the global managed pipeline remains the only authoritative Stop hook.",
                details,
            )
        } else {
            QualificationGate::fail(
                "helper_force_normalization_flow",
                "Helper qualification proves multi-Stop conflict detection and force-based normalization back to exactly one authoritative Codex1 Stop pipeline across user and project layers.",
                "The helper flow did not prove the expected duplicate Stop rejection and global-pipeline force-normalization behavior.",
                details,
            )
        },
    )
}

fn run_helper_partial_install_repair_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create helper partial-install sandbox")?;
    let repo_root = sandbox.path().join("repo");
    let home_root = sandbox.path().join("home");
    seed_helper_sandbox(&repo_root, &home_root)?;

    fs::create_dir_all(repo_root.join(".codex")).context("create repo .codex dir")?;
    fs::write(
        repo_root.join(".codex/config.toml"),
        "model = \"gpt-5.4\"\n[features]\ncodex_hooks = true\n",
    )
    .context("seed partial config")?;
    fs::write(
        repo_root.join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "codex1 internal stop-hook"
      }
    ]
  }
}"#,
    )
    .context("seed partial hooks")?;

    let setup_global = run_smoke_step(
        "setup_global",
        &binary,
        &repo_root,
        &home_root,
        &["setup", "--json"],
    )?;
    let doctor_before_repair = run_smoke_step(
        "doctor_before_repair",
        &binary,
        &repo_root,
        &home_root,
        &[
            "doctor",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;
    let setup_repair = run_smoke_step(
        "setup_repair",
        &binary,
        &repo_root,
        &home_root,
        &[
            "init",
            "--force",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;
    let doctor_after_repair = run_smoke_step(
        "doctor_after_repair",
        &binary,
        &repo_root,
        &home_root,
        &[
            "doctor",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;
    let before_supported = doctor_report_support_ready(&doctor_before_repair);
    let after_supported = doctor_report_support_ready(&doctor_after_repair);
    let details = Some(json!({
        "binary": binary.display().to_string(),
        "sandbox_root": sandbox.path().display().to_string(),
        "setup_global": setup_global,
        "doctor_before_repair": doctor_before_repair,
        "setup_repair": setup_repair,
        "doctor_after_repair": doctor_after_repair,
    }));

    Ok(
        if setup_global.success && !before_supported && setup_repair.success && after_supported {
            QualificationGate::pass(
                "helper_partial_install_repair_flow",
                "Helper qualification proves rerunning init --force repairs a partially written support surface representative of an interrupted install.",
                "The helper flow started from a partial support surface, reported it unsupported, and returned it to a support-ready state by rerunning init --force.",
                details,
            )
        } else {
            QualificationGate::fail(
                "helper_partial_install_repair_flow",
                "Helper qualification proves rerunning init --force repairs a partially written support surface representative of an interrupted install.",
                "The helper flow did not prove clean repair from the seeded partial support surface.",
                details,
            )
        },
    )
}

fn run_helper_drift_detection_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create helper drift sandbox")?;
    let repo_root = sandbox.path().join("repo");
    let home_root = sandbox.path().join("home");
    seed_helper_sandbox(&repo_root, &home_root)?;

    let setup_initial = run_smoke_step(
        "setup_initial",
        &binary,
        &repo_root,
        &home_root,
        &["init", "--repo-root", repo_root.to_str().unwrap(), "--json"],
    )?;
    if !setup_initial.success {
        return Ok(QualificationGate::fail(
            "helper_drift_detection_flow",
            "Helper qualification proves drifted managed files are detected and handled safely.",
            "The initial setup failed before drift detection could be exercised.",
            Some(json!({
                "binary": binary.display().to_string(),
                "sandbox_root": sandbox.path().display().to_string(),
            "setup_initial": setup_initial,
            })),
        ));
    }

    fs::write(
        repo_root.join("AGENTS.md"),
        "<!-- codex1:begin -->\n## Codex1\n- Drifted shared block.\n<!-- codex1:end -->\n",
    )
    .context("drift AGENTS block")?;

    let doctor_after_drift = run_smoke_step(
        "doctor_after_drift",
        &binary,
        &repo_root,
        &home_root,
        &[
            "doctor",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;
    let restore_after_drift = run_smoke_step(
        "restore_after_drift",
        &binary,
        &repo_root,
        &home_root,
        &[
            "restore",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;
    let uninstall_after_drift = run_smoke_step(
        "uninstall_after_drift",
        &binary,
        &repo_root,
        &home_root,
        &[
            "uninstall",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
    )?;

    let doctor_findings = parse_step_json(&doctor_after_drift)
        .and_then(|value| value.get("findings").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    let agents_failed = doctor_findings.iter().any(|finding| {
        finding.get("check").and_then(Value::as_str) == Some("agents_md")
            && finding.get("status").and_then(Value::as_str) == Some("fail")
    });
    let restore_failed_safe = !restore_after_drift.success
        && format!(
            "{}{}",
            restore_after_drift.stdout, restore_after_drift.stderr
        )
        .contains("drifted");
    let uninstall_failed_safe = !uninstall_after_drift.success
        && format!(
            "{}{}",
            uninstall_after_drift.stdout, uninstall_after_drift.stderr
        )
        .contains("drifted");

    let details = Some(json!({
        "binary": binary.display().to_string(),
        "sandbox_root": sandbox.path().display().to_string(),
        "setup_initial": setup_initial,
        "doctor_after_drift": doctor_after_drift,
        "restore_after_drift": restore_after_drift,
        "uninstall_after_drift": uninstall_after_drift,
    }));

    Ok(
        if agents_failed && restore_failed_safe && uninstall_failed_safe {
            QualificationGate::pass(
                "helper_drift_detection_flow",
                "Helper qualification proves drifted managed shared files are surfaced honestly and fail safe under restore/uninstall.",
                "The helper flow detected drift in a managed shared file and both restore and uninstall refused to guess past it.",
                details,
            )
        } else {
            QualificationGate::fail(
                "helper_drift_detection_flow",
                "Helper qualification proves drifted managed shared files are surfaced honestly and fail safe under restore/uninstall.",
                "The helper flow did not prove the expected drift detection and fail-safe behavior.",
                details,
            )
        },
    )
}

fn run_runtime_backend_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create runtime-flow sandbox")?;
    let repo_root = sandbox.path().join("repo");
    fs::create_dir_all(&repo_root).context("failed to create runtime-flow repo root")?;

    let mut steps = Vec::new();
    steps.push(run_json_smoke_step(
        "init_mission",
        &binary,
        &repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "title": "Qualification Runtime Flow",
            "objective": "Prove the internal runtime can create and advance mission contracts.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    )?);
    steps.push(run_json_smoke_step(
        "write_blueprint",
        &binary,
        &repo_root,
        &[
            "internal",
            "materialize-plan",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "body_markdown": canonical_blueprint_body(
                "Use deterministic internal runtime commands.",
                &["runtime_core", "runtime_support"]
            ),
            "plan_level": 4,
            "problem_size": "M",
            "status": "approved",
            "proof_matrix": [{
                "claim_ref": "claim:runtime-proof",
                "statement": "The runtime path has explicit proof and review coverage.",
                "required_evidence": ["RECEIPTS/test-output.txt"],
                "review_lenses": ["correctness", "evidence_adequacy"],
                "governing_contract_refs": ["blueprint"]
            }],
            "decision_obligations": [{
                "obligation_id": "obligation:runtime-route",
                "question": "Should the runtime qualification route stay on deterministic internal commands?",
                "why_it_matters": "It changes the selected route and the review contract.",
                "affects": ["architecture_boundary", "review_contract"],
                "governing_contract_refs": ["blueprint"],
                "review_contract_refs": ["review:runtime"],
                "mission_close_claim_refs": ["claim:runtime-proof"],
                "blockingness": "major",
                "candidate_route_count": 1,
                "required_evidence": ["RECEIPTS/test-output.txt"],
                "status": "selected",
                "resolution_rationale": "The deterministic path is the supported route for qualification.",
                "evidence_refs": ["RECEIPTS/test-output.txt"]
            }],
            "selected_target_ref": "spec:runtime_core",
            "specs": [
                {
                    "spec_id": "runtime_core",
                    "purpose": "Create a runnable workstream.",
                    "body_markdown": qualification_spec_body(
                        "Create a runnable workstream.",
                        &["src"],
                        &["src"],
                        &["cargo test"]
                    ),
                    "artifact_status": "active",
                    "packetization_status": "runnable",
                    "execution_status": "packaged"
                },
                {
                    "spec_id": "runtime_support",
                    "purpose": "Carry supporting qualification work.",
                    "body_markdown": qualification_spec_body(
                        "Carry supporting qualification work.",
                        &["docs"],
                        &["docs"],
                        &["qualify evidence"]
                    ),
                    "artifact_status": "active",
                    "packetization_status": "runnable",
                    "execution_status": "not_started"
                }
            ],
            "execution_graph": {
                "nodes": [
                    {
                        "spec_id": "runtime_core",
                        "depends_on": [],
                        "produces": ["artifact:runtime-core"],
                        "read_paths": ["src"],
                        "write_paths": ["src"],
                        "exclusive_resources": [],
                        "coupling_tags": [],
                        "ownership_domains": ["runtime"],
                        "risk_class": "normal",
                        "acceptance_checks": ["cargo test"],
                        "evidence_type": "receipt"
                    },
                    {
                        "spec_id": "runtime_support",
                        "depends_on": [],
                        "produces": ["artifact:runtime-support"],
                        "read_paths": ["docs"],
                        "write_paths": ["docs"],
                        "exclusive_resources": [],
                        "coupling_tags": [],
                        "ownership_domains": ["runtime"],
                        "risk_class": "normal",
                        "acceptance_checks": ["qualify evidence"],
                        "evidence_type": "receipt"
                    }
                ],
                "obligations": [
                    {
                        "obligation_id": "runtime-core-proof",
                        "kind": "validation",
                        "target_spec_id": "runtime_core",
                        "discharges_claim_ref": "claim:runtime-proof",
                        "proof_rows": ["cargo test"],
                        "acceptance_checks": ["cargo test"],
                        "required_evidence": ["RECEIPTS/test-output.txt"],
                        "review_lenses": ["correctness", "evidence_adequacy"],
                        "blocking": true,
                        "status": "open",
                        "satisfied_by": [],
                        "evidence_refs": []
                    },
                    {
                        "obligation_id": "runtime-support-proof",
                        "kind": "review",
                        "target_spec_id": "runtime_support",
                        "discharges_claim_ref": "claim:runtime-proof",
                        "proof_rows": ["qualify evidence"],
                        "acceptance_checks": ["qualify evidence"],
                        "required_evidence": ["RECEIPTS/test-output.txt"],
                        "review_lenses": ["correctness"],
                        "blocking": true,
                        "status": "open",
                        "satisfied_by": [],
                        "evidence_refs": []
                    }
                ]
            }
        }),
    )?);
    steps.push(run_json_smoke_step(
        "validate_artifacts_after_blueprint",
        &binary,
        &repo_root,
        &[
            "internal",
            "validate-mission-artifacts",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--mission-id",
            "qualify-runtime",
            "--json",
        ],
        &json!({}),
    )?);
    steps.push(run_json_smoke_step(
        "compile_execution_package",
        &binary,
        &repo_root,
        &[
            "internal",
            "compile-execution-package",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [{
                "name": "lock_current",
                "satisfied": true,
                "detail": "Outcome Lock revision is current."
            }],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["spec review"]
        }),
    )?);

    let package_step = steps
        .last()
        .and_then(|step| serde_json::from_str::<Value>(&step.stdout).ok())
        .context("failed to parse runtime execution-package output")?;
    let package_id = package_step
        .get("package_id")
        .and_then(Value::as_str)
        .context("runtime execution-package output is missing package_id")?
        .to_string();

    steps.push(run_json_smoke_step(
        "derive_writer_packet",
        &binary,
        &repo_root,
        &[
            "internal",
            "derive-writer-packet",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "source_package_id": package_id,
            "target_spec_id": "runtime_core",
            "required_checks": ["cargo test"],
            "review_lenses": ["correctness", "evidence_adequacy"],
            "explicitly_disallowed_decisions": ["do not expand write scope"]
        }),
    )?);
    steps.push(run_json_smoke_step(
        "compile_review_bundle",
        &binary,
        &repo_root,
        &[
            "internal",
            "compile-review-bundle",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "source_package_id": package_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness", "evidence_adequacy"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test"],
            "receipts": ["RECEIPTS/test-output.txt"],
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["runtime contract"]
        }),
    )?);

    let bundle_step = steps
        .last()
        .and_then(|step| serde_json::from_str::<Value>(&step.stdout).ok())
        .context("failed to parse runtime review-bundle output")?;
    let bundle_id = bundle_step
        .get("bundle_id")
        .and_then(Value::as_str)
        .context("runtime review-bundle output is missing bundle_id")?
        .to_string();
    let review_truth_snapshot =
        capture_review_truth_snapshot_payload(&binary, &repo_root, "qualify-runtime", &bundle_id)?;
    let spec_review_output_ref = record_none_reviewer_output_ref(
        &binary,
        &repo_root,
        "qualify-runtime",
        &bundle_id,
        "qualification-runtime-spec-review",
        &mut steps,
    )?;
    let code_review_output_ref = record_none_reviewer_output_ref_after_snapshot(
        &binary,
        &repo_root,
        "qualify-runtime",
        &bundle_id,
        "qualification-runtime-code-review",
        &mut steps,
    )?;

    steps.push(run_json_smoke_step(
        "record_review_result",
        &binary,
        &repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "bundle_id": bundle_id,
            "reviewer": "qualify-codex",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                spec_review_output_ref,
                code_review_output_ref,
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Qualification review path is clean."],
            "next_required_branch": "execution",
            "review_truth_snapshot": review_truth_snapshot
        }),
    )?);
    steps.push(run_json_smoke_step(
        "rewrite_blueprint_to_runtime_core_only",
        &binary,
        &repo_root,
        &[
            "internal",
            "materialize-plan",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "body_markdown": canonical_blueprint_body(
                "Converge the runtime qualification frontier to the shipped spec.",
                &["runtime_core"]
            ),
            "plan_level": 4,
            "problem_size": "M",
            "status": "approved",
            "proof_matrix": [{
                "claim_ref": "claim:runtime-proof",
                "statement": "The runtime path has explicit proof and review coverage.",
                "required_evidence": ["RECEIPTS/test-output.txt"],
                "review_lenses": ["correctness", "evidence_adequacy"],
                "governing_contract_refs": ["blueprint"]
            }],
            "decision_obligations": [{
                "obligation_id": "obligation:runtime-route",
                "question": "Should the runtime qualification route stay on deterministic internal commands?",
                "why_it_matters": "It changes the selected route and the review contract.",
                "affects": ["architecture_boundary", "review_contract"],
                "governing_contract_refs": ["blueprint"],
                "review_contract_refs": ["review:runtime"],
                "mission_close_claim_refs": ["claim:runtime-proof"],
                "blockingness": "major",
                "candidate_route_count": 1,
                "required_evidence": ["RECEIPTS/test-output.txt"],
                "status": "selected",
                "resolution_rationale": "The deterministic path is the supported route for qualification.",
                "evidence_refs": ["RECEIPTS/test-output.txt"]
            }],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Create a runnable workstream.",
                "body_markdown": qualification_spec_body(
                    "Create a runnable workstream.",
                    &["src"],
                    &["src"],
                    &["cargo test"]
                ),
                "artifact_status": "active",
                "packetization_status": "runnable",
                "execution_status": "packaged"
            }]
        }),
    )?);
    steps.push(run_json_smoke_step(
        "recompile_execution_package_for_mission_close",
        &binary,
        &repo_root,
        &[
            "internal",
            "compile-execution-package",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [{
                "name": "lock_current",
                "satisfied": true,
                "detail": "Outcome Lock revision is current."
            }],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["mission close"]
        }),
    )?);
    let mission_close_package_step = steps
        .last()
        .and_then(|step| serde_json::from_str::<Value>(&step.stdout).ok())
        .context("failed to parse mission-close execution-package output")?;
    let mission_close_package_id = mission_close_package_step
        .get("package_id")
        .and_then(Value::as_str)
        .context("mission-close execution-package output is missing package_id")?
        .to_string();
    steps.push(run_json_smoke_step(
        "compile_post_completion_review_bundle",
        &binary,
        &repo_root,
        &[
            "internal",
            "compile-review-bundle",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "source_package_id": mission_close_package_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness", "evidence_adequacy"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test", "review clean"],
            "receipts": ["RECEIPTS/test-output.txt"],
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["runtime contract"]
        }),
    )?);
    let post_completion_bundle_step = steps
        .last()
        .and_then(|step| serde_json::from_str::<Value>(&step.stdout).ok())
        .context("failed to parse post-completion review bundle output")?;
    let post_completion_bundle_id = post_completion_bundle_step
        .get("bundle_id")
        .and_then(Value::as_str)
        .context("post-completion review bundle output is missing bundle_id")?
        .to_string();
    let post_completion_review_truth_snapshot = capture_review_truth_snapshot_payload(
        &binary,
        &repo_root,
        "qualify-runtime",
        &post_completion_bundle_id,
    )?;
    let post_completion_output_ref = record_none_reviewer_output_ref(
        &binary,
        &repo_root,
        "qualify-runtime",
        &post_completion_bundle_id,
        "qualification-runtime-post-completion-spec-review",
        &mut steps,
    )?;
    let post_completion_code_output_ref = record_none_reviewer_output_ref_after_snapshot(
        &binary,
        &repo_root,
        "qualify-runtime",
        &post_completion_bundle_id,
        "qualification-runtime-post-completion-code-review",
        &mut steps,
    )?;
    steps.push(run_json_smoke_step(
        "record_post_completion_review_result",
        &binary,
        &repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "bundle_id": post_completion_bundle_id,
            "reviewer": "qualify-codex",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                post_completion_output_ref,
                post_completion_code_output_ref,
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Post-completion review path is clean."],
            "next_required_branch": "mission_close",
            "review_truth_snapshot": post_completion_review_truth_snapshot
        }),
    )?);
    steps.push(run_json_smoke_step(
        "compile_mission_close_bundle",
        &binary,
        &repo_root,
        &[
            "internal",
            "compile-review-bundle",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "source_package_id": mission_close_package_id,
            "bundle_kind": "mission_close",
            "mandatory_review_lenses": [
                "spec_conformance",
                "correctness",
                "interface_compatibility",
                "safety_security_policy",
                "operability_rollback_observability",
                "evidence_adequacy"
            ],
            "mission_level_proof_rows": ["cargo test", "review clean"],
            "cross_spec_claim_refs": ["runtime_core complete"],
            "visible_artifact_refs": [
                fs::canonicalize(repo_root.join("PLANS/qualify-runtime/OUTCOME-LOCK.md")).context("canonicalize mission-close outcome lock")?.display().to_string(),
                fs::canonicalize(repo_root.join("PLANS/qualify-runtime/PROGRAM-BLUEPRINT.md")).context("canonicalize mission-close blueprint")?.display().to_string(),
                fs::canonicalize(repo_root.join("PLANS/qualify-runtime/REVIEW-LEDGER.md")).context("canonicalize mission-close review ledger")?.display().to_string()
            ],
            "deferred_descoped_follow_on_refs": [],
            "open_finding_summary": []
        }),
    )?);

    let mission_close_step = steps
        .last()
        .and_then(|step| serde_json::from_str::<Value>(&step.stdout).ok())
        .context("failed to parse mission-close bundle output")?;
    let mission_close_bundle_id = mission_close_step
        .get("bundle_id")
        .and_then(Value::as_str)
        .context("mission-close bundle output is missing bundle_id")?
        .to_string();
    let mission_close_review_truth_snapshot = capture_review_truth_snapshot_payload(
        &binary,
        &repo_root,
        "qualify-runtime",
        &mission_close_bundle_id,
    )?;
    let mission_close_code_output_ref = record_none_reviewer_output_ref(
        &binary,
        &repo_root,
        "qualify-runtime",
        &mission_close_bundle_id,
        "qualification-runtime-mission-close-code",
        &mut steps,
    )?;
    let mission_close_spec_output_ref = record_none_reviewer_output_ref_after_snapshot(
        &binary,
        &repo_root,
        "qualify-runtime",
        &mission_close_bundle_id,
        "qualification-runtime-mission-close-spec",
        &mut steps,
    )?;

    steps.push(run_json_smoke_step(
        "record_mission_close_review",
        &binary,
        &repo_root,
        &[
            "internal",
            "record-review-outcome",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "bundle_id": mission_close_bundle_id,
            "reviewer": "qualify-codex",
            "verdict": "complete",
            "governing_refs": ["mission-close-bundle"],
            "evidence_refs": [
                mission_close_code_output_ref,
                mission_close_spec_output_ref,
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Qualification mission-close review is clean."],
            "review_truth_snapshot": mission_close_review_truth_snapshot
        }),
    )?);
    let reached_complete_before_contradiction =
        fs::read_to_string(repo_root.join(".ralph/missions/qualify-runtime/state.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|value| {
                value
                    .get("verdict")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .is_some_and(|verdict| verdict == "complete");
    steps.push(run_json_smoke_step(
        "record_contradiction",
        &binary,
        &repo_root,
        &[
            "internal",
            "record-contradiction",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime",
            "discovered_in_phase": "execution",
            "discovered_by": "qualify-codex",
            "target_type": "spec",
            "target_id": "runtime_core",
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "violated_assumption_or_contract": "Qualification contradiction smoke.",
            "suggested_reopen_layer": "execution_package",
            "reason_code": "qualification_smoke",
            "governing_revision": "package"
        }),
    )?);
    steps.push(run_json_smoke_step(
        "init_backup_waiting_mission",
        &binary,
        &repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "backup-mission",
            "title": "Backup Waiting Mission",
            "objective": "Create a second non-terminal mission for resume-selection qualification.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose how to continue the backup mission.",
                "resume_condition": "The user chooses the backup path."
            }
        }),
    )?);
    steps.push(run_json_smoke_step(
        "init_tertiary_waiting_mission",
        &binary,
        &repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "tertiary-mission",
            "title": "Tertiary Waiting Mission",
            "objective": "Create deterministic resume-selection ambiguity for qualification.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose how to continue the tertiary mission.",
                "resume_condition": "The user chooses the tertiary path."
            }
        }),
    )?);
    steps.push(run_json_smoke_step(
        "resolve_resume_wait",
        &binary,
        &repo_root,
        &[
            "internal",
            "resolve-resume",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({}),
    )?);
    let selection_state_after_wait: Value = serde_json::from_slice(
        &fs::read(repo_root.join(".ralph/selection-state.json"))
            .context("failed to read selection state after wait")?,
    )
    .context("failed to parse selection state after wait")?;
    let selection_request_id = selection_state_after_wait
        .get("selection_request_id")
        .and_then(Value::as_str)
        .context("selection state after wait is missing selection_request_id")?
        .to_string();
    let cleared_before_bind = selection_state_after_wait.get("cleared_at").cloned();
    steps.push(run_json_smoke_step(
        "resolve_selection_wait",
        &binary,
        &repo_root,
        &[
            "internal",
            "resolve-selection-wait",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "selected_mission_id": "backup-mission"
        }),
    )?);
    steps.push(run_json_smoke_step(
        "resolve_resume_selected",
        &binary,
        &repo_root,
        &[
            "internal",
            "resolve-resume",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({}),
    )?);
    let selection_state_after_bind: Value = serde_json::from_slice(
        &fs::read(repo_root.join(".ralph/selection-state.json"))
            .context("failed to read selection state after bind")?,
    )
    .context("failed to parse selection state after bind")?;
    steps.push(run_json_smoke_step(
        "resolve_resume_explicit",
        &binary,
        &repo_root,
        &[
            "internal",
            "resolve-resume",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "qualify-runtime"
        }),
    )?);

    let all_steps_passed = steps.iter().all(|step| step.success);
    let paths_ok = [
        repo_root.join("PLANS/qualify-runtime/PROGRAM-BLUEPRINT.md"),
        repo_root.join("PLANS/qualify-runtime/specs/runtime_core/SPEC.md"),
        repo_root.join(".ralph/missions/qualify-runtime/gates.json"),
        repo_root.join(".ralph/missions/qualify-runtime/state.json"),
        repo_root.join(".ralph/missions/qualify-runtime/contradictions.ndjson"),
        repo_root.join(".ralph/selection-state.json"),
    ]
    .into_iter()
    .all(|path| path.exists());

    let explicit_resume_surfaces_contradiction = steps
        .iter()
        .find(|step| step.step == "resolve_resume_explicit")
        .and_then(|step| serde_json::from_str::<Value>(&step.stdout).ok())
        .and_then(|value| {
            value
                .get("resume_status")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .is_some_and(|status| status == "contradictory_state");

    let details = Some(json!({
        "binary": binary.display().to_string(),
        "sandbox_root": sandbox.path().display().to_string(),
        "steps": steps,
        "required_paths_present": paths_ok,
        "reached_complete_before_contradiction": reached_complete_before_contradiction,
        "explicit_resume_surfaces_contradiction": explicit_resume_surfaces_contradiction,
        "selection_request_id": selection_request_id,
        "selection_cleared_before_bind": cleared_before_bind,
        "selection_state_after_bind": selection_state_after_bind,
    }));

    Ok(
        if all_steps_passed
            && paths_ok
            && reached_complete_before_contradiction
            && explicit_resume_surfaces_contradiction
        {
            QualificationGate::pass(
                "runtime_backend_flow",
                "The internal runtime backend can create mission artifacts, packages, writer packets, review bundles, contradictions, and resume-selection state in an isolated repo.",
                "The runtime backend flow completed successfully in an isolated temp repo, including mission-close completion and honest contradiction resume handling.",
                details,
            )
        } else if !all_steps_passed {
            QualificationGate::fail(
                "runtime_backend_flow",
                "The internal runtime backend can create mission artifacts, packages, writer packets, review bundles, contradictions, and resume-selection state in an isolated repo.",
                "At least one runtime backend step failed in the isolated temp repo.",
                details,
            )
        } else {
            QualificationGate::fail(
                "runtime_backend_flow",
                "The internal runtime backend can create mission artifacts, packages, writer packets, review bundles, contradictions, and resume-selection state in an isolated repo.",
                "Runtime backend commands ran, but required persisted artifacts or terminal completion state were missing.",
                details,
            )
        },
    )
}

fn run_waiting_stop_hook_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create waiting-flow sandbox")?;
    let repo_root = sandbox.path().join("repo");
    fs::create_dir_all(&repo_root).context("failed to create waiting-flow repo root")?;

    let init_step = run_json_smoke_step(
        "init_waiting_mission",
        &binary,
        &repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "waiting-mission",
            "title": "Waiting Mission",
            "objective": "Prove Ralph waiting state remains non-terminal.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Please choose the rollout posture.",
                "resume_condition": "The user chooses one rollout posture."
            }
        }),
    )?;

    let stop_output = Command::new(&binary)
        .args(["internal", "stop-hook"])
        .current_dir(&repo_root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn stop-hook probe")?;

    let output = run_stop_hook_probe(
        stop_output,
        &json!({ "cwd": repo_root.display().to_string() }),
    )?;
    let parsed: Value =
        serde_json::from_slice(&output.stdout).context("failed to parse stop-hook JSON")?;

    let waiting_state: Value = serde_json::from_slice(
        &fs::read(repo_root.join(".ralph/missions/waiting-mission/state.json"))
            .context("failed to read waiting mission state")?,
    )
    .context("failed to parse waiting mission state")?;
    let waiting_request_id = waiting_state
        .get("waiting_request_id")
        .and_then(Value::as_str)
        .context("waiting mission state is missing waiting_request_id")?;

    let acknowledge_waiting = run_json_smoke_step(
        "acknowledge_waiting_request",
        &binary,
        &repo_root,
        &[
            "internal",
            "acknowledge-waiting-request",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--mission-id",
            "waiting-mission",
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "waiting_request_id": waiting_request_id
        }),
    )?;

    let second_output = run_stop_hook_probe(
        Command::new(&binary)
            .args(["internal", "stop-hook"])
            .current_dir(&repo_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn second stop-hook probe")?,
        &json!({ "cwd": repo_root.display().to_string() }),
    )?;
    let second_parsed: Value = serde_json::from_slice(&second_output.stdout)
        .context("failed to parse second stop-hook JSON")?;
    let waiting_state_after_second_emit: Value = serde_json::from_slice(
        &fs::read(repo_root.join(".ralph/missions/waiting-mission/state.json"))
            .context("failed to reread waiting mission state")?,
    )
    .context("failed to parse waiting mission state after second emit")?;

    let second_waiting_step = run_json_smoke_step(
        "init_fallback_waiting_mission",
        &binary,
        &repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "fallback-mission",
            "title": "Fallback Waiting Mission",
            "objective": "Create selection ambiguity for stop-hook qualification.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose the fallback mission path.",
                "resume_condition": "The user chooses the fallback mission."
            }
        }),
    )?;

    let selection_step = run_json_smoke_step(
        "resolve_resume_wait",
        &binary,
        &repo_root,
        &[
            "internal",
            "resolve-resume",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--json",
        ],
        &json!({}),
    )?;
    let selection_output = run_stop_hook_probe(
        Command::new(&binary)
            .args(["internal", "stop-hook"])
            .current_dir(&repo_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn selection stop-hook probe")?,
        &json!({ "cwd": repo_root.display().to_string() }),
    )?;
    let selection_parsed: Value = serde_json::from_slice(&selection_output.stdout)
        .context("failed to parse selection stop-hook JSON")?;
    let selection_state: Value = serde_json::from_slice(
        &fs::read(repo_root.join(".ralph/selection-state.json"))
            .context("failed to read selection state")?,
    )
    .context("failed to parse selection state")?;
    let selection_request_id = selection_state
        .get("selection_request_id")
        .and_then(Value::as_str)
        .context("selection state is missing selection_request_id")?;
    let acknowledge_selection = run_json_smoke_step(
        "acknowledge_selection_request",
        &binary,
        &repo_root,
        &[
            "internal",
            "acknowledge-selection-request",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "selection_request_id": selection_request_id
        }),
    )?;
    let second_selection_output = run_stop_hook_probe(
        Command::new(&binary)
            .args(["internal", "stop-hook"])
            .current_dir(&repo_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn second selection stop-hook probe")?,
        &json!({ "cwd": repo_root.display().to_string() }),
    )?;
    let second_selection_parsed: Value = serde_json::from_slice(&second_selection_output.stdout)
        .context("failed to parse second selection stop-hook JSON")?;
    let selection_state_after_second_emit: Value = serde_json::from_slice(
        &fs::read(repo_root.join(".ralph/selection-state.json"))
            .context("failed to reread selection state")?,
    )
    .context("failed to parse selection state after second emit")?;

    let success = init_step.success
        && output.status.success()
        && acknowledge_waiting.success
        && second_output.status.success()
        && second_waiting_step.success
        && selection_step.success
        && selection_output.status.success()
        && acknowledge_selection.success
        && second_selection_output.status.success()
        && parsed.get("decision").is_none()
        && parsed
            .get("systemMessage")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("rollout posture"))
        && second_parsed
            .get("systemMessage")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("rollout posture"))
        && waiting_state_after_second_emit
            .get("waiting_request_id")
            .and_then(Value::as_str)
            == Some(waiting_request_id)
        && waiting_state_after_second_emit
            .get("request_emitted_at")
            .and_then(Value::as_str)
            .is_some()
        && selection_parsed
            .get("systemMessage")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("Select the mission to resume"))
        && second_selection_parsed
            .get("systemMessage")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("Select the mission to resume"))
        && selection_state_after_second_emit
            .get("selection_request_id")
            .and_then(Value::as_str)
            == Some(selection_request_id)
        && selection_state_after_second_emit
            .get("request_emitted_at")
            .and_then(Value::as_str)
            .is_some();

    let details = Some(json!({
        "binary": binary.display().to_string(),
        "sandbox_root": sandbox.path().display().to_string(),
        "init_step": init_step,
        "acknowledge_waiting": acknowledge_waiting,
        "second_waiting_step": second_waiting_step,
        "selection_step": selection_step,
        "acknowledge_selection": acknowledge_selection,
        "stop_hook_stdout": trim_output(&output.stdout),
        "stop_hook_stderr": trim_output(&output.stderr),
        "parsed_stop_hook": parsed,
        "second_stop_hook_stdout": trim_output(&second_output.stdout),
        "second_stop_hook_stderr": trim_output(&second_output.stderr),
        "second_parsed_stop_hook": second_parsed,
        "waiting_state_after_second_emit": waiting_state_after_second_emit,
        "selection_stop_hook_stdout": trim_output(&selection_output.stdout),
        "selection_stop_hook_stderr": trim_output(&selection_output.stderr),
        "selection_parsed_stop_hook": selection_parsed,
        "second_selection_stop_hook_stdout": trim_output(&second_selection_output.stdout),
        "second_selection_stop_hook_stderr": trim_output(&second_selection_output.stderr),
        "second_selection_parsed_stop_hook": second_selection_parsed,
        "selection_state_after_second_emit": selection_state_after_second_emit,
    }));

    Ok(if success {
        QualificationGate::pass(
            "waiting_stop_hook_flow",
            "A durable `needs_user` waiting state yields through the Stop hook without falsely terminalizing the mission.",
            "The Stop hook preserved unresolved waits and kept surfacing the canonical waiting requests until the waits were actually resolved.",
            details,
        )
    } else {
        QualificationGate::fail(
            "waiting_stop_hook_flow",
            "A durable `needs_user` waiting state yields through the Stop hook without falsely terminalizing the mission.",
            "The waiting-state Stop hook flow did not surface the expected non-terminal yield behavior.",
            details,
        )
    })
}

fn run_stop_hook_smoke_probe(
    step: &'static str,
    binary: &Path,
    repo_root: &Path,
    input: &Value,
) -> Result<(SmokeStep, Option<Value>)> {
    let output = run_stop_hook_probe(
        Command::new(binary)
            .args(["internal", "stop-hook"])
            .current_dir(repo_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn {step} stop-hook probe"))?,
        input,
    )?;
    let stdout = trim_output(&output.stdout);
    let parsed = serde_json::from_str::<Value>(&stdout).ok();
    Ok((
        SmokeStep {
            step,
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout,
            stderr: trim_output(&output.stderr),
        },
        parsed,
    ))
}

fn run_control_loop_boundary_flow() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create control-loop sandbox")?;
    let repo_root = sandbox.path().join("repo");
    fs::create_dir_all(repo_root.join(".codex"))
        .context("failed to create control-loop repo support surface")?;
    fs::write(
        repo_root.join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "codex1 internal stop-hook"
          }
        ]
      }
    ]
  }
}"#,
    )
    .context("failed to write control-loop hooks.json")?;

    let mission_id = "control-loop-boundary";
    let (mut steps, _bundle_id) = open_reviewer_boundary_bundle(&binary, &repo_root, mission_id)?;

    let parent_input = json!({ "cwd": repo_root.display().to_string() });
    let (no_lease_step, no_lease_json) =
        run_stop_hook_smoke_probe("no_lease_parent_stop", &binary, &repo_root, &parent_input)?;
    let no_lease_yielded = no_lease_step.success
        && no_lease_json.as_ref().is_some_and(|value| {
            value.get("decision").is_none_or(Value::is_null)
                && value
                    .get("systemMessage")
                    .and_then(Value::as_str)
                    .is_some_and(|message| message.contains("Ralph loop is not active"))
        });
    steps.push(no_lease_step);

    let child_input = json!({
        "cwd": repo_root.display().to_string(),
        "laneRole": "child_helper",
        "agentName": "/root/control-loop-child"
    });
    let (child_step, child_json) =
        run_stop_hook_smoke_probe("subagent_stop", &binary, &repo_root, &child_input)?;
    let subagent_yielded = child_step.success
        && child_json.as_ref().is_some_and(|value| {
            value.get("decision").is_none_or(Value::is_null)
                && value
                    .get("systemMessage")
                    .and_then(Value::as_str)
                    .is_some_and(|message| message.contains("Subagent lane may stop"))
        });
    steps.push(child_step);

    let begin_lease = run_json_smoke_step(
        "begin_parent_loop_lease",
        &binary,
        &repo_root,
        &[
            "internal",
            "begin-loop-lease",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "parent-review-loop",
            "reason": "Qualification proves active parent leases still enforce open review gates."
        }),
    )?;
    let lease_active = begin_lease.success
        && serde_json::from_str::<Value>(&begin_lease.stdout)
            .ok()
            .is_some_and(|value| {
                value.get("status").and_then(Value::as_str) == Some("active")
                    && value.get("mode").and_then(Value::as_str) == Some("review_loop")
            });
    let loop_authority_token = serde_json::from_str::<Value>(&begin_lease.stdout)
        .ok()
        .and_then(|value| {
            value
                .get("parent_authority_token")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    steps.push(begin_lease);

    let (active_parent_step, active_parent_json) =
        run_stop_hook_smoke_probe("active_parent_stop", &binary, &repo_root, &parent_input)?;
    let active_parent_blocked = active_parent_step.success
        && active_parent_json.as_ref().is_some_and(|value| {
            value.get("decision").and_then(Value::as_str) == Some("block")
                && value
                    .get("reason")
                    .and_then(Value::as_str)
                    .is_some_and(|reason| reason.contains("review gate"))
        });
    steps.push(active_parent_step);

    let pause_lease = if let Some(token) = loop_authority_token.as_deref() {
        run_json_smoke_step_with_env(
            "pause_parent_loop_lease",
            &binary,
            &repo_root,
            &[
                "internal",
                "pause-loop-lease",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &json!({
                "mission_id": mission_id,
                "paused_by": "qualification",
                "reason": "Qualification proves $close-style pause lets user discussion stop normally."
            }),
            &[("CODEX1_PARENT_LOOP_AUTHORITY_TOKEN", token)],
        )?
    } else {
        SmokeStep {
            step: "pause_parent_loop_lease",
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "missing parent authority token".to_string(),
        }
    };
    let lease_paused = pause_lease.success
        && serde_json::from_str::<Value>(&pause_lease.stdout)
            .ok()
            .is_some_and(|value| {
                value.pointer("/lease/status").and_then(Value::as_str) == Some("paused")
            });
    steps.push(pause_lease);

    let (paused_parent_step, paused_parent_json) =
        run_stop_hook_smoke_probe("paused_parent_stop", &binary, &repo_root, &parent_input)?;
    let paused_parent_yielded = paused_parent_step.success
        && paused_parent_json.as_ref().is_some_and(|value| {
            value.get("decision").is_none_or(Value::is_null)
                && value
                    .get("systemMessage")
                    .and_then(Value::as_str)
                    .is_some_and(|message| message.contains("Ralph loop is not active"))
        });
    steps.push(paused_parent_step);

    let clear_lease = if let Some(token) = loop_authority_token.as_deref() {
        run_json_smoke_step_with_env(
            "clear_parent_loop_lease",
            &binary,
            &repo_root,
            &[
                "internal",
                "clear-loop-lease",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--json",
            ],
            &json!({}),
            &[("CODEX1_PARENT_LOOP_AUTHORITY_TOKEN", token)],
        )?
    } else {
        SmokeStep {
            step: "clear_parent_loop_lease",
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "missing parent authority token".to_string(),
        }
    };
    let lease_cleared = clear_lease.success;
    steps.push(clear_lease);

    let hook_surface_installed = repo_root.join(".codex/hooks.json").is_file();
    let details = Some(json!({
        "sandbox_root": sandbox.path().display().to_string(),
        "hooks_json": repo_root.join(".codex/hooks.json").display().to_string(),
        "checks": {
            "hook_surface_installed": hook_surface_installed,
            "no_lease_yielded": no_lease_yielded,
            "subagent_yielded": subagent_yielded,
            "lease_active": lease_active,
            "active_parent_blocked": active_parent_blocked,
            "lease_paused": lease_paused,
            "paused_parent_yielded": paused_parent_yielded,
            "lease_cleared": lease_cleared
        },
        "steps": steps
    }));

    if hook_surface_installed
        && no_lease_yielded
        && subagent_yielded
        && lease_active
        && active_parent_blocked
        && lease_paused
        && paused_parent_yielded
        && lease_cleared
    {
        Ok(QualificationGate::pass(
            CONTROL_LOOP_BOUNDARY_GATE,
            "Scoped Ralph control-loop qualification proves hooks are safe because Stop-hook enforcement is lease-scoped.",
            "With an installed Stop hook and an open review gate, no-lease parent turns yielded, subagent turns yielded, an active parent lease blocked, and a paused lease yielded again.",
            details,
        ))
    } else {
        Ok(QualificationGate::fail(
            CONTROL_LOOP_BOUNDARY_GATE,
            "Scoped Ralph control-loop qualification proves hooks are safe because Stop-hook enforcement is lease-scoped.",
            "The scoped control-loop proof did not satisfy no-lease yield, subagent yield, active parent block, and pause-yield expectations.",
            details,
        ))
    }
}

fn run_native_stop_hook_live_flow(live: bool) -> Result<QualificationGate> {
    if !live {
        return Ok(QualificationGate::skipped(
            "native_stop_hook_live_flow",
            "The trusted Codex build exercises the real native Stop-hook dispatch path with the Ralph continuation prompt intact.",
            "Skipped because live Codex qualification was disabled for this run.",
            None,
        ));
    }

    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create native stop-hook sandbox")?;
    let repo_root = sandbox.path().join("repo");
    let codex_home_root = sandbox.path().join("codex-home");
    fs::create_dir_all(repo_root.join(".codex/hooks"))
        .context("failed to create native stop-hook repo surfaces")?;
    seed_native_codex_repo(&repo_root)?;
    seed_native_codex_home(&repo_root, &codex_home_root)?;
    fs::write(
        repo_root.join(".codex/config.toml"),
        "model = \"gpt-5.4\"\n[features]\ncodex_hooks = true\n",
    )
    .context("failed to write native stop-hook config")?;
    let hook_script = repo_root.join(".codex/hooks/native-stop-probe.sh");
    fs::write(
        &hook_script,
        format!(
            "#!/bin/sh\nset -eu\nstdin_file=\".codex/hooks/native-stop-probe.stdin.json\"\nstdout_file=\".codex/hooks/native-stop-probe.stdout.json\"\nstderr_file=\".codex/hooks/native-stop-probe.stderr.txt\"\ncat > \"$stdin_file\"\n\"{}\" internal stop-hook < \"$stdin_file\" > \"$stdout_file\" 2> \"$stderr_file\"\ncat \"$stdout_file\"\n",
            binary.display()
        ),
    )
    .context("failed to write native stop-hook probe script")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&hook_script)
            .context("failed to stat native stop-hook probe script")?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook_script, permissions)
            .context("failed to chmod native stop-hook probe script")?;
    }
    fs::write(
        repo_root.join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "./.codex/hooks/native-stop-probe.sh",
            "statusMessage": "Codex1 Ralph stop hook"
          }
        ]
      }
    ]
  }
}"#,
    )
    .context("failed to write native stop-hook hooks.json")?;

    let init_step = run_json_smoke_step(
        "init_native_stop_waiting_mission",
        &binary,
        &repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "native-stop",
            "title": "Native Stop Waiting Mission",
            "objective": "Prove native Codex dispatches the repo-local Ralph Stop hook.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Please choose the rollout posture.",
                "resume_condition": "The user chooses a rollout posture."
            }
        }),
    )?;

    let native = run_native_codex_prompt(
        &repo_root,
        &codex_home_root,
        NativeCodexInvocation::Exec,
        "Reply with only STOP_PROBE.",
    )?;
    let hook_stdout_path = repo_root.join(".codex/hooks/native-stop-probe.stdout.json");
    let hook_stdin_path = repo_root.join(".codex/hooks/native-stop-probe.stdin.json");
    let hook_stdout = fs::read_to_string(&hook_stdout_path).ok();
    let hook_stdin = fs::read_to_string(&hook_stdin_path).ok();
    let parsed_hook_stdout = hook_stdout
        .as_deref()
        .and_then(|contents| serde_json::from_str::<Value>(contents).ok());
    let success = init_step.success
        && native.status.success()
        && hook_stdout.is_some()
        && hook_stdin.is_some()
        && parsed_hook_stdout
            .as_ref()
            .and_then(|value| value.get("systemMessage"))
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("Please choose the rollout posture."));

    Ok(if success {
        QualificationGate::pass(
            "native_stop_hook_live_flow",
            "The trusted Codex build exercises the real native Stop-hook dispatch path with the Ralph continuation prompt intact.",
            "A live native Codex exec run dispatched the repo-local Ralph Stop hook and captured the canonical waiting request it emitted.",
            Some(json!({
                "sandbox_root": sandbox.path().display().to_string(),
                "repo_root": repo_root.display().to_string(),
                "codex_home_root": codex_home_root.display().to_string(),
                "init_step": init_step,
                "native_stdout": trim_output(&native.stdout),
                "native_stderr": trim_output(&native.stderr),
                "hook_probe_stdin": hook_stdin,
                "hook_probe_stdout": hook_stdout,
                "parsed_hook_probe_stdout": parsed_hook_stdout,
            })),
        )
    } else {
        QualificationGate::fail(
            "native_stop_hook_live_flow",
            "The trusted Codex build exercises the real native Stop-hook dispatch path with the Ralph continuation prompt intact.",
            "The live native Codex run did not dispatch the repo-local Ralph Stop hook or did not preserve the canonical waiting request.",
            Some(json!({
                "sandbox_root": sandbox.path().display().to_string(),
                "repo_root": repo_root.display().to_string(),
                "codex_home_root": codex_home_root.display().to_string(),
                "init_step": init_step,
                "native_stdout": trim_output(&native.stdout),
                "native_stderr": trim_output(&native.stderr),
                "hook_probe_stdin": hook_stdin,
                "hook_probe_stdout": hook_stdout,
                "parsed_hook_probe_stdout": parsed_hook_stdout,
            })),
        )
    })
}

fn run_native_exec_resume_flow(live: bool) -> Result<QualificationGate> {
    if !live {
        return Ok(QualificationGate::skipped(
            "native_exec_resume_flow",
            "The trusted Codex build can create an exec session and resume the same thread through `codex exec resume`.",
            "Skipped because live Codex qualification was disabled for this run.",
            None,
        ));
    }

    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create native exec-resume sandbox")?;
    let repo_root = sandbox.path().join("repo");
    let codex_home_root = sandbox.path().join("codex-home");
    fs::create_dir_all(repo_root.join(".codex/hooks"))
        .context("failed to create native exec-resume repo surfaces")?;
    seed_native_codex_repo(&repo_root)?;
    seed_native_codex_home(&repo_root, &codex_home_root)?;
    fs::write(
        repo_root.join(".codex/config.toml"),
        "model = \"gpt-5.4\"\n[features]\ncodex_hooks = true\n",
    )
    .context("failed to write native exec-resume config")?;
    let hook_script = repo_root.join(".codex/hooks/native-exec-resume-probe.sh");
    fs::write(
        &hook_script,
        format!(
            "#!/bin/sh\nset -eu\nstdin_file=\".codex/hooks/native-exec-resume.stdin.json\"\nstdout_file=\".codex/hooks/native-exec-resume.stdout.json\"\nstderr_file=\".codex/hooks/native-exec-resume.stderr.txt\"\ncat > \"$stdin_file\"\n\"{}\" internal stop-hook < \"$stdin_file\" > \"$stdout_file\" 2> \"$stderr_file\"\ncat \"$stdout_file\"\n",
            binary.display()
        ),
    )
    .context("failed to write native exec-resume probe script")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&hook_script)
            .context("failed to stat native exec-resume probe script")?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook_script, permissions)
            .context("failed to chmod native exec-resume probe script")?;
    }
    fs::write(
        repo_root.join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "./.codex/hooks/native-exec-resume-probe.sh",
            "statusMessage": "Codex1 Ralph stop hook"
          }
        ]
      }
    ]
  }
}"#,
    )
    .context("failed to write native exec-resume hooks.json")?;
    let init_step = run_json_smoke_step(
        "init_native_exec_resume_waiting_mission",
        &binary,
        &repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": "native-exec-resume",
            "title": "Native Exec Resume Waiting Mission",
            "objective": "Prove native exec resume preserves Ralph waiting continuity.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose the rollout posture before resuming.",
                "resume_condition": "The user chooses a rollout posture."
            }
        }),
    )?;
    let state_path = repo_root.join(".ralph/missions/native-exec-resume/state.json");
    let expected_waiting_request = "Choose the rollout posture before resuming.";
    let waiting_state_before: Value = serde_json::from_slice(
        &fs::read(&state_path).context("failed to read initial native exec-resume state")?,
    )
    .context("failed to parse initial native exec-resume state")?;
    let waiting_request_id_before = waiting_state_before
        .get("waiting_request_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .context("native exec-resume state is missing waiting_request_id")?;

    let first = run_native_codex_prompt(
        &repo_root,
        &codex_home_root,
        NativeCodexInvocation::Exec,
        "Reply with only FIRST.",
    )?;
    let thread_id = parse_thread_id_from_jsonl(&first.stdout)?
        .context("native exec run did not emit thread.started")?;
    let first_message = last_agent_message_text(&first.stdout)?;
    let first_hook_stdout =
        fs::read_to_string(repo_root.join(".codex/hooks/native-exec-resume.stdout.json")).ok();
    let first_parsed_hook_stdout = first_hook_stdout
        .as_deref()
        .and_then(|contents| serde_json::from_str::<Value>(contents).ok());

    let second = run_native_codex_prompt(
        &repo_root,
        &codex_home_root,
        NativeCodexInvocation::ExecResume {
            thread_id: thread_id.clone(),
        },
        "Reply with only SECOND.",
    )?;
    let resumed_thread_id = parse_thread_id_from_jsonl(&second.stdout)?
        .context("native exec resume run did not emit thread.started")?;
    let second_message = last_agent_message_text(&second.stdout)?;
    let same_thread = resumed_thread_id == thread_id;
    let second_hook_stdout =
        fs::read_to_string(repo_root.join(".codex/hooks/native-exec-resume.stdout.json")).ok();
    let second_parsed_hook_stdout = second_hook_stdout
        .as_deref()
        .and_then(|contents| serde_json::from_str::<Value>(contents).ok());
    let waiting_state_after: Value = serde_json::from_slice(
        &fs::read(&state_path).context("failed to reread native exec-resume state")?,
    )
    .context("failed to parse native exec-resume state after resume")?;
    let waiting_request_id_after = waiting_state_after
        .get("waiting_request_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    let details = Some(json!({
        "sandbox_root": sandbox.path().display().to_string(),
        "repo_root": repo_root.display().to_string(),
        "codex_home_root": codex_home_root.display().to_string(),
        "init_step": init_step,
        "thread_id": thread_id,
        "resumed_thread_id": resumed_thread_id,
        "first_stdout": trim_output(&first.stdout),
        "first_stderr": trim_output(&first.stderr),
        "second_stdout": trim_output(&second.stdout),
        "second_stderr": trim_output(&second.stderr),
        "first_message": first_message,
        "second_message": second_message,
        "first_hook_stdout": first_hook_stdout,
        "second_hook_stdout": second_hook_stdout,
        "first_parsed_hook_stdout": first_parsed_hook_stdout,
        "second_parsed_hook_stdout": second_parsed_hook_stdout,
        "waiting_state_before": waiting_state_before,
        "waiting_state_after": waiting_state_after,
    }));

    Ok(
        if init_step.success
            && first.status.success()
            && second.status.success()
            && first_message.as_deref() == Some("FIRST")
            && second_message.as_deref() == Some("SECOND")
            && same_thread
            && first_parsed_hook_stdout
                .as_ref()
                .and_then(|value| value.get("systemMessage"))
                .and_then(Value::as_str)
                == Some(expected_waiting_request)
            && second_parsed_hook_stdout
                .as_ref()
                .and_then(|value| value.get("systemMessage"))
                .and_then(Value::as_str)
                == Some(expected_waiting_request)
            && waiting_request_id_after.as_deref() == Some(waiting_request_id_before.as_str())
        {
            QualificationGate::pass(
                "native_exec_resume_flow",
                "The trusted Codex build can create an exec session and resume the same thread through `codex exec resume`.",
                "The native Codex exec surface resumed the same thread while preserving the same Ralph waiting mission and canonical waiting request.",
                details,
            )
        } else {
            QualificationGate::fail(
                "native_exec_resume_flow",
                "The trusted Codex build can create an exec session and resume the same thread through `codex exec resume`.",
                "The native Codex exec resume round-trip did not preserve the expected thread, waiting mission, or canonical waiting request.",
                details,
            )
        },
    )
}

fn run_native_multi_agent_resume_flow(live: bool) -> Result<QualificationGate> {
    const GATE: &str = "native_multi_agent_resume_flow";
    const DESCRIPTION: &str = "The trusted Codex build exposes native child-agent tools and Codex1 reconciles their live snapshot honestly on resume.";

    if !live {
        return Ok(QualificationGate::skipped(
            GATE,
            DESCRIPTION,
            "Skipped because live Codex qualification was disabled for this run.",
            None,
        ));
    }

    let result = (|| -> Result<QualificationGate> {
        let binary = qualification_binary()?;
        let sandbox = tempfile::tempdir().context("failed to create native multi-agent sandbox")?;
        let repo_root = sandbox.path().join("repo");
        let codex_home_root = sandbox.path().join("codex-home");
        fs::create_dir_all(&repo_root).context("failed to create native multi-agent repo root")?;
        seed_native_codex_repo(&repo_root)?;
        seed_native_codex_home(&repo_root, &codex_home_root)?;
        fs::write(
            repo_root.join(".codex/config.toml"),
            "model = \"gpt-5.4\"\n[agents]\nmax_threads = 16\nmax_depth = 1\n",
        )
        .context("failed to write native multi-agent config")?;

        let prompt = r#"Use the native child-agent tools in this order.
Do not claim that you used a tool unless the tool call actually succeeded.
This gate is primarily about proving live child inspection and honest resume reconciliation.
1. spawn one child with task_name "qualify_child" and tell it to run shell command `sleep 20` and then reply with the single word PING.
2. call list_agents and record the status for qualify_child.
3. if the surface exposes queue-only child messaging, call send_message to send a short queued note to qualify_child and record that you used it.
4. if the surface exposes the turn-triggering child-delivery tool, call assign_task; if this build instead surfaces the current-development alias, call followup_task. Tell qualify_child to reply with only PING and use interrupt=true only if needed. Record which tool name you used.
5. call wait_agent once with timeout_ms 5000.
6. call list_agents again and record the status for qualify_child.
7. call close_agent on qualify_child and then call list_agents one more time so you can report whether the child is still visible after close.
8. reply with one JSON object only containing keys: used_spawn_agent, used_send_message, used_assign_task, used_followup_task, used_list_agents, used_wait_agent, used_close_agent, child_task_path, child_seen_before_wait, child_status_before_wait, child_seen_after_wait, child_status_after_wait, child_seen_after_close, child_status_after_close, wait_summary_present, list_agents_before_wait, list_agents_after_wait, list_agents_after_close."#;

        let native = run_native_codex_prompt(
            &repo_root,
            &codex_home_root,
            NativeCodexInvocation::Exec,
            prompt,
        )?;
        let native_events = parse_jsonl_events(&native.stdout)?;
        let final_agent_message = last_agent_message_text(&native.stdout)?;
        let (reported_summary, reported_summary_parse_error) =
            match parse_native_multi_agent_summary(&native.stdout) {
                Ok(summary) => (Some(summary), None),
                Err(error) => (None, Some(format!("{error:#}"))),
            };
        let native_summary = summarize_native_multi_agent_events(&native_events);

        if native_summary.child_task_path.trim().is_empty() {
            return Ok(native_multi_agent_gate_failure(
                "The native child-agent probe did not produce a raw child identity that resume reconciliation could bind to.",
                Some(json!({
                    "sandbox_root": sandbox.path().display().to_string(),
                    "repo_root": repo_root.display().to_string(),
                    "codex_home_root": codex_home_root.display().to_string(),
                    "native_stdout": full_output(&native.stdout),
                    "native_stderr": full_output(&native.stderr),
                    "native_events": native_events,
                    "final_agent_message": final_agent_message,
                    "reported_summary": reported_summary,
                    "reported_summary_parse_error": reported_summary_parse_error,
                    "native_summary": native_summary,
                    "failure_classification": "child_task_path_missing",
                })),
            ));
        }

        let init_step = run_json_smoke_step(
            "init_native_multi_agent_mission",
            &binary,
            &repo_root,
            &[
                "internal",
                "init-mission",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &json!({
                "mission_id": "native-reconcile",
                "title": "Native Reconcile",
                "objective": "Prove native child lanes reconcile honestly on resume.",
                "clarify_status": "ratified",
                "lock_status": "locked"
            }),
        )?;

        let child_status = native_child_status_for_resume(&native_summary);
        let child_task_path = native_summary.child_task_path.clone();
        let write_closeout_step = run_json_smoke_step(
            "write_native_child_closeout",
            &binary,
            &repo_root,
            &[
                "internal",
                "append-closeout",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--mission-id",
                "native-reconcile",
                "--input-json",
                "-",
                "--json",
            ],
            &native_child_probe_closeout_payload(&native_summary),
        )?;

        let live_child_lanes = match child_status {
            Some(status) => vec![json!({
                "task_path": child_task_path,
                "status": status,
            })],
            None => Vec::new(),
        };

        let resolve_resume_step = run_json_smoke_step(
            "resolve_native_child_resume",
            &binary,
            &repo_root,
            &[
                "internal",
                "resolve-resume",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &json!({
                "mission_id": "native-reconcile",
                "live_child_lanes": live_child_lanes,
            }),
        )?;

        let resolve_report: Value = serde_json::from_str(&resolve_resume_step.stdout)
            .context("failed to parse native child resume report")?;
        let reconciliation_entries = resolve_report
            .get("child_reconciliation")
            .and_then(|value| value.get("entries"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let matching_entry = reconciliation_entries.iter().find(|entry| {
            entry
                .get("task_path")
                .and_then(Value::as_str)
                .is_some_and(|task_path| task_path == child_task_path)
        });
        let observed_classification = matching_entry
            .and_then(|entry| entry.get("classification"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let expected_classification = match child_status {
            Some("live_non_final") => Some("live_non_final"),
            Some("final_success") => Some("final_success_unintegrated"),
            Some("final_non_success") => Some("final_non_success"),
            None => Some("missing"),
            Some(_) => None,
        };
        let resume_status = resolve_report
            .get("resume_status")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let assessment = assess_native_multi_agent_gate(
            native.status.success(),
            &init_step,
            &write_closeout_step,
            &resolve_resume_step,
            &native_summary,
            expected_classification.as_deref(),
            observed_classification.as_deref(),
            resume_status.as_deref(),
        );

        let details = Some(json!({
            "sandbox_root": sandbox.path().display().to_string(),
            "repo_root": repo_root.display().to_string(),
            "codex_home_root": codex_home_root.display().to_string(),
            "native_stdout": full_output(&native.stdout),
            "native_stderr": full_output(&native.stderr),
            "native_events": native_events,
            "final_agent_message": final_agent_message,
            "reported_summary": reported_summary,
            "reported_summary_parse_error": reported_summary_parse_error,
            "native_summary": native_summary,
            "init_step": init_step,
            "write_closeout_step": write_closeout_step,
            "resolve_resume_step": resolve_resume_step,
            "resume_status": resume_status,
            "expected_classification": expected_classification,
            "observed_classification": observed_classification,
            "failure_classification": assessment.failure_classification,
            "missing_required_tools": assessment.missing_required_tools,
            "missing_evidence": assessment.missing_evidence,
            "observed_optional_delivery_tools": assessment.observed_optional_delivery_tools,
        }));

        Ok(if assessment.success {
            QualificationGate::pass(GATE, DESCRIPTION, assessment.message, details)
        } else {
            QualificationGate::fail(GATE, DESCRIPTION, assessment.message, details)
        })
    })();

    Ok(match result {
        Ok(gate) => gate,
        Err(error) => native_multi_agent_gate_failure(
            "The native child-agent probe could not complete, so qualification recorded a failing gate instead of aborting.",
            Some(json!({
                "error": format!("{error:#}"),
                "failure_classification": "native_probe_setup_failed",
            })),
        ),
    })
}

fn native_multi_agent_gate_failure(
    message: impl Into<String>,
    details: Option<Value>,
) -> QualificationGate {
    QualificationGate::fail(
        "native_multi_agent_resume_flow",
        "The trusted Codex build exposes native child-agent tools and Codex1 reconciles their live snapshot honestly on resume.",
        message,
        details,
    )
}

fn native_child_probe_closeout_payload(summary: &NativeMultiAgentSummary) -> Value {
    json!({
        "closeout_seq": 0,
        "mission_id": "native-reconcile",
        "phase": "execution",
        "activity": "native_child_reconciliation_probe",
        "verdict": "continue_required",
        "terminality": "actionable_non_terminal",
        "resume_mode": "continue",
        "next_phase": "execution",
        "next_action": "Reconcile expected child lanes before continuing execution.",
        "target": "spec:native_reconcile",
        "cycle_kind": "bounded_progress",
        "reason_code": "native_multi_agent_probe",
        "summary": "Qualification recorded a native child lane for resume reconciliation.",
        "continuation_prompt": "Reconcile expected child lanes before continuing execution.",
        "governing_revision": "native-child-probe",
        "active_child_task_paths": [summary.child_task_path],
        "artifact_fingerprints": {
            "qualification_probe": format!("native-child-probe:{}", summary.child_task_path),
        }
    })
}

fn qualification_binary() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("CODEX1_QUALIFY_EXECUTABLE") {
        return Ok(PathBuf::from(path));
    }

    std::env::current_exe().context("failed to resolve the current codex1 binary")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NativeMultiAgentSummary {
    #[serde(default, deserialize_with = "bool_or_false")]
    used_spawn_agent: bool,
    #[serde(default, deserialize_with = "bool_or_false")]
    used_send_message: bool,
    #[serde(default, deserialize_with = "bool_or_false")]
    used_assign_task: bool,
    #[serde(default, deserialize_with = "bool_or_false")]
    used_followup_task: bool,
    #[serde(default, deserialize_with = "bool_or_false")]
    used_list_agents: bool,
    #[serde(default, deserialize_with = "bool_or_false")]
    used_wait_agent: bool,
    #[serde(default, deserialize_with = "bool_or_false")]
    used_close_agent: bool,
    #[serde(default, deserialize_with = "string_or_empty")]
    child_task_path: String,
    #[serde(default, deserialize_with = "bool_or_false")]
    child_seen_before_wait: bool,
    #[serde(default)]
    child_status_before_wait: Value,
    #[serde(default, deserialize_with = "bool_or_false")]
    child_seen_after_wait: bool,
    #[serde(default)]
    child_status_after_wait: Value,
    #[serde(default, deserialize_with = "bool_or_false")]
    child_seen_after_close: bool,
    #[serde(default)]
    child_status_after_close: Value,
    #[serde(default, deserialize_with = "bool_or_false")]
    wait_summary_present: bool,
    #[serde(default)]
    list_agents_before_wait: Value,
    #[serde(default)]
    list_agents_after_wait: Value,
    #[serde(default)]
    list_agents_after_close: Value,
}

fn bool_or_false<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or(false))
}

fn string_or_empty<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, PartialEq, Eq)]
struct NativeMultiAgentGateAssessment {
    success: bool,
    message: String,
    failure_classification: Option<&'static str>,
    missing_required_tools: Vec<&'static str>,
    missing_evidence: Vec<&'static str>,
    observed_optional_delivery_tools: Vec<&'static str>,
}

fn assess_native_multi_agent_gate(
    native_success: bool,
    init_step: &SmokeStep,
    write_closeout_step: &SmokeStep,
    resolve_resume_step: &SmokeStep,
    native_summary: &NativeMultiAgentSummary,
    expected_classification: Option<&str>,
    observed_classification: Option<&str>,
    resume_status: Option<&str>,
) -> NativeMultiAgentGateAssessment {
    let mut missing_required_tools = Vec::new();
    if !native_summary.used_spawn_agent {
        missing_required_tools.push("spawn_agent");
    }
    if !native_summary.used_list_agents {
        missing_required_tools.push("list_agents");
    }
    if !native_summary.used_wait_agent {
        missing_required_tools.push("wait_agent");
    }
    if !native_summary.used_close_agent {
        missing_required_tools.push("close_agent");
    }

    let mut missing_evidence = Vec::new();
    if !native_summary.child_seen_before_wait || native_summary.list_agents_before_wait.is_null() {
        missing_evidence.push("pre_wait_child_snapshot");
    }
    if !native_summary.child_seen_after_wait || native_summary.list_agents_after_wait.is_null() {
        missing_evidence.push("post_wait_child_snapshot");
    }
    if !native_summary.wait_summary_present {
        missing_evidence.push("wait_mailbox_summary");
    }
    if observed_classification.is_none() {
        missing_evidence.push("child_reconciliation_entry");
    }

    let mut observed_optional_delivery_tools = Vec::new();
    if native_summary.used_send_message {
        observed_optional_delivery_tools.push("send_message");
    }
    if native_summary.used_assign_task {
        observed_optional_delivery_tools.push("assign_task");
    }
    if native_summary.used_followup_task {
        observed_optional_delivery_tools.push("followup_task");
    }

    if !native_success {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: "The native Codex child-agent probe did not complete successfully."
                .to_string(),
            failure_classification: Some("native_exec_failed"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if !init_step.success {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: "The native child-agent probe could not initialize the reconciliation mission scaffold.".to_string(),
            failure_classification: Some("mission_init_failed"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if !write_closeout_step.success {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: "The native child-agent probe failed before resume reconciliation because the synthetic child-lane closeout could not be appended.".to_string(),
            failure_classification: Some("closeout_write_failed"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if !resolve_resume_step.success {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: "The native child-agent probe did not complete the resume-resolution step needed for lane reconciliation.".to_string(),
            failure_classification: Some("resolve_resume_failed"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if !missing_required_tools.is_empty() {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: format!(
                "The trusted build did not surface or successfully exercise the live child-inspection tool set required for this gate: {}.",
                missing_required_tools.join(", ")
            ),
            failure_classification: Some("required_tool_surface_gap"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if !missing_evidence.is_empty() {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: format!(
                "The native child-agent probe did not produce the live child evidence required for honest resume reconciliation: {}.",
                missing_evidence.join(", ")
            ),
            failure_classification: Some("live_snapshot_gap"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if observed_classification != expected_classification {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: format!(
                "Codex1 reconciled the native child lane differently than the probe evidence implied (expected {:?}, observed {:?}).",
                expected_classification, observed_classification
            ),
            failure_classification: Some("reconciliation_mismatch"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if resume_status == Some("complete") {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: "The resume report incorrectly surfaced terminal completion for a mission that should remain non-terminal during child-lane reconciliation.".to_string(),
            failure_classification: Some("false_terminality"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    if native_summary.child_seen_after_close {
        return NativeMultiAgentGateAssessment {
            success: false,
            message: "The native child lane remained visible after close, so the cleanup edge of the probe was not proven.".to_string(),
            failure_classification: Some("close_agent_cleanup_gap"),
            missing_required_tools,
            missing_evidence,
            observed_optional_delivery_tools,
        };
    }

    NativeMultiAgentGateAssessment {
        success: true,
        message: "A live native child lane exercised spawn, inspect, wait, and close behavior, then Codex1 reconciled the live snapshot without false completion.".to_string(),
        failure_classification: None,
        missing_required_tools,
        missing_evidence,
        observed_optional_delivery_tools,
    }
}

enum NativeCodexInvocation {
    Exec,
    ExecResume { thread_id: String },
}

fn seed_native_codex_repo(repo_root: &Path) -> Result<()> {
    fs::create_dir_all(repo_root).with_context(|| format!("create {}", repo_root.display()))?;
    fs::create_dir_all(repo_root.join(".codex"))
        .with_context(|| format!("create {}", repo_root.join(".codex").display()))?;
    fs::write(
        repo_root.join("README.md"),
        "# Native Qualification Sandbox\n\nTemporary repo for native Codex qualification.\n",
    )
    .context("failed to seed native qualification repo")?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo_root)
        .status()
        .context("failed to initialize native qualification git repo")?;
    if !status.success() {
        bail!("git init failed for {}", repo_root.display());
    }
    Ok(())
}

fn run_native_codex_prompt(
    repo_root: &Path,
    codex_home_root: &Path,
    invocation: NativeCodexInvocation,
    prompt: &str,
) -> Result<Output> {
    let mut command = Command::new("codex");
    match invocation {
        NativeCodexInvocation::Exec => {
            command.args(["exec", "--json", "-"]);
        }
        NativeCodexInvocation::ExecResume { thread_id } => {
            command.args(["exec", "resume", "--json", &thread_id, "-"]);
        }
    }

    let mut child = command
        .current_dir(repo_root)
        .env("CODEX_HOME", codex_home_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn native codex qualification process")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .context("failed to write native codex prompt")?;
    }

    child
        .wait_with_output()
        .context("failed to collect native codex output")
}

fn seed_native_codex_home(repo_root: &Path, codex_home_root: &Path) -> Result<()> {
    let source_codex_home = codex_home()?;
    seed_native_codex_home_from(&source_codex_home, repo_root, codex_home_root)
}

fn seed_native_codex_home_from(
    source_codex_home: &Path,
    repo_root: &Path,
    codex_home_root: &Path,
) -> Result<()> {
    fs::create_dir_all(codex_home_root)
        .with_context(|| format!("create {}", codex_home_root.display()))?;

    copy_directory_contents(source_codex_home, codex_home_root)?;

    if !codex_home_root.join("auth.json").is_file() {
        bail!(
            "native Codex qualification requires {} so the sandbox can authenticate",
            source_codex_home.join("auth.json").display()
        );
    }

    let canonical_repo_root = fs::canonicalize(repo_root)
        .with_context(|| format!("failed to canonicalize {}", repo_root.display()))?;
    let existing_config = read_optional_string(&codex_home_root.join("config.toml"))?;
    fs::write(
        codex_home_root.join("config.toml"),
        append_trusted_project_entry(existing_config.as_deref(), &canonical_repo_root),
    )
    .with_context(|| format!("seed {}", codex_home_root.join("config.toml").display()))?;

    Ok(())
}

fn copy_directory_contents(source: &Path, target: &Path) -> Result<()> {
    if !source.is_dir() {
        bail!("{} is not a directory", source.display());
    }

    for entry in fs::read_dir(source).with_context(|| format!("read {}", source.display()))? {
        let entry = entry.with_context(|| format!("read entry under {}", source.display()))?;
        let entry_path = entry.path();
        let destination = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&entry_path)
            .with_context(|| format!("stat {}", entry_path.display()))?;
        if metadata.file_type().is_dir() {
            fs::create_dir_all(&destination)
                .with_context(|| format!("create {}", destination.display()))?;
            copy_directory_contents(&entry_path, &destination)?;
        } else if metadata.file_type().is_symlink() {
            let link_target = fs::read_link(&entry_path)
                .with_context(|| format!("read link {}", entry_path.display()))?;
            create_symlink(&link_target, &destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create {}", parent.display()))?;
            }
            fs::copy(&entry_path, &destination).with_context(|| {
                format!("copy {} -> {}", entry_path.display(), destination.display())
            })?;
        }
    }

    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::os::unix::fs::symlink(target, link)
        .with_context(|| format!("symlink {} -> {}", link.display(), target.display()))
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<()> {
    Ok(())
}

fn append_trusted_project_entry(existing: Option<&str>, repo_root: &Path) -> String {
    let marker = format!("[projects.\"{}\"]", repo_root.display());
    let existing = existing.unwrap_or_default().trim_end();
    if existing.contains(&marker) {
        let mut output = existing.to_string();
        if !output.ends_with('\n') {
            output.push('\n');
        }
        return output;
    }

    let mut output = String::new();
    if existing.is_empty() {
        output.push_str("# qualification native sandbox\ntelemetry = false\n");
    } else {
        output.push_str(existing);
        output.push('\n');
    }

    if !output.ends_with("\n\n") {
        output.push('\n');
    }
    output.push_str(&format!(
        "[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
        repo_root.display()
    ));
    output
}

fn parse_thread_id_from_jsonl(bytes: &[u8]) -> Result<Option<String>> {
    for value in parse_jsonl_events(bytes)? {
        if value.get("type").and_then(Value::as_str) == Some("thread.started") {
            return Ok(value
                .get("thread_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned));
        }
    }

    Ok(None)
}

fn last_agent_message_text(bytes: &[u8]) -> Result<Option<String>> {
    let mut last = None;
    for value in parse_jsonl_events(bytes)? {
        if value.get("type").and_then(Value::as_str) == Some("item.completed")
            && value
                .get("item")
                .and_then(|item| item.get("type"))
                .and_then(Value::as_str)
                == Some("agent_message")
        {
            last = value
                .get("item")
                .and_then(|item| item.get("text"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
    }

    Ok(last)
}

fn parse_native_multi_agent_summary(bytes: &[u8]) -> Result<NativeMultiAgentSummary> {
    let text = last_agent_message_text(bytes)?
        .context("JSONL stream did not contain a final agent message")?;
    parse_native_multi_agent_summary_text(&text)
}

fn parse_native_multi_agent_summary_text(text: &str) -> Result<NativeMultiAgentSummary> {
    serde_json::from_str(text).context("failed to parse final agent message JSON")
}

fn parse_jsonl_events(bytes: &[u8]) -> Result<Vec<Value>> {
    let mut events = Vec::new();
    for line in String::from_utf8_lossy(bytes).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            events.push(value);
        }
    }
    Ok(events)
}

fn summarize_native_multi_agent_events(events: &[Value]) -> NativeMultiAgentSummary {
    let mut summary = NativeMultiAgentSummary::default();
    let mut fallback_child_path = None::<String>;
    let mut completed_waits = 0usize;
    let mut completed_closes = 0usize;

    for item in completed_collab_tool_items(events) {
        let Some(tool) = item.get("tool").and_then(Value::as_str) else {
            continue;
        };

        match tool {
            "spawn_agent" => {
                summary.used_spawn_agent = true;
                if fallback_child_path.is_none() {
                    fallback_child_path = extract_first_receiver_thread_id(item)
                        .or_else(|| extract_single_status_key(item.get("agents_states")));
                }
            }
            "send_message" => {
                summary.used_send_message = true;
            }
            "assign_task" => {
                summary.used_assign_task = true;
            }
            "followup_task" => {
                summary.used_followup_task = true;
            }
            "list_agents" => {
                summary.used_list_agents = true;
                if let Some(snapshot) = extract_list_agents_snapshot(item) {
                    if summary.child_task_path.is_empty() {
                        if let Some(path) =
                            find_agent_path_by_task_name(&snapshot, QUALIFY_CHILD_TASK_NAME)
                        {
                            summary.child_task_path = path;
                        }
                    }

                    if summary.list_agents_before_wait.is_null() && completed_waits == 0 {
                        summary.list_agents_before_wait = snapshot;
                    } else if summary.list_agents_after_wait.is_null() && completed_closes == 0 {
                        summary.list_agents_after_wait = snapshot;
                    } else if summary.list_agents_after_close.is_null() {
                        summary.list_agents_after_close = snapshot;
                    }
                }
            }
            "wait" | "wait_agent" => {
                summary.used_wait_agent = true;
                summary.wait_summary_present = true;
                completed_waits += 1;
                if summary.child_task_path.is_empty()
                    && let Some(path) = extract_single_status_key(item.get("agents_states"))
                {
                    summary.child_task_path = path;
                }
                if summary.child_task_path.is_empty()
                    && let Some(path) = fallback_child_path.clone()
                {
                    summary.child_task_path = path;
                }
                if !summary.child_task_path.is_empty()
                    && let Some(status) = item.get("agents_states").and_then(|snapshot| {
                        status_value_for_task_path(snapshot, &summary.child_task_path)
                    })
                {
                    summary.child_seen_after_wait = true;
                    summary.child_status_after_wait = status;
                }
            }
            "close_agent" => {
                summary.used_close_agent = true;
                completed_closes += 1;
                if summary.child_task_path.is_empty()
                    && let Some(path) = extract_single_status_key(item.get("agents_states"))
                {
                    summary.child_task_path = path;
                }
                if summary.child_task_path.is_empty()
                    && let Some(path) = fallback_child_path.clone()
                {
                    summary.child_task_path = path;
                }
                if !summary.child_task_path.is_empty()
                    && let Some(status) = item.get("agents_states").and_then(|snapshot| {
                        status_value_for_task_path(snapshot, &summary.child_task_path)
                    })
                {
                    summary.child_seen_after_close = true;
                    summary.child_status_after_close = status;
                }
            }
            _ => {}
        }
    }

    if summary.child_task_path.is_empty() {
        if let Some(path) =
            find_agent_path_by_task_name(&summary.list_agents_before_wait, QUALIFY_CHILD_TASK_NAME)
                .or_else(|| {
                    find_agent_path_by_task_name(
                        &summary.list_agents_after_wait,
                        QUALIFY_CHILD_TASK_NAME,
                    )
                })
                .or_else(|| {
                    find_agent_path_by_task_name(
                        &summary.list_agents_after_close,
                        QUALIFY_CHILD_TASK_NAME,
                    )
                })
                .or(fallback_child_path)
        {
            summary.child_task_path = path;
        }
    }

    if !summary.child_task_path.is_empty() {
        if let Some(status) =
            status_value_for_task_path(&summary.list_agents_before_wait, &summary.child_task_path)
        {
            summary.child_seen_before_wait = true;
            summary.child_status_before_wait = status;
        }
        if let Some(status) =
            status_value_for_task_path(&summary.list_agents_after_wait, &summary.child_task_path)
        {
            summary.child_seen_after_wait = true;
            summary.child_status_after_wait = status;
        }
        if let Some(status) =
            status_value_for_task_path(&summary.list_agents_after_close, &summary.child_task_path)
        {
            summary.child_seen_after_close = true;
            summary.child_status_after_close = status;
        }
    }

    summary
}

fn completed_collab_tool_items(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter_map(|value| {
            let item = value.get("item")?;
            let is_completed = value.get("type").and_then(Value::as_str) == Some("item.completed");
            let is_collab_tool =
                item.get("type").and_then(Value::as_str) == Some("collab_tool_call");
            (is_completed && is_collab_tool).then_some(item)
        })
        .collect()
}

fn extract_first_receiver_thread_id(item: &Value) -> Option<String> {
    item.get("receiver_thread_ids")
        .and_then(Value::as_array)
        .and_then(|receivers| receivers.first())
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn extract_single_status_key(snapshot: Option<&Value>) -> Option<String> {
    snapshot
        .and_then(Value::as_object)
        .filter(|map| map.len() == 1)
        .and_then(|map| map.keys().next().cloned())
}

fn extract_list_agents_snapshot(item: &Value) -> Option<Value> {
    match item {
        Value::Array(items) => items.iter().find_map(extract_list_agents_snapshot),
        Value::Object(map) => {
            if let Some(agents) = map.get("agents").and_then(Value::as_array) {
                return Some(Value::Array(agents.clone()));
            }
            map.values().find_map(extract_list_agents_snapshot)
        }
        _ => None,
    }
}

fn find_agent_path_by_task_name(snapshot: &Value, task_name: &str) -> Option<String> {
    match snapshot {
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_agent_path_by_task_name(item, task_name)),
        Value::Object(map) => {
            if let Some(path) = map
                .get("task_path")
                .or_else(|| map.get("agent_name"))
                .and_then(Value::as_str)
                .filter(|path| path_matches_task_name(path, task_name))
            {
                return Some(path.to_string());
            }
            map.values()
                .find_map(|value| find_agent_path_by_task_name(value, task_name))
        }
        _ => None,
    }
}

fn path_matches_task_name(path: &str, task_name: &str) -> bool {
    path == task_name || path.ends_with(&format!("/{task_name}"))
}

fn native_child_status_for_resume(summary: &NativeMultiAgentSummary) -> Option<&'static str> {
    if summary.child_task_path.is_empty() {
        return None;
    }

    let status = status_for_task_path(&summary.list_agents_after_wait, &summary.child_task_path)
        .or_else(|| native_child_status_name(&summary.child_status_after_wait));

    match status.as_deref() {
        Some("pending_init") | Some("running") | Some("interrupted") => Some("live_non_final"),
        Some("completed") => Some("final_success"),
        Some("errored") | Some("shutdown") | Some("not_found") => Some("final_non_success"),
        _ => None,
    }
}

fn status_value_for_task_path(snapshot: &Value, task_path: &str) -> Option<Value> {
    match snapshot {
        Value::Array(items) => items
            .iter()
            .find_map(|item| status_value_for_task_path(item, task_path)),
        Value::Object(map) => {
            if map
                .get("task_path")
                .or_else(|| map.get("agent_name"))
                .and_then(Value::as_str)
                == Some(task_path)
            {
                return map
                    .get("status")
                    .cloned()
                    .or_else(|| map.get("agent_status").cloned());
            }
            if let Some(value) = map.get(task_path) {
                if let Some(status) = value
                    .get("status")
                    .cloned()
                    .or_else(|| value.get("agent_status").cloned())
                {
                    return Some(status);
                }
            }
            map.values()
                .find_map(|value| status_value_for_task_path(value, task_path))
        }
        _ => None,
    }
}

fn status_for_task_path(snapshot: &Value, task_path: &str) -> Option<String> {
    status_value_for_task_path(snapshot, task_path)
        .and_then(|value| native_child_status_name(&value))
}

fn native_child_status_name(value: &Value) -> Option<String> {
    if let Some(status) = value.as_str() {
        return Some(status.to_string());
    }

    let object = value.as_object()?;
    for key in [
        "completed",
        "errored",
        "shutdown",
        "not_found",
        "running",
        "pending_init",
        "interrupted",
    ] {
        if object.contains_key(key) {
            return Some(key.to_string());
        }
    }

    None
}

fn run_smoke_step(
    step: &'static str,
    binary: &Path,
    repo_root: &Path,
    home_root: &Path,
    args: &[&str],
) -> Result<SmokeStep> {
    let output = Command::new(binary)
        .args(args)
        .current_dir(repo_root)
        .env("HOME", home_root)
        .env("XDG_CONFIG_HOME", home_root.join(".config"))
        .env("CODEX_HOME", home_root.join(".codex"))
        .env("CODEX1_QUALIFY_EXECUTABLE", binary)
        .output()
        .with_context(|| format!("failed to run `{}`", args.join(" ")))?;

    Ok(SmokeStep {
        step,
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: trim_output(&output.stdout),
        stderr: trim_output(&output.stderr),
    })
}

fn run_smoke_step_strings(
    step: &'static str,
    binary: &Path,
    repo_root: &Path,
    home_root: &Path,
    args: &[String],
) -> Result<SmokeStep> {
    let output = Command::new(binary)
        .args(args)
        .current_dir(repo_root)
        .env("HOME", home_root)
        .env("XDG_CONFIG_HOME", home_root.join(".config"))
        .env("CODEX_HOME", home_root.join(".codex"))
        .env("CODEX1_QUALIFY_EXECUTABLE", binary)
        .output()
        .with_context(|| format!("failed to run `{}`", args.join(" ")))?;

    Ok(SmokeStep {
        step,
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: trim_output(&output.stdout),
        stderr: trim_output(&output.stderr),
    })
}

fn parse_step_json(step: &SmokeStep) -> Option<Value> {
    serde_json::from_str(&step.stdout).ok()
}

fn smoke_step_backup_id(step: &SmokeStep) -> Option<String> {
    parse_step_json(step)
        .and_then(|value| {
            value
                .get("backup_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .filter(|backup_id| !backup_id.trim().is_empty())
}

fn doctor_report_support_ready(step: &SmokeStep) -> bool {
    parse_step_json(step)
        .map(|value| {
            let supported = value
                .get("supported")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let qualification_only_block = value
                .get("findings")
                .and_then(Value::as_array)
                .is_some_and(|findings| {
                    findings.iter().all(|finding| {
                        let check = finding.get("check").and_then(Value::as_str);
                        let status = finding.get("status").and_then(Value::as_str);
                        status != Some("fail") || check == Some("qualification")
                    })
                });
            supported || qualification_only_block
        })
        .unwrap_or(false)
}

fn seed_helper_sandbox(repo_root: &Path, home_root: &Path) -> Result<()> {
    let baseline_home = home_root.join(".codex");
    fs::create_dir_all(repo_root).with_context(|| format!("create {}", repo_root.display()))?;
    fs::create_dir_all(&baseline_home)
        .with_context(|| format!("create {}", baseline_home.display()))?;
    let canonical_repo_root = fs::canonicalize(repo_root)
        .with_context(|| format!("failed to canonicalize {}", repo_root.display()))?;
    fs::write(
        repo_root.join("README.md"),
        "# Qualification Sandbox\n\nTemporary repo for codex1 qualification smoke flows.\n",
    )
    .with_context(|| format!("seed {}", repo_root.join("README.md").display()))?;
    fs::write(
        baseline_home.join("config.toml"),
        format!(
            "# user baseline\ntelemetry = false\n\n[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
            canonical_repo_root.display()
        ),
    )
    .with_context(|| format!("seed {}", baseline_home.join("config.toml").display()))?;
    Ok(())
}

fn run_json_smoke_step(
    step: &'static str,
    binary: &Path,
    repo_root: &Path,
    args: &[&str],
    input: &Value,
) -> Result<SmokeStep> {
    run_json_smoke_step_inner(step, binary, repo_root, args, input, &[], true)
}

fn run_json_smoke_step_with_env(
    step: &'static str,
    binary: &Path,
    repo_root: &Path,
    args: &[&str],
    input: &Value,
    extra_env: &[(&str, &str)],
) -> Result<SmokeStep> {
    run_json_smoke_step_inner(step, binary, repo_root, args, input, extra_env, true)
}

fn run_json_smoke_step_without_parent_authority(
    step: &'static str,
    binary: &Path,
    repo_root: &Path,
    args: &[&str],
    input: &Value,
) -> Result<SmokeStep> {
    run_json_smoke_step_inner(step, binary, repo_root, args, input, &[], false)
}

fn run_json_smoke_step_inner(
    step: &'static str,
    binary: &Path,
    repo_root: &Path,
    args: &[&str],
    input: &Value,
    extra_env: &[(&str, &str)],
    include_parent_authority_marker: bool,
) -> Result<SmokeStep> {
    let home_root = repo_root
        .parent()
        .map(|parent| parent.join("home"))
        .unwrap_or_else(|| repo_root.join(".qualify-home"));
    fs::create_dir_all(home_root.join(".config"))
        .with_context(|| format!("failed to create {}", home_root.join(".config").display()))?;
    fs::create_dir_all(home_root.join(".codex"))
        .with_context(|| format!("failed to create {}", home_root.join(".codex").display()))?;
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(repo_root)
        .env("HOME", &home_root)
        .env("XDG_CONFIG_HOME", home_root.join(".config"))
        .env("CODEX_HOME", home_root.join(".codex"))
        .env("CODEX1_QUALIFY_EXECUTABLE", binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if include_parent_authority_marker
        && let Some(token) = qualification_parent_authority_token(repo_root)
    {
        command.env("CODEX1_PARENT_LOOP_AUTHORITY_TOKEN", token);
    }
    if include_parent_authority_marker && args.contains(&"begin-loop-lease") {
        command.env("CODEX1_PARENT_LOOP_BEGIN", "1");
    }
    command.envs(extra_env.iter().copied());
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to run `{}`", args.join(" ")))?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::to_vec(input).context("failed to encode JSON smoke input")?;
        stdin
            .write_all(&payload)
            .context("failed to write smoke-step stdin")?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("failed to collect output for `{}`", args.join(" ")))?;

    Ok(SmokeStep {
        step,
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: trim_output(&output.stdout),
        stderr: trim_output(&output.stderr),
    })
}

fn qualification_parent_authority_token_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".ralph/qualification-parent-authority-token")
}

fn qualification_parent_authority_token(repo_root: &Path) -> Option<String> {
    fs::read_to_string(qualification_parent_authority_token_path(repo_root))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn capture_review_truth_snapshot_payload(
    binary: &Path,
    repo_root: &Path,
    mission_id: &str,
    bundle_id: &str,
) -> Result<Value> {
    let lease_step = run_json_smoke_step(
        "begin_parent_review_loop_authority",
        binary,
        repo_root,
        &[
            "internal",
            "begin-loop-lease",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "qualification-parent-review-loop",
            "reason": "Qualification helper needs parent review authority."
        }),
    )?;
    if !lease_step.success {
        anyhow::bail!(
            "failed to begin parent review loop authority: {}",
            lease_step.stderr
        );
    }
    let lease = serde_json::from_str::<Value>(&lease_step.stdout)
        .context("failed to parse parent review loop authority lease")?;
    let token = lease
        .get("parent_authority_token")
        .and_then(Value::as_str)
        .context("parent review loop authority token missing")?;
    if let Some(parent) = qualification_parent_authority_token_path(repo_root).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(qualification_parent_authority_token_path(repo_root), token)
        .context("failed to persist qualification parent authority token marker")?;

    let step = run_json_smoke_step(
        "capture_review_truth_snapshot",
        binary,
        repo_root,
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_id,
            "--json",
        ],
        &json!({}),
    )?;
    if !step.success {
        anyhow::bail!("failed to capture review truth snapshot: {}", step.stderr);
    }
    serde_json::from_str::<Value>(&step.stdout)
        .with_context(|| format!("failed to parse review truth snapshot for {bundle_id}"))
}

fn record_none_reviewer_output_ref(
    binary: &Path,
    repo_root: &Path,
    mission_id: &str,
    bundle_id: &str,
    reviewer_id: &str,
    steps: &mut Vec<SmokeStep>,
) -> Result<String> {
    steps.push(run_json_smoke_step(
        "capture_review_evidence_snapshot",
        binary,
        repo_root,
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_id,
            "--json",
        ],
        &json!({}),
    )?);
    let step = steps.last().expect("capture review evidence step");
    if !step.success {
        anyhow::bail!(
            "failed to capture review evidence snapshot: {}",
            step.stderr
        );
    }
    record_none_reviewer_output_ref_after_snapshot(
        binary,
        repo_root,
        mission_id,
        bundle_id,
        reviewer_id,
        steps,
    )
}

fn record_none_reviewer_output_ref_after_snapshot(
    binary: &Path,
    repo_root: &Path,
    mission_id: &str,
    bundle_id: &str,
    reviewer_id: &str,
    steps: &mut Vec<SmokeStep>,
) -> Result<String> {
    steps.push(run_json_smoke_step(
        "record_reviewer_output",
        binary,
        repo_root,
        &[
            "internal",
            "record-reviewer-output",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer_id": reviewer_id,
            "output_kind": "none",
            "findings": []
        }),
    )?);
    let step = steps.last().expect("reviewer output step");
    if !step.success {
        anyhow::bail!("failed to record reviewer output: {}", step.stderr);
    }
    parse_required_id(step, "evidence_ref")
}

fn run_stop_hook_probe(mut child: Child, input: &Value) -> Result<Output> {
    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::to_vec(input).context("failed to encode stop-hook input")?;
        stdin
            .write_all(&payload)
            .context("failed to write stop-hook stdin")?;
    }

    child
        .wait_with_output()
        .context("failed to collect stop-hook output")
}

fn trim_output(bytes: &[u8]) -> String {
    let output = String::from_utf8_lossy(bytes).trim().to_string();
    const LIMIT: usize = 8_000;
    if output.len() <= LIMIT {
        output
    } else {
        let truncated: String = output.chars().take(LIMIT).collect();
        format!("{truncated}...[truncated]")
    }
}

fn full_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn snapshot_tree(root: &Path) -> Result<BTreeMap<String, SnapshotEntry>> {
    let mut entries = BTreeMap::new();
    if !root.exists() {
        return Ok(entries);
    }

    snapshot_tree_recursive(root, root, &mut entries)?;
    Ok(entries)
}

fn snapshot_tree_recursive(
    base: &Path,
    current: &Path,
    entries: &mut BTreeMap<String, SnapshotEntry>,
) -> Result<()> {
    for entry in
        fs::read_dir(current).with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", current.display()))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;
        let relative = path
            .strip_prefix(base)
            .expect("snapshot path should stay under the snapshot root")
            .to_string_lossy()
            .to_string();

        if metadata.file_type().is_dir() {
            entries.insert(relative, SnapshotEntry::Directory);
            snapshot_tree_recursive(base, &path, entries)?;
            continue;
        }

        let snapshot_entry = if metadata.file_type().is_symlink() {
            SnapshotEntry::Symlink(
                fs::read_link(&path)
                    .with_context(|| format!("failed to read symlink {}", path.display()))?
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            SnapshotEntry::File(
                fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
            )
        };

        entries.insert(relative, snapshot_entry);
    }

    Ok(())
}

fn snapshot_diff(
    before: &BTreeMap<String, SnapshotEntry>,
    after: &BTreeMap<String, SnapshotEntry>,
) -> Vec<Value> {
    let mut diffs = Vec::new();
    let all_paths: BTreeSet<_> = before.keys().chain(after.keys()).cloned().collect();
    for path in all_paths {
        match (before.get(&path), after.get(&path)) {
            (Some(left), Some(right)) if left == right => {}
            (Some(_), Some(_)) => diffs.push(json!({ "path": path, "change": "modified" })),
            (Some(_), None) => diffs.push(json!({ "path": path, "change": "removed" })),
            (None, Some(_)) => diffs.push(json!({ "path": path, "change": "added" })),
            (None, None) => {}
        }
    }
    diffs
}

fn self_hosting_gate(repo_root: &Path, enabled: bool) -> QualificationGate {
    if !enabled {
        return QualificationGate::skipped(
            "self_hosting_source_repo",
            "Self-hosting source repo qualification is explicitly requested and recorded.",
            "Self-hosting verification was disabled for this run.",
            None,
        );
    }

    let cargo_manifest = repo_root.join("Cargo.toml");
    let prd = repo_root.join(PRD_MARKER);
    if cargo_manifest.exists() && prd.exists() {
        let expected = [
            repo_root.join("crates/codex1/Cargo.toml"),
            repo_root.join("crates/codex1-core/Cargo.toml"),
            repo_root.join(".codex/config.toml"),
            repo_root.join(".codex/hooks.json"),
        ];
        let missing: Vec<String> = expected
            .iter()
            .filter(|path| !path.exists())
            .map(|path| path.display().to_string())
            .collect();
        let agents_path = repo_root.join("AGENTS.md");
        let agents_raw = match read_optional_string(&agents_path) {
            Ok(raw) => raw,
            Err(error) => {
                return QualificationGate::fail(
                    "self_hosting_source_repo",
                    "Self-hosting source repo qualification is explicitly requested and recorded.",
                    "The source workspace AGENTS.md could not be read for self-hosting verification.",
                    Some(json!({
                        "path": agents_path.display().to_string(),
                        "error": error.to_string(),
                    })),
                );
            }
        };
        let agents_inspection = inspect_agents_scaffold_details(agents_raw.as_deref());
        let skill_inspection = match inspect_skill_surface(repo_root) {
            Ok(inspection) => inspection,
            Err(error) => {
                return QualificationGate::fail(
                    "self_hosting_source_repo",
                    "Self-hosting source repo qualification is explicitly requested and recorded.",
                    "The source workspace skill surface could not be inspected for self-hosting verification.",
                    Some(json!({
                        "repo_root": repo_root.display().to_string(),
                        "error": error.to_string(),
                    })),
                );
            }
        };

        if missing.is_empty()
            && agents_inspection.status == AgentsScaffoldStatus::Present
            && agents_inspection.command_status == AgentsCommandStatus::Concrete
            && skill_inspection.status == SkillSurfaceStatus::ValidExisting
        {
            QualificationGate::pass(
                "self_hosting_source_repo",
                "Self-hosting source repo qualification is explicitly requested and recorded.",
                "Source-repo markers and managed Codex1 surfaces are present and match the enforced support surface in the source workspace.",
                Some(json!({
                    "cargo_manifest": cargo_manifest.display().to_string(),
                    "prd": prd.display().to_string(),
                    "agents_state": agents_inspection.status,
                    "agents_command_status": agents_inspection.command_status,
                    "skill_surface_status": skill_inspection.status,
                    "managed_surfaces": [
                        repo_root.join(".codex/config.toml").display().to_string(),
                        repo_root.join(".codex/hooks.json").display().to_string(),
                        repo_root.join("AGENTS.md").display().to_string(),
                        skill_inspection.discovery_root.display().to_string(),
                    ],
                })),
            )
        } else {
            QualificationGate::fail(
                "self_hosting_source_repo",
                "Self-hosting source repo qualification is explicitly requested and recorded.",
                "The source workspace is missing, placeholder-filled, or has drifted one or more required managed Codex1 surfaces.",
                Some(json!({
                    "missing_paths": missing,
                    "agents_state": agents_inspection.status,
                    "agents_command_status": agents_inspection.command_status,
                    "skill_surface_status": skill_inspection.status,
                    "missing_required_public_skills": skill_inspection.missing_required_public_skills,
                    "drifted_managed_skill_files": skill_inspection.drifted_managed_files,
                    "suggested_invocation": format!(
                        "cargo run -p codex1 -- init --repo-root {} --json",
                        repo_root.display()
                    ),
                })),
            )
        }
    } else {
        QualificationGate::skipped(
            "self_hosting_source_repo",
            "Self-hosting source repo qualification is explicitly requested and recorded.",
            "Target repo does not look like the codex1 source workspace, so the self-hosting gate was skipped.",
            Some(json!({
                "expected_markers": [
                    cargo_manifest.display().to_string(),
                    prd.display().to_string(),
                ],
            })),
        )
    }
}

fn manual_autopilot_parity_gate() -> Result<QualificationGate> {
    let binary = qualification_binary()?;
    let sandbox = tempfile::tempdir().context("failed to create parity sandbox")?;
    let manual_repo = sandbox.path().join("manual-repo");
    let autopilot_repo = sandbox.path().join("autopilot-repo");
    fs::create_dir_all(&manual_repo).context("failed to create manual parity repo")?;
    fs::create_dir_all(&autopilot_repo).context("failed to create autopilot parity repo")?;

    let manual = run_manual_parity_flow(&binary, &manual_repo)?;
    let autopilot = run_autopilot_parity_flow(&binary, &autopilot_repo)?;
    let success = manual.steps.iter().all(|step| step.success)
        && autopilot.steps.iter().all(|step| step.success)
        && manual.summary.is_some()
        && autopilot.summary.is_some()
        && manual.summary == autopilot.summary;

    let details = Some(json!({
        "sandbox_root": sandbox.path().display().to_string(),
        "manual_repo_root": manual_repo.display().to_string(),
        "autopilot_repo_root": autopilot_repo.display().to_string(),
        "manual_flow": manual,
        "autopilot_flow": autopilot,
    }));

    Ok(if success {
        QualificationGate::pass(
            INTERNAL_CONTRACT_PARITY_GATE,
            "An explicit manual backend sequence and an autopilot-style backend composition converge to the same durable artifact state and gate outcomes for the same mission truth.",
            "The manual path and an autopilot-style composition over the same internal contracts converged to the same validated durable artifact summary in isolated repos.",
            details,
        )
    } else {
        QualificationGate::fail(
            INTERNAL_CONTRACT_PARITY_GATE,
            "An explicit manual backend sequence and an autopilot-style backend composition converge to the same durable artifact state and gate outcomes for the same mission truth.",
            "The manual path and the autopilot-style composition did not converge to the same durable artifact summary.",
            details,
        )
    })
}

fn run_manual_parity_flow(binary: &Path, repo_root: &Path) -> Result<ParityFlowOutcome> {
    let mut steps = Vec::new();

    steps.push(run_json_smoke_step(
        "manual_init_mission",
        binary,
        repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &parity_init_payload(),
    )?);
    steps.push(run_json_smoke_step(
        "manual_write_blueprint",
        binary,
        repo_root,
        &[
            "internal",
            "materialize-plan",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &parity_blueprint_payload(),
    )?);
    steps.push(run_json_smoke_step(
        "manual_compile_execution_package",
        binary,
        repo_root,
        &[
            "internal",
            "compile-execution-package",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &parity_execution_package_payload(),
    )?);

    let mut validate_success = false;
    if steps.iter().all(|step| step.success) {
        let package_id = parse_required_id(steps.last().expect("package step"), "package_id")
            .context("manual parity package output missing package_id")?;
        steps.push(run_json_smoke_step(
            "manual_derive_writer_packet",
            binary,
            repo_root,
            &[
                "internal",
                "derive-writer-packet",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &parity_writer_packet_payload(&package_id),
        )?);
        steps.push(run_json_smoke_step(
            "manual_compile_spec_review_bundle",
            binary,
            repo_root,
            &[
                "internal",
                "compile-review-bundle",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &parity_spec_review_bundle_payload(&package_id),
        )?);
    }

    if steps.iter().all(|step| step.success) {
        let bundle_id = parse_required_id(steps.last().expect("bundle step"), "bundle_id")
            .context("manual parity spec-review output missing bundle_id")?;
        let review_truth_snapshot = capture_review_truth_snapshot_payload(
            binary,
            repo_root,
            PARITY_MISSION_ID,
            &bundle_id,
        )?;
        let reviewer_output_ref = record_none_reviewer_output_ref(
            binary,
            repo_root,
            PARITY_MISSION_ID,
            &bundle_id,
            "parity-spec-review",
            &mut steps,
        )?;
        let code_reviewer_output_ref = record_none_reviewer_output_ref_after_snapshot(
            binary,
            repo_root,
            PARITY_MISSION_ID,
            &bundle_id,
            "parity-code-review",
            &mut steps,
        )?;
        steps.push(run_json_smoke_step(
            "manual_record_spec_review_result",
            binary,
            repo_root,
            &[
                "internal",
                "record-review-outcome",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &parity_spec_review_result_payload(
                &bundle_id,
                &[reviewer_output_ref, code_reviewer_output_ref],
                review_truth_snapshot,
            ),
        )?);
    }

    if steps.iter().all(|step| step.success) {
        let package_id = load_single_execution_package_id(&MissionPaths::try_new(
            repo_root,
            PARITY_MISSION_ID,
        )?)?;
        steps.push(run_json_smoke_step(
            "manual_compile_mission_close_bundle",
            binary,
            repo_root,
            &[
                "internal",
                "compile-review-bundle",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &parity_mission_close_bundle_payload(repo_root, &package_id)?,
        )?);
    }

    if steps.iter().all(|step| step.success) {
        let bundle_id = parse_required_id(steps.last().expect("mission close bundle"), "bundle_id")
            .context("manual parity mission-close output missing bundle_id")?;
        let review_truth_snapshot = capture_review_truth_snapshot_payload(
            binary,
            repo_root,
            PARITY_MISSION_ID,
            &bundle_id,
        )?;
        let code_reviewer_output_ref = record_none_reviewer_output_ref(
            binary,
            repo_root,
            PARITY_MISSION_ID,
            &bundle_id,
            "parity-mission-close-code",
            &mut steps,
        )?;
        let spec_reviewer_output_ref = record_none_reviewer_output_ref_after_snapshot(
            binary,
            repo_root,
            PARITY_MISSION_ID,
            &bundle_id,
            "parity-mission-close-spec",
            &mut steps,
        )?;
        steps.push(run_json_smoke_step(
            "manual_record_mission_close_review",
            binary,
            repo_root,
            &[
                "internal",
                "record-review-outcome",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--input-json",
                "-",
                "--json",
            ],
            &parity_mission_close_result_payload(
                &bundle_id,
                &[code_reviewer_output_ref, spec_reviewer_output_ref],
                review_truth_snapshot,
            ),
        )?);
    }

    if steps.iter().all(|step| step.success) {
        let validate_step = run_json_smoke_step(
            "manual_validate_artifacts",
            binary,
            repo_root,
            &[
                "internal",
                "validate-mission-artifacts",
                "--repo-root",
                repo_root.to_str().unwrap(),
                "--mission-id",
                PARITY_MISSION_ID,
                "--json",
            ],
            &json!({}),
        )?;
        validate_success = parse_json_bool(&validate_step.stdout, "success").unwrap_or(false);
        steps.push(validate_step);
    }

    let summary = if steps.iter().all(|step| step.success) {
        Some(parity_artifact_summary(repo_root, validate_success)?)
    } else {
        None
    };

    Ok(ParityFlowOutcome { steps, summary })
}

fn run_autopilot_parity_flow(binary: &Path, repo_root: &Path) -> Result<ParityFlowOutcome> {
    let mut steps = Vec::new();
    let paths = MissionPaths::try_new(repo_root, PARITY_MISSION_ID)?;

    steps.push(run_json_smoke_step(
        "autopilot_init_mission",
        binary,
        repo_root,
        &[
            "internal",
            "init-mission",
            "--repo-root",
            repo_root.to_str().unwrap(),
            "--input-json",
            "-",
            "--json",
        ],
        &parity_init_payload(),
    )?);

    let mut validate_success = false;
    for _ in 0..8 {
        if !steps.iter().all(|step| step.success) {
            break;
        }
        match next_parity_autopilot_step(&paths)? {
            ParityAutopilotStep::WriteBlueprint => steps.push(run_json_smoke_step(
                "autopilot_write_blueprint",
                binary,
                repo_root,
                &[
                    "internal",
                    "materialize-plan",
                    "--repo-root",
                    repo_root.to_str().unwrap(),
                    "--input-json",
                    "-",
                    "--json",
                ],
                &parity_blueprint_payload(),
            )?),
            ParityAutopilotStep::CompileExecutionPackage => steps.push(run_json_smoke_step(
                "autopilot_compile_execution_package",
                binary,
                repo_root,
                &[
                    "internal",
                    "compile-execution-package",
                    "--repo-root",
                    repo_root.to_str().unwrap(),
                    "--input-json",
                    "-",
                    "--json",
                ],
                &parity_execution_package_payload(),
            )?),
            ParityAutopilotStep::DeriveWriterPacket => {
                let package_id = load_single_execution_package_id(&paths)?;
                steps.push(run_json_smoke_step(
                    "autopilot_derive_writer_packet",
                    binary,
                    repo_root,
                    &[
                        "internal",
                        "derive-writer-packet",
                        "--repo-root",
                        repo_root.to_str().unwrap(),
                        "--input-json",
                        "-",
                        "--json",
                    ],
                    &parity_writer_packet_payload(&package_id),
                )?);
            }
            ParityAutopilotStep::CompileSpecReviewBundle => {
                let package_id = load_single_execution_package_id(&paths)?;
                steps.push(run_json_smoke_step(
                    "autopilot_compile_spec_review_bundle",
                    binary,
                    repo_root,
                    &[
                        "internal",
                        "compile-review-bundle",
                        "--repo-root",
                        repo_root.to_str().unwrap(),
                        "--input-json",
                        "-",
                        "--json",
                    ],
                    &parity_spec_review_bundle_payload(&package_id),
                )?);
            }
            ParityAutopilotStep::RecordSpecReviewResult => {
                let bundle_id = load_single_bundle_id(&paths, BundleKind::SpecReview)?;
                let review_truth_snapshot = capture_review_truth_snapshot_payload(
                    binary,
                    repo_root,
                    PARITY_MISSION_ID,
                    &bundle_id,
                )?;
                let reviewer_output_ref = record_none_reviewer_output_ref(
                    binary,
                    repo_root,
                    PARITY_MISSION_ID,
                    &bundle_id,
                    "parity-spec-review",
                    &mut steps,
                )?;
                let code_reviewer_output_ref = record_none_reviewer_output_ref_after_snapshot(
                    binary,
                    repo_root,
                    PARITY_MISSION_ID,
                    &bundle_id,
                    "parity-code-review",
                    &mut steps,
                )?;
                steps.push(run_json_smoke_step(
                    "autopilot_record_spec_review_result",
                    binary,
                    repo_root,
                    &[
                        "internal",
                        "record-review-outcome",
                        "--repo-root",
                        repo_root.to_str().unwrap(),
                        "--input-json",
                        "-",
                        "--json",
                    ],
                    &parity_spec_review_result_payload(
                        &bundle_id,
                        &[reviewer_output_ref, code_reviewer_output_ref],
                        review_truth_snapshot,
                    ),
                )?);
            }
            ParityAutopilotStep::CompileMissionCloseBundle => {
                let package_id = load_single_execution_package_id(&paths)?;
                steps.push(run_json_smoke_step(
                    "autopilot_compile_mission_close_bundle",
                    binary,
                    repo_root,
                    &[
                        "internal",
                        "compile-review-bundle",
                        "--repo-root",
                        repo_root.to_str().unwrap(),
                        "--input-json",
                        "-",
                        "--json",
                    ],
                    &parity_mission_close_bundle_payload(repo_root, &package_id)?,
                )?);
            }
            ParityAutopilotStep::RecordMissionCloseReview => {
                let bundle_id = load_single_bundle_id(&paths, BundleKind::MissionClose)?;
                let review_truth_snapshot = capture_review_truth_snapshot_payload(
                    binary,
                    repo_root,
                    PARITY_MISSION_ID,
                    &bundle_id,
                )?;
                let code_reviewer_output_ref = record_none_reviewer_output_ref(
                    binary,
                    repo_root,
                    PARITY_MISSION_ID,
                    &bundle_id,
                    "parity-mission-close-code",
                    &mut steps,
                )?;
                let spec_reviewer_output_ref = record_none_reviewer_output_ref_after_snapshot(
                    binary,
                    repo_root,
                    PARITY_MISSION_ID,
                    &bundle_id,
                    "parity-mission-close-spec",
                    &mut steps,
                )?;
                steps.push(run_json_smoke_step(
                    "autopilot_record_mission_close_review",
                    binary,
                    repo_root,
                    &[
                        "internal",
                        "record-review-outcome",
                        "--repo-root",
                        repo_root.to_str().unwrap(),
                        "--input-json",
                        "-",
                        "--json",
                    ],
                    &parity_mission_close_result_payload(
                        &bundle_id,
                        &[code_reviewer_output_ref, spec_reviewer_output_ref],
                        review_truth_snapshot,
                    ),
                )?);
            }
            ParityAutopilotStep::Done => {
                let validate_step = run_json_smoke_step(
                    "autopilot_validate_artifacts",
                    binary,
                    repo_root,
                    &[
                        "internal",
                        "validate-mission-artifacts",
                        "--repo-root",
                        repo_root.to_str().unwrap(),
                        "--mission-id",
                        PARITY_MISSION_ID,
                        "--json",
                    ],
                    &json!({}),
                )?;
                validate_success =
                    parse_json_bool(&validate_step.stdout, "success").unwrap_or(false);
                steps.push(validate_step);
                break;
            }
        }
    }

    let summary = if steps.iter().all(|step| step.success) {
        Some(parity_artifact_summary(repo_root, validate_success)?)
    } else {
        None
    };

    Ok(ParityFlowOutcome { steps, summary })
}

fn next_parity_autopilot_step(paths: &MissionPaths) -> Result<ParityAutopilotStep> {
    if !paths.program_blueprint().is_file() {
        return Ok(ParityAutopilotStep::WriteBlueprint);
    }
    if read_json_files::<ExecutionPackage>(&paths.execution_packages_dir())?.is_empty() {
        return Ok(ParityAutopilotStep::CompileExecutionPackage);
    }
    if read_json_files::<WriterPacket>(&paths.packets_dir())?.is_empty() {
        return Ok(ParityAutopilotStep::DeriveWriterPacket);
    }

    let bundles = read_json_files::<ReviewBundle>(&paths.bundles_dir())?;
    if !bundles
        .iter()
        .any(|bundle| bundle.bundle_kind == BundleKind::SpecReview)
    {
        return Ok(ParityAutopilotStep::CompileSpecReviewBundle);
    }

    let gates: MissionGateIndex = serde_json::from_slice(
        &fs::read(paths.gates_json())
            .with_context(|| format!("failed to read {}", paths.gates_json().display()))?,
    )
    .with_context(|| format!("failed to parse {}", paths.gates_json().display()))?;
    if gates.gates.iter().any(|gate| {
        gate.gate_kind == GateKind::BlockingReview && gate.status == MissionGateStatus::Open
    }) {
        return Ok(ParityAutopilotStep::RecordSpecReviewResult);
    }
    if !bundles
        .iter()
        .any(|bundle| bundle.bundle_kind == BundleKind::MissionClose)
    {
        return Ok(ParityAutopilotStep::CompileMissionCloseBundle);
    }
    if gates.gates.iter().any(|gate| {
        gate.gate_kind == GateKind::MissionCloseReview && gate.status == MissionGateStatus::Open
    }) {
        return Ok(ParityAutopilotStep::RecordMissionCloseReview);
    }

    let state: Value = serde_json::from_slice(
        &fs::read(paths.state_json())
            .with_context(|| format!("failed to read {}", paths.state_json().display()))?,
    )
    .with_context(|| format!("failed to parse {}", paths.state_json().display()))?;
    if state.get("verdict").and_then(Value::as_str) == Some("complete") {
        return Ok(ParityAutopilotStep::Done);
    }

    bail!(
        "unable to determine the next autopilot parity step from {}",
        paths.hidden_mission_root().display()
    )
}

fn parity_artifact_summary(
    repo_root: &Path,
    validate_success: bool,
) -> Result<ParityArtifactSummary> {
    let paths = MissionPaths::try_new(repo_root, PARITY_MISSION_ID)?;
    let state: Value = serde_json::from_slice(
        &fs::read(paths.state_json())
            .with_context(|| format!("failed to read {}", paths.state_json().display()))?,
    )
    .with_context(|| format!("failed to parse {}", paths.state_json().display()))?;
    let gates: MissionGateIndex = serde_json::from_slice(
        &fs::read(paths.gates_json())
            .with_context(|| format!("failed to read {}", paths.gates_json().display()))?,
    )
    .with_context(|| format!("failed to parse {}", paths.gates_json().display()))?;
    let _lock_doc = ArtifactDocument::<OutcomeLockFrontmatter>::parse(
        &fs::read_to_string(paths.outcome_lock())
            .with_context(|| format!("failed to read {}", paths.outcome_lock().display()))?,
    )
    .with_context(|| format!("failed to parse {}", paths.outcome_lock().display()))?;
    let _blueprint_doc = ArtifactDocument::<ProgramBlueprintFrontmatter>::parse(
        &fs::read_to_string(paths.program_blueprint())
            .with_context(|| format!("failed to read {}", paths.program_blueprint().display()))?,
    )
    .with_context(|| format!("failed to parse {}", paths.program_blueprint().display()))?;

    let mut specs = Vec::new();
    let specs_root = paths.specs_root();
    if specs_root.is_dir() {
        let mut spec_dirs = fs::read_dir(&specs_root)
            .with_context(|| format!("failed to read {}", specs_root.display()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("failed to enumerate {}", specs_root.display()))?;
        spec_dirs.sort_by_key(|entry| entry.file_name());
        for entry in spec_dirs {
            let spec_doc = ArtifactDocument::<WorkstreamSpecFrontmatter>::parse(
                &fs::read_to_string(entry.path().join("SPEC.md")).with_context(|| {
                    format!("failed to read {}", entry.path().join("SPEC.md").display())
                })?,
            )
            .with_context(|| {
                format!("failed to parse {}", entry.path().join("SPEC.md").display())
            })?;
            specs.push(ParitySpecSummary {
                spec_id: spec_doc.frontmatter.spec_id.clone(),
                spec_revision: spec_doc.frontmatter.spec_revision,
                blueprint_revision: spec_doc.frontmatter.blueprint_revision,
                artifact_status: serde_label(&spec_doc.frontmatter.artifact_status)?,
                packetization_status: serde_label(&spec_doc.frontmatter.packetization_status)?,
                execution_status: serde_label(&spec_doc.frontmatter.execution_status)?,
            });
        }
    }

    let mut packages = read_json_files::<ExecutionPackage>(&paths.execution_packages_dir())?
        .into_iter()
        .map(|package| ParityPackageSummary {
            target_type: serde_label(&package.target_type)
                .unwrap_or_else(|_| "unknown".to_string()),
            target_id: package.target_id,
            status: serde_label(&package.status).unwrap_or_else(|_| "unknown".to_string()),
            included_specs: package
                .included_specs
                .into_iter()
                .map(|spec| spec.spec_id)
                .collect(),
            proof_obligations: package.proof_obligations,
            review_obligations: package.review_obligations,
        })
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| {
        left.target_type
            .cmp(&right.target_type)
            .then(left.target_id.cmp(&right.target_id))
            .then(left.status.cmp(&right.status))
    });

    let mut packets = read_json_files::<WriterPacket>(&paths.packets_dir())?
        .into_iter()
        .map(|packet| ParityPacketSummary {
            target_spec_id: packet.target_spec_id,
            required_checks: packet.required_checks,
            review_lenses: packet.review_lenses,
        })
        .collect::<Vec<_>>();
    packets.sort_by(|left, right| left.target_spec_id.cmp(&right.target_spec_id));

    let mut bundles = read_json_files::<ReviewBundle>(&paths.bundles_dir())?
        .into_iter()
        .map(|bundle| ParityBundleSummary {
            bundle_kind: serde_label(&bundle.bundle_kind).unwrap_or_else(|_| "unknown".to_string()),
            target_spec_id: bundle.target_spec_id,
            mandatory_review_lenses: bundle.mandatory_review_lenses,
            proof_rows_under_review: bundle.proof_rows_under_review,
            mission_level_proof_rows: bundle.mission_level_proof_rows,
        })
        .collect::<Vec<_>>();
    bundles.sort_by(|left, right| {
        left.bundle_kind
            .cmp(&right.bundle_kind)
            .then(left.target_spec_id.cmp(&right.target_spec_id))
    });

    let mut visible_artifacts = BTreeMap::new();
    for (name, present) in [
        ("mission_state", paths.mission_state().is_file()),
        ("outcome_lock", paths.outcome_lock().is_file()),
        ("program_blueprint", paths.program_blueprint().is_file()),
        ("review_ledger", paths.review_ledger().is_file()),
        ("spec", paths.spec_file(PARITY_SPEC_ID).is_file()),
        ("spec_review", paths.review_file(PARITY_SPEC_ID).is_file()),
        ("spec_notes", paths.notes_file(PARITY_SPEC_ID).is_file()),
        (
            "receipts_readme",
            paths
                .receipts_dir(PARITY_SPEC_ID)
                .join("README.md")
                .is_file(),
        ),
    ] {
        visible_artifacts.insert(name.to_string(), present);
    }

    let hidden_artifact_counts = BTreeMap::from([
        (
            "execution_packages".to_string(),
            read_json_files::<ExecutionPackage>(&paths.execution_packages_dir())?.len(),
        ),
        (
            "writer_packets".to_string(),
            read_json_files::<WriterPacket>(&paths.packets_dir())?.len(),
        ),
        (
            "review_bundles".to_string(),
            read_json_files::<ReviewBundle>(&paths.bundles_dir())?.len(),
        ),
    ]);

    let mut gate_summaries = gates
        .gates
        .iter()
        .map(|gate| {
            Ok(ParityGateSummary {
                gate_kind: serde_label(&gate.gate_kind)?,
                target_ref: gate.target_ref.clone(),
                status: serde_label(&gate.status)?,
                blocking: gate.blocking,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    gate_summaries.sort_by(|left, right| {
        left.gate_kind
            .cmp(&right.gate_kind)
            .then(left.target_ref.cmp(&right.target_ref))
            .then(left.status.cmp(&right.status))
    });

    Ok(ParityArtifactSummary {
        validate_success,
        execution_graph_present: paths.execution_graph().is_file(),
        visible_artifacts,
        hidden_artifact_counts,
        state: ParityStateSummary {
            phase: state
                .get("phase")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            activity: state
                .get("activity")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            verdict: state
                .get("verdict")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            terminality: state
                .get("terminality")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            resume_mode: state
                .get("resume_mode")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            next_phase: state
                .get("next_phase")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            target: state
                .get("target")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        },
        gate_phase: gates.current_phase,
        gates: gate_summaries,
        specs,
        packages,
        packets,
        bundles,
    })
}

fn parity_init_payload() -> Value {
    json!({
        "mission_id": PARITY_MISSION_ID,
        "title": "Qualification Parity Mission",
        "objective": "Prove manual and autopilot mission progression converge to the same durable mission truth.",
        "clarify_status": "ratified",
        "lock_status": "locked",
        "mission_state_body": "# Mission State\n\nThe parity mission is already clarified enough for planning.\n",
        "outcome_lock_body": "# Outcome Lock\n\n## Objective\n\nProve manual and autopilot mission progression converge to the same durable mission truth.\n"
    })
}

fn canonical_blueprint_body(route: &str, workstream_overview: &[&str]) -> String {
    format!(
        "# Program Blueprint\n\n## Locked Mission Reference\n\n- Qualification mission truth is locked.\n\n## Truth Register Summary\n\n- The repo uses deterministic internal qualification commands.\n\n## System Model\n\n- Touched surfaces: runtime artifacts and review bundles.\n\n## Invariants And Protected Behaviors\n\n- Keep mission truth visible and review-gated.\n\n## Proof Matrix\n\n- claim:qualification-proof\n\n## Decision Obligations\n\n- obligation:qualification-route\n\n## In-Scope Work Inventory\n\n{}\n\n## Selected Architecture\n\n{route}\n\n## Execution Graph and Safe-Wave Rules\n\n- Single-node qualification routes may execute directly; multi-node routes must follow the declared graph frontier.\n\n## Decision Log\n\n- Chose the deterministic internal route so qualification proofs stay reproducible.\n\n## Review Bundle Design\n\n- Mandatory review lenses: correctness, evidence_adequacy\n\n## Workstream Overview\n\n{}\n\n## Risks And Unknowns\n\n- Qualification must not overstate what it proves.\n\n## Replan Policy\n\n- Reopen planning if the selected route or proof contract changes.\n",
        workstream_overview
            .iter()
            .map(|spec_id| format!("- {spec_id}"))
            .collect::<Vec<_>>()
            .join("\n"),
        workstream_overview
            .iter()
            .map(|spec_id| format!("- {spec_id}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn qualification_spec_body(
    purpose: &str,
    read_scope: &[&str],
    write_scope: &[&str],
    proof_expectations: &[&str],
) -> String {
    format!(
        "# Workstream Spec\n\n## Purpose\n\n{purpose}\n\n## In Scope\n\n- Execute the bounded qualification slice.\n\n## Out Of Scope\n\n- Unrelated repo changes.\n\n## Dependencies\n\n- Outcome Lock and Program Blueprint stay current.\n\n## Touched Surfaces\n\n- Qualification runtime support.\n\n## Read Scope\n\n{}\n\n## Write Scope\n\n{}\n\n## Interfaces And Contracts Touched\n\n- qualification runtime contracts\n\n## Implementation Shape\n\nKeep the qualification slice explicit and reviewable.\n\n## Proof-Of-Completion Expectations\n\n{}\n\n## Non-Breakage Expectations\n\n- Existing mission contracts still validate.\n\n## Review Lenses\n\n- correctness\n- evidence_adequacy\n\n## Replan Boundary\n\n- Reopen planning on scope expansion.\n\n## Truth Basis Refs\n\n- PROGRAM-BLUEPRINT.md\n\n## Freshness Notes\n\n- Current for qualification smoke coverage.\n\n## Support Files\n\n- `REVIEW.md`\n",
        read_scope
            .iter()
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n"),
        write_scope
            .iter()
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n"),
        proof_expectations
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn parity_blueprint_payload() -> Value {
    json!({
        "mission_id": PARITY_MISSION_ID,
        "body_markdown": canonical_blueprint_body(
            "Advance one bounded spec and close the mission through review-clean completion.",
            &[PARITY_SPEC_ID]
        ),
        "plan_level": 4,
        "problem_size": "M",
        "status": "approved",
        "proof_matrix": [{
            "claim_ref": "claim:parity-proof",
            "statement": "The manual and autopilot paths converge to the same durable mission truth.",
            "required_evidence": ["RECEIPTS/parity-proof.txt"],
            "review_lenses": ["correctness", "evidence_adequacy"],
            "governing_contract_refs": ["blueprint"]
        }],
        "decision_obligations": [{
            "obligation_id": "obligation:parity-route",
            "question": "Should qualification treat autopilot as a composition over the same deterministic backend steps?",
            "why_it_matters": "It changes the durable parity contract under test.",
            "affects": ["architecture_boundary", "review_contract"],
            "governing_contract_refs": ["blueprint"],
            "review_contract_refs": ["review:parity"],
            "mission_close_claim_refs": ["claim:parity-proof"],
            "blockingness": "major",
            "candidate_route_count": 1,
            "required_evidence": ["RECEIPTS/parity-proof.txt"],
            "status": "selected",
            "resolution_rationale": "Qualification treats autopilot as a composition over the same backend contracts.",
            "evidence_refs": ["RECEIPTS/parity-proof.txt"]
        }],
        "selected_target_ref": format!("spec:{PARITY_SPEC_ID}"),
        "specs": [{
            "spec_id": PARITY_SPEC_ID,
            "purpose": "Carry the parity mission through one bounded execution slice.",
            "body_markdown": qualification_spec_body(
                "Carry the parity mission through one bounded execution slice.",
                &["src"],
                &["src"],
                &["cargo test"]
            ),
            "artifact_status": "active",
            "packetization_status": "runnable",
            "execution_status": "not_started"
        }]
    })
}

fn parity_execution_package_payload() -> Value {
    json!({
        "mission_id": PARITY_MISSION_ID,
        "target_type": "spec",
        "target_id": PARITY_SPEC_ID,
        "included_spec_ids": [PARITY_SPEC_ID],
        "dependency_satisfaction_state": [{
            "name": "lock_current",
            "satisfied": true,
            "detail": "Outcome Lock revision is current."
        }],
        "read_scope": ["src"],
        "write_scope": ["src"],
        "proof_obligations": ["cargo test"],
        "review_obligations": ["spec review"]
    })
}

fn parity_writer_packet_payload(package_id: &str) -> Value {
    json!({
        "mission_id": PARITY_MISSION_ID,
        "source_package_id": package_id,
        "target_spec_id": PARITY_SPEC_ID,
        "required_checks": ["cargo test"],
        "review_lenses": ["correctness", "evidence_adequacy"],
        "explicitly_disallowed_decisions": ["do not expand write scope"]
    })
}

fn parity_spec_review_bundle_payload(package_id: &str) -> Value {
    json!({
        "mission_id": PARITY_MISSION_ID,
        "source_package_id": package_id,
        "bundle_kind": "spec_review",
        "mandatory_review_lenses": ["correctness", "evidence_adequacy"],
        "target_spec_id": PARITY_SPEC_ID,
        "proof_rows_under_review": ["cargo test"],
        "receipts": ["RECEIPTS/parity-proof.txt"],
        "changed_files_or_diff": ["src/lib.rs"],
        "touched_interface_contracts": ["parity contract"]
    })
}

fn parity_spec_review_result_payload(
    bundle_id: &str,
    reviewer_output_refs: &[String],
    review_truth_snapshot: Value,
) -> Value {
    let evidence_refs = reviewer_output_refs
        .iter()
        .cloned()
        .chain(std::iter::once("RECEIPTS/parity-proof.txt".to_string()))
        .collect::<Vec<_>>();
    json!({
        "mission_id": PARITY_MISSION_ID,
        "bundle_id": bundle_id,
        "reviewer": "qualify-codex",
        "verdict": "clean",
        "target_spec_id": PARITY_SPEC_ID,
        "governing_refs": ["bundle"],
        "evidence_refs": evidence_refs,
        "findings": [],
        "disposition_notes": ["Parity spec review is clean."],
        "next_required_branch": "mission_close",
        "review_truth_snapshot": review_truth_snapshot
    })
}

fn parity_mission_close_bundle_payload(repo_root: &Path, package_id: &str) -> Result<Value> {
    Ok(json!({
        "mission_id": PARITY_MISSION_ID,
        "source_package_id": package_id,
        "bundle_kind": "mission_close",
        "mandatory_review_lenses": [
            "spec_conformance",
            "correctness",
            "interface_compatibility",
            "safety_security_policy",
            "operability_rollback_observability",
            "evidence_adequacy"
        ],
        "mission_level_proof_rows": ["cargo test", "review clean"],
        "cross_spec_claim_refs": [format!("{PARITY_SPEC_ID} complete")],
        "visible_artifact_refs": [
            fs::canonicalize(repo_root.join(format!("PLANS/{PARITY_MISSION_ID}/OUTCOME-LOCK.md")))
                .context("canonicalize parity outcome lock")?
                .display()
                .to_string(),
            fs::canonicalize(repo_root.join(format!("PLANS/{PARITY_MISSION_ID}/PROGRAM-BLUEPRINT.md")))
                .context("canonicalize parity blueprint")?
                .display()
                .to_string(),
            fs::canonicalize(repo_root.join(format!("PLANS/{PARITY_MISSION_ID}/REVIEW-LEDGER.md")))
                .context("canonicalize parity review ledger")?
                .display()
                .to_string()
        ],
        "deferred_descoped_follow_on_refs": [],
        "open_finding_summary": []
    }))
}

fn parity_mission_close_result_payload(
    bundle_id: &str,
    reviewer_output_refs: &[String],
    review_truth_snapshot: Value,
) -> Value {
    let evidence_refs = reviewer_output_refs
        .iter()
        .cloned()
        .chain(std::iter::once("RECEIPTS/parity-proof.txt".to_string()))
        .collect::<Vec<_>>();
    json!({
        "mission_id": PARITY_MISSION_ID,
        "bundle_id": bundle_id,
        "reviewer": "qualify-codex",
        "verdict": "complete",
        "governing_refs": ["mission-close-bundle"],
        "evidence_refs": evidence_refs,
        "findings": [],
        "disposition_notes": ["Parity mission-close review is clean."],
        "review_truth_snapshot": review_truth_snapshot
    })
}

fn parse_required_id(step: &SmokeStep, key: &str) -> Result<String> {
    serde_json::from_str::<Value>(&step.stdout)
        .with_context(|| format!("failed to parse JSON for step {}", step.step))?
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .with_context(|| format!("step {} did not emit {}", step.step, key))
}

fn parse_json_bool(stdout: &str, key: &str) -> Option<bool> {
    serde_json::from_str::<Value>(stdout)
        .ok()?
        .get(key)
        .and_then(Value::as_bool)
}

fn load_single_execution_package_id(paths: &MissionPaths) -> Result<String> {
    let packages = read_json_files::<ExecutionPackage>(&paths.execution_packages_dir())?;
    if packages.len() != 1 {
        bail!(
            "expected exactly one execution package under {}, found {}",
            paths.execution_packages_dir().display(),
            packages.len()
        );
    }
    Ok(packages
        .into_iter()
        .next()
        .expect("package vector should contain one entry")
        .package_id)
}

fn load_single_bundle_id(paths: &MissionPaths, kind: BundleKind) -> Result<String> {
    let bundles = read_json_files::<ReviewBundle>(&paths.bundles_dir())?
        .into_iter()
        .filter(|bundle| bundle.bundle_kind == kind)
        .collect::<Vec<_>>();
    if bundles.len() != 1 {
        bail!(
            "expected exactly one {:?} bundle under {}, found {}",
            kind,
            paths.bundles_dir().display(),
            bundles.len()
        );
    }
    Ok(bundles
        .into_iter()
        .next()
        .expect("bundle vector should contain one entry")
        .bundle_id)
}

fn read_json_files<T>(dir: &Path) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to enumerate {}", dir.display()))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            serde_json::from_slice(
                &fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
            )
            .with_context(|| format!("failed to parse {}", path.display()))
        })
        .collect()
}

fn serde_label<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_value(value)
        .context("failed to serialize label")?
        .as_str()
        .map(ToOwned::to_owned)
        .context("serialized label was not a string")
}

fn evidence_paths(
    repo_root: &Path,
    qualified_at: OffsetDateTime,
    codex_build: Option<&CodexBuildInfo>,
    qualification_id: &str,
) -> EvidencePaths {
    let timestamp = qualified_at
        .format(format_description!(
            "[year][month][day]T[hour][minute][second]Z"
        ))
        .expect("timestamp formatting should succeed");
    let build_slug = codex_build
        .and_then(|build| build.normalized_version.as_deref())
        .unwrap_or("unknown")
        .replace('.', "_");
    let short_id = &qualification_id[..8];
    let report_name = format!("{timestamp}--{build_slug}--{short_id}.json");

    let gate_evidence_dir = repo_root
        .join(REPORTS_DIR)
        .join(format!("{timestamp}--{build_slug}--{short_id}"));

    EvidencePaths {
        report_path: repo_root.join(REPORTS_DIR).join(report_name),
        latest_path: repo_root.join(LATEST_REPORT),
        gate_evidence_dir,
    }
}

fn persist_gate_evidence(report: &mut QualificationReport) -> Result<()> {
    fs::create_dir_all(&report.evidence.gate_evidence_dir).with_context(|| {
        format!(
            "failed to create {}",
            report.evidence.gate_evidence_dir.display()
        )
    })?;

    for gate in &mut report.gates {
        let Some(details) = gate.details.as_ref() else {
            continue;
        };
        let path = report
            .evidence
            .gate_evidence_dir
            .join(format!("{}.json", sanitize_gate_filename(gate.gate)));
        let payload = serde_json::to_vec_pretty(&json!({
            "gate": gate.gate,
            "description": gate.description,
            "status": gate.status,
            "message": gate.message,
            "details": details,
        }))
        .context("failed to encode gate evidence payload")?;
        fs::write(&path, payload).with_context(|| format!("failed to write {}", path.display()))?;
        gate.evidence_path = Some(path);
    }

    Ok(())
}

fn sanitize_gate_filename(gate: &str) -> String {
    gate.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn write_report(report: &QualificationReport) -> Result<()> {
    let report_path = PathBuf::from(&report.evidence.report_path);
    let latest_path = PathBuf::from(&report.evidence.latest_path);
    let payload =
        serde_json::to_vec_pretty(report).context("failed to encode qualification report")?;

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    if let Some(parent) = latest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(&report_path, &payload)
        .with_context(|| format!("failed to write {}", report_path.display()))?;
    fs::write(&latest_path, &payload)
        .with_context(|| format!("failed to write {}", latest_path.display()))?;

    Ok(())
}

fn emit_report(report: &QualificationReport, json_output: bool) -> Result<()> {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(report).context("failed to encode qualification JSON")?
        );
        return Ok(());
    }

    println!(
        "Qualification {}: {} passed, {} failed, {} skipped",
        if report.summary.supported_build_qualified {
            "passed (supported build)"
        } else if report.summary.failed > 0 {
            "failed"
        } else {
            "completed (not qualified for a supported build)"
        },
        report.summary.passed,
        report.summary.failed,
        report.summary.skipped
    );
    println!("Repo root: {}", report.repo_root.display());
    println!("Codex build: {}", report.codex_build);
    println!("Evidence: {}", report.evidence.report_path.display());
    for gate in &report.gates {
        println!(
            "- {:>7} {}: {}",
            match gate.status {
                GateStatus::Pass => "PASS",
                GateStatus::Fail => "FAIL",
                GateStatus::Skipped => "SKIP",
            },
            gate.gate,
            gate.message
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CodexBuildInfo, EvidencePaths, GateStatus, INTERNAL_CONTRACT_PARITY_GATE,
        NativeMultiAgentSummary, QualificationGate, QualificationReport, QualificationSummary,
        RequestedQualification, ReviewLoopFinding, ReviewLoopNextBranch, ReviewLoopSeverity,
        SmokeStep, assess_native_multi_agent_gate, count_stop_hooks, detect_codex_hooks_setting,
        native_child_probe_closeout_payload, parse_jsonl_events,
        parse_native_multi_agent_summary_text, persist_gate_evidence, project_agents_scaffold_gate,
        review_loop_decision, run_helper_drift_detection_flow, run_helper_force_normalization_flow,
        run_helper_partial_install_repair_flow, run_isolated_helper_flow,
        seed_native_codex_home_from, self_hosting_gate, summarize_native_multi_agent_events,
    };
    use crate::support_surface::{AGENTS_BLOCK, managed_skill_files, resolve_source_skills_root};
    use serde_json::{Value, json};
    use std::fs;
    use tempfile::TempDir;
    use time::OffsetDateTime;

    #[test]
    fn parses_features_table_hook_setting() {
        let contents = r#"
[features]
codex_hooks = true
"#;

        assert_eq!(detect_codex_hooks_setting(contents), Some(true));
    }

    #[test]
    fn parses_dotted_hook_setting() {
        let contents = r#"
features.codex_hooks = false
"#;

        assert_eq!(detect_codex_hooks_setting(contents), Some(false));
    }

    #[test]
    fn review_loop_clean_path_continues_and_resets_count() {
        let decision = review_loop_decision(
            &[ReviewLoopFinding {
                severity: ReviewLoopSeverity::P3,
                root_cause_key: "docs-polish".to_string(),
            }],
            5,
        );

        assert_eq!(decision.next_branch, ReviewLoopNextBranch::Continue);
        assert_eq!(decision.blocking_findings, 0);
        assert_eq!(decision.unique_blocking_root_causes, 0);
        assert_eq!(decision.consecutive_non_clean_loops, 0);
    }

    #[test]
    fn review_loop_non_clean_path_repairs_before_cap() {
        let decision = review_loop_decision(
            &[
                ReviewLoopFinding {
                    severity: ReviewLoopSeverity::P1,
                    root_cause_key: "stale-gate".to_string(),
                },
                ReviewLoopFinding {
                    severity: ReviewLoopSeverity::P2,
                    root_cause_key: "stale-gate".to_string(),
                },
            ],
            4,
        );

        assert_eq!(decision.next_branch, ReviewLoopNextBranch::Repair);
        assert_eq!(decision.blocking_findings, 2);
        assert_eq!(decision.unique_blocking_root_causes, 1);
        assert_eq!(decision.consecutive_non_clean_loops, 5);
    }

    #[test]
    fn review_loop_sixth_non_clean_path_routes_to_replan() {
        let decision = review_loop_decision(
            &[ReviewLoopFinding {
                severity: ReviewLoopSeverity::P0,
                root_cause_key: "architecture-contract".to_string(),
            }],
            5,
        );

        assert_eq!(decision.next_branch, ReviewLoopNextBranch::Replan);
        assert_eq!(decision.blocking_findings, 1);
        assert_eq!(decision.unique_blocking_root_causes, 1);
        assert_eq!(decision.consecutive_non_clean_loops, 6);
    }

    #[test]
    fn counts_stop_hooks_across_common_shapes() {
        let payload = json!({
            "hooks": [
                { "event": "Stop", "command": ["python3", "hook.py"] }
            ],
            "extra": {
                "Stop": [
                    { "command": ["python3", "secondary.py"] }
                ]
            }
        });

        assert_eq!(count_stop_hooks(&payload), 2);
    }

    fn seed_source_repo_surface(temp_dir: &TempDir, agents_contents: &str) {
        std::fs::write(temp_dir.path().join("Cargo.toml"), "[workspace]\n").expect("Cargo.toml");
        std::fs::create_dir_all(temp_dir.path().join("docs")).expect("docs dir");
        std::fs::create_dir_all(temp_dir.path().join("crates/codex1")).expect("crate dir");
        std::fs::create_dir_all(temp_dir.path().join("crates/codex1-core")).expect("core dir");
        std::fs::create_dir_all(temp_dir.path().join(".codex")).expect("codex dir");
        std::fs::write(
            temp_dir.path().join("crates/codex1/Cargo.toml"),
            "[package]\nname = \"codex1\"\n",
        )
        .expect("codex1 cargo");
        std::fs::write(
            temp_dir.path().join("crates/codex1-core/Cargo.toml"),
            "[package]\nname = \"codex1-core\"\n",
        )
        .expect("codex1-core cargo");
        std::fs::write(
            temp_dir.path().join(".codex/config.toml"),
            "[features]\ncodex_hooks = true\n",
        )
        .expect("config");
        std::fs::write(
            temp_dir.path().join(".codex/hooks.json"),
            "{\"hooks\":{\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"codex1 internal stop-hook\"}]}]}}",
        )
        .expect("hooks");
        std::fs::write(temp_dir.path().join("AGENTS.md"), agents_contents).expect("agents");
        std::fs::write(temp_dir.path().join("docs/codex1-prd.md"), "# PRD\n").expect("prd");

        let source_root = resolve_source_skills_root().expect("source skills root");
        for managed in managed_skill_files(&source_root).expect("managed skill files") {
            let target = temp_dir
                .path()
                .join(".codex/skills")
                .join(&managed.relative_path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).expect("skill parent");
            }
            std::fs::write(target, managed.contents).expect("skill contents");
        }
    }

    #[test]
    fn self_hosting_gate_detects_source_repo_markers() {
        let temp_dir = TempDir::new().expect("temp dir");
        seed_source_repo_surface(
            &temp_dir,
            &AGENTS_BLOCK
                .replace("{{BUILD_COMMAND}}", "cargo build -p codex1")
                .replace("{{TEST_COMMAND}}", "cargo test -p codex1")
                .replace("{{LINT_OR_FORMAT_COMMAND}}", "cargo fmt --all --check"),
        );

        let gate = self_hosting_gate(temp_dir.path(), true);
        assert_eq!(gate.status, GateStatus::Pass);
        assert!(gate.message.contains("match the enforced support surface"));
    }

    #[test]
    fn self_hosting_gate_fails_on_drifted_agents_block() {
        let temp_dir = TempDir::new().expect("temp dir");
        seed_source_repo_surface(
            &temp_dir,
            "<!-- codex1:begin -->\n## Codex1\n- drifted\n<!-- codex1:end -->\n",
        );

        let gate = self_hosting_gate(temp_dir.path(), true);
        assert_eq!(gate.status, GateStatus::Fail);
        assert!(gate.message.contains("placeholder-filled"));
    }

    #[test]
    fn project_agents_scaffold_gate_fails_on_drifted_block() {
        let gate = project_agents_scaffold_gate(
            std::path::Path::new("/repo/AGENTS.md"),
            Some("<!-- codex1:begin -->\n## Codex1\n- drifted\n<!-- codex1:end -->\n"),
        );

        assert_eq!(gate.status, GateStatus::Fail);
        assert!(gate.message.contains("has drifted"));
    }

    #[test]
    fn project_agents_scaffold_gate_fails_on_placeholder_commands() {
        let gate = project_agents_scaffold_gate(
            std::path::Path::new("/repo/AGENTS.md"),
            Some(AGENTS_BLOCK),
        );

        assert_eq!(gate.status, GateStatus::Fail);
        assert!(gate.message.contains("placeholders or missing"));
    }

    #[test]
    fn qualification_report_serializes_qualified_at_contract() {
        let report = QualificationReport {
            schema_version: "codex1.qualify.v1",
            qualification_id: "qual-1".to_string(),
            repo_root: std::path::PathBuf::from("/repo"),
            requested: RequestedQualification {
                live: false,
                self_hosting: false,
            },
            codex_build: "disabled".to_string(),
            codex_build_details: Some(CodexBuildInfo {
                command: "codex --version",
                raw_version: "disabled".to_string(),
                normalized_version: None,
            }),
            qualified_at: OffsetDateTime::UNIX_EPOCH,
            tested_at: Some(OffsetDateTime::UNIX_EPOCH),
            support_surface_signature: "sig".to_string(),
            summary: QualificationSummary {
                passed: 1,
                failed: 0,
                skipped: 0,
                passed_all_required_gates: true,
                qualification_scope: None,
                supported_build_qualified: false,
            },
            gates: vec![QualificationGate::pass("gate", "desc", "message", None)],
            evidence_root: std::path::PathBuf::from("/repo/.codex1/qualification"),
            evidence: EvidencePaths {
                report_path: std::path::PathBuf::from("/repo/report.json"),
                latest_path: std::path::PathBuf::from("/repo/latest.json"),
                gate_evidence_dir: std::path::PathBuf::from("/repo/gates"),
            },
        };

        let payload = serde_json::to_value(&report).expect("serialize qualification report");
        assert!(payload.get("qualified_at").is_some());
        assert!(payload.get("tested_at").is_some());
    }

    #[test]
    fn persist_gate_evidence_writes_raw_gate_payloads() {
        let temp_dir = TempDir::new().expect("tempdir should succeed");
        let gate_dir = temp_dir.path().join("gate-evidence");
        let mut report = QualificationReport {
            schema_version: "codex1.qualify.v1",
            qualification_id: "qual-1".to_string(),
            repo_root: temp_dir.path().to_path_buf(),
            requested: RequestedQualification {
                live: false,
                self_hosting: false,
            },
            codex_build: "disabled".to_string(),
            codex_build_details: None,
            qualified_at: OffsetDateTime::UNIX_EPOCH,
            tested_at: Some(OffsetDateTime::UNIX_EPOCH),
            support_surface_signature: "sig".to_string(),
            summary: QualificationSummary {
                passed: 1,
                failed: 0,
                skipped: 0,
                passed_all_required_gates: true,
                qualification_scope: None,
                supported_build_qualified: false,
            },
            gates: vec![QualificationGate::pass(
                "native_exec_resume_flow",
                "desc",
                "message",
                Some(json!({"raw_stdout": "FIRST", "raw_stderr": ""})),
            )],
            evidence_root: temp_dir.path().join(".codex1/qualification"),
            evidence: EvidencePaths {
                report_path: temp_dir.path().join("report.json"),
                latest_path: temp_dir.path().join("latest.json"),
                gate_evidence_dir: gate_dir.clone(),
            },
        };

        persist_gate_evidence(&mut report).expect("gate evidence should persist");

        let gate = report.gates.first().expect("gate should exist");
        let evidence_path = gate.evidence_path.as_ref().expect("evidence path");
        assert!(evidence_path.starts_with(&gate_dir));
        let contents =
            fs::read_to_string(evidence_path).expect("persisted gate evidence should be readable");
        assert!(contents.contains("\"raw_stdout\": \"FIRST\""));
    }

    #[test]
    fn scoped_subset_does_not_claim_full_build_from_internal_contract_parity() {
        let gates = vec![
            QualificationGate::pass("waiting_stop_hook_flow", "desc", "message", None),
            QualificationGate::pass("native_exec_resume_flow", "desc", "message", None),
            QualificationGate::pass("native_multi_agent_resume_flow", "desc", "message", None),
            QualificationGate::skipped(
                INTERNAL_CONTRACT_PARITY_GATE,
                "desc",
                "not automated",
                None,
            ),
        ];

        let summary = QualificationSummary::from_gates(&gates);
        assert!(!summary.passed_all_required_gates);
        assert_eq!(summary.qualification_scope, Some("scoped_supported_subset"));
        assert!(!summary.supported_build_qualified);
    }

    #[test]
    fn internal_contract_parity_keeps_scope_scoped_subset() {
        let gates = vec![
            QualificationGate::pass("waiting_stop_hook_flow", "desc", "message", None),
            QualificationGate::pass("native_exec_resume_flow", "desc", "message", None),
            QualificationGate::pass("native_multi_agent_resume_flow", "desc", "message", None),
            QualificationGate::pass(INTERNAL_CONTRACT_PARITY_GATE, "desc", "message", None),
        ];

        let summary = QualificationSummary::from_gates(&gates);
        assert!(summary.passed_all_required_gates);
        assert_eq!(summary.qualification_scope, Some("scoped_supported_subset"));
        assert!(summary.supported_build_qualified);
    }

    #[test]
    fn optional_self_hosting_skip_does_not_clear_required_gate_summary() {
        let gates = vec![
            QualificationGate::pass("waiting_stop_hook_flow", "desc", "message", None),
            QualificationGate::pass("native_exec_resume_flow", "desc", "message", None),
            QualificationGate::pass("native_multi_agent_resume_flow", "desc", "message", None),
            QualificationGate::pass(INTERNAL_CONTRACT_PARITY_GATE, "desc", "message", None),
            QualificationGate::skipped("self_hosting_source_repo", "desc", "message", None),
        ];

        let summary = QualificationSummary::from_gates(&gates);
        assert!(summary.passed_all_required_gates);
        assert_eq!(summary.qualification_scope, Some("scoped_supported_subset"));
        assert!(summary.supported_build_qualified);
    }

    fn smoke_step(success: bool) -> SmokeStep {
        SmokeStep {
            step: "test",
            success,
            exit_code: Some(if success { 0 } else { 1 }),
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    fn native_summary() -> NativeMultiAgentSummary {
        NativeMultiAgentSummary {
            used_spawn_agent: true,
            used_send_message: false,
            used_assign_task: false,
            used_followup_task: false,
            used_list_agents: true,
            used_wait_agent: true,
            used_close_agent: true,
            child_task_path: "/root/qualify_child".to_string(),
            child_seen_before_wait: true,
            child_status_before_wait: json!("running"),
            child_seen_after_wait: true,
            child_status_after_wait: json!("running"),
            child_seen_after_close: false,
            child_status_after_close: Value::Null,
            wait_summary_present: true,
            list_agents_before_wait: json!([{
                "task_path": "/root/qualify_child",
                "status": "running"
            }]),
            list_agents_after_wait: json!([{
                "task_path": "/root/qualify_child",
                "status": "running"
            }]),
            list_agents_after_close: json!([]),
        }
    }

    #[test]
    fn native_multi_agent_assessment_passes_with_resume_critical_surface() {
        let assessment = assess_native_multi_agent_gate(
            true,
            &smoke_step(true),
            &smoke_step(true),
            &smoke_step(true),
            &native_summary(),
            Some("live_non_final"),
            Some("live_non_final"),
            Some("actionable_non_terminal"),
        );

        assert!(assessment.success);
        assert_eq!(assessment.failure_classification, None);
        assert!(assessment.missing_required_tools.is_empty());
    }

    #[test]
    fn native_multi_agent_assessment_reports_required_tool_surface_gap() {
        let mut summary = native_summary();
        summary.used_list_agents = false;

        let assessment = assess_native_multi_agent_gate(
            true,
            &smoke_step(true),
            &smoke_step(true),
            &smoke_step(true),
            &summary,
            Some("missing"),
            Some("missing"),
            Some("actionable_non_terminal"),
        );

        assert!(!assessment.success);
        assert_eq!(
            assessment.failure_classification,
            Some("required_tool_surface_gap")
        );
        assert!(assessment.missing_required_tools.contains(&"list_agents"));
    }

    #[test]
    fn native_multi_agent_assessment_reports_closeout_write_failure_first() {
        let assessment = assess_native_multi_agent_gate(
            true,
            &smoke_step(true),
            &smoke_step(false),
            &smoke_step(true),
            &native_summary(),
            Some("live_non_final"),
            Some("live_non_final"),
            Some("actionable_non_terminal"),
        );

        assert!(!assessment.success);
        assert_eq!(
            assessment.failure_classification,
            Some("closeout_write_failed")
        );
    }

    #[test]
    fn native_multi_agent_assessment_reports_reconciliation_mismatch() {
        let assessment = assess_native_multi_agent_gate(
            true,
            &smoke_step(true),
            &smoke_step(true),
            &smoke_step(true),
            &native_summary(),
            Some("live_non_final"),
            Some("final_non_success"),
            Some("actionable_non_terminal"),
        );

        assert!(!assessment.success);
        assert_eq!(
            assessment.failure_classification,
            Some("reconciliation_mismatch")
        );
    }

    #[test]
    fn native_multi_agent_summary_parser_tolerates_null_optional_fields() {
        let summary = parse_native_multi_agent_summary_text(
            r#"{
                "used_spawn_agent": true,
                "used_list_agents": true,
                "used_wait_agent": true,
                "used_close_agent": null,
                "child_task_path": "/root/qualify_child",
                "child_seen_before_wait": true,
                "child_status_before_wait": "running",
                "child_seen_after_wait": true,
                "child_status_after_wait": "running",
                "child_seen_after_close": null,
                "child_status_after_close": null,
                "wait_summary_present": true,
                "list_agents_before_wait": null,
                "list_agents_after_wait": null,
                "list_agents_after_close": null
            }"#,
        )
        .expect("summary should parse");

        assert!(summary.used_spawn_agent);
        assert!(summary.used_list_agents);
        assert!(summary.used_wait_agent);
        assert!(!summary.used_close_agent);
        assert!(!summary.child_seen_after_close);
        assert_eq!(summary.child_status_after_close, Value::Null);
        assert_eq!(summary.list_agents_after_wait, Value::Null);
    }

    #[test]
    fn native_multi_agent_raw_summary_prefers_raw_events_over_final_summary_claims() {
        let events = parse_jsonl_events(
            br#"{"type":"thread.started","thread_id":"thread-root"}
{"type":"item.started","item":{"id":"item_1","type":"collab_tool_call","tool":"spawn_agent","status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_1","type":"collab_tool_call","tool":"spawn_agent","receiver_thread_ids":["thread-child"],"agents_states":{"thread-child":{"status":"pending_init","message":null}},"status":"completed"}}
{"type":"item.started","item":{"id":"item_2","type":"collab_tool_call","tool":"wait","status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_2","type":"collab_tool_call","tool":"wait","receiver_thread_ids":["thread-child"],"agents_states":{"thread-child":{"status":"completed","message":"PING"}},"status":"completed"}}
{"type":"item.completed","item":{"id":"item_3","type":"agent_message","text":"{\"used_spawn_agent\":true,\"used_list_agents\":true,\"used_wait_agent\":true,\"used_close_agent\":true,\"child_task_path\":\"/root/qualify_child\",\"child_seen_before_wait\":true,\"child_status_before_wait\":\"running\",\"child_seen_after_wait\":true,\"child_status_after_wait\":\"running\",\"wait_summary_present\":true,\"list_agents_before_wait\":[{\"agent_name\":\"/root/qualify_child\",\"agent_status\":\"running\"}],\"list_agents_after_wait\":[{\"agent_name\":\"/root/qualify_child\",\"agent_status\":\"running\"}]}"}}"#,
        )
        .expect("events should parse");

        let summary = summarize_native_multi_agent_events(&events);

        assert!(summary.used_spawn_agent);
        assert!(!summary.used_list_agents);
        assert!(summary.used_wait_agent);
        assert!(!summary.used_close_agent);
        assert_eq!(summary.child_task_path, "thread-child");
        assert_eq!(summary.child_status_after_wait, json!("completed"));
        assert!(summary.wait_summary_present);
        assert_eq!(summary.list_agents_before_wait, Value::Null);
    }

    #[test]
    fn native_multi_agent_assessment_ignores_misleading_final_summary_without_raw_list_agents() {
        let events = parse_jsonl_events(
            br#"{"type":"item.completed","item":{"id":"item_1","type":"collab_tool_call","tool":"spawn_agent","receiver_thread_ids":["thread-child"],"agents_states":{"thread-child":{"status":"pending_init","message":null}},"status":"completed"}}
{"type":"item.completed","item":{"id":"item_2","type":"collab_tool_call","tool":"wait","receiver_thread_ids":["thread-child"],"agents_states":{"thread-child":{"status":"running","message":null}},"status":"completed"}}
{"type":"item.completed","item":{"id":"item_3","type":"collab_tool_call","tool":"close_agent","receiver_thread_ids":["thread-child"],"agents_states":{"thread-child":{"status":"shutdown","message":null}},"status":"completed"}}
{"type":"item.completed","item":{"id":"item_4","type":"agent_message","text":"{\"used_spawn_agent\":true,\"used_list_agents\":true,\"used_wait_agent\":true,\"used_close_agent\":true,\"child_task_path\":\"/root/qualify_child\",\"child_seen_before_wait\":true,\"child_status_before_wait\":\"running\",\"child_seen_after_wait\":true,\"child_status_after_wait\":\"running\",\"wait_summary_present\":true,\"list_agents_before_wait\":[{\"agent_name\":\"/root/qualify_child\",\"agent_status\":\"running\"}],\"list_agents_after_wait\":[{\"agent_name\":\"/root/qualify_child\",\"agent_status\":\"running\"}]}"}}"#,
        )
        .expect("events should parse");
        let summary = summarize_native_multi_agent_events(&events);

        let assessment = assess_native_multi_agent_gate(
            true,
            &smoke_step(true),
            &smoke_step(true),
            &smoke_step(true),
            &summary,
            Some("missing"),
            Some("missing"),
            Some("actionable_non_terminal"),
        );

        assert!(!assessment.success);
        assert_eq!(
            assessment.failure_classification,
            Some("required_tool_surface_gap")
        );
        assert!(assessment.missing_required_tools.contains(&"list_agents"));
    }

    #[test]
    fn native_child_probe_closeout_payload_includes_artifact_fingerprints() {
        let payload = native_child_probe_closeout_payload(&native_summary());
        let fingerprints = payload
            .get("artifact_fingerprints")
            .and_then(Value::as_object)
            .expect("payload should include artifact fingerprints");

        assert!(!fingerprints.is_empty());
        assert_eq!(
            fingerprints.get("qualification_probe"),
            Some(&json!("native-child-probe:/root/qualify_child"))
        );
    }

    #[test]
    fn seed_native_codex_home_preserves_profile_and_trusts_repo() {
        let temp_dir = TempDir::new().expect("tempdir should succeed");
        let source_codex_home = temp_dir.path().join("source-codex-home");
        let repo_root = temp_dir.path().join("repo");
        let target_codex_home = temp_dir.path().join("target-codex-home");

        fs::create_dir_all(&source_codex_home).expect("source codex home should exist");
        fs::create_dir_all(&repo_root).expect("repo root should exist");
        fs::write(source_codex_home.join("auth.json"), "{\"token\":\"fake\"}")
            .expect("auth file should be written");
        fs::write(source_codex_home.join("installation_id"), "installation")
            .expect("installation id should be written");
        fs::create_dir_all(source_codex_home.join("profiles/default")).expect("profile dir");
        fs::write(
            source_codex_home.join("profiles/default/state.json"),
            "{\"surface\":\"full\"}",
        )
        .expect("profile state should be written");
        fs::write(
            source_codex_home.join("config.toml"),
            "model = \"gpt-5.4\"\n",
        )
        .expect("config should be written");

        seed_native_codex_home_from(&source_codex_home, &repo_root, &target_codex_home)
            .expect("native codex home should seed");

        assert_eq!(
            fs::read_to_string(target_codex_home.join("auth.json")).unwrap(),
            "{\"token\":\"fake\"}"
        );
        assert_eq!(
            fs::read_to_string(target_codex_home.join("installation_id")).unwrap(),
            "installation"
        );
        assert_eq!(
            fs::read_to_string(target_codex_home.join("profiles/default/state.json")).unwrap(),
            "{\"surface\":\"full\"}"
        );

        let config = fs::read_to_string(target_codex_home.join("config.toml"))
            .expect("trusted config should be written");
        let canonical_repo_root = fs::canonicalize(&repo_root).unwrap();
        assert!(config.contains("model = \"gpt-5.4\""));
        assert!(config.contains("trust_level = \"trusted\""));
        assert!(config.contains(&canonical_repo_root.display().to_string()));
    }

    #[test]
    fn qualification_cli_support_surface_helper_flows_pass() {
        let binary = assert_cmd::cargo::cargo_bin("codex1");
        let previous = std::env::var_os("CODEX1_QUALIFY_EXECUTABLE");
        unsafe {
            std::env::set_var("CODEX1_QUALIFY_EXECUTABLE", &binary);
        }
        for gate in [
            run_isolated_helper_flow().expect("isolated helper flow should run"),
            run_helper_force_normalization_flow()
                .expect("force normalization helper flow should run"),
            run_helper_partial_install_repair_flow()
                .expect("partial install repair flow should run"),
            run_helper_drift_detection_flow().expect("drift detection flow should run"),
        ] {
            assert_eq!(
                gate.status,
                GateStatus::Pass,
                "{} should pass qualification proof: {}",
                gate.gate,
                gate.message
            );
        }
        match previous {
            Some(value) => unsafe {
                std::env::set_var("CODEX1_QUALIFY_EXECUTABLE", value);
            },
            None => unsafe {
                std::env::remove_var("CODEX1_QUALIFY_EXECUTABLE");
            },
        }
    }
}
