//! `codex1 init --mission <id> --title <title>` — scaffold a new mission.
//!
//! Sequence:
//!
//! 1. Resolve repo root (`--repo-root` or cwd).
//! 2. Validate the mission id (`MISSION_ID_INVALID` on failure).
//! 3. Build `MissionPaths`; refuse if `PLANS/<id>/` already exists
//!    (`MISSION_EXISTS`, exit 3).
//! 4. Create the mission directory.
//! 5. Write `OUTCOME-LOCK.md` template (frontmatter + three required
//!    headings; `lock_status: draft`).
//! 6. Write `PROGRAM-BLUEPRINT.md` with empty `tasks: []` between markers.
//! 7. `StateStore::init` writes `STATE.json` (`state_revision=1`,
//!    `phase: clarify`) and appends the initial `mission_initialized` event
//!    (seq=1) under the per-mission fs2 lock.
//! 8. Emit the success envelope.

use serde_json::json;

use crate::envelope;
use crate::error::CliError;
use crate::fs_atomic;
use crate::mission::{self, resolve_mission};
use crate::state::StateStore;

use super::{Cli, emit_error, emit_success, resolve_repo};

const SCHEMA: &str = "codex1.init.v1";

pub fn cmd_init(cli: &Cli, mission: &str, title: &str) -> i32 {
    match run(cli, mission, title) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run(cli: &Cli, mission: &str, title: &str) -> Result<serde_json::Value, CliError> {
    if title.trim().is_empty() {
        return Err(CliError::Internal {
            message: "--title must be non-empty".into(),
        });
    }
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;

    if paths.mission_dir.exists() {
        return Err(CliError::MissionExists {
            path: paths.mission_dir.display().to_string(),
        });
    }

    std::fs::create_dir_all(&paths.mission_dir).map_err(|e| CliError::Io {
        path: paths.mission_dir.display().to_string(),
        source: e,
    })?;

    let now = super::now_rfc3339();

    // OUTCOME-LOCK.md
    let lock_body = render_lock_template(mission, title, &now);
    fs_atomic::atomic_write(&paths.outcome_lock(), lock_body.as_bytes()).map_err(|e| {
        CliError::Io {
            path: paths.outcome_lock().display().to_string(),
            source: e,
        }
    })?;

    // PROGRAM-BLUEPRINT.md
    let blueprint_body = render_blueprint_template(title);
    fs_atomic::atomic_write(&paths.program_blueprint(), blueprint_body.as_bytes()).map_err(
        |e| CliError::Io {
            path: paths.program_blueprint().display().to_string(),
            source: e,
        },
    )?;

    // STATE.json + events.jsonl (takes fs2 lock internally)
    let store = StateStore::new(paths.mission_dir.clone());
    let state = store.init(mission)?;

    let created_paths = serde_json::json!({
        "outcome_lock": paths.outcome_lock().display().to_string(),
        "program_blueprint": paths.program_blueprint().display().to_string(),
        "state": store.state_path().display().to_string(),
        "events": store.events_path().display().to_string(),
    });

    Ok(envelope::success(
        SCHEMA,
        &json!({
            "mission_id": state.mission_id,
            "title": title,
            "mission_dir": paths.mission_dir.display().to_string(),
            "state_revision": state.state_revision,
            "phase": state.phase,
            "created": created_paths,
            "message": format!("Initialised mission {} at {}", state.mission_id, paths.mission_dir.display()),
        }),
    ))
}

fn render_lock_template(mission: &str, title: &str, now: &str) -> String {
    // Use serde_yaml for the frontmatter so special characters in `title`
    // are escaped correctly.
    let fm = mission::lock::Frontmatter {
        mission_id: mission.to_string(),
        title: title.to_string(),
        lock_status: mission::lock::LockStatus::Draft,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    };
    let fm_yaml = serde_yaml::to_string(&fm).unwrap_or_else(|_| String::new());
    format!(
        "---\n\
{fm_yaml}---\n\
\n\
# Outcome Lock: {title}\n\
\n\
## Destination\n\
<!-- What does \"done\" look like from the user's perspective? Written during $clarify. -->\n\
\n\
## Constraints\n\
<!-- Hard limits, budgets, surfaces that must not change, invariants that must hold. -->\n\
\n\
## Success Criteria\n\
<!-- Observable, checkable statements that make \"done\" testable. -->\n"
    )
}

fn render_blueprint_template(title: &str) -> String {
    // Use concatenation rather than `\<newline>` continuations because the
    // continuation form swallows leading whitespace on the next line and
    // breaks YAML indentation.
    let mut s = String::new();
    s.push_str("# Program Blueprint: ");
    s.push_str(title);
    s.push_str("\n\n");
    s.push_str("<!-- codex1:plan-dag:start -->\n");
    s.push_str("planning:\n");
    s.push_str("  requested_level: light\n");
    s.push_str("  risk_floor: light\n");
    s.push_str("  effective_level: light\n");
    s.push_str("  graph_revision: 1\n");
    s.push('\n');
    s.push_str("tasks: []\n");
    s.push_str("<!-- codex1:plan-dag:end -->\n");
    s
}
