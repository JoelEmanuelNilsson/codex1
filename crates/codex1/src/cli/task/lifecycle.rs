//! Task lifecycle helpers: plan parsing, dependency resolution, wave derivation.
//!
//! Kept private to the `task` module. Other units derive their own views
//! from PLAN.yaml + STATE.json rather than importing this.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use serde::Deserialize;
use serde_yaml::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::core::error::CliError;
use crate::core::paths::{ensure_artifact_file_read_safe, MissionPaths};
use crate::state::schema::{MissionState, TaskId, TaskRecord, TaskStatus};

/// Minimal view of a single task's fields we care about in the task
/// lifecycle. Extra fields are ignored so we are tolerant of schema
/// evolution by sibling units.
#[derive(Debug, Clone, Deserialize)]
pub struct PlanTask {
    pub id: TaskId,
    #[serde(default)]
    pub title: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    #[serde(default)]
    pub spec: Option<String>,
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    #[serde(default)]
    pub unknown_side_effects: bool,
    #[serde(default)]
    pub proof: Vec<String>,
    #[serde(default)]
    pub review_target: Option<ReviewTarget>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewTarget {
    #[serde(default)]
    pub tasks: Vec<TaskId>,
}

fn default_kind() -> String {
    "code".to_string()
}

/// Parsed PLAN.yaml. Only the task list is required for this unit.
#[derive(Debug, Clone)]
pub struct ParsedPlan {
    pub tasks: Vec<PlanTask>,
    /// Index by id for O(1) lookup.
    pub by_id: BTreeMap<TaskId, PlanTask>,
}

impl ParsedPlan {
    pub fn get(&self, id: &str) -> Option<&PlanTask> {
        self.by_id.get(id)
    }

    /// Find the review task (if any) whose `review_target.tasks` includes `id`.
    pub fn review_task_targeting(&self, id: &str) -> Option<&PlanTask> {
        self.tasks.iter().find(|t| {
            t.kind == "review"
                && t.review_target
                    .as_ref()
                    .is_some_and(|rt| rt.tasks.iter().any(|tid| tid == id))
        })
    }
}

/// Load and parse PLAN.yaml from disk. Returns `PlanInvalid` if the
/// top-level shape is unreadable.
pub fn load_plan(paths: &MissionPaths, state: &MissionState) -> Result<ParsedPlan, CliError> {
    crate::state::require_locked_plan_snapshot(paths, state)?;
    let plan_path = paths.plan();
    ensure_artifact_file_read_safe(paths, &plan_path, "PLAN.yaml")?;
    let raw = fs::read_to_string(&plan_path).map_err(|err| CliError::PlanInvalid {
        message: format!("Failed to read PLAN.yaml at {}: {err}", plan_path.display()),
        hint: Some("Run `codex1 plan scaffold` to create a plan skeleton.".to_string()),
    })?;
    let doc: Value = serde_yaml::from_str(&raw).map_err(|err| CliError::PlanInvalid {
        message: format!("PLAN.yaml is not valid YAML: {err}"),
        hint: None,
    })?;
    let tasks_val = doc.get("tasks").cloned().unwrap_or(Value::Sequence(vec![]));
    let tasks: Vec<PlanTask> =
        serde_yaml::from_value(tasks_val).map_err(|err| CliError::PlanInvalid {
            message: format!("PLAN.yaml `tasks` is malformed: {err}"),
            hint: Some("Each task needs at least `id`, `depends_on`, and `kind`.".to_string()),
        })?;
    let mut by_id = BTreeMap::new();
    for t in &tasks {
        if by_id.insert(t.id.clone(), t.clone()).is_some() {
            return Err(CliError::PlanInvalid {
                message: format!("Duplicate task id `{}` in PLAN.yaml", t.id),
                hint: None,
            });
        }
    }
    Ok(ParsedPlan { tasks, by_id })
}

/// A `TaskRecord` reconciled with on-demand readiness (so `Pending` may
/// promote to `Ready` without mutating state).
#[derive(Debug, Clone)]
pub struct EffectiveTask {
    pub id: TaskId,
    pub kind: String,
    pub status: TaskStatus,
    pub depends_on: Vec<TaskId>,
    pub exclusive_resources: Vec<String>,
    pub unknown_side_effects: bool,
}

/// Project a unified view of PLAN.yaml tasks joined against STATE.json.
/// Tasks that are `Pending` in state but have all deps satisfied are
/// reported as `Ready` here (the promotion is computed, not persisted).
pub fn effective_tasks(plan: &ParsedPlan, state: &MissionState) -> Vec<EffectiveTask> {
    plan.tasks
        .iter()
        .map(|t| {
            let record = state.tasks.get(&t.id);
            let raw_status = record.map_or(TaskStatus::Pending, |r| r.status.clone());
            let status = if matches!(raw_status, TaskStatus::Pending) && deps_satisfied(t, state) {
                TaskStatus::Ready
            } else {
                raw_status
            };
            EffectiveTask {
                id: t.id.clone(),
                kind: t.kind.clone(),
                status,
                depends_on: t.depends_on.clone(),
                exclusive_resources: t.exclusive_resources.clone(),
                unknown_side_effects: t.unknown_side_effects,
            }
        })
        .collect()
}

/// True iff every dep of `task` is "satisfied enough" for `task` to be
/// ready. A `review` task's deps are satisfied when the dep is in
/// `AwaitingReview` or `Complete` (so the review can run against fresh
/// work). Superseded deps are history after a replan, not live readiness.
pub fn deps_satisfied(task: &PlanTask, state: &MissionState) -> bool {
    let is_review = task.kind == "review";
    task.depends_on.iter().all(|dep| {
        let Some(r) = state.tasks.get(dep) else {
            return false;
        };
        match &r.status {
            TaskStatus::Complete => true,
            TaskStatus::AwaitingReview if is_review => true,
            _ => false,
        }
    })
}

/// Map each dep id to its current `TaskStatus` string (or `pending` if
/// absent from state). Used by `task status`.
pub fn deps_status_map(deps: &[TaskId], state: &MissionState) -> BTreeMap<String, String> {
    deps.iter()
        .map(|d| {
            let s = state
                .tasks
                .get(d)
                .map_or("pending", |r| status_str(&r.status))
                .to_string();
            (d.clone(), s)
        })
        .collect()
}

pub fn status_str(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Ready => "ready",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::AwaitingReview => "awaiting_review",
        TaskStatus::Complete => "complete",
        TaskStatus::Superseded => "superseded",
    }
}

/// The current ready wave: the set of tasks whose deps are all complete
/// and whose effective status is `Ready`, restricted to the first
/// topological depth that has ready work. Preserves PLAN.yaml order.
pub fn ready_wave(effective: &[EffectiveTask]) -> Vec<EffectiveTask> {
    let Some(depth) = topological_depth(effective) else {
        return Vec::new();
    };
    let Some(first_ready_depth) = effective
        .iter()
        .filter(|t| t.kind != "review" && matches!(t.status, TaskStatus::Ready))
        .filter_map(|t| depth.get(&t.id).copied())
        .min()
    else {
        return Vec::new();
    };
    effective
        .iter()
        .filter(|t| t.kind != "review" && matches!(t.status, TaskStatus::Ready))
        .filter(|t| depth.get(&t.id).copied() == Some(first_ready_depth))
        .cloned()
        .collect()
}

fn topological_depth(effective: &[EffectiveTask]) -> Option<BTreeMap<String, usize>> {
    let ids: BTreeSet<&str> = effective.iter().map(|t| t.id.as_str()).collect();
    for task in effective {
        for dep in &task.depends_on {
            if !ids.contains(dep.as_str()) {
                return None;
            }
        }
    }
    let mut depth = BTreeMap::new();
    for root in effective.iter().filter(|t| t.depends_on.is_empty()) {
        depth.insert(root.id.clone(), 1usize);
    }
    let max_iters = effective
        .len()
        .saturating_mul(effective.len())
        .saturating_add(1);
    for _ in 0..max_iters {
        let mut changed = false;
        for task in effective {
            if task.depends_on.is_empty() {
                continue;
            }
            let Some(parent_depth) = task
                .depends_on
                .iter()
                .map(|dep| depth.get(dep).copied())
                .collect::<Option<Vec<_>>>()
                .and_then(|depths| depths.into_iter().max())
            else {
                continue;
            };
            let next = parent_depth.saturating_add(1);
            if depth.get(&task.id).copied().unwrap_or(0) < next {
                depth.insert(task.id.clone(), next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if depth.len() == effective.len() {
        Some(depth)
    } else {
        None
    }
}

/// Any task in `AwaitingReview` — we surface these so `task next` can
/// route to the matching review task.
pub fn awaiting_review_targets(effective: &[EffectiveTask]) -> Vec<TaskId> {
    effective
        .iter()
        .filter(|t| matches!(t.status, TaskStatus::AwaitingReview))
        .map(|t| t.id.clone())
        .collect()
}

/// Find the first ready review task whose `review_target.tasks` includes
/// at least one id from `awaiting`. Returns `(review_task, covered_targets)`.
pub fn next_ready_review<'a>(
    plan: &'a ParsedPlan,
    effective: &[EffectiveTask],
    awaiting: &[TaskId],
) -> Option<(&'a PlanTask, Vec<TaskId>)> {
    let awaiting_set: BTreeSet<&TaskId> = awaiting.iter().collect();
    for et in effective {
        if !matches!(et.status, TaskStatus::Ready) {
            continue;
        }
        let Some(pt) = plan.get(&et.id) else { continue };
        if pt.kind != "review" {
            continue;
        }
        let Some(rt) = &pt.review_target else {
            continue;
        };
        let covered: Vec<TaskId> = rt
            .tasks
            .iter()
            .filter(|t| awaiting_set.contains(t))
            .cloned()
            .collect();
        if !covered.is_empty() {
            return Some((pt, covered));
        }
    }
    None
}

/// RFC3339 timestamp for state transitions.
pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

/// Record the effective status for a task, defaulting to pending if
/// absent. Used when we want to merge in a fresh TaskRecord.
pub fn ensure_task_record<'a>(state: &'a mut MissionState, id: &str) -> &'a mut TaskRecord {
    state
        .tasks
        .entry(id.to_string())
        .or_insert_with(|| TaskRecord {
            id: id.to_string(),
            status: TaskStatus::Pending,
            started_at: None,
            finished_at: None,
            proof_path: None,
            superseded_by: None,
        })
}

/// True iff every task in the plan is currently Complete or Superseded
/// (reads state directly since effective view would promote Pending→Ready).
pub fn all_tasks_terminal(plan: &ParsedPlan, state: &MissionState) -> bool {
    if plan.tasks.is_empty() {
        return false;
    }
    plan.tasks.iter().all(|t| {
        state
            .tasks
            .get(&t.id)
            .is_some_and(|r| matches!(r.status, TaskStatus::Complete | TaskStatus::Superseded))
    })
}
