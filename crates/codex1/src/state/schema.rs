//! STATE.json schema — declared in full up front.
//!
//! Downstream units (`outcome`, `plan`, `task`, `review`, `replan`,
//! `loop_`, `close`, `status`) only mutate the fields they own. The
//! overall shape is frozen: adding a new field requires bumping
//! `schema_version` in a coordinated change.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type TaskId = String;

/// Current schema version. Bump via Foundation only.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Clarify,
    Plan,
    Execute,
    ReviewLoop,
    MissionClose,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopMode {
    None,
    Clarify,
    Plan,
    Execute,
    ReviewLoop,
    MissionClose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopState {
    pub active: bool,
    pub paused: bool,
    pub mode: LoopMode,
}

impl Default for LoopState {
    fn default() -> Self {
        Self {
            active: false,
            paused: false,
            mode: LoopMode::None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeState {
    pub ratified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanLevel {
    Light,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanState {
    pub locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_level: Option<PlanLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_level: Option<PlanLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Full list of task ids from the locked PLAN.yaml DAG (in the order
    /// they appear). Populated by `plan check` at lock time so
    /// `readiness::tasks_complete` can recognize "all DAG nodes done"
    /// without silently ignoring missing entries in `state.tasks`.
    #[serde(default)]
    pub task_ids: Vec<TaskId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Ready,
    InProgress,
    AwaitingReview,
    Complete,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: TaskId,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_path: Option<String>,
    #[serde(default)]
    pub superseded_by: Option<TaskId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Pending,
    Clean,
    Dirty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRecordCategory {
    AcceptedCurrent,
    LateSameBoundary,
    StaleSuperseded,
    ContaminatedAfterTerminal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewRecord {
    pub task_id: TaskId,
    pub verdict: ReviewVerdict,
    #[serde(default)]
    pub reviewers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings_file: Option<String>,
    pub category: ReviewRecordCategory,
    pub recorded_at: String,
    pub boundary_revision: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplanState {
    #[serde(default)]
    pub consecutive_dirty_by_target: BTreeMap<TaskId, u32>,
    pub triggered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissionCloseReviewState {
    NotStarted,
    Open,
    Passed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloseState {
    pub review_state: MissionCloseReviewState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_at: Option<String>,
}

impl Default for CloseState {
    fn default() -> Self {
        Self {
            review_state: MissionCloseReviewState::NotStarted,
            terminal_at: None,
        }
    }
}

/// Top-level STATE.json shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissionState {
    pub mission_id: String,
    pub revision: u64,
    pub schema_version: u32,
    pub phase: Phase,
    #[serde(rename = "loop")]
    pub loop_: LoopState,
    pub outcome: OutcomeState,
    pub plan: PlanState,
    #[serde(default)]
    pub tasks: BTreeMap<TaskId, TaskRecord>,
    #[serde(default)]
    pub reviews: BTreeMap<TaskId, ReviewRecord>,
    pub replan: ReplanState,
    pub close: CloseState,
    pub events_cursor: u64,
}

impl MissionState {
    /// Fresh state for `codex1 init`.
    #[must_use]
    pub fn fresh(mission_id: impl Into<String>) -> Self {
        Self {
            mission_id: mission_id.into(),
            revision: 0,
            schema_version: SCHEMA_VERSION,
            phase: Phase::Clarify,
            loop_: LoopState::default(),
            outcome: OutcomeState::default(),
            plan: PlanState::default(),
            tasks: BTreeMap::new(),
            reviews: BTreeMap::new(),
            replan: ReplanState::default(),
            close: CloseState::default(),
            events_cursor: 0,
        }
    }
}
