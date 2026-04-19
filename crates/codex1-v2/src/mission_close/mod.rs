//! Mission-close readiness check.
//!
//! V2 distinguishes three states to avoid V1's "final execution feels like
//! completion" drift:
//!
//! * **ready for mission-close review** — all non-superseded tasks
//!   terminal, no open blocking findings; the mission-close bundle can
//!   now be opened.
//! * **mission-close review passed** — the mission-close bundle closed
//!   clean.
//! * **terminal complete** — `mission-close complete` has transitioned
//!   `STATE.json` to `phase: complete`.
//!
//! This module computes which of those three states the mission is in.

#![allow(dead_code)]

use serde::Serialize;

use crate::graph::Dag;
use crate::mission::lock::{LockStatus, OutcomeLock};
use crate::review::bundle::{
    ReviewBundle, ReviewStatus, ReviewTarget, mission_close_evidence_hash,
};
use crate::state::{Phase, State, TaskStatus};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReadinessReport {
    pub can_close: bool,
    pub can_complete: bool,
    /// Empty iff `can_close`.
    pub blocking_reasons: Vec<BlockingReason>,
    /// Mission-close bundle id if one exists.
    pub mission_close_bundle: Option<String>,
    /// Whether the mission-close bundle has been closed clean.
    pub mission_close_clean: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BlockingReason {
    pub code: String,
    pub task_id: Option<String>,
    pub bundle_id: Option<String>,
    pub detail: String,
}

impl BlockingReason {
    fn task(code: &str, task_id: &str, detail: String) -> Self {
        Self {
            code: code.to_string(),
            task_id: Some(task_id.to_string()),
            bundle_id: None,
            detail,
        }
    }
    fn bundle(code: &str, bundle_id: &str, detail: String) -> Self {
        Self {
            code: code.to_string(),
            task_id: None,
            bundle_id: Some(bundle_id.to_string()),
            detail,
        }
    }
    fn global(code: &str, detail: String) -> Self {
        Self {
            code: code.to_string(),
            task_id: None,
            bundle_id: None,
            detail,
        }
    }
}

/// Compute mission-close readiness from lock + state + DAG + bundle inventory.
#[must_use]
pub fn check_readiness(
    lock: &OutcomeLock,
    state: &State,
    dag: &Dag,
    bundles: &[ReviewBundle],
) -> ReadinessReport {
    let mut reasons: Vec<BlockingReason> = Vec::new();

    // Round 8 Fix #1: the outcome lock must be ratified before terminal
    // close. $clarify flips `lock_status: draft → ratified` only after
    // Destination/Constraints/Success Criteria are committed. Without
    // this check, mission-close can finalize a mission whose destination
    // never left the init template.
    if lock.frontmatter.lock_status != LockStatus::Ratified {
        reasons.push(BlockingReason::global(
            "LOCK_NOT_RATIFIED",
            "OUTCOME-LOCK.md is still draft; run $clarify to ratify \
             before closing the mission"
                .into(),
        ));
    }

    // DAG must not be empty; empty DAG cannot be "complete".
    if dag.is_empty() {
        reasons.push(BlockingReason::global(
            "DAG_EMPTY",
            "mission has no tasks; cannot be completed".into(),
        ));
    }

    // Every non-superseded task must be terminal (ReviewClean or Complete).
    for id in dag.ids() {
        let status = state
            .tasks
            .get(&id)
            .map_or(TaskStatus::Planned, |t| t.status);
        if status == TaskStatus::Superseded {
            continue;
        }
        if !matches!(status, TaskStatus::ReviewClean | TaskStatus::Complete) {
            reasons.push(BlockingReason::task(
                "TASK_NOT_CLEAN",
                &id,
                format!("task status is {status:?}"),
            ));
        }
    }

    // Any task-targeting bundle that is Open or Failed is a blocker.
    for bundle in bundles {
        match (&bundle.target, bundle.status) {
            (ReviewTarget::Task { task_id, .. }, ReviewStatus::Open) => {
                reasons.push(BlockingReason::bundle(
                    "REVIEW_BUNDLE_OPEN",
                    &bundle.bundle_id,
                    format!(
                        "review bundle {} for task {task_id} is still open",
                        bundle.bundle_id
                    ),
                ));
            }
            (ReviewTarget::Task { task_id, .. }, ReviewStatus::Failed) => {
                reasons.push(BlockingReason::bundle(
                    "REVIEW_BUNDLE_FAILED",
                    &bundle.bundle_id,
                    format!(
                        "review bundle {} for task {task_id} has blocking findings",
                        bundle.bundle_id
                    ),
                ));
            }
            _ => {}
        }
    }

    // Every mission-close bundle must be Clean. If any Open or Failed
    // mission-close bundle exists, it blocks — the mission is ambiguously
    // "closed" in some copies and not in others, and V2's whole point is
    // to reject that failure class. The bundle_id surfaced in the report
    // points at the first blocker (so the caller sees what to fix), or
    // falls back to a clean id when the mission is ready to complete.
    let (mc_summary, mc_reasons) = summarise_mission_close(bundles);
    reasons.extend(mc_reasons);

    // Round 8 Fix #2c: a Clean mission-close bundle whose stored
    // evidence hash no longer matches the current terminal-state
    // fingerprint is stale — task truth drifted after the reviewer
    // certified this mission. The bundle is not auto-marked Failed
    // (a projection must not mutate state); the operator re-runs
    // `review open-mission-close` to snapshot the new truth.
    let current_mc_hash = mission_close_evidence_hash(state, dag);
    for b in bundles {
        if !matches!(b.target, ReviewTarget::MissionClose) {
            continue;
        }
        if b.status != ReviewStatus::Clean {
            continue;
        }
        if b.evidence_snapshot_hash != current_mc_hash {
            reasons.push(BlockingReason::bundle(
                "MISSION_CLOSE_STALE",
                &b.bundle_id,
                "mission-close bundle's evidence predates current \
                 terminal task truth; re-run review open-mission-close"
                    .into(),
            ));
        }
    }

    let can_close = reasons.is_empty();
    let can_complete = can_close && state.phase != Phase::Complete;

    ReadinessReport {
        can_close,
        can_complete,
        blocking_reasons: reasons,
        mission_close_bundle: mc_summary.report_id,
        mission_close_clean: mc_summary.all_clean,
    }
}

struct MissionCloseSummary {
    /// True iff at least one Clean bundle exists and there are zero
    /// Open or Failed mission-close bundles.
    all_clean: bool,
    /// Id to surface in the readiness report; prefers a blocker so the
    /// caller sees what needs fixing, falls back to a Clean bundle.
    report_id: Option<String>,
}

fn summarise_mission_close(bundles: &[ReviewBundle]) -> (MissionCloseSummary, Vec<BlockingReason>) {
    let mut mc_open: Vec<&ReviewBundle> = Vec::new();
    let mut mc_failed: Vec<&ReviewBundle> = Vec::new();
    let mut mc_clean: Vec<&ReviewBundle> = Vec::new();
    for b in bundles {
        if !matches!(b.target, ReviewTarget::MissionClose) {
            continue;
        }
        match b.status {
            ReviewStatus::Open => mc_open.push(b),
            ReviewStatus::Failed => mc_failed.push(b),
            ReviewStatus::Clean => mc_clean.push(b),
        }
    }

    let mut reasons: Vec<BlockingReason> = Vec::new();
    if mc_open.is_empty() && mc_failed.is_empty() && mc_clean.is_empty() {
        reasons.push(BlockingReason::global(
            "MISSION_CLOSE_BUNDLE_MISSING",
            "no mission-close review bundle has been opened".into(),
        ));
    }
    for b in &mc_open {
        reasons.push(BlockingReason::bundle(
            "MISSION_CLOSE_OPEN",
            &b.bundle_id,
            "mission-close bundle still open".into(),
        ));
    }
    for b in &mc_failed {
        reasons.push(BlockingReason::bundle(
            "MISSION_CLOSE_FAILED",
            &b.bundle_id,
            "mission-close bundle closed with blocking findings".into(),
        ));
    }

    let all_clean = !mc_clean.is_empty() && mc_open.is_empty() && mc_failed.is_empty();
    let report_id = mc_open
        .first()
        .or(mc_failed.first())
        .or(mc_clean.first())
        .map(|b| b.bundle_id.clone());

    (
        MissionCloseSummary {
            all_clean,
            report_id,
        },
        reasons,
    )
}

#[cfg(test)]
mod tests {
    use super::check_readiness;
    use crate::blueprint::{Blueprint, Level, Planning, TaskSpec};
    use crate::graph::validate::build_dag;
    use crate::mission::lock::{Frontmatter, LockStatus, OutcomeLock};
    use crate::review::bundle::{
        ReviewBundle, ReviewStatus, ReviewTarget, mission_close_evidence_hash,
    };
    use crate::state::{ParentLoop, Phase, State, TaskState, TaskStatus};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn ratified_lock() -> OutcomeLock {
        lock(LockStatus::Ratified)
    }

    fn draft_lock() -> OutcomeLock {
        lock(LockStatus::Draft)
    }

    fn lock(status: LockStatus) -> OutcomeLock {
        OutcomeLock {
            path: PathBuf::from("/x/OUTCOME-LOCK.md"),
            frontmatter: Frontmatter {
                mission_id: "m".into(),
                title: "t".into(),
                lock_status: status,
                created_at: "2026-04-19T00:00:00Z".into(),
                updated_at: "2026-04-19T00:00:00Z".into(),
            },
        }
    }

    fn planning() -> Planning {
        Planning {
            requested_level: Level::Light,
            risk_floor: None,
            effective_level: None,
            graph_revision: 1,
        }
    }

    fn task(id: &str) -> TaskSpec {
        TaskSpec {
            id: id.into(),
            title: id.into(),
            kind: "code".into(),
            depends_on: vec![],
            spec_ref: None,
            read_paths: vec![],
            write_paths: vec![],
            exclusive_resources: vec![],
            proof: vec![],
            review_profiles: vec![],
            unknown_side_effects: false,
            package_manager_mutation: false,
            schema_or_migration: false,
            generated_paths: vec![],
            shared_state: vec![],
            commands: vec![],
            external_services: vec![],
            env_mutations: vec![],
            supersedes: vec![],
        }
    }

    fn state_with(pairs: &[(&str, TaskStatus)], phase: Phase) -> State {
        let mut map = BTreeMap::new();
        for (id, status) in pairs {
            map.insert(
                (*id).to_string(),
                TaskState {
                    status: *status,
                    started_at: None,
                    finished_at: None,
                    reviewed_at: None,
                    task_run_id: None,
                    proof_ref: None,
                    proof_hash: None,
                },
            );
        }
        State {
            mission_id: "m".into(),
            state_revision: 1,
            phase,
            parent_loop: ParentLoop::default(),
            tasks: map,
        }
    }

    fn mission_close_bundle(
        status: ReviewStatus,
        state: &State,
        dag: &crate::graph::Dag,
    ) -> ReviewBundle {
        mission_close_bundle_id("B9", status, state, dag)
    }

    fn mission_close_bundle_id(
        id: &str,
        status: ReviewStatus,
        state: &State,
        dag: &crate::graph::Dag,
    ) -> ReviewBundle {
        ReviewBundle {
            bundle_id: id.into(),
            mission_id: "m".into(),
            graph_revision: 1,
            state_revision: 1,
            target: ReviewTarget::MissionClose,
            requirements: vec![],
            evidence_refs: vec![],
            // Round 8 Fix #2: the evidence hash binds to the terminal
            // truth the reviewer certified. Helper picks the current
            // `(state, dag)` so clean bundles don't trip MISSION_CLOSE_STALE.
            evidence_snapshot_hash: mission_close_evidence_hash(state, dag),
            status,
            opened_at: "t".into(),
            closed_at: None,
            opener_role: "parent".into(),
        }
    }

    #[test]
    fn empty_dag_cannot_close() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[], Phase::Clarify);
        let r = check_readiness(&ratified_lock(), &s, &dag, &[]);
        assert!(!r.can_close);
        assert!(r.blocking_reasons.iter().any(|b| b.code == "DAG_EMPTY"));
    }

    #[test]
    fn non_terminal_task_blocks() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::Ready)], Phase::Executing);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[mission_close_bundle(ReviewStatus::Clean, &s, &dag)],
        );
        assert!(!r.can_close);
        assert!(
            r.blocking_reasons
                .iter()
                .any(|b| b.code == "TASK_NOT_CLEAN")
        );
    }

    #[test]
    fn missing_mission_close_bundle_blocks() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(&ratified_lock(), &s, &dag, &[]);
        assert!(!r.can_close);
        assert!(
            r.blocking_reasons
                .iter()
                .any(|b| b.code == "MISSION_CLOSE_BUNDLE_MISSING")
        );
    }

    #[test]
    fn open_mission_close_bundle_blocks() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[mission_close_bundle(ReviewStatus::Open, &s, &dag)],
        );
        assert!(!r.can_close);
        assert!(
            r.blocking_reasons
                .iter()
                .any(|b| b.code == "MISSION_CLOSE_OPEN")
        );
    }

    #[test]
    fn failed_mission_close_blocks() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[mission_close_bundle(ReviewStatus::Failed, &s, &dag)],
        );
        assert!(!r.can_close);
        assert!(
            r.blocking_reasons
                .iter()
                .any(|b| b.code == "MISSION_CLOSE_FAILED")
        );
    }

    #[test]
    fn clean_tasks_plus_clean_mission_close_can_close() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[mission_close_bundle(ReviewStatus::Clean, &s, &dag)],
        );
        assert!(r.can_close);
        assert!(r.can_complete);
        assert!(r.mission_close_clean);
        assert_eq!(r.mission_close_bundle.as_deref(), Some("B9"));
    }

    #[test]
    fn already_complete_phase_prevents_re_completion() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::Complete)], Phase::Complete);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[mission_close_bundle(ReviewStatus::Clean, &s, &dag)],
        );
        assert!(r.can_close); // can_close is about readiness, not idempotency
        assert!(!r.can_complete); // but complete would be a no-op
    }

    #[test]
    fn superseded_tasks_are_ignored_for_readiness() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1"), task("T2")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(
            &[("T1", TaskStatus::Superseded), ("T2", TaskStatus::Complete)],
            Phase::Executing,
        );
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[mission_close_bundle(ReviewStatus::Clean, &s, &dag)],
        );
        assert!(r.can_close);
    }

    #[test]
    fn clean_plus_open_mission_close_blocks_on_the_open_one() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[
                mission_close_bundle_id("B1", ReviewStatus::Clean, &s, &dag),
                mission_close_bundle_id("B2", ReviewStatus::Open, &s, &dag),
            ],
        );
        assert!(!r.can_close);
        assert!(!r.mission_close_clean);
        let reason = r
            .blocking_reasons
            .iter()
            .find(|b| b.code == "MISSION_CLOSE_OPEN")
            .expect("open mission-close bundle must be a blocking reason");
        assert_eq!(reason.bundle_id.as_deref(), Some("B2"));
        assert_eq!(r.mission_close_bundle.as_deref(), Some("B2"));
    }

    #[test]
    fn clean_plus_failed_mission_close_blocks_on_the_failed_one() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[
                mission_close_bundle_id("B1", ReviewStatus::Clean, &s, &dag),
                mission_close_bundle_id("B2", ReviewStatus::Failed, &s, &dag),
            ],
        );
        assert!(!r.can_close);
        assert!(
            r.blocking_reasons
                .iter()
                .any(|b| b.code == "MISSION_CLOSE_FAILED")
        );
        assert_eq!(r.mission_close_bundle.as_deref(), Some("B2"));
    }

    #[test]
    fn multiple_clean_mission_close_bundles_still_close() {
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[
                mission_close_bundle_id("B1", ReviewStatus::Clean, &s, &dag),
                mission_close_bundle_id("B2", ReviewStatus::Clean, &s, &dag),
            ],
        );
        assert!(r.can_close);
        assert!(r.mission_close_clean);
    }

    #[test]
    fn draft_lock_blocks_even_when_everything_else_is_clean() {
        // Round 8 Fix #1: a mission with review_clean tasks and a clean
        // mission-close bundle is still not ready to close if the
        // outcome lock is still draft. $clarify must have ratified the
        // destination first.
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let r = check_readiness(
            &draft_lock(),
            &s,
            &dag,
            &[mission_close_bundle(ReviewStatus::Clean, &s, &dag)],
        );
        assert!(!r.can_close);
        assert!(
            r.blocking_reasons
                .iter()
                .any(|b| b.code == "LOCK_NOT_RATIFIED")
        );
    }

    #[test]
    fn open_task_bundle_blocks() {
        use crate::review::bundle::ReviewRequirement;
        let dag = build_dag(&Blueprint {
            planning: planning(),
            tasks: vec![task("T1")],
            review_boundaries: vec![],
        })
        .unwrap();
        let s = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let task_bundle = ReviewBundle {
            bundle_id: "B1".into(),
            mission_id: "m".into(),
            graph_revision: 1,
            state_revision: 1,
            target: ReviewTarget::Task {
                task_id: "T1".into(),
                task_run_id: "r".into(),
            },
            requirements: vec![ReviewRequirement {
                id: "req".into(),
                profile: "code_bug_correctness".into(),
                min_outputs: 1,
                allowed_roles: vec!["reviewer".into()],
            }],
            evidence_refs: vec![],
            evidence_snapshot_hash: "sha256:x".into(),
            status: ReviewStatus::Open,
            opened_at: "t".into(),
            closed_at: None,
            opener_role: "parent".into(),
        };
        let r = check_readiness(
            &ratified_lock(),
            &s,
            &dag,
            &[
                task_bundle,
                mission_close_bundle(ReviewStatus::Clean, &s, &dag),
            ],
        );
        assert!(!r.can_close);
        assert!(
            r.blocking_reasons
                .iter()
                .any(|b| b.code == "REVIEW_BUNDLE_OPEN")
        );
    }
}
