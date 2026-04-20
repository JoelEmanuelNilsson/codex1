//! Foundation integration tests.
//!
//! Exercises `codex1 init`, `codex1 doctor`, `codex1 status`, the stub
//! command dispatches, STATE.json/EVENTS.jsonl atomicity, and envelope
//! shape. Phase B units add their own integration tests under
//! `tests/<unit>.rs`.

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;
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

#[test]
fn help_prints_full_command_tree() {
    let output = cmd().arg("--help").output().expect("runs");
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "init", "doctor", "hook", "outcome", "plan", "task", "review", "replan", "loop", "close",
        "status",
    ] {
        assert!(
            text.contains(expected),
            "help missing `{expected}`:\n{text}"
        );
    }
}

#[test]
fn doctor_runs_without_auth() {
    let output = cmd().arg("doctor").output().expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert!(json["data"]["version"].is_string());
    assert_eq!(json["data"]["auth"]["required"], Value::Bool(false));
}

#[test]
fn init_creates_mission_scaffold() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    assert!(mission_dir.join("STATE.json").is_file());
    assert!(mission_dir.join("OUTCOME.md").is_file());
    assert!(mission_dir.join("PLAN.yaml").is_file());
    assert!(mission_dir.join("EVENTS.jsonl").is_file());
    assert!(mission_dir.join("specs").is_dir());
    assert!(mission_dir.join("reviews").is_dir());

    let state: Value =
        serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap();
    assert_eq!(state["mission_id"], "demo");
    assert_eq!(state["revision"], 0);
    assert_eq!(state["schema_version"], 1);
    assert_eq!(state["phase"], "clarify");
    assert_eq!(state["outcome"]["ratified"], false);
    assert_eq!(state["plan"]["locked"], false);
    assert_eq!(state["loop"]["active"], false);
}

#[test]
fn init_refuses_to_overwrite() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(false));
    assert_eq!(json["code"], "STATE_CORRUPT");
}

#[test]
fn status_resolves_existing_mission_and_reports_stop_allowed() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["stop"]["allow"], true);
}

#[test]
fn status_without_mission_reports_needs_user() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["stop"]["allow"], true);
}

// `outcome_stubs_return_not_implemented` removed: Phase B Unit 2
// (cli-outcome) has replaced the stub with the real implementation.
// See `tests/outcome.rs` for the Phase B integration coverage.

#[test]
fn plan_stubs_return_not_implemented() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "waves", "--mission", "demo"])
        .output()
        .expect("runs");
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "NOT_IMPLEMENTED");
    assert_eq!(json["context"]["command"], "plan waves");
}

#[test]
fn review_stubs_return_not_implemented() {
    // The task stubs were replaced by Unit 6 (cli-task). Stand-in: any
    // still-stubbed subcommand. Review stays stubbed until Unit 7 lands.
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["review", "status", "T1", "--mission", "demo"])
        .output()
        .expect("runs");
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "NOT_IMPLEMENTED");
}

#[test]
fn mission_not_found_has_helpful_hint() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "does-not-exist"])
        .output()
        .expect("runs");
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "MISSION_NOT_FOUND");
    assert!(json["hint"].is_string());
}

#[test]
fn hook_snippet_is_informational_only() {
    let output = cmd().args(["hook", "snippet"]).output().expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["hook"]["event"], "Stop");
    assert!(json["data"]["install"].is_object());
}

#[test]
fn init_dry_run_does_not_create_files() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "demo", "--dry-run"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["dry_run"], true);
    assert!(!tmp.path().join("PLANS").exists());
}

#[test]
fn events_jsonl_is_append_only_friendly() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let events = mission_dir.join("EVENTS.jsonl");
    // Empty after init (no mutations yet — revision 0 is the initial state).
    let content = fs::read_to_string(&events).unwrap();
    assert_eq!(content, "");
}
