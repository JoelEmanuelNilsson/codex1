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

pub const MANAGED_SKILLS: [&str; 8] = [
    ".agents/skills/clarify/SKILL.md",
    ".agents/skills/create-prd/SKILL.md",
    ".agents/skills/plan/SKILL.md",
    ".agents/skills/tdd/SKILL.md",
    ".agents/skills/diagnose/SKILL.md",
    ".agents/skills/improve-codebase-architecture/SKILL.md",
    ".agents/skills/codex-review/SKILL.md",
    ".agents/skills/handoff/SKILL.md",
];

pub const MANAGED_SUPPORTING_DOCS: [&str; 27] = [
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
    ".agents/skills/codex-review/agents/openai.yaml",
    ".agents/skills/codex-review/scripts/codex-review",
    ".agents/skills/handoff/agents/openai.yaml",
    "docs/agents/codex1-workflow.md",
    "docs/agents/codex1-domain.md",
    "docs/agents/codex1-artifact-briefs.md",
];

pub const LEGACY_EXECUTION_PROMPT_FORMAT_BODY: &str = r#"# Execution Prompt Format

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
