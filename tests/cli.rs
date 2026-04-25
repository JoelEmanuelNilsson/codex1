use std::fs;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("codex1").unwrap()
}

fn repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    dir
}

fn json_output(command: &mut Command) -> Value {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn json_output_with_stdin(command: &mut Command, stdin: String) -> Value {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn init(repo: &TempDir, mission: &str) {
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", mission, "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""ok": true"#));
}

#[test]
fn init_returns_success_envelope() {
    let repo = repo();
    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "init"]),
    );
    assert_eq!(value["ok"], true);
    assert!(repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/ready")
        .is_dir());
}

#[test]
fn argument_errors_can_be_json() {
    bin()
        .args(["--json", "init"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("ARGUMENT_ERROR"));
}

#[test]
fn unsafe_mission_id_is_rejected() {
    let repo = repo();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "../bad", "init"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));
}

#[test]
fn prd_interview_writes_artifact_and_respects_collision_policy() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("answers.json");
    fs::write(
        &answers,
        r#"{
          "title": "Alpha PRD",
          "original_request": "Build alpha",
          "interpreted_destination": "A deterministic alpha",
          "success_criteria": ["artifact exists"],
          "proof_expectations": ["cargo test"],
          "pr_intent": "No PR"
        }"#,
    )
    .unwrap();

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "prd", "--answers"])
        .arg(&answers)
        .assert()
        .success();

    let prd = repo.path().join(".codex1/missions/alpha/PRD.md");
    assert!(fs::read_to_string(&prd).unwrap().contains("# Alpha PRD"));

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "prd", "--answers"])
        .arg(&answers)
        .assert()
        .failure()
        .stdout(predicate::str::contains("ARTIFACT_VALIDATION_ERROR"));
}

#[test]
fn collection_artifacts_get_unique_names_and_subplans_can_move() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("subplan.json");
    fs::write(
        &answers,
        r#"{
          "title": "First Slice",
          "goal": "Do the first slice",
          "linked_prd": "PRD.md",
          "linked_plan": "PLAN.md",
          "owner": "main",
          "scope": ["CLI"],
          "steps": ["write file"],
          "expected_proof": ["test"],
          "exit_criteria": ["done"]
        }"#,
    )
    .unwrap();

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "subplan", "--answers"])
        .arg(&answers)
        .assert()
        .success();
    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "subplan", "--answers"])
        .arg(&answers)
        .assert()
        .success();

    let first = repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/ready/0001-first-slice.md");
    let second = repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/ready/0002-first-slice.md");
    assert!(first.is_file());
    assert!(second.is_file());

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "subplan",
            "move",
            "--id",
            "0001-first-slice",
            "--to",
            "active",
        ])
        .assert()
        .success();
    assert!(repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/active/0001-first-slice.md")
        .is_file());
    assert!(second.is_file());
}

#[test]
fn inspect_is_inventory_only() {
    let repo = repo();
    init(&repo, "alpha");
    let output = bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "inspect"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for forbidden in [
        "next_action",
        "complete",
        "blocked",
        "review_passed",
        "close_ready",
        "replan_required",
        "task_status",
    ] {
        assert!(!text.contains(forbidden), "{forbidden} leaked into inspect");
    }
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["ok"], true);
    assert!(value["data"]["artifacts"].is_object());
}

#[test]
fn loop_state_and_ralph_block_only_for_explicit_active_loop() {
    let repo = repo();
    init(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");

    let allow = json_output_with_stdin(
        bin()
            .args(["--repo-root"])
            .arg(repo.path())
            .args(["ralph", "stop-hook"]),
        format!(r#"{{"cwd":"{}"}}"#, mission_dir.display()),
    );
    assert_eq!(allow["decision"], "approve");

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "loop",
            "start",
            "--mode",
            "autopilot",
            "--message",
            "Continue the mission.",
        ])
        .assert()
        .success();

    let block = json_output_with_stdin(
        bin()
            .args(["--repo-root"])
            .arg(repo.path())
            .args(["ralph", "stop-hook"]),
        format!(r#"{{"cwd":"{}"}}"#, mission_dir.display()),
    );
    assert_eq!(block["decision"], "block");
    assert!(block["reason"]
        .as_str()
        .unwrap()
        .contains("Continue the mission."));

    let allow_active_hook = json_output_with_stdin(
        bin()
            .args(["--repo-root"])
            .arg(repo.path())
            .args(["ralph", "stop-hook"]),
        format!(
            r#"{{"cwd":"{}","stop_hook_active":true}}"#,
            mission_dir.display()
        ),
    );
    assert_eq!(allow_active_hook["decision"], "approve");
}
