//! Integration tests for `codex1 plan check`.
//!
//! Each test creates a temporary mission with `codex1 init --mission`,
//! then writes a hand-crafted PLAN.yaml plus spec files and exercises
//! `plan check` against it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

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

fn write_spec(mission_dir: &Path, task_id: &str, body: &str) {
    let spec_dir = mission_dir.join("specs").join(task_id);
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join("SPEC.md"), body).unwrap();
}

fn write_plan(mission_dir: &Path, yaml: &str) {
    fs::write(mission_dir.join("PLAN.yaml"), yaml).unwrap();
}

fn parse_stdout_json(output: &Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(stdout).unwrap_or_else(|e| {
        panic!("expected JSON stdout, got:\n{stdout}\nerror: {e}");
    })
}

fn run_check(tmp: &TempDir, mission: &str, extra_args: &[&str]) -> Output {
    let mut c = cmd();
    c.current_dir(tmp.path())
        .args(["plan", "check", "--mission", mission]);
    for a in extra_args {
        c.arg(a);
    }
    c.output().expect("runs")
}

fn valid_4_task_yaml() -> String {
    r#"mission_id: demo

planning_level:
  requested: medium
  effective: medium

outcome_interpretation:
  summary: "Ship the demo mission."

architecture:
  summary: "Single crate, one executable."
  key_decisions:
    - "Use a single Cargo workspace."

planning_process:
  evidence:
    - kind: direct_reasoning
      summary: "Author-reviewed top-level design."

tasks:
  - id: T1
    title: "Design API"
    kind: design
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: "Implement API"
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
  - id: T3
    title: "Write docs"
    kind: docs
    depends_on: [T2]
    spec: specs/T3/SPEC.md
  - id: T4
    title: "Review integration"
    kind: review
    depends_on: [T2, T3]
    spec: specs/T4/SPEC.md
    review_target:
      tasks: [T2]
    review_profiles:
      - code_bug_correctness

risks:
  - risk: "Scope creep."
    mitigation: "Freeze plan at lock."

mission_close:
  criteria:
    - "All tasks complete and reviews clean."
"#
    .to_string()
}

fn seed_valid_mission(tmp: &TempDir, mission: &str) -> PathBuf {
    let mission_dir = init_demo(tmp, mission);
    for task in ["T1", "T2", "T3", "T4"] {
        write_spec(&mission_dir, task, &format!("# {task} SPEC\n"));
    }
    write_plan(&mission_dir, &valid_4_task_yaml());
    mission_dir
}

fn read_state(mission_dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap()
}

fn read_events(mission_dir: &Path) -> Vec<Value> {
    let content = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap();
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

// --- Tests ---

#[test]
fn valid_plan_locks_state_and_advances_phase() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = seed_valid_mission(&tmp, "demo");

    // Put the mission into the `plan` phase manually (init leaves it in
    // `clarify`; we don't want `plan check` to unexpectedly skip the
    // phase transition). We simulate by editing STATE.json directly —
    // in practice `outcome ratify` + `plan scaffold` would do this.
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["phase"] = Value::String("plan".to_string());
    fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap()).unwrap();

    let output = run_check(&tmp, "demo", &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["tasks"], 4);
    assert_eq!(json["data"]["review_tasks"], 1);
    assert_eq!(json["data"]["locked"], true);
    assert!(json["data"]["plan_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));

    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["locked"], true);
    assert_eq!(state["phase"], "execute");
    assert!(state["plan"]["hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));

    let events = read_events(&mission_dir);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "plan.checked");
}

#[test]
fn missing_depends_on_returns_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T1", "# T1\n");
    let yaml = r#"mission_id: demo

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
    title: "t1"
    kind: code
    spec: specs/T1/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(
        json["message"].as_str().unwrap().contains("depends_on"),
        "message missing depends_on: {}",
        json["message"]
    );
}

#[test]
fn duplicate_task_ids_returns_plan_invalid_with_context() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T1", "# T1\n");
    let yaml = r#"mission_id: demo

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
    title: "t1"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T1
    title: "t1-dup"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    let dups = json["context"]["duplicate_ids"]
        .as_array()
        .expect("duplicate_ids array");
    assert!(dups.iter().any(|v| v == "T1"));
}

#[test]
fn task_id_pattern_violation_returns_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T01", "# T01\n");
    let yaml = r#"mission_id: demo

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
  - id: T01
    title: "t01"
    kind: code
    depends_on: []
    spec: specs/T01/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(json["message"].as_str().unwrap().contains("T01"));
}

#[test]
fn cycle_returns_dag_cycle_with_edges() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T1", "# T1\n");
    write_spec(&mission_dir, "T2", "# T2\n");
    let yaml = r#"mission_id: demo

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
    title: "t1"
    kind: code
    depends_on: [T2]
    spec: specs/T1/SPEC.md
  - id: T2
    title: "t2"
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "DAG_CYCLE");
    let edges = json["context"]["cycle_edges"]
        .as_array()
        .expect("cycle_edges array");
    assert!(!edges.is_empty(), "cycle_edges should not be empty");
}

#[test]
fn missing_dep_returns_dag_missing_dep_with_task_id() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T3", "# T3\n");
    let yaml = r#"mission_id: demo

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
  - id: T3
    title: "t3"
    kind: code
    depends_on: [T99]
    spec: specs/T3/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "DAG_MISSING_DEP");
    assert_eq!(json["context"]["task_id"], "T3");
    assert_eq!(json["context"]["missing_dep"], "T99");
}

#[test]
fn fill_marker_returns_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T1", "# T1\n");
    let yaml = r#"mission_id: demo

planning_level:
  requested: light
  effective: light

outcome_interpretation:
  summary: "[codex1-fill:outcome_interpretation]"

architecture:
  summary: "x"
  key_decisions: ["x"]

planning_process:
  evidence:
    - kind: direct_reasoning
      summary: "x"

tasks:
  - id: T1
    title: "t1"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(
        json["message"].as_str().unwrap().contains("codex1-fill"),
        "message should mention fill marker: {}",
        json["message"]
    );
}

#[test]
fn hard_plan_without_evidence_returns_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T1", "# T1\n");
    let yaml = r#"mission_id: demo

planning_level:
  requested: hard
  effective: hard

outcome_interpretation:
  summary: "x"

architecture:
  summary: "x"
  key_decisions: ["x"]

planning_process:
  evidence: []

tasks:
  - id: T1
    title: "t1"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert_eq!(json["context"]["missing_hard_evidence"], true);
}

#[test]
fn review_task_without_review_target_returns_plan_invalid() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    write_spec(&mission_dir, "T1", "# T1\n");
    write_spec(&mission_dir, "T2", "# T2\n");
    let yaml = r#"mission_id: demo

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
    title: "t1"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: "review"
    kind: review
    depends_on: [T1]
    spec: specs/T2/SPEC.md

risks:
  - risk: "r"
    mitigation: "m"

mission_close:
  criteria: ["c"]
"#;
    write_plan(&mission_dir, yaml);

    let output = run_check(&tmp, "demo", &[]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(
        json["message"].as_str().unwrap().contains("review_target"),
        "message missing review_target: {}",
        json["message"]
    );
}

#[test]
fn dry_run_does_not_lock_plan() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = seed_valid_mission(&tmp, "demo");

    let output = run_check(&tmp, "demo", &["--dry-run"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["locked"], false);
    assert_eq!(json["data"]["tasks"], 4);

    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["locked"], false);
    let events = read_events(&mission_dir);
    assert!(events.is_empty(), "dry-run must not append events");
}

#[test]
fn expect_revision_mismatch_returns_revision_conflict() {
    let tmp = TempDir::new().unwrap();
    seed_valid_mission(&tmp, "demo");

    let output = run_check(&tmp, "demo", &["--expect-revision", "999"]);
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["context"]["expected"], 999);
}

#[test]
fn re_running_on_locked_plan_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = seed_valid_mission(&tmp, "demo");

    let first = run_check(&tmp, "demo", &[]);
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_json = parse_stdout_json(&first);
    assert_eq!(first_json["data"]["locked"], true);
    let revision_after_first = read_state(&mission_dir)["revision"].as_u64().unwrap();
    let events_after_first = read_events(&mission_dir).len();
    let hash = first_json["data"]["plan_hash"]
        .as_str()
        .unwrap()
        .to_string();

    let second = run_check(&tmp, "demo", &[]);
    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_json = parse_stdout_json(&second);
    assert_eq!(second_json["data"]["plan_hash"], hash);
    assert_eq!(second_json["data"]["locked"], true);

    let revision_after_second = read_state(&mission_dir)["revision"].as_u64().unwrap();
    let events_after_second = read_events(&mission_dir).len();
    assert_eq!(
        revision_after_first, revision_after_second,
        "idempotent re-check must not bump revision"
    );
    assert_eq!(
        events_after_first, events_after_second,
        "idempotent re-check must not append events"
    );
}

/// Regression guard for F11 (iter-4): a mission locked by a pre-`plan.task_ids`
/// binary must be able to backfill `task_ids` on the next `plan check`,
/// THEN resume idempotent behavior.
#[test]
fn plan_check_backfills_missing_task_ids_and_then_stays_idempotent() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = seed_valid_mission(&tmp, "demo");

    let first = run_check(&tmp, "demo", &[]);
    assert!(first.status.success());
    let state_after_first = read_state(&mission_dir);
    let revision_after_first = state_after_first["revision"].as_u64().unwrap();
    let task_ids_after_first: Vec<String> = state_after_first["plan"]["task_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(task_ids_after_first, ["T1", "T2", "T3", "T4"]);

    let mut stripped = state_after_first.clone();
    if let Some(plan_obj) = stripped["plan"].as_object_mut() {
        plan_obj.remove("task_ids");
    }
    fs::write(
        mission_dir.join("STATE.json"),
        serde_json::to_vec_pretty(&stripped).unwrap(),
    )
    .unwrap();

    let backfill = run_check(&tmp, "demo", &[]);
    assert!(backfill.status.success());
    let state_after_backfill = read_state(&mission_dir);
    let revision_after_backfill = state_after_backfill["revision"].as_u64().unwrap();
    let task_ids_after_backfill: Vec<String> = state_after_backfill["plan"]["task_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(task_ids_after_backfill, ["T1", "T2", "T3", "T4"]);
    assert!(
        revision_after_backfill > revision_after_first,
        "backfill must bump revision (was {revision_after_first}, now {revision_after_backfill})"
    );

    let idempotent = run_check(&tmp, "demo", &[]);
    assert!(idempotent.status.success());
    let revision_after_idempotent = read_state(&mission_dir)["revision"].as_u64().unwrap();
    assert_eq!(
        revision_after_backfill, revision_after_idempotent,
        "second re-check after backfill must be idempotent (no revision bump)"
    );
}
