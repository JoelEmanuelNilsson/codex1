//! CLOSEOUT.md renderer for `codex1 close complete`.
//!
//! Emits a human-friendly summary pulled from `STATE.json` plus an
//! excerpt of `interpreted_destination` from the ratified OUTCOME.md.

use std::fmt::Write as _;
use std::path::Path;

use crate::core::paths::{ensure_artifact_file_read_safe, MissionPaths};
use crate::state::schema::{MissionCloseReviewState, MissionState};

use super::{serde_variant, MISSION_CLOSE_TARGET};

/// Render the CLOSEOUT.md body from the post-mutation mission state.
#[must_use]
pub fn render(state: &MissionState, paths: &MissionPaths) -> String {
    let mut out = String::with_capacity(2048);
    let terminal_at = state.close.terminal_at.as_deref().unwrap_or("unknown");

    writeln!(out, "# CLOSEOUT — {}", state.mission_id).ok();
    writeln!(out).ok();
    writeln!(out, "**Terminal at:** {terminal_at}").ok();
    writeln!(out, "**Final revision:** {}", state.revision).ok();
    let planning_level = state
        .plan
        .effective_level
        .as_ref()
        .map_or("unspecified".to_string(), serde_variant);
    writeln!(out, "**Planning level:** {planning_level}").ok();
    writeln!(out).ok();

    writeln!(out, "## Outcome").ok();
    writeln!(out).ok();
    let destination = read_interpreted_destination(&paths.outcome())
        .unwrap_or_else(|| "_interpreted_destination not found in OUTCOME.md_".to_string());
    writeln!(out, "{}", destination.trim()).ok();
    writeln!(out).ok();

    writeln!(out, "## Tasks").ok();
    writeln!(out).ok();
    if state.tasks.is_empty() {
        writeln!(out, "_No tasks recorded._").ok();
    } else {
        writeln!(out, "| ID | Status | Proof |").ok();
        writeln!(out, "|---|---|---|").ok();
        for (id, task) in &state.tasks {
            let proof = task.proof_path.clone().unwrap_or_else(|| "—".to_string());
            writeln!(out, "| {id} | {} | {proof} |", serde_variant(&task.status)).ok();
        }
    }
    writeln!(out).ok();

    writeln!(out, "## Reviews").ok();
    writeln!(out).ok();
    let has_mission_close = !matches!(
        state.close.review_state,
        MissionCloseReviewState::NotStarted
    );
    if state.reviews.is_empty() && !has_mission_close {
        writeln!(out, "_No reviews recorded._").ok();
    } else {
        writeln!(out, "| Review ID | Verdict | Reviewers | Findings |").ok();
        writeln!(out, "|---|---|---|---|").ok();
        for (id, record) in &state.reviews {
            writeln!(
                out,
                "| {id} | {} | {} | {} |",
                serde_variant(&record.verdict),
                if record.reviewers.is_empty() {
                    "—".to_string()
                } else {
                    record.reviewers.join(",")
                },
                record
                    .findings_file
                    .clone()
                    .unwrap_or_else(|| "—".to_string()),
            )
            .ok();
        }
        if has_mission_close {
            let mc_verdict = match state.close.review_state {
                MissionCloseReviewState::Passed => "clean",
                MissionCloseReviewState::Open => "open",
                MissionCloseReviewState::NotStarted => "—",
            };
            writeln!(out, "| MC | {mc_verdict} | — | — |").ok();
        }
    }
    writeln!(out).ok();

    writeln!(out, "## Mission-close review").ok();
    writeln!(out).ok();
    let dirty_rounds = state
        .replan
        .consecutive_dirty_by_target
        .get(MISSION_CLOSE_TARGET)
        .copied()
        .unwrap_or(0);
    let historical_dirty_rounds = mission_close_dirty_rounds(paths);
    let dirty_rounds = dirty_rounds.max(historical_dirty_rounds);
    let summary = match state.close.review_state {
        MissionCloseReviewState::Passed => {
            if dirty_rounds == 0 {
                "Clean on the first round.".to_string()
            } else {
                format!(
                    "Clean after {dirty_rounds} dirty round{} along the way.",
                    if dirty_rounds == 1 { "" } else { "s" }
                )
            }
        }
        MissionCloseReviewState::Open => {
            format!("Review was still open at closeout ({dirty_rounds} recorded dirty rounds).")
        }
        MissionCloseReviewState::NotStarted => {
            "Mission-close review was never started.".to_string()
        }
    };
    writeln!(out, "{summary}").ok();
    if state.replan.triggered {
        let reason = state
            .replan
            .triggered_reason
            .clone()
            .unwrap_or_else(|| "replan triggered".to_string());
        writeln!(out).ok();
        writeln!(out, "Replan trigger noted in final state: {reason}").ok();
    }

    out
}

fn mission_close_dirty_rounds(paths: &MissionPaths) -> u32 {
    std::fs::read_dir(paths.reviews_dir())
        .ok()
        .into_iter()
        .flat_map(|iter| iter.filter_map(Result::ok))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            name.starts_with("mission-close-")
                && Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .count() as u32
}

/// Extract `interpreted_destination` from the YAML frontmatter of
/// OUTCOME.md. Returns `None` if the file is missing, has no
/// frontmatter, or the key is absent. Failure to parse is tolerated —
/// CLOSEOUT.md is an auditor artifact, not a gate.
fn read_interpreted_destination(path: &Path) -> Option<String> {
    let mission_dir = path.parent()?;
    let repo_root = mission_dir.parent()?.parent()?.to_path_buf();
    let mission_id = mission_dir.file_name()?.to_string_lossy().to_string();
    let paths = MissionPaths {
        repo_root,
        mission_id,
        mission_dir: mission_dir.to_path_buf(),
    };
    ensure_artifact_file_read_safe(&paths, path, "OUTCOME.md").ok()?;
    let raw = std::fs::read_to_string(path).ok()?;
    let frontmatter = extract_frontmatter(&raw)?;
    let value: serde_yaml::Value = serde_yaml::from_str(frontmatter).ok()?;
    let dest = value.get("interpreted_destination")?.as_str()?;
    Some(dest.trim().to_string())
}

fn extract_frontmatter(raw: &str) -> Option<&str> {
    let rest = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---").or_else(|| rest.find("\r\n---"))?;
    Some(&rest[..end])
}
