//! Integration tests for `codex1 close check`, `close complete`, and
//! `close record-review`.
//!
//! States are seeded by writing `STATE.json` directly because other Phase
//! B units (outcome/plan/task/review) are not yet merged. These tests
//! exercise only the `close` surface; downstream-unit tests will cover
//! their own mutations.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{json, Value};
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn init_demo(tmp: &TempDir, mission: &str) -> PathBuf {
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", mission])
        .assert()
        .success();
    tmp.path().join("PLANS").join(mission)
}

fn write_state(mission_dir: &Path, state: &Value) {
    let bytes = serde_json::to_vec_pretty(state).expect("serialize state");
    fs::write(mission_dir.join("STATE.json"), bytes).expect("write STATE.json");
}

fn read_state(mission_dir: &Path) -> Value {
    let raw = fs::read_to_string(mission_dir.join("STATE.json")).expect("read STATE.json");
    serde_json::from_str(&raw).expect("parse STATE.json")
}

fn read_events(mission_dir: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("parse event"))
        .collect()
}

fn parse_stdout(output: &std::process::Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str(stdout).unwrap_or_else(|e| {
        panic!("expected JSON stdout, got:\n{stdout}\nerror: {e}");
    })
}

fn write_findings(mission_dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = mission_dir.join(name);
    fs::write(&path, body).expect("write findings");
    path
}

/// Builder for seeding `STATE.json` into common configurations.
struct StateBuilder {
    value: Value,
    plan_locked: bool,
    explicit_task_ids: Option<Vec<String>>,
}

impl StateBuilder {
    fn fresh(mission_id: &str) -> Self {
        Self {
            value: json!({
                "mission_id": mission_id,
                "revision": 0,
                "schema_version": 1,
                "phase": "clarify",
                "loop": { "active": false, "paused": false, "mode": "none" },
                "outcome": { "ratified": false },
                "plan": { "locked": false },
                "tasks": {},
                "reviews": {},
                "replan": { "consecutive_dirty_by_target": {}, "triggered": false },
                "close": { "review_state": "not_started" },
                "events_cursor": 0
            }),
            plan_locked: false,
            explicit_task_ids: None,
        }
    }

    fn ratified(mut self) -> Self {
        self.value["outcome"] = json!({
            "ratified": true,
            "ratified_at": "2026-04-20T00:00:00Z"
        });
        self
    }

    fn plan_locked(mut self) -> Self {
        self.value["plan"] = json!({
            "locked": true,
            "requested_level": "medium",
            "effective_level": "medium"
        });
        self.plan_locked = true;
        self
    }

    /// Override the computed DAG task-id list. By default, `build()`
    /// derives `plan.task_ids` from the tasks added via `.task(...)`
    /// on a plan-locked state; use this when a test needs to simulate
    /// "plan locked with DAG node Tk but Tk never started".
    #[allow(dead_code)]
    fn dag_task_ids(mut self, ids: &[&str]) -> Self {
        self.explicit_task_ids = Some(ids.iter().map(std::string::ToString::to_string).collect());
        self
    }

    fn phase(mut self, phase: &str) -> Self {
        self.value["phase"] = Value::String(phase.to_string());
        self
    }

    fn task(mut self, id: &str, status: &str) -> Self {
        self.value["tasks"][id] = json!({
            "id": id,
            "status": status,
            "proof_path": if status == "complete" {
                Value::String(format!("specs/{id}/PROOF.md"))
            } else {
                Value::Null
            },
            "superseded_by": Value::Null,
        });
        self
    }

    fn review(mut self, id: &str, verdict: &str) -> Self {
        self.value["reviews"][id] = json!({
            "task_id": id,
            "verdict": verdict,
            "reviewers": ["ci"],
            "category": "accepted_current",
            "recorded_at": "2026-04-20T00:00:00Z",
            "boundary_revision": 1,
        });
        self
    }

    fn mission_close_review(mut self, state: &str) -> Self {
        self.value["close"]["review_state"] = Value::String(state.to_string());
        self
    }

    fn replan_triggered(mut self, reason: &str) -> Self {
        self.value["replan"]["triggered"] = Value::Bool(true);
        self.value["replan"]["triggered_reason"] = Value::String(reason.to_string());
        self
    }

    fn terminal(mut self, at: &str) -> Self {
        self.value["close"]["terminal_at"] = Value::String(at.to_string());
        self.value["phase"] = Value::String("terminal".to_string());
        self
    }

    fn revision(mut self, rev: u64) -> Self {
        self.value["revision"] = Value::Number(rev.into());
        self
    }

    fn build(mut self) -> Value {
        if self.plan_locked {
            let ids: Vec<String> = if let Some(explicit) = &self.explicit_task_ids {
                explicit.clone()
            } else if let Some(tasks_map) = self.value["tasks"].as_object() {
                tasks_map.keys().cloned().collect()
            } else {
                Vec::new()
            };
            if let Some(plan_obj) = self.value["plan"].as_object_mut() {
                plan_obj.insert(
                    "task_ids".to_string(),
                    Value::Array(ids.into_iter().map(Value::String).collect()),
                );
            }
        }
        self.value
    }
}

/// Convenience: fresh mission brought through ratify+plan+task completion.
fn seed_ready_for_mission_close_review(mission_dir: &Path) {
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .phase("mission_close")
        .task("T1", "complete")
        .task("T2", "complete")
        .review("T2", "clean")
        .revision(5)
        .build();
    write_state(mission_dir, &state);
}

// ---------------------------------------------------------------------------
// check
// ---------------------------------------------------------------------------

#[test]
fn check_fresh_mission_reports_outcome_not_ratified() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["ready"], Value::Bool(false));
    assert_eq!(json["data"]["verdict"], "needs_user");
    let blockers = json["data"]["blockers"].as_array().unwrap();
    assert!(
        blockers.iter().any(|b| b["code"] == "OUTCOME_NOT_RATIFIED"),
        "missing OUTCOME_NOT_RATIFIED: {blockers:?}"
    );
    // mission_dir is unused — silence the let-binding.
    let _ = mission_dir;
}

#[test]
fn check_mid_execute_reports_task_not_ready_and_continue_required() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .phase("execute")
        .task("T1", "complete")
        .task("T7", "pending")
        .build();
    write_state(&mission_dir, &state);
    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["ready"], Value::Bool(false));
    assert_eq!(json["data"]["verdict"], "continue_required");
    let blockers = json["data"]["blockers"].as_array().unwrap();
    assert!(
        blockers
            .iter()
            .any(|b| b["code"] == "TASK_NOT_READY" && b["detail"].as_str().unwrap().contains("T7")),
        "blockers missing T7 TASK_NOT_READY: {blockers:?}"
    );
}

#[test]
fn check_all_tasks_complete_but_no_mission_close_review_reports_ready_for_review() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_ready_for_mission_close_review(&mission_dir);

    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["verdict"], "ready_for_mission_close_review");
    assert_eq!(json["data"]["ready"], Value::Bool(false));
    let blockers = json["data"]["blockers"].as_array().unwrap();
    assert!(
        blockers.iter().any(|b| b["code"] == "CLOSE_NOT_READY"),
        "missing CLOSE_NOT_READY blocker: {blockers:?}"
    );
}

#[test]
fn check_review_dirty_reports_review_findings_block() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .phase("review_loop")
        .task("T1", "complete")
        .review("T5", "dirty")
        .build();
    write_state(&mission_dir, &state);
    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["verdict"], "blocked");
    let blockers = json["data"]["blockers"].as_array().unwrap();
    assert!(
        blockers.iter().any(|b| b["code"] == "REVIEW_FINDINGS_BLOCK"
            && b["detail"].as_str().unwrap().contains("T5")),
        "missing REVIEW_FINDINGS_BLOCK T5: {blockers:?}"
    );
}

// ---------------------------------------------------------------------------
// record-review
// ---------------------------------------------------------------------------

#[test]
fn record_review_clean_transitions_state_to_passed() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_ready_for_mission_close_review(&mission_dir);

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "close",
            "record-review",
            "--clean",
            "--mission",
            "demo",
            "--reviewers",
            "alice,bob",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["verdict"], "clean");
    assert_eq!(json["data"]["review_state"], "passed");
    assert_eq!(json["data"]["dry_run"], false);

    let state = read_state(&mission_dir);
    assert_eq!(state["close"]["review_state"], "passed");

    let events = read_events(&mission_dir);
    assert!(events.iter().any(|e| e["kind"] == "close.review.clean"));
}

#[test]
fn check_after_review_clean_reports_mission_close_review_passed_and_ready() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_ready_for_mission_close_review(&mission_dir);

    cmd()
        .current_dir(tmp.path())
        .args(["close", "record-review", "--clean", "--mission", "demo"])
        .assert()
        .success();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["verdict"], "mission_close_review_passed");
    assert_eq!(json["data"]["ready"], Value::Bool(true));
    assert_eq!(json["data"]["blockers"].as_array().map_or(0, Vec::len), 0);
}

#[test]
fn record_review_findings_writes_review_file_and_bumps_counter() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_ready_for_mission_close_review(&mission_dir);
    let findings = write_findings(&mission_dir, "round1.md", "# P0: broke everything\n");

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "close",
            "record-review",
            "--findings-file",
            findings.to_str().unwrap(),
            "--mission",
            "demo",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["verdict"], "dirty");
    assert_eq!(json["data"]["review_state"], "open");
    assert_eq!(json["data"]["consecutive_dirty"], 1);

    let state = read_state(&mission_dir);
    assert_eq!(state["close"]["review_state"], "open");
    assert_eq!(
        state["replan"]["consecutive_dirty_by_target"]["__mission_close__"],
        1
    );
    let events = read_events(&mission_dir);
    assert!(events.iter().any(|e| e["kind"] == "close.review.dirty"));
    // A review artifact was written under reviews/.
    let entries: Vec<_> = fs::read_dir(mission_dir.join("reviews"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().into_string().unwrap())
        .filter(|n| n.starts_with("mission-close-"))
        .collect();
    assert!(
        !entries.is_empty(),
        "expected at least one mission-close review file under reviews/: {entries:?}"
    );
}

#[test]
fn record_review_six_consecutive_dirty_triggers_replan() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_ready_for_mission_close_review(&mission_dir);
    let findings = write_findings(&mission_dir, "r.md", "# P0 fail\n");

    for _ in 0..6 {
        cmd()
            .current_dir(tmp.path())
            .args([
                "close",
                "record-review",
                "--findings-file",
                findings.to_str().unwrap(),
                "--mission",
                "demo",
            ])
            .assert()
            .success();
    }

    let state = read_state(&mission_dir);
    assert_eq!(state["replan"]["triggered"], true);
    assert_eq!(
        state["replan"]["consecutive_dirty_by_target"]["__mission_close__"],
        6
    );

    // The 7th attempt should fail closed because the verdict is now `blocked`.
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "close",
            "record-review",
            "--findings-file",
            findings.to_str().unwrap(),
            "--mission",
            "demo",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["code"], "CLOSE_NOT_READY");
}

#[test]
fn record_review_rejects_when_verdict_is_not_mission_close_ready() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    // Mid-execute: not yet ready for mission-close review.
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .task("T1", "pending")
        .build();
    write_state(&mission_dir, &state);
    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "record-review", "--clean", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["code"], "CLOSE_NOT_READY");
}

#[test]
fn record_review_dry_run_does_not_mutate() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_ready_for_mission_close_review(&mission_dir);
    let before = read_state(&mission_dir);

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "close",
            "record-review",
            "--clean",
            "--mission",
            "demo",
            "--dry-run",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["dry_run"], true);

    let after = read_state(&mission_dir);
    assert_eq!(before, after, "dry-run must not mutate STATE.json");
    assert!(
        read_events(&mission_dir).is_empty(),
        "dry-run must not append events"
    );
}

#[test]
fn record_review_expect_revision_mismatch_returns_conflict() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_ready_for_mission_close_review(&mission_dir); // revision = 5.

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "close",
            "record-review",
            "--clean",
            "--mission",
            "demo",
            "--expect-revision",
            "99",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["context"]["expected"], 99);
    assert_eq!(json["context"]["actual"], 5);
}

// ---------------------------------------------------------------------------
// complete
// ---------------------------------------------------------------------------

#[test]
fn complete_on_ready_writes_closeout_and_marks_terminal() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    // Seed with interpreted_destination in OUTCOME so we can assert it
    // shows up in CLOSEOUT.md.
    fs::write(
        mission_dir.join("OUTCOME.md"),
        "---\nmission_id: demo\nstatus: ratified\ninterpreted_destination: |\n  Build a robust close surface.\n---\n\n# OUTCOME\n",
    )
    .unwrap();
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .phase("mission_close")
        .task("T1", "complete")
        .review("T2", "clean")
        .mission_close_review("passed")
        .revision(7)
        .build();
    write_state(&mission_dir, &state);

    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "complete", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert!(json["data"]["terminal_at"].is_string());
    assert_eq!(json["data"]["mission_id"], "demo");
    assert_eq!(json["data"]["dry_run"], false);

    let closeout = fs::read_to_string(mission_dir.join("CLOSEOUT.md")).unwrap();
    assert!(
        closeout.contains("CLOSEOUT — demo"),
        "closeout missing header: {closeout}"
    );
    assert!(
        closeout.contains("Build a robust close surface."),
        "closeout missing interpreted_destination excerpt: {closeout}"
    );
    assert!(closeout.contains("| T1 |"));
    assert!(closeout.contains("| T2 |"));
    assert!(closeout.contains("| MC |"));

    let state = read_state(&mission_dir);
    assert_eq!(state["phase"], "terminal");
    assert!(state["close"]["terminal_at"].is_string());
    assert_eq!(state["loop"]["active"], false);
    assert_eq!(state["loop"]["paused"], false);
    assert_eq!(state["loop"]["mode"], "none");

    let events = read_events(&mission_dir);
    assert!(events.iter().any(|e| e["kind"] == "close.complete"));
}

#[test]
fn complete_twice_returns_terminal_already_complete() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .task("T1", "complete")
        .mission_close_review("passed")
        .terminal("2026-04-01T00:00:00Z")
        .revision(9)
        .build();
    write_state(&mission_dir, &state);

    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "complete", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["code"], "TERMINAL_ALREADY_COMPLETE");
    assert_eq!(json["context"]["closed_at"], "2026-04-01T00:00:00Z");
}

#[test]
fn complete_when_not_ready_returns_close_not_ready_with_blockers() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .task("T1", "pending")
        .build();
    write_state(&mission_dir, &state);

    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "complete", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["code"], "CLOSE_NOT_READY");
    let message = json["message"].as_str().unwrap();
    assert!(
        message.contains("TASK_NOT_READY") || message.contains("T1"),
        "message should mention blockers: {message}"
    );
}

#[test]
fn complete_dry_run_does_not_mutate() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .task("T1", "complete")
        .mission_close_review("passed")
        .revision(6)
        .build();
    write_state(&mission_dir, &state);
    let before = read_state(&mission_dir);

    let output = cmd()
        .current_dir(tmp.path())
        .args(["close", "complete", "--mission", "demo", "--dry-run"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["data"]["dry_run"], true);
    assert!(json["data"]["terminal_at"].is_string());

    let after = read_state(&mission_dir);
    assert_eq!(before, after, "dry-run must not mutate STATE.json");
    assert!(
        !mission_dir.join("CLOSEOUT.md").exists(),
        "dry-run must not write CLOSEOUT.md"
    );
    assert!(
        read_events(&mission_dir).is_empty(),
        "dry-run must not append events"
    );
}

#[test]
fn complete_expect_revision_mismatch_returns_conflict() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state = StateBuilder::fresh("demo")
        .ratified()
        .plan_locked()
        .task("T1", "complete")
        .mission_close_review("passed")
        .revision(6)
        .build();
    write_state(&mission_dir, &state);

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "close",
            "complete",
            "--mission",
            "demo",
            "--expect-revision",
            "1",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout(&output);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["context"]["expected"], 1);
    assert_eq!(json["context"]["actual"], 6);
}

// ---------------------------------------------------------------------------
// Property: close check and status always agree on close_ready / verdict.
// ---------------------------------------------------------------------------

#[test]
fn status_and_close_check_always_agree() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let cases = agreement_cases();
    assert!(
        cases.len() >= 20,
        "need at least 20 states for agreement property"
    );
    for (idx, case) in cases.iter().enumerate() {
        write_state(&mission_dir, case);
        let status = cmd()
            .current_dir(tmp.path())
            .args(["status", "--mission", "demo"])
            .output()
            .expect("status runs");
        assert!(
            status.status.success(),
            "status failed on case {idx}: {}",
            String::from_utf8_lossy(&status.stderr)
        );
        let s = parse_stdout(&status);

        let check = cmd()
            .current_dir(tmp.path())
            .args(["close", "check", "--mission", "demo"])
            .output()
            .expect("close check runs");
        assert!(
            check.status.success(),
            "close check failed on case {idx}: {}",
            String::from_utf8_lossy(&check.stderr)
        );
        let c = parse_stdout(&check);

        let status_verdict = s["data"]["verdict"].as_str().unwrap_or("").to_string();
        let check_verdict = c["data"]["verdict"].as_str().unwrap_or("").to_string();
        assert_eq!(
            status_verdict, check_verdict,
            "verdict disagreement on case {idx}: status={status_verdict}, check={check_verdict}"
        );

        let status_ready = s["data"]["close_ready"].as_bool().unwrap_or(false);
        let check_ready = c["data"]["ready"].as_bool().unwrap_or(false);
        assert_eq!(
            status_ready, check_ready,
            "close_ready disagreement on case {idx}: status={status_ready}, check={check_ready}"
        );
    }
}

/// Deterministic fixture set of 24 states covering every readiness branch:
/// fresh, ratified only, plan locked, replan triggered, dirty review,
/// incomplete tasks, all tasks complete in each mission-close review
/// substate, terminal, and several combinations.
fn agreement_cases() -> Vec<Value> {
    vec![
        StateBuilder::fresh("demo").build(),
        StateBuilder::fresh("demo").ratified().build(),
        StateBuilder::fresh("demo").ratified().plan_locked().build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "pending")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "in_progress")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "ready")
            .task("T2", "pending")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .task("T2", "awaiting_review")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .task("T2", "superseded")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .review("T1", "dirty")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .review("T1", "clean")
            .replan_triggered("six dirty on T1")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .task("T2", "complete")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .mission_close_review("open")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .mission_close_review("passed")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .mission_close_review("passed")
            .terminal("2026-04-10T00:00:00Z")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .task("T2", "ready")
            .mission_close_review("passed")
            .build(),
        StateBuilder::fresh("demo")
            .plan_locked() // outcome not ratified
            .task("T1", "complete")
            .mission_close_review("passed")
            .build(),
        StateBuilder::fresh("demo")
            .ratified() // plan not locked
            .task("T1", "complete")
            .mission_close_review("passed")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .review("T1", "clean")
            .review("T2", "dirty")
            .mission_close_review("open")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .review("T1", "pending")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .task("T2", "complete")
            .task("T3", "complete")
            .task("T4", "complete")
            .mission_close_review("open")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .replan_triggered("manual")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "superseded")
            .task("T2", "complete")
            .mission_close_review("passed")
            .build(),
        StateBuilder::fresh("demo")
            .ratified()
            .plan_locked()
            .task("T1", "complete")
            .mission_close_review("not_started") // same as default, but explicit
            .build(),
    ]
}
