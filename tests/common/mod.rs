#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn bin() -> Command {
    Command::cargo_bin("codex1").unwrap()
}

pub fn repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    dir
}

pub fn json_output(command: &mut Command) -> Value {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

pub fn setup_status(repo: &TempDir) -> Value {
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "status"]),
    )
}

pub fn managed_skill_paths(repo: &TempDir) -> Vec<String> {
    status_collection_paths(&setup_status(repo), "skills")
}

pub fn managed_supporting_doc_paths(repo: &TempDir) -> Vec<String> {
    status_collection_paths(&setup_status(repo), "supporting_docs")
}

pub fn status_collection_paths(status_output: &Value, collection: &str) -> Vec<String> {
    status_output["data"]["status"][collection]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap().to_string())
        .collect()
}

pub fn planned_materialized_paths(value: &Value) -> Vec<PathBuf> {
    value["data"]["plan"]["materialized"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| PathBuf::from(path.as_str().unwrap()))
        .collect()
}

pub fn init(repo: &TempDir, mission: &str) {
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", mission, "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""ok": true"#));
}

#[cfg(unix)]
pub fn fake_codex_script(body: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fake-codex");
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    (dir, path)
}

#[cfg(unix)]
pub fn fake_codex_script_named(name: &str, body: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    (dir, path)
}

#[cfg(unix)]
pub fn fake_codex_jsonl_script(final_message: &str) -> (TempDir, PathBuf) {
    let body = format!(
        "#!/usr/bin/env bash\ncat <<'JSONL'\n{}\n{}\n{}\nJSONL\n",
        serde_json::json!({"type": "thread.started", "thread_id": "test"}),
        serde_json::json!({
            "type": "item.completed",
            "item": {
                "id": "item_0",
                "type": "agent_message",
                "text": final_message
            }
        }),
        serde_json::json!({"type": "turn.completed"})
    );
    fake_codex_script(&body)
}

#[cfg(unix)]
pub fn process_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
pub fn wait_until(deadline: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    condition()
}
