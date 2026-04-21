//! End-to-end full-mission integration test.
//!
//! Drives one complete mission through every public CLI surface:
//! init -> outcome ratify -> plan choose-level/scaffold/check/waves ->
//! task start/finish (for work tasks) -> review start/record ->
//! close check/record-review/complete -> status terminal_complete.
//!
//! The test uses only `assert_cmd::Command::cargo_bin("codex1")` —
//! internals are never stubbed. STATE.json is read only as an oracle
//! for `revision` monotonicity and phase assertions; the only direct
//! file writes are to user-owned fixtures (OUTCOME.md, PLAN.yaml,
//! SPEC.md, PROOF.md) that the CLI consumes.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

const MISSION: &str = "demo";

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn parse(output: &std::process::Output) -> Value {
    let s = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(s).unwrap_or_else(|e| panic!("bad JSON:\n{s}\n{e}"))
}

fn run_ok(cwd: &Path, args: &[&str]) -> Value {
    let out = cmd().current_dir(cwd).args(args).output().expect("runs");
    assert!(
        out.status.success(),
        "expected success ({args:?}); stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    parse(&out)
}

fn run_err(cwd: &Path, args: &[&str]) -> Value {
    let out = cmd().current_dir(cwd).args(args).output().expect("runs");
    assert!(
        !out.status.success(),
        "expected failure ({args:?}); stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );
    parse(&out)
}

fn read_state(mission_dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(mission_dir.join("STATE.json")).unwrap()).unwrap()
}

fn read_events(mission_dir: &Path) -> Vec<Value> {
    fs::read_to_string(mission_dir.join("EVENTS.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("event line is JSON"))
        .collect()
}

fn write_fixture(path: &Path, body: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, body).unwrap();
}

const INTERPRETED_DEST: &str =
    "Codex1 drives the full mission flow from clarify to terminal close without manual patches.";

fn seed_outcome(mission_dir: &Path) {
    let body = format!(
        r"---
mission_id: {MISSION}
status: draft
title: Full mission e2e fixture

original_user_goal: |
  Exercise the entire codex1 CLI surface with a single ratified plan.

interpreted_destination: |
  {INTERPRETED_DEST}

must_be_true:
  - The mission ratifies, plans, executes, reviews, and closes via CLI alone.
  - Every mutation increments STATE.json.revision by one.

success_criteria:
  - codex1 close complete writes a CLOSEOUT.md that mentions every task.
  - codex1 status reports terminal_complete with stop.allow true.

non_goals:
  - Do not cover cli-creator ergonomics; tests exercise only the Phase B surface.

constraints:
  - Use tempfile-backed mission directories.
  - No direct writes to STATE.json.

definitions:
  terminal_complete: Mission has passed mission-close review and close complete.

quality_bar:
  - Tests fail closed on any non-zero exit from a CLI step.

proof_expectations:
  - cargo test --test e2e_full_mission passes.

review_expectations:
  - A planned review task covers the parallel work wave.

known_risks:
  - CLI behavior drift between Phase B units could break the chained flow.

resolved_questions:
  - question: Does phase auto-advance to mission_close?
    answer: No, verdict keys off task completion, not the phase field.
---

# OUTCOME

End-to-end fixture body.
"
    );
    write_fixture(&mission_dir.join("OUTCOME.md"), &body);
}

/// A 5-task plan: T1 root, T2/T3 parallel under T1, T4 joins T2/T3,
/// T5 reviews [T2,T3]. Matches the Unit 20 brief shape.
const PLAN_YAML: &str = r"mission_id: demo

planning_level:
  requested: medium
  effective: medium

outcome_interpretation:
  summary: Exercise the full CLI lifecycle end-to-end.

architecture:
  summary: Single crate; CLI-driven mission.
  key_decisions:
    - Drive every subcommand through assert_cmd.

planning_process:
  evidence:
    - kind: direct_reasoning
      summary: Author-designed fixture with explicit DAG.

tasks:
  - id: T1
    title: Root setup
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
    write_paths:
      - src/T1/**
    proof:
      - cargo test T1
  - id: T2
    title: Branch A
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    write_paths:
      - src/T2/**
    proof:
      - cargo test T2
  - id: T3
    title: Branch B
    kind: code
    depends_on: [T1]
    spec: specs/T3/SPEC.md
    write_paths:
      - src/T3/**
    proof:
      - cargo test T3
  - id: T4
    title: Join
    kind: code
    depends_on: [T2, T3]
    spec: specs/T4/SPEC.md
    write_paths:
      - src/T4/**
    proof:
      - cargo test T4
  - id: T5
    title: Review of branches
    kind: review
    depends_on: [T2, T3]
    spec: specs/T5/SPEC.md
    review_target:
      tasks: [T2, T3]
    review_profiles:
      - code_bug_correctness
      - local_spec_intent

risks:
  - risk: CLI drift between units
    mitigation: Run this test in CI.

mission_close:
  criteria:
    - All tasks complete and reviews clean.
";

fn seed_plan(mission_dir: &Path) {
    write_fixture(&mission_dir.join("PLAN.yaml"), PLAN_YAML);
    for tid in ["T1", "T2", "T3", "T4", "T5"] {
        write_fixture(
            &mission_dir.join("specs").join(tid).join("SPEC.md"),
            &format!("# Spec for {tid}\n\nDo the {tid} work.\n"),
        );
    }
}

fn write_proof(mission_dir: &Path, task_id: &str) -> String {
    let rel = format!("specs/{task_id}/PROOF.md");
    write_fixture(
        &mission_dir.join(&rel),
        &format!("# Proof for {task_id}\n\ncargo test {task_id}\n"),
    );
    rel
}

fn finish_work_task(cwd: &Path, mission_dir: &Path, task_id: &str) -> Value {
    run_ok(cwd, &["task", "start", task_id, "--mission", MISSION]);
    let proof = write_proof(mission_dir, task_id);
    run_ok(
        cwd,
        &[
            "task",
            "finish",
            task_id,
            "--proof",
            &proof,
            "--mission",
            MISSION,
        ],
    )
}

fn assert_events_monotonic(mission_dir: &Path) -> u64 {
    let events = read_events(mission_dir);
    assert!(!events.is_empty(), "expected at least one event");
    let mut last: Option<u64> = None;
    for ev in &events {
        let seq = ev["seq"]
            .as_u64()
            .unwrap_or_else(|| panic!("event missing seq field: {ev}"));
        if let Some(prev) = last {
            assert!(
                seq > prev,
                "seq not strictly increasing: prev={prev}, now={seq}, ev={ev}"
            );
        }
        last = Some(seq);
    }
    last.unwrap()
}

#[test]
fn e2e_full_mission_drives_every_phase_to_terminal_complete() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = tmp.path().join("PLANS").join(MISSION);

    // 1. init
    run_ok(tmp.path(), &["init", "--mission", MISSION]);
    let state = read_state(&mission_dir);
    assert_eq!(state["revision"], 0);
    assert_eq!(state["phase"], "clarify");
    assert_eq!(state["outcome"]["ratified"], false);

    // 2. Seed a fully-filled OUTCOME.md.
    seed_outcome(&mission_dir);

    // 3. outcome check → ratifiable.
    let check = run_ok(tmp.path(), &["outcome", "check", "--mission", MISSION]);
    assert_eq!(check["data"]["ratifiable"], true);
    assert_eq!(check["data"]["missing_fields"].as_array().unwrap().len(), 0);
    assert_eq!(check["data"]["placeholders"].as_array().unwrap().len(), 0);

    // 4. outcome ratify → phase transitions to plan.
    let ratified = run_ok(tmp.path(), &["outcome", "ratify", "--mission", MISSION]);
    assert_eq!(ratified["data"]["phase"], "plan");
    assert!(ratified["data"]["ratified_at"].is_string());
    let rev_after_ratify = ratified["revision"].as_u64().unwrap();
    assert!(rev_after_ratify > 0, "ratify must bump revision");

    // 5. plan choose-level
    let chosen = run_ok(
        tmp.path(),
        &[
            "plan",
            "choose-level",
            "--mission",
            MISSION,
            "--level",
            "medium",
        ],
    );
    assert_eq!(chosen["data"]["requested_level"], "medium");
    assert_eq!(chosen["data"]["effective_level"], "medium");
    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["requested_level"], "medium");
    assert_eq!(state["plan"]["effective_level"], "medium");

    // 6. plan scaffold writes a skeleton PLAN.yaml.
    let scaffolded = run_ok(
        tmp.path(),
        &[
            "plan",
            "scaffold",
            "--mission",
            MISSION,
            "--level",
            "medium",
        ],
    );
    assert!(scaffolded["data"]["wrote"]
        .as_str()
        .unwrap()
        .ends_with("PLAN.yaml"));
    assert!(fs::read_to_string(mission_dir.join("PLAN.yaml"))
        .unwrap()
        .contains("[codex1-fill:"));

    // 7. Overwrite with the real 5-task DAG and per-task SPEC.md files.
    seed_plan(&mission_dir);

    // 8. plan check → locks plan, advances phase to execute, appends event.
    let checked = run_ok(tmp.path(), &["plan", "check", "--mission", MISSION]);
    assert_eq!(checked["data"]["tasks"], 5);
    assert_eq!(checked["data"]["review_tasks"], 1);
    assert_eq!(checked["data"]["locked"], true);
    let state = read_state(&mission_dir);
    assert_eq!(state["plan"]["locked"], true);
    assert_eq!(state["phase"], "execute");

    // 9. plan waves --json derives the expected set of waves.
    let waves = run_ok(
        tmp.path(),
        &["plan", "waves", "--mission", MISSION, "--json"],
    );
    let wave_list = waves["data"]["waves"].as_array().expect("waves array");
    assert!(
        wave_list.len() >= 2,
        "expected multi-wave derivation, got {wave_list:?}"
    );
    assert_eq!(wave_list[0]["tasks"], serde_json::json!(["T1"]));
    let w2: BTreeSet<String> = wave_list[1]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        w2,
        BTreeSet::from(["T2".to_string(), "T3".to_string()]),
        "W2 should contain T2 and T3"
    );
    // T4 and T5 must appear in some wave after W2 (they depend on T2, T3).
    let later_ids: BTreeSet<String> = wave_list
        .iter()
        .skip(2)
        .flat_map(|w| w["tasks"].as_array().unwrap())
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    for tid in ["T4", "T5"] {
        assert!(
            later_ids.contains(tid),
            "{tid} missing from later waves: {later_ids:?}"
        );
    }
    assert_eq!(waves["data"]["current_ready_wave"], "W1");
    assert_eq!(waves["data"]["all_tasks_complete"], false);

    // 10-11. Run T1 (-> Complete), then T2/T3 (-> AwaitingReview since T5 targets them).
    assert_eq!(
        finish_work_task(tmp.path(), &mission_dir, "T1")["data"]["status"],
        "complete"
    );
    for tid in ["T2", "T3"] {
        let r = finish_work_task(tmp.path(), &mission_dir, tid);
        assert_eq!(r["data"]["status"], "awaiting_review", "T={tid}");
    }

    // 12. Planned review T5 → clean → T2, T3 flip to Complete.
    run_ok(tmp.path(), &["review", "start", "T5", "--mission", MISSION]);
    let reviewed = run_ok(
        tmp.path(),
        &[
            "review",
            "record",
            "T5",
            "--clean",
            "--reviewers",
            "alice,bob",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(reviewed["data"]["verdict"], "clean");
    assert_eq!(reviewed["data"]["category"], "accepted_current");
    let reviewers = reviewed["data"]["reviewers"].as_array().unwrap();
    assert_eq!(reviewers.len(), 2);
    let state = read_state(&mission_dir);
    assert_eq!(state["tasks"]["T2"]["status"], "complete");
    assert_eq!(state["tasks"]["T3"]["status"], "complete");

    // 13. Run T4 (work) through start + finish; no review targets it.
    let t4 = finish_work_task(tmp.path(), &mission_dir, "T4");
    assert_eq!(t4["data"]["status"], "complete");

    // 14. status reports ready_for_mission_close_review.
    let status = run_ok(tmp.path(), &["status", "--mission", MISSION]);
    assert_eq!(status["data"]["verdict"], "ready_for_mission_close_review");
    assert_eq!(status["data"]["close_ready"], false);
    assert_eq!(
        status["data"]["next_action"]["kind"],
        "mission_close_review"
    );

    // 15. close record-review --clean transitions review_state → passed.
    let close_review = run_ok(
        tmp.path(),
        &[
            "close",
            "record-review",
            "--clean",
            "--reviewers",
            "carol,dan",
            "--mission",
            MISSION,
        ],
    );
    assert_eq!(close_review["data"]["review_state"], "passed");
    assert_eq!(close_review["data"]["verdict"], "clean");
    let state = read_state(&mission_dir);
    assert_eq!(state["close"]["review_state"], "passed");

    // 16. close check → ready: true.
    let ready = run_ok(tmp.path(), &["close", "check", "--mission", MISSION]);
    assert_eq!(ready["data"]["ready"], true);
    assert_eq!(ready["data"]["verdict"], "mission_close_review_passed");
    assert_eq!(ready["data"]["blockers"].as_array().unwrap().len(), 0);

    // 17. close complete writes CLOSEOUT.md and marks terminal.
    let closed = run_ok(tmp.path(), &["close", "complete", "--mission", MISSION]);
    assert!(closed["data"]["terminal_at"].is_string());
    assert_eq!(closed["data"]["mission_id"], MISSION);
    let state = read_state(&mission_dir);
    assert_eq!(state["phase"], "terminal");
    assert!(state["close"]["terminal_at"].is_string());
    let final_revision = state["revision"].as_u64().unwrap();
    assert!(final_revision > rev_after_ratify);

    // 18. Assertions on closeout content + events + idempotency.
    let closeout = fs::read_to_string(mission_dir.join("CLOSEOUT.md")).unwrap();
    assert!(
        closeout.contains("CLOSEOUT"),
        "CLOSEOUT.md missing header: {closeout}"
    );
    for tid in ["T1", "T2", "T3", "T4", "T5"] {
        assert!(
            closeout.contains(tid),
            "CLOSEOUT.md missing {tid}:\n{closeout}"
        );
    }
    assert!(
        closeout.contains(INTERPRETED_DEST),
        "CLOSEOUT.md should echo interpreted_destination:\n{closeout}"
    );

    let last_seq = assert_events_monotonic(&mission_dir);
    assert_eq!(
        last_seq, final_revision,
        "final event seq should equal STATE.revision"
    );

    // Terminal status: verdict + stop.
    let terminal = run_ok(tmp.path(), &["status", "--mission", MISSION]);
    assert_eq!(terminal["data"]["verdict"], "terminal_complete");
    assert_eq!(terminal["data"]["stop"]["allow"], true);
    assert_eq!(terminal["data"]["stop"]["reason"], "terminal");

    // Second close complete → idempotent error.
    let replay = run_err(tmp.path(), &["close", "complete", "--mission", MISSION]);
    assert_eq!(replay["code"], "TERMINAL_ALREADY_COMPLETE");
    assert!(replay["context"]["closed_at"].is_string());
}
