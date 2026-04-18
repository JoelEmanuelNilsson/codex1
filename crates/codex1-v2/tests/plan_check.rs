//! Wave 1 acceptance: `codex1-v2 plan check` catches DAG problems.

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
        "# Program Blueprint\n\n<!-- codex1:plan-dag:start -->\n{yaml_body}\n<!-- codex1:plan-dag:end -->\n"
    );
    fs::write(&path, content).unwrap();
}

fn last_json(out: &[u8]) -> Value {
    let s = std::str::from_utf8(out).unwrap();
    serde_json::from_str(s.lines().last().unwrap()).unwrap()
}

#[test]
fn empty_tasks_passes_plan_check() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], "codex1.plan.check.v1");
    assert_eq!(env["task_count"], 0);
}

#[test]
fn single_task_dag_passes() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Scaffold\n    kind: code\n    depends_on: []\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["task_count"], 1);
    assert_eq!(env["task_ids"], serde_json::json!(["T1"]));
}

#[test]
fn duplicate_id_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n\
         \x20 - id: T1\n    title: B\n    kind: code\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_DUPLICATE_ID");
}

#[test]
fn cycle_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    depends_on: [T2]\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    depends_on: [T1]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_CYCLE");
}

#[test]
fn invalid_id_format_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    for bad in ["t1", "TASK1", "T-1"] {
        write_blueprint(
            dir.path(),
            &format!(
                "planning:\n  requested_level: light\n  graph_revision: 1\n\
                 tasks:\n  - id: {bad}\n    title: X\n    kind: code\n"
            ),
        );
        let out = bin(&dir)
            .args(["--json", "plan", "check", "--mission", "m1"])
            .assert()
            .failure()
            .get_output()
            .stdout
            .clone();
        let env = last_json(&out);
        assert_eq!(env["code"], "DAG_BAD_ID", "{bad} should be rejected");
    }
}

#[test]
fn missing_dep_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    depends_on: [T99]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_MISSING_DEP");
    assert_eq!(env["details"]["missing"], "T99");
}

#[test]
fn missing_markers_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let path = dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    fs::write(&path, "# just prose\n").unwrap();
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_NO_BLOCK");
}

#[test]
fn unknown_task_field_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    wat: unexpected\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_BAD_SCHEMA");
}

#[test]
fn mission_not_found_when_no_init() {
    let dir = TempDir::new().unwrap();
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "MISSION_NOT_FOUND");
}
