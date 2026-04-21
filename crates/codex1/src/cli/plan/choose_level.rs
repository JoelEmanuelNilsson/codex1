//! `codex1 plan choose-level` — record the requested/effective planning level.
//!
//! Accepts `1`/`2`/`3` or `light`/`medium`/`hard` (never `low`/`high`).
//! When `--escalate <reason>` is supplied, the effective level is bumped
//! to `hard`. Interactive TTY callers see a STDERR prompt; non-interactive
//! callers without `--level` get `PARSE_ERROR`.

use std::io::{self, BufRead, IsTerminal, Write};

use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::{self, Phase, PlanLevel};

/// Handle `codex1 plan choose-level`.
pub fn run(level: Option<String>, escalate: Option<String>, ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;

    // Gate on outcome ratification: the handoff specifies that planning
    // cannot begin until OUTCOME.md has been ratified. This matches the
    // error-code set documented in docs/cli-reference.md for this command.
    let current = state::load(&paths)?;
    if !current.outcome.ratified {
        return Err(CliError::OutcomeNotRatified);
    }

    let requested = match level {
        Some(raw) => parse_level(&raw)?,
        None => prompt_for_level()?,
    };
    // `--escalate` pins the effective level to `hard`, but the handoff
    // (02-cli-contract.md:325 Implementation rule) requires that
    // `escalation_reason` is emitted only when the effective level is
    // *higher* than the requested level. If the caller passes
    // `--escalate` while already asking for `hard`, the request is a
    // no-op and we drop the reason rather than stamp a phantom on it.
    let (effective, escalation_reason) = match escalate {
        Some(reason) if level_rank(&requested) < level_rank(&PlanLevel::Hard) => {
            (PlanLevel::Hard, Some(reason))
        }
        Some(_) => (requested.clone(), None),
        None => (requested.clone(), None),
    };

    let data = build_payload(&requested, &effective, escalation_reason.as_deref());

    if ctx.dry_run {
        // Validate against current state, but do not write.
        let state = state::load(&paths)?;
        state::check_expected_revision(ctx.expect_revision, &state)?;
        let env = JsonOk::new(
            Some(paths.mission_id.clone()),
            Some(state.revision),
            with_dry_run(data),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let payload_event = json!({
        "requested_level": level_str(&requested),
        "effective_level": level_str(&effective),
        "escalation_reason": escalation_reason,
    });

    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        "plan.choose_level",
        payload_event,
        |state| {
            state.plan.requested_level = Some(requested.clone());
            state.plan.effective_level = Some(effective.clone());
            if matches!(state.phase, Phase::Clarify) {
                state.phase = Phase::Plan;
            }
            Ok(())
        },
    )?;

    let env = JsonOk::new(
        Some(paths.mission_id.clone()),
        Some(mutation.new_revision),
        data,
    );
    println!("{}", env.to_pretty());
    Ok(())
}

/// Parse a raw level string into a `PlanLevel`. Accepts product verbs and
/// the `1/2/3` numeric aliases. Rejects `low`/`high` or anything else.
///
/// Shared with `plan scaffold` so both commands use the same rules.
pub fn parse_level(raw: &str) -> CliResult<PlanLevel> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "light" => Ok(PlanLevel::Light),
        "2" | "medium" => Ok(PlanLevel::Medium),
        "3" | "hard" => Ok(PlanLevel::Hard),
        other => Err(CliError::ParseError {
            message: format!(
                "invalid planning level `{other}`; expected one of 1/light, 2/medium, 3/hard"
            ),
        }),
    }
}

/// Print the menu to STDERR and read one line from STDIN. Used only when
/// stdin is a TTY; non-interactive callers must pass `--level`.
fn prompt_for_level() -> CliResult<PlanLevel> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return Err(CliError::ParseError {
            message: "`--level` required in non-interactive mode".to_string(),
        });
    }
    let mut stderr = io::stderr().lock();
    writeln!(
        stderr,
        "Choose planning level:\n1. light  - small/local/obvious work\n2. medium - normal multi-step work\n3. hard   - architecture/risky/autonomous/multi-agent work"
    )?;
    stderr.flush()?;
    drop(stderr);

    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    parse_level(line.trim())
}

/// Canonical string form of a `PlanLevel` (always lowercase verbs).
pub fn level_str(level: &PlanLevel) -> &'static str {
    match level {
        PlanLevel::Light => "light",
        PlanLevel::Medium => "medium",
        PlanLevel::Hard => "hard",
    }
}

fn build_payload(
    requested: &PlanLevel,
    effective: &PlanLevel,
    escalation_reason: Option<&str>,
) -> serde_json::Value {
    let effective_str = level_str(effective);
    let escalated = level_rank(effective) > level_rank(requested);
    let mut data = json!({
        "requested_level": level_str(requested),
        "effective_level": effective_str,
        "escalation_required": escalated,
        "next_action": {
            "kind": "plan_scaffold",
            "args": ["codex1", "plan", "scaffold", "--level", effective_str],
        },
    });
    if escalated {
        if let Some(reason) = escalation_reason {
            // `json!({…})` above always returns an Object, so the
            // `as_object_mut` call below cannot fail.
            data.as_object_mut()
                .expect("build_payload constructs a JSON object literal")
                .insert("escalation_reason".to_string(), json!(reason));
        }
    }
    data
}

/// Numeric rank for `PlanLevel` so the escalation guard can compare
/// `effective > requested` without reaching for `Ord` on the enum.
pub(crate) fn level_rank(level: &PlanLevel) -> u8 {
    match level {
        PlanLevel::Light => 0,
        PlanLevel::Medium => 1,
        PlanLevel::Hard => 2,
    }
}

fn with_dry_run(mut data: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = data.as_object_mut() {
        obj.insert("dry_run".to_string(), json!(true));
    }
    data
}
