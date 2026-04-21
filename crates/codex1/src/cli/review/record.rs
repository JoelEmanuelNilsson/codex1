//! `codex1 review record <id>` — record the outcome of a planned review.
//!
//! The main thread pipes reviewer findings through this command. The CLI
//! classifies the record (`accepted_current | late_same_boundary |
//! stale_superseded | contaminated_after_terminal`) and mutates state
//! accordingly. See `docs/cli-contract-schemas.md` § Review record
//! freshness.

use std::path::{Path, PathBuf};

use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cli::review::classify::{category_str, classify, verdict_str, ClassifyInput};
use crate::cli::review::plan_read::{fetch_review_task, load_tasks, review_targets};
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::MissionPaths;
use crate::state::fs_atomic::atomic_write;
use crate::state::schema::{
    MissionState, ReviewRecord, ReviewRecordCategory, ReviewVerdict, TaskStatus,
};
use crate::state::{self};

/// Threshold at which consecutive dirty reviews trigger a replan.
const DIRTY_STREAK_THRESHOLD: u32 = 6;

pub struct RecordInputs<'a> {
    pub task_id: &'a str,
    pub clean: bool,
    pub findings_file: Option<PathBuf>,
    pub reviewers_csv: Option<String>,
}

pub fn run(ctx: &Ctx, inputs: &RecordInputs<'_>) -> CliResult<()> {
    if inputs.clean == inputs.findings_file.is_some() {
        // Either both supplied or neither — clap's `conflicts_with`/`required`
        // should already reject this, but fail closed if we ever get here.
        return Err(CliError::ParseError {
            message: "review record requires exactly one of --clean or --findings-file".to_string(),
        });
    }
    let paths = resolve_mission(&ctx.selector(), true)?;
    let peek = state::load(&paths)?;
    state::check_expected_revision(ctx.expect_revision, &peek)?;

    let plan_tasks = load_tasks(&paths.plan())?;
    let review_task = fetch_review_task(&plan_tasks, inputs.task_id)?;
    let targets = review_targets(&review_task)?;

    // Parse reviewers early so dry-run + wet run share the same vec.
    let reviewers = parse_reviewers(inputs.reviewers_csv.as_deref());

    let findings_path = inputs.findings_file.as_deref();
    if let Some(p) = findings_path {
        if !p.is_file() {
            return Err(CliError::ReviewFindingsBlock {
                message: format!("findings file not found: {}", p.display()),
            });
        }
    }

    let verdict = if inputs.clean {
        ReviewVerdict::Clean
    } else {
        ReviewVerdict::Dirty
    };

    // Use a preflight state snapshot for dry-run (and to surface terminal/stale
    // errors before we enter the mutation closure in the wet path).
    // Refuse to record a review while the plan is unlocked. We allow
    // the terminal-contamination path above to still return its
    // specific error code (it runs after classification); the
    // plan-locked guard kicks in only when the state is non-terminal.
    if peek.close.terminal_at.is_none() {
        state::require_plan_locked(&peek)?;
    }
    let peek_category = classify(&ClassifyInput {
        state: &peek,
        review_task_id: inputs.task_id,
        target_task_ids: &targets,
        // In peek-mode we just want the pre-mutate revision; classification
        // thresholds use the closure's pre-bump revision too.
        state_revision_at_record: peek.revision,
    });

    if matches!(
        peek_category,
        ReviewRecordCategory::ContaminatedAfterTerminal
    ) {
        if !ctx.dry_run {
            state::mutate(
                &paths,
                ctx.expect_revision,
                "review.contaminated_after_terminal",
                json!({
                    "review_task_id": inputs.task_id,
                    "verdict": verdict_str(&verdict),
                    "reviewers": reviewers,
                    "targets": targets,
                    "category": "contaminated_after_terminal",
                }),
                |_state| Ok(()),
            )?;
        }
        let closed_at = peek
            .close
            .terminal_at
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        return Err(CliError::TerminalAlreadyComplete { closed_at });
    }

    if ctx.dry_run {
        let stored_findings = findings_path.map(|p| {
            relative_from_repo(&paths, &paths.review_file_for(inputs.task_id))
                .or_else(|| Some(p.display().to_string()))
                .unwrap_or_default()
        });
        let env = JsonOk::new(
            Some(peek.mission_id.clone()),
            Some(peek.revision),
            json!({
                "dry_run": true,
                "review_task_id": inputs.task_id,
                "verdict": verdict_str(&verdict),
                "category": category_str(peek_category),
                "reviewers": reviewers,
                "findings_file": stored_findings,
                "replan_triggered": false,
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    // Stale records: emit `"review.stale"` event (mutation that touches no
    // truth-bearing fields) and return `STALE_REVIEW_RECORD`.
    if matches!(peek_category, ReviewRecordCategory::StaleSuperseded) {
        state::mutate(
            &paths,
            ctx.expect_revision,
            "review.stale",
            json!({
                "review_task_id": inputs.task_id,
                "verdict": verdict_str(&verdict),
                "reviewers": reviewers,
                "targets": targets,
            }),
            |_state| Ok(()),
        )?;
        return Err(CliError::StaleReviewRecord {
            message: format!(
                "Review {} or one of its targets is superseded; record not applied",
                inputs.task_id
            ),
        });
    }

    // If a findings file was provided, copy it into PLANS/<mission>/reviews/<id>.md
    // BEFORE the state mutation closure so the mutation remains a pure
    // state update. Idempotent: skip copy if source == destination.
    let stored_findings_rel = if let Some(src) = findings_path {
        let dest = paths.review_file_for(inputs.task_id);
        copy_if_different(src, &dest)?;
        relative_from_repo(&paths, &dest)
    } else {
        None
    };

    let review_task_id = inputs.task_id.to_string();
    let targets_for_closure = targets.clone();
    let reviewers_for_closure = reviewers.clone();
    let findings_for_closure = stored_findings_rel.clone();

    let event_kind = if matches!(verdict, ReviewVerdict::Clean) {
        "review.recorded.clean"
    } else {
        "review.recorded.dirty"
    };

    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        event_kind,
        json!({
            "review_task_id": review_task_id,
            "verdict": verdict_str(&verdict),
            "reviewers": reviewers_for_closure,
            "targets": targets_for_closure,
        }),
        |state| {
            // Re-check `plan.locked` under the exclusive lock (skipped
            // on terminal states so terminal-contamination classifies
            // through `apply_record` first). Closes the TOCTOU between
            // the pre-mutate shared-lock load and this closure; see
            // round-2 correctness P1-1.
            if state.close.terminal_at.is_none() {
                state::require_plan_locked(state)?;
            }
            apply_record(
                state,
                &review_task_id,
                &targets_for_closure,
                &verdict,
                &reviewers_for_closure,
                findings_for_closure.clone(),
            )
        },
    )?;

    let category = mutation
        .state
        .reviews
        .get(inputs.task_id)
        .map_or(ReviewRecordCategory::AcceptedCurrent, |r| {
            r.category.clone()
        });
    let replan_triggered = mutation.state.replan.triggered;
    let warnings = match category {
        ReviewRecordCategory::LateSameBoundary => {
            vec!["recorded as late_same_boundary: state advanced since review start".to_string()]
        }
        _ => Vec::new(),
    };

    let env = JsonOk::new(
        Some(mutation.state.mission_id.clone()),
        Some(mutation.new_revision),
        json!({
            "review_task_id": inputs.task_id,
            "verdict": verdict_str(&verdict),
            "category": category_str(category),
            "reviewers": reviewers,
            "findings_file": stored_findings_rel,
            "replan_triggered": replan_triggered,
            "warnings": warnings,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

/// Mutate the state inside `state::mutate`. Classification is re-computed
/// against the fresh state read under the lock (not the peek).
fn apply_record(
    state: &mut MissionState,
    review_task_id: &str,
    targets: &[String],
    verdict: &ReviewVerdict,
    reviewers: &[String],
    findings_rel: Option<String>,
) -> Result<(), CliError> {
    // Re-classify under the lock — the peek may be stale if another writer
    // mutated between peek and lock acquisition.
    let category = classify(&ClassifyInput {
        state,
        review_task_id,
        target_task_ids: targets,
        state_revision_at_record: state.revision,
    });

    match category {
        ReviewRecordCategory::ContaminatedAfterTerminal => {
            // Should already be handled before we entered the closure; fail
            // closed so we never silently mutate a terminal mission.
            let closed_at = state
                .close
                .terminal_at
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            return Err(CliError::TerminalAlreadyComplete { closed_at });
        }
        ReviewRecordCategory::StaleSuperseded => {
            // Likewise: stale path is handled before the mutation closure.
            return Err(CliError::StaleReviewRecord {
                message: format!("Review {review_task_id} or one of its targets is superseded"),
            });
        }
        _ => {}
    }

    let boundary_revision = state
        .reviews
        .get(review_task_id)
        .map_or(state.revision.saturating_add(1), |r| r.boundary_revision);
    let recorded_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    let record = ReviewRecord {
        task_id: review_task_id.to_string(),
        verdict: verdict.clone(),
        reviewers: reviewers.to_vec(),
        findings_file: findings_rel,
        category: category.clone(),
        recorded_at,
        boundary_revision,
    };
    state.reviews.insert(review_task_id.to_string(), record);

    // Only accepted_current records affect the dirty counter / target
    // status. late_same_boundary is still accepted per spec (same active
    // review, same boundary) but does not mutate truth beyond recording.
    if matches!(category, ReviewRecordCategory::AcceptedCurrent) {
        match *verdict {
            ReviewVerdict::Clean => apply_clean(state, review_task_id, targets),
            ReviewVerdict::Dirty => apply_dirty(state, review_task_id, targets),
            ReviewVerdict::Pending => {}
        }
    }
    Ok(())
}

fn apply_clean(state: &mut MissionState, review_task_id: &str, targets: &[String]) {
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    for tid in targets {
        if let Some(task) = state.tasks.get_mut(tid) {
            if matches!(task.status, TaskStatus::AwaitingReview) {
                task.status = TaskStatus::Complete;
                if task.finished_at.is_none() {
                    task.finished_at = Some(now.clone());
                }
            }
        }
        state
            .replan
            .consecutive_dirty_by_target
            .insert(tid.clone(), 0);
    }
    // The review task itself transitions to Complete so `state.tasks`
    // stays a truthful picture of every DAG node. Without this, clients
    // reading only `state.tasks` (e.g. CLOSEOUT.md writer, status
    // ready-task projection) see the review task as eternally pending.
    mark_review_task_complete(state, review_task_id, &now);
}

fn mark_review_task_complete(state: &mut MissionState, review_task_id: &str, now: &str) {
    use crate::state::schema::TaskRecord;
    let entry = state
        .tasks
        .entry(review_task_id.to_string())
        .or_insert_with(|| TaskRecord {
            id: review_task_id.to_string(),
            status: TaskStatus::Complete,
            started_at: None,
            finished_at: None,
            proof_path: None,
            superseded_by: None,
        });
    entry.status = TaskStatus::Complete;
    if entry.finished_at.is_none() {
        entry.finished_at = Some(now.to_string());
    }
}

fn apply_dirty(state: &mut MissionState, _review_task_id: &str, targets: &[String]) {
    for tid in targets {
        let entry = state
            .replan
            .consecutive_dirty_by_target
            .entry(tid.clone())
            .or_insert(0);
        *entry = entry.saturating_add(1);
        if *entry >= DIRTY_STREAK_THRESHOLD {
            state.replan.triggered = true;
            state.replan.triggered_reason = Some(format!(
                "{DIRTY_STREAK_THRESHOLD} consecutive dirty reviews for {tid}"
            ));
        }
    }
}

fn parse_reviewers(csv: Option<&str>) -> Vec<String> {
    let Some(raw) = csv else { return Vec::new() };
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn copy_if_different(src: &Path, dest: &Path) -> Result<(), CliError> {
    if paths_equal(src, dest) {
        return Ok(());
    }
    let data = std::fs::read(src)?;
    atomic_write(dest, &data)?;
    Ok(())
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    let ra = std::fs::canonicalize(a).ok();
    let rb = std::fs::canonicalize(b).ok();
    match (ra, rb) {
        (Some(ra), Some(rb)) => ra == rb,
        _ => a == b,
    }
}

fn relative_from_repo(paths: &MissionPaths, abs: &Path) -> Option<String> {
    abs.strip_prefix(&paths.repo_root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}
