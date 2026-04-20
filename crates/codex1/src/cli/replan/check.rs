//! `codex1 replan check` — read-only trigger probe.

use serde_json::{json, Value};

use crate::cli::replan::triggers;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::state;

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;

    let (required, reason) = match triggers::breach(&state) {
        Some((target, count)) => (
            true,
            Value::String(format!(
                "Target {target} has {count} consecutive dirty reviews"
            )),
        ),
        None => (false, Value::Null),
    };

    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        json!({
            "required": required,
            "reason": reason,
            "consecutive_dirty_by_target": state.replan.consecutive_dirty_by_target,
            "triggered_already": state.replan.triggered,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}
