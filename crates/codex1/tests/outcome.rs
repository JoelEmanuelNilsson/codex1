//! Integration tests for `codex1 outcome check` and `codex1 outcome ratify`.
//!
//! Coverage:
//!   1. Fresh init fails `check` with OUTCOME_INCOMPLETE + fill markers.
//!   2. Filled OUTCOME.md passes `check` with ratifiable: true.
//!   3. Boilerplate (`TODO`, `TBD`, …) is flagged by `check`.
//!   4. `ratify` on invalid OUTCOME.md does not mutate STATE.json.
//!   5. `ratify` on valid OUTCOME.md bumps revision, writes event, flips
//!      frontmatter status, advances phase to `plan`.
//!   6. `ratify --dry-run` validates but does not mutate.
//!   7. `ratify --expect-revision 999` returns REVISION_CONFLICT.
//!   8. `ratify` then `check` → still ratifiable (idempotent view).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("codex1").expect("binary builds")
}

fn init_demo(tmp: &TempDir, mission: &str) -> PathBuf {
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--mission", mission])
        .assert()
        .success();
    tmp.path().join("PLANS").join(mission)
}

fn parse_json(output: &Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).expect("utf-8 stdout");
    serde_json::from_str::<Value>(stdout).unwrap_or_else(|e| {
        panic!("expected JSON stdout, got:\n{stdout}\nerror: {e}");
    })
}

/// Overwrite OUTCOME.md with a fully-filled, ratifiable template.
fn seed_valid_outcome(mission_dir: &Path, mission_id: &str) {
    let body = format!(
        r"---
mission_id: {mission_id}
status: draft
title: Demo mission for outcome tests

original_user_goal: |
  The user wants to exercise the outcome ratification flow end to end,
  validating that every required field is present and free of placeholders.

interpreted_destination: |
  Codex1 correctly treats a fully-filled OUTCOME.md as ratifiable, flips
  the frontmatter status to ratified, and advances the mission phase.

must_be_true:
  - The mission has a single clarified destination captured in OUTCOME.md.
  - Every required field contains concrete content, not fill markers.

success_criteria:
  - codex1 outcome check returns ratifiable true for this fixture.
  - codex1 outcome ratify flips phase from clarify to plan and records ratified_at.

non_goals:
  - Do not implement planning logic in this unit.
  - Do not change Foundation files.

constraints:
  - Tests must use tempfile-backed mission directories only.
  - OUTCOME.md rewrite must preserve the markdown body byte-for-byte.

quality_bar:
  - The check command is idempotent and side-effect free.

proof_expectations:
  - cargo test outcome passes for this unit.

review_expectations:
  - The main thread reviews outcome validation before plan scaffolding.

known_risks:
  - YAML frontmatter reordering on rewrite could silently corrupt missions.

resolved_questions:
  - question: Should status ratification rewrite the file?
    answer: Yes, atomically, only the status line inside the frontmatter.
---

# OUTCOME

Body paragraph for the test fixture.
"
    );
    let outcome_path = mission_dir.join("OUTCOME.md");
    fs::write(&outcome_path, body).expect("write OUTCOME.md");
}

fn read_state(mission_dir: &Path) -> Value {
    let raw = fs::read_to_string(mission_dir.join("STATE.json")).expect("read state");
    serde_json::from_str(&raw).expect("parse state")
}

fn read_events(mission_dir: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(mission_dir.join("EVENTS.jsonl")).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("parse event line"))
        .collect()
}

#[test]
fn check_on_fresh_init_reports_fill_markers() {
    let tmp = TempDir::new().unwrap();
    init_demo(&tmp, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success(), "expected non-zero exit");
    let json = parse_json(&output);
    assert_eq!(json["ok"], Value::Bool(false));
    assert_eq!(json["code"], "OUTCOME_INCOMPLETE");
    let placeholders = json["context"]["placeholders"]
        .as_array()
        .expect("placeholders array");
    assert!(
        !placeholders.is_empty(),
        "expected fill markers in context.placeholders, got: {json}"
    );
    let any_marker = placeholders
        .iter()
        .any(|p| p.as_str().unwrap_or("").contains("[codex1-fill:"));
    assert!(
        any_marker,
        "expected at least one [codex1-fill:…] marker, got: {placeholders:?}"
    );
    assert_eq!(json["context"]["mission_id"], "demo");
}

#[test]
fn check_on_valid_outcome_is_ratifiable() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_valid_outcome(&mission_dir, "demo");
    let output = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success(), "expected success: {:?}", output);
    let json = parse_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["ratifiable"], true);
    assert_eq!(json["data"]["missing_fields"].as_array().unwrap().len(), 0);
    assert_eq!(json["data"]["placeholders"].as_array().unwrap().len(), 0);
    assert_eq!(json["mission_id"], "demo");
}

#[test]
fn check_flags_boilerplate_placeholders() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    // Seed the mandatory fields but inject boilerplate in multiple places.
    let body = r"---
mission_id: demo
status: draft
title: TODO

original_user_goal: |
  TBD

interpreted_destination: |
  Codex1 works well.

must_be_true:
  - TODO

success_criteria:
  - works well
  - reliable

non_goals: []

constraints: []

quality_bar:
  - TBD

proof_expectations: []

review_expectations: []

known_risks: []

resolved_questions: []
---

# OUTCOME
";
    fs::write(mission_dir.join("OUTCOME.md"), body).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_json(&output);
    assert_eq!(json["code"], "OUTCOME_INCOMPLETE");
    let placeholders: Vec<String> = json["context"]["placeholders"]
        .as_array()
        .expect("placeholders array")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    // Every injected boilerplate should surface somewhere in the list.
    let joined = placeholders.join("\n");
    assert!(
        joined.to_lowercase().contains("todo"),
        "missing TODO detection in: {joined}"
    );
    assert!(
        joined.to_lowercase().contains("tbd"),
        "missing TBD detection in: {joined}"
    );
    assert!(
        joined.to_lowercase().contains("works well")
            || joined.to_lowercase().contains("codex1 works well"),
        "missing 'works well' detection in: {joined}"
    );
    assert!(
        joined.to_lowercase().contains("reliable"),
        "missing 'reliable' detection in: {joined}"
    );
}

#[test]
fn ratify_on_invalid_outcome_does_not_mutate_state() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    let state_before = read_state(&mission_dir);
    let outcome_before = fs::read_to_string(mission_dir.join("OUTCOME.md")).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "ratify", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_json(&output);
    assert_eq!(json["code"], "OUTCOME_INCOMPLETE");

    let state_after = read_state(&mission_dir);
    assert_eq!(state_before["revision"], state_after["revision"]);
    assert_eq!(state_before["phase"], state_after["phase"]);
    assert_eq!(state_after["outcome"]["ratified"], false);
    let events = read_events(&mission_dir);
    assert!(
        events.is_empty(),
        "failed ratify must not append events; got {events:?}"
    );
    // OUTCOME.md untouched.
    let outcome_after = fs::read_to_string(mission_dir.join("OUTCOME.md")).unwrap();
    assert_eq!(outcome_before, outcome_after);
}

#[test]
fn ratify_on_valid_outcome_bumps_state_and_rewrites_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_valid_outcome(&mission_dir, "demo");
    let rev_before = read_state(&mission_dir)["revision"].as_u64().unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "ratify", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(output.status.success(), "ratify failed: {:?}", output);
    let json = parse_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["mission_id"], "demo");
    assert_eq!(json["data"]["phase"], "plan");
    assert!(
        json["data"]["ratified_at"].is_string(),
        "ratified_at missing: {json}"
    );
    assert_eq!(json["revision"], rev_before + 1);

    // State mutated.
    let state_after = read_state(&mission_dir);
    assert_eq!(state_after["revision"], rev_before + 1);
    assert_eq!(state_after["phase"], "plan");
    assert_eq!(state_after["outcome"]["ratified"], true);
    assert!(state_after["outcome"]["ratified_at"].is_string());

    // Event appended.
    let events = read_events(&mission_dir);
    assert_eq!(events.len(), 1, "expected one event, got {events:?}");
    assert_eq!(events[0]["kind"], "outcome.ratified");
    assert_eq!(events[0]["payload"]["mission_id"], "demo");

    // OUTCOME.md frontmatter flipped.
    let outcome = fs::read_to_string(mission_dir.join("OUTCOME.md")).unwrap();
    assert!(
        outcome.contains("status: ratified"),
        "frontmatter not flipped: {outcome}"
    );
    assert!(
        !outcome.contains("status: draft"),
        "old status: draft still present: {outcome}"
    );
    // Body preserved.
    assert!(outcome.contains("# OUTCOME"));
    assert!(outcome.contains("Body paragraph for the test fixture."));
}

#[test]
fn ratify_dry_run_does_not_mutate() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_valid_outcome(&mission_dir, "demo");
    let state_before = read_state(&mission_dir);
    let outcome_before = fs::read_to_string(mission_dir.join("OUTCOME.md")).unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "ratify", "--mission", "demo", "--dry-run"])
        .output()
        .expect("runs");
    assert!(output.status.success(), "dry-run failed: {:?}", output);
    let json = parse_json(&output);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["dry_run"], true);
    assert_eq!(json["data"]["phase"], "plan");

    let state_after = read_state(&mission_dir);
    assert_eq!(state_before["revision"], state_after["revision"]);
    assert_eq!(state_before["phase"], state_after["phase"]);
    assert_eq!(state_after["outcome"]["ratified"], false);
    let events = read_events(&mission_dir);
    assert!(events.is_empty());
    let outcome_after = fs::read_to_string(mission_dir.join("OUTCOME.md")).unwrap();
    assert_eq!(outcome_before, outcome_after);
}

#[test]
fn ratify_rejects_stale_expect_revision() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_valid_outcome(&mission_dir, "demo");
    let rev_before = read_state(&mission_dir)["revision"].as_u64().unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "outcome",
            "ratify",
            "--mission",
            "demo",
            "--expect-revision",
            "999",
        ])
        .output()
        .expect("runs");
    assert!(!output.status.success());
    let json = parse_json(&output);
    assert_eq!(json["code"], "REVISION_CONFLICT");
    assert_eq!(json["retryable"], true);
    assert_eq!(json["context"]["expected"], 999);
    assert_eq!(json["context"]["actual"], rev_before);

    // State unchanged after rejected mutation.
    let state_after = read_state(&mission_dir);
    assert_eq!(state_after["revision"], rev_before);
    assert_eq!(state_after["outcome"]["ratified"], false);
}

#[test]
fn check_after_ratify_still_ratifiable() {
    let tmp = TempDir::new().unwrap();
    let mission_dir = init_demo(&tmp, "demo");
    seed_valid_outcome(&mission_dir, "demo");

    let ratify = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "ratify", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(ratify.status.success());

    let check = cmd()
        .current_dir(tmp.path())
        .args(["outcome", "check", "--mission", "demo"])
        .output()
        .expect("runs");
    assert!(
        check.status.success(),
        "check after ratify failed: {:?}",
        check
    );
    let json = parse_json(&check);
    assert_eq!(json["ok"], Value::Bool(true));
    assert_eq!(json["data"]["ratifiable"], true);
}
