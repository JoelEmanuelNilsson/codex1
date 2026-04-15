use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::fingerprint::Fingerprint;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    ContinueRequired,
    ReviewRequired,
    RepairRequired,
    ReplanRequired,
    NeedsUser,
    Complete,
    HardBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Terminality {
    Terminal,
    ActionableNonTerminal,
    WaitingNonTerminal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeMode {
    AllowStop,
    Continue,
    YieldToUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CycleKind {
    BoundedProgress,
    GateEvaluation,
    WaitingHandshake,
    RecoveryReentry,
    MissionClose,
    ContradictionHandling,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Open,
    Passed,
    Failed,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateEntry {
    pub name: String,
    pub status: GateStatus,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloseoutRecord {
    #[serde(default)]
    pub closeout_id: Option<String>,
    #[serde(alias = "seq")]
    pub closeout_seq: u64,
    pub mission_id: String,
    pub phase: String,
    #[serde(default)]
    pub activity: String,
    pub verdict: Verdict,
    pub terminality: Terminality,
    pub resume_mode: ResumeMode,
    pub next_phase: Option<String>,
    pub next_action: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub cycle_kind: Option<CycleKind>,
    #[serde(default)]
    pub lock_revision: Option<u64>,
    #[serde(default)]
    pub lock_fingerprint: Option<String>,
    #[serde(default)]
    pub blueprint_revision: Option<u64>,
    #[serde(default)]
    pub blueprint_fingerprint: Option<String>,
    #[serde(default)]
    pub governing_revision: Option<String>,
    #[serde(default)]
    pub reason_code: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub continuation_prompt: Option<String>,
    #[serde(default)]
    pub cycle_id: Option<String>,
    #[serde(default)]
    pub waiting_request_id: Option<String>,
    #[serde(default)]
    pub waiting_for: Option<String>,
    #[serde(default)]
    pub canonical_waiting_request: Option<String>,
    #[serde(default)]
    pub resume_condition: Option<String>,
    #[serde(default)]
    pub request_emitted_at: Option<String>,
    #[serde(default)]
    pub active_child_task_paths: Vec<String>,
    #[serde(default)]
    pub artifact_fingerprints: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChildLaneIntegrationStatus {
    Pending,
    Integrated,
    Superseded,
    Abandoned,
}

impl Default for ChildLaneIntegrationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildLaneExpectation {
    pub task_path: String,
    pub lane_kind: String,
    pub expected_deliverable_ref: String,
    #[serde(default)]
    pub integration_status: ChildLaneIntegrationStatus,
    #[serde(default)]
    pub target_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveCycleState {
    pub cycle_id: String,
    pub mission_id: String,
    pub phase: String,
    #[serde(default)]
    pub opened_after_closeout_seq: Option<u64>,
    #[serde(default)]
    pub cycle_kind: Option<CycleKind>,
    #[serde(default)]
    pub activity: Option<String>,
    #[serde(default)]
    pub current_target: Option<String>,
    #[serde(default)]
    pub lock_revision: Option<u64>,
    #[serde(default)]
    pub lock_fingerprint: Option<String>,
    #[serde(default)]
    pub blueprint_revision: Option<u64>,
    #[serde(default)]
    pub blueprint_fingerprint: Option<String>,
    #[serde(default)]
    pub governing_revision: Option<String>,
    #[serde(default)]
    pub current_bounded_action: Option<String>,
    #[serde(default)]
    pub preconditions_checked: Vec<String>,
    #[serde(default)]
    pub expected_outputs: Vec<String>,
    #[serde(default)]
    pub attempt_index: u32,
    #[serde(default)]
    pub active_packet_refs: Vec<String>,
    #[serde(default)]
    pub active_bundle_refs: Vec<String>,
    #[serde(default)]
    pub expected_child_lanes: Vec<ChildLaneExpectation>,
    #[serde(default, rename = "expected_child_task_paths", skip_serializing)]
    pub(crate) legacy_expected_child_task_paths: Vec<String>,
}

impl ActiveCycleState {
    #[must_use]
    pub fn new(
        cycle_id: String,
        mission_id: String,
        phase: String,
        current_target: Option<String>,
        expected_child_lanes: Vec<ChildLaneExpectation>,
    ) -> Self {
        Self {
            cycle_id,
            mission_id,
            phase,
            opened_after_closeout_seq: None,
            cycle_kind: None,
            activity: None,
            current_target,
            lock_revision: None,
            lock_fingerprint: None,
            blueprint_revision: None,
            blueprint_fingerprint: None,
            governing_revision: None,
            current_bounded_action: None,
            preconditions_checked: Vec::new(),
            expected_outputs: Vec::new(),
            attempt_index: 1,
            active_packet_refs: Vec::new(),
            active_bundle_refs: Vec::new(),
            expected_child_lanes,
            legacy_expected_child_task_paths: Vec::new(),
        }
    }

    #[must_use]
    pub fn normalized_expected_child_lanes(&self) -> Vec<ChildLaneExpectation> {
        if !self.expected_child_lanes.is_empty() {
            return self.expected_child_lanes.clone();
        }

        self.legacy_expected_child_task_paths
            .iter()
            .map(|task_path| ChildLaneExpectation {
                task_path: task_path.clone(),
                lane_kind: "unknown".to_string(),
                expected_deliverable_ref: format!("lane:{task_path}"),
                integration_status: ChildLaneIntegrationStatus::Pending,
                target_ref: None,
            })
            .collect()
    }

    #[must_use]
    pub fn normalized_expected_child_task_paths(&self) -> Vec<String> {
        self.normalized_expected_child_lanes()
            .into_iter()
            .map(|lane| lane.task_path)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RalphState {
    pub mission_id: String,
    pub phase: String,
    #[serde(default, rename = "current_target", alias = "target")]
    pub target: Option<String>,
    pub verdict: Verdict,
    pub terminality: Terminality,
    pub resume_mode: ResumeMode,
    pub next_phase: Option<String>,
    pub next_action: String,
    #[serde(default)]
    pub lock_revision: Option<u64>,
    #[serde(default)]
    pub lock_fingerprint: Option<String>,
    #[serde(default)]
    pub blueprint_revision: Option<u64>,
    #[serde(default)]
    pub blueprint_fingerprint: Option<String>,
    #[serde(default)]
    pub governing_revision: Option<String>,
    #[serde(default)]
    pub reason_code: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub continuation_prompt: Option<String>,
    pub last_valid_closeout_seq: u64,
    #[serde(default)]
    pub last_valid_closeout_ref: Option<String>,
    pub last_applied_closeout_seq: u64,
    pub active_cycle_id: Option<String>,
    #[serde(default)]
    pub waiting_request_id: Option<String>,
    #[serde(default)]
    pub waiting_for: Option<String>,
    #[serde(default)]
    pub canonical_waiting_request: Option<String>,
    #[serde(default)]
    pub resume_condition: Option<String>,
    #[serde(default)]
    pub request_emitted_at: Option<String>,
    #[serde(default)]
    pub active_child_task_paths: Vec<String>,
    #[serde(default)]
    pub artifact_fingerprints: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StopHookDecision {
    pub should_block: bool,
    pub should_stop: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StopHookOutput {
    #[serde(rename = "continue")]
    pub continue_processing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(rename = "systemMessage", skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveCycleLoad {
    Missing,
    Parsed(ActiveCycleState),
    Malformed,
}

fn is_machine_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub fn validate_closeout(record: &CloseoutRecord) -> Result<()> {
    for (label, value) in [
        ("closeout_id", record.closeout_id.as_deref()),
        ("cycle_id", record.cycle_id.as_deref()),
        ("reason_code", record.reason_code.as_deref()),
    ] {
        if value.is_none_or(str::is_empty) {
            bail!("closeout is missing {label}");
        }
    }
    if record.activity.trim().is_empty() {
        bail!("closeout activity must not be empty");
    }
    if record.target.as_deref().is_none_or(str::is_empty) {
        bail!("closeout target must not be empty");
    }
    if !is_machine_reason_code(record.reason_code.as_deref().unwrap_or_default()) {
        bail!("closeout reason_code must be lowercase snake_case");
    }
    if record.cycle_kind.is_none() {
        bail!("closeout cycle_kind must be present");
    }
    if record
        .governing_revision
        .as_deref()
        .is_none_or(str::is_empty)
    {
        bail!("closeout governing_revision must not be empty");
    }
    if record.artifact_fingerprints.is_empty() {
        bail!("closeout artifact_fingerprints must not be empty");
    }
    for (label, value) in [
        ("lock_fingerprint", record.lock_fingerprint.as_deref()),
        (
            "blueprint_fingerprint",
            record.blueprint_fingerprint.as_deref(),
        ),
    ] {
        if let Some(value) = value {
            Fingerprint::parse(value)
                .map_err(anyhow::Error::new)
                .with_context(|| format!("closeout {label} is invalid"))?;
        }
    }
    for (artifact, value) in &record.artifact_fingerprints {
        Fingerprint::parse(value)
            .map_err(anyhow::Error::new)
            .with_context(|| format!("closeout artifact_fingerprints[{artifact}] is invalid"))?;
    }
    match record.verdict {
        Verdict::Complete | Verdict::HardBlocked => {
            if record.terminality != Terminality::Terminal {
                bail!("terminal verdict must use terminal terminality");
            }
            if record.resume_mode != ResumeMode::AllowStop {
                bail!("terminal verdict must use allow_stop resume mode");
            }
            if record
                .next_phase
                .as_deref()
                .is_some_and(|phase| phase != "complete")
            {
                bail!("terminal closeouts may only use next_phase `complete` or null");
            }
        }
        Verdict::NeedsUser => {
            if record.terminality != Terminality::WaitingNonTerminal {
                bail!("needs_user must use waiting_non_terminal terminality");
            }
            if record.resume_mode != ResumeMode::YieldToUser {
                bail!("needs_user must use yield_to_user resume mode");
            }
            for (label, value) in [
                ("waiting_request_id", &record.waiting_request_id),
                ("waiting_for", &record.waiting_for),
                (
                    "canonical_waiting_request",
                    &record.canonical_waiting_request,
                ),
                ("resume_condition", &record.resume_condition),
            ] {
                if value.as_deref().is_none_or(str::is_empty) {
                    bail!("needs_user closeout is missing {label}");
                }
            }
        }
        Verdict::ContinueRequired
        | Verdict::ReviewRequired
        | Verdict::RepairRequired
        | Verdict::ReplanRequired => {
            if record.terminality != Terminality::ActionableNonTerminal {
                bail!("actionable verdict must use actionable_non_terminal terminality");
            }
            if record.resume_mode != ResumeMode::Continue {
                bail!("actionable verdict must use continue resume mode");
            }
            if record
                .continuation_prompt
                .as_deref()
                .is_none_or(str::is_empty)
            {
                bail!("actionable closeout must include continuation_prompt");
            }
        }
    }

    if !matches!(record.verdict, Verdict::Complete | Verdict::HardBlocked)
        && record.next_phase.as_deref().is_none_or(str::is_empty)
    {
        bail!("non-terminal closeouts must declare next_phase");
    }

    if record.next_action.trim().is_empty() {
        bail!("closeout next_action must not be empty");
    }

    if record.closeout_seq == 0 {
        bail!("closeout_seq must be greater than zero");
    }

    Ok(())
}

pub fn rebuild_state_from_closeouts(
    closeouts: &[CloseoutRecord],
    active_cycle: Option<&ActiveCycleState>,
) -> Result<RalphState> {
    let latest = closeouts
        .last()
        .context("cannot rebuild state without closeouts")?;
    validate_closeout(latest)?;
    let interrupted_cycle = active_cycle.filter(|cycle| {
        cycle.mission_id == latest.mission_id
            && latest.cycle_id.as_deref() != Some(cycle.cycle_id.as_str())
            && cycle
                .opened_after_closeout_seq
                .is_none_or(|opened_after| opened_after >= latest.closeout_seq)
            && !closeouts
                .iter()
                .any(|record| record.cycle_id.as_deref() == Some(cycle.cycle_id.as_str()))
    });

    Ok(RalphState {
        mission_id: latest.mission_id.clone(),
        phase: latest.phase.clone(),
        target: latest.target.clone(),
        verdict: if interrupted_cycle.is_some() {
            Verdict::ContinueRequired
        } else {
            latest.verdict.clone()
        },
        terminality: if interrupted_cycle.is_some() {
            Terminality::ActionableNonTerminal
        } else {
            latest.terminality.clone()
        },
        resume_mode: if interrupted_cycle.is_some() {
            ResumeMode::Continue
        } else {
            latest.resume_mode.clone()
        },
        next_phase: latest.next_phase.clone(),
        next_action: interrupted_cycle.map_or_else(
            || latest.next_action.clone(),
            |cycle| {
                format!(
                    "Recover interrupted cycle {} before continuing.",
                    cycle.cycle_id
                )
            },
        ),
        lock_revision: latest.lock_revision,
        lock_fingerprint: latest.lock_fingerprint.clone(),
        blueprint_revision: latest.blueprint_revision,
        blueprint_fingerprint: latest.blueprint_fingerprint.clone(),
        governing_revision: latest.governing_revision.clone(),
        reason_code: latest.reason_code.clone(),
        summary: latest.summary.clone(),
        continuation_prompt: interrupted_cycle.map_or_else(
            || latest.continuation_prompt.clone(),
            |_| Some("Recover interrupted cycle before yielding or continuing.".to_string()),
        ),
        last_valid_closeout_seq: latest.closeout_seq,
        last_valid_closeout_ref: latest.closeout_id.clone(),
        last_applied_closeout_seq: latest.closeout_seq,
        active_cycle_id: interrupted_cycle.map(|cycle| cycle.cycle_id.clone()),
        waiting_request_id: interrupted_cycle
            .is_some()
            .then_some(None)
            .unwrap_or_else(|| latest.waiting_request_id.clone()),
        waiting_for: interrupted_cycle
            .is_some()
            .then_some(None)
            .unwrap_or_else(|| latest.waiting_for.clone()),
        canonical_waiting_request: interrupted_cycle
            .is_some()
            .then_some(None)
            .unwrap_or_else(|| latest.canonical_waiting_request.clone()),
        resume_condition: interrupted_cycle
            .is_some()
            .then_some(None)
            .unwrap_or_else(|| latest.resume_condition.clone()),
        request_emitted_at: interrupted_cycle
            .is_some()
            .then_some(None)
            .unwrap_or_else(|| latest.request_emitted_at.clone()),
        active_child_task_paths: latest.active_child_task_paths.clone(),
        artifact_fingerprints: latest.artifact_fingerprints.clone(),
    })
}

pub fn determine_stop_decision(state: &RalphState) -> StopHookDecision {
    match state.resume_mode {
        ResumeMode::AllowStop => StopHookDecision {
            should_block: false,
            should_stop: false,
            reason: format!(
                "mission {} may stop cleanly with verdict {:?}",
                state.mission_id, state.verdict
            ),
        },
        ResumeMode::Continue => StopHookDecision {
            should_block: true,
            should_stop: false,
            reason: format!(
                "mission {} remains active: next action is {}",
                state.mission_id, state.next_action
            ),
        },
        ResumeMode::YieldToUser => StopHookDecision {
            should_block: false,
            should_stop: false,
            reason: state
                .canonical_waiting_request
                .clone()
                .unwrap_or_else(|| "mission is waiting for user input".to_string()),
        },
    }
}

impl StopHookOutput {
    #[must_use]
    pub fn from_state(decision: &StopHookDecision, state: &RalphState) -> Self {
        if decision.should_block {
            Self {
                continue_processing: true,
                decision: Some("block".to_string()),
                reason: Some(decision.reason.clone()),
                system_message: None,
            }
        } else if state.resume_mode == ResumeMode::YieldToUser {
            Self {
                continue_processing: true,
                decision: None,
                reason: None,
                system_message: Some(decision.reason.clone()),
            }
        } else {
            Self {
                continue_processing: true,
                decision: None,
                reason: None,
                system_message: None,
            }
        }
    }

    #[must_use]
    pub fn for_selection_wait(state: &crate::runtime::SelectionState) -> Self {
        if state.selected_mission_id.is_none() {
            Self {
                continue_processing: true,
                decision: None,
                reason: None,
                system_message: Some(state.canonical_selection_request.clone()),
            }
        } else {
            Self {
                continue_processing: true,
                decision: None,
                reason: None,
                system_message: None,
            }
        }
    }
}

pub fn load_closeouts(path: &Path) -> Result<Vec<CloseoutRecord>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let file = File::open(path)
        .with_context(|| format!("failed to open closeouts file {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let lines: Vec<String> = reader
        .lines()
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to read {}", path.display()))?;

    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<CloseoutRecord>(line) {
            Ok(record) => {
                validate_closeout(&record)?;
                records.push(record);
            }
            Err(error) if index + 1 == lines.len() => {
                // Treat a malformed final line as truncated debris.
                let _ = error;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to parse closeout line {} in {}",
                        index + 1,
                        path.display()
                    )
                });
            }
        }
    }

    validate_closeout_history(&records, path)?;

    Ok(records)
}

fn validate_closeout_history(records: &[CloseoutRecord], path: &Path) -> Result<()> {
    let mut seen_closeout_ids = BTreeSet::new();
    let mut expected_seq = 1_u64;
    let mut mission_id: Option<String> = None;
    for record in records {
        if record.closeout_seq != expected_seq {
            bail!(
                "closeout history in {} has sequence gap or disorder: expected {}, got {}",
                path.display(),
                expected_seq,
                record.closeout_seq
            );
        }
        expected_seq += 1;
        if let Some(existing_mission_id) = &mission_id {
            if existing_mission_id != &record.mission_id {
                bail!(
                    "closeout history in {} mixes mission ids: {} and {}",
                    path.display(),
                    existing_mission_id,
                    record.mission_id
                );
            }
        } else {
            mission_id = Some(record.mission_id.clone());
        }
        let closeout_id = record
            .closeout_id
            .as_deref()
            .context("validated closeout is missing closeout_id")?;
        if !seen_closeout_ids.insert(closeout_id.to_string()) {
            bail!(
                "closeout history in {} contains duplicate closeout_id {}",
                path.display(),
                closeout_id
            );
        }
    }

    Ok(())
}

pub fn load_state(path: &Path) -> Result<Option<RalphState>> {
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    match serde_json::from_slice(&raw) {
        Ok(state) => Ok(Some(state)),
        Err(_) => Ok(None),
    }
}

pub fn load_active_cycle(path: &Path) -> Result<Option<ActiveCycleState>> {
    Ok(match inspect_active_cycle(path)? {
        ActiveCycleLoad::Missing | ActiveCycleLoad::Malformed => None,
        ActiveCycleLoad::Parsed(cycle) => Some(cycle),
    })
}

pub fn inspect_active_cycle(path: &Path) -> Result<ActiveCycleLoad> {
    if !path.is_file() {
        return Ok(ActiveCycleLoad::Missing);
    }
    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    match serde_json::from_slice(&raw) {
        Ok(cycle) => Ok(ActiveCycleLoad::Parsed(cycle)),
        Err(_) => Ok(ActiveCycleLoad::Malformed),
    }
}

pub fn append_closeout_and_rebuild_state(
    mission_dir: &Path,
    closeout: &CloseoutRecord,
    active_cycle: Option<&ActiveCycleState>,
) -> Result<RalphState> {
    validate_closeout(closeout)?;
    fs::create_dir_all(mission_dir)
        .with_context(|| format!("failed to create {}", mission_dir.display()))?;

    let closeouts_path = mission_dir.join("closeouts.ndjson");
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&closeouts_path)
        .with_context(|| format!("failed to open {}", closeouts_path.display()))?;
    file.lock_exclusive()
        .with_context(|| format!("failed to lock {}", closeouts_path.display()))?;
    let mut closeouts = load_closeouts(&closeouts_path)?;
    if let Some(existing) = closeouts.last()
        && existing.closeout_id == closeout.closeout_id
        && existing.cycle_id == closeout.cycle_id
        && existing.closeout_seq == closeout.closeout_seq
    {
        let next_state = rebuild_state_from_closeouts(&closeouts, None)?;
        let state_path = mission_dir.join("state.json");
        atomic_write_json(
            &state_path,
            &next_state,
            "failed to serialize Ralph state for atomic write",
        )?;
        let active_cycle_path = mission_dir.join("active-cycle.json");
        if active_cycle_path.exists() {
            atomic_remove_file(&active_cycle_path)?;
        }
        fsync_dir(mission_dir)?;
        file.unlock()
            .with_context(|| format!("failed to unlock {}", closeouts_path.display()))?;
        return Ok(next_state);
    }
    let expected_seq = closeouts.last().map_or(1, |record| record.closeout_seq + 1);
    if closeout.closeout_seq != expected_seq {
        bail!(
            "closeout sequence mismatch for mission {}: expected {}, got {}",
            closeout.mission_id,
            expected_seq,
            closeout.closeout_seq
        );
    }
    if let Some(active_cycle) = active_cycle {
        if active_cycle.mission_id != closeout.mission_id {
            bail!("active cycle mission does not match closeout mission");
        }
        if closeout.cycle_id.as_deref() != Some(active_cycle.cycle_id.as_str()) {
            bail!("active cycle id does not match closeout cycle id");
        }
    }
    closeouts.push(closeout.clone());
    validate_closeout_history(&closeouts, &closeouts_path)?;
    let next_state = rebuild_state_from_closeouts(&closeouts, None)?;
    serde_json::to_writer(&file, closeout).context("failed to serialize closeout")?;
    file.write_all(b"\n")
        .context("failed to terminate closeout line")?;
    file.sync_all()
        .with_context(|| format!("failed to fsync {}", closeouts_path.display()))?;
    let state_path = mission_dir.join("state.json");
    atomic_write_json(
        &state_path,
        &next_state,
        "failed to serialize Ralph state for atomic write",
    )?;

    let active_cycle_path = mission_dir.join("active-cycle.json");
    if active_cycle_path.exists() {
        atomic_remove_file(&active_cycle_path)?;
    }
    fsync_dir(mission_dir)?;
    file.unlock()
        .with_context(|| format!("failed to unlock {}", closeouts_path.display()))?;

    Ok(next_state)
}

pub fn rebuild_state_from_files(mission_dir: &Path) -> Result<Option<RalphState>> {
    let closeouts = load_closeouts(&mission_dir.join("closeouts.ndjson"))?;
    if closeouts.is_empty() {
        return match inspect_active_cycle(&mission_dir.join("active-cycle.json"))? {
            ActiveCycleLoad::Missing => Ok(None),
            ActiveCycleLoad::Parsed(active_cycle) => Ok(Some(orphan_active_cycle_state(
                &active_cycle,
                "Recover interrupted cycle before continuing.",
            ))),
            ActiveCycleLoad::Malformed => Ok(None),
        };
    }
    match inspect_active_cycle(&mission_dir.join("active-cycle.json"))? {
        ActiveCycleLoad::Missing => rebuild_state_from_closeouts(&closeouts, None).map(Some),
        ActiveCycleLoad::Parsed(active_cycle) => {
            rebuild_state_from_closeouts(&closeouts, Some(&active_cycle)).map(Some)
        }
        ActiveCycleLoad::Malformed => {
            let latest = closeouts
                .last()
                .context("cannot rebuild state without closeouts")?;
            Ok(Some(contradictory_active_cycle_state(
                latest,
                None,
                "Repair malformed active-cycle state before resume.",
            )))
        }
    }
}

pub fn list_non_terminal_missions(missions_root: &Path) -> Result<Vec<(String, RalphState)>> {
    let mut results = Vec::new();
    if !missions_root.is_dir() {
        return Ok(results);
    }
    for entry in fs::read_dir(missions_root)
        .with_context(|| format!("failed to read {}", missions_root.display()))?
    {
        let entry = entry.context("failed to read mission dir entry")?;
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(state) = rebuild_state_from_files(&entry.path())?
            && state.terminality != Terminality::Terminal
        {
            results.push((entry.file_name().to_string_lossy().to_string(), state));
        }
    }
    results.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(results)
}

pub fn selection_state_path(ralph_root: &Path) -> PathBuf {
    ralph_root.join("selection-state.json")
}

fn atomic_write_json(path: &Path, value: &impl Serialize, error_context: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;

    let bytes = serde_json::to_vec_pretty(value).with_context(|| error_context.to_string())?;
    let mut temp = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temp file in {}", parent.display()))?;
    temp.write_all(&bytes)
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

pub fn contradictory_active_cycle_state(
    latest: &CloseoutRecord,
    active_cycle_id: Option<String>,
    next_action: &str,
) -> RalphState {
    RalphState {
        mission_id: latest.mission_id.clone(),
        phase: latest.phase.clone(),
        target: latest.target.clone(),
        verdict: Verdict::ContinueRequired,
        terminality: Terminality::ActionableNonTerminal,
        resume_mode: ResumeMode::Continue,
        next_phase: latest.next_phase.clone(),
        next_action: next_action.to_string(),
        lock_revision: latest.lock_revision,
        lock_fingerprint: latest.lock_fingerprint.clone(),
        blueprint_revision: latest.blueprint_revision,
        blueprint_fingerprint: latest.blueprint_fingerprint.clone(),
        governing_revision: latest.governing_revision.clone(),
        reason_code: Some("contradictory_active_cycle".to_string()),
        summary: latest.summary.clone(),
        continuation_prompt: Some(next_action.to_string()),
        last_valid_closeout_seq: latest.closeout_seq,
        last_valid_closeout_ref: latest.closeout_id.clone(),
        last_applied_closeout_seq: latest.closeout_seq,
        active_cycle_id,
        waiting_request_id: latest.waiting_request_id.clone(),
        waiting_for: latest.waiting_for.clone(),
        canonical_waiting_request: latest.canonical_waiting_request.clone(),
        resume_condition: latest.resume_condition.clone(),
        request_emitted_at: latest.request_emitted_at.clone(),
        active_child_task_paths: latest.active_child_task_paths.clone(),
        artifact_fingerprints: latest.artifact_fingerprints.clone(),
    }
}

fn orphan_active_cycle_state(active_cycle: &ActiveCycleState, next_action: &str) -> RalphState {
    RalphState {
        mission_id: active_cycle.mission_id.clone(),
        phase: active_cycle.phase.clone(),
        target: active_cycle.current_target.clone(),
        verdict: Verdict::ContinueRequired,
        terminality: Terminality::ActionableNonTerminal,
        resume_mode: ResumeMode::Continue,
        next_phase: Some(active_cycle.phase.clone()),
        next_action: format!("{next_action} ({})", active_cycle.cycle_id),
        lock_revision: None,
        lock_fingerprint: None,
        blueprint_revision: None,
        blueprint_fingerprint: None,
        governing_revision: Some(format!("interrupted-cycle:{}", active_cycle.cycle_id)),
        reason_code: Some("orphan_active_cycle".to_string()),
        summary: Some("Recovered an interrupted mission from active-cycle state.".to_string()),
        continuation_prompt: Some(next_action.to_string()),
        last_valid_closeout_seq: 0,
        last_valid_closeout_ref: None,
        last_applied_closeout_seq: 0,
        active_cycle_id: Some(active_cycle.cycle_id.clone()),
        waiting_request_id: None,
        waiting_for: None,
        canonical_waiting_request: None,
        resume_condition: None,
        request_emitted_at: None,
        active_child_task_paths: active_cycle.normalized_expected_child_task_paths(),
        artifact_fingerprints: BTreeMap::new(),
    }
}

fn atomic_remove_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    let tombstone = parent.join(format!(
        ".{}.delete-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("tmp"),
        uuid::Uuid::new_v4()
    ));
    fs::rename(path, &tombstone).with_context(|| {
        format!(
            "failed to rename {} to {} for atomic delete",
            path.display(),
            tombstone.display()
        )
    })?;
    fsync_dir(parent)?;
    fs::remove_file(&tombstone)
        .with_context(|| format!("failed to remove {}", tombstone.display()))?;
    fsync_dir(parent)?;
    Ok(())
}

fn fsync_dir(path: &Path) -> Result<()> {
    File::open(path)
        .with_context(|| format!("failed to open directory {}", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to fsync directory {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use crate::fingerprint::Fingerprint;

    use super::{
        ActiveCycleState, CloseoutRecord, ResumeMode, Terminality, Verdict,
        append_closeout_and_rebuild_state, determine_stop_decision, load_closeouts,
        rebuild_state_from_files,
    };

    fn closeout(
        verdict: Verdict,
        terminality: Terminality,
        resume_mode: ResumeMode,
    ) -> CloseoutRecord {
        let next_phase = if matches!(verdict, Verdict::Complete | Verdict::HardBlocked) {
            Some("complete".to_string())
        } else {
            Some("review".to_string())
        };
        CloseoutRecord {
            closeout_id: Some("closeout-1".to_string()),
            closeout_seq: 1,
            mission_id: "mission-1".to_string(),
            phase: "plan".to_string(),
            activity: "test".to_string(),
            verdict,
            terminality,
            resume_mode,
            next_phase,
            next_action: "run review".to_string(),
            target: Some("mission:mission-1".to_string()),
            cycle_kind: Some(super::CycleKind::BoundedProgress),
            lock_revision: Some(1),
            lock_fingerprint: Some(Fingerprint::from_bytes(b"lock-fp").to_string()),
            blueprint_revision: Some(1),
            blueprint_fingerprint: Some(Fingerprint::from_bytes(b"blueprint-fp").to_string()),
            governing_revision: Some("blueprint:1".to_string()),
            reason_code: Some("test_closeout".to_string()),
            summary: Some("test summary".to_string()),
            continuation_prompt: Some("continue".to_string()),
            cycle_id: Some("cycle-1".to_string()),
            waiting_request_id: Some("wait-1".to_string()),
            waiting_for: Some("user_input".to_string()),
            canonical_waiting_request: Some("Please choose rollout posture".to_string()),
            resume_condition: Some("user answered".to_string()),
            request_emitted_at: Some("2026-04-12T00:00:00Z".to_string()),
            active_child_task_paths: vec!["/root/explorer".to_string()],
            artifact_fingerprints: BTreeMap::from([(
                "mission-state".to_string(),
                Fingerprint::from_bytes(b"mission-state-fp").to_string(),
            )]),
        }
    }

    use std::collections::BTreeMap;

    #[test]
    fn actionable_state_blocks_stop() {
        let state = super::rebuild_state_from_closeouts(
            &[closeout(
                Verdict::ReviewRequired,
                Terminality::ActionableNonTerminal,
                ResumeMode::Continue,
            )],
            None,
        )
        .expect("state should build");
        let decision = determine_stop_decision(&state);
        assert!(decision.should_block);
    }

    #[test]
    fn append_closeout_writes_state() {
        let temp = TempDir::new().expect("temp dir should exist");
        let state = append_closeout_and_rebuild_state(
            temp.path(),
            &closeout(
                Verdict::Complete,
                Terminality::Terminal,
                ResumeMode::AllowStop,
            ),
            None,
        )
        .expect("closeout append should work");
        assert_eq!(state.verdict, Verdict::Complete);
        assert_eq!(state.active_cycle_id, None);
        assert!(
            rebuild_state_from_files(temp.path())
                .expect("state rebuild should work")
                .is_some()
        );
    }

    #[test]
    fn append_closeout_rejects_duplicate_closeout_ids() {
        let temp = TempDir::new().expect("temp dir should exist");
        append_closeout_and_rebuild_state(
            temp.path(),
            &closeout(
                Verdict::ReviewRequired,
                Terminality::ActionableNonTerminal,
                ResumeMode::Continue,
            ),
            None,
        )
        .expect("first closeout append should work");

        let mut duplicate = closeout(
            Verdict::Complete,
            Terminality::Terminal,
            ResumeMode::AllowStop,
        );
        duplicate.closeout_seq = 2;

        let error = append_closeout_and_rebuild_state(temp.path(), &duplicate, None)
            .expect_err("duplicate closeout id should fail before write");
        assert!(error.to_string().contains("duplicate closeout_id"));
    }

    #[test]
    fn interrupted_waiting_cycle_requires_recovery_before_waiting() {
        let state = super::rebuild_state_from_closeouts(
            &[closeout(
                Verdict::NeedsUser,
                Terminality::WaitingNonTerminal,
                ResumeMode::YieldToUser,
            )],
            Some(&ActiveCycleState::new(
                "cycle-2".to_string(),
                "mission-1".to_string(),
                "review".to_string(),
                Some("mission:mission-1".to_string()),
                Vec::new(),
            )),
        )
        .expect("state should build");
        assert_eq!(state.verdict, Verdict::ContinueRequired);
        assert_eq!(state.terminality, Terminality::ActionableNonTerminal);
        assert_eq!(state.resume_mode, ResumeMode::Continue);
        assert_eq!(state.canonical_waiting_request, None);
    }

    #[test]
    fn active_cycle_supports_legacy_child_task_paths() {
        let cycle: ActiveCycleState = serde_json::from_str(
            r#"{
                "cycle_id":"cycle-1",
                "mission_id":"mission-1",
                "phase":"execute",
                "current_target":"spec:alpha",
                "expected_child_task_paths":["/root/specdrafter1","/root/reviewer1"]
            }"#,
        )
        .expect("legacy active cycle should deserialize");

        let lanes = cycle.normalized_expected_child_lanes();
        assert_eq!(lanes.len(), 2);
        assert_eq!(lanes[0].task_path, "/root/specdrafter1");
        assert_eq!(lanes[0].lane_kind, "unknown");
        assert_eq!(lanes[1].expected_deliverable_ref, "lane:/root/reviewer1");
    }

    #[test]
    fn load_closeouts_ignores_truncated_final_line() {
        let temp = TempDir::new().expect("temp dir should exist");
        let path = temp.path().join("closeouts.ndjson");
        let record = closeout(
            Verdict::ReviewRequired,
            Terminality::ActionableNonTerminal,
            ResumeMode::Continue,
        );
        let encoded = serde_json::to_string(&record).expect("encode closeout");
        fs::write(&path, format!("{encoded}\n{{\"broken\":")).expect("write closeouts file");

        let loaded = load_closeouts(&path).expect("load closeouts");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].closeout_seq, 1);
    }

    #[test]
    fn load_closeouts_rejects_sequence_gaps() {
        let temp = TempDir::new().expect("temp dir should exist");
        let path = temp.path().join("closeouts.ndjson");
        let mut first = closeout(
            Verdict::ReviewRequired,
            Terminality::ActionableNonTerminal,
            ResumeMode::Continue,
        );
        let mut second = closeout(
            Verdict::ReviewRequired,
            Terminality::ActionableNonTerminal,
            ResumeMode::Continue,
        );
        second.closeout_seq = 4;
        second.closeout_id = Some("closeout-4".to_string());
        fs::write(
            &path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).expect("encode first closeout"),
                serde_json::to_string(&second).expect("encode second closeout")
            ),
        )
        .expect("write closeouts file");

        let error = load_closeouts(&path).expect_err("sequence gaps must be rejected");
        assert!(error.to_string().contains("sequence gap or disorder"));
        first.closeout_seq = 1;
    }

    #[test]
    fn rebuild_state_ignores_stale_matching_active_cycle_file() {
        let temp = TempDir::new().expect("temp dir should exist");
        append_closeout_and_rebuild_state(
            temp.path(),
            &closeout(
                Verdict::Complete,
                Terminality::Terminal,
                ResumeMode::AllowStop,
            ),
            None,
        )
        .expect("closeout append should work");

        let active_cycle = ActiveCycleState::new(
            "cycle-1".to_string(),
            "mission-1".to_string(),
            "review".to_string(),
            Some("mission:mission-1".to_string()),
            Vec::new(),
        );
        fs::write(
            temp.path().join("active-cycle.json"),
            serde_json::to_vec_pretty(&active_cycle).expect("encode cycle"),
        )
        .expect("write stale active cycle");

        let rebuilt = rebuild_state_from_files(temp.path())
            .expect("rebuild should work")
            .expect("state should exist");
        assert_eq!(rebuilt.verdict, Verdict::Complete);
        assert_eq!(rebuilt.active_cycle_id, None);
    }

    #[test]
    fn terminal_closeouts_reject_non_terminal_next_phase() {
        let mut record = closeout(
            Verdict::Complete,
            Terminality::Terminal,
            ResumeMode::AllowStop,
        );
        record.next_phase = Some("review".to_string());

        let error =
            super::validate_closeout(&record).expect_err("terminal next_phase must be constrained");
        assert!(
            error
                .to_string()
                .contains("terminal closeouts may only use next_phase")
        );
    }

    #[test]
    fn superseded_older_active_cycle_does_not_override_newer_terminal_closeout() {
        let mut first = closeout(
            Verdict::ReviewRequired,
            Terminality::ActionableNonTerminal,
            ResumeMode::Continue,
        );
        first.closeout_id = Some("closeout-1".to_string());
        first.closeout_seq = 1;
        first.cycle_id = Some("cycle-1".to_string());
        first.next_phase = Some("review".to_string());

        let mut latest = closeout(
            Verdict::Complete,
            Terminality::Terminal,
            ResumeMode::AllowStop,
        );
        latest.closeout_id = Some("closeout-2".to_string());
        latest.closeout_seq = 2;
        latest.cycle_id = Some("cycle-2".to_string());
        latest.next_phase = Some("complete".to_string());

        let mut stale_cycle = ActiveCycleState::new(
            "cycle-1".to_string(),
            "mission-1".to_string(),
            "review".to_string(),
            Some("mission:mission-1".to_string()),
            Vec::new(),
        );
        stale_cycle.opened_after_closeout_seq = Some(0);

        let rebuilt = super::rebuild_state_from_closeouts(&[first, latest], Some(&stale_cycle))
            .expect("rebuild should prefer newer terminal closeout");
        assert_eq!(rebuilt.verdict, Verdict::Complete);
        assert_eq!(rebuilt.active_cycle_id, None);
    }
}
