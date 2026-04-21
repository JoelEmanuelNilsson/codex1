//! `codex1 plan waves` — derive wave ordering from the DAG plus current
//! task state. Waves are never stored; they are recomputed on every call.
//!
//! Algorithm (in short): parse PLAN.yaml, build DAG depth on the task
//! graph, use STATE.json task statuses only to compute
//! `current_ready_wave` and `all_tasks_complete`. The wave *list* itself
//! is structural (depth-driven), not state-filtered.

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::{ensure_artifact_file_read_safe, MissionPaths};
use crate::state::{self, MissionState, TaskStatus};

/// Minimal task shape needed for wave derivation and graph emission. Kept
/// local so this module does not depend on Unit 4's parser.
#[derive(Debug, Clone, Deserialize)]
pub struct ParsedTask {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub review_target: Option<ReviewTarget>,
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    #[serde(default)]
    pub unknown_side_effects: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReviewTarget {
    #[serde(default)]
    pub tasks: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ParsedPlan {
    #[serde(default)]
    tasks: Vec<ParsedTask>,
}

/// Load and parse PLAN.yaml into the minimal task shape used by waves/graph.
pub fn load_plan_tasks(paths: &MissionPaths, state: &MissionState) -> CliResult<Vec<ParsedTask>> {
    state::require_locked_plan_snapshot(paths, state)?;
    let plan_path = paths.plan();
    ensure_artifact_file_read_safe(paths, &plan_path, "PLAN.yaml")?;
    if !plan_path.is_file() {
        return Err(CliError::PlanInvalid {
            message: format!("PLAN.yaml not found at {}", plan_path.display()),
            hint: Some("Run `codex1 plan scaffold --level <level>` first.".to_string()),
        });
    }
    let raw = std::fs::read_to_string(plan_path)?;
    let plan: ParsedPlan = serde_yaml::from_str(&raw).map_err(|err| CliError::PlanInvalid {
        message: format!("Failed to parse PLAN.yaml: {err}"),
        hint: Some("Check YAML syntax and task shape (id, depends_on, ...).".to_string()),
    })?;
    Ok(plan.tasks)
}

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;

    // If the plan is not locked yet, return empty waves with a note rather
    // than surfacing a PLAN_INVALID from parsing work-in-progress YAML.
    if !state.plan.locked {
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "waves": [],
                "current_ready_wave": Value::Null,
                "all_tasks_complete": false,
                "note": "Plan is not locked yet; waves cannot be derived.",
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let tasks = load_plan_tasks(&paths, &state)?;
    let waves = derive_waves(&tasks, &state.tasks)?;

    let globally_blocked =
        state.replan.triggered || state::readiness::has_current_dirty_review(&state);
    let current_ready_wave = if globally_blocked {
        None
    } else {
        waves
            .iter()
            .find(|w| wave_is_current_ready(w, &tasks, &state.tasks))
            .map(|w| w.wave_id.clone())
    };

    let all_tasks_complete = !tasks.is_empty()
        && tasks.iter().all(|t| {
            matches!(
                state
                    .tasks
                    .get(&t.id)
                    .map_or(TaskStatus::Pending, |r| r.status.clone()),
                TaskStatus::Complete | TaskStatus::Superseded
            )
        });

    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        json!({
            "waves": waves.iter().map(Wave::to_json).collect::<Vec<_>>(),
            "current_ready_wave": current_ready_wave,
            "all_tasks_complete": all_tasks_complete,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Wave {
    pub wave_id: String,
    pub tasks: Vec<String>,
    pub parallel_safe: bool,
    pub blockers: Vec<String>,
}

impl Wave {
    fn to_json(&self) -> Value {
        json!({
            "wave_id": self.wave_id,
            "tasks": self.tasks,
            "parallel_safe": self.parallel_safe,
            "blockers": self.blockers,
        })
    }
}

/// Compute DAG-depth-based waves plus per-wave parallel-safety data. The
/// wave list itself is structural; task state only affects
/// `current_ready_wave` (computed by the caller).
pub fn derive_waves(
    tasks: &[ParsedTask],
    state_tasks: &BTreeMap<String, crate::state::TaskRecord>,
) -> CliResult<Vec<Wave>> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }
    // Skip tasks that are Superseded in state; they are history, not part
    // of the live DAG.
    let live: Vec<&ParsedTask> = tasks
        .iter()
        .filter(|t| {
            !matches!(
                state_tasks.get(&t.id).map(|r| r.status.clone()),
                Some(TaskStatus::Superseded)
            )
        })
        .collect();

    let ids: BTreeSet<&str> = live.iter().map(|t| t.id.as_str()).collect();

    // Validate every depends_on points at a known live task. Missing deps
    // are a PLAN.yaml integrity failure — Unit 4's `plan check` catches
    // them too, but we surface a clear error here in case a user runs
    // `plan waves` directly.
    let all_ids: BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    for t in &live {
        for dep in &t.depends_on {
            if !ids.contains(dep.as_str()) {
                if all_ids.contains(dep.as_str())
                    && matches!(
                        state_tasks.get(dep).map(|r| r.status.clone()),
                        Some(TaskStatus::Superseded)
                    )
                {
                    continue;
                }
                return Err(CliError::DagMissingDep {
                    message: format!("Task {} depends on unknown task {dep}", t.id),
                });
            }
        }
    }

    // Compute depth for every task via memoized DFS. Cycles cause a
    // DAG_CYCLE error — again this is redundant with `plan check` but
    // keeps waves fail-closed.
    let mut depth: BTreeMap<String, usize> = BTreeMap::new();
    let mut visiting: BTreeSet<String> = BTreeSet::new();
    let by_id: BTreeMap<&str, &ParsedTask> = live.iter().map(|t| (t.id.as_str(), *t)).collect();
    for t in &live {
        compute_depth(&t.id, &by_id, &mut depth, &mut visiting)?;
    }

    // Bucket tasks by depth. Keep plan-order within each bucket for
    // deterministic output.
    let mut buckets: BTreeMap<usize, Vec<&ParsedTask>> = BTreeMap::new();
    for t in &live {
        let d = depth[&t.id];
        buckets.entry(d).or_default().push(*t);
    }

    let mut waves = Vec::with_capacity(buckets.len());
    for (idx, (_d, bucket)) in buckets.into_iter().enumerate() {
        let wave_id = format!("W{}", idx + 1);
        let tasks: Vec<String> = bucket.iter().map(|t| t.id.clone()).collect();
        let (parallel_safe, blockers) = compute_parallel_safety(&bucket);
        waves.push(Wave {
            wave_id,
            tasks,
            parallel_safe,
            blockers,
        });
    }
    Ok(waves)
}

fn compute_depth(
    id: &str,
    by_id: &BTreeMap<&str, &ParsedTask>,
    depth: &mut BTreeMap<String, usize>,
    visiting: &mut BTreeSet<String>,
) -> CliResult<usize> {
    if let Some(d) = depth.get(id) {
        return Ok(*d);
    }
    if !visiting.insert(id.to_string()) {
        return Err(CliError::DagCycle {
            message: format!("Cycle detected at task {id}"),
        });
    }
    let task = by_id.get(id).ok_or_else(|| CliError::DagMissingDep {
        message: format!("Unknown task {id}"),
    })?;
    let mut d = 0usize;
    for dep in &task.depends_on {
        if !by_id.contains_key(dep.as_str()) {
            continue;
        }
        let parent = compute_depth(dep, by_id, depth, visiting)?;
        if parent + 1 > d {
            d = parent + 1;
        }
    }
    depth.insert(id.to_string(), d);
    visiting.remove(id);
    Ok(d)
}

/// Parallel-safe unless two tasks share an exclusive resource or any task
/// has `unknown_side_effects`. Blockers are deduplicated for readability.
fn compute_parallel_safety(bucket: &[&ParsedTask]) -> (bool, Vec<String>) {
    let mut blockers: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    let mut resource_owners: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for task in bucket {
        for res in &task.exclusive_resources {
            resource_owners
                .entry(res.as_str())
                .or_default()
                .push(task.id.as_str());
        }
    }
    for (res, owners) in &resource_owners {
        if owners.len() > 1 {
            let key = format!("exclusive_resource:{res}");
            if seen.insert(key.clone()) {
                blockers.push(key);
            }
        }
    }

    for task in bucket {
        if task.unknown_side_effects {
            let key = format!("unknown_side_effects:{}", task.id);
            if seen.insert(key.clone()) {
                blockers.push(key);
            }
        }
    }

    (blockers.is_empty(), blockers)
}

/// True when every task in the wave is either Pending or explicitly
/// Ready — i.e. the wave has not started yet. Waves that contain
/// InProgress, AwaitingReview, Complete, or Superseded tasks do not
/// match; the "current ready" wave is the next *untouched* one.
fn wave_is_current_ready(
    wave: &Wave,
    plan_tasks: &[ParsedTask],
    state_tasks: &BTreeMap<String, crate::state::TaskRecord>,
) -> bool {
    let by_id: BTreeMap<&str, &ParsedTask> =
        plan_tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    wave.tasks.iter().any(|id| {
        by_id
            .get(id.as_str())
            .is_some_and(|task| task_is_actionable(task, state_tasks))
    })
}

fn task_is_actionable(
    task: &ParsedTask,
    state_tasks: &BTreeMap<String, crate::state::TaskRecord>,
) -> bool {
    let is_review = matches!(task.kind.as_deref(), Some("review"));
    let targets: BTreeSet<&str> = task
        .review_target
        .as_ref()
        .map(|target| target.tasks.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let deps_ok = task.depends_on.iter().all(|dep| {
        state_tasks.get(dep).is_some_and(|r| {
            matches!(r.status, TaskStatus::Complete)
                || (is_review
                    && targets.contains(dep.as_str())
                    && matches!(r.status, TaskStatus::AwaitingReview))
        })
    });
    if !deps_ok {
        return false;
    }
    matches!(
        state_tasks
            .get(&task.id)
            .map_or(TaskStatus::Pending, |r| r.status.clone()),
        TaskStatus::Pending | TaskStatus::Ready
    )
}
