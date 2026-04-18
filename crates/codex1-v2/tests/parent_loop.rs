//! Wave 4 acceptance: `codex1 parent-loop activate | pause | resume |
//! deactivate` round-trips and correctly drives the status envelope's
//! `stop_policy` and `parent_loop` fields.

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

fn init(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
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
fn fresh_mission_has_no_active_loop() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let s = status(&dir);
    assert_eq!(s["parent_loop"]["active"], false);
    assert_eq!(s["parent_loop"]["mode"], "none");
    assert_eq!(s["parent_loop"]["paused"], false);
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "no_active_loop");
}

#[test]
fn activate_blocks_stop_with_active_parent_loop() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    bin(&dir)
        .args([
            "--json",
            "parent-loop",
            "activate",
            "--mission",
            "m1",
            "--mode",
            "execute",
        ])
        .assert()
        .success();
    let s = status(&dir);
    assert_eq!(s["parent_loop"]["active"], true);
    assert_eq!(s["parent_loop"]["mode"], "execute");
    assert_eq!(s["parent_loop"]["paused"], false);
    assert_eq!(s["stop_policy"]["allow_stop"], false);
    assert_eq!(s["stop_policy"]["reason"], "active_parent_loop");
}

#[test]
fn pause_allows_stop_with_discussion_pause() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    bin(&dir)
        .args([
            "--json",
            "parent-loop",
            "activate",
            "--mission",
            "m1",
            "--mode",
            "review",
        ])
        .assert()
        .success();
    bin(&dir)
        .args(["--json", "parent-loop", "pause", "--mission", "m1"])
        .assert()
        .success();
    let s = status(&dir);
    assert_eq!(s["parent_loop"]["active"], true);
    assert_eq!(s["parent_loop"]["paused"], true);
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "discussion_pause");
}

#[test]
fn resume_restores_blocking_stop() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    bin(&dir)
        .args([
            "--json",
            "parent-loop",
            "activate",
            "--mission",
            "m1",
            "--mode",
            "execute",
        ])
        .assert()
        .success();
    bin(&dir)
        .args(["--json", "parent-loop", "pause", "--mission", "m1"])
        .assert()
        .success();
    bin(&dir)
        .args(["--json", "parent-loop", "resume", "--mission", "m1"])
        .assert()
        .success();
    let s = status(&dir);
    assert_eq!(s["stop_policy"]["allow_stop"], false);
    assert_eq!(s["stop_policy"]["reason"], "active_parent_loop");
    assert_eq!(s["parent_loop"]["paused"], false);
}

#[test]
fn deactivate_resets_loop_and_allows_stop() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    bin(&dir)
        .args([
            "--json",
            "parent-loop",
            "activate",
            "--mission",
            "m1",
            "--mode",
            "autopilot",
        ])
        .assert()
        .success();
    bin(&dir)
        .args(["--json", "parent-loop", "deactivate", "--mission", "m1"])
        .assert()
        .success();
    let s = status(&dir);
    assert_eq!(s["parent_loop"]["active"], false);
    assert_eq!(s["parent_loop"]["mode"], "none");
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "no_active_loop");
}

#[test]
fn pause_without_active_loop_errors() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let out = bin(&dir)
        .args(["--json", "parent-loop", "pause", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "INTERNAL_ERROR");
}

#[test]
fn activate_with_invalid_mode_errors() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let out = bin(&dir)
        .args([
            "--json",
            "parent-loop",
            "activate",
            "--mission",
            "m1",
            "--mode",
            "bogus",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "INTERNAL_ERROR");
    assert!(
        env["message"]
            .as_str()
            .unwrap()
            .contains("unknown parent-loop mode")
    );
}

#[test]
fn full_round_trip_ends_clean() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    for (cmd, args) in [
        ("activate", vec!["--mode", "execute"]),
        ("pause", vec![]),
        ("resume", vec![]),
        ("deactivate", vec![]),
    ] {
        let mut full = vec!["--json", "parent-loop", cmd, "--mission", "m1"];
        full.extend(args);
        bin(&dir).args(&full).assert().success();
    }
    let s = status(&dir);
    assert_eq!(s["parent_loop"]["mode"], "none");
    // And state_revision should have advanced by 4 (one per transition).
    let state: Value =
        serde_json::from_slice(&fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap()).unwrap();
    assert!(state["state_revision"].as_u64().unwrap() >= 5);
}
