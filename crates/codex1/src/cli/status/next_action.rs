//! Minimal wave derivation for `codex1 status`.
//!
//! Parses the PLAN.yaml tasks table just enough to determine the next
//! ready wave (and the next review task within it). Intentionally
//! local to `status` so this unit can ship without depending on the
//! full `plan waves` implementation.
//!
//! When PLAN.yaml is missing or unparseable, returns `Ok(None)` from
//! `load_plan_tasks` so the status projection can degrade gracefully.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::core::paths::MissionPaths;
use crate::state::schema::{MissionState, TaskStatus};

/// Light task shape extracted from PLAN.yaml. Only the fields needed
/// for wave / review-target derivation are parsed; unknown keys are
/// tolerated so the full PLAN.yaml schema can evolve without breaking
/// `status`.
#[derive(Debug, Clone, Deserialize)]
pub struct PlanTask {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub review_target: Option<ReviewTarget>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReviewTarget {
    #[serde(default)]
    pub tasks: Vec<String>,
}

#[derive(Deserialize)]
struct PlanEnvelope {
    #[serde(default)]
    tasks: Vec<PlanTask>,
}

/// Load tasks from PLAN.yaml. Returns `Ok(None)` when the file is
/// absent or unparseable (status must still emit a sane envelope in
/// those cases — downstream derivation treats missing plan data as
/// "no wave known").
pub fn load_plan_tasks(paths: &MissionPaths) -> Option<Vec<PlanTask>> {
    read_plan(&paths.plan())
}

fn read_plan(path: &Path) -> Option<Vec<PlanTask>> {
    let raw = std::fs::read_to_string(path).ok()?;
    let env: PlanEnvelope = serde_yaml::from_str(&raw).ok()?;
    Some(env.tasks)
}

/// Result of next-wave derivation.
#[derive(Debug, Clone)]
pub struct ReadyWave {
    pub wave_id: String,
    pub tasks: Vec<PlanTask>,
}

/// Derive the next ready wave. A task is "ready" when:
/// - every `depends_on` is complete or superseded in STATE.json, and
/// - the task's own status is not Complete/Superseded/InProgress.
///
/// Waves are numbered by topological depth from the root; the first
/// wave that contains at least one ready task is returned.
pub fn next_ready_wave(tasks: &[PlanTask], state: &MissionState) -> Option<ReadyWave> {
    if tasks.is_empty() {
        return None;
    }
    let depth = topological_depth(tasks)?;
    let mut waves: BTreeMap<u32, Vec<PlanTask>> = BTreeMap::new();
    for task in tasks {
        if let Some(d) = depth.get(&task.id) {
            waves.entry(*d).or_default().push(task.clone());
        }
    }
    for (idx, (_, tasks_in_wave)) in waves.iter().enumerate() {
        let ready: Vec<PlanTask> = tasks_in_wave
            .iter()
            .filter(|t| task_is_ready(t, state))
            .cloned()
            .collect();
        if !ready.is_empty() {
            return Some(ReadyWave {
                wave_id: format!("W{}", idx + 1),
                tasks: ready,
            });
        }
    }
    None
}

/// Review-kind tasks whose dependencies are satisfied (so the review
/// could fire now). Returns `(review_task_id, target_task_ids)` tuples.
pub fn ready_reviews(tasks: &[PlanTask], state: &MissionState) -> Vec<(String, Vec<String>)> {
    tasks
        .iter()
        .filter(|t| is_review_kind(t) && task_is_ready(t, state))
        .map(|t| (t.id.clone(), review_targets(t)))
        .collect()
}

/// Extract `review_target.tasks` (cloned), falling back to an empty
/// list when the task is a review kind without explicit targets.
fn review_targets(t: &PlanTask) -> Vec<String> {
    t.review_target
        .as_ref()
        .map_or_else(Vec::new, |r| r.tasks.clone())
}

/// Map reviews with a Dirty verdict to the list of target task ids
/// that still need repair. Relies on PLAN.yaml `review_target.tasks`
/// to resolve which executable tasks belong to a dirty review record.
pub fn dirty_repair_targets(tasks: &[PlanTask], state: &MissionState) -> Vec<String> {
    let mut targets: BTreeSet<String> = BTreeSet::new();
    for (review_task_id, review_record) in &state.reviews {
        if !matches!(
            review_record.verdict,
            crate::state::schema::ReviewVerdict::Dirty
        ) {
            continue;
        }
        // Resolve targets from PLAN.yaml. If no explicit targets,
        // fall back to the task's depends_on list (reviews are planned
        // immediately after their target tasks).
        let plan_task = tasks.iter().find(|t| &t.id == review_task_id);
        if let Some(t) = plan_task {
            let explicit = t
                .review_target
                .as_ref()
                .map_or(&[][..], |r| r.tasks.as_slice());
            if explicit.is_empty() {
                for id in &t.depends_on {
                    targets.insert(id.clone());
                }
            } else {
                for id in explicit {
                    targets.insert(id.clone());
                }
            }
        }
    }
    targets.into_iter().collect()
}

fn is_review_kind(t: &PlanTask) -> bool {
    matches!(t.kind.as_deref(), Some("review"))
}

/// True when all `depends_on` entries are complete/superseded AND the
/// task itself is not already finished or actively being worked.
fn task_is_ready(task: &PlanTask, state: &MissionState) -> bool {
    let deps_ok = task.depends_on.iter().all(|dep| {
        state
            .tasks
            .get(dep)
            .is_some_and(|r| matches!(r.status, TaskStatus::Complete | TaskStatus::Superseded))
    });
    if !deps_ok {
        return false;
    }
    match state.tasks.get(&task.id).map(|r| &r.status) {
        Some(TaskStatus::Complete | TaskStatus::Superseded | TaskStatus::InProgress) => false,
        // Pending, Ready, AwaitingReview, or absent-from-state all count
        // as "work left to do" once dependencies are satisfied.
        _ => true,
    }
}

/// Compute topological depth for every task. Returns None on cycle or
/// missing dependency (plan check owns those error codes; here we
/// just degrade to "no wave derivable").
fn topological_depth(tasks: &[PlanTask]) -> Option<BTreeMap<String, u32>> {
    let ids: BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    for task in tasks {
        for dep in &task.depends_on {
            if !ids.contains(dep.as_str()) {
                return None;
            }
        }
    }

    let mut depth: BTreeMap<String, u32> = BTreeMap::new();

    for root in tasks.iter().filter(|t| t.depends_on.is_empty()) {
        depth.insert(root.id.clone(), 0);
    }
    if depth.is_empty() {
        return None; // cycle or dependency-only graph
    }

    // Fixed-point relaxation; O(V*E) is fine for the small DAGs we see.
    let max_iters = tasks.len() * tasks.len() + 1;
    let mut iters = 0;
    loop {
        let mut changed = false;
        for task in tasks {
            if task.depends_on.is_empty() {
                continue;
            }
            let mut parent_depth = 0u32;
            let mut all_known = true;
            for dep in &task.depends_on {
                if let Some(d) = depth.get(dep) {
                    parent_depth = parent_depth.max(*d);
                } else {
                    all_known = false;
                    break;
                }
            }
            if !all_known {
                continue;
            }
            let new_depth = parent_depth + 1;
            let entry = depth.entry(task.id.clone()).or_insert(u32::MAX);
            if *entry == u32::MAX || *entry < new_depth {
                *entry = new_depth;
                changed = true;
            }
        }
        iters += 1;
        if !changed {
            break;
        }
        if iters > max_iters {
            return None; // cycle guard
        }
    }

    if depth.len() != tasks.len() {
        return None;
    }
    Some(depth)
}
