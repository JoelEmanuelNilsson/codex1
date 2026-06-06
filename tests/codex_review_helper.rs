#![cfg(unix)]

mod common;

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;

use common::*;

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

    assert!(wait_until(Duration::from_secs(10), || review_pid_path
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

    assert!(wait_until(Duration::from_secs(6), || helper
        .try_wait()
        .unwrap()
        .is_some()));
    assert!(wait_until(Duration::from_secs(6), || !process_is_alive(
        review_child_pid
    )));
    assert!(wait_until(Duration::from_secs(6), || !process_is_alive(
        test_child_pid
    )));
}
