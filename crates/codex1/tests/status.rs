//! Integration tests for `codex1 status --json` (Phase B Unit 11).
//!
//! Seeds `MissionState` and PLAN.yaml by hand, then shells out to the
//! CLI and asserts on the published projection. This catches any
//! drift between `state::readiness` and the JSON shape consumed by
//! Ralph/skills.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use codex1::state::schema::{
    CloseState, LoopMode, LoopState, MissionCloseReviewState, MissionState, OutcomeState, Phase,
    PlanState, ReplanState, ReviewRecord, ReviewRecordCategory, ReviewVerdict, TaskRecord,
    TaskStatus, SCHEMA_VERSION,
};
use serde_json::Value;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

struct Fixture {
    tmp: TempDir,
    mission: String,
}

impl Fixture {
    fn new(mission: &str) -> Self {
        let tmp = TempDir::new().unwrap();
        let mission_dir = tmp.path().join("PLANS").join(mission);
        fs::create_dir_all(mission_dir.join("specs")).unwrap();
        fs::create_dir_all(mission_dir.join("reviews")).unwrap();
        fs::write(mission_dir.join("EVENTS.jsonl"), "").unwrap();
        Self {
            tmp,
            mission: mission.to_string(),
        }
    }

    fn mission_dir(&self) -> PathBuf {
        self.tmp.path().join("PLANS").join(&self.mission)
    }

    fn write_state(&self, state: &MissionState) {
        let path = self.mission_dir().join("STATE.json");
        let raw = serde_json::to_vec_pretty(state).unwrap();
        write_atomic(&path, &raw);
    }

    fn write_plan(&self, body: &str) {
        let path = self.mission_dir().join("PLAN.yaml");
        write_atomic(&path, body.as_bytes());
    }

    fn status(&self) -> Value {
        let out = cmd()
            .current_dir(self.tmp.path())
            .args(["status", "--mission", &self.mission])
            .output()
            .expect("runs");
        let stdout = std::str::from_utf8(&out.stdout).expect("utf-8");
        serde_json::from_str(stdout).unwrap_or_else(|e| panic!("non-JSON stdout:\n{stdout}\n{e}"))
    }
}

fn write_atomic(path: &Path, data: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, data).unwrap();
}

fn base_state(mission_id: &str) -> MissionState {
    MissionState {
        mission_id: mission_id.to_string(),
        revision: 0,
        schema_version: SCHEMA_VERSION,
        phase: Phase::Clarify,
        loop_: LoopState::default(),
        outcome: OutcomeState::default(),
        plan: PlanState::default(),
        tasks: BTreeMap::new(),
        reviews: BTreeMap::new(),
        replan: ReplanState::default(),
        close: CloseState::default(),
        events_cursor: 0,
    }
}

fn simple_plan() -> &'static str {
    r"mission_id: demo
tasks:
  - id: T1
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
  - id: T3
    kind: code
    depends_on: [T1]
    spec: specs/T3/SPEC.md
  - id: T4
    kind: review
    depends_on: [T2, T3]
    spec: specs/T4/SPEC.md
    review_target:
      tasks: [T2, T3]
"
}

fn task(id: &str, status: TaskStatus) -> (String, TaskRecord) {
    (
        id.to_string(),
        TaskRecord {
            id: id.to_string(),
            status,
            started_at: None,
            finished_at: None,
            proof_path: None,
            superseded_by: None,
        },
    )
}

fn review(
    task_id: &str,
    verdict: ReviewVerdict,
    category: ReviewRecordCategory,
    boundary: u64,
) -> (String, ReviewRecord) {
    (
        task_id.to_string(),
        ReviewRecord {
            task_id: task_id.to_string(),
            verdict,
            reviewers: vec!["code-reviewer".to_string()],
            findings_file: None,
            category,
            recorded_at: "2026-04-20T00:00:00Z".to_string(),
            boundary_revision: boundary,
        },
    )
}

#[test]
fn fresh_init_reports_clarify_next_action() {
    let fx = Fixture::new("demo");
    let state = base_state("demo");
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["stop"]["allow"], true);
    assert_eq!(json["data"]["next_action"]["kind"], "clarify");
    assert_eq!(json["data"]["outcome_ratified"], false);
    assert_eq!(json["data"]["plan_locked"], false);
}

#[test]
fn outcome_ratified_plan_unlocked_reports_plan_next_action() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.phase = Phase::Plan;
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["next_action"]["kind"], "plan");
    assert_eq!(json["data"]["outcome_ratified"], true);
    assert_eq!(json["data"]["plan_locked"], false);
}

#[test]
fn plan_locked_and_tasks_pending_reports_run_wave_or_task() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::Execute;
    state.tasks.insert(
        task("T1", TaskStatus::Ready).0,
        task("T1", TaskStatus::Ready).1,
    );
    state.tasks.insert(
        task("T2", TaskStatus::Pending).0,
        task("T2", TaskStatus::Pending).1,
    );
    state.tasks.insert(
        task("T3", TaskStatus::Pending).0,
        task("T3", TaskStatus::Pending).1,
    );
    state.tasks.insert(
        task("T4", TaskStatus::Pending).0,
        task("T4", TaskStatus::Pending).1,
    );
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "continue_required");
    let kind = json["data"]["next_action"]["kind"].as_str().unwrap();
    assert!(
        kind == "run_wave" || kind == "run_task",
        "expected run_wave or run_task, got {kind}"
    );
    // W1 has only T1, so this should be run_task.
    assert_eq!(kind, "run_task");
    assert_eq!(json["data"]["next_action"]["task_id"], "T1");
}

#[test]
fn plan_locked_wave_with_two_ready_reports_run_wave() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::Execute;
    state.tasks.insert(
        task("T1", TaskStatus::Complete).0,
        task("T1", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T2", TaskStatus::Ready).0,
        task("T2", TaskStatus::Ready).1,
    );
    state.tasks.insert(
        task("T3", TaskStatus::Ready).0,
        task("T3", TaskStatus::Ready).1,
    );
    state.tasks.insert(
        task("T4", TaskStatus::Pending).0,
        task("T4", TaskStatus::Pending).1,
    );
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["next_action"]["kind"], "run_wave");
    assert_eq!(json["data"]["next_action"]["wave_id"], "W2");
    let tasks: Vec<String> = json["data"]["next_action"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(tasks.contains(&"T2".to_string()));
    assert!(tasks.contains(&"T3".to_string()));
    assert_eq!(json["data"]["parallel_safe"], true);
}

#[test]
fn active_loop_not_paused_sets_stop_active_loop() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::Execute;
    state.loop_ = LoopState {
        active: true,
        paused: false,
        mode: LoopMode::Execute,
    };
    state.tasks.insert(
        task("T1", TaskStatus::Ready).0,
        task("T1", TaskStatus::Ready).1,
    );
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "continue_required");
    assert_eq!(json["data"]["stop"]["allow"], false);
    assert_eq!(json["data"]["stop"]["reason"], "active_loop");
}

#[test]
fn active_loop_paused_sets_stop_paused() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::Execute;
    state.loop_ = LoopState {
        active: true,
        paused: true,
        mode: LoopMode::Execute,
    };
    state.tasks.insert(
        task("T1", TaskStatus::Ready).0,
        task("T1", TaskStatus::Ready).1,
    );
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["stop"]["allow"], true);
    assert_eq!(json["data"]["stop"]["reason"], "paused");
}

#[test]
fn replan_triggered_reports_blocked_and_replan_next_action() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::Execute;
    state.replan.triggered = true;
    state.replan.triggered_reason = Some("six consecutive dirty reviews".to_string());
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "blocked");
    assert_eq!(json["data"]["next_action"]["kind"], "replan");
    assert_eq!(json["data"]["replan_required"], true);
}

#[test]
fn all_tasks_complete_reports_ready_for_mission_close_review() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.plan.task_ids = vec!["T1".into(), "T2".into(), "T3".into(), "T4".into()];
    state.phase = Phase::MissionClose;
    state.tasks.insert(
        task("T1", TaskStatus::Complete).0,
        task("T1", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T2", TaskStatus::Complete).0,
        task("T2", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T3", TaskStatus::Complete).0,
        task("T3", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T4", TaskStatus::Complete).0,
        task("T4", TaskStatus::Complete).1,
    );
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "ready_for_mission_close_review");
    assert_eq!(json["data"]["next_action"]["kind"], "mission_close_review");
    assert_eq!(json["data"]["close_ready"], false);
}

#[test]
fn mission_close_review_passed_reports_close_next_action() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.plan.task_ids = vec!["T1".into(), "T2".into(), "T3".into(), "T4".into()];
    state.phase = Phase::MissionClose;
    state.tasks.insert(
        task("T1", TaskStatus::Complete).0,
        task("T1", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T2", TaskStatus::Complete).0,
        task("T2", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T3", TaskStatus::Complete).0,
        task("T3", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T4", TaskStatus::Complete).0,
        task("T4", TaskStatus::Complete).1,
    );
    state.close.review_state = MissionCloseReviewState::Passed;
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "mission_close_review_passed");
    assert_eq!(json["data"]["close_ready"], true);
    assert_eq!(json["data"]["next_action"]["kind"], "close");
}

#[test]
fn terminal_complete_reports_closed_next_action() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::Terminal;
    state.close.terminal_at = Some("2026-04-20T12:00:00Z".to_string());
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "terminal_complete");
    assert_eq!(json["data"]["next_action"]["kind"], "closed");
    assert_eq!(json["data"]["stop"]["allow"], true);
    assert_eq!(json["data"]["stop"]["reason"], "terminal");
}

#[test]
fn dirty_review_reports_repair_next_action() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::ReviewLoop;
    state.tasks.insert(
        task("T1", TaskStatus::Complete).0,
        task("T1", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T2", TaskStatus::AwaitingReview).0,
        task("T2", TaskStatus::AwaitingReview).1,
    );
    state.tasks.insert(
        task("T3", TaskStatus::AwaitingReview).0,
        task("T3", TaskStatus::AwaitingReview).1,
    );
    state.tasks.insert(
        task("T4", TaskStatus::Pending).0,
        task("T4", TaskStatus::Pending).1,
    );
    let (rid, rec) = review(
        "T4",
        ReviewVerdict::Dirty,
        ReviewRecordCategory::AcceptedCurrent,
        3,
    );
    state.reviews.insert(rid, rec);
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "blocked");
    assert_eq!(json["data"]["next_action"]["kind"], "repair");
    let ids: Vec<String> = json["data"]["next_action"]["task_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"T2".to_string()));
    assert!(ids.contains(&"T3".to_string()));
}

#[test]
fn ready_review_task_sets_review_required() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.phase = Phase::ReviewLoop;
    state.tasks.insert(
        task("T1", TaskStatus::Complete).0,
        task("T1", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T2", TaskStatus::Complete).0,
        task("T2", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T3", TaskStatus::Complete).0,
        task("T3", TaskStatus::Complete).1,
    );
    state.tasks.insert(
        task("T4", TaskStatus::Ready).0,
        task("T4", TaskStatus::Ready).1,
    );
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "continue_required");
    let reviews = json["data"]["review_required"].as_array().unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0]["task_id"], "T4");
    assert_eq!(json["data"]["next_action"]["kind"], "run_review");
    assert_eq!(json["data"]["next_action"]["review_task_id"], "T4");
}

#[test]
fn revision_matches_state_revision() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.revision = 15;
    state.outcome.ratified = true;
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["revision"], 15);
    assert_eq!(json["mission_id"], "demo");
}

/// Regression for round-2 e2e P2-1: `codex1 task next` must honor the
/// same `plan.locked=false` / `replan.triggered=true` short-circuits
/// as `codex1 status`. Without these, a skill calling `task next`
/// directly could be handed a wave while `status` is telling it to
/// plan or replan.
#[test]
fn task_next_unlocked_plan_emits_plan_kind() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = false;
    // Seed task records that WOULD otherwise show up as a wave.
    state
        .tasks
        .insert("T1".into(), task("T1", TaskStatus::Ready).1);
    state
        .tasks
        .insert("T2".into(), task("T2", TaskStatus::Pending).1);
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let out = cmd()
        .current_dir(fx.tmp.path())
        .args(["task", "next", "--mission", &fx.mission])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "task next should succeed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["data"]["next"]["kind"], "plan");
}

/// Regression for round-2 e2e P2-1: when `replan.triggered=true` but
/// the plan is still locked (the 6th-dirty review just fired), `task
/// next` must emit `kind=replan`, not a fresh review/wave.
#[test]
fn task_next_replan_triggered_emits_replan_kind() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.replan.triggered = true;
    state.replan.triggered_reason = Some("six consecutive dirty reviews for T3".to_string());
    state
        .tasks
        .insert("T1".into(), task("T1", TaskStatus::Complete).1);
    state
        .tasks
        .insert("T2".into(), task("T2", TaskStatus::AwaitingReview).1);
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let out = cmd()
        .current_dir(fx.tmp.path())
        .args(["task", "next", "--mission", &fx.mission])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "task next should succeed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["data"]["next"]["kind"], "replan");
    assert_eq!(
        json["data"]["next"]["reason"],
        "six consecutive dirty reviews for T3"
    );
}

/// Regression for e2e-walkthrough round-1 P2-2: when the plan is
/// unlocked (e.g. after `replan record`), `ready_tasks` must be empty
/// so skills reading `ready_tasks nonempty + next_action.kind=plan`
/// never get mixed signals.
#[test]
fn unlocked_plan_emits_empty_ready_tasks_and_review_required() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = false;
    // Seed task records that WOULD otherwise show up as ready.
    state
        .tasks
        .insert("T1".into(), task("T1", TaskStatus::Ready).1);
    state
        .tasks
        .insert("T2".into(), task("T2", TaskStatus::Pending).1);
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["plan_locked"], false);
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["next_action"]["kind"], "plan");
    assert_eq!(json["data"]["ready_tasks"], serde_json::json!([]));
    assert_eq!(json["data"]["review_required"], serde_json::json!([]));
    assert_eq!(json["data"]["parallel_safe"], false);
}

/// Regression for e2e-walkthrough round-1 P2-1b: the "No ready wave
/// derivable" fallback must not claim the plan is missing/empty when
/// the real blocker is a task sitting in `awaiting_review` with no
/// ready review task.
#[test]
fn blocked_surfaces_awaiting_review_when_plan_is_valid() {
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = true;
    state.plan.task_ids = vec!["T1".into(), "T2".into(), "T3".into()];
    // Seed tasks: T1 complete, T2 awaiting_review, T3 (the review)
    // already complete. No wave is ready (nothing to start), but T2
    // is still awaiting_review. The projection must surface that
    // concrete block instead of the "plan may be missing" fallback.
    state
        .tasks
        .insert("T1".into(), task("T1", TaskStatus::Complete).1);
    state
        .tasks
        .insert("T2".into(), task("T2", TaskStatus::AwaitingReview).1);
    state
        .tasks
        .insert("T3".into(), task("T3", TaskStatus::Complete).1);
    let plan = r"mission_id: demo
tasks:
  - id: T1
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
  - id: T3
    kind: review
    depends_on: [T1]
    spec: specs/T3/SPEC.md
    review_target:
      tasks: [T1]
";
    fx.write_plan(plan);
    fx.write_state(&state);
    let json = fx.status();
    // The derived next_action must be `blocked`, and its reason must
    // name the awaiting_review task.
    assert_eq!(json["data"]["next_action"]["kind"], "blocked");
    let reason = json["data"]["next_action"]["reason"].as_str().unwrap();
    assert!(
        reason.contains("T2") && reason.contains("awaiting review"),
        "reason should mention the awaiting-review task: {reason}"
    );
    assert!(
        !reason.contains("PLAN.yaml may be missing"),
        "reason should not claim PLAN.yaml is missing when it is valid: {reason}"
    );
}

#[test]
fn active_loop_with_stop_allowing_verdict_does_not_contradict_itself() {
    // Loop is active-unpaused but verdict is `needs_user` (plan not
    // locked). `stop_allowed` returns true; the projection must not
    // emit `reason=active_loop` with `message="active loop in progress"`
    // because that contradicts `allow=true`.
    let fx = Fixture::new("demo");
    let mut state = base_state("demo");
    state.outcome.ratified = true;
    state.plan.locked = false;
    state.loop_ = LoopState {
        active: true,
        paused: false,
        mode: LoopMode::Execute,
    };
    fx.write_state(&state);
    fx.write_plan(simple_plan());

    let json = fx.status();
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["stop"]["allow"], true);
    let reason = json["data"]["stop"]["reason"].as_str().unwrap();
    assert_ne!(
        reason, "active_loop",
        "stop.reason must not be active_loop when allow=true"
    );
}
