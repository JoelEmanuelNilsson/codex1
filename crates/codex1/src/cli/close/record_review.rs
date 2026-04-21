//! `codex1 close record-review --json` — record the mission-close review.
//!
//! Clean: sets `state.close.review_state = Passed`.
//! Dirty: appends the provided findings file into the reviews dir, bumps
//! `state.replan.consecutive_dirty_by_target["__mission_close__"]`, flips
//! `state.replan.triggered` if the counter reaches six, and keeps
//! `review_state = Open` so the next round restarts.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::{ensure_artifact_parent_write_safe, MissionPaths};
use crate::state::readiness::Verdict;
use crate::state::schema::MissionCloseReviewState;
use crate::state::{self};

use super::check::ReadinessReport;
use super::{serde_variant, MISSION_CLOSE_TARGET};

/// Six consecutive dirty reviews on the same target triggers replan.
const DIRTY_REPLAN_THRESHOLD: u32 = 6;

/// Parsed CLI input after validating the clean/findings-file pairing.
enum Outcome {
    Clean,
    Dirty { findings_file: PathBuf },
}

impl Outcome {
    fn from_flags(clean: bool, findings_file: Option<PathBuf>) -> CliResult<Self> {
        match (clean, findings_file) {
            (true, None) => Ok(Self::Clean),
            (false, Some(path)) => Ok(Self::Dirty {
                findings_file: path,
            }),
            (true, Some(_)) => Err(CliError::ParseError {
                message: "pass either --clean or --findings-file, not both".to_string(),
            }),
            (false, None) => Err(CliError::ParseError {
                message: "pass either --clean or --findings-file".to_string(),
            }),
        }
    }
}

pub fn run(
    ctx: &Ctx,
    clean: bool,
    findings_file: Option<PathBuf>,
    reviewers_csv: Option<&str>,
) -> CliResult<()> {
    let outcome = Outcome::from_flags(clean, findings_file)?;
    let paths = resolve_mission(&ctx.selector(), true)?;
    let current = state::load(&paths)?;
    state::check_expected_revision(ctx.expect_revision, &current)?;
    let reviewers = parse_reviewers(reviewers_csv);

    if let Some(closed_at) = current.close.terminal_at.clone() {
        if !ctx.dry_run {
            state::mutate(
                &paths,
                ctx.expect_revision,
                "close.review.contaminated_after_terminal",
                json!({
                "verdict": if matches!(outcome, Outcome::Clean) { "clean" } else { "dirty" },
                    "reviewers": reviewers.clone(),
                    "category": "contaminated_after_terminal",
                }),
                |_state| Ok(()),
            )?;
        }
        return Err(CliError::TerminalAlreadyComplete { closed_at });
    }

    // Gate: the mission must be in a state where recording a mission-close
    // review is sensible. We allow both `ready_for_mission_close_review`
    // (the first attempt, which transitions review_state to Open before
    // recording) and `mission_close_review_open` (subsequent attempts).
    let report = ReadinessReport::from_state_and_paths(&current, &paths);
    if !matches!(
        report.verdict,
        Verdict::ReadyForMissionCloseReview | Verdict::MissionCloseReviewOpen
    ) {
        return Err(CliError::CloseNotReady {
            message: format!(
                "cannot record mission-close review while verdict is `{}` ({})",
                report.verdict.as_str(),
                report.blocker_summary()
            ),
        });
    }

    match outcome {
        Outcome::Clean => record_clean(ctx, &paths, &current, &reviewers),
        Outcome::Dirty { findings_file } => {
            record_dirty(ctx, &paths, &current, &findings_file, &reviewers)
        }
    }
}

fn record_clean(
    ctx: &Ctx,
    paths: &MissionPaths,
    current: &state::MissionState,
    reviewers: &[String],
) -> CliResult<()> {
    if ctx.dry_run {
        emit_success(
            &current.mission_id,
            Some(current.revision),
            "clean",
            None,
            reviewers,
            /*dry_run=*/ true,
            MissionCloseReviewState::Passed,
            /*replan_triggered=*/ current.replan.triggered,
            /*dirty_counter=*/ 0,
        );
        return Ok(());
    }

    let payload = json!({
        "verdict": "clean",
        "reviewers": reviewers,
    });
    let mutation = state::mutate(
        paths,
        ctx.expect_revision,
        "close.review.clean",
        payload,
        |state| {
            let report = ReadinessReport::from_state_and_paths(state, paths);
            if !matches!(
                report.verdict,
                Verdict::ReadyForMissionCloseReview | Verdict::MissionCloseReviewOpen
            ) {
                return Err(CliError::CloseNotReady {
                    message: format!(
                        "cannot record mission-close review while verdict is `{}` ({})",
                        report.verdict.as_str(),
                        report.blocker_summary()
                    ),
                });
            }
            state.close.review_state = MissionCloseReviewState::Passed;
            state
                .replan
                .consecutive_dirty_by_target
                .insert(MISSION_CLOSE_TARGET.to_string(), 0);
            Ok(())
        },
    )?;

    emit_success(
        &mutation.state.mission_id,
        Some(mutation.new_revision),
        "clean",
        None,
        reviewers,
        /*dry_run=*/ false,
        MissionCloseReviewState::Passed,
        mutation.state.replan.triggered,
        current_counter(&mutation.state),
    );
    Ok(())
}

fn record_dirty(
    ctx: &Ctx,
    paths: &MissionPaths,
    current: &state::MissionState,
    findings_source: &Path,
    reviewers: &[String],
) -> CliResult<()> {
    if !findings_source.is_file() {
        return Err(CliError::ReviewFindingsBlock {
            message: format!("findings file not found: {}", findings_source.display()),
        });
    }
    let findings_body = std::fs::read_to_string(findings_source)?;

    if ctx.dry_run {
        // Preview the post-mutation values off the loaded snapshot. Under
        // contention with another writer the preview may drift, but a
        // dry-run is advisory by definition.
        let predicted_revision = current.revision.saturating_add(1);
        let findings_target = mission_close_review_path(paths, predicted_revision);
        let predicted_counter = current_counter(current).saturating_add(1);
        let predicted_trigger =
            current.replan.triggered || predicted_counter >= DIRTY_REPLAN_THRESHOLD;
        emit_success(
            &current.mission_id,
            Some(current.revision),
            "dirty",
            Some(&findings_target),
            reviewers,
            /*dry_run=*/ true,
            MissionCloseReviewState::Open,
            predicted_trigger,
            predicted_counter,
        );
        return Ok(());
    }

    let mutation = state::mutate(
        paths,
        ctx.expect_revision,
        "close.review.dirty",
        json!({
            "verdict": "dirty",
            "reviewers": reviewers,
        }),
        |state| {
            let report = ReadinessReport::from_state_and_paths(state, paths);
            if !matches!(
                report.verdict,
                Verdict::ReadyForMissionCloseReview | Verdict::MissionCloseReviewOpen
            ) {
                return Err(CliError::CloseNotReady {
                    message: format!(
                        "cannot record mission-close review while verdict is `{}` ({})",
                        report.verdict.as_str(),
                        report.blocker_summary()
                    ),
                });
            }
            let findings_target =
                mission_close_review_path(paths, state.revision.saturating_add(1));
            ensure_artifact_parent_write_safe(paths, &findings_target)?;
            crate::state::fs_atomic::atomic_write(&findings_target, findings_body.as_bytes())?;
            let counter = state
                .replan
                .consecutive_dirty_by_target
                .entry(MISSION_CLOSE_TARGET.to_string())
                .or_insert(0);
            *counter = counter.saturating_add(1);
            let hit = *counter >= DIRTY_REPLAN_THRESHOLD;
            if hit && !state.replan.triggered {
                state.replan.triggered = true;
                state.replan.triggered_reason = Some(format!(
                    "{DIRTY_REPLAN_THRESHOLD} consecutive dirty mission-close reviews"
                ));
            }
            state.close.review_state = MissionCloseReviewState::Open;
            Ok(())
        },
    )?;

    emit_success(
        &mutation.state.mission_id,
        Some(mutation.new_revision),
        "dirty",
        Some(&mission_close_review_path(paths, mutation.new_revision)),
        reviewers,
        /*dry_run=*/ false,
        MissionCloseReviewState::Open,
        mutation.state.replan.triggered,
        current_counter(&mutation.state),
    );
    Ok(())
}

fn current_counter(state: &state::MissionState) -> u32 {
    state
        .replan
        .consecutive_dirty_by_target
        .get(MISSION_CLOSE_TARGET)
        .copied()
        .unwrap_or(0)
}

pub(crate) fn mission_close_review_path(paths: &MissionPaths, revision: u64) -> PathBuf {
    paths
        .reviews_dir()
        .join(format!("mission-close-{revision}.md"))
}

fn parse_reviewers(csv: Option<&str>) -> Vec<String> {
    match csv {
        None => Vec::new(),
        Some(raw) => raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect(),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_success(
    mission_id: &str,
    revision: Option<u64>,
    verdict: &str,
    findings_file: Option<&Path>,
    reviewers: &[String],
    dry_run: bool,
    review_state: MissionCloseReviewState,
    replan_triggered: bool,
    dirty_counter: u32,
) {
    let env = JsonOk::new(
        Some(mission_id.to_string()),
        revision,
        json!({
            "verdict": verdict,
            "review_state": serde_variant(&review_state),
            "reviewers": reviewers,
            "findings_file": findings_file.map(Path::to_path_buf),
            "consecutive_dirty": dirty_counter,
            "replan_triggered": replan_triggered,
            "dry_run": dry_run,
        }),
    );
    println!("{}", env.to_pretty());
}
