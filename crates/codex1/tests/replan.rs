//! Integration tests for `codex1 replan check` and `codex1 replan record`.
//!
//! These tests drive the binary through `assert_cmd`, seeding STATE.json
//! directly where the scenarios require pre-populated task records and
//! dirty-review counters.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{json, Value};
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn init_demo(tmp: &TempDir, mission: &str) -> PathBuf {
    let mission_dir = tmp.path().join("PLANS").join(mission);
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", mission])
        .assert()
        .success();
    mission_dir
}

fn parse_stdout_json(output: &std::process::Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(stdout).unwrap_or_else(|e| {
        panic!("expected JSON stdout, got:\n{stdout}\nerror: {e}");
    })
}

fn read_state(mission_dir: &Path) -> Value {
    let raw = fs::read_to_string(mission_dir.join("STATE.json")).expect("state file readable");
    serde_json::from_str(&raw).expect("state file parses")
}

fn write_state(mission_dir: &Path, state: &Value) {
    let raw = serde_json::to_string_pretty(state).expect("state serializes");
    fs::write(mission_dir.join("STATE.json"), raw).expect("state file writable");
}

/// Mutate the mission state in place via a closure so tests can seed
/// tasks, counters, or flags without spinning up multiple CLI runs.
fn patch_state<F: FnOnce(&mut Value)>(mission_dir: &Path, patch: F) {
    let mut state = read_state(mission_dir);
    patch(&mut state);
    write_state(mission_dir, &state);
}

fn task(status: &str) -> Value {
    json!({ "id": "", "status": status })
}

#[test]
fn check_fresh_mission_reports_no_replan() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");

    let output = cmd()
        .current_dir(tmp.path())
        .args(["replan", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["required"], false);
    assert!(json["data"]["reason"].is_null());
    assert_eq!(json["data"]["consecutive_dirty_by_target"], json!({}));
    assert_eq!(json["data"]["triggered_already"], false);
}

#[test]
fn check_reports_required_when_target_reaches_threshold() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        state["replan"]["consecutive_dirty_by_target"] = json!({ "T4": 6 });
    });

    let output = cmd()
        .current_dir(tmp.path())
        .args(["replan", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["required"], true);
    let reason = json["data"]["reason"].as_str().expect("reason string");
    assert!(reason.contains("T4"), "reason should name T4: {reason}");
    assert_eq!(json["data"]["consecutive_dirty_by_target"]["T4"], 6);
}

#[test]
fn check_reports_triggered_already() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        state["replan"]["triggered"] = json!(true);
        state["replan"]["triggered_reason"] = json!("six_dirty");
    });

    let output = cmd()
        .current_dir(tmp.path())
        .args(["replan", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["triggered_already"], true);
}

#[test]
fn record_six_dirty_supersedes_tasks_and_unlocks_plan() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        state["phase"] = json!("execute");
        state["plan"]["locked"] = json!(true);
        state["replan"]["consecutive_dirty_by_target"] = json!({ "T2": 6 });
        let mut t2 = task("in_progress");
        t2["id"] = json!("T2");
        let mut t5 = task("ready");
        t5["id"] = json!("T5");
        state["tasks"] = json!({ "T2": t2, "T5": t5 });
    });

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "six_dirty",
            "--supersedes",
            "T2",
            "--supersedes",
            "T5",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["reason"], "six_dirty");
    assert_eq!(json["data"]["supersedes"], json!(["T2", "T5"]));
    assert_eq!(json["data"]["phase_after"], "plan");
    assert_eq!(json["data"]["plan_locked"], false);

    let state = read_state(&mission_dir);
    assert_eq!(state["phase"], "plan");
    assert_eq!(state["plan"]["locked"], false);
    assert_eq!(state["replan"]["triggered"], true);
    assert_eq!(state["replan"]["triggered_reason"], "six_dirty");
    assert_eq!(state["replan"]["consecutive_dirty_by_target"], json!({}));
    assert_eq!(state["tasks"]["T2"]["status"], "superseded");
    assert_eq!(state["tasks"]["T5"]["status"], "superseded");
    for id in ["T2", "T5"] {
        let marker = state["tasks"][id]["superseded_by"]
            .as_str()
            .unwrap_or_else(|| panic!("{id} missing superseded_by"));
        assert!(
            marker.starts_with("replan-"),
            "{id} superseded_by should be a replan-<rev> marker: {marker}"
        );
    }

    let events = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap();
    let last = events.lines().last().expect("at least one event line");
    let event: Value = serde_json::from_str(last).expect("event is JSON");
    assert_eq!(event["kind"], "replan.recorded");
    assert_eq!(event["payload"]["reason"], "six_dirty");
    assert_eq!(event["payload"]["supersedes"], json!(["T2", "T5"]));
}

#[test]
fn record_resets_passed_mission_close_review_state() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        state["phase"] = json!("mission_close");
        state["plan"]["locked"] = json!(true);
        state["close"]["review_state"] = json!("passed");
        let mut t1 = task("complete");
        t1["id"] = json!("T1");
        state["tasks"] = json!({ "T1": t1 });
    });

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "user_request",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let state = read_state(&mission_dir);
    assert_eq!(state["close"]["review_state"], "not_started");
}

#[test]
fn record_rejects_terminal_mission() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        state["phase"] = json!("terminal");
        state["close"]["terminal_at"] = json!("2026-04-21T00:00:00Z");
        state["plan"]["locked"] = json!(true);
    });
    let before = read_state(&mission_dir);
    let events_before = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "user_request",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "TERMINAL_ALREADY_COMPLETE");
    assert_eq!(read_state(&mission_dir), before);
    assert_eq!(
        fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap(),
        events_before
    );
}

#[test]
fn record_rejects_unknown_reason() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "invalid",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
}

#[test]
fn record_rejects_unknown_supersedes_id() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        // No T99 in tasks.
        let mut t1 = task("ready");
        t1["id"] = json!("T1");
        state["tasks"] = json!({ "T1": t1 });
    });

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "six_dirty",
            "--supersedes",
            "T99",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    let message = json["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("T99"),
        "message should mention T99: {message}"
    );
}

#[test]
fn record_rejects_supersedes_of_complete_task() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        let mut t1 = task("complete");
        t1["id"] = json!("T1");
        state["tasks"] = json!({ "T1": t1 });
    });

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "six_dirty",
            "--supersedes",
            "T1",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    let message = json["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("T1"),
        "message should mention T1: {message}"
    );
    assert!(
        message.to_lowercase().contains("cannot be superseded")
            || message.to_lowercase().contains("superseded"),
        "message should flag non-supersedable: {message}"
    );
}

#[test]
fn record_dry_run_does_not_mutate() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        state["plan"]["locked"] = json!(true);
        state["phase"] = json!("execute");
        let mut t2 = task("in_progress");
        t2["id"] = json!("T2");
        state["tasks"] = json!({ "T2": t2 });
    });
    let before = read_state(&mission_dir);

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "scope_change",
            "--supersedes",
            "T2",
            "--dry-run",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["dry_run"], true);
    assert_eq!(json["data"]["phase_after"], "plan");
    assert_eq!(json["data"]["plan_locked"], false);

    let after = read_state(&mission_dir);
    assert_eq!(before, after, "state must not change on --dry-run");

    // EVENTS.jsonl still empty.
    let events = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap();
    assert!(!events.contains("replan.recorded"), "events: {events}");
}

#[test]
fn record_enforces_expect_revision() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "user_request",
            "--expect-revision",
            "999",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["retryable"], true);
}

#[test]
fn check_after_record_shows_cleared_counters_and_triggered() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        state["replan"]["consecutive_dirty_by_target"] = json!({ "T3": 6 });
        let mut t3 = task("awaiting_review");
        t3["id"] = json!("T3");
        state["tasks"] = json!({ "T3": t3 });
    });

    cmd()
        .current_dir(tmp.path())
        .args([
            "replan",
            "record",
            "--mission",
            "demo",
            "--reason",
            "six_dirty",
            "--supersedes",
            "T3",
        ])
        .assert()
        .success();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["replan", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["required"], false);
    assert_eq!(json["data"]["consecutive_dirty_by_target"], json!({}));
    assert_eq!(json["data"]["triggered_already"], true);
}

/// Regression for e2e-walkthrough round-1 P1-1: after `replan record`
/// unlocks the plan, work-phase mutations must refuse until a new
/// `plan check` re-locks. Prior behavior accepted `task start` /
/// `task finish` in this window, attaching mutations to tasks whose
/// spec could subsequently change.
#[test]
fn task_start_after_replan_record_refuses_with_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    patch_state(&mission_dir, |state| {
        // Seed a mission that has been through replan record: plan
        // unlocked, replan.triggered, phase back to plan, and one
        // task in ready state the replan flow preserves.
        state["phase"] = json!("plan");
        state["outcome"]["ratified"] = json!(true);
        state["plan"]["locked"] = json!(false);
        state["plan"]["task_ids"] = json!(["T1", "T2"]);
        state["replan"]["triggered"] = json!(true);
        state["replan"]["triggered_reason"] = json!("six_dirty");
        let mut t1 = task("superseded");
        t1["id"] = json!("T1");
        t1["superseded_by"] = json!("replan-5");
        let mut t2 = task("ready");
        t2["id"] = json!("T2");
        state["tasks"] = json!({ "T1": t1, "T2": t2 });
    });
    // Seed a PLAN.yaml so the command gets past the plan-parse step.
    let plan = r#"mission_id: demo
planning_level:
  requested: light
  effective: light
outcome_interpretation:
  summary: "x"
architecture:
  summary: "x"
  key_decisions: ["x"]
planning_process:
  evidence:
    - kind: direct_reasoning
      summary: "x"
tasks:
  - id: T1
    title: "one"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: "two"
    kind: code
    depends_on: []
    spec: specs/T2/SPEC.md
risks:
  - risk: x
    mitigation: y
mission_close:
  criteria: [ok]
"#;
    std::fs::write(mission_dir.join("PLAN.yaml"), plan).unwrap();
    for id in ["T1", "T2"] {
        let d = mission_dir.join("specs").join(id);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("SPEC.md"), format!("# {id} spec\n")).unwrap();
    }

    // task start must refuse with PLAN_INVALID, not accept the start.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["task", "start", "T2", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(
        !output.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    let hint = json["hint"].as_str().unwrap_or_default();
    assert!(
        hint.contains("plan check"),
        "hint should steer to plan check: {hint}"
    );

    // After a fresh relock, the guard relaxes. Simulate the post-plan-check
    // state by restoring `plan.locked`, clearing the replan trigger, and
    // recording the current plan hash.
    patch_state(&mission_dir, |s| {
        s["plan"]["locked"] = json!(true);
        s["plan"]["hash"] = json!(codex1::state::plan_hash(plan.as_bytes()));
        s["replan"]["triggered"] = json!(false);
        s["replan"]["triggered_reason"] = Value::Null;
    });
    let output = cmd()
        .current_dir(tmp.path())
        .args(["task", "start", "T2", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(
        output.status.success(),
        "task start should succeed after relock: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
