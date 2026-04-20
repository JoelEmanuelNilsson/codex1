//! End-to-end Ralph Stop hook contract tests.
//!
//! Drives `scripts/ralph-stop-hook.sh` directly with a fake `codex1` bash
//! wrapper on PATH. The hook must be purely status-JSON driven:
//!
//! - `stop.allow: true`  -> exit 0
//! - `stop.allow: false` -> exit 2, stderr mentions "blocking"
//! - empty / missing JSON -> exit 0 (conservative default)
//! - no `codex1` on PATH -> exit 0 with warning
//! - malformed / missing `stop.allow` field -> exit 0 (conservative default)
//!
//! The sibling `tests/ralph_hook.rs` covers similar ground; this file is
//! the Unit 20 e2e driver and duplicates just enough setup to stand alone.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// Repo-root-relative path to the Ralph Stop hook script.
fn hook_script() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("crates/codex1 has a repo root ancestor")
        .join("scripts")
        .join("ralph-stop-hook.sh")
}

/// Write an executable bash script at `tmp/codex1` with the given body.
fn write_fake_codex1(tmp: &TempDir, body: &str) -> PathBuf {
    let path = tmp.path().join("codex1");
    fs::write(&path, body).expect("write fake codex1");
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// Build a PATH with `prepend` at the front and any real `codex1` stripped
/// so the "no codex1" case is deterministic across dev environments.
fn sanitized_path(prepend: Option<&Path>) -> OsString {
    let base = std::env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"));
    let mut parts: Vec<PathBuf> = std::env::split_paths(&base)
        .filter(|p| !p.join("codex1").exists())
        .collect();
    if let Some(extra) = prepend {
        parts.insert(0, extra.to_path_buf());
    }
    std::env::join_paths(parts).expect("join PATH")
}

/// Invoke `ralph-stop-hook.sh` via bash with stdin closed. The returned
/// `Output` carries exit code plus stderr for assertions.
fn run_hook(prepend: Option<&Path>) -> Output {
    let hook = hook_script();
    assert!(hook.is_file(), "hook script missing: {}", hook.display());
    let mut cmd = Command::new("bash");
    cmd.arg(&hook)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env_clear()
        .env("PATH", sanitized_path(prepend));
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    cmd.output().expect("run ralph-stop-hook.sh")
}

/// Convenience: fabricate a fake `codex1` printing `status_json`, then run.
fn run_hook_with_status(status_json: &str) -> (Output, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    // Escape single quotes for embedding inside a shell-single-quoted literal.
    let escaped = status_json.replace('\'', "'\"'\"'");
    let body = format!("#!/usr/bin/env bash\nprintf '%s\\n' '{escaped}'\n");
    write_fake_codex1(&tmp, &body);
    let path = tmp.path().to_path_buf();
    let out = run_hook(Some(&path));
    (out, tmp)
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn bash_available() -> bool {
    Command::new("bash")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn e2e_stop_allow_true_exits_zero() {
    if !bash_available() {
        eprintln!("skipping: bash unavailable");
        return;
    }
    let (out, _tmp) = run_hook_with_status(
        r#"{"ok":true,"mission_id":"demo","data":{"stop":{"allow":true,"reason":"idle","message":"Stop is allowed."}}}"#,
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}

#[test]
fn e2e_stop_allow_false_blocks_with_exit_two() {
    if !bash_available() {
        eprintln!("skipping: bash unavailable");
        return;
    }
    let (out, _tmp) = run_hook_with_status(
        r#"{"ok":true,"mission_id":"demo","data":{"stop":{"allow":false,"reason":"active_loop","message":"Run $close to pause."}}}"#,
    );
    assert_eq!(out.status.code(), Some(2), "expected exit 2 to block Stop");
    let err = stderr(&out);
    assert!(
        err.contains("blocking"),
        "stderr should mention blocking; got: {err}"
    );
}

#[test]
fn e2e_empty_status_output_allows_stop() {
    if !bash_available() {
        eprintln!("skipping: bash unavailable");
        return;
    }
    let tmp = TempDir::new().expect("tempdir");
    // A codex1 that prints nothing (hook must default to allowing Stop).
    write_fake_codex1(&tmp, "#!/usr/bin/env bash\nexit 0\n");
    let out = run_hook(Some(tmp.path()));
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stderr(&out).contains("empty status output"),
        "stderr should warn about empty output; got: {}",
        stderr(&out)
    );
}

#[test]
fn e2e_missing_codex1_allows_stop_with_warning() {
    if !bash_available() {
        eprintln!("skipping: bash unavailable");
        return;
    }
    let out = run_hook(None); // PATH stripped of any codex1
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stderr(&out).contains("codex1 not on PATH"),
        "stderr should warn about missing codex1; got: {}",
        stderr(&out)
    );
}

#[test]
fn e2e_malformed_json_defaults_to_allow() {
    if !bash_available() {
        eprintln!("skipping: bash unavailable");
        return;
    }
    // `stop.allow` absent: the hook must conservatively allow Stop rather
    // than misread `null` as `false` and block Ralph forever.
    let (out, _tmp) =
        run_hook_with_status(r#"{"ok":true,"mission_id":"demo","data":{"phase":"clarify"}}"#);
    assert_eq!(
        out.status.code(),
        Some(0),
        "missing stop.allow should allow Stop; stderr: {}",
        stderr(&out)
    );
}
