//! Wave 1 acceptance: `codex1-v2 status` emits a stable, contract-shaped
//! envelope across every verdict it can produce.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn bin(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codex1").expect("binary built");
    cmd.arg("--repo-root").arg(dir.path());
    cmd
}

fn init(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "Test"])
        .assert()
        .success();
}

fn write_blueprint(dir: &Path, yaml_body: &str) {
    let path = dir.join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    let content = format!(
        "# BP\n\n<!-- codex1:plan-dag:start -->\n{yaml_body}\n<!-- codex1:plan-dag:end -->\n"
    );
    fs::write(&path, content).unwrap();
}

fn set_state(dir: &Path, tasks: &[(&str, &str)], phase: &str) {
    let path = dir.join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let mut tasks_obj = serde_json::Map::new();
    for (id, status) in tasks {
        tasks_obj.insert((*id).to_string(), serde_json::json!({ "status": status }));
    }
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": phase,
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": serde_json::Value::Object(tasks_obj),
    });
    fs::write(&path, serde_json::to_vec_pretty(&new).unwrap()).unwrap();
}

fn last_json(out: &[u8]) -> Value {
    let s = std::str::from_utf8(out).unwrap();
    serde_json::from_str(s.lines().last().unwrap()).unwrap()
}

fn status(dir: &TempDir) -> Value {
    let out = bin(dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    last_json(&out)
}

#[test]
fn fresh_mission_is_needs_user_with_plan_dag_empty() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let env = status(&dir);
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], "codex1.status.v1");
    assert_eq!(env["verdict"], "needs_user");
    assert_eq!(env["next_action"]["kind"], "user_decision");
    assert_eq!(env["required_user_decision"], "plan_dag_empty");
    assert_eq!(env["parent_loop"]["active"], false);
    assert_eq!(env["parent_loop"]["mode"], "none");
    assert_eq!(env["parent_loop"]["paused"], false);
    assert_eq!(env["stop_policy"]["allow_stop"], true);
    assert_eq!(env["stop_policy"]["reason"], "no_active_loop");
    assert_eq!(env["terminality"], "non_terminal");
}

#[test]
fn ready_task_produces_continue_required_with_start_task() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    depends_on: []\n",
    );
    set_state(dir.path(), &[("T1", "ready")], "executing");
    let env = status(&dir);
    assert_eq!(env["verdict"], "continue_required");
    assert_eq!(env["next_action"]["kind"], "start_task");
    assert_eq!(env["next_action"]["task_id"], "T1");
    assert_eq!(env["ready_tasks"], serde_json::json!(["T1"]));
}

#[test]
fn all_complete_tasks_give_verdict_complete() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    set_state(dir.path(), &[("T1", "complete")], "complete");
    let env = status(&dir);
    assert_eq!(env["verdict"], "complete");
    assert_eq!(env["terminality"], "terminal");
    assert_eq!(env["next_action"]["kind"], "complete");
    assert_eq!(env["stop_policy"]["reason"], "complete");
}

#[test]
fn phase_complete_with_non_terminal_task_is_invalid_state() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    set_state(dir.path(), &[("T1", "ready")], "complete");
    let env = status(&dir);
    assert_eq!(env["verdict"], "invalid_state");
    assert_eq!(env["next_action"]["kind"], "invalid_state");
    assert_eq!(env["stop_policy"]["reason"], "invalid_state");
}

#[test]
fn envelope_contains_all_contract_fields() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let env = status(&dir);
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
        assert!(env.get(key).is_some(), "missing field {key}");
    }
}

#[test]
fn task_next_mirrors_next_action() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let out = bin(&dir)
        .args(["--json", "task", "next", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["schema"], "codex1.task.next.v1");
    assert_eq!(env["verdict"], "needs_user");
    assert_eq!(env["next_action"]["kind"], "user_decision");
    // Round 4: `task next` must surface the wave-parallel hint so $execute
    // does not need a separate `status` call to see it.
    assert!(
        env.get("ready_wave_parallel_safe").is_some(),
        "task next must include ready_wave_parallel_safe"
    );
    // No ready tasks on a fresh empty-DAG mission → false.
    assert_eq!(env["ready_wave_parallel_safe"], false);
}

#[test]
fn task_next_reports_ready_wave_parallel_safe_true_for_disjoint_ready_tasks() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    write_paths: [src/a/**]\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    write_paths: [src/b/**]\n",
    );
    set_state(dir.path(), &[("T1", "ready"), ("T2", "ready")], "executing");
    let out = bin(&dir)
        .args(["--json", "task", "next", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["verdict"], "continue_required");
    assert_eq!(env["ready_wave_parallel_safe"], true);
    assert_eq!(env["ready_tasks"].as_array().unwrap().len(), 2);
}

#[test]
fn dep_blocked_status_is_blocked_verdict() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    depends_on: [T1]\n",
    );
    set_state(
        dir.path(),
        &[("T1", "in_progress"), ("T2", "ready")],
        "executing",
    );
    let env = status(&dir);
    assert_eq!(env["verdict"], "blocked");
    assert_eq!(env["blocked"], serde_json::json!(["T2"]));
    assert_eq!(env["running_tasks"], serde_json::json!(["T1"]));
}
