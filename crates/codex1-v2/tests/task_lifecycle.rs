//! Wave 2 acceptance: `task start` → `task finish` → `task status` lifecycle.

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

fn last_json(out: &[u8]) -> Value {
    let s = std::str::from_utf8(out).unwrap();
    serde_json::from_str(s.lines().last().unwrap()).unwrap()
}

fn init(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
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

fn write_proof(dir: &Path, task_id: &str, content: &[u8]) {
    let proof = dir.join(format!("PLANS/m1/specs/{task_id}/PROOF.md"));
    fs::create_dir_all(proof.parent().unwrap()).unwrap();
    fs::write(&proof, content).unwrap();
}

fn mk_single_task_mission(dir: &TempDir) {
    init(dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n",
    );
    set_state(dir.path(), &[("T1", "ready")], "executing");
}

#[test]
fn start_transitions_ready_to_in_progress_and_mints_run_id() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    let out = bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], "codex1.task.start.v1");
    assert_eq!(env["status"], "in_progress");
    assert!(env["task_run_id"].as_str().unwrap().starts_with("run-"));
    assert!(env["started_at"].is_string());

    // Verify STATE.json reflects the transition.
    let state: Value =
        serde_json::from_slice(&fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap()).unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "in_progress");
    assert_eq!(state["phase"], "executing");
}

#[test]
fn start_refuses_when_task_not_ready() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    set_state(dir.path(), &[("T1", "complete")], "complete");
    let out = bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "TASK_STATE_INVALID");
}

#[test]
fn start_refuses_when_deps_not_clean() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: a\n    kind: code\n\
         \x20 - id: T2\n    title: b\n    kind: code\n    depends_on: [T1]\n",
    );
    set_state(
        dir.path(),
        &[("T1", "in_progress"), ("T2", "ready")],
        "executing",
    );
    let out = bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T2"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "TASK_STATE_INVALID");
    assert!(
        env["details"]["current"]
            .as_str()
            .unwrap()
            .contains("dep_T1")
    );
}

#[test]
fn finish_reads_default_proof_and_hashes_it() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T1"])
        .assert()
        .success();
    write_proof(dir.path(), "T1", b"proof body");

    let out = bin(&dir)
        .args(["--json", "task", "finish", "--mission", "m1", "T1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], "codex1.task.finish.v1");
    assert_eq!(env["status"], "proof_submitted");
    assert_eq!(env["proof_ref"], "specs/T1/PROOF.md");
    assert!(env["proof_hash"].as_str().unwrap().starts_with("sha256:"));
}

#[test]
fn finish_errors_when_proof_missing() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T1"])
        .assert()
        .success();
    // No proof file written.
    let out = bin(&dir)
        .args(["--json", "task", "finish", "--mission", "m1", "T1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "PROOF_INVALID");
}

#[test]
fn finish_refuses_when_not_in_progress() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    write_proof(dir.path(), "T1", b"x");
    let out = bin(&dir)
        .args(["--json", "task", "finish", "--mission", "m1", "T1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "TASK_STATE_INVALID");
}

/// Round 6 Fix #1: Planned → Ready must go through the CLI, not a
/// hand-edit of STATE.json. `task ready` is the supported path.
#[test]
fn task_ready_transitions_planned_to_ready_and_emits_event() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n",
    );
    // Leave STATE.json untouched: T1 is implicitly `planned`.
    let out = bin(&dir)
        .args(["--json", "task", "ready", "--mission", "m1", "T1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], "codex1.task.ready.v1");
    assert_eq!(env["status"], "ready");
    // state_revision must have bumped past the init value of 1.
    assert!(env["state_revision"].as_u64().unwrap() >= 2);

    // STATE.json reflects Ready; events.jsonl carries the audit record.
    let state: Value =
        serde_json::from_slice(&fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap()).unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "ready");
    let events = fs::read_to_string(dir.path().join("PLANS/m1/events.jsonl")).unwrap();
    assert!(events.contains("\"kind\":\"task_marked_ready\""));
    assert!(events.contains("\"task_id\":\"T1\""));
}

#[test]
fn task_ready_refuses_when_not_planned() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir); // T1 is already `ready`
    let out = bin(&dir)
        .args(["--json", "task", "ready", "--mission", "m1", "T1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "TASK_STATE_INVALID");
}

#[test]
fn status_envelope_after_proof_submitted_wants_review_open() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T1"])
        .assert()
        .success();
    write_proof(dir.path(), "T1", b"p");
    bin(&dir)
        .args(["--json", "task", "finish", "--mission", "m1", "T1"])
        .assert()
        .success();

    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["verdict"], "continue_required");
    assert_eq!(env["next_action"]["kind"], "review_open");
    assert_eq!(env["next_action"]["task_id"], "T1");
    assert_eq!(env["review_required"], serde_json::json!(["T1"]));
}

/// Round 6 Fix #5: mutating commands accept --dry-run. Preconditions
/// still validated, envelope carries `dry_run: true`, and STATE.json +
/// events.jsonl remain unchanged.
#[test]
fn task_start_dry_run_validates_but_does_not_mutate() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);

    let sp = dir.path().join("PLANS/m1/STATE.json");
    let ep = dir.path().join("PLANS/m1/events.jsonl");
    let state_before: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let events_before = fs::read_to_string(&ep).unwrap();

    // Dry-run succeeds (preconditions pass), reports dry_run: true.
    let out = bin(&dir)
        .args([
            "--json",
            "--dry-run",
            "task",
            "start",
            "--mission",
            "m1",
            "T1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], true);
    assert_eq!(env["dry_run"], true);
    assert_eq!(env["schema"], "codex1.task.start.v1");
    // state_revision reports the WOULD-BE value.
    assert!(env["state_revision"].as_u64().unwrap() >= 2);

    // STATE.json + events.jsonl are unchanged on disk.
    let state_after: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let events_after = fs::read_to_string(&ep).unwrap();
    assert_eq!(state_before, state_after, "STATE.json must not change");
    assert_eq!(events_before, events_after, "events.jsonl must not change");
}

#[test]
fn task_start_dry_run_rejects_invalid_transition() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    // No blueprint → T1 not in DAG; dry-run still returns the error.
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks: []\n",
    );
    let out = bin(&dir)
        .args([
            "--json",
            "--dry-run",
            "task",
            "start",
            "--mission",
            "m1",
            "T99",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "TASK_STATE_INVALID");
}

#[test]
fn task_status_reports_spec_and_state() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T1"])
        .assert()
        .success();
    let out = bin(&dir)
        .args(["--json", "task", "status", "--mission", "m1", "T1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["schema"], "codex1.task.status.v1");
    assert_eq!(env["task_id"], "T1");
    assert_eq!(env["state"]["status"], "in_progress");
    assert!(env["state"]["task_run_id"].is_string());
}

#[test]
fn expect_revision_mismatch_returns_revision_conflict() {
    let dir = TempDir::new().unwrap();
    mk_single_task_mission(&dir);
    // Current state_revision after init + set_state is 1 (manual set_state
    // didn't bump). Pass a wrong expectation.
    let out = bin(&dir)
        .args([
            "--json",
            "--expect-revision",
            "99",
            "task",
            "start",
            "--mission",
            "m1",
            "T1",
        ])
        .assert()
        .code(4)
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "REVISION_CONFLICT");
    assert_eq!(env["retryable"], true);
}
