use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn minimal_supported_config_toml() -> &'static str {
    "model = \"gpt-5.4\"\nreview_model = \"gpt-5.4-mini\"\nmodel_reasoning_effort = \"high\"\n[features]\ncodex_hooks = true\n[agents]\nmax_threads = 16\nmax_depth = 1\n[codex1_orchestration]\nmodel = \"gpt-5.4\"\nreasoning_effort = \"high\"\n[codex1_review]\nmodel = \"gpt-5.4-mini\"\nreasoning_effort = \"high\"\n[codex1_fast_parallel]\nmodel = \"gpt-5.3-codex-spark\"\nreasoning_effort = \"high\"\n[codex1_hard_coding]\nmodel = \"gpt-5.3-codex\"\nreasoning_effort = \"xhigh\"\n"
}

fn write_minimal_supported_config(repo_root: &Path) {
    let codex_dir = repo_root.join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(
        codex_dir.join("config.toml"),
        minimal_supported_config_toml(),
    )
    .expect("write config.toml");
    fs::write(
        codex_dir.join("hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "codex1 internal stop-hook"
          }
        ]
      }
    ]
  }
}"#,
    )
    .expect("write hooks.json");
}

fn write_agents_scaffold(repo_root: &Path) {
    fs::write(
        repo_root.join("AGENTS.md"),
        "<!-- codex1:begin -->\n## Codex1\n### Workflow Stance\n- Use the native Codex skills surface for `clarify`, `plan`, `execute`, `review-loop`, and `autopilot`.\n- Keep mission truth in visible repo artifacts instead of hidden chat state.\n- Replan stays internal unless the repo truth explicitly says otherwise.\n\n### Quality Bar\n- Work is complete only when the locked outcome, proof, review, and closeout contracts are all satisfied.\n- Review is mandatory before mission completion.\n- Hold the repo to production-grade changes with explicit validation and review-clean closeout.\n\n### Repo Commands\n- Build: cargo build -p codex1\n- Test: cargo test -p codex1\n- Lint or format: cargo fmt --all --check\n\n### Artifact Conventions\n- Mission packages live under `PLANS/<mission-id>/`.\n- `OUTCOME-LOCK.md` is canonical for destination truth.\n- `PROGRAM-BLUEPRINT.md` is canonical for route truth.\n- `specs/*/SPEC.md` is canonical for one bounded execution slice.\n<!-- codex1:end -->\n",
    )
    .expect("write AGENTS scaffold");
}

fn copy_source_skills(repo_root: &Path) {
    copy_source_skills_to(&repo_root.join(".codex/skills"));
}

fn copy_source_skills_to(destination_root: &Path) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.codex/skills")
        .canonicalize()
        .expect("canonical source skill root");
    for entry in walkdir::WalkDir::new(&source) {
        let entry = entry.expect("walk source skills");
        let relative = entry
            .path()
            .strip_prefix(&source)
            .expect("relative skill path");
        let destination = destination_root.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination).expect("create destination dir");
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).expect("create parent dir");
            }
            fs::copy(entry.path(), &destination).expect("copy skill file");
        }
    }
}

fn source_skill_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.codex/skills")
        .canonicalize()
        .expect("canonical source skill root")
}

fn run_qualify(repo_root: &Path) -> std::process::Output {
    let home = prepare_trusted_home(repo_root);
    run_qualify_with_home(repo_root, home.path())
}

fn run_qualify_with_home(repo_root: &Path, home_root: &Path) -> std::process::Output {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    Command::new(binary.clone())
        .arg("qualify-codex")
        .arg("--json")
        .arg("--live=false")
        .arg("--repo-root")
        .arg(repo_root)
        .env("HOME", home_root)
        .env("CODEX1_QUALIFY_EXECUTABLE", binary)
        .output()
        .expect("run codex1 qualify-codex")
}

fn run_doctor(repo_root: &Path, home_root: &Path) -> std::process::Output {
    run_doctor_with_args(repo_root, home_root, &[])
}

fn run_doctor_with_args(
    repo_root: &Path,
    home_root: &Path,
    extra_args: &[&str],
) -> std::process::Output {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    Command::new(binary)
        .arg("doctor")
        .args(extra_args)
        .arg("--json")
        .arg("--repo-root")
        .arg(repo_root)
        .env("HOME", home_root)
        .output()
        .expect("run codex1 doctor")
}

fn run_command_with_home(
    binary: PathBuf,
    repo_root: &Path,
    home_root: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(binary)
        .args(args)
        .arg("--repo-root")
        .arg(repo_root)
        .arg("--json")
        .env("HOME", home_root)
        .output()
        .expect("run codex1 command")
}

fn run_command_in_repo_with_home(
    binary: PathBuf,
    repo_root: &Path,
    home_root: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(binary)
        .args(args)
        .arg("--json")
        .current_dir(repo_root)
        .env("HOME", home_root)
        .output()
        .expect("run codex1 command")
}

fn run_setup_with_codex_home(
    binary: PathBuf,
    cwd: &Path,
    home_root: &Path,
    codex_home: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(binary)
        .args(args)
        .arg("--json")
        .current_dir(cwd)
        .env("HOME", home_root)
        .env("CODEX_HOME", codex_home)
        .output()
        .expect("run codex1 setup")
}

fn run_with_codex_home(
    binary: PathBuf,
    cwd: &Path,
    home_root: &Path,
    codex_home: &Path,
    args: &[&str],
    json: bool,
) -> std::process::Output {
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", home_root)
        .env("CODEX_HOME", codex_home);
    if json {
        command.arg("--json");
    }
    command.output().expect("run codex1 command")
}

fn prepare_trusted_home(repo_root: &Path) -> TempDir {
    let home = TempDir::new().expect("temp home");
    let codex_dir = home.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create ~/.codex");
    let canonical_repo_root = fs::canonicalize(repo_root).expect("canonical repo root");
    fs::write(
        codex_dir.join("config.toml"),
        format!(
            "[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
            canonical_repo_root.display()
        ),
    )
    .expect("write trusted home config");
    home
}

fn prepare_trusted_home_with_hooks(repo_root: &Path, hooks_json: &str) -> TempDir {
    let home = prepare_trusted_home(repo_root);
    fs::write(home.path().join(".codex/hooks.json"), hooks_json).expect("write user hooks");
    home
}

fn parse_report(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should contain JSON")
}

#[test]
fn setup_does_not_create_project_local_codex_or_agents_files() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_in_repo_with_home(binary, repo.path(), home.path(), &["setup"]);
    assert!(
        setup.status.success(),
        "global setup boundary should succeed without project mutation: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    assert!(
        !repo.path().join(".codex").exists(),
        "codex1 setup must not create repo-local .codex"
    );
    assert!(
        !repo.path().join("AGENTS.md").exists(),
        "codex1 setup must not create repo-local AGENTS.md"
    );

    let report = parse_report(&setup);
    assert_eq!(report["scope"], "global");
    assert!(report["changed_paths"].as_array().is_some());
}

#[test]
fn setup_rejects_project_repo_root_flag() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["setup"]);
    assert!(
        !setup.status.success(),
        "global setup should reject project-scoped --repo-root"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&setup.stdout),
        String::from_utf8_lossy(&setup.stderr)
    );
    assert!(combined.contains("use codex1 init --repo-root"));
}

#[test]
fn setup_writes_global_codex_home_with_user_scope_backups() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = home.path().join("codex-home");
    fs::create_dir_all(&codex_home).expect("create codex home");
    fs::write(
        codex_home.join("config.toml"),
        "[features]\nvoice_transcription = true\n",
    )
    .expect("seed global config");
    fs::write(
        codex_home.join("hooks.json"),
        r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"python3 observe.py","codex1_observational":true}]}]}}"#,
    )
    .expect("seed global hooks");
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup =
        run_setup_with_codex_home(binary, repo.path(), home.path(), &codex_home, &["setup"]);
    assert!(
        setup.status.success(),
        "global setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    assert!(
        !repo.path().join(".codex").exists(),
        "global setup must not create project .codex"
    );
    assert!(
        !repo.path().join("AGENTS.md").exists(),
        "global setup must not create project AGENTS.md"
    );

    let config = fs::read_to_string(codex_home.join("config.toml")).expect("read config");
    assert!(config.contains("voice_transcription = true"));
    assert!(config.contains("codex_hooks = true"));
    let hooks = fs::read_to_string(codex_home.join("hooks.json")).expect("read hooks");
    assert!(hooks.contains("codex1 internal stop-hook"));
    assert!(hooks.contains("python3 observe.py"));
    assert!(codex_home.join("skills/clarify/SKILL.md").is_file());
    assert!(codex_home.join("skills/close/SKILL.md").is_file());

    let report = parse_report(&setup);
    assert_eq!(report["scope"], "global");
    assert_eq!(
        report["codex_home"],
        fs::canonicalize(&codex_home)
            .expect("canonical codex home")
            .display()
            .to_string()
    );
    assert!(report["backup_root"].as_str().is_some());
    let changed_paths = report["changed_paths"].as_array().expect("changed paths");
    assert!(
        changed_paths
            .iter()
            .any(|entry| entry["component"] == "user_config")
    );
    assert!(
        changed_paths
            .iter()
            .any(|entry| entry["component"] == "user_hooks")
    );
    assert!(changed_paths.iter().any(|entry| {
        entry["component"] == "skill_file"
            && entry["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("skills/clarify/SKILL.md"))
    }));
    assert!(changed_paths.iter().any(|entry| {
        entry["component"] == "skill_file"
            && entry["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("skills/close/SKILL.md"))
    }));
    let backup_id = report["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .find_map(|note| note.as_str()?.strip_prefix("backup id: "))
        .expect("backup id note");
    let manifest_path = home
        .path()
        .join(".codex1/backups")
        .join(backup_id)
        .join("manifest.json");
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read global setup manifest"))
            .expect("parse manifest");
    let paths = manifest["paths"].as_array().expect("manifest paths");
    assert!(paths.len() >= 10);
    assert!(paths.iter().all(|entry| entry["scope"] == "user"));
    assert!(paths.iter().all(|entry| entry["applied"] == true));
    for component in ["user_config", "user_hooks"] {
        let entry = paths
            .iter()
            .find(|entry| entry["component"] == component)
            .expect("manifest should include modified user config and hooks");
        assert!(
            entry["backup_path"]
                .as_str()
                .is_some_and(|path| Path::new(path).exists()),
            "{component} should have a backup copy"
        );
    }
}

#[test]
fn setup_human_output_lists_global_changed_paths() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = home.path().join("codex-home");
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_with_codex_home(
        binary,
        repo.path(),
        home.path(),
        &codex_home,
        &["setup"],
        false,
    );
    assert!(
        setup.status.success(),
        "global setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let stdout = String::from_utf8_lossy(&setup.stdout);
    assert!(stdout.contains("changed paths:"));
    let final_report = stdout
        .split_once("scope: global\n")
        .map(|(_, report)| report)
        .expect("human setup output should include a final global report");
    assert!(final_report.contains("changed paths:"));
    assert!(final_report.contains("config.toml (created, user_config)"));
    assert!(final_report.contains("hooks.json (created, user_hooks)"));
    assert!(final_report.contains("skills/clarify/SKILL.md (created, skill_file)"));
    assert!(final_report.contains("skills/close/SKILL.md (created, skill_file)"));
}

#[test]
fn setup_is_idempotent_for_global_codex_home() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = home.path().join("codex-home");
    fs::create_dir_all(&codex_home).expect("create codex home");
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let first = run_setup_with_codex_home(
        binary.clone(),
        repo.path(),
        home.path(),
        &codex_home,
        &["setup"],
    );
    assert!(
        first.status.success(),
        "initial global setup should succeed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let second =
        run_setup_with_codex_home(binary, repo.path(), home.path(), &codex_home, &["setup"]);
    assert!(
        second.status.success(),
        "second global setup should succeed: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let report = parse_report(&second);
    assert!(
        report["changed_paths"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "idempotent setup should report no changes"
    );
    assert!(
        report["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .any(|note| note == "global setup is already in the desired state")
    );
}

#[test]
fn setup_is_semantically_idempotent_for_global_codex_home() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = home.path().join("codex-home");
    fs::create_dir_all(&codex_home).expect("create codex home");
    fs::write(
        codex_home.join("config.toml"),
        "[features]\ncodex_hooks = true\n",
    )
    .expect("seed desired config");
    fs::write(
        codex_home.join("hooks.json"),
        r#"{"hooks":{"Stop":[{"hooks":[{"statusMessage":"Codex1 Ralph stop hook","command":"/Users/joel/codex1/target/debug/codex1 internal stop-hook","type":"command"}]}]}}"#,
    )
    .expect("seed minified desired hooks");
    copy_source_skills_to(&codex_home.join("skills"));
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup =
        run_setup_with_codex_home(binary, repo.path(), home.path(), &codex_home, &["setup"]);
    assert!(
        setup.status.success(),
        "semantic idempotent setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let report = parse_report(&setup);
    assert!(
        report["changed_paths"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "semantic desired config/hooks should not be rewritten"
    );
}

#[test]
fn init_allows_global_managed_stop_hook_without_project_hook_duplication() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = home.path().join(".codex");
    fs::create_dir_all(&codex_home).expect("create codex home");
    let canonical_repo = fs::canonicalize(repo.path()).expect("canonical repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    fs::write(
        codex_home.join("config.toml"),
        format!(
            "[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
            canonical_repo.display()
        ),
    )
    .expect("seed trusted global config");
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_setup_with_codex_home(
        binary.clone(),
        repo.path(),
        home.path(),
        &codex_home,
        &["setup"],
    );
    assert!(
        setup.status.success(),
        "global setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let init = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        init.status.success(),
        "project init should allow the global managed Stop hook: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(
        !repo.path().join(".codex/hooks.json").exists(),
        "project init must not add a second authoritative project Stop hook when global setup already installed one"
    );
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".codex/skills/clarify/SKILL.md").is_file());
}

#[test]
fn doctor_distinguishes_global_setup_from_project_init() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = home.path().join(".codex");
    fs::create_dir_all(&codex_home).expect("create codex home");
    let canonical_repo = fs::canonicalize(repo.path()).expect("canonical repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    fs::write(
        codex_home.join("config.toml"),
        format!(
            "[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
            canonical_repo.display()
        ),
    )
    .expect("seed trusted global config");
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_setup_with_codex_home(
        binary.clone(),
        repo.path(),
        home.path(),
        &codex_home,
        &["setup"],
    );
    assert!(
        setup.status.success(),
        "global setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let before_init = run_doctor(repo.path(), home.path());
    assert!(
        before_init.status.success(),
        "doctor should emit report after global setup: {}",
        String::from_utf8_lossy(&before_init.stderr)
    );
    let before = parse_report(&before_init);
    let before_findings = before["findings"].as_array().expect("findings");
    assert_finding_status(before_findings, "global_config", "pass");
    assert_finding_status(before_findings, "global_hooks", "pass");
    assert_finding_status(before_findings, "global_skill_surface", "pass");
    assert_finding_status(before_findings, "hooks_json", "pass");
    assert_finding_status(before_findings, "skill_surface", "fail");
    assert_eq!(before["supported"], false);

    let init = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        init.status.success(),
        "project init should succeed after global setup: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    let after_init = run_doctor(repo.path(), home.path());
    assert!(
        after_init.status.success(),
        "doctor should emit report after init: {}",
        String::from_utf8_lossy(&after_init.stderr)
    );
    let after = parse_report(&after_init);
    let after_findings = after["findings"].as_array().expect("findings");
    assert_finding_status(after_findings, "global_config", "pass");
    assert_finding_status(after_findings, "global_hooks", "pass");
    assert_finding_status(after_findings, "global_skill_surface", "pass");
    assert_finding_status(after_findings, "hooks_json", "pass");
    assert_finding_status(after_findings, "skill_surface", "pass");
    assert_finding_status(after_findings, "qualification", "warn");
    assert_eq!(after["supported"], false);
    assert!(
        !repo.path().join(".codex/hooks.json").exists(),
        "init should keep using the global managed Stop hook"
    );
}

fn assert_finding_status(findings: &[Value], check: &str, expected: &str) {
    let finding = findings
        .iter()
        .find(|finding| finding["check"] == check)
        .unwrap_or_else(|| panic!("missing doctor finding {check}"));
    assert_eq!(finding["status"], expected, "unexpected status for {check}");
}

#[test]
fn restore_accepts_global_setup_user_scope_backup() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = home.path().join("codex-home");
    fs::create_dir_all(&codex_home).expect("create codex home");
    fs::write(
        codex_home.join("config.toml"),
        "[features]\nvoice_transcription = true\n",
    )
    .expect("seed original config");
    fs::write(codex_home.join("hooks.json"), r#"{"hooks":{}}"#).expect("seed original hooks");
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_setup_with_codex_home(
        binary.clone(),
        repo.path(),
        home.path(),
        &codex_home,
        &["setup"],
    );
    assert!(
        setup.status.success(),
        "global setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let setup_report = parse_report(&setup);
    let backup_id = setup_report["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .find_map(|note| note.as_str()?.strip_prefix("backup id: "))
        .expect("backup id note");

    let restore = run_setup_with_codex_home(
        binary,
        repo.path(),
        home.path(),
        &codex_home,
        &["restore", "--backup-id", backup_id],
    );
    assert!(
        restore.status.success(),
        "restore should accept user-scope global setup backup: {}",
        String::from_utf8_lossy(&restore.stderr)
    );
    let restored_config =
        fs::read_to_string(codex_home.join("config.toml")).expect("read restored config");
    let restored_hooks =
        fs::read_to_string(codex_home.join("hooks.json")).expect("read restored hooks");
    assert!(restored_config.contains("voice_transcription = true"));
    assert!(!restored_config.contains("codex_hooks = true"));
    assert_eq!(restored_hooks, r#"{"hooks":{}}"#);
}

#[test]
fn restore_accepts_relative_codex_home_global_setup_backup() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let codex_home = repo.path().join("codex-home");
    fs::create_dir_all(&codex_home).expect("create relative codex home target");
    fs::write(
        codex_home.join("config.toml"),
        "[features]\nvoice_transcription = true\n",
    )
    .expect("seed original config");
    fs::write(codex_home.join("hooks.json"), r#"{"hooks":{}}"#).expect("seed original hooks");
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let relative_codex_home = Path::new("codex-home");

    let setup = run_setup_with_codex_home(
        binary.clone(),
        repo.path(),
        home.path(),
        relative_codex_home,
        &["setup"],
    );
    assert!(
        setup.status.success(),
        "global setup should succeed with relative CODEX_HOME: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let setup_report = parse_report(&setup);
    assert_eq!(
        setup_report["codex_home"],
        fs::canonicalize(&codex_home)
            .expect("canonical codex home")
            .display()
            .to_string()
    );
    let backup_id = setup_report["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .find_map(|note| note.as_str()?.strip_prefix("backup id: "))
        .expect("backup id note");

    let restore = run_setup_with_codex_home(
        binary,
        repo.path(),
        home.path(),
        relative_codex_home,
        &["restore", "--backup-id", backup_id],
    );
    assert!(
        restore.status.success(),
        "restore should accept relative CODEX_HOME global backup: {}",
        String::from_utf8_lossy(&restore.stderr)
    );
    let restored_config =
        fs::read_to_string(codex_home.join("config.toml")).expect("read restored config");
    let restored_hooks =
        fs::read_to_string(codex_home.join("hooks.json")).expect("read restored hooks");
    assert!(restored_config.contains("voice_transcription = true"));
    assert!(!restored_config.contains("codex_hooks = true"));
    assert_eq!(restored_hooks, r#"{"hooks":{}}"#);
    assert!(
        !codex_home.join("codex-home").exists(),
        "restore must not operate on a duplicated relative CODEX_HOME path"
    );
}

#[test]
fn restore_preserves_global_codex_home_root_after_deleting_created_files() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let relative_codex_home = Path::new("codex-home");
    let codex_home = repo.path().join(relative_codex_home);

    let setup = run_setup_with_codex_home(
        binary.clone(),
        repo.path(),
        home.path(),
        relative_codex_home,
        &["setup"],
    );
    assert!(
        setup.status.success(),
        "global setup should succeed with relative CODEX_HOME: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    assert!(codex_home.join("config.toml").is_file());
    assert!(codex_home.join("hooks.json").is_file());
    assert!(codex_home.join("skills/clarify/SKILL.md").is_file());
    let setup_report = parse_report(&setup);
    let backup_id = setup_report["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .find_map(|note| note.as_str()?.strip_prefix("backup id: "))
        .expect("backup id note");

    let restore = run_setup_with_codex_home(
        binary,
        repo.path(),
        home.path(),
        relative_codex_home,
        &["restore", "--backup-id", backup_id],
    );
    assert!(
        restore.status.success(),
        "restore should accept relative CODEX_HOME global backup: {}",
        String::from_utf8_lossy(&restore.stderr)
    );
    assert!(
        codex_home.is_dir(),
        "restore must not prune the CODEX_HOME root after deleting created files"
    );
    assert!(!codex_home.join("config.toml").exists());
    assert!(!codex_home.join("hooks.json").exists());
    assert!(!codex_home.join("skills/clarify/SKILL.md").exists());
    assert!(
        !codex_home.join("codex-home").exists(),
        "restore must not operate on a duplicated relative CODEX_HOME path"
    );
}

#[test]
fn uninstall_accepts_relative_codex_home_global_setup_backup() {
    let repo = TempDir::new().expect("temp repo");
    let home = TempDir::new().expect("temp home");
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let relative_codex_home = Path::new("codex-home");
    let codex_home = repo.path().join(relative_codex_home);

    let setup = run_setup_with_codex_home(
        binary.clone(),
        repo.path(),
        home.path(),
        relative_codex_home,
        &["setup"],
    );
    assert!(
        setup.status.success(),
        "global setup should succeed with relative CODEX_HOME: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    assert!(codex_home.join("config.toml").is_file());
    assert!(codex_home.join("hooks.json").is_file());
    assert!(codex_home.join("skills/clarify/SKILL.md").is_file());
    let setup_report = parse_report(&setup);
    let backup_id = setup_report["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .find_map(|note| note.as_str()?.strip_prefix("backup id: "))
        .expect("backup id note");

    let uninstall = run_setup_with_codex_home(
        binary,
        repo.path(),
        home.path(),
        relative_codex_home,
        &["uninstall", "--backup-id", backup_id],
    );
    assert!(
        uninstall.status.success(),
        "uninstall should accept relative CODEX_HOME global backup: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
    assert!(
        codex_home.is_dir(),
        "uninstall must not prune the CODEX_HOME root after deleting created files"
    );
    assert!(!codex_home.join("config.toml").exists());
    assert!(!codex_home.join("hooks.json").exists());
    assert!(!codex_home.join("skills/clarify/SKILL.md").exists());
    assert!(
        !codex_home.join("codex-home").exists(),
        "uninstall must not operate on a duplicated relative CODEX_HOME path"
    );
}

#[test]
fn qualify_reports_missing_project_config_surfaces() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let output = run_qualify(repo.path());
    assert!(
        !output.status.success(),
        "missing config should fail qualification"
    );

    let report = parse_report(&output);
    assert_eq!(report["schema_version"], "codex1.qualify.v1");
    assert_eq!(
        report["repo_root"],
        fs::canonicalize(repo.path())
            .expect("canonical repo path")
            .display()
            .to_string()
    );
    assert!(report["summary"]["failed"].as_u64().unwrap_or_default() >= 1);

    let gates = report["gates"].as_array().expect("gates array");
    assert!(
        gates
            .iter()
            .any(|gate| gate["gate"] == "project_config_present")
    );
    assert!(
        gates
            .iter()
            .any(|gate| gate["gate"] == "project_hooks_file_present")
    );

    let latest_path = Path::new(
        report["evidence"]["latest_path"]
            .as_str()
            .expect("latest path"),
    );
    assert!(
        latest_path.exists(),
        "latest qualification report should be written"
    );
}

#[test]
fn qualify_writes_latest_and_versioned_reports_on_successful_smoke_inputs() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    write_agents_scaffold(repo.path());
    copy_source_skills(repo.path());

    let output = run_qualify(repo.path());
    assert!(
        output.status.success(),
        "expected qualification to pass with minimal supported config; stderr was {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&output);
    assert_eq!(report["summary"]["failed"], 0);
    assert_eq!(report["summary"]["passed_all_required_gates"], false);

    let report_path = Path::new(
        report["evidence"]["report_path"]
            .as_str()
            .expect("report path"),
    );
    let latest_path = Path::new(
        report["evidence"]["latest_path"]
            .as_str()
            .expect("latest path"),
    );
    assert!(report_path.exists(), "versioned report should exist");
    assert!(latest_path.exists(), "latest report should exist");

    let latest_report: Value =
        serde_json::from_slice(&fs::read(latest_path).expect("read latest report"))
            .expect("parse latest report");
    assert_eq!(
        latest_report["qualification_id"],
        report["qualification_id"]
    );

    let gates = report["gates"].as_array().expect("gates array");
    let stop_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "project_stop_hook_authority")
        .expect("stop hook gate");
    assert_eq!(stop_gate["status"], "pass");
    let skill_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "project_skill_surface_valid")
        .expect("skill gate");
    assert_eq!(skill_gate["status"], "pass");
    let agents_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "project_agents_scaffold_present")
        .expect("agents gate");
    assert_eq!(agents_gate["status"], "pass");
    let runtime_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "runtime_backend_flow")
        .expect("runtime gate");
    assert_eq!(runtime_gate["status"], "pass");
    let reviewer_boundary_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "reviewer_capability_boundary")
        .expect("reviewer capability boundary gate");
    assert_eq!(reviewer_boundary_gate["status"], "pass");
    let delegated_review_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "delegated_review_authority")
        .expect("delegated review authority gate");
    assert_eq!(delegated_review_gate["status"], "pass");
    let waiting_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "waiting_stop_hook_flow")
        .expect("waiting gate");
    assert_eq!(waiting_gate["status"], "pass");
    let control_loop_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "control_loop_boundary")
        .expect("control-loop boundary gate");
    assert_eq!(control_loop_gate["status"], "pass");
    let normalization_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "helper_force_normalization_flow")
        .expect("force-normalization gate");
    assert_eq!(normalization_gate["status"], "pass");
    let partial_repair_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "helper_partial_install_repair_flow")
        .expect("partial-repair gate");
    assert_eq!(partial_repair_gate["status"], "pass");
    let drift_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "helper_drift_detection_flow")
        .expect("drift-detection gate");
    assert_eq!(drift_gate["status"], "pass");
    let parity_gate = gates
        .iter()
        .find(|gate| gate["gate"] == "manual_internal_contract_parity")
        .expect("parity gate");
    assert_eq!(parity_gate["status"], "pass");
}

#[test]
fn control_loop_boundary_gate_proves_lease_scoped_stop_hook_behavior() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    write_agents_scaffold(repo.path());
    copy_source_skills(repo.path());

    let output = run_qualify(repo.path());
    assert!(
        output.status.success(),
        "expected qualification to pass with control-loop gate; stderr was {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&output);
    let control_loop_gate = report["gates"]
        .as_array()
        .expect("gates array")
        .iter()
        .find(|gate| gate["gate"] == "control_loop_boundary")
        .expect("control-loop boundary gate");
    assert_eq!(control_loop_gate["status"], "pass");
    let checks = &control_loop_gate["details"]["checks"];
    for check in [
        "hook_surface_installed",
        "no_lease_yielded",
        "subagent_yielded",
        "lease_active",
        "active_parent_blocked",
        "lease_paused",
        "paused_parent_yielded",
        "lease_cleared",
    ] {
        assert_eq!(checks[check], true, "{check} should be proven");
    }
    let failed_steps = control_loop_gate["details"]["steps"]
        .as_array()
        .expect("control-loop steps")
        .iter()
        .filter(|step| step["success"].as_bool() == Some(false))
        .collect::<Vec<_>>();
    assert!(
        failed_steps.is_empty(),
        "passing control-loop gate must not contain failed smoke steps: {failed_steps:?}"
    );
}

#[test]
fn delegated_review_authority_gate_proves_docs_and_runtime_rejections() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    write_agents_scaffold(repo.path());
    copy_source_skills(repo.path());

    let output = run_qualify(repo.path());
    assert!(
        output.status.success(),
        "expected qualification to pass with delegated review gate; stderr was {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&output);
    let delegated_review_gate = report["gates"]
        .as_array()
        .expect("gates array")
        .iter()
        .find(|gate| gate["gate"] == "delegated_review_authority")
        .expect("delegated review authority gate");
    assert_eq!(delegated_review_gate["status"], "pass");
    assert_eq!(
        delegated_review_gate["details"]["missing_reviewer_output_rejected"],
        true
    );
    assert_eq!(
        delegated_review_gate["details"]["missing_snapshot_rejected"],
        true
    );
    assert!(
        delegated_review_gate["details"]["docs_requirements"]
            .as_array()
            .expect("docs requirements")
            .iter()
            .all(|check| check["passed"].as_bool() == Some(true))
    );
}

#[test]
fn doctor_serializes_structured_qualification_metadata() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    write_agents_scaffold(repo.path());
    copy_source_skills(repo.path());

    let qualify = run_qualify_with_home(repo.path(), home.path());
    assert!(
        qualify.status.success(),
        "qualification should pass before doctor: {}",
        String::from_utf8_lossy(&qualify.stderr)
    );

    let doctor = run_doctor(repo.path(), home.path());
    assert!(
        doctor.status.success(),
        "doctor should succeed: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );

    let report = parse_report(&doctor);
    assert!(report["qualification"].is_object());
    assert!(
        report["qualification"]["latest_report_path"]
            .as_str()
            .is_some_and(|path| path.ends_with(".codex1/qualification/latest.json"))
    );
    assert!(report["qualification"]["status"].is_string());
    assert!(report["qualification"]["stale_for_build"].is_boolean());
    assert!(report["qualification"]["stale_for_support_surface"].is_boolean());
}

#[test]
fn qualify_fails_when_agents_scaffold_is_missing() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    copy_source_skills(repo.path());

    let output = run_qualify(repo.path());
    assert!(
        !output.status.success(),
        "missing AGENTS scaffold should fail qualification"
    );

    let report = parse_report(&output);
    let agents_gate = report["gates"]
        .as_array()
        .expect("gates array")
        .iter()
        .find(|gate| gate["gate"] == "project_agents_scaffold_present")
        .expect("agents gate");
    assert_eq!(agents_gate["status"], "fail");
}

#[test]
fn doctor_marks_clean_setup_without_qualification_evidence_as_unsupported() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let doctor = run_doctor(repo.path(), home.path());
    assert!(
        doctor.status.success(),
        "doctor should succeed on a clean setup: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let report = parse_report(&doctor);
    assert_eq!(report["supported"], false);
    let qualification = report["findings"]
        .as_array()
        .expect("doctor findings")
        .iter()
        .find(|finding| finding["check"] == "qualification")
        .expect("qualification finding");
    assert_eq!(qualification["status"], "warn");
}

#[test]
fn setup_keeps_unknown_agents_commands_as_placeholders_and_blocks_qualification() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents.contains("{{BUILD_COMMAND}}"));
    assert!(agents.contains("{{TEST_COMMAND}}"));

    let qualify = run_qualify_with_home(repo.path(), home.path());
    assert!(
        !qualify.status.success(),
        "qualification should fail honestly when setup cannot infer repo commands"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&qualify.stdout),
        String::from_utf8_lossy(&qualify.stderr)
    );
    assert!(combined.contains("placeholders or missing"));
}

#[test]
fn setup_json_emits_preflight_to_stderr_before_final_report() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let stderr = String::from_utf8_lossy(&setup.stderr);
    assert!(stderr.contains("planned Codex surface changes before apply:"));
    assert!(stderr.contains(".codex/config.toml"));

    let report = parse_report(&setup);
    assert!(report["backup_id"].is_string());
}

#[test]
fn doctor_reports_runtime_overrides_as_highest_precedence_effective_config() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let doctor = run_doctor_with_args(
        repo.path(),
        home.path(),
        &["--runtime-override", "model=gpt-4.1"],
    );
    assert!(
        doctor.status.success(),
        "doctor should still emit a report: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );

    let report = parse_report(&doctor);
    let model_entry = report["effective_config"]
        .as_array()
        .expect("effective config entries")
        .iter()
        .find(|entry| entry["key"] == "model")
        .expect("model entry");
    assert_eq!(model_entry["source_layer"], "runtime_flag");
    assert_eq!(model_entry["effective_value"], "gpt-4.1");
    assert_eq!(model_entry["status"], "fail");

    let finding = report["findings"]
        .as_array()
        .expect("doctor findings")
        .iter()
        .find(|finding| finding["check"] == "config:model")
        .expect("config:model finding");
    assert_eq!(finding["status"], "fail");
}

#[test]
fn doctor_counts_direct_stop_handlers() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    fs::write(
        repo.path().join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "codex1 internal stop-hook"
      }
    ]
  }
}"#,
    )
    .expect("rewrite hooks");

    let doctor = run_doctor(repo.path(), home.path());
    assert!(
        doctor.status.success(),
        "doctor should succeed with direct handler hooks: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let report = parse_report(&doctor);
    assert_eq!(report["hook_summary"]["total_stop_handlers"], 1);
    assert_eq!(report["hook_summary"]["managed_stop_handlers"], 1);
}

#[test]
fn doctor_rejects_dead_direct_stop_hook_commands() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    fs::write(
        repo.path().join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "/definitely/missing/codex1 internal stop-hook",
        "statusMessage": "Codex1 Ralph stop hook"
      }
    ]
  }
}"#,
    )
    .expect("rewrite hooks");

    let doctor = run_doctor(repo.path(), home.path());
    assert!(
        doctor.status.success(),
        "doctor should still emit a report: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let report = parse_report(&doctor);
    assert_eq!(report["supported"], false);
    assert_eq!(report["hook_summary"]["managed_stop_handlers"], 0);
}

#[test]
fn doctor_respects_trusted_repo_lines_with_inline_comments() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    copy_source_skills(repo.path());

    let home = TempDir::new().expect("temp home");
    let codex_dir = home.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create ~/.codex");
    let canonical_repo_root = fs::canonicalize(repo.path()).expect("canonical repo root");
    fs::write(
        codex_dir.join("config.toml"),
        format!(
            "[projects.\"{}\"] # trusted repo\ntrust_level = \"trusted\" # inline comment\n",
            canonical_repo_root.display()
        ),
    )
    .expect("write trusted home config");

    let doctor = run_doctor(repo.path(), home.path());
    assert!(
        doctor.status.success(),
        "doctor should succeed: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let report = parse_report(&doctor);
    let trusted_repo = report["findings"]
        .as_array()
        .expect("doctor findings")
        .iter()
        .find(|finding| finding["check"] == "trusted_repo")
        .expect("trusted_repo finding");
    assert_eq!(trusted_repo["status"], "pass");
}

#[test]
fn restore_fails_safe_when_managed_file_has_drifted() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary.clone(), repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    fs::write(
        repo.path().join(".codex/config.toml"),
        "model = \"custom\"\nreview_model = \"custom\"\n",
    )
    .expect("drift managed config");

    let restore = run_command_with_home(binary, repo.path(), home.path(), &["restore"]);
    assert!(
        !restore.status.success(),
        "restore should fail safe when the managed config has drifted"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&restore.stdout),
        String::from_utf8_lossy(&restore.stderr)
    );
    assert!(combined.contains("drifted after setup"));
}

#[test]
fn uninstall_fails_safe_when_managed_file_has_drifted() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary.clone(), repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    fs::write(
        repo.path().join(".codex/skills/clarify/SKILL.md"),
        "drifted content\n",
    )
    .expect("drift managed skill");

    let uninstall = run_command_with_home(binary, repo.path(), home.path(), &["uninstall"]);
    assert!(
        !uninstall.status.success(),
        "uninstall should fail safe when managed skills have drifted"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&uninstall.stdout),
        String::from_utf8_lossy(&uninstall.stderr)
    );
    assert!(combined.contains("drifted after setup"));
}

#[test]
fn uninstall_fails_safe_when_managed_config_has_drifted() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary.clone(), repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    fs::write(
        repo.path().join(".codex/config.toml"),
        minimal_supported_config_toml().replacen("model = \"gpt-5.4\"", "model = \"custom\"", 1),
    )
    .expect("drift managed config");

    let uninstall = run_command_with_home(binary, repo.path(), home.path(), &["uninstall"]);
    assert!(
        !uninstall.status.success(),
        "uninstall should fail safe when managed config has drifted"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&uninstall.stdout),
        String::from_utf8_lossy(&uninstall.stderr)
    );
    assert!(combined.contains("drifted after setup"));
}

#[test]
fn uninstall_removes_direct_managed_stop_hook_after_unrelated_hook_drift() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary.clone(), repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let installed_hooks =
        fs::read_to_string(repo.path().join(".codex/hooks.json")).expect("read installed hooks");
    let managed_command = installed_hooks
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("\"command\": \"")
                .and_then(|value| value.strip_suffix("\","))
                .map(str::to_string)
        })
        .expect("setup should install a managed command");

    fs::write(
        repo.path().join(".codex/hooks.json"),
        format!(
            r#"{{
  "hooks": {{
    "Stop": [
      {{
        "type": "command",
        "command": "{}"
      }},
      {{
        "type": "command",
        "command": "echo user-hook"
      }}
    ]
  }}
}}"#,
            managed_command
        ),
    )
    .expect("rewrite hooks with direct managed entry and unrelated drift");

    let uninstall = run_command_with_home(binary, repo.path(), home.path(), &["uninstall"]);
    assert!(
        uninstall.status.success(),
        "uninstall should still remove the direct managed stop hook: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
    let hooks = fs::read_to_string(repo.path().join(".codex/hooks.json")).expect("read hooks");
    assert!(hooks.contains("echo user-hook"));
    assert!(!hooks.contains(&managed_command));
}

#[test]
fn doctor_fails_when_agents_block_has_drifted() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    fs::write(
        repo.path().join("AGENTS.md"),
        "<!-- codex1:begin -->\n## Codex1\n- Drifted block.\n<!-- codex1:end -->\n",
    )
    .expect("drift agents block");

    let doctor = run_doctor(repo.path(), home.path());
    assert!(doctor.status.success(), "doctor should still emit JSON");
    let report = parse_report(&doctor);
    assert_eq!(report["supported"], false);
    let finding = report["findings"]
        .as_array()
        .expect("doctor findings")
        .iter()
        .find(|finding| finding["check"] == "agents_md")
        .expect("agents finding");
    assert_eq!(finding["status"], "fail");
}

#[test]
fn setup_updates_legacy_agents_block_in_place() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    fs::write(
        repo.path().join("AGENTS.md"),
        "<!-- CODEX1:BEGIN MANAGED BLOCK -->\n## Codex1\n### Workflow Stance\n- Use the native Codex skills surface for `clarify`, `plan`, `execute`, `review-loop`, and `autopilot`.\n- Keep mission truth in visible repo artifacts instead of hidden chat state.\n- Replan stays internal unless the repo truth explicitly says otherwise.\n\n### Quality Bar\n- Work is complete only when the locked outcome, proof, review, and closeout contracts are all satisfied.\n- Review is mandatory before mission completion.\n- Hold the repo to production-grade changes with explicit validation and review-clean closeout.\n\n### Repo Commands\n- Build: {{BUILD_COMMAND}}\n- Test: {{TEST_COMMAND}}\n- Lint or format: {{LINT_OR_FORMAT_COMMAND}}\n\n### Artifact Conventions\n- Mission packages live under `PLANS/<mission-id>/`.\n- `OUTCOME-LOCK.md` is canonical for destination truth.\n- `PROGRAM-BLUEPRINT.md` is canonical for route truth.\n- `specs/*/SPEC.md` is canonical for one bounded execution slice.\n<!-- CODEX1:END MANAGED BLOCK -->\n",
    )
    .expect("seed legacy AGENTS.md");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should succeed on legacy AGENTS markers: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert_eq!(
        agents
            .matches("<!-- CODEX1:BEGIN MANAGED BLOCK -->")
            .count(),
        1
    );
    assert_eq!(agents.matches("<!-- codex1:begin -->").count(), 0);
    assert!(agents.contains("{{BUILD_COMMAND}}"));
    assert!(agents.contains("{{TEST_COMMAND}}"));
}

#[test]
fn setup_force_rejects_malformed_agents_markers() {
    let repo = TempDir::new().expect("temp repo");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    fs::write(
        repo.path().join("AGENTS.md"),
        "<!-- codex1:begin -->\n## Codex1\n- one\n<!-- codex1:begin -->\n## Codex1\n- two\n<!-- codex1:end -->\n",
    )
    .expect("write malformed agents");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init", "--force"]);
    assert!(
        !setup.status.success(),
        "setup --force should fail safe on malformed shared AGENTS.md markers"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&setup.stdout),
        String::from_utf8_lossy(&setup.stderr)
    );
    assert!(combined.contains("repair the shared file manually"));
}

#[test]
fn setup_and_qualification_allow_observational_user_stop_hooks() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    write_agents_scaffold(repo.path());
    copy_source_skills(repo.path());

    let home = prepare_trusted_home_with_hooks(
        repo.path(),
        r#"{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "python3 observe.py",
        "codex1_observational": true
      }
    ]
  }
}"#,
    );
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        setup.status.success(),
        "setup should allow observational user stop hooks: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let doctor = run_doctor(repo.path(), home.path());
    assert!(doctor.status.success(), "doctor should emit JSON");
    let doctor_report = parse_report(&doctor);
    let finding = doctor_report["findings"]
        .as_array()
        .expect("doctor findings")
        .iter()
        .find(|finding| finding["check"] == "user_stop_hook_conflict")
        .expect("user stop hook finding");
    assert_eq!(finding["status"], "pass");

    let qualify = run_qualify_with_home(repo.path(), home.path());
    assert!(
        qualify.status.success(),
        "qualification should allow observational user stop hooks: {}",
        String::from_utf8_lossy(&qualify.stderr)
    );
    let report = parse_report(&qualify);
    let gate = report["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|gate| gate["gate"] == "cross_layer_stop_hook_authority")
        .expect("cross-layer gate");
    assert_eq!(gate["status"], "pass");
}

#[test]
fn setup_force_clears_invalid_skills_config_bridge_and_reports_copied_surface() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    fs::create_dir_all(repo.path().join(".codex")).expect("create .codex");
    fs::write(
        repo.path().join(".codex/config.toml"),
        "[[skills.config]]\npath = \"./missing-skills\"\nenabled = true\n",
    )
    .expect("write invalid bridge config");
    let home = prepare_trusted_home(repo.path());
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init", "--force"]);
    assert!(
        setup.status.success(),
        "setup --force should repair invalid bridge installs: {}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let setup_report = parse_report(&setup);
    assert_eq!(
        setup_report["skill_surface_root"],
        fs::canonicalize(repo.path().join(".codex/skills"))
            .expect("canonical skill root")
            .display()
            .to_string()
    );

    let config = fs::read_to_string(repo.path().join(".codex/config.toml")).expect("read config");
    assert!(!config.contains("[[skills.config]]"));

    let doctor = run_doctor(repo.path(), home.path());
    assert!(doctor.status.success(), "doctor should emit JSON");
    let report = parse_report(&doctor);
    assert_eq!(report["skill_surface"]["status"], "pass");
    assert_eq!(report["skill_surface"]["install_mode"], "copied_skills");
}

#[test]
fn doctor_and_qualification_accept_skills_config_bridge_mode() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_agents_scaffold(repo.path());
    let source = source_skill_root();
    let config = format!(
        "{}\n[[skills.config]]\npath = \"{}\"\nenabled = true\n",
        minimal_supported_config_toml(),
        source.display()
    );
    let codex_dir = repo.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(codex_dir.join("config.toml"), config).expect("write config");
    fs::write(
        codex_dir.join("hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "codex1 internal stop-hook"
          }
        ]
      }
    ]
  }
}"#,
    )
    .expect("write hooks");

    let home = prepare_trusted_home(repo.path());
    let doctor = run_doctor(repo.path(), home.path());
    assert!(doctor.status.success(), "doctor should emit JSON");
    let doctor_report = parse_report(&doctor);
    assert_eq!(doctor_report["skill_surface"]["status"], "pass");
    assert_eq!(
        doctor_report["skill_surface"]["install_mode"],
        "skills_config_bridge"
    );

    let qualify = run_qualify_with_home(repo.path(), home.path());
    assert!(
        qualify.status.success(),
        "qualification should accept skills.config bridge mode: {}",
        String::from_utf8_lossy(&qualify.stderr)
    );
    let report = parse_report(&qualify);
    let gate = report["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|gate| gate["gate"] == "project_skill_surface_valid")
        .expect("skill surface gate");
    assert_eq!(gate["status"], "pass");
}

#[test]
fn setup_and_qualification_allow_observational_project_stop_hooks() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    write_agents_scaffold(repo.path());
    copy_source_skills(repo.path());
    fs::write(
        repo.path().join(".codex/hooks.json"),
        r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "codex1 internal stop-hook",
            "statusMessage": "Codex1 Ralph stop hook"
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 observe.py",
            "codex1_observational": true
          }
        ]
      }
    ]
  }
}"#,
    )
    .expect("write project hooks");

    let home = prepare_trusted_home(repo.path());
    let setup = run_command_with_home(
        assert_cmd::cargo::cargo_bin("codex1"),
        repo.path(),
        home.path(),
        &["init"],
    );
    assert!(
        setup.status.success(),
        "setup should allow observational project stop hooks: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let qualify = run_qualify_with_home(repo.path(), home.path());
    assert!(
        qualify.status.success(),
        "qualification should allow observational project stop hooks: {}",
        String::from_utf8_lossy(&qualify.stderr)
    );
    let report = parse_report(&qualify);
    let gate = report["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|gate| gate["gate"] == "project_stop_hook_authority")
        .expect("project hook gate");
    assert_eq!(gate["status"], "pass");
}

#[test]
fn setup_rejects_authoritative_user_stop_hooks() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    let home = prepare_trusted_home_with_hooks(
        repo.path(),
        r#"{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "python3 decide.py"
      }
    ]
  }
}"#,
    );
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        !setup.status.success(),
        "setup should reject authoritative user stop hooks"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&setup.stdout),
        String::from_utf8_lossy(&setup.stderr)
    );
    assert!(combined.contains("authoritative Stop handler"));
}

#[test]
fn managed_user_stop_hooks_cannot_self_label_as_observational() {
    let repo = TempDir::new().expect("temp repo");
    fs::write(repo.path().join("README.md"), "# sandbox\n").expect("seed repo");
    write_minimal_supported_config(repo.path());
    write_agents_scaffold(repo.path());
    copy_source_skills(repo.path());

    let home = prepare_trusted_home_with_hooks(
        repo.path(),
        r#"{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "codex1 internal stop-hook",
        "statusMessage": "Codex1 Ralph stop hook",
        "codex1_observational": true
      }
    ]
  }
}"#,
    );
    let binary = assert_cmd::cargo::cargo_bin("codex1");

    let setup = run_command_with_home(binary, repo.path(), home.path(), &["init"]);
    assert!(
        !setup.status.success(),
        "setup should reject mislabeled managed stop hooks"
    );

    let qualify = run_qualify_with_home(repo.path(), home.path());
    assert!(
        !qualify.status.success(),
        "qualification should reject mislabeled managed stop hooks"
    );
    let report = parse_report(&qualify);
    let gate = report["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|gate| gate["gate"] == "cross_layer_stop_hook_authority")
        .expect("cross-layer gate");
    assert_eq!(gate["status"], "fail");
}
