//! Integration tests for `scripts/ralph-stop-hook.sh`.
//!
//! Each test builds a tempdir with a fake `codex1` bash wrapper that emits a
//! known status payload, prepends the tempdir to `PATH`, and runs the hook
//! script. We assert on exit code and stderr contents.
//!
//! The hook must be status-only: these tests never build real mission state.
//! They only feed the shell script a canned JSON blob and observe how it
//! decides to exit. That is the whole Ralph contract.
//!
//! Tests are skipped gracefully if `bash` is not available on the host.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

fn hook_script_path() -> PathBuf {
    // CARGO_MANIFEST_DIR resolves to <repo>/crates/codex1 at test time.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent() // crates/
        .and_then(Path::parent) // repo root
        .expect("crates/codex1 has a repo root ancestor")
        .join("scripts")
        .join("ralph-stop-hook.sh")
}

fn write_fake_codex1(tmp: &TempDir, body: &str) -> PathBuf {
    let path = tmp.path().join("codex1");
    fs::write(&path, body).expect("write fake codex1");
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// Run the hook with `codex1` stubbed to emit `fake_status_json`.
fn run_hook_with_mocked_codex1(fake_status_json: &str) -> Output {
    let tmp = TempDir::new().expect("tempdir");
    // Escape the JSON for single-quoted bash heredoc-alternative.
    let escaped = fake_status_json.replace('\'', "'\"'\"'");
    let body = format!("#!/usr/bin/env bash\nprintf '%s\\n' '{}'\n", escaped);
    write_fake_codex1(&tmp, &body);
    run_hook_with_path(Some(tmp.path().to_path_buf()))
}

fn run_hook_with_path(prepend: Option<PathBuf>) -> Output {
    let hook = hook_script_path();
    assert!(hook.is_file(), "hook script missing at {}", hook.display());

    // Build PATH: prepend optional tempdir, otherwise strip any codex1 that
    // might be in the environment so we can test the "no codex1 on PATH"
    // case deterministically.
    let path = build_path(prepend.as_deref());

    let mut cmd = Command::new("/bin/bash");
    cmd.arg(&hook);
    // Close stdin, capture stdout/stderr so tests can inspect them.
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.env_clear();
    cmd.env("PATH", path);
    // Preserve HOME so jq/bash don't complain on some shells.
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }

    cmd.output().expect("run hook")
}

fn run_hook_with_exact_path(path: OsString) -> Output {
    let hook = hook_script_path();
    assert!(hook.is_file(), "hook script missing at {}", hook.display());

    let mut cmd = Command::new("/bin/bash");
    cmd.arg(&hook);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.env_clear();
    cmd.env("PATH", path);
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }

    cmd.output().expect("run hook")
}

fn build_path(prepend: Option<&Path>) -> OsString {
    // Start from a safe minimum so jq is discoverable. Drop any entry
    // containing a literal `codex1` binary so the no-codex1 test is
    // deterministic.
    let baseline = std::env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"));
    let mut parts: Vec<PathBuf> = std::env::split_paths(&baseline)
        .filter(|p| !p.join("codex1").exists())
        .collect();
    if let Some(extra) = prepend {
        parts.insert(0, extra.to_path_buf());
    }
    std::env::join_paths(parts).expect("join PATH")
}

fn bash_available() -> bool {
    Command::new("bash")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn stop_allow_true_exits_zero() {
    if !bash_available() {
        eprintln!("skipping: bash not available");
        return;
    }
    let out = run_hook_with_mocked_codex1(
        r#"{"ok":true,"mission_id":"demo","data":{"stop":{"allow":true,"reason":"idle","message":"Stop is allowed."}}}"#,
    );
    assert!(
        out.status.success(),
        "hook should exit 0 when stop.allow=true; stderr: {}",
        stderr_str(&out)
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn stop_allow_false_blocks_with_exit_two() {
    if !bash_available() {
        eprintln!("skipping: bash not available");
        return;
    }
    let out = run_hook_with_mocked_codex1(
        r#"{"ok":true,"mission_id":"demo","data":{"stop":{"allow":false,"reason":"active_loop","message":"Run $close to pause."}}}"#,
    );
    assert_eq!(out.status.code(), Some(2), "expected exit 2 to block Stop");
    let err = stderr_str(&out);
    assert!(
        err.contains("blocking Stop"),
        "stderr should mention blocking Stop; got: {err}"
    );
    assert!(
        err.contains("active_loop"),
        "stderr should include the reason; got: {err}"
    );
}

#[test]
fn empty_status_output_allows_stop() {
    if !bash_available() {
        eprintln!("skipping: bash not available");
        return;
    }
    // A fake codex1 that prints nothing.
    let tmp = TempDir::new().expect("tempdir");
    write_fake_codex1(&tmp, "#!/usr/bin/env bash\nexit 0\n");
    let out = run_hook_with_path(Some(tmp.path().to_path_buf()));
    assert_eq!(out.status.code(), Some(0));
    let err = stderr_str(&out);
    assert!(
        err.contains("empty status output"),
        "stderr should warn about empty output; got: {err}"
    );
}

#[test]
fn missing_codex1_binary_allows_stop_with_warning() {
    if !bash_available() {
        eprintln!("skipping: bash not available");
        return;
    }
    // No prepend -> PATH with codex1 stripped; hook must still exit 0.
    let out = run_hook_with_path(None);
    assert_eq!(out.status.code(), Some(0));
    let err = stderr_str(&out);
    assert!(
        err.contains("codex1 not on PATH"),
        "stderr should warn about missing codex1; got: {err}"
    );
}

#[test]
fn malformed_json_defaults_to_allow() {
    if !bash_available() {
        eprintln!("skipping: bash not available");
        return;
    }
    // No `stop.allow` field -> jq returns `null`, hook falls through to the
    // `*)` branch in the case statement and conservatively allows Stop.
    let out = run_hook_with_mocked_codex1(
        r#"{"ok":true,"mission_id":"demo","data":{"phase":"clarify"}}"#,
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "missing stop.allow should default to allowing Stop; stderr: {}",
        stderr_str(&out)
    );
}

#[cfg(unix)]
#[test]
fn ambiguous_mission_blocks_stop_with_jq_parser() {
    if !bash_available() {
        eprintln!("skipping: bash not available");
        return;
    }
    let out = run_hook_with_mocked_codex1(
        r#"{"ok":false,"code":"MISSION_NOT_FOUND","message":"ambiguous","context":{"ambiguous":true}}"#,
    );
    assert_eq!(out.status.code(), Some(2));
    let err = stderr_str(&out);
    assert!(
        err.contains("ambiguous Codex1 mission"),
        "stderr should mention ambiguity; got: {err}"
    );
}

#[cfg(unix)]
#[test]
fn ambiguous_mission_blocks_stop_without_jq_parser() {
    use std::os::unix::fs::symlink;

    if !bash_available() {
        eprintln!("skipping: bash not available");
        return;
    }
    let tmp = TempDir::new().expect("tempdir");
    let escaped = r#"{"ok":false,"code":"MISSION_NOT_FOUND","context":{"ambiguous":true}}"#
        .replace('\'', "'\"'\"'");
    write_fake_codex1(&tmp, &format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", escaped));
    for (name, path) in [
        ("cat", "/bin/cat"),
        ("grep", "/usr/bin/grep"),
        ("head", "/usr/bin/head"),
        ("awk", "/usr/bin/awk"),
        ("tr", "/usr/bin/tr"),
    ] {
        symlink(path, tmp.path().join(name)).expect("symlink fallback tool");
    }
    let out = run_hook_with_exact_path(tmp.path().as_os_str().to_os_string());
    assert_eq!(out.status.code(), Some(2));
    let err = stderr_str(&out);
    assert!(
        err.contains("ambiguous Codex1 mission"),
        "stderr should mention ambiguity in fallback mode; got: {err}"
    );
}

#[test]
fn hook_script_is_executable() {
    let hook = hook_script_path();
    let meta = fs::metadata(&hook).expect("hook script exists");
    let mode = meta.permissions().mode();
    assert!(
        mode & 0o111 != 0,
        "hook script must be executable; mode={mode:o}"
    );
}
