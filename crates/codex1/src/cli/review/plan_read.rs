//! Minimal PLAN.yaml reader for review commands.
//!
//! We only pull what review needs: the review task's `kind`, its review
//! target task ids, review profiles, and per-target `write_paths`.
//! Anything richer belongs to `plan check` (Phase B Unit 3).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::core::error::CliError;

/// One entry in `tasks:`. Only the fields review cares about.
#[derive(Debug, Clone, Deserialize)]
pub struct PlanTask {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub spec: Option<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
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
struct PlanDoc {
    #[serde(default)]
    tasks: Vec<PlanTask>,
}

/// Load PLAN.yaml and return tasks indexed by id. Missing plan file is an
/// error (planned reviews require a plan to exist).
pub fn load_tasks(plan_path: &Path) -> Result<BTreeMap<String, PlanTask>, CliError> {
    if !plan_path.is_file() {
        return Err(CliError::PlanInvalid {
            message: format!("PLAN.yaml missing at {}", plan_path.display()),
            hint: Some("Run `codex1 plan scaffold --level <level>` first.".to_string()),
        });
    }
    let raw = std::fs::read_to_string(plan_path)?;
    let doc: PlanDoc = serde_yaml::from_str(&raw).map_err(|err| CliError::PlanInvalid {
        message: format!("Failed to parse PLAN.yaml: {err}"),
        hint: None,
    })?;
    let mut out = BTreeMap::new();
    for task in doc.tasks {
        out.insert(task.id.clone(), task);
    }
    Ok(out)
}

/// Fetch the review task by id, erroring if missing or not `kind: review`.
pub fn fetch_review_task(
    tasks: &BTreeMap<String, PlanTask>,
    task_id: &str,
) -> Result<PlanTask, CliError> {
    let Some(task) = tasks.get(task_id) else {
        return Err(CliError::PlanInvalid {
            message: format!("Task {task_id} not found in PLAN.yaml"),
            hint: None,
        });
    };
    let kind = task.kind.as_deref().unwrap_or("");
    if kind != "review" {
        return Err(CliError::PlanInvalid {
            message: format!("Task {task_id} has kind `{kind}`, not `review`"),
            hint: Some("Review commands only operate on tasks with `kind: review`.".to_string()),
        });
    }
    Ok(task.clone())
}

/// Targets declared on the review task. Empty `review_target.tasks` is an
/// error — a review without targets has nothing to classify.
pub fn review_targets(task: &PlanTask) -> Result<Vec<String>, CliError> {
    let targets = task
        .review_target
        .as_ref()
        .map(|t| t.tasks.clone())
        .unwrap_or_default();
    if targets.is_empty() {
        return Err(CliError::PlanInvalid {
            message: format!("Review task {} has no review_target.tasks", task.id),
            hint: Some(
                "Add a `review_target: { tasks: [T…] }` block to the review task.".to_string(),
            ),
        });
    }
    Ok(targets)
}
