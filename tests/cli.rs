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
use std::os::unix::fs::{symlink, PermissionsExt};

const MANAGED_SKILLS: [&str; 11] = [
    ".agents/skills/codex1/SKILL.md",
    ".agents/skills/clarify/SKILL.md",
    ".agents/skills/create-prd/SKILL.md",
    ".agents/skills/plan/SKILL.md",
    ".agents/skills/tdd/SKILL.md",
    ".agents/skills/diagnose/SKILL.md",
    ".agents/skills/improve-codebase-architecture/SKILL.md",
    ".agents/skills/prototype/SKILL.md",
    ".agents/skills/codex-review/SKILL.md",
    ".agents/skills/brutal-review/SKILL.md",
    ".agents/skills/handoff/SKILL.md",
];

const MANAGED_SUPPORTING_DOCS: [&str; 32] = [
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
    ".agents/skills/brutal-review/agents/openai.yaml",
    ".agents/skills/handoff/agents/openai.yaml",
    "docs/agents/codex1-workflow.md",
    "docs/agents/codex1-domain.md",
    "docs/agents/codex1-artifact-briefs.md",
];

const LEGACY_EXECUTION_PROMPT_FORMAT_BODY: &str = r#"# Execution Prompt Format

`EXECUTION_PROMPT.md` contains the objective text the user copies after typing native `/goal`.

It is a copy source. The prompt must not tell Codex to read `EXECUTION_PROMPT.md`; the user has already copied from it.

## Native Goal Objective Must Include

- Mission path
- Primary artifacts to read
- Execution order
- Subplan selection rules
- Worker/subagent rules when useful
- Editable scope
- Proof recording rules
- Review and triage rules
- Explicit completion criteria
- If completion cannot be reached
- Closeout rules
- Prohibited actions

## Completion Criteria

Completion criteria are only completion criteria. Do not include pause, escalation, "ask the user", or "wait for clarification" criteria.

Good completion criteria are observable:

- Required ready subplans are complete or explicitly triaged not applicable.
- Required proofs exist and were audited.
- PRD success criteria are satisfied or recorded as deferred with reason.
- Closeout summarizes completed, superseded, paused, deferred, and risky work.

## No-question Execution

The `/goal` execution phase may not ask questions. If artifacts are insufficient, Codex should record non-completion evidence, blockers, accepted risks, or deferred work rather than inventing scope or asking the user.

## Worker Rules

When using workers, give each worker explicit ownership, relevant artifacts, editable scope, proof expectations, and non-goals. Workers should not edit mission-level artifacts unless assigned.

## Prohibited Actions

Always prohibit:

- Creating, inspecting, or completing native goal state from Codex1.
- Treating `codex1 inspect`, setup status, events, or receipts as completion proof.
- Reading `EXECUTION_PROMPT.md` as the first step of the pasted objective.
"#;

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

#[cfg(unix)]
fn fake_codex_script(body: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fake-codex");
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    (dir, path)
}

#[cfg(unix)]
fn fake_codex_script_named(name: &str, body: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    (dir, path)
}

#[cfg(unix)]
fn fake_codex_jsonl_script(final_message: &str) -> (TempDir, PathBuf) {
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
fn process_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
fn wait_until(deadline: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    condition()
}

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
fn codex_review_helper_no_yolo_dry_run_omits_full_access() {
    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--no-yolo",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("review: codex exec review --json --uncommitted"));
    assert!(!stdout.contains("--dangerously-bypass-approvals-and-sandbox"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_is_quiet_by_default() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' '{"type":"notice","message":"LOUD LINE"}'
printf '%s\n' '{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"I did not find any discrete correctness issues in the changed helper."}}'
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review clean"));
    assert!(!stdout.contains("LOUD LINE"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_accepts_found_no_discrete_clean_signal() {
    let (_dir, fake_codex) =
        fake_codex_jsonl_script("I found no discrete correctness issues in the changes.");

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review clean"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_requires_explicit_clean_signal() {
    let (_dir, fake_codex) = fake_codex_jsonl_script("No findings here");

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review inconclusive"));
    assert!(stdout.contains("full output: "));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_malformed_jsonl_is_failure() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' '{"type":"item.completed"'
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review failed: malformed JSONL output"));
    assert!(stdout.contains("full output: "));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_error_event_is_failure() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' '{"type":"error","message":"boom"}'
printf '%s\n' '{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"No findings were reported."}}'
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review failed: JSONL error event reported"));
    assert!(stdout.contains("full output: "));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_requires_final_agent_message() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' '{"type":"turn.completed"}'
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review inconclusive: no final agent_message found"));
    assert!(stdout.contains("full output: "));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_output_writes_jsonl_and_stderr_sidecar() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' 'Full review comments:' >&2
printf '%s\n' '- [P2] stderr is not a finding - file:1' >&2
printf '%s\n' '{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"No findings were reported."}}'
"#,
    );
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("review.jsonl");

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .args(["--output"])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review clean"));
    assert!(!stdout.contains("stderr is not a finding"));
    assert!(fs::read_to_string(&output_path)
        .unwrap()
        .contains(r#""type":"agent_message""#));
    assert!(
        fs::read_to_string(output_path.with_extension("jsonl.stderr"))
            .unwrap()
            .contains("stderr is not a finding")
    );
}

#[cfg(unix)]
#[test]
fn codex_review_helper_verbose_streams_nested_output() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' '{"type":"notice","message":"LOUD LINE"}'
printf '%s\n' '{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"No actionable correctness issues were found."}}'
sleep 1
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .arg("--verbose")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("LOUD LINE"));
    assert!(stdout.contains("codex-review clean"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_verbose_preserves_fast_failure_output() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' 'FAST FAILURE' >&2
exit 127
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .arg("--verbose")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("FAST FAILURE"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_prints_findings_not_full_noise() {
    let (_dir, fake_codex) = fake_codex_jsonl_script(
        "Full review comments:\n\n- [P2] Bad thing - file:1\n  Detail line\n\n- [P1] Worse thing - file:2\n  Second detail\n",
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review findings"));
    assert!(stdout.contains("- [P2] Bad thing - file:1"));
    assert!(stdout.contains("  Detail line"));
    assert!(stdout.contains("- [P1] Worse thing - file:2"));
    assert!(stdout.contains("  Second detail"));
    assert!(stdout.contains("full output: "));
    assert!(!stdout.contains("LOUD LINE"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_preserves_findings_when_parallel_tests_fail() {
    let (_dir, fake_codex) =
        fake_codex_jsonl_script("Review comment:\n- [P2] Real finding - file:1\n  Detail line\n");

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .args(["--parallel-tests", "false"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("tests exit: 1"));
    assert!(stdout.contains("- [P2] Real finding - file:1"));
    assert!(stdout.contains("full output: "));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_ignores_finding_like_command_output() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
printf '%s\n' '{"type":"item.completed","item":{"id":"item_exec","type":"command_execution","text":"Full review comments:\n- [P2] Fake command output - file:1"}}'
printf '%s\n' '{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"No actionable correctness issues were found."}}'
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review clean"));
    assert!(!stdout.contains("Fake command output"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_refuses_nested_invocation() {
    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CODEX_REVIEW_HELPER_ACTIVE", "1")
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("nested codex-review helper invocation refused"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_allows_explicit_fake_codex_under_nested_marker() {
    let (_dir, fake_codex) =
        fake_codex_jsonl_script("No actionable correctness issues were found.");

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CODEX_REVIEW_HELPER_ACTIVE", "1")
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review clean"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_blocks_codex_spawned_by_reviewer() {
    let inner_stderr = tempfile::NamedTempFile::new().unwrap();
    let inner_stderr_path = inner_stderr.path().to_path_buf();
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
if codex exec review --json --commit HEAD 2>"$INNER_CODEX_STDERR"; then
  echo "inner codex unexpectedly succeeded" >&2
  exit 1
fi
cat <<'JSONL'
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"No actionable correctness issues were found."}}
JSONL
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("INNER_CODEX_STDERR", &inner_stderr_path)
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review clean"));
    let inner_stderr = fs::read_to_string(inner_stderr_path).unwrap();
    assert!(inner_stderr.contains("nested codex invocation refused by codex-review helper"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_refuses_nested_real_codex_bin() {
    let body = format!(
        "#!/usr/bin/env bash\ncat <<'JSONL'\n{}\nJSONL\n",
        serde_json::json!({
            "type": "item.completed",
            "item": {
                "id": "item_0",
                "type": "agent_message",
                "text": "No actionable correctness issues were found."
            }
        })
    );
    let (_dir, fake_codex) = fake_codex_script_named("codex", &body);

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CODEX_REVIEW_HELPER_ACTIVE", "1")
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("nested codex-review helper invocation refused"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_timeout_kills_child_tree() {
    let child_pid_file = tempfile::NamedTempFile::new().unwrap();
    let child_pid_path = child_pid_file.path().to_path_buf();
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
bash -c 'trap "" TERM; sleep 30' &
printf '%s\n' "$!" > "$FAKE_CHILD_PID_FILE"
wait
"#,
    );

    let helper = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("FAKE_CHILD_PID_FILE", &child_pid_path)
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--timeout-seconds",
            "10",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    assert!(wait_until(Duration::from_secs(6), || child_pid_path
        .metadata()
        .is_ok_and(|metadata| metadata.len() > 0)));
    let child_pid: u32 = fs::read_to_string(&child_pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let output = helper.wait_with_output().unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review timed out after"));
    assert!(stdout.contains("full output: "));
    assert!(wait_until(Duration::from_secs(2), || !process_is_alive(
        child_pid
    )));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_interrupt_kills_child_tree() {
    let child_pid_file = tempfile::NamedTempFile::new().unwrap();
    let child_pid_path = child_pid_file.path().to_path_buf();
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
sleep 30 &
printf '%s\n' "$!" > "$FAKE_CHILD_PID_FILE"
wait
"#,
    );

    let mut helper = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("FAKE_CHILD_PID_FILE", &child_pid_path)
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--timeout-seconds",
            "0",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    assert!(wait_until(Duration::from_secs(6), || child_pid_path
        .metadata()
        .is_ok_and(|metadata| metadata.len() > 0)));
    let child_pid: u32 = fs::read_to_string(&child_pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    Command::new("kill")
        .args(["-TERM", &helper.id().to_string()])
        .status()
        .unwrap();

    assert!(wait_until(Duration::from_secs(6), || helper
        .try_wait()
        .unwrap()
        .is_some()));
    assert!(wait_until(Duration::from_secs(6), || !process_is_alive(
        child_pid
    )));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_parallel_timeout_reports_timeout() {
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
sleep 30
"#,
    );

    let output = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--timeout-seconds",
            "2",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .args(["--parallel-tests", "true"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-review timed out after"));
}

#[cfg(unix)]
#[test]
fn codex_review_helper_parallel_interrupt_kills_review_and_tests() {
    let review_pid_file = tempfile::NamedTempFile::new().unwrap();
    let review_pid_path = review_pid_file.path().to_path_buf();
    let test_pid_file = tempfile::NamedTempFile::new().unwrap();
    let test_pid_path = test_pid_file.path().to_path_buf();
    let (_dir, fake_codex) = fake_codex_script(
        r#"#!/usr/bin/env bash
sleep 30 &
printf '%s\n' "$!" > "$FAKE_REVIEW_CHILD_PID_FILE"
wait
"#,
    );

    let parallel_tests = "sleep 30 & printf '%s\\n' \"$!\" > \"$FAKE_TEST_CHILD_PID_FILE\"; wait";
    let mut helper = Command::new("bash")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("FAKE_REVIEW_CHILD_PID_FILE", &review_pid_path)
        .env("FAKE_TEST_CHILD_PID_FILE", &test_pid_path)
        .args([
            ".agents/skills/codex-review/scripts/codex-review",
            "--mode",
            "local",
            "--timeout-seconds",
            "0",
            "--codex-bin",
        ])
        .arg(&fake_codex)
        .args(["--parallel-tests", parallel_tests])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    assert!(wait_until(Duration::from_secs(5), || review_pid_path
        .metadata()
        .is_ok_and(|metadata| metadata.len() > 0)
        && test_pid_path
            .metadata()
            .is_ok_and(|metadata| metadata.len() > 0)));
    let review_child_pid: u32 = fs::read_to_string(&review_pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let test_child_pid: u32 = fs::read_to_string(&test_pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    Command::new("kill")
        .args(["-TERM", &helper.id().to_string()])
        .status()
        .unwrap();

    assert!(wait_until(Duration::from_secs(2), || helper
        .try_wait()
        .unwrap()
        .is_some()));
    assert!(wait_until(Duration::from_secs(2), || !process_is_alive(
        review_child_pid
    )));
    assert!(wait_until(Duration::from_secs(2), || !process_is_alive(
        test_child_pid
    )));
}

#[test]
fn setup_install_materializes_repo_scoped_guidance() {
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

    let guidance = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert!(guidance.contains("codex1-managed setup guidance start"));
    let clarify = fs::read_to_string(repo.path().join(".agents/skills/clarify/SKILL.md")).unwrap();
    assert!(clarify.contains("clarify observable success outcomes and boundaries"));
    assert!(clarify.contains("Before considering clarification complete"));
    assert!(clarify.contains("assume the final finished product"));
    assert!(clarify.contains("Always Preserve"));
    let prd_skill =
        fs::read_to_string(repo.path().join(".agents/skills/create-prd/SKILL.md")).unwrap();
    assert!(prd_skill.contains("per-story acceptance-criteria engine"));
    assert!(prd_skill.contains("final finished-product contract"));
    assert!(prd_skill.contains("## Boundaries"));
    assert!(prd_skill.contains("do not introduce fallback paths, legacy compatibility"));
    let prd_format =
        fs::read_to_string(repo.path().join(".agents/skills/create-prd/PRD-FORMAT.md")).unwrap();
    assert!(prd_format.contains("A long numbered list of behavior-focused user stories"));
    assert!(prd_format.contains("final finished-product contract"));
    assert!(prd_format.contains("do not introduce fallback paths, legacy compatibility"));
    assert!(prd_format.contains("### Always Preserve"));
    assert!(prd_format.contains("### Ask Before Changing"));
    let plan = fs::read_to_string(repo.path().join(".agents/skills/plan/SKILL.md")).unwrap();
    assert!(plan.contains("subplans are implementation slices, not product stages"));
    let artifact_briefs =
        fs::read_to_string(repo.path().join("docs/agents/codex1-artifact-briefs.md")).unwrap();
    assert!(artifact_briefs.contains("assume the final finished product"));
    assert!(!repo.path().join(".codex/config.toml").exists());
}

#[test]
fn setup_without_subcommand_materializes_repo_scoped_guidance() {
    let repo = repo();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .arg("setup"),
    );

    assert_eq!(value["ok"], true);
    assert!(repo.path().join(".agents/skills/codex1/SKILL.md").is_file());
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".codex1/setup-bundle.json").is_file());
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
    assert!(!value.to_string().contains("native_goal_state"));
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
fn setup_enable_repairs_stale_managed_skill_and_marker() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let marker_path = repo.path().join(".codex1/setup-bundle.json");
    let marker = fs::read_to_string(&marker_path).unwrap();
    fs::write(
        &marker_path,
        marker.replace(r#""version": 13"#, r#""version": 12"#),
    )
    .unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# Stale managed skill\n",
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
    assert!(repo.path().join(".agents/skills/plan/SKILL.md").is_file());
    assert!(marker.contains(r#""version": 13"#));
}

#[test]
fn setup_enable_upgrades_pre_handoff_bundle() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    fs::remove_dir_all(repo.path().join(".agents/skills/handoff")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# Old managed overview\n",
    )
    .unwrap();

    let marker_path = repo.path().join(".codex1/setup-bundle.json");
    let mut marker: Value =
        serde_json::from_str(&fs::read_to_string(&marker_path).unwrap()).unwrap();
    marker["version"] = serde_json::json!(11);
    let files = marker["files"].as_array_mut().unwrap();
    files.retain(|file| {
        !matches!(
            file.as_str(),
            Some(".agents/skills/handoff/SKILL.md")
                | Some(".agents/skills/handoff/agents/openai.yaml")
        )
    });
    fs::write(
        &marker_path,
        serde_json::to_string_pretty(&marker).unwrap() + "\n",
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "enable"]),
    );

    let status = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "status"]),
    );
    assert_eq!(status["data"]["status"]["repo_bundle_materialized"], true);
    assert!(repo
        .path()
        .join(".agents/skills/handoff/SKILL.md")
        .is_file());
    assert!(repo
        .path()
        .join(".agents/skills/handoff/agents/openai.yaml")
        .is_file());
    let overview = fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap();
    assert!(overview.contains("$handoff"));
}

#[test]
fn setup_uninstall_accepts_v1_managed_marker() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
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

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "uninstall"]),
    );

    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!repo.path().join(".codex1/setup-bundle.json").exists());
}

#[test]
fn setup_uninstall_refuses_modified_marker_owned_skill() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let skill = repo.path().join(".agents/skills/codex1/SKILL.md");
    fs::write(&skill, "# User edited skill\n").unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "uninstall"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

    assert_eq!(fs::read_to_string(skill).unwrap(), "# User edited skill\n");
}

#[test]
fn setup_install_removes_retired_managed_bundle_file_before_rewriting_marker() {
    let repo = repo();
    let retired = repo
        .path()
        .join(".agents/skills/plan/EXECUTION-PROMPT-FORMAT.md");
    fs::create_dir_all(retired.parent().unwrap()).unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(&retired, LEGACY_EXECUTION_PROMPT_FORMAT_BODY).unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 4,
            "files": [
                ".agents/skills/codex1/SKILL.md",
                ".agents/skills/clarify/SKILL.md",
                ".agents/skills/clarify/ADR-FORMAT.md",
                ".agents/skills/clarify/CONTEXT-FORMAT.md",
                ".agents/skills/create-prd/SKILL.md",
                ".agents/skills/create-prd/PRD-FORMAT.md",
                ".agents/skills/plan/SKILL.md",
                ".agents/skills/plan/ADR-FORMAT.md",
                ".agents/skills/plan/SUBPLAN-BRIEF.md",
                ".agents/skills/plan/EXECUTION-PROMPT-FORMAT.md",
                "docs/agents/codex1-workflow.md",
                "docs/agents/codex1-domain.md",
                "docs/agents/codex1-artifact-briefs.md",
                "AGENTS.md"
            ]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    assert!(!retired.exists());
}

#[test]
fn setup_uninstall_refuses_marker_with_unmanaged_docs_path() {
    let repo = repo();
    let private_doc = repo.path().join("docs/agents/private.md");
    fs::create_dir_all(private_doc.parent().unwrap()).unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(&private_doc, "# User doc\n").unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": ["docs/agents/private.md"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "uninstall"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

    assert_eq!(fs::read_to_string(private_doc).unwrap(), "# User doc\n");
}

#[test]
fn setup_install_refuses_marker_with_unmanaged_paths() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": ["src/lib.rs"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "install"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));
}

#[test]
fn setup_install_refuses_partial_managed_marker() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# User skill\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": [".agents/skills/codex1/SKILL.md"]
        }))
        .unwrap()
            + "\n",
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
fn setup_install_refuses_unmanaged_managed_files_without_marker() {
    for relative in [MANAGED_SKILLS[0], MANAGED_SUPPORTING_DOCS[0]] {
        let repo = repo();
        let path = repo.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# User file\n").unwrap();

        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

        assert_eq!(fs::read_to_string(path).unwrap(), "# User file\n");
    }
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
fn setup_backups_restore_previous_marker_absence_from_prior_bundle() {
    let repo = repo();
    let repo_root = fs::canonicalize(repo.path()).unwrap();
    fs::create_dir_all(repo_root.join(".codex1/setup-backups")).unwrap();
    let marker = repo_root.join(".codex1/setup-bundle.json");
    fs::write(
        &marker,
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 7,
            "files": [
                ".agents/skills/codex1/SKILL.md",
                ".agents/skills/codex1/agents/openai.yaml",
                ".agents/skills/clarify/SKILL.md",
                ".agents/skills/clarify/agents/openai.yaml",
                ".agents/skills/clarify/ADR-FORMAT.md",
                ".agents/skills/clarify/CONTEXT-FORMAT.md",
                ".agents/skills/create-prd/SKILL.md",
                ".agents/skills/create-prd/agents/openai.yaml",
                ".agents/skills/create-prd/PRD-FORMAT.md",
                ".agents/skills/plan/SKILL.md",
                ".agents/skills/plan/agents/openai.yaml",
                ".agents/skills/plan/ADR-FORMAT.md",
                ".agents/skills/plan/SUBPLAN-BRIEF.md",
                ".agents/skills/plan/GOAL-BRIEF-FORMAT.md",
                ".agents/skills/tdd/SKILL.md",
                ".agents/skills/tdd/agents/openai.yaml",
                ".agents/skills/tdd/tests.md",
                ".agents/skills/tdd/mocking.md",
                ".agents/skills/tdd/deep-modules.md",
                ".agents/skills/tdd/interface-design.md",
                ".agents/skills/tdd/refactoring.md",
                ".agents/skills/diagnose/SKILL.md",
                ".agents/skills/diagnose/agents/openai.yaml",
                ".agents/skills/diagnose/scripts/hitl-loop.template.sh",
                ".agents/skills/improve-codebase-architecture/SKILL.md",
                ".agents/skills/improve-codebase-architecture/agents/openai.yaml",
                ".agents/skills/improve-codebase-architecture/LANGUAGE.md",
                ".agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md",
                ".agents/skills/improve-codebase-architecture/DEEPENING.md",
                ".agents/skills/prototype/SKILL.md",
                ".agents/skills/prototype/agents/openai.yaml",
                ".agents/skills/prototype/LOGIC.md",
                ".agents/skills/prototype/UI.md",
                ".agents/skills/codex-review/SKILL.md",
                ".agents/skills/codex-review/agents/openai.yaml",
                ".agents/skills/codex-review/scripts/codex-review",
                ".agents/skills/brutal-review/SKILL.md",
                ".agents/skills/brutal-review/agents/openai.yaml",
                ".agents/skills/handoff/SKILL.md",
                ".agents/skills/handoff/agents/openai.yaml",
                "docs/agents/codex1-workflow.md",
                "docs/agents/codex1-domain.md",
                "docs/agents/codex1-artifact-briefs.md",
                "AGENTS.md"
            ]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    fs::write(
        repo_root.join(".codex1/setup-backups/manifest.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "records": [{
                "id": "old-marker-absence",
                "timestamp": "2026-05-02T00:00:00Z",
                "target_kind": "repo-setup",
                "target_path": marker,
                "target_path_label": ".codex1/setup-bundle.json",
                "backup_path": null,
                "existed": false,
                "reason": "bundle marker"
            }]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    json_output(bin().args(["--json", "--repo-root"]).arg(&repo_root).args([
        "setup",
        "backups",
        "restore",
        "old-marker-absence",
        "--force",
    ]));

    assert!(!marker.exists());
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
}

#[test]
fn unknown_setup_options_fail_through_argument_parser() {
    let repo = repo();

    for args in [
        vec!["setup", "nope"],
        vec!["setup", "install", "--bad-flag"],
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
