//! Wave 4 acceptance: the Ralph contract.
//!
//! Ralph consumes `codex1 status --mission <id> --json` and blocks stop
//! based on `stop_policy.allow_stop`. These tests exercise every
//! (active, paused, verdict) combination the status envelope can emit
//! and assert the derived policy matches the V2 contract table.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn bin(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codex1").expect("binary built");
    cmd.arg("--repo-root").arg(dir.path());
    cmd
}

fn last_json(out: &[u8]) -> Value {
    let s = std::str::from_utf8(out).unwrap();
    serde_json::from_str(s.lines().last().unwrap()).unwrap()
}

fn init(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    // Round 10: status refuses to route past a draft lock, so ratify
    // it here so ralph-policy tests can exercise downstream routing.
    let lock_path = dir.path().join("PLANS/m1/OUTCOME-LOCK.md");
    let content = fs::read_to_string(&lock_path).unwrap();
    fs::write(
        &lock_path,
        content.replace("lock_status: draft", "lock_status: ratified"),
    )
    .unwrap();
}

fn write_blueprint(dir: &Path, yaml_body: &str) {
    let p = dir.join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    fs::write(
        &p,
        format!(
            "# BP\n\n<!-- codex1:plan-dag:start -->\n{yaml_body}\n<!-- codex1:plan-dag:end -->\n"
        ),
    )
    .unwrap();
}

fn write_state(dir: &Path, phase: &str, mode: &str, paused: bool, tasks: &[(&str, &str)]) {
    let sp = dir.join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let mut tasks_obj = serde_json::Map::new();
    for (id, status) in tasks {
        tasks_obj.insert((*id).to_string(), serde_json::json!({ "status": status }));
    }
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": phase,
        "parent_loop": { "mode": mode, "paused": paused },
        "tasks": serde_json::Value::Object(tasks_obj),
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();
}

fn status(dir: &TempDir) -> Value {
    let out = bin(dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    last_json(&out)
}

/// Table-driven: (mode, paused, expected `allow_stop`, expected reason).
#[test]
fn stop_policy_matrix_empty_dag() {
    let cases: &[(&str, bool, bool, &str)] = &[
        ("none", false, true, "no_active_loop"),
        ("execute", false, false, "active_parent_loop"),
        ("execute", true, true, "discussion_pause"),
        ("review", false, false, "active_parent_loop"),
        ("review", true, true, "discussion_pause"),
        ("autopilot", false, false, "active_parent_loop"),
        ("autopilot", true, true, "discussion_pause"),
        ("close", false, false, "active_parent_loop"),
        ("close", true, true, "discussion_pause"),
    ];
    for (mode, paused, allow, reason) in cases {
        let dir = TempDir::new().unwrap();
        init(&dir);
        write_state(dir.path(), "clarify", mode, *paused, &[]);
        let s = status(&dir);
        assert_eq!(
            s["stop_policy"]["allow_stop"],
            serde_json::Value::Bool(*allow),
            "({mode}, paused={paused}) should allow_stop={allow}"
        );
        assert_eq!(
            s["stop_policy"]["reason"], *reason,
            "({mode}, paused={paused}) reason"
        );
    }
}

#[test]
fn stop_policy_for_complete_when_inactive() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(dir.path(), "complete", "none", false, &[("T1", "complete")]);
    let s = status(&dir);
    assert_eq!(s["verdict"], "complete");
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "complete");
}

#[test]
fn stop_policy_for_invalid_state_when_inactive() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    // phase=complete but task=ready → invalid_state.
    write_state(dir.path(), "complete", "none", false, &[("T1", "ready")]);
    let s = status(&dir);
    assert_eq!(s["verdict"], "invalid_state");
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "invalid_state");
}

#[test]
fn active_loop_blocks_even_when_task_eligible() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(
        dir.path(),
        "executing",
        "execute",
        false,
        &[("T1", "ready")],
    );
    let s = status(&dir);
    assert_eq!(s["verdict"], "continue_required");
    assert_eq!(s["stop_policy"]["allow_stop"], false);
    assert_eq!(s["stop_policy"]["reason"], "active_parent_loop");
}

#[test]
fn paused_loop_allows_even_when_task_eligible() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(dir.path(), "executing", "execute", true, &[("T1", "ready")]);
    let s = status(&dir);
    assert_eq!(s["stop_policy"]["allow_stop"], true);
    assert_eq!(s["stop_policy"]["reason"], "discussion_pause");
}

fn ralph_hook_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set");
    Path::new(&manifest)
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .join("scripts/ralph-status-hook.sh")
}

#[test]
fn ralph_scan_blocks_mission_shaped_dir_missing_state_json() {
    // Round 7 P2: a PLANS/<mid>/ directory that looks mission-shaped
    // (has OUTCOME-LOCK.md or PROGRAM-BLUEPRINT.md) but is missing
    // STATE.json is corrupt — the hook must surface it rather than
    // silently allow stop.
    let dir = TempDir::new().unwrap();
    let corrupt = dir.path().join("PLANS/broken");
    fs::create_dir_all(&corrupt).unwrap();
    fs::write(
        corrupt.join("OUTCOME-LOCK.md"),
        b"---\nlock_status: draft\n---\n",
    )
    .unwrap();
    // No STATE.json — simulates a mission where init crashed mid-write
    // or the operator deleted the state by accident.

    let out = std::process::Command::new("bash")
        .arg(ralph_hook_path())
        .arg("--repo-root")
        .arg(dir.path())
        // Round 13: bypass the parent-lane lease gate so this test
        // exercises the scan logic directly without hook plumbing.
        .env("CODEX1_SKIP_LANE_CHECK", "1")
        .output()
        .expect("bash available");
    assert!(
        !out.status.success(),
        "corrupt mission dir must block stop; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("STATE.json missing"),
        "expected corruption reason in stderr, got: {stderr}"
    );
}

#[test]
fn ralph_scan_ignores_truly_empty_subdirs_in_plans() {
    // A PLANS/<name>/ with no mission files is non-mission scratch and
    // must not block stop. Otherwise anything a developer drops into
    // PLANS/ breaks the hook.
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("PLANS/scratch")).unwrap();
    fs::write(dir.path().join("PLANS/scratch/notes.md"), b"just notes\n").unwrap();

    let out = std::process::Command::new("bash")
        .arg(ralph_hook_path())
        .arg("--repo-root")
        .arg(dir.path())
        .env("CODEX1_SKIP_LANE_CHECK", "1")
        .output()
        .expect("bash available");
    assert!(
        out.status.success(),
        "non-mission scratch dir must not block; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn ralph_hook_script_syntax_is_valid() {
    // Ensure the shipped Ralph hook is syntactically valid bash.
    let script = ralph_hook_path();
    assert!(script.exists(), "expected {}", script.display());
    let status = std::process::Command::new("bash")
        .arg("-n")
        .arg(&script)
        .status()
        .expect("bash available");
    assert!(status.success(), "ralph-status-hook.sh failed `bash -n`");
}

/// Round 15 P1: first-claim-wins with PID-based staleness. A second
/// session's `SessionStart` must not overwrite a lease whose pid is
/// still alive — the parent's ownership is sticky. Without this, a
/// subagent / second terminal / env-inheriting process would silently
/// take over the parent lane.
#[test]
fn secondary_session_cannot_steal_lease() {
    use std::io::Write;

    let dir = TempDir::new().unwrap();
    init(&dir);
    let lease_script = ralph_hook_path()
        .parent()
        .unwrap()
        .join("ralph-session-lease.sh");

    // Use the test process's PID as the "parent" — guaranteed alive
    // for the test's lifetime.
    let live_pid = std::process::id().to_string();

    // Session A claims first.
    let mut claim_a = std::process::Command::new("bash")
        .arg(&lease_script)
        .arg("claim")
        .env("CODEX1_REPO_ROOT", dir.path())
        .env("CODEX1_PARENT_PID", &live_pid)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    claim_a
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-A"}"#)
        .unwrap();
    assert!(claim_a.wait().unwrap().success());

    // Session B tries to claim with a DIFFERENT pid (also alive) —
    // the existing lease's pid is still live, so claim must be a
    // no-op regardless of B's own env.
    let mut claim_b = std::process::Command::new("bash")
        .arg(&lease_script)
        .arg("claim")
        .env("CODEX1_REPO_ROOT", dir.path())
        .env("CODEX1_PARENT_PID", &live_pid)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    claim_b
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-B"}"#)
        .unwrap();
    assert!(claim_b.wait().unwrap().success());

    let lease_path = dir.path().join(".codex1/parent-session.json");
    let lease: Value = serde_json::from_slice(&fs::read(&lease_path).unwrap()).unwrap();
    assert_eq!(
        lease["session_id"], "sess-A",
        "first claim wins; a second live-pid claim must not steal"
    );
}

/// Round 15 P1: if the lease's recorded pid is dead (Ralph crashed
/// without releasing), a fresh session's `SessionStart` takes over
/// as stale recovery. Without this, a crashed Ralph would forever
/// prevent the next Ralph from claiming.
#[test]
fn stale_pid_lease_allows_takeover() {
    use std::io::Write;

    let dir = TempDir::new().unwrap();
    init(&dir);
    let lease_script = ralph_hook_path()
        .parent()
        .unwrap()
        .join("ralph-session-lease.sh");

    // Plant a lease whose pid is guaranteed dead.
    let lease_path = dir.path().join(".codex1/parent-session.json");
    fs::create_dir_all(lease_path.parent().unwrap()).unwrap();
    fs::write(
        &lease_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "session_id": "sess-crashed",
            "pid": 99_999_999,
            "claimed_at": "2026-04-19T10:00:00Z",
        }))
        .unwrap(),
    )
    .unwrap();

    // Fresh session C claims with an alive pid → stale recovery.
    let live_pid = std::process::id().to_string();
    let mut claim_c = std::process::Command::new("bash")
        .arg(&lease_script)
        .arg("claim")
        .env("CODEX1_REPO_ROOT", dir.path())
        .env("CODEX1_PARENT_PID", &live_pid)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    claim_c
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-C"}"#)
        .unwrap();
    assert!(claim_c.wait().unwrap().success());

    let lease: Value = serde_json::from_slice(&fs::read(&lease_path).unwrap()).unwrap();
    assert_eq!(
        lease["session_id"], "sess-C",
        "stale lease (dead pid) must allow a fresh claim to take over"
    );
}

/// Round 15 P1: if the lease exists but its pid is dead, the Stop
/// hook must treat it as no-lease and exit 0. Otherwise a crashed
/// parent would leave every future session blocked on a ghost.
#[test]
fn stale_pid_lease_stop_hook_exits_zero() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(
        dir.path(),
        "executing",
        "execute",
        false,
        &[("T1", "ready")],
    );

    // Plant a stale lease (dead pid).
    let lease_path = dir.path().join(".codex1/parent-session.json");
    fs::create_dir_all(lease_path.parent().unwrap()).unwrap();
    fs::write(
        &lease_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "session_id": "sess-crashed",
            "pid": 99_999_999,
            "claimed_at": "2026-04-19T10:00:00Z",
        }))
        .unwrap(),
    )
    .unwrap();

    let out = std::process::Command::new("bash")
        .arg(ralph_hook_path())
        .arg("--repo-root")
        .arg(dir.path())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("bash available");
    assert!(
        out.status.success(),
        "stale-pid lease must not block; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Round 14 P1: when the lease exists but the hook input does not
/// include a `session_id`, the Stop hook must scan (fail-closed) rather
/// than exit 0. Otherwise a non-Claude-Code runtime or a stripped
/// hook payload silently disables Ralph.
#[test]
fn no_session_id_falls_back_to_scan_when_lease_present() {
    use std::io::Write;

    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(
        dir.path(),
        "executing",
        "execute",
        false,
        &[("T1", "ready")],
    );

    // Create a lease so the parent-lane gate has something to inspect.
    let lease_dir = dir.path().join(".codex1");
    fs::create_dir_all(&lease_dir).unwrap();
    fs::write(
        lease_dir.join("parent-session.json"),
        br#"{"session_id": "sess-some-parent"}"#,
    )
    .unwrap();

    // Invoke the Stop hook with NO stdin — HOOK_SESSION_ID stays
    // empty. New semantics: lease present + no session_id → scan →
    // block (loop is active).
    let out = std::process::Command::new("bash")
        .arg(ralph_hook_path())
        .arg("--repo-root")
        .arg(dir.path())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("bash available");
    assert!(
        !out.status.success(),
        "missing session_id with active loop must block; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // For comparison: a Stop with a matching session_id still blocks.
    let mut child = std::process::Command::new("bash")
        .arg(ralph_hook_path())
        .arg("--repo-root")
        .arg(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-some-parent"}"#)
        .unwrap();
    let out2 = child.wait_with_output().unwrap();
    assert!(
        !out2.status.success(),
        "matching parent session still blocks"
    );
}

#[test]
fn ralph_session_lease_script_syntax_is_valid() {
    // Round 13 P1: the SessionStart/SessionEnd lease script must also
    // parse cleanly. Without this a broken lease script would silently
    // fail to claim the parent lane and deployments would lose the
    // Stop-blocking semantics.
    let script = ralph_hook_path()
        .parent()
        .unwrap()
        .join("ralph-session-lease.sh");
    assert!(script.exists(), "expected {}", script.display());
    let status = std::process::Command::new("bash")
        .arg("-n")
        .arg(&script)
        .status()
        .expect("bash available");
    assert!(status.success(), "ralph-session-lease.sh failed `bash -n`");
}

/// Round 13 P1: only the session that holds the parent-lane lease may
/// block stop. Any other session (ad-hoc `claude` in the same repo, a
/// reviewer subagent invoked via Bash, etc.) must exit 0 silently
/// even when a mission has an active unpaused parent loop.
#[test]
fn parent_lane_gate_blocks_only_the_lease_holder() {
    use std::io::Write;

    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    // Active, unpaused loop so status reports allow_stop: false.
    write_state(
        dir.path(),
        "executing",
        "execute",
        false,
        &[("T1", "ready")],
    );

    let hook = ralph_hook_path();
    let lease_script = hook.parent().unwrap().join("ralph-session-lease.sh");

    // Parent session A claims the lease via SessionStart hook.
    // Round 15 P1: no env gate — claim records the parent pid and
    // uses first-claim-wins + PID-based staleness. Test process's
    // PID serves as the "alive parent pid" for the fixture.
    let live_pid = std::process::id().to_string();
    let mut claim = std::process::Command::new("bash")
        .arg(&lease_script)
        .arg("claim")
        .env("CODEX1_REPO_ROOT", dir.path())
        .env("CODEX1_PARENT_PID", &live_pid)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn claim");
    claim
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-A"}"#)
        .unwrap();
    let claim_out = claim.wait_with_output().unwrap();
    assert!(
        claim_out.status.success(),
        "claim must succeed; stderr={}",
        String::from_utf8_lossy(&claim_out.stderr)
    );
    let lease = dir.path().join(".codex1/parent-session.json");
    assert!(lease.exists(), "claim should create the lease");

    // Stop hook from parent session A: blocks (it holds the lease
    // AND the loop is active).
    let mut parent_stop = std::process::Command::new("bash")
        .arg(&hook)
        .arg("--repo-root")
        .arg(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn parent stop");
    parent_stop
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-A"}"#)
        .unwrap();
    let parent_out = parent_stop.wait_with_output().unwrap();
    assert!(
        !parent_out.status.success(),
        "parent session Stop must block when loop is active; stderr={}",
        String::from_utf8_lossy(&parent_out.stderr)
    );

    // Stop hook from a secondary session B: must exit 0 even though
    // the loop is active, because B does NOT own the lease.
    let mut subagent_stop = std::process::Command::new("bash")
        .arg(&hook)
        .arg("--repo-root")
        .arg(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn subagent stop");
    subagent_stop
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-B"}"#)
        .unwrap();
    let sub_out = subagent_stop.wait_with_output().unwrap();
    assert!(
        sub_out.status.success(),
        "subagent session Stop must NOT be blocked by another session's loop; stderr={}",
        String::from_utf8_lossy(&sub_out.stderr)
    );
}

/// Round 13 P1: no lease → exit 0 even if a mission is active. This
/// is the fail-open default for deployments that haven't wired the
/// `SessionStart` hook yet.
#[test]
fn no_parent_lease_fails_open() {
    use std::io::Write;

    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n",
    );
    write_state(
        dir.path(),
        "executing",
        "execute",
        false,
        &[("T1", "ready")],
    );

    let hook = ralph_hook_path();
    let mut child = std::process::Command::new("bash")
        .arg(&hook)
        .arg("--repo-root")
        .arg(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"session_id": "sess-anyone"}"#)
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "no lease must produce exit 0 (fail-open); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
