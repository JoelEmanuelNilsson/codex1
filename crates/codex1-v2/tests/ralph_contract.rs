//! Wave 4 acceptance: the Ralph contract.
//!
//! Ralph consumes `codex1 status --mission <id> --json` and blocks stop
//! based on `stop_policy.allow_stop`. These tests exercise every
//! (active, paused, verdict) combination the status envelope can emit
//! and assert the derived policy matches the V2 contract table.

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
    let p = dir.join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    fs::write(
        &p,
        format!(
            "# BP\n\n<!-- codex1:plan-dag:start -->\n{yaml_body}\n<!-- codex1:plan-dag:end -->\n"
        ),
    )
    .unwrap();
}

fn write_state(dir: &Path, phase: &str, mode: &str, paused: bool, tasks: &[(&str, &str)]) {
    let sp = dir.join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let mut tasks_obj = serde_json::Map::new();
    for (id, status) in tasks {
        tasks_obj.insert((*id).to_string(), serde_json::json!({ "status": status }));
    }
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": phase,
        "parent_loop": { "mode": mode, "paused": paused },
        "tasks": serde_json::Value::Object(tasks_obj),
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();
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

/// Table-driven: (mode, paused, expected `allow_stop`, expected reason).
#[test]
fn stop_policy_matrix_empty_dag() {
    let cases: &[(&str, bool, bool, &str)] = &[
        ("none", false, true, "no_active_loop"),
        ("execute", false, false, "active_parent_loop"),
        ("execute", true, true, "discussion_pause"),
        ("review", false, false, "active_parent_loop"),
        ("review", true, true, "discussion_pause"),
        ("autopilot", false, false, "active_parent_loop"),
        ("autopilot", true, true, "discussion_pause"),
        ("close", false, false, "active_parent_loop"),
        ("close", true, true, "discussion_pause"),
    ];
    for (mode, paused, allow, reason) in cases {
        let dir = TempDir::new().unwrap();
        init(&dir);
        write_state(dir.path(), "clarify", mode, *paused, &[]);
        let s = status(&dir);
        assert_eq!(
            s["stop_policy"]["allow_stop"],
            serde_json::Value::Bool(*allow),
            "({mode}, paused={paused}) should allow_stop={allow}"
        );
        assert_eq!(
            s["stop_policy"]["reason"], *reason,
            "({mode}, paused={paused}) reason"
        );
    }
}

#[test]
fn stop_policy_for_complete_when_inactive() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(dir.path(), "complete", "none", false, &[("T1", "complete")]);
    let s = status(&dir);
    assert_eq!(s["verdict"], "complete");
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "complete");
}

#[test]
fn stop_policy_for_invalid_state_when_inactive() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    // phase=complete but task=ready → invalid_state.
    write_state(dir.path(), "complete", "none", false, &[("T1", "ready")]);
    let s = status(&dir);
    assert_eq!(s["verdict"], "invalid_state");
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "invalid_state");
}

#[test]
fn active_loop_blocks_even_when_task_eligible() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(
        dir.path(),
        "executing",
        "execute",
        false,
        &[("T1", "ready")],
    );
    let s = status(&dir);
    assert_eq!(s["verdict"], "continue_required");
    assert_eq!(s["stop_policy"]["allow_stop"], false);
    assert_eq!(s["stop_policy"]["reason"], "active_parent_loop");
}

#[test]
fn paused_loop_allows_even_when_task_eligible() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(dir.path(), "executing", "execute", true, &[("T1", "ready")]);
    let s = status(&dir);
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "discussion_pause");
}

#[test]
fn ralph_hook_script_syntax_is_valid() {
    // Ensure the shipped Ralph hook is syntactically valid bash. The
    // script lives at repo root so we walk up from CARGO_MANIFEST_DIR.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set");
    let script = Path::new(&manifest)
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .join("scripts/ralph-status-hook.sh");
    assert!(script.exists(), "expected {}", script.display());
    let status = std::process::Command::new("bash")
        .arg("-n")
        .arg(&script)
        .status()
        .expect("bash available");
    assert!(status.success(), "ralph-status-hook.sh failed `bash -n`");
}
