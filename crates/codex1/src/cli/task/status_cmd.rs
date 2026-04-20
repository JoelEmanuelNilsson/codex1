//! `codex1 task status <id>` — read-only snapshot of a single task.

use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::{self, schema::TaskStatus};

use super::lifecycle::{deps_satisfied, deps_status_map, load_plan, status_str};

pub fn run(task_id: &str, ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let plan = load_plan(&paths)?;

    let Some(plan_task) = plan.get(task_id) else {
        return Err(CliError::TaskNotReady {
            message: format!("Task `{task_id}` not found in PLAN.yaml"),
        });
    };

    let record = state.tasks.get(task_id);
    let raw_status = record.map_or(TaskStatus::Pending, |r| r.status.clone());
    // Promote Pending→Ready on-demand when all deps are satisfied.
    let effective_status =
        if matches!(raw_status, TaskStatus::Pending) && deps_satisfied(plan_task, &state) {
            TaskStatus::Ready
        } else {
            raw_status
        };

    let deps_status = deps_status_map(&plan_task.depends_on, &state);
    let mut data = serde_json::Map::new();
    data.insert("task_id".into(), json!(task_id));
    data.insert("kind".into(), json!(plan_task.kind));
    data.insert("status".into(), json!(status_str(&effective_status)));
    data.insert("depends_on".into(), json!(plan_task.depends_on));
    data.insert("deps_status".into(), json!(deps_status));
    if let Some(r) = record {
        if let Some(s) = &r.started_at {
            data.insert("started_at".into(), json!(s));
        }
        if let Some(f) = &r.finished_at {
            data.insert("finished_at".into(), json!(f));
        }
        if let Some(p) = &r.proof_path {
            data.insert("proof_path".into(), json!(p));
        }
    }

    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        serde_json::Value::Object(data),
    );
    println!("{}", env.to_pretty());
    Ok(())
}
