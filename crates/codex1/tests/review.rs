//! Integration tests for the `codex1 review` lifecycle.
//!
//! Seeds a small mission (T1..T4 work, T5 review targeting [T2,T3]) by
//! writing STATE.json and PLAN.yaml directly under a TempDir. Drives the
//! CLI through `assert_cmd` to exercise the public contract.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

const MISSION: &str = "demo";

struct Seeded {
    tmp: TempDir,
    mission_dir: PathBuf,
}

impl Seeded {
    fn new() -> Self {
        Self::with_targets(&["T2", "T3"])
    }

    /// Seed a mission with:
    /// - T1 work (Complete)
    /// - each target in `targets` as AwaitingReview
    /// - T5 review targeting those targets
    fn with_targets(targets: &[&str]) -> Self {
        let tmp = TempDir::new().unwrap();
        cmd()
            .current_dir(tmp.path())
            .args(["init", "--mission", MISSION])
            .assert()
            .success();
        let mission_dir = tmp.path().join("PLANS").join(MISSION);
        write_plan(&mission_dir, targets);
        set_targets_awaiting(&mission_dir, targets);
        write_specs(&mission_dir, targets);
        Self { tmp, mission_dir }
    }

    fn path(&self) -> &Path {
        self.tmp.path()
    }

    fn review_file(&self) -> PathBuf {
        self.mission_dir.join("reviews").join("T5.md")
    }
}

fn write_plan(mission_dir: &Path, targets: &[&str]) {
    use std::fmt::Write;
    let mut tasks = String::new();
    // T1 is a root work task already complete in the seeded state below.
    tasks.push_str(
        "  - id: T1\n    title: Root\n    kind: code\n    depends_on: []\n    spec: specs/T1/SPEC.md\n",
    );
    for t in targets {
        let _ = write!(
            tasks,
            "  - id: {t}\n    title: Work {t}\n    kind: code\n    depends_on: [T1]\n    spec: specs/{t}/SPEC.md\n    write_paths:\n      - src/{t}/**\n",
        );
    }
    tasks.push_str("  - id: T4\n    title: Later\n    kind: code\n    depends_on: [T1]\n    spec: specs/T4/SPEC.md\n");
    let target_list = targets
        .iter()
        .map(|t| (*t).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let review_deps = targets
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let plan = format!(
        r"mission_id: {MISSION}

planning_level:
  requested: light
  effective: light

outcome_interpretation:
  summary: 'test'

architecture:
  summary: 'test'
  key_decisions: []

planning_process:
  evidence: []

tasks:
{tasks}  - id: T5
    title: Review wave
    kind: review
    depends_on: [{review_deps}]
    spec: specs/T5/SPEC.md
    review_target:
      tasks: [{target_list}]
    review_profiles:
      - code_bug_correctness
      - local_spec_intent

risks: []

mission_close:
  criteria: []
"
    );
    fs::write(mission_dir.join("PLAN.yaml"), plan).unwrap();
}

fn write_specs(mission_dir: &Path, targets: &[&str]) {
    let specs_dir = mission_dir.join("specs");
    for tid in std::iter::once("T1")
        .chain(targets.iter().copied())
        .chain(["T4", "T5"])
    {
        let dir = specs_dir.join(tid);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SPEC.md"),
            format!("# Spec for {tid}\n\nDo the thing.\n"),
        )
        .unwrap();
        // Only targets get a PROOF.md (they're "done").
        if targets.contains(&tid) {
            fs::write(
                dir.join("PROOF.md"),
                format!("# Proof for {tid}\n\ncargo test passed.\n"),
            )
            .unwrap();
        }
    }
}

/// Mutate STATE.json in place: mark T1 Complete and each target as AwaitingReview.
fn set_targets_awaiting(mission_dir: &Path, targets: &[&str]) {
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let tasks = state["tasks"].as_object_mut().unwrap();
    tasks.insert(
        "T1".into(),
        serde_json::json!({
            "id": "T1",
            "status": "complete",
            "started_at": "2026-04-20T00:00:00Z",
            "finished_at": "2026-04-20T00:00:01Z",
            "superseded_by": null,
        }),
    );
    for t in targets {
        tasks.insert(
            (*t).into(),
            serde_json::json!({
                "id": *t,
                "status": "awaiting_review",
                "started_at": "2026-04-20T00:00:00Z",
                "finished_at": "2026-04-20T00:00:01Z",
                "superseded_by": null,
            }),
        );
    }
    // Also mark outcome ratified + plan locked so status is cleaner in the tests.
    state["outcome"] =
        serde_json::json!({ "ratified": true, "ratified_at": "2026-04-20T00:00:00Z" });
    state["plan"]["locked"] = Value::Bool(true);
    state["phase"] = Value::String("execute".into());
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
}

fn parse_stdout(output: &std::process::Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(stdout)
        .unwrap_or_else(|e| panic!("expected JSON stdout, got:\n{stdout}\nerror: {e}"))
}

fn run_ok(tmp: &Path, args: &[&str]) -> Value {
    let output = cmd().current_dir(tmp).args(args).output().expect("runs");
    assert!(
        output.status.success(),
        "expected success, got:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    parse_stdout(&output)
}

fn run_err(tmp: &Path, args: &[&str]) -> Value {
    let output = cmd().current_dir(tmp).args(args).output().expect("runs");
    assert!(
        !output.status.success(),
        "expected failure:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    parse_stdout(&output)
}

fn set_task_status(mission_dir: &Path, task_id: &str, status: &str) {
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let task = state["tasks"]
        .as_object_mut()
        .unwrap()
        .entry(task_id.to_string())
        .or_insert_with(|| {
            serde_json::json!({
                "id": task_id,
                "status": "pending",
                "superseded_by": null,
            })
        });
    task["status"] = Value::String(status.into());
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
}

fn bump_revision_without_mutation(mission_dir: &Path) {
    // Simulate a concurrent mutation so the next record classifies as
    // late_same_boundary. Incrementing the revision is the minimum that
    // the classifier keys off.
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let rev = state["revision"].as_u64().unwrap();
    state["revision"] = serde_json::json!(rev + 1);
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
}

fn set_terminal(mission_dir: &Path) {
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["close"] = serde_json::json!({
        "review_state": "passed",
        "terminal_at": "2026-04-20T00:00:00Z",
    });
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
}

fn set_target_superseded(mission_dir: &Path, task_id: &str) {
    let state_path = mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let tasks = state["tasks"].as_object_mut().unwrap();
    let task = tasks.get_mut(task_id).unwrap();
    task["status"] = Value::String("superseded".into());
    task["superseded_by"] = Value::String("T99".into());
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
}

#[test]
fn t1_start_before_targets_finished_returns_task_not_ready() {
    let s = Seeded::new();
    set_task_status(&s.mission_dir, "T2", "in_progress");
    let err = run_err(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    assert_eq!(err["code"], "TASK_NOT_READY");
}

#[test]
fn t2_start_records_pending_and_event() {
    let s = Seeded::new();
    let ok = run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    assert_eq!(ok["ok"], Value::Bool(true));
    assert_eq!(ok["data"]["review_task_id"], "T5");
    assert_eq!(ok["data"]["verdict"], "pending");

    // Record appears in STATE.
    let state: Value =
        serde_json::from_str(&fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap())
            .unwrap();
    assert_eq!(state["reviews"]["T5"]["verdict"], "pending");

    // Event is in EVENTS.jsonl.
    let events = fs::read_to_string(s.mission_dir.join("EVENTS.jsonl")).unwrap();
    assert!(events.contains("review.started"), "events: {events}");
}

#[test]
fn t3_record_clean_transitions_targets_and_resets_streak() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    // Seed a prior dirty streak so we can confirm the reset.
    let state_path = s.mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["replan"]["consecutive_dirty_by_target"] = serde_json::json!({ "T2": 2, "T3": 1 });
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let ok = run_ok(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--clean",
            "--reviewers",
            "a,b",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(ok["data"]["verdict"], "clean");
    assert_eq!(ok["data"]["category"], "accepted_current");
    assert_eq!(ok["data"]["replan_triggered"], false);
    let reviewers = ok["data"]["reviewers"].as_array().unwrap();
    assert_eq!(reviewers.len(), 2);

    let state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(state["tasks"]["T2"]["status"], "complete");
    assert_eq!(state["tasks"]["T3"]["status"], "complete");
    assert_eq!(state["replan"]["consecutive_dirty_by_target"]["T2"], 0);
    assert_eq!(state["replan"]["consecutive_dirty_by_target"]["T3"], 0);
}

#[test]
fn t4_record_findings_increments_dirty_and_copies_file() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);

    let findings_src = s.path().join("findings.md");
    fs::write(&findings_src, "# Findings\n- P1: something is off.\n").unwrap();

    let ok = run_ok(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--findings-file",
            findings_src.to_str().unwrap(),
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(ok["data"]["verdict"], "dirty");
    assert_eq!(ok["data"]["category"], "accepted_current");

    let state: Value =
        serde_json::from_str(&fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap())
            .unwrap();
    assert_eq!(state["replan"]["consecutive_dirty_by_target"]["T2"], 1);
    assert_eq!(state["replan"]["consecutive_dirty_by_target"]["T3"], 1);

    // File was copied to reviews/T5.md.
    let stored = s.review_file();
    assert!(stored.is_file(), "expected {} to exist", stored.display());
    assert!(fs::read_to_string(&stored)
        .unwrap()
        .contains("P1: something"));
    // Source is untouched.
    assert!(findings_src.is_file());
}

#[test]
fn t5_six_dirty_triggers_replan() {
    let s = Seeded::with_targets(&["T2"]);
    let findings_src = s.path().join("findings.md");
    fs::write(&findings_src, "# F\n- P0: bug\n").unwrap();

    for i in 0..6 {
        // Reset T2 to AwaitingReview so `review start` accepts it each round.
        set_task_status(&s.mission_dir, "T2", "awaiting_review");
        run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
        let ok = run_ok(
            s.path(),
            &[
                "review",
                "record",
                "T5",
                "--findings-file",
                findings_src.to_str().unwrap(),
                "--mission",
                MISSION,
            ],
        );
        let triggered = ok["data"]["replan_triggered"].as_bool().unwrap_or(false);
        if i < 5 {
            assert!(!triggered, "round {i}: should not trigger yet");
        } else {
            assert!(triggered, "round {i}: should trigger after 6th");
        }
    }
    let state: Value =
        serde_json::from_str(&fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap())
            .unwrap();
    assert_eq!(state["replan"]["triggered"], true);
    assert!(state["replan"]["triggered_reason"]
        .as_str()
        .unwrap()
        .contains("T2"));
}

#[test]
fn t6_clean_interrupts_dirty_streak() {
    let s = Seeded::with_targets(&["T2"]);
    let findings_src = s.path().join("findings.md");
    fs::write(&findings_src, "# F\n- P0: x\n").unwrap();

    for _ in 0..3 {
        set_task_status(&s.mission_dir, "T2", "awaiting_review");
        run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
        run_ok(
            s.path(),
            &[
                "review",
                "record",
                "T5",
                "--findings-file",
                findings_src.to_str().unwrap(),
                "--mission",
                MISSION,
            ],
        );
    }
    let state: Value =
        serde_json::from_str(&fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap())
            .unwrap();
    assert_eq!(state["replan"]["consecutive_dirty_by_target"]["T2"], 3);

    // Clean reset.
    set_task_status(&s.mission_dir, "T2", "awaiting_review");
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    run_ok(
        s.path(),
        &["review", "record", "T5", "--clean", "--mission", MISSION],
    );
    let state: Value =
        serde_json::from_str(&fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap())
            .unwrap();
    assert_eq!(state["replan"]["consecutive_dirty_by_target"]["T2"], 0);
    assert_eq!(state["replan"]["triggered"], false);
}

#[test]
fn t7_clap_rejects_conflicting_clean_and_findings() {
    let s = Seeded::new();
    let output = cmd()
        .current_dir(s.path())
        .args([
            "review",
            "record",
            "T5",
            "--clean",
            "--findings-file",
            "x.md",
            "--mission",
            MISSION,
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflict"),
        "stderr: {stderr}"
    );
}

#[test]
fn t8_clap_rejects_neither_clean_nor_findings() {
    let s = Seeded::new();
    let output = cmd()
        .current_dir(s.path())
        .args(["review", "record", "T5", "--mission", MISSION])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required") || stderr.contains("error:"),
        "stderr: {stderr}"
    );
}

#[test]
fn t9_stale_superseded_returns_stale_review_record() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    // Supersede one target.
    set_target_superseded(&s.mission_dir, "T2");
    let err = run_err(
        s.path(),
        &["review", "record", "T5", "--clean", "--mission", MISSION],
    );
    assert_eq!(err["code"], "STALE_REVIEW_RECORD");
    // Event should be appended.
    let events = fs::read_to_string(s.mission_dir.join("EVENTS.jsonl")).unwrap();
    assert!(events.contains("review.stale"), "events: {events}");
    // State truth must not reflect clean: record stays Pending.
    let state: Value =
        serde_json::from_str(&fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap())
            .unwrap();
    assert_eq!(state["reviews"]["T5"]["verdict"], "pending");
}

#[test]
fn t10_terminal_already_complete_refused() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    set_terminal(&s.mission_dir);
    let events_before = fs::read_to_string(s.mission_dir.join("EVENTS.jsonl")).unwrap();
    let err = run_err(
        s.path(),
        &["review", "record", "T5", "--clean", "--mission", MISSION],
    );
    assert_eq!(err["code"], "TERMINAL_ALREADY_COMPLETE");
    let events_after = fs::read_to_string(s.mission_dir.join("EVENTS.jsonl")).unwrap();
    assert!(
        events_after.len() > events_before.len()
            && events_after.contains("review.contaminated_after_terminal"),
        "terminal-contaminated review must be audited: {events_after}"
    );
}

#[test]
fn t11_dry_run_preserves_state() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    let before = fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap();
    let events_before = fs::read_to_string(s.mission_dir.join("EVENTS.jsonl")).unwrap();

    let ok = run_ok(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--clean",
            "--dry-run",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(ok["data"]["dry_run"], true);
    let after = fs::read_to_string(s.mission_dir.join("STATE.json")).unwrap();
    let events_after = fs::read_to_string(s.mission_dir.join("EVENTS.jsonl")).unwrap();
    assert_eq!(before, after, "state must not change on dry run");
    assert_eq!(events_before, events_after, "events must not change");
}

/// Regression for round-2 correctness P2-1: `review start` dry-run
/// was not running through `state::check_expected_revision`, so a
/// caller passing `--dry-run --expect-revision N` against a state
/// whose revision had moved past N would get `ok:true`. The fix
/// wires the helper into the dry-run branch to match the other
/// short-circuit paths (task/start.rs, close/complete.rs, etc.).
#[test]
fn review_start_dry_run_enforces_expect_revision() {
    let s = Seeded::new();
    let err = run_err(
        s.path(),
        &[
            "review",
            "start",
            "T5",
            "--dry-run",
            "--expect-revision",
            "999",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(err["code"], "REVISION_CONFLICT");
    assert_eq!(err["retryable"], true);
}

#[test]
fn review_start_stale_revision_wins_over_unlocked_plan() {
    let s = Seeded::new();
    let state_path = s.mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["plan"]["locked"] = Value::Bool(false);
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let err = run_err(
        s.path(),
        &[
            "review",
            "start",
            "T5",
            "--expect-revision",
            "999",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(err["code"], "REVISION_CONFLICT");
}

#[test]
fn review_record_stale_revision_wins_over_missing_findings() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    let err = run_err(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--findings-file",
            "missing-findings.md",
            "--expect-revision",
            "999",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(err["code"], "REVISION_CONFLICT");
}

#[test]
fn t12_expect_revision_mismatch_returns_conflict() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    let err = run_err(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--clean",
            "--expect-revision",
            "999",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(err["code"], "REVISION_CONFLICT");
    assert_eq!(err["retryable"], true);
}

#[test]
fn t13_status_after_record_returns_record() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    run_ok(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--clean",
            "--reviewers",
            "code-reviewer",
            "--mission",
            MISSION,
        ],
    );
    let ok = run_ok(s.path(), &["review", "status", "T5", "--mission", MISSION]);
    assert_eq!(ok["data"]["review_task_id"], "T5");
    assert_eq!(ok["data"]["record"]["verdict"], "clean");
    assert_eq!(ok["data"]["record"]["category"], "accepted_current");
    assert_eq!(ok["data"]["record"]["reviewers"][0], "code-reviewer");
    let targets = ok["data"]["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 2);
}

#[test]
fn t14_packet_includes_reviewer_instructions() {
    let s = Seeded::new();
    let ok = run_ok(s.path(), &["review", "packet", "T5", "--mission", MISSION]);
    assert_eq!(ok["data"]["task_id"], "T5");
    let instructions = ok["data"]["reviewer_instructions"].as_str().unwrap();
    assert!(
        instructions.contains("Do not edit files"),
        "instructions: {instructions}"
    );
    assert!(instructions.contains("NONE or P0/P1/P2"));
    let profiles = ok["data"]["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 2);
    let diffs = ok["data"]["diffs"].as_array().unwrap();
    // T2 + T3 each have one write_path → 2 entries total.
    assert_eq!(diffs.len(), 2);
    let targets = ok["data"]["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 2);
}

#[test]
fn review_packet_rejects_escaping_target_spec_even_if_bad_plan_is_locked() {
    let s = Seeded::new();
    fs::write(s.path().join("secret.md"), "# secret\n").unwrap();
    let plan = fs::read_to_string(s.mission_dir.join("PLAN.yaml"))
        .unwrap()
        .replace("spec: specs/T2/SPEC.md", "spec: ../../secret.md");
    fs::write(s.mission_dir.join("PLAN.yaml"), plan).unwrap();

    let err = run_err(s.path(), &["review", "packet", "T5", "--mission", MISSION]);
    assert_eq!(err["code"], "PLAN_INVALID");
}

#[test]
fn late_same_boundary_is_flagged() {
    let s = Seeded::new();
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    // Simulate another mutation advancing the revision.
    bump_revision_without_mutation(&s.mission_dir);
    let ok = run_ok(
        s.path(),
        &["review", "record", "T5", "--clean", "--mission", MISSION],
    );
    assert_eq!(ok["data"]["category"], "late_same_boundary");
    let warnings = ok["data"]["warnings"].as_array().unwrap();
    assert!(!warnings.is_empty(), "expected late warning");
}

/// Regression for test-adequacy round-1 P2-1: a `late_same_boundary`
/// record classifies only — it must NOT move the
/// `consecutive_dirty_by_target` counter, and must NOT reset an
/// existing streak. The existing `late_same_boundary_is_flagged` test
/// only checks the category string, not the counter invariant.
#[test]
fn late_same_boundary_does_not_bump_or_reset_dirty_counter() {
    // Seed a mission with an existing dirty streak on T2 (say 3). Then
    // start T5, bump revision (to force late classification), and
    // record a dirty finding. The counter for T2 must stay at exactly 3.
    let s = Seeded::new();
    // Pre-seed the dirty counter. Also write a findings file for the
    // dirty path so the record is not --clean.
    let state_path = s.mission_dir.join("STATE.json");
    let mut state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    state["replan"]["consecutive_dirty_by_target"] = serde_json::json!({
        "T2": 3,
        "T3": 3,
    });
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let findings = s.mission_dir.join("findings.md");
    fs::write(&findings, "# P1 finding\n").unwrap();

    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    bump_revision_without_mutation(&s.mission_dir);
    let ok = run_ok(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--findings-file",
            findings.to_str().unwrap(),
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(ok["data"]["category"], "late_same_boundary");
    // Counter must be untouched — neither bumped (dirty path) nor reset
    // (clean path would zero it, but clean is also gated on
    // AcceptedCurrent).
    let state_after: Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(
        state_after["replan"]["consecutive_dirty_by_target"]["T2"],
        3
    );
    assert_eq!(
        state_after["replan"]["consecutive_dirty_by_target"]["T3"],
        3
    );
    // replan.triggered must stay false (would only flip at 6).
    assert_eq!(state_after["replan"]["triggered"], false);
}

#[test]
fn status_without_start_returns_null_record() {
    let s = Seeded::new();
    let ok = run_ok(s.path(), &["review", "status", "T5", "--mission", MISSION]);
    assert_eq!(ok["data"]["record"], Value::Null);
}

/// Regression for round-4 cli-contract/e2e-walkthrough P2-1: the old
/// substring-based parser in `review/packet.rs::read_interpreted_destination`
/// leaked the YAML block-scalar indicator `|` into `mission_summary`
/// because `trim_end()` left the leading space between `:` and `|`
/// intact, so the "skip the `|` line" guard never fired. The fix
/// replaces the parser with `serde_yaml::from_str` on the frontmatter,
/// matching the sibling implementations at `task/worker_packet.rs:60-67`
/// and `close/closeout.rs:136-142`.
#[test]
fn review_packet_mission_summary_strips_yaml_block_scalar() {
    let s = Seeded::new();
    // Overwrite OUTCOME.md with a valid-frontmatter block-scalar body
    // that would defeat the old substring parser. The space between
    // `:` and `|` is the exact shape the scaffolded template emits at
    // `init.rs:82`.
    let outcome_body = "---\n\
mission_id: demo\n\
status: ratified\n\
title: 'test'\n\
original_user_goal: |\n  do a thing\n\
interpreted_destination: |\n  Body line 1\n  Body line 2\n\
must_be_true:\n  - thing\n\
success_criteria:\n  - ok\n\
out_of_scope: []\n\
---\n\
# OUTCOME\n\
body\n";
    fs::write(s.mission_dir.join("OUTCOME.md"), outcome_body).unwrap();

    let ok = run_ok(s.path(), &["review", "packet", "T5", "--mission", MISSION]);
    let summary = ok["data"]["mission_summary"]
        .as_str()
        .expect("mission_summary present");
    assert!(
        !summary.contains('|'),
        "mission_summary must not leak the YAML block-scalar indicator: {summary:?}"
    );
    assert!(
        !summary.starts_with('|'),
        "mission_summary must not start with `|`: {summary:?}"
    );
    assert!(
        summary.contains("Body line 1"),
        "mission_summary should contain the first body line: {summary:?}"
    );
    assert!(
        summary.contains("Body line 2"),
        "mission_summary should contain the second body line: {summary:?}"
    );
    // serde_yaml's literal-block (`|`) parse yields `"Body line 1\nBody line 2\n"`;
    // we trim, so the body lines should be joined by a single newline.
    assert!(
        summary.contains("Body line 1\nBody line 2"),
        "mission_summary should preserve block-scalar newlines: {summary:?}"
    );
}

/// Regression for round-4 test-adequacy P2-1: `CliError::ReviewFindingsBlock`
/// is constructed at `src/cli/review/record.rs:57` when the
/// `--findings-file` path does not exist, but no prior integration test
/// asserts the error envelope produced by that specific construction
/// site. The `REVIEW_FINDINGS_BLOCK` string match in `tests/close.rs:304`
/// comes from a Blocker struct in `close check`, not from this CliError
/// variant. This test triggers the variant directly and pins the
/// envelope shape.
#[test]
fn review_record_findings_then_retry_returns_review_findings_block_envelope() {
    let s = Seeded::new();
    // `review start` is optional here — the missing-file check at
    // `record.rs:55-60` fires before any state read — but starting
    // first matches the real caller flow.
    run_ok(s.path(), &["review", "start", "T5", "--mission", MISSION]);
    let err = run_err(
        s.path(),
        &[
            "review",
            "record",
            "T5",
            "--findings-file",
            "does/not/exist.md",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(err["ok"], false);
    assert_eq!(err["code"], "REVIEW_FINDINGS_BLOCK");
    assert_eq!(err["retryable"], false);
    let message = err["message"]
        .as_str()
        .expect("error envelope carries a message string");
    assert!(
        !message.is_empty(),
        "message should be non-empty: {message:?}"
    );
    assert!(
        message.contains("findings file not found"),
        "message should identify the missing file: {message:?}"
    );
    assert!(
        message.contains("does/not/exist.md"),
        "message should include the offending path: {message:?}"
    );
}
