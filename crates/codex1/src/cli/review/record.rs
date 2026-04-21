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
use crate::cli::review::start::ensure_review_deps_ready;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::{ensure_artifact_parent_write_safe, MissionPaths};
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

    if !peek.plan.locked && peek.close.terminal_at.is_none() {
        if !ctx.dry_run {
            state::mutate(
                &paths,
                ctx.expect_revision,
                "review.stale",
                json!({
                    "review_task_id": inputs.task_id,
                    "verdict": verdict_str(&verdict),
                    "reviewers": reviewers,
                    "targets": [],
                }),
                |_state| Ok(()),
            )?;
        }
        return Err(CliError::StaleReviewRecord {
            message: format!(
                "Review {} arrived while PLAN.yaml is unlocked for replan; record not applied",
                inputs.task_id
            ),
        });
    }

    let plan_tasks = load_tasks(&paths, &peek)?;
    let review_task = fetch_review_task(&plan_tasks, inputs.task_id)?;
    let targets = review_targets(&review_task)?;

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
    ensure_review_deps_ready(&peek, &review_task)?;

    let findings_body = if let Some(src) = findings_path {
        Some(std::fs::read(src)?)
    } else {
        None
    };
    let findings_source_is_existing_artifact = findings_path.is_some_and(|src| {
        let dest = paths.review_file_for(inputs.task_id);
        src.canonicalize().ok() == dest.canonicalize().ok()
    });

    let review_task_id = inputs.task_id.to_string();
    let review_task_for_closure = review_task.clone();
    let targets_for_closure = targets.clone();
    let reviewers_for_closure = reviewers.clone();
    let findings_body_for_closure = findings_body.clone();

    let event_kind = if matches!(verdict, ReviewVerdict::Clean) {
        "review.recorded.clean"
    } else {
        "review.recorded.dirty"
    };

    let mutation = state::mutate_dynamic(&paths, ctx.expect_revision, |state| {
        let applied = apply_record(
            state,
            ApplyRecordInput {
                review_task_id: &review_task_id,
                review_task: &review_task_for_closure,
                targets: &targets_for_closure,
                verdict: &verdict,
                reviewers: &reviewers_for_closure,
                paths: &paths,
                findings_body: findings_body_for_closure.as_deref(),
                findings_source_is_existing_artifact,
            },
        )?;
        let event_kind = if matches!(applied.category, ReviewRecordCategory::StaleSuperseded) {
            "review.stale".to_string()
        } else {
            event_kind.to_string()
        };
        Ok((
            event_kind,
            json!({
                "review_task_id": review_task_id,
                "verdict": verdict_str(&verdict),
                "reviewers": reviewers_for_closure,
                "targets": targets_for_closure,
                "category": category_str(applied.category.clone()),
                "findings_file": applied.findings_rel,
            }),
        ))
    })?;

    let category = mutation
        .event
        .payload
        .get("category")
        .and_then(|v| v.as_str())
        .map_or(ReviewRecordCategory::AcceptedCurrent, category_from_str);
    let stored_findings_rel = mutation
        .event
        .payload
        .get("findings_file")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let replan_triggered = mutation.state.replan.triggered;
    let warnings = match category {
        ReviewRecordCategory::LateSameBoundary => {
            vec!["recorded as late_same_boundary: state advanced since review start".to_string()]
        }
        _ => Vec::new(),
    };

    if matches!(category, ReviewRecordCategory::StaleSuperseded) {
        return Err(CliError::StaleReviewRecord {
            message: format!(
                "Review {} or one of its targets is superseded; record not applied",
                inputs.task_id
            ),
        });
    }

    if matches!(category, ReviewRecordCategory::AcceptedCurrent) {
        if let (Some(body), Some(_rel)) = (findings_body.as_deref(), stored_findings_rel.as_ref()) {
            let dest = paths.review_file_for(inputs.task_id);
            ensure_artifact_parent_write_safe(&paths, &dest)?;
            atomic_write(&dest, body)?;
        }
    }

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

fn category_from_str(raw: &str) -> ReviewRecordCategory {
    match raw {
        "late_same_boundary" => ReviewRecordCategory::LateSameBoundary,
        "stale_superseded" => ReviewRecordCategory::StaleSuperseded,
        "contaminated_after_terminal" => ReviewRecordCategory::ContaminatedAfterTerminal,
        _ => ReviewRecordCategory::AcceptedCurrent,
    }
}

/// Mutate the state inside `state::mutate`. Classification is re-computed
/// against the fresh state read under the lock (not the peek).
struct ApplyRecordInput<'a> {
    review_task_id: &'a str,
    review_task: &'a crate::cli::review::plan_read::PlanTask,
    targets: &'a [String],
    verdict: &'a ReviewVerdict,
    reviewers: &'a [String],
    paths: &'a MissionPaths,
    findings_body: Option<&'a [u8]>,
    findings_source_is_existing_artifact: bool,
}

fn apply_record(
    state: &mut MissionState,
    input: ApplyRecordInput<'_>,
) -> Result<AppliedRecord, CliError> {
    // Re-classify under the lock — the peek may be stale if another writer
    // mutated between peek and lock acquisition.
    let category = classify(&ClassifyInput {
        state,
        review_task_id: input.review_task_id,
        target_task_ids: input.targets,
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
            return Ok(AppliedRecord {
                category,
                findings_rel: None,
            });
        }
        _ => {}
    }

    if state.close.terminal_at.is_none() {
        state::require_plan_locked(state)?;
    }
    ensure_review_deps_ready(state, input.review_task)?;

    let boundary_revision = state
        .reviews
        .get(input.review_task_id)
        .map_or(state.revision.saturating_add(1), |r| r.boundary_revision);
    let recorded_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    let findings_rel = if matches!(category, ReviewRecordCategory::AcceptedCurrent) {
        if let Some(body) = input.findings_body {
            let dest = input.paths.review_file_for(input.review_task_id);
            if input.findings_source_is_existing_artifact
                && std::fs::read(&dest).ok().as_deref() == Some(body)
            {
                return Ok(AppliedRecord {
                    category: ReviewRecordCategory::LateSameBoundary,
                    findings_rel: None,
                });
            }
            relative_from_repo(input.paths, &dest)
        } else {
            None
        }
    } else {
        None
    };

    let record = ReviewRecord {
        task_id: input.review_task_id.to_string(),
        verdict: input.verdict.clone(),
        reviewers: input.reviewers.to_vec(),
        findings_file: findings_rel.clone(),
        category: category.clone(),
        recorded_at,
        boundary_revision,
    };

    // Only accepted_current records affect the dirty counter / target
    // status. Non-current categories are audit-only and must not replace
    // current review truth in STATE.json.
    if matches!(category, ReviewRecordCategory::AcceptedCurrent) {
        state
            .reviews
            .insert(input.review_task_id.to_string(), record);
        match *input.verdict {
            ReviewVerdict::Clean => apply_clean(state, input.review_task_id, input.targets),
            ReviewVerdict::Dirty => apply_dirty(state, input.review_task_id, input.targets),
            ReviewVerdict::Pending => {}
        }
    }
    Ok(AppliedRecord {
        category,
        findings_rel,
    })
}

struct AppliedRecord {
    category: ReviewRecordCategory,
    findings_rel: Option<String>,
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

fn relative_from_repo(paths: &MissionPaths, abs: &Path) -> Option<String> {
    abs.strip_prefix(&paths.repo_root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}
