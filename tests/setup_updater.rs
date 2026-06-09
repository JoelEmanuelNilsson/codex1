use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

fn updater_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".agents/skills/update-codex1-setups/scripts/update-codex1-setups.sh")
}

fn codex1_bin() -> PathBuf {
    assert_cmd::cargo::cargo_bin("codex1")
}

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?}\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_init(repo: &Path) {
    fs::create_dir_all(repo).unwrap();
    git(repo, &["init"]);
}

fn tracked_setup_repo(search_root: &TempDir, name: &str) -> PathBuf {
    let repo = search_root.path().join(name);
    git_init(&repo);
    Command::new(codex1_bin())
        .args(["--repo-root"])
        .arg(&repo)
        .args(["setup", "install"])
        .assert()
        .success();
    git(&repo, &["add", ".codex1/setup-bundle.json"]);
    repo
}

#[test]
fn updater_script_has_valid_bash_syntax() {
    Command::new("bash")
        .arg("-n")
        .arg(updater_script())
        .assert()
        .success();
}

#[test]
fn updater_dry_run_discovers_only_tracked_setup_markers() {
    let search_root = tempfile::tempdir().unwrap();
    let tracked = tracked_setup_repo(&search_root, "tracked");
    let tracked = fs::canonicalize(tracked).unwrap();

    let untracked = search_root.path().join("untracked");
    git_init(&untracked);
    fs::create_dir_all(untracked.join(".codex1")).unwrap();
    fs::write(
        untracked.join(".codex1/setup-bundle.json"),
        include_str!("../.codex1/setup-bundle.json"),
    )
    .unwrap();
    let untracked = fs::canonicalize(untracked).unwrap();

    Command::new("bash")
        .arg(updater_script())
        .arg("--dry-run")
        .env("CODEX1_SETUP_SEARCH_ROOT", search_root.path())
        .env("CODEX1_SETUP_BIN", codex1_bin())
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "== {} ==",
            tracked.display()
        )))
        .stdout(predicate::str::contains("dry-run complete for 1 repos"))
        .stdout(predicate::str::contains(untracked.display().to_string()).not());
}

#[test]
fn updater_dry_run_reports_when_no_valid_setup_repos_are_found() {
    let search_root = tempfile::tempdir().unwrap();

    Command::new("bash")
        .arg(updater_script())
        .arg("--dry-run")
        .env("CODEX1_SETUP_SEARCH_ROOT", search_root.path())
        .env("CODEX1_SETUP_BIN", codex1_bin())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "no valid Codex1 setup repos found",
        ));
}

#[test]
fn updater_apply_reports_dirty_source_checkout_before_building() {
    let source_root = tempfile::tempdir().unwrap();
    let search_root = tempfile::tempdir().unwrap();
    git_init(source_root.path());
    fs::write(source_root.path().join("dirty.txt"), "dirty\n").unwrap();

    Command::new("bash")
        .arg(updater_script())
        .arg("--apply")
        .env("CODEX1_SETUP_SOURCE_ROOT", source_root.path())
        .env("CODEX1_SETUP_SEARCH_ROOT", search_root.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing apply: source checkout has uncommitted changes",
        ))
        .stderr(predicate::str::contains("?? dirty.txt"));
}
