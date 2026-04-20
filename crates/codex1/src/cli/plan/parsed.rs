//! PLAN.yaml parsed types.
//!
//! These types mirror the plan contract in
//! `docs/codex1-rebuild-handoff/03-planning-artifacts.md`. Deserialization
//! stays lenient on optional fields and strict on required ones so that
//! `plan check` can produce actionable `PLAN_INVALID` errors.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ParsedPlan {
    pub mission_id: Option<String>,
    pub planning_level: Option<PlanningLevel>,
    pub outcome_interpretation: Option<OutcomeInterpretation>,
    pub architecture: Option<Architecture>,
    pub planning_process: Option<PlanningProcess>,
    #[serde(default)]
    pub tasks: Vec<TaskSpec>,
    #[serde(default)]
    pub risks: Vec<Risk>,
    pub mission_close: Option<MissionClose>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanningLevel {
    pub requested: Option<String>,
    pub effective: Option<String>,
    #[serde(default)]
    pub escalation_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutcomeInterpretation {
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Architecture {
    pub summary: Option<String>,
    #[serde(default)]
    pub key_decisions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanningProcess {
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Evidence {
    pub kind: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub required_for_hard: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskSpec {
    pub id: Option<String>,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub spec: Option<String>,
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    #[serde(default)]
    pub unknown_side_effects: Option<bool>,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub proof: Vec<String>,
    #[serde(default)]
    pub review_target: Option<ReviewTarget>,
    #[serde(default)]
    pub review_profiles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewTarget {
    #[serde(default)]
    pub tasks: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Risk {
    pub risk: Option<String>,
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MissionClose {
    #[serde(default)]
    pub criteria: Vec<String>,
}

/// Canonical task kinds.
pub const TASK_KINDS: &[&str] = &[
    "design", "code", "docs", "test", "research", "repair", "review",
];

/// Canonical planning levels.
pub const PLAN_LEVELS: &[&str] = &["light", "medium", "hard"];

/// Evidence kinds that satisfy the hard-plan evidence requirement.
pub const HARD_EVIDENCE_KINDS: &[&str] = &["explorer", "advisor", "plan_review"];
