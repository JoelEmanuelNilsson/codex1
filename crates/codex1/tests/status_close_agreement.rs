//! `status` vs readiness-helper agreement (Phase B Unit 11).
//!
//! The real invariant this test guards: `codex1 status` must publish
//! the same verdict and `close_ready` that `state::readiness` derives.
//! When `codex1 close check` lands (Unit 10), the CLI agreement is
//! immediate because both commands call the same helpers — this test
//! pins that contract at the library level today, which is the only
//! path that will not regress when `close check` ships.
//!
//! The `close_check_cli_agrees_when_implemented` test demonstrates
//! that once `close check` returns success it already agrees (the
//! current stub returns NOT_IMPLEMENTED, which we explicitly tolerate).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use codex1::state::readiness;
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
        write_atomic(&path, serde_json::to_vec_pretty(state).unwrap().as_slice());
    }

    fn write_plan(&self, body: &str) {
        write_atomic(&self.mission_dir().join("PLAN.yaml"), body.as_bytes());
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

    fn close_check(&self) -> (bool, Value) {
        let out = cmd()
            .current_dir(self.tmp.path())
            .args(["close", "check", "--mission", &self.mission])
            .output()
            .expect("runs");
        let stdout = std::str::from_utf8(&out.stdout).expect("utf-8");
        let json: Value = serde_json::from_str(stdout)
            .unwrap_or_else(|e| panic!("non-JSON stdout from close check:\n{stdout}\n{e}"));
        (out.status.success(), json)
    }
}

fn write_atomic(path: &Path, data: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, data).unwrap();
}

fn base(mission_id: &str) -> MissionState {
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

fn minimal_plan() -> &'static str {
    r"mission_id: demo
tasks:
  - id: T1
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    kind: review
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    review_target:
      tasks: [T1]
"
}

fn task_rec(id: &str, status: TaskStatus) -> TaskRecord {
    TaskRecord {
        id: id.to_string(),
        status,
        started_at: None,
        finished_at: None,
        proof_path: None,
        superseded_by: None,
    }
}

fn review_rec(task_id: &str, verdict: ReviewVerdict, boundary: u64) -> ReviewRecord {
    ReviewRecord {
        task_id: task_id.to_string(),
        verdict,
        reviewers: vec!["code-reviewer".to_string()],
        findings_file: None,
        category: ReviewRecordCategory::AcceptedCurrent,
        recorded_at: "2026-04-20T00:00:00Z".to_string(),
        boundary_revision: boundary,
    }
}

/// Build 20 representative MissionState fixtures that together cover
/// every verdict branch in `readiness::derive_verdict`.
fn fixtures() -> Vec<(String, MissionState)> {
    let mut out = Vec::new();

    // 1. Fresh, nothing ratified → needs_user.
    out.push(("fresh".into(), base("demo")));

    // 2. Outcome ratified, plan unlocked → needs_user (plan branch).
    let mut s = base("demo");
    s.outcome.ratified = true;
    out.push(("outcome_only".into(), s));

    // 3. Plan locked, no tasks → continue_required (tasks empty isn't "complete").
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    out.push(("plan_locked_no_tasks".into(), s));

    // 4. Plan locked, T1 pending → continue_required.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Pending));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Pending));
    out.push(("tasks_pending".into(), s));

    // 5. Plan locked, T1 in progress → continue_required.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::InProgress));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Pending));
    out.push(("task_in_progress".into(), s));

    // 6. Plan locked, T1 awaiting review → continue_required.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::AwaitingReview));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Ready));
    out.push(("task_awaiting_review".into(), s));

    // 7. All tasks complete, close NotStarted → ready_for_mission_close_review.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Complete));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Complete));
    out.push(("ready_for_close_review".into(), s));

    // 8. All tasks complete, close Open → mission_close_review_open.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Complete));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Complete));
    s.close.review_state = MissionCloseReviewState::Open;
    out.push(("close_review_open".into(), s));

    // 9. All tasks complete, close Passed → mission_close_review_passed (close_ready=true).
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Complete));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Complete));
    s.close.review_state = MissionCloseReviewState::Passed;
    out.push(("close_review_passed".into(), s));

    // 10. Terminal → terminal_complete.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Complete));
    s.close.terminal_at = Some("2026-04-20T00:00:00Z".into());
    out.push(("terminal".into(), s));

    // 11. Replan triggered → blocked.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.replan.triggered = true;
    s.replan.triggered_reason = Some("six dirty reviews".into());
    out.push(("replan".into(), s));

    // 12. Dirty review → blocked.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::AwaitingReview));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Pending));
    s.reviews
        .insert("T2".into(), review_rec("T2", ReviewVerdict::Dirty, 4));
    out.push(("dirty_review".into(), s));

    // 13. Clean review, tasks still pending → continue_required.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Complete));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Pending));
    s.reviews
        .insert("T2".into(), review_rec("T2", ReviewVerdict::Clean, 4));
    out.push(("clean_review_continuing".into(), s));

    // 14. Loop active, task in progress → continue_required; stop disallowed.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.loop_ = LoopState {
        active: true,
        paused: false,
        mode: LoopMode::Execute,
    };
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::InProgress));
    out.push(("active_loop".into(), s));

    // 15. Loop paused → continue_required; stop allowed via loop_paused.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.loop_ = LoopState {
        active: true,
        paused: true,
        mode: LoopMode::Execute,
    };
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Ready));
    out.push(("loop_paused".into(), s));

    // 16. Superseded tasks + complete tasks only → ready_for_mission_close_review.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Superseded));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Complete));
    out.push(("superseded_mix".into(), s));

    // 17. Mixed task states with a ready + pending + complete combination → continue_required.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::Complete));
    s.tasks
        .insert("T2".into(), task_rec("T2", TaskStatus::Ready));
    s.tasks
        .insert("T3".into(), task_rec("T3", TaskStatus::Pending));
    out.push(("mixed_ready_pending".into(), s));

    // 18. Terminal but loop still active (state oddity) → terminal_complete.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.loop_ = LoopState {
        active: true,
        paused: false,
        mode: LoopMode::Execute,
    };
    s.close.terminal_at = Some("2026-04-20T10:00:00Z".into());
    out.push(("terminal_with_active_loop".into(), s));

    // 19. Outcome ratified but with revision bumped by prior activity.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.revision = 7;
    s.events_cursor = 7;
    out.push(("revision_bumped".into(), s));

    // 20. Plan locked + replan NOT triggered + dirty review alongside pending task.
    let mut s = base("demo");
    s.outcome.ratified = true;
    s.plan.locked = true;
    s.tasks
        .insert("T1".into(), task_rec("T1", TaskStatus::AwaitingReview));
    s.reviews
        .insert("T2".into(), review_rec("T2", ReviewVerdict::Dirty, 3));
    s.replan.consecutive_dirty_by_target.insert("T1".into(), 2);
    out.push(("dirty_without_replan".into(), s));

    out
}

#[test]
fn status_agrees_with_readiness_helpers_for_all_fixtures() {
    for (name, state) in fixtures() {
        let fx = Fixture::new("demo");
        fx.write_state(&state);
        fx.write_plan(minimal_plan());

        let status_json = fx.status();
        let expected_verdict = readiness::derive_verdict(&state);
        let expected_close_ready = readiness::close_ready(&state);

        assert_eq!(
            status_json["data"]["verdict"].as_str().unwrap(),
            expected_verdict.as_str(),
            "verdict mismatch for fixture {name}: {:#?}",
            status_json["data"]
        );
        assert_eq!(
            status_json["data"]["close_ready"].as_bool().unwrap(),
            expected_close_ready,
            "close_ready mismatch for fixture {name}"
        );
        assert_eq!(
            status_json["revision"].as_u64().unwrap(),
            state.revision,
            "envelope revision should match state.revision for fixture {name}"
        );
    }
}

#[test]
fn close_check_cli_agrees_when_implemented() {
    // Once Unit 10 lands `close check`, this test asserts end-to-end
    // CLI agreement. While the stub returns NOT_IMPLEMENTED we verify
    // the contract holds on every fixture that does succeed.
    for (name, state) in fixtures() {
        let fx = Fixture::new("demo");
        fx.write_state(&state);
        fx.write_plan(minimal_plan());

        let status_json = fx.status();
        let (success, close_json) = fx.close_check();
        if !success {
            // Skip until Unit 10 lands. Record that the stub is the
            // only reason we skipped so the test wakes up the moment
            // `close check` starts returning success.
            assert_eq!(
                close_json["code"], "NOT_IMPLEMENTED",
                "close check failed for fixture {name} with non-stub error: {close_json:#?}"
            );
            continue;
        }
        assert_eq!(
            status_json["data"]["close_ready"], close_json["data"]["ready"],
            "close_ready disagreement for fixture {name}"
        );
        assert_eq!(
            status_json["data"]["verdict"], close_json["data"]["verdict"],
            "verdict disagreement for fixture {name}"
        );
    }
}

/// After `codex1 init`, `status --json` reports the freshly-written
/// revision. A stronger end-to-end "mutation bumps revision" test
/// belongs to whichever Phase B unit first exposes a mutating CLI
/// command — init writes via `init_write`, not `state::mutate`, so
/// the revision stays zero. The in-process fixture above already
/// covers state.revision → envelope.revision for arbitrary values.
#[test]
fn status_after_init_reports_revision_zero() {
    let tmp = TempDir::new().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "demo"])
        .assert()
        .success();
    let state_path = tmp.path().join("PLANS").join("demo").join("STATE.json");
    let state: MissionState =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(state.revision, 0);
    let out = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .expect("runs");
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["revision"].as_u64().unwrap(), 0);
}
