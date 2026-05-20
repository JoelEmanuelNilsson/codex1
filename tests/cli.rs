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

const MANAGED_SKILLS: [&str; 9] = [
    ".agents/skills/codex1/SKILL.md",
    ".agents/skills/clarify/SKILL.md",
    ".agents/skills/create-prd/SKILL.md",
    ".agents/skills/plan/SKILL.md",
    ".agents/skills/tdd/SKILL.md",
    ".agents/skills/diagnose/SKILL.md",
    ".agents/skills/improve-codebase-architecture/SKILL.md",
    ".agents/skills/prototype/SKILL.md",
    ".agents/skills/codex-review/SKILL.md",
];
const MANAGED_SUPPORTING_DOCS: [&str; 30] = [
    ".agents/skills/codex1/agents/openai.yaml",
    ".agents/skills/clarify/agents/openai.yaml",
    ".agents/skills/clarify/ADR-FORMAT.md",
    ".agents/skills/clarify/CONTEXT-FORMAT.md",
    ".agents/skills/create-prd/agents/openai.yaml",
    ".agents/skills/create-prd/PRD-FORMAT.md",
    ".agents/skills/plan/agents/openai.yaml",
    ".agents/skills/plan/ADR-FORMAT.md",
    ".agents/skills/plan/SUBPLAN-BRIEF.md",
    ".agents/skills/plan/GOAL-BRIEF-FORMAT.md",
    ".agents/skills/tdd/agents/openai.yaml",
    ".agents/skills/tdd/tests.md",
    ".agents/skills/tdd/mocking.md",
    ".agents/skills/tdd/deep-modules.md",
    ".agents/skills/tdd/interface-design.md",
    ".agents/skills/tdd/refactoring.md",
    ".agents/skills/diagnose/agents/openai.yaml",
    ".agents/skills/diagnose/scripts/hitl-loop.template.sh",
    ".agents/skills/improve-codebase-architecture/agents/openai.yaml",
    ".agents/skills/improve-codebase-architecture/LANGUAGE.md",
    ".agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md",
    ".agents/skills/improve-codebase-architecture/DEEPENING.md",
    ".agents/skills/prototype/agents/openai.yaml",
    ".agents/skills/prototype/LOGIC.md",
    ".agents/skills/prototype/UI.md",
    ".agents/skills/codex-review/agents/openai.yaml",
    ".agents/skills/codex-review/scripts/codex-review",
    "docs/agents/codex1-workflow.md",
    "docs/agents/codex1-domain.md",
    "docs/agents/codex1-artifact-briefs.md",
];

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
    assert!(!descriptors
        .iter()
        .any(|descriptor| descriptor["kind"] == "loop-state"));
    assert!(descriptors
        .iter()
        .any(|descriptor| descriptor["kind"] == "receipts"));
    assert!(descriptors
        .iter()
        .any(|descriptor| descriptor["kind"] == "goal-brief"));
    assert!(!descriptors
        .iter()
        .any(|descriptor| descriptor["kind"] == "execution-prompt"));
    assert!(repo
        .path()
        .join(".codex1/missions/alpha/SUBPLANS/ready")
        .is_dir());
    assert!(!repo
        .path()
        .join(".codex1/missions/alpha/GOAL_BRIEF.md")
        .exists());
    assert!(!repo
        .path()
        .join(".codex1/missions/alpha/.codex1/LOOP.json")
        .exists());
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
          "slice_type": "AFK - executable from artifacts",
          "linked_prd": "PRD.md",
          "linked_plan": "PLAN.md",
          "owner": "main",
          "current_behavior": "No slice exists",
          "desired_behavior": "A durable slice exists",
          "scope": ["CLI"],
          "out_of_scope": ["Unrelated behavior"],
          "steps": ["write file"],
          "acceptance_criteria": ["subplan exists"],
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

    let events = read_events(&repo, "alpha");
    let kinds: Vec<_> = events
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"subplan_moved"));
    assert!(kinds.contains(&"receipt_appended"));
    assert!(kinds.iter().all(|kind| !kind.starts_with("loop_")));

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

    let text = event_log_text(&repo, "alpha");
    for private in ["receipt text must not leak", repo.path().to_str().unwrap()] {
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

    assert_eq!(event_log_text(&repo, "alpha"), before);
}

#[test]
fn template_list_and_show_expose_goal_brief() {
    let list = json_output(bin().args(["--json", "template", "list"]));
    let templates = list["data"].as_array().unwrap();
    assert!(templates
        .iter()
        .any(|template| template["kind"] == "goal-brief" && template["name"] == "Goal Brief"));
    assert!(!templates
        .iter()
        .any(|template| template["kind"] == "execution-prompt"));

    let show = json_output(bin().args(["--json", "template", "show", "goal-brief"]));
    assert_eq!(show["data"]["kind"], "goal-brief");
    assert_eq!(show["data"]["name"], "Goal Brief");
    let sections = show["data"]["sections"].as_array().unwrap();
    assert!(sections
        .iter()
        .any(|section| section["id"] == "suggested_goal_request"
            && section["heading"] == "Suggested Goal Request"));
}

#[test]
fn plan_template_is_execution_route_not_project_management() {
    let show = json_output(bin().args(["--json", "template", "show", "plan"]));
    let sections = show["data"]["sections"].as_array().unwrap();
    let ids: Vec<_> = sections
        .iter()
        .map(|section| section["id"].as_str().unwrap())
        .collect();

    assert!(ids.contains(&"outcome_contract"));
    assert!(ids.contains(&"implementation_shape"));
    assert!(ids.contains(&"execution_order"));
    assert!(ids.contains(&"parallelization_notes"));
    assert!(ids.contains(&"ready_subplans"));
    assert!(ids.contains(&"proof_strategy"));
    assert!(ids.contains(&"human_decisions"));
    assert!(!ids.contains(&"workstreams"));
    assert!(!ids.contains(&"phases"));
}

#[test]
fn removed_execution_prompt_command_fails_through_argument_parser() {
    let repo = repo();
    init(&repo, "alpha");
    let output = bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "execution-prompt"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "ARGUMENT_ERROR");
    assert!(!repo
        .path()
        .join(".codex1/missions/alpha/EXECUTION_PROMPT.md")
        .exists());
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
    let events = read_events(&repo, "alpha");
    for (kind, code) in [
        ("artifact_write_failed", "ARTIFACT_VALIDATION_ERROR"),
        ("subplan_move_failed", "ARTIFACT_VALIDATION_ERROR"),
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
          "slice_type": "AFK - executable from artifacts",
          "linked_prd": "PRD.md",
          "linked_plan": "PLAN.md",
          "owner": "main",
          "current_behavior": "No slice exists",
          "desired_behavior": "A durable slice exists",
          "scope": ["CLI"],
          "out_of_scope": ["Unrelated behavior"],
          "steps": ["write file"],
          "acceptance_criteria": ["subplan exists"],
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
    assert_eq!(value["data"]["artifacts"]["goal_brief"], 0);
    assert!(value["data"]["artifacts"].get("execution_prompt").is_none());
}

#[test]
fn removed_loop_commands_fail_through_argument_parser() {
    let repo = repo();

    let output = bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "typo", "loop", "status"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "ARGUMENT_ERROR");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("unrecognized subcommand 'loop'"));
    assert!(!repo.path().join(".codex1/missions/typo").exists());
}

#[test]
fn removed_ralph_commands_fail_through_argument_parser() {
    let repo = repo();

    let output = bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["ralph", "stop-hook"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "ARGUMENT_ERROR");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("unrecognized subcommand 'ralph'"));
}

#[test]
fn help_does_not_advertise_removed_continuation_commands() {
    let output = bin().arg("--help").output().unwrap();

    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("loop"));
    assert!(!text.contains("ralph"));
}

#[test]
fn setup_install_materializes_repo_scoped_guidance_without_hooks() {
    let repo = repo();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    assert_eq!(value["ok"], true);
    for skill in MANAGED_SKILLS {
        assert!(repo.path().join(skill).is_file(), "{skill}");
    }
    for doc in MANAGED_SUPPORTING_DOCS {
        assert!(repo.path().join(doc).is_file(), "{doc}");
    }
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".codex1/setup-bundle.json").is_file());

    let skill = MANAGED_SKILLS
        .iter()
        .map(|skill| fs::read_to_string(repo.path().join(skill)).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let docs = MANAGED_SUPPORTING_DOCS
        .iter()
        .map(|doc| fs::read_to_string(repo.path().join(doc)).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let guidance = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    let combined = format!("{skill}\n{docs}\n{guidance}");
    assert!(combined.contains("$clarify"));
    assert!(combined.contains("$create-prd"));
    assert!(combined.contains("$plan"));
    assert!(combined.contains("$tdd"));
    assert!(combined.contains("$diagnose"));
    assert!(combined.contains("$improve-codebase-architecture"));
    assert!(combined.contains("$prototype"));
    assert!(combined.contains("$codex-review"));
    assert!(combined.contains("Codex1 Local Use"));
    assert!(combined.contains("Codex Review"));
    assert!(combined.contains("review output as advisory"));
    assert!(combined.contains("accepted/actionable findings"));
    assert!(combined.contains("Do not add a separate execution lane just to run review"));
    assert!(combined.contains("red-green-refactor"));
    assert!(combined.contains("Build a feedback loop"));
    assert!(combined.contains("Improve Codebase Architecture"));
    assert!(combined.contains("throwaway code that answers a question"));
    assert!(combined.contains("Execution Lane"));
    assert!(combined.contains("proof-qa"));
    assert!(combined.contains("standard"));
    assert!(combined.contains("default_prompt"));
    assert!(combined.contains("mission-scoped"));
    assert!(combined.contains("not a broad default dogfood audit"));
    assert!(combined.contains("Do NOT interview the user"));
    assert!(combined.contains("Write `PRD.md` as a Codex1 mission artifact"));
    assert!(combined.contains("Interview me relentlessly"));
    assert!(combined.contains("questions are still allowed"));
    assert!(combined.contains("Ask the questions one at a time"));
    assert!(combined.contains("Walk down each branch of the design tree"));
    assert!(combined.contains("Discuss concrete scenarios"));
    assert!(combined.contains("Cross-reference with code"));
    assert!(combined.contains("update `CONTEXT.md` right there"));
    assert!(combined.contains("CONTEXT.md Format"));
    assert!(combined.contains("ADR Format"));
    assert!(combined.contains("Hard to reverse"));
    assert!(combined.contains("Create files lazily"));
    assert!(combined.contains("Problem Statement"));
    assert!(combined.contains("PRD Format"));
    assert!(combined.contains("User Stories"));
    assert!(combined.contains("Module Sketch"));
    assert!(combined.contains("Implementation Decisions"));
    assert!(combined.contains("Testing Decisions"));
    assert!(combined.contains("deep modules"));
    assert!(combined.contains("Do NOT include specific file paths"));
    assert!(combined.contains("tracer-bullet vertical slices"));
    assert!(combined.contains("simple serial order by default"));
    assert!(combined.contains("not a dependency graph engine"));
    assert!(combined.contains("Subplans As Agent Briefs"));
    assert!(combined.contains("Subplan Brief Format"));
    assert!(combined.contains("Goal Brief Format"));
    assert!(combined.contains("AFK"));
    assert!(combined.contains("HITL"));
    assert!(combined.contains("ADRS/"));
    assert!(combined.contains("Write `PRD.md` into the Codex1 mission artifact tree"));
    assert!(combined.contains("explicit completion criteria"));
    assert!(combined.contains("Do not put pause, escalation"));
    assert!(combined.contains("GOAL_BRIEF.md"));
    assert!(combined.contains("native goal brief"));
    assert!(combined.contains("must not say to read `GOAL_BRIEF.md`"));
    assert!(combined.contains("native `/goal`"));
    assert!(!combined.contains("Execution Prompt Format"));
    assert!(!combined.contains("EXECUTION_PROMPT.md as the current artifact"));
    for forbidden in [
        "ralph",
        "stop-hook",
        "hooks.stop",
        "loop start",
        "loop_error",
    ] {
        assert!(!combined.to_lowercase().contains(forbidden), "{forbidden}");
    }
    assert!(!repo.path().join(".codex/config.toml").exists());
}

#[test]
fn setup_status_reports_bundle_state_only() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "status"]),
    );

    assert_eq!(value["ok"], true);
    let status = &value["data"]["status"];
    assert_eq!(status["repo_bundle_materialized"], true);
    assert_eq!(status["marker"], "current");
    assert_eq!(status["skill"], "current");
    assert_eq!(status["supporting_doc"], "current");
    assert_eq!(status["guidance"], "current");
    let skills = status["skills"].as_array().unwrap();
    assert_eq!(skills.len(), MANAGED_SKILLS.len());
    for skill in skills {
        assert_eq!(skill["state"], "current");
    }
    let docs = status["supporting_docs"].as_array().unwrap();
    assert_eq!(docs.len(), MANAGED_SUPPORTING_DOCS.len());
    for doc in docs {
        assert_eq!(doc["state"], "current");
    }
    assert!(status.get("global_hook_installed").is_none());
    assert!(status.get("project_hook_installed").is_none());
    assert!(status.get("duplicate_hook_risk").is_none());
    assert!(!value.to_string().contains("native_goal_state"));
}

#[test]
fn checked_in_docs_mark_execution_prompt_mentions_as_legacy_only() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "README.md",
        "AGENTS.md",
        "docs/agents/codex1-workflow.md",
        "docs/agents/codex1-artifact-briefs.md",
        "docs/agents/codex1-domain.md",
        "docs/artifact-model.md",
        "docs/cli-contract.md",
        "docs/skill-workflows.md",
        ".agents/skills/codex1/SKILL.md",
        ".agents/skills/clarify/SKILL.md",
        ".agents/skills/create-prd/SKILL.md",
        ".agents/skills/plan/SKILL.md",
        ".agents/skills/plan/GOAL-BRIEF-FORMAT.md",
        ".agents/skills/plan/SUBPLAN-BRIEF.md",
        ".agents/skills/tdd/SKILL.md",
        ".agents/skills/diagnose/SKILL.md",
        ".agents/skills/improve-codebase-architecture/SKILL.md",
        ".agents/skills/prototype/SKILL.md",
        ".agents/skills/codex-review/SKILL.md",
    ];
    for file in files {
        let text = fs::read_to_string(root.join(file)).unwrap();
        assert!(
            !text.contains("EXECUTION-PROMPT-FORMAT.md"),
            "{file} mentions the old format file"
        );
        let lines: Vec<_> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let mentions_old = line.contains("EXECUTION_PROMPT")
                || line.contains("execution-prompt")
                || line.contains("Execution Prompt")
                || line.contains("execution prompt");
            if !mentions_old {
                continue;
            }
            let context = [
                index.checked_sub(1).and_then(|i| lines.get(i)).copied(),
                Some(*line),
                lines.get(index + 1).copied(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
            assert!(
                context.contains("legacy"),
                "{file}:{} mentions old execution-prompt wording outside legacy guidance",
                index + 1
            );
        }
    }
}

#[test]
fn setup_doctor_reports_malformed_backup_manifest() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    fs::write(
        repo.path().join(".codex1/setup-backups/manifest.json"),
        "not json\n",
    )
    .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "doctor"]),
    );

    assert_eq!(value["ok"], true);
    let backup_manifest = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "backup_manifest")
        .unwrap();
    assert_eq!(backup_manifest["ok"], false);
    assert!(backup_manifest["error"]
        .as_str()
        .unwrap()
        .contains("failed to parse backup manifest"));
}

#[test]
fn setup_install_dry_run_does_not_materialize_files() {
    let repo = repo();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install", "--dry-run"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["plan"]["dry_run"], true);
    for skill in MANAGED_SKILLS {
        assert!(!repo.path().join(skill).exists(), "{skill}");
    }
    for doc in MANAGED_SUPPORTING_DOCS {
        assert!(!repo.path().join(doc).exists(), "{doc}");
    }
    assert!(!repo.path().join("AGENTS.md").exists());
    assert!(!repo.path().join(".codex1/setup-bundle.json").exists());
    assert!(!repo
        .path()
        .join(".codex1/setup-backups/manifest.json")
        .exists());
}

#[test]
fn setup_disable_and_enable_preserve_user_guidance_and_missions() {
    let repo = repo();
    fs::write(
        repo.path().join("AGENTS.md"),
        "# Local Rules\n\nKeep this.\n",
    )
    .unwrap();
    init(&repo, "alpha");
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "disable"]),
    );

    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("Keep this."));
    assert!(!agents.contains("codex1-managed setup guidance start"));
    for skill in MANAGED_SKILLS {
        assert!(!repo.path().join(skill).exists(), "{skill}");
    }
    for doc in MANAGED_SUPPORTING_DOCS {
        assert!(!repo.path().join(doc).exists(), "{doc}");
    }
    assert!(repo.path().join(".codex1/missions/alpha").is_dir());

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "enable"]),
    );
    let restored = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert!(restored.contains("Keep this."));
    assert!(restored.contains("codex1-managed setup guidance start"));
}

#[test]
fn setup_uninstall_without_marker_preserves_unmanaged_repo_files() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# User skill\n",
    )
    .unwrap();
    fs::write(repo.path().join("AGENTS.md"), "# Local Rules\n").unwrap();
    init(&repo, "alpha");

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "uninstall"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(
        fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap(),
        "# User skill\n"
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("AGENTS.md")).unwrap(),
        "# Local Rules\n"
    );
    assert!(repo.path().join(".codex1/missions/alpha").is_dir());
}

#[test]
fn setup_uninstall_removes_expanded_managed_skill_bundle() {
    let repo = repo();
    init(&repo, "alpha");
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "uninstall"]),
    );

    for skill in MANAGED_SKILLS {
        assert!(!repo.path().join(skill).exists(), "{skill}");
    }
    for doc in MANAGED_SUPPORTING_DOCS {
        assert!(!repo.path().join(doc).exists(), "{doc}");
    }
    assert!(!repo.path().join("AGENTS.md").exists());
    assert!(!repo.path().join(".codex1/setup-bundle.json").exists());
    assert!(repo.path().join(".codex1/missions/alpha").is_dir());
}

#[test]
fn setup_enable_repairs_stale_managed_skill_and_marker() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# Old managed skill\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 0,
            "files": [".agents/skills/codex1/SKILL.md", "AGENTS.md"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "enable"]),
    );

    let skill = fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap();
    let marker = fs::read_to_string(repo.path().join(".codex1/setup-bundle.json")).unwrap();
    assert!(skill.contains("$clarify"));
    assert!(repo
        .path()
        .join(".agents/skills/clarify/SKILL.md")
        .is_file());
    assert!(repo
        .path()
        .join(".agents/skills/create-prd/SKILL.md")
        .is_file());
    assert!(repo.path().join(".agents/skills/plan/SKILL.md").is_file());
    assert!(marker.contains(r#""version": 7"#));
}

#[test]
fn setup_install_refuses_unmanaged_skill_without_marker() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# User skill\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "install"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

    assert_eq!(
        fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap(),
        "# User skill\n"
    );
}

#[test]
fn setup_install_refuses_unmanaged_workflow_skills_without_marker() {
    for skill in MANAGED_SKILLS {
        let repo = repo();
        let path = repo.path().join(skill);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# User skill\n").unwrap();

        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

        assert_eq!(fs::read_to_string(path).unwrap(), "# User skill\n");
    }
}

#[test]
fn setup_install_refuses_unmanaged_supporting_docs_without_marker() {
    for doc in MANAGED_SUPPORTING_DOCS {
        let repo = repo();
        let path = repo.path().join(doc);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# User doc\n").unwrap();

        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

        assert_eq!(fs::read_to_string(path).unwrap(), "# User doc\n");
    }
}

#[test]
fn setup_enable_with_legacy_marker_refuses_unmanaged_new_skill_without_partial_install() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::create_dir_all(repo.path().join(".agents/skills/plan")).unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# Old managed skill\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".agents/skills/plan/SKILL.md"),
        "# User plan skill\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": [".agents/skills/codex1/SKILL.md", "AGENTS.md"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "enable"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

    assert_eq!(
        fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap(),
        "# Old managed skill\n"
    );
    assert_eq!(
        fs::read_to_string(repo.path().join(".agents/skills/plan/SKILL.md")).unwrap(),
        "# User plan skill\n"
    );
    assert!(!repo.path().join(".agents/skills/clarify/SKILL.md").exists());
    assert!(!repo
        .path()
        .join(".agents/skills/create-prd/SKILL.md")
        .exists());
}

#[test]
fn setup_backups_restore_previous_absence() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let backups = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "list"]),
    );
    let id = backups["data"]["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["target_path_label"]
                .as_str()
                .unwrap()
                .ends_with("AGENTS.md")
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "restore", &id, "--force"]),
    );

    assert!(!repo.path().join("AGENTS.md").exists());
}

#[test]
fn setup_backups_restore_supporting_doc_previous_absence() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let backups = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "list"]),
    );

    for doc in [
        "docs/agents/codex1-workflow.md",
        ".agents/skills/clarify/ADR-FORMAT.md",
    ] {
        assert!(repo.path().join(doc).is_file());
        let id = backups["data"]["backups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["target_path_label"].as_str().unwrap().ends_with(doc))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        json_output(
            bin()
                .args(["--json", "--repo-root"])
                .arg(repo.path())
                .args(["setup", "backups", "restore", &id, "--force"]),
        );

        assert!(!repo.path().join(doc).exists());
    }
}

#[test]
fn setup_backups_restore_absence_preserves_later_user_guidance() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let backups = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "list"]),
    );
    let id = backups["data"]["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["target_path_label"]
                .as_str()
                .unwrap()
                .ends_with("AGENTS.md")
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    fs::write(repo.path().join("AGENTS.md"), "# User guidance\n").unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "backups", "restore", &id, "--force"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_RESTORE_ERROR"));

    assert_eq!(
        fs::read_to_string(repo.path().join("AGENTS.md")).unwrap(),
        "# User guidance\n"
    );
}

#[test]
fn setup_backups_restore_absence_dry_run_validates_user_guidance() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let backups = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "list"]),
    );
    let id = backups["data"]["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["target_path_label"]
                .as_str()
                .unwrap()
                .ends_with("AGENTS.md")
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    fs::write(repo.path().join("AGENTS.md"), "# User guidance\n").unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "backups", "restore", &id, "--dry-run"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_RESTORE_ERROR"));

    assert_eq!(
        fs::read_to_string(repo.path().join("AGENTS.md")).unwrap(),
        "# User guidance\n"
    );
}

#[test]
fn setup_backups_restore_absence_removes_managed_block_only() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let backups = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "list"]),
    );
    let id = backups["data"]["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["target_path_label"]
                .as_str()
                .unwrap()
                .ends_with("AGENTS.md")
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let managed_guidance = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    fs::write(
        repo.path().join("AGENTS.md"),
        format!("# User guidance\n\n{managed_guidance}\n# Keep this too\n"),
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "restore", &id, "--force"]),
    );

    let guidance = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert!(guidance.contains("# User guidance"));
    assert!(guidance.contains("# Keep this too"));
    assert!(!guidance.contains("codex1-managed setup guidance start"));
}

#[test]
fn setup_backups_restore_rejects_non_setup_targets() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex1/setup-backups/files/tampered")).unwrap();
    fs::write(
        repo.path()
            .join(".codex1/setup-backups/files/tampered/PRD.md"),
        "# Backup\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".codex1/setup-backups/manifest.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "records": [{
                "id": "tampered",
                "timestamp": "2026-05-02T00:00:00Z",
                "target_kind": "repo-setup",
                "target_path": repo.path().join(".codex1/missions/alpha/PRD.md"),
                "target_path_label": "PRD.md",
                "backup_path": repo.path().join(".codex1/setup-backups/files/tampered/PRD.md"),
                "existed": true,
                "reason": "tampered"
            }]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "backups", "restore", "tampered", "--force"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_RESTORE_ERROR"));

    assert!(!repo.path().join(".codex1/missions/alpha/PRD.md").exists());
}

#[test]
fn setup_backups_restore_rejects_escaping_backup_path() {
    let repo = repo();
    let target = repo.path().join(".agents/skills/codex1/SKILL.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "# Current skill\n").unwrap();
    fs::create_dir_all(repo.path().join(".codex1/setup-backups/files/tampered")).unwrap();
    fs::write(repo.path().join("AGENTS.md"), "# Not a backup\n").unwrap();
    fs::write(
        repo.path().join(".codex1/setup-backups/manifest.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "records": [{
                "id": "tampered",
                "timestamp": "2026-05-02T00:00:00Z",
                "target_kind": "repo-setup",
                "target_path": target,
                "target_path_label": ".agents/skills/codex1/SKILL.md",
                "backup_path": repo.path().join(".codex1/setup-backups/files/tampered/../../../../AGENTS.md"),
                "existed": true,
                "reason": "tampered"
            }]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "backups", "restore", "tampered", "--force"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_RESTORE_ERROR"));

    assert_eq!(fs::read_to_string(target).unwrap(), "# Current skill\n");
}

#[test]
fn removed_setup_hook_options_fail_through_argument_parser() {
    let repo = repo();

    for args in [
        vec!["setup", "migrate", "--to", "project"],
        vec!["setup", "install", "--scope", "project"],
        vec!["setup", "install", "--mode", "all"],
    ] {
        let output = bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "ARGUMENT_ERROR");
    }
}

#[cfg(unix)]
#[test]
fn symlinked_mission_root_is_rejected_before_inspect_reads() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let mission_dir = repo.path().join(".codex1/missions/alpha");
    fs::remove_dir_all(&mission_dir).unwrap();
    symlink(external.path(), &mission_dir).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "inspect"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));
}

#[cfg(unix)]
#[test]
fn symlinked_missions_directory_is_rejected_before_reads() {
    let repo = repo();
    let external = tempfile::tempdir().unwrap();
    init(&repo, "alpha");
    let missions_dir = repo.path().join(".codex1/missions");
    fs::remove_dir_all(&missions_dir).unwrap();
    symlink(external.path(), &missions_dir).unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "inspect"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("MISSION_PATH_ERROR"));
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
fn doctor_runs_installed_command_smoke() {
    let value = json_output(bin().args(["--json", "doctor"]));
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["data"]["installed_command"]["json_error_envelope"],
        true
    );
    assert!(value["data"].get("loop_schema_version").is_none());
    assert!(value["data"].get("loop_ralph_smoke").is_none());
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
fn goal_brief_interview_writes_native_goal_brief() {
    let repo = repo();
    init(&repo, "alpha");
    let answers = repo.path().join("goal-brief.json");
    fs::write(
        &answers,
        r#"{
          "title": "Execute Alpha",
          "purpose": "Use this brief to create or refine a native Codex mission goal for `.codex1/missions/alpha`.",
          "suggested_goal_request": "Execute the Codex1 mission at `.codex1/missions/alpha`.\n\nUse `PRD.md` as the outcome contract and complete the native goal only after evidence is audited.",
          "mission_path": ".codex1/missions/alpha",
          "primary_artifacts": ["PRD.md", "PLAN.md", "SPECS/", "SUBPLANS/ready/"],
          "execution_order": ["Select one ready subplan", "Implement it", "Record proof"],
          "subplan_selection": ["Prefer dependency-free ready subplans"],
          "editable_scope": ["Implementation files", "Assigned mission artifacts"],
          "proof_rules": ["Write a proof after each completed slice"],
          "completion_criteria": ["All required ready subplans are complete or explicitly triaged as not applicable"],
          "non_completion_behavior": ["If completion cannot be reached from artifacts, record why and do not ask questions"],
          "closeout_rules": ["Write CLOSEOUT.md only after PRD satisfaction is audited"],
          "prohibited_actions": ["Do not treat inspect, events, receipts, or folder placement as completion proof"]
        }"#,
    )
    .unwrap();

    bin()
        .args(["--repo-root"])
        .arg(repo.path())
        .args(["--mission", "alpha", "interview", "goal-brief", "--answers"])
        .arg(&answers)
        .assert()
        .success();

    let rendered =
        fs::read_to_string(repo.path().join(".codex1/missions/alpha/GOAL_BRIEF.md")).unwrap();
    assert!(rendered.contains("codex1_template: goal-brief"));
    assert!(rendered.contains("<!-- codex1-section: suggested_goal_request -->"));
    assert!(rendered.contains("Execute the Codex1 mission at `.codex1/missions/alpha`."));
    assert!(rendered.contains("<!-- codex1-section: completion_criteria -->"));
    assert!(!rendered.contains("Read `GOAL_BRIEF.md`"));
    assert!(!repo
        .path()
        .join(".codex1/missions/alpha/EXECUTION_PROMPT.md")
        .exists());

    let events = read_events(&repo, "alpha");
    let event = events.last().unwrap();
    assert_eq!(event["metadata"]["artifact_kind"], "goal-brief");
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
          "slice_type": "AFK - executable from artifacts",
          "linked_prd": "PRD.md",
          "linked_plan": "PLAN.md",
          "owner": "main",
          "current_behavior": "No slice exists",
          "desired_behavior": "A durable slice exists",
          "scope": ["CLI"],
          "out_of_scope": ["Unrelated behavior"],
          "steps": ["write file"],
          "acceptance_criteria": ["subplan exists"],
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
