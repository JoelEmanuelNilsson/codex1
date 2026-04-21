//! `codex1 plan scaffold` — write a PLAN.yaml skeleton for the chosen level.
//!
//! Requires a prior `plan choose-level` so the `--level` argument can be
//! matched against the recorded requested/effective level. Level mismatch
//! returns `PLAN_INVALID`. Does **not** set `plan.locked`; that transition
//! belongs to `plan check` (Phase B Unit 4).

use serde_json::json;

use crate::cli::plan::choose_level::{level_str, parse_level};
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::{ensure_artifact_parent_write_safe, MissionPaths};
use crate::state::{self, fs_atomic::atomic_write, MissionState, PlanLevel};

/// Handle `codex1 plan scaffold --level <level>`.
pub fn run(level_raw: String, ctx: &Ctx) -> CliResult<()> {
    let level = parse_level(&level_raw)?;
    let paths = resolve_mission(&ctx.selector(), true)?;

    let plan_relpath = relative_plan_path(&paths);

    if ctx.dry_run {
        let state = state::load(&paths)?;
        if let Some(expected) = ctx.expect_revision {
            if expected != state.revision {
                return Err(CliError::RevisionConflict {
                    expected,
                    actual: state.revision,
                });
            }
        }
        check_level_matches_recorded(&state, &level)?;
        let env = JsonOk::new(
            Some(paths.mission_id.clone()),
            Some(state.revision),
            json!({
                "dry_run": true,
                "wrote": plan_relpath,
                "specs_created": Vec::<String>::new(),
                "level": level_str(&level),
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    // Idempotently ensure specs/reviews dirs exist (normally created by init).
    std::fs::create_dir_all(paths.specs_dir())?;
    std::fs::create_dir_all(paths.reviews_dir())?;

    let payload_event = json!({
        "level": level_str(&level),
        "wrote": plan_relpath.clone(),
    });

    let plan_path = paths.plan();
    let mut skeleton = String::new();
    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        "plan.scaffold",
        payload_event,
        |state| {
            // Validate level match under the mutation lock to avoid a
            // TOCTOU window between load and mutate.
            check_level_matches_recorded(state, &level)?;
            skeleton = render_skeleton(state, &level);
            // No state-field mutations; the revision bump marks the
            // scaffold event for audit. `plan.locked` stays false.
            Ok(())
        },
    )?;

    ensure_artifact_parent_write_safe(&paths, &plan_path)?;
    atomic_write(&plan_path, skeleton.as_bytes())?;

    let env = JsonOk::new(
        Some(paths.mission_id.clone()),
        Some(mutation.new_revision),
        json!({
            "wrote": plan_relpath,
            "specs_created": Vec::<String>::new(),
            "level": level_str(&level),
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

/// Return `PLAN_INVALID` unless the recorded effective (or requested)
/// level matches `level`.
fn check_level_matches_recorded(state: &MissionState, level: &PlanLevel) -> CliResult<()> {
    let recorded = state
        .plan
        .effective_level
        .as_ref()
        .or(state.plan.requested_level.as_ref())
        .ok_or_else(|| CliError::PlanInvalid {
            message: "no planning level recorded; run `codex1 plan choose-level` first".to_string(),
            hint: Some("Example: `codex1 plan choose-level --level medium --json`.".to_string()),
        })?;
    if recorded != level {
        let recorded_str = level_str(recorded);
        return Err(CliError::PlanInvalid {
            message: "--level does not match recorded plan level".to_string(),
            hint: Some(format!(
                "Recorded effective level is `{recorded_str}`. Re-run with --level {recorded_str}, or re-run `plan choose-level` to change it."
            )),
        });
    }
    Ok(())
}

/// Compose the skeleton PLAN.yaml for the given level. `mission_id` and
/// `planning_level` reflect the STATE.json-recorded values. Hard plans
/// include explorer/advisor/plan_reviewer evidence markers.
fn render_skeleton(state: &MissionState, level: &PlanLevel) -> String {
    let mission_id = &state.mission_id;
    let requested = state
        .plan
        .requested_level
        .as_ref()
        .map_or("[codex1-fill:planning_level]", level_str);
    let effective = level_str(level);

    let evidence_block = match level {
        PlanLevel::Hard => {
            "  evidence:\n\
             \x20   - kind: explorer\n\
             \x20     summary: '[codex1-fill:explorer_evidence]'\n\
             \x20     required_for_hard: true\n\
             \x20   - kind: advisor\n\
             \x20     summary: '[codex1-fill:advisor_evidence]'\n\
             \x20     required_for_hard: true\n\
             \x20   - kind: plan_review\n\
             \x20     summary: '[codex1-fill:plan_reviewer_evidence]'\n\
             \x20     required_for_hard: true\n"
        }
        _ => "  evidence: []\n",
    };

    format!(
        "mission_id: {mission_id}\n\
         \n\
         planning_level:\n\
         \x20 requested: {requested}\n\
         \x20 effective: {effective}\n\
         \n\
         outcome_interpretation:\n\
         \x20 summary: '[codex1-fill:outcome_interpretation]'\n\
         \n\
         architecture:\n\
         \x20 summary: '[codex1-fill:architecture_summary]'\n\
         \x20 key_decisions:\n\
         \x20   - '[codex1-fill:architecture_key_decision_1]'\n\
         \n\
         planning_process:\n\
         {evidence_block}\n\
         tasks: []\n\
         \n\
         risks:\n\
         \x20 - risk: '[codex1-fill:risk_1]'\n\
         \x20   mitigation: '[codex1-fill:mitigation_1]'\n\
         \n\
         mission_close:\n\
         \x20 criteria:\n\
         \x20   - '[codex1-fill:mission_close_criteria_1]'\n"
    )
}

fn relative_plan_path(paths: &MissionPaths) -> String {
    let abs = paths.plan();
    if let Ok(rel) = abs.strip_prefix(&paths.repo_root) {
        rel.display().to_string()
    } else {
        abs.display().to_string()
    }
}
