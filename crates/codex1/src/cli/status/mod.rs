//! `codex1 status` stub — owned by Phase B Unit 11.
//!
//! Foundation emits a minimal status envelope so Ralph can rely on the
//! command existing from day one. Phase B replaces this with the
//! full unified projection and shares the readiness helper in
//! `state::readiness`.

use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::{self, readiness};

pub fn run(ctx: &Ctx) -> CliResult<()> {
    // If a mission can be resolved, emit a minimal but truthful projection
    // based on the current state. Phase B broadens this into the full
    // unified status shape; the `verdict` and `stop.allow` fields are
    // already derived from the shared readiness helper, so the Ralph
    // contract is honored from Foundation onward.
    match resolve_mission(&ctx.selector(), true) {
        Ok(paths) => {
            let state = state::load(&paths)?;
            let verdict = readiness::derive_verdict(&state);
            let stop_allow = readiness::stop_allowed(&state);
            let env = JsonOk::new(
                Some(state.mission_id.clone()),
                Some(state.revision),
                json!({
                    "phase": state.phase,
                    "verdict": verdict.as_str(),
                    "loop": state.loop_,
                    "close_ready": readiness::close_ready(&state),
                    "stop": {
                        "allow": stop_allow,
                        "reason": if stop_allow { "idle" } else { "active_loop" },
                        "message": if stop_allow {
                            "Stop is allowed."
                        } else {
                            "Active loop in progress. Run $close to pause."
                        },
                    },
                    "foundation_only": true,
                    "note": "Phase B Unit 11 replaces this projection with the full unified status view."
                }),
            );
            println!("{}", env.to_pretty());
            Ok(())
        }
        Err(err @ CliError::MissionNotFound { .. }) => {
            // If the caller explicitly passed --mission <id>, an unresolved
            // mission is an error. If they did not, auto-discovery failing
            // is a normal "nothing in progress" state — emit a graceful
            // stop-allowed projection so Ralph never blocks the shell.
            if ctx.mission.is_some() {
                Err(err)
            } else {
                let env = JsonOk::global(json!({
                    "verdict": "needs_user",
                    "stop": {
                        "allow": true,
                        "reason": "no_mission",
                        "message": "No mission resolved; Stop is allowed."
                    },
                    "foundation_only": true,
                }));
                println!("{}", env.to_pretty());
                Ok(())
            }
        }
        Err(other) => Err(other),
    }
}
