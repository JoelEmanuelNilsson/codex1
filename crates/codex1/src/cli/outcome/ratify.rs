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
use crate::core::paths::{ensure_artifact_file_read_safe, ensure_artifact_file_write_safe};
use crate::state::{self, fs_atomic::atomic_write, Phase};

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    state::check_expected_revision(ctx.expect_revision, &state)?;
    if state.plan.locked {
        return Err(CliError::PlanInvalid {
            message: "cannot ratify OUTCOME.md after PLAN.yaml is locked".to_string(),
            hint: Some(
                "If the clarified destination changed, record a replan instead of re-ratifying the locked mission."
                    .to_string(),
            ),
        });
    }
    let outcome_path = paths.outcome();
    ensure_artifact_file_read_safe(&paths, &outcome_path, "OUTCOME.md")?;
    ensure_artifact_file_write_safe(&paths, &outcome_path, "OUTCOME.md")?;
    let report = validate_outcome(&outcome_path, &state.mission_id)?;

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
    let top_level_indent = detect_top_level_key_indent(frontmatter);
    for line in frontmatter.split_inclusive('\n') {
        if !rewrote && is_top_level_status_line(line, top_level_indent) {
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
    // Always terminate the closing fence with its own newline, then paste
    // the body verbatim. Historical behavior omitted the trailing `\n` on
    // the assumption that `body` begins with `\n` — true when there is a
    // blank line between the closing fence and the first body heading
    // (`split_frontmatter` puts that blank line into `body`), but false
    // on hand-written files of the shape `…\n---\n# Heading…`. Under the
    // old behavior a single ratify on a no-blank-line file collapsed
    // `---` and `# Heading` onto one line, permanently breaking
    // `split_frontmatter`. Emitting `---\n` unconditionally is byte-stable
    // across both shapes: when `body` already begins with `\n` the result
    // is `...---\n\n# Heading...` (blank line preserved), and when it
    // does not the result is `...---\n# Heading...` (closing fence
    // remains a standalone line).
    out.push_str("---\n");
    out.push_str(body);
    Ok(out)
}

fn detect_top_level_key_indent(frontmatter: &str) -> usize {
    frontmatter
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let indent = line.len().saturating_sub(line.trim_start().len());
            let trimmed_start = line.trim_start();
            let is_key = trimmed_start
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                && trimmed_start.contains(':');
            is_key.then_some(indent)
        })
        .min()
        .unwrap_or(0)
}

fn is_top_level_status_line(line: &str, top_level_indent: usize) -> bool {
    let trimmed_end = line.trim_end_matches(['\r', '\n']);
    let actual_indent = trimmed_end
        .chars()
        .take_while(char::is_ascii_whitespace)
        .count();
    if actual_indent != top_level_indent {
        return false;
    }
    trimmed_end
        .trim_start()
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

    #[test]
    fn rewrites_indented_top_level_status_line() {
        let front = "  mission_id: demo\n  status: draft\n  title: Thing\n";
        let got = rewrite_status_to_ratified(front, "").unwrap();
        assert!(got.contains("  status: ratified\n"));
    }
}
