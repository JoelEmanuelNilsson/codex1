//! `codex1 outcome check` — read-only OUTCOME.md validation.
//!
//! Reads `PLANS/<mission>/OUTCOME.md`, parses its YAML frontmatter,
//! and reports whether it is ready to ratify. Never mutates state.

use serde_json::json;

use crate::cli::outcome::emit::emit_outcome_incomplete;
use crate::cli::outcome::validate::validate_outcome;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::core::paths::ensure_artifact_file_read_safe;
use crate::state;

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    ensure_artifact_file_read_safe(&paths, &paths.outcome(), "OUTCOME.md")?;
    let report = validate_outcome(&paths.outcome(), &state.mission_id)?;

    if !report.ratifiable {
        // Print the OUTCOME_INCOMPLETE envelope (with context) and exit(1).
        // Returning a plain CliError would cause dispatch to re-print a
        // context-less envelope, so we emit-and-exit here instead.
        emit_outcome_incomplete(
            &state.mission_id,
            report.missing_fields,
            report.placeholders,
        );
    }

    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        json!({
            "ratifiable": true,
            "missing_fields": report.missing_fields,
            "placeholders": report.placeholders,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}
