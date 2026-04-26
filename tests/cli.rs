use std::fs;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

fn bin() -> Command {
    let mut command = Command::cargo_bin("codex1").unwrap();
    let base = std::env::temp_dir().join(format!("codex1-test-home-{}", std::process::id()));
    let home = base.join("home");
    let codex_home = base.join("codex-home");
    let codex1_home = base.join("codex1-home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&codex_home).unwrap();
    fs::create_dir_all(&codex1_home).unwrap();
    command
        .env("HOME", home)
        .env("CODEX_HOME", codex_home)
        .env("CODEX1_HOME", codex1_home);
    command
}

fn repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    dir
}

fn setup_env(command: &mut Command, home: &TempDir) {
    let user_home = home.path().join("home");
    let codex_home = home.path().join("codex-home");
    let codex1_home = home.path().join("codex1-home");
    fs::create_dir_all(&user_home).unwrap();
    fs::create_dir_all(&codex_home).unwrap();
    fs::create_dir_all(&codex1_home).unwrap();
    command
        .env("HOME", user_home)
        .env("CODEX_HOME", codex_home)
        .env("CODEX1_HOME", codex1_home);
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

fn events_path(repo: &TempDir, mission: &str) -> std::path::PathBuf {
    repo.path()
        .join(".codex1/missions")
        .join(mission)
        .join(".codex1/events.jsonl")
}

fn read_events(repo: &TempDir, mission: &str) -> Vec<Value> {
    fs::read_to_string(events_path(repo, mission))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn event_log_text(repo: &TempDir, mission: &str) -> String {
    fs::read_to_string(events_path(repo, mission)).unwrap()
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
    let descriptors = value["data"]["artifacts"].as_array().unwrap();
    assert!(descriptors
        .iter()
        .any(|descriptor| descriptor["kind"] == "loop-state"));
    assert!(descriptors
        .iter()
        .any(|descriptor| descriptor["kind"] == "receipts"));
    assert!(repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/ready")
        .is_dir());
}

#[test]
fn init_appends_mission_initialized_event() {
    let repo = repo();
    init(&repo, "alpha");

    let events = read_events(&repo, "alpha");
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event["version"], 1);
    assert!(event["timestamp"].as_str().unwrap().contains('T'));
    assert_eq!(event["mission_id"], "alpha");
    assert_eq!(event["command"], "init");
    assert_eq!(event["kind"], "mission_initialized");
    assert_eq!(event["result"], "success");
    assert!(event["duration_ms"].as_u64().is_some());
    assert_eq!(event["metadata"], serde_json::json!({}));
    assert!(event.get("sequence").is_none());
    assert!(event.get("argv").is_none());
    assert!(event.get("stdout").is_none());
    assert!(event.get("stderr").is_none());
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
fn interactive_json_interview_requires_answers_file() {
    let repo = repo();
    let output = bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "prd"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "ARGUMENT_ERROR");
    assert!(!repo.path().join(".codex1/missions/alpha").exists());
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
fn leading_hyphen_mission_id_is_rejected() {
    let repo = repo();
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
fn setup_status_reports_activation_only() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let output = command
        .args(["--json", "setup", "status", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for forbidden in [
        "next_action",
        "task_status",
        "review_passed",
        "proof_sufficient",
        "close_ready",
        "prd_satisfied",
    ] {
        assert!(
            !text.contains(forbidden),
            "{forbidden} leaked into setup status"
        );
    }
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["status"]["effective_active"], false);
    assert_eq!(
        value["data"]["status"]["anti_oracle"],
        "setup status reports activation/config only"
    );
}

#[test]
fn setup_status_requires_parseable_hook_config_before_reporting_active() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );
    fs::write(
        home.path().join("codex-home/config.toml"),
        "# codex1-managed-ralph-start\n[[hooks.Stop]\n# codex1-managed-ralph-end\n",
    )
    .unwrap();

    let mut status = bin();
    setup_env(&mut status, &home);
    let value = json_output(
        status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["status"]["global_hook_installed"], false);
    assert_eq!(value["data"]["status"]["effective_active"], false);
}

#[test]
fn setup_status_requires_valid_bundle_before_reporting_active() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );
    fs::write(repo.path().join(".codex1/setup-bundle.json"), "{not json").unwrap();

    let mut status = bin();
    setup_env(&mut status, &home);
    let value = json_output(
        status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["status"]["repo_bundle_materialized"], false);
    assert_eq!(value["data"]["status"]["effective_active"], false);
}

#[test]
fn setup_install_default_enables_only_target_repo_with_backups_and_bundle() {
    let repo = repo();
    let other_repo = crate::repo();
    let home = tempfile::tempdir().unwrap();
    let codex_home = home.path().join("codex-home");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join("config.toml"), "model = \"gpt-test\"\n").unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    let value = json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["activation_mode"], "allowlist");

    let codex_config = fs::read_to_string(codex_home.join("config.toml")).unwrap();
    assert!(codex_config.contains("model = \"gpt-test\""));
    assert!(codex_config.contains("codex1-managed-ralph-start"));
    assert!(codex_config.contains("ralph stop-hook"));

    let codex1_config = fs::read_to_string(home.path().join("codex1-home/config.toml")).unwrap();
    assert!(codex1_config.contains("mode = \"allowlist\""));
    assert!(codex1_config.contains(&repo.path().canonicalize().unwrap().display().to_string()));
    assert!(!codex1_config.contains(
        &other_repo
            .path()
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    ));

    assert!(repo.path().join(".agents/skills/codex1/SKILL.md").is_file());
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".codex1/setup-bundle.json").is_file());
    assert!(home
        .path()
        .join("codex1-home/backups/manifest.json")
        .is_file());

    let mut status = bin();
    setup_env(&mut status, &home);
    let active = json_output(
        status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(active["data"]["status"]["effective_active"], true);

    let mut other_status = bin();
    setup_env(&mut other_status, &home);
    let inactive = json_output(
        other_status
            .args(["--json", "setup", "status", "--repo"])
            .arg(other_repo.path()),
    );
    assert_eq!(inactive["data"]["status"]["repo_policy_enabled"], false);
}

#[test]
fn setup_commands_honor_global_repo_root_flag() {
    let repo = repo();
    let other_repo = crate::repo();
    let home = tempfile::tempdir().unwrap();
    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    assert!(repo.path().join(".agents/skills/codex1/SKILL.md").is_file());
    assert!(!other_repo
        .path()
        .join(".agents/skills/codex1/SKILL.md")
        .exists());

    let codex1_config = fs::read_to_string(home.path().join("codex1-home/config.toml")).unwrap();
    assert!(codex1_config.contains(&repo.path().canonicalize().unwrap().display().to_string()));
    assert!(!codex1_config.contains(
        &other_repo
            .path()
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    ));
}

#[test]
fn setup_install_mode_all_overrides_disabled_repo_entry() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );
    let mut disable = bin();
    setup_env(&mut disable, &home);
    json_output(
        disable
            .args(["--json", "setup", "disable", "--repo"])
            .arg(repo.path()),
    );

    let mut all = bin();
    setup_env(&mut all, &home);
    json_output(
        all.args(["--json", "setup", "install", "--mode", "all", "--repo"])
            .arg(repo.path()),
    );
    assert!(repo.path().join(".agents/skills/codex1/SKILL.md").is_file());

    let mut status = bin();
    setup_env(&mut status, &home);
    let value = json_output(
        status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["status"]["activation_mode"], "all");
    assert_eq!(value["data"]["status"]["repo_policy_enabled"], true);
}

#[test]
fn setup_install_mode_all_records_materialized_repo_for_off_cleanup() {
    let repo = repo();
    let other_repo = crate::repo();
    let home = tempfile::tempdir().unwrap();

    let mut install_one = bin();
    setup_env(&mut install_one, &home);
    json_output(
        install_one
            .args(["--json", "setup", "install", "--mode", "all", "--repo"])
            .arg(repo.path()),
    );

    let mut install_other = bin();
    setup_env(&mut install_other, &home);
    json_output(
        install_other
            .args(["--json", "setup", "install", "--mode", "all", "--repo"])
            .arg(other_repo.path()),
    );
    assert!(other_repo
        .path()
        .join(".agents/skills/codex1/SKILL.md")
        .exists());

    let mut off = bin();
    setup_env(&mut off, &home);
    json_output(
        off.args(["--json", "setup", "install", "--mode", "off", "--repo"])
            .arg(repo.path()),
    );

    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!other_repo
        .path()
        .join(".agents/skills/codex1/SKILL.md")
        .exists());
}

#[test]
fn setup_disable_overrides_all_mode_activation() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut all = bin();
    setup_env(&mut all, &home);
    json_output(
        all.args(["--json", "setup", "install", "--mode", "all", "--repo"])
            .arg(repo.path()),
    );

    let mut disable = bin();
    setup_env(&mut disable, &home);
    json_output(
        disable
            .args(["--json", "setup", "disable", "--repo"])
            .arg(repo.path()),
    );

    let mut status = bin();
    setup_env(&mut status, &home);
    let value = json_output(
        status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["status"]["activation_mode"], "denylist");
    assert_eq!(value["data"]["status"]["repo_policy_enabled"], false);
    assert_eq!(value["data"]["status"]["effective_active"], false);
}

#[test]
fn setup_install_does_not_leave_global_hook_when_policy_write_fails() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut install = bin();
    setup_env(&mut install, &home);
    let codex1_home = home.path().join("codex1-home");
    fs::remove_dir_all(&codex1_home).unwrap();
    fs::write(&codex1_home, "not a directory").unwrap();

    let output = install
        .args(["--json", "setup", "install", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!home.path().join("codex-home/config.toml").exists());
    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!repo.path().join("AGENTS.md").exists());
}

#[test]
fn setup_dry_run_does_not_write_files() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let value = json_output(
        command
            .args(["--json", "setup", "install", "--dry-run", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["plan"]["dry_run"], true);
    assert!(!home.path().join("codex-home/config.toml").exists());
    assert!(!home.path().join("codex1-home/config.toml").exists());
    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
}

#[test]
fn setup_enable_dry_run_bootstraps_same_plan_as_real_enable() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let value = json_output(
        command
            .args(["--json", "setup", "enable", "--dry-run", "--repo"])
            .arg(repo.path()),
    );
    let writes = value["data"]["plan"]["writes"].as_array().unwrap();
    assert!(writes
        .iter()
        .any(|path| path.as_str().unwrap().ends_with("codex-home/config.toml")));
    assert!(writes
        .iter()
        .any(|path| path.as_str().unwrap().ends_with("codex1-home/config.toml")));
    assert!(!home.path().join("codex-home/config.toml").exists());
}

#[test]
fn setup_install_off_does_not_materialize_repo_bundle() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let value = json_output(
        command
            .args(["--json", "setup", "install", "--mode", "off", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["activation_mode"], "off");
    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!repo.path().join("AGENTS.md").exists());
}

#[test]
fn setup_install_off_removes_bundles_for_known_policy_repos() {
    let repo = repo();
    let other_repo = crate::repo();
    let home = tempfile::tempdir().unwrap();

    let mut install_one = bin();
    setup_env(&mut install_one, &home);
    json_output(
        install_one
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );

    let mut enable_other = bin();
    setup_env(&mut enable_other, &home);
    json_output(
        enable_other
            .args(["--json", "setup", "enable", "--repo"])
            .arg(other_repo.path()),
    );
    assert!(repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(other_repo
        .path()
        .join(".agents/skills/codex1/SKILL.md")
        .exists());

    let mut off = bin();
    setup_env(&mut off, &home);
    json_output(
        off.args(["--json", "setup", "install", "--mode", "off", "--repo"])
            .arg(repo.path()),
    );

    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!other_repo
        .path()
        .join(".agents/skills/codex1/SKILL.md")
        .exists());
}

#[test]
fn setup_project_install_rejects_mode_off() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let output = command
        .args([
            "--json", "setup", "install", "--scope", "project", "--mode", "off", "--repo",
        ])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_ARGUMENT_ERROR");
    assert!(!repo.path().join(".codex/config.toml").exists());
    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
}

#[cfg(unix)]
#[test]
fn setup_project_install_rejects_symlinked_project_config() {
    let repo = repo();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "model = \"outside\"\n").unwrap();
    fs::create_dir_all(repo.path().join(".codex")).unwrap();
    symlink(outside.path(), repo.path().join(".codex/config.toml")).unwrap();

    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let output = command
        .args(["--json", "setup", "install", "--scope", "project", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_CONFIG_WRITE_ERROR");
    assert_eq!(
        fs::read_to_string(outside.path()).unwrap(),
        "model = \"outside\"\n"
    );
}

#[test]
fn setup_project_install_rejects_malformed_config_before_bundle_write() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex")).unwrap();
    fs::write(repo.path().join(".codex/config.toml"), "model = [nope\n").unwrap();

    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let output = command
        .args(["--json", "setup", "install", "--scope", "project", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_CONFIG_PARSE_ERROR");
    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!repo.path().join("AGENTS.md").exists());
}

#[test]
fn setup_project_install_preflights_bundle_before_hook_write() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "user-authored skill that says codex1-managed",
    )
    .unwrap();

    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let output = command
        .args(["--json", "setup", "install", "--scope", "project", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_BUNDLE_ERROR");
    assert!(!repo.path().join(".codex/config.toml").exists());
    assert_eq!(
        fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap(),
        "user-authored skill that says codex1-managed"
    );
}

#[test]
fn setup_doctor_treats_project_hook_as_installed_hook() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--scope", "project", "--repo"])
            .arg(repo.path()),
    );

    let mut doctor = bin();
    setup_env(&mut doctor, &home);
    let value = json_output(
        doctor
            .args(["--json", "setup", "doctor", "--repo"])
            .arg(repo.path()),
    );
    let check = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "global_hook_installed")
        .unwrap();
    assert_eq!(check["ok"], true);
}

#[test]
fn setup_mutation_preserves_malformed_backup_manifest() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let manifest = home.path().join("codex1-home/backups/manifest.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(&manifest, "{not json").unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    let output = install
        .args(["--json", "setup", "install", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_BACKUP_ERROR");
    assert_eq!(fs::read_to_string(&manifest).unwrap(), "{not json");
}

#[test]
fn setup_backups_list_reports_malformed_manifest() {
    let home = tempfile::tempdir().unwrap();
    let manifest = home.path().join("codex1-home/backups/manifest.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(&manifest, "{not json").unwrap();

    let mut list = bin();
    setup_env(&mut list, &home);
    let output = list
        .args(["--json", "setup", "backups", "list"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_BACKUP_ERROR");
}

#[test]
fn setup_doctor_reports_malformed_backup_manifest() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let manifest = home.path().join("codex1-home/backups/manifest.json");
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    fs::write(&manifest, "{not json").unwrap();

    let mut doctor = bin();
    setup_env(&mut doctor, &home);
    let value = json_output(
        doctor
            .args(["--json", "setup", "doctor", "--repo"])
            .arg(repo.path()),
    );
    let check = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "backup_manifest_parseable")
        .unwrap();
    assert_eq!(check["ok"], false);
}

#[test]
fn setup_disable_removes_bundle_without_deleting_mission_artifacts() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let mission_prd = repo.path().join(".codex1/missions/alpha/PRD.md");
    fs::write(&mission_prd, "mission truth").unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );

    let mut disable = bin();
    setup_env(&mut disable, &home);
    let value = json_output(
        disable
            .args(["--json", "setup", "disable", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["ok"], true);
    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(mission_prd.is_file());
}

#[test]
fn setup_disable_removes_managed_bundle_files_when_marker_is_missing() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );
    fs::remove_file(repo.path().join(".codex1/setup-bundle.json")).unwrap();

    let mut disable = bin();
    setup_env(&mut disable, &home);
    json_output(
        disable
            .args(["--json", "setup", "disable", "--repo"])
            .arg(repo.path()),
    );

    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!repo.path().join("AGENTS.md").exists());
}

#[test]
fn setup_disable_rejects_tampered_bundle_marker_paths_outside_repo() {
    let repo = repo();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "codex1-managed outside").unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        format!(
            r#"{{
  "managed_by": "codex1-managed",
  "version": 1,
  "files": ["{}"]
}}"#,
            outside.path().display()
        ),
    )
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut disable = bin();
    setup_env(&mut disable, &home);
    let output = disable
        .args(["--json", "setup", "disable", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_BUNDLE_ERROR");
    assert!(outside.path().exists());
}

#[test]
fn setup_disable_rejects_tampered_marker_for_unmanaged_repo_file() {
    let repo = repo();
    let user_file = repo.path().join("notes.md");
    fs::write(&user_file, "user text mentioning codex1-managed").unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        r#"{
  "managed_by": "codex1-managed",
  "version": 1,
  "files": ["notes.md"]
}"#,
    )
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut disable = bin();
    setup_env(&mut disable, &home);
    let output = disable
        .args(["--json", "setup", "disable", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_BUNDLE_ERROR");
    assert!(user_file.exists());
}

#[cfg(unix)]
#[test]
fn setup_install_rejects_symlinked_repo_bundle_roots() {
    let repo = repo();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), repo.path().join(".agents")).unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let output = command
        .args(["--json", "setup", "install", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_BUNDLE_ERROR");
    assert!(!outside.path().join("skills/codex1/SKILL.md").exists());
}

#[test]
fn setup_install_rejects_existing_file_that_only_mentions_managed_marker() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(
        repo.path().join("AGENTS.md"),
        "human note mentioning codex1-managed in passing",
    )
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut command = bin();
    setup_env(&mut command, &home);
    let output = command
        .args(["--json", "setup", "install", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "SETUP_BUNDLE_ERROR");
    assert_eq!(
        fs::read_to_string(repo.path().join("AGENTS.md")).unwrap(),
        "human note mentioning codex1-managed in passing"
    );
}

#[test]
fn setup_enable_reinstalls_missing_global_hook_for_existing_policy() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );

    let mut uninstall = bin();
    setup_env(&mut uninstall, &home);
    json_output(
        uninstall
            .args([
                "--json",
                "setup",
                "uninstall",
                "--scope",
                "global",
                "--repo",
            ])
            .arg(repo.path()),
    );
    assert!(
        !fs::read_to_string(home.path().join("codex-home/config.toml"))
            .unwrap()
            .contains("codex1-managed-ralph-start")
    );

    let mut enable = bin();
    setup_env(&mut enable, &home);
    json_output(
        enable
            .args(["--json", "setup", "enable", "--repo"])
            .arg(repo.path()),
    );
    assert!(
        fs::read_to_string(home.path().join("codex-home/config.toml"))
            .unwrap()
            .contains("codex1-managed-ralph-start")
    );
}

#[test]
fn setup_backups_can_restore_existing_and_missing_config_states() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let codex_home = home.path().join("codex-home");
    fs::create_dir_all(&codex_home).unwrap();
    let config = codex_home.join("config.toml");
    fs::write(&config, "model = \"before\"\n").unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );

    let mut list = bin();
    setup_env(&mut list, &home);
    let backups = json_output(list.args(["--json", "setup", "backups", "list"]));
    let id = backups["data"]["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["target_path"] == config.display().to_string())
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut restore = bin();
    setup_env(&mut restore, &home);
    json_output(restore.args(["--json", "setup", "backups", "restore", &id, "--force"]));
    assert_eq!(fs::read_to_string(&config).unwrap(), "model = \"before\"\n");
}

#[test]
fn setup_doctor_reports_stale_managed_hook_executable() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let codex_home = home.path().join("codex-home");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(
        codex_home.join("config.toml"),
        r#"# codex1-managed-ralph-start
[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = "'/definitely/missing/codex1' ralph stop-hook --scope global"
timeout = 10
statusMessage = "Codex1 Ralph"
# codex1-managed-ralph-end
"#,
    )
    .unwrap();

    let mut doctor = bin();
    setup_env(&mut doctor, &home);
    let value = json_output(
        doctor
            .args(["--json", "setup", "doctor", "--repo"])
            .arg(repo.path()),
    );
    let check = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "managed_hook_executable")
        .unwrap();
    assert_eq!(check["ok"], false);
}

#[test]
fn setup_doctor_reports_stale_project_hook_executable() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo.path().join(".codex")).unwrap();
    fs::write(
        repo.path().join(".codex/config.toml"),
        r#"# codex1-managed-ralph-start
[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = "'/definitely/missing/codex1' ralph stop-hook --scope project"
timeout = 10
statusMessage = "Codex1 Ralph"
# codex1-managed-ralph-end
"#,
    )
    .unwrap();

    let mut doctor = bin();
    setup_env(&mut doctor, &home);
    let value = json_output(
        doctor
            .args(["--json", "setup", "doctor", "--repo"])
            .arg(repo.path()),
    );
    let check = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "managed_hook_executable")
        .unwrap();
    assert_eq!(check["ok"], false);
}

#[test]
fn ralph_obeys_setup_activation_policy_and_fails_open() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    init(&repo, "alpha");

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );

    let mut start = bin();
    setup_env(&mut start, &home);
    json_output(
        start
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
                "Keep going",
            ]),
    );

    let mut ralph_block = bin();
    setup_env(&mut ralph_block, &home);
    let blocked = json_output_with_stdin(
        ralph_block
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "ralph", "stop-hook"]),
        "{}".to_string(),
    );
    assert_eq!(blocked["decision"], "block");

    let mut disable = bin();
    setup_env(&mut disable, &home);
    json_output(
        disable
            .args(["--json", "setup", "disable", "--repo"])
            .arg(repo.path()),
    );
    let mut ralph_disabled = bin();
    setup_env(&mut ralph_disabled, &home);
    let allowed = json_output_with_stdin(
        ralph_disabled
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "ralph", "stop-hook"]),
        "{}".to_string(),
    );
    assert!(allowed.get("decision").is_none());

    fs::write(
        home.path().join("codex1-home/config.toml"),
        "mode = [nope\n",
    )
    .unwrap();
    let mut ralph_malformed = bin();
    setup_env(&mut ralph_malformed, &home);
    let malformed_allowed = json_output_with_stdin(
        ralph_malformed
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "ralph", "stop-hook"]),
        "{}".to_string(),
    );
    assert!(malformed_allowed.get("decision").is_none());
}

#[test]
fn global_all_and_denylist_modes_scan_repos_without_materialized_bundles() {
    for mode in ["all", "denylist"] {
        let setup_repo = repo();
        let scanned_repo = crate::repo();
        let home = tempfile::tempdir().unwrap();
        init(&scanned_repo, "alpha");

        let mut install = bin();
        setup_env(&mut install, &home);
        json_output(
            install
                .args(["--json", "setup", "install", "--mode", mode, "--repo"])
                .arg(setup_repo.path()),
        );
        assert!(!scanned_repo
            .path()
            .join(".codex1/setup-bundle.json")
            .exists());

        let mut start = bin();
        setup_env(&mut start, &home);
        json_output(
            start
                .args(["--json", "--repo-root"])
                .arg(scanned_repo.path())
                .args([
                    "--mission",
                    "alpha",
                    "loop",
                    "start",
                    "--mode",
                    "autopilot",
                    "--message",
                    "Keep going",
                ]),
        );

        let mut ralph = bin();
        setup_env(&mut ralph, &home);
        let value = json_output_with_stdin(
            ralph
                .args(["--json", "--repo-root"])
                .arg(scanned_repo.path())
                .args(["--mission", "alpha", "ralph", "stop-hook"]),
            "{}".to_string(),
        );
        assert_eq!(value["decision"], "block", "mode={mode}");

        let mut status = bin();
        setup_env(&mut status, &home);
        let status_value = json_output(
            status
                .args(["--json", "setup", "status", "--repo"])
                .arg(scanned_repo.path()),
        );
        assert_eq!(
            status_value["data"]["status"]["effective_active"], true,
            "mode={mode}"
        );
    }
}

#[test]
fn project_scoped_ralph_requires_materialized_bundle() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    init(&repo, "alpha");

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--scope", "project", "--repo"])
            .arg(repo.path()),
    );
    fs::remove_file(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap();

    let mut start = bin();
    setup_env(&mut start, &home);
    json_output(
        start
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
                "Keep going",
            ]),
    );

    let mut ralph = bin();
    setup_env(&mut ralph, &home);
    let value = json_output_with_stdin(
        ralph
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args([
                "--mission",
                "alpha",
                "ralph",
                "stop-hook",
                "--scope",
                "project",
            ]),
        "{}".to_string(),
    );
    assert!(value.get("decision").is_none());
}

#[test]
fn project_scoped_ralph_rejects_tampered_bundle_marker() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    init(&repo, "alpha");

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--scope", "project", "--repo"])
            .arg(repo.path()),
    );
    fs::write(repo.path().join(".codex1/setup-bundle.json"), "{not json").unwrap();

    let mut start = bin();
    setup_env(&mut start, &home);
    json_output(
        start
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
                "Keep going",
            ]),
    );

    let mut ralph = bin();
    setup_env(&mut ralph, &home);
    let value = json_output_with_stdin(
        ralph
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args([
                "--mission",
                "alpha",
                "ralph",
                "stop-hook",
                "--scope",
                "project",
            ]),
        "{}".to_string(),
    );
    assert!(value.get("decision").is_none());
}

#[test]
fn project_scope_ralph_ignores_global_disable_after_migration() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    init(&repo, "alpha");

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );

    let mut migrate = bin();
    setup_env(&mut migrate, &home);
    json_output(
        migrate
            .args(["--json", "setup", "migrate", "--to", "project", "--repo"])
            .arg(repo.path()),
    );

    let mut start = bin();
    setup_env(&mut start, &home);
    json_output(
        start
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
                "Keep going",
            ]),
    );

    let mut global_hook = bin();
    setup_env(&mut global_hook, &home);
    let global_allowed = json_output_with_stdin(
        global_hook
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args([
                "--mission",
                "alpha",
                "ralph",
                "stop-hook",
                "--scope",
                "global",
            ]),
        "{}".to_string(),
    );
    assert!(global_allowed.get("decision").is_none());

    let mut project_hook = bin();
    setup_env(&mut project_hook, &home);
    let project_blocked = json_output_with_stdin(
        project_hook
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args([
                "--mission",
                "alpha",
                "ralph",
                "stop-hook",
                "--scope",
                "project",
            ]),
        "{}".to_string(),
    );
    assert_eq!(project_blocked["decision"], "block");
}

#[test]
fn migrate_to_global_does_not_enable_global_hook_before_later_failures() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut project_install = bin();
    setup_env(&mut project_install, &home);
    json_output(
        project_install
            .args(["--json", "setup", "install", "--scope", "project", "--repo"])
            .arg(repo.path()),
    );
    fs::write(repo.path().join(".codex/config.toml"), "model = [nope\n").unwrap();

    let mut migrate = bin();
    setup_env(&mut migrate, &home);
    let output = migrate
        .args(["--json", "setup", "migrate", "--to", "global", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    let global_config = home.path().join("codex-home/config.toml");
    assert!(
        !global_config.exists()
            || !fs::read_to_string(global_config)
                .unwrap()
                .contains("codex1-managed-ralph-start")
    );
}

#[test]
fn migrate_to_project_keeps_global_policy_when_project_hook_install_fails() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );
    fs::create_dir_all(repo.path().join(".codex")).unwrap();
    fs::write(repo.path().join(".codex/config.toml"), "model = [nope\n").unwrap();

    let mut migrate = bin();
    setup_env(&mut migrate, &home);
    let output = migrate
        .args(["--json", "setup", "migrate", "--to", "project", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    let mut status = bin();
    setup_env(&mut status, &home);
    let value = json_output(
        status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["status"]["repo_policy_enabled"], true);
    assert_eq!(value["data"]["status"]["effective_active"], true);
}

#[test]
fn migrate_to_global_keeps_project_hook_when_global_hook_install_fails() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--scope", "project", "--repo"])
            .arg(repo.path()),
    );
    let global_config = home.path().join("codex-home/config.toml");
    fs::write(&global_config, "model = [nope\n").unwrap();

    let mut migrate = bin();
    setup_env(&mut migrate, &home);
    let output = migrate
        .args(["--json", "setup", "migrate", "--to", "global", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    assert!(fs::read_to_string(repo.path().join(".codex/config.toml"))
        .unwrap()
        .contains("codex1-managed-ralph-start"));
    let mut status = bin();
    setup_env(&mut status, &home);
    let value = json_output(
        status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(value["data"]["status"]["effective_active"], true);
}

#[test]
fn setup_project_migrate_uninstall_and_enable_flows_are_reversible() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();

    let mut project_install = bin();
    setup_env(&mut project_install, &home);
    json_output(
        project_install
            .args(["--json", "setup", "install", "--scope", "project", "--repo"])
            .arg(repo.path()),
    );
    let project_config = repo.path().join(".codex/config.toml");
    assert!(fs::read_to_string(&project_config)
        .unwrap()
        .contains("codex1-managed-ralph-start"));
    assert!(!home.path().join("codex-home/config.toml").exists());

    let mut project_status = bin();
    setup_env(&mut project_status, &home);
    let status = json_output(
        project_status
            .args(["--json", "setup", "status", "--repo"])
            .arg(repo.path()),
    );
    assert_eq!(status["data"]["status"]["effective_active"], true);
    assert_eq!(status["data"]["status"]["project_trust_caveat"], true);

    let mut migrate_global = bin();
    setup_env(&mut migrate_global, &home);
    json_output(
        migrate_global
            .args(["--json", "setup", "migrate", "--to", "global", "--repo"])
            .arg(repo.path()),
    );
    assert!(!fs::read_to_string(&project_config)
        .unwrap_or_default()
        .contains("codex1-managed-ralph-start"));
    assert!(
        fs::read_to_string(home.path().join("codex-home/config.toml"))
            .unwrap()
            .contains("codex1-managed-ralph-start")
    );

    let mut uninstall_global = bin();
    setup_env(&mut uninstall_global, &home);
    json_output(
        uninstall_global
            .args([
                "--json",
                "setup",
                "uninstall",
                "--scope",
                "global",
                "--repo",
            ])
            .arg(repo.path()),
    );
    assert!(
        !fs::read_to_string(home.path().join("codex-home/config.toml"))
            .unwrap()
            .contains("codex1-managed-ralph-start")
    );

    let mut disable = bin();
    setup_env(&mut disable, &home);
    json_output(
        disable
            .args(["--json", "setup", "disable", "--repo"])
            .arg(repo.path()),
    );
    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());

    let mut enable = bin();
    setup_env(&mut enable, &home);
    json_output(
        enable
            .args(["--json", "setup", "enable", "--repo"])
            .arg(repo.path()),
    );
    assert!(repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(
        fs::read_to_string(home.path().join("codex1-home/config.toml"))
            .unwrap()
            .contains("enabled = true")
    );
}

#[test]
fn setup_restore_can_restore_a_previously_missing_file_to_absence() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let codex1_config = home.path().join("codex1-home/config.toml");

    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );
    assert!(codex1_config.exists());

    let mut list = bin();
    setup_env(&mut list, &home);
    let backups = json_output(list.args(["--json", "setup", "backups", "list"]));
    let id = backups["data"]["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["target_path"] == codex1_config.display().to_string()
                && record["existed"] == false
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut restore = bin();
    setup_env(&mut restore, &home);
    json_output(restore.args(["--json", "setup", "backups", "restore", &id, "--force"]));
    assert!(!codex1_config.exists());
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
fn artifact_interview_appends_private_metadata_only_event() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("answers-private.json");
    fs::write(
        &answers,
        r#"{
          "title": "Secret Alpha PRD",
          "original_request": "payload text that must stay out of events",
          "interpreted_destination": "A deterministic alpha",
          "success_criteria": ["generated markdown body marker"],
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

    let events = read_events(&repo, "alpha");
    let event = events.last().unwrap();
    assert_eq!(event["command"], "interview");
    assert_eq!(event["kind"], "artifact_written");
    assert_eq!(event["result"], "success");
    assert_eq!(event["metadata"]["artifact_kind"], "prd");
    assert_eq!(event["metadata"]["template_version"], 1);
    assert_eq!(event["metadata"]["overwrite"], false);
    assert_eq!(event["metadata"]["path"], "PRD.md");

    let text = event_log_text(&repo, "alpha");
    assert!(!text.contains("payload text that must stay out of events"));
    assert!(!text.contains("generated markdown body marker"));
    assert!(!text.contains("answers-private.json"));
    assert!(!text.contains(repo.path().to_str().unwrap()));
}

#[test]
fn successful_mutations_append_forensic_events_without_messages() {
    let repo = repo();
    init(&repo, "alpha");
    let subplan_answers = repo.path().join("subplan-event.json");
    fs::write(
        &subplan_answers,
        r#"{
          "title": "Move Me",
          "goal": "Create a subplan",
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
        .arg(&subplan_answers)
        .assert()
        .success();

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "subplan",
            "move",
            "--id",
            "0001-move-me",
            "--to",
            "active",
        ])
        .assert()
        .success();
    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "receipt",
            "append",
            "--message",
            "receipt text must not leak",
        ])
        .assert()
        .success();
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
            "loop message must not leak",
        ])
        .assert()
        .success();
    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "loop",
            "pause",
            "--reason",
            "pause reason must not leak",
        ])
        .assert()
        .success();
    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "loop", "resume"])
        .assert()
        .success();
    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "loop",
            "stop",
            "--reason",
            "stop reason must not leak",
        ])
        .assert()
        .success();

    let events = read_events(&repo, "alpha");
    let kinds: Vec<_> = events
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"subplan_moved"));
    assert!(kinds.contains(&"receipt_appended"));
    assert!(kinds.contains(&"loop_started"));
    assert!(kinds.contains(&"loop_paused"));
    assert!(kinds.contains(&"loop_resumed"));
    assert!(kinds.contains(&"loop_stopped"));

    let moved = events
        .iter()
        .find(|event| event["kind"] == "subplan_moved")
        .unwrap();
    assert_eq!(
        moved["metadata"]["from_path"],
        "SUBPLANS/ready/0001-move-me.md"
    );
    assert_eq!(
        moved["metadata"]["to_path"],
        "SUBPLANS/active/0001-move-me.md"
    );
    assert_eq!(moved["metadata"]["from_lifecycle"], "ready");
    assert_eq!(moved["metadata"]["to_lifecycle"], "active");

    let receipt = events
        .iter()
        .find(|event| event["kind"] == "receipt_appended")
        .unwrap();
    assert_eq!(
        receipt["metadata"]["path"],
        ".codex1/receipts/receipts.jsonl"
    );

    let started = events
        .iter()
        .find(|event| event["kind"] == "loop_started")
        .unwrap();
    assert_eq!(started["metadata"]["mode"], "autopilot");
    assert_eq!(started["metadata"]["message_present"], true);

    let paused = events
        .iter()
        .find(|event| event["kind"] == "loop_paused")
        .unwrap();
    assert_eq!(paused["metadata"]["reason_present"], true);

    let text = event_log_text(&repo, "alpha");
    for private in [
        "receipt text must not leak",
        "loop message must not leak",
        "pause reason must not leak",
        "stop reason must not leak",
        repo.path().to_str().unwrap(),
    ] {
        assert!(!text.contains(private));
    }
}

#[test]
fn event_append_failure_warns_without_failing_primary_mutation() {
    let json_repo = repo();
    init(&json_repo, "alpha");
    let event_path = events_path(&json_repo, "alpha");
    fs::remove_file(&event_path).unwrap();
    fs::create_dir(&event_path).unwrap();
    let answers = json_repo.path().join("warning-answers.json");
    fs::write(
        &answers,
        r#"{
          "title": "Warning PRD",
          "original_request": "Build alpha",
          "interpreted_destination": "A deterministic alpha",
          "success_criteria": ["artifact exists"],
          "proof_expectations": ["cargo test"],
          "pr_intent": "No PR"
        }"#,
    )
    .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(json_repo.path())
            .args(["--mission", "alpha", "interview", "prd", "--answers"])
            .arg(&answers),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(value["warnings"][0]["code"], "EVENT_LOG_APPEND_FAILED");
    assert!(json_repo
        .path()
        .join(".codex1/missions/alpha/PRD.md")
        .is_file());

    let human = repo();
    init(&human, "alpha");
    let event_path = events_path(&human, "alpha");
    fs::remove_file(&event_path).unwrap();
    fs::create_dir(&event_path).unwrap();
    let output = bin()
        .args(["--repo-root"])
        .arg(human.path())
        .args([
            "--mission",
            "alpha",
            "receipt",
            "append",
            "--message",
            "human warning primary mutation",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Appended optional receipt"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("EVENT_LOG_APPEND_FAILED"));
    assert!(human
        .path()
        .join(".codex1/missions/alpha/.codex1/receipts/receipts.jsonl")
        .is_file());
}

#[test]
fn read_only_commands_do_not_append_events() {
    let repo = repo();
    init(&repo, "alpha");
    let before = event_log_text(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "inspect"])
        .assert()
        .success();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["template", "list"])
        .assert()
        .success();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["template", "show", "prd"])
        .assert()
        .success();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["doctor"])
        .assert()
        .success();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "loop", "status"])
        .assert()
        .failure();
    let _ = json_output_with_stdin(
        bin()
            .args(["--repo-root"])
            .arg(repo.path())
            .args(["ralph", "stop-hook"]),
        format!(r#"{{"cwd":"{}"}}"#, mission_dir.display()),
    );

    assert_eq!(event_log_text(&repo, "alpha"), before);
}

#[test]
fn inspect_reports_event_count_and_malformed_event_warnings_only() {
    let repo = repo();
    init(&repo, "alpha");
    fs::OpenOptions::new()
        .append(true)
        .open(events_path(&repo, "alpha"))
        .unwrap()
        .write_all(b"not-json\n{\"version\":99,\"kind\":\"future\"}\n{\"version\":1}\n[]\n")
        .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "inspect"]),
    );
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["artifacts"]["events"], 1);
    let warnings = value["data"]["mechanical_warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "MALFORMED_EVENT_LOG_LINE"));
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "UNSUPPORTED_EVENT_LOG_VERSION"));
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "MISSING_EVENT_LOG_KIND"));
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "NON_OBJECT_EVENT_LOG_LINE"));

    let text = serde_json::to_string(&value).unwrap();
    for forbidden in [
        "last_event",
        "last_activity",
        "activity_status",
        "progress",
        "ready",
        "complete",
        "next_action",
    ] {
        assert!(!text.contains(forbidden), "{forbidden} leaked into inspect");
    }
}

#[cfg(unix)]
#[test]
fn inspect_does_not_scan_events_through_symlinked_meta_directory() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    fs::write(
        external.path().join("events.jsonl"),
        r#"{"version":1,"kind":"mission_initialized"}"#,
    )
    .unwrap();
    let meta_dir = repo.path().join(".codex1/missions/alpha/.codex1");
    fs::remove_dir_all(&meta_dir).unwrap();
    symlink(external.path(), &meta_dir).unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "inspect"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["artifacts"]["events"], 0);
    let warnings = value["data"]["mechanical_warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "SYMLINKED_PATH"));
}

#[cfg(unix)]
#[test]
fn inspect_does_not_scan_events_through_in_mission_symlinked_meta_directory() {
    let repo = repo();
    init(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");
    let linked_meta = mission_dir.join("linked-meta");
    fs::create_dir(&linked_meta).unwrap();
    fs::write(
        linked_meta.join("events.jsonl"),
        r#"{"version":1,"kind":"mission_initialized"}"#,
    )
    .unwrap();
    let meta_dir = mission_dir.join(".codex1");
    fs::remove_dir_all(&meta_dir).unwrap();
    symlink(&linked_meta, &meta_dir).unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "inspect"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["artifacts"]["events"], 0);
    let warnings = value["data"]["mechanical_warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "SYMLINKED_PATH"));
}

#[test]
fn inspect_treats_invalid_utf8_event_lines_as_malformed() {
    let repo = repo();
    init(&repo, "alpha");
    fs::OpenOptions::new()
        .append(true)
        .open(events_path(&repo, "alpha"))
        .unwrap()
        .write_all(b"{\"version\":1,\"kind\":\"artifact_written\"}\n\xff\n")
        .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "inspect"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["artifacts"]["events"], 2);
    let warnings = value["data"]["mechanical_warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|warning| warning["code"] == "MALFORMED_EVENT_LOG_LINE"
            && warning["detail"] == "events.jsonl line 3"));
}

#[cfg(unix)]
#[test]
fn event_append_rejects_fifo_without_hanging_primary_mutation() {
    let repo = repo();
    init(&repo, "alpha");
    let event_path = events_path(&repo, "alpha");
    fs::remove_file(&event_path).unwrap();
    let status = Command::new("mkfifo").arg(&event_path).status().unwrap();
    assert!(status.success());

    let mut child = bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "receipt",
            "append",
            "--message",
            "fifo should not hang",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(2) {
        if child.try_wait().unwrap().is_some() {
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "stdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["ok"], true);
            assert_eq!(value["warnings"][0]["code"], "EVENT_LOG_APPEND_FAILED");
            assert!(repo
                .path()
                .join(".codex1/missions/alpha/.codex1/receipts/receipts.jsonl")
                .is_file());
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    let _ = child.wait();
    panic!("event append blocked on FIFO");
}

#[cfg(unix)]
#[test]
fn unsafe_path_failures_do_not_append_failure_events() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let before = event_log_text(&repo, "alpha");
    fs::write(external.path().join("PRD.md"), "# external\n").unwrap();
    symlink(
        external.path().join("PRD.md"),
        repo.path().join(".codex1/missions/alpha/PRD.md"),
    )
    .unwrap();
    let answers = repo.path().join("unsafe-path-answers.json");
    fs::write(
        &answers,
        r#"{
          "title": "Unsafe PRD",
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
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));

    assert_eq!(event_log_text(&repo, "alpha"), before);
}

#[test]
fn malformed_event_logs_do_not_block_future_appends() {
    let repo = repo();
    init(&repo, "alpha");
    fs::OpenOptions::new()
        .append(true)
        .open(events_path(&repo, "alpha"))
        .unwrap()
        .write_all(b"not-json\n")
        .unwrap();

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "receipt",
            "append",
            "--message",
            "append after malformed log",
        ])
        .assert()
        .success();

    let text = event_log_text(&repo, "alpha");
    assert!(text.contains("not-json"));
    assert!(text.lines().any(|line| serde_json::from_str::<Value>(line)
        .ok()
        .is_some_and(|event| event["kind"] == "receipt_appended")));
}

#[test]
fn safe_mutation_failures_append_failure_events_without_hiding_errors() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("failure-answers.json");
    fs::write(
        &answers,
        r#"{
          "title": "Failure PRD",
          "original_request": "failure payload must not leak",
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

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "prd", "--answers"])
        .arg(&answers)
        .assert()
        .failure()
        .stdout(predicate::str::contains("ARTIFACT_VALIDATION_ERROR"));
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "subplan",
            "move",
            "--id",
            "missing",
            "--to",
            "active",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("ARTIFACT_VALIDATION_ERROR"));
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "loop", "pause"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("IO_ERROR"));

    let events = read_events(&repo, "alpha");
    for (kind, code) in [
        ("artifact_write_failed", "ARTIFACT_VALIDATION_ERROR"),
        ("subplan_move_failed", "ARTIFACT_VALIDATION_ERROR"),
        ("loop_pause_failed", "IO_ERROR"),
    ] {
        let event = events
            .iter()
            .find(|event| event["kind"] == kind)
            .unwrap_or_else(|| panic!("{kind} was not logged"));
        assert_eq!(event["result"], "error");
        assert_eq!(event["metadata"]["error_code"], code);
    }

    assert!(!event_log_text(&repo, "alpha").contains("failure payload must not leak"));
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
fn ralph_blocks_from_normal_repo_cwd_for_single_active_loop() {
    let repo = repo();
    init(&repo, "alpha");

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
            "Continue from repo cwd.",
        ])
        .assert()
        .success();

    let mut command = bin();
    command
        .current_dir(repo.path())
        .args(["ralph", "stop-hook"]);
    let block = json_output_with_stdin(
        &mut command,
        format!(r#"{{"cwd":"{}"}}"#, repo.path().display()),
    );
    assert_eq!(block["decision"], "block");
    let reason = block["reason"].as_str().unwrap();
    assert!(reason.contains("Continue from repo cwd."));
    assert!(reason.contains("codex1 --mission=alpha loop pause"));
    assert!(reason.contains("codex1 --mission=alpha loop stop"));
}

#[test]
fn ralph_blocks_with_deterministic_guidance_for_multiple_active_loops() {
    let repo = repo();
    init(&repo, "beta");
    init(&repo, "alpha");

    for (mission, message) in [("beta", "Continue beta."), ("alpha", "Continue alpha.")] {
        bin()
            .args(["--repo-root"])
            .arg(repo.path())
            .args([
                "--mission",
                mission,
                "loop",
                "start",
                "--mode",
                "autopilot",
                "--message",
                message,
            ])
            .assert()
            .success();
    }

    let mut command = bin();
    command
        .current_dir(repo.path())
        .args(["ralph", "stop-hook"]);
    let block = json_output_with_stdin(
        &mut command,
        format!(r#"{{"cwd":"{}"}}"#, repo.path().display()),
    );
    assert_eq!(block["decision"], "block");
    let reason = block["reason"].as_str().unwrap();
    assert!(reason.contains("Multiple active Codex1 loops exist"));
    assert!(reason.find("- alpha:").unwrap() < reason.find("- beta:").unwrap());
    assert!(reason.contains("codex1 --mission=alpha loop pause"));
    assert!(reason.contains("codex1 --mission=beta loop stop"));
}

#[cfg(unix)]
#[test]
fn ralph_fails_open_for_symlinked_loop_state() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");
    fs::write(
        external.path().join("LOOP.json"),
        r#"{
          "version": 1,
          "active": true,
          "paused": false,
          "mode": "autopilot",
          "message": "External loop should not block.",
          "pause_command": "codex1 --mission=alpha loop pause --reason <reason>",
          "stop_command": "codex1 --mission=alpha loop stop --reason <reason>",
          "updated_at": "2026-04-26T00:00:00Z"
        }"#,
    )
    .unwrap();
    symlink(
        external.path().join("LOOP.json"),
        mission_dir.join(".codex1/LOOP.json"),
    )
    .unwrap();

    let allow = json_output_with_stdin(
        bin().args(["--repo-root"]).arg(repo.path()).args([
            "--mission",
            "alpha",
            "ralph",
            "stop-hook",
        ]),
        "{}".to_string(),
    );
    assert!(allow.as_object().unwrap().get("decision").is_none());
}

#[cfg(unix)]
#[test]
fn symlinked_mission_root_is_rejected_before_reads() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    fs::create_dir_all(external.path().join(".codex1")).unwrap();
    fs::write(
        external.path().join(".codex1/LOOP.json"),
        r#"{
          "version": 1,
          "active": true,
          "paused": false,
          "mode": "autopilot",
          "message": "External mission should not be trusted.",
          "pause_command": "codex1 --mission=alpha loop pause --reason <reason>",
          "stop_command": "codex1 --mission=alpha loop stop --reason <reason>",
          "updated_at": "2026-04-26T00:00:00Z"
        }"#,
    )
    .unwrap();
    let mission_dir = repo.path().join(".codex1/missions/alpha");
    fs::remove_dir_all(&mission_dir).unwrap();
    symlink(external.path(), &mission_dir).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "loop", "status"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));

    let allow = json_output_with_stdin(
        bin().args(["--repo-root"]).arg(repo.path()).args([
            "--mission",
            "alpha",
            "ralph",
            "stop-hook",
        ]),
        "{}".to_string(),
    );
    assert!(allow.as_object().unwrap().get("decision").is_none());
}

#[cfg(unix)]
#[test]
fn symlinked_missions_directory_is_not_scanned_by_ralph() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    fs::create_dir_all(external.path().join("alpha/.codex1")).unwrap();
    fs::write(
        external.path().join("alpha/.codex1/LOOP.json"),
        r#"{
          "version": 1,
          "active": true,
          "paused": false,
          "mode": "autopilot",
          "message": "External missions directory should not be scanned.",
          "pause_command": "codex1 --mission=alpha loop pause --reason <reason>",
          "stop_command": "codex1 --mission=alpha loop stop --reason <reason>",
          "updated_at": "2026-04-26T00:00:00Z"
        }"#,
    )
    .unwrap();
    let missions_dir = repo.path().join(".codex1/missions");
    fs::remove_dir_all(&missions_dir).unwrap();
    symlink(external.path(), &missions_dir).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "loop", "status"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));

    let mut command = bin();
    command
        .current_dir(repo.path())
        .args(["ralph", "stop-hook"]);
    let allow = json_output_with_stdin(
        &mut command,
        format!(r#"{{"cwd":"{}"}}"#, repo.path().display()),
    );
    assert!(allow.as_object().unwrap().get("decision").is_none());
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

#[test]
fn loop_status_does_not_create_missing_mission() {
    let repo = repo();
    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "typo", "loop", "status"])
        .assert()
        .failure();

    assert!(!repo.path().join(".codex1/missions/typo").exists());
}

#[test]
fn doctor_runs_installed_command_and_loop_smoke() {
    let value = json_output(bin().args(["--json", "doctor"]));
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["installed_command"]["json_error_envelope"],
        true
    );
    assert_eq!(value["data"]["loop_ralph_smoke"]["blocked"], true);
}

#[test]
fn doctor_ralph_smoke_ignores_setup_allowlist_policy() {
    let repo = repo();
    let home = tempfile::tempdir().unwrap();
    let mut install = bin();
    setup_env(&mut install, &home);
    json_output(
        install
            .args(["--json", "setup", "install", "--repo"])
            .arg(repo.path()),
    );

    let mut doctor = bin();
    setup_env(&mut doctor, &home);
    let value = json_output(doctor.args(["--json", "doctor"]));
    assert_eq!(value["data"]["loop_ralph_smoke"]["blocked"], true);
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

#[test]
fn inspect_warns_on_malformed_collection_frontmatter() {
    let repo = repo();
    init(&repo, "alpha");
    fs::write(
        repo.path().join(".codex1/missions/alpha/SPECS/0001-bad.md"),
        "# Missing Frontmatter\n",
    )
    .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "inspect"]),
    );
    assert_eq!(value["data"]["artifacts"]["specs"], 1);
    let warnings = value["data"]["mechanical_warnings"].as_array().unwrap();
    assert!(warnings.iter().any(|warning| {
        warning["code"] == "MALFORMED_FRONTMATTER"
            && warning["detail"]
                .as_str()
                .unwrap()
                .contains("SPECS/0001-bad.md")
    }));
}

#[test]
fn inspect_warns_on_unterminated_collection_frontmatter() {
    let repo = repo();
    init(&repo, "alpha");
    fs::write(
        repo.path()
            .join(".codex1/missions/alpha/SPECS/0001-unterminated.md"),
        "---\ntemplate_version: 1\n# Missing Close\n",
    )
    .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["--mission", "alpha", "inspect"]),
    );
    let warnings = value["data"]["mechanical_warnings"].as_array().unwrap();
    assert!(warnings.iter().any(|warning| {
        warning["code"] == "MALFORMED_FRONTMATTER"
            && warning["detail"]
                .as_str()
                .unwrap()
                .contains("SPECS/0001-unterminated.md")
    }));
}

#[test]
fn subplan_ids_stay_unique_across_lifecycle_folders() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("subplan.json");
    fs::write(
        &answers,
        r#"{
          "title": "Repeat Slice",
          "goal": "Do the repeated slice",
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
        .args([
            "--mission",
            "alpha",
            "subplan",
            "move",
            "--id",
            "0001-repeat-slice",
            "--to",
            "active",
        ])
        .assert()
        .success();
    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "subplan", "--answers"])
        .arg(&answers)
        .assert()
        .success();

    assert!(repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/active/0001-repeat-slice.md")
        .is_file());
    assert!(repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/ready/0002-repeat-slice.md")
        .is_file());
    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args([
            "--mission",
            "alpha",
            "subplan",
            "move",
            "--id",
            "0002-repeat-slice",
            "--to",
            "done",
        ])
        .assert()
        .success();
}
