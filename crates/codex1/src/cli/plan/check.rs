//! `codex1 plan check` — validate PLAN.yaml structure and task DAG.
//!
//! On success (non-dry-run): lock the plan, record its SHA-256 hash, and
//! advance `state.phase` from `plan` to `execute` via the mutation
//! protocol. Re-running on an already-locked plan with an unchanged hash
//! is idempotent — no state change, no new event.
//!
//! Error envelopes for PLAN_INVALID / DAG_CYCLE / DAG_MISSING_DEP carry a
//! structured `context` field (e.g. `{ "task_id": "T3", "missing_dep":
//! "T99" }`). Because the canonical `CliError::context()` for these
//! variants defaults to `Value::Null` and `src/core/**` is foundation-owned,
//! validation failures are printed here via `exit_with_validation_error`
//! (exits code 1) so the top-level dispatcher never re-prints them.

use std::collections::BTreeSet;
use std::fs;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::cli::Ctx;
use crate::core::envelope::{JsonErr, JsonOk};
use crate::core::error::{CliError, CliResult};
use crate::core::mission::resolve_mission;
use crate::core::paths::{resolve_existing_mission_file, MissionPaths};
use crate::state::{self, Phase, PlanLevel, TaskStatus};

use super::dag::{topo_sort, TopoOutcome};
use super::parsed::{ParsedPlan, TaskSpec, HARD_EVIDENCE_KINDS, PLAN_LEVELS, TASK_KINDS};

pub fn run(ctx: &Ctx) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let current = state::load(&paths)?;
    state::check_expected_revision(ctx.expect_revision, &current)?;
    if !current.outcome.ratified {
        return Err(CliError::OutcomeNotRatified);
    }
    let plan_path = paths.plan();
    if !plan_path.is_file() {
        return Err(CliError::PlanInvalid {
            message: format!("PLAN.yaml missing at {}", plan_path.display()),
            hint: Some("Run `codex1 plan scaffold --level <level>` first.".to_string()),
        });
    }

    let raw = fs::read_to_string(&plan_path)?;
    if let Some(pos) = raw.find("[codex1-fill:") {
        let preview: String = raw[pos..].chars().take(60).collect();
        exit_with_validation_error(
            "PLAN_INVALID",
            &format!("PLAN.yaml still contains a [codex1-fill:…] marker: {preview}"),
            Some("Replace every [codex1-fill:…] marker with the real plan content."),
            json!({ "marker_preview": preview }),
        );
    }

    let parsed: ParsedPlan = serde_yaml::from_str(&raw).map_err(|err| CliError::PlanInvalid {
        message: format!("PLAN.yaml is not valid YAML: {err}"),
        hint: Some("Re-run `codex1 plan scaffold` to restore the template.".to_string()),
    })?;

    let summary = validate(&parsed, &paths, &current);

    let hash = plan_hash(raw.as_bytes());

    // Idempotent short-circuit: same hash on an already-locked plan → no mutation.
    //
    // Exception — upgrade-in-place: if a previously-locked state has no
    // `plan.task_ids` snapshot (pre-F8 binary, or hand-edited state),
    // fall through so the mutation closure backfills it. Without this
    // guard, an upgraded binary would perpetually re-enter the
    // short-circuit and `status` / `close check` would be stuck with
    // `verdict=continue_required` and no actionable blockers.
    let hash_matches = current.plan.locked && current.plan.hash.as_deref() == Some(hash.as_str());
    let task_ids_missing = current.plan.task_ids.is_empty();
    let already_locked_same = hash_matches && !task_ids_missing;

    if ctx.dry_run || already_locked_same {
        // Dry-run reports `locked: false` regardless of current state
        // (the invariant is "this call did not mutate"). Idempotent
        // re-runs report `locked: true` to confirm the plan is locked.
        let reported_locked = !ctx.dry_run && already_locked_same;
        let env = JsonOk::new(
            Some(paths.mission_id.clone()),
            Some(current.revision),
            json!({
                "tasks": summary.total_tasks,
                "review_tasks": summary.review_tasks,
                "hard_evidence": summary.hard_evidence_count,
                "plan_hash": hash,
                "locked": reported_locked,
            }),
        );
        println!("{}", env.to_pretty());
        return Ok(());
    }

    let event_payload = json!({
        "plan_hash": hash,
        "tasks": summary.total_tasks,
        "review_tasks": summary.review_tasks,
        "hard_evidence": summary.hard_evidence_count,
        "requested_level": level_str(&summary.requested_level),
        "effective_level": level_str(&summary.effective_level),
    });

    let task_ids_to_record = summary.task_ids.clone();
    let mutation = state::mutate(
        &paths,
        ctx.expect_revision,
        "plan.checked",
        event_payload,
        |s| {
            if !s.outcome.ratified {
                return Err(CliError::OutcomeNotRatified);
            }
            s.plan.locked = true;
            s.plan.hash = Some(hash.clone());
            s.plan.requested_level = Some(summary.requested_level.clone());
            s.plan.effective_level = Some(summary.effective_level.clone());
            // Snapshot the full DAG task-id list so `state::readiness`
            // can recognize "all DAG nodes done" without silently
            // ignoring DAG nodes that were never started.
            s.plan.task_ids.clone_from(&task_ids_to_record);
            if matches!(s.phase, Phase::Plan) {
                s.phase = Phase::Execute;
            }
            // A successful `plan check` that relocks the plan resolves
            // any pending replan: the replan cycle ended when the user
            // authored a new/amended PLAN.yaml and asked us to lock it.
            // Without this clear, `state.replan.triggered` would stay
            // `true` forever — `status.verdict` stuck at `blocked`,
            // `close check`/`close complete` refusing to advance,
            // `close record-review` refusing to record. See round-2
            // e2e P0-1.
            s.replan.triggered = false;
            s.replan.triggered_reason = None;
            Ok(())
        },
    )?;

    let env = JsonOk::new(
        Some(paths.mission_id.clone()),
        Some(mutation.new_revision),
        json!({
            "tasks": summary.total_tasks,
            "review_tasks": summary.review_tasks,
            "hard_evidence": summary.hard_evidence_count,
            "plan_hash": hash,
            "locked": true,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

struct Summary {
    total_tasks: usize,
    review_tasks: usize,
    hard_evidence_count: usize,
    requested_level: PlanLevel,
    effective_level: PlanLevel,
    /// Full DAG task-id list in plan order, used by `plan.task_ids`.
    task_ids: Vec<String>,
}

/// Validate the parsed plan. On first failure this exits the process with
/// a structured error envelope on stdout — never returns the error up the
/// call stack, so callers never have to worry about context round-tripping.
fn validate(plan: &ParsedPlan, paths: &MissionPaths, state: &state::MissionState) -> Summary {
    // Top-level sections.
    require_string("mission_id", plan.mission_id.as_deref());

    let level = plan.planning_level.as_ref().unwrap_or_else(|| {
        exit_with_validation_error(
            "PLAN_INVALID",
            "planning_level missing",
            Some("Add planning_level.requested and planning_level.effective."),
            Value::Null,
        )
    });
    let requested_level = parse_level("planning_level.requested", level.requested.as_deref());
    let effective_level = parse_level("planning_level.effective", level.effective.as_deref());

    let outcome = plan.outcome_interpretation.as_ref().unwrap_or_else(|| {
        exit_with_validation_error(
            "PLAN_INVALID",
            "outcome_interpretation missing",
            Some("Add outcome_interpretation.summary."),
            Value::Null,
        )
    });
    require_string("outcome_interpretation.summary", outcome.summary.as_deref());

    let arch = plan.architecture.as_ref().unwrap_or_else(|| {
        exit_with_validation_error(
            "PLAN_INVALID",
            "architecture missing",
            Some("Add architecture.summary and architecture.key_decisions[]."),
            Value::Null,
        )
    });
    require_string("architecture.summary", arch.summary.as_deref());
    if arch.key_decisions.is_empty() {
        exit_with_validation_error(
            "PLAN_INVALID",
            "architecture.key_decisions is empty",
            Some("Record at least one key decision."),
            Value::Null,
        );
    }

    let process = plan.planning_process.as_ref().unwrap_or_else(|| {
        exit_with_validation_error(
            "PLAN_INVALID",
            "planning_process missing",
            Some("Add planning_process.evidence[]."),
            Value::Null,
        )
    });

    if plan.tasks.is_empty() {
        exit_with_validation_error(
            "PLAN_INVALID",
            "tasks list is empty",
            Some("Every mission needs at least one task."),
            Value::Null,
        );
    }

    if plan.risks.is_empty() {
        exit_with_validation_error(
            "PLAN_INVALID",
            "risks list is empty",
            Some("Record at least one risk + mitigation."),
            Value::Null,
        );
    }
    for (idx, risk) in plan.risks.iter().enumerate() {
        require_string(&format!("risks[{idx}].risk"), risk.risk.as_deref());
        require_string(
            &format!("risks[{idx}].mitigation"),
            risk.mitigation.as_deref(),
        );
    }

    let mission_close = plan.mission_close.as_ref().unwrap_or_else(|| {
        exit_with_validation_error(
            "PLAN_INVALID",
            "mission_close missing",
            Some("Add mission_close.criteria[]."),
            Value::Null,
        )
    });
    if mission_close.criteria.is_empty() {
        exit_with_validation_error(
            "PLAN_INVALID",
            "mission_close.criteria is empty",
            Some("List the conditions that must hold before close."),
            Value::Null,
        );
    }

    // Task-level validation.
    let task_ids = validate_tasks(&plan.tasks, paths, state);

    // Review-loop deadlock: a non-review task `Tx` depends on a task
    // `Td` that is the review target of some review `Tr`, AND `Tr`
    // (transitively) depends on `Tx`. `Td` can only become Complete
    // via `Tr` clean review, but `Tr` cannot start until `Tx` is
    // Complete — unreachable. Detect this before locking so skills do
    // not land a plan that deadlocks on first execution. See
    // `docs/audits/round-1/e2e-walkthrough.md` P2-1.
    detect_review_loop_deadlock(&plan.tasks);

    // Hard-plan evidence gate.
    let hard_evidence_count = process
        .evidence
        .iter()
        .filter(|e| {
            e.kind
                .as_deref()
                .is_some_and(|k| HARD_EVIDENCE_KINDS.contains(&k))
        })
        .count();
    if matches!(effective_level, PlanLevel::Hard) {
        if process.evidence.is_empty() {
            exit_with_validation_error(
                "PLAN_INVALID",
                "planning_process.evidence is empty but effective level is hard",
                Some("Hard planning requires recorded evidence (explorer/advisor/plan_review)."),
                json!({ "missing_hard_evidence": true, "hard_evidence_count": 0 }),
            );
        }
        if hard_evidence_count == 0 {
            exit_with_validation_error(
                "PLAN_INVALID",
                "no hard-qualifying evidence entries in planning_process.evidence",
                Some(
                    "Add at least one evidence entry with kind in {explorer, advisor, plan_review}.",
                ),
                json!({
                    "missing_hard_evidence": true,
                    "recorded_evidence_kinds": process
                        .evidence
                        .iter()
                        .filter_map(|e| e.kind.clone())
                        .collect::<Vec<_>>(),
                }),
            );
        }
    }

    let review_tasks = plan
        .tasks
        .iter()
        .filter(|t| t.kind.as_deref() == Some("review"))
        .count();

    // Cycle check (missing-dep already handled in validate_tasks).
    let mut deps = std::collections::BTreeMap::new();
    for t in &plan.tasks {
        let id = t.id.clone().unwrap_or_default();
        let ds = t.depends_on.clone().unwrap_or_default();
        deps.insert(id, ds);
    }
    if let TopoOutcome::Cycle { remaining, edges } = topo_sort(&deps) {
        let edges_json: Vec<Value> = edges.iter().map(|(a, b)| json!([a, b])).collect();
        exit_with_validation_error(
            "DAG_CYCLE",
            &format!("cycle involving task(s): {}", remaining.join(", ")),
            Some("Break the cycle by removing or redirecting one of the depends_on edges."),
            json!({
                "cycle_nodes": remaining,
                "cycle_edges": edges_json,
            }),
        );
    }

    Summary {
        total_tasks: task_ids.len(),
        review_tasks,
        hard_evidence_count,
        requested_level,
        effective_level,
        task_ids,
    }
}

/// Detect the review-loop deadlock shape described in the module
/// docstring. Exits with a structured `PLAN_INVALID` envelope on hit
/// and returns normally when the plan is reachable.
fn detect_review_loop_deadlock(tasks: &[TaskSpec]) {
    use std::collections::{BTreeMap, BTreeSet};
    // Build dep lookup keyed by id (exclude entries with missing ids —
    // earlier passes already reject those).
    let deps_by_id: BTreeMap<String, BTreeSet<String>> = tasks
        .iter()
        .filter_map(|t| {
            let id = t.id.clone()?;
            let deps = t.depends_on.clone().unwrap_or_default();
            Some((id, deps.into_iter().collect()))
        })
        .collect();
    // For every review task, enumerate the set of ids that (transitively)
    // depend on it — these are the tasks that cannot run until the
    // review clears.
    for review_task in tasks.iter().filter(|t| t.kind.as_deref() == Some("review")) {
        let Some(review_id) = review_task.id.clone() else {
            continue;
        };
        let Some(target) = review_task.review_target.as_ref() else {
            continue;
        };
        let targets: BTreeSet<String> = target.tasks.iter().cloned().collect();
        let upstream = ancestors(&review_id, &deps_by_id);
        // Any non-review task whose `depends_on` contains one of the
        // review's targets AND that also appears upstream of the
        // review is a deadlock: the target cannot progress without
        // the review, and the review cannot progress without the task.
        for task in tasks {
            let Some(tid) = task.id.clone() else { continue };
            if task.kind.as_deref() == Some("review") {
                continue;
            }
            if !upstream.contains(&tid) {
                continue;
            }
            let Some(task_deps) = task.depends_on.as_ref() else {
                continue;
            };
            if let Some(bad_dep) = task_deps.iter().find(|d| targets.contains(*d)) {
                exit_with_validation_error(
                    "PLAN_INVALID",
                    &format!(
                        "review-loop deadlock: task `{tid}` depends on `{bad_dep}` which can only be reviewed by `{review_id}`, but `{review_id}` transitively depends on `{tid}`"
                    ),
                    Some(
                        "Either move the review after its targets (remove the upstream dep) or drop the circular dep.",
                    ),
                    json!({
                        "task_id": tid,
                        "dep": bad_dep,
                        "review_task_id": review_id,
                    }),
                );
            }
        }
    }
}

/// Return the set of task ids that are (transitively) depended upon by
/// `start`. Self is not included. Walks the `depends_on` graph upward
/// with a visited-set to avoid infinite loops on malformed cycles
/// (cycles are caught separately by `topo_sort`).
fn ancestors(
    start: &str,
    deps_by_id: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
) -> std::collections::BTreeSet<String> {
    use std::collections::BTreeSet;
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = deps_by_id
        .get(start)
        .map(|ds| ds.iter().cloned().collect())
        .unwrap_or_default();
    while let Some(id) = stack.pop() {
        if !out.insert(id.clone()) {
            continue;
        }
        if let Some(ds) = deps_by_id.get(&id) {
            for d in ds {
                if !out.contains(d) {
                    stack.push(d.clone());
                }
            }
        }
    }
    out
}

fn validate_tasks(
    tasks: &[TaskSpec],
    paths: &MissionPaths,
    state: &state::MissionState,
) -> Vec<String> {
    let mut ids: Vec<String> = Vec::with_capacity(tasks.len());
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();

    // First pass: presence, pattern, duplicates.
    for (idx, task) in tasks.iter().enumerate() {
        let id = task.id.clone().unwrap_or_else(|| {
            exit_with_validation_error(
                "PLAN_INVALID",
                &format!("tasks[{idx}] is missing id"),
                Some("Every task needs `id: T<n>`."),
                json!({ "task_index": idx }),
            )
        });
        if !is_valid_task_id(&id) {
            exit_with_validation_error(
                "PLAN_INVALID",
                &format!("task id `{id}` does not match ^T\\d+$"),
                Some("Use ids like T1, T2, T3; no leading zero, no suffixes."),
                json!({ "invalid_id": id }),
            );
        }
        if !seen.insert(id.clone()) {
            duplicates.insert(id.clone());
        }
        ids.push(id);
    }
    if !duplicates.is_empty() {
        let dups: Vec<String> = duplicates.iter().cloned().collect();
        exit_with_validation_error(
            "PLAN_INVALID",
            &format!("duplicate task ids: {}", dups.join(", ")),
            Some("Task ids must be unique across the plan."),
            json!({ "duplicate_ids": dups }),
        );
    }
    let id_set: BTreeSet<_> = ids.iter().cloned().collect();

    // Second pass: required fields, kind, deps, review targets, spec file.
    let non_review_ids: BTreeSet<String> = tasks
        .iter()
        .filter(|t| t.kind.as_deref() != Some("review"))
        .filter_map(|t| t.id.clone())
        .collect();

    for task in tasks {
        let id = task.id.clone().unwrap_or_default();
        let task_is_superseded = state
            .tasks
            .get(&id)
            .is_some_and(|record| matches!(record.status, TaskStatus::Superseded));
        require_string(&format!("tasks[{id}].title"), task.title.as_deref());

        let kind = task.kind.as_deref().unwrap_or_else(|| {
            exit_with_validation_error(
                "PLAN_INVALID",
                &format!("tasks[{id}] is missing kind"),
                Some(&format!("Use one of {}.", TASK_KINDS.join(", "))),
                json!({ "task_id": id }),
            )
        });
        if !TASK_KINDS.contains(&kind) {
            exit_with_validation_error(
                "PLAN_INVALID",
                &format!("tasks[{id}].kind `{kind}` is not a known kind"),
                Some(&format!("Use one of {}.", TASK_KINDS.join(", "))),
                json!({ "task_id": id, "kind": kind }),
            );
        }

        let deps = task.depends_on.as_ref().unwrap_or_else(|| {
            exit_with_validation_error(
                "PLAN_INVALID",
                &format!("tasks[{id}] is missing depends_on"),
                Some("Use `depends_on: []` for root tasks."),
                json!({ "task_id": id }),
            )
        });
        for dep in deps {
            if dep == &id {
                exit_with_validation_error(
                    "DAG_CYCLE",
                    &format!("tasks[{id}] depends on itself"),
                    Some("Break the cycle by removing or redirecting one of the depends_on edges."),
                    json!({
                        "cycle_nodes": [id.clone()],
                        "cycle_edges": [[id.clone(), id.clone()]],
                    }),
                );
            }
            if !id_set.contains(dep) {
                exit_with_validation_error(
                    "DAG_MISSING_DEP",
                    &format!("tasks[{id}] depends_on unknown task `{dep}`"),
                    Some(
                        "Ensure every depends_on entry references an existing task id (e.g. T1, T2).",
                    ),
                    json!({
                        "task_id": id,
                        "missing_dep": dep,
                    }),
                );
            }
            if !task_is_superseded
                && task.kind.as_deref() != Some("review")
                && state
                    .tasks
                    .get(dep)
                    .is_some_and(|record| matches!(record.status, TaskStatus::Superseded))
            {
                exit_with_validation_error(
                    "PLAN_INVALID",
                    &format!("tasks[{id}] depends on superseded task `{dep}`"),
                    Some("Retarget live work to replacement task ids after replan."),
                    json!({
                        "task_id": id,
                        "superseded_dep": dep,
                    }),
                );
            }
        }

        require_string(&format!("tasks[{id}].spec"), task.spec.as_deref());
        let spec_rel = task.spec.as_deref().unwrap_or("");
        if let Err(err) =
            resolve_existing_mission_file(paths, spec_rel, &format!("tasks[{id}].spec"))
        {
            let hint = err.hint();
            exit_with_validation_error(
                err.code(),
                &err.to_string(),
                hint.as_deref(),
                json!({
                    "task_id": id,
                    "spec": spec_rel,
                }),
            );
        }

        if kind == "review" {
            let target = task.review_target.as_ref().unwrap_or_else(|| {
                exit_with_validation_error(
                    "PLAN_INVALID",
                    &format!("review task {id} is missing review_target"),
                    Some(
                        "Add `review_target:\n  tasks: [T…]` listing the non-review tasks under review.",
                    ),
                    json!({ "task_id": id }),
                )
            });
            if target.tasks.is_empty() {
                exit_with_validation_error(
                    "PLAN_INVALID",
                    &format!("review task {id} has empty review_target.tasks"),
                    Some("List at least one target task id."),
                    json!({ "task_id": id }),
                );
            }
            for t in &target.tasks {
                if !non_review_ids.contains(t) {
                    exit_with_validation_error(
                        "PLAN_INVALID",
                        &format!(
                            "review task {id} targets `{t}` which is not a known non-review task"
                        ),
                        Some("review_target.tasks must reference existing non-review task ids."),
                        json!({
                            "task_id": id,
                            "invalid_target": t,
                        }),
                    );
                }
                if !task_is_superseded
                    && state
                        .tasks
                        .get(t)
                        .is_some_and(|record| matches!(record.status, TaskStatus::Superseded))
                {
                    exit_with_validation_error(
                        "PLAN_INVALID",
                        &format!("review task {id} targets superseded task `{t}`"),
                        Some(
                            "Remove obsolete review tasks or retarget them to live replacement task ids.",
                        ),
                        json!({
                            "task_id": id,
                            "superseded_target": t,
                        }),
                    );
                }
                if !deps.iter().any(|dep| dep == t) {
                    exit_with_validation_error(
                        "PLAN_INVALID",
                        &format!("review task {id} targets `{t}` but does not depend on it"),
                        Some(
                            "Add every review_target task id to the review task's depends_on list.",
                        ),
                        json!({
                            "task_id": id,
                            "missing_dep": t,
                            "review_target": target.tasks,
                        }),
                    );
                }
            }
        }
    }

    ids
}

fn parse_level(field: &str, value: Option<&str>) -> PlanLevel {
    let value = value.unwrap_or_else(|| {
        exit_with_validation_error(
            "PLAN_INVALID",
            &format!("{field} missing"),
            Some(&format!("Use one of {}.", PLAN_LEVELS.join(", "))),
            Value::Null,
        )
    });
    match value {
        "light" => PlanLevel::Light,
        "medium" => PlanLevel::Medium,
        "hard" => PlanLevel::Hard,
        other => exit_with_validation_error(
            "PLAN_INVALID",
            &format!("{field} `{other}` is not a known planning level"),
            Some(&format!("Use one of {}.", PLAN_LEVELS.join(", "))),
            json!({ "field": field, "value": other }),
        ),
    }
}

fn require_string(field: &str, value: Option<&str>) {
    match value.map(str::trim) {
        Some(s) if !s.is_empty() => {}
        _ => exit_with_validation_error(
            "PLAN_INVALID",
            &format!("{field} is missing or empty"),
            None,
            json!({ "field": field }),
        ),
    }
}

/// Print a validation error envelope to stdout and exit the process with
/// code 1. Used for every fail-early branch in `plan check`.
#[cold]
fn exit_with_validation_error(code: &str, message: &str, hint: Option<&str>, context: Value) -> ! {
    use std::io::Write as _;
    let env = JsonErr::new(
        code.to_string(),
        message.to_string(),
        hint.map(ToString::to_string),
        false,
        context,
    );
    println!("{}", env.to_pretty());
    // Explicit flush in case process::exit skips destructors that would
    // drain buffered stdout.
    let _ = std::io::stdout().flush();
    std::process::exit(1)
}

/// Matches `^T\d+$` with the additional constraint that the digit run
/// cannot start with `0` (so `T1` is valid but `T01` and `T0` are not).
fn is_valid_task_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('T') else {
        return false;
    };
    let mut chars = rest.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() && c != '0' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_digit())
}

fn plan_hash(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for b in digest {
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn level_str(l: &PlanLevel) -> &'static str {
    match l {
        PlanLevel::Light => "light",
        PlanLevel::Medium => "medium",
        PlanLevel::Hard => "hard",
    }
}
