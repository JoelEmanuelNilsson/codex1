use std::fs;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

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
    assert!(allow.as_object().unwrap().get("decision").is_none());

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
    assert!(allow_active_hook
        .as_object()
        .unwrap()
        .get("decision")
        .is_none());
}

#[test]
fn ralph_resolves_repo_root_from_hook_cwd_when_invoked_elsewhere() {
    let repo = repo();
    let outside = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");

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
            "Continue from cwd.",
        ])
        .assert()
        .success();

    let mut command = bin();
    command
        .current_dir(outside.path())
        .args(["ralph", "stop-hook"]);
    let block = json_output_with_stdin(
        &mut command,
        format!(r#"{{"cwd":"{}"}}"#, mission_dir.display()),
    );
    assert_eq!(block["decision"], "block");
    assert!(block["reason"]
        .as_str()
        .unwrap()
        .contains("Continue from cwd."));
}

#[test]
fn repeatable_answers_file_sections_must_be_arrays() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("bad-repeatable.json");
    fs::write(
        &answers,
        r#"{
          "title": "Bad PRD",
          "original_request": "Build alpha",
          "interpreted_destination": "A deterministic alpha",
          "success_criteria": "artifact exists",
          "proof_expectations": ["cargo test"],
          "pr_intent": "No PR"
        }"#,
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "prd", "--answers"])
        .arg(&answers)
        .assert()
        .failure()
        .stdout(predicate::str::contains("must be a list of strings"));
}

#[cfg(unix)]
#[test]
fn loop_state_write_rejects_symlinked_meta_directory() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");
    let meta_dir = mission_dir.join(".codex1");
    fs::remove_dir_all(&meta_dir).unwrap();
    symlink(external.path(), &meta_dir).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "loop",
            "start",
            "--mode",
            "autopilot",
            "--message",
            "Do not write outside.",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));

    assert!(!external.path().join("LOOP.json").exists());
}

#[cfg(unix)]
#[test]
fn receipt_append_rejects_symlinked_receipts_directory() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let receipts_dir = repo.path().join(".codex1/missions/alpha/.codex1/receipts");
    fs::remove_dir_all(&receipts_dir).unwrap();
    symlink(external.path(), &receipts_dir).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "receipt",
            "append",
            "--message",
            "do not append outside",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));

    assert!(!external.path().join("receipts.jsonl").exists());
}

#[cfg(unix)]
#[test]
fn subplan_move_rejects_symlinked_lifecycle_directory() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    fs::write(external.path().join("0001-external.md"), "# External\n").unwrap();
    let ready_dir = repo.path().join(".codex1/missions/alpha/SUBPLANS/ready");
    fs::remove_dir_all(&ready_dir).unwrap();
    symlink(external.path(), &ready_dir).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "subplan",
            "move",
            "--id",
            "0001-external",
            "--to",
            "active",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));

    assert!(external.path().join("0001-external.md").is_file());
}

#[cfg(unix)]
#[test]
fn writes_reject_dangling_symlink_targets() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");

    let prd_answers = repo.path().join("prd.json");
    fs::write(
        &prd_answers,
        r#"{
          "title": "Dangling PRD",
          "original_request": "Build alpha",
          "interpreted_destination": "A deterministic alpha",
          "success_criteria": ["artifact exists"],
          "proof_expectations": ["cargo test"],
          "pr_intent": "No PR"
        }"#,
    )
    .unwrap();
    let outside_prd = external.path().join("outside-prd.md");
    symlink(&outside_prd, mission_dir.join("PRD.md")).unwrap();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "prd", "--answers"])
        .arg(&prd_answers)
        .assert()
        .failure()
        .stdout(predicate::str::contains("target must not be a symlink"));
    assert!(!outside_prd.exists());

    let outside_loop = external.path().join("outside-loop.json");
    symlink(&outside_loop, mission_dir.join(".codex1/LOOP.json")).unwrap();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "loop",
            "start",
            "--mode",
            "autopilot",
            "--message",
            "do not follow",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("target must not be a symlink"));
    assert!(!outside_loop.exists());

    let outside_receipt = external.path().join("outside-receipts.jsonl");
    symlink(
        &outside_receipt,
        mission_dir.join(".codex1/receipts/receipts.jsonl"),
    )
    .unwrap();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "receipt",
            "append",
            "--message",
            "do not follow",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("target must not be a symlink"));
    assert!(!outside_receipt.exists());
}

#[cfg(unix)]
#[test]
fn inspect_skips_symlinked_inventory_paths() {
    let repo = repo();
    let external_collection = tempfile::tempdir().unwrap();
    let external_subplan = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    fs::write(
        external_collection.path().join("outside-research.md"),
        "# Outside\n",
    )
    .unwrap();
    fs::write(
        external_subplan.path().join("outside-subplan.md"),
        "# Outside\n",
    )
    .unwrap();

    let mission_dir = repo.path().join(".codex1/missions/alpha");
    let research_dir = mission_dir.join("RESEARCH");
    fs::remove_dir_all(&research_dir).unwrap();
    symlink(external_collection.path(), &research_dir).unwrap();
    symlink(
        external_subplan.path(),
        mission_dir.join("SUBPLANS/ready/external"),
    )
    .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "inspect"]),
    );
    assert_eq!(value["data"]["artifacts"]["research"], 0);
    assert_eq!(value["data"]["artifacts"]["subplans"], 0);
    let warnings = value["data"]["mechanical_warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "SYMLINKED_PATH"));
}

#[test]
fn answers_file_rejects_duplicate_json_keys() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("duplicate-keys.json");
    fs::write(
        &answers,
        r#"{
          "title": "First",
          "title": "Second",
          "original_request": "Build alpha",
          "interpreted_destination": "A deterministic alpha",
          "success_criteria": ["artifact exists"],
          "proof_expectations": ["cargo test"],
          "pr_intent": "No PR"
        }"#,
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "prd", "--answers"])
        .arg(&answers)
        .assert()
        .failure()
        .stdout(predicate::str::contains("duplicate JSON key: title"));
}

#[test]
fn review_template_accepts_structured_finding_fields() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("review.json");
    fs::write(
        &answers,
        r#"{
          "title": "Review",
          "target": "src/main.rs",
          "reviewer_role": "reviewer",
          "overall_assessment": "Needs one fix",
          "confidence": "high",
          "findings": ["Reject symlink targets"],
          "finding_priorities": ["P1"],
          "finding_confidences": ["high"],
          "finding_locations": ["src/paths.rs:225"],
          "finding_rationales": ["Dangling symlinks can escape containment"],
          "recommended_followup": ["Patch path helper"]
        }"#,
    )
    .unwrap();

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "review", "--answers"])
        .arg(&answers)
        .assert()
        .success();

    let rendered = fs::read_to_string(
        repo.path()
            .join(".codex1/missions/alpha/REVIEWS/0001-review.md"),
    )
    .unwrap();
    assert!(rendered.contains("<!-- codex1-section: finding_priorities -->"));
    assert!(rendered.contains("<!-- codex1-section: finding_locations -->"));
    assert!(rendered.contains("<!-- codex1-section: finding_rationales -->"));
}
