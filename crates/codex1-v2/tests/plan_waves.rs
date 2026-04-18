//! Wave 1 acceptance: `codex1-v2 plan waves` derives the correct wave.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn bin(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codex1-v2").expect("binary built");
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

/// Overwrite STATE.json with a new task map + phase. Bump state_revision so
/// validate wouldn't fault; events.jsonl lag is acceptable per V2 rules.
fn set_state(dir: &Path, tasks: &[(&str, &str)], phase: &str) {
    let path = dir.join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let mut tasks_obj = serde_json::Map::new();
    for (id, status) in tasks {
        tasks_obj.insert(
            (*id).to_string(),
            serde_json::json!({ "status": status }),
        );
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

fn waves(dir: &TempDir) -> Value {
    let out = bin(dir)
        .args(["--json", "plan", "waves", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    last_json(&out)
}

#[test]
fn empty_dag_produces_no_waves() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let env = waves(&dir);
    assert_eq!(env["waves"].as_array().unwrap().len(), 0);
    assert_eq!(env["blocked"].as_array().unwrap().len(), 0);
}

#[test]
fn single_ready_task_is_serial_wave() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    depends_on: []\n",
    );
    set_state(dir.path(), &[("T1", "ready")], "executing");
    let env = waves(&dir);
    let w = &env["waves"][0];
    assert_eq!(w["tasks"], serde_json::json!(["T1"]));
    assert_eq!(w["mode"], "serial");
}

#[test]
fn two_independent_disjoint_writes_run_parallel() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    write_paths: [src/a/**]\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    write_paths: [src/b/**]\n",
    );
    set_state(
        dir.path(),
        &[("T1", "ready"), ("T2", "ready")],
        "executing",
    );
    let env = waves(&dir);
    let w = &env["waves"][0];
    assert_eq!(w["mode"], "parallel");
    assert_eq!(w["safety"]["write_paths_disjoint"], true);
}

#[test]
fn overlapping_writes_force_serial() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    write_paths: [src/**]\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    write_paths: [src/foo/**]\n",
    );
    set_state(
        dir.path(),
        &[("T1", "ready"), ("T2", "ready")],
        "executing",
    );
    let env = waves(&dir);
    let w = &env["waves"][0];
    assert_eq!(w["mode"], "serial");
    assert_eq!(w["safety"]["write_paths_disjoint"], false);
}

#[test]
fn unknown_side_effects_forces_serial() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    write_paths: [src/a/**]\n    unknown_side_effects: true\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    write_paths: [src/b/**]\n",
    );
    set_state(
        dir.path(),
        &[("T1", "ready"), ("T2", "ready")],
        "executing",
    );
    let env = waves(&dir);
    let w = &env["waves"][0];
    assert_eq!(w["mode"], "serial");
    assert_eq!(w["safety"]["unknown_side_effects"], true);
}

#[test]
fn shared_exclusive_resources_force_serial() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    write_paths: [a/**]\n    exclusive_resources: [r]\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    write_paths: [b/**]\n    exclusive_resources: [r]\n",
    );
    set_state(
        dir.path(),
        &[("T1", "ready"), ("T2", "ready")],
        "executing",
    );
    let env = waves(&dir);
    let w = &env["waves"][0];
    assert_eq!(w["mode"], "serial");
    assert_eq!(w["safety"]["exclusive_resources_disjoint"], false);
}

#[test]
fn dep_not_clean_puts_task_in_blocked() {
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
    let env = waves(&dir);
    assert_eq!(env["waves"].as_array().unwrap().len(), 0);
    assert_eq!(env["blocked"][0]["task_id"], "T2");
    assert_eq!(env["blocked"][0]["blocked_by"], serde_json::json!(["T1"]));
}

#[test]
fn read_write_conflict_records_pair_and_forces_serial() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    read_paths: [src/foo/**]\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    write_paths: [src/foo/bar.rs]\n",
    );
    set_state(
        dir.path(),
        &[("T1", "ready"), ("T2", "ready")],
        "executing",
    );
    let env = waves(&dir);
    let w = &env["waves"][0];
    assert_eq!(w["mode"], "serial");
    let conflicts = w["safety"]["read_write_conflicts"].as_array().unwrap();
    assert!(!conflicts.is_empty());
    assert_eq!(conflicts[0]["reader"], "T1");
    assert_eq!(conflicts[0]["writer"], "T2");
}
