//! Integration tests for Unit 3 (`codex1 plan choose-level` / `plan scaffold`).
//!
//! Exercises the parsing rules (product verbs + 1/2/3 aliases, rejection of
//! `low`/`high`/out-of-range), the STATE.json mutation (phase transition,
//! requested/effective level), `--escalate`, `--dry-run`, the scaffolded
//! PLAN.yaml shape, and the level-mismatch guard.

use std::fs;
use std::path::{Path, PathBuf};

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

fn read_state(mission_dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap()
}

fn read_events(mission_dir: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap();
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

#[test]
fn choose_level_accepts_numeric_alias_1() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "choose-level", "--mission", "demo", "--level", "1"])
        .output()
        .expect("runs");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["requested_level"], "light");
    assert_eq!(json["data"]["effective_level"], "light");
    assert_eq!(json["data"]["next_action"]["kind"], "plan_scaffold");
    assert_eq!(json["data"]["next_action"]["args"][4], "light");
    assert!(json["data"].get("escalation_reason").is_none());

    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["requested_level"], "light");
    assert_eq!(state["plan"]["effective_level"], "light");
    assert_eq!(state["phase"], "plan");
    assert_eq!(state["revision"], 1);

    let events = read_events(&mission_dir);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "plan.choose_level");
    assert_eq!(events[0]["payload"]["requested_level"], "light");
}

#[test]
fn choose_level_accepts_string_light() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "plan",
            "choose-level",
            "--mission",
            "demo",
            "--level",
            "light",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["requested_level"], "light");
    assert_eq!(json["data"]["effective_level"], "light");

    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["requested_level"], "light");
}

#[test]
fn choose_level_with_escalate_bumps_to_hard() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "plan",
            "choose-level",
            "--mission",
            "demo",
            "--level",
            "medium",
            "--escalate",
            "touches hooks",
        ])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["requested_level"], "medium");
    assert_eq!(json["data"]["effective_level"], "hard");
    assert_eq!(json["data"]["escalation_reason"], "touches hooks");
    assert_eq!(json["data"]["next_action"]["args"][4], "hard");

    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["requested_level"], "medium");
    assert_eq!(state["plan"]["effective_level"], "hard");

    let events = read_events(&mission_dir);
    assert_eq!(
        events[0]["payload"]["escalation_reason"], "touches hooks",
        "audit event should capture escalation reason"
    );
}

#[test]
fn choose_level_rejects_high() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "plan",
            "choose-level",
            "--mission",
            "demo",
            "--level",
            "high",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PARSE_ERROR");
}

#[test]
fn choose_level_rejects_low() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "plan",
            "choose-level",
            "--mission",
            "demo",
            "--level",
            "low",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PARSE_ERROR");
}

#[test]
fn choose_level_rejects_out_of_range_numeric() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "choose-level", "--mission", "demo", "--level", "4"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PARSE_ERROR");
}

#[test]
fn scaffold_hard_writes_hard_evidence_and_rejects_level_mismatch() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    cmd()
        .current_dir(tmp.path())
        .args([
            "plan",
            "choose-level",
            "--mission",
            "demo",
            "--level",
            "hard",
        ])
        .assert()
        .success();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "scaffold", "--mission", "demo", "--level", "hard"])
        .output()
        .expect("runs");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["level"], "hard");
    assert!(json["data"]["wrote"]
        .as_str()
        .unwrap()
        .ends_with("PLAN.yaml"));

    let plan = fs::read_to_string(mission_dir.join("PLAN.yaml")).unwrap();
    for marker in [
        "mission_id: demo",
        "requested: hard",
        "effective: hard",
        "kind: explorer",
        "kind: advisor",
        "kind: plan_review",
        "[codex1-fill:explorer_evidence]",
        "[codex1-fill:advisor_evidence]",
        "[codex1-fill:plan_reviewer_evidence]",
        "outcome_interpretation:",
        "architecture:",
        "planning_process:",
        "tasks:",
        "risks:",
        "mission_close:",
    ] {
        assert!(
            plan.contains(marker),
            "scaffolded PLAN.yaml missing `{marker}`:\n{plan}"
        );
    }

    // Parse to YAML to confirm shape is valid.
    let yaml: serde_yaml::Value = serde_yaml::from_str(&plan).expect("parses as yaml");
    assert_eq!(yaml["mission_id"], serde_yaml::Value::String("demo".into()));
    assert_eq!(
        yaml["planning_level"]["effective"],
        serde_yaml::Value::String("hard".into())
    );
    assert!(yaml["planning_process"]["evidence"].is_sequence());
    assert_eq!(
        yaml["planning_process"]["evidence"]
            .as_sequence()
            .unwrap()
            .len(),
        3
    );

    // Second scaffold at the wrong level must reject.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "scaffold", "--mission", "demo", "--level", "medium"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
}

#[test]
fn choose_level_dry_run_does_not_mutate() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let before = read_state(&mission_dir);
    assert_eq!(before["revision"], 0);

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "plan",
            "choose-level",
            "--mission",
            "demo",
            "--level",
            "medium",
            "--dry-run",
        ])
        .output()
        .expect("runs");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["dry_run"], true);
    assert_eq!(json["data"]["requested_level"], "medium");

    let after = read_state(&mission_dir);
    assert_eq!(after["revision"], 0);
    assert!(after["plan"]["requested_level"].is_null());
    assert_eq!(after["phase"], "clarify");
    let events = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap();
    assert_eq!(events, "");
}

#[test]
fn choose_level_non_interactive_requires_flag() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    // `assert_cmd` pipes stdin, which is not a TTY — so this run hits the
    // non-interactive branch without a `--level`.
    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "choose-level", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PARSE_ERROR");
    assert!(
        json["message"]
            .as_str()
            .unwrap_or("")
            .contains("required in non-interactive mode"),
        "unexpected message: {}",
        json["message"]
    );
}

#[test]
fn scaffold_before_choose_level_rejects() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "scaffold", "--mission", "demo", "--level", "hard"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
}
