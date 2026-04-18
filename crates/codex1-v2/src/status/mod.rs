//! Status projection — `(State, Dag) → StatusEnvelope`.
//!
//! This module produces the payload Ralph consumes from
//! `codex1 status --mission <id> --json`. `verdict` is the primary field;
//! everything else must be internally consistent with it.
//!
//! Wave 1 emits four `verdict` values:
//!
//! | `verdict` | when |
//! | --- | --- |
//! | `continue_required` | at least one task is eligible to start |
//! | `needs_user` | no eligible task but mission is not complete |
//! | `blocked` | ready/needs-repair tasks exist but their deps are not clean |
//! | `complete` | every non-superseded task is terminal (and DAG non-empty) |
//! | `invalid_state` | consistency check between stored phase and task states failed |
//!
//! Parent-loop is static `{active: false, mode: none, paused: false}` in
//! Wave 1; later waves populate it via `parent_loop::project`.

// T12 (`cli::status`) is the first non-test caller.
#![allow(dead_code)]

use serde::Serialize;

use crate::graph::{waves::derive_waves, Dag};
use crate::state::{ParentLoopMode, Phase, State, TaskStatus};

/// Schema string for the status envelope.
pub const SCHEMA: &str = "codex1.status.v1";

/// Status envelope as emitted on the wire.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StatusEnvelope {
    pub mission_id: String,
    pub state_revision: u64,
    pub phase: Phase,
    pub terminality: Terminality,
    pub verdict: Verdict,
    pub parent_loop: ParentLoopView,
    pub stop_policy: StopPolicy,
    pub next_action: NextAction,
    pub ready_tasks: Vec<String>,
    pub running_tasks: Vec<String>,
    pub review_required: Vec<String>,
    pub blocked: Vec<String>,
    pub stale: Vec<String>,
    pub required_user_decision: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Terminality {
    NonTerminal,
    Terminal,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    ContinueRequired,
    NeedsUser,
    Blocked,
    Complete,
    InvalidState,
}

/// Projection view of `parent_loop`. Distinct from `state::ParentLoop` so the
/// wire format can expose the derived `active` bool without changing STATE.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ParentLoopView {
    pub active: bool,
    pub mode: ParentLoopMode,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StopPolicy {
    pub allow_stop: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NextAction {
    pub kind: NextActionKind,
    pub task_id: Option<String>,
    pub args: Vec<String>,
    pub display_message: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NextActionKind {
    StartTask,
    /// Wave 2+: a task has `ProofSubmitted` or `ReviewOwed` status and needs
    /// a review bundle opened against it.
    ReviewOpen,
    UserDecision,
    Complete,
    InvalidState,
}

/// Project the given state and DAG into a Wave-1 status envelope.
#[must_use]
#[allow(clippy::too_many_lines)] // Envelope construction is linear and reads better
                                 // as one function than split into helpers.
pub fn project_status(state: &State, dag: &Dag) -> StatusEnvelope {
    let parent_loop = ParentLoopView {
        active: false,
        mode: state.parent_loop.mode,
        paused: state.parent_loop.paused,
    };

    if let Some(reason) = detect_invalid_state(state, dag) {
        return StatusEnvelope {
            mission_id: state.mission_id.clone(),
            state_revision: state.state_revision,
            phase: state.phase,
            terminality: Terminality::NonTerminal,
            verdict: Verdict::InvalidState,
            parent_loop,
            stop_policy: StopPolicy {
                allow_stop: true,
                reason: "invalid_state".into(),
            },
            next_action: NextAction {
                kind: NextActionKind::InvalidState,
                task_id: None,
                args: vec![],
                display_message:
                    "Mission state is inconsistent; run codex1 validate."
                        .into(),
            },
            ready_tasks: vec![],
            running_tasks: running_task_ids(state),
            review_required: vec![],
            blocked: vec![],
            stale: vec![],
            required_user_decision: Some(reason),
        };
    }

    if mission_is_complete(state, dag) {
        return StatusEnvelope {
            mission_id: state.mission_id.clone(),
            state_revision: state.state_revision,
            phase: state.phase,
            terminality: Terminality::Terminal,
            verdict: Verdict::Complete,
            parent_loop,
            stop_policy: StopPolicy {
                allow_stop: true,
                reason: "complete".into(),
            },
            next_action: NextAction {
                kind: NextActionKind::Complete,
                task_id: None,
                args: vec![],
                display_message: "Mission is complete.".into(),
            },
            ready_tasks: vec![],
            running_tasks: vec![],
            review_required: vec![],
            blocked: vec![],
            stale: vec![],
            required_user_decision: None,
        };
    }

    let waves = derive_waves(dag, state);
    let ready_tasks: Vec<String> = waves
        .waves
        .iter()
        .flat_map(|w| w.tasks.iter().cloned())
        .collect();
    let blocked_ids: Vec<String> = waves
        .blocked
        .iter()
        .map(|b| b.task_id.clone())
        .collect();
    let review_required = collect_review_required(state);

    if let Some(first_task) = ready_tasks.first().cloned() {
        return StatusEnvelope {
            mission_id: state.mission_id.clone(),
            state_revision: state.state_revision,
            phase: state.phase,
            terminality: Terminality::NonTerminal,
            verdict: Verdict::ContinueRequired,
            parent_loop,
            stop_policy: StopPolicy {
                allow_stop: true,
                reason: "no_active_loop".into(),
            },
            next_action: NextAction {
                kind: NextActionKind::StartTask,
                task_id: Some(first_task.clone()),
                args: vec![
                    "--mission".into(),
                    state.mission_id.clone(),
                    first_task.clone(),
                ],
                display_message: format!("Start task {first_task}."),
            },
            ready_tasks,
            running_tasks: running_task_ids(state),
            review_required: review_required.clone(),
            blocked: blocked_ids,
            stale: vec![],
            required_user_decision: None,
        };
    }

    // No eligible task — but review may still be owed. Emit review_open when
    // at least one task is proof_submitted or review_owed.
    if let Some(first_review) = review_required.first().cloned() {
        return StatusEnvelope {
            mission_id: state.mission_id.clone(),
            state_revision: state.state_revision,
            phase: state.phase,
            terminality: Terminality::NonTerminal,
            verdict: Verdict::ContinueRequired,
            parent_loop,
            stop_policy: StopPolicy {
                allow_stop: true,
                reason: "no_active_loop".into(),
            },
            next_action: NextAction {
                kind: NextActionKind::ReviewOpen,
                task_id: Some(first_review.clone()),
                args: vec![
                    "--mission".into(),
                    state.mission_id.clone(),
                    "--task".into(),
                    first_review.clone(),
                ],
                display_message: format!(
                    "Open a review for task {first_review}."
                ),
            },
            ready_tasks: vec![],
            running_tasks: running_task_ids(state),
            review_required,
            blocked: blocked_ids,
            stale: vec![],
            required_user_decision: None,
        };
    }

    // No eligible task. Either the DAG is empty, tasks are blocked, tasks are
    // Planned (need a $plan/$clarify transition), or some other user-decision
    // condition.
    let (verdict, required) = categorise_no_eligible(state, dag, &blocked_ids);
    let display = decision_message(required.as_deref(), &blocked_ids);
    StatusEnvelope {
        mission_id: state.mission_id.clone(),
        state_revision: state.state_revision,
        phase: state.phase,
        terminality: Terminality::NonTerminal,
        verdict,
        parent_loop,
        stop_policy: StopPolicy {
            allow_stop: true,
            reason: "no_active_loop".into(),
        },
        next_action: NextAction {
            kind: NextActionKind::UserDecision,
            task_id: None,
            args: vec![],
            display_message: display,
        },
        ready_tasks: vec![],
        running_tasks: running_task_ids(state),
        review_required,
        blocked: blocked_ids,
        stale: vec![],
        required_user_decision: required,
    }
}

fn detect_invalid_state(state: &State, dag: &Dag) -> Option<String> {
    let statuses: Vec<TaskStatus> = dag
        .ids()
        .iter()
        .map(|id| task_status(state, id))
        .collect();
    let non_sup: Vec<TaskStatus> = statuses
        .iter()
        .copied()
        .filter(|s| *s != TaskStatus::Superseded)
        .collect();

    if state.phase == Phase::Complete
        && non_sup.iter().any(|s| !s.is_terminal())
    {
        return Some(
            "stored_phase_complete_but_non_terminal_task".into(),
        );
    }

    let has_in_progress = non_sup.contains(&TaskStatus::InProgress);
    if has_in_progress && state.phase != Phase::Executing {
        return Some(
            "in_progress_task_requires_executing_phase".into(),
        );
    }

    if !dag.is_empty()
        && !non_sup.is_empty()
        && non_sup.iter().all(|s| s.is_terminal())
        && state.phase != Phase::Complete
    {
        return Some(
            "all_tasks_terminal_requires_complete_phase".into(),
        );
    }

    None
}

fn mission_is_complete(state: &State, dag: &Dag) -> bool {
    if dag.is_empty() {
        return false;
    }
    let non_sup: Vec<TaskStatus> = dag
        .ids()
        .iter()
        .map(|id| task_status(state, id))
        .filter(|s| *s != TaskStatus::Superseded)
        .collect();
    !non_sup.is_empty() && non_sup.iter().all(|s| s.is_terminal())
}

fn collect_review_required(state: &State) -> Vec<String> {
    let mut v: Vec<String> = state
        .tasks
        .iter()
        .filter(|(_, t)| {
            matches!(
                t.status,
                TaskStatus::ProofSubmitted | TaskStatus::ReviewOwed
            )
        })
        .map(|(id, _)| id.clone())
        .collect();
    v.sort();
    v
}

fn running_task_ids(state: &State) -> Vec<String> {
    let mut v: Vec<String> = state
        .tasks
        .iter()
        .filter(|(_, t)| t.status == TaskStatus::InProgress)
        .map(|(id, _)| id.clone())
        .collect();
    v.sort();
    v
}

fn task_status(state: &State, id: &str) -> TaskStatus {
    state
        .tasks
        .get(id)
        .map_or(TaskStatus::Planned, |t| t.status)
}

fn categorise_no_eligible(
    state: &State,
    dag: &Dag,
    blocked_ids: &[String],
) -> (Verdict, Option<String>) {
    if dag.is_empty() {
        return (Verdict::NeedsUser, Some("plan_dag_empty".into()));
    }
    if !blocked_ids.is_empty() {
        return (Verdict::Blocked, Some("dependencies_blocked".into()));
    }
    // DAG has tasks but none are Ready/NeedsRepair/Terminal — probably all
    // Planned (no state entry, or explicit Planned). A $plan run is needed
    // to promote them to Ready.
    let any_planned = dag
        .ids()
        .iter()
        .any(|id| matches!(task_status(state, id), TaskStatus::Planned));
    if any_planned {
        (Verdict::NeedsUser, Some("tasks_awaiting_plan_ready".into()))
    } else {
        (Verdict::NeedsUser, Some("no_runnable_tasks".into()))
    }
}

fn decision_message(required: Option<&str>, blocked: &[String]) -> String {
    match required {
        Some("plan_dag_empty") => "Run $plan to author the task DAG.".into(),
        Some("tasks_awaiting_plan_ready") => {
            "Tasks exist but none are ready; run $plan or $clarify.".into()
        }
        Some("dependencies_blocked") => {
            format!(
                "All runnable tasks are blocked on dependencies: {}.",
                blocked.join(", ")
            )
        }
        _ => "No runnable tasks; inspect mission state.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        project_status, NextActionKind, Terminality, Verdict,
    };
    use crate::blueprint::{Blueprint, Level, Planning, TaskSpec};
    use crate::graph::validate::build_dag;
    use crate::state::{ParentLoop, Phase, State, TaskState, TaskStatus};
    use std::collections::BTreeMap;

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

    fn state_with(tasks: &[(&str, TaskStatus)], phase: Phase) -> State {
        let mut map = BTreeMap::new();
        for (id, status) in tasks {
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
            mission_id: "example".into(),
            state_revision: 1,
            phase,
            parent_loop: ParentLoop::default(),
            tasks: map,
        }
    }

    fn dag_of(tasks: Vec<TaskSpec>) -> crate::graph::Dag {
        build_dag(&Blueprint {
            planning: planning(),
            tasks,
            review_boundaries: vec![],
        })
        .unwrap()
    }

    #[test]
    fn fresh_mission_with_empty_dag_is_needs_user() {
        let dag = dag_of(vec![]);
        let state = state_with(&[], Phase::Clarify);
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::NeedsUser);
        assert_eq!(s.next_action.kind, NextActionKind::UserDecision);
        assert_eq!(s.required_user_decision.as_deref(), Some("plan_dag_empty"));
        assert_eq!(s.terminality, Terminality::NonTerminal);
        assert!(!s.parent_loop.paused);
        assert!(!s.parent_loop.active);
        assert!(s.stop_policy.allow_stop);
        assert_eq!(s.stop_policy.reason, "no_active_loop");
    }

    #[test]
    fn ready_task_produces_continue_required_and_start_task() {
        let dag = dag_of(vec![task("T1")]);
        let state = state_with(&[("T1", TaskStatus::Ready)], Phase::Clarify);
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::ContinueRequired);
        assert_eq!(s.next_action.kind, NextActionKind::StartTask);
        assert_eq!(s.next_action.task_id.as_deref(), Some("T1"));
        assert_eq!(
            s.next_action.args,
            vec!["--mission", "example", "T1"]
        );
        assert!(s
            .next_action
            .display_message
            .contains("Start task T1"));
        assert_eq!(s.ready_tasks, vec!["T1".to_string()]);
    }

    #[test]
    fn all_terminal_plus_complete_phase_produces_complete() {
        let dag = dag_of(vec![task("T1")]);
        let state = state_with(&[("T1", TaskStatus::Complete)], Phase::Complete);
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::Complete);
        assert_eq!(s.terminality, Terminality::Terminal);
        assert_eq!(s.next_action.kind, NextActionKind::Complete);
        assert_eq!(s.stop_policy.reason, "complete");
    }

    #[test]
    fn dep_blocked_task_produces_blocked_verdict() {
        let mut t2 = task("T2");
        t2.depends_on = vec!["T1".into()];
        let dag = dag_of(vec![task("T1"), t2]);
        let state = state_with(
            &[
                ("T1", TaskStatus::InProgress),
                ("T2", TaskStatus::Ready),
            ],
            Phase::Executing,
        );
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::Blocked);
        assert_eq!(
            s.required_user_decision.as_deref(),
            Some("dependencies_blocked")
        );
        assert_eq!(s.running_tasks, vec!["T1".to_string()]);
        assert_eq!(s.blocked, vec!["T2".to_string()]);
    }

    #[test]
    fn invalid_state_phase_complete_with_non_terminal_task() {
        let dag = dag_of(vec![task("T1")]);
        let state = state_with(&[("T1", TaskStatus::Ready)], Phase::Complete);
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::InvalidState);
        assert_eq!(s.next_action.kind, NextActionKind::InvalidState);
        assert_eq!(s.stop_policy.reason, "invalid_state");
        assert_eq!(
            s.required_user_decision.as_deref(),
            Some("stored_phase_complete_but_non_terminal_task")
        );
    }

    #[test]
    fn invalid_state_in_progress_without_executing_phase() {
        let dag = dag_of(vec![task("T1")]);
        let state = state_with(&[("T1", TaskStatus::InProgress)], Phase::Clarify);
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::InvalidState);
        assert_eq!(
            s.required_user_decision.as_deref(),
            Some("in_progress_task_requires_executing_phase")
        );
    }

    #[test]
    fn invalid_state_all_terminal_without_complete_phase() {
        let dag = dag_of(vec![task("T1")]);
        let state = state_with(&[("T1", TaskStatus::ReviewClean)], Phase::Executing);
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::InvalidState);
        assert_eq!(
            s.required_user_decision.as_deref(),
            Some("all_tasks_terminal_requires_complete_phase")
        );
    }

    #[test]
    fn tasks_awaiting_plan_ready_when_only_planned_tasks_exist() {
        let dag = dag_of(vec![task("T1")]);
        let state = state_with(&[("T1", TaskStatus::Planned)], Phase::Clarify);
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::NeedsUser);
        assert_eq!(
            s.required_user_decision.as_deref(),
            Some("tasks_awaiting_plan_ready")
        );
    }

    #[test]
    fn wave_1_parent_loop_is_static_inactive_none_unpaused() {
        let dag = dag_of(vec![]);
        let state = state_with(&[], Phase::Clarify);
        let s = project_status(&state, &dag);
        assert!(!s.parent_loop.active);
        assert!(!s.parent_loop.paused);
        // mode matches state.parent_loop.mode (none by default)
        assert!(matches!(
            s.parent_loop.mode,
            crate::state::ParentLoopMode::None
        ));
    }

    #[test]
    fn superseded_tasks_do_not_block_complete() {
        let mut t2 = task("T2");
        t2.supersedes = vec!["T1".into()];
        let dag = dag_of(vec![task("T1"), t2]);
        let state = state_with(
            &[
                ("T1", TaskStatus::Superseded),
                ("T2", TaskStatus::Complete),
            ],
            Phase::Complete,
        );
        let s = project_status(&state, &dag);
        assert_eq!(s.verdict, Verdict::Complete);
    }

    #[test]
    fn envelope_serializes_with_expected_field_names() {
        let dag = dag_of(vec![]);
        let state = state_with(&[], Phase::Clarify);
        let s = project_status(&state, &dag);
        let json = serde_json::to_value(&s).unwrap();
        for key in [
            "mission_id",
            "state_revision",
            "phase",
            "terminality",
            "verdict",
            "parent_loop",
            "stop_policy",
            "next_action",
            "ready_tasks",
            "running_tasks",
            "review_required",
            "blocked",
            "stale",
            "required_user_decision",
        ] {
            assert!(
                json.get(key).is_some(),
                "expected field {key:?} in envelope"
            );
        }
        assert_eq!(json["verdict"], "needs_user");
        assert_eq!(json["parent_loop"]["mode"], "none");
    }
}
