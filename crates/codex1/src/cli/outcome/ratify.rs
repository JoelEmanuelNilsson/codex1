//! `codex1 outcome ratify` — validate OUTCOME.md, flip it to `ratified`,
//! and advance the mission phase to `plan`.

use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cli::outcome::emit::emit_outcome_incomplete;
use crate::cli::outcome::validate::validate_outcome;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::state::{self, fs_atomic::atomic_write, Phase};

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let report = validate_outcome(&paths.outcome())?;

    if !report.ratifiable {
        emit_outcome_incomplete(
            &state.mission_id,
            report.missing_fields,
            report.placeholders,
        );
    }

    let ratified_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    if ctx.dry_run {
        state::check_expected_revision(ctx.expect_revision, &state)?;
        let would_phase = advance_phase(&state.phase);
        let env = JsonOk::new(
            Some(state.mission_id.clone()),
            Some(state.revision),
            json!({
                "dry_run": true,
                "would_ratified_at": ratified_at,
                "mission_id": state.mission_id,
                "phase": would_phase,
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let outcome_path = paths.outcome();
    let rewritten_outcome = rewrite_status_to_ratified(&report.frontmatter_raw, &report.body)?;

    // Mutate STATE.json first. OUTCOME.md is the auxiliary artifact; if
    // the state bump fails the file must not already read `ratified`,
    // otherwise a subsequent ratify attempt sees a half-flipped world.
    // The reverse order (write OUTCOME.md first, then mutate state) was
    // an atomicity bug: a crash between the two writes would leave
    // OUTCOME.md `ratified` on disk while `state.outcome.ratified`
    // remained `false`, breaking the file-vs-state consistency invariant.
    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        "outcome.ratified",
        json!({
            "mission_id": state.mission_id,
            "ratified_at": ratified_at,
        }),
        |state_mut| {
            state_mut.outcome.ratified = true;
            state_mut.outcome.ratified_at = Some(ratified_at.clone());
            state_mut.phase = advance_phase(&state_mut.phase);
            Ok(())
        },
    )?;
    atomic_write(&outcome_path, rewritten_outcome.as_bytes())?;

    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(mutation.new_revision),
        json!({
            "ratified_at": ratified_at,
            "mission_id": state.mission_id,
            "phase": mutation.state.phase,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

/// Advance `Clarify` to `Plan`; leave every other phase alone. Shared by
/// the dry-run projection and the real mutation so they cannot drift.
fn advance_phase(current: &Phase) -> Phase {
    match current {
        Phase::Clarify => Phase::Plan,
        other => other.clone(),
    }
}

/// Rewrite OUTCOME.md's frontmatter so `status: draft` (or any other
/// value) becomes `status: ratified`. Only the first `status:` line
/// inside the frontmatter is touched; the body is preserved byte-for-byte.
///
/// We avoid a `serde_yaml` round-trip to preserve field order, comments,
/// quoting style, and whitespace authored by `$clarify`.
fn rewrite_status_to_ratified(frontmatter: &str, body: &str) -> Result<String, CliError> {
    let mut new_front = String::with_capacity(frontmatter.len() + 16);
    let mut rewrote = false;
    for line in frontmatter.split_inclusive('\n') {
        if !rewrote && is_top_level_status_line(line) {
            let indent_end = line.find("status:").unwrap_or(0);
            let indent = &line[..indent_end];
            let line_ending = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            new_front.push_str(indent);
            new_front.push_str("status: ratified");
            new_front.push_str(line_ending);
            rewrote = true;
        } else {
            new_front.push_str(line);
        }
    }
    if !rewrote {
        // Validation guarantees the field exists, but be explicit if the
        // line couldn't be located via the simple scan.
        return Err(CliError::OutcomeIncomplete {
            message: "Could not locate `status:` line in OUTCOME.md frontmatter".to_string(),
            hint: Some("Add a `status:` field at the top level of the YAML block.".to_string()),
        });
    }
    let mut out = String::with_capacity(frontmatter.len() + body.len() + 8);
    out.push_str("---\n");
    out.push_str(&new_front);
    out.push_str("---");
    // Preserve the exact body, including any leading newline that was
    // part of the file (split_frontmatter leaves `\n# Body…` in `body`).
    out.push_str(body);
    Ok(out)
}

fn is_top_level_status_line(line: &str) -> bool {
    // Top-level keys have no leading whitespace in YAML frontmatter.
    // We require the line to start with `status:` followed by a space,
    // tab, newline, or end-of-string.
    let trimmed_end = line.trim_end_matches(['\r', '\n']);
    trimmed_end
        .strip_prefix("status:")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_status_line_preserving_body() {
        let front = "mission_id: demo\nstatus: draft\ntitle: Thing\n";
        let body = "\n# OUTCOME\nsome text with status: in prose\n";
        let got = rewrite_status_to_ratified(front, body).unwrap();
        assert!(got.contains("status: ratified"));
        assert!(got.contains("status: in prose"));
        assert!(!got.contains("status: draft"));
    }

    #[test]
    fn rewrites_only_first_status_line() {
        let front = "status: draft\nstatus: stale\n";
        let got = rewrite_status_to_ratified(front, "").unwrap();
        // Only the first `status:` is rewritten; the second survives as
        // documentation of the original YAML shape.
        assert!(got.contains("status: ratified\nstatus: stale"));
    }

    #[test]
    fn errors_when_status_missing() {
        let err = rewrite_status_to_ratified("mission_id: demo\n", "").unwrap_err();
        assert!(matches!(err, CliError::OutcomeIncomplete { .. }));
    }
}
