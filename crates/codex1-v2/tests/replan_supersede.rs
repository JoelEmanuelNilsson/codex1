//! Wave 3 acceptance: `replan record` marks tasks Superseded without
//! erasing history; `replan check` flags mandatory triggers.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn bin(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codex1").expect("binary built");
    cmd.arg("--repo-root").arg(dir.path());
    cmd
}

fn last_json(out: &[u8]) -> Value {
    let s = std::str::from_utf8(out).unwrap();
    serde_json::from_str(s.lines().last().unwrap()).unwrap()
}

fn boot(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    fs::write(
        dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md"),
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         <!-- codex1:plan-dag:end -->\n",
    )
    .unwrap();
}

#[test]
fn replan_record_marks_task_superseded_and_writes_log() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    // Give T1 some prior status.
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": "executing",
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": { "T1": { "status": "needs_repair" } }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();

    let out = bin(&dir)
        .args([
            "--json",
            "replan",
            "record",
            "--mission",
            "m1",
            "--reason",
            "six_consecutive_non_clean_reviews",
            "--supersedes",
            "T1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["reason"], "six_consecutive_non_clean_reviews");
    assert_eq!(env["superseded_task_ids"], serde_json::json!(["T1"]));

    // REPLAN-LOG.md exists and contains the event.
    let log_path = dir.path().join("PLANS/m1/REPLAN-LOG.md");
    let log = fs::read_to_string(&log_path).unwrap();
    assert!(log.contains("six_consecutive_non_clean_reviews"));
    assert!(log.contains("T1"));

    // T1 is now Superseded.
    let state: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "superseded");
    assert_eq!(state["phase"], "replanning");
}

#[test]
fn replan_record_requires_existing_task_id() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let out = bin(&dir)
        .args([
            "--json",
            "replan",
            "record",
            "--mission",
            "m1",
            "--reason",
            "any",
            "--supersedes",
            "T99",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_MISSING_DEP");
}

#[test]
fn replan_check_with_no_failures_has_no_triggers() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let out = bin(&dir)
        .args(["--json", "replan", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["schema"], "codex1.replan.check.v1");
    assert_eq!(env["mandatory_triggers"].as_array().unwrap().len(), 0);
}

#[test]
fn plan_graph_emits_parsed_dag() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let out = bin(&dir)
        .args(["--json", "plan", "graph", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["schema"], "codex1.plan.graph.v1");
    assert_eq!(env["tasks"][0]["id"], "T1");
}
