//! `codex1 task packet <id>` — worker-subagent prompt packet.

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state;

use super::lifecycle::load_plan;
use super::worker_packet::build_packet;

pub fn run(task_id: &str, ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let plan = load_plan(&paths)?;

    let Some(plan_task) = plan.get(task_id) else {
        return Err(CliError::TaskNotReady {
            message: format!("Task `{task_id}` not found in PLAN.yaml"),
        });
    };

    let data = build_packet(&paths, plan_task)?;
    let env = JsonOk::new(Some(state.mission_id.clone()), Some(state.revision), data);
    println!("{}", env.to_pretty());
    Ok(())
}
