//! Integration tests for `codex1 loop {activate,pause,resume,deactivate}`.
//!
//! These exercise the state transitions, idempotency, `--dry-run`, and
//! `--expect-revision`, plus the Ralph stop-allow projection that hangs
//! off the paused-loop check in `status`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

use codex1::state::schema::{LoopMode, LoopState, MissionCloseReviewState, PlanLevel, TaskStatus};
use codex1::state::{self, MissionState, TaskRecord};

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

fn parse_stdout_json(output: &Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(stdout)
        .unwrap_or_else(|e| panic!("expected JSON stdout:\n{stdout}\nerror: {e}"))
}

fn read_state(mission_dir: &Path) -> Value {
    let raw = fs::read_to_string(mission_dir.join("STATE.json")).expect("read STATE.json");
    serde_json::from_str(&raw).expect("parse STATE.json")
}

fn read_event_lines(mission_dir: &Path) -> Vec<String> {
    let raw = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap_or_default();
    raw.lines().map(ToString::to_string).collect()
}

fn event_kinds(mission_dir: &Path) -> Vec<String> {
    read_event_lines(mission_dir)
        .into_iter()
        .map(|line| {
            let v: Value = serde_json::from_str(&line).expect("event jsonl");
            v["kind"].as_str().unwrap_or_default().to_string()
        })
        .collect()
}

#[test]
fn fresh_mission_loop_inactive() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["active"], false);
    assert_eq!(state["loop"]["paused"], false);
    assert_eq!(state["loop"]["mode"], "none");
}

#[test]
fn activate_sets_state_and_appends_event() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo", "--mode", "execute"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["active"], true);
    assert_eq!(json["data"]["paused"], false);
    assert_eq!(json["data"]["mode"], "execute");
    assert_eq!(json["data"]["noop"], false);
    assert_eq!(json["revision"], 1);

    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["active"], true);
    assert_eq!(state["loop"]["mode"], "execute");
    assert_eq!(event_kinds(&mission_dir), vec!["loop.activated"]);
}

#[test]
fn activate_default_mode_is_execute() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["mode"], "execute");
}

#[test]
fn pause_sets_paused_and_appends_event() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");

    // Activate first.
    cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo"])
        .assert()
        .success();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "pause", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["active"], true);
    assert_eq!(json["data"]["paused"], true);
    assert_eq!(json["data"]["noop"], false);

    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["paused"], true);
    assert_eq!(
        event_kinds(&mission_dir),
        vec!["loop.activated", "loop.paused"]
    );
}

#[test]
fn resume_clears_paused_and_appends_event() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo"])
        .assert()
        .success();
    cmd()
        .current_dir(tmp.path())
        .args(["loop", "pause", "--mission", "demo"])
        .assert()
        .success();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "resume", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["active"], true);
    assert_eq!(json["data"]["paused"], false);
    assert_eq!(json["data"]["noop"], false);

    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["paused"], false);
    assert_eq!(
        event_kinds(&mission_dir),
        vec!["loop.activated", "loop.paused", "loop.resumed"]
    );
}

#[test]
fn pause_twice_is_idempotent_with_no_extra_event() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo"])
        .assert()
        .success();
    cmd()
        .current_dir(tmp.path())
        .args(["loop", "pause", "--mission", "demo"])
        .assert()
        .success();
    let revision_after_first_pause = read_state(&mission_dir)["revision"].as_u64().unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "pause", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["noop"], true);
    assert_eq!(json["revision"], revision_after_first_pause);

    let state = read_state(&mission_dir);
    assert_eq!(
        state["revision"].as_u64().unwrap(),
        revision_after_first_pause
    );
    // No second `loop.paused` event.
    assert_eq!(
        event_kinds(&mission_dir),
        vec!["loop.activated", "loop.paused"]
    );
}

#[test]
fn deactivate_resets_to_default() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo", "--mode", "plan"])
        .assert()
        .success();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "deactivate", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["active"], false);
    assert_eq!(json["data"]["paused"], false);
    assert_eq!(json["data"]["mode"], "none");

    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["active"], false);
    assert_eq!(state["loop"]["mode"], "none");
    assert_eq!(
        event_kinds(&mission_dir),
        vec!["loop.activated", "loop.deactivated"]
    );
}

#[test]
fn deactivate_twice_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    // No activate — state already matches default deactivated.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "deactivate", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["noop"], true);
    assert_eq!(json["revision"], 0);
    // Nothing appended.
    assert!(read_event_lines(&mission_dir).is_empty());

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "deactivate", "--mission", "demo"])
        .output()
        .unwrap();
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["noop"], true);
    assert!(read_event_lines(&mission_dir).is_empty());
}

#[test]
fn pause_inactive_loop_is_task_not_ready() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "pause", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], false);
    assert_eq!(json["code"], "TASK_NOT_READY");
    assert!(json["message"].as_str().unwrap().contains("No active loop"));
    // State untouched.
    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["active"], false);
    assert_eq!(state["revision"], 0);
}

#[test]
fn resume_unpaused_active_loop_is_noop() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo"])
        .assert()
        .success();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "resume", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["noop"], true);
    // No resumed event appended.
    assert_eq!(event_kinds(&mission_dir), vec!["loop.activated"]);
}

#[test]
fn resume_inactive_loop_is_task_not_ready() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "resume", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "TASK_NOT_READY");
    assert!(json["message"].as_str().unwrap().contains("not active"));
}

#[test]
fn activate_bogus_mode_is_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "loop",
            "activate",
            "--mission",
            "demo",
            "--mode",
            "whatever",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["active"], false);
    assert_eq!(state["revision"], 0);
}

#[test]
fn activate_mode_none_is_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mission", "demo", "--mode", "none"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(json["hint"].as_str().unwrap().contains("loop deactivate"));
}

#[test]
fn dry_run_preserves_state_on_activate() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "loop",
            "activate",
            "--mission",
            "demo",
            "--mode",
            "execute",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["dry_run"], true);
    assert_eq!(json["data"]["active"], true);
    assert_eq!(json["data"]["before"]["active"], false);
    // Nothing was written.
    let state = read_state(&mission_dir);
    assert_eq!(state["loop"]["active"], false);
    assert_eq!(state["revision"], 0);
    assert!(read_event_lines(&mission_dir).is_empty());
}

#[test]
fn dry_run_preserves_state_on_pause_and_still_errors_when_inactive() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    // pause --dry-run against inactive: honest error, no mutation.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "pause", "--mission", "demo", "--dry-run"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "TASK_NOT_READY");
    let state = read_state(&mission_dir);
    assert_eq!(state["revision"], 0);
}

#[test]
fn expect_revision_enforced_on_mutating_transitions() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    // Fresh state has revision 0. Passing --expect-revision 5 should
    // fail with REVISION_CONFLICT.
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "loop",
            "activate",
            "--mission",
            "demo",
            "--expect-revision",
            "5",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["retryable"], true);
    assert_eq!(json["context"]["expected"], 5);
    assert_eq!(json["context"]["actual"], 0);
}

#[test]
fn expect_revision_enforced_on_noop_transitions() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    // `deactivate` on a fresh mission is a noop at revision 0.
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "loop",
            "deactivate",
            "--mission",
            "demo",
            "--expect-revision",
            "9",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "REVISION_CONFLICT");
}

#[test]
fn expect_revision_matches_allow_mutation() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "loop",
            "activate",
            "--mission",
            "demo",
            "--expect-revision",
            "0",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["revision"], 1);
}

#[test]
fn paused_loop_allows_stop_in_status() {
    // Build a mission by hand that sits in `continue_required`:
    // ratified outcome, locked plan, one pending task. With the loop
    // active and unpaused, status.stop.allow == false. Pause and it
    // flips to true.
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");

    // Hand-craft state past clarify/plan so the verdict is
    // `continue_required`. Foundation's helpers make this safe.
    let mut state = MissionState::fresh("demo");
    state.revision = 0;
    state.phase = codex1::state::schema::Phase::Execute;
    state.outcome.ratified = true;
    state.outcome.ratified_at = Some("2026-04-20T12:00:00Z".to_string());
    state.plan.locked = true;
    state.plan.requested_level = Some(PlanLevel::Medium);
    state.plan.effective_level = Some(PlanLevel::Medium);
    state.plan.hash = Some("deadbeef".to_string());
    state.tasks.insert(
        "T1".to_string(),
        TaskRecord {
            id: "T1".to_string(),
            status: TaskStatus::Pending,
            started_at: None,
            finished_at: None,
            proof_path: None,
            superseded_by: None,
        },
    );
    state.close.review_state = MissionCloseReviewState::NotStarted;
    state.loop_ = LoopState {
        active: true,
        paused: false,
        mode: LoopMode::Execute,
    };
    let raw = serde_json::to_vec_pretty(&state).unwrap();
    state::fs_atomic::atomic_write(&mission_dir.join("STATE.json"), &raw).unwrap();

    // Sanity: with active loop + continue_required, stop.allow is false.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["verdict"], "continue_required");
    assert_eq!(json["data"]["stop"]["allow"], false);

    // Pause the loop.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "pause", "--mission", "demo"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // After pause, stop.allow flips to true because loop.paused == true.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .unwrap();
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["verdict"], "continue_required");
    assert_eq!(json["data"]["loop"]["paused"], true);
    assert_eq!(json["data"]["stop"]["allow"], true);
}

#[test]
fn pause_resume_cycle_appends_events_in_order() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    for args in [
        &["loop", "activate", "--mission", "demo", "--mode", "execute"][..],
        &["loop", "pause", "--mission", "demo"][..],
        &["loop", "resume", "--mission", "demo"][..],
        &["loop", "pause", "--mission", "demo"][..],
        &["loop", "deactivate", "--mission", "demo"][..],
    ] {
        cmd().current_dir(tmp.path()).args(args).assert().success();
    }
    assert_eq!(
        event_kinds(&mission_dir),
        vec![
            "loop.activated",
            "loop.paused",
            "loop.resumed",
            "loop.paused",
            "loop.deactivated"
        ]
    );
}
