//! Integration tests for `codex1 task` (Phase B Unit 6).
//!
//! Each test seeds a mission directory on disk (STATE.json, PLAN.yaml,
//! specs/*) rather than going through `plan scaffold`/`outcome ratify`
//! which are implemented by sibling units that may not have merged yet.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{json, Value};
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn parse_json(out: &std::process::Output) -> Value {
    let s = std::str::from_utf8(&out.stdout).expect("utf-8 stdout");
    serde_json::from_str(s).unwrap_or_else(|e| panic!("bad JSON:\n{s}\nerror: {e}"))
}

fn write(path: &Path, content: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Seed a fresh mission directory with a ratified outcome, locked plan,
/// and empty STATE.json at revision 0. Returns (tmp, mission_dir).
fn seed_mission(plan_yaml: &str, specs: &[(&str, &str)]) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let mission_dir = tmp.path().join("PLANS").join("demo");
    fs::create_dir_all(mission_dir.join("specs")).unwrap();
    fs::create_dir_all(mission_dir.join("reviews")).unwrap();

    write(&mission_dir.join("OUTCOME.md"), OUTCOME_FIXTURE);
    write(&mission_dir.join("PLAN.yaml"), plan_yaml);
    write(
        &mission_dir.join("STATE.json"),
        r#"{
  "mission_id": "demo",
  "revision": 0,
  "schema_version": 1,
  "phase": "execute",
  "loop": { "active": false, "paused": false, "mode": "none" },
  "outcome": { "ratified": true, "ratified_at": "2026-04-20T00:00:00Z" },
  "plan": { "locked": true, "requested_level": "medium", "effective_level": "medium", "hash": "abc" },
  "tasks": {},
  "reviews": {},
  "replan": { "consecutive_dirty_by_target": {}, "triggered": false },
  "close": { "review_state": "not_started" },
  "events_cursor": 0
}
"#,
    );
    write(&mission_dir.join("EVENTS.jsonl"), "");

    for (id, body) in specs {
        write(&mission_dir.join("specs").join(id).join("SPEC.md"), body);
    }

    (tmp, mission_dir)
}

fn read_state(mission_dir: &Path) -> Value {
    let raw = fs::read_to_string(mission_dir.join("STATE.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn events(mission_dir: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn run(mission_dir_root: &Path, args: &[&str]) -> std::process::Output {
    cmd()
        .current_dir(mission_dir_root)
        .args(args)
        .output()
        .expect("runs")
}

const OUTCOME_FIXTURE: &str = r#"---
mission_id: demo
status: ratified
title: "Demo Mission"

original_user_goal: |
  do the thing

interpreted_destination: |
  A compact mission used to exercise `codex1 task` subcommands end to end.

must_be_true:
  - one

success_criteria:
  - two

non_goals:
  - three

constraints:
  - four

definitions:
  task: A planned unit of work.

quality_bar:
  - five

proof_expectations:
  - six

review_expectations:
  - seven

known_risks:
  - eight

resolved_questions:
  - question: Is this a task fixture?
    answer: Yes.
---

# OUTCOME
"#;

// --- Fixture plans ---

const PLAN_LINEAR_NO_REVIEW: &str = r#"mission_id: demo

planning_level:
  requested: medium
  effective: medium

outcome_interpretation:
  summary: "demo"

architecture:
  summary: "demo"
  key_decisions: []

planning_process:
  evidence: []

tasks:
  - id: T1
    title: "Root task"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
    read_paths: []
    write_paths: ["src/foo/**"]
    proof:
      - "cargo test foo"
  - id: T2
    title: "Dependent task"
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    write_paths: ["src/bar/**"]
    proof:
      - "cargo test bar"

risks: []
mission_close:
  criteria: []
"#;

const PLAN_WITH_REVIEW: &str = r#"mission_id: demo

planning_level:
  requested: medium
  effective: medium

outcome_interpretation:
  summary: "demo"

architecture:
  summary: "demo"
  key_decisions: []

planning_process:
  evidence: []

tasks:
  - id: T1
    title: "Root"
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
    write_paths: ["src/foo/**"]
    proof: ["cargo test foo"]
  - id: T2
    title: "Work to be reviewed"
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    write_paths: ["src/bar/**"]
    proof: ["cargo test bar"]
  - id: T3
    title: "Review of T2"
    kind: review
    depends_on: [T2]
    spec: specs/T3/SPEC.md
    review_target:
      tasks: [T2]

risks: []
mission_close:
  criteria: []
"#;

// --- Tests ---

#[test]
fn next_fresh_ratified_reports_single_ready_task() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let json = parse_json(&out);
    assert_eq!(json["ok"], json!(true));
    assert_eq!(json["data"]["next"]["kind"], "run_task");
    assert_eq!(json["data"]["next"]["task_id"], "T1");
    assert_eq!(json["data"]["next"]["task_kind"], "code");
}

#[test]
fn next_multi_ready_reports_wave() {
    // T2 and T3 both depend only on T1; complete T1 to make them ready.
    let plan = r"mission_id: demo

planning_level: { requested: medium, effective: medium }
outcome_interpretation: { summary: demo }
architecture: { summary: demo, key_decisions: [] }
planning_process: { evidence: [] }

tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: Two
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
  - id: T3
    title: Three
    kind: code
    depends_on: [T1]
    spec: specs/T3/SPEC.md

risks: []
mission_close: { criteria: [] }
";
    let (tmp, dir) = seed_mission(plan, &[]);
    // Mark T1 complete directly in STATE.
    write(&dir.join("specs/T1/PROOF.md"), "proof");
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T1/PROOF.md"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["next"]["kind"], "run_wave");
    assert_eq!(json["data"]["next"]["tasks"], json!(["T2", "T3"]));
    assert_eq!(json["data"]["next"]["parallel_safe"], json!(true));
}

#[test]
fn next_multi_ready_reports_parallel_blockers() {
    let plan = r"mission_id: demo

planning_level: { requested: medium, effective: medium }
outcome_interpretation: { summary: demo }
architecture: { summary: demo, key_decisions: [] }
planning_process: { evidence: [] }

tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: Two
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    exclusive_resources: [shared-db]
  - id: T3
    title: Three
    kind: code
    depends_on: [T1]
    spec: specs/T3/SPEC.md
    exclusive_resources: [shared-db]

risks: []
mission_close: { criteria: [] }
";
    let (tmp, dir) = seed_mission(plan, &[]);
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T1/PROOF.md"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["next"]["kind"], "run_wave");
    assert_eq!(json["data"]["next"]["parallel_safe"], json!(false));
    assert_eq!(
        json["data"]["next"]["parallel_blockers"],
        json!(["exclusive_resource:shared-db"])
    );
}

#[test]
fn next_multi_ready_reports_unknown_side_effects_blocker() {
    let plan = r"mission_id: demo

planning_level: { requested: medium, effective: medium }
outcome_interpretation: { summary: demo }
architecture: { summary: demo, key_decisions: [] }
planning_process: { evidence: [] }

tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: Two
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    unknown_side_effects: true
  - id: T3
    title: Three
    kind: code
    depends_on: [T1]
    spec: specs/T3/SPEC.md

risks: []
mission_close: { criteria: [] }
";
    let (tmp, dir) = seed_mission(plan, &[]);
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T1/PROOF.md"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["next"]["kind"], "run_wave");
    assert_eq!(json["data"]["next"]["parallel_safe"], json!(false));
    assert_eq!(
        json["data"]["next"]["parallel_blockers"],
        json!(["unknown_side_effects:T2"])
    );
}

#[test]
fn next_does_not_surface_review_when_target_superseded() {
    let plan = r"mission_id: demo

planning_level: { requested: medium, effective: medium }
outcome_interpretation: { summary: demo }
architecture: { summary: demo, key_decisions: [] }
planning_process: { evidence: [] }

tasks:
  - id: T1
    title: Root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: Review
    kind: review
    depends_on: [T1]
    spec: specs/T2/SPEC.md
    review_target:
      tasks: [T1]

risks: []
mission_close: { criteria: [] }
";
    let (tmp, dir) = seed_mission(plan, &[]);
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "superseded",
            "superseded_by": "replan-1"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_ne!(json["data"]["next"]["kind"], "run_review");
}

#[test]
fn next_does_not_surface_work_when_dependency_is_superseded() {
    let plan = r"mission_id: demo

planning_level: { requested: medium, effective: medium }
outcome_interpretation: { summary: demo }
architecture: { summary: demo, key_decisions: [] }
planning_process: { evidence: [] }

tasks:
  - id: T1
    title: Retired root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md
  - id: T2
    title: Live dependent
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md

risks: []
mission_close: { criteria: [] }
";
    let (tmp, dir) = seed_mission(plan, &[]);
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "superseded",
            "superseded_by": "replan-1"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_ne!(json["data"]["next"]["task_id"], "T2");
}

#[test]
fn next_wave_id_uses_topological_depth_not_plan_order() {
    let plan = r"mission_id: demo

planning_level: { requested: medium, effective: medium }
outcome_interpretation: { summary: demo }
architecture: { summary: demo, key_decisions: [] }
planning_process: { evidence: [] }

tasks:
  - id: T4
    title: Four
    kind: code
    depends_on: [T2]
    spec: specs/T4/SPEC.md
  - id: T5
    title: Five
    kind: code
    depends_on: [T2]
    spec: specs/T5/SPEC.md
  - id: T2
    title: Two
    kind: code
    depends_on: [T1]
    spec: specs/T2/SPEC.md
  - id: T1
    title: Root
    kind: code
    depends_on: []
    spec: specs/T1/SPEC.md

risks: []
mission_close: { criteria: [] }
";
    let (tmp, dir) = seed_mission(plan, &[]);
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T1/PROOF.md"
        },
        "T2": {
            "id": "T2",
            "status": "complete",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T2/PROOF.md"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["next"]["kind"], "run_wave");
    assert_eq!(json["data"]["next"]["wave_id"], "W3");
    assert_eq!(json["data"]["next"]["tasks"], json!(["T4", "T5"]));
}

#[test]
fn packet_rejects_escaping_spec_even_if_bad_plan_is_locked() {
    let plan = r"mission_id: demo

planning_level: { requested: medium, effective: medium }
outcome_interpretation: { summary: demo }
architecture: { summary: demo, key_decisions: [] }
planning_process: { evidence: [] }

tasks:
  - id: T1
    title: Root task
    kind: code
    depends_on: []
    spec: ../../secret.md

risks: []
mission_close: { criteria: [] }
";
    let (tmp, _dir) = seed_mission(plan, &[]);
    fs::write(tmp.path().join("secret.md"), "# secret\n").unwrap();

    let out = run(tmp.path(), &["task", "packet", "T1", "--mission", "demo"]);
    assert!(!out.status.success());
    let json = parse_json(&out);
    assert_eq!(json["code"], "PLAN_INVALID");
}

#[test]
fn start_ready_task_transitions_to_in_progress() {
    let (tmp, dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let json = parse_json(&out);
    assert_eq!(json["ok"], json!(true));
    assert_eq!(json["data"]["status"], "in_progress");
    assert_eq!(json["data"]["idempotent"], json!(false));
    assert_eq!(json["revision"], json!(1));

    let state = read_state(&dir);
    assert_eq!(state["tasks"]["T1"]["status"], "in_progress");
    assert!(state["tasks"]["T1"]["started_at"].is_string());
    assert_eq!(state["revision"], 1);

    let evs = events(&dir);
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0]["kind"], "task.started");
    assert_eq!(evs[0]["payload"]["task_id"], "T1");
}

#[test]
fn start_with_incomplete_deps_returns_task_not_ready() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(tmp.path(), &["task", "start", "T2", "--mission", "demo"]);
    assert!(!out.status.success());
    let json = parse_json(&out);
    assert_eq!(json["ok"], json!(false));
    assert_eq!(json["code"], "TASK_NOT_READY");
}

#[test]
fn start_stale_revision_wins_over_unlocked_plan() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(
        tmp.path(),
        &[
            "task",
            "start",
            "T1",
            "--mission",
            "demo",
            "--expect-revision",
            "999",
        ],
    );
    assert!(!out.status.success());
    let json = parse_json(&out);
    assert_eq!(json["code"], "REVISION_CONFLICT");
}

#[test]
fn start_twice_is_idempotent() {
    let (tmp, dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let first = run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    assert!(first.status.success());
    let rev_after_first = read_state(&dir)["revision"].as_u64().unwrap();
    let evs_after_first = events(&dir).len();

    let second = run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    assert!(second.status.success());
    let json = parse_json(&second);
    assert_eq!(json["data"]["idempotent"], json!(true));
    assert_eq!(
        read_state(&dir)["revision"].as_u64().unwrap(),
        rev_after_first
    );
    assert_eq!(events(&dir).len(), evs_after_first);
}

#[test]
fn finish_no_review_target_transitions_to_complete() {
    let (tmp, dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    // Start T1 first.
    run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    write(&dir.join("specs/T1/PROOF.md"), "ok");
    let out = run(
        tmp.path(),
        &[
            "task",
            "finish",
            "T1",
            "--proof",
            "specs/T1/PROOF.md",
            "--mission",
            "demo",
        ],
    );
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let json = parse_json(&out);
    assert_eq!(json["data"]["status"], "complete");
    assert_eq!(json["data"]["proof_path"], "specs/T1/PROOF.md");

    let state = read_state(&dir);
    assert_eq!(state["tasks"]["T1"]["status"], "complete");
    assert!(state["tasks"]["T1"]["finished_at"].is_string());
    let evs = events(&dir);
    assert_eq!(evs.last().unwrap()["kind"], "task.finished");
}

#[test]
fn finish_with_review_target_transitions_to_awaiting_review() {
    let (tmp, dir) = seed_mission(PLAN_WITH_REVIEW, &[]);
    // Complete T1, then start T2.
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T1/PROOF.md"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );
    run(tmp.path(), &["task", "start", "T2", "--mission", "demo"]);
    write(&dir.join("specs/T2/PROOF.md"), "ok");

    let out = run(
        tmp.path(),
        &[
            "task",
            "finish",
            "T2",
            "--proof",
            "specs/T2/PROOF.md",
            "--mission",
            "demo",
        ],
    );
    let json = parse_json(&out);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    assert_eq!(json["data"]["status"], "awaiting_review");
    assert_eq!(read_state(&dir)["tasks"]["T2"]["status"], "awaiting_review");
}

#[test]
fn finish_with_missing_proof_file_errors() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    let out = run(
        tmp.path(),
        &[
            "task",
            "finish",
            "T1",
            "--proof",
            "does/not/exist.md",
            "--mission",
            "demo",
        ],
    );
    assert!(!out.status.success());
    let json = parse_json(&out);
    assert_eq!(json["code"], "PROOF_MISSING");
    assert!(json["context"]["path"].is_string());
}

#[test]
fn finish_stale_revision_wins_over_missing_proof() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(
        tmp.path(),
        &[
            "task",
            "finish",
            "T1",
            "--proof",
            "specs/T1/PROOF.md",
            "--mission",
            "demo",
            "--expect-revision",
            "999",
        ],
    );
    assert!(!out.status.success());
    let json = parse_json(&out);
    assert_eq!(json["code"], "REVISION_CONFLICT");
}

#[test]
fn finish_without_proof_arg_is_clap_error() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(tmp.path(), &["task", "finish", "T1", "--mission", "demo"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--proof") || stderr.contains("required"),
        "expected clap error about --proof, got: {stderr}"
    );
}

#[test]
fn status_returns_expected_envelope() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(tmp.path(), &["task", "status", "T1", "--mission", "demo"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let json = parse_json(&out);
    assert_eq!(json["data"]["task_id"], "T1");
    assert_eq!(json["data"]["kind"], "code");
    assert_eq!(json["data"]["status"], "ready");
    assert_eq!(json["data"]["depends_on"], json!([]));
    assert!(json["data"]["deps_status"].is_object());
}

#[test]
fn status_reports_deps_status_map() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(tmp.path(), &["task", "status", "T2", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["status"], "pending");
    assert_eq!(json["data"]["depends_on"], json!(["T1"]));
    assert_eq!(json["data"]["deps_status"]["T1"], "pending");
}

#[test]
fn packet_returns_expected_fields() {
    let spec = "# T2\n\nDo the bar work.\n\nReference the API per the spec.\n";
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[("T2", spec)]);
    let out = run(tmp.path(), &["task", "packet", "T2", "--mission", "demo"]);
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let json = parse_json(&out);
    assert_eq!(json["data"]["task_id"], "T2");
    assert_eq!(json["data"]["kind"], "code");
    assert!(json["data"]["spec_excerpt"]
        .as_str()
        .unwrap()
        .contains("Do the bar work."));
    assert_eq!(json["data"]["write_paths"], json!(["src/bar/**"]));
    assert_eq!(json["data"]["proof_commands"], json!(["cargo test bar"]));
    let instructions = json["data"]["worker_instructions"].as_str().unwrap();
    assert!(instructions.contains("Codex1 worker"));
    assert!(instructions.contains("T2"));
    assert!(instructions.contains("must not"));
    let summary = json["data"]["mission_summary"].as_str().unwrap();
    assert!(
        summary.contains("mission used to exercise"),
        "expected interpreted_destination in mission_summary, got {summary:?}"
    );
}

#[test]
fn next_after_work_done_with_review_pending_runs_review() {
    let (tmp, dir) = seed_mission(PLAN_WITH_REVIEW, &[]);
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {
            "id": "T1",
            "status": "complete",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T1/PROOF.md"
        },
        "T2": {
            "id": "T2",
            "status": "awaiting_review",
            "finished_at": "2026-04-20T00:00:00Z",
            "proof_path": "specs/T2/PROOF.md"
        }
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["next"]["kind"], "run_review");
    assert_eq!(json["data"]["next"]["task_id"], "T3");
    assert_eq!(json["data"]["next"]["targets"], json!(["T2"]));
}

#[test]
fn next_all_complete_reports_mission_close_review() {
    let (tmp, dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let mut state = read_state(&dir);
    state["tasks"] = json!({
        "T1": {"id":"T1","status":"complete"},
        "T2": {"id":"T2","status":"complete"}
    });
    write(
        &dir.join("STATE.json"),
        &serde_json::to_string_pretty(&state).unwrap(),
    );
    let out = run(tmp.path(), &["task", "next", "--mission", "demo"]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["next"]["kind"], "mission_close_review");
}

#[test]
fn start_dry_run_does_not_mutate() {
    let (tmp, dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(
        tmp.path(),
        &["task", "start", "T1", "--dry-run", "--mission", "demo"],
    );
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let json = parse_json(&out);
    assert_eq!(json["data"]["dry_run"], json!(true));
    assert_eq!(read_state(&dir)["revision"], 0);
    assert!(events(&dir).is_empty());
}

#[test]
fn finish_dry_run_does_not_mutate() {
    let (tmp, dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    let rev_before = read_state(&dir)["revision"].as_u64().unwrap();
    let ev_before = events(&dir).len();
    write(&dir.join("specs/T1/PROOF.md"), "ok");
    let out = run(
        tmp.path(),
        &[
            "task",
            "finish",
            "T1",
            "--proof",
            "specs/T1/PROOF.md",
            "--dry-run",
            "--mission",
            "demo",
        ],
    );
    assert!(out.status.success(), "stderr: {:?}", out.stderr);
    let json = parse_json(&out);
    assert_eq!(json["data"]["dry_run"], json!(true));
    assert_eq!(read_state(&dir)["revision"].as_u64().unwrap(), rev_before);
    assert_eq!(events(&dir).len(), ev_before);
}

#[test]
fn start_expect_revision_mismatch_returns_revision_conflict() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let out = run(
        tmp.path(),
        &[
            "task",
            "start",
            "T1",
            "--expect-revision",
            "99",
            "--mission",
            "demo",
        ],
    );
    assert!(!out.status.success());
    let json = parse_json(&out);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["retryable"], json!(true));
    assert_eq!(json["context"]["expected"], json!(99));
    assert_eq!(json["context"]["actual"], json!(0));
}

#[test]
fn finish_expect_revision_mismatch_returns_revision_conflict() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    let mission_dir = tmp.path().join("PLANS/demo");
    write(&mission_dir.join("specs/T1/PROOF.md"), "ok");
    let out = run(
        tmp.path(),
        &[
            "task",
            "finish",
            "T1",
            "--proof",
            "specs/T1/PROOF.md",
            "--expect-revision",
            "99",
            "--mission",
            "demo",
        ],
    );
    assert!(!out.status.success());
    let json = parse_json(&out);
    assert_eq!(json["code"], "REVISION_CONFLICT");
}

/// Regression for correctness-invariants round-1 P1-5: the idempotent
/// short-circuit in `task start` (already-in-progress) must still
/// enforce `--expect-revision`, otherwise stale-writer probes silently
/// succeed against a state that moved on.
#[test]
fn start_idempotent_branch_still_enforces_expect_revision() {
    let (tmp, _dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    // First start lands cleanly and takes revision from 0 to 1.
    let ok = run(tmp.path(), &["task", "start", "T1", "--mission", "demo"]);
    assert!(ok.status.success());
    let json = parse_json(&ok);
    assert_eq!(json["revision"], 1);
    // Second call hits the idempotent branch (task is InProgress) but
    // passes a stale `--expect-revision`. It MUST fail REVISION_CONFLICT
    // rather than silently return `idempotent: true`.
    let out = run(
        tmp.path(),
        &[
            "task",
            "start",
            "T1",
            "--expect-revision",
            "99",
            "--mission",
            "demo",
        ],
    );
    assert!(
        !out.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let json = parse_json(&out);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["context"]["expected"], 99);
    assert_eq!(json["context"]["actual"], 1);
}

#[test]
fn concurrent_task_start_is_idempotent_under_lock() {
    use std::sync::Arc;
    use std::thread;

    let (tmp, dir) = seed_mission(PLAN_LINEAR_NO_REVIEW, &[]);
    let root = Arc::new(tmp.path().to_path_buf());

    let h1_root = root.clone();
    let h1 = thread::spawn(move || run(&h1_root, &["task", "start", "T1", "--mission", "demo"]));
    let h2_root = root.clone();
    let h2 = thread::spawn(move || run(&h2_root, &["task", "start", "T1", "--mission", "demo"]));

    let o1 = h1.join().unwrap();
    let o2 = h2.join().unwrap();
    assert!(
        o1.status.success() && o2.status.success(),
        "both starts should succeed idempotently:\n{}\n---\n{}",
        String::from_utf8_lossy(&o1.stdout),
        String::from_utf8_lossy(&o2.stdout)
    );
    let j1 = parse_json(&o1);
    let j2 = parse_json(&o2);
    let idempotent_count = [&j1, &j2]
        .iter()
        .filter(|j| j["data"]["idempotent"] == json!(true))
        .count();
    assert_eq!(
        idempotent_count, 1,
        "exactly one racing start should become the idempotent no-op"
    );

    let state = read_state(&dir);
    assert_eq!(state["revision"], 1);
    assert_eq!(state["tasks"]["T1"]["status"], "in_progress");
    let task_started_events = events(&dir)
        .into_iter()
        .filter(|event| event["kind"] == "task.started")
        .count();
    assert_eq!(task_started_events, 1);
}
