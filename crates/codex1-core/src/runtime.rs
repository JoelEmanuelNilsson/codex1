use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::artifacts::{
    ArtifactDocument, ArtifactKind, BlueprintStatus, ClarifyStatus, DecisionAffect,
    DecisionBlockingness, DecisionObligation, DecisionStatus, LockPosture, LockStatus,
    MissionStateFrontmatter, OwnerMode, PacketizationStatus, ProblemSize,
    ProgramBlueprintFrontmatter, ProofMatrixRow, SpecArtifactStatus, SpecExecutionStatus,
    WorkstreamSpecFrontmatter,
};
use crate::fingerprint::Fingerprint;
use crate::paths::MissionPaths;
use crate::ralph::{
    ActiveCycleLoad, ActiveCycleState, ChildLaneExpectation, ChildLaneIntegrationStatus,
    CloseoutRecord, CycleKind, RalphState, ResumeMode, StopHookOutput, Terminality, Verdict,
    append_closeout_and_rebuild_state, contradictory_active_cycle_state, inspect_active_cycle,
    load_closeouts, load_state, rebuild_state_from_closeouts,
};
use crate::selection_state_path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    Mission,
    Spec,
    Wave,
}

impl TargetType {
    #[must_use]
    pub const fn as_phase_target(&self) -> &'static str {
        match self {
            Self::Mission => "mission",
            Self::Spec => "spec",
            Self::Wave => "wave",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPackageStatus {
    Draft,
    ReadyForGate,
    Passed,
    Failed,
    Superseded,
    Consumed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BundleKind {
    SpecReview,
    MissionClose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReopenLayer {
    ExecutionLocal,
    ExecutionPackage,
    Blueprint,
    MissionLock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerCode {
    WriteScopeExpansion,
    InterfaceContractChange,
    DependencyTruthChange,
    ProofObligationChange,
    ReviewContractChange,
    ProtectedSurfaceChange,
    MigrationRolloutChange,
    OutcomeLockChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    OutcomeLock,
    PlanningCompletion,
    ExecutionPackage,
    BlockingReview,
    MissionCloseReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissionGateStatus {
    Open,
    Passed,
    Failed,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionStatus {
    Open,
    Triaged,
    AcceptedForRepair,
    AcceptedForReplan,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriageDecision {
    ContainLocally,
    RepairInLayer,
    ReopenExecutionPackage,
    ReopenBlueprint,
    ReopenMissionLock,
    Dismiss,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MachineAction {
    ContinueLocalExecution,
    ForceReview,
    ForceRepair,
    ForceReplan,
    YieldNeedsUser,
    HaltHardBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NextRequiredBranch {
    Execution,
    Review,
    Repair,
    Replan,
    NeedsUser,
    MissionClose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RalphLoopLeaseMode {
    PlanningLoop,
    ExecutionLoop,
    ReviewLoop,
    AutopilotLoop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RalphLoopLeaseStatus {
    Active,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RalphLoopLease {
    pub mission_id: String,
    pub mode: RalphLoopLeaseMode,
    pub status: RalphLoopLeaseStatus,
    pub owner: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub acquired_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    pub parent_authority_verifier: Option<Fingerprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_authority_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RalphLoopLeaseInput {
    pub mission_id: String,
    pub mode: RalphLoopLeaseMode,
    pub owner: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RalphLoopLeasePauseInput {
    #[serde(default)]
    pub mission_id: Option<String>,
    pub paused_by: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RalphLoopLeaseReport {
    pub repo_root: PathBuf,
    #[serde(default)]
    pub lease: Option<RalphLoopLease>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerRule {
    pub trigger_code: TriggerCode,
    pub reopen_layer: ReopenLayer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplanBoundary {
    pub local_repair_allowed: bool,
    pub trigger_matrix: Vec<TriggerRule>,
}

impl Default for ReplanBoundary {
    fn default() -> Self {
        Self {
            local_repair_allowed: false,
            trigger_matrix: vec![
                TriggerRule {
                    trigger_code: TriggerCode::WriteScopeExpansion,
                    reopen_layer: ReopenLayer::ExecutionPackage,
                },
                TriggerRule {
                    trigger_code: TriggerCode::InterfaceContractChange,
                    reopen_layer: ReopenLayer::Blueprint,
                },
                TriggerRule {
                    trigger_code: TriggerCode::DependencyTruthChange,
                    reopen_layer: ReopenLayer::ExecutionPackage,
                },
                TriggerRule {
                    trigger_code: TriggerCode::ProofObligationChange,
                    reopen_layer: ReopenLayer::Blueprint,
                },
                TriggerRule {
                    trigger_code: TriggerCode::ReviewContractChange,
                    reopen_layer: ReopenLayer::Blueprint,
                },
                TriggerRule {
                    trigger_code: TriggerCode::ProtectedSurfaceChange,
                    reopen_layer: ReopenLayer::MissionLock,
                },
                TriggerRule {
                    trigger_code: TriggerCode::MigrationRolloutChange,
                    reopen_layer: ReopenLayer::Blueprint,
                },
                TriggerRule {
                    trigger_code: TriggerCode::OutcomeLockChange,
                    reopen_layer: ReopenLayer::MissionLock,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncludedSpecRef {
    pub spec_id: String,
    pub spec_revision: u64,
    pub spec_fingerprint: Fingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyCheck {
    pub name: String,
    pub satisfied: bool,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageGateCheck {
    pub gate_id: String,
    pub passed: bool,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPackage {
    pub package_id: String,
    pub mission_id: String,
    pub target_type: TargetType,
    pub target_id: String,
    pub lock_revision: u64,
    pub lock_fingerprint: Fingerprint,
    pub blueprint_revision: u64,
    pub blueprint_fingerprint: Fingerprint,
    pub dependency_snapshot_fingerprint: Fingerprint,
    #[serde(default)]
    pub wave_fingerprint: Option<Fingerprint>,
    pub included_specs: Vec<IncludedSpecRef>,
    pub dependency_satisfaction_state: Vec<DependencyCheck>,
    pub read_scope: Vec<String>,
    pub write_scope: Vec<String>,
    pub proof_obligations: Vec<String>,
    pub review_obligations: Vec<String>,
    pub replan_boundary: ReplanBoundary,
    #[serde(default)]
    pub wave_context: Option<String>,
    #[serde(default)]
    pub wave_specs: Vec<WaveSpecInput>,
    pub gate_checks: Vec<PackageGateCheck>,
    pub validation_failures: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub validated_at: OffsetDateTime,
    pub status: ExecutionPackageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WaveRiskClass {
    Normal,
    Meta,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaveSpecInput {
    pub spec_id: String,
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    #[serde(default)]
    pub coupling_tags: Vec<String>,
    #[serde(default)]
    pub ownership_domains: Vec<String>,
    #[serde(default)]
    pub risk_class: Option<WaveRiskClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaveManifest {
    pub mission_id: String,
    pub wave_id: String,
    pub included_specs: Vec<IncludedSpecRef>,
    pub read_scope: Vec<String>,
    pub write_scope: Vec<String>,
    #[serde(default)]
    pub wave_specs: Vec<WaveSpecInput>,
    #[serde(with = "time::serde::rfc3339")]
    pub generated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionGraphNodeInput {
    pub spec_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    #[serde(default)]
    pub coupling_tags: Vec<String>,
    #[serde(default)]
    pub ownership_domains: Vec<String>,
    #[serde(default)]
    pub risk_class: Option<WaveRiskClass>,
    #[serde(default)]
    pub acceptance_checks: Vec<String>,
    #[serde(default)]
    pub evidence_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionGraphNode {
    pub spec_id: String,
    pub spec_revision: u64,
    pub spec_fingerprint: Fingerprint,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    #[serde(default)]
    pub coupling_tags: Vec<String>,
    #[serde(default)]
    pub ownership_domains: Vec<String>,
    #[serde(default)]
    pub risk_class: Option<WaveRiskClass>,
    #[serde(default)]
    pub acceptance_checks: Vec<String>,
    pub evidence_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionGraphObligationKind {
    Validation,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionGraphObligationStatus {
    Open,
    Satisfied,
    Failed,
    Descoped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionGraphObligationInput {
    pub obligation_id: String,
    pub kind: ExecutionGraphObligationKind,
    pub target_spec_id: String,
    pub discharges_claim_ref: String,
    #[serde(default)]
    pub proof_rows: Vec<String>,
    #[serde(default)]
    pub acceptance_checks: Vec<String>,
    #[serde(default)]
    pub required_evidence: Vec<String>,
    #[serde(default)]
    pub review_lenses: Vec<String>,
    pub blocking: bool,
    #[serde(default = "default_execution_graph_obligation_status")]
    pub status: ExecutionGraphObligationStatus,
    #[serde(default)]
    pub satisfied_by: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionGraphObligation {
    pub obligation_id: String,
    pub kind: ExecutionGraphObligationKind,
    pub target_spec_id: String,
    pub target_spec_revision: u64,
    pub target_spec_fingerprint: Fingerprint,
    pub discharges_claim_ref: String,
    #[serde(default)]
    pub proof_rows: Vec<String>,
    #[serde(default)]
    pub acceptance_checks: Vec<String>,
    #[serde(default)]
    pub required_evidence: Vec<String>,
    #[serde(default)]
    pub review_lenses: Vec<String>,
    pub blocking: bool,
    pub status: ExecutionGraphObligationStatus,
    #[serde(default)]
    pub satisfied_by: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionGraphInput {
    #[serde(default)]
    pub nodes: Vec<ExecutionGraphNodeInput>,
    #[serde(default)]
    pub obligations: Vec<ExecutionGraphObligationInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionGraph {
    pub mission_id: String,
    pub blueprint_revision: u64,
    pub blueprint_fingerprint: Fingerprint,
    pub nodes: Vec<ExecutionGraphNode>,
    pub obligations: Vec<ExecutionGraphObligation>,
    #[serde(with = "time::serde::rfc3339")]
    pub generated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionGraphValidationReport {
    pub mission_id: String,
    pub blueprint_revision: u64,
    pub valid: bool,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriterPacket {
    pub packet_id: String,
    pub mission_id: String,
    pub source_package_id: String,
    pub target_spec_id: String,
    pub blueprint_revision: u64,
    pub spec_revision: u64,
    pub allowed_read_paths: Vec<String>,
    pub allowed_write_paths: Vec<String>,
    pub proof_rows: Vec<String>,
    pub required_checks: Vec<String>,
    pub review_lenses: Vec<String>,
    pub replan_boundary: ReplanBoundary,
    pub explicitly_disallowed_decisions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriterPacketInput {
    pub mission_id: String,
    pub source_package_id: String,
    pub target_spec_id: String,
    #[serde(default)]
    pub required_checks: Vec<String>,
    #[serde(default)]
    pub review_lenses: Vec<String>,
    #[serde(default)]
    pub explicitly_disallowed_decisions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriterPacketValidationReport {
    pub mission_id: String,
    pub packet_id: String,
    pub valid: bool,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplanLogInput {
    pub mission_id: String,
    pub reopened_layer: ReopenLayer,
    pub summary: String,
    #[serde(default)]
    pub preserved_refs: Vec<String>,
    #[serde(default)]
    pub invalidated_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplanLogReport {
    pub mission_id: String,
    pub reopened_layer: ReopenLayer,
    pub log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewBundle {
    pub bundle_id: String,
    pub mission_id: String,
    pub bundle_kind: BundleKind,
    pub source_package_id: String,
    pub lock_revision: u64,
    pub lock_fingerprint: Fingerprint,
    pub blueprint_revision: u64,
    pub blueprint_fingerprint: Fingerprint,
    pub governing_revision: String,
    pub mandatory_review_lenses: Vec<String>,
    #[serde(default)]
    pub target_spec_id: Option<String>,
    #[serde(default)]
    pub spec_revision: Option<u64>,
    #[serde(default)]
    pub spec_fingerprint: Option<Fingerprint>,
    #[serde(default)]
    pub proof_rows_under_review: Vec<String>,
    #[serde(default)]
    pub receipts: Vec<String>,
    #[serde(default)]
    pub changed_files_or_diff: Vec<String>,
    #[serde(default)]
    pub touched_interface_contracts: Vec<String>,
    #[serde(default)]
    pub mission_level_proof_rows: Vec<String>,
    #[serde(default)]
    pub cross_spec_claim_refs: Vec<String>,
    #[serde(default)]
    pub included_spec_refs: Vec<String>,
    #[serde(default)]
    pub visible_artifact_refs: Vec<String>,
    #[serde(default)]
    pub deferred_descoped_follow_on_refs: Vec<String>,
    #[serde(default)]
    pub open_finding_summary: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub generated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionGateRecord {
    pub gate_id: String,
    pub gate_kind: GateKind,
    pub target_ref: String,
    pub governing_refs: Vec<String>,
    pub status: MissionGateStatus,
    pub blocking: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub opened_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub evaluated_at: Option<OffsetDateTime>,
    #[serde(default)]
    pub evaluated_against_ref: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub failure_refs: Vec<String>,
    #[serde(default)]
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionGateIndex {
    pub mission_id: String,
    pub current_phase: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub gates: Vec<MissionGateRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContradictionRecord {
    pub contradiction_id: String,
    pub discovered_in_phase: String,
    pub discovered_by: String,
    pub target_type: TargetType,
    pub target_id: String,
    pub evidence_refs: Vec<String>,
    pub violated_assumption_or_contract: String,
    pub suggested_reopen_layer: ReopenLayer,
    pub reason_code: String,
    pub status: ContradictionStatus,
    pub governing_revision: String,
    #[serde(default)]
    pub triage_decision: Option<TriageDecision>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub triaged_at: Option<OffsetDateTime>,
    #[serde(default)]
    pub triaged_by: Option<String>,
    #[serde(default)]
    pub machine_action: Option<MachineAction>,
    #[serde(default)]
    pub next_required_branch: Option<NextRequiredBranch>,
    #[serde(default)]
    pub resolution_ref: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resolved_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionState {
    pub selection_request_id: String,
    pub candidate_mission_ids: Vec<String>,
    pub canonical_selection_request: String,
    #[serde(default)]
    pub selected_mission_id: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub request_emitted_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resolved_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub cleared_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaitingRequest {
    pub waiting_for: String,
    pub canonical_request: String,
    pub resume_condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionInitInput {
    pub title: String,
    pub objective: String,
    #[serde(default)]
    pub mission_id: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub root_mission_id: Option<String>,
    #[serde(default)]
    pub parent_mission_id: Option<String>,
    #[serde(default)]
    pub clarify_status: Option<ClarifyStatus>,
    #[serde(default)]
    pub lock_status: Option<LockStatus>,
    #[serde(default)]
    pub lock_posture: Option<LockPosture>,
    #[serde(default)]
    pub mission_state_body: Option<String>,
    #[serde(default)]
    pub outcome_lock_body: Option<String>,
    #[serde(default)]
    pub readme_body: Option<String>,
    #[serde(default)]
    pub waiting_request: Option<WaitingRequest>,
    #[serde(default)]
    pub next_action: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionBootstrapReport {
    pub mission_id: String,
    pub mission_root: PathBuf,
    pub hidden_root: PathBuf,
    pub lock_fingerprint: Option<Fingerprint>,
    pub clarify_status: ClarifyStatus,
    pub lock_status: LockStatus,
    pub closeout_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamSpecInput {
    pub spec_id: String,
    pub purpose: String,
    #[serde(default)]
    pub body_markdown: Option<String>,
    #[serde(default)]
    pub artifact_status: Option<SpecArtifactStatus>,
    #[serde(default)]
    pub packetization_status: Option<PacketizationStatus>,
    #[serde(default)]
    pub execution_status: Option<SpecExecutionStatus>,
    #[serde(default)]
    pub owner_mode: Option<OwnerMode>,
    #[serde(default)]
    pub replan_boundary: Option<ReplanBoundary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanningWriteInput {
    pub mission_id: String,
    pub body_markdown: String,
    pub plan_level: u8,
    #[serde(default)]
    pub problem_size: Option<ProblemSize>,
    #[serde(default)]
    pub status: Option<BlueprintStatus>,
    #[serde(default)]
    pub blueprint_revision: Option<u64>,
    #[serde(default)]
    pub proof_matrix: Vec<ProofMatrixRow>,
    #[serde(default)]
    pub decision_obligations: Vec<DecisionObligation>,
    #[serde(default)]
    pub specs: Vec<WorkstreamSpecInput>,
    #[serde(default)]
    pub selected_target_ref: Option<String>,
    #[serde(default)]
    pub execution_graph: Option<ExecutionGraphInput>,
    #[serde(default)]
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanningWriteReport {
    pub mission_id: String,
    pub blueprint_revision: u64,
    pub blueprint_fingerprint: Fingerprint,
    pub written_specs: Vec<IncludedSpecRef>,
}

#[derive(Debug, Clone)]
struct PlanningWriteContext {
    lock_revision: u64,
    lock_body: String,
    lock_fingerprint: Fingerprint,
    blueprint_revision: u64,
    blueprint_rendered: String,
    blueprint_fingerprint: Fingerprint,
    planning_contract_changed: bool,
    normalized_execution_graph: Option<ExecutionGraphInput>,
    prior_active_spec_ids: Vec<String>,
    input_spec_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct PlanningSpecSyncResult {
    written_specs: Vec<IncludedSpecRef>,
    planning_contract_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPackageInput {
    pub mission_id: String,
    pub target_type: TargetType,
    pub target_id: String,
    pub included_spec_ids: Vec<String>,
    #[serde(default)]
    pub dependency_satisfaction_state: Vec<DependencyCheck>,
    #[serde(default)]
    pub read_scope: Vec<String>,
    #[serde(default)]
    pub write_scope: Vec<String>,
    #[serde(default)]
    pub proof_obligations: Vec<String>,
    #[serde(default)]
    pub review_obligations: Vec<String>,
    #[serde(default)]
    pub replan_boundary: Option<ReplanBoundary>,
    #[serde(default)]
    pub wave_context: Option<String>,
    #[serde(default)]
    pub wave_fingerprint: Option<Fingerprint>,
    #[serde(default)]
    pub wave_specs: Vec<WaveSpecInput>,
    #[serde(default)]
    pub gate_checks: Vec<PackageGateCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageValidationReport {
    pub mission_id: String,
    pub package_id: String,
    pub valid: bool,
    pub findings: Vec<String>,
    pub governing_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewBundleInput {
    pub mission_id: String,
    pub source_package_id: String,
    pub bundle_kind: BundleKind,
    pub mandatory_review_lenses: Vec<String>,
    #[serde(default)]
    pub target_spec_id: Option<String>,
    #[serde(default)]
    pub proof_rows_under_review: Vec<String>,
    #[serde(default)]
    pub receipts: Vec<String>,
    #[serde(default)]
    pub changed_files_or_diff: Vec<String>,
    #[serde(default)]
    pub touched_interface_contracts: Vec<String>,
    #[serde(default)]
    pub mission_level_proof_rows: Vec<String>,
    #[serde(default)]
    pub cross_spec_claim_refs: Vec<String>,
    #[serde(default)]
    pub visible_artifact_refs: Vec<String>,
    #[serde(default)]
    pub deferred_descoped_follow_on_refs: Vec<String>,
    #[serde(default)]
    pub open_finding_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewBundleValidationReport {
    pub mission_id: String,
    pub bundle_id: String,
    pub valid: bool,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewEvidenceSnapshot {
    pub mission_id: String,
    pub bundle_id: String,
    pub bundle_kind: BundleKind,
    pub source_package_id: String,
    #[serde(default)]
    pub target_spec_id: Option<String>,
    pub lock_revision: u64,
    pub lock_fingerprint: Fingerprint,
    pub blueprint_revision: u64,
    pub blueprint_fingerprint: Fingerprint,
    #[serde(default)]
    pub spec_revision: Option<u64>,
    #[serde(default)]
    pub spec_fingerprint: Option<Fingerprint>,
    pub mandatory_review_lenses: Vec<String>,
    pub proof_rows_under_review: Vec<String>,
    pub receipts: Vec<String>,
    pub changed_files_or_diff: Vec<String>,
    pub touched_interface_contracts: Vec<String>,
    #[serde(default)]
    pub mission_level_proof_rows: Vec<String>,
    #[serde(default)]
    pub cross_spec_claim_refs: Vec<String>,
    #[serde(default)]
    pub visible_artifact_refs: Vec<String>,
    #[serde(default)]
    pub deferred_descoped_follow_on_refs: Vec<String>,
    #[serde(default)]
    pub open_finding_summary: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub reviewer_instructions: Vec<String>,
    pub review_truth_guard: ReviewTruthGuardBinding,
    #[serde(with = "time::serde::rfc3339")]
    pub generated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewTruthGuardBinding {
    pub bundle_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
    pub artifact_fingerprint_count: usize,
    pub guard_fingerprint: Fingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewEvidenceSnapshotValidationReport {
    pub mission_id: String,
    pub bundle_id: String,
    pub valid: bool,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone)]
struct LoadedSpecContext {
    included: IncludedSpecRef,
    artifact_status: SpecArtifactStatus,
    packetization_status: PacketizationStatus,
    blueprint_revision: u64,
    blueprint_fingerprint: Option<Fingerprint>,
    replan_boundary: Option<ReplanBoundary>,
}

#[derive(Debug, Clone)]
struct ExecutionPackageContractEvaluation {
    included_specs: Vec<IncludedSpecRef>,
    gate_checks: Vec<PackageGateCheck>,
    findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewFindingInput {
    pub class: String,
    pub summary: String,
    pub blocking: bool,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewResultInput {
    pub mission_id: String,
    pub bundle_id: String,
    pub reviewer: String,
    pub verdict: String,
    #[serde(default)]
    pub target_spec_id: Option<String>,
    #[serde(default)]
    pub governing_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub findings: Vec<ReviewFindingInput>,
    #[serde(default)]
    pub disposition_notes: Vec<String>,
    #[serde(default)]
    pub next_required_branch: Option<NextRequiredBranch>,
    #[serde(default)]
    pub waiting_request: Option<WaitingRequest>,
    #[serde(default)]
    pub review_truth_snapshot: Option<ReviewTruthSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewTruthSnapshot {
    pub mission_id: String,
    pub bundle_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
    pub artifact_fingerprints: BTreeMap<String, String>,
    #[serde(default)]
    pub writeback_authority_verifier: Option<Fingerprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writeback_authority_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerOutputKind {
    None,
    Findings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewerOutputFinding {
    pub severity: String,
    pub title: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub rationale: String,
    pub suggested_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewerOutputInput {
    pub mission_id: String,
    pub bundle_id: String,
    pub reviewer_id: String,
    #[serde(default)]
    pub output_kind: Option<ReviewerOutputKind>,
    #[serde(default)]
    pub findings: Vec<ReviewerOutputFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewerOutputArtifact {
    pub mission_id: String,
    pub bundle_id: String,
    pub reviewer_id: String,
    pub reviewer_output_id: String,
    pub output_kind: ReviewerOutputKind,
    #[serde(default)]
    pub findings: Vec<ReviewerOutputFinding>,
    pub source_evidence_snapshot: String,
    pub source_evidence_snapshot_fingerprint: Fingerprint,
    #[serde(with = "time::serde::rfc3339")]
    pub recorded_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewerOutputReport {
    pub mission_id: String,
    pub bundle_id: String,
    pub reviewer_output_id: String,
    pub evidence_ref: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewResultReport {
    pub mission_id: String,
    pub review_id: String,
    pub blocking_findings: usize,
    pub updated_gate: GateKind,
}

#[derive(Debug, Clone)]
struct ReviewOutcomeContext {
    review_id: String,
    bundle: ReviewBundle,
    resolved_target_spec_id: Option<String>,
    blocking_findings: usize,
    review_passed: bool,
    gate_kind: GateKind,
}

#[derive(Debug, Clone)]
struct ReviewGateUpdate {
    gates: MissionGateIndex,
    updated_gate_id: String,
}

#[derive(Debug, Clone)]
struct ReviewArtifactWriteResult {
    closeout_spec_fingerprint: Option<Fingerprint>,
    expected_outputs: Vec<String>,
}

const ALLOWED_REVIEW_FINDING_CLASSES: &[&str] =
    &["B-Arch", "B-Spec", "B-Proof", "NB-Hardening", "NB-Note"];

const REVIEWER_AGENT_OUTPUT_EVIDENCE_PREFIXES: &[&str] = &[
    "reviewer-output:",
    "reviewer-output/",
    "reviewer-agent-output:",
    "reviewer-agent-output/",
    "child-reviewer-output:",
    "child-reviewer-output/",
];

const REVIEW_WAVE_CONTAMINATION_EVIDENCE_PREFIXES: &[&str] = &[
    "review-wave-contaminated:",
    "review-wave-contamination:",
    "reviewer-lane-contamination:",
    "reviewer-lane-truth-mutation:",
];

const CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV: &str = "CODEX1_PARENT_LOOP_AUTHORITY_TOKEN";
const CODEX1_PARENT_LOOP_BEGIN_ENV: &str = "CODEX1_PARENT_LOOP_BEGIN";

const REQUIRED_MISSION_CLOSE_REVIEW_LENSES: &[&str] = &[
    "spec_conformance",
    "correctness",
    "interface_compatibility",
    "safety_security_policy",
    "operability_rollback_observability",
    "evidence_adequacy",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContradictionInput {
    pub mission_id: String,
    pub discovered_in_phase: String,
    pub discovered_by: String,
    pub target_type: TargetType,
    pub target_id: String,
    pub evidence_refs: Vec<String>,
    pub violated_assumption_or_contract: String,
    pub suggested_reopen_layer: ReopenLayer,
    pub reason_code: String,
    pub governing_revision: String,
    #[serde(default)]
    pub status: Option<ContradictionStatus>,
    #[serde(default)]
    pub triage_decision: Option<TriageDecision>,
    #[serde(default)]
    pub triaged_by: Option<String>,
    #[serde(default)]
    pub machine_action: Option<MachineAction>,
    #[serde(default)]
    pub next_required_branch: Option<NextRequiredBranch>,
    #[serde(default)]
    pub resolution_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionStateInput {
    pub candidate_mission_ids: Vec<String>,
    pub canonical_selection_request: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionResolutionInput {
    pub selected_mission_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaitingRequestAcknowledgementInput {
    pub waiting_request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionAcknowledgementInput {
    pub selection_request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionConsumptionInput {
    pub selection_request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveChildLaneStatus {
    LiveNonFinal,
    FinalSuccess,
    FinalNonSuccess,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveChildLaneSnapshot {
    pub task_path: String,
    pub status: LiveChildLaneStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChildLaneReconciliationClass {
    LiveNonFinal,
    Missing,
    FinalSuccessUnintegrated,
    FinalSuccessIntegrated,
    FinalNonSuccess,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildLaneReconciliationEntry {
    pub task_path: String,
    pub lane_kind: String,
    pub expected_deliverable_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_role: Option<crate::ralph::ChildLaneRole>,
    #[serde(default)]
    pub target_ref: Option<String>,
    pub integration_status: ChildLaneIntegrationStatus,
    pub classification: ChildLaneReconciliationClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildLaneReconciliation {
    pub entries: Vec<ChildLaneReconciliationEntry>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectionOutcome {
    ExplicitMissionOverride,
    PreservedSelectionWait,
    ConsumedResolvedSelection,
    AutoBoundSingleCandidate,
    OpenedSelectionWait,
    NoActiveMission,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeStatus {
    NoActiveMission,
    WaitingSelection,
    WaitingNeedsUser,
    ActionableNonTerminal,
    InterruptedCycle,
    Terminal,
    ContradictoryState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActiveCycleStatus {
    None,
    Interrupted,
    StaleMatchingCloseout,
    StaleSupersededCycle,
    Contradictory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStateAction {
    None,
    Opened,
    Preserved,
    Consumed,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolveResumeInput {
    #[serde(default)]
    pub mission_id: Option<String>,
    #[serde(default)]
    pub live_child_lanes: Vec<LiveChildLaneSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolveResumeReport {
    #[serde(default)]
    pub selected_mission_id: Option<String>,
    pub selection_outcome: SelectionOutcome,
    pub resume_status: ResumeStatus,
    #[serde(default)]
    pub next_phase: Option<String>,
    pub next_action: String,
    #[serde(default)]
    pub latest_closeout_ref: Option<String>,
    pub active_cycle_status: ActiveCycleStatus,
    #[serde(default)]
    pub child_reconciliation: Option<ChildLaneReconciliation>,
    pub selection_state_action: SelectionStateAction,
    #[serde(default)]
    pub state_repairs_applied: Vec<String>,
}

fn default_execution_graph_obligation_status() -> ExecutionGraphObligationStatus {
    ExecutionGraphObligationStatus::Open
}

fn unresolved_contradiction_records(path: &Path) -> Result<Vec<ContradictionRecord>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut records = Vec::new();
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let record: ContradictionRecord = serde_json::from_str(line)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if !matches!(
            record.status,
            ContradictionStatus::Resolved | ContradictionStatus::Dismissed
        ) {
            records.push(record);
        }
    }
    Ok(records)
}

pub fn initialize_mission(
    paths: &MissionPaths,
    input: &MissionInitInput,
) -> Result<MissionBootstrapReport> {
    fs::create_dir_all(paths.mission_root())
        .with_context(|| format!("failed to create {}", paths.mission_root().display()))?;
    fs::create_dir_all(paths.specs_root())
        .with_context(|| format!("failed to create {}", paths.specs_root().display()))?;
    fs::create_dir_all(paths.hidden_mission_root())
        .with_context(|| format!("failed to create {}", paths.hidden_mission_root().display()))?;
    fs::create_dir_all(paths.execution_packages_dir()).with_context(|| {
        format!(
            "failed to create {}",
            paths.execution_packages_dir().display()
        )
    })?;
    fs::create_dir_all(paths.waves_dir())
        .with_context(|| format!("failed to create {}", paths.waves_dir().display()))?;
    fs::create_dir_all(paths.bundles_dir())
        .with_context(|| format!("failed to create {}", paths.bundles_dir().display()))?;
    fs::create_dir_all(paths.receipts_root())
        .with_context(|| format!("failed to create {}", paths.receipts_root().display()))?;
    fs::create_dir_all(paths.packets_dir())
        .with_context(|| format!("failed to create {}", paths.packets_dir().display()))?;

    let mission_id = paths.mission_id().to_owned();
    let root_mission_id = input
        .root_mission_id
        .clone()
        .unwrap_or_else(|| mission_id.clone());
    let slug = input.slug.clone().unwrap_or_else(|| slugify(&input.title));
    let clarify_status = input.clarify_status.unwrap_or(match input.waiting_request {
        Some(_) => ClarifyStatus::WaitingUser,
        None => ClarifyStatus::Clarifying,
    });
    let lock_status = input.lock_status.unwrap_or(match clarify_status {
        ClarifyStatus::Ratified => LockStatus::Locked,
        ClarifyStatus::Superseded => LockStatus::Superseded,
        _ => LockStatus::Draft,
    });
    let lock_posture = input.lock_posture.unwrap_or(LockPosture::Unconstrained);

    let mission_state_doc = ArtifactDocument {
        frontmatter: MissionStateFrontmatter {
            artifact: ArtifactKind::MissionState,
            mission_id: mission_id.clone(),
            root_mission_id: root_mission_id.clone(),
            parent_mission_id: input.parent_mission_id.clone(),
            version: 1,
            clarify_status,
            slug: slug.clone(),
            current_lock_revision: (lock_status == LockStatus::Locked).then_some(1),
            reopened_from_lock_revision: None,
        },
        body: input
            .mission_state_body
            .clone()
            .unwrap_or_else(|| default_mission_state_body(paths, &input.title, &input.objective)),
    };
    let mission_state_fingerprint = mission_state_doc.fingerprint()?;
    let mission_state_rendered = mission_state_doc.render()?;
    fs::write(paths.mission_state(), mission_state_rendered)
        .with_context(|| format!("failed to write {}", paths.mission_state().display()))?;

    let lock_fingerprint = if lock_status == LockStatus::Locked {
        let outcome_lock_doc = ArtifactDocument {
            frontmatter: crate::artifacts::OutcomeLockFrontmatter {
                artifact: ArtifactKind::OutcomeLock,
                mission_id: mission_id.clone(),
                root_mission_id,
                parent_mission_id: input.parent_mission_id.clone(),
                version: 1,
                lock_revision: 1,
                status: lock_status,
                lock_posture,
                slug,
            },
            body: input
                .outcome_lock_body
                .clone()
                .unwrap_or_else(|| default_outcome_lock_body(paths, &input.objective)),
        };
        let lock_rendered = outcome_lock_doc.render()?;
        let lock_fingerprint = outcome_lock_doc.fingerprint()?;
        fs::write(paths.outcome_lock(), lock_rendered)
            .with_context(|| format!("failed to write {}", paths.outcome_lock().display()))?;
        Some(lock_fingerprint)
    } else {
        if paths.outcome_lock().is_file() {
            fs::remove_file(paths.outcome_lock()).with_context(|| {
                format!("failed to remove stale {}", paths.outcome_lock().display())
            })?;
        }
        None
    };

    fs::write(
        paths.readme(),
        input.readme_body.clone().unwrap_or_else(|| {
            default_readme_body(paths, &mission_id, &input.title, &input.objective)
        }),
    )
    .with_context(|| format!("failed to write {}", paths.readme().display()))?;
    ensure_file(paths.review_ledger(), &default_review_ledger_body(paths))?;

    let gate_index = MissionGateIndex {
        mission_id: mission_id.clone(),
        current_phase: if lock_status == LockStatus::Locked {
            "planning".to_string()
        } else {
            "clarify".to_string()
        },
        updated_at: OffsetDateTime::now_utc(),
        gates: initial_gates(&mission_id, lock_status == LockStatus::Locked),
    };
    write_json(paths.gates_json(), &gate_index)?;

    let existing_closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    let next_seq = existing_closeouts
        .last()
        .map_or(1, |record| record.closeout_seq + 1);
    let cycle_id = Uuid::new_v4().to_string();

    let (verdict, terminality, resume_mode, next_phase, next_action) =
        match input.waiting_request.as_ref() {
            Some(waiting) => (
                Verdict::NeedsUser,
                Terminality::WaitingNonTerminal,
                ResumeMode::YieldToUser,
                Some("clarify".to_string()),
                waiting.canonical_request.clone(),
            ),
            None if lock_status == LockStatus::Locked => (
                Verdict::NeedsUser,
                Terminality::WaitingNonTerminal,
                ResumeMode::YieldToUser,
                Some("planning".to_string()),
                input.next_action.clone().unwrap_or_else(|| {
                    "Manual clarify is locked. Wait for the user to invoke $plan explicitly, unless $autopilot owns the workflow.".to_string()
                }),
            ),
            _ => (
                Verdict::ContinueRequired,
                Terminality::ActionableNonTerminal,
                ResumeMode::Continue,
                Some("clarify".to_string()),
                input.next_action.clone().unwrap_or_else(|| {
                    "Continue clarify until the lock is safe to ratify.".to_string()
                }),
            ),
        };

    let closeout = CloseoutRecord {
        closeout_id: Some(Uuid::new_v4().to_string()),
        closeout_seq: next_seq,
        mission_id: mission_id.clone(),
        phase: if lock_status == LockStatus::Locked {
            "clarify".to_string()
        } else {
            "clarify".to_string()
        },
        activity: "mission_bootstrap".to_string(),
        verdict: verdict.clone(),
        terminality,
        resume_mode,
        next_phase,
        next_action: next_action.clone(),
        target: Some(format!("mission:{}", mission_id)),
        cycle_kind: Some(CycleKind::BoundedProgress),
        lock_revision: (lock_status == LockStatus::Locked).then_some(1),
        lock_fingerprint: lock_fingerprint.as_ref().map(ToString::to_string),
        blueprint_revision: None,
        blueprint_fingerprint: None,
        governing_revision: Some(if lock_status == LockStatus::Locked {
            "lock:1".to_string()
        } else {
            "clarify:mission_state".to_string()
        }),
        reason_code: Some(input.reason_code.clone().unwrap_or_else(|| {
            if input.waiting_request.is_some() {
                "clarify_waiting_on_user"
            } else if lock_status == LockStatus::Locked {
                "manual_clarify_waiting_for_plan_invocation"
            } else {
                "mission_bootstrapped"
            }
            .to_string()
        })),
        summary: Some(
            input
                .summary
                .clone()
                .unwrap_or_else(|| format!("Bootstrapped mission {}", mission_id)),
        ),
        continuation_prompt: Some(next_action),
        cycle_id: Some(cycle_id),
        waiting_request_id: if lock_status == LockStatus::Locked && input.waiting_request.is_none()
        {
            Some(Uuid::new_v4().to_string())
        } else {
            input
                .waiting_request
                .as_ref()
                .map(|_| Uuid::new_v4().to_string())
        },
        waiting_for: if lock_status == LockStatus::Locked && input.waiting_request.is_none() {
            Some("manual_plan_invocation".to_string())
        } else {
            input
                .waiting_request
                .as_ref()
                .map(|waiting| waiting.waiting_for.clone())
        },
        canonical_waiting_request: if lock_status == LockStatus::Locked
            && input.waiting_request.is_none()
        {
            Some("Invoke $plan manually when ready, or invoke $autopilot when you want Codex1 to continue automatically from clarify into planning.".to_string())
        } else {
            input
                .waiting_request
                .as_ref()
                .map(|waiting| waiting.canonical_request.clone())
        },
        resume_condition: if lock_status == LockStatus::Locked && input.waiting_request.is_none() {
            Some("The user explicitly invokes $plan, or an autopilot-owned workflow resumes this mission.".to_string())
        } else {
            input
                .waiting_request
                .as_ref()
                .map(|waiting| waiting.resume_condition.clone())
        },
        request_emitted_at: None,
        active_child_task_paths: Vec::new(),
        artifact_fingerprints: {
            let mut fingerprints = BTreeMap::from([(
                "mission_state".to_string(),
                mission_state_fingerprint.to_string(),
            )]);
            if let Some(lock_fingerprint) = lock_fingerprint.as_ref() {
                fingerprints.insert("outcome_lock".to_string(), lock_fingerprint.to_string());
            }
            fingerprints
        },
    };
    let mut preconditions_checked = vec!["mission_state_written".to_string()];
    let mut expected_outputs = vec![paths.mission_state().display().to_string()];
    if lock_status == LockStatus::Locked {
        preconditions_checked.push("outcome_lock_locked".to_string());
        expected_outputs.push(paths.outcome_lock().display().to_string());
    }
    let active_cycle = active_cycle_from_closeout(
        &closeout,
        Vec::new(),
        preconditions_checked,
        expected_outputs,
        Vec::new(),
        Vec::new(),
    );
    append_closeout_for_active_cycle(paths, &closeout, &active_cycle)?;

    Ok(MissionBootstrapReport {
        mission_id,
        mission_root: paths.mission_root(),
        hidden_root: paths.hidden_mission_root(),
        lock_fingerprint,
        clarify_status,
        lock_status,
        closeout_seq: next_seq,
    })
}

pub fn write_planning_artifacts(
    paths: &MissionPaths,
    input: &PlanningWriteInput,
) -> Result<PlanningWriteReport> {
    let context = prepare_planning_write_context(paths, input)?;
    fs::write(paths.program_blueprint(), &context.blueprint_rendered)
        .with_context(|| format!("failed to write {}", paths.program_blueprint().display()))?;
    supersede_omitted_planning_specs(paths, &context)?;
    let spec_sync = sync_planning_specs(paths, input, &context)?;
    sync_planning_execution_graph(paths, &context)?;
    refresh_planning_runtime_state(
        paths,
        input,
        &context,
        context.planning_contract_changed || spec_sync.planning_contract_changed,
    )?;
    let closeout = build_planning_closeout(paths, input, &context)?;
    let mut expected_outputs = vec![paths.program_blueprint().display().to_string()];
    if context.normalized_execution_graph.is_some() {
        expected_outputs.push(paths.execution_graph().display().to_string());
    }
    expected_outputs.extend(
        spec_sync
            .written_specs
            .iter()
            .map(|spec| paths.spec_file(&spec.spec_id).display().to_string()),
    );
    let active_cycle = active_cycle_from_closeout(
        &closeout,
        Vec::new(),
        vec![
            "outcome_lock_locked".to_string(),
            "planning_artifacts_rendered".to_string(),
        ],
        expected_outputs,
        Vec::new(),
        Vec::new(),
    );
    append_closeout_for_active_cycle(paths, &closeout, &active_cycle)?;

    Ok(PlanningWriteReport {
        mission_id: input.mission_id.clone(),
        blueprint_revision: context.blueprint_revision,
        blueprint_fingerprint: context.blueprint_fingerprint,
        written_specs: spec_sync.written_specs,
    })
}

fn prepare_planning_write_context(
    paths: &MissionPaths,
    input: &PlanningWriteInput,
) -> Result<PlanningWriteContext> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    if !(1..=5).contains(&input.plan_level) {
        bail!("plan_level must be between 1 and 5");
    }
    if !paths.outcome_lock().is_file() {
        anyhow::bail!(
            "cannot write planning artifacts for mission {} until the outcome lock is locked",
            input.mission_id
        );
    }
    let lock_doc =
        load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())?;
    if lock_doc.frontmatter.status != LockStatus::Locked {
        anyhow::bail!(
            "cannot write planning artifacts for mission {} until the outcome lock is locked",
            input.mission_id
        );
    }
    let lock_fingerprint = lock_doc.fingerprint()?;
    let existing_blueprint = if paths.program_blueprint().is_file() {
        Some(load_markdown::<ProgramBlueprintFrontmatter>(
            &paths.program_blueprint(),
        )?)
    } else {
        None
    };
    let existing_execution_graph = load_execution_graph(paths)?;
    let existing_execution_graph_contract = existing_execution_graph
        .as_ref()
        .map(execution_graph_to_contract_input);
    let existing_proof_matrix = existing_blueprint
        .as_ref()
        .map(|existing| normalize_proof_matrix(&existing.frontmatter.proof_matrix))
        .unwrap_or_default();
    let existing_decision_obligations = existing_blueprint
        .as_ref()
        .map(|existing| normalize_decision_obligations(&existing.frontmatter.decision_obligations))
        .unwrap_or_default();
    let prior_active_spec_ids = existing_blueprint
        .as_ref()
        .map(|existing| {
            load_active_blueprint_spec_ids(paths, existing.frontmatter.blueprint_revision)
        })
        .transpose()?
        .unwrap_or_default();
    let input_spec_ids = unique_strings(
        &input
            .specs
            .iter()
            .map(|spec| spec.spec_id.clone())
            .collect::<Vec<_>>(),
    );
    let frontier_changed =
        !prior_active_spec_ids.is_empty() && prior_active_spec_ids != input_spec_ids;
    let mut blueprint_revision = next_blueprint_revision(
        existing_blueprint.as_ref(),
        input,
        lock_doc.frontmatter.lock_revision,
    );
    if frontier_changed
        && let Some(existing) = existing_blueprint.as_ref()
        && blueprint_revision == existing.frontmatter.blueprint_revision
    {
        blueprint_revision += 1;
    }
    let mut planning_contract_changed = existing_blueprint
        .as_ref()
        .map(|existing| existing.frontmatter.blueprint_revision != blueprint_revision)
        .unwrap_or(true);
    planning_contract_changed |= frontier_changed;
    let normalized_proof_matrix = normalize_proof_matrix(&input.proof_matrix);
    validate_proof_matrix(&normalized_proof_matrix)?;
    let normalized_decision_obligations =
        normalize_decision_obligations(&input.decision_obligations);
    validate_decision_obligations(&normalized_decision_obligations)?;
    let runnable_spec_ids_hint = runnable_spec_ids_from_inputs(&input.specs);
    let risk_floor = compute_planning_risk_floor(
        &lock_doc.body,
        input.problem_size,
        runnable_spec_ids_hint.len(),
        input.selected_target_ref.as_deref(),
    );
    if input.plan_level < risk_floor {
        bail!(
            "plan_level {} is below the computed risk floor {} for mission {}",
            input.plan_level,
            risk_floor,
            input.mission_id
        );
    }
    if input.status.unwrap_or(BlueprintStatus::Draft) == BlueprintStatus::Approved
        && !runnable_spec_ids_hint.is_empty()
        && normalized_proof_matrix.is_empty()
    {
        bail!(
            "approved planning for mission {} requires a non-empty proof_matrix for the runnable frontier",
            input.mission_id
        );
    }
    if input.status.unwrap_or(BlueprintStatus::Draft) == BlueprintStatus::Approved
        && normalized_decision_obligations
            .iter()
            .any(decision_obligation_blocks_planning_completion)
    {
        bail!(
            "approved planning for mission {} still has blocking decision obligations",
            input.mission_id
        );
    }
    let graph_required_hint = execution_graph_required(
        runnable_spec_ids_hint.len(),
        input.selected_target_ref.as_deref(),
    );
    let normalized_execution_graph = match (graph_required_hint, input.execution_graph.as_ref()) {
        (true, Some(execution_graph)) => Some(normalize_execution_graph_input(execution_graph)),
        (true, None) => {
            bail!(
                "planning input for mission {} requires execution_graph when more than one runnable spec or a wave target exists",
                input.mission_id
            );
        }
        (false, Some(execution_graph)) => Some(normalize_execution_graph_input(execution_graph)),
        (false, None) => None,
    };
    validate_blueprint_body_contract(
        &input.body_markdown,
        input.status.unwrap_or(BlueprintStatus::Draft),
        graph_required_hint,
    )?;
    if existing_proof_matrix != normalized_proof_matrix {
        planning_contract_changed = true;
    }
    if existing_decision_obligations != normalized_decision_obligations {
        planning_contract_changed = true;
    }
    if existing_execution_graph_contract != normalized_execution_graph {
        planning_contract_changed = true;
    }
    let blueprint_doc = ArtifactDocument {
        frontmatter: ProgramBlueprintFrontmatter {
            artifact: ArtifactKind::ProgramBlueprint,
            mission_id: input.mission_id.clone(),
            version: 1,
            lock_revision: lock_doc.frontmatter.lock_revision,
            blueprint_revision,
            plan_level: input.plan_level,
            risk_floor,
            problem_size: input.problem_size,
            status: input.status.unwrap_or(BlueprintStatus::Draft),
            proof_matrix: normalized_proof_matrix,
            decision_obligations: normalized_decision_obligations,
            selected_target_ref: input.selected_target_ref.clone(),
        },
        body: input.body_markdown.clone(),
    };
    let blueprint_rendered = blueprint_doc.render()?;
    let blueprint_fingerprint = compute_blueprint_contract_fingerprint(
        &blueprint_doc,
        normalized_execution_graph.as_ref(),
    )?;

    Ok(PlanningWriteContext {
        lock_revision: lock_doc.frontmatter.lock_revision,
        lock_body: lock_doc.body,
        lock_fingerprint,
        blueprint_revision,
        blueprint_rendered,
        blueprint_fingerprint,
        planning_contract_changed,
        normalized_execution_graph,
        prior_active_spec_ids,
        input_spec_ids,
    })
}

fn supersede_omitted_planning_specs(
    paths: &MissionPaths,
    context: &PlanningWriteContext,
) -> Result<()> {
    let omitted_active_spec_ids = context
        .prior_active_spec_ids
        .iter()
        .filter(|spec_id| {
            !context
                .input_spec_ids
                .iter()
                .any(|candidate| candidate == *spec_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    for spec_id in &omitted_active_spec_ids {
        supersede_omitted_spec(
            paths,
            spec_id,
            context.blueprint_revision,
            &context.blueprint_fingerprint,
        )?;
    }
    Ok(())
}

fn sync_planning_specs(
    paths: &MissionPaths,
    input: &PlanningWriteInput,
    context: &PlanningWriteContext,
) -> Result<PlanningSpecSyncResult> {
    let mut written_specs = Vec::new();
    let mut planning_contract_changed = false;
    for spec in &input.specs {
        let body = spec
            .body_markdown
            .clone()
            .unwrap_or_else(|| default_spec_body(paths, &spec.spec_id, &spec.purpose));
        let artifact_status = spec.artifact_status.unwrap_or(SpecArtifactStatus::Draft);
        let packetization_status = spec
            .packetization_status
            .unwrap_or(PacketizationStatus::NearFrontier);
        let execution_status = spec
            .execution_status
            .unwrap_or(SpecExecutionStatus::NotStarted);
        validate_spec_body_contract(&spec.spec_id, &body, &spec.purpose)?;
        validate_planning_spec_state(
            &spec.spec_id,
            artifact_status,
            packetization_status,
            execution_status,
        )?;
        if !paths.spec_file(&spec.spec_id).is_file() {
            planning_contract_changed = true;
        }
        let spec_revision = next_spec_revision(paths, spec, &body)?;
        if !planning_contract_changed
            && let Ok(existing_spec) =
                load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&spec.spec_id))
            && existing_spec.frontmatter.spec_revision != spec_revision
        {
            planning_contract_changed = true;
        }
        let doc = ArtifactDocument {
            frontmatter: WorkstreamSpecFrontmatter {
                artifact: ArtifactKind::WorkstreamSpec,
                mission_id: input.mission_id.clone(),
                spec_id: spec.spec_id.clone(),
                version: 1,
                spec_revision,
                artifact_status,
                packetization_status,
                execution_status,
                owner_mode: spec.owner_mode.unwrap_or(OwnerMode::Solo),
                blueprint_revision: context.blueprint_revision,
                blueprint_fingerprint: Some(context.blueprint_fingerprint.clone()),
                spec_fingerprint: None,
                replan_boundary: Some(spec.replan_boundary.clone().unwrap_or_default()),
            },
            body,
        };
        let rendered = doc.render()?;
        let spec_root = paths.spec_root(&spec.spec_id);
        fs::create_dir_all(paths.receipts_dir(&spec.spec_id)).with_context(|| {
            format!(
                "failed to create {}",
                paths.receipts_dir(&spec.spec_id).display()
            )
        })?;
        fs::create_dir_all(&spec_root)
            .with_context(|| format!("failed to create {}", spec_root.display()))?;
        fs::write(paths.spec_file(&spec.spec_id), &rendered).with_context(|| {
            format!(
                "failed to write {}",
                paths.spec_file(&spec.spec_id).display()
            )
        })?;
        ensure_file(
            paths.review_file(&spec.spec_id),
            &default_spec_review_body(paths, &spec.spec_id),
        )?;
        ensure_file(
            paths.notes_file(&spec.spec_id),
            &default_spec_notes_body(paths, &spec.spec_id),
        )?;
        ensure_file(
            paths.receipts_dir(&spec.spec_id).join("README.md"),
            &default_receipts_readme_body(paths, &spec.spec_id),
        )?;

        let parsed = ArtifactDocument::<WorkstreamSpecFrontmatter>::parse(&rendered)?;
        written_specs.push(IncludedSpecRef {
            spec_id: spec.spec_id.clone(),
            spec_revision: parsed.frontmatter.spec_revision,
            spec_fingerprint: parsed.fingerprint()?,
        });
    }
    Ok(PlanningSpecSyncResult {
        written_specs,
        planning_contract_changed,
    })
}

fn sync_planning_execution_graph(
    paths: &MissionPaths,
    context: &PlanningWriteContext,
) -> Result<()> {
    let runnable_spec_ids = load_runnable_blueprint_spec_ids(paths, context.blueprint_revision)?;
    if let Some(execution_graph_input) = context.normalized_execution_graph.as_ref() {
        let execution_graph = build_execution_graph(
            paths,
            context.blueprint_revision,
            &context.blueprint_fingerprint,
            execution_graph_input,
            &runnable_spec_ids,
        )?;
        write_json(paths.execution_graph(), &execution_graph)?;
    } else if paths.execution_graph().is_file() {
        fs::remove_file(paths.execution_graph()).with_context(|| {
            format!(
                "failed to remove stale execution graph {}",
                paths.execution_graph().display()
            )
        })?;
    }
    Ok(())
}

fn refresh_planning_runtime_state(
    paths: &MissionPaths,
    input: &PlanningWriteInput,
    context: &PlanningWriteContext,
    planning_contract_changed: bool,
) -> Result<()> {
    let mut gates = load_gate_index(paths)?;
    let planning_gate_id = planning_gate_id(&input.mission_id, context.blueprint_revision);
    invalidate_post_planning_history(
        &mut gates,
        &input.mission_id,
        &planning_gate_id,
        planning_contract_changed,
    );
    append_gate(
        &mut gates,
        MissionGateRecord {
            gate_id: planning_gate_id,
            gate_kind: GateKind::PlanningCompletion,
            target_ref: format!("mission:{}", input.mission_id),
            governing_refs: vec![
                format!("lock:{}", context.lock_revision),
                format!("blueprint:{}", context.blueprint_revision),
            ],
            status: MissionGateStatus::Open,
            blocking: true,
            opened_at: OffsetDateTime::now_utc(),
            evaluated_at: None,
            evaluated_against_ref: None,
            evidence_refs: {
                let mut refs = vec![paths.program_blueprint().display().to_string()];
                if context.normalized_execution_graph.is_some() {
                    refs.push(paths.execution_graph().display().to_string());
                }
                refs
            },
            failure_refs: Vec::new(),
            superseded_by: None,
        },
    );
    gates.current_phase = if input.selected_target_ref.is_some() {
        "execution_package".to_string()
    } else {
        "planning".to_string()
    };
    gates.updated_at = OffsetDateTime::now_utc();
    write_json(paths.gates_json(), &gates)?;

    if let Some(target_ref) = &input.selected_target_ref {
        let body = format!(
            "# {}\n\n## Snapshot\n\n- Mission id: `{}`\n- Current phase: `execution_package`\n- Selected target: `{}`\n- Objective: {}\n",
            paths.mission_id(),
            paths.mission_id(),
            target_ref,
            extract_first_heading_or_sentence(&context.lock_body)
        );
        fs::write(paths.readme(), body)
            .with_context(|| format!("failed to write {}", paths.readme().display()))?;
    }

    Ok(())
}

fn build_planning_closeout(
    paths: &MissionPaths,
    input: &PlanningWriteInput,
    context: &PlanningWriteContext,
) -> Result<CloseoutRecord> {
    let existing_closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    let next_seq = existing_closeouts
        .last()
        .map_or(1, |record| record.closeout_seq + 1);
    let cycle_id = Uuid::new_v4().to_string();

    Ok(CloseoutRecord {
        closeout_id: Some(Uuid::new_v4().to_string()),
        closeout_seq: next_seq,
        mission_id: input.mission_id.clone(),
        phase: "planning".to_string(),
        activity: "blueprint_writeback".to_string(),
        verdict: Verdict::ContinueRequired,
        terminality: Terminality::ActionableNonTerminal,
        resume_mode: ResumeMode::Continue,
        next_phase: Some(
            if input.selected_target_ref.is_some() {
                "execution_package"
            } else {
                "planning"
            }
            .to_string(),
        ),
        next_action: input.next_action.clone().unwrap_or_else(|| {
            if input.selected_target_ref.is_some() {
                "Compile or refresh the next execution package.".to_string()
            } else {
                "Select the next execution target and keep planning.".to_string()
            }
        }),
        target: input.selected_target_ref.clone(),
        cycle_kind: Some(CycleKind::BoundedProgress),
        lock_revision: Some(context.lock_revision),
        lock_fingerprint: Some(context.lock_fingerprint.to_string()),
        blueprint_revision: Some(context.blueprint_revision),
        blueprint_fingerprint: Some(context.blueprint_fingerprint.to_string()),
        governing_revision: Some(format!("blueprint:{}", context.blueprint_revision)),
        reason_code: Some("planning_artifacts_written".to_string()),
        summary: Some(format!(
            "Updated blueprint revision {}",
            context.blueprint_revision
        )),
        continuation_prompt: Some(if input.selected_target_ref.is_some() {
            "Compile or refresh the execution package for the selected target.".to_string()
        } else {
            "Planning is still open; select the next execution target before packaging.".to_string()
        }),
        cycle_id: Some(cycle_id),
        waiting_request_id: None,
        waiting_for: None,
        canonical_waiting_request: None,
        resume_condition: None,
        request_emitted_at: None,
        active_child_task_paths: Vec::new(),
        artifact_fingerprints: BTreeMap::from([
            (
                "outcome_lock".to_string(),
                context.lock_fingerprint.to_string(),
            ),
            (
                "program_blueprint".to_string(),
                context.blueprint_fingerprint.to_string(),
            ),
        ]),
    })
}

pub fn compile_execution_package(
    paths: &MissionPaths,
    input: &ExecutionPackageInput,
) -> Result<ExecutionPackage> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    validate_parent_loop_authority_for_mission_mutation(paths, "compile-execution-package")?;
    let spec_contexts = load_spec_contexts(paths, &input.included_spec_ids)?;
    let lock_doc =
        load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())?;
    let blueprint_doc = load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())?;
    let lock_fingerprint = lock_doc.fingerprint()?;
    let blueprint_fingerprint = current_blueprint_contract_fingerprint(paths, &blueprint_doc)?;
    let dependency_snapshot_fingerprint =
        dependency_snapshot_fingerprint(paths, &input.dependency_satisfaction_state)?;
    let execution_graph = load_execution_graph(paths)?;
    let normalized_wave_specs = if input.target_type == TargetType::Wave
        && input.wave_specs.is_empty()
    {
        execution_graph
            .as_ref()
            .map(|graph| derive_wave_specs_from_execution_graph(graph, &input.included_spec_ids))
            .unwrap_or_default()
    } else {
        normalize_wave_specs(&input.wave_specs)
    };
    let (resolved_replan_boundary, replan_boundary_findings) =
        derive_package_replan_boundary(&spec_contexts, input.replan_boundary.as_ref());
    let wave_contract = match input.target_type {
        TargetType::Wave => Some(build_wave_manifest(
            paths,
            &input.mission_id,
            &input.target_id,
            &input.included_spec_ids,
            &input.read_scope,
            &input.write_scope,
            &normalized_wave_specs,
        )?),
        _ => None,
    };
    let mut evaluation = evaluate_execution_package_contract(
        paths,
        &blueprint_doc,
        &input.target_type,
        &input.target_id,
        &input.included_spec_ids,
        &input.dependency_satisfaction_state,
        &input.read_scope,
        &input.write_scope,
        &input.proof_obligations,
        &input.review_obligations,
        &normalized_wave_specs,
        input.wave_context.as_deref(),
        wave_contract.as_ref().map(|(_, fingerprint)| fingerprint),
        Some(&input.gate_checks),
    )?;
    evaluation.findings.extend(replan_boundary_findings);
    if let (Some((_, computed_wave_fingerprint)), Some(provided_wave_fingerprint)) =
        (wave_contract.as_ref(), input.wave_fingerprint.as_ref())
        && provided_wave_fingerprint != computed_wave_fingerprint
    {
        evaluation
            .findings
            .push("wave_fingerprint_mismatch_with_manifest".to_string());
    }

    let status = if evaluation.findings.is_empty()
        && evaluation.gate_checks.iter().all(|gate| gate.passed)
    {
        ExecutionPackageStatus::Passed
    } else {
        ExecutionPackageStatus::Failed
    };
    let mut included_specs = evaluation.included_specs.clone();
    if status == ExecutionPackageStatus::Passed && input.target_type != TargetType::Mission {
        for included in &evaluation.included_specs {
            let mut spec_doc =
                load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&included.spec_id))?;
            if spec_doc.frontmatter.execution_status != SpecExecutionStatus::Packaged {
                spec_doc.frontmatter.execution_status = SpecExecutionStatus::Packaged;
                fs::write(paths.spec_file(&included.spec_id), spec_doc.render()?).with_context(
                    || {
                        format!(
                            "failed to write {}",
                            paths.spec_file(&included.spec_id).display()
                        )
                    },
                )?;
            }
        }
        included_specs = load_spec_contexts(paths, &input.included_spec_ids)?
            .into_iter()
            .map(|context| context.included)
            .collect();
    }
    let package = ExecutionPackage {
        package_id: Uuid::new_v4().to_string(),
        mission_id: input.mission_id.clone(),
        target_type: input.target_type.clone(),
        target_id: input.target_id.clone(),
        lock_revision: lock_doc.frontmatter.lock_revision,
        lock_fingerprint,
        blueprint_revision: blueprint_doc.frontmatter.blueprint_revision,
        blueprint_fingerprint,
        dependency_snapshot_fingerprint,
        wave_fingerprint: wave_contract
            .as_ref()
            .map(|(_, fingerprint)| fingerprint.clone()),
        included_specs,
        dependency_satisfaction_state: input.dependency_satisfaction_state.clone(),
        read_scope: unique_strings(&input.read_scope),
        write_scope: unique_strings(&input.write_scope),
        proof_obligations: unique_strings(&input.proof_obligations),
        review_obligations: unique_strings(&input.review_obligations),
        replan_boundary: resolved_replan_boundary.unwrap_or_default(),
        wave_context: input.wave_context.clone(),
        wave_specs: normalized_wave_specs,
        gate_checks: evaluation.gate_checks,
        validation_failures: evaluation.findings.clone(),
        validated_at: OffsetDateTime::now_utc(),
        status: status.clone(),
    };
    if let Some((wave_manifest, _)) = wave_contract.as_ref() {
        write_json(paths.wave_manifest(&wave_manifest.wave_id), &wave_manifest)?;
    }
    let package_path = paths.execution_package(&package.package_id);
    write_json(&package_path, &package)?;

    let mut gates = load_gate_index(paths)?;
    let gate_status = if package.status == ExecutionPackageStatus::Passed {
        MissionGateStatus::Passed
    } else {
        MissionGateStatus::Failed
    };
    let target_ref = gate_target_ref(&input.target_type, &input.target_id);
    let execution_gate_id = execution_gate_id(&package);
    supersede_matching_gates(
        &mut gates,
        GateKind::ExecutionPackage,
        &target_ref,
        &execution_gate_id,
    );
    invalidate_review_history_for_execution_target(&mut gates, &package);
    append_gate(
        &mut gates,
        MissionGateRecord {
            gate_id: execution_gate_id,
            gate_kind: GateKind::ExecutionPackage,
            target_ref: target_ref.clone(),
            governing_refs: vec![
                format!("lock:{}", package.lock_revision),
                format!("blueprint:{}", package.blueprint_revision),
            ],
            status: gate_status,
            blocking: true,
            opened_at: OffsetDateTime::now_utc(),
            evaluated_at: Some(package.validated_at),
            evaluated_against_ref: Some(package.package_id.clone()),
            evidence_refs: vec![package_path.display().to_string()],
            failure_refs: package.validation_failures.clone(),
            superseded_by: None,
        },
    );
    sync_planning_completion_gate(
        &mut gates,
        &package.mission_id,
        package.blueprint_revision,
        package.status == ExecutionPackageStatus::Passed,
        &package.package_id,
        &package_path,
        &package.validation_failures,
    );
    gates.current_phase = if package.status == ExecutionPackageStatus::Passed {
        "execution".to_string()
    } else {
        "execution_package".to_string()
    };
    gates.updated_at = OffsetDateTime::now_utc();
    write_json(paths.gates_json(), &gates)?;

    let existing_closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    let next_seq = existing_closeouts
        .last()
        .map_or(1, |record| record.closeout_seq + 1);
    let cycle_id = Uuid::new_v4().to_string();
    let (verdict, next_phase, reason_code, summary) =
        if package.status == ExecutionPackageStatus::Passed {
            (
                Verdict::ContinueRequired,
                Some("execution".to_string()),
                "execution_package_passed",
                format!("Execution package {} passed", package.package_id),
            )
        } else {
            (
                Verdict::ContinueRequired,
                Some("execution_package".to_string()),
                "execution_package_failed",
                format!("Execution package {} failed", package.package_id),
            )
        };
    let closeout = CloseoutRecord {
        closeout_id: Some(Uuid::new_v4().to_string()),
        closeout_seq: next_seq,
        mission_id: input.mission_id.clone(),
        phase: "execution_package".to_string(),
        activity: "package_gate_evaluation".to_string(),
        verdict: verdict.clone(),
        terminality: Terminality::ActionableNonTerminal,
        resume_mode: ResumeMode::Continue,
        next_phase,
        next_action: if package.status == ExecutionPackageStatus::Passed {
            format!(
                "Execute target {}:{} from package {}.",
                input.target_type.as_phase_target(),
                input.target_id,
                package.package_id
            )
        } else {
            format!(
                "Repair planning truth or repackage target {}:{}.",
                input.target_type.as_phase_target(),
                input.target_id
            )
        },
        target: Some(format!(
            "{}:{}",
            input.target_type.as_phase_target(),
            input.target_id
        )),
        cycle_kind: Some(CycleKind::GateEvaluation),
        lock_revision: Some(package.lock_revision),
        lock_fingerprint: Some(package.lock_fingerprint.to_string()),
        blueprint_revision: Some(package.blueprint_revision),
        blueprint_fingerprint: Some(package.blueprint_fingerprint.to_string()),
        governing_revision: Some(format!("package:{}", package.package_id)),
        reason_code: Some(reason_code.to_string()),
        summary: Some(summary),
        continuation_prompt: Some(if package.status == ExecutionPackageStatus::Passed {
            format!("Run execution from package {}.", package.package_id)
        } else {
            format!(
                "Do not execute; package {} is not valid.",
                package.package_id
            )
        }),
        cycle_id: Some(cycle_id),
        waiting_request_id: None,
        waiting_for: None,
        canonical_waiting_request: None,
        resume_condition: None,
        request_emitted_at: None,
        active_child_task_paths: Vec::new(),
        artifact_fingerprints: BTreeMap::from([
            (
                "outcome_lock".to_string(),
                package.lock_fingerprint.to_string(),
            ),
            (
                "program_blueprint".to_string(),
                package.blueprint_fingerprint.to_string(),
            ),
        ]),
    };
    let mut expected_outputs = vec![package_path.display().to_string()];
    if let Some((wave_manifest, _)) = wave_contract.as_ref() {
        expected_outputs.push(
            paths
                .wave_manifest(&wave_manifest.wave_id)
                .display()
                .to_string(),
        );
    }
    let active_cycle = active_cycle_from_closeout(
        &closeout,
        Vec::new(),
        vec![
            "package_target_resolved".to_string(),
            "package_gate_evaluated".to_string(),
        ],
        expected_outputs,
        vec![package.package_id.clone()],
        Vec::new(),
    );
    append_closeout_for_active_cycle(paths, &closeout, &active_cycle)?;

    Ok(package)
}

fn default_dependency_governing_ref(name: &str) -> Option<String> {
    match name {
        "lock_current" => Some("outcome_lock".to_string()),
        "blueprint_current" => Some("program_blueprint".to_string()),
        spec_ref
            if spec_ref
                .strip_prefix("spec:")
                .is_some_and(|spec_id| !spec_id.trim().is_empty()) =>
        {
            Some(spec_ref.to_string())
        }
        _ => None,
    }
}

fn current_dependency_fingerprint(
    paths: &MissionPaths,
    governing_ref: &str,
) -> Result<Option<Fingerprint>> {
    match governing_ref {
        "outcome_lock" => Ok(Some(
            load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())?
                .fingerprint()?,
        )),
        "program_blueprint" => {
            let blueprint_doc =
                load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())?;
            Ok(Some(current_blueprint_contract_fingerprint(
                paths,
                &blueprint_doc,
            )?))
        }
        ref_name if ref_name.starts_with("spec:") => {
            let spec_id = ref_name.trim_start_matches("spec:");
            Ok(Some(
                load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(spec_id))?
                    .fingerprint()?,
            ))
        }
        _ => Ok(None),
    }
}

#[derive(Serialize)]
struct DependencyFingerprintBinding {
    name: String,
    satisfied: bool,
    detail: String,
    governing_ref: String,
    governing_fingerprint: String,
}

fn dependency_snapshot_fingerprint(
    paths: &MissionPaths,
    dependencies: &[DependencyCheck],
) -> Result<Fingerprint> {
    let bindings = dependencies
        .iter()
        .map(|dependency| {
            let governing_ref = default_dependency_governing_ref(&dependency.name)
                .unwrap_or_else(|| format!("unbound:{}", dependency.name));
            let governing_fingerprint = current_dependency_fingerprint(paths, &governing_ref)?
                .map(|fingerprint| fingerprint.to_string())
                .unwrap_or_else(|| "unbound".to_string());
            Ok(DependencyFingerprintBinding {
                name: dependency.name.clone(),
                satisfied: dependency.satisfied,
                detail: dependency.detail.clone(),
                governing_ref,
                governing_fingerprint,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    fingerprint_json(&bindings)
}

pub fn validate_execution_package(
    paths: &MissionPaths,
    package_id: &str,
) -> Result<PackageValidationReport> {
    let package_path = paths.execution_package(package_id);
    let package: ExecutionPackage = load_json(&package_path)?;
    let spec_contexts = load_spec_contexts(
        paths,
        &package
            .included_specs
            .iter()
            .map(|included| included.spec_id.clone())
            .collect::<Vec<_>>(),
    )?;
    let gates = load_gate_index(paths)?;
    let current_lock =
        load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())?;
    let current_blueprint =
        load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())?;
    let current_lock_fp = current_lock.fingerprint()?;
    let current_blueprint_fp = current_blueprint_contract_fingerprint(paths, &current_blueprint)?;
    let evaluation = evaluate_execution_package_contract(
        paths,
        &current_blueprint,
        &package.target_type,
        &package.target_id,
        &package
            .included_specs
            .iter()
            .map(|included| included.spec_id.clone())
            .collect::<Vec<_>>(),
        &package.dependency_satisfaction_state,
        &package.read_scope,
        &package.write_scope,
        &package.proof_obligations,
        &package.review_obligations,
        &package.wave_specs,
        package.wave_context.as_deref(),
        package.wave_fingerprint.as_ref(),
        Some(&package.gate_checks),
    )?;
    let mut findings = evaluation.findings;
    if package.mission_id != paths.mission_id() {
        findings.push("package_mission_id_mismatch".to_string());
    }

    if package.status != ExecutionPackageStatus::Passed {
        findings.push(format!(
            "package_status_not_executable:{:?}",
            package.status
        ));
    }
    if package.lock_fingerprint != current_lock_fp {
        findings.push("lock_fingerprint_mismatch".to_string());
    }
    if package.blueprint_fingerprint != current_blueprint_fp {
        findings.push("blueprint_fingerprint_mismatch".to_string());
    }
    let normalized_current_included_specs = package
        .included_specs
        .iter()
        .map(|included| normalize_current_included_spec_for_package_validation(paths, included))
        .collect::<Result<Vec<_>>>()?;
    if package.included_specs != normalized_current_included_specs {
        findings.push("included_spec_set_mismatch".to_string());
    }
    let (resolved_replan_boundary, replan_boundary_findings) =
        derive_package_replan_boundary(&spec_contexts, Some(&package.replan_boundary));
    findings.extend(replan_boundary_findings);
    if resolved_replan_boundary.as_ref() != Some(&package.replan_boundary) {
        findings.push("package_replan_boundary_invalid".to_string());
    }
    if dependency_snapshot_fingerprint(paths, &package.dependency_satisfaction_state)?
        != package.dependency_snapshot_fingerprint
    {
        findings.push("dependency_snapshot_fingerprint_mismatch".to_string());
    }
    for dependency in &package.dependency_satisfaction_state {
        let Some(governing_ref) = default_dependency_governing_ref(&dependency.name) else {
            findings.push(format!(
                "dependency_governing_ref_missing:{}",
                dependency.name
            ));
            continue;
        };
        match current_dependency_fingerprint(paths, &governing_ref)? {
            Some(_) => {}
            None => findings.push(format!(
                "dependency_governing_ref_unknown:{}",
                dependency.name
            )),
        }
    }
    if package.target_type == TargetType::Wave {
        let wave_manifest_path = paths.wave_manifest(&package.target_id);
        if !wave_manifest_path.is_file() {
            findings.push("wave_manifest_missing".to_string());
        } else {
            let manifest: WaveManifest = load_json(&wave_manifest_path)?;
            let manifest_fingerprint = Fingerprint::from_json(&manifest)?;
            if package.wave_fingerprint.as_ref() != Some(&manifest_fingerprint) {
                findings.push("wave_manifest_fingerprint_mismatch".to_string());
            }
            if manifest.included_specs != package.included_specs {
                findings.push("wave_manifest_included_specs_mismatch".to_string());
            }
            if manifest.read_scope != package.read_scope {
                findings.push("wave_manifest_read_scope_mismatch".to_string());
            }
            if manifest.write_scope != package.write_scope {
                findings.push("wave_manifest_write_scope_mismatch".to_string());
            }
            if manifest.wave_specs != normalize_wave_specs(&package.wave_specs) {
                findings.push("wave_manifest_wave_specs_mismatch".to_string());
            }
        }
    }
    for (included, normalized_current) in package
        .included_specs
        .iter()
        .zip(normalized_current_included_specs.iter())
    {
        if normalized_current.spec_fingerprint != included.spec_fingerprint {
            findings.push(format!("spec_fingerprint_mismatch:{}", included.spec_id));
        }
        if normalized_current.spec_revision != included.spec_revision {
            findings.push(format!("spec_revision_mismatch:{}", included.spec_id));
        }
    }
    if let Some(gate) = gates.gates.iter().find(|gate| {
        gate.gate_kind == GateKind::ExecutionPackage
            && gate.evaluated_against_ref.as_deref() == Some(package.package_id.as_str())
    }) && !matches!(gate.status, MissionGateStatus::Passed)
    {
        findings.push(format!("execution_gate_status:{:?}", gate.status));
    } else if !gates.gates.iter().any(|gate| {
        gate.gate_kind == GateKind::ExecutionPackage
            && gate.evaluated_against_ref.as_deref() == Some(package.package_id.as_str())
            && matches!(gate.status, MissionGateStatus::Passed)
    }) {
        findings.push("execution_gate_missing".to_string());
    }
    Ok(PackageValidationReport {
        mission_id: package.mission_id.clone(),
        package_id: package.package_id.clone(),
        valid: findings.is_empty(),
        findings,
        governing_refs: vec![
            format!("lock:{}", package.lock_revision),
            format!("blueprint:{}", package.blueprint_revision),
        ],
    })
}

fn normalize_current_included_spec_for_package_validation(
    paths: &MissionPaths,
    included: &IncludedSpecRef,
) -> Result<IncludedSpecRef> {
    let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&included.spec_id))?;
    let spec_revision = spec_doc.frontmatter.spec_revision;
    let current_fingerprint = spec_doc.fingerprint()?;

    if spec_revision != included.spec_revision || current_fingerprint == included.spec_fingerprint {
        return Ok(IncludedSpecRef {
            spec_id: included.spec_id.clone(),
            spec_revision,
            spec_fingerprint: current_fingerprint,
        });
    }

    if spec_doc.frontmatter.execution_status == SpecExecutionStatus::Complete {
        return Ok(included.clone());
    }

    Ok(IncludedSpecRef {
        spec_id: included.spec_id.clone(),
        spec_revision,
        spec_fingerprint: current_fingerprint,
    })
}

pub fn derive_writer_packet(
    paths: &MissionPaths,
    input: &WriterPacketInput,
) -> Result<WriterPacket> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    validate_parent_loop_authority_for_mission_mutation(paths, "derive-writer-packet")?;
    let package: ExecutionPackage = load_json(&paths.execution_package(&input.source_package_id))?;
    let package_validation = validate_execution_package(paths, &input.source_package_id)?;
    if package.status != ExecutionPackageStatus::Passed {
        bail!(
            "writer packets may only derive from passed execution packages; {} is {:?}",
            input.source_package_id,
            package.status
        );
    }
    if !package_validation.valid {
        bail!(
            "execution package {} is stale or invalid: {}",
            input.source_package_id,
            package_validation.findings.join(", ")
        );
    }
    if package.mission_id != input.mission_id {
        bail!(
            "writer packet mission {} does not match source package mission {}",
            input.mission_id,
            package.mission_id
        );
    }
    if !package_authorizes_spec(&package, &input.target_spec_id) {
        bail!(
            "execution package {} does not authorize writer work for spec {}",
            input.source_package_id,
            input.target_spec_id
        );
    }

    let spec_doc =
        load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&input.target_spec_id))?;
    let packet_scope = derive_writer_packet_scope(paths, &package, &input.target_spec_id)?;
    if packet_scope.read_paths.is_empty() {
        bail!(
            "execution package {} does not authorize any bounded read scope for spec {}",
            input.source_package_id,
            input.target_spec_id
        );
    }
    if packet_scope.write_paths.is_empty() {
        bail!(
            "execution package {} does not authorize any bounded write scope for spec {}",
            input.source_package_id,
            input.target_spec_id
        );
    }
    let packet = WriterPacket {
        packet_id: Uuid::new_v4().to_string(),
        mission_id: input.mission_id.clone(),
        source_package_id: input.source_package_id.clone(),
        target_spec_id: input.target_spec_id.clone(),
        blueprint_revision: package.blueprint_revision,
        spec_revision: spec_doc.frontmatter.spec_revision,
        allowed_read_paths: packet_scope.read_paths,
        allowed_write_paths: packet_scope.write_paths,
        proof_rows: package.proof_obligations.clone(),
        required_checks: unique_strings(&input.required_checks),
        review_lenses: unique_strings(&input.review_lenses),
        replan_boundary: package.replan_boundary.clone(),
        explicitly_disallowed_decisions: unique_strings(&input.explicitly_disallowed_decisions),
    };
    write_json(paths.writer_packet(&packet.packet_id), &packet)?;
    Ok(packet)
}

pub fn validate_writer_packet(
    paths: &MissionPaths,
    packet_id: &str,
) -> Result<WriterPacketValidationReport> {
    let packet: WriterPacket = load_json(&paths.writer_packet(packet_id))?;
    let package_validation = validate_execution_package(paths, &packet.source_package_id)?;
    let spec_doc =
        load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&packet.target_spec_id))?;

    let mut findings = Vec::new();
    if !package_validation.valid {
        findings.extend(package_validation.findings);
    }
    let package: ExecutionPackage = load_json(&paths.execution_package(&packet.source_package_id))?;
    if !package_authorizes_spec(&package, &packet.target_spec_id) {
        findings.push("writer_packet_target_not_authorized_by_package".to_string());
    }
    if packet.mission_id != package.mission_id {
        findings.push("writer_packet_mission_mismatch".to_string());
    }
    if packet.mission_id != paths.mission_id() {
        findings.push("writer_packet_path_mission_mismatch".to_string());
    }
    if spec_doc.frontmatter.spec_revision != packet.spec_revision {
        findings.push("writer_packet_spec_revision_mismatch".to_string());
    }
    let expected_scope = derive_writer_packet_scope(paths, &package, &packet.target_spec_id)?;
    if packet.allowed_read_paths != expected_scope.read_paths {
        findings.push("writer_packet_read_scope_mismatch".to_string());
    }
    if packet.allowed_write_paths != expected_scope.write_paths {
        findings.push("writer_packet_write_scope_mismatch".to_string());
    }
    if packet.proof_rows != package.proof_obligations {
        findings.push("writer_packet_proof_rows_mismatch".to_string());
    }
    if packet.replan_boundary != package.replan_boundary {
        findings.push("writer_packet_replan_boundary_mismatch".to_string());
    }
    if packet.allowed_read_paths.is_empty() {
        findings.push("writer_packet_read_scope_missing".to_string());
    }
    if packet.allowed_write_paths.is_empty() {
        findings.push("writer_packet_write_scope_missing".to_string());
    }
    if packet.proof_rows.is_empty() {
        findings.push("writer_packet_proof_rows_missing".to_string());
    }

    Ok(WriterPacketValidationReport {
        mission_id: packet.mission_id,
        packet_id: packet.packet_id,
        valid: findings.is_empty(),
        findings,
    })
}

pub fn compile_review_bundle(
    paths: &MissionPaths,
    input: &ReviewBundleInput,
) -> Result<ReviewBundle> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    validate_parent_loop_authority_for_mission_mutation(paths, "compile-review-bundle")?;
    let package: ExecutionPackage = load_json(&paths.execution_package(&input.source_package_id))?;
    if package.mission_id != input.mission_id {
        bail!(
            "execution package {} belongs to mission {}, not {}",
            input.source_package_id,
            package.mission_id,
            input.mission_id
        );
    }
    let bundle = match input.bundle_kind {
        BundleKind::SpecReview => {
            let package_validation = validate_execution_package(paths, &input.source_package_id)?;
            if !package_validation.valid {
                bail!(
                    "execution package {} is stale or invalid: {}",
                    input.source_package_id,
                    package_validation.findings.join(", ")
                );
            }
            let target_spec_id = input
                .target_spec_id
                .clone()
                .context("spec_review bundles require target_spec_id")?;
            if !package_authorizes_spec(&package, &target_spec_id) {
                bail!(
                    "execution package {} does not authorize review for spec {}",
                    input.source_package_id,
                    target_spec_id
                );
            }
            let spec_doc =
                load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&target_spec_id))?;
            let blueprint_doc =
                load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())?;
            let spec_review_lenses = markdown_section_list_items(&spec_doc.body, "Review Lenses");
            let spec_proof_expectations =
                markdown_section_list_items(&spec_doc.body, "Proof-Of-Completion Expectations");
            let blueprint_review_lenses = markdown_section_labeled_values(
                &blueprint_doc.body,
                "Review Bundle Design",
                "Mandatory review lenses",
            );
            ReviewBundle {
                bundle_id: Uuid::new_v4().to_string(),
                mission_id: input.mission_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                source_package_id: package.package_id.clone(),
                lock_revision: package.lock_revision,
                lock_fingerprint: package.lock_fingerprint,
                blueprint_revision: package.blueprint_revision,
                blueprint_fingerprint: package.blueprint_fingerprint,
                governing_revision: format!(
                    "spec:{}:{}",
                    target_spec_id, spec_doc.frontmatter.spec_revision
                ),
                mandatory_review_lenses: unique_strings(
                    &package
                        .review_obligations
                        .iter()
                        .cloned()
                        .chain(blueprint_review_lenses)
                        .chain(spec_review_lenses)
                        .chain(input.mandatory_review_lenses.iter().cloned())
                        .collect::<Vec<_>>(),
                ),
                target_spec_id: Some(target_spec_id),
                spec_revision: Some(spec_doc.frontmatter.spec_revision),
                spec_fingerprint: Some(spec_doc.fingerprint()?),
                proof_rows_under_review: unique_strings(
                    &package
                        .proof_obligations
                        .iter()
                        .cloned()
                        .chain(spec_proof_expectations)
                        .chain(input.proof_rows_under_review.iter().cloned())
                        .collect::<Vec<_>>(),
                ),
                receipts: unique_strings(&input.receipts),
                changed_files_or_diff: unique_strings(&input.changed_files_or_diff),
                touched_interface_contracts: unique_strings(&input.touched_interface_contracts),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                included_spec_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: unique_strings(&input.open_finding_summary),
                generated_at: OffsetDateTime::now_utc(),
            }
        }
        BundleKind::MissionClose => {
            let source_package_findings = validate_mission_close_source_package(paths, &package)?;
            if !source_package_findings.is_empty() {
                bail!(
                    "execution package {} is not a valid mission-close source: {}",
                    input.source_package_id,
                    source_package_findings.join(", ")
                );
            }
            let blueprint_doc =
                load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())?;
            let blueprint_review_lenses = markdown_section_labeled_values(
                &blueprint_doc.body,
                "Review Bundle Design",
                "Mandatory review lenses",
            );
            ReviewBundle {
                bundle_id: Uuid::new_v4().to_string(),
                mission_id: input.mission_id.clone(),
                bundle_kind: BundleKind::MissionClose,
                source_package_id: package.package_id.clone(),
                lock_revision: package.lock_revision,
                lock_fingerprint: package.lock_fingerprint,
                blueprint_revision: package.blueprint_revision,
                blueprint_fingerprint: package.blueprint_fingerprint,
                governing_revision: format!("mission:{}:close", input.mission_id),
                mandatory_review_lenses: unique_strings(
                    &required_mission_close_review_lenses()
                        .into_iter()
                        .chain(blueprint_review_lenses)
                        .chain(input.mandatory_review_lenses.iter().cloned())
                        .collect::<Vec<_>>(),
                ),
                target_spec_id: None,
                spec_revision: None,
                spec_fingerprint: None,
                proof_rows_under_review: Vec::new(),
                receipts: Vec::new(),
                changed_files_or_diff: Vec::new(),
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: unique_strings(&input.mission_level_proof_rows),
                cross_spec_claim_refs: unique_strings(&input.cross_spec_claim_refs),
                included_spec_refs: load_mission_close_spec_ids(paths, package.blueprint_revision)?,
                visible_artifact_refs: unique_strings(&input.visible_artifact_refs),
                deferred_descoped_follow_on_refs: unique_strings(
                    &input.deferred_descoped_follow_on_refs,
                ),
                open_finding_summary: unique_strings(&input.open_finding_summary),
                generated_at: OffsetDateTime::now_utc(),
            }
        }
    };
    let bundle_path = paths.review_bundle(&bundle.bundle_id);
    write_json(&bundle_path, &bundle)?;

    let mut gates = load_gate_index(paths)?;
    let gate_kind = match bundle.bundle_kind {
        BundleKind::SpecReview => GateKind::BlockingReview,
        BundleKind::MissionClose => GateKind::MissionCloseReview,
    };
    let review_target_ref = review_target_ref(&bundle);
    let review_gate_id = review_gate_id(&bundle, gate_kind.clone());
    supersede_matching_gates(
        &mut gates,
        gate_kind.clone(),
        &review_target_ref,
        &review_gate_id,
    );
    append_gate(
        &mut gates,
        MissionGateRecord {
            gate_id: review_gate_id,
            gate_kind,
            target_ref: review_target_ref,
            governing_refs: vec![
                format!("lock:{}", bundle.lock_revision),
                format!("blueprint:{}", bundle.blueprint_revision),
                format!("bundle:{}", bundle.bundle_id),
            ],
            status: MissionGateStatus::Open,
            blocking: true,
            opened_at: bundle.generated_at,
            evaluated_at: None,
            evaluated_against_ref: Some(bundle.bundle_id.clone()),
            evidence_refs: vec![bundle_path.display().to_string()],
            failure_refs: Vec::new(),
            superseded_by: None,
        },
    );
    gates.current_phase = review_phase_for_bundle(&bundle.bundle_kind).to_string();
    gates.updated_at = OffsetDateTime::now_utc();
    write_json(paths.gates_json(), &gates)?;
    append_review_required_closeout(paths, &bundle)?;

    Ok(bundle)
}

pub fn validate_review_bundle(
    paths: &MissionPaths,
    bundle_id: &str,
) -> Result<ReviewBundleValidationReport> {
    let bundle: ReviewBundle = load_json(&paths.review_bundle(bundle_id))?;
    let gates = load_gate_index(paths)?;
    let package: ExecutionPackage = load_json(&paths.execution_package(&bundle.source_package_id))?;
    let mut findings = Vec::new();
    if bundle.mission_id != paths.mission_id() {
        findings.push("bundle_mission_id_mismatch".to_string());
    }
    if package.mission_id != paths.mission_id() {
        findings.push("package_mission_id_mismatch".to_string());
    }
    if bundle.mission_id != package.mission_id {
        findings.push("bundle_package_mission_id_mismatch".to_string());
    }
    if bundle.mandatory_review_lenses.is_empty() {
        findings.push("mandatory_review_lenses_missing".to_string());
    }
    match bundle.bundle_kind {
        BundleKind::SpecReview => {
            let package_validation = validate_execution_package(paths, &bundle.source_package_id)?;
            if !package_validation.valid {
                findings.extend(package_validation.findings);
            }
            if bundle.target_spec_id.is_none() {
                findings.push("target_spec_id_missing".to_string());
            }
            if bundle.spec_revision.is_none() || bundle.spec_fingerprint.is_none() {
                findings.push("spec_governing_context_missing".to_string());
            }
            if bundle.proof_rows_under_review.is_empty() {
                findings.push("proof_rows_under_review_missing".to_string());
            }
            for obligation in &package.proof_obligations {
                if !bundle
                    .proof_rows_under_review
                    .iter()
                    .any(|row| row == obligation)
                {
                    findings.push(format!(
                        "proof_obligation_missing_from_review:{}",
                        obligation
                    ));
                }
            }
            for obligation in &package.review_obligations {
                if !bundle
                    .mandatory_review_lenses
                    .iter()
                    .any(|lens| lens == obligation)
                {
                    findings.push(format!(
                        "review_obligation_missing_from_bundle:{}",
                        obligation
                    ));
                }
            }
            if let Some(target_spec_id) = bundle.target_spec_id.as_deref() {
                if !package_authorizes_spec(&package, target_spec_id) {
                    findings.push("bundle_target_not_authorized_by_package".to_string());
                }
                let normalized_current_spec =
                    match (bundle.spec_revision, bundle.spec_fingerprint.as_ref()) {
                        (Some(spec_revision), Some(spec_fingerprint)) => {
                            Some(normalize_current_included_spec_for_package_validation(
                                paths,
                                &IncludedSpecRef {
                                    spec_id: target_spec_id.to_string(),
                                    spec_revision,
                                    spec_fingerprint: spec_fingerprint.clone(),
                                },
                            )?)
                        }
                        _ => None,
                    };
                if bundle.lock_revision != package.lock_revision {
                    findings.push("bundle_lock_revision_mismatch".to_string());
                }
                if bundle.lock_fingerprint != package.lock_fingerprint {
                    findings.push("bundle_lock_fingerprint_mismatch".to_string());
                }
                if bundle.blueprint_revision != package.blueprint_revision {
                    findings.push("bundle_blueprint_revision_mismatch".to_string());
                }
                if bundle.blueprint_fingerprint != package.blueprint_fingerprint {
                    findings.push("bundle_blueprint_fingerprint_mismatch".to_string());
                }
                if bundle.spec_revision
                    != normalized_current_spec
                        .as_ref()
                        .map(|current| current.spec_revision)
                {
                    findings.push("bundle_spec_revision_mismatch".to_string());
                }
                if bundle.spec_fingerprint.as_ref()
                    != normalized_current_spec
                        .as_ref()
                        .map(|current| &current.spec_fingerprint)
                {
                    findings.push("bundle_spec_fingerprint_mismatch".to_string());
                }
                if bundle.governing_revision
                    != normalized_current_spec
                        .as_ref()
                        .map(|current| format!("spec:{target_spec_id}:{}", current.spec_revision))
                        .unwrap_or_else(|| format!("spec:{target_spec_id}:unknown"))
                {
                    findings.push("bundle_governing_revision_mismatch".to_string());
                }
            }
            if bundle.receipts.is_empty() {
                findings.push("receipts_missing".to_string());
            }
            if bundle.changed_files_or_diff.is_empty() {
                findings.push("changed_files_or_diff_missing".to_string());
            }
        }
        BundleKind::MissionClose => {
            findings.extend(validate_mission_close_source_package(paths, &package)?);
            for lens in REQUIRED_MISSION_CLOSE_REVIEW_LENSES {
                if !bundle
                    .mandatory_review_lenses
                    .iter()
                    .any(|value| value == lens)
                {
                    findings.push(format!("mission_close_required_review_lens_missing:{lens}"));
                }
            }
            if bundle.mission_level_proof_rows.is_empty() {
                findings.push("mission_level_proof_rows_missing".to_string());
            }
            if bundle.included_spec_refs.is_empty() {
                findings.push("included_spec_refs_missing".to_string());
            }
            let package: ExecutionPackage =
                load_json(&paths.execution_package(&bundle.source_package_id))?;
            if bundle.lock_revision != package.lock_revision {
                findings.push("bundle_lock_revision_mismatch".to_string());
            }
            if bundle.lock_fingerprint != package.lock_fingerprint {
                findings.push("bundle_lock_fingerprint_mismatch".to_string());
            }
            if bundle.blueprint_revision != package.blueprint_revision {
                findings.push("bundle_blueprint_revision_mismatch".to_string());
            }
            if bundle.blueprint_fingerprint != package.blueprint_fingerprint {
                findings.push("bundle_blueprint_fingerprint_mismatch".to_string());
            }
            if bundle.governing_revision != format!("mission:{}:close", bundle.mission_id) {
                findings.push("bundle_governing_revision_mismatch".to_string());
            }
            let expected_included = load_mission_close_spec_ids(paths, package.blueprint_revision)?;
            if bundle.included_spec_refs != expected_included {
                findings.push("included_spec_refs_mismatch".to_string());
            }
            let expected_descoped =
                load_descoped_mission_close_spec_ids(paths, package.blueprint_revision)?;
            if !expected_descoped.is_empty() && bundle.deferred_descoped_follow_on_refs.is_empty() {
                findings.push("deferred_descoped_follow_on_refs_missing".to_string());
            }
            for spec_id in expected_descoped {
                if !bundle
                    .deferred_descoped_follow_on_refs
                    .iter()
                    .any(|value| value == &spec_id || value.contains(&spec_id))
                {
                    findings.push(format!("descoped_spec_not_represented:{spec_id}"));
                }
            }
            if bundle.cross_spec_claim_refs.is_empty() && bundle.included_spec_refs.len() > 1 {
                findings.push("cross_spec_claim_refs_missing".to_string());
            }
            let required_visible_refs = [
                paths.outcome_lock(),
                paths.program_blueprint(),
                paths.review_ledger(),
            ];
            for required_ref in required_visible_refs {
                if !bundle
                    .visible_artifact_refs
                    .iter()
                    .any(|value| std::path::Path::new(value) == required_ref)
                {
                    findings.push(format!(
                        "visible_artifact_ref_missing:{}",
                        required_ref
                            .file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("artifact")
                    ));
                } else if !required_ref.exists() {
                    findings.push(format!(
                        "visible_artifact_ref_missing_on_disk:{}",
                        required_ref.display()
                    ));
                }
            }
            findings.extend(mission_close_eligibility_findings(paths, &bundle, &gates)?);
            findings.extend(mission_close_contradiction_findings(paths)?);
        }
    }
    if let Some(gate) = gates
        .gates
        .iter()
        .find(|gate| gate.evaluated_against_ref.as_deref() == Some(bundle.bundle_id.as_str()))
        && !matches!(
            gate.status,
            MissionGateStatus::Open | MissionGateStatus::Passed
        )
    {
        findings.push(format!("review_gate_status:{:?}", gate.status));
    }
    Ok(ReviewBundleValidationReport {
        mission_id: bundle.mission_id,
        bundle_id: bundle.bundle_id,
        valid: findings.is_empty(),
        findings,
    })
}

pub fn capture_review_evidence_snapshot(
    paths: &MissionPaths,
    bundle_id: &str,
) -> Result<ReviewEvidenceSnapshot> {
    validate_required_parent_loop_authority_for_review_writeback(
        paths,
        "capture-review-evidence-snapshot",
    )?;
    let bundle_validation = validate_review_bundle(paths, bundle_id)?;
    if !bundle_validation.valid {
        anyhow::bail!(
            "review bundle {} is invalid: {}",
            bundle_id,
            bundle_validation.findings.join(", ")
        );
    }
    let bundle: ReviewBundle = load_json(&paths.review_bundle(bundle_id))?;
    let review_truth_snapshot_path = paths.review_truth_snapshot(bundle_id);
    let review_truth_snapshot: ReviewTruthSnapshot = load_json(&review_truth_snapshot_path)
        .with_context(|| {
            format!(
                "capture-review-evidence-snapshot requires a parent-held review truth snapshot first; run capture-review-truth-snapshot for bundle {}",
                bundle_id
            )
        })?;
    let review_truth_guard = review_truth_guard_binding(&review_truth_snapshot)?;
    let evidence_refs = unique_strings(
        &std::iter::once(paths.review_bundle(bundle_id).display().to_string())
            .chain(std::iter::once(
                paths
                    .execution_package(&bundle.source_package_id)
                    .display()
                    .to_string(),
            ))
            .chain(bundle.receipts.iter().cloned())
            .chain(bundle.changed_files_or_diff.iter().cloned())
            .chain(bundle.visible_artifact_refs.iter().cloned())
            .chain(bundle.deferred_descoped_follow_on_refs.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let snapshot = ReviewEvidenceSnapshot {
        mission_id: bundle.mission_id.clone(),
        bundle_id: bundle.bundle_id.clone(),
        bundle_kind: bundle.bundle_kind.clone(),
        source_package_id: bundle.source_package_id.clone(),
        target_spec_id: bundle.target_spec_id.clone(),
        lock_revision: bundle.lock_revision,
        lock_fingerprint: bundle.lock_fingerprint.clone(),
        blueprint_revision: bundle.blueprint_revision,
        blueprint_fingerprint: bundle.blueprint_fingerprint.clone(),
        spec_revision: bundle.spec_revision,
        spec_fingerprint: bundle.spec_fingerprint.clone(),
        mandatory_review_lenses: bundle.mandatory_review_lenses.clone(),
        proof_rows_under_review: bundle.proof_rows_under_review.clone(),
        receipts: bundle.receipts.clone(),
        changed_files_or_diff: bundle.changed_files_or_diff.clone(),
        touched_interface_contracts: bundle.touched_interface_contracts.clone(),
        mission_level_proof_rows: bundle.mission_level_proof_rows.clone(),
        cross_spec_claim_refs: bundle.cross_spec_claim_refs.clone(),
        visible_artifact_refs: bundle.visible_artifact_refs.clone(),
        deferred_descoped_follow_on_refs: bundle.deferred_descoped_follow_on_refs.clone(),
        open_finding_summary: bundle.open_finding_summary.clone(),
        evidence_refs,
        reviewer_instructions: review_evidence_snapshot_instructions(),
        review_truth_guard,
        generated_at: OffsetDateTime::now_utc(),
    };
    fs::create_dir_all(paths.review_evidence_snapshots_dir()).with_context(|| {
        format!(
            "failed to create {}",
            paths.review_evidence_snapshots_dir().display()
        )
    })?;
    write_json(paths.review_evidence_snapshot(bundle_id), &snapshot)?;
    Ok(snapshot)
}

pub fn validate_review_evidence_snapshot(
    paths: &MissionPaths,
    bundle_id: &str,
) -> Result<ReviewEvidenceSnapshotValidationReport> {
    let mut findings = Vec::new();
    let snapshot: ReviewEvidenceSnapshot =
        match load_json(&paths.review_evidence_snapshot(bundle_id)) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Ok(ReviewEvidenceSnapshotValidationReport {
                    mission_id: paths.mission_id().to_string(),
                    bundle_id: bundle_id.to_string(),
                    valid: false,
                    findings: vec![format!("review_evidence_snapshot_unreadable:{error}")],
                });
            }
        };
    let bundle: ReviewBundle = load_json(&paths.review_bundle(bundle_id))?;

    if snapshot.mission_id != bundle.mission_id {
        findings.push("snapshot_mission_id_mismatch".to_string());
    }
    if snapshot.bundle_id != bundle.bundle_id {
        findings.push("snapshot_bundle_id_mismatch".to_string());
    }
    if snapshot.bundle_kind != bundle.bundle_kind {
        findings.push("snapshot_bundle_kind_mismatch".to_string());
    }
    if snapshot.source_package_id != bundle.source_package_id {
        findings.push("snapshot_source_package_id_mismatch".to_string());
    }
    if snapshot.target_spec_id != bundle.target_spec_id {
        findings.push("snapshot_target_spec_id_mismatch".to_string());
    }
    if snapshot.lock_revision != bundle.lock_revision
        || snapshot.lock_fingerprint != bundle.lock_fingerprint
    {
        findings.push("snapshot_lock_binding_mismatch".to_string());
    }
    if snapshot.blueprint_revision != bundle.blueprint_revision
        || snapshot.blueprint_fingerprint != bundle.blueprint_fingerprint
    {
        findings.push("snapshot_blueprint_binding_mismatch".to_string());
    }
    if snapshot.spec_revision != bundle.spec_revision
        || snapshot.spec_fingerprint != bundle.spec_fingerprint
    {
        findings.push("snapshot_spec_binding_mismatch".to_string());
    }
    if snapshot.mandatory_review_lenses.is_empty() {
        findings.push("mandatory_review_lenses_missing".to_string());
    }
    if unique_strings(&snapshot.mandatory_review_lenses)
        != unique_strings(&bundle.mandatory_review_lenses)
    {
        findings.push("mandatory_review_lenses_mismatch".to_string());
    }
    if snapshot.bundle_kind == BundleKind::SpecReview && snapshot.proof_rows_under_review.is_empty()
    {
        findings.push("proof_rows_under_review_missing".to_string());
    }
    if unique_strings(&snapshot.proof_rows_under_review)
        != unique_strings(&bundle.proof_rows_under_review)
    {
        findings.push("proof_rows_under_review_mismatch".to_string());
    }
    if snapshot.bundle_kind == BundleKind::SpecReview && snapshot.receipts.is_empty() {
        findings.push("receipts_missing".to_string());
    }
    if unique_strings(&snapshot.receipts) != unique_strings(&bundle.receipts) {
        findings.push("receipts_mismatch".to_string());
    }
    if snapshot.bundle_kind == BundleKind::SpecReview && snapshot.changed_files_or_diff.is_empty() {
        findings.push("changed_files_or_diff_missing".to_string());
    }
    if unique_strings(&snapshot.changed_files_or_diff)
        != unique_strings(&bundle.changed_files_or_diff)
    {
        findings.push("changed_files_or_diff_mismatch".to_string());
    }
    if unique_strings(&snapshot.touched_interface_contracts)
        != unique_strings(&bundle.touched_interface_contracts)
    {
        findings.push("touched_interface_contracts_mismatch".to_string());
    }
    if snapshot.bundle_kind == BundleKind::MissionClose
        && snapshot.mission_level_proof_rows.is_empty()
    {
        findings.push("mission_level_proof_rows_missing".to_string());
    }
    if unique_strings(&snapshot.mission_level_proof_rows)
        != unique_strings(&bundle.mission_level_proof_rows)
    {
        findings.push("mission_level_proof_rows_mismatch".to_string());
    }
    if unique_strings(&snapshot.cross_spec_claim_refs)
        != unique_strings(&bundle.cross_spec_claim_refs)
    {
        findings.push("cross_spec_claim_refs_mismatch".to_string());
    }
    if unique_strings(&snapshot.visible_artifact_refs)
        != unique_strings(&bundle.visible_artifact_refs)
    {
        findings.push("visible_artifact_refs_mismatch".to_string());
    }
    if unique_strings(&snapshot.deferred_descoped_follow_on_refs)
        != unique_strings(&bundle.deferred_descoped_follow_on_refs)
    {
        findings.push("deferred_descoped_follow_on_refs_mismatch".to_string());
    }
    if unique_strings(&snapshot.open_finding_summary)
        != unique_strings(&bundle.open_finding_summary)
    {
        findings.push("open_finding_summary_mismatch".to_string());
    }
    if snapshot.evidence_refs.is_empty() {
        findings.push("evidence_refs_missing".to_string());
    }
    for required_ref in std::iter::once(paths.review_bundle(bundle_id).display().to_string())
        .chain(std::iter::once(
            paths
                .execution_package(&bundle.source_package_id)
                .display()
                .to_string(),
        ))
        .chain(bundle.receipts.iter().cloned())
        .chain(bundle.changed_files_or_diff.iter().cloned())
        .chain(bundle.visible_artifact_refs.iter().cloned())
        .chain(bundle.deferred_descoped_follow_on_refs.iter().cloned())
    {
        if !snapshot
            .evidence_refs
            .iter()
            .any(|value| value == &required_ref)
        {
            findings.push(format!("evidence_ref_missing:{required_ref}"));
        }
    }
    if !snapshot
        .reviewer_instructions
        .iter()
        .any(|instruction| instruction.contains("NONE"))
    {
        findings.push("reviewer_instruction_none_contract_missing".to_string());
    }
    if !snapshot
        .reviewer_instructions
        .iter()
        .any(|instruction| instruction.contains("Do not mutate mission truth"))
    {
        findings.push("reviewer_instruction_no_mutation_missing".to_string());
    }
    if let Err(error) =
        validate_review_truth_guard_binding(paths, &bundle, &snapshot.review_truth_guard)
    {
        findings.push(format!("review_truth_guard_binding_invalid:{error}"));
    }

    Ok(ReviewEvidenceSnapshotValidationReport {
        mission_id: snapshot.mission_id,
        bundle_id: snapshot.bundle_id,
        valid: findings.is_empty(),
        findings,
    })
}

pub fn record_reviewer_output(
    paths: &MissionPaths,
    input: &ReviewerOutputInput,
) -> Result<ReviewerOutputReport> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    validate_reviewer_output_input(input)?;
    let bundle_validation = validate_review_bundle(paths, &input.bundle_id)?;
    if !bundle_validation.valid {
        anyhow::bail!(
            "review bundle {} is invalid: {}",
            input.bundle_id,
            bundle_validation.findings.join(", ")
        );
    }
    let snapshot_report = validate_review_evidence_snapshot(paths, &input.bundle_id)?;
    if !snapshot_report.valid {
        anyhow::bail!(
            "review evidence snapshot {} is invalid: {}",
            input.bundle_id,
            snapshot_report.findings.join(", ")
        );
    }

    let output_kind = reviewer_output_kind(input)?;
    let reviewer_output_id = Uuid::new_v4().to_string();
    let source_evidence_snapshot = paths.review_evidence_snapshot(&input.bundle_id);
    let source_evidence_snapshot_fingerprint = Fingerprint::from_file(&source_evidence_snapshot)
        .with_context(|| {
            format!(
                "failed to fingerprint review evidence snapshot {}",
                source_evidence_snapshot.display()
            )
        })?;
    let artifact = ReviewerOutputArtifact {
        mission_id: input.mission_id.clone(),
        bundle_id: input.bundle_id.clone(),
        reviewer_id: input.reviewer_id.clone(),
        reviewer_output_id: reviewer_output_id.clone(),
        output_kind,
        findings: input.findings.clone(),
        source_evidence_snapshot: source_evidence_snapshot.display().to_string(),
        source_evidence_snapshot_fingerprint,
        recorded_at: OffsetDateTime::now_utc(),
    };
    validate_reviewer_output_artifact(paths, &artifact)?;
    let path = paths.reviewer_output(&input.bundle_id, &reviewer_output_id);
    write_json(&path, &artifact)?;
    Ok(ReviewerOutputReport {
        mission_id: input.mission_id.clone(),
        bundle_id: input.bundle_id.clone(),
        reviewer_output_id: reviewer_output_id.clone(),
        evidence_ref: reviewer_output_evidence_ref(&input.bundle_id, &reviewer_output_id),
        path: path.display().to_string(),
    })
}

fn validate_reviewer_output_input(input: &ReviewerOutputInput) -> Result<()> {
    if input.reviewer_id.trim().is_empty() {
        anyhow::bail!("reviewer output requires reviewer_id");
    }
    let output_kind = reviewer_output_kind(input)?;
    match output_kind {
        ReviewerOutputKind::None if !input.findings.is_empty() => {
            anyhow::bail!("reviewer output kind none cannot include findings");
        }
        ReviewerOutputKind::Findings if input.findings.is_empty() => {
            anyhow::bail!("reviewer output kind findings requires at least one finding");
        }
        _ => {}
    }
    for finding in &input.findings {
        validate_reviewer_output_finding(finding)?;
    }
    Ok(())
}

fn reviewer_output_kind(input: &ReviewerOutputInput) -> Result<ReviewerOutputKind> {
    let inferred = if input.findings.is_empty() {
        ReviewerOutputKind::None
    } else {
        ReviewerOutputKind::Findings
    };
    if let Some(declared) = &input.output_kind
        && declared != &inferred
    {
        anyhow::bail!(
            "reviewer output kind {:?} does not match findings payload",
            declared
        );
    }
    Ok(inferred)
}

fn validate_reviewer_output_finding(finding: &ReviewerOutputFinding) -> Result<()> {
    let severity = finding.severity.trim();
    if !matches!(severity, "P0" | "P1" | "P2" | "P3") {
        anyhow::bail!("reviewer output finding severity must be P0, P1, P2, or P3");
    }
    if finding.title.trim().is_empty() {
        anyhow::bail!("reviewer output finding title must not be empty");
    }
    if finding.evidence_refs.is_empty() {
        anyhow::bail!("reviewer output finding evidence_refs must not be empty");
    }
    if finding.rationale.trim().is_empty() {
        anyhow::bail!("reviewer output finding rationale must not be empty");
    }
    if finding.suggested_next_action.trim().is_empty() {
        anyhow::bail!("reviewer output finding suggested_next_action must not be empty");
    }
    Ok(())
}

fn validate_reviewer_output_artifact(
    paths: &MissionPaths,
    artifact: &ReviewerOutputArtifact,
) -> Result<()> {
    if artifact.mission_id != paths.mission_id() {
        anyhow::bail!(
            "reviewer output mission {} does not match selected mission {}",
            artifact.mission_id,
            paths.mission_id()
        );
    }
    let bundle: ReviewBundle = load_json(&paths.review_bundle(&artifact.bundle_id))?;
    if artifact.bundle_id != bundle.bundle_id {
        anyhow::bail!("reviewer output bundle binding mismatch");
    }
    let snapshot_path = paths.review_evidence_snapshot(&artifact.bundle_id);
    if artifact.source_evidence_snapshot != snapshot_path.display().to_string() {
        anyhow::bail!("reviewer output source evidence snapshot does not match canonical path");
    }
    let actual_snapshot_fingerprint =
        Fingerprint::from_file(&snapshot_path).with_context(|| {
            format!(
                "failed to fingerprint review evidence snapshot {}",
                snapshot_path.display()
            )
        })?;
    if actual_snapshot_fingerprint != artifact.source_evidence_snapshot_fingerprint {
        anyhow::bail!("reviewer output source evidence snapshot fingerprint mismatch");
    }
    match artifact.output_kind {
        ReviewerOutputKind::None if !artifact.findings.is_empty() => {
            anyhow::bail!("reviewer output artifact kind none cannot include findings");
        }
        ReviewerOutputKind::Findings if artifact.findings.is_empty() => {
            anyhow::bail!("reviewer output artifact kind findings requires findings");
        }
        _ => {}
    }
    for finding in &artifact.findings {
        validate_reviewer_output_finding(finding)?;
    }
    Ok(())
}

fn review_evidence_snapshot_instructions() -> Vec<String> {
    vec![
        "Review only this frozen evidence snapshot and the refs it names.".to_string(),
        "Return exactly NONE or structured findings with severity, evidence refs, rationale, and suggested next action, then persist only that bounded result with record-reviewer-output.".to_string(),
        "Do not mutate mission truth, clear gates, write artifacts outside the reviewer-output inbox, call record-review-outcome, or decide mission completion.".to_string(),
        "This evidence snapshot is not a writeback capability; only the parent keeps the full review truth snapshot for record-review-outcome.".to_string(),
        "If live repo reads are necessary, the parent review-loop must keep its parent-held review truth snapshot guard active.".to_string(),
    ]
}

fn append_review_required_closeout(paths: &MissionPaths, bundle: &ReviewBundle) -> Result<()> {
    let existing_closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    let next_seq = existing_closeouts
        .last()
        .map_or(1, |record| record.closeout_seq + 1);
    let mut artifact_fingerprints = BTreeMap::from([
        (
            "outcome_lock".to_string(),
            bundle.lock_fingerprint.to_string(),
        ),
        (
            "program_blueprint".to_string(),
            bundle.blueprint_fingerprint.to_string(),
        ),
    ]);
    if let (Some(spec_id), Some(spec_fingerprint)) = (
        bundle.target_spec_id.as_ref(),
        bundle.spec_fingerprint.as_ref(),
    ) {
        artifact_fingerprints.insert(format!("spec:{spec_id}"), spec_fingerprint.to_string());
    }
    let closeout = CloseoutRecord {
        closeout_id: Some(Uuid::new_v4().to_string()),
        closeout_seq: next_seq,
        mission_id: bundle.mission_id.clone(),
        phase: review_phase_for_bundle(&bundle.bundle_kind).to_string(),
        activity: "review_bundle_compiled".to_string(),
        verdict: Verdict::ReviewRequired,
        terminality: Terminality::ActionableNonTerminal,
        resume_mode: ResumeMode::Continue,
        next_phase: Some(review_phase_for_bundle(&bundle.bundle_kind).to_string()),
        next_action: match bundle.bundle_kind {
            BundleKind::SpecReview => format!(
                "Run blocking review for bundle {} before returning to execution.",
                bundle.bundle_id
            ),
            BundleKind::MissionClose => format!(
                "Run mission-close review for bundle {} before terminalizing the mission.",
                bundle.bundle_id
            ),
        },
        target: Some(review_target_ref(bundle)),
        cycle_kind: Some(CycleKind::GateEvaluation),
        lock_revision: Some(bundle.lock_revision),
        lock_fingerprint: Some(bundle.lock_fingerprint.to_string()),
        blueprint_revision: Some(bundle.blueprint_revision),
        blueprint_fingerprint: Some(bundle.blueprint_fingerprint.to_string()),
        governing_revision: Some(bundle.governing_revision.clone()),
        reason_code: Some(match bundle.bundle_kind {
            BundleKind::SpecReview => "blocking_review_opened".to_string(),
            BundleKind::MissionClose => "mission_close_review_opened".to_string(),
        }),
        summary: Some(format!(
            "Compiled review bundle {} and opened a blocking review gate",
            bundle.bundle_id
        )),
        continuation_prompt: Some(match bundle.bundle_kind {
            BundleKind::SpecReview => {
                "Blocking review is now required before execution can continue.".to_string()
            }
            BundleKind::MissionClose => {
                "Mission-close review is now required before the mission can stop.".to_string()
            }
        }),
        cycle_id: Some(Uuid::new_v4().to_string()),
        waiting_request_id: None,
        waiting_for: None,
        canonical_waiting_request: None,
        resume_condition: None,
        request_emitted_at: None,
        active_child_task_paths: Vec::new(),
        artifact_fingerprints,
    };
    let active_cycle = active_cycle_from_closeout(
        &closeout,
        Vec::new(),
        vec!["review_bundle_compiled".to_string()],
        vec![paths.review_bundle(&bundle.bundle_id).display().to_string()],
        vec![bundle.source_package_id.clone()],
        vec![bundle.bundle_id.clone()],
    );
    append_closeout_for_active_cycle(paths, &closeout, &active_cycle)?;
    Ok(())
}

pub fn record_review_result(
    paths: &MissionPaths,
    input: &ReviewResultInput,
) -> Result<ReviewResultReport> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    validate_required_parent_loop_authority_for_review_writeback(paths, "record-review-outcome")?;
    let context = prepare_review_outcome_context(paths, input)?;
    if let Some(snapshot) = &input.review_truth_snapshot {
        validate_review_truth_snapshot_binding(paths, &context.bundle, snapshot)?;
        validate_review_truth_snapshot(paths, &context.bundle, snapshot)?;
    }
    let gate_update = apply_review_gate_outcome(paths, input, &context)?;
    let artifact_write = write_review_artifacts(paths, input, &context)?;
    let execution_graph_outputs =
        reconcile_execution_graph_obligations_from_passed_review_gates(paths, &gate_update.gates)?;
    let closeout = build_review_closeout(paths, input, &context, &gate_update, &artifact_write)?;
    let active_cycle = active_cycle_from_closeout(
        &closeout,
        Vec::new(),
        vec![
            "review_bundle_validated".to_string(),
            "review_gate_updated".to_string(),
        ],
        artifact_write
            .expected_outputs
            .into_iter()
            .chain(execution_graph_outputs)
            .collect(),
        vec![context.bundle.source_package_id.clone()],
        vec![context.bundle.bundle_id.clone()],
    );
    append_closeout_for_active_cycle(paths, &closeout, &active_cycle)?;

    Ok(ReviewResultReport {
        mission_id: input.mission_id.clone(),
        review_id: context.review_id,
        blocking_findings: context.blocking_findings,
        updated_gate: context.gate_kind,
    })
}

pub fn capture_review_truth_snapshot(
    paths: &MissionPaths,
    bundle_id: &str,
) -> Result<ReviewTruthSnapshot> {
    validate_required_parent_loop_authority_for_review_writeback(
        paths,
        "capture-review-truth-snapshot",
    )?;
    if !paths.review_bundle(bundle_id).is_file() {
        anyhow::bail!("review bundle {} does not exist", bundle_id);
    }
    let snapshot_path = paths.review_truth_snapshot(bundle_id);
    if snapshot_path.is_file() {
        anyhow::bail!(
            "review_truth_snapshot already captured for bundle {}; refusing to remint parent writeback authority; compile a fresh review bundle for a new review wave",
            bundle_id
        );
    }
    let writeback_authority_token = Uuid::new_v4().to_string();
    let writeback_authority_verifier =
        review_writeback_authority_verifier(bundle_id, &writeback_authority_token)?;
    let snapshot = ReviewTruthSnapshot {
        mission_id: paths.mission_id().to_string(),
        bundle_id: bundle_id.to_string(),
        captured_at: OffsetDateTime::now_utc(),
        artifact_fingerprints: current_review_truth_fingerprints(paths)?,
        writeback_authority_verifier: Some(writeback_authority_verifier),
        writeback_authority_token: Some(writeback_authority_token),
    };
    fs::create_dir_all(paths.review_truth_snapshots_dir()).with_context(|| {
        format!(
            "failed to create {}",
            paths.review_truth_snapshots_dir().display()
        )
    })?;
    write_json(snapshot_path, &persisted_review_truth_snapshot(&snapshot))?;
    Ok(snapshot)
}

fn validate_review_truth_snapshot(
    paths: &MissionPaths,
    bundle: &ReviewBundle,
    snapshot: &ReviewTruthSnapshot,
) -> Result<()> {
    if snapshot.mission_id != bundle.mission_id {
        anyhow::bail!(
            "review_truth_snapshot mission {} does not match bundle mission {}",
            snapshot.mission_id,
            bundle.mission_id
        );
    }
    if snapshot.bundle_id != bundle.bundle_id {
        anyhow::bail!(
            "review_truth_snapshot bundle {} does not match review bundle {}",
            snapshot.bundle_id,
            bundle.bundle_id
        );
    }

    let current = current_review_truth_fingerprints(paths)?;
    let mut findings = Vec::new();
    for (path, expected) in &snapshot.artifact_fingerprints {
        match current.get(path) {
            Some(actual) if actual == expected => {}
            Some(actual) => findings.push(format!(
                "review_truth_modified:{path}:expected={expected}:actual={actual}"
            )),
            None => findings.push(format!("review_truth_deleted:{path}")),
        }
    }
    for path in current.keys() {
        if !snapshot.artifact_fingerprints.contains_key(path) {
            findings.push(format!("review_truth_added:{path}"));
        }
    }

    if !findings.is_empty() {
        anyhow::bail!(
            "reviewer_lane_truth_mutation_detected: {}",
            findings.join(", ")
        );
    }
    Ok(())
}

fn validate_review_truth_snapshot_binding(
    paths: &MissionPaths,
    bundle: &ReviewBundle,
    snapshot: &ReviewTruthSnapshot,
) -> Result<()> {
    if snapshot.mission_id != bundle.mission_id {
        anyhow::bail!(
            "review_truth_snapshot mission {} does not match bundle mission {}",
            snapshot.mission_id,
            bundle.mission_id
        );
    }
    if snapshot.bundle_id != bundle.bundle_id {
        anyhow::bail!(
            "review_truth_snapshot bundle {} does not match review bundle {}",
            snapshot.bundle_id,
            bundle.bundle_id
        );
    }
    if snapshot.artifact_fingerprints.is_empty() {
        anyhow::bail!("review_truth_snapshot artifact_fingerprints are missing");
    }
    validate_review_writeback_authority(snapshot)?;

    let canonical_path = paths.review_truth_snapshot(&bundle.bundle_id);
    let canonical_snapshot: ReviewTruthSnapshot =
        load_json(&canonical_path).with_context(|| {
            format!(
                "failed to read canonical review truth snapshot {}",
                canonical_path.display()
            )
        })?;
    if canonical_snapshot != persisted_review_truth_snapshot(snapshot) {
        anyhow::bail!(
            "review_truth_snapshot does not match canonical captured snapshot {}",
            canonical_path.display()
        );
    }
    Ok(())
}

fn review_truth_guard_binding(snapshot: &ReviewTruthSnapshot) -> Result<ReviewTruthGuardBinding> {
    let persisted_snapshot = persisted_review_truth_snapshot(snapshot);
    let guard_fingerprint = Fingerprint::from_bytes(
        serde_json::to_vec(&persisted_snapshot)
            .context("failed to serialize review truth snapshot for guard binding")?
            .as_slice(),
    );
    Ok(ReviewTruthGuardBinding {
        bundle_id: snapshot.bundle_id.clone(),
        captured_at: snapshot.captured_at,
        artifact_fingerprint_count: snapshot.artifact_fingerprints.len(),
        guard_fingerprint,
    })
}

fn validate_review_truth_guard_binding(
    paths: &MissionPaths,
    bundle: &ReviewBundle,
    binding: &ReviewTruthGuardBinding,
) -> Result<()> {
    if binding.bundle_id != bundle.bundle_id {
        anyhow::bail!(
            "review_truth_guard bundle {} does not match review bundle {}",
            binding.bundle_id,
            bundle.bundle_id
        );
    }
    if binding.artifact_fingerprint_count == 0 {
        anyhow::bail!("review_truth_guard artifact_fingerprint_count is missing");
    }

    let canonical_path = paths.review_truth_snapshot(&bundle.bundle_id);
    let canonical_snapshot: ReviewTruthSnapshot =
        load_json(&canonical_path).with_context(|| {
            format!(
                "failed to read canonical review truth snapshot {}",
                canonical_path.display()
            )
        })?;
    let expected = review_truth_guard_binding(&canonical_snapshot)?;
    if &expected != binding {
        anyhow::bail!(
            "review_truth_guard does not match canonical captured snapshot {}",
            canonical_path.display()
        );
    }
    Ok(())
}

fn persisted_review_truth_snapshot(snapshot: &ReviewTruthSnapshot) -> ReviewTruthSnapshot {
    let mut persisted = snapshot.clone();
    persisted.writeback_authority_token = None;
    persisted
}

fn review_writeback_authority_verifier(bundle_id: &str, token: &str) -> Result<Fingerprint> {
    #[derive(Serialize)]
    struct Binding<'a> {
        bundle_id: &'a str,
        token: &'a str,
    }

    Ok(Fingerprint::from_json(&Binding { bundle_id, token })?)
}

fn validate_review_writeback_authority(snapshot: &ReviewTruthSnapshot) -> Result<()> {
    let Some(expected) = snapshot.writeback_authority_verifier.as_ref() else {
        anyhow::bail!("review_truth_snapshot writeback_authority_verifier is missing");
    };
    let Some(token) = snapshot.writeback_authority_token.as_deref() else {
        anyhow::bail!(
            "review_truth_snapshot writeback_authority_token is required for parent-owned writeback"
        );
    };
    let actual = review_writeback_authority_verifier(&snapshot.bundle_id, token)?;
    if &actual != expected {
        anyhow::bail!("review_truth_snapshot writeback_authority_token does not match verifier");
    }
    Ok(())
}

fn current_review_truth_fingerprints(paths: &MissionPaths) -> Result<BTreeMap<String, String>> {
    let mut entries = BTreeMap::new();
    collect_review_truth_fingerprints(paths.repo_root(), &paths.mission_root(), &mut entries)?;
    collect_review_truth_fingerprints(
        paths.repo_root(),
        &paths.hidden_mission_root(),
        &mut entries,
    )?;
    Ok(entries)
}

fn collect_review_truth_fingerprints(
    repo_root: &Path,
    root: &Path,
    entries: &mut BTreeMap<String, String>,
) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to stat {}", path.display()))?;
        if metadata.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("review-evidence-snapshots")
                || path.file_name().and_then(|name| name.to_str()) == Some("review-truth-snapshots")
                || path.file_name().and_then(|name| name.to_str()) == Some("reviewer-outputs")
            {
                continue;
            }
            let mut children = fs::read_dir(&path)
                .with_context(|| format!("failed to read {}", path.display()))?
                .map(|entry| {
                    entry
                        .map(|entry| entry.path())
                        .with_context(|| format!("failed to read entry in {}", path.display()))
                })
                .collect::<Result<Vec<_>>>()?;
            children.sort();
            stack.extend(children.into_iter().rev());
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(repo_root)
                .with_context(|| {
                    format!(
                        "{} is not contained by repo root {}",
                        path.display(),
                        repo_root.display()
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let fingerprint = Fingerprint::from_file(&path)
                .with_context(|| format!("failed to fingerprint {}", path.display()))?;
            entries.insert(relative, fingerprint.to_string());
        }
    }
    Ok(())
}

fn reconcile_execution_graph_obligations_from_passed_review_gates(
    paths: &MissionPaths,
    gates: &MissionGateIndex,
) -> Result<Vec<String>> {
    let Some(mut execution_graph) = load_execution_graph(paths)? else {
        return Ok(Vec::new());
    };

    let mut changed = false;
    for obligation in &mut execution_graph.obligations {
        let target_ref = format!("spec:{}", obligation.target_spec_id);
        let Some(passed_gate) = gates.gates.iter().find(|gate| {
            gate.gate_kind == GateKind::BlockingReview
                && gate.target_ref == target_ref
                && (gate.status == MissionGateStatus::Passed
                    || (gate.status == MissionGateStatus::Stale
                        && !unresolved_review_gate_blocks_resume(paths, gate)))
        }) else {
            continue;
        };
        if obligation.status != ExecutionGraphObligationStatus::Satisfied {
            obligation.status = ExecutionGraphObligationStatus::Satisfied;
            changed = true;
        }

        let merged_satisfied_by = unique_strings(
            &obligation
                .satisfied_by
                .iter()
                .cloned()
                .chain(
                    std::iter::once(format!("gate:{}", passed_gate.gate_id)).chain(
                        passed_gate
                            .evaluated_against_ref
                            .clone()
                            .into_iter()
                            .map(|bundle_id| format!("bundle:{bundle_id}")),
                    ),
                )
                .collect::<Vec<_>>(),
        );
        if merged_satisfied_by != obligation.satisfied_by {
            obligation.satisfied_by = merged_satisfied_by;
            changed = true;
        }

        let merged_evidence_refs = unique_strings(
            &obligation
                .evidence_refs
                .iter()
                .cloned()
                .chain(passed_gate.evidence_refs.iter().cloned())
                .collect::<Vec<_>>(),
        );
        if merged_evidence_refs != obligation.evidence_refs {
            obligation.evidence_refs = merged_evidence_refs;
            changed = true;
        }
    }

    if !changed {
        return Ok(Vec::new());
    }

    write_json(paths.execution_graph(), &execution_graph)?;
    Ok(vec![paths.execution_graph().display().to_string()])
}

fn prepare_review_outcome_context(
    paths: &MissionPaths,
    input: &ReviewResultInput,
) -> Result<ReviewOutcomeContext> {
    if input.reviewer.trim().is_empty() {
        anyhow::bail!("review results must record a non-empty reviewer identity");
    }
    validate_review_finding_inputs(&input.findings)?;
    validate_delegated_review_authority_evidence(paths, input)?;
    let bundle_validation = validate_review_bundle(paths, &input.bundle_id)?;
    if !bundle_validation.valid {
        anyhow::bail!(
            "review bundle {} is invalid: {}",
            input.bundle_id,
            bundle_validation.findings.join(", ")
        );
    }
    let bundle: ReviewBundle = load_json(&paths.review_bundle(&input.bundle_id))?;
    let resolved_target_spec_id = match bundle.bundle_kind {
        BundleKind::SpecReview => {
            let target_spec_id = bundle
                .target_spec_id
                .clone()
                .context("spec review bundles must bind target_spec_id")?;
            if let Some(input_target) = &input.target_spec_id
                && input_target != &target_spec_id
            {
                anyhow::bail!(
                    "review result target {} does not match bundle target {}",
                    input_target,
                    target_spec_id
                );
            }
            Some(target_spec_id)
        }
        BundleKind::MissionClose => None,
    };
    let blocking_findings = input
        .findings
        .iter()
        .filter(|finding| finding.blocking)
        .count();
    let clean_verdict = review_verdict_is_clean(&input.verdict);
    if !clean_verdict && blocking_findings == 0 {
        anyhow::bail!("non-clean review results must include at least one blocking finding");
    }

    let gate_kind = match &bundle.bundle_kind {
        BundleKind::SpecReview => GateKind::BlockingReview,
        BundleKind::MissionClose => GateKind::MissionCloseReview,
    };

    Ok(ReviewOutcomeContext {
        review_id: Uuid::new_v4().to_string(),
        bundle,
        resolved_target_spec_id,
        blocking_findings,
        review_passed: blocking_findings == 0 && clean_verdict,
        gate_kind,
    })
}

fn apply_review_gate_outcome(
    paths: &MissionPaths,
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
) -> Result<ReviewGateUpdate> {
    let mut gates = load_gate_index(paths)?;
    let mut updated_gate_id: Option<String> = None;
    for gate in &mut gates.gates {
        if gate.evaluated_against_ref.as_deref() == Some(context.bundle.bundle_id.as_str()) {
            if gate.status != MissionGateStatus::Open {
                anyhow::bail!(
                    "review bundle {} cannot be recorded because gate {} is not open",
                    context.bundle.bundle_id,
                    gate.gate_id
                );
            }
            gate.status = if context.review_passed {
                MissionGateStatus::Passed
            } else {
                MissionGateStatus::Failed
            };
            gate.evaluated_at = Some(OffsetDateTime::now_utc());
            gate.evidence_refs.extend(input.evidence_refs.clone());
            gate.failure_refs = review_failure_refs(input, context.review_passed);
            updated_gate_id = Some(gate.gate_id.clone());
        }
    }
    let updated_gate_id = updated_gate_id.context("review bundle has no matching gate record")?;
    gates.updated_at = OffsetDateTime::now_utc();
    write_json(paths.gates_json(), &gates)?;

    Ok(ReviewGateUpdate {
        gates,
        updated_gate_id,
    })
}

fn write_review_artifacts(
    paths: &MissionPaths,
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
) -> Result<ReviewArtifactWriteResult> {
    let existing_ledger = if paths.review_ledger().is_file() {
        Some(
            fs::read_to_string(paths.review_ledger())
                .with_context(|| format!("failed to read {}", paths.review_ledger().display()))?,
        )
    } else {
        None
    };
    let ledger = render_review_ledger(
        &context.review_id,
        input,
        &context.bundle,
        context.review_passed,
        existing_ledger,
    );
    fs::write(paths.review_ledger(), ledger)
        .with_context(|| format!("failed to write {}", paths.review_ledger().display()))?;

    let mut closeout_spec_fingerprint = context.bundle.spec_fingerprint.clone();
    let mut expected_outputs = vec![paths.review_ledger().display().to_string()];
    if let Some(spec_id) = &context.resolved_target_spec_id {
        let existing_review = if paths.review_file(spec_id).is_file() {
            Some(
                fs::read_to_string(paths.review_file(spec_id)).with_context(|| {
                    format!("failed to read {}", paths.review_file(spec_id).display())
                })?,
            )
        } else {
            None
        };
        let review_body = render_spec_review(
            &context.review_id,
            spec_id,
            input,
            &context.bundle,
            existing_review,
        );
        fs::write(paths.review_file(spec_id), review_body)
            .with_context(|| format!("failed to write {}", paths.review_file(spec_id).display()))?;
        expected_outputs.push(paths.review_file(spec_id).display().to_string());
        expected_outputs.push(paths.spec_file(spec_id).display().to_string());

        if context.review_passed {
            let mut spec_doc =
                load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(spec_id))?;
            if spec_doc.frontmatter.execution_status != SpecExecutionStatus::Complete {
                spec_doc.frontmatter.execution_status = SpecExecutionStatus::Complete;
                closeout_spec_fingerprint = Some(spec_doc.fingerprint()?);
                let rendered = spec_doc.render()?;
                fs::write(paths.spec_file(spec_id), rendered).with_context(|| {
                    format!("failed to write {}", paths.spec_file(spec_id).display())
                })?;
            } else {
                closeout_spec_fingerprint = Some(spec_doc.fingerprint()?);
            }
        } else {
            let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(spec_id))?;
            closeout_spec_fingerprint = Some(spec_doc.fingerprint()?);
        }
    }

    Ok(ReviewArtifactWriteResult {
        closeout_spec_fingerprint,
        expected_outputs,
    })
}

fn build_review_closeout(
    paths: &MissionPaths,
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
    gate_update: &ReviewGateUpdate,
    artifact_write: &ReviewArtifactWriteResult,
) -> Result<CloseoutRecord> {
    let existing_closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    let next_seq = existing_closeouts
        .last()
        .map_or(1, |record| record.closeout_seq + 1);
    let cycle_id = Uuid::new_v4().to_string();

    let unresolved_gates = unresolved_blocking_gate_refs(
        &gate_update.gates,
        Some(gate_update.updated_gate_id.as_str()),
    );
    let mission_close_findings = if context.bundle.bundle_kind == BundleKind::MissionClose {
        let mut findings =
            mission_close_eligibility_findings(paths, &context.bundle, &gate_update.gates)?;
        findings.extend(mission_close_contradiction_findings(paths)?);
        unique_strings(&findings)
    } else {
        Vec::new()
    };
    let unresolved_gates = if context.bundle.bundle_kind == BundleKind::MissionClose {
        unresolved_blocking_gate_refs_for_source_package(
            &gate_update.gates,
            context.bundle.source_package_id.as_str(),
            Some(gate_update.updated_gate_id.as_str()),
        )
    } else {
        unresolved_gates
    };
    let verdict = review_result_verdict(input, context, &unresolved_gates, &mission_close_findings);
    if verdict == Verdict::NeedsUser && input.waiting_request.is_none() {
        anyhow::bail!("review needs_user dispositions must include waiting_request");
    }
    let (resume_mode, terminality) = match verdict {
        Verdict::NeedsUser => (ResumeMode::YieldToUser, Terminality::WaitingNonTerminal),
        Verdict::Complete | Verdict::HardBlocked => (ResumeMode::AllowStop, Terminality::Terminal),
        _ => (ResumeMode::Continue, Terminality::ActionableNonTerminal),
    };
    let waiting_request_id = input
        .waiting_request
        .as_ref()
        .map(|_| Uuid::new_v4().to_string());
    let mut artifact_fingerprints = BTreeMap::from([
        (
            "outcome_lock".to_string(),
            context.bundle.lock_fingerprint.to_string(),
        ),
        (
            "program_blueprint".to_string(),
            context.bundle.blueprint_fingerprint.to_string(),
        ),
    ]);
    if let (Some(spec_id), Some(spec_fingerprint)) = (
        context.resolved_target_spec_id.as_ref(),
        artifact_write.closeout_spec_fingerprint.as_ref(),
    ) {
        artifact_fingerprints.insert(format!("spec:{spec_id}"), spec_fingerprint.to_string());
    }

    Ok(CloseoutRecord {
        closeout_id: Some(Uuid::new_v4().to_string()),
        closeout_seq: next_seq,
        mission_id: input.mission_id.clone(),
        phase: review_phase_for_bundle(&context.bundle.bundle_kind).to_string(),
        activity: "review_disposition".to_string(),
        verdict: verdict.clone(),
        terminality,
        resume_mode,
        next_phase: Some(review_result_next_phase(input, context, &verdict)),
        next_action: review_result_next_action(
            input,
            context,
            &verdict,
            &unresolved_gates,
            &mission_close_findings,
        ),
        target: context
            .resolved_target_spec_id
            .clone()
            .map(|spec| format!("spec:{spec}"))
            .or_else(|| Some(format!("mission:{}", input.mission_id))),
        cycle_kind: Some(CycleKind::GateEvaluation),
        lock_revision: Some(context.bundle.lock_revision),
        lock_fingerprint: Some(context.bundle.lock_fingerprint.to_string()),
        blueprint_revision: Some(context.bundle.blueprint_revision),
        blueprint_fingerprint: Some(context.bundle.blueprint_fingerprint.to_string()),
        governing_revision: Some(context.bundle.governing_revision.clone()),
        reason_code: Some(review_result_reason_code(
            input,
            context,
            &unresolved_gates,
            &mission_close_findings,
        )),
        summary: Some(format!(
            "Recorded review bundle {} with {} blocking finding(s)",
            context.bundle.bundle_id, context.blocking_findings
        )),
        continuation_prompt: Some(review_result_continuation_prompt(
            input,
            context,
            &verdict,
            &unresolved_gates,
            &mission_close_findings,
        )),
        cycle_id: Some(cycle_id),
        waiting_request_id,
        waiting_for: input
            .waiting_request
            .as_ref()
            .map(|waiting| waiting.waiting_for.clone()),
        canonical_waiting_request: input
            .waiting_request
            .as_ref()
            .map(|waiting| waiting.canonical_request.clone()),
        resume_condition: input
            .waiting_request
            .as_ref()
            .map(|waiting| waiting.resume_condition.clone()),
        request_emitted_at: None,
        active_child_task_paths: Vec::new(),
        artifact_fingerprints,
    })
}

fn validate_review_finding_inputs(findings: &[ReviewFindingInput]) -> Result<()> {
    for finding in findings {
        if !ALLOWED_REVIEW_FINDING_CLASSES.contains(&finding.class.as_str()) {
            bail!(
                "review finding class {} is not allowed; expected one of {}",
                finding.class,
                ALLOWED_REVIEW_FINDING_CLASSES.join(", ")
            );
        }
        if finding.class.starts_with("B-") && !finding.blocking {
            bail!(
                "review finding class {} must be recorded as blocking",
                finding.class
            );
        }
        if finding.class.starts_with("NB-") && finding.blocking {
            bail!(
                "review finding class {} must be recorded as non-blocking",
                finding.class
            );
        }
    }
    Ok(())
}

fn validate_delegated_review_authority_evidence(
    paths: &MissionPaths,
    input: &ReviewResultInput,
) -> Result<()> {
    if input.review_truth_snapshot.is_none() {
        bail!(
            "review outcome requires review_truth_snapshot; capture it before launching reviewer lanes"
        );
    }

    let evidence_refs = delegated_review_evidence_refs(input);
    validate_parent_owned_review_writeback_identity(input)?;
    if evidence_refs
        .iter()
        .any(|evidence_ref| reviewer_agent_output_evidence_ref(evidence_ref))
    {
        validate_reviewer_output_evidence_refs(paths, input, &evidence_refs)?;
        return Ok(());
    }

    let contaminated_replan_route = !review_verdict_is_clean(&input.verdict)
        && matches!(
            input.next_required_branch,
            Some(NextRequiredBranch::Replan | NextRequiredBranch::Review)
        )
        && evidence_refs
            .iter()
            .any(|evidence_ref| review_wave_contamination_evidence_ref(evidence_ref));
    if contaminated_replan_route {
        return Ok(());
    }

    bail!(
        "review outcome requires reviewer-agent output evidence; cite a reviewer output ref such as reviewer-output:<lane-or-artifact>, or route a contaminated wave with review-wave-contaminated:<reason>"
    );
}

fn validate_reviewer_output_evidence_refs(
    paths: &MissionPaths,
    input: &ReviewResultInput,
    evidence_refs: &[&str],
) -> Result<()> {
    #[cfg(test)]
    if evidence_refs
        .iter()
        .copied()
        .any(test_synthetic_reviewer_output_evidence_ref)
    {
        return Ok(());
    }

    let mut reviewer_outputs = Vec::new();
    for evidence_ref in evidence_refs
        .iter()
        .copied()
        .filter(|evidence_ref| reviewer_agent_output_evidence_ref(evidence_ref))
    {
        let artifact =
            load_reviewer_output_from_evidence_ref(paths, &input.bundle_id, evidence_ref)
                .with_context(|| format!("invalid reviewer-output evidence ref {evidence_ref}"))?;
        validate_reviewer_output_artifact(paths, &artifact)?;
        reviewer_outputs.push(artifact);
    }
    if reviewer_outputs.is_empty() {
        anyhow::bail!("review outcome requires at least one reviewer-output inbox artifact");
    }
    let reviewer_found_blocking = reviewer_outputs
        .iter()
        .flat_map(|artifact| artifact.findings.iter())
        .any(reviewer_output_finding_blocks_review);
    if reviewer_found_blocking && review_verdict_is_clean(&input.verdict) {
        anyhow::bail!(
            "review outcome cannot be clean while cited reviewer-output artifact contains P0/P1/P2 findings"
        );
    }
    validate_reviewer_outputs_follow_parent_truth_snapshot(input, &reviewer_outputs)?;
    validate_clean_review_lane_completion(paths, input, &reviewer_outputs)?;
    Ok(())
}

#[cfg(test)]
fn test_synthetic_reviewer_output_evidence_ref(evidence_ref: &str) -> bool {
    let normalized = evidence_ref.trim().replace('\\', "/");
    let Some(payload) = normalized
        .strip_prefix("reviewer-output:")
        .or_else(|| normalized.strip_prefix("reviewer-output/"))
    else {
        return false;
    };
    !payload.contains(':') && !payload.contains('/')
}

fn validate_reviewer_outputs_follow_parent_truth_snapshot(
    input: &ReviewResultInput,
    reviewer_outputs: &[ReviewerOutputArtifact],
) -> Result<()> {
    let Some(snapshot) = input.review_truth_snapshot.as_ref() else {
        return Ok(());
    };

    for artifact in reviewer_outputs {
        if artifact.recorded_at < snapshot.captured_at {
            anyhow::bail!(
                "reviewer-output {} was recorded before parent review_truth_snapshot capture; parent writeback authority must be captured before launching reviewer lanes",
                artifact.reviewer_output_id
            );
        }
    }

    Ok(())
}

fn validate_clean_review_lane_completion(
    paths: &MissionPaths,
    input: &ReviewResultInput,
    reviewer_outputs: &[ReviewerOutputArtifact],
) -> Result<()> {
    if !review_verdict_is_clean(&input.verdict) {
        return Ok(());
    }

    let bundle: ReviewBundle = load_json(&paths.review_bundle(&input.bundle_id))?;
    let required_lanes = required_reviewer_output_lanes_for_bundle(&bundle);
    if required_lanes.is_empty() {
        return Ok(());
    }

    let mut missing = Vec::new();
    let mut used_output_ids = BTreeSet::new();
    for lane in required_lanes {
        let present = reviewer_outputs.iter().find(|output| {
            !used_output_ids.contains(&output.reviewer_output_id)
                && reviewer_output_satisfies_required_lane(output, lane)
        });
        if let Some(output) = present {
            used_output_ids.insert(output.reviewer_output_id.clone());
            continue;
        }
        missing.push(lane);
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "clean review outcome missing required reviewer-output lane coverage: {}",
            missing.join(", ")
        );
    }

    Ok(())
}

fn required_reviewer_output_lanes_for_bundle(bundle: &ReviewBundle) -> Vec<&'static str> {
    let lenses = bundle
        .mandatory_review_lenses
        .iter()
        .map(|lens| lens.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    let needs_correctness = lenses
        .iter()
        .any(|lens| lens == "correctness" || lens.contains("correctness"));
    if !needs_correctness {
        return Vec::new();
    }
    if bundle.bundle_kind == BundleKind::MissionClose {
        return vec!["code_bug_correctness", "spec_intent_or_proof"];
    }
    if !review_bundle_has_code_changed_files(bundle) {
        return Vec::new();
    }

    vec!["code_bug_correctness", "spec_intent_or_proof"]
}

fn review_bundle_has_code_changed_files(bundle: &ReviewBundle) -> bool {
    bundle
        .changed_files_or_diff
        .iter()
        .any(|path| review_path_looks_code_producing(path))
}

fn review_path_looks_code_producing(path: &str) -> bool {
    let normalized = path.trim().to_ascii_lowercase();
    let code_extensions = [
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".java", ".kt", ".swift", ".c", ".cc",
        ".cpp", ".h", ".hpp", ".cs", ".rb", ".php", ".scala", ".sh", ".zsh", ".fish",
    ];
    normalized.starts_with("crates/")
        || normalized.starts_with("src/")
        || normalized.starts_with("tests/")
        || normalized.starts_with("scripts/")
        || code_extensions
            .iter()
            .any(|extension| normalized.ends_with(extension))
}

fn reviewer_output_satisfies_required_lane(output: &ReviewerOutputArtifact, lane: &str) -> bool {
    let reviewer_id = output.reviewer_id.trim().to_ascii_lowercase();
    match lane {
        "code_bug_correctness" => {
            reviewer_id.contains("code")
                || reviewer_id.contains("bug")
                || reviewer_id.contains("correctness")
                || reviewer_id.contains("hard_coding")
        }
        "spec_intent_or_proof" => {
            reviewer_id.contains("spec")
                || reviewer_id.contains("intent")
                || reviewer_id.contains("proof")
                || reviewer_id.contains("evidence")
        }
        _ => false,
    }
}

fn load_reviewer_output_from_evidence_ref(
    paths: &MissionPaths,
    expected_bundle_id: &str,
    evidence_ref: &str,
) -> Result<ReviewerOutputArtifact> {
    let (bundle_id, output_id) = parse_reviewer_output_evidence_ref(evidence_ref)?;
    if bundle_id != expected_bundle_id {
        anyhow::bail!(
            "reviewer-output bundle {} does not match review result bundle {}",
            bundle_id,
            expected_bundle_id
        );
    }
    load_json(paths.reviewer_output(&bundle_id, &output_id))
}

fn parse_reviewer_output_evidence_ref(evidence_ref: &str) -> Result<(String, String)> {
    let normalized = evidence_ref.trim().replace('\\', "/");
    let Some(payload) = normalized
        .strip_prefix("reviewer-output:")
        .or_else(|| normalized.strip_prefix("reviewer-output/"))
    else {
        anyhow::bail!("reviewer output ref must start with reviewer-output:");
    };
    let mut parts = payload.split([':', '/']);
    let bundle_id = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .context("reviewer output ref missing bundle id")?;
    let output_id = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .context("reviewer output ref missing output id")?
        .trim_end_matches(".json");
    if parts.next().is_some() {
        anyhow::bail!("reviewer output ref must be reviewer-output:<bundle-id>:<output-id>");
    }
    Ok((bundle_id.to_string(), output_id.to_string()))
}

fn reviewer_output_evidence_ref(bundle_id: &str, output_id: &str) -> String {
    format!("reviewer-output:{bundle_id}:{output_id}")
}

fn reviewer_output_finding_blocks_review(finding: &ReviewerOutputFinding) -> bool {
    matches!(finding.severity.trim(), "P0" | "P1" | "P2")
}

fn validate_parent_owned_review_writeback_identity(input: &ReviewResultInput) -> Result<()> {
    if !reviewer_identity_looks_like_child_lane(&input.reviewer) {
        if let Some(evidence_ref) = delegated_review_evidence_refs(input)
            .into_iter()
            .find(|evidence_ref| reviewer_output_ref_looks_like_self_writeback(evidence_ref, input))
        {
            bail!(
                "review outcome writeback is parent-owned; reviewer-output evidence {} looks like child reviewer self-writeback for bundle {}",
                evidence_ref,
                input.bundle_id
            );
        }

        return Ok(());
    }

    bail!(
        "review outcome writeback is parent-owned; reviewer-lane identity {} must return bounded reviewer output instead of calling record-review-outcome",
        input.reviewer
    );
}

fn delegated_review_evidence_refs(input: &ReviewResultInput) -> Vec<&str> {
    input
        .evidence_refs
        .iter()
        .map(String::as_str)
        .chain(
            input
                .findings
                .iter()
                .flat_map(|finding| finding.evidence_refs.iter().map(String::as_str)),
        )
        .collect()
}

fn reviewer_agent_output_evidence_ref(evidence_ref: &str) -> bool {
    let normalized = evidence_ref.trim().to_ascii_lowercase().replace('\\', "/");
    REVIEWER_AGENT_OUTPUT_EVIDENCE_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
}

fn reviewer_output_ref_looks_like_self_writeback(
    evidence_ref: &str,
    input: &ReviewResultInput,
) -> bool {
    let normalized = evidence_ref.trim().to_ascii_lowercase().replace('\\', "/");
    let Some(payload) = REVIEWER_AGENT_OUTPUT_EVIDENCE_PREFIXES
        .iter()
        .find_map(|prefix| normalized.strip_prefix(prefix))
    else {
        return false;
    };
    if !payload.contains(&input.bundle_id.to_ascii_lowercase()) {
        return false;
    }
    let lane = payload.split([':', '/']).next().unwrap_or_default().trim();
    reviewer_identity_looks_like_child_lane(lane)
}

fn reviewer_identity_looks_like_child_lane(reviewer: &str) -> bool {
    let normalized = reviewer.trim().to_ascii_lowercase().replace('\\', "/");
    let leaf = normalized.rsplit('/').next().unwrap_or(normalized.as_str());
    leaf.starts_with("review_")
        || leaf.starts_with("review-")
        || leaf.starts_with("reviewer_")
        || leaf.starts_with("reviewer-")
        || leaf.starts_with("child-review")
        || leaf.starts_with("child_reviewer")
        || leaf.contains("_reviewer_")
        || leaf.contains("-reviewer-")
}

fn review_wave_contamination_evidence_ref(evidence_ref: &str) -> bool {
    let normalized = evidence_ref.trim().to_ascii_lowercase().replace('\\', "/");
    REVIEW_WAVE_CONTAMINATION_EVIDENCE_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
}

fn review_failure_refs(input: &ReviewResultInput, review_passed: bool) -> Vec<String> {
    if review_passed {
        return Vec::new();
    }

    let mut refs = input
        .findings
        .iter()
        .filter(|finding| finding.blocking)
        .map(|finding| finding.summary.clone())
        .collect::<Vec<_>>();
    if refs.is_empty() {
        refs.push(format!("review_verdict:{}", input.verdict));
    }
    refs
}

fn review_result_verdict(
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
    unresolved_gates: &[String],
    mission_close_findings: &[String],
) -> Verdict {
    if context.review_passed {
        if matches!(
            input.next_required_branch,
            Some(NextRequiredBranch::NeedsUser)
        ) {
            Verdict::NeedsUser
        } else if context.bundle.bundle_kind == BundleKind::MissionClose
            && unresolved_gates.is_empty()
            && mission_close_findings.is_empty()
        {
            Verdict::Complete
        } else {
            Verdict::ContinueRequired
        }
    } else {
        match input
            .next_required_branch
            .clone()
            .unwrap_or(NextRequiredBranch::Repair)
        {
            NextRequiredBranch::Review => Verdict::ReviewRequired,
            NextRequiredBranch::Replan => Verdict::ReplanRequired,
            NextRequiredBranch::NeedsUser => Verdict::NeedsUser,
            _ => Verdict::RepairRequired,
        }
    }
}

fn review_result_next_phase(
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
    verdict: &Verdict,
) -> String {
    if !context.review_passed {
        return match input
            .next_required_branch
            .clone()
            .unwrap_or(NextRequiredBranch::Repair)
        {
            NextRequiredBranch::Review => "review",
            NextRequiredBranch::Replan => "replan",
            NextRequiredBranch::NeedsUser => review_phase_for_bundle(&context.bundle.bundle_kind),
            _ => "execution",
        }
        .to_string();
    }

    match verdict {
        Verdict::ReplanRequired => "replan",
        Verdict::NeedsUser => review_phase_for_bundle(&context.bundle.bundle_kind),
        Verdict::Complete => "complete",
        Verdict::ReviewRequired => review_phase_for_bundle(&context.bundle.bundle_kind),
        _ if matches!(
            input.next_required_branch,
            Some(NextRequiredBranch::MissionClose)
        ) =>
        {
            "mission_close"
        }
        _ if context.bundle.bundle_kind == BundleKind::MissionClose => "mission_close",
        _ => "execution",
    }
    .to_string()
}

fn review_result_next_action(
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
    verdict: &Verdict,
    unresolved_gates: &[String],
    mission_close_findings: &[String],
) -> String {
    if *verdict == Verdict::NeedsUser {
        return input
            .waiting_request
            .as_ref()
            .map(|waiting| waiting.canonical_request.clone())
            .unwrap_or_else(|| "Await user input.".to_string());
    }

    if context.review_passed {
        if context.bundle.bundle_kind == BundleKind::MissionClose
            && unresolved_gates.is_empty()
            && mission_close_findings.is_empty()
        {
            "Mission-close review passed; the mission may stop as complete.".to_string()
        } else if context.bundle.bundle_kind == BundleKind::MissionClose {
            let mut blockers = unresolved_gates.to_vec();
            blockers.extend(mission_close_findings.to_vec());
            format!(
                "Refresh unresolved mission-close blockers before closeout: {}.",
                blockers.join(", ")
            )
        } else if matches!(
            input.next_required_branch,
            Some(NextRequiredBranch::MissionClose)
        ) {
            "Review is clean; advance into mission-close review.".to_string()
        } else {
            "Continue from clean review results or move toward mission close review.".to_string()
        }
    } else {
        format!(
            "Review did not pass cleanly; address {} blocking review finding(s) or reconcile verdict {}.",
            context.blocking_findings, input.verdict
        )
    }
}

fn review_result_reason_code(
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
    unresolved_gates: &[String],
    mission_close_findings: &[String],
) -> String {
    if context.review_passed {
        if context.bundle.bundle_kind == BundleKind::MissionClose
            && unresolved_gates.is_empty()
            && mission_close_findings.is_empty()
        {
            "mission_close_review_passed"
        } else if context.bundle.bundle_kind == BundleKind::MissionClose {
            "mission_close_requires_fresh_gates"
        } else if matches!(
            input.next_required_branch,
            Some(NextRequiredBranch::MissionClose)
        ) {
            "review_clean_ready_for_mission_close"
        } else {
            "review_clean"
        }
    } else {
        "review_blocked"
    }
    .to_string()
}

fn review_result_continuation_prompt(
    input: &ReviewResultInput,
    context: &ReviewOutcomeContext,
    verdict: &Verdict,
    unresolved_gates: &[String],
    mission_close_findings: &[String],
) -> String {
    if *verdict == Verdict::NeedsUser {
        return input
            .waiting_request
            .as_ref()
            .map(|waiting| waiting.canonical_request.clone())
            .unwrap_or_else(|| "Await user input.".to_string());
    }

    if context.review_passed {
        if context.bundle.bundle_kind == BundleKind::MissionClose
            && unresolved_gates.is_empty()
            && mission_close_findings.is_empty()
        {
            "Mission is complete.".to_string()
        } else if context.bundle.bundle_kind == BundleKind::MissionClose {
            let mut reasons = unresolved_gates.to_vec();
            reasons.extend(mission_close_findings.to_vec());
            format!(
                "Mission-close review is clean, but completion is still blocked by: {}.",
                reasons.join(", ")
            )
        } else if matches!(
            input.next_required_branch,
            Some(NextRequiredBranch::MissionClose)
        ) {
            "Prepare the integrated mission-close review bundle and close honestly.".to_string()
        } else {
            "Continue mission flow from a clean review state.".to_string()
        }
    } else {
        "Do not close the mission; repair or replan is required.".to_string()
    }
}

pub fn append_contradiction(
    paths: &MissionPaths,
    input: &ContradictionInput,
) -> Result<ContradictionRecord> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    validate_parent_loop_authority_for_mission_mutation(paths, "record-contradiction")?;
    let now = OffsetDateTime::now_utc();
    let status = input.status.clone().unwrap_or_else(|| {
        if input.machine_action.is_some() || input.next_required_branch.is_some() {
            ContradictionStatus::AcceptedForReplan
        } else {
            ContradictionStatus::Open
        }
    });
    match status {
        ContradictionStatus::AcceptedForRepair | ContradictionStatus::AcceptedForReplan => {
            if input.triage_decision.is_none()
                || input.machine_action.is_none()
                || input.next_required_branch.is_none()
            {
                bail!(
                    "accepted contradictions require triage_decision, machine_action, and next_required_branch"
                );
            }
        }
        ContradictionStatus::Resolved | ContradictionStatus::Dismissed => {
            if input.resolution_ref.as_deref().is_none_or(str::is_empty) {
                bail!("resolved or dismissed contradictions require resolution_ref");
            }
        }
        ContradictionStatus::Triaged => {
            if input.triage_decision.is_none() {
                bail!("triaged contradictions require triage_decision");
            }
        }
        ContradictionStatus::Open => {}
    }
    if !matches!(status, ContradictionStatus::Open)
        && (input.triage_decision.is_none()
            || input.triaged_by.as_deref().is_none_or(str::is_empty))
    {
        bail!("non-open contradictions require triage_decision and triaged_by");
    }
    if matches!(
        input.suggested_reopen_layer,
        ReopenLayer::Blueprint | ReopenLayer::MissionLock
    ) && matches!(
        input.machine_action,
        Some(
            MachineAction::ContinueLocalExecution
                | MachineAction::ForceRepair
                | MachineAction::ForceReview
        )
    ) {
        bail!(
            "non-local contradictions targeting {:?} cannot continue via local execution, repair, or review",
            input.suggested_reopen_layer
        );
    }
    let record = ContradictionRecord {
        contradiction_id: Uuid::new_v4().to_string(),
        discovered_in_phase: input.discovered_in_phase.clone(),
        discovered_by: input.discovered_by.clone(),
        target_type: input.target_type.clone(),
        target_id: input.target_id.clone(),
        evidence_refs: unique_strings(&input.evidence_refs),
        violated_assumption_or_contract: input.violated_assumption_or_contract.clone(),
        suggested_reopen_layer: input.suggested_reopen_layer.clone(),
        reason_code: input.reason_code.clone(),
        status: status.clone(),
        governing_revision: input.governing_revision.clone(),
        triage_decision: input.triage_decision.clone(),
        triaged_at: (!matches!(status, ContradictionStatus::Open)).then_some(now),
        triaged_by: input.triaged_by.clone(),
        machine_action: input.machine_action.clone(),
        next_required_branch: input.next_required_branch.clone(),
        resolution_ref: input.resolution_ref.clone(),
        resolved_at: input.resolution_ref.as_ref().map(|_| now),
    };
    fs::create_dir_all(paths.hidden_mission_root())
        .with_context(|| format!("failed to create {}", paths.hidden_mission_root().display()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.contradictions_ndjson())
        .with_context(|| format!("failed to open {}", paths.contradictions_ndjson().display()))?;
    serde_json::to_writer(&file, &record).context("failed to serialize contradiction")?;
    file.write_all(b"\n")
        .context("failed to terminate contradiction line")?;
    file.sync_all()
        .context("failed to fsync contradiction log")?;
    Ok(record)
}

pub fn append_replan_log(paths: &MissionPaths, input: &ReplanLogInput) -> Result<ReplanLogReport> {
    ensure_paths_match_mission(paths, &input.mission_id)?;
    validate_parent_loop_authority_for_mission_mutation(paths, "append-replan-log")?;
    if input.summary.trim().is_empty() {
        bail!("replan log summary must not be empty");
    }

    let timestamp = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .context("format replan log timestamp")?;
    let existing = if paths.replan_log().is_file() {
        fs::read_to_string(paths.replan_log())
            .with_context(|| format!("failed to read {}", paths.replan_log().display()))?
    } else {
        default_replan_log_body(paths)
    };

    let mut body = String::new();
    body.push_str(&format!("\n## {timestamp}\n"));
    body.push_str(&format!("- Reopened layer: `{:?}`\n", input.reopened_layer));
    body.push_str(&format!("- Summary: {}\n", input.summary.trim()));
    if !input.preserved_refs.is_empty() {
        body.push_str("- Preserved:\n");
        for value in &input.preserved_refs {
            body.push_str(&format!("  - {value}\n"));
        }
    }
    if !input.invalidated_refs.is_empty() {
        body.push_str("- Invalidated:\n");
        for value in &input.invalidated_refs {
            body.push_str(&format!("  - {value}\n"));
        }
    }
    if !input.evidence_refs.is_empty() {
        body.push_str("- Evidence refs:\n");
        for value in &input.evidence_refs {
            body.push_str(&format!("  - {value}\n"));
        }
    }

    fs::write(paths.replan_log(), format!("{existing}{body}"))
        .with_context(|| format!("failed to write {}", paths.replan_log().display()))?;

    Ok(ReplanLogReport {
        mission_id: input.mission_id.clone(),
        reopened_layer: input.reopened_layer.clone(),
        log_path: paths.replan_log().display().to_string(),
    })
}

pub fn open_selection_wait(
    ralph_root: &Path,
    input: &SelectionStateInput,
) -> Result<SelectionState> {
    let candidates = unique_strings(&input.candidate_mission_ids);
    if candidates.len() < 2 {
        bail!("selection waits require at least two distinct candidate missions");
    }
    let state = SelectionState {
        selection_request_id: Uuid::new_v4().to_string(),
        candidate_mission_ids: candidates,
        canonical_selection_request: input.canonical_selection_request.clone(),
        selected_mission_id: None,
        request_emitted_at: None,
        created_at: OffsetDateTime::now_utc(),
        resolved_at: None,
        cleared_at: None,
    };
    if let Some(parent) = selection_state_path(ralph_root).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    write_json(selection_state_path(ralph_root), &state)?;
    Ok(state)
}

pub fn resolve_selection_wait(
    ralph_root: &Path,
    input: &SelectionResolutionInput,
) -> Result<SelectionState> {
    let path = selection_state_path(ralph_root);
    let mut state: SelectionState = load_json(&path)?;
    if !state
        .candidate_mission_ids
        .iter()
        .any(|mission_id| mission_id == &input.selected_mission_id)
    {
        anyhow::bail!(
            "selected mission {} is not present in the open selection candidates",
            input.selected_mission_id
        );
    }
    if state.cleared_at.is_some() {
        anyhow::bail!("selection state has already been cleared");
    }
    state.selected_mission_id = Some(input.selected_mission_id.clone());
    if state.resolved_at.is_none() {
        state.resolved_at = Some(OffsetDateTime::now_utc());
    }
    write_json(path, &state)?;
    Ok(state)
}

pub fn consume_selection_wait(
    ralph_root: &Path,
    input: &SelectionConsumptionInput,
) -> Result<SelectionState> {
    let path = selection_state_path(ralph_root);
    let mut state: SelectionState = load_json(&path)?;
    if state.selection_request_id != input.selection_request_id {
        anyhow::bail!(
            "selection request id mismatch: expected {}, got {}",
            state.selection_request_id,
            input.selection_request_id
        );
    }
    if state.selected_mission_id.is_none() {
        anyhow::bail!("selection state has not been resolved yet");
    }
    if state.cleared_at.is_none() {
        state.cleared_at = Some(OffsetDateTime::now_utc());
        write_json(path, &state)?;
    }
    Ok(state)
}

pub fn supersede_selection_wait(
    ralph_root: &Path,
    input: &SelectionConsumptionInput,
) -> Result<SelectionState> {
    let path = selection_state_path(ralph_root);
    let mut state: SelectionState = load_json(&path)?;
    if state.selection_request_id != input.selection_request_id {
        anyhow::bail!(
            "selection request id mismatch: expected {}, got {}",
            state.selection_request_id,
            input.selection_request_id
        );
    }
    if state.cleared_at.is_none() {
        state.cleared_at = Some(OffsetDateTime::now_utc());
        write_json(path, &state)?;
    }
    Ok(state)
}

pub fn acknowledge_selection_request(
    ralph_root: &Path,
    input: &SelectionAcknowledgementInput,
) -> Result<SelectionState> {
    let path = selection_state_path(ralph_root);
    let mut state: SelectionState = load_json(&path)?;
    if state.selection_request_id != input.selection_request_id {
        anyhow::bail!(
            "selection request id mismatch: expected {}, got {}",
            state.selection_request_id,
            input.selection_request_id
        );
    }
    if state.request_emitted_at.is_none() {
        state.request_emitted_at = Some(OffsetDateTime::now_utc());
        write_json(path, &state)?;
    }
    Ok(state)
}

pub fn resolve_resume(repo_root: &Path, input: &ResolveResumeInput) -> Result<ResolveResumeReport> {
    let ralph_root = repo_root.join(".ralph");
    let missions_root = ralph_root.join("missions");
    let mut state_repairs_applied = Vec::new();

    if let Some(mission_id) = &input.mission_id {
        let pending_selection_request_id =
            load_optional_selection_state(&ralph_root)?.and_then(|selection_state| {
                (selection_state.cleared_at.is_none())
                    .then_some(selection_state.selection_request_id)
            });
        let mut report = resolve_selected_mission(
            repo_root,
            mission_id,
            SelectionOutcome::ExplicitMissionOverride,
            if pending_selection_request_id.is_some() {
                SelectionStateAction::Superseded
            } else {
                SelectionStateAction::None
            },
            &input.live_child_lanes,
            &mut state_repairs_applied,
        )?;
        if let Some(selection_request_id) = pending_selection_request_id {
            supersede_selection_wait(
                &ralph_root,
                &SelectionConsumptionInput {
                    selection_request_id,
                },
            )?;
            state_repairs_applied
                .push("superseded_selection_state_for_explicit_mission".to_string());
            report.state_repairs_applied = state_repairs_applied.clone();
        }
        return Ok(report);
    }

    if let Some(selection_state) = load_optional_selection_state(&ralph_root)? {
        if selection_state.selected_mission_id.is_none()
            && selection_state.cleared_at.is_none()
            && selection_state.candidate_mission_ids.len() < 2
        {
            supersede_selection_wait(
                &ralph_root,
                &SelectionConsumptionInput {
                    selection_request_id: selection_state.selection_request_id,
                },
            )?;
            state_repairs_applied.push("cleared_invalid_selection_state".to_string());
        } else if selection_state.selected_mission_id.is_none()
            && selection_state.cleared_at.is_none()
        {
            let current_candidates = crate::ralph::list_non_terminal_missions(&missions_root)?;
            match current_candidates.len() {
                0 => {
                    supersede_selection_wait(
                        &ralph_root,
                        &SelectionConsumptionInput {
                            selection_request_id: selection_state.selection_request_id,
                        },
                    )?;
                    state_repairs_applied
                        .push("cleared_stale_selection_state_without_candidates".to_string());
                    return Ok(ResolveResumeReport {
                        selected_mission_id: None,
                        selection_outcome: SelectionOutcome::NoActiveMission,
                        resume_status: ResumeStatus::NoActiveMission,
                        next_phase: None,
                        next_action: "No non-terminal missions are available for resume."
                            .to_string(),
                        latest_closeout_ref: None,
                        active_cycle_status: ActiveCycleStatus::None,
                        child_reconciliation: None,
                        selection_state_action: SelectionStateAction::Superseded,
                        state_repairs_applied,
                    });
                }
                1 => {
                    let selection_request_id = selection_state.selection_request_id.clone();
                    let mut report = resolve_selected_mission(
                        repo_root,
                        &current_candidates[0].0,
                        SelectionOutcome::AutoBoundSingleCandidate,
                        SelectionStateAction::Superseded,
                        &input.live_child_lanes,
                        &mut state_repairs_applied,
                    )?;
                    supersede_selection_wait(
                        &ralph_root,
                        &SelectionConsumptionInput {
                            selection_request_id,
                        },
                    )?;
                    state_repairs_applied
                        .push("collapsed_selection_state_to_single_candidate".to_string());
                    report.state_repairs_applied = state_repairs_applied.clone();
                    return Ok(report);
                }
                _ => {}
            }
            return Ok(selection_wait_report(
                selection_state.canonical_selection_request,
                SelectionOutcome::PreservedSelectionWait,
                SelectionStateAction::Preserved,
                state_repairs_applied,
            ));
        } else if selection_state.selected_mission_id.is_some()
            && selection_state.resolved_at.is_none()
            && selection_state.cleared_at.is_none()
        {
            supersede_selection_wait(
                &ralph_root,
                &SelectionConsumptionInput {
                    selection_request_id: selection_state.selection_request_id,
                },
            )?;
            state_repairs_applied.push("cleared_selection_state_missing_resolved_at".to_string());
        } else if let Some(selected_mission_id) = selection_state.selected_mission_id.clone()
            && selection_state.cleared_at.is_none()
        {
            let selection_request_id = selection_state.selection_request_id.clone();
            let mut report = resolve_selected_mission(
                repo_root,
                &selected_mission_id,
                SelectionOutcome::ConsumedResolvedSelection,
                SelectionStateAction::Consumed,
                &input.live_child_lanes,
                &mut state_repairs_applied,
            )?;
            consume_selection_wait(
                &ralph_root,
                &SelectionConsumptionInput {
                    selection_request_id,
                },
            )?;
            state_repairs_applied.push("consumed_resolved_selection_state".to_string());
            report.state_repairs_applied = state_repairs_applied.clone();
            return Ok(report);
        }
    }

    let candidates = crate::ralph::list_non_terminal_missions(&missions_root)?;
    match candidates.len() {
        0 => Ok(ResolveResumeReport {
            selected_mission_id: None,
            selection_outcome: SelectionOutcome::NoActiveMission,
            resume_status: ResumeStatus::NoActiveMission,
            next_phase: None,
            next_action: "No non-terminal missions are available for resume.".to_string(),
            latest_closeout_ref: None,
            active_cycle_status: ActiveCycleStatus::None,
            child_reconciliation: None,
            selection_state_action: SelectionStateAction::None,
            state_repairs_applied,
        }),
        1 => resolve_selected_mission(
            repo_root,
            &candidates[0].0,
            SelectionOutcome::AutoBoundSingleCandidate,
            SelectionStateAction::None,
            &input.live_child_lanes,
            &mut state_repairs_applied,
        ),
        _ => {
            let canonical_selection_request = "Select the mission to resume.".to_string();
            open_selection_wait(
                &ralph_root,
                &SelectionStateInput {
                    candidate_mission_ids: candidates
                        .iter()
                        .map(|(mission_id, _)| mission_id.clone())
                        .collect(),
                    canonical_selection_request: canonical_selection_request.clone(),
                },
            )?;
            Ok(selection_wait_report(
                canonical_selection_request,
                SelectionOutcome::OpenedSelectionWait,
                SelectionStateAction::Opened,
                state_repairs_applied,
            ))
        }
    }
}

pub fn resolve_stop_hook_output(
    repo_root: &Path,
    live_child_lanes: &[LiveChildLaneSnapshot],
) -> Result<StopHookOutput> {
    let ralph_root = repo_root.join(".ralph");

    if let Err(error) = load_optional_selection_state(&ralph_root) {
        return Ok(block_stop_output(format!(
            "Repair malformed selection state before continuing: {}",
            error
        )));
    }

    let lease = match load_ralph_loop_lease(repo_root) {
        Ok(lease) => lease,
        Err(error) => {
            return Ok(block_stop_output(format!(
                "Repair malformed Ralph loop lease before continuing: {}",
                error
            )));
        }
    };
    let enforce_actionable = matches!(
        lease.as_ref().map(|lease| &lease.status),
        Some(RalphLoopLeaseStatus::Active)
    );
    let selected_mission_id = lease
        .as_ref()
        .filter(|_| enforce_actionable)
        .map(|lease| lease.mission_id.clone());
    let report = match resolve_resume(
        repo_root,
        &ResolveResumeInput {
            mission_id: selected_mission_id,
            live_child_lanes: live_child_lanes.to_vec(),
        },
    ) {
        Ok(report) => report,
        Err(error) => {
            return Ok(block_stop_output(format!(
                "Repair resume state before continuing: {}",
                error
            )));
        }
    };

    stop_output_from_resume_report(repo_root, &ralph_root, &report, enforce_actionable)
}

pub fn begin_ralph_loop_lease(
    repo_root: &Path,
    input: &RalphLoopLeaseInput,
) -> Result<RalphLoopLease> {
    let paths = MissionPaths::try_new(repo_root, input.mission_id.clone())?;
    if input.owner.trim().is_empty() {
        anyhow::bail!("loop lease owner must not be empty");
    }
    match load_ralph_loop_lease(repo_root)? {
        Some(existing)
            if existing.parent_authority_verifier.is_some()
                && !parent_loop_authority_token_matches(&existing)? =>
        {
            anyhow::bail!(
                "active Ralph loop lease already owns parent authority for mission {}; pause or clear it from the parent, or provide {}",
                existing.mission_id,
                CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV
            );
        }
        Some(existing)
            if existing.parent_authority_verifier.is_none()
                && std::env::var(CODEX1_PARENT_LOOP_BEGIN_ENV).as_deref() != Ok("1") =>
        {
            anyhow::bail!(
                "begin-loop-lease cannot upgrade non-verifier lease for mission {} without parent begin authority; set {}=1 only in parent loop workflows",
                existing.mission_id,
                CODEX1_PARENT_LOOP_BEGIN_ENV
            );
        }
        Some(_) => {}
        None if std::env::var(CODEX1_PARENT_LOOP_BEGIN_ENV).as_deref() != Ok("1") => {
            anyhow::bail!(
                "begin-loop-lease requires parent begin authority; set {}=1 only in parent loop workflows",
                CODEX1_PARENT_LOOP_BEGIN_ENV
            );
        }
        None => {}
    }
    let now = OffsetDateTime::now_utc();
    let parent_authority_token = Uuid::new_v4().to_string();
    let parent_authority_verifier = ralph_loop_parent_authority_verifier(
        &input.mission_id,
        &input.mode,
        &parent_authority_token,
    )?;
    let lease = RalphLoopLease {
        mission_id: input.mission_id.clone(),
        mode: input.mode.clone(),
        status: RalphLoopLeaseStatus::Active,
        owner: input.owner.clone(),
        reason: input.reason.clone(),
        acquired_at: now,
        updated_at: now,
        parent_authority_verifier: Some(parent_authority_verifier),
        parent_authority_token: Some(parent_authority_token),
    };
    write_json(paths.loop_lease(), &persisted_ralph_loop_lease(&lease))?;
    Ok(lease)
}

pub fn pause_ralph_loop_lease(
    repo_root: &Path,
    input: &RalphLoopLeasePauseInput,
) -> Result<RalphLoopLeaseReport> {
    let path = ralph_loop_lease_path(repo_root);
    let Some(mut lease) = load_ralph_loop_lease(repo_root)? else {
        return Ok(RalphLoopLeaseReport {
            repo_root: repo_root.to_path_buf(),
            lease: None,
        });
    };
    if let Some(expected_mission_id) = input.mission_id.as_deref()
        && lease.mission_id != expected_mission_id
    {
        anyhow::bail!(
            "active loop lease belongs to mission {}, not {}",
            lease.mission_id,
            expected_mission_id
        );
    }
    if input.paused_by.trim().is_empty() {
        anyhow::bail!("loop lease paused_by must not be empty");
    }
    if lease.parent_authority_verifier.is_some() && !parent_loop_authority_token_matches(&lease)? {
        anyhow::bail!(
            "pause-loop-lease requires parent loop authority for verifier-backed lease {}; set {} from begin-loop-lease output",
            lease.mission_id,
            CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV
        );
    }
    lease.status = RalphLoopLeaseStatus::Paused;
    lease.owner = input.paused_by.clone();
    lease.reason = input.reason.clone();
    lease.updated_at = OffsetDateTime::now_utc();
    write_json(path, &lease)?;
    Ok(RalphLoopLeaseReport {
        repo_root: repo_root.to_path_buf(),
        lease: Some(lease),
    })
}

pub fn clear_ralph_loop_lease(repo_root: &Path) -> Result<RalphLoopLeaseReport> {
    let lease = load_ralph_loop_lease(repo_root)?;
    let path = ralph_loop_lease_path(repo_root);
    if let Some(existing) = lease.as_ref()
        && existing.parent_authority_verifier.is_some()
        && !parent_loop_authority_token_matches(existing)?
    {
        anyhow::bail!(
            "clear-loop-lease requires parent loop authority for verifier-backed lease {}; set {} from begin-loop-lease output",
            existing.mission_id,
            CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV
        );
    }
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
        if let Some(parent) = path.parent() {
            fsync_dir(parent)?;
        }
    }
    Ok(RalphLoopLeaseReport {
        repo_root: repo_root.to_path_buf(),
        lease,
    })
}

pub fn inspect_ralph_loop_lease(repo_root: &Path) -> Result<RalphLoopLeaseReport> {
    Ok(RalphLoopLeaseReport {
        repo_root: repo_root.to_path_buf(),
        lease: load_ralph_loop_lease(repo_root)?,
    })
}

fn load_ralph_loop_lease(repo_root: &Path) -> Result<Option<RalphLoopLease>> {
    let path = ralph_loop_lease_path(repo_root);
    if !path.is_file() {
        return Ok(None);
    }
    let lease = load_json(&path)
        .with_context(|| format!("failed to read Ralph loop lease {}", path.display()))?;
    Ok(Some(lease))
}

fn ralph_loop_lease_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".ralph").join("loop-lease.json")
}

fn persisted_ralph_loop_lease(lease: &RalphLoopLease) -> RalphLoopLease {
    let mut persisted = lease.clone();
    persisted.parent_authority_token = None;
    persisted
}

fn ralph_loop_parent_authority_verifier(
    mission_id: &str,
    mode: &RalphLoopLeaseMode,
    token: &str,
) -> Result<Fingerprint> {
    #[derive(Serialize)]
    struct Binding<'a> {
        mission_id: &'a str,
        mode: &'a RalphLoopLeaseMode,
        token: &'a str,
    }

    Ok(Fingerprint::from_json(&Binding {
        mission_id,
        mode,
        token,
    })?)
}

fn parent_loop_authority_token_matches(lease: &RalphLoopLease) -> Result<bool> {
    let Some(expected) = lease.parent_authority_verifier.as_ref() else {
        return Ok(true);
    };
    let Some(token) = std::env::var(CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(false);
    };
    let actual = ralph_loop_parent_authority_verifier(&lease.mission_id, &lease.mode, &token)?;
    Ok(&actual == expected)
}

fn validate_parent_loop_authority_for_mission_mutation(
    paths: &MissionPaths,
    action: &str,
) -> Result<()> {
    let Some(lease) = load_ralph_loop_lease(paths.repo_root())? else {
        return Ok(());
    };
    if lease.parent_authority_verifier.is_none() || lease.mission_id != paths.mission_id() {
        return Ok(());
    }
    if lease.status != RalphLoopLeaseStatus::Active {
        anyhow::bail!(
            "{} requires an active parent loop lease for mission {}; current lease is {:?}",
            action,
            paths.mission_id(),
            lease.status
        );
    }
    if !parent_loop_authority_token_matches(&lease)? {
        anyhow::bail!(
            "{} requires parent loop authority; set {} from begin-loop-lease output and do not pass it to child lanes",
            action,
            CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV
        );
    }
    Ok(())
}

fn validate_required_parent_loop_authority_for_review_writeback(
    paths: &MissionPaths,
    action: &str,
) -> Result<()> {
    let Some(lease) = load_ralph_loop_lease(paths.repo_root())? else {
        #[cfg(test)]
        {
            return Ok(());
        }
        #[cfg(not(test))]
        anyhow::bail!(
            "{} requires parent loop authority; begin a parent review loop lease and provide {}",
            action,
            CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV
        );
    };
    if lease.mission_id != paths.mission_id() {
        anyhow::bail!(
            "{} requires parent loop authority for mission {}; active lease belongs to {}",
            action,
            paths.mission_id(),
            lease.mission_id
        );
    }
    if lease.parent_authority_verifier.is_none() {
        anyhow::bail!(
            "{} requires a verifier-backed parent loop lease for mission {}",
            action,
            paths.mission_id()
        );
    }
    if lease.status != RalphLoopLeaseStatus::Active {
        anyhow::bail!(
            "{} requires an active parent loop lease for mission {}; current lease is {:?}",
            action,
            paths.mission_id(),
            lease.status
        );
    }
    if !parent_loop_authority_token_matches(&lease)? {
        anyhow::bail!(
            "{} requires parent loop authority; set {} from begin-loop-lease output and do not pass it to child lanes",
            action,
            CODEX1_PARENT_LOOP_AUTHORITY_TOKEN_ENV
        );
    }
    Ok(())
}

pub fn acknowledge_waiting_request(
    paths: &MissionPaths,
    input: &WaitingRequestAcknowledgementInput,
) -> Result<CloseoutRecord> {
    let existing_closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    let latest_closeout = existing_closeouts
        .last()
        .cloned()
        .context("waiting mission has no closeouts")?;
    if latest_closeout.verdict != Verdict::NeedsUser {
        anyhow::bail!(
            "mission {} is not currently waiting for user input",
            paths.mission_id()
        );
    }
    if latest_closeout.waiting_request_id.as_deref() != Some(input.waiting_request_id.as_str()) {
        anyhow::bail!(
            "waiting request id mismatch for mission {}; expected {:?}, got {}",
            paths.mission_id(),
            latest_closeout.waiting_request_id,
            input.waiting_request_id
        );
    }
    if latest_closeout.request_emitted_at.is_some() {
        return Ok(latest_closeout);
    }

    let next_seq = latest_closeout.closeout_seq + 1;
    let emitted_at = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .context("format waiting acknowledgement timestamp")?;
    let cycle_id = Uuid::new_v4().to_string();
    let closeout = CloseoutRecord {
        closeout_id: Some(Uuid::new_v4().to_string()),
        closeout_seq: next_seq,
        mission_id: paths.mission_id().to_string(),
        phase: latest_closeout.phase.clone(),
        activity: "waiting_request_acknowledged".to_string(),
        verdict: Verdict::NeedsUser,
        terminality: Terminality::WaitingNonTerminal,
        resume_mode: ResumeMode::YieldToUser,
        next_phase: latest_closeout.next_phase.clone(),
        next_action: latest_closeout.next_action.clone(),
        target: latest_closeout.target.clone(),
        cycle_kind: Some(CycleKind::WaitingHandshake),
        lock_revision: latest_closeout.lock_revision,
        lock_fingerprint: latest_closeout.lock_fingerprint.clone(),
        blueprint_revision: latest_closeout.blueprint_revision,
        blueprint_fingerprint: latest_closeout.blueprint_fingerprint.clone(),
        governing_revision: latest_closeout.governing_revision.clone(),
        reason_code: Some("waiting_request_emitted".to_string()),
        summary: Some(format!(
            "Acknowledged waiting request {} for mission {}",
            input.waiting_request_id,
            paths.mission_id()
        )),
        continuation_prompt: latest_closeout.continuation_prompt.clone(),
        cycle_id: Some(cycle_id),
        waiting_request_id: latest_closeout.waiting_request_id.clone(),
        waiting_for: latest_closeout.waiting_for.clone(),
        canonical_waiting_request: latest_closeout.canonical_waiting_request.clone(),
        resume_condition: latest_closeout.resume_condition.clone(),
        request_emitted_at: Some(emitted_at),
        active_child_task_paths: latest_closeout.active_child_task_paths.clone(),
        artifact_fingerprints: latest_closeout.artifact_fingerprints.clone(),
    };
    let active_cycle = active_cycle_from_closeout(
        &closeout,
        child_lane_expectations_from_task_paths(&latest_closeout.active_child_task_paths),
        vec!["waiting_request_persisted".to_string()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    append_closeout_for_active_cycle(paths, &closeout, &active_cycle)?;
    Ok(closeout)
}

pub fn write_closeout(
    paths: &MissionPaths,
    mut closeout: CloseoutRecord,
) -> Result<CloseoutRecord> {
    if matches!(closeout.verdict, Verdict::Complete | Verdict::HardBlocked) {
        anyhow::bail!(
            "terminal closeouts must come from workflow-specific reviewed paths, not codex1 internal write-closeout"
        );
    }
    let existing_closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    for (label, value) in [
        (
            "cycle_kind",
            closeout.cycle_kind.as_ref().map(|_| "present"),
        ),
        (
            "governing_revision",
            closeout
                .governing_revision
                .as_deref()
                .filter(|value| !value.is_empty()),
        ),
        (
            "reason_code",
            closeout
                .reason_code
                .as_deref()
                .filter(|value| !value.is_empty()),
        ),
        (
            "summary",
            closeout
                .summary
                .as_deref()
                .filter(|value| !value.is_empty()),
        ),
        (
            "continuation_prompt",
            closeout
                .continuation_prompt
                .as_deref()
                .filter(|value| !value.is_empty()),
        ),
    ] {
        if value.is_none() {
            anyhow::bail!("internal write-closeout requires {label}");
        }
    }
    if closeout.next_phase.as_deref().is_none_or(str::is_empty) {
        anyhow::bail!("internal write-closeout requires next_phase");
    }
    closeout.closeout_seq = existing_closeouts
        .last()
        .map_or(1, |record| record.closeout_seq + 1);
    if closeout.closeout_id.is_none() {
        closeout.closeout_id = Some(Uuid::new_v4().to_string());
    }
    let cycle_id = closeout
        .cycle_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    closeout.cycle_id = Some(cycle_id);
    let active_cycle = active_cycle_from_closeout(
        &closeout,
        child_lane_expectations_from_task_paths(&closeout.active_child_task_paths),
        Vec::new(),
        closeout.summary.clone().into_iter().collect::<Vec<_>>(),
        Vec::new(),
        Vec::new(),
    );
    append_closeout_for_active_cycle(paths, &closeout, &active_cycle)?;
    Ok(closeout)
}

fn load_latest_unresolved_contradiction(path: &Path) -> Result<Option<ContradictionRecord>> {
    let mut latest: Option<(u8, ContradictionRecord)> = None;
    for record in unresolved_contradiction_records(path)? {
        let severity = contradiction_resume_severity(&record);
        match &latest {
            Some((current, _)) if *current > severity => {}
            _ => latest = Some((severity, record)),
        }
    }
    Ok(latest.map(|(_, record)| record))
}

fn mission_close_contradiction_findings(paths: &MissionPaths) -> Result<Vec<String>> {
    Ok(
        unresolved_contradiction_records(&paths.contradictions_ndjson())?
            .into_iter()
            .map(|record| {
                format!(
                    "mission_close_unresolved_contradiction:{}:{:?}",
                    record.contradiction_id, record.status
                )
            })
            .collect(),
    )
}

fn contradiction_resume_override(record: &ContradictionRecord) -> Option<ResumeStateOverride> {
    let (resume_status, verdict, next_phase, reason_code, waiting_for, resume_condition, action) =
        match record.machine_action.clone() {
            Some(MachineAction::ContinueLocalExecution)
                if matches!(record.suggested_reopen_layer, ReopenLayer::ExecutionLocal) =>
            {
                return None;
            }
            Some(MachineAction::ForceReview) => (
                ResumeStatus::ContradictoryState,
                Verdict::ReviewRequired,
                Some("review".to_string()),
                "unresolved_contradiction_force_review".to_string(),
                None,
                None,
                format!(
                    "Review contradiction {} before continuing: {}.",
                    record.contradiction_id, record.violated_assumption_or_contract
                ),
            ),
            Some(MachineAction::ForceRepair) => (
                ResumeStatus::ContradictoryState,
                Verdict::RepairRequired,
                Some("execution".to_string()),
                "unresolved_contradiction_force_repair".to_string(),
                None,
                None,
                format!(
                    "Repair contradiction {} before continuing: {}.",
                    record.contradiction_id, record.violated_assumption_or_contract
                ),
            ),
            Some(MachineAction::YieldNeedsUser) => (
                ResumeStatus::WaitingNeedsUser,
                Verdict::NeedsUser,
                Some(record.discovered_in_phase.clone()),
                "unresolved_contradiction_needs_user".to_string(),
                Some("human_decision".to_string()),
                Some("The user resolves the contradiction truthfully.".to_string()),
                format!(
                    "Resolve contradiction {} with user input: {}.",
                    record.contradiction_id, record.violated_assumption_or_contract
                ),
            ),
            Some(MachineAction::HaltHardBlocked) => (
                ResumeStatus::ContradictoryState,
                Verdict::ReplanRequired,
                Some("replan".to_string()),
                "unresolved_contradiction_pending_hard_block_closeout".to_string(),
                None,
                None,
                format!(
                    "Mission may be honestly hard blocked because of contradiction {}: {}. Record a reviewed hard-block closeout before allowing stop.",
                    record.contradiction_id, record.violated_assumption_or_contract
                ),
            ),
            Some(MachineAction::ForceReplan) | None => (
                ResumeStatus::ContradictoryState,
                Verdict::ReplanRequired,
                Some("replan".to_string()),
                "unresolved_contradiction_force_replan".to_string(),
                None,
                None,
                format!(
                    "Replan after contradiction {}: {}.",
                    record.contradiction_id, record.violated_assumption_or_contract
                ),
            ),
            Some(MachineAction::ContinueLocalExecution) => (
                ResumeStatus::ContradictoryState,
                Verdict::ReplanRequired,
                Some(
                    match record.suggested_reopen_layer {
                        ReopenLayer::ExecutionPackage => "execution_package",
                        _ => "replan",
                    }
                    .to_string(),
                ),
                "unresolved_contradiction_force_replan".to_string(),
                None,
                None,
                format!(
                    "Replan after contradiction {}: {}.",
                    record.contradiction_id, record.violated_assumption_or_contract
                ),
            ),
        };
    Some(ResumeStateOverride {
        resume_status,
        verdict: verdict.clone(),
        next_phase,
        next_action: action.clone(),
        reason_code,
        waiting_for,
        canonical_waiting_request: if matches!(verdict, Verdict::NeedsUser) {
            Some(action)
        } else {
            None
        },
        resume_condition,
    })
}

fn contradiction_resume_severity(record: &ContradictionRecord) -> u8 {
    match record.machine_action {
        Some(MachineAction::ForceReplan)
        | Some(MachineAction::HaltHardBlocked)
        | Some(MachineAction::YieldNeedsUser) => 3,
        Some(MachineAction::ForceReview) | Some(MachineAction::ForceRepair) => 2,
        Some(MachineAction::ContinueLocalExecution) | None => 1,
    }
}

fn resolve_selected_mission(
    repo_root: &Path,
    mission_id: &str,
    selection_outcome: SelectionOutcome,
    selection_state_action: SelectionStateAction,
    live_child_lanes: &[LiveChildLaneSnapshot],
    state_repairs_applied: &mut Vec<String>,
) -> Result<ResolveResumeReport> {
    let paths = MissionPaths::try_new(repo_root, mission_id.to_string())?;
    let closeouts = load_closeouts(&paths.closeouts_ndjson())?;
    let latest_closeout = closeouts
        .last()
        .cloned()
        .with_context(|| format!("mission {mission_id} has no valid closeouts to resume"))?;

    let loaded_active_cycle = inspect_active_cycle(&paths.active_cycle())?;
    let mut active_cycle = match &loaded_active_cycle {
        ActiveCycleLoad::Parsed(cycle) => Some(cycle.clone()),
        ActiveCycleLoad::Missing | ActiveCycleLoad::Malformed => None,
    };
    let active_cycle_status = match loaded_active_cycle {
        ActiveCycleLoad::Malformed => ActiveCycleStatus::Contradictory,
        _ => classify_active_cycle(active_cycle.as_ref(), &latest_closeout),
    };

    if matches!(
        active_cycle_status,
        ActiveCycleStatus::StaleMatchingCloseout | ActiveCycleStatus::StaleSupersededCycle
    ) {
        fs::remove_file(paths.active_cycle()).with_context(|| {
            format!("failed to remove stale {}", paths.active_cycle().display())
        })?;
        active_cycle = None;
        state_repairs_applied.push("removed_stale_active_cycle".to_string());
    }

    let gates = load_gate_index(&paths)?;
    if !reconcile_execution_graph_obligations_from_passed_review_gates(&paths, &gates)?.is_empty() {
        state_repairs_applied
            .push("reconciled_execution_graph_obligations_from_passed_reviews".to_string());
    }

    let rebuilt_state = if active_cycle_status == ActiveCycleStatus::Contradictory {
        contradictory_active_cycle_state(
            &latest_closeout,
            active_cycle.as_ref().map(|cycle| cycle.cycle_id.clone()),
            "Repair contradictory active-cycle state before resume.",
        )
    } else {
        rebuild_state_from_closeouts(&closeouts, active_cycle.as_ref())?
    };
    let cached_state = load_state(&paths.state_json())?;

    let fingerprint_findings = current_fingerprint_findings(&paths, &latest_closeout)?;
    let contradiction_override =
        load_latest_unresolved_contradiction(&paths.contradictions_ndjson())?
            .and_then(|record| contradiction_resume_override(&record));
    let review_gate_override = open_review_gate_requirement(&paths)?;
    let child_reconciliation = reconcile_child_lanes(
        active_cycle.as_ref(),
        &rebuilt_state.active_child_task_paths,
        live_child_lanes,
    );
    let mut effective_state = rebuilt_state.clone();
    let (resume_status, next_phase, next_action, override_applied) =
        if active_cycle_status == ActiveCycleStatus::Contradictory {
            let override_state = ResumeStateOverride {
                resume_status: ResumeStatus::ContradictoryState,
                verdict: Verdict::ReplanRequired,
                next_phase: rebuilt_state.next_phase.clone(),
                next_action: "Repair contradictory active-cycle state before resume.".to_string(),
                reason_code: "contradictory_active_cycle".to_string(),
                waiting_for: None,
                canonical_waiting_request: None,
                resume_condition: None,
            };
            effective_state = apply_resume_state_override(&rebuilt_state, &override_state);
            (
                override_state.resume_status,
                override_state.next_phase.clone(),
                override_state.next_action.clone(),
                true,
            )
        } else if let Some(mut override_state) = contradiction_override {
            if !fingerprint_findings.is_empty() {
                override_state.next_action = format!(
                    "{} Governing artifact drift is also present: {}.",
                    override_state.next_action,
                    fingerprint_findings.join(", ")
                );
            }
            effective_state = apply_resume_state_override(&rebuilt_state, &override_state);
            preserve_cached_waiting_identity(
                &mut effective_state,
                cached_state.as_ref(),
                &override_state,
            );
            (
                override_state.resume_status,
                override_state.next_phase.clone(),
                override_state.next_action.clone(),
                true,
            )
        } else if !fingerprint_findings.is_empty() {
            let override_state = ResumeStateOverride {
                resume_status: ResumeStatus::ContradictoryState,
                verdict: Verdict::ReplanRequired,
                next_phase: Some("replan".to_string()),
                next_action: format!(
                    "Repair governing artifact drift before resume: {}.",
                    fingerprint_findings.join(", ")
                ),
                reason_code: "governing_artifact_drift".to_string(),
                waiting_for: None,
                canonical_waiting_request: None,
                resume_condition: None,
            };
            effective_state = apply_resume_state_override(&rebuilt_state, &override_state);
            (
                override_state.resume_status,
                override_state.next_phase.clone(),
                override_state.next_action.clone(),
                true,
            )
        } else if rebuilt_state.terminality == Terminality::Terminal {
            (
                ResumeStatus::Terminal,
                None,
                format!(
                    "Mission {} is already terminal with verdict {:?}.",
                    mission_id, rebuilt_state.verdict
                ),
                false,
            )
        } else if rebuilt_state.verdict == Verdict::NeedsUser {
            (
                ResumeStatus::WaitingNeedsUser,
                rebuilt_state.next_phase.clone(),
                rebuilt_state
                    .canonical_waiting_request
                    .clone()
                    .unwrap_or_else(|| rebuilt_state.next_action.clone()),
                false,
            )
        } else if let Some(next_action) = review_gate_override {
            let override_state = ResumeStateOverride {
                resume_status: ResumeStatus::ActionableNonTerminal,
                verdict: Verdict::ReviewRequired,
                next_phase: Some("review".to_string()),
                next_action,
                reason_code: "open_review_gate".to_string(),
                waiting_for: None,
                canonical_waiting_request: None,
                resume_condition: None,
            };
            effective_state = apply_resume_state_override(&rebuilt_state, &override_state);
            (
                override_state.resume_status,
                override_state.next_phase.clone(),
                override_state.next_action.clone(),
                true,
            )
        } else if active_cycle_status == ActiveCycleStatus::Interrupted {
            (
                ResumeStatus::InterruptedCycle,
                rebuilt_state.next_phase.clone(),
                format!(
                    "Recover interrupted cycle for mission {} before continuing: {}.",
                    mission_id, rebuilt_state.next_action
                ),
                false,
            )
        } else {
            (
                ResumeStatus::ActionableNonTerminal,
                rebuilt_state.next_phase.clone(),
                rebuilt_state.next_action.clone(),
                false,
            )
        };

    let persistable_state = if effective_state.reason_code.as_deref() == Some("open_review_gate") {
        &rebuilt_state
    } else {
        &effective_state
    };

    if cached_state.as_ref() != Some(persistable_state) {
        write_json(paths.state_json(), persistable_state)?;
        state_repairs_applied.push(if override_applied {
            "persisted_resume_state_repairs".to_string()
        } else {
            "rebuilt_state_from_closeouts".to_string()
        });
    }

    Ok(ResolveResumeReport {
        selected_mission_id: Some(mission_id.to_string()),
        selection_outcome,
        resume_status,
        next_phase,
        next_action,
        latest_closeout_ref: effective_state.last_valid_closeout_ref.clone(),
        active_cycle_status,
        child_reconciliation,
        selection_state_action,
        state_repairs_applied: state_repairs_applied.clone(),
    })
}

fn selection_wait_report(
    canonical_request: String,
    selection_outcome: SelectionOutcome,
    selection_state_action: SelectionStateAction,
    state_repairs_applied: Vec<String>,
) -> ResolveResumeReport {
    ResolveResumeReport {
        selected_mission_id: None,
        selection_outcome,
        resume_status: ResumeStatus::WaitingSelection,
        next_phase: None,
        next_action: canonical_request,
        latest_closeout_ref: None,
        active_cycle_status: ActiveCycleStatus::None,
        child_reconciliation: None,
        selection_state_action,
        state_repairs_applied,
    }
}

fn stop_output_from_resume_report(
    repo_root: &Path,
    ralph_root: &Path,
    report: &ResolveResumeReport,
    enforce_actionable: bool,
) -> Result<StopHookOutput> {
    match report.resume_status {
        ResumeStatus::NoActiveMission => Ok(StopHookOutput {
            continue_processing: true,
            decision: None,
            reason: None,
            system_message: None,
        }),
        ResumeStatus::WaitingSelection => {
            let current_selection_state = load_optional_selection_state(ralph_root)?
                .context("selection wait report returned without selection state")?;
            let emitted_selection_state = if current_selection_state.request_emitted_at.is_none() {
                acknowledge_selection_request(
                    ralph_root,
                    &SelectionAcknowledgementInput {
                        selection_request_id: current_selection_state.selection_request_id.clone(),
                    },
                )?
            } else {
                current_selection_state
            };
            Ok(StopHookOutput::for_selection_wait(&emitted_selection_state))
        }
        ResumeStatus::WaitingNeedsUser => {
            let mission_id = report
                .selected_mission_id
                .clone()
                .context("resolved waiting mission did not bind a mission id")?;
            let paths = MissionPaths::try_new(repo_root, mission_id)?;
            let state = load_state(&paths.state_json())?
                .context("waiting mission resolved without mission state")?;
            if state.request_emitted_at.is_none()
                && let Some(waiting_request_id) = state.waiting_request_id.clone()
                && latest_closeout_supports_waiting_ack(&paths, &waiting_request_id)?
            {
                if let Err(error) = acknowledge_waiting_request(
                    &paths,
                    &WaitingRequestAcknowledgementInput { waiting_request_id },
                ) {
                    return Ok(block_stop_output(format!(
                        "Repair waiting emission acknowledgement before continuing: {}",
                        error
                    )));
                }
            }
            Ok(StopHookOutput {
                continue_processing: true,
                decision: None,
                reason: None,
                system_message: Some(report.next_action.clone()),
            })
        }
        ResumeStatus::ActionableNonTerminal
        | ResumeStatus::InterruptedCycle
        | ResumeStatus::ContradictoryState => {
            if enforce_actionable {
                Ok(StopHookOutput {
                    continue_processing: true,
                    decision: Some("block".to_string()),
                    reason: Some(report.next_action.clone()),
                    system_message: None,
                })
            } else {
                Ok(StopHookOutput {
                    continue_processing: true,
                    decision: None,
                    reason: None,
                    system_message: Some(format!(
                        "Ralph loop is not active; pending mission action is not blocking this turn: {}",
                        report.next_action
                    )),
                })
            }
        }
        ResumeStatus::Terminal => {
            let mission_id = report
                .selected_mission_id
                .clone()
                .context("resolved terminal mission did not bind a mission id")?;
            let paths = MissionPaths::try_new(repo_root, mission_id)?;
            if latest_closeout_is_terminal(&paths)? {
                Ok(StopHookOutput {
                    continue_processing: true,
                    decision: None,
                    reason: None,
                    system_message: None,
                })
            } else {
                Ok(block_stop_output(
                    "Repair terminal mission state before continuing: latest closeout is not terminal.",
                ))
            }
        }
    }
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

fn classify_active_cycle(
    active_cycle: Option<&ActiveCycleState>,
    latest_closeout: &CloseoutRecord,
) -> ActiveCycleStatus {
    let Some(active_cycle) = active_cycle else {
        return ActiveCycleStatus::None;
    };

    if active_cycle.mission_id != latest_closeout.mission_id {
        return ActiveCycleStatus::Contradictory;
    }

    match latest_closeout.cycle_id.as_deref() {
        Some(closeout_cycle_id) if closeout_cycle_id == active_cycle.cycle_id => {
            ActiveCycleStatus::StaleMatchingCloseout
        }
        _ if active_cycle
            .opened_after_closeout_seq
            .is_none_or(|opened_after| opened_after >= latest_closeout.closeout_seq) =>
        {
            ActiveCycleStatus::Interrupted
        }
        _ => ActiveCycleStatus::StaleSupersededCycle,
    }
}

fn current_fingerprint_findings(
    paths: &MissionPaths,
    latest_closeout: &CloseoutRecord,
) -> Result<Vec<String>> {
    let mut findings = Vec::new();

    if let Some(lock_revision) = latest_closeout.lock_revision {
        let current_lock =
            load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())?;
        if current_lock.frontmatter.lock_revision != lock_revision {
            findings.push(format!(
                "outcome_lock_revision:{}!=current:{}",
                lock_revision, current_lock.frontmatter.lock_revision
            ));
        }
        if latest_closeout.lock_fingerprint.as_deref() != Some(current_lock.fingerprint()?.as_str())
        {
            findings.push("outcome_lock_fingerprint_drift".to_string());
        }
    }

    if let Some(blueprint_revision) = latest_closeout.blueprint_revision {
        let current_blueprint = load_markdown::<crate::artifacts::ProgramBlueprintFrontmatter>(
            &paths.program_blueprint(),
        )?;
        if current_blueprint.frontmatter.blueprint_revision != blueprint_revision {
            findings.push(format!(
                "program_blueprint_revision:{}!=current:{}",
                blueprint_revision, current_blueprint.frontmatter.blueprint_revision
            ));
        }
        let current_blueprint_contract_fingerprint =
            current_blueprint_contract_fingerprint(paths, &current_blueprint)?;
        if latest_closeout.blueprint_fingerprint.as_deref()
            != Some(current_blueprint_contract_fingerprint.as_str())
        {
            findings.push("program_blueprint_fingerprint_drift".to_string());
        }
    }

    if let Some(governing_revision) = latest_closeout.governing_revision.as_deref() {
        if let Some(package_id) = governing_revision.strip_prefix("package:") {
            let package_path = paths.execution_package(package_id);
            if !package_path.is_file() {
                findings.push(format!("governing_package_missing:{package_id}"));
            } else {
                let package_validation = validate_execution_package(paths, package_id)?;
                if !package_validation.valid {
                    findings.push(format!(
                        "governing_package_invalid:{}",
                        package_validation.findings.join("|")
                    ));
                }
            }
        } else if let Some(spec_ref) = governing_revision.strip_prefix("spec:")
            && let Some((spec_id, revision_raw)) = spec_ref.rsplit_once(':')
        {
            let spec_doc = load_markdown::<crate::artifacts::WorkstreamSpecFrontmatter>(
                &paths.spec_file(spec_id),
            )?;
            if spec_doc.frontmatter.spec_revision.to_string() != revision_raw {
                findings.push(format!(
                    "governing_spec_revision:{}!=current:{}",
                    revision_raw, spec_doc.frontmatter.spec_revision
                ));
            }
            if let Some(expected_fingerprint) = latest_closeout
                .artifact_fingerprints
                .get(&format!("spec:{spec_id}"))
                && spec_doc.fingerprint()?.as_str() != expected_fingerprint
            {
                findings.push(format!("governing_spec_fingerprint_drift:{spec_id}"));
            }
        }
    }

    Ok(findings)
}

fn reconcile_child_lanes(
    active_cycle: Option<&ActiveCycleState>,
    active_child_task_paths: &[String],
    live_child_lanes: &[LiveChildLaneSnapshot],
) -> Option<ChildLaneReconciliation> {
    let expected_lanes = expected_child_lanes_for_resume(active_cycle, active_child_task_paths);

    if expected_lanes.is_empty() {
        return None;
    }

    let mut live_index: BTreeMap<String, LiveChildLaneSnapshot> = BTreeMap::new();
    for lane in live_child_lanes {
        match live_index.get(&lane.task_path) {
            Some(existing)
                if live_child_lane_priority(&existing.status)
                    >= live_child_lane_priority(&lane.status) => {}
            _ => {
                live_index.insert(lane.task_path.clone(), lane.clone());
            }
        }
    }

    let entries = expected_lanes
        .iter()
        .map(|lane| {
            let classification = match live_index.get(&lane.task_path) {
                None => ChildLaneReconciliationClass::Missing,
                Some(snapshot) => match snapshot.status {
                    LiveChildLaneStatus::LiveNonFinal => ChildLaneReconciliationClass::LiveNonFinal,
                    LiveChildLaneStatus::FinalSuccess => {
                        if lane.integration_status == ChildLaneIntegrationStatus::Integrated {
                            ChildLaneReconciliationClass::FinalSuccessIntegrated
                        } else {
                            ChildLaneReconciliationClass::FinalSuccessUnintegrated
                        }
                    }
                    LiveChildLaneStatus::FinalNonSuccess => {
                        ChildLaneReconciliationClass::FinalNonSuccess
                    }
                },
            };
            ChildLaneReconciliationEntry {
                task_path: lane.task_path.clone(),
                lane_kind: lane.lane_kind.clone(),
                expected_deliverable_ref: lane.expected_deliverable_ref.clone(),
                lane_role: lane.lane_role.clone(),
                target_ref: lane.target_ref.clone(),
                integration_status: lane.integration_status.clone(),
                classification,
            }
        })
        .collect::<Vec<_>>();

    let recommended_action = if entries.iter().any(|entry| {
        matches!(
            entry.classification,
            ChildLaneReconciliationClass::Missing | ChildLaneReconciliationClass::FinalNonSuccess
        )
    }) {
        "recover_missing_or_failed_lanes".to_string()
    } else if entries
        .iter()
        .any(|entry| entry.classification == ChildLaneReconciliationClass::FinalSuccessUnintegrated)
    {
        "integrate_or_reject_completed_lane_deliverables".to_string()
    } else if entries
        .iter()
        .any(|entry| entry.classification == ChildLaneReconciliationClass::LiveNonFinal)
    {
        "continue_waiting_or_message_live_lanes".to_string()
    } else {
        "none".to_string()
    };

    Some(ChildLaneReconciliation {
        entries,
        recommended_action,
    })
}

fn expected_child_lanes_for_resume(
    active_cycle: Option<&ActiveCycleState>,
    active_child_task_paths: &[String],
) -> Vec<ChildLaneExpectation> {
    let active_by_path = active_cycle
        .map(|cycle| {
            cycle
                .normalized_expected_child_lanes()
                .into_iter()
                .map(|lane| (lane.task_path.clone(), lane))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    let mut lanes = Vec::new();

    for task_path in unique_strings(active_child_task_paths) {
        if !seen.insert(task_path.clone()) {
            continue;
        }
        if let Some(lane) = active_by_path.get(&task_path) {
            lanes.push(lane.clone());
        } else {
            lanes.push(ChildLaneExpectation {
                task_path: task_path.clone(),
                lane_kind: "closeout_state".to_string(),
                expected_deliverable_ref: format!("lane:{task_path}"),
                lane_role: None,
                integration_status: ChildLaneIntegrationStatus::Pending,
                target_ref: None,
            });
        }
    }

    if let Some(active_cycle) = active_cycle {
        for lane in active_cycle.normalized_expected_child_lanes() {
            if seen.insert(lane.task_path.clone()) {
                lanes.push(lane);
            }
        }
    }

    lanes
}

fn live_child_lane_priority(status: &LiveChildLaneStatus) -> u8 {
    match status {
        LiveChildLaneStatus::LiveNonFinal => 3,
        LiveChildLaneStatus::FinalNonSuccess => 2,
        LiveChildLaneStatus::FinalSuccess => 1,
    }
}

fn child_lane_expectations_from_task_paths(task_paths: &[String]) -> Vec<ChildLaneExpectation> {
    unique_strings(task_paths)
        .into_iter()
        .map(|task_path| ChildLaneExpectation {
            task_path: task_path.clone(),
            lane_kind: "unknown".to_string(),
            expected_deliverable_ref: format!("lane:{task_path}"),
            lane_role: None,
            integration_status: ChildLaneIntegrationStatus::Pending,
            target_ref: None,
        })
        .collect()
}

fn load_optional_selection_state(ralph_root: &Path) -> Result<Option<SelectionState>> {
    let path = selection_state_path(ralph_root);
    if !path.is_file() {
        return Ok(None);
    }
    load_json(&path).map(Some)
}

fn extract_first_heading(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        line.trim()
            .strip_prefix("# ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    })
}

fn default_mission_state_body(paths: &MissionPaths, title: &str, objective: &str) -> String {
    render_template_body_or_fallback(
        paths,
        "MISSION-STATE.md",
        &[
            ("MISSION_TITLE", title.to_string()),
            ("INTERPRETED_OBJECTIVE", objective.to_string()),
            ("CURRENT_PHASE_HINT", "clarify".to_string()),
            (
                "OBJECTIVE_CLARITY_NOTE",
                "Clarify the destination until the outcome lock can be ratified.".to_string(),
            ),
            (
                "OBJECTIVE_CLARITY_REDUCER",
                "Ask the highest-leverage mission question next.".to_string(),
            ),
            ("OBJECTIVE_CLARITY_SOURCE", "user ask".to_string()),
            (
                "SUCCESS_PROOF_NOTE",
                "Success criteria are not fully explicit yet.".to_string(),
            ),
            (
                "SUCCESS_PROOF_REDUCER",
                "Lock concrete done-when and proof expectations.".to_string(),
            ),
            ("SUCCESS_PROOF_SOURCE", "clarify synthesis".to_string()),
            (
                "PROTECTED_SURFACES_NOTE",
                "Protected surfaces still need to be named explicitly.".to_string(),
            ),
            (
                "PROTECTED_SURFACES_REDUCER",
                "Read the repo and confirm the sensitive surfaces.".to_string(),
            ),
            ("PROTECTED_SURFACES_SOURCE", "repo + user".to_string()),
            (
                "TRADEOFF_NOTE",
                "Tradeoff vetoes still need confirmation.".to_string(),
            ),
            (
                "TRADEOFF_REDUCER",
                "Ask what outcomes are unacceptable even if faster.".to_string(),
            ),
            ("TRADEOFF_SOURCE", "user ask".to_string()),
            (
                "SCOPE_NOTE",
                "Scope boundaries are still being narrowed.".to_string(),
            ),
            (
                "SCOPE_REDUCER",
                "Bound what is in mission scope and what must stay out.".to_string(),
            ),
            ("SCOPE_SOURCE", "clarify synthesis".to_string()),
            (
                "AUTONOMY_NOTE",
                "Codex autonomy boundaries are not fully explicit yet.".to_string(),
            ),
            (
                "AUTONOMY_REDUCER",
                "Record what Codex may decide and what remains user-only.".to_string(),
            ),
            ("AUTONOMY_SOURCE", "user ask".to_string()),
            (
                "BASELINE_FACTS_NOTE",
                "Baseline repo facts may still need verification.".to_string(),
            ),
            (
                "BASELINE_FACTS_REDUCER",
                "Read the repo until the critical current-state facts are grounded.".to_string(),
            ),
            ("BASELINE_FACTS_SOURCE", "repo".to_string()),
            (
                "ROLLOUT_NOTE",
                "Rollout and migration constraints are still provisional.".to_string(),
            ),
            (
                "ROLLOUT_REDUCER",
                "Ask whether rollout posture or compatibility constraints matter.".to_string(),
            ),
            ("ROLLOUT_SOURCE", "user ask".to_string()),
        ],
        "TBD during clarify",
        || {
            format!(
                "# Mission State\n\n## Objective Snapshot\n\n- Mission title: {title}\n- Current interpreted objective: {objective}\n- Current phase hint: clarify\n\n## Highest-Value Next Question\n\nWhat still needs to be made explicit before the Outcome Lock can be ratified?\n"
            )
        },
    )
}

fn default_outcome_lock_body(paths: &MissionPaths, objective: &str) -> String {
    render_template_body_or_fallback(
        paths,
        "OUTCOME-LOCK.md",
        &[("LOCKED_OBJECTIVE", objective.to_string())],
        "TBD during clarify",
        || {
            format!(
                "# Outcome Lock\n\n## Objective\n\n{objective}\n\n## Done-When Criteria\n\n- TBD during clarify\n\n## Protected Surfaces\n\n- TBD during clarify\n\n## Unacceptable Tradeoffs\n\n- TBD during clarify\n"
            )
        },
    )
}

fn current_primary_spec_id(paths: &MissionPaths, state: &RalphState) -> String {
    if let Some(spec_id) = state
        .target
        .as_deref()
        .and_then(|target| target.strip_prefix("spec:"))
    {
        return spec_id.to_string();
    }

    if paths.program_blueprint().is_file()
        && let Ok(blueprint) =
            load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())
        && let Ok(active_spec_ids) =
            load_active_blueprint_spec_ids(paths, blueprint.frontmatter.blueprint_revision)
        && let Some(spec_id) = active_spec_ids.first()
    {
        return spec_id.clone();
    }

    "spec-id".to_string()
}

fn current_objective_summary(paths: &MissionPaths) -> String {
    if paths.outcome_lock().is_file()
        && let Ok(lock_doc) =
            load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())
    {
        return extract_first_heading_or_sentence(&lock_doc.body);
    }

    if paths.mission_state().is_file()
        && let Ok(mission_state) = load_markdown::<MissionStateFrontmatter>(&paths.mission_state())
    {
        return extract_first_heading_or_sentence(&mission_state.body);
    }

    "See MISSION-STATE.md for the current objective snapshot.".to_string()
}

fn current_blockers(paths: &MissionPaths, state: &RalphState) -> Vec<String> {
    let mut blockers = Vec::new();

    if state.terminality == Terminality::WaitingNonTerminal {
        if let Some(request) = state.canonical_waiting_request.as_deref() {
            blockers.push(request.to_string());
        } else if let Some(waiting_for) = state.waiting_for.as_deref() {
            blockers.push(format!("Waiting on `{waiting_for}`."));
        }
    }

    if state.verdict != Verdict::Complete
        && let Ok(gates) = load_gate_index(paths)
    {
        for gate_ref in unresolved_blocking_gate_refs(&gates, None)
            .into_iter()
            .take(2)
        {
            blockers.push(format!("Blocking gate remains open: `{gate_ref}`."));
        }
    }

    if blockers.is_empty() {
        blockers.push(match state.verdict {
            Verdict::ContinueRequired => {
                "No blocker is recorded beyond the current governed next action.".to_string()
            }
            Verdict::ReviewRequired => {
                "A blocking review must pass before the mission can advance.".to_string()
            }
            Verdict::RepairRequired => {
                "A bounded repair is required before the mission can advance.".to_string()
            }
            Verdict::ReplanRequired => {
                "The governing contract must be reopened before execution can continue.".to_string()
            }
            Verdict::NeedsUser => state.next_action.clone(),
            Verdict::Complete => "Mission close conditions are satisfied.".to_string(),
            Verdict::HardBlocked => "The mission is hard blocked.".to_string(),
        });
    }

    unique_strings(&blockers)
}

fn next_gate_summary(state: &RalphState) -> String {
    if let Some(next_phase) = state.next_phase.as_deref() {
        format!("Next governed phase: `{next_phase}`.")
    } else {
        "No further phase is currently recorded.".to_string()
    }
}

fn verdict_label(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::ContinueRequired => "continue_required",
        Verdict::ReviewRequired => "review_required",
        Verdict::RepairRequired => "repair_required",
        Verdict::ReplanRequired => "replan_required",
        Verdict::NeedsUser => "needs_user",
        Verdict::Complete => "complete",
        Verdict::HardBlocked => "hard_blocked",
    }
}

fn render_mission_readme(paths: &MissionPaths, state: &RalphState) -> String {
    let title = fs::read_to_string(paths.readme())
        .ok()
        .and_then(|raw| extract_first_heading(&raw))
        .unwrap_or_else(|| paths.mission_id().to_string());
    let target = state
        .target
        .clone()
        .unwrap_or_else(|| format!("mission:{}", paths.mission_id()));
    let blockers = current_blockers(paths, state);
    let blocker_1 = blockers
        .first()
        .cloned()
        .unwrap_or_else(|| "No blocker recorded.".to_string());
    let blocker_2 = blockers
        .get(1)
        .cloned()
        .unwrap_or_else(|| blocker_1.clone());
    let why_this_is_next = state.summary.clone().unwrap_or_else(|| {
        format!(
            "The latest valid closeout requires this branch to advance via `{}`.",
            state.next_action
        )
    });

    render_raw_template_or_fallback(
        paths,
        "README.md",
        &[
            ("MISSION_TITLE", title.clone()),
            ("MISSION_ID", paths.mission_id().to_string()),
            ("CURRENT_PHASE", state.phase.clone()),
            ("CURRENT_VERDICT", verdict_label(&state.verdict).to_string()),
            ("NEXT_ACTION", state.next_action.clone()),
            ("CURRENT_BLOCKER", blocker_1.clone()),
            (
                "MISSION_OBJECTIVE_SUMMARY",
                current_objective_summary(paths),
            ),
            ("CURRENT_TARGET", target),
            ("WHY_THIS_IS_NEXT", why_this_is_next),
            ("NEXT_GATE", next_gate_summary(state)),
            ("BLOCKER_1", blocker_1),
            ("BLOCKER_2", blocker_2),
            ("PRIMARY_SPEC_ID", current_primary_spec_id(paths, state)),
        ],
        "TBD",
        || {
            format!(
                "# {title}\n\n## Snapshot\n\n- Mission id: `{}`\n- Current phase: `{}`\n- Current verdict: `{}`\n- Next recommended action: {}\n- Current blocker: {}\n\n## Start Here\n\n1. Read `OUTCOME-LOCK.md` for destination truth.\n2. Read `PROGRAM-BLUEPRINT.md` for route truth.\n3. Read `specs/{}/SPEC.md` if execution is active.\n\n## Objective Summary\n\n{}\n",
                paths.mission_id(),
                state.phase,
                verdict_label(&state.verdict),
                state.next_action,
                current_blockers(paths, state)
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "No blocker recorded.".to_string()),
                current_primary_spec_id(paths, state),
                current_objective_summary(paths)
            )
        },
    )
}

fn refresh_mission_readme(paths: &MissionPaths, state: &RalphState) -> Result<()> {
    fs::write(paths.readme(), render_mission_readme(paths, state))
        .with_context(|| format!("failed to write {}", paths.readme().display()))
}

fn default_readme_body(
    paths: &MissionPaths,
    mission_id: &str,
    title: &str,
    objective: &str,
) -> String {
    render_raw_template_or_fallback(
        paths,
        "README.md",
        &[
            ("MISSION_TITLE", title.to_string()),
            ("MISSION_ID", mission_id.to_string()),
            ("CURRENT_PHASE", "clarify".to_string()),
            ("CURRENT_VERDICT", "continue_required".to_string()),
            (
                "NEXT_ACTION",
                "Continue clarify until the lock is safe to ratify.".to_string(),
            ),
            (
                "CURRENT_BLOCKER",
                "Clarify still has open ambiguity that must be reduced honestly.".to_string(),
            ),
            ("MISSION_OBJECTIVE_SUMMARY", objective.to_string()),
            ("CURRENT_TARGET", format!("mission:{mission_id}")),
            (
                "WHY_THIS_IS_NEXT",
                "Clarify owns the mission until the outcome lock is ratified.".to_string(),
            ),
            ("NEXT_GATE", "Next governed phase: `clarify`.".to_string()),
            (
                "BLOCKER_1",
                "Clarify still has open ambiguity that must be reduced honestly.".to_string(),
            ),
            (
                "BLOCKER_2",
                "Read the repo and ask the highest-leverage next question.".to_string(),
            ),
            ("PRIMARY_SPEC_ID", "spec-id".to_string()),
        ],
        "TBD",
        || {
            format!(
                "# {title}\n\n## Snapshot\n\n- Mission id: `{mission_id}`\n- Current phase: `clarify`\n- Current verdict: `continue_required`\n- Next recommended action: Continue clarify until the lock is safe to ratify.\n- Current blocker: Clarify still has open ambiguity that must be reduced honestly.\n\n## Objective Summary\n\n{objective}\n"
            )
        },
    )
}

fn default_spec_body(paths: &MissionPaths, spec_id: &str, purpose: &str) -> String {
    let scope_hint = default_spec_scope_hint(spec_id);
    render_template_body_or_fallback(
        paths,
        "specs/SPEC.md",
        &[
            ("MISSION_ID", paths.mission_id().to_string()),
            ("SPEC_ID", spec_id.to_string()),
            ("SPEC_PURPOSE", purpose.to_string()),
            (
                "IN_SCOPE_1",
                format!("Execute the bounded `{spec_id}` slice."),
            ),
            (
                "IN_SCOPE_2",
                "Keep the workstream inside the declared scope frontier.".to_string(),
            ),
            (
                "IN_SCOPE_3",
                "Leave unrelated repo surfaces untouched.".to_string(),
            ),
            (
                "OUT_OF_SCOPE_1",
                "Unrelated feature work outside this bounded slice.".to_string(),
            ),
            (
                "OUT_OF_SCOPE_2",
                "Silent scope expansion without replanning.".to_string(),
            ),
            (
                "OUT_OF_SCOPE_3",
                "Undeclared proof or review contract changes.".to_string(),
            ),
            (
                "DEPENDENCY_1",
                "Outcome Lock and Program Blueprint stay current.".to_string(),
            ),
            (
                "DEPENDENCY_2",
                "The current execution package remains fresh.".to_string(),
            ),
            (
                "DEPENDENCY_3",
                "Review obligations stay aligned with the selected route.".to_string(),
            ),
            ("TOUCHED_SURFACE_1", scope_hint.clone()),
            (
                "TOUCHED_SURFACE_2",
                format!("PLANS/{}/specs/{spec_id}", paths.mission_id()),
            ),
            (
                "TOUCHED_SURFACE_3",
                "Mission review/proof support files.".to_string(),
            ),
            ("READ_PATH_1", scope_hint.clone()),
            ("READ_PATH_2", scope_hint.clone()),
            ("READ_PATH_3", scope_hint.clone()),
            ("WRITE_PATH_1", scope_hint.clone()),
            ("WRITE_PATH_2", scope_hint.clone()),
            ("WRITE_PATH_3", scope_hint.clone()),
            (
                "INTERFACE_1",
                format!("{spec_id} visible and hidden contracts"),
            ),
            (
                "INTERFACE_2",
                "Execution package and review bundle bindings".to_string(),
            ),
            ("INTERFACE_3", "Mission closeout continuity".to_string()),
            (
                "IMPLEMENTATION_SHAPE",
                "Keep the implementation bounded, explicit, and reviewable.".to_string(),
            ),
            ("PROOF_EXPECTATION_1", "cargo test".to_string()),
            (
                "PROOF_EXPECTATION_2",
                "Validate the selected bounded slice directly.".to_string(),
            ),
            (
                "PROOF_EXPECTATION_3",
                "Preserve existing non-breakage guarantees.".to_string(),
            ),
            (
                "NON_BREAKAGE_EXPECTATION_1",
                "Existing mission contracts still validate.".to_string(),
            ),
            (
                "NON_BREAKAGE_EXPECTATION_2",
                "No unrelated surfaces drift silently.".to_string(),
            ),
            (
                "NON_BREAKAGE_EXPECTATION_3",
                "Review evidence remains attributable.".to_string(),
            ),
            ("REVIEW_LENS_1", "correctness".to_string()),
            ("REVIEW_LENS_2", "evidence_adequacy".to_string()),
            ("REVIEW_LENS_3", "scope_conformance".to_string()),
            ("TRUTH_BASIS_REF_1", "OUTCOME-LOCK.md".to_string()),
            ("TRUTH_BASIS_REF_2", "PROGRAM-BLUEPRINT.md".to_string()),
            ("TRUTH_BASIS_REF_3", format!("specs/{spec_id}/REVIEW.md")),
            (
                "FRESHNESS_NOTE_1",
                "Generated from the current mission planning context.".to_string(),
            ),
            (
                "FRESHNESS_NOTE_2",
                "Refresh on scope, proof, or review contract change.".to_string(),
            ),
        ],
        "Fill in during planning",
        || {
            format!(
                "# Workstream Spec\n\n## Purpose\n\n{purpose}\n\n## In Scope\n\n- Fill in with the selected planning slice.\n\n## Out Of Scope\n\n- State what this spec must not absorb silently.\n\n## Dependencies\n\n- Fill in during planning\n\n## Touched Surfaces\n\n- Fill in during planning\n\n## Read Scope\n\n- Fill in during planning\n\n## Write Scope\n\n- Fill in during planning\n\n## Interfaces And Contracts Touched\n\n- Fill in during planning\n\n## Implementation Shape\n\nFill in during planning\n\n## Proof-Of-Completion Expectations\n\n- Fill in during planning\n\n## Non-Breakage Expectations\n\n- Fill in during planning\n\n## Review Lenses\n\n- Fill in during planning\n\n## Replan Boundary\n\n- Fill in during planning\n\n## Truth Basis Refs\n\n- Fill in during planning\n\n## Freshness Notes\n\n- Fill in during planning\n\n## Support Files\n\n- `REVIEW.md`\n- `NOTES.md`\n- `RECEIPTS/`\n"
            )
        },
    )
}

fn default_spec_scope_hint(spec_id: &str) -> String {
    let trimmed = spec_id
        .strip_prefix("spec_")
        .or_else(|| spec_id.strip_prefix("spec-"))
        .unwrap_or(spec_id)
        .trim_matches(|ch: char| ch == '_' || ch == '-');
    if trimmed.is_empty() {
        return "src".to_string();
    }

    format!("src/{}", trimmed.replace(['_', '-'], "/"))
}

fn default_spec_review_body(paths: &MissionPaths, spec_id: &str) -> String {
    render_raw_template_or_fallback(
        paths,
        "specs/REVIEW.md",
        &[
            ("MISSION_ID", paths.mission_id().to_string()),
            ("SPEC_ID", spec_id.to_string()),
            ("REVIEW_BUNDLE_ID", "pending".to_string()),
            ("REVIEW_BUNDLE_KIND", "pending".to_string()),
            ("SOURCE_PACKAGE_ID", "pending".to_string()),
            ("SPEC_REVIEW_GOVERNING_REFS", "pending".to_string()),
            ("LOCK_FINGERPRINT", "pending".to_string()),
            ("BLUEPRINT_FINGERPRINT", "pending".to_string()),
            ("SPEC_REVISION", "pending".to_string()),
            ("SPEC_FINGERPRINT", "pending".to_string()),
            ("MANDATORY_REVIEW_LENSES", "pending".to_string()),
            ("PROOF_ROWS_UNDER_REVIEW", "pending".to_string()),
            ("REVIEW_RECEIPTS", "pending".to_string()),
            ("CHANGED_FILES_OR_DIFF", "pending".to_string()),
            ("TOUCHED_INTERFACE_CONTRACTS", "pending".to_string()),
            ("REVIEW_BUNDLE_FRESHNESS", "pending".to_string()),
        ],
        "No review events recorded yet.",
        || "# Spec Review Notes\n\nNo review events recorded yet.\n".to_string(),
    )
}

fn default_spec_notes_body(paths: &MissionPaths, spec_id: &str) -> String {
    render_raw_template_or_fallback(
        paths,
        "specs/NOTES.md",
        &[
            ("MISSION_ID", paths.mission_id().to_string()),
            ("SPEC_ID", spec_id.to_string()),
        ],
        "No local notes recorded yet.",
        || "# Spec Notes\n\nNo local notes recorded yet.\n".to_string(),
    )
}

fn default_receipts_readme_body(paths: &MissionPaths, spec_id: &str) -> String {
    render_raw_template_or_fallback(
        paths,
        "specs/RECEIPTS/README.md",
        &[
            ("MISSION_ID", paths.mission_id().to_string()),
            ("SPEC_ID", spec_id.to_string()),
        ],
        "TBD",
        || "# Receipts\n\nStore proof receipts here.\n".to_string(),
    )
}

fn default_review_ledger_body(paths: &MissionPaths) -> String {
    render_raw_template_or_fallback(
        paths,
        "REVIEW-LEDGER.md",
        &[("MISSION_ID", paths.mission_id().to_string())],
        "No review events recorded yet.",
        || "# Review Ledger\n\nNo review events recorded yet.\n".to_string(),
    )
}

fn default_replan_log_body(paths: &MissionPaths) -> String {
    render_raw_template_or_fallback(
        paths,
        "REPLAN-LOG.md",
        &[("MISSION_ID", paths.mission_id().to_string())],
        "No replan events recorded yet.",
        || "# Replan Log\n\nNo replan events recorded yet.\n".to_string(),
    )
}

fn validate_planning_spec_state(
    spec_id: &str,
    artifact_status: SpecArtifactStatus,
    packetization_status: PacketizationStatus,
    execution_status: SpecExecutionStatus,
) -> Result<()> {
    if artifact_status != SpecArtifactStatus::Active
        && execution_status != SpecExecutionStatus::NotStarted
    {
        bail!(
            "spec {} has invalid state combination: {:?} specs cannot be {:?}",
            spec_id,
            artifact_status,
            execution_status
        );
    }
    if matches!(
        packetization_status,
        PacketizationStatus::NearFrontier
            | PacketizationStatus::ProvisionalBacklog
            | PacketizationStatus::DeferredTruthMotion
            | PacketizationStatus::Descoped
    ) && execution_status != SpecExecutionStatus::NotStarted
    {
        bail!(
            "spec {} has invalid state combination: {:?} packetization cannot be {:?}",
            spec_id,
            packetization_status,
            execution_status
        );
    }
    Ok(())
}

fn render_review_ledger(
    review_id: &str,
    input: &ReviewResultInput,
    bundle: &ReviewBundle,
    clean: bool,
    existing: Option<String>,
) -> String {
    let blocking = input
        .findings
        .iter()
        .filter(|finding| finding.blocking)
        .collect::<Vec<_>>();
    let open_blocking = if blocking.is_empty() {
        "| None | n/a | n/a | No open blocking findings | n/a | continue |".to_string()
    } else {
        blocking
            .iter()
            .map(|finding| {
                format!(
                    "| {} | {} | {} | {} | codex1 | {} |",
                    Uuid::new_v4(),
                    input
                        .target_spec_id
                        .clone()
                        .unwrap_or_else(|| "mission".to_string()),
                    finding.class,
                    finding.summary,
                    input
                        .next_required_branch
                        .clone()
                        .map(|branch| format!("{:?}", branch))
                        .unwrap_or_else(|| "repair".to_string())
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let all_findings = if input.findings.is_empty() {
        "| None | No findings recorded | no | none | n/a |".to_string()
    } else {
        input
            .findings
            .iter()
            .map(|finding| {
                format!(
                    "| {} | {} | {} | {} | {} |",
                    finding.class,
                    finding.summary,
                    if finding.blocking { "yes" } else { "no" },
                    if finding.evidence_refs.is_empty() {
                        "none".to_string()
                    } else {
                        finding.evidence_refs.join(", ")
                    },
                    if finding.disposition.is_empty() {
                        "pending".to_string()
                    } else {
                        finding.disposition.clone()
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let evidence_refs = if input.evidence_refs.is_empty() {
        "none".to_string()
    } else {
        input.evidence_refs.join(", ")
    };
    let disposition_notes = if input.disposition_notes.is_empty() {
        "- None recorded.".to_string()
    } else {
        input
            .disposition_notes
            .iter()
            .map(|note| format!("- {note}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mission_close_section = if bundle.bundle_kind == BundleKind::MissionClose {
        format!(
            "\n## Mission-Close Review\n\n- Mission id: `{}`\n- Bundle id: `{}`\n- Source package id: `{}`\n- Governing refs: lock:{} ({}) ; blueprint:{} ({})\n- Verdict: {}\n- Mission-level proof rows checked: {}\n- Cross-spec claims checked: {}\n- Visible artifact refs: {}\n- Open finding summary: {}\n- Deferred or descoped follow-ons: {}\n- Deferred or descoped work represented honestly: {}\n",
            bundle.mission_id,
            bundle.bundle_id,
            bundle.source_package_id,
            bundle.lock_revision,
            bundle.lock_fingerprint,
            bundle.blueprint_revision,
            bundle.blueprint_fingerprint,
            input.verdict,
            if bundle.mission_level_proof_rows.is_empty() {
                "none".to_string()
            } else {
                bundle.mission_level_proof_rows.join(", ")
            },
            if bundle.cross_spec_claim_refs.is_empty() {
                "none".to_string()
            } else {
                bundle.cross_spec_claim_refs.join(", ")
            },
            if bundle.visible_artifact_refs.is_empty() {
                "none".to_string()
            } else {
                bundle.visible_artifact_refs.join(", ")
            },
            if bundle.open_finding_summary.is_empty() {
                "none".to_string()
            } else {
                bundle.open_finding_summary.join("; ")
            },
            if bundle.deferred_descoped_follow_on_refs.is_empty() {
                "none".to_string()
            } else {
                bundle.deferred_descoped_follow_on_refs.join(", ")
            },
            if clean { "yes" } else { "no" },
        )
    } else {
        String::new()
    };

    let entry = format!(
        "## Review Event `{review_id}`\n\n### Open Blocking Findings\n\n| Finding id | Scope | Class | Summary | Owner | Next action |\n| --- | --- | --- | --- | --- | --- |\n{open_blocking}\n\n### Review Event Summary\n\n| Review id | Mission id | Reviewer | Kind | Bundle id | Source package id | Governing refs | Verdict | Blocking findings | Evidence refs |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n| {review_id} | {} | {} | {:?} | {} | {} | {} | {} | {} | {} |\n\n### Findings\n\n| Class | Summary | Blocking | Evidence refs | Disposition |\n| --- | --- | --- | --- | --- |\n{all_findings}\n\n### Dispositions\n\n{}{mission_close_section}",
        bundle.mission_id,
        input.reviewer,
        bundle.bundle_kind,
        bundle.bundle_id,
        bundle.source_package_id,
        if input.governing_refs.is_empty() {
            bundle.governing_revision.clone()
        } else {
            input.governing_refs.join(", ")
        },
        input.verdict,
        blocking.len(),
        evidence_refs,
        disposition_notes,
    );
    append_review_history(existing, "# Review Ledger", &entry)
}

fn render_spec_review(
    review_id: &str,
    spec_id: &str,
    input: &ReviewResultInput,
    bundle: &ReviewBundle,
    existing: Option<String>,
) -> String {
    let events = format!(
        "| {} | spec_review | {} | {} | {} | {} | {} | {} | {} |",
        review_id,
        input.reviewer,
        if input.governing_refs.is_empty() {
            "bundle-governing-context".to_string()
        } else {
            input.governing_refs.join(", ")
        },
        bundle.bundle_id,
        bundle.source_package_id,
        bundle.lock_fingerprint,
        bundle.blueprint_fingerprint,
        input.verdict,
    );
    let findings = if input.findings.is_empty() {
        "| NB-Note | No findings recorded | no | none | n/a |".to_string()
    } else {
        input
            .findings
            .iter()
            .map(|finding| {
                format!(
                    "| {} | {} | {} | {} | {} |",
                    finding.class,
                    finding.summary,
                    if finding.blocking { "yes" } else { "no" },
                    if finding.evidence_refs.is_empty() {
                        "none".to_string()
                    } else {
                        finding.evidence_refs.join(", ")
                    },
                    if finding.disposition.is_empty() {
                        "pending"
                    } else {
                        &finding.disposition
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let entry = format!(
        "## Review Event `{review_id}`\n\n### Spec\n\n- Mission id: `{}`\n- Spec id: `{}`\n- Bundle id: `{}`\n- Source package id: `{}`\n\n### Review Events\n\n| Review id | Kind | Reviewer | Governing refs | Bundle id | Source package id | Lock fp | Blueprint fp | Verdict |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n{}\n\n### Findings\n\n| Class | Summary | Blocking | Evidence refs | Disposition |\n| --- | --- | --- | --- | --- |\n{}\n",
        bundle.mission_id, spec_id, bundle.bundle_id, bundle.source_package_id, events, findings,
    );
    append_review_history(existing, "# Spec Review Notes", &entry)
}

fn append_review_history(existing: Option<String>, title: &str, entry: &str) -> String {
    match existing {
        Some(existing) if !existing.trim().is_empty() => {
            format!("{}\n\n{}\n", existing.trim_end(), entry)
        }
        _ => format!("{title}\n\n{entry}\n"),
    }
}

fn initial_gates(mission_id: &str, lock_passed: bool) -> Vec<MissionGateRecord> {
    let now = OffsetDateTime::now_utc();
    let mut gates = vec![MissionGateRecord {
        gate_id: format!("{}:outcome_lock:lock:1", mission_id),
        gate_kind: GateKind::OutcomeLock,
        target_ref: format!("mission:{}", mission_id),
        governing_refs: vec![format!("mission:{}", mission_id)],
        status: if lock_passed {
            MissionGateStatus::Passed
        } else {
            MissionGateStatus::Open
        },
        blocking: true,
        opened_at: now,
        evaluated_at: lock_passed.then_some(now),
        evaluated_against_ref: lock_passed.then_some("OUTCOME-LOCK.md".to_string()),
        evidence_refs: if lock_passed {
            vec![format!("PLANS/{mission_id}/OUTCOME-LOCK.md")]
        } else {
            Vec::new()
        },
        failure_refs: Vec::new(),
        superseded_by: None,
    }];
    if lock_passed {
        gates.push(MissionGateRecord {
            gate_id: format!("{}:planning_completion:lock:1", mission_id),
            gate_kind: GateKind::PlanningCompletion,
            target_ref: format!("mission:{}", mission_id),
            governing_refs: vec![format!("lock:{}", 1)],
            status: MissionGateStatus::Open,
            blocking: true,
            opened_at: now,
            evaluated_at: None,
            evaluated_against_ref: None,
            evidence_refs: Vec::new(),
            failure_refs: Vec::new(),
            superseded_by: None,
        });
    }
    gates
}

fn load_gate_index(paths: &MissionPaths) -> Result<MissionGateIndex> {
    if !paths.gates_json().is_file() {
        return Ok(MissionGateIndex {
            mission_id: paths.mission_id().to_string(),
            current_phase: "clarify".to_string(),
            updated_at: OffsetDateTime::now_utc(),
            gates: Vec::new(),
        });
    }
    load_json(&paths.gates_json())
}

fn append_gate(index: &mut MissionGateIndex, gate: MissionGateRecord) {
    if let Some(existing) = index
        .gates
        .iter_mut()
        .find(|existing| existing.gate_id == gate.gate_id)
    {
        *existing = gate;
    } else {
        index.gates.push(gate);
    }
}

fn supersede_matching_gates(
    index: &mut MissionGateIndex,
    gate_kind: GateKind,
    target_ref: &str,
    superseded_by: &str,
) {
    let now = OffsetDateTime::now_utc();
    for gate in &mut index.gates {
        if gate.gate_kind == gate_kind
            && gate.target_ref == target_ref
            && gate.gate_id != superseded_by
            && gate.status != MissionGateStatus::Superseded
        {
            gate.status = MissionGateStatus::Superseded;
            gate.superseded_by = Some(superseded_by.to_string());
            gate.evaluated_at.get_or_insert(now);
        }
    }
}

fn stale_matching_gates(index: &mut MissionGateIndex, gate_kind: GateKind, target_ref: &str) {
    let now = OffsetDateTime::now_utc();
    for gate in &mut index.gates {
        if gate.gate_kind == gate_kind
            && gate.target_ref == target_ref
            && !matches!(
                gate.status,
                MissionGateStatus::Superseded | MissionGateStatus::Stale
            )
        {
            gate.status = MissionGateStatus::Stale;
            gate.evaluated_at.get_or_insert(now);
        }
    }
}

fn unresolved_blocking_gate_refs(
    index: &MissionGateIndex,
    exclude_gate_id: Option<&str>,
) -> Vec<String> {
    index
        .gates
        .iter()
        .filter(|gate| gate.blocking)
        .filter(|gate| exclude_gate_id != Some(gate.gate_id.as_str()))
        .filter(|gate| {
            matches!(
                gate.status,
                MissionGateStatus::Open | MissionGateStatus::Failed | MissionGateStatus::Stale
            )
        })
        .map(|gate| format!("{}:{}", gate.gate_id, gate.target_ref))
        .collect()
}

fn unresolved_review_gate_blocks_resume(paths: &MissionPaths, gate: &MissionGateRecord) -> bool {
    if !gate.blocking
        || !matches!(
            gate.gate_kind,
            GateKind::BlockingReview | GateKind::MissionCloseReview
        )
        || !matches!(
            gate.status,
            MissionGateStatus::Open | MissionGateStatus::Failed | MissionGateStatus::Stale
        )
    {
        return false;
    }

    if gate.gate_kind == GateKind::BlockingReview
        && gate.status == MissionGateStatus::Stale
        && stale_review_gate_matches_completed_spec_contract(paths, gate).unwrap_or(false)
    {
        return false;
    }

    true
}

fn stale_review_gate_matches_completed_spec_contract(
    paths: &MissionPaths,
    gate: &MissionGateRecord,
) -> Result<bool> {
    let Some(spec_id) = gate.target_ref.strip_prefix("spec:") else {
        return Ok(false);
    };
    let Some(bundle_id) = gate.evaluated_against_ref.as_deref() else {
        return Ok(false);
    };
    let bundle: ReviewBundle = load_json(paths.review_bundle(bundle_id))?;
    if bundle.bundle_kind != BundleKind::SpecReview
        || bundle.target_spec_id.as_deref() != Some(spec_id)
    {
        return Ok(false);
    }
    let (Some(spec_revision), Some(spec_fingerprint)) =
        (bundle.spec_revision, bundle.spec_fingerprint.clone())
    else {
        return Ok(false);
    };
    let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(spec_id))?;
    if spec_doc.frontmatter.artifact_status != SpecArtifactStatus::Active
        || spec_doc.frontmatter.execution_status != SpecExecutionStatus::Complete
    {
        return Ok(false);
    }

    if spec_doc.frontmatter.spec_revision != spec_revision {
        return Ok(false);
    }

    if spec_doc.fingerprint()? == spec_fingerprint {
        return Ok(true);
    }

    let mut route_normalized_doc = spec_doc;
    route_normalized_doc.frontmatter.blueprint_revision = bundle.blueprint_revision;
    route_normalized_doc.frontmatter.blueprint_fingerprint = Some(bundle.blueprint_fingerprint);
    if route_normalized_doc.fingerprint()? == spec_fingerprint {
        return Ok(true);
    }

    route_normalized_doc.frontmatter.execution_status = SpecExecutionStatus::Packaged;
    Ok(route_normalized_doc.fingerprint()? == spec_fingerprint)
}

fn unresolved_blocking_gate_refs_for_source_package(
    index: &MissionGateIndex,
    source_package_id: &str,
    exclude_gate_id: Option<&str>,
) -> Vec<String> {
    index
        .gates
        .iter()
        .filter(|gate| gate.blocking)
        .filter(|gate| exclude_gate_id != Some(gate.gate_id.as_str()))
        .filter(|gate| gate.evaluated_against_ref.as_deref() == Some(source_package_id))
        .filter(|gate| {
            matches!(
                gate.status,
                MissionGateStatus::Open | MissionGateStatus::Failed | MissionGateStatus::Stale
            )
        })
        .map(|gate| format!("{}:{}", gate.gate_id, gate.target_ref))
        .collect()
}

fn gate_target_ref(target_type: &TargetType, target_id: &str) -> String {
    format!("{}:{}", target_type.as_phase_target(), target_id)
}

fn review_target_ref(bundle: &ReviewBundle) -> String {
    bundle.target_spec_id.clone().map_or_else(
        || format!("mission:{}", bundle.mission_id),
        |spec| format!("spec:{spec}"),
    )
}

fn review_phase_for_bundle(bundle_kind: &BundleKind) -> &'static str {
    match bundle_kind {
        BundleKind::SpecReview => "review",
        BundleKind::MissionClose => "mission_close",
    }
}

fn mission_close_target_ref(mission_id: &str) -> String {
    format!("mission:{mission_id}")
}

fn open_review_gate_requirement(paths: &MissionPaths) -> Result<Option<String>> {
    let gates = load_gate_index(paths)?;
    Ok(gates
        .gates
        .iter()
        .find(|gate| {
            unresolved_review_gate_blocks_resume(paths, gate)
                && !stale_mission_close_gate_is_obsolete_repair_context(&gates, gate)
        })
        .map(|gate| {
            format!(
                "Resolve the {:?} review gate {} for {} before continuing.",
                gate.status, gate.gate_id, gate.target_ref
            )
        }))
}

fn stale_mission_close_gate_is_obsolete_repair_context(
    index: &MissionGateIndex,
    gate: &MissionGateRecord,
) -> bool {
    gate.gate_kind == GateKind::MissionCloseReview
        && gate.status == MissionGateStatus::Stale
        && index.current_phase != "mission_close"
}

fn review_verdict_is_clean(verdict: &str) -> bool {
    matches!(
        verdict.trim().to_ascii_lowercase().as_str(),
        "clean" | "pass" | "passed" | "approved" | "approve" | "complete"
    )
}

fn execution_gate_id(package: &ExecutionPackage) -> String {
    format!(
        "{}:execution_package:{}:{}",
        package.mission_id,
        gate_target_ref(&package.target_type, &package.target_id),
        package.package_id
    )
}

fn planning_gate_id(mission_id: &str, blueprint_revision: u64) -> String {
    format!(
        "{}:planning_completion:blueprint:{}",
        mission_id, blueprint_revision
    )
}

fn review_gate_id(bundle: &ReviewBundle, gate_kind: GateKind) -> String {
    format!(
        "{}:{:?}:{}:{}",
        bundle.mission_id,
        gate_kind,
        review_target_ref(bundle),
        bundle.bundle_id
    )
}

fn invalidate_review_history_for_execution_target(
    index: &mut MissionGateIndex,
    package: &ExecutionPackage,
) {
    if package.target_type == TargetType::Mission {
        stale_matching_gates(
            index,
            GateKind::MissionCloseReview,
            &mission_close_target_ref(&package.mission_id),
        );
        for spec_id in package
            .included_specs
            .iter()
            .map(|included| format!("spec:{}", included.spec_id))
            .collect::<BTreeSet<_>>()
        {
            stale_matching_gates(index, GateKind::BlockingReview, &spec_id);
        }
        return;
    }

    stale_matching_gates(
        index,
        GateKind::BlockingReview,
        &gate_target_ref(&package.target_type, &package.target_id),
    );
    for spec_id in package
        .included_specs
        .iter()
        .map(|included| format!("spec:{}", included.spec_id))
        .collect::<BTreeSet<_>>()
    {
        stale_matching_gates(index, GateKind::BlockingReview, &spec_id);
    }
    stale_matching_gates(
        index,
        GateKind::MissionCloseReview,
        &mission_close_target_ref(&package.mission_id),
    );
}

fn required_mission_close_review_lenses() -> Vec<String> {
    REQUIRED_MISSION_CLOSE_REVIEW_LENSES
        .iter()
        .map(|lens| lens.to_string())
        .collect()
}

fn active_cycle_from_closeout(
    closeout: &CloseoutRecord,
    expected_child_lanes: Vec<ChildLaneExpectation>,
    preconditions_checked: Vec<String>,
    expected_outputs: Vec<String>,
    active_packet_refs: Vec<String>,
    active_bundle_refs: Vec<String>,
) -> ActiveCycleState {
    let cycle_id = closeout
        .cycle_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut active_cycle = ActiveCycleState::new(
        cycle_id,
        closeout.mission_id.clone(),
        closeout.phase.clone(),
        closeout.target.clone(),
        expected_child_lanes,
    );
    active_cycle.opened_after_closeout_seq = Some(closeout.closeout_seq.saturating_sub(1));
    active_cycle.cycle_kind = closeout.cycle_kind.clone();
    active_cycle.activity = Some(closeout.activity.clone());
    active_cycle.lock_revision = closeout.lock_revision;
    active_cycle.lock_fingerprint = closeout.lock_fingerprint.clone();
    active_cycle.blueprint_revision = closeout.blueprint_revision;
    active_cycle.blueprint_fingerprint = closeout.blueprint_fingerprint.clone();
    active_cycle.governing_revision = closeout.governing_revision.clone();
    active_cycle.current_bounded_action = Some(closeout.next_action.clone());
    active_cycle.preconditions_checked = unique_strings(&preconditions_checked);
    active_cycle.expected_outputs = unique_strings(&expected_outputs);
    active_cycle.active_packet_refs = unique_strings(&active_packet_refs);
    active_cycle.active_bundle_refs = unique_strings(&active_bundle_refs);
    active_cycle
}

fn append_closeout_for_active_cycle(
    paths: &MissionPaths,
    closeout: &CloseoutRecord,
    active_cycle: &ActiveCycleState,
) -> Result<()> {
    write_json(paths.active_cycle(), active_cycle)?;
    match append_closeout_and_rebuild_state(
        &paths.hidden_mission_root(),
        closeout,
        Some(active_cycle),
    ) {
        Ok(state) => {
            refresh_mission_readme(paths, &state)?;
            Ok(())
        }
        Err(error) => {
            cleanup_transient_active_cycle(&paths.active_cycle(), &active_cycle.cycle_id)?;
            Err(error)
        }
    }
}

fn cleanup_transient_active_cycle(path: &Path, cycle_id: &str) -> Result<()> {
    match inspect_active_cycle(path)? {
        ActiveCycleLoad::Parsed(active_cycle) if active_cycle.cycle_id == cycle_id => {
            if path.exists() {
                fs::remove_file(path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ResumeStateOverride {
    resume_status: ResumeStatus,
    verdict: Verdict,
    next_phase: Option<String>,
    next_action: String,
    reason_code: String,
    waiting_for: Option<String>,
    canonical_waiting_request: Option<String>,
    resume_condition: Option<String>,
}

fn apply_resume_state_override(
    base: &RalphState,
    override_state: &ResumeStateOverride,
) -> RalphState {
    let mut state = base.clone();
    state.verdict = override_state.verdict.clone();
    state.terminality = match override_state.verdict {
        Verdict::Complete | Verdict::HardBlocked => Terminality::Terminal,
        Verdict::NeedsUser => Terminality::WaitingNonTerminal,
        _ => Terminality::ActionableNonTerminal,
    };
    state.resume_mode = match override_state.verdict {
        Verdict::Complete | Verdict::HardBlocked => ResumeMode::AllowStop,
        Verdict::NeedsUser => ResumeMode::YieldToUser,
        _ => ResumeMode::Continue,
    };
    state.next_phase = override_state.next_phase.clone();
    state.next_action = override_state.next_action.clone();
    state.reason_code = Some(override_state.reason_code.clone());
    state.summary = Some(override_state.next_action.clone());
    state.continuation_prompt = Some(override_state.next_action.clone());
    if override_state.verdict == Verdict::NeedsUser {
        state.waiting_for = override_state.waiting_for.clone();
        state.canonical_waiting_request = override_state.canonical_waiting_request.clone();
        state.resume_condition = override_state.resume_condition.clone();
        if state
            .waiting_request_id
            .as_deref()
            .is_none_or(str::is_empty)
        {
            state.waiting_request_id = Some(Uuid::new_v4().to_string());
        }
        state.request_emitted_at = None;
    } else {
        state.waiting_request_id = None;
        state.waiting_for = None;
        state.canonical_waiting_request = None;
        state.resume_condition = None;
        state.request_emitted_at = None;
    }
    state
}

fn preserve_cached_waiting_identity(
    state: &mut RalphState,
    cached_state: Option<&RalphState>,
    override_state: &ResumeStateOverride,
) {
    if override_state.verdict != Verdict::NeedsUser {
        return;
    }
    let Some(cached) = cached_state else {
        return;
    };
    if cached.verdict != Verdict::NeedsUser
        || cached
            .waiting_request_id
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return;
    }
    if cached.waiting_for != override_state.waiting_for
        || cached.canonical_waiting_request != override_state.canonical_waiting_request
        || cached.resume_condition != override_state.resume_condition
        || cached.next_phase != override_state.next_phase
        || cached.reason_code.as_deref() != Some(override_state.reason_code.as_str())
    {
        return;
    }
    state.waiting_request_id = cached.waiting_request_id.clone();
    state.request_emitted_at = cached.request_emitted_at.clone();
}

fn invalidate_post_planning_history(
    index: &mut MissionGateIndex,
    mission_id: &str,
    new_gate_id: &str,
    planning_contract_changed: bool,
) {
    let mission_ref = mission_close_target_ref(mission_id);
    supersede_matching_gates(
        index,
        GateKind::PlanningCompletion,
        &mission_ref,
        new_gate_id,
    );
    if !planning_contract_changed {
        return;
    }
    for gate in &mut index.gates {
        if matches!(
            gate.gate_kind,
            GateKind::ExecutionPackage | GateKind::BlockingReview | GateKind::MissionCloseReview
        ) && !matches!(
            gate.status,
            MissionGateStatus::Superseded | MissionGateStatus::Stale
        ) {
            gate.status = MissionGateStatus::Stale;
            gate.evaluated_at.get_or_insert(OffsetDateTime::now_utc());
        }
    }
}

fn load_active_blueprint_spec_ids(
    paths: &MissionPaths,
    blueprint_revision: u64,
) -> Result<Vec<String>> {
    let mut spec_ids = Vec::new();
    if !paths.specs_root().is_dir() {
        return Ok(spec_ids);
    }
    for entry in fs::read_dir(paths.specs_root())
        .with_context(|| format!("failed to read {}", paths.specs_root().display()))?
    {
        let entry = entry.context("failed to read spec dir entry")?;
        if !entry.path().is_dir() {
            continue;
        }
        let spec_id = entry.file_name().to_string_lossy().to_string();
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&spec_id))?;
        if spec_doc.frontmatter.artifact_status == SpecArtifactStatus::Active
            && spec_doc.frontmatter.blueprint_revision == blueprint_revision
        {
            spec_ids.push(spec_id);
        }
    }
    spec_ids.sort();
    Ok(spec_ids)
}

fn supersede_omitted_spec(
    paths: &MissionPaths,
    spec_id: &str,
    blueprint_revision: u64,
    blueprint_fingerprint: &Fingerprint,
) -> Result<()> {
    let path = paths.spec_file(spec_id);
    let mut spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&path)?;
    if spec_doc.frontmatter.artifact_status != SpecArtifactStatus::Active {
        return Ok(());
    }
    spec_doc.frontmatter.spec_revision += 1;
    spec_doc.frontmatter.artifact_status = SpecArtifactStatus::Superseded;
    spec_doc.frontmatter.blueprint_revision = blueprint_revision;
    spec_doc.frontmatter.blueprint_fingerprint = Some(blueprint_fingerprint.clone());
    fs::write(&path, spec_doc.render()?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    let path = path.as_ref();
    let encoded = serde_json::to_vec_pretty(value).context("failed to serialize json")?;
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let mut temp = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temp file in {}", parent.display()))?;
    temp.write_all(&encoded)
        .with_context(|| format!("failed to write temp file for {}", path.display()))?;
    temp.as_file()
        .sync_all()
        .with_context(|| format!("failed to fsync temp file for {}", path.display()))?;
    temp.persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to persist {}", path.display()))?;
    fsync_dir(parent)?;
    Ok(())
}

fn fsync_dir(path: &Path) -> Result<()> {
    File::open(path)
        .with_context(|| format!("failed to open directory {}", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to fsync directory {}", path.display()))
}

fn load_json<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let path = path.as_ref();
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

fn resolve_templates_root(paths: &MissionPaths) -> Option<PathBuf> {
    if let Some(explicit) = env::var_os("CODEX1_TEMPLATES_ROOT") {
        let candidate = PathBuf::from(explicit);
        if candidate.join("mission").is_dir() {
            return fs::canonicalize(candidate).ok();
        }
    }

    let repo_candidate = paths.repo_root().join("templates");
    if repo_candidate.join("mission").is_dir() {
        if let Ok(canonical) = fs::canonicalize(repo_candidate) {
            return Some(canonical);
        }
    }

    let exe = env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    for candidate in [
        exe_dir.join("templates"),
        exe_dir.join("../share/codex1/templates"),
    ] {
        if candidate.join("mission").is_dir() {
            if let Ok(canonical) = fs::canonicalize(candidate) {
                return Some(canonical);
            }
        }
    }

    for ancestor in exe.ancestors() {
        let candidate = ancestor.join("templates");
        if ancestor.join("docs/codex1-prd.md").is_file() && candidate.join("mission").is_dir() {
            if let Ok(canonical) = fs::canonicalize(candidate) {
                return Some(canonical);
            }
        }
    }

    None
}

fn render_template(
    template: &str,
    replacements: &[(&str, String)],
    unresolved_placeholder: &str,
) -> String {
    let replacements = replacements
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut rendered = String::with_capacity(template.len());
    let mut cursor = 0;

    while let Some(start_rel) = template[cursor..].find("{{") {
        let start = cursor + start_rel;
        rendered.push_str(&template[cursor..start]);
        let placeholder_start = start + 2;
        if let Some(end_rel) = template[placeholder_start..].find("}}") {
            let end = placeholder_start + end_rel;
            let key = template[placeholder_start..end].trim();
            if let Some(value) = replacements.get(key) {
                rendered.push_str(value);
            } else {
                rendered.push_str(unresolved_placeholder);
            }
            cursor = end + 2;
        } else {
            rendered.push_str(&template[start..]);
            cursor = template.len();
        }
    }

    if cursor < template.len() {
        rendered.push_str(&template[cursor..]);
    }

    rendered
}

fn extract_markdown_template_body(raw: &str) -> Option<&str> {
    if !raw.starts_with("---\n") {
        return None;
    }
    let remainder = &raw[4..];
    let end = remainder.find("\n---\n")?;
    Some(&remainder[end + 5..])
}

fn render_template_body_or_fallback(
    paths: &MissionPaths,
    relative_path: &str,
    replacements: &[(&str, String)],
    unresolved_placeholder: &str,
    fallback: impl FnOnce() -> String,
) -> String {
    if let Some(root) = resolve_templates_root(paths) {
        let path = root.join("mission").join(relative_path);
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Some(body) = extract_markdown_template_body(&raw) {
                return render_template(body, replacements, unresolved_placeholder);
            }
        }
    }
    fallback()
}

fn render_raw_template_or_fallback(
    paths: &MissionPaths,
    relative_path: &str,
    replacements: &[(&str, String)],
    unresolved_placeholder: &str,
    fallback: impl FnOnce() -> String,
) -> String {
    if let Some(root) = resolve_templates_root(paths) {
        let path = root.join("mission").join(relative_path);
        if let Ok(raw) = fs::read_to_string(&path) {
            return render_template(&raw, replacements, unresolved_placeholder);
        }
    }
    fallback()
}

fn load_markdown<F>(path: &Path) -> Result<ArtifactDocument<F>>
where
    F: crate::TypedArtifactFrontmatter,
{
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(ArtifactDocument::<F>::parse(&raw)?)
}

fn load_spec_contexts(paths: &MissionPaths, spec_ids: &[String]) -> Result<Vec<LoadedSpecContext>> {
    let mut contexts = Vec::new();
    for spec_id in unique_strings(spec_ids) {
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&spec_id))?;
        contexts.push(LoadedSpecContext {
            included: IncludedSpecRef {
                spec_id,
                spec_revision: spec_doc.frontmatter.spec_revision,
                spec_fingerprint: spec_doc.fingerprint()?,
            },
            artifact_status: spec_doc.frontmatter.artifact_status,
            packetization_status: spec_doc.frontmatter.packetization_status,
            blueprint_revision: spec_doc.frontmatter.blueprint_revision,
            blueprint_fingerprint: spec_doc.frontmatter.blueprint_fingerprint.clone(),
            replan_boundary: spec_doc.frontmatter.replan_boundary.clone(),
        });
    }
    Ok(contexts)
}

fn is_runnable_packetization_status(status: PacketizationStatus) -> bool {
    matches!(
        status,
        PacketizationStatus::Runnable | PacketizationStatus::ProofGatedSpike
    )
}

fn load_runnable_blueprint_spec_ids(
    paths: &MissionPaths,
    blueprint_revision: u64,
) -> Result<Vec<String>> {
    let mut spec_ids = Vec::new();
    if !paths.specs_root().is_dir() {
        return Ok(spec_ids);
    }
    for entry in fs::read_dir(paths.specs_root())
        .with_context(|| format!("failed to read {}", paths.specs_root().display()))?
    {
        let entry = entry.context("failed to read spec dir entry")?;
        if !entry.path().is_dir() {
            continue;
        }
        let spec_id = entry.file_name().to_string_lossy().to_string();
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&spec_id))?;
        if spec_doc.frontmatter.artifact_status == SpecArtifactStatus::Active
            && spec_doc.frontmatter.blueprint_revision == blueprint_revision
            && is_runnable_packetization_status(spec_doc.frontmatter.packetization_status)
        {
            spec_ids.push(spec_id);
        }
    }
    spec_ids.sort();
    Ok(spec_ids)
}

fn execution_graph_spec_contract_fingerprint(
    spec_doc: &ArtifactDocument<WorkstreamSpecFrontmatter>,
) -> Result<Fingerprint> {
    let mut normalized = spec_doc.clone();
    normalized.frontmatter.execution_status = SpecExecutionStatus::NotStarted;
    Ok(normalized.fingerprint()?)
}

fn runnable_spec_ids_from_inputs(specs: &[WorkstreamSpecInput]) -> Vec<String> {
    unique_strings(
        &specs
            .iter()
            .filter(|spec| {
                spec.artifact_status.unwrap_or(SpecArtifactStatus::Draft)
                    == SpecArtifactStatus::Active
                    && is_runnable_packetization_status(
                        spec.packetization_status
                            .unwrap_or(PacketizationStatus::NearFrontier),
                    )
            })
            .map(|spec| spec.spec_id.clone())
            .collect::<Vec<_>>(),
    )
}

fn normalize_proof_matrix_row(row: &ProofMatrixRow) -> ProofMatrixRow {
    let mut normalized = row.clone();
    normalized.claim_ref = normalized.claim_ref.trim().to_string();
    normalized.statement = normalized.statement.trim().to_string();
    normalized.required_evidence = unique_strings(&normalized.required_evidence);
    normalized.review_lenses = unique_strings(&normalized.review_lenses);
    normalized.governing_contract_refs = unique_strings(&normalized.governing_contract_refs);
    normalized
}

fn normalize_proof_matrix(rows: &[ProofMatrixRow]) -> Vec<ProofMatrixRow> {
    let mut normalized = rows
        .iter()
        .map(normalize_proof_matrix_row)
        .collect::<Vec<_>>();
    normalized.sort_by(|left, right| left.claim_ref.cmp(&right.claim_ref));
    normalized
}

fn normalize_decision_obligation(obligation: &DecisionObligation) -> DecisionObligation {
    let mut normalized = obligation.clone();
    normalized.obligation_id = normalized.obligation_id.trim().to_string();
    normalized.question = normalized.question.trim().to_string();
    normalized.why_it_matters = normalized.why_it_matters.trim().to_string();
    normalized.affects.sort();
    normalized.affects.dedup();
    normalized.governing_contract_refs = unique_strings(&normalized.governing_contract_refs);
    normalized.review_contract_refs = unique_strings(&normalized.review_contract_refs);
    normalized.mission_close_claim_refs = unique_strings(&normalized.mission_close_claim_refs);
    normalized.required_evidence = unique_strings(&normalized.required_evidence);
    normalized.resolution_rationale = normalized
        .resolution_rationale
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    normalized.evidence_refs = unique_strings(&normalized.evidence_refs);
    normalized.proof_spike_scope = normalized
        .proof_spike_scope
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    normalized.proof_spike_success_criteria =
        unique_strings(&normalized.proof_spike_success_criteria);
    normalized.proof_spike_failure_criteria =
        unique_strings(&normalized.proof_spike_failure_criteria);
    normalized.proof_spike_discharge_artifacts =
        unique_strings(&normalized.proof_spike_discharge_artifacts);
    normalized
}

fn normalize_decision_obligations(obligations: &[DecisionObligation]) -> Vec<DecisionObligation> {
    let mut normalized = obligations
        .iter()
        .map(normalize_decision_obligation)
        .collect::<Vec<_>>();
    normalized.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    normalized
}

fn validate_proof_matrix(rows: &[ProofMatrixRow]) -> Result<()> {
    let mut claim_refs = BTreeSet::new();
    for row in rows {
        if row.claim_ref.is_empty() {
            bail!("proof matrix row claim_ref must not be empty");
        }
        if row.statement.is_empty() {
            bail!(
                "proof matrix row {} statement must not be empty",
                row.claim_ref
            );
        }
        if row.required_evidence.is_empty() {
            bail!(
                "proof matrix row {} required_evidence must not be empty",
                row.claim_ref
            );
        }
        if row.review_lenses.is_empty() {
            bail!(
                "proof matrix row {} review_lenses must not be empty",
                row.claim_ref
            );
        }
        if !claim_refs.insert(row.claim_ref.clone()) {
            bail!("proof matrix row {} is duplicated", row.claim_ref);
        }
    }
    Ok(())
}

fn validate_decision_obligations(obligations: &[DecisionObligation]) -> Result<()> {
    let mut obligation_ids = BTreeSet::new();
    for obligation in obligations {
        if obligation.obligation_id.is_empty() {
            bail!("decision obligation id must not be empty");
        }
        if !obligation_ids.insert(obligation.obligation_id.clone()) {
            bail!(
                "decision obligation {} is duplicated",
                obligation.obligation_id
            );
        }
        if obligation.question.is_empty() {
            bail!(
                "decision obligation {} question must not be empty",
                obligation.obligation_id
            );
        }
        if obligation.why_it_matters.is_empty() {
            bail!(
                "decision obligation {} why_it_matters must not be empty",
                obligation.obligation_id
            );
        }
        if obligation.affects.is_empty() {
            bail!(
                "decision obligation {} affects must not be empty",
                obligation.obligation_id
            );
        }
        if obligation.candidate_route_count < 1 {
            bail!(
                "decision obligation {} candidate_route_count must be at least 1",
                obligation.obligation_id
            );
        }
        if obligation.required_evidence.is_empty() {
            bail!(
                "decision obligation {} required_evidence must not be empty",
                obligation.obligation_id
            );
        }
        if obligation.affects.contains(&DecisionAffect::ReviewContract)
            && obligation.review_contract_refs.is_empty()
        {
            bail!(
                "decision obligation {} review_contract_refs must not be empty when affects includes review_contract",
                obligation.obligation_id
            );
        }
        if obligation.status != DecisionStatus::Open && obligation.resolution_rationale.is_none() {
            bail!(
                "decision obligation {} resolution_rationale is required when status is not open",
                obligation.obligation_id
            );
        }
        if obligation.status == DecisionStatus::ProofGatedSpike
            && (obligation.proof_spike_scope.is_none()
                || obligation.proof_spike_success_criteria.is_empty()
                || obligation.proof_spike_failure_criteria.is_empty()
                || obligation.proof_spike_discharge_artifacts.is_empty()
                || obligation.proof_spike_failure_route.is_none())
        {
            bail!(
                "decision obligation {} proof_gated_spike details are incomplete",
                obligation.obligation_id
            );
        }
    }
    Ok(())
}

fn decision_obligation_blocks_planning_completion(obligation: &DecisionObligation) -> bool {
    matches!(
        obligation.status,
        DecisionStatus::Open | DecisionStatus::Researched
    ) && matches!(
        obligation.blockingness,
        DecisionBlockingness::Critical | DecisionBlockingness::Major
    )
}

fn normalize_markdown_heading(heading: &str) -> String {
    heading
        .trim()
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '.' || ch == ' ')
        .to_ascii_lowercase()
}

fn markdown_level_two_sections(body: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current = None::<String>;
    let mut lines = Vec::new();

    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some(current) = current.replace(normalize_markdown_heading(heading)) {
                sections.insert(current, lines.join("\n").trim().to_string());
                lines.clear();
            }
            continue;
        }

        if current.is_some() {
            lines.push(line);
        }
    }

    if let Some(current) = current {
        sections.insert(current, lines.join("\n").trim().to_string());
    }

    sections
}

fn markdown_section_list_items(body: &str, heading: &str) -> Vec<String> {
    markdown_level_two_sections(body)
        .get(&normalize_markdown_heading(heading))
        .map(|section| {
            section
                .lines()
                .filter_map(|line| {
                    line.trim()
                        .strip_prefix("- ")
                        .map(|item| item.trim().to_string())
                })
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn markdown_section_table_rows(body: &str, heading: &str) -> Vec<Vec<String>> {
    markdown_level_two_sections(body)
        .get(&normalize_markdown_heading(heading))
        .map(|section| {
            section
                .lines()
                .filter_map(parse_markdown_table_row)
                .filter(|row| !markdown_table_row_is_separator(row))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_markdown_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return None;
    }
    Some(
        trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect(),
    )
}

fn markdown_table_row_is_separator(row: &[String]) -> bool {
    !row.is_empty()
        && row
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

fn is_placeholder_artifact_text(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.is_empty()
        || normalized == "tbd"
        || normalized == "todo"
        || normalized == "n/a"
        || normalized == "fill in during planning"
        || normalized == "tbd during clarify"
        || normalized.starts_with("{{")
        || normalized.ends_with("}}")
}

fn markdown_table_row_matches_header(row: &[String], expected: &[&str]) -> bool {
    row.len() == expected.len()
        && row
            .iter()
            .zip(expected.iter())
            .all(|(cell, expected)| cell.eq_ignore_ascii_case(expected))
}

fn blueprint_section_has_meaningful_entries(body: &str, heading: &str) -> bool {
    if markdown_section_list_items(body, heading)
        .into_iter()
        .any(|item| !is_placeholder_artifact_text(&item))
    {
        return true;
    }

    match heading {
        "In-Scope Work Inventory" => {
            markdown_section_table_rows(body, heading)
                .into_iter()
                .any(|row| {
                    !markdown_table_row_matches_header(
                        &row,
                        &[
                            "Work item",
                            "Class",
                            "Why it exists",
                            "Finish in this mission?",
                        ],
                    ) && row
                        .first()
                        .is_some_and(|cell| !is_placeholder_artifact_text(cell))
                })
        }
        "Decision Log" => markdown_section_table_rows(body, heading)
            .into_iter()
            .any(|row| {
                !markdown_table_row_matches_header(
                    &row,
                    &[
                        "Decision id",
                        "Statement",
                        "Rationale",
                        "Evidence refs",
                        "Adopted in revision",
                    ],
                ) && row
                    .get(1)
                    .is_some_and(|cell| !is_placeholder_artifact_text(cell))
                    && row
                        .get(2)
                        .is_some_and(|cell| !is_placeholder_artifact_text(cell))
            }),
        _ => false,
    }
}

fn invalid_scope_paths(paths: &[String]) -> Vec<String> {
    let mut invalid = paths
        .iter()
        .map(|path| path.trim())
        .filter(|path| {
            !path.is_empty()
                && (is_placeholder_artifact_text(path) || normalize_scoped_path(path).is_empty())
        })
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    invalid.sort();
    invalid.dedup();
    invalid
}

fn markdown_section_labeled_values(body: &str, heading: &str, label: &str) -> Vec<String> {
    markdown_section_list_items(body, heading)
        .into_iter()
        .flat_map(|item| {
            item.strip_prefix(label)
                .map(|value| {
                    value
                        .trim()
                        .trim_start_matches(':')
                        .split(',')
                        .map(|entry| entry.trim().to_string())
                        .filter(|entry| !entry.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect()
}

fn missing_markdown_sections(body: &str, required: &[&str]) -> Vec<String> {
    let sections = markdown_level_two_sections(body);
    required
        .iter()
        .filter(|heading| !sections.contains_key(&normalize_markdown_heading(heading)))
        .map(|heading| (*heading).to_string())
        .collect()
}

fn required_blueprint_sections(
    require_execution_graph_and_safe_wave_rules: bool,
) -> Vec<&'static str> {
    let mut sections = vec![
        "Locked Mission Reference",
        "Truth Register Summary",
        "System Model",
        "Invariants And Protected Behaviors",
        "Proof Matrix",
        "Decision Obligations",
        "In-Scope Work Inventory",
        "Selected Architecture",
        "Review Bundle Design",
        "Workstream Overview",
        "Risks And Unknowns",
        "Decision Log",
        "Replan Policy",
    ];
    if require_execution_graph_and_safe_wave_rules {
        sections.push("Execution Graph and Safe-Wave Rules");
    }
    sections
}

fn validate_blueprint_body_contract(
    body: &str,
    status: BlueprintStatus,
    require_execution_graph_and_safe_wave_rules: bool,
) -> Result<()> {
    if status != BlueprintStatus::Approved {
        return Ok(());
    }

    let required_sections =
        required_blueprint_sections(require_execution_graph_and_safe_wave_rules);
    let missing = missing_markdown_sections(body, &required_sections);
    if !missing.is_empty() {
        bail!(
            "PROGRAM-BLUEPRINT.md is missing required canonical sections: {}",
            missing.join(", ")
        );
    }
    for heading in ["In-Scope Work Inventory", "Decision Log"] {
        if !blueprint_section_has_meaningful_entries(body, heading) {
            bail!(
                "PROGRAM-BLUEPRINT.md section {heading} must contain at least one meaningful planning entry"
            );
        }
    }
    Ok(())
}

fn validate_spec_body_contract(spec_id: &str, body: &str, purpose: &str) -> Result<()> {
    let missing = missing_markdown_sections(
        body,
        &[
            "Purpose",
            "In Scope",
            "Out Of Scope",
            "Dependencies",
            "Touched Surfaces",
            "Read Scope",
            "Write Scope",
            "Interfaces And Contracts Touched",
            "Implementation Shape",
            "Proof-Of-Completion Expectations",
            "Non-Breakage Expectations",
            "Review Lenses",
            "Replan Boundary",
            "Truth Basis Refs",
            "Freshness Notes",
            "Support Files",
        ],
    );
    if !missing.is_empty() {
        bail!(
            "SPEC.md for {} is missing required canonical sections: {}",
            spec_id,
            missing.join(", ")
        );
    }
    if !body.contains(purpose) {
        bail!(
            "SPEC.md for {} must include its declared purpose in the visible artifact body",
            spec_id
        );
    }
    for (heading, paths) in [
        (
            "Read Scope",
            markdown_section_list_items(body, "Read Scope"),
        ),
        (
            "Write Scope",
            markdown_section_list_items(body, "Write Scope"),
        ),
    ] {
        if paths.is_empty() {
            bail!(
                "SPEC.md for {} section {heading} must contain at least one path",
                spec_id
            );
        }
        let invalid = invalid_scope_paths(&paths);
        if !invalid.is_empty() {
            bail!(
                "SPEC.md for {} section {heading} contains invalid scoped path entries: {}",
                spec_id,
                invalid.join(", ")
            );
        }
    }
    Ok(())
}

fn compute_planning_risk_floor(
    lock_body: &str,
    problem_size: Option<ProblemSize>,
    runnable_spec_count: usize,
    selected_target_ref: Option<&str>,
) -> u8 {
    let lock_body = lock_body.to_ascii_lowercase();
    let mut floor = match problem_size {
        Some(ProblemSize::XL) => 5,
        Some(ProblemSize::L) => 4,
        Some(ProblemSize::M) => 2,
        Some(ProblemSize::S) | None => 1,
    };
    if runnable_spec_count > 1
        || selected_target_ref.is_some_and(|target| target.starts_with("wave:"))
    {
        floor = floor.max(3);
    }
    if [
        "migration",
        "rollout",
        "rollback",
        "public interface",
        "public api",
        "public contract",
        "external contract",
        "compatibility",
        "backward compatibility",
        "ops",
        "operational",
        "blast radius",
        "protected surface",
        "cross-surface",
        "cross surface",
        "shared surface",
    ]
    .iter()
    .any(|signal| lock_body.contains(signal))
    {
        floor = floor.max(4);
    }
    if [
        "mission-critical",
        "mission critical",
        "rollback-limited",
        "no rollback",
        "irreversible",
        "billing",
        "authentication",
        "authorization",
        "security",
        "data migration",
        "schema migration",
        "database migration",
        "rollback difficulty",
        "rollback difficult",
        "hard to rollback",
        "cannot rollback",
        "protected behavior",
        "sensitive data",
        "payments",
        "login",
        "identity",
        "permission",
        "cross-service",
        "cross service",
    ]
    .iter()
    .any(|signal| lock_body.contains(signal))
    {
        floor = floor.max(5);
    }
    floor
}

fn derive_package_replan_boundary(
    spec_contexts: &[LoadedSpecContext],
    provided: Option<&ReplanBoundary>,
) -> (Option<ReplanBoundary>, Vec<String>) {
    let mut derived = None;
    let mut findings = Vec::new();

    for context in spec_contexts {
        let Some(boundary) = context.replan_boundary.as_ref() else {
            findings.push(format!(
                "spec_replan_boundary_missing:{}",
                context.included.spec_id
            ));
            continue;
        };
        match derived.as_ref() {
            None => derived = Some(boundary.clone()),
            Some(existing) if existing != boundary => findings.push(format!(
                "spec_replan_boundary_mismatch:{}",
                context.included.spec_id
            )),
            Some(_) => {}
        }
    }

    match (derived.as_ref(), provided) {
        (Some(expected), Some(actual)) if expected != actual => {
            findings.push("package_replan_boundary_mismatch_with_specs".to_string());
        }
        (None, Some(_)) => {
            findings.push("package_replan_boundary_without_spec_contract".to_string());
        }
        _ => {}
    }

    (provided.cloned().or(derived), unique_strings(&findings))
}

fn execution_graph_required(active_spec_count: usize, selected_target_ref: Option<&str>) -> bool {
    active_spec_count > 1
        || selected_target_ref.is_some_and(|target_ref| target_ref.starts_with("wave:"))
}

fn normalize_execution_graph_node_input(node: &ExecutionGraphNodeInput) -> ExecutionGraphNodeInput {
    let mut normalized = node.clone();
    normalized.depends_on = unique_strings(&normalized.depends_on);
    normalized.produces = unique_strings(&normalized.produces);
    normalized.read_paths = unique_strings(&normalized.read_paths);
    normalized.write_paths = unique_strings(&normalized.write_paths);
    normalized.exclusive_resources = unique_strings(&normalized.exclusive_resources);
    normalized.coupling_tags = unique_strings(&normalized.coupling_tags);
    normalized.ownership_domains = unique_strings(&normalized.ownership_domains);
    normalized.acceptance_checks = unique_strings(&normalized.acceptance_checks);
    normalized.evidence_type = normalized.evidence_type.trim().to_string();
    normalized
}

fn normalize_execution_graph_obligation_input(
    obligation: &ExecutionGraphObligationInput,
) -> ExecutionGraphObligationInput {
    let mut normalized = obligation.clone();
    normalized.proof_rows = unique_strings(&normalized.proof_rows);
    normalized.acceptance_checks = unique_strings(&normalized.acceptance_checks);
    normalized.required_evidence = unique_strings(&normalized.required_evidence);
    normalized.review_lenses = unique_strings(&normalized.review_lenses);
    normalized.satisfied_by = unique_strings(&normalized.satisfied_by);
    normalized.evidence_refs = unique_strings(&normalized.evidence_refs);
    normalized.discharges_claim_ref = normalized.discharges_claim_ref.trim().to_string();
    normalized
}

fn normalize_execution_graph_input(input: &ExecutionGraphInput) -> ExecutionGraphInput {
    let mut normalized = ExecutionGraphInput {
        nodes: input
            .nodes
            .iter()
            .map(normalize_execution_graph_node_input)
            .collect(),
        obligations: input
            .obligations
            .iter()
            .map(normalize_execution_graph_obligation_input)
            .collect(),
    };
    normalized
        .nodes
        .sort_by(|left, right| left.spec_id.cmp(&right.spec_id));
    normalized
        .obligations
        .sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    normalized
}

fn execution_graph_to_contract_input(graph: &ExecutionGraph) -> ExecutionGraphInput {
    normalize_execution_graph_input(&ExecutionGraphInput {
        nodes: graph
            .nodes
            .iter()
            .map(|node| ExecutionGraphNodeInput {
                spec_id: node.spec_id.clone(),
                depends_on: node.depends_on.clone(),
                produces: node.produces.clone(),
                read_paths: node.read_paths.clone(),
                write_paths: node.write_paths.clone(),
                exclusive_resources: node.exclusive_resources.clone(),
                coupling_tags: node.coupling_tags.clone(),
                ownership_domains: node.ownership_domains.clone(),
                risk_class: node.risk_class.clone(),
                acceptance_checks: node.acceptance_checks.clone(),
                evidence_type: node.evidence_type.clone(),
            })
            .collect(),
        obligations: graph
            .obligations
            .iter()
            .map(|obligation| ExecutionGraphObligationInput {
                obligation_id: obligation.obligation_id.clone(),
                kind: obligation.kind.clone(),
                target_spec_id: obligation.target_spec_id.clone(),
                discharges_claim_ref: obligation.discharges_claim_ref.clone(),
                proof_rows: obligation.proof_rows.clone(),
                acceptance_checks: obligation.acceptance_checks.clone(),
                required_evidence: obligation.required_evidence.clone(),
                review_lenses: obligation.review_lenses.clone(),
                blocking: obligation.blocking,
                status: ExecutionGraphObligationStatus::Open,
                satisfied_by: Vec::new(),
                evidence_refs: Vec::new(),
            })
            .collect(),
    })
}

fn compute_blueprint_contract_fingerprint(
    blueprint_doc: &ArtifactDocument<ProgramBlueprintFrontmatter>,
    execution_graph: Option<&ExecutionGraphInput>,
) -> Result<Fingerprint> {
    if execution_graph.is_none() {
        return Ok(blueprint_doc.fingerprint()?);
    }

    #[derive(Serialize)]
    struct BlueprintContract<'a> {
        blueprint_markdown: String,
        execution_graph: Option<&'a ExecutionGraphInput>,
    }

    let contract = BlueprintContract {
        blueprint_markdown: blueprint_doc.render()?,
        execution_graph,
    };
    Ok(Fingerprint::from_json(&contract)?)
}

fn load_execution_graph(paths: &MissionPaths) -> Result<Option<ExecutionGraph>> {
    if !paths.execution_graph().is_file() {
        return Ok(None);
    }
    load_json(paths.execution_graph()).map(Some)
}

fn current_blueprint_contract_fingerprint(
    paths: &MissionPaths,
    blueprint_doc: &ArtifactDocument<ProgramBlueprintFrontmatter>,
) -> Result<Fingerprint> {
    let execution_graph = load_execution_graph(paths)?;
    let execution_graph = execution_graph
        .as_ref()
        .filter(|graph| graph.blueprint_revision == blueprint_doc.frontmatter.blueprint_revision)
        .map(execution_graph_to_contract_input);
    compute_blueprint_contract_fingerprint(blueprint_doc, execution_graph.as_ref())
}

fn build_execution_graph(
    paths: &MissionPaths,
    blueprint_revision: u64,
    blueprint_fingerprint: &Fingerprint,
    input: &ExecutionGraphInput,
    active_spec_ids: &[String],
) -> Result<ExecutionGraph> {
    let normalized = normalize_execution_graph_input(input);
    let spec_contexts = load_spec_contexts(paths, active_spec_ids)?;
    let spec_by_id = spec_contexts
        .iter()
        .map(|context| {
            let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(
                &paths.spec_file(&context.included.spec_id),
            )?;
            Ok((
                context.included.spec_id.clone(),
                (
                    context.included.clone(),
                    execution_graph_spec_contract_fingerprint(&spec_doc)?,
                ),
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    let nodes = normalized
        .nodes
        .iter()
        .map(|node| {
            let (included, spec_contract_fingerprint) =
                spec_by_id.get(&node.spec_id).with_context(|| {
                    format!("execution graph references unknown spec {}", node.spec_id)
                })?;
            Ok(ExecutionGraphNode {
                spec_id: node.spec_id.clone(),
                spec_revision: included.spec_revision,
                spec_fingerprint: spec_contract_fingerprint.clone(),
                depends_on: node.depends_on.clone(),
                produces: node.produces.clone(),
                read_paths: node.read_paths.clone(),
                write_paths: node.write_paths.clone(),
                exclusive_resources: node.exclusive_resources.clone(),
                coupling_tags: node.coupling_tags.clone(),
                ownership_domains: node.ownership_domains.clone(),
                risk_class: node.risk_class.clone(),
                acceptance_checks: node.acceptance_checks.clone(),
                evidence_type: node.evidence_type.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let obligations = normalized
        .obligations
        .iter()
        .map(|obligation| {
            let (included, spec_contract_fingerprint) = spec_by_id
                .get(&obligation.target_spec_id)
                .with_context(|| {
                    format!(
                        "execution graph obligation {} references unknown spec {}",
                        obligation.obligation_id, obligation.target_spec_id
                    )
                })?;
            Ok(ExecutionGraphObligation {
                obligation_id: obligation.obligation_id.clone(),
                kind: obligation.kind.clone(),
                target_spec_id: obligation.target_spec_id.clone(),
                target_spec_revision: included.spec_revision,
                target_spec_fingerprint: spec_contract_fingerprint.clone(),
                discharges_claim_ref: obligation.discharges_claim_ref.clone(),
                proof_rows: obligation.proof_rows.clone(),
                acceptance_checks: obligation.acceptance_checks.clone(),
                required_evidence: obligation.required_evidence.clone(),
                review_lenses: obligation.review_lenses.clone(),
                blocking: obligation.blocking,
                status: obligation.status.clone(),
                satisfied_by: obligation.satisfied_by.clone(),
                evidence_refs: obligation.evidence_refs.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let graph = ExecutionGraph {
        mission_id: paths.mission_id().to_string(),
        blueprint_revision,
        blueprint_fingerprint: blueprint_fingerprint.clone(),
        nodes,
        obligations,
        generated_at: OffsetDateTime::now_utc(),
    };
    let report = validate_execution_graph_for_blueprint(
        paths,
        blueprint_revision,
        blueprint_fingerprint,
        Some(&graph),
        active_spec_ids,
        None,
    )?;
    if !report.valid {
        bail!(
            "execution graph is invalid for mission {}: {}",
            paths.mission_id(),
            report.findings.join(", ")
        );
    }
    Ok(graph)
}

fn validate_execution_graph_for_blueprint(
    paths: &MissionPaths,
    blueprint_revision: u64,
    blueprint_fingerprint: &Fingerprint,
    graph: Option<&ExecutionGraph>,
    active_spec_ids: &[String],
    selected_target_ref: Option<&str>,
) -> Result<ExecutionGraphValidationReport> {
    let mut findings = Vec::new();
    let graph_required = execution_graph_required(active_spec_ids.len(), selected_target_ref);
    let active_spec_ids = unique_strings(active_spec_ids);

    let Some(graph) = graph else {
        if graph_required {
            findings.push("execution_graph_missing".to_string());
        }
        return Ok(ExecutionGraphValidationReport {
            mission_id: paths.mission_id().to_string(),
            blueprint_revision,
            valid: findings.is_empty(),
            findings,
        });
    };

    if graph.mission_id != paths.mission_id() {
        findings.push("execution_graph_mission_mismatch".to_string());
    }
    if graph.blueprint_revision != blueprint_revision {
        findings.push("execution_graph_blueprint_revision_mismatch".to_string());
    }
    if &graph.blueprint_fingerprint != blueprint_fingerprint {
        findings.push("execution_graph_blueprint_fingerprint_mismatch".to_string());
    }
    if graph.nodes.is_empty() {
        findings.push("execution_graph_nodes_missing".to_string());
    }

    let mut node_ids = BTreeSet::new();
    let mut node_by_id = BTreeMap::new();
    for node in &graph.nodes {
        if !node_ids.insert(node.spec_id.clone()) {
            findings.push(format!("execution_graph_duplicate_node:{}", node.spec_id));
            continue;
        }
        if !active_spec_ids
            .iter()
            .any(|spec_id| spec_id == &node.spec_id)
        {
            findings.push(format!(
                "execution_graph_node_not_active_in_blueprint:{}",
                node.spec_id
            ));
        }
        if node.evidence_type.trim().is_empty() {
            findings.push(format!(
                "execution_graph_node_evidence_type_missing:{}",
                node.spec_id
            ));
        }
        if node.acceptance_checks.is_empty() {
            findings.push(format!(
                "execution_graph_node_acceptance_checks_missing:{}",
                node.spec_id
            ));
        }
        if node.read_paths.is_empty() && node.write_paths.is_empty() {
            findings.push(format!(
                "execution_graph_node_scope_missing:{}",
                node.spec_id
            ));
        }
        for path in invalid_scope_paths(&node.read_paths) {
            findings.push(format!(
                "execution_graph_node_invalid_read_scope_path:{}:{}",
                node.spec_id, path
            ));
        }
        for path in invalid_scope_paths(&node.write_paths) {
            findings.push(format!(
                "execution_graph_node_invalid_write_scope_path:{}:{}",
                node.spec_id, path
            ));
        }
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&node.spec_id))?;
        let declared_scope = spec_declared_path_scope(&spec_doc);
        for read_path in scope_paths_outside_frontier(&node.read_paths, &declared_scope.read_paths)
        {
            findings.push(format!(
                "execution_graph_node_read_scope_outside_spec:{}:{}",
                node.spec_id, read_path
            ));
        }
        for write_path in
            scope_paths_outside_frontier(&node.write_paths, &declared_scope.write_paths)
        {
            findings.push(format!(
                "execution_graph_node_write_scope_outside_spec:{}:{}",
                node.spec_id, write_path
            ));
        }
        if spec_doc.frontmatter.spec_revision != node.spec_revision {
            findings.push(format!(
                "execution_graph_node_spec_revision_mismatch:{}",
                node.spec_id
            ));
        }
        if execution_graph_spec_contract_fingerprint(&spec_doc)? != node.spec_fingerprint {
            findings.push(format!(
                "execution_graph_node_spec_fingerprint_mismatch:{}",
                node.spec_id
            ));
        }
        for dependency in &node.depends_on {
            if dependency == &node.spec_id {
                findings.push(format!("execution_graph_self_dependency:{}", node.spec_id));
            }
        }
        node_by_id.insert(node.spec_id.clone(), node);
    }

    for spec_id in &active_spec_ids {
        if !node_by_id.contains_key(spec_id) {
            findings.push(format!("execution_graph_missing_active_node:{spec_id}"));
        }
    }

    for node in &graph.nodes {
        for dependency in &node.depends_on {
            if !node_by_id.contains_key(dependency) {
                findings.push(format!(
                    "execution_graph_unknown_dependency:{}:{}",
                    node.spec_id, dependency
                ));
            }
        }
    }

    fn dfs_cycle<'a>(
        node_id: &'a str,
        node_by_id: &BTreeMap<String, &'a ExecutionGraphNode>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
        findings: &mut Vec<String>,
    ) {
        if visited.contains(node_id) {
            return;
        }
        if !visiting.insert(node_id.to_string()) {
            findings.push(format!("execution_graph_cycle_detected:{node_id}"));
            return;
        }
        if let Some(node) = node_by_id.get(node_id) {
            for dependency in &node.depends_on {
                if node_by_id.contains_key(dependency) {
                    dfs_cycle(dependency, node_by_id, visiting, visited, findings);
                }
            }
        }
        visiting.remove(node_id);
        visited.insert(node_id.to_string());
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for spec_id in node_by_id.keys() {
        dfs_cycle(
            spec_id,
            &node_by_id,
            &mut visiting,
            &mut visited,
            &mut findings,
        );
    }

    let mut obligation_ids = BTreeSet::new();
    for obligation in &graph.obligations {
        if !obligation_ids.insert(obligation.obligation_id.clone()) {
            findings.push(format!(
                "execution_graph_duplicate_obligation:{}",
                obligation.obligation_id
            ));
        }
        let Some(node) = node_by_id.get(&obligation.target_spec_id) else {
            findings.push(format!(
                "execution_graph_obligation_unknown_target:{}:{}",
                obligation.obligation_id, obligation.target_spec_id
            ));
            continue;
        };
        if node.spec_revision != obligation.target_spec_revision {
            findings.push(format!(
                "execution_graph_obligation_spec_revision_mismatch:{}",
                obligation.obligation_id
            ));
        }
        if node.spec_fingerprint != obligation.target_spec_fingerprint {
            findings.push(format!(
                "execution_graph_obligation_spec_fingerprint_mismatch:{}",
                obligation.obligation_id
            ));
        }
        if obligation.discharges_claim_ref.trim().is_empty() {
            findings.push(format!(
                "execution_graph_obligation_claim_missing:{}",
                obligation.obligation_id
            ));
        }
        if obligation.proof_rows.is_empty() {
            findings.push(format!(
                "execution_graph_obligation_proof_rows_missing:{}",
                obligation.obligation_id
            ));
        }
        if obligation.acceptance_checks.is_empty() {
            findings.push(format!(
                "execution_graph_obligation_acceptance_checks_missing:{}",
                obligation.obligation_id
            ));
        }
        if obligation.required_evidence.is_empty() {
            findings.push(format!(
                "execution_graph_obligation_required_evidence_missing:{}",
                obligation.obligation_id
            ));
        }
        if obligation.review_lenses.is_empty() {
            findings.push(format!(
                "execution_graph_obligation_review_lenses_missing:{}",
                obligation.obligation_id
            ));
        }
        for acceptance_check in &obligation.acceptance_checks {
            if !node
                .acceptance_checks
                .iter()
                .any(|check| check == acceptance_check)
            {
                findings.push(format!(
                    "execution_graph_obligation_acceptance_check_not_declared:{}:{}",
                    obligation.obligation_id, acceptance_check
                ));
            }
        }
    }

    for node in node_by_id.values() {
        for acceptance_check in &node.acceptance_checks {
            let covered = graph.obligations.iter().any(|obligation| {
                obligation.target_spec_id == node.spec_id
                    && obligation
                        .acceptance_checks
                        .iter()
                        .any(|candidate| candidate == acceptance_check)
            });
            if !covered {
                findings.push(format!(
                    "execution_graph_acceptance_check_unbound:{}:{}",
                    node.spec_id, acceptance_check
                ));
            }
        }
    }

    Ok(ExecutionGraphValidationReport {
        mission_id: paths.mission_id().to_string(),
        blueprint_revision,
        valid: findings.is_empty(),
        findings: unique_strings(&findings),
    })
}

pub fn validate_execution_graph(paths: &MissionPaths) -> Result<ExecutionGraphValidationReport> {
    let blueprint_doc = load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())?;
    let blueprint_fingerprint = current_blueprint_contract_fingerprint(paths, &blueprint_doc)?;
    let active_spec_ids =
        load_runnable_blueprint_spec_ids(paths, blueprint_doc.frontmatter.blueprint_revision)?;
    validate_execution_graph_for_blueprint(
        paths,
        blueprint_doc.frontmatter.blueprint_revision,
        &blueprint_fingerprint,
        load_execution_graph(paths)?.as_ref(),
        &active_spec_ids,
        blueprint_doc.frontmatter.selected_target_ref.as_deref(),
    )
}

fn normalize_wave_specs(wave_specs: &[WaveSpecInput]) -> Vec<WaveSpecInput> {
    let mut normalized = wave_specs
        .iter()
        .cloned()
        .map(|mut spec| {
            spec.read_paths = unique_strings(&spec.read_paths);
            spec.write_paths = unique_strings(&spec.write_paths);
            spec.exclusive_resources = unique_strings(&spec.exclusive_resources);
            spec.coupling_tags = unique_strings(&spec.coupling_tags);
            spec.ownership_domains = unique_strings(&spec.ownership_domains);
            spec
        })
        .collect::<Vec<_>>();
    normalized.sort_by(|left, right| left.spec_id.cmp(&right.spec_id));
    normalized
}

fn derive_wave_specs_from_execution_graph(
    graph: &ExecutionGraph,
    included_spec_ids: &[String],
) -> Vec<WaveSpecInput> {
    let included = unique_strings(included_spec_ids);
    normalize_wave_specs(
        &graph
            .nodes
            .iter()
            .filter(|node| included.iter().any(|spec_id| spec_id == &node.spec_id))
            .map(|node| WaveSpecInput {
                spec_id: node.spec_id.clone(),
                read_paths: node.read_paths.clone(),
                write_paths: node.write_paths.clone(),
                exclusive_resources: node.exclusive_resources.clone(),
                coupling_tags: node.coupling_tags.clone(),
                ownership_domains: node.ownership_domains.clone(),
                risk_class: node.risk_class.clone(),
            })
            .collect::<Vec<_>>(),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PathScope {
    read_paths: Vec<String>,
    write_paths: Vec<String>,
}

fn normalize_scope_paths(paths: &[String]) -> Vec<String> {
    let mut normalized = paths
        .iter()
        .map(|path| normalize_scoped_path(path))
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn scope_paths_within_frontier(scope: &[String], frontier: &[String]) -> Vec<String> {
    let normalized_frontier = normalize_scope_paths(frontier);
    normalize_scope_paths(scope)
        .into_iter()
        .filter(|path| {
            normalized_frontier
                .iter()
                .any(|allowed| scoped_path_within(path, allowed))
        })
        .collect()
}

fn scope_paths_outside_frontier(scope: &[String], frontier: &[String]) -> Vec<String> {
    let normalized_frontier = normalize_scope_paths(frontier);
    normalize_scope_paths(scope)
        .into_iter()
        .filter(|path| {
            !normalized_frontier
                .iter()
                .any(|allowed| scoped_path_within(path, allowed))
        })
        .collect()
}

fn scoped_path_within(candidate: &str, frontier: &str) -> bool {
    let candidate = normalize_scoped_path(candidate);
    let frontier = normalize_scoped_path(frontier);
    candidate == frontier
        || candidate
            .strip_prefix(&frontier)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn union_path_scopes(scopes: &[PathScope]) -> PathScope {
    PathScope {
        read_paths: normalize_scope_paths(
            &scopes
                .iter()
                .flat_map(|scope| scope.read_paths.clone())
                .collect::<Vec<_>>(),
        ),
        write_paths: normalize_scope_paths(
            &scopes
                .iter()
                .flat_map(|scope| scope.write_paths.clone())
                .collect::<Vec<_>>(),
        ),
    }
}

fn spec_declared_path_scope(spec_doc: &ArtifactDocument<WorkstreamSpecFrontmatter>) -> PathScope {
    PathScope {
        read_paths: normalize_scope_paths(&markdown_section_list_items(
            &spec_doc.body,
            "Read Scope",
        )),
        write_paths: normalize_scope_paths(&markdown_section_list_items(
            &spec_doc.body,
            "Write Scope",
        )),
    }
}

fn execution_graph_node_path_scope(node: &ExecutionGraphNode) -> PathScope {
    PathScope {
        read_paths: normalize_scope_paths(&node.read_paths),
        write_paths: normalize_scope_paths(&node.write_paths),
    }
}

fn effective_spec_path_scope(
    spec_doc: &ArtifactDocument<WorkstreamSpecFrontmatter>,
    node: Option<&ExecutionGraphNode>,
) -> PathScope {
    node.map_or_else(
        || spec_declared_path_scope(spec_doc),
        execution_graph_node_path_scope,
    )
}

fn derive_writer_packet_scope(
    paths: &MissionPaths,
    package: &ExecutionPackage,
    target_spec_id: &str,
) -> Result<PathScope> {
    let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(target_spec_id))?;
    let execution_graph = load_execution_graph(paths)?;
    let frontier = effective_spec_path_scope(
        &spec_doc,
        execution_graph.as_ref().and_then(|graph| {
            graph
                .nodes
                .iter()
                .find(|node| node.spec_id == target_spec_id)
        }),
    );
    Ok(PathScope {
        read_paths: scope_paths_within_frontier(&package.read_scope, &frontier.read_paths),
        write_paths: scope_paths_within_frontier(&package.write_scope, &frontier.write_paths),
    })
}

fn normalize_scoped_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed.contains('\\') || trimmed.starts_with('/') {
        return String::new();
    }

    let without_edges = trimmed.trim_matches('/');
    if without_edges.is_empty() {
        return String::new();
    }

    let mut normalized = Vec::new();
    for segment in without_edges.split('/') {
        let segment = segment.trim();
        if segment.is_empty() || segment == "." || segment == ".." || segment.ends_with(':') {
            return String::new();
        }
        normalized.push(segment.to_ascii_lowercase());
    }

    normalized.join("/")
}

fn scoped_paths_overlap(left: &str, right: &str) -> bool {
    let left = normalize_scoped_path(left);
    let right = normalize_scoped_path(right);
    left == right
        || left
            .strip_prefix(&right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(&left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_singleton_wave_path(path: &str) -> bool {
    let normalized = normalize_scoped_path(path);
    let basename = normalized.rsplit('/').next().unwrap_or(normalized.as_str());
    basename.ends_with(".lock")
        || matches!(
            basename,
            "cargo.lock"
                | "package-lock.json"
                | "pnpm-lock.yaml"
                | "yarn.lock"
                | "poetry.lock"
                | "bun.lockb"
                | "schema.prisma"
                | "docker-compose.yml"
                | "docker-compose.yaml"
        )
        || normalized.contains("/migrations/")
        || normalized.contains("/deploy/")
        || normalized.contains("/schema/")
}

fn dependency_check_satisfies_spec(
    dependency_satisfaction_state: &[DependencyCheck],
    spec_id: &str,
) -> bool {
    dependency_satisfaction_state.iter().any(|dependency| {
        dependency.satisfied
            && (dependency.name == spec_id || dependency.name == format!("spec:{spec_id}"))
    })
}

fn validate_wave_safe_parallelism(
    included_spec_ids: &[String],
    wave_specs: &[WaveSpecInput],
) -> Vec<String> {
    let normalized = normalize_wave_specs(wave_specs);
    let included = unique_strings(included_spec_ids);
    let mut findings = Vec::new();

    if normalized.is_empty() {
        findings.push("wave_specs_missing".to_string());
        return findings;
    }

    let wave_ids = normalized
        .iter()
        .map(|spec| spec.spec_id.clone())
        .collect::<Vec<_>>();
    for spec_id in &included {
        if !wave_ids.iter().any(|candidate| candidate == spec_id) {
            findings.push(format!("wave_spec_missing:{spec_id}"));
        }
    }
    for spec_id in &wave_ids {
        if !included.iter().any(|candidate| candidate == spec_id) {
            findings.push(format!("wave_spec_not_included:{spec_id}"));
        }
    }

    for spec in &normalized {
        if spec.read_paths.is_empty() && spec.write_paths.is_empty() {
            findings.push(format!("wave_spec_scope_missing:{}", spec.spec_id));
        }
        if normalized.len() > 1 {
            match spec.risk_class.clone().unwrap_or(WaveRiskClass::Unknown) {
                WaveRiskClass::Normal => {}
                WaveRiskClass::Meta => findings.push(format!(
                    "wave_spec_meta_risk_singleton_required:{}",
                    spec.spec_id
                )),
                WaveRiskClass::Unknown => {
                    findings.push(format!("wave_spec_unknown_risk_class:{}", spec.spec_id));
                }
            }
            if spec.ownership_domains.is_empty() {
                findings.push(format!(
                    "wave_spec_ownership_domains_missing:{}",
                    spec.spec_id
                ));
            }
            for write_path in &spec.write_paths {
                if is_singleton_wave_path(write_path) {
                    findings.push(format!(
                        "wave_singleton_write_path:{}:{}",
                        spec.spec_id, write_path
                    ));
                }
            }
        }
    }

    for (index, left) in normalized.iter().enumerate() {
        for right in normalized.iter().skip(index + 1) {
            for left_write in &left.write_paths {
                for right_write in &right.write_paths {
                    if scoped_paths_overlap(left_write, right_write) {
                        findings.push(format!(
                            "wave_write_path_overlap:{}:{}:{}",
                            left.spec_id, right.spec_id, left_write
                        ));
                    }
                }
                for right_read in &right.read_paths {
                    if scoped_paths_overlap(left_write, right_read) {
                        findings.push(format!(
                            "wave_write_read_overlap:{}:{}:{}",
                            left.spec_id, right.spec_id, left_write
                        ));
                    }
                }
            }
            for right_write in &right.write_paths {
                for left_read in &left.read_paths {
                    if scoped_paths_overlap(right_write, left_read) {
                        findings.push(format!(
                            "wave_write_read_overlap:{}:{}:{}",
                            right.spec_id, left.spec_id, right_write
                        ));
                    }
                }
            }
            for resource in &left.exclusive_resources {
                if right
                    .exclusive_resources
                    .iter()
                    .any(|candidate| candidate == resource)
                {
                    findings.push(format!(
                        "wave_exclusive_resource_overlap:{}:{}:{}",
                        left.spec_id, right.spec_id, resource
                    ));
                }
            }
            for domain in &left.ownership_domains {
                if right
                    .ownership_domains
                    .iter()
                    .any(|candidate| candidate == domain)
                {
                    findings.push(format!(
                        "wave_ownership_domain_overlap:{}:{}:{}",
                        left.spec_id, right.spec_id, domain
                    ));
                }
            }
            for tag in &left.coupling_tags {
                if right.coupling_tags.iter().any(|candidate| candidate == tag) {
                    findings.push(format!(
                        "wave_coupling_tag_overlap:{}:{}:{}",
                        left.spec_id, right.spec_id, tag
                    ));
                }
            }
        }
    }

    unique_strings(&findings)
}

#[allow(clippy::too_many_arguments)]
fn evaluate_execution_package_contract(
    paths: &MissionPaths,
    blueprint_doc: &ArtifactDocument<ProgramBlueprintFrontmatter>,
    target_type: &TargetType,
    target_id: &str,
    included_spec_ids: &[String],
    dependency_satisfaction_state: &[DependencyCheck],
    read_scope: &[String],
    write_scope: &[String],
    proof_obligations: &[String],
    review_obligations: &[String],
    wave_specs: &[WaveSpecInput],
    wave_context: Option<&str>,
    wave_fingerprint: Option<&Fingerprint>,
    gate_checks: Option<&[PackageGateCheck]>,
) -> Result<ExecutionPackageContractEvaluation> {
    let spec_contexts = load_spec_contexts(paths, included_spec_ids)?;
    let current_blueprint_fingerprint =
        current_blueprint_contract_fingerprint(paths, blueprint_doc)?;
    let lock_doc =
        load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())?;
    let included_specs = spec_contexts
        .iter()
        .cloned()
        .map(|context| context.included)
        .collect::<Vec<_>>();
    let selected_target_ref = blueprint_doc.frontmatter.selected_target_ref.as_deref();
    let active_spec_ids =
        load_runnable_blueprint_spec_ids(paths, blueprint_doc.frontmatter.blueprint_revision)?;
    let execution_graph = load_execution_graph(paths)?;
    let target_ref = gate_target_ref(target_type, target_id);
    let mut findings = Vec::new();
    let graph_required = execution_graph_required(active_spec_ids.len(), selected_target_ref);
    let proof_matrix = normalize_proof_matrix(&blueprint_doc.frontmatter.proof_matrix);
    if let Err(error) = validate_proof_matrix(&proof_matrix) {
        findings.push(format!("blueprint_proof_matrix_invalid:{error}"));
    }
    for missing in missing_markdown_sections(
        &blueprint_doc.body,
        &required_blueprint_sections(graph_required),
    ) {
        findings.push(format!("blueprint_body_missing_section:{missing}"));
    }
    for heading in ["In-Scope Work Inventory", "Decision Log"] {
        if !blueprint_section_has_meaningful_entries(&blueprint_doc.body, heading) {
            findings.push(format!("blueprint_body_empty_section:{heading}"));
        }
    }
    let decision_obligations =
        normalize_decision_obligations(&blueprint_doc.frontmatter.decision_obligations);
    if let Err(error) = validate_decision_obligations(&decision_obligations) {
        findings.push(format!("blueprint_decision_obligations_invalid:{error}"));
    }
    let computed_risk_floor = compute_planning_risk_floor(
        &lock_doc.body,
        blueprint_doc.frontmatter.problem_size,
        active_spec_ids.len(),
        selected_target_ref,
    );
    if blueprint_doc.frontmatter.risk_floor != computed_risk_floor {
        findings.push("blueprint_risk_floor_mismatch".to_string());
    }
    if blueprint_doc.frontmatter.plan_level < computed_risk_floor {
        findings.push("blueprint_plan_level_below_risk_floor".to_string());
    }
    if blueprint_doc.frontmatter.status == BlueprintStatus::Approved
        && !active_spec_ids.is_empty()
        && proof_matrix.is_empty()
    {
        findings.push("blueprint_proof_matrix_missing".to_string());
    }
    for obligation in &decision_obligations {
        if decision_obligation_blocks_planning_completion(obligation) {
            findings.push(format!(
                "blocking_decision_obligation_unresolved:{}",
                obligation.obligation_id
            ));
        }
    }
    let graph_validation = validate_execution_graph_for_blueprint(
        paths,
        blueprint_doc.frontmatter.blueprint_revision,
        &current_blueprint_fingerprint,
        execution_graph.as_ref(),
        &active_spec_ids,
        selected_target_ref,
    )?;
    findings.extend(graph_validation.findings);

    let execution_graph_node_by_id = execution_graph
        .as_ref()
        .map(|graph| {
            graph
                .nodes
                .iter()
                .map(|node| (node.spec_id.as_str(), node))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut effective_scopes = Vec::new();
    let mut effective_scope_by_spec_id = BTreeMap::new();

    if included_specs.is_empty() {
        findings.push("included_specs must not be empty".to_string());
    }
    if dependency_satisfaction_state.is_empty() {
        findings.push("dependency_satisfaction_state must not be empty".to_string());
    }
    if read_scope.is_empty() {
        findings.push("read_scope must not be empty".to_string());
    }
    if write_scope.is_empty() {
        findings.push("write_scope must not be empty".to_string());
    }
    if proof_obligations.is_empty() {
        findings.push("proof_obligations must not be empty".to_string());
    }
    if review_obligations.is_empty() {
        findings.push("review_obligations must not be empty".to_string());
    }
    for path in invalid_scope_paths(read_scope) {
        findings.push(format!("package_invalid_read_scope_path:{path}"));
    }
    for path in invalid_scope_paths(write_scope) {
        findings.push(format!("package_invalid_write_scope_path:{path}"));
    }

    match selected_target_ref {
        None => findings.push("planning_selected_target_missing".to_string()),
        Some(selected) if selected != target_ref => findings.push(format!(
            "planning_selected_target_mismatch:{}!=package:{}",
            selected, target_ref
        )),
        Some(_) => {}
    }
    if lock_doc.frontmatter.status != LockStatus::Locked {
        findings.push(format!(
            "outcome_lock_not_locked:{:?}",
            lock_doc.frontmatter.status
        ));
    }
    if blueprint_doc.frontmatter.status != BlueprintStatus::Approved {
        findings.push(format!(
            "blueprint_not_approved:{:?}",
            blueprint_doc.frontmatter.status
        ));
    }

    for dependency in dependency_satisfaction_state {
        if !dependency.satisfied {
            findings.push(format!("dependency_unsatisfied:{}", dependency.name));
        }
        if default_dependency_governing_ref(&dependency.name).is_none() {
            findings.push(format!(
                "dependency_governing_ref_missing:{}",
                dependency.name
            ));
        }
    }

    for context in &spec_contexts {
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(
            &paths.spec_file(&context.included.spec_id),
        )?;
        let declared_read_paths = markdown_section_list_items(&spec_doc.body, "Read Scope");
        let declared_write_paths = markdown_section_list_items(&spec_doc.body, "Write Scope");
        for missing in missing_markdown_sections(
            &spec_doc.body,
            &[
                "Purpose",
                "In Scope",
                "Out Of Scope",
                "Dependencies",
                "Touched Surfaces",
                "Read Scope",
                "Write Scope",
                "Interfaces And Contracts Touched",
                "Implementation Shape",
                "Proof-Of-Completion Expectations",
                "Non-Breakage Expectations",
                "Review Lenses",
                "Replan Boundary",
                "Truth Basis Refs",
                "Freshness Notes",
                "Support Files",
            ],
        ) {
            findings.push(format!(
                "spec_body_missing_section:{}:{}",
                context.included.spec_id, missing
            ));
        }
        if declared_read_paths.is_empty() {
            findings.push(format!(
                "spec_body_empty_section:{}:Read Scope",
                context.included.spec_id
            ));
        }
        if declared_write_paths.is_empty() {
            findings.push(format!(
                "spec_body_empty_section:{}:Write Scope",
                context.included.spec_id
            ));
        }
        for path in invalid_scope_paths(&declared_read_paths) {
            findings.push(format!(
                "spec_invalid_read_scope_path:{}:{}",
                context.included.spec_id, path
            ));
        }
        for path in invalid_scope_paths(&declared_write_paths) {
            findings.push(format!(
                "spec_invalid_write_scope_path:{}:{}",
                context.included.spec_id, path
            ));
        }
        if context.artifact_status != SpecArtifactStatus::Active {
            findings.push(format!(
                "included_spec_not_active:{}:{:?}",
                context.included.spec_id, context.artifact_status
            ));
        }
        if !matches!(
            context.packetization_status,
            PacketizationStatus::Runnable | PacketizationStatus::ProofGatedSpike
        ) {
            findings.push(format!(
                "included_spec_not_executable:{}:{:?}",
                context.included.spec_id, context.packetization_status
            ));
        }
        if context.blueprint_revision != blueprint_doc.frontmatter.blueprint_revision {
            findings.push(format!(
                "included_spec_blueprint_revision_mismatch:{}:{}!=current:{}",
                context.included.spec_id,
                context.blueprint_revision,
                blueprint_doc.frontmatter.blueprint_revision
            ));
        }
        if context.blueprint_fingerprint.as_ref() != Some(&current_blueprint_fingerprint) {
            findings.push(format!(
                "included_spec_blueprint_fingerprint_mismatch:{}",
                context.included.spec_id
            ));
        }
        let effective_scope = effective_spec_path_scope(
            &spec_doc,
            execution_graph_node_by_id
                .get(context.included.spec_id.as_str())
                .copied(),
        );
        effective_scopes.push(effective_scope.clone());
        effective_scope_by_spec_id.insert(context.included.spec_id.clone(), effective_scope);
    }

    let package_scope_frontier = union_path_scopes(&effective_scopes);
    for path in scope_paths_outside_frontier(read_scope, &package_scope_frontier.read_paths) {
        findings.push(format!("package_read_scope_outside_frontier:{path}"));
    }
    for path in scope_paths_outside_frontier(write_scope, &package_scope_frontier.write_paths) {
        findings.push(format!("package_write_scope_outside_frontier:{path}"));
    }
    for included_spec_id in included_spec_ids {
        if let Some(scope) = effective_scope_by_spec_id.get(included_spec_id) {
            for path in scope_paths_outside_frontier(&scope.read_paths, read_scope) {
                findings.push(format!(
                    "package_read_scope_missing_for_included_spec:{included_spec_id}:{path}"
                ));
            }
            for path in scope_paths_outside_frontier(&scope.write_paths, write_scope) {
                findings.push(format!(
                    "package_write_scope_missing_for_included_spec:{included_spec_id}:{path}"
                ));
            }
        }
    }

    if let Some(execution_graph) = execution_graph.as_ref() {
        let valid_claim_refs = proof_matrix
            .iter()
            .map(|row| row.claim_ref.clone())
            .chain(
                decision_obligations
                    .iter()
                    .flat_map(|obligation| obligation.mission_close_claim_refs.clone()),
            )
            .collect::<BTreeSet<_>>();
        for obligation in &execution_graph.obligations {
            if !valid_claim_refs.contains(&obligation.discharges_claim_ref) {
                findings.push(format!(
                    "execution_graph_obligation_unknown_claim_ref:{}:{}",
                    obligation.obligation_id, obligation.discharges_claim_ref
                ));
            }
        }
        let node_by_id = execution_graph
            .nodes
            .iter()
            .map(|node| (node.spec_id.clone(), node))
            .collect::<BTreeMap<_, _>>();
        for included_spec_id in included_spec_ids {
            let Some(node) = node_by_id.get(included_spec_id) else {
                findings.push(format!(
                    "package_spec_not_in_execution_graph:{}",
                    included_spec_id
                ));
                continue;
            };
            for dependency in &node.depends_on {
                if included_spec_ids
                    .iter()
                    .any(|candidate| candidate == dependency)
                {
                    continue;
                }
                if !dependency_check_satisfies_spec(dependency_satisfaction_state, dependency) {
                    findings.push(format!(
                        "graph_dependency_unsatisfied:{}:{}",
                        included_spec_id, dependency
                    ));
                }
            }
        }

        if *target_type == TargetType::Wave {
            let derived_wave_specs =
                derive_wave_specs_from_execution_graph(execution_graph, included_spec_ids);
            if normalize_wave_specs(wave_specs) != derived_wave_specs {
                findings.push("wave_specs_mismatch_with_execution_graph".to_string());
            }
            for included_spec_id in included_spec_ids {
                if let Some(node) = node_by_id.get(included_spec_id) {
                    for dependency in &node.depends_on {
                        if included_spec_ids
                            .iter()
                            .any(|candidate| candidate == dependency)
                        {
                            findings.push(format!(
                                "wave_dependency_not_frontier:{}:{}",
                                included_spec_id, dependency
                            ));
                        }
                    }
                }
            }
        }
    }

    match target_type {
        TargetType::Spec => {
            if included_specs.len() != 1 {
                findings.push("spec_target_must_include_exactly_one_spec".to_string());
            }
            if !included_specs.iter().any(|spec| spec.spec_id == target_id) {
                findings.push(format!("target_spec_missing_from_package:{target_id}"));
            }
            if let Some(scope) = effective_scope_by_spec_id.get(target_id) {
                for path in scope_paths_outside_frontier(read_scope, &scope.read_paths) {
                    findings.push(format!("package_read_scope_outside_target_spec:{path}"));
                }
                for path in scope_paths_outside_frontier(write_scope, &scope.write_paths) {
                    findings.push(format!("package_write_scope_outside_target_spec:{path}"));
                }
            }
            if !wave_specs.is_empty() {
                findings.push("spec_target_must_not_set_wave_specs".to_string());
            }
        }
        TargetType::Mission => {
            if wave_fingerprint.is_some() {
                findings.push("mission_target_must_not_set_wave_fingerprint".to_string());
            }
            if !wave_specs.is_empty() {
                findings.push("mission_target_must_not_set_wave_specs".to_string());
            }
        }
        TargetType::Wave => {
            if wave_context.is_none_or(str::is_empty) {
                findings.push("wave_context_missing".to_string());
            }
            if wave_fingerprint.is_none() {
                findings.push("wave_fingerprint_missing".to_string());
            }
            let normalized_wave_specs = normalize_wave_specs(wave_specs);
            let wave_scope = PathScope {
                read_paths: normalize_scope_paths(
                    &normalized_wave_specs
                        .iter()
                        .flat_map(|spec| spec.read_paths.clone())
                        .collect::<Vec<_>>(),
                ),
                write_paths: normalize_scope_paths(
                    &normalized_wave_specs
                        .iter()
                        .flat_map(|spec| spec.write_paths.clone())
                        .collect::<Vec<_>>(),
                ),
            };
            if normalize_scope_paths(read_scope) != wave_scope.read_paths {
                findings.push("wave_package_read_scope_mismatch".to_string());
            }
            if normalize_scope_paths(write_scope) != wave_scope.write_paths {
                findings.push("wave_package_write_scope_mismatch".to_string());
            }
            findings.extend(validate_wave_safe_parallelism(
                included_spec_ids,
                wave_specs,
            ));
        }
    }

    let required_gate_checks = vec![
        PackageGateCheck {
            gate_id: "target_resolution".to_string(),
            passed: !findings.iter().any(|finding| {
                finding.starts_with("planning_selected_target_")
                    || finding.starts_with("target_spec_missing_from_package:")
                    || finding == "spec_target_must_include_exactly_one_spec"
                    || finding.ends_with("_must_not_set_wave_specs")
                    || finding == "wave_context_missing"
                    || finding == "wave_fingerprint_missing"
                    || finding.starts_with("outcome_lock_not_locked:")
                    || finding.starts_with("blueprint_not_approved:")
            }),
            detail: format!("resolved {target_ref}"),
        },
        PackageGateCheck {
            gate_id: "dependency_truth_current".to_string(),
            passed: !dependency_satisfaction_state.is_empty()
                && dependency_satisfaction_state.iter().all(|dep| {
                    dep.satisfied && default_dependency_governing_ref(&dep.name).is_some()
                }),
            detail: "dependency satisfaction is explicit, revision-bearing, and fully satisfied"
                .to_string(),
        },
        PackageGateCheck {
            gate_id: "scope_declared".to_string(),
            passed: !read_scope.is_empty() && !write_scope.is_empty(),
            detail: "read and write scope are explicit".to_string(),
        },
        PackageGateCheck {
            gate_id: "scope_bounded_to_frontier".to_string(),
            passed: !findings.iter().any(|finding| {
                finding.starts_with("package_read_scope_outside_")
                    || finding.starts_with("package_write_scope_outside_")
                    || finding.starts_with("package_read_scope_missing_for_included_spec:")
                    || finding.starts_with("package_write_scope_missing_for_included_spec:")
                    || finding == "wave_package_read_scope_mismatch"
                    || finding == "wave_package_write_scope_mismatch"
            }),
            detail: "package scope stays inside the declared execution frontier".to_string(),
        },
        PackageGateCheck {
            gate_id: "proof_contract_declared".to_string(),
            passed: !proof_obligations.is_empty(),
            detail: "proof obligations are explicit".to_string(),
        },
        PackageGateCheck {
            gate_id: "review_contract_declared".to_string(),
            passed: !review_obligations.is_empty(),
            detail: "review obligations are explicit".to_string(),
        },
        PackageGateCheck {
            gate_id: "included_specs_runnable".to_string(),
            passed: !included_specs.is_empty()
                && spec_contexts.iter().all(|context| {
                    context.artifact_status == SpecArtifactStatus::Active
                        && matches!(
                            context.packetization_status,
                            PacketizationStatus::Runnable | PacketizationStatus::ProofGatedSpike
                        )
                }),
            detail: "included specs are active executable frontier specs".to_string(),
        },
        PackageGateCheck {
            gate_id: "planning_truth_current".to_string(),
            passed: lock_doc.frontmatter.status == LockStatus::Locked
                && blueprint_doc.frontmatter.status == BlueprintStatus::Approved
                && spec_contexts.iter().all(|context| {
                    context.blueprint_revision == blueprint_doc.frontmatter.blueprint_revision
                        && context.blueprint_fingerprint.as_ref()
                            == Some(&current_blueprint_fingerprint)
                }),
            detail: "lock, blueprint, and included spec bindings are current".to_string(),
        },
        PackageGateCheck {
            gate_id: "wave_validation".to_string(),
            passed: match target_type {
                TargetType::Wave => !findings.iter().any(|finding| finding.starts_with("wave_")),
                _ => true,
            },
            detail: "wave same-workspace safety rules are satisfied".to_string(),
        },
    ];

    let gate_checks = merge_package_gate_checks(
        gate_checks.unwrap_or(&[]),
        &required_gate_checks,
        &mut findings,
    );
    for gate in &gate_checks {
        if !gate.passed {
            findings.push(format!("gate_check_failed:{}", gate.gate_id));
        }
    }

    Ok(ExecutionPackageContractEvaluation {
        included_specs,
        gate_checks,
        findings: unique_strings(&findings),
    })
}

fn build_wave_manifest(
    paths: &MissionPaths,
    mission_id: &str,
    wave_id: &str,
    included_spec_ids: &[String],
    read_scope: &[String],
    write_scope: &[String],
    wave_specs: &[WaveSpecInput],
) -> Result<(WaveManifest, Fingerprint)> {
    let included_specs = load_spec_contexts(paths, included_spec_ids)?
        .into_iter()
        .map(|context| context.included)
        .collect::<Vec<_>>();
    let manifest = WaveManifest {
        mission_id: mission_id.to_string(),
        wave_id: wave_id.to_string(),
        included_specs,
        read_scope: unique_strings(read_scope),
        write_scope: unique_strings(write_scope),
        wave_specs: normalize_wave_specs(wave_specs),
        generated_at: OffsetDateTime::now_utc(),
    };
    let fingerprint = Fingerprint::from_json(&manifest)?;
    Ok((manifest, fingerprint))
}

fn sync_planning_completion_gate(
    gates: &mut MissionGateIndex,
    mission_id: &str,
    blueprint_revision: u64,
    passed: bool,
    package_id: &str,
    package_path: &Path,
    validation_failures: &[String],
) {
    let gate_id = planning_gate_id(mission_id, blueprint_revision);
    let now = OffsetDateTime::now_utc();
    if let Some(gate) = gates.gates.iter_mut().find(|gate| gate.gate_id == gate_id) {
        gate.status = if passed {
            MissionGateStatus::Passed
        } else {
            MissionGateStatus::Failed
        };
        gate.evaluated_at = Some(now);
        gate.evaluated_against_ref = Some(format!("package:{package_id}"));
        gate.evidence_refs.push(package_path.display().to_string());
        gate.failure_refs = validation_failures.to_vec();
        return;
    }

    append_gate(
        gates,
        MissionGateRecord {
            gate_id,
            gate_kind: GateKind::PlanningCompletion,
            target_ref: format!("mission:{mission_id}"),
            governing_refs: vec![format!("blueprint:{blueprint_revision}")],
            status: if passed {
                MissionGateStatus::Passed
            } else {
                MissionGateStatus::Failed
            },
            blocking: true,
            opened_at: now,
            evaluated_at: Some(now),
            evaluated_against_ref: Some(format!("package:{package_id}")),
            evidence_refs: vec![package_path.display().to_string()],
            failure_refs: validation_failures.to_vec(),
            superseded_by: None,
        },
    );
}

fn package_authorizes_spec(package: &ExecutionPackage, target_spec_id: &str) -> bool {
    match package.target_type {
        TargetType::Spec => {
            package.target_id == target_spec_id
                && package.included_specs.len() == 1
                && package
                    .included_specs
                    .iter()
                    .any(|included| included.spec_id == target_spec_id)
        }
        TargetType::Mission | TargetType::Wave => package
            .included_specs
            .iter()
            .any(|included| included.spec_id == target_spec_id),
    }
}

fn merge_package_gate_checks(
    provided: &[PackageGateCheck],
    required: &[PackageGateCheck],
    findings: &mut Vec<String>,
) -> Vec<PackageGateCheck> {
    let mut provided_by_id = BTreeMap::new();
    for gate in provided {
        provided_by_id.insert(gate.gate_id.clone(), gate.clone());
    }

    let mut merged = Vec::new();
    for gate in required {
        if let Some(provided_gate) = provided_by_id.remove(&gate.gate_id) {
            merged.push(PackageGateCheck {
                gate_id: gate.gate_id.clone(),
                passed: gate.passed && provided_gate.passed,
                detail: if provided_gate.detail.is_empty() {
                    gate.detail.clone()
                } else {
                    provided_gate.detail
                },
            });
        } else {
            merged.push(gate.clone());
        }
    }

    for unknown in provided_by_id.into_keys() {
        findings.push(format!("unknown_gate_check:{unknown}"));
    }

    merged
}

fn load_mission_close_spec_ids(
    paths: &MissionPaths,
    _blueprint_revision: u64,
) -> Result<Vec<String>> {
    let mut spec_ids = Vec::new();
    if !paths.specs_root().is_dir() {
        return Ok(spec_ids);
    }
    for entry in fs::read_dir(paths.specs_root())
        .with_context(|| format!("failed to read {}", paths.specs_root().display()))?
    {
        let entry = entry.context("failed to read spec dir entry")?;
        if !entry.path().is_dir() {
            continue;
        }
        let spec_id = entry.file_name().to_string_lossy().to_string();
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&spec_id))?;
        if spec_doc.frontmatter.artifact_status == SpecArtifactStatus::Active
            && spec_doc.frontmatter.packetization_status != PacketizationStatus::Descoped
        {
            spec_ids.push(spec_id);
        }
    }
    spec_ids.sort();
    Ok(spec_ids)
}

fn load_descoped_mission_close_spec_ids(
    paths: &MissionPaths,
    _blueprint_revision: u64,
) -> Result<Vec<String>> {
    let mut spec_ids = Vec::new();
    if !paths.specs_root().is_dir() {
        return Ok(spec_ids);
    }
    for entry in fs::read_dir(paths.specs_root())
        .with_context(|| format!("failed to read {}", paths.specs_root().display()))?
    {
        let entry = entry.context("failed to read spec dir entry")?;
        if !entry.path().is_dir() {
            continue;
        }
        let spec_id = entry.file_name().to_string_lossy().to_string();
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&spec_id))?;
        if spec_doc.frontmatter.artifact_status == SpecArtifactStatus::Active
            && spec_doc.frontmatter.packetization_status == PacketizationStatus::Descoped
        {
            spec_ids.push(spec_id);
        }
    }
    spec_ids.sort();
    Ok(spec_ids)
}

fn mission_close_eligibility_findings(
    paths: &MissionPaths,
    bundle: &ReviewBundle,
    gates: &MissionGateIndex,
) -> Result<Vec<String>> {
    let mut findings = Vec::new();
    let execution_graph = load_execution_graph(paths)?;
    for spec_id in &bundle.included_spec_refs {
        let spec_doc = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(spec_id))?;
        let frontmatter = spec_doc.frontmatter;
        if frontmatter.artifact_status != SpecArtifactStatus::Active {
            findings.push(format!("mission_close_spec_not_active:{spec_id}"));
        }
        if frontmatter.blueprint_revision != bundle.blueprint_revision
            && frontmatter.execution_status != SpecExecutionStatus::Complete
        {
            findings.push(format!("mission_close_spec_blueprint_drift:{spec_id}"));
        }
        if matches!(
            frontmatter.packetization_status,
            PacketizationStatus::NearFrontier
                | PacketizationStatus::ProofGatedSpike
                | PacketizationStatus::ProvisionalBacklog
                | PacketizationStatus::DeferredTruthMotion
        ) {
            findings.push(format!(
                "mission_close_spec_not_close_eligible_packetization:{spec_id}:{:?}",
                frontmatter.packetization_status
            ));
        }
        if frontmatter.packetization_status == PacketizationStatus::Descoped {
            continue;
        }
        if frontmatter.execution_status != SpecExecutionStatus::Complete {
            findings.push(format!(
                "mission_close_spec_not_complete:{spec_id}:{:?}",
                frontmatter.execution_status
            ));
        }
        let target_ref = format!("spec:{spec_id}");
        let has_passed_review = gates.gates.iter().any(|gate| {
            gate.gate_kind == GateKind::BlockingReview
                && gate.target_ref == target_ref
                && (gate.status == MissionGateStatus::Passed
                    || (gate.status == MissionGateStatus::Stale
                        && !unresolved_review_gate_blocks_resume(paths, gate)))
        });
        if !has_passed_review {
            findings.push(format!(
                "mission_close_spec_review_missing_or_not_passed:{spec_id}"
            ));
        }
        if let Some(execution_graph) = execution_graph.as_ref() {
            for obligation in execution_graph.obligations.iter().filter(|obligation| {
                obligation.target_spec_id == *spec_id
                    && obligation.blocking
                    && obligation.status != ExecutionGraphObligationStatus::Satisfied
                    && obligation.status != ExecutionGraphObligationStatus::Descoped
            }) {
                findings.push(format!(
                    "mission_close_obligation_not_satisfied:{}:{:?}",
                    obligation.obligation_id, obligation.status
                ));
            }
        }
    }
    Ok(findings)
}

fn validate_mission_close_source_package(
    paths: &MissionPaths,
    package: &ExecutionPackage,
) -> Result<Vec<String>> {
    let current_lock =
        load_markdown::<crate::artifacts::OutcomeLockFrontmatter>(&paths.outcome_lock())?;
    let current_blueprint =
        load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())?;
    let current_lock_fp = current_lock.fingerprint()?;
    let current_blueprint_fp = current_blueprint_contract_fingerprint(paths, &current_blueprint)?;
    let gates = load_gate_index(paths)?;
    let mut findings = Vec::new();

    if package.status != ExecutionPackageStatus::Passed {
        findings.push(format!(
            "package_status_not_executable:{:?}",
            package.status
        ));
    }
    if package.lock_fingerprint != current_lock_fp {
        findings.push("lock_fingerprint_mismatch".to_string());
    }
    if package.blueprint_fingerprint != current_blueprint_fp {
        findings.push("blueprint_fingerprint_mismatch".to_string());
    }
    if package.target_type == TargetType::Wave {
        let wave_manifest_path = paths.wave_manifest(&package.target_id);
        if !wave_manifest_path.is_file() {
            findings.push("wave_manifest_missing".to_string());
        } else {
            let manifest: WaveManifest = load_json(&wave_manifest_path)?;
            let manifest_fingerprint = Fingerprint::from_json(&manifest)?;
            if package.wave_fingerprint.as_ref() != Some(&manifest_fingerprint) {
                findings.push("wave_manifest_fingerprint_mismatch".to_string());
            }
            if manifest.included_specs != package.included_specs {
                findings.push("wave_manifest_included_specs_mismatch".to_string());
            }
            if manifest.read_scope != package.read_scope {
                findings.push("wave_manifest_read_scope_mismatch".to_string());
            }
            if manifest.write_scope != package.write_scope {
                findings.push("wave_manifest_write_scope_mismatch".to_string());
            }
            if manifest.wave_specs != normalize_wave_specs(&package.wave_specs) {
                findings.push("wave_manifest_wave_specs_mismatch".to_string());
            }
        }
    }
    if let Some(gate) = gates.gates.iter().find(|gate| {
        gate.gate_kind == GateKind::ExecutionPackage
            && gate.evaluated_against_ref.as_deref() == Some(package.package_id.as_str())
    }) && !matches!(gate.status, MissionGateStatus::Passed)
    {
        findings.push(format!("execution_gate_status:{:?}", gate.status));
    } else if !gates.gates.iter().any(|gate| {
        gate.gate_kind == GateKind::ExecutionPackage
            && gate.evaluated_against_ref.as_deref() == Some(package.package_id.as_str())
            && matches!(gate.status, MissionGateStatus::Passed)
    }) {
        findings.push("execution_gate_missing".to_string());
    }

    for included in &package.included_specs {
        let spec_doc =
            load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file(&included.spec_id))?;
        if spec_doc.frontmatter.spec_revision != included.spec_revision {
            findings.push(format!("spec_revision_mismatch:{}", included.spec_id));
            continue;
        }

        let current_fp = spec_doc.fingerprint()?;
        if current_fp == included.spec_fingerprint {
            continue;
        }

        if spec_doc.frontmatter.execution_status == SpecExecutionStatus::Complete {
            let mut normalized_doc = spec_doc.clone();
            normalized_doc.frontmatter.execution_status = SpecExecutionStatus::Packaged;
            if normalized_doc.fingerprint()? == included.spec_fingerprint {
                continue;
            }
        }

        findings.push(format!("spec_fingerprint_mismatch:{}", included.spec_id));
    }

    Ok(findings)
}

fn next_spec_revision(paths: &MissionPaths, spec: &WorkstreamSpecInput, body: &str) -> Result<u64> {
    let spec_path = paths.spec_file(&spec.spec_id);
    if !spec_path.is_file() {
        return Ok(1);
    }

    let existing = load_markdown::<WorkstreamSpecFrontmatter>(&spec_path)?;
    if spec_materially_matches_existing(&existing, spec, body) {
        Ok(existing.frontmatter.spec_revision)
    } else {
        Ok(existing.frontmatter.spec_revision + 1)
    }
}

fn next_blueprint_revision(
    existing: Option<&ArtifactDocument<ProgramBlueprintFrontmatter>>,
    input: &PlanningWriteInput,
    lock_revision: u64,
) -> u64 {
    let requested = input.blueprint_revision.unwrap_or(1);
    let Some(existing) = existing else {
        return requested.max(1);
    };
    let unchanged = blueprint_materially_matches_existing(existing, input, lock_revision);
    if unchanged {
        existing.frontmatter.blueprint_revision
    } else {
        requested.max(existing.frontmatter.blueprint_revision + 1)
    }
}

fn blueprint_materially_matches_existing(
    existing: &ArtifactDocument<ProgramBlueprintFrontmatter>,
    input: &PlanningWriteInput,
    lock_revision: u64,
) -> bool {
    existing.body.trim_end_matches('\n') == input.body_markdown.trim_end_matches('\n')
        && existing.frontmatter.lock_revision == lock_revision
        && existing.frontmatter.plan_level == input.plan_level
        && existing.frontmatter.problem_size == input.problem_size
        && existing.frontmatter.status == input.status.unwrap_or(BlueprintStatus::Draft)
        && normalize_proof_matrix(&existing.frontmatter.proof_matrix)
            == normalize_proof_matrix(&input.proof_matrix)
        && normalize_decision_obligations(&existing.frontmatter.decision_obligations)
            == normalize_decision_obligations(&input.decision_obligations)
        && existing.frontmatter.selected_target_ref == input.selected_target_ref
}

fn spec_materially_matches_existing(
    existing: &ArtifactDocument<WorkstreamSpecFrontmatter>,
    spec: &WorkstreamSpecInput,
    body: &str,
) -> bool {
    existing.body.trim_end_matches('\n') == body.trim_end_matches('\n')
        && existing.frontmatter.artifact_status
            == spec.artifact_status.unwrap_or(SpecArtifactStatus::Draft)
        && existing.frontmatter.packetization_status
            == spec
                .packetization_status
                .unwrap_or(PacketizationStatus::NearFrontier)
        && existing.frontmatter.execution_status
            == spec
                .execution_status
                .unwrap_or(SpecExecutionStatus::NotStarted)
        && existing.frontmatter.owner_mode == spec.owner_mode.unwrap_or(OwnerMode::Solo)
}

fn fingerprint_json<T: Serialize>(value: &T) -> Result<Fingerprint> {
    let bytes = serde_json::to_vec(value).context("failed to encode json for fingerprint")?;
    Ok(Fingerprint::from_bytes(&bytes))
}

fn ensure_file(path: PathBuf, contents: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn unique_strings(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            result.push(value.clone());
        }
    }
    result
}

fn ensure_paths_match_mission(paths: &MissionPaths, mission_id: &str) -> Result<()> {
    if paths.mission_id() != mission_id {
        anyhow::bail!(
            "mission id mismatch: mission paths target {}, but input requested {}",
            paths.mission_id(),
            mission_id
        );
    }
    Ok(())
}

fn extract_first_heading_or_sentence(markdown: &str) -> String {
    markdown
        .lines()
        .find(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map_or_else(
            || "See Outcome Lock".to_string(),
            |line| line.trim().to_string(),
        )
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::TempDir;
    use time::OffsetDateTime;

    use super::{
        BlueprintStatus, BundleKind, ClarifyStatus, ContradictionInput, ContradictionRecord,
        ContradictionStatus, DependencyCheck, ExecutionGraph, ExecutionGraphInput,
        ExecutionGraphNode, ExecutionGraphNodeInput, ExecutionGraphObligation,
        ExecutionGraphObligationInput, ExecutionGraphObligationKind,
        ExecutionGraphObligationStatus, ExecutionPackage, ExecutionPackageInput, GateKind,
        IncludedSpecRef, LockStatus, MachineAction, MissionGateIndex, MissionGateRecord,
        MissionGateStatus, MissionInitInput, NextRequiredBranch, PacketizationStatus,
        PlanningWriteInput, ProblemSize, ProgramBlueprintFrontmatter, ReopenLayer, ReplanBoundary,
        ResolveResumeInput, ReviewBundleInput, ReviewFindingInput, ReviewResultInput,
        SelectionConsumptionInput, SelectionOutcome, SelectionResolutionInput,
        SelectionStateAction, SelectionStateInput, SpecArtifactStatus, SpecExecutionStatus,
        TargetType, TriageDecision, WaitingRequest, WaitingRequestAcknowledgementInput,
        WaveRiskClass, WaveSpecInput, WorkstreamSpecFrontmatter, WorkstreamSpecInput,
        WriterPacketInput, acknowledge_waiting_request, active_cycle_from_closeout,
        append_contradiction, compile_execution_package, compile_review_bundle,
        consume_selection_wait, contradiction_resume_override, derive_writer_packet,
        execution_graph_to_contract_input, initialize_mission,
        invalidate_review_history_for_execution_target, load_active_blueprint_spec_ids,
        load_closeouts, load_execution_graph, load_gate_index, load_markdown, open_selection_wait,
        rebuild_state_from_closeouts, record_review_result, resolve_resume, resolve_selection_wait,
        unresolved_blocking_gate_refs, unresolved_blocking_gate_refs_for_source_package,
        unresolved_review_gate_blocks_resume, validate_execution_package, validate_review_bundle,
        write_closeout, write_json, write_planning_artifacts,
    };
    use crate::{
        ActiveCycleState, ChildLaneExpectation, ChildLaneIntegrationStatus, CloseoutRecord,
        DecisionAffect, DecisionBlockingness, DecisionObligation, DecisionStatus,
        ExecutionPackageStatus, Fingerprint, MissionPaths, ProofMatrixRow, ResumeMode,
        ResumeStatus, Terminality, Verdict, load_state,
    };

    fn execution_graph_node(
        spec_id: &str,
        depends_on: &[&str],
        read_paths: &[&str],
        write_paths: &[&str],
        ownership_domains: &[&str],
        risk_class: Option<WaveRiskClass>,
        acceptance_checks: &[&str],
    ) -> ExecutionGraphNodeInput {
        ExecutionGraphNodeInput {
            spec_id: spec_id.to_string(),
            depends_on: depends_on.iter().map(|value| value.to_string()).collect(),
            produces: vec![format!("artifact:{spec_id}")],
            read_paths: read_paths.iter().map(|value| value.to_string()).collect(),
            write_paths: write_paths.iter().map(|value| value.to_string()).collect(),
            exclusive_resources: Vec::new(),
            coupling_tags: Vec::new(),
            ownership_domains: ownership_domains
                .iter()
                .map(|value| value.to_string())
                .collect(),
            risk_class,
            acceptance_checks: acceptance_checks
                .iter()
                .map(|value| value.to_string())
                .collect(),
            evidence_type: "receipt".to_string(),
        }
    }

    fn execution_graph_with_default_obligations(
        nodes: Vec<ExecutionGraphNodeInput>,
    ) -> ExecutionGraphInput {
        let obligations = nodes
            .iter()
            .flat_map(|node| {
                node.acceptance_checks.iter().map(|acceptance_check| {
                    ExecutionGraphObligationInput {
                        obligation_id: format!("{}:{acceptance_check}", node.spec_id),
                        kind: ExecutionGraphObligationKind::Validation,
                        target_spec_id: node.spec_id.clone(),
                        discharges_claim_ref: "claim:default-proof".to_string(),
                        proof_rows: vec![acceptance_check.clone()],
                        acceptance_checks: vec![acceptance_check.clone()],
                        required_evidence: vec!["RECEIPTS/test.txt".to_string()],
                        review_lenses: vec!["correctness".to_string()],
                        blocking: true,
                        status: ExecutionGraphObligationStatus::Open,
                        satisfied_by: Vec::new(),
                        evidence_refs: Vec::new(),
                    }
                })
            })
            .collect();

        ExecutionGraphInput { nodes, obligations }
    }

    fn default_proof_matrix() -> Vec<ProofMatrixRow> {
        vec![ProofMatrixRow {
            claim_ref: "claim:default-proof".to_string(),
            statement: "The selected route has a declared proof contract.".to_string(),
            required_evidence: vec!["RECEIPTS/test.txt".to_string()],
            review_lenses: vec!["correctness".to_string()],
            governing_contract_refs: vec!["blueprint".to_string()],
        }]
    }

    fn reviewer_output_evidence(label: &str, receipts: &[&str]) -> Vec<String> {
        std::iter::once(format!("reviewer-output:{label}"))
            .chain(receipts.iter().map(|receipt| (*receipt).to_string()))
            .collect()
    }

    fn review_truth_snapshot_for(
        paths: &MissionPaths,
        bundle_id: &str,
    ) -> Option<super::ReviewTruthSnapshot> {
        Some(
            super::capture_review_truth_snapshot(paths, bundle_id)
                .expect("capture review truth snapshot"),
        )
    }

    fn canonical_blueprint_body() -> String {
        "# Program Blueprint

## Locked Mission Reference

- Mission id: `mission_alpha`

## Truth Register Summary

- Locked facts are reflected here.

## System Model

- Touched surfaces: API and storage.

## Invariants And Protected Behaviors

- Keep the locked outcome honest.

## Proof Matrix

- claim:default-proof

## Decision Obligations

- obligation:route-choice

## In-Scope Work Inventory

- runtime_core

## Selected Architecture

Route truth.

## Execution Graph and Safe-Wave Rules

- Single-node routes may execute directly; multi-node routes must follow the declared graph frontier.

## Decision Log

- Chose the canonical runtime route because it keeps proof and review contracts visible.

## Review Bundle Design

- Mandatory review lenses: correctness, security_review

## Workstream Overview

- api

## Risks And Unknowns

- Rollout coupling remains explicit.

## Replan Policy

- Reopen planning if scope or proof changes.
"
        .to_string()
    }

    fn table_blueprint_body() -> String {
        "# Program Blueprint

## Locked Mission Reference

- Mission id: `mission_alpha`

## Truth Register Summary

- Locked facts are reflected here.

## System Model

- Touched surfaces: API and storage.

## Invariants And Protected Behaviors

- Keep the locked outcome honest.

## Proof Matrix

- claim:default-proof

## Decision Obligations

- obligation:route-choice

## In-Scope Work Inventory

| Work item | Class | Why it exists | Finish in this mission? |
| --- | --- | --- | --- |
| runtime_core | runnable_frontier | It carries the current mission target. | yes |

## Selected Architecture

Route truth.

## Execution Graph and Safe-Wave Rules

- Single-node routes may execute directly; multi-node routes must follow the declared graph frontier.

## Decision Log

| Decision id | Statement | Rationale | Evidence refs | Adopted in revision |
| --- | --- | --- | --- | --- |
| D1 | Keep the runtime route bounded. | It preserves proof and review clarity. | RECEIPTS/test.txt | 1 |

## Review Bundle Design

- Mandatory review lenses: correctness, security_review

## Workstream Overview

- api

## Risks And Unknowns

- Rollout coupling remains explicit.

## Replan Policy

- Reopen planning if scope or proof changes.
"
        .to_string()
    }

    fn canonical_spec_body(purpose: &str) -> String {
        canonical_spec_body_with_scope(purpose, &["src/api"], &["src/api"])
    }

    fn canonical_spec_body_with_scope(
        purpose: &str,
        read_scope: &[&str],
        write_scope: &[&str],
    ) -> String {
        format!(
            "# Workstream Spec

## Purpose

{purpose}

## In Scope

- Update the selected slice.

## Out Of Scope

- UI work.

## Dependencies

- Locked mission truth.

## Touched Surfaces

- API.

## Read Scope

{}

## Write Scope

{}

## Interfaces And Contracts Touched

- public-api

## Implementation Shape

Keep the implementation bounded and explicit.

## Proof-Of-Completion Expectations

- cargo test

## Non-Breakage Expectations

- Existing callers still work.

## Review Lenses

- correctness

## Replan Boundary

- Reopen planning on scope expansion.

## Truth Basis Refs

- OUTCOME-LOCK.md

## Freshness Notes

- Current as of planning time.

## Support Files

- `REVIEW.md`
",
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
        )
    }

    fn open_major_decision_obligation() -> Vec<DecisionObligation> {
        vec![DecisionObligation {
            obligation_id: "obligation:route-choice".to_string(),
            question: "Which rollout route is safe enough to select?".to_string(),
            why_it_matters: "It changes the selected route and review contract.".to_string(),
            affects: vec![
                DecisionAffect::MigrationRollout,
                DecisionAffect::ReviewContract,
            ],
            governing_contract_refs: vec!["blueprint".to_string()],
            review_contract_refs: vec!["review:mission".to_string()],
            mission_close_claim_refs: vec!["claim:mission-close".to_string()],
            blockingness: DecisionBlockingness::Major,
            candidate_route_count: 2,
            required_evidence: vec!["RECEIPTS/research.txt".to_string()],
            status: DecisionStatus::Open,
            resolution_rationale: None,
            evidence_refs: Vec::new(),
            proof_spike_scope: None,
            proof_spike_success_criteria: Vec::new(),
            proof_spike_failure_criteria: Vec::new(),
            proof_spike_discharge_artifacts: Vec::new(),
            proof_spike_failure_route: None,
        }]
    }

    #[test]
    fn creates_mission_and_runtime_contracts() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::Packaged),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the UI slice",
                            &["src/ui"],
                            &["src/ui"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["backend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p api"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &[],
                        &["src/ui"],
                        &["src/ui"],
                        &["frontend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p ui"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![
                    DependencyCheck {
                        name: "lock_current".to_string(),
                        satisfied: true,
                        detail: "current lock is ratified".to_string(),
                    },
                    DependencyCheck {
                        name: "blueprint_current".to_string(),
                        satisfied: true,
                        detail: "current blueprint selects mission close".to_string(),
                    },
                    DependencyCheck {
                        name: "spec:spec_api".to_string(),
                        satisfied: true,
                        detail: "spec_api is complete and review-clean".to_string(),
                    },
                ],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["tests pass".to_string()],
                review_obligations: vec!["spec review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        assert_eq!(package.status, super::ExecutionPackageStatus::Passed);

        let validation = validate_execution_package(&paths, &package.package_id)
            .expect("package validation should work");
        assert!(validation.valid);

        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string(), "proof".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["tests pass".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        let bundle_validation = validate_review_bundle(&paths, &bundle.bundle_id)
            .expect("bundle validation should work");
        assert!(bundle_validation.valid);
    }

    #[test]
    fn runtime_refreshes_readme_and_spec_support_files_from_templates() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the UI slice",
                            &["src/ui"],
                            &["src/ui"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("mission:mission_alpha".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["backend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p api"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &[],
                        &["src/ui"],
                        &["src/ui"],
                        &["frontend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p ui"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let readme = std::fs::read_to_string(paths.readme()).expect("read README");
        assert!(readme.contains("## Start Here"));
        assert!(readme.contains("Next recommended action"));
        assert!(readme.contains("Current blocker"));

        let spec_review =
            std::fs::read_to_string(paths.review_file("spec_api")).expect("read spec review");
        assert!(spec_review.contains("## Review Events"));
        assert!(spec_review.contains("## Findings"));
    }

    #[test]
    fn stale_blocking_gates_remain_unresolved() {
        let stale_gate = MissionGateRecord {
            gate_id: "mission_alpha:blocking_review:spec:spec_api:review_bundle_1".to_string(),
            gate_kind: GateKind::BlockingReview,
            target_ref: "spec:spec_api".to_string(),
            governing_refs: vec!["bundle:review_bundle_1".to_string()],
            status: MissionGateStatus::Stale,
            blocking: true,
            opened_at: OffsetDateTime::now_utc(),
            evaluated_at: Some(OffsetDateTime::now_utc()),
            evaluated_against_ref: Some("review_bundle_1".to_string()),
            evidence_refs: vec![".ralph/review-bundles/review_bundle_1.json".to_string()],
            failure_refs: vec!["review bundle superseded".to_string()],
            superseded_by: None,
        };
        let index = MissionGateIndex {
            mission_id: "mission_alpha".to_string(),
            current_phase: "review".to_string(),
            updated_at: OffsetDateTime::now_utc(),
            gates: vec![stale_gate],
        };

        let unresolved = unresolved_blocking_gate_refs(&index, None);

        assert_eq!(
            unresolved,
            vec![
                "mission_alpha:blocking_review:spec:spec_api:review_bundle_1:spec:spec_api"
                    .to_string()
            ]
        );
    }

    #[test]
    fn stale_review_gate_for_completed_spec_does_not_block_resume() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Complete),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        let raw = std::fs::read_to_string(paths.spec_file("spec_api")).expect("read spec");
        std::fs::write(
            paths.spec_file("spec_api"),
            raw.replace("execution_status: packaged", "execution_status: complete"),
        )
        .expect("mark spec complete without changing its revision");

        let stale_gate = MissionGateRecord {
            gate_id: format!(
                "mission_alpha:blocking_review:spec:spec_api:{}",
                bundle.bundle_id
            ),
            gate_kind: GateKind::BlockingReview,
            target_ref: "spec:spec_api".to_string(),
            governing_refs: vec![format!("bundle:{}", bundle.bundle_id)],
            status: MissionGateStatus::Stale,
            blocking: true,
            opened_at: OffsetDateTime::now_utc(),
            evaluated_at: Some(OffsetDateTime::now_utc()),
            evaluated_against_ref: Some(bundle.bundle_id.clone()),
            evidence_refs: vec![paths.review_bundle(&bundle.bundle_id).display().to_string()],
            failure_refs: vec!["blueprint advanced after clean review".to_string()],
            superseded_by: None,
        };

        assert!(
            !unresolved_review_gate_blocks_resume(&paths, &stale_gate),
            "stale review history for an already-complete spec should not block the parent resume loop"
        );
    }

    #[test]
    fn stale_review_gate_for_changed_completed_spec_still_blocks_resume() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Complete),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        let mut raw = std::fs::read_to_string(paths.spec_file("spec_api")).expect("read spec");
        raw.push_str(
            "\n## Post Review Drift\n\n- Changed after review while still marked complete.\n",
        );
        std::fs::write(paths.spec_file("spec_api"), raw).expect("write drifted spec");

        let stale_gate = MissionGateRecord {
            gate_id: format!(
                "mission_alpha:blocking_review:spec:spec_api:{}",
                bundle.bundle_id
            ),
            gate_kind: GateKind::BlockingReview,
            target_ref: "spec:spec_api".to_string(),
            governing_refs: vec![format!("bundle:{}", bundle.bundle_id)],
            status: MissionGateStatus::Stale,
            blocking: true,
            opened_at: OffsetDateTime::now_utc(),
            evaluated_at: Some(OffsetDateTime::now_utc()),
            evaluated_against_ref: Some(bundle.bundle_id.clone()),
            evidence_refs: vec![paths.review_bundle(&bundle.bundle_id).display().to_string()],
            failure_refs: vec!["spec changed after review".to_string()],
            superseded_by: None,
        };

        assert!(
            unresolved_review_gate_blocks_resume(&paths, &stale_gate),
            "stale review history for a changed completed spec must still block re-review"
        );
    }

    #[test]
    fn wave_packages_fail_when_parallel_specs_overlap_write_scope() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the API slice",
                            &["src/api"],
                            &["src/shared"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::Packaged),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the UI slice",
                            &["src/ui"],
                            &["src/shared"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::Packaged),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("wave:wave_alpha".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/shared"],
                        &["backend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &[],
                        &["src/ui"],
                        &["src/shared"],
                        &["frontend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Wave,
                target_id: "wave_alpha".to_string(),
                included_spec_ids: vec!["spec_api".to_string(), "spec_ui".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src".to_string()],
                write_scope: vec!["src".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["wave review".to_string()],
                replan_boundary: None,
                wave_context: Some("parallel_same_workspace".to_string()),
                wave_fingerprint: None,
                wave_specs: vec![
                    WaveSpecInput {
                        spec_id: "spec_api".to_string(),
                        read_paths: vec!["src/api".to_string()],
                        write_paths: vec!["src/shared".to_string()],
                        exclusive_resources: Vec::new(),
                        coupling_tags: Vec::new(),
                        ownership_domains: vec!["backend".to_string()],
                        risk_class: Some(WaveRiskClass::Normal),
                    },
                    WaveSpecInput {
                        spec_id: "spec_ui".to_string(),
                        read_paths: vec!["src/ui".to_string()],
                        write_paths: vec!["src/shared".to_string()],
                        exclusive_resources: Vec::new(),
                        coupling_tags: Vec::new(),
                        ownership_domains: vec!["frontend".to_string()],
                        risk_class: Some(WaveRiskClass::Normal),
                    },
                ],
                gate_checks: Vec::new(),
            },
        )
        .expect("wave package compilation should work");

        assert_eq!(package.status, super::ExecutionPackageStatus::Failed);
        assert!(
            package
                .validation_failures
                .iter()
                .any(|finding| finding == "wave_write_path_overlap:spec_api:spec_ui:src/shared")
        );
    }

    #[test]
    fn active_cycle_captures_closeout_recovery_context() {
        let closeout = CloseoutRecord {
            closeout_id: Some("closeout-1".to_string()),
            closeout_seq: 7,
            mission_id: "mission_alpha".to_string(),
            phase: "review".to_string(),
            activity: "review_bundle_compiled".to_string(),
            verdict: Verdict::ContinueRequired,
            terminality: Terminality::ActionableNonTerminal,
            resume_mode: ResumeMode::Continue,
            next_phase: Some("review".to_string()),
            next_action: "Run blocking review for spec_api.".to_string(),
            target: Some("spec:spec_api".to_string()),
            cycle_kind: Some(super::CycleKind::GateEvaluation),
            lock_revision: Some(3),
            lock_fingerprint: Some(Fingerprint::from_bytes(b"lock-fp").to_string()),
            blueprint_revision: Some(5),
            blueprint_fingerprint: Some(Fingerprint::from_bytes(b"blueprint-fp").to_string()),
            governing_revision: Some("bundle:bundle_123".to_string()),
            reason_code: Some("review_required".to_string()),
            summary: Some("Review is required before execution can continue.".to_string()),
            continuation_prompt: Some("Continue into review.".to_string()),
            cycle_id: Some("cycle-123".to_string()),
            waiting_request_id: None,
            waiting_for: None,
            canonical_waiting_request: None,
            resume_condition: None,
            request_emitted_at: None,
            active_child_task_paths: vec!["lanes/spec_api-review.md".to_string()],
            artifact_fingerprints: BTreeMap::new(),
        };

        let child_lane = ChildLaneExpectation {
            task_path: "lanes/spec_api-review.md".to_string(),
            lane_kind: "review".to_string(),
            expected_deliverable_ref: "bundle:bundle_123".to_string(),
            lane_role: Some(crate::ralph::ChildLaneRole::FindingsOnlyReviewer),
            integration_status: ChildLaneIntegrationStatus::Pending,
            target_ref: Some("spec:spec_api".to_string()),
        };
        let active_cycle = active_cycle_from_closeout(
            &closeout,
            vec![child_lane.clone()],
            vec![
                "bundle_compiled".to_string(),
                "review_gate_opened".to_string(),
            ],
            vec![".ralph/review-bundles/bundle_123.json".to_string()],
            vec!["packet_123".to_string()],
            vec!["bundle_123".to_string()],
        );

        assert_eq!(active_cycle.cycle_id, "cycle-123");
        assert_eq!(
            active_cycle.cycle_kind,
            Some(super::CycleKind::GateEvaluation)
        );
        assert_eq!(
            active_cycle.activity.as_deref(),
            Some("review_bundle_compiled")
        );
        assert_eq!(
            active_cycle.current_target.as_deref(),
            Some("spec:spec_api")
        );
        assert_eq!(active_cycle.lock_revision, Some(3));
        assert_eq!(
            active_cycle.lock_fingerprint.as_deref(),
            Some(Fingerprint::from_bytes(b"lock-fp").as_str())
        );
        assert_eq!(active_cycle.blueprint_revision, Some(5));
        assert_eq!(
            active_cycle.blueprint_fingerprint.as_deref(),
            Some(Fingerprint::from_bytes(b"blueprint-fp").as_str())
        );
        assert_eq!(
            active_cycle.governing_revision.as_deref(),
            Some("bundle:bundle_123")
        );
        assert_eq!(
            active_cycle.current_bounded_action.as_deref(),
            Some("Run blocking review for spec_api.")
        );
        assert_eq!(
            active_cycle.preconditions_checked,
            vec![
                "bundle_compiled".to_string(),
                "review_gate_opened".to_string()
            ]
        );
        assert_eq!(
            active_cycle.expected_outputs,
            vec![".ralph/review-bundles/bundle_123.json".to_string()]
        );
        assert_eq!(
            active_cycle.active_packet_refs,
            vec!["packet_123".to_string()]
        );
        assert_eq!(
            active_cycle.active_bundle_refs,
            vec!["bundle_123".to_string()]
        );
        assert_eq!(active_cycle.expected_child_lanes, vec![child_lane]);
    }

    #[test]
    fn execution_package_replan_boundary_must_match_visible_spec_contract() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: None,
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Packaged),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let mut mismatched_boundary = ReplanBoundary::default();
        mismatched_boundary.local_repair_allowed = true;

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["spec review".to_string()],
                replan_boundary: Some(mismatched_boundary),
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");

        assert_eq!(package.status, super::ExecutionPackageStatus::Failed);
        assert!(
            package
                .validation_failures
                .iter()
                .any(|finding| { finding == "package_replan_boundary_mismatch_with_specs" })
        );
    }

    #[test]
    fn planning_requires_locked_outcome_lock() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Clarifying),
                lock_status: Some(LockStatus::Draft),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: Vec::new(),
                selected_target_ref: None,
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("planning should require a locked outcome lock");
        assert!(
            error
                .to_string()
                .contains("until the outcome lock is locked")
        );
        assert!(!paths.outcome_lock().exists());
    }

    #[test]
    fn planning_rejects_plan_level_below_computed_risk_floor() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship a public API migration with rollback constraints.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: Some(
                    "# Outcome Lock\n\nThis is a public API migration with rollback constraints.\n"
                        .to_string(),
                ),
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 2,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: None,
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("planning should reject plan levels below the risk floor");

        assert!(error.to_string().contains("below the computed risk floor"));
    }

    #[test]
    fn approved_planning_rejects_blocking_decision_obligations() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 4,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: open_major_decision_obligation(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: None,
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("approved planning should reject blocking decision obligations");

        assert!(error.to_string().contains("blocking decision obligations"));
    }

    #[test]
    fn descoped_active_specs_do_not_force_execution_graph_authoring() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 4,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_descoped".to_string(),
                        purpose: "Track descoped follow-on work".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Descoped),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("descoped active specs should not require an execution graph");

        assert!(!paths.execution_graph().exists());
    }

    #[test]
    fn clarify_bootstrap_leaves_lock_unratified() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        let report = initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Clarify the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Clarifying),
                lock_status: Some(LockStatus::Draft),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let mission_state =
            load_markdown::<crate::artifacts::MissionStateFrontmatter>(&paths.mission_state())
                .expect("mission state should parse");
        let closeouts = load_closeouts(&paths.closeouts_ndjson()).expect("closeouts should load");
        let latest = closeouts.last().expect("bootstrap closeout should exist");
        let gates = load_gate_index(&paths).expect("gate index should load");

        assert_eq!(report.lock_fingerprint, None);
        assert_eq!(mission_state.frontmatter.current_lock_revision, None);
        assert!(!paths.outcome_lock().exists());
        assert_eq!(latest.lock_revision, None);
        assert_eq!(latest.lock_fingerprint, None);
        assert_eq!(
            latest.governing_revision.as_deref(),
            Some("clarify:mission_state")
        );
        assert!(
            latest.artifact_fingerprints.contains_key("mission_state"),
            "clarify bootstrap should fingerprint mission-state truth"
        );
        assert!(
            !latest.artifact_fingerprints.contains_key("outcome_lock"),
            "clarify bootstrap should not claim an unratified outcome lock"
        );
        let outcome_lock_gate = gates
            .gates
            .iter()
            .find(|gate| gate.gate_kind == GateKind::OutcomeLock)
            .expect("outcome-lock gate should exist");
        assert_eq!(outcome_lock_gate.status, MissionGateStatus::Open);
        assert_eq!(outcome_lock_gate.evaluated_at, None);
        assert_eq!(outcome_lock_gate.evaluated_against_ref, None);
        assert!(outcome_lock_gate.evidence_refs.is_empty());
    }

    #[test]
    fn runtime_rejects_mission_id_mismatches() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_beta".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: Vec::new(),
                selected_target_ref: None,
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("mismatched mission ids should be rejected");
        assert!(error.to_string().contains("mission id mismatch"));
    }

    #[test]
    fn selection_resolution_requires_explicit_consumption() {
        let temp = TempDir::new().expect("temp dir");
        let ralph_root = temp.path().join(".ralph");
        let opened = open_selection_wait(
            &ralph_root,
            &SelectionStateInput {
                candidate_mission_ids: vec![
                    "mission_alpha".to_string(),
                    "mission_beta".to_string(),
                ],
                canonical_selection_request: "Select the mission to resume.".to_string(),
            },
        )
        .expect("selection wait should open");

        let resolved = resolve_selection_wait(
            &ralph_root,
            &SelectionResolutionInput {
                selected_mission_id: "mission_alpha".to_string(),
            },
        )
        .expect("selection should resolve");
        assert_eq!(
            resolved.selected_mission_id.as_deref(),
            Some("mission_alpha")
        );
        assert!(resolved.resolved_at.is_some());
        assert!(resolved.cleared_at.is_none());

        let consumed = consume_selection_wait(
            &ralph_root,
            &SelectionConsumptionInput {
                selection_request_id: opened.selection_request_id,
            },
        )
        .expect("selection should consume");
        assert!(consumed.cleared_at.is_some());
    }

    #[test]
    fn resolve_resume_clears_selection_state_missing_resolved_at() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let selection = open_selection_wait(
            &repo_root.join(".ralph"),
            &SelectionStateInput {
                candidate_mission_ids: vec![
                    "mission_alpha".to_string(),
                    "mission_beta".to_string(),
                ],
                canonical_selection_request: "Select the mission to resume.".to_string(),
            },
        )
        .expect("selection wait should open");
        let mut corrupted = selection.clone();
        corrupted.selected_mission_id = Some("mission_alpha".to_string());
        corrupted.resolved_at = None;
        write_json(repo_root.join(".ralph/selection-state.json"), &corrupted)
            .expect("rewrite corrupted selection state");

        let report = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: None,
                live_child_lanes: Vec::new(),
            },
        )
        .expect("resume should clear the invalid selection state and re-resolve");
        assert_eq!(
            report.selection_outcome,
            SelectionOutcome::AutoBoundSingleCandidate
        );
        assert!(
            report
                .state_repairs_applied
                .iter()
                .any(|repair| repair == "cleared_selection_state_missing_resolved_at")
        );

        let preserved: super::SelectionState =
            super::load_json(repo_root.join(".ralph/selection-state.json"))
                .expect("selection state should still parse");
        assert_eq!(
            preserved.selected_mission_id.as_deref(),
            Some("mission_alpha")
        );
        assert!(preserved.cleared_at.is_some());
    }

    #[test]
    fn selection_wait_rejects_impossible_candidate_sets() {
        let temp = TempDir::new().expect("temp dir");
        let error = open_selection_wait(
            &temp.path().join(".ralph"),
            &SelectionStateInput {
                candidate_mission_ids: vec!["mission_alpha".to_string()],
                canonical_selection_request: "Select the mission to resume.".to_string(),
            },
        )
        .expect_err("selection wait should reject singleton candidates");

        assert!(
            error
                .to_string()
                .contains("at least two distinct candidate missions")
        );
    }

    #[test]
    fn execution_package_fails_for_unsatisfied_dependencies_and_target_drift() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("mission:mission_alpha".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["backend"],
                        None,
                        &["tests pass"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &["spec_api"],
                        &["src/ui"],
                        &["src/ui"],
                        &["frontend"],
                        None,
                        &["tests pass"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_ui".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: false,
                    detail: "current lock is stale".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["tests pass".to_string()],
                review_obligations: vec!["spec review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");

        assert_eq!(package.status, super::ExecutionPackageStatus::Failed);
        assert!(
            package
                .validation_failures
                .iter()
                .any(|finding| finding == "dependency_unsatisfied:lock_current")
        );
        assert!(
            package
                .validation_failures
                .iter()
                .any(|finding| finding == "target_spec_missing_from_package:spec_api")
        );
    }

    #[test]
    fn passed_package_marks_included_specs_as_packaged() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        assert_eq!(package.status, super::ExecutionPackageStatus::Passed);

        let spec_doc =
            super::load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file("spec_api"))
                .expect("load spec after packaging");
        assert_eq!(
            spec_doc.frontmatter.execution_status,
            SpecExecutionStatus::Packaged
        );
    }

    #[test]
    fn resolve_resume_consumes_resolved_selection_and_reconciles_child_lanes() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let selection = open_selection_wait(
            &repo_root.join(".ralph"),
            &SelectionStateInput {
                candidate_mission_ids: vec![
                    "mission_alpha".to_string(),
                    "mission_beta".to_string(),
                ],
                canonical_selection_request: "Select the mission to resume.".to_string(),
            },
        )
        .expect("selection wait should open");
        resolve_selection_wait(
            &repo_root.join(".ralph"),
            &SelectionResolutionInput {
                selected_mission_id: "mission_alpha".to_string(),
            },
        )
        .expect("selection should resolve");

        let interrupted_cycle = ActiveCycleState::new(
            "cycle-interrupted".to_string(),
            "mission_alpha".to_string(),
            "execute".to_string(),
            Some("spec:alpha".to_string()),
            vec![ChildLaneExpectation {
                task_path: "/root/specdrafter1".to_string(),
                lane_kind: "spec_writer".to_string(),
                expected_deliverable_ref: "spec:alpha".to_string(),
                lane_role: None,
                integration_status: ChildLaneIntegrationStatus::Pending,
                target_ref: Some("spec:alpha".to_string()),
            }],
        );
        std::fs::write(
            paths.active_cycle(),
            serde_json::to_vec_pretty(&interrupted_cycle).expect("serialize active cycle"),
        )
        .expect("write interrupted cycle");

        let report = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: None,
                live_child_lanes: vec![super::LiveChildLaneSnapshot {
                    task_path: "/root/specdrafter1".to_string(),
                    status: super::LiveChildLaneStatus::FinalSuccess,
                }],
            },
        )
        .expect("resume resolution should work");

        assert_eq!(report.selected_mission_id.as_deref(), Some("mission_alpha"));
        assert_eq!(
            report.selection_outcome,
            SelectionOutcome::ConsumedResolvedSelection
        );
        assert_eq!(
            report.selection_state_action,
            SelectionStateAction::Consumed
        );
        assert!(
            report
                .state_repairs_applied
                .iter()
                .any(|repair| repair == "consumed_resolved_selection_state")
        );

        let reconciliation = report
            .child_reconciliation
            .expect("child reconciliation should exist");
        assert_eq!(reconciliation.entries.len(), 1);
        assert_eq!(
            reconciliation.entries[0].expected_deliverable_ref,
            "spec:alpha"
        );
        assert_eq!(
            reconciliation.entries[0].classification,
            super::ChildLaneReconciliationClass::FinalSuccessUnintegrated
        );

        let consumed_state: super::SelectionState = serde_json::from_slice(
            &std::fs::read(repo_root.join(".ralph/selection-state.json"))
                .expect("selection state should exist"),
        )
        .expect("selection state should parse");
        assert_eq!(
            consumed_state.selection_request_id,
            selection.selection_request_id
        );
        assert!(consumed_state.cleared_at.is_some());
    }

    #[test]
    fn resolve_resume_preserves_resolved_selection_when_bind_fails() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let ralph_root = repo_root.join(".ralph");
        let selection = open_selection_wait(
            &ralph_root,
            &SelectionStateInput {
                candidate_mission_ids: vec![
                    "mission_alpha".to_string(),
                    "mission_beta".to_string(),
                ],
                canonical_selection_request: "Select the mission to resume.".to_string(),
            },
        )
        .expect("selection wait should open");
        resolve_selection_wait(
            &ralph_root,
            &SelectionResolutionInput {
                selected_mission_id: "mission_alpha".to_string(),
            },
        )
        .expect("selection should resolve");

        let error = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: None,
                live_child_lanes: Vec::new(),
            },
        )
        .expect_err("resume should fail because the selected mission has no closeouts");
        assert!(
            error
                .to_string()
                .contains("has no valid closeouts to resume")
        );

        let preserved_state: super::SelectionState = serde_json::from_slice(
            &std::fs::read(repo_root.join(".ralph/selection-state.json"))
                .expect("selection state should remain on disk"),
        )
        .expect("selection state should parse");
        assert_eq!(
            preserved_state.selection_request_id,
            selection.selection_request_id
        );
        assert_eq!(
            preserved_state.selected_mission_id.as_deref(),
            Some("mission_alpha")
        );
        assert!(preserved_state.cleared_at.is_none());
    }

    #[test]
    fn explicit_mission_override_preserves_selection_when_bind_fails() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let ralph_root = repo_root.join(".ralph");
        let selection = open_selection_wait(
            &ralph_root,
            &SelectionStateInput {
                candidate_mission_ids: vec![
                    "mission_alpha".to_string(),
                    "mission_beta".to_string(),
                ],
                canonical_selection_request: "Select the mission to resume.".to_string(),
            },
        )
        .expect("selection wait should open");

        let error = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: Some("mission_alpha".to_string()),
                live_child_lanes: Vec::new(),
            },
        )
        .expect_err("override should fail because the mission has no closeouts");
        assert!(
            error
                .to_string()
                .contains("has no valid closeouts to resume")
        );

        let preserved_state: super::SelectionState = serde_json::from_slice(
            &std::fs::read(repo_root.join(".ralph/selection-state.json"))
                .expect("selection state should remain on disk"),
        )
        .expect("selection state should parse");
        assert_eq!(
            preserved_state.selection_request_id,
            selection.selection_request_id
        );
        assert!(preserved_state.cleared_at.is_none());
    }

    #[test]
    fn stale_selection_wait_stays_open_until_user_chooses() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::WaitingUser),
                lock_status: Some(LockStatus::Draft),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: Some(super::WaitingRequest {
                    waiting_for: "human_decision".to_string(),
                    canonical_request: "Choose the only live mission.".to_string(),
                    resume_condition: "User answers.".to_string(),
                }),
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        let selection = open_selection_wait(
            &repo_root.join(".ralph"),
            &SelectionStateInput {
                candidate_mission_ids: vec![
                    "mission_alpha".to_string(),
                    "mission_beta".to_string(),
                ],
                canonical_selection_request: "Select the mission to resume.".to_string(),
            },
        )
        .expect("selection wait should open");

        let report = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: None,
                live_child_lanes: Vec::new(),
            },
        )
        .expect("resume resolution should work");
        assert_eq!(
            report.selection_outcome,
            SelectionOutcome::AutoBoundSingleCandidate
        );
        assert_eq!(
            report.selection_state_action,
            SelectionStateAction::Superseded
        );
        assert_eq!(report.selected_mission_id.as_deref(), Some("mission_alpha"));

        let preserved_state: super::SelectionState = serde_json::from_slice(
            &std::fs::read(repo_root.join(".ralph/selection-state.json"))
                .expect("selection state should remain on disk"),
        )
        .expect("selection state should parse");
        assert_eq!(
            preserved_state.selection_request_id,
            selection.selection_request_id
        );
        assert!(preserved_state.cleared_at.is_some());
    }

    #[test]
    fn spec_review_bundle_must_cover_packaged_proof_and_review_contracts() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["snapshot".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        let mut tampered: super::ReviewBundle =
            super::load_json(&paths.review_bundle(&bundle.bundle_id)).expect("load review bundle");
        tampered.proof_rows_under_review = vec!["snapshot".to_string()];
        tampered.mandatory_review_lenses = vec!["correctness".to_string()];
        super::write_json(paths.review_bundle(&bundle.bundle_id), &tampered)
            .expect("rewrite tampered bundle");

        let validation = validate_review_bundle(&paths, &bundle.bundle_id)
            .expect("bundle validation should work");
        assert!(!validation.valid);
        assert!(
            validation
                .findings
                .iter()
                .any(|finding| finding == "proof_obligation_missing_from_review:cargo test")
        );
        assert!(
            validation
                .findings
                .iter()
                .any(|finding| finding == "review_obligation_missing_from_bundle:security_review")
        );
    }

    #[test]
    fn validation_rejects_tampered_package_and_bundle_mission_identity() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");

        let mut tampered_package: ExecutionPackage =
            super::load_json(paths.execution_package(&package.package_id)).expect("load package");
        tampered_package.mission_id = "mission_beta".to_string();
        write_json(
            paths.execution_package(&package.package_id),
            &tampered_package,
        )
        .expect("rewrite tampered package");

        let validation = validate_execution_package(&paths, &package.package_id)
            .expect("package validation should work");
        assert!(!validation.valid);
        assert!(
            validation
                .findings
                .iter()
                .any(|finding| finding == "package_mission_id_mismatch")
        );

        write_json(paths.execution_package(&package.package_id), &package)
            .expect("restore package");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        let mut tampered_bundle: super::ReviewBundle =
            super::load_json(paths.review_bundle(&bundle.bundle_id)).expect("load bundle");
        tampered_bundle.mission_id = "mission_beta".to_string();
        write_json(paths.review_bundle(&bundle.bundle_id), &tampered_bundle)
            .expect("rewrite tampered bundle");

        let bundle_validation = validate_review_bundle(&paths, &bundle.bundle_id)
            .expect("bundle validation should work");
        assert!(!bundle_validation.valid);
        assert!(
            bundle_validation
                .findings
                .iter()
                .any(|finding| finding == "bundle_mission_id_mismatch")
        );
    }

    #[test]
    fn unresolved_contradictions_block_mission_close_review_recording() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let spec_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("spec review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: spec_bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "spec-review-clean",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &spec_bundle.bundle_id),
            },
        )
        .expect("clean spec review should record");

        let mission_close_package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("post-review package compilation should work");
        let post_completion_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: mission_close_package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string(), "review clean".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("post-completion spec review bundle should compile");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: post_completion_bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "post-completion-review-clean",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(
                    &paths,
                    &post_completion_bundle.bundle_id,
                ),
            },
        )
        .expect("post-completion review should record");
        let mission_close_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: mission_close_package.package_id.clone(),
                bundle_kind: BundleKind::MissionClose,
                mandatory_review_lenses: vec![
                    "spec_conformance".to_string(),
                    "correctness".to_string(),
                    "interface_compatibility".to_string(),
                    "safety_security_policy".to_string(),
                    "operability_rollback_observability".to_string(),
                    "evidence_adequacy".to_string(),
                ],
                target_spec_id: None,
                proof_rows_under_review: Vec::new(),
                receipts: Vec::new(),
                changed_files_or_diff: Vec::new(),
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: vec!["cargo test".to_string()],
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: vec![
                    paths.outcome_lock().display().to_string(),
                    paths.program_blueprint().display().to_string(),
                    paths.review_ledger().display().to_string(),
                ],
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("mission-close bundle compilation should work");
        append_contradiction(
            &paths,
            &ContradictionInput {
                mission_id: "mission_alpha".to_string(),
                discovered_in_phase: "review".to_string(),
                discovered_by: "codex".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                evidence_refs: reviewer_output_evidence(
                    "mission-close-unresolved-contradiction",
                    &["RECEIPTS/test.txt"],
                ),
                violated_assumption_or_contract: "Blueprint truth changed under review."
                    .to_string(),
                suggested_reopen_layer: ReopenLayer::Blueprint,
                reason_code: "review_contract_changed".to_string(),
                governing_revision: "package:test".to_string(),
                status: Some(ContradictionStatus::AcceptedForReplan),
                triage_decision: Some(TriageDecision::ReopenBlueprint),
                triaged_by: Some("codex".to_string()),
                machine_action: Some(MachineAction::ForceReplan),
                next_required_branch: Some(NextRequiredBranch::Replan),
                resolution_ref: None,
            },
        )
        .expect("contradiction should record");

        let error = record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: mission_close_bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: None,
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "mission-close-unresolved-contradiction",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(
                    &paths,
                    &mission_close_bundle.bundle_id,
                ),
            },
        )
        .expect_err("mission close review should fail while contradictions remain open");
        assert!(
            error
                .to_string()
                .contains("mission_close_unresolved_contradiction")
        );
    }

    #[test]
    fn resolve_resume_detects_spec_fingerprint_drift_after_clean_review() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "resume-drift-clean-review",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect("review result should record");

        let mut spec_contents =
            std::fs::read_to_string(paths.spec_file("spec_api")).expect("read spec file");
        spec_contents.push_str("\n## Drift\n\n- Changed after review without a new revision.\n");
        std::fs::write(paths.spec_file("spec_api"), spec_contents).expect("mutate spec body");

        let report = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: Some("mission_alpha".to_string()),
                live_child_lanes: Vec::new(),
            },
        )
        .expect("resume resolution should work");
        assert_eq!(
            report.resume_status,
            super::ResumeStatus::ContradictoryState
        );
        assert!(
            report
                .next_action
                .contains("governing_spec_fingerprint_drift:spec_api")
        );

        let state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        assert_eq!(state.verdict, Verdict::ReplanRequired);
        assert_eq!(
            state.reason_code.as_deref(),
            Some("governing_artifact_drift")
        );
        assert_eq!(state.next_phase.as_deref(), Some("replan"));
    }

    #[test]
    fn mission_target_repackaging_stales_included_spec_review_gates() {
        let mut gates = MissionGateIndex {
            mission_id: "mission_alpha".to_string(),
            current_phase: "review".to_string(),
            updated_at: OffsetDateTime::now_utc(),
            gates: vec![
                MissionGateRecord {
                    gate_id: "review-a".to_string(),
                    gate_kind: GateKind::BlockingReview,
                    target_ref: "spec:spec_a".to_string(),
                    governing_refs: Vec::new(),
                    status: MissionGateStatus::Passed,
                    blocking: true,
                    opened_at: OffsetDateTime::now_utc(),
                    evaluated_at: None,
                    evaluated_against_ref: None,
                    evidence_refs: Vec::new(),
                    failure_refs: Vec::new(),
                    superseded_by: None,
                },
                MissionGateRecord {
                    gate_id: "review-b".to_string(),
                    gate_kind: GateKind::BlockingReview,
                    target_ref: "spec:spec_b".to_string(),
                    governing_refs: Vec::new(),
                    status: MissionGateStatus::Passed,
                    blocking: true,
                    opened_at: OffsetDateTime::now_utc(),
                    evaluated_at: None,
                    evaluated_against_ref: None,
                    evidence_refs: Vec::new(),
                    failure_refs: Vec::new(),
                    superseded_by: None,
                },
            ],
        };
        let package = ExecutionPackage {
            package_id: "pkg-1".to_string(),
            mission_id: "mission_alpha".to_string(),
            target_type: TargetType::Mission,
            target_id: "mission_alpha".to_string(),
            lock_revision: 1,
            lock_fingerprint: Fingerprint::from_bytes(b"lock"),
            blueprint_revision: 1,
            blueprint_fingerprint: Fingerprint::from_bytes(b"blueprint"),
            dependency_snapshot_fingerprint: Fingerprint::from_bytes(b"deps"),
            wave_fingerprint: None,
            included_specs: vec![
                IncludedSpecRef {
                    spec_id: "spec_a".to_string(),
                    spec_revision: 1,
                    spec_fingerprint: Fingerprint::from_bytes(b"spec-a"),
                },
                IncludedSpecRef {
                    spec_id: "spec_b".to_string(),
                    spec_revision: 1,
                    spec_fingerprint: Fingerprint::from_bytes(b"spec-b"),
                },
            ],
            dependency_satisfaction_state: Vec::new(),
            read_scope: Vec::new(),
            write_scope: Vec::new(),
            proof_obligations: Vec::new(),
            review_obligations: Vec::new(),
            replan_boundary: Default::default(),
            wave_context: None,
            wave_specs: Vec::new(),
            gate_checks: Vec::new(),
            validation_failures: Vec::new(),
            validated_at: OffsetDateTime::now_utc(),
            status: super::ExecutionPackageStatus::Passed,
        };

        invalidate_review_history_for_execution_target(&mut gates, &package);

        assert!(
            gates
                .gates
                .iter()
                .all(|gate| gate.status == MissionGateStatus::Stale)
        );
    }

    #[test]
    fn validate_execution_package_tolerates_completion_only_spec_fingerprint_drift() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the API slice",
                            &["src/api"],
                            &["src/api"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the UI slice",
                            &["src/ui"],
                            &["src/ui"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("mission:mission_alpha".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["backend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p api"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &[],
                        &["src/ui"],
                        &["src/ui"],
                        &["frontend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p ui"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Mission,
                target_id: "mission_alpha".to_string(),
                included_spec_ids: vec!["spec_api".to_string(), "spec_ui".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string(), "src/ui".to_string()],
                write_scope: vec!["src/api".to_string(), "src/ui".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        assert_eq!(package.status, ExecutionPackageStatus::Passed);

        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test -p api".to_string()],
                receipts: vec!["RECEIPTS/test-api.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("first review bundle should compile");
        let first_bundle_id = bundle.bundle_id.clone();
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: first_bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "mission-repackage-spec-api",
                    &["RECEIPTS/test-api.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::Execution),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &first_bundle_id),
            },
        )
        .expect("first clean review should record");

        let package_validation =
            validate_execution_package(&paths, &package.package_id).expect("validate package");
        assert_eq!(
            package_validation.valid, true,
            "{:?}",
            package_validation.findings
        );
        let first_bundle_validation =
            validate_review_bundle(&paths, &first_bundle_id).expect("validate first bundle");
        assert_eq!(first_bundle_validation.valid, true);

        let second_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id,
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_ui".to_string()),
                proof_rows_under_review: vec!["cargo test -p ui".to_string()],
                receipts: vec!["RECEIPTS/test-ui.txt".to_string()],
                changed_files_or_diff: vec!["src/ui/mod.rs".to_string()],
                touched_interface_contracts: vec!["UiSurface".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("second review bundle should still compile from the same package");
        assert_eq!(second_bundle.target_spec_id.as_deref(), Some("spec_ui"));
    }

    #[test]
    fn planning_write_rejects_impossible_spec_state_combinations() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Impossible spec".to_string(),
                    body_markdown: None,
                    artifact_status: Some(SpecArtifactStatus::Draft),
                    packetization_status: Some(PacketizationStatus::NearFrontier),
                    execution_status: Some(SpecExecutionStatus::Packaged),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("impossible planning spec state should fail");
        assert!(error.to_string().contains("invalid state combination"));
    }

    #[test]
    fn review_results_require_non_empty_reviewer_identity() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        let error = record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "   ".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "recompile-clean-review",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect_err("empty reviewer identity should fail");
        assert!(error.to_string().contains("non-empty reviewer identity"));

        let missing_snapshot_error = record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "missing-core-snapshot",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: None,
            },
        )
        .expect_err("missing core review truth snapshot should fail");
        assert!(
            missing_snapshot_error
                .to_string()
                .contains("requires review_truth_snapshot")
        );
    }

    #[test]
    fn recompiling_after_clean_review_resets_spec_to_packaged() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "recompile-clean-review",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect("review result should record");
        let reviewed_spec =
            load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file("spec_api"))
                .expect("load reviewed spec");
        assert_eq!(
            reviewed_spec.frontmatter.execution_status,
            SpecExecutionStatus::Complete
        );

        compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("repackaging should work");
        let repackaged_spec =
            load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file("spec_api"))
                .expect("load repackaged spec");
        assert_eq!(
            repackaged_spec.frontmatter.execution_status,
            SpecExecutionStatus::Packaged
        );
    }

    #[test]
    fn write_closeout_failure_does_not_leave_transient_active_cycle() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_closeout(
            &paths,
            CloseoutRecord {
                closeout_id: Some("closeout-test".to_string()),
                closeout_seq: 0,
                mission_id: "mission_alpha".to_string(),
                phase: "review".to_string(),
                activity: "broken_needs_user".to_string(),
                verdict: Verdict::NeedsUser,
                terminality: Terminality::WaitingNonTerminal,
                resume_mode: ResumeMode::YieldToUser,
                next_phase: Some("review".to_string()),
                next_action: "Need user input.".to_string(),
                target: Some("mission:mission_alpha".to_string()),
                cycle_kind: Some(super::CycleKind::WaitingHandshake),
                lock_revision: Some(1),
                lock_fingerprint: Some(Fingerprint::from_bytes(b"lock").to_string()),
                blueprint_revision: None,
                blueprint_fingerprint: None,
                governing_revision: Some("lock:1".to_string()),
                reason_code: Some("broken_needs_user".to_string()),
                summary: Some("This should fail validation.".to_string()),
                continuation_prompt: Some("Need user input.".to_string()),
                cycle_id: None,
                waiting_request_id: None,
                waiting_for: None,
                canonical_waiting_request: None,
                resume_condition: None,
                request_emitted_at: None,
                active_child_task_paths: Vec::new(),
                artifact_fingerprints: BTreeMap::from([(
                    "mission-state".to_string(),
                    Fingerprint::from_bytes(b"mission-state-fp").to_string(),
                )]),
            },
        )
        .expect_err("invalid closeout should fail");
        assert!(error.to_string().contains("needs_user closeout is missing"));
        assert!(!paths.active_cycle().exists());
    }

    #[test]
    fn acknowledge_waiting_request_uses_latest_closeout_instead_of_stale_cached_state() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::WaitingUser),
                lock_status: None,
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: Some(WaitingRequest {
                    waiting_for: "human_decision".to_string(),
                    canonical_request: "Choose the rollout posture.".to_string(),
                    resume_condition: "The user chooses the rollout posture.".to_string(),
                }),
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let mut stale_state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        stale_state.request_emitted_at = Some("2026-04-14T00:00:00Z".to_string());
        write_json(paths.state_json(), &stale_state).expect("rewrite stale state");

        let closeout = acknowledge_waiting_request(
            &paths,
            &WaitingRequestAcknowledgementInput {
                waiting_request_id: stale_state
                    .waiting_request_id
                    .clone()
                    .expect("waiting request id"),
            },
        )
        .expect("acknowledgement should use latest closeout truth");
        assert!(closeout.request_emitted_at.is_some());
        assert_eq!(closeout.closeout_seq, 2);
    }

    #[test]
    fn clean_review_can_route_directly_to_mission_close() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        let pending_review_state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist after opening review");
        assert_eq!(pending_review_state.verdict, Verdict::ReviewRequired);
        assert_eq!(pending_review_state.next_phase.as_deref(), Some("review"));

        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "execution-graph-obligation",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect("review result should record");

        let state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        assert_eq!(state.next_phase.as_deref(), Some("mission_close"));
        assert!(state.next_action.contains("mission-close"));

        let report = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: Some("mission_alpha".to_string()),
                live_child_lanes: Vec::new(),
            },
        )
        .expect("resume resolution should work");
        assert_eq!(report.resume_status, ResumeStatus::ActionableNonTerminal);
    }

    #[test]
    fn clean_spec_review_satisfies_matching_execution_graph_obligations() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the API slice",
                            &["src/api"],
                            &["src/api"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::Complete),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the UI slice",
                            &["crates/codex1"],
                            &["crates/codex1"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::Complete),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["runtime"],
                        None,
                        &["cargo test"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &[],
                        &["crates/codex1"],
                        &["crates/codex1"],
                        &["frontend"],
                        None,
                        &["ui test"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");

        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "non-clean-review-branch",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::Execution),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect("review result should record");

        let execution_graph = load_execution_graph(&paths)
            .expect("load execution graph")
            .expect("execution graph should exist");
        let api_obligation = execution_graph
            .obligations
            .iter()
            .find(|obligation| obligation.target_spec_id == "spec_api")
            .expect("api obligation should exist");
        assert_eq!(
            api_obligation.status,
            ExecutionGraphObligationStatus::Satisfied
        );
        assert!(
            api_obligation
                .satisfied_by
                .iter()
                .any(|value: &String| value.starts_with("gate:")),
            "clean review should discharge the matching execution-graph obligation"
        );

        let ui_obligation = execution_graph
            .obligations
            .iter()
            .find(|obligation| obligation.target_spec_id == "spec_ui")
            .expect("ui obligation should exist");
        assert_eq!(ui_obligation.status, ExecutionGraphObligationStatus::Open);
    }

    #[test]
    fn compile_execution_package_accepts_spec_dependency_governing_refs() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_core".to_string(),
                        purpose: "Implement the core dependency".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the core dependency",
                            &["src/core"],
                            &["src/core"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::Complete),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the dependent UI slice".to_string(),
                        body_markdown: Some(canonical_spec_body_with_scope(
                            "Implement the dependent UI slice",
                            &["src/ui"],
                            &["src/ui"],
                        )),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("spec:spec_ui".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_core",
                        &[],
                        &["src/core"],
                        &["src/core"],
                        &["runtime"],
                        None,
                        &["cargo test -p core"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &["spec_core"],
                        &["src/ui"],
                        &["src/ui"],
                        &["frontend"],
                        None,
                        &["cargo test -p ui"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_ui".to_string(),
                included_spec_ids: vec!["spec_ui".to_string()],
                dependency_satisfaction_state: vec![
                    DependencyCheck {
                        name: "lock_current".to_string(),
                        satisfied: true,
                        detail: "current lock is ratified".to_string(),
                    },
                    DependencyCheck {
                        name: "blueprint_current".to_string(),
                        satisfied: true,
                        detail: "current blueprint selects spec_ui".to_string(),
                    },
                    DependencyCheck {
                        name: "spec:spec_core".to_string(),
                        satisfied: true,
                        detail: "spec_core is complete and review-clean".to_string(),
                    },
                ],
                read_scope: vec!["src/ui".to_string()],
                write_scope: vec!["src/ui".to_string()],
                proof_obligations: vec!["cargo test -p ui".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");

        assert_eq!(package.status, ExecutionPackageStatus::Passed);
        assert!(
            package.validation_failures.is_empty(),
            "spec dependency rows should satisfy dependency truth without weakening graph deps: {:?}",
            package.validation_failures
        );
    }

    #[test]
    fn mission_close_review_requires_satisfied_execution_graph_obligations() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Complete),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("mission:mission_alpha".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["runtime"],
                        None,
                        &["cargo test"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Mission,
                target_id: "mission_alpha".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["mission close".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");

        let mission_close_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::MissionClose,
                mandatory_review_lenses: vec![
                    "spec_conformance".to_string(),
                    "correctness".to_string(),
                    "evidence_adequacy".to_string(),
                ],
                target_spec_id: None,
                proof_rows_under_review: Vec::new(),
                receipts: Vec::new(),
                changed_files_or_diff: Vec::new(),
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: vec!["cargo test".to_string()],
                cross_spec_claim_refs: vec!["claim:default-proof".to_string()],
                visible_artifact_refs: vec![
                    paths.outcome_lock().display().to_string(),
                    paths.program_blueprint().display().to_string(),
                    paths.review_ledger().display().to_string(),
                ],
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("mission-close bundle compilation should work");

        let validation = validate_review_bundle(&paths, &mission_close_bundle.bundle_id)
            .expect("mission-close bundle validation should work");
        assert!(
            !validation.valid,
            "mission-close should stay invalid while blocking graph obligations are still open"
        );
        assert!(
            validation.findings.iter().any(|finding| {
                finding == "mission_close_obligation_not_satisfied:spec_api:cargo test:Open"
            }),
            "validation should surface the unsatisfied execution-graph obligation"
        );
    }

    #[test]
    fn execution_graph_contract_input_ignores_dynamic_obligation_state() {
        let base_graph = ExecutionGraph {
            mission_id: "mission_alpha".to_string(),
            blueprint_revision: 1,
            blueprint_fingerprint: Fingerprint::from_bytes(b"blueprint"),
            nodes: vec![ExecutionGraphNode {
                spec_id: "spec_api".to_string(),
                spec_revision: 1,
                spec_fingerprint: Fingerprint::from_bytes(b"spec_api"),
                depends_on: Vec::new(),
                produces: vec!["artifact:spec_api".to_string()],
                read_paths: vec!["src/api".to_string()],
                write_paths: vec!["src/api".to_string()],
                exclusive_resources: Vec::new(),
                coupling_tags: Vec::new(),
                ownership_domains: vec!["backend".to_string()],
                risk_class: None,
                acceptance_checks: vec!["cargo test".to_string()],
                evidence_type: "receipt".to_string(),
            }],
            obligations: vec![ExecutionGraphObligation {
                obligation_id: "spec_api:cargo test".to_string(),
                kind: ExecutionGraphObligationKind::Validation,
                target_spec_id: "spec_api".to_string(),
                target_spec_revision: 1,
                target_spec_fingerprint: Fingerprint::from_bytes(b"spec_api"),
                discharges_claim_ref: "claim:default-proof".to_string(),
                proof_rows: vec!["cargo test".to_string()],
                acceptance_checks: vec!["cargo test".to_string()],
                required_evidence: vec!["RECEIPTS/test.txt".to_string()],
                review_lenses: vec!["correctness".to_string()],
                blocking: true,
                status: ExecutionGraphObligationStatus::Open,
                satisfied_by: Vec::new(),
                evidence_refs: Vec::new(),
            }],
            generated_at: OffsetDateTime::now_utc(),
        };

        let mut satisfied_graph = base_graph.clone();
        let obligation = satisfied_graph
            .obligations
            .first_mut()
            .expect("obligation should exist");
        obligation.status = ExecutionGraphObligationStatus::Satisfied;
        obligation.satisfied_by = vec!["gate:review".to_string(), "bundle:abc".to_string()];
        obligation.evidence_refs = vec!["RECEIPTS/test.txt".to_string()];

        assert_eq!(
            execution_graph_to_contract_input(&base_graph),
            execution_graph_to_contract_input(&satisfied_graph),
            "dynamic execution-graph obligation state should not change the blueprint contract input"
        );
    }

    #[test]
    fn spec_review_can_validate_without_interface_contract_changes() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        let validation = validate_review_bundle(&paths, &bundle.bundle_id)
            .expect("review bundle validation should work");
        assert!(validation.valid, "{:?}", validation.findings);
    }

    #[test]
    fn open_review_gate_forces_resume_into_review_after_interrupted_bundle_compile() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let _bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        let mut closeouts = load_closeouts(&paths.closeouts_ndjson()).expect("load closeouts");
        closeouts.pop();
        let rendered = closeouts
            .into_iter()
            .map(|record| serde_json::to_string(&record).expect("serialize closeout"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(
            paths.closeouts_ndjson(),
            if rendered.is_empty() {
                String::new()
            } else {
                format!("{rendered}\n")
            },
        )
        .expect("rewrite closeouts");
        let rebuilt_state = rebuild_state_from_closeouts(
            &load_closeouts(&paths.closeouts_ndjson()).expect("reload closeouts"),
            None,
        )
        .expect("rebuild state");
        write_json(paths.state_json(), &rebuilt_state).expect("rewrite state");

        let report = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: Some("mission_alpha".to_string()),
                live_child_lanes: Vec::new(),
            },
        )
        .expect("resume resolution should work");
        assert_eq!(report.resume_status, ResumeStatus::ActionableNonTerminal);
        assert_eq!(report.next_phase.as_deref(), Some("review"));
        assert!(report.next_action.contains("review gate"));
    }

    #[test]
    fn non_clean_review_with_review_branch_stays_in_review_phase() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "blocked".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "mission-close-spec-clean",
                    &["RECEIPTS/test.txt"],
                ),
                findings: vec![ReviewFindingInput {
                    class: "B-Spec".to_string(),
                    summary: "Need another review pass".to_string(),
                    blocking: true,
                    evidence_refs: reviewer_output_evidence(
                        "non-clean-review-branch-finding",
                        &["RECEIPTS/test.txt"],
                    ),
                    disposition: "keep review gate open".to_string(),
                }],
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::Review),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect("review result should record");
        let state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        assert_eq!(state.verdict, Verdict::ReviewRequired);
        assert_eq!(state.next_phase.as_deref(), Some("review"));
    }

    #[test]
    fn clean_review_can_yield_to_needs_user() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "clean-review-needs-user",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::NeedsUser),
                waiting_request: Some(WaitingRequest {
                    waiting_for: "user_decision".to_string(),
                    canonical_request: "Choose the rollout posture.".to_string(),
                    resume_condition: "The user chooses the rollout posture.".to_string(),
                }),
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect("review result should record");
        let state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        assert_eq!(state.verdict, Verdict::NeedsUser);
        assert_eq!(state.next_phase.as_deref(), Some("review"));
        assert_eq!(
            state.canonical_waiting_request.as_deref(),
            Some("Choose the rollout posture.")
        );
    }

    #[test]
    fn review_result_rejects_noncanonical_finding_classes() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");

        let error = record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "blocked".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: vec!["RECEIPTS/test.txt".to_string()],
                findings: vec![ReviewFindingInput {
                    class: "custom".to_string(),
                    summary: "Non-canonical review class".to_string(),
                    blocking: true,
                    evidence_refs: vec!["RECEIPTS/test.txt".to_string()],
                    disposition: "repair".to_string(),
                }],
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::Review),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect_err("non-canonical review classes should be rejected");
        assert!(
            error
                .to_string()
                .contains("review finding class custom is not allowed")
        );
    }

    #[test]
    fn mission_close_respects_non_clean_reviewer_verdict_and_descoped_specs() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: Some(
                            "# Workstream Spec\n\n## Purpose\n\nImplement the API slice\n\n## In Scope\n\n- Execute the bounded integration slice.\n\n## Out Of Scope\n\n- Unrelated repo changes.\n\n## Dependencies\n\n- Outcome Lock and Program Blueprint stay current.\n\n## Touched Surfaces\n\n- Runtime backend.\n\n## Read Scope\n\n- src/api\n\n## Write Scope\n\n- src/api\n\n## Interfaces And Contracts Touched\n\n- internal command JSON contract\n\n## Implementation Shape\n\nKeep the workstream bounded and reviewable.\n\n## Proof-Of-Completion Expectations\n\n- cargo test -p api\n\n## Non-Breakage Expectations\n\n- Existing mission contracts still validate.\n\n## Review Lenses\n\n- correctness\n\n## Replan Boundary\n\n- Reopen planning on scope expansion.\n\n## Truth Basis Refs\n\n- PROGRAM-BLUEPRINT.md\n\n## Freshness Notes\n\n- Current for the integration test.\n\n## Support Files\n\n- `REVIEW.md`\n".to_string(),
                        ),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_descoped".to_string(),
                        purpose: "Descoped follow-on work".to_string(),
                        body_markdown: Some(canonical_spec_body("Descoped follow-on work")),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Descoped),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["backend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p api"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let spec_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("spec review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: spec_bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "non-clean-review-branch",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &spec_bundle.bundle_id),
            },
        )
        .expect("clean spec review should record");

        let mission_close_package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("post-review package compilation should work");
        let post_completion_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: mission_close_package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string(), "review clean".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("post-completion review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: post_completion_bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "mission-close-post-completion-clean",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(
                    &paths,
                    &post_completion_bundle.bundle_id,
                ),
            },
        )
        .expect("post-completion review should record");
        let mission_close_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: mission_close_package.package_id.clone(),
                bundle_kind: BundleKind::MissionClose,
                mandatory_review_lenses: vec![
                    "spec_conformance".to_string(),
                    "correctness".to_string(),
                    "interface_compatibility".to_string(),
                    "safety_security_policy".to_string(),
                    "operability_rollback_observability".to_string(),
                    "evidence_adequacy".to_string(),
                ],
                target_spec_id: None,
                proof_rows_under_review: Vec::new(),
                receipts: Vec::new(),
                changed_files_or_diff: Vec::new(),
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: vec!["cargo test".to_string()],
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: vec![
                    paths.outcome_lock().display().to_string(),
                    paths.program_blueprint().display().to_string(),
                    paths.review_ledger().display().to_string(),
                ],
                deferred_descoped_follow_on_refs: vec!["spec_descoped".to_string()],
                open_finding_summary: Vec::new(),
            },
        )
        .expect("mission-close bundle compilation should work");
        let validation = validate_review_bundle(&paths, &mission_close_bundle.bundle_id)
            .expect("mission-close validation should work");
        assert!(validation.valid, "{:?}", validation.findings);
        assert_eq!(
            mission_close_bundle.included_spec_refs,
            vec!["spec_api".to_string()]
        );
        let mission_close_state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        assert_eq!(mission_close_state.phase, "mission_close");
        assert_eq!(
            mission_close_state.next_phase.as_deref(),
            Some("mission_close")
        );

        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: mission_close_bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "blocked".to_string(),
                target_spec_id: None,
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "mission-close-non-clean",
                    &["RECEIPTS/test.txt"],
                ),
                findings: vec![ReviewFindingInput {
                    class: "B-Spec".to_string(),
                    summary: "Mission-close review found unresolved integrated scope drift."
                        .to_string(),
                    blocking: true,
                    evidence_refs: reviewer_output_evidence(
                        "mission-close-non-clean-finding",
                        &["RECEIPTS/test.txt"],
                    ),
                    disposition: "repair before close".to_string(),
                }],
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::Review),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(
                    &paths,
                    &mission_close_bundle.bundle_id,
                ),
            },
        )
        .expect("non-clean mission-close review should still record");

        let state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        assert_ne!(state.verdict, Verdict::Complete);
        assert_eq!(state.phase, "mission_close");
        assert_eq!(state.next_phase.as_deref(), Some("review"));
    }

    #[test]
    fn mission_close_bundle_rejects_stale_source_packages() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: None,
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Packaged),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let stale_package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["spec review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("initial package compilation should work");

        let _fresh_package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string(), "src/shared".to_string()],
                write_scope: vec!["src/api".to_string(), "src/shared".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["mission close".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("second package compilation should supersede the first");

        let stale_validation = validate_execution_package(&paths, &stale_package.package_id)
            .expect("stale package validation should work");
        assert!(
            !stale_validation.valid,
            "expected original package to become stale once superseded"
        );

        let error = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: stale_package.package_id.clone(),
                bundle_kind: BundleKind::MissionClose,
                mandatory_review_lenses: vec![
                    "spec_conformance".to_string(),
                    "correctness".to_string(),
                    "interface_compatibility".to_string(),
                    "safety_security_policy".to_string(),
                    "operability_rollback_observability".to_string(),
                    "evidence_adequacy".to_string(),
                ],
                target_spec_id: None,
                proof_rows_under_review: Vec::new(),
                receipts: Vec::new(),
                changed_files_or_diff: Vec::new(),
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: vec!["cargo test".to_string()],
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: vec![
                    paths.outcome_lock().display().to_string(),
                    paths.program_blueprint().display().to_string(),
                    paths.review_ledger().display().to_string(),
                ],
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect_err("stale packages must not power mission-close review");
        assert!(error.to_string().contains("execution package"));
        assert!(error.to_string().contains("execution_gate_status"));
    }

    #[test]
    fn mission_close_review_ignores_stale_historical_execution_packages() {
        let now = OffsetDateTime::now_utc();
        let index = MissionGateIndex {
            mission_id: "mission_alpha".to_string(),
            current_phase: "mission_close".to_string(),
            updated_at: now,
            gates: vec![
                MissionGateRecord {
                    gate_id: "mission_alpha:execution_package:spec:spec_api:old_package"
                        .to_string(),
                    gate_kind: GateKind::ExecutionPackage,
                    target_ref: "spec:spec_api".to_string(),
                    governing_refs: vec!["lock:1".to_string(), "blueprint:3".to_string()],
                    status: MissionGateStatus::Stale,
                    blocking: true,
                    opened_at: now,
                    evaluated_at: Some(now),
                    evaluated_against_ref: Some("old_package".to_string()),
                    evidence_refs: vec![".ralph/execution-packages/old_package.json".to_string()],
                    failure_refs: vec!["superseded by a newer package".to_string()],
                    superseded_by: None,
                },
                MissionGateRecord {
                    gate_id:
                        "mission_alpha:execution_package:mission:mission_alpha:current_package"
                            .to_string(),
                    gate_kind: GateKind::ExecutionPackage,
                    target_ref: "mission:mission_alpha".to_string(),
                    governing_refs: vec!["lock:1".to_string(), "blueprint:10".to_string()],
                    status: MissionGateStatus::Passed,
                    blocking: true,
                    opened_at: now,
                    evaluated_at: Some(now),
                    evaluated_against_ref: Some("current_package".to_string()),
                    evidence_refs: vec![
                        ".ralph/execution-packages/current_package.json".to_string(),
                    ],
                    failure_refs: Vec::new(),
                    superseded_by: None,
                },
            ],
        };

        assert_eq!(
            unresolved_blocking_gate_refs(&index, None),
            vec![
                "mission_alpha:execution_package:spec:spec_api:old_package:spec:spec_api"
                    .to_string()
            ]
        );
        assert_eq!(
            unresolved_blocking_gate_refs_for_source_package(&index, "current_package", None),
            Vec::<String>::new()
        );

        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Packaged),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let stale_package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["spec review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("initial package compilation should work");

        let spec_review = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: stale_package.package_id.clone(),
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: vec!["ApiService".to_string()],
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("spec review bundle should compile");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: spec_review.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "mission-close-api-clean",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &spec_review.bundle_id),
            },
        )
        .expect("clean spec review should record");

        let _fresh_package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string(), "src/shared".to_string()],
                write_scope: vec!["src/api".to_string(), "src/shared".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["mission close".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("second package compilation should supersede the first");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(2),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body_with_scope(
                        "Implement the API slice",
                        &["src/api"],
                        &["src/api"],
                    )),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Complete),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("mission:mission_alpha".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning should advance to mission close");

        let mission_close_package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Mission,
                target_id: "mission_alpha".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["mission close".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("mission package compilation should work");
        assert_eq!(
            mission_close_package.status,
            ExecutionPackageStatus::Passed,
            "{:?}",
            mission_close_package.validation_failures
        );

        let mission_close_bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: mission_close_package.package_id.clone(),
                bundle_kind: BundleKind::MissionClose,
                mandatory_review_lenses: vec![
                    "spec_conformance".to_string(),
                    "correctness".to_string(),
                    "interface_compatibility".to_string(),
                    "safety_security_policy".to_string(),
                    "operability_rollback_observability".to_string(),
                    "evidence_adequacy".to_string(),
                ],
                target_spec_id: None,
                proof_rows_under_review: Vec::new(),
                receipts: Vec::new(),
                changed_files_or_diff: Vec::new(),
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: vec!["cargo test".to_string()],
                cross_spec_claim_refs: vec!["claim:integration".to_string()],
                visible_artifact_refs: vec![
                    paths.outcome_lock().display().to_string(),
                    paths.program_blueprint().display().to_string(),
                    paths.review_ledger().display().to_string(),
                ],
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("mission-close bundle should compile");

        let mission_close_review = record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: mission_close_bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "complete".to_string(),
                target_spec_id: None,
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "mission-close-clean",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(
                    &paths,
                    &mission_close_bundle.bundle_id,
                ),
            },
        )
        .expect("mission close review should record cleanly");

        assert_eq!(mission_close_review.blocking_findings, 0);
        let state = load_state(&paths.state_json())
            .expect("load state")
            .expect("state should exist");
        assert_eq!(state.verdict, Verdict::Complete);
        assert_eq!(state.terminality, Terminality::Terminal);
    }

    #[test]
    fn derive_writer_packet_rejects_mismatched_mission_identity() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["security_review".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");

        let error = derive_writer_packet(
            &paths,
            &WriterPacketInput {
                mission_id: "mission_beta".to_string(),
                source_package_id: package.package_id,
                target_spec_id: "spec_api".to_string(),
                required_checks: vec!["cargo test".to_string()],
                review_lenses: vec!["correctness".to_string()],
                explicitly_disallowed_decisions: Vec::new(),
            },
        )
        .expect_err("writer packet should reject mismatched mission identity");
        assert!(
            error
                .to_string()
                .contains("mission id mismatch: mission paths target mission_alpha, but input requested mission_beta")
        );
    }

    #[test]
    fn approved_blueprint_requires_inventory_and_decision_log_sections() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: "# Program Blueprint\n\n## Locked Mission Reference\n\n- locked\n\n## Truth Register Summary\n\n- current\n\n## System Model\n\n- system\n\n## Invariants And Protected Behaviors\n\n- invariant\n\n## Proof Matrix\n\n- claim:default-proof\n\n## Decision Obligations\n\n- obligation:route\n\n## Selected Architecture\n\nRoute truth.\n\n## Review Bundle Design\n\n- correctness\n\n## Workstream Overview\n\n- runtime_core\n\n## Risks And Unknowns\n\n- risk\n\n## Replan Policy\n\n- reopen on contract change.\n".to_string(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("approved blueprint should require canonical planning sections");
        assert!(error.to_string().contains("In-Scope Work Inventory"));
    }

    #[test]
    fn approved_blueprint_accepts_table_based_inventory_and_decision_log_sections() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: table_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("approved blueprint tables should be accepted");
    }

    #[test]
    fn approved_blueprint_rejects_header_only_inventory_and_decision_log_tables() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: "# Program Blueprint\n\n## Locked Mission Reference\n\n- Mission id: `mission_alpha`\n\n## Truth Register Summary\n\n- Locked facts are reflected here.\n\n## System Model\n\n- Touched surfaces: API and storage.\n\n## Invariants And Protected Behaviors\n\n- Keep the locked outcome honest.\n\n## Proof Matrix\n\n- claim:default-proof\n\n## Decision Obligations\n\n- obligation:route-choice\n\n## In-Scope Work Inventory\n\n| Work item | Class | Why it exists | Finish in this mission? |\n| --- | --- | --- | --- |\n\n## Selected Architecture\n\nRoute truth.\n\n## Execution Graph and Safe-Wave Rules\n\n- Single-node routes may execute directly.\n\n## Decision Log\n\n| Decision id | Statement | Rationale | Evidence refs | Adopted in revision |\n| --- | --- | --- | --- | --- |\n\n## Review Bundle Design\n\n- Mandatory review lenses: correctness\n\n## Workstream Overview\n\n- api\n\n## Risks And Unknowns\n\n- Rollout coupling remains explicit.\n\n## Replan Policy\n\n- Reopen planning if scope or proof changes.\n".to_string(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("header-only planning tables should be rejected");

        assert!(
            error
                .to_string()
                .contains("must contain at least one meaningful planning entry")
        );
    }

    #[test]
    fn spec_body_requires_non_empty_scope_sections() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body_with_scope(
                        "Implement the API slice",
                        &[],
                        &[],
                    )),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("specs without explicit scope should be rejected");

        assert!(error.to_string().contains("Read Scope"));
    }

    #[test]
    fn spec_body_rejects_invalid_scope_paths() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body_with_scope(
                        "Implement the API slice",
                        &["src/api/../secrets"],
                        &["src/api.rs"],
                    )),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("specs with escaping scope paths should be rejected");

        assert!(error.to_string().contains("invalid scoped path"));
    }

    #[test]
    fn spec_body_rejects_placeholder_scope_paths() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body_with_scope(
                        "Implement the API slice",
                        &["{{READ_PATH_1}}"],
                        &["Fill in during planning"],
                    )),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("placeholder scope entries should be rejected");

        assert!(error.to_string().contains("invalid scoped path"));
    }

    #[test]
    fn compile_execution_package_rejects_scope_outside_spec_frontier() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src".to_string()],
                write_scope: vec!["src".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should still emit a failed package");

        assert_eq!(package.status, ExecutionPackageStatus::Failed);
        assert!(
            package
                .validation_failures
                .iter()
                .any(|finding| finding == "package_write_scope_outside_frontier:src")
        );
    }

    #[test]
    fn compile_execution_package_rejects_partial_included_spec_scope_coverage() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body_with_scope(
                        "Implement the API slice",
                        &["src/api.rs", "src/api_types.rs"],
                        &["src/api.rs", "src/api_types.rs"],
                    )),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api.rs".to_string()],
                write_scope: vec!["src/api.rs".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should still return a failed package");

        assert_eq!(package.status, ExecutionPackageStatus::Failed);
        assert!(package.validation_failures.iter().any(|finding| {
            finding == "package_read_scope_missing_for_included_spec:spec_api:src/api_types.rs"
        }));
        assert!(package.validation_failures.iter().any(|finding| {
            finding == "package_write_scope_missing_for_included_spec:spec_api:src/api_types.rs"
        }));
    }

    #[test]
    fn compile_execution_package_rejects_invalid_package_scope_paths() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api/../secrets".to_string()],
                write_scope: vec!["src/api.rs".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should still return a failed package");

        assert_eq!(package.status, ExecutionPackageStatus::Failed);
        assert!(
            package
                .validation_failures
                .iter()
                .any(|finding| { finding == "package_invalid_read_scope_path:src/api/../secrets" })
        );
    }

    #[test]
    fn derive_writer_packet_clips_scope_to_target_spec_frontier() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: Some(
                            "# Workstream Spec\n\n## Purpose\n\nImplement the API slice\n\n## In Scope\n\n- Execute the bounded integration slice.\n\n## Out Of Scope\n\n- Unrelated repo changes.\n\n## Dependencies\n\n- Outcome Lock and Program Blueprint stay current.\n\n## Touched Surfaces\n\n- Runtime backend.\n\n## Read Scope\n\n- src/api\n\n## Write Scope\n\n- src/api\n\n## Interfaces And Contracts Touched\n\n- internal command JSON contract\n\n## Implementation Shape\n\nKeep the workstream bounded and reviewable.\n\n## Proof-Of-Completion Expectations\n\n- cargo test -p api\n\n## Non-Breakage Expectations\n\n- Existing mission contracts still validate.\n\n## Review Lenses\n\n- correctness\n\n## Replan Boundary\n\n- Reopen planning on scope expansion.\n\n## Truth Basis Refs\n\n- PROGRAM-BLUEPRINT.md\n\n## Freshness Notes\n\n- Current for the integration test.\n\n## Support Files\n\n- `REVIEW.md`\n".to_string(),
                        ),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: Some(
                            "# Workstream Spec\n\n## Purpose\n\nImplement the UI slice\n\n## In Scope\n\n- Execute the bounded integration slice.\n\n## Out Of Scope\n\n- Unrelated repo changes.\n\n## Dependencies\n\n- Outcome Lock and Program Blueprint stay current.\n\n## Touched Surfaces\n\n- Runtime backend.\n\n## Read Scope\n\n- src/ui\n\n## Write Scope\n\n- src/ui\n\n## Interfaces And Contracts Touched\n\n- internal command JSON contract\n\n## Implementation Shape\n\nKeep the workstream bounded and reviewable.\n\n## Proof-Of-Completion Expectations\n\n- cargo test -p ui\n\n## Non-Breakage Expectations\n\n- Existing mission contracts still validate.\n\n## Review Lenses\n\n- correctness\n\n## Replan Boundary\n\n- Reopen planning on scope expansion.\n\n## Truth Basis Refs\n\n- PROGRAM-BLUEPRINT.md\n\n## Freshness Notes\n\n- Current for the integration test.\n\n## Support Files\n\n- `REVIEW.md`\n".to_string(),
                        ),
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("mission:mission_alpha".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["backend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p api"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &[],
                        &["src/ui"],
                        &["src/ui"],
                        &["frontend"],
                        Some(WaveRiskClass::Normal),
                        &["cargo test -p ui"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("planning writeback should work");

        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Mission,
                target_id: "mission_alpha".to_string(),
                included_spec_ids: vec!["spec_api".to_string(), "spec_ui".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string(), "src/ui".to_string()],
                write_scope: vec!["src/api".to_string(), "src/ui".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        assert_eq!(package.status, ExecutionPackageStatus::Passed);

        let packet = derive_writer_packet(
            &paths,
            &WriterPacketInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id,
                target_spec_id: "spec_api".to_string(),
                required_checks: vec!["cargo test -p api".to_string()],
                review_lenses: vec!["correctness".to_string()],
                explicitly_disallowed_decisions: Vec::new(),
            },
        )
        .expect("writer packet derivation should work");

        assert_eq!(packet.allowed_read_paths, vec!["src/api".to_string()]);
        assert_eq!(packet.allowed_write_paths, vec!["src/api".to_string()]);
    }

    #[test]
    fn contradiction_resume_override_preserves_discovered_phase_for_needs_user() {
        let record = ContradictionRecord {
            contradiction_id: "contradiction_1".to_string(),
            discovered_in_phase: "execution".to_string(),
            discovered_by: "codex".to_string(),
            target_type: TargetType::Spec,
            target_id: "spec_api".to_string(),
            evidence_refs: vec!["RECEIPTS/test.txt".to_string()],
            violated_assumption_or_contract: "Need the user to choose rollout posture.".to_string(),
            suggested_reopen_layer: ReopenLayer::ExecutionPackage,
            reason_code: "needs_user".to_string(),
            status: ContradictionStatus::Open,
            governing_revision: "spec:spec_api:1".to_string(),
            triage_decision: None,
            triaged_at: None,
            triaged_by: None,
            machine_action: Some(MachineAction::YieldNeedsUser),
            next_required_branch: Some(NextRequiredBranch::NeedsUser),
            resolution_ref: None,
            resolved_at: None,
        };

        let override_state =
            contradiction_resume_override(&record).expect("needs-user contradiction override");
        assert_eq!(override_state.resume_status, ResumeStatus::WaitingNeedsUser);
        assert_eq!(override_state.next_phase.as_deref(), Some("execution"));
    }

    #[test]
    fn idempotent_planning_write_keeps_downstream_review_gates_passed() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");
        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning writeback should work");
        let package = compile_execution_package(
            &paths,
            &ExecutionPackageInput {
                mission_id: "mission_alpha".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                included_spec_ids: vec!["spec_api".to_string()],
                dependency_satisfaction_state: vec![DependencyCheck {
                    name: "lock_current".to_string(),
                    satisfied: true,
                    detail: "current lock is ratified".to_string(),
                }],
                read_scope: vec!["src/api".to_string()],
                write_scope: vec!["src/api".to_string()],
                proof_obligations: vec!["cargo test".to_string()],
                review_obligations: vec!["correctness".to_string()],
                replan_boundary: None,
                wave_context: None,
                wave_fingerprint: None,
                wave_specs: Vec::new(),
                gate_checks: Vec::new(),
            },
        )
        .expect("package compilation should work");
        let bundle = compile_review_bundle(
            &paths,
            &ReviewBundleInput {
                mission_id: "mission_alpha".to_string(),
                source_package_id: package.package_id,
                bundle_kind: BundleKind::SpecReview,
                mandatory_review_lenses: vec!["correctness".to_string()],
                target_spec_id: Some("spec_api".to_string()),
                proof_rows_under_review: vec!["cargo test".to_string()],
                receipts: vec!["RECEIPTS/test.txt".to_string()],
                changed_files_or_diff: vec!["src/api/mod.rs".to_string()],
                touched_interface_contracts: Vec::new(),
                mission_level_proof_rows: Vec::new(),
                cross_spec_claim_refs: Vec::new(),
                visible_artifact_refs: Vec::new(),
                deferred_descoped_follow_on_refs: Vec::new(),
                open_finding_summary: Vec::new(),
            },
        )
        .expect("review bundle compilation should work");
        record_review_result(
            &paths,
            &ReviewResultInput {
                mission_id: "mission_alpha".to_string(),
                bundle_id: bundle.bundle_id.clone(),
                reviewer: "codex".to_string(),
                verdict: "clean".to_string(),
                target_spec_id: Some("spec_api".to_string()),
                governing_refs: Vec::new(),
                evidence_refs: reviewer_output_evidence(
                    "idempotent-planning-review",
                    &["RECEIPTS/test.txt"],
                ),
                findings: Vec::new(),
                disposition_notes: Vec::new(),
                next_required_branch: Some(NextRequiredBranch::MissionClose),
                waiting_request: None,
                review_truth_snapshot: review_truth_snapshot_for(&paths, &bundle.bundle_id),
            },
        )
        .expect("clean review should record");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: Some(canonical_spec_body("Implement the API slice")),
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::Complete),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("idempotent planning write should work");

        let gates = load_gate_index(&paths).expect("load gates");
        let review_gate = gates
            .gates
            .iter()
            .find(|gate| {
                gate.gate_kind == GateKind::BlockingReview && gate.target_ref == "spec:spec_api"
            })
            .expect("spec review gate");
        assert_eq!(review_gate.status, MissionGateStatus::Passed);
    }

    #[test]
    fn planning_write_supersedes_omitted_frontier_specs() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![
                    WorkstreamSpecInput {
                        spec_id: "spec_api".to_string(),
                        purpose: "Implement the API slice".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                    WorkstreamSpecInput {
                        spec_id: "spec_ui".to_string(),
                        purpose: "Implement the UI slice".to_string(),
                        body_markdown: None,
                        artifact_status: Some(SpecArtifactStatus::Active),
                        packetization_status: Some(PacketizationStatus::Runnable),
                        execution_status: Some(SpecExecutionStatus::NotStarted),
                        owner_mode: None,
                        replan_boundary: None,
                    },
                ],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: Some(execution_graph_with_default_obligations(vec![
                    execution_graph_node(
                        "spec_api",
                        &[],
                        &["src/api"],
                        &["src/api"],
                        &["backend"],
                        None,
                        &["cargo test"],
                    ),
                    execution_graph_node(
                        "spec_ui",
                        &["spec_api"],
                        &["src/ui"],
                        &["src/ui"],
                        &["frontend"],
                        None,
                        &["ui smoke"],
                    ),
                ])),
                next_action: None,
            },
        )
        .expect("initial planning write should work");

        let original_spec = load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file("spec_ui"))
            .expect("load original omitted spec");
        assert_eq!(
            original_spec.frontmatter.artifact_status,
            SpecArtifactStatus::Active
        );

        write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 5,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: vec![WorkstreamSpecInput {
                    spec_id: "spec_api".to_string(),
                    purpose: "Implement the API slice".to_string(),
                    body_markdown: None,
                    artifact_status: Some(SpecArtifactStatus::Active),
                    packetization_status: Some(PacketizationStatus::Runnable),
                    execution_status: Some(SpecExecutionStatus::NotStarted),
                    owner_mode: None,
                    replan_boundary: None,
                }],
                selected_target_ref: Some("spec:spec_api".to_string()),
                execution_graph: None,
                next_action: None,
            },
        )
        .expect("planning write with omitted frontier spec should work");

        let superseded_spec =
            load_markdown::<WorkstreamSpecFrontmatter>(&paths.spec_file("spec_ui"))
                .expect("load superseded omitted spec");
        assert_eq!(
            superseded_spec.frontmatter.artifact_status,
            SpecArtifactStatus::Superseded
        );
        assert!(
            superseded_spec.frontmatter.spec_revision > original_spec.frontmatter.spec_revision
        );

        let blueprint = load_markdown::<ProgramBlueprintFrontmatter>(&paths.program_blueprint())
            .expect("load blueprint");
        assert_eq!(blueprint.frontmatter.blueprint_revision, 2);
        assert_eq!(
            load_active_blueprint_spec_ids(&paths, blueprint.frontmatter.blueprint_revision)
                .expect("load active spec ids"),
            vec!["spec_api".to_string()]
        );
    }

    #[test]
    fn planning_write_rejects_noncanonical_plan_levels() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = write_planning_artifacts(
            &paths,
            &PlanningWriteInput {
                mission_id: "mission_alpha".to_string(),
                body_markdown: canonical_blueprint_body(),
                plan_level: 6,
                problem_size: Some(ProblemSize::M),
                status: Some(BlueprintStatus::Approved),
                blueprint_revision: Some(1),
                proof_matrix: default_proof_matrix(),
                decision_obligations: Vec::new(),
                specs: Vec::new(),
                selected_target_ref: None,
                execution_graph: None,
                next_action: None,
            },
        )
        .expect_err("invalid plan level should fail");
        assert!(
            error
                .to_string()
                .contains("plan_level must be between 1 and 5")
        );
    }

    #[test]
    fn non_local_contradictions_cannot_route_back_into_local_execution() {
        let temp = TempDir::new().expect("temp dir");
        let paths = MissionPaths::new(temp.path(), "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let error = append_contradiction(
            &paths,
            &ContradictionInput {
                mission_id: "mission_alpha".to_string(),
                discovered_in_phase: "execution".to_string(),
                discovered_by: "codex".to_string(),
                target_type: TargetType::Spec,
                target_id: "spec_api".to_string(),
                evidence_refs: vec!["RECEIPTS/test.txt".to_string()],
                violated_assumption_or_contract: "Blueprint truth changed.".to_string(),
                suggested_reopen_layer: ReopenLayer::Blueprint,
                reason_code: "blueprint_change".to_string(),
                governing_revision: "spec:spec_api:1".to_string(),
                status: Some(ContradictionStatus::AcceptedForReplan),
                triage_decision: Some(TriageDecision::RepairInLayer),
                triaged_by: Some("codex".to_string()),
                machine_action: Some(MachineAction::ForceRepair),
                next_required_branch: Some(NextRequiredBranch::Repair),
                resolution_ref: None,
            },
        )
        .expect_err("non-local contradictions should not route into local repair");
        assert!(
            error
                .to_string()
                .contains("cannot continue via local execution, repair, or review")
        );
    }

    #[test]
    fn resolve_resume_reports_contradictory_active_cycle_instead_of_bailing() {
        let temp = TempDir::new().expect("temp dir");
        let repo_root = temp.path();
        let paths = MissionPaths::new(repo_root, "mission_alpha");
        initialize_mission(
            &paths,
            &MissionInitInput {
                title: "Mission Alpha".to_string(),
                objective: "Ship the alpha flow safely.".to_string(),
                mission_id: None,
                slug: None,
                root_mission_id: None,
                parent_mission_id: None,
                clarify_status: Some(ClarifyStatus::Ratified),
                lock_status: Some(LockStatus::Locked),
                lock_posture: None,
                mission_state_body: None,
                outcome_lock_body: None,
                readme_body: None,
                waiting_request: None,
                next_action: None,
                summary: None,
                reason_code: None,
            },
        )
        .expect("mission bootstrap should work");

        let contradictory_cycle = ActiveCycleState::new(
            "cycle-interrupted".to_string(),
            "different_mission".to_string(),
            "execute".to_string(),
            Some("spec:alpha".to_string()),
            Vec::new(),
        );
        std::fs::write(
            paths.active_cycle(),
            serde_json::to_vec_pretty(&contradictory_cycle).expect("serialize active cycle"),
        )
        .expect("write contradictory cycle");

        let report = resolve_resume(
            repo_root,
            &ResolveResumeInput {
                mission_id: Some("mission_alpha".to_string()),
                live_child_lanes: Vec::new(),
            },
        )
        .expect("resume resolution should work");

        assert_eq!(
            report.resume_status,
            super::ResumeStatus::ContradictoryState
        );
        assert_eq!(
            report.active_cycle_status,
            super::ActiveCycleStatus::Contradictory
        );
    }
}
