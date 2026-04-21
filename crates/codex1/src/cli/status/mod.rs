//! `codex1 status --json` — the unified mission projection.
//!
//! Owned by Phase B Unit 11. `status` and `close check` share the
//! `state::readiness` helpers (`derive_verdict`, `close_ready`,
//! `stop_allowed`); those are the single source of truth for any
//! judgment about mission readiness. This module never re-derives a
//! verdict — it only projects state and PLAN.yaml tasks into the
//! published shape.

mod next_action;
mod project;

use serde_json::json;

use crate::cli::close::check::ReadinessReport;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::{is_true_bare_no_mission, resolve_mission};
use crate::state;

pub fn run(ctx: &Ctx) -> CliResult<()> {
    match resolve_mission(&ctx.selector(), true) {
        Ok(paths) => {
            let state = state::load(&paths)?;
            if state.outcome.ratified && state.plan.locked {
                if let Err(err) = state::require_locked_plan_snapshot(&paths, &state) {
                    let data = project::build_invalid_state(&state, &err.to_string());
                    let env =
                        JsonOk::new(Some(state.mission_id.clone()), Some(state.revision), data);
                    println!("{}", env.to_pretty());
                    return Ok(());
                }
            }
            let tasks = next_action::load_plan_tasks(&paths).unwrap_or_default();
            let close_report = ReadinessReport::from_state_and_paths(&state, &paths);
            let data = project::build(&state, &tasks, close_report.ready);
            let env = JsonOk::new(Some(state.mission_id.clone()), Some(state.revision), data);
            println!("{}", env.to_pretty());
            Ok(())
        }
        Err(err @ CliError::MissionNotFound { ambiguous, .. }) => {
            // Explicit --mission <id> that doesn't resolve → error.
            // Bare `codex1 status` with nothing found → graceful "no
            // mission" projection so Ralph never blocks the shell.
            if ctx.mission.is_some()
                || ctx.repo_root.is_some()
                || ambiguous
                || !is_true_bare_no_mission(&err)
            {
                return Err(err);
            }
            let env = JsonOk::global(json!({
                "verdict": "needs_user",
                "stop": {
                    "allow": true,
                    "reason": "no_mission",
                    "message": "No mission resolved; stop is allowed.",
                },
                "foundation_only": true,
            }));
            println!("{}", env.to_pretty());
            Ok(())
        }
        Err(other) => Err(other),
    }
}
