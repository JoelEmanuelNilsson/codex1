//! `codex1 init` — create a fresh mission scaffold under PLANS/<id>/.
//!
//! Creates OUTCOME.md (with fill markers), minimal PLAN.yaml header,
//! fresh STATE.json with `revision: 0` and `phase: clarify`, an empty
//! EVENTS.jsonl, and `specs/` + `reviews/` directories.

use clap::Args;
use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission_for_init;
use crate::state::{self, MissionState};

#[derive(Debug, Clone, Args)]
pub struct InitArgs {}

pub fn run(_args: InitArgs, ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission_for_init(&ctx.selector())?;
    let state = MissionState::fresh(&paths.mission_id);

    if ctx.dry_run {
        let env = JsonOk::new(
            Some(paths.mission_id.clone()),
            Some(state.revision),
            json!({
                "dry_run": true,
                "would_create": {
                    "mission_dir": paths.mission_dir,
                    "outcome": paths.outcome(),
                    "plan": paths.plan(),
                    "state": paths.state(),
                    "events": paths.events(),
                    "specs_dir": paths.specs_dir(),
                    "reviews_dir": paths.reviews_dir(),
                },
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    state::init_write(&paths, &state)?;
    write_outcome_template(&paths.outcome(), &paths.mission_id)?;
    write_plan_template(&paths.plan(), &paths.mission_id)?;

    let env = JsonOk::new(
        Some(paths.mission_id.clone()),
        Some(state.revision),
        json!({
            "created": {
                "mission_dir": paths.mission_dir,
                "outcome": paths.outcome(),
                "plan": paths.plan(),
                "state": paths.state(),
                "events": paths.events(),
                "specs_dir": paths.specs_dir(),
                "reviews_dir": paths.reviews_dir(),
            },
            "next_action": {
                "kind": "clarify",
                "command": "$clarify",
                "hint": "Fill in OUTCOME.md, then run `codex1 outcome ratify`.",
            }
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

fn write_outcome_template(path: &std::path::Path, mission_id: &str) -> CliResult<()> {
    let content = format!(
        r"---
mission_id: {mission_id}
status: draft
title: '[codex1-fill:title]'

original_user_goal: |
  [codex1-fill:original_user_goal]

interpreted_destination: |
  [codex1-fill:interpreted_destination]

must_be_true:
  - '[codex1-fill:must_be_true_1]'

success_criteria:
  - '[codex1-fill:success_criteria_1]'

non_goals:
  - '[codex1-fill:non_goals_1]'

constraints:
  - '[codex1-fill:constraints_1]'

definitions: {{}}

quality_bar:
  - '[codex1-fill:quality_bar_1]'

proof_expectations:
  - '[codex1-fill:proof_expectations_1]'

review_expectations:
  - '[codex1-fill:review_expectations_1]'

known_risks:
  - '[codex1-fill:known_risks_1]'

resolved_questions: []
---

# OUTCOME

This file captures the clarified mission destination. Run `$clarify` to
fill every `[codex1-fill:...]` marker. Only a ratified OUTCOME.md can
drive planning.
"
    );
    crate::state::fs_atomic::atomic_write(path, content.as_bytes())?;
    Ok(())
}

fn write_plan_template(path: &std::path::Path, mission_id: &str) -> CliResult<()> {
    let content = format!(
        r"mission_id: {mission_id}

planning_level:
  requested: '[codex1-fill:planning_level]'
  effective: '[codex1-fill:planning_level]'

outcome_interpretation:
  summary: '[codex1-fill:outcome_interpretation]'

architecture:
  summary: '[codex1-fill:architecture_summary]'
  key_decisions: []

planning_process:
  evidence: []

tasks: []

risks: []

mission_close:
  criteria: []
"
    );
    crate::state::fs_atomic::atomic_write(path, content.as_bytes())?;
    Ok(())
}
