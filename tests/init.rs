mod common;

use std::fs;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;

use common::*;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[test]
fn init_creates_only_the_path_safe_mission_layout() {
    let repo = repo();
    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "init"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["mission_id"], "alpha");
    let descriptors = value["data"]["artifacts"].as_array().unwrap();
    let kinds: Vec<_> = descriptors
        .iter()
        .map(|descriptor| descriptor["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        vec![
            "prd",
            "plan",
            "research-plan",
            "goal-brief",
            "research",
            "spec",
            "subplan",
            "adr",
            "review",
            "triage",
            "proof",
            "closeout",
        ]
    );

    let mission = repo.path().join(".codex1/missions/alpha");
    for dir in [
        "RESEARCH",
        "SPECS",
        "ADRS",
        "REVIEWS",
        "TRIAGE",
        "PROOFS",
        "SUBPLANS/ready",
        "SUBPLANS/active",
        "SUBPLANS/done",
        "SUBPLANS/paused",
        "SUBPLANS/superseded",
    ] {
        assert!(mission.join(dir).is_dir(), "{dir}");
    }
    assert!(!mission.join(".codex1").exists());
    assert!(!mission.join("GOAL_BRIEF.md").exists());
}

#[test]
fn json_argument_errors_are_wrapped() {
    bin()
        .args(["--json", "init"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("ARGUMENT_ERROR"));
}

#[test]
fn unsafe_mission_ids_are_rejected() {
    let repo = repo();
    for mission in ["../bad", ".hidden", "bad/id", "bad..id"] {
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", mission, "init"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("MISSION_PATH_ERROR"));
    }

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .arg("--mission=-bad")
        .arg("init")
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));
}

#[test]
fn nested_cargo_manifest_uses_outer_git_repo_root() {
    let repo = repo();
    let nested = repo.path().join("crates/inner");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("Cargo.toml"), "[package]\nname = \"inner\"\n").unwrap();

    bin()
        .current_dir(&nested)
        .args(["--mission", "alpha", "init"])
        .assert()
        .success();

    assert!(repo.path().join(".codex1/missions/alpha").is_dir());
    assert!(!nested.join(".codex1/missions/alpha").exists());
}

#[test]
fn unknown_commands_fail_through_the_argument_parser() {
    let repo = repo();
    let output = bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "not-a-command"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "ARGUMENT_ERROR");
    assert!(!repo.path().join(".codex1/missions/alpha").exists());
}

#[test]
fn help_only_advertises_init_and_setup() {
    let output = bin().arg("--help").output().unwrap();

    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let commands: Vec<_> = text
        .lines()
        .skip_while(|line| line.trim() != "Commands:")
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert_eq!(commands, vec!["init", "setup", "help"]);
}

#[cfg(unix)]
#[test]
fn symlinked_mission_path_components_are_rejected() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    symlink(external.path(), repo.path().join(".codex1/missions")).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "init"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));

    assert!(!external.path().join("alpha").exists());
}
