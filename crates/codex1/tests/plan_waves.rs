//! Integration tests for Phase B Unit 5 (`plan waves` / `plan graph`).
//!
//! Each test seeds its own PLANS/<mission>/ layout on disk via the real
//! `codex1 init` binary, then rewrites PLAN.yaml and STATE.json to
//! exercise the derivation logic. Waves are derived on every call, so
//! STATE.json changes should reshape `current_ready_wave` without
//! changing the wave list itself.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn init_mission(tmp: &TempDir, mission: &str) -> PathBuf {
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

/// Overwrite the mission's PLAN.yaml and mark plan.locked = true in
/// STATE.json, preserving the schema-version and revision fields.
fn seed_plan(mission_dir: &Path, plan_yaml: &str) {
    fs::write(mission_dir.join("PLAN.yaml"), plan_yaml).unwrap();
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["plan"]["locked"] = Value::Bool(true);
    state["plan"]["hash"] = Value::String(codex1::state::plan_hash(plan_yaml.as_bytes()));
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
}

/// Write a STATE.json task record with a specific status. All tests use
/// this instead of relying on tasks being absent (which is treated as
/// Pending by the derivation).
fn set_task_status(mission_dir: &Path, task_id: &str, status: &str) {
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let tasks = state["tasks"].as_object_mut().unwrap();
    tasks.insert(
        task_id.to_string(),
        serde_json::json!({ "id": task_id, "status": status }),
    );
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
}

const FIVE_TASK_PLAN: &str = r"mission_id: demo
tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
  - id: T2
    title: Branch A
    kind: code
    depends_on: [T1]
  - id: T3
    title: Branch B
    kind: code
    depends_on: [T1]
  - id: T4
    title: Join
    kind: code
    depends_on: [T2, T3]
  - id: T5
    title: Review after join
    kind: review
    depends_on: [T4]
";

fn run_waves(tmp: &TempDir, mission: &str) -> Value {
    let output = cmd()
        .current_dir(tmp.path())
        .args(["plan", "waves", "--mission", mission])
        .output()
        .expect("runs");
    parse_stdout_json(&output)
}

fn run_graph(tmp: &TempDir, mission: &str, extra: &[&str]) -> Value {
    let mut args = vec!["plan", "graph", "--mission", mission];
    args.extend_from_slice(extra);
    let output = cmd()
        .current_dir(tmp.path())
        .args(&args)
        .output()
        .expect("runs");
    parse_stdout_json(&output)
}

#[test]
fn waves_from_five_task_dag() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    seed_plan(&mission_dir, FIVE_TASK_PLAN);

    let json = run_waves(&tmp, "demo");
    assert_eq!(json["ok"], Value::Bool(true), "{json}");
    let waves = json["data"]["waves"].as_array().unwrap();
    assert_eq!(waves.len(), 4, "waves: {waves:?}");
    assert_eq!(waves[0]["wave_id"], "W1");
    assert_eq!(waves[0]["tasks"], serde_json::json!(["T1"]));
    assert_eq!(waves[1]["wave_id"], "W2");
    let mut w2 = waves[1]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    w2.sort();
    assert_eq!(w2, vec!["T2", "T3"]);
    assert_eq!(waves[2]["wave_id"], "W3");
    assert_eq!(waves[2]["tasks"], serde_json::json!(["T4"]));
    assert_eq!(waves[3]["wave_id"], "W4");
    assert_eq!(waves[3]["tasks"], serde_json::json!(["T5"]));

    assert_eq!(json["data"]["current_ready_wave"], "W1");
    assert_eq!(json["data"]["all_tasks_complete"], Value::Bool(false));
    for wave in waves {
        assert_eq!(wave["parallel_safe"], Value::Bool(true));
        assert_eq!(wave["blockers"], serde_json::json!([]));
    }
}

#[test]
fn current_ready_wave_advances_when_t1_complete() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    seed_plan(&mission_dir, FIVE_TASK_PLAN);
    set_task_status(&mission_dir, "T1", "complete");

    let json = run_waves(&tmp, "demo");
    // The wave list itself does NOT recompute — it stays structural.
    let waves = json["data"]["waves"].as_array().unwrap();
    assert_eq!(waves.len(), 4);
    assert_eq!(waves[0]["tasks"], serde_json::json!(["T1"]));
    // But current_ready_wave skips past the completed W1.
    assert_eq!(json["data"]["current_ready_wave"], "W2");
}

#[test]
fn current_ready_wave_does_not_skip_in_progress_dependency() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    seed_plan(&mission_dir, FIVE_TASK_PLAN);
    set_task_status(&mission_dir, "T1", "in_progress");

    let json = run_waves(&tmp, "demo");
    assert_eq!(json["ok"], Value::Bool(true), "{json}");
    assert_eq!(json["data"]["current_ready_wave"], Value::Null);
}

#[test]
fn exclusive_resource_collision_marks_wave_unsafe() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    let plan = r"mission_id: demo
tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
  - id: T2
    title: Branch A
    kind: code
    depends_on: [T1]
    exclusive_resources: [shared-db]
  - id: T3
    title: Branch B
    kind: code
    depends_on: [T1]
    exclusive_resources: [shared-db]
";
    seed_plan(&mission_dir, plan);
    let json = run_waves(&tmp, "demo");
    let waves = json["data"]["waves"].as_array().unwrap();
    assert_eq!(waves[1]["wave_id"], "W2");
    assert_eq!(waves[1]["parallel_safe"], Value::Bool(false));
    let blockers = waves[1]["blockers"].as_array().unwrap();
    assert!(
        blockers.iter().any(|b| b == "exclusive_resource:shared-db"),
        "blockers missing shared-db: {blockers:?}"
    );
}

#[test]
fn unknown_side_effects_marks_wave_unsafe() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    let plan = r"mission_id: demo
tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
  - id: T2
    title: Branch A
    kind: code
    depends_on: [T1]
    unknown_side_effects: true
  - id: T3
    title: Branch B
    kind: code
    depends_on: [T1]
";
    seed_plan(&mission_dir, plan);
    let json = run_waves(&tmp, "demo");
    let waves = json["data"]["waves"].as_array().unwrap();
    assert_eq!(waves[1]["wave_id"], "W2");
    assert_eq!(waves[1]["parallel_safe"], Value::Bool(false));
    let blockers = waves[1]["blockers"].as_array().unwrap();
    assert!(
        blockers.iter().any(|b| b == "unknown_side_effects:T2"),
        "blockers missing unknown_side_effects:T2: {blockers:?}"
    );
}

#[test]
fn waves_on_fresh_mission_without_plan_lock() {
    let tmp = TempDir::new().unwrap();
    init_mission(&tmp, "demo");
    // plan.locked is false straight out of init.
    let json = run_waves(&tmp, "demo");
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["waves"], serde_json::json!([]));
    assert_eq!(json["data"]["current_ready_wave"], Value::Null);
    assert_eq!(json["data"]["all_tasks_complete"], Value::Bool(false));
    assert!(json["data"]["note"].is_string());
}

#[test]
fn graph_mermaid_emits_flowchart_with_all_nodes() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    seed_plan(&mission_dir, FIVE_TASK_PLAN);
    let json = run_graph(&tmp, "demo", &["--format", "mermaid"]);
    let mermaid = json["data"]["mermaid"].as_str().unwrap();
    assert!(mermaid.starts_with("flowchart TD\n"), "got: {mermaid}");
    for id in ["T1", "T2", "T3", "T4", "T5"] {
        assert!(
            mermaid.contains(&format!("{id}[\"")),
            "mermaid missing node {id}:\n{mermaid}"
        );
    }
    // Edges rendered.
    assert!(mermaid.contains("T1 --> T2"));
    assert!(mermaid.contains("T4 --> T5"));
    // Class lines use snake_case tokens.
    assert!(mermaid.contains("class T1 ready"));
}

#[test]
fn graph_json_emits_nodes_and_edges() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    seed_plan(&mission_dir, FIVE_TASK_PLAN);
    let json = run_graph(&tmp, "demo", &["--format", "json"]);
    let graph = &json["data"]["graph"];
    let nodes = graph["nodes"].as_array().unwrap();
    let edges = graph["edges"].as_array().unwrap();
    assert_eq!(nodes.len(), 5);
    assert_eq!(edges.len(), 5);
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    for id in ["T1", "T2", "T3", "T4", "T5"] {
        assert!(ids.contains(&id), "nodes missing {id}: {ids:?}");
    }
    // Every node reports a status string.
    assert!(nodes.iter().all(|n| n["status"].is_string()));
}

#[test]
fn graph_out_flag_writes_file_and_reports_path() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    seed_plan(&mission_dir, FIVE_TASK_PLAN);
    let out = tmp.path().join("graph.mmd");
    let json = run_graph(
        &tmp,
        "demo",
        &["--format", "mermaid", "--out", out.to_str().unwrap()],
    );
    assert_eq!(json["data"]["path"], out.to_string_lossy().as_ref());
    let content = fs::read_to_string(&out).unwrap();
    assert!(content.starts_with("flowchart TD\n"));
    assert!(content.contains("T1 --> T2"));
}

#[test]
fn graph_and_waves_reject_locked_plan_hash_drift() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp, "demo");
    seed_plan(&mission_dir, FIVE_TASK_PLAN);
    fs::write(
        mission_dir.join("PLAN.yaml"),
        FIVE_TASK_PLAN.replace("depends_on: [T2, T3]", "depends_on: [T2]"),
    )
    .unwrap();

    let waves = cmd()
        .current_dir(tmp.path())
        .args(["plan", "waves", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!waves.status.success());
    let waves_json = parse_stdout_json(&waves);
    assert_eq!(waves_json["code"], "PLAN_INVALID");

    let graph = cmd()
        .current_dir(tmp.path())
        .args(["plan", "graph", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!graph.status.success());
    let graph_json = parse_stdout_json(&graph);
    assert_eq!(graph_json["code"], "PLAN_INVALID");
}
