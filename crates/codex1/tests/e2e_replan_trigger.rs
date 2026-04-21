//! End-to-end replan-trigger integration test.
//!
//! Walks a mission up to a plan-locked state, issues a planned review
//! task, and records six consecutive dirty reviews against the same
//! target. After the sixth dirty:
//! - `replan check` must report `required: true` with a non-null reason
//!   mentioning the target.
//! - `state.replan.triggered` must be `true`.
//!
//! Then runs `replan record --reason six_dirty --supersedes <Tn>` and
//! asserts the mission rolls back to `phase: plan`, `plan.locked: false`,
//! and all counters clear.
//!
//! The review-target reset between rounds is performed by writing
//! STATE.json directly (same pattern `tests/review.rs` uses). This is
//! accepted per the Unit 20 brief — "issue a planned review task; record
//! 6 consecutive dirty reviews" — and matches the mission-close semantics
//! where only the main thread (not the CLI) can hand a target back into
//! review.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{json, Value};
use tempfile::TempDir;

const MISSION: &str = "demo";

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn init_mission(tmp: &TempDir) -> PathBuf {
    let mission_dir = tmp.path().join("PLANS").join(MISSION);
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", MISSION])
        .assert()
        .success();
    mission_dir
}

fn parse(output: &std::process::Output) -> Value {
    let s = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(s).unwrap_or_else(|e| panic!("bad JSON:\n{s}\n{e}"))
}

fn run_ok(cwd: &Path, args: &[&str]) -> Value {
    let out = cmd().current_dir(cwd).args(args).output().expect("runs");
    assert!(
        out.status.success(),
        "expected success ({args:?}); stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    parse(&out)
}

fn read_state(mission_dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap()
}

fn write_state(mission_dir: &Path, state: &Value) {
    fs::write(
        mission_dir.join("STATE.json"),
        serde_json::to_vec_pretty(state).unwrap(),
    )
    .unwrap();
}

/// Mission fixture: T1 (work, complete) + T2 (work, awaiting_review) +
/// T3 (review targeting T2). Plan is locked; outcome ratified.
fn seed_plan_locked(mission_dir: &Path) {
    let plan = r"mission_id: demo

planning_level:
  requested: light
  effective: light

outcome_interpretation:
  summary: replan-trigger e2e

architecture:
  summary: replan-trigger e2e
  key_decisions: []

planning_process:
  evidence: []

tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
    write_paths:
      - src/T1/**
  - id: T2
    title: Work under review
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    write_paths:
      - src/T2/**
  - id: T3
    title: Review of T2
    kind: review
    depends_on: [T2]
    spec: specs/T3/SPEC.md
    review_target:
      tasks: [T2]
    review_profiles:
      - code_bug_correctness

risks: []

mission_close:
  criteria: []
";
    fs::write(mission_dir.join("PLAN.yaml"), plan).unwrap();
    for tid in ["T1", "T2", "T3"] {
        let dir = mission_dir.join("specs").join(tid);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SPEC.md"), format!("# {tid}\n")).unwrap();
        if matches!(tid, "T1" | "T2") {
            fs::write(dir.join("PROOF.md"), format!("# proof {tid}\n")).unwrap();
        }
    }
    let mut state = read_state(mission_dir);
    state["outcome"] = json!({ "ratified": true, "ratified_at": "2026-04-20T00:00:00Z" });
    state["plan"]["locked"] = Value::Bool(true);
    state["plan"]["requested_level"] = Value::String("light".into());
    state["plan"]["effective_level"] = Value::String("light".into());
    state["plan"]["task_ids"] = json!(["T1", "T2", "T3"]);
    state["phase"] = Value::String("execute".into());
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "started_at": "2026-04-20T00:00:00Z",
            "finished_at": "2026-04-20T00:00:01Z",
            "proof_path": "specs/T1/PROOF.md",
            "superseded_by": null,
        },
        "T2": {
            "id": "T2",
            "status": "awaiting_review",
            "started_at": "2026-04-20T00:00:02Z",
            "finished_at": "2026-04-20T00:00:03Z",
            "proof_path": "specs/T2/PROOF.md",
            "superseded_by": null,
        },
    });
    write_state(mission_dir, &state);
}

/// Bounce T2 back into AwaitingReview so we can re-run `review start`.
fn reset_target(mission_dir: &Path, task_id: &str) {
    let mut state = read_state(mission_dir);
    state["tasks"][task_id]["status"] = Value::String("awaiting_review".into());
    state["tasks"][task_id]["finished_at"] = Value::String("2999-01-01T00:00:00Z".into());
    write_state(mission_dir, &state);
}

#[test]
fn e2e_six_dirty_reviews_trigger_replan_and_record_clears_counters() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp);
    seed_plan_locked(&mission_dir);

    // Write findings file once; every round reuses it.
    let findings = tmp.path().join("findings.md");
    fs::write(&findings, "# Findings\n- P0: regression\n").unwrap();

    // Initial replan check: nothing yet.
    let before = run_ok(tmp.path(), &["replan", "check", "--mission", MISSION]);
    assert_eq!(before["data"]["required"], false);
    assert!(before["data"]["reason"].is_null());
    assert_eq!(before["data"]["triggered_already"], false);

    // Record six dirty reviews against T3 targeting T2. Reset T2 to
    // AwaitingReview between rounds so `review start` accepts it each time.
    for i in 0..6 {
        reset_target(&mission_dir, "T2");
        run_ok(tmp.path(), &["review", "start", "T3", "--mission", MISSION]);
        let ok = run_ok(
            tmp.path(),
            &[
                "review",
                "record",
                "T3",
                "--findings-file",
                findings.to_str().unwrap(),
                "--mission",
                MISSION,
            ],
        );
        assert_eq!(ok["data"]["verdict"], "dirty");
        let triggered = ok["data"]["replan_triggered"].as_bool().unwrap_or(false);
        assert_eq!(triggered, i == 5, "round {i}: replan_triggered unexpected");
    }

    // After 6 dirty, replan check must require replan and mention T2.
    let required = run_ok(tmp.path(), &["replan", "check", "--mission", MISSION]);
    assert_eq!(required["data"]["required"], true);
    let reason = required["data"]["reason"].as_str().expect("reason present");
    assert!(reason.contains("T2"), "reason missing T2: {reason}");
    assert_eq!(required["data"]["triggered_already"], true);
    assert_eq!(
        required["data"]["consecutive_dirty_by_target"]["T2"], 6,
        "T2 counter should be at threshold"
    );

    let state = read_state(&mission_dir);
    assert_eq!(state["replan"]["triggered"], true);

    // Record the replan, superseding T2 (the dirty target).
    let recorded = run_ok(
        tmp.path(),
        &[
            "replan",
            "record",
            "--mission",
            MISSION,
            "--reason",
            "six_dirty",
            "--supersedes",
            "T2",
            "--supersedes",
            "T3",
        ],
    );
    assert_eq!(recorded["data"]["reason"], "six_dirty");
    assert_eq!(recorded["data"]["supersedes"], json!(["T2", "T3"]));
    assert_eq!(recorded["data"]["phase_after"], "plan");
    assert_eq!(recorded["data"]["plan_locked"], false);

    let state = read_state(&mission_dir);
    assert_eq!(state["phase"], "plan");
    assert_eq!(state["plan"]["locked"], false);
    assert_eq!(state["replan"]["triggered"], true);
    assert_eq!(state["replan"]["consecutive_dirty_by_target"], json!({}));
    assert_eq!(state["tasks"]["T2"]["status"], "superseded");
    let superseded_by = state["tasks"]["T2"]["superseded_by"]
        .as_str()
        .expect("T2 superseded_by present");
    assert!(
        superseded_by.starts_with("replan-"),
        "superseded_by should be a replan-<rev> marker: {superseded_by}"
    );

    let after = run_ok(tmp.path(), &["replan", "check", "--mission", MISSION]);
    assert_eq!(after["data"]["required"], false);
    assert_eq!(after["data"]["consecutive_dirty_by_target"], json!({}));
    assert_eq!(after["data"]["triggered_already"], true);
}

/// A valid plan that passes `plan check`: non-empty
/// architecture.key_decisions, a risk, a mission_close criterion, and
/// a direct_reasoning evidence entry (the seed for
/// `seed_plan_locked` above is intentionally minimal for the dirty-
/// review flow and does not round-trip through `plan check`).
fn write_valid_plan(mission_dir: &Path) {
    let plan = r"mission_id: demo

planning_level:
  requested: light
  effective: light

outcome_interpretation:
  summary: replan P0 regression

architecture:
  summary: replan P0 regression
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
    title: Work under review
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
  - id: T3
    title: Review of T2
    kind: review
    depends_on: [T2]
    spec: specs/T3/SPEC.md
    review_target:
      tasks: [T2]

risks:
  - risk: x
    mitigation: y

mission_close:
  criteria:
    - ok
";
    fs::write(mission_dir.join("PLAN.yaml"), plan).unwrap();
    for tid in ["T1", "T2", "T3"] {
        let dir = mission_dir.join("specs").join(tid);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SPEC.md"), format!("# {tid}\n")).unwrap();
    }
}

fn write_valid_replan_plan(mission_dir: &Path) {
    let plan = r"mission_id: demo

planning_level:
  requested: light
  effective: light

outcome_interpretation:
  summary: replan replacement plan

architecture:
  summary: replacement-only plan
  key_decisions:
    - one

planning_process:
  evidence:
    - kind: direct_reasoning
      summary: x

tasks:
  - id: T4
    title: Replacement work
    kind: code
    depends_on: []
    spec: specs/T4/SPEC.md
  - id: T5
    title: Review of T4
    kind: review
    depends_on: [T4]
    spec: specs/T5/SPEC.md
    review_target:
      tasks: [T4]
    review_profiles:
      - code_bug_correctness

risks:
  - risk: x
    mitigation: y

mission_close:
  criteria:
    - ok
";
    fs::write(mission_dir.join("PLAN.yaml"), plan).unwrap();
    for tid in ["T4", "T5"] {
        let dir = mission_dir.join("specs").join(tid);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SPEC.md"), format!("# {tid}\n")).unwrap();
    }
}

/// Regression for round-2 e2e P0-1: `replan record` unlocks the plan and
/// sets `state.replan.triggered = true`; a successful `plan check` that
/// relocks the plan must ALSO clear `state.replan.triggered` and
/// `state.replan.triggered_reason`. Without this, any mission that
/// enters replan is bricked — `status.verdict` stays `blocked`,
/// `close check` / `close complete` refuse to advance.
#[test]
fn plan_check_after_replan_record_clears_triggered() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp);
    write_valid_plan(&mission_dir);
    // Seed state: outcome ratified, plan.locked=true (without actually
    // running plan check — we want to test the post-replan relock
    // transition, not the first-lock transition).
    let mut state = read_state(&mission_dir);
    state["outcome"] = json!({ "ratified": true, "ratified_at": "2026-04-20T00:00:00Z" });
    state["plan"]["locked"] = Value::Bool(true);
    state["plan"]["task_ids"] = json!(["T1", "T2", "T3"]);
    state["phase"] = Value::String("execute".into());
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
            "status": "awaiting_review",
            "started_at": "2026-04-20T00:00:02Z",
            "finished_at": "2026-04-20T00:00:03Z",
            "superseded_by": null,
        },
    });
    write_state(&mission_dir, &state);

    // Drive a replan so plan.locked=false and replan.triggered=true.
    run_ok(
        tmp.path(),
        &[
            "replan",
            "record",
            "--mission",
            MISSION,
            "--reason",
            "scope_change",
            "--supersedes",
            "T2",
            "--supersedes",
            "T3",
        ],
    );

    let before = read_state(&mission_dir);
    assert_eq!(before["plan"]["locked"], false);
    assert_eq!(before["replan"]["triggered"], true);
    assert_eq!(before["replan"]["triggered_reason"], "scope_change");

    write_valid_replan_plan(&mission_dir);

    // Relock via `plan check`. The closure must clear `replan.triggered`.
    run_ok(tmp.path(), &["plan", "check", "--mission", MISSION]);

    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["locked"], true);
    assert_eq!(
        state["replan"]["triggered"], false,
        "plan check must clear replan.triggered after relock"
    );
    assert!(
        state["replan"]["triggered_reason"].is_null(),
        "plan check must clear replan.triggered_reason after relock: {}",
        state["replan"]["triggered_reason"]
    );
}

/// Regression for round-2 e2e P0-1 (full reproducer): drive the mission
/// through `replan record` → subsequent work on a new task → mission-
/// close review → `close complete`. Without the P0 fix the final
/// `close complete` returns `CLOSE_NOT_READY` with a leftover
/// `REPLAN_REQUIRED` blocker.
#[test]
fn full_mission_close_after_replan_reaches_terminal() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_mission(&tmp);
    // Plan with T1 (root, complete), T2 (awaiting_review),
    // T3 (review of T2), T4 (work), T5 (review of T4). Replan will
    // supersede T2 (and its review T3 is no longer actionable), then
    // execute T4 + T5 and mission-close review.
    let plan = r"mission_id: demo

planning_level:
  requested: light
  effective: light

outcome_interpretation:
  summary: replan full path

architecture:
  summary: replan full path
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
    title: Work to supersede
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
  - id: T3
    title: Review of T2
    kind: review
    depends_on: [T2]
    spec: specs/T3/SPEC.md
    review_target:
      tasks: [T2]
    review_profiles:
      - code_bug_correctness
  - id: T4
    title: Replacement work
    kind: code
    depends_on: [T1]
    spec: specs/T4/SPEC.md
  - id: T5
    title: Review of T4
    kind: review
    depends_on: [T4]
    spec: specs/T5/SPEC.md
    review_target:
      tasks: [T4]
    review_profiles:
      - code_bug_correctness

risks:
  - risk: x
    mitigation: y

mission_close:
  criteria:
    - ok
";
    fs::write(mission_dir.join("PLAN.yaml"), plan).unwrap();
    for tid in ["T1", "T2", "T3", "T4", "T5"] {
        let dir = mission_dir.join("specs").join(tid);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SPEC.md"), format!("# {tid}\n")).unwrap();
        if matches!(tid, "T1" | "T2") {
            fs::write(dir.join("PROOF.md"), format!("# proof {tid}\n")).unwrap();
        }
    }
    // Pre-seed T1 complete and T2 awaiting_review (bypassing the usual
    // `plan check` + `task start`/`finish` path since we only care
    // about the post-replan close path).
    let mut state = read_state(&mission_dir);
    state["outcome"] = json!({ "ratified": true, "ratified_at": "2026-04-20T00:00:00Z" });
    state["plan"]["locked"] = Value::Bool(true);
    state["plan"]["requested_level"] = Value::String("light".into());
    state["plan"]["effective_level"] = Value::String("light".into());
    state["plan"]["task_ids"] = json!(["T1", "T2", "T3", "T4", "T5"]);
    state["phase"] = Value::String("execute".into());
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "started_at": "2026-04-20T00:00:00Z",
            "finished_at": "2026-04-20T00:00:01Z",
            "proof_path": "specs/T1/PROOF.md",
            "superseded_by": null,
        },
        "T2": {
            "id": "T2",
            "status": "awaiting_review",
            "started_at": "2026-04-20T00:00:02Z",
            "finished_at": "2026-04-20T00:00:03Z",
            "proof_path": "specs/T2/PROOF.md",
            "superseded_by": null,
        },
    });
    write_state(&mission_dir, &state);

    // Replan: supersede T2 (the only task currently tracked in
    // state.tasks besides T1-complete). plan.locked → false,
    // replan.triggered → true.
    run_ok(
        tmp.path(),
        &[
            "replan",
            "record",
            "--mission",
            MISSION,
            "--reason",
            "scope_change",
            "--supersedes",
            "T2",
            "--supersedes",
            "T3",
        ],
    );

    write_valid_replan_plan(&mission_dir);

    // Re-lock the replacement plan; the P0 fix must clear
    // replan.triggered here.
    run_ok(tmp.path(), &["plan", "check", "--mission", MISSION]);
    let relocked = read_state(&mission_dir);
    assert_eq!(
        relocked["replan"]["triggered"], false,
        "replan.triggered should clear on relock: {}",
        relocked["replan"]["triggered"]
    );

    // Mark T3 (review of superseded T2) as superseded in STATE so it
    // does not block close-ready. The CLI only tracks tasks it has
    // seen; T3 was never started, so a manual bump is acceptable
    // here (matches the pattern used elsewhere in this file).
    let mut state = read_state(&mission_dir);
    state["tasks"]["T3"] = json!({
        "id": "T3",
        "status": "superseded",
        "superseded_by": "replan-T2",
    });
    write_state(&mission_dir, &state);

    // Execute T4 + T5 (the replacement work + its review).
    run_ok(tmp.path(), &["task", "start", "T4", "--mission", MISSION]);
    let proof = mission_dir.join("specs/T4/PROOF.md");
    fs::write(&proof, "# T4 proof\n").unwrap();
    run_ok(
        tmp.path(),
        &[
            "task",
            "finish",
            "T4",
            "--proof",
            "specs/T4/PROOF.md",
            "--mission",
            MISSION,
        ],
    );
    run_ok(tmp.path(), &["review", "start", "T5", "--mission", MISSION]);
    run_ok(
        tmp.path(),
        &[
            "review",
            "record",
            "T5",
            "--clean",
            "--reviewers",
            "r1",
            "--mission",
            MISSION,
        ],
    );

    // Now record the mission-close review and finalize.
    run_ok(
        tmp.path(),
        &[
            "close",
            "record-review",
            "--clean",
            "--reviewers",
            "c1",
            "--mission",
            MISSION,
        ],
    );
    let close = run_ok(tmp.path(), &["close", "complete", "--mission", MISSION]);
    assert_eq!(close["ok"], true, "close complete must succeed: {close}");

    let state = read_state(&mission_dir);
    assert!(
        state["close"]["terminal_at"].is_string(),
        "terminal_at not set: {state}"
    );
    assert_eq!(state["replan"]["triggered"], false);

    // Regression for round-3 test-adequacy P2-2: the "full reproducer"
    // label on this test claims coverage of `close complete`. Asserting
    // `terminal_at` alone does not verify the CLOSEOUT.md write; a
    // regression that skipped `atomic_write(CLOSEOUT.md)` while still
    // bumping STATE would silently pass. Assert the artifact exists and
    // carries the expected content shape.
    let closeout = fs::read_to_string(mission_dir.join("CLOSEOUT.md"))
        .expect("CLOSEOUT.md written after post-replan close complete");
    assert!(
        closeout.contains("CLOSEOUT"),
        "CLOSEOUT.md missing CLOSEOUT header: {closeout}"
    );
    // Mission id should appear in the header line `# CLOSEOUT — demo`.
    assert!(
        closeout.contains(MISSION),
        "CLOSEOUT.md missing mission id `{MISSION}`: {closeout}"
    );
    // The tasks completed through the post-replan path must both appear
    // in the tasks table: T1 (pre-replan complete) and T4 (replacement
    // work driven through task start/finish after the relock).
    for tid in ["T1", "T4"] {
        assert!(
            closeout.contains(tid),
            "CLOSEOUT.md missing {tid}: {closeout}"
        );
    }
    // Terminal_at stamp surfaces in the body.
    let terminal_at = state["close"]["terminal_at"].as_str().unwrap();
    assert!(
        closeout.contains(terminal_at),
        "CLOSEOUT.md missing terminal_at `{terminal_at}`: {closeout}"
    );
}
