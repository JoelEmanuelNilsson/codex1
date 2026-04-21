//! Foundation integration tests.
//!
//! Exercises `codex1 init`, `codex1 doctor`, `codex1 status`, the stub
//! command dispatches, STATE.json/EVENTS.jsonl atomicity, and envelope
//! shape. Phase B units add their own integration tests under
//! `tests/<unit>.rs`.

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn init_demo(tmp: &TempDir, mission: &str) -> PathBuf {
    let mission_dir = tmp.path().join("PLANS").join(mission);
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", mission])
        .assert()
        .success();
    mission_dir
}

fn parse_stdout_json(output: &std::process::Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(stdout).unwrap_or_else(|e| {
        panic!("expected JSON stdout, got:\n{stdout}\nerror: {e}");
    })
}

#[test]
fn help_prints_full_command_tree() {
    let output = cmd().arg("--help").output().expect("runs");
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "init", "doctor", "hook", "outcome", "plan", "task", "review", "replan", "loop", "close",
        "status",
    ] {
        assert!(
            text.contains(expected),
            "help missing `{expected}`:\n{text}"
        );
    }
}

#[test]
fn doctor_runs_without_auth() {
    let output = cmd().arg("doctor").output().expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert!(json["data"]["version"].is_string());
    assert_eq!(json["data"]["auth"]["required"], Value::Bool(false));
}

#[test]
fn init_creates_mission_scaffold() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    assert!(mission_dir.join("STATE.json").is_file());
    assert!(mission_dir.join("OUTCOME.md").is_file());
    assert!(mission_dir.join("PLAN.yaml").is_file());
    assert!(mission_dir.join("EVENTS.jsonl").is_file());
    assert!(mission_dir.join("specs").is_dir());
    assert!(mission_dir.join("reviews").is_dir());

    let state: Value =
        serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap();
    assert_eq!(state["mission_id"], "demo");
    assert_eq!(state["revision"], 0);
    assert_eq!(state["schema_version"], 1);
    assert_eq!(state["phase"], "clarify");
    assert_eq!(state["outcome"]["ratified"], false);
    assert_eq!(state["plan"]["locked"], false);
    assert_eq!(state["loop"]["active"], false);
}

#[test]
fn init_refuses_to_overwrite() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(false));
    assert_eq!(json["code"], "STATE_CORRUPT");
}

#[test]
fn init_rejects_mission_id_path_traversal() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "../escape"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "MISSION_NOT_FOUND");
    assert!(
        !tmp.path().join("escape").exists(),
        "invalid mission id must not create files outside PLANS/"
    );
}

#[test]
fn init_rejects_absolute_mission_id() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "/tmp/codex1-escape"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "MISSION_NOT_FOUND");
}

#[cfg(unix)]
#[test]
fn init_rejects_symlinked_mission_directory() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let outside = tmp.path().join("outside");
    let plans = tmp.path().join("PLANS");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&plans).unwrap();
    symlink(&outside, plans.join("demo")).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(
        !outside.join("STATE.json").exists(),
        "init must not write through a symlinked mission dir"
    );
}

#[cfg(unix)]
#[test]
fn mutation_rejects_symlinked_events_file() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let outside = tmp.path().join("outside-events.jsonl");
    fs::remove_file(mission_dir.join("EVENTS.jsonl")).unwrap();
    symlink(&outside, mission_dir.join("EVENTS.jsonl")).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["loop", "activate", "--mode", "execute", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(!outside.exists(), "must not append through EVENTS symlink");
}

#[cfg(unix)]
#[test]
fn status_rejects_symlinked_state_lock() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let outside = tmp.path().join("outside-lock");
    fs::remove_file(mission_dir.join("STATE.json.lock")).ok();
    symlink(&outside, mission_dir.join("STATE.json.lock")).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(!outside.exists(), "must not create lock through symlink");
}

#[cfg(unix)]
#[test]
fn status_rejects_symlinked_state_file() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let outside = tmp.path().join("outside-state.json");
    let mut state: Value =
        serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap();
    state["mission_id"] = Value::String("outside".to_string());
    state["revision"] = Value::Number(42.into());
    fs::write(&outside, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    fs::remove_file(mission_dir.join("STATE.json")).unwrap();
    symlink(&outside, mission_dir.join("STATE.json")).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "PLAN_INVALID");
}

#[test]
fn status_resolves_existing_mission_and_reports_stop_allowed() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["stop"]["allow"], true);
}

#[test]
fn status_without_mission_reports_needs_user() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["verdict"], "needs_user");
    assert_eq!(json["data"]["stop"]["allow"], true);
    assert_eq!(json["data"]["foundation_only"], true);
}

// `outcome_stubs_return_not_implemented` removed: Phase B Unit 2
// (cli-outcome) has replaced the stub with the real implementation.
// See `tests/outcome.rs` for the Phase B integration coverage.

// `plan_stubs_return_not_implemented` and `review_stubs_return_not_implemented`
// removed: every Phase B sub-command group now has a real implementation;
// per-command coverage lives in tests/{outcome,plan_check,plan_waves,
// plan_scaffold,task,review,replan,loop_,close,status,status_close_agreement}.rs.

#[test]
fn mission_not_found_has_helpful_hint() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "does-not-exist"])
        .output()
        .expect("runs");
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "MISSION_NOT_FOUND");
    assert!(json["hint"].is_string());
}

#[test]
fn bare_status_reports_ambiguous_missions() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "one");
    init_demo(&tmp, "two");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["status"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["code"], "MISSION_NOT_FOUND");
    assert_eq!(json["context"]["ambiguous"], true);
}

#[test]
fn hook_snippet_is_informational_only() {
    let output = cmd().args(["hook", "snippet"]).output().expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["hook"]["event"], "Stop");
    assert!(json["data"]["install"].is_object());
}

#[test]
fn init_dry_run_does_not_create_files() {
    let tmp = TempDir::new().unwrap();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", "demo", "--dry-run"])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let json = parse_stdout_json(&output);
    assert_eq!(json["data"]["dry_run"], true);
    assert!(!tmp.path().join("PLANS").exists());
}

#[test]
fn events_jsonl_is_append_only_friendly() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let events = mission_dir.join("EVENTS.jsonl");
    // Empty after init (no mutations yet — revision 0 is the initial state).
    let content = fs::read_to_string(&events).unwrap();
    assert_eq!(content, "");
}

/// Regression for test-adequacy round-1 P2-3: assert the full error
/// envelope shape for three representative error codes so a regression
/// that silently drops `hint`, flips `retryable`, or reshapes
/// `context` is caught.
#[test]
fn error_envelope_shape_is_stable_across_representative_codes() {
    // MISSION_NOT_FOUND — hint expected, retryable=false.
    let tmp = TempDir::new().unwrap();
    let json = parse_stdout_json(
        &cmd()
            .current_dir(tmp.path())
            .args(["status", "--mission", "does-not-exist"])
            .output()
            .expect("runs"),
    );
    assert_eq!(json["ok"], Value::Bool(false));
    assert_eq!(json["code"], "MISSION_NOT_FOUND");
    assert!(json["message"].as_str().is_some_and(|m| !m.is_empty()));
    assert_eq!(json["retryable"], Value::Bool(false));
    assert!(json["hint"].is_string());

    // REVISION_CONFLICT — retryable=true, context has expected+actual.
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let json = parse_stdout_json(
        &cmd()
            .current_dir(tmp.path())
            .args([
                "loop",
                "activate",
                "--mission",
                "demo",
                "--expect-revision",
                "99",
            ])
            .output()
            .expect("runs"),
    );
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["retryable"], Value::Bool(true));
    assert_eq!(json["context"]["expected"], 99);
    // Actual revision for a freshly-init'd mission is 0.
    assert_eq!(json["context"]["actual"], 0);
    assert!(json["hint"].is_string());
}

/// Regression for round-3 test-adequacy P2-1: the `STATE_CORRUPT`
/// parse-failure branch in `state::load` must surface as a `STATE_CORRUPT`
/// envelope end-to-end. Round-1 P1-4 covered PLAN.yaml parse (→
/// `PLAN_INVALID`) and the `Io → PARSE_ERROR` mapping, but the
/// `serde_json::from_str` failure branch at `src/state/mod.rs:84`
/// (reached on bad JSON in STATE.json) had no direct integration
/// trigger. Prior coverage only hit the `:159` refuse-to-overwrite
/// branch via `init_refuses_to_overwrite`.
#[test]
fn state_corrupt_envelope_on_invalid_state_json() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    // Overwrite STATE.json with garbage bytes. The lock file is left
    // intact so acquire_shared_lock succeeds and the failure is isolated
    // to the JSON parser.
    fs::write(mission_dir.join("STATE.json"), "{ this is not json").expect("write garbage");

    let output = cmd()
        .current_dir(tmp.path())
        .args(["status", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(
        !output.status.success(),
        "status on corrupt STATE.json must fail: {output:?}"
    );
    let json = parse_stdout_json(&output);
    assert_eq!(json["ok"], Value::Bool(false));
    assert_eq!(json["code"], "STATE_CORRUPT");
    assert!(
        json["message"]
            .as_str()
            .is_some_and(|m| m.contains("Failed to parse STATE.json")),
        "message should reference STATE.json parse failure: {json}"
    );
    assert_eq!(json["retryable"], Value::Bool(false));
}

/// Regression for test-adequacy round-1 P2-2 / correctness-invariants
/// P2-1: the fs2 exclusive lock on `STATE.json.lock` must serialize
/// concurrent writers. Two `codex1 loop activate` processes racing on
/// the same mission with `--expect-revision 0` must produce exactly
/// one success (revision=1) and exactly one `REVISION_CONFLICT`. The
/// test proves (a) lock serialization and (b) `--expect-revision`
/// fail-closed under contention.
#[test]
fn concurrent_loop_activate_serializes_via_fs2_lock() {
    use std::sync::Arc;
    use std::thread;

    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let root = Arc::new(tmp.path().to_path_buf());

    let r1 = root.clone();
    let h1 = thread::spawn(move || {
        Command::cargo_bin("codex1")
            .expect("binary builds")
            .current_dir(&*r1)
            .args([
                "loop",
                "activate",
                "--mission",
                "demo",
                "--expect-revision",
                "0",
            ])
            .output()
            .expect("runs")
    });
    let r2 = root.clone();
    let h2 = thread::spawn(move || {
        Command::cargo_bin("codex1")
            .expect("binary builds")
            .current_dir(&*r2)
            .args([
                "loop",
                "activate",
                "--mission",
                "demo",
                "--expect-revision",
                "0",
            ])
            .output()
            .expect("runs")
    });
    let o1 = h1.join().unwrap();
    let o2 = h2.join().unwrap();

    let successes: usize = [&o1, &o2].iter().filter(|o| o.status.success()).count();
    assert_eq!(
        successes,
        1,
        "exactly one activate should succeed; got both success={:?}, both stdout=\n{}\n---\n{}",
        (o1.status.success(), o2.status.success()),
        String::from_utf8_lossy(&o1.stdout),
        String::from_utf8_lossy(&o2.stdout),
    );
    // The other one must surface REVISION_CONFLICT (not a raw deadlock /
    // corrupt error, and not NotImplemented).
    let failing = if o1.status.success() { &o2 } else { &o1 };
    let json: Value = serde_json::from_slice(&failing.stdout).expect("failing run emits JSON");
    assert_eq!(json["ok"], Value::Bool(false));
    assert_eq!(json["code"], "REVISION_CONFLICT");

    // Final revision on disk is exactly 1 (one successful write).
    let state: Value = serde_json::from_str(
        &fs::read_to_string(tmp.path().join("PLANS/demo/STATE.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(state["revision"], 1);
    // EVENTS.jsonl has exactly one new event line with seq=1.
    let events = fs::read_to_string(tmp.path().join("PLANS/demo/EVENTS.jsonl")).unwrap();
    let lines: Vec<&str> = events.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "exactly one event line expected; got {}: {events}",
        lines.len()
    );
    let evt: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(evt["seq"], 1);
}

/// Regression for test-adequacy round-1 P1-4: a corrupt PLAN.yaml must
/// surface `PLAN_INVALID` through the named-conversion error path so
/// callers get the canonical code. (The reviewer asked for PARSE_ERROR
/// as an alternative, but the production path at
/// `cli/plan/check.rs:52` translates bad YAML into `CliError::PlanInvalid`
/// with a hint — which is the actually-reachable shape.)
#[test]
fn corrupt_plan_yaml_returns_plan_invalid_with_hint() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["outcome"] = serde_json::json!({
        "ratified": true,
        "ratified_at": "2026-04-21T00:00:00Z"
    });
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    // Overwrite the scaffolded PLAN.yaml with invalid YAML.
    fs::write(
        mission_dir.join("PLAN.yaml"),
        "tasks:\n  - id: T1\n    depends_on: [unterminated",
    )
    .unwrap();
    let json = parse_stdout_json(
        &cmd()
            .current_dir(tmp.path())
            .args(["plan", "check", "--mission", "demo"])
            .output()
            .expect("runs"),
    );
    assert_eq!(json["ok"], Value::Bool(false));
    assert_eq!(json["code"], "PLAN_INVALID");
    assert!(json["hint"].is_string());
}

/// Regression for round-2 correctness P1-1: `require_plan_locked` is
/// now re-checked inside the exclusive-lock mutate closure so a
/// concurrent `replan record` that unlocks the plan between the
/// pre-mutate shared-lock load and the closure cannot leave
/// `task.status == InProgress` sitting on top of `!plan.locked`.
///
/// This test races two processes: one runs `task start T2`, the other
/// runs `replan record --reason six_dirty --supersedes T2`. Neither
/// passes `--expect-revision`, so the revision-conflict check does
/// NOT catch the race; only the in-closure plan-locked re-check does.
/// The assertion is one-sided: the final on-disk state must never be
/// the corrupt shape `!plan.locked && task.status == "in_progress"`.
#[test]
fn concurrent_replan_and_task_start_preserves_plan_locked_invariant() {
    use serde_json::json;
    use std::sync::Arc;
    use std::thread;

    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    // Seed a plan-locked mission with T1 complete and T2 ready; also
    // seed the dirty counter at threshold so `replan record` has a
    // realistic reason to fire.
    let plan = r"mission_id: demo

planning_level:
  requested: light
  effective: light

outcome_interpretation:
  summary: toctou race regression

architecture:
  summary: toctou race regression
  key_decisions:
    - one

planning_process:
  evidence:
    - kind: direct_reasoning
      summary: x

tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: Under race
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md

risks:
  - risk: x
    mitigation: y

mission_close:
  criteria:
    - ok
";
    fs::write(mission_dir.join("PLAN.yaml"), plan).unwrap();
    for tid in ["T1", "T2"] {
        let dir = mission_dir.join("specs").join(tid);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SPEC.md"), format!("# {tid}\n")).unwrap();
    }
    let mut state: Value =
        serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap();
    state["outcome"] = json!({ "ratified": true, "ratified_at": "2026-04-20T00:00:00Z" });
    state["plan"]["locked"] = Value::Bool(true);
    state["plan"]["task_ids"] = json!(["T1", "T2"]);
    state["phase"] = Value::String("execute".into());
    state["replan"]["consecutive_dirty_by_target"] = json!({ "T2": 6 });
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "started_at": "2026-04-20T00:00:00Z",
            "finished_at": "2026-04-20T00:00:01Z",
            "superseded_by": null,
        },
        "T2": {
            "id": "T2",
            "status": "ready",
            "superseded_by": null,
        },
    });
    fs::write(
        mission_dir.join("STATE.json"),
        serde_json::to_vec_pretty(&state).unwrap(),
    )
    .unwrap();

    // Run both races multiple times. Even if the race is rarely
    // triggered, the invariant check must hold on every iteration.
    for _ in 0..4 {
        // Reset state before each iteration.
        fs::write(
            mission_dir.join("STATE.json"),
            serde_json::to_vec_pretty(&state).unwrap(),
        )
        .unwrap();
        // Blank EVENTS.jsonl so every iteration starts clean.
        fs::write(mission_dir.join("EVENTS.jsonl"), "").unwrap();

        let root = Arc::new(tmp.path().to_path_buf());
        let r1 = root.clone();
        let r2 = root.clone();
        let h_task = thread::spawn(move || {
            Command::cargo_bin("codex1")
                .expect("binary builds")
                .current_dir(&*r1)
                .args(["task", "start", "T2", "--mission", "demo"])
                .output()
                .expect("runs")
        });
        let h_replan = thread::spawn(move || {
            Command::cargo_bin("codex1")
                .expect("binary builds")
                .current_dir(&*r2)
                .args([
                    "replan",
                    "record",
                    "--mission",
                    "demo",
                    "--reason",
                    "six_dirty",
                    "--supersedes",
                    "T2",
                ])
                .output()
                .expect("runs")
        });
        h_task.join().unwrap();
        h_replan.join().unwrap();

        let end: Value =
            serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap())
                .unwrap();
        let plan_locked = end["plan"]["locked"].as_bool().unwrap_or(true);
        let t2_status = end["tasks"]["T2"]["status"].as_str().unwrap_or("");
        assert!(
            plan_locked || t2_status != "in_progress",
            "invariant violated: plan.locked={plan_locked} && tasks.T2.status={t2_status}; end state:\n{end:#}"
        );
    }
}
