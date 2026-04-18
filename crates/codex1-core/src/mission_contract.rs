use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::fingerprint::Fingerprint;
use crate::ralph::{ActiveCycleState, CloseoutRecord, ResumeMode, Terminality, Verdict};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionContractSnapshot {
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

impl MissionContractSnapshot {
    #[must_use]
    pub fn stop_reason(&self) -> String {
        match self.resume_mode {
            ResumeMode::AllowStop => format!(
                "mission {} may stop cleanly with verdict {:?}",
                self.mission_id, self.verdict
            ),
            ResumeMode::Continue => format!(
                "mission {} remains active: next action is {}",
                self.mission_id, self.next_action
            ),
            ResumeMode::YieldToUser => self
                .canonical_waiting_request
                .clone()
                .unwrap_or_else(|| "mission is waiting for user input".to_string()),
        }
    }

    #[must_use]
    pub const fn blocks_clean_stop(&self) -> bool {
        matches!(self.resume_mode, ResumeMode::Continue)
    }

    #[must_use]
    pub const fn yields_to_user(&self) -> bool {
        matches!(self.resume_mode, ResumeMode::YieldToUser)
    }
}

fn is_machine_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub fn validate_closeout_contract(record: &CloseoutRecord) -> Result<()> {
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

fn interrupted_cycle<'a>(
    closeouts: &[CloseoutRecord],
    latest: &CloseoutRecord,
    active_cycle: Option<&'a ActiveCycleState>,
) -> Option<&'a ActiveCycleState> {
    active_cycle.filter(|cycle| {
        cycle.mission_id == latest.mission_id
            && latest.cycle_id.as_deref() != Some(cycle.cycle_id.as_str())
            && cycle
                .opened_after_closeout_seq
                .is_none_or(|opened_after| opened_after >= latest.closeout_seq)
            && !closeouts
                .iter()
                .any(|record| record.cycle_id.as_deref() == Some(cycle.cycle_id.as_str()))
    })
}

fn snapshot_from_latest_closeout(latest: &CloseoutRecord) -> MissionContractSnapshot {
    MissionContractSnapshot {
        mission_id: latest.mission_id.clone(),
        phase: latest.phase.clone(),
        target: latest.target.clone(),
        verdict: latest.verdict.clone(),
        terminality: latest.terminality.clone(),
        resume_mode: latest.resume_mode.clone(),
        next_phase: latest.next_phase.clone(),
        next_action: latest.next_action.clone(),
        lock_revision: latest.lock_revision,
        lock_fingerprint: latest.lock_fingerprint.clone(),
        blueprint_revision: latest.blueprint_revision,
        blueprint_fingerprint: latest.blueprint_fingerprint.clone(),
        governing_revision: latest.governing_revision.clone(),
        reason_code: latest.reason_code.clone(),
        summary: latest.summary.clone(),
        continuation_prompt: latest.continuation_prompt.clone(),
        last_valid_closeout_seq: latest.closeout_seq,
        last_valid_closeout_ref: latest.closeout_id.clone(),
        last_applied_closeout_seq: latest.closeout_seq,
        active_cycle_id: None,
        waiting_request_id: latest.waiting_request_id.clone(),
        waiting_for: latest.waiting_for.clone(),
        canonical_waiting_request: latest.canonical_waiting_request.clone(),
        resume_condition: latest.resume_condition.clone(),
        request_emitted_at: latest.request_emitted_at.clone(),
        active_child_task_paths: latest.active_child_task_paths.clone(),
        artifact_fingerprints: latest.artifact_fingerprints.clone(),
    }
}

pub fn derive_snapshot_from_closeouts(
    closeouts: &[CloseoutRecord],
    active_cycle: Option<&ActiveCycleState>,
) -> Result<MissionContractSnapshot> {
    let latest = closeouts
        .last()
        .context("cannot rebuild state without closeouts")?;
    validate_closeout_contract(latest)?;

    let mut snapshot = snapshot_from_latest_closeout(latest);
    if let Some(interrupted_cycle) = interrupted_cycle(closeouts, latest, active_cycle) {
        snapshot.verdict = Verdict::ContinueRequired;
        snapshot.terminality = Terminality::ActionableNonTerminal;
        snapshot.resume_mode = ResumeMode::Continue;
        snapshot.next_action = format!(
            "Recover interrupted cycle {} before continuing.",
            interrupted_cycle.cycle_id
        );
        snapshot.continuation_prompt =
            Some("Recover interrupted cycle before yielding or continuing.".to_string());
        snapshot.active_cycle_id = Some(interrupted_cycle.cycle_id.clone());
        snapshot.waiting_request_id = None;
        snapshot.waiting_for = None;
        snapshot.canonical_waiting_request = None;
        snapshot.resume_condition = None;
        snapshot.request_emitted_at = None;
    }

    Ok(snapshot)
}

pub fn contradictory_snapshot(
    latest: &CloseoutRecord,
    active_cycle_id: Option<String>,
    next_action: &str,
) -> MissionContractSnapshot {
    MissionContractSnapshot {
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

pub fn orphan_active_cycle_snapshot(
    active_cycle: &ActiveCycleState,
    next_action: &str,
) -> MissionContractSnapshot {
    MissionContractSnapshot {
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
        governing_revision: active_cycle.governing_revision.clone(),
        reason_code: Some("orphan_active_cycle".to_string()),
        summary: Some("Recovered orphaned active cycle state.".to_string()),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::fingerprint::Fingerprint;
    use crate::ralph::{ActiveCycleState, CloseoutRecord, ResumeMode, Terminality, Verdict};

    use super::{MissionContractSnapshot, derive_snapshot_from_closeouts};

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
            cycle_kind: Some(crate::ralph::CycleKind::BoundedProgress),
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

    #[test]
    fn interrupted_cycle_snapshot_clears_waiting_identity() {
        let snapshot = derive_snapshot_from_closeouts(
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
        .expect("snapshot should derive");

        assert_eq!(snapshot.verdict, Verdict::ContinueRequired);
        assert_eq!(snapshot.resume_mode, ResumeMode::Continue);
        assert_eq!(snapshot.canonical_waiting_request, None);
        assert_eq!(snapshot.active_cycle_id.as_deref(), Some("cycle-2"));
    }

    #[test]
    fn snapshot_round_trips_through_ralph_state_projection() {
        let snapshot = derive_snapshot_from_closeouts(
            &[closeout(
                Verdict::ReviewRequired,
                Terminality::ActionableNonTerminal,
                ResumeMode::Continue,
            )],
            None,
        )
        .expect("snapshot should derive");

        let encoded = serde_json::to_vec(&snapshot).expect("encode snapshot");
        let decoded: crate::ralph::RalphState =
            serde_json::from_slice(&encoded).expect("decode via RalphState alias");
        assert_eq!(snapshot, decoded);

        let typed: MissionContractSnapshot =
            serde_json::from_slice(&encoded).expect("decode via kernel type");
        assert_eq!(snapshot, typed);
    }
}
