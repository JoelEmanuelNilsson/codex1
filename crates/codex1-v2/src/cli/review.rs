//! `codex1 review open | submit | status | close`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use walkdir::WalkDir;

use crate::blueprint;
use crate::envelope;
use crate::error::CliError;
use crate::events::append_event;
use crate::fs_atomic::atomic_write;
use crate::graph;
use crate::mission::resolve_mission;
use crate::review::bundle::{ReviewBundle, ReviewRequirement, ReviewStatus, ReviewTarget};
use crate::review::clean::{CurrentTruth, compute_cleanliness};
use crate::review::output::ReviewerOutput;
use crate::review::{BUNDLES_DIRNAME, OUTPUTS_DIRNAME};
use crate::state::{EventDraft, Phase, StateStore, TaskStatus};

use super::{Cli, emit_error, emit_success, now_rfc3339, resolve_repo};

const OPEN_SCHEMA: &str = "codex1.review.open.v1";
const SUBMIT_SCHEMA: &str = "codex1.review.submit.v1";
const STATUS_SCHEMA: &str = "codex1.review.status.v1";
const CLOSE_SCHEMA: &str = "codex1.review.close.v1";

/// Parent role identifier — `review submit` refuses outputs with this role
/// (parent self-review guard).
const PARENT_ROLE: &str = "parent";

pub fn cmd_review_open(cli: &Cli, mission: &str, task: &str, profiles: &str) -> i32 {
    match run_open(cli, mission, task, profiles) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

#[allow(clippy::too_many_lines)] // Linear bundle construction; splitting obscures the flow.
fn run_open(
    cli: &Cli,
    mission: &str,
    task: &str,
    profiles_csv: &str,
) -> Result<serde_json::Value, CliError> {
    graph::validate::validate_id_format(task)?;
    let profiles: Vec<String> = profiles_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if profiles.is_empty() {
        return Err(CliError::Internal {
            message: "--profiles must list at least one profile".into(),
        });
    }

    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;
    if !dag.tasks.contains_key(task) {
        return Err(CliError::TaskStateTransitionInvalid {
            task_id: task.into(),
            current: "not_in_blueprint".into(),
            attempted: "review_open".into(),
        });
    }

    // Round 6 Fix #3: the provided profiles must be a superset of the
    // task's blueprint review_profiles. Otherwise the parent could open a
    // bundle that drops a mandatory review (e.g., asking only for
    // local_spec_intent on a code task that requires code_bug_correctness)
    // and clear it without the CLI ever noticing the omission.
    let task_spec = dag.tasks.get(task).expect("contains_key checked above");
    let blueprint_profiles: std::collections::BTreeSet<&str> = task_spec
        .review_profiles
        .iter()
        .map(String::as_str)
        .collect();
    let provided_set: std::collections::BTreeSet<&str> =
        profiles.iter().map(String::as_str).collect();
    let missing: Vec<String> = blueprint_profiles
        .difference(&provided_set)
        .map(|s| (*s).to_string())
        .collect();
    if !missing.is_empty() {
        return Err(CliError::ReviewProfileMissing {
            task_id: task.into(),
            missing,
            blueprint: task_spec.review_profiles.clone(),
            provided: profiles.clone(),
        });
    }

    // Check task is in a reviewable state.
    let store = StateStore::new(paths.mission_dir.clone());
    let pre = store.load()?;
    let task_state =
        pre.tasks
            .get(task)
            .cloned()
            .ok_or_else(|| CliError::TaskStateTransitionInvalid {
                task_id: task.into(),
                current: "missing_in_state".into(),
                attempted: "review_open".into(),
            })?;
    if !matches!(
        task_state.status,
        TaskStatus::ProofSubmitted | TaskStatus::ReviewOwed
    ) {
        return Err(CliError::TaskStateTransitionInvalid {
            task_id: task.into(),
            current: format!("{:?}", task_state.status),
            attempted: "review_open".into(),
        });
    }
    let proof_hash = task_state
        .proof_hash
        .clone()
        .ok_or_else(|| CliError::Internal {
            message: format!(
                "task {task} has no recorded proof_hash; run codex1 task finish first"
            ),
        })?;
    let task_run_id = task_state
        .task_run_id
        .clone()
        .ok_or_else(|| CliError::Internal {
            message: format!("task {task} has no recorded task_run_id"),
        })?;

    // Mint bundle id.
    let bundles_dir = paths.mission_dir.join(BUNDLES_DIRNAME);
    fs::create_dir_all(&bundles_dir).map_err(|e| CliError::Io {
        path: bundles_dir.display().to_string(),
        source: e,
    })?;

    // Round 10 P1: refuse if an open review bundle already exists for
    // this (task_id, task_run_id). Without the check, two opens produce
    // two bundles; closing the first flips the task to review_clean and
    // the second bundle can no longer be closed (TASK_STATE_INVALID),
    // leaving `mission-close check` permanently blocked by the dangling
    // open bundle.
    let existing = crate::review::load_all_bundles(&bundles_dir)?;
    if let Some(dup) = existing.iter().find(|b| {
        if b.status != ReviewStatus::Open {
            return false;
        }
        let ReviewTarget::Task {
            task_id: b_task_id,
            task_run_id: b_run_id,
        } = &b.target
        else {
            return false;
        };
        b_task_id == task && b_run_id == &task_run_id
    }) {
        return Err(CliError::ReviewBundleAlreadyOpen {
            task_id: task.into(),
            task_run_id: task_run_id.clone(),
            bundle_id: dup.bundle_id.clone(),
        });
    }

    let bundle_id = next_bundle_id(&bundles_dir)?;

    let requirements: Vec<ReviewRequirement> = profiles
        .iter()
        .map(|profile| ReviewRequirement {
            id: format!("{bundle_id}-{}", profile_slug(profile)),
            profile: profile.clone(),
            min_outputs: 1,
            allowed_roles: vec!["reviewer".into()],
        })
        .collect();

    let bundle = ReviewBundle {
        bundle_id: bundle_id.clone(),
        mission_id: mission.into(),
        graph_revision: dag.graph_revision,
        state_revision: pre.state_revision + 1,
        target: ReviewTarget::Task {
            task_id: task.into(),
            task_run_id,
        },
        requirements,
        evidence_refs: vec![
            task_state
                .proof_ref
                .clone()
                .unwrap_or_else(|| format!("specs/{task}/PROOF.md")),
        ],
        evidence_snapshot_hash: proof_hash.clone(),
        status: ReviewStatus::Open,
        opened_at: now_rfc3339(),
        closed_at: None,
        opener_role: PARENT_ROLE.into(),
    };

    // Mutate state: transition ProofSubmitted → ReviewOwed; persist bundle.
    let task_owned = task.to_string();
    let bundle_id_for_closure = bundle_id.clone();
    let graph_rev = dag.graph_revision;
    let state_after = store.mutate_checked(cli.expect_revision, cli.dry_run, move |state| {
        let entry = state.tasks.get_mut(&task_owned).ok_or_else(|| {
            CliError::TaskStateTransitionInvalid {
                task_id: task_owned.clone(),
                current: "missing_in_state".into(),
                attempted: "review_open".into(),
            }
        })?;
        entry.status = TaskStatus::ReviewOwed;
        state.phase = Phase::Reviewing;
        Ok(EventDraft::new("review_opened")
            .with("task_id", task_owned.as_str())
            .with("bundle_id", bundle_id_for_closure.as_str())
            .with("graph_revision", graph_rev))
    })?;

    // Persist bundle after state mutation committed. In dry-run, the
    // STATE mutation was a preview and the bundle file write must be
    // skipped too; the envelope below still reports the bundle_id that
    // WOULD have been minted.
    if !cli.dry_run {
        let bundle_path = bundle_path(&paths.mission_dir, &bundle_id);
        let bundle_bytes = serde_json::to_vec_pretty(&bundle).map_err(|e| CliError::Internal {
            message: format!("serialize bundle: {e}"),
        })?;
        atomic_write(&bundle_path, &bundle_bytes).map_err(|e| CliError::Io {
            path: bundle_path.display().to_string(),
            source: e,
        })?;
    }

    Ok(envelope::success(
        OPEN_SCHEMA,
        &json!({
            "mission_id": mission,
            "bundle_id": bundle_id,
            "task_id": task,
            "profiles": profiles,
            "evidence_snapshot_hash": proof_hash,
            "state_revision": state_after.state_revision,
            "message": format!("Opened review bundle {bundle_id} for task {task}."),
        }),
    ))
}

pub fn cmd_review_open_mission_close(cli: &Cli, mission: &str, profiles_csv: &str) -> i32 {
    match run_open_mission_close(cli, mission, profiles_csv) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

/// Round 8 Fix #2a: refuse to open a mission-close bundle while any
/// non-superseded task is non-terminal. A mission-close review must
/// certify the terminal surface that exists at open time; opening
/// before tasks are `ReviewClean` or `Complete` lets the bundle close
/// "clean" on work that hasn't actually happened yet.
fn check_tasks_terminal(
    state: &crate::state::State,
    dag: &crate::graph::Dag,
) -> Result<(), CliError> {
    use crate::state::TaskStatus;
    let mut non_terminal: Vec<String> = Vec::new();
    for id in dag.ids() {
        let status = state
            .tasks
            .get(&id)
            .map_or(TaskStatus::Planned, |t| t.status);
        if matches!(status, TaskStatus::Superseded) {
            continue;
        }
        if !matches!(status, TaskStatus::ReviewClean | TaskStatus::Complete) {
            non_terminal.push(id);
        }
    }
    if non_terminal.is_empty() {
        return Ok(());
    }
    let count = non_terminal.len();
    Err(CliError::MissionCloseNotReady {
        task_ids: non_terminal,
        non_terminal_count: count,
    })
}

fn run_open_mission_close(
    cli: &Cli,
    mission: &str,
    profiles_csv: &str,
) -> Result<serde_json::Value, CliError> {
    let profiles: Vec<String> = profiles_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if profiles.is_empty() {
        return Err(CliError::Internal {
            message: "--profiles must list at least one profile".into(),
        });
    }
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    let blueprint = blueprint::parse_blueprint(&paths.program_blueprint())?;
    let dag = graph::build_dag(&blueprint)?;
    let state = StateStore::new(paths.mission_dir.clone()).load()?;

    check_tasks_terminal(&state, &dag)?;

    let bundles_dir = paths.mission_dir.join(BUNDLES_DIRNAME);
    std::fs::create_dir_all(&bundles_dir).map_err(|e| CliError::Io {
        path: bundles_dir.display().to_string(),
        source: e,
    })?;
    let bundle_id = next_bundle_id(&bundles_dir)?;

    let requirements: Vec<ReviewRequirement> = profiles
        .iter()
        .map(|profile| ReviewRequirement {
            id: format!("{bundle_id}-{}", profile_slug(profile)),
            profile: profile.clone(),
            min_outputs: 1,
            allowed_roles: vec!["reviewer".into()],
        })
        .collect();

    // Round 8 Fix #2b: the evidence snapshot binds to terminal truth
    // (graph_revision + sorted (task_id, status, proof_hash) of each
    // non-superseded task), not just the DAG shape. Any post-close
    // state drift makes `check_readiness` recompute a different hash
    // and mark the clean bundle stale (MISSION_CLOSE_STALE).
    let evidence_hash = crate::review::bundle::mission_close_evidence_hash(&state, &dag);

    let bundle = ReviewBundle {
        bundle_id: bundle_id.clone(),
        mission_id: mission.into(),
        graph_revision: dag.graph_revision,
        state_revision: state.state_revision,
        target: ReviewTarget::MissionClose,
        requirements,
        evidence_refs: vec!["PROGRAM-BLUEPRINT.md".into(), "STATE.json".into()],
        evidence_snapshot_hash: evidence_hash.clone(),
        status: ReviewStatus::Open,
        opened_at: now_rfc3339(),
        closed_at: None,
        opener_role: PARENT_ROLE.into(),
    };

    // Dry-run: skip bundle file write + event append; envelope below
    // still reports the bundle_id that WOULD have been minted.
    if !cli.dry_run {
        let bundle_path = bundle_path(&paths.mission_dir, &bundle_id);
        let bytes = serde_json::to_vec_pretty(&bundle).map_err(|e| CliError::Internal {
            message: format!("serialize bundle: {e}"),
        })?;
        crate::fs_atomic::atomic_write(&bundle_path, &bytes).map_err(|e| CliError::Io {
            path: bundle_path.display().to_string(),
            source: e,
        })?;

        // Emit audit event.
        let event = crate::events::Event {
            seq: state.state_revision,
            kind: "mission_close_review_opened".into(),
            at: now_rfc3339(),
            extra: serde_json::Map::from_iter([("bundle_id".into(), json!(bundle_id.clone()))]),
        };
        crate::events::append_event(&paths.mission_dir.join("events.jsonl"), &event).map_err(
            |e| CliError::Io {
                path: paths.mission_dir.display().to_string(),
                source: e,
            },
        )?;
    }

    Ok(envelope::success(
        "codex1.review.open_mission_close.v1",
        &json!({
            "mission_id": mission,
            "bundle_id": bundle_id,
            "profiles": profiles,
            "evidence_snapshot_hash": evidence_hash,
            "message": format!("Opened mission-close review bundle {bundle_id}."),
        }),
    ))
}

pub fn cmd_review_submit(cli: &Cli, mission: &str, bundle: &str, input: &Path) -> i32 {
    match run_submit(cli, mission, bundle, input) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

#[allow(clippy::too_many_lines)] // Submit pipeline is linear.
fn run_submit(
    cli: &Cli,
    mission: &str,
    bundle_id: &str,
    input_path: &Path,
) -> Result<serde_json::Value, CliError> {
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    if !paths.mission_dir.exists() {
        return Err(CliError::MissionNotFound {
            path: paths.mission_dir.display().to_string(),
        });
    }
    let bundle = load_bundle(&paths.mission_dir, bundle_id)?;

    // Read and parse the reviewer output.
    let abs_input = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        repo_root.join(input_path)
    };
    let bytes = fs::read(&abs_input).map_err(|e| CliError::Io {
        path: abs_input.display().to_string(),
        source: e,
    })?;
    let output: ReviewerOutput =
        serde_json::from_slice(&bytes).map_err(|e| CliError::Internal {
            message: format!("parse reviewer output JSON: {e}"),
        })?;
    if output.bundle_id != bundle.bundle_id {
        return Err(CliError::StaleOutput {
            task_id: output.task_id.clone(),
            bundle_id: Some(output.bundle_id.clone()),
            reason: format!(
                "bundle_id mismatch: output {:?} vs bundle {:?}",
                output.bundle_id, bundle.bundle_id
            ),
        });
    }

    // Parent self-review refusal at this layer too — defence in depth.
    if output.reviewer_role == bundle.opener_role {
        return Err(CliError::StaleOutput {
            task_id: output.task_id.clone(),
            bundle_id: Some(bundle.bundle_id.clone()),
            reason: format!(
                "reviewer_role {:?} equals opener_role (parent self-review refused)",
                output.reviewer_role
            ),
        });
    }

    // Compute current truth and run staleness check via cleanliness machinery
    // (a single accepted output tells us the binding was valid).
    let state = StateStore::new(paths.mission_dir.clone()).load()?;
    let (task_id, task_run_id) = match &bundle.target {
        ReviewTarget::Task {
            task_id,
            task_run_id,
        } => (Some(task_id.as_str()), Some(task_run_id.as_str())),
        _ => (None, None),
    };
    let current = CurrentTruth {
        graph_revision: bundle.graph_revision,
        state_revision: state.state_revision,
        evidence_snapshot_hash: &bundle.evidence_snapshot_hash,
        task_run_id,
        task_id,
    };
    let verdict = compute_cleanliness(&bundle, std::slice::from_ref(&output), &current);
    if !verdict.stale_outputs.is_empty() {
        return Err(CliError::StaleOutput {
            task_id: output.task_id.clone(),
            bundle_id: Some(bundle.bundle_id.clone()),
            reason: "binding mismatch (task_run_id / graph_revision / evidence hash)".into(),
        });
    }
    if !verdict.self_review_refused.is_empty() {
        return Err(CliError::StaleOutput {
            task_id: output.task_id.clone(),
            bundle_id: Some(bundle.bundle_id.clone()),
            reason: "reviewer_role not in requirement.allowed_roles".into(),
        });
    }

    // Persist the reviewer output to reviews/outputs/R<N>.json. In
    // dry-run, compute the would-be id + path but don't create the file
    // or append an event.
    let outputs_dir = paths.mission_dir.join(OUTPUTS_DIRNAME);
    if !cli.dry_run {
        fs::create_dir_all(&outputs_dir).map_err(|e| CliError::Io {
            path: outputs_dir.display().to_string(),
            source: e,
        })?;
    }
    let output_id = next_output_id(&outputs_dir).unwrap_or_else(|_| "R1".into());
    let output_path = outputs_dir.join(format!("{output_id}.json"));
    if !cli.dry_run {
        let output_bytes = serde_json::to_vec_pretty(&output).map_err(|e| CliError::Internal {
            message: format!("serialize reviewer output: {e}"),
        })?;
        atomic_write(&output_path, &output_bytes).map_err(|e| CliError::Io {
            path: output_path.display().to_string(),
            source: e,
        })?;

        // Append audit event (outside the state mutation: reviewer submissions do
        // not mutate STATE.json; only `review close` does).
        let event = crate::events::Event {
            seq: state.state_revision,
            kind: "review_output_submitted".into(),
            at: now_rfc3339(),
            extra: serde_json::Map::from_iter([
                ("bundle_id".into(), json!(bundle.bundle_id.clone())),
                ("reviewer_id".into(), json!(output.reviewer_id.clone())),
                ("output_id".into(), json!(output_id.clone())),
                (
                    "result".into(),
                    serde_json::to_value(output.result).unwrap(),
                ),
            ]),
        };
        let events_path = paths.mission_dir.join("events.jsonl");
        append_event(&events_path, &event).map_err(|e| CliError::Io {
            path: events_path.display().to_string(),
            source: e,
        })?;
    }

    Ok(envelope::success(
        SUBMIT_SCHEMA,
        &json!({
            "mission_id": mission,
            "bundle_id": bundle.bundle_id,
            "output_id": output_id,
            "output_path": output_path.display().to_string(),
            "message": format!("Accepted reviewer output {output_id} for bundle {}", bundle.bundle_id),
        }),
    ))
}

pub fn cmd_review_status(cli: &Cli, mission: &str, bundle_id: &str) -> i32 {
    match run_status(cli, mission, bundle_id) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

fn run_status(cli: &Cli, mission: &str, bundle_id: &str) -> Result<serde_json::Value, CliError> {
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    let bundle = load_bundle(&paths.mission_dir, bundle_id)?;
    let outputs = load_outputs_for_bundle(&paths.mission_dir, bundle_id)?;
    let state = StateStore::new(paths.mission_dir.clone()).load()?;
    let (task_id, task_run_id) = match &bundle.target {
        ReviewTarget::Task {
            task_id,
            task_run_id,
        } => (Some(task_id.as_str()), Some(task_run_id.as_str())),
        _ => (None, None),
    };
    let verdict = compute_cleanliness(
        &bundle,
        &outputs,
        &CurrentTruth {
            graph_revision: bundle.graph_revision,
            state_revision: state.state_revision,
            evidence_snapshot_hash: &bundle.evidence_snapshot_hash,
            task_run_id,
            task_id,
        },
    );
    Ok(envelope::success(
        STATUS_SCHEMA,
        &json!({
            "mission_id": mission,
            "bundle_id": bundle.bundle_id,
            "status": bundle.status,
            "clean": verdict.clean,
            "missing_profiles": verdict.missing_profiles,
            "blocking_findings": verdict.blocking_findings,
            "stale_outputs": verdict.stale_outputs,
            "self_review_refused": verdict.self_review_refused,
            "accepted_outputs": verdict.accepted_outputs,
        }),
    ))
}

pub fn cmd_review_close(cli: &Cli, mission: &str, bundle_id: &str) -> i32 {
    match run_close(cli, mission, bundle_id) {
        Ok(env) => emit_success(cli, &env),
        Err(err) => emit_error(cli, &err),
    }
}

#[allow(clippy::too_many_lines)] // Linear close pipeline; splitting obscures the flow.
fn run_close(cli: &Cli, mission: &str, bundle_id: &str) -> Result<serde_json::Value, CliError> {
    let repo_root = resolve_repo(cli)?;
    let paths = resolve_mission(&repo_root, mission)?;
    let bundle = load_bundle(&paths.mission_dir, bundle_id)?;
    if bundle.status != ReviewStatus::Open {
        return Err(CliError::Internal {
            message: format!(
                "bundle {bundle_id} is already {:?}; cannot close twice",
                bundle.status
            ),
        });
    }
    let outputs = load_outputs_for_bundle(&paths.mission_dir, bundle_id)?;

    let store = StateStore::new(paths.mission_dir.clone());
    let state = store.load()?;
    let (task_id, task_run_id) = match &bundle.target {
        ReviewTarget::Task {
            task_id,
            task_run_id,
        } => (Some(task_id.as_str()), Some(task_run_id.as_str())),
        _ => (None, None),
    };
    let verdict = compute_cleanliness(
        &bundle,
        &outputs,
        &CurrentTruth {
            graph_revision: bundle.graph_revision,
            state_revision: state.state_revision,
            evidence_snapshot_hash: &bundle.evidence_snapshot_hash,
            task_run_id,
            task_id,
        },
    );

    let clean = verdict.clean;
    let task_owned = task_id.map(str::to_string);
    let bundle_id_owned = bundle.bundle_id.clone();
    let final_status = if clean {
        ReviewStatus::Clean
    } else {
        ReviewStatus::Failed
    };

    let state_after =
        store.mutate_checked(cli.expect_revision, cli.dry_run, move |state| {
            if let Some(tid) = task_owned.as_ref() {
                let entry = state.tasks.get_mut(tid).ok_or_else(|| {
                    CliError::TaskStateTransitionInvalid {
                        task_id: tid.clone(),
                        current: "missing_in_state".into(),
                        attempted: "review_close".into(),
                    }
                })?;
                if entry.status != TaskStatus::ReviewOwed {
                    return Err(CliError::TaskStateTransitionInvalid {
                        task_id: tid.clone(),
                        current: format!("{:?}", entry.status),
                        attempted: "review_close".into(),
                    });
                }
                // Route failed reviews straight to NeedsRepair so the task is
                // immediately eligible for a retry via `task start`. The audit
                // log preserves which bundle failed.
                entry.status = if clean {
                    TaskStatus::ReviewClean
                } else {
                    TaskStatus::NeedsRepair
                };
                entry.reviewed_at = Some(now_rfc3339());
                if clean {
                    state.phase = Phase::Executing;
                } else {
                    state.phase = Phase::Repairing;
                }
            }
            Ok(EventDraft::new("review_closed")
                .with("bundle_id", bundle_id_owned.clone())
                .with("clean", clean)
                .with("blocking_findings", verdict.blocking_findings))
        })?;

    // Persist the bundle's new status. Skip the file write in dry-run.
    if !cli.dry_run {
        let mut bundle_updated = bundle.clone();
        bundle_updated.status = final_status;
        bundle_updated.closed_at = Some(now_rfc3339());
        let bundle_path = bundle_path(&paths.mission_dir, &bundle.bundle_id);
        let bundle_bytes =
            serde_json::to_vec_pretty(&bundle_updated).map_err(|e| CliError::Internal {
                message: format!("serialize bundle: {e}"),
            })?;
        atomic_write(&bundle_path, &bundle_bytes).map_err(|e| CliError::Io {
            path: bundle_path.display().to_string(),
            source: e,
        })?;
    }

    Ok(envelope::success(
        CLOSE_SCHEMA,
        &json!({
            "mission_id": mission,
            "bundle_id": bundle.bundle_id,
            "clean": clean,
            "task_id": task_id,
            "blocking_findings": verdict.blocking_findings,
            "state_revision": state_after.state_revision,
            "message": if clean {
                format!("Closed review bundle {} — clean.", bundle.bundle_id)
            } else {
                format!(
                    "Closed review bundle {} — {} blocking findings; task routed to repair.",
                    bundle.bundle_id, verdict.blocking_findings
                )
            },
        }),
    ))
}

// ---- helpers ----

fn profile_slug(profile: &str) -> String {
    profile
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn bundle_path(mission_dir: &Path, bundle_id: &str) -> PathBuf {
    mission_dir
        .join(BUNDLES_DIRNAME)
        .join(format!("{bundle_id}.json"))
}

fn next_bundle_id(bundles_dir: &Path) -> Result<String, CliError> {
    let n = scan_max_index(bundles_dir, 'B')?;
    Ok(format!("B{}", n + 1))
}

fn next_output_id(outputs_dir: &Path) -> Result<String, CliError> {
    let n = scan_max_index(outputs_dir, 'R')?;
    Ok(format!("R{}", n + 1))
}

fn scan_max_index(dir: &Path, prefix: char) -> Result<u64, CliError> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut max_n = 0_u64;
    for entry in WalkDir::new(dir).min_depth(1).max_depth(1) {
        let entry = entry.map_err(|e| CliError::Io {
            path: dir.display().to_string(),
            source: e
                .into_io_error()
                .unwrap_or_else(|| std::io::Error::other("walkdir iteration error")),
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_suffix(".json")
            && let Some(rest) = stem.strip_prefix(prefix)
            && let Ok(n) = rest.parse::<u64>()
        {
            max_n = max_n.max(n);
        }
    }
    Ok(max_n)
}

fn load_bundle(mission_dir: &Path, bundle_id: &str) -> Result<ReviewBundle, CliError> {
    let path = bundle_path(mission_dir, bundle_id);
    let bytes = fs::read(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::Internal {
                message: format!("review bundle {bundle_id} not found at {}", path.display()),
            }
        } else {
            CliError::Io {
                path: path.display().to_string(),
                source: e,
            }
        }
    })?;
    serde_json::from_slice(&bytes).map_err(|e| CliError::Internal {
        message: format!("parse bundle {bundle_id}: {e}"),
    })
}

fn load_outputs_for_bundle(
    mission_dir: &Path,
    bundle_id: &str,
) -> Result<Vec<ReviewerOutput>, CliError> {
    let dir = mission_dir.join(OUTPUTS_DIRNAME);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(&dir).min_depth(1).max_depth(1) {
        let entry = entry.map_err(|e| CliError::Io {
            path: dir.display().to_string(),
            source: e
                .into_io_error()
                .unwrap_or_else(|| std::io::Error::other("walkdir error")),
        })?;
        if entry.file_type().is_file() {
            let bytes = fs::read(entry.path()).map_err(|e| CliError::Io {
                path: entry.path().display().to_string(),
                source: e,
            })?;
            let parsed: ReviewerOutput =
                serde_json::from_slice(&bytes).map_err(|e| CliError::Internal {
                    message: format!("parse reviewer output {}: {e}", entry.path().display()),
                })?;
            if parsed.bundle_id == bundle_id {
                out.push(parsed);
            }
        }
    }
    out.sort_by(|a, b| a.packet_id.cmp(&b.packet_id));
    Ok(out)
}
