//! Wave 1 acceptance: `codex1-v2 init` then `codex1-v2 validate`.
//!
//! Exercises the binary via `assert_cmd::Command::cargo_bin("codex1")`
//! inside a `tempfile::TempDir` so nothing leaks into the repo.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn bin(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codex1").expect("binary built");
    cmd.arg("--repo-root").arg(dir.path());
    cmd
}

fn parse_json(output: &[u8]) -> Value {
    let s = std::str::from_utf8(output).unwrap();
    let last = s.lines().last().expect("at least one stdout line");
    serde_json::from_str(last).expect("last stdout line is JSON")
}

#[test]
fn init_creates_all_four_files_with_initial_revisions() {
    let dir = TempDir::new().unwrap();
    let out = bin(&dir)
        .args([
            "--json",
            "init",
            "--mission",
            "smoke",
            "--title",
            "Smoke test",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], "codex1.init.v1");
    assert_eq!(env["mission_id"], "smoke");
    assert_eq!(env["state_revision"], 1);

    let mission_dir = dir.path().join("PLANS").join("smoke");
    for f in &[
        "OUTCOME-LOCK.md",
        "PROGRAM-BLUEPRINT.md",
        "STATE.json",
        "events.jsonl",
    ] {
        assert!(mission_dir.join(f).is_file(), "missing {f}");
    }

    let state: Value =
        serde_json::from_slice(&fs::read(mission_dir.join("STATE.json")).unwrap()).unwrap();
    assert_eq!(state["state_revision"], 1);
    assert_eq!(state["phase"], "clarify");
    assert_eq!(state["mission_id"], "smoke");

    let events_raw = fs::read_to_string(mission_dir.join("events.jsonl")).unwrap();
    let first_event: Value = serde_json::from_str(events_raw.lines().next().unwrap()).unwrap();
    assert_eq!(first_event["seq"], 1);
    assert_eq!(first_event["kind"], "mission_initialized");
}

#[test]
fn init_refuses_when_mission_dir_exists() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .success();
    let out = bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["ok"], false);
    assert_eq!(env["code"], "MISSION_EXISTS");
}

#[test]
fn init_rejects_unsafe_mission_ids() {
    let dir = TempDir::new().unwrap();
    for bad in ["..", "Upper", "has space", "/abs", "with/slash"] {
        let out = bin(&dir)
            .args(["--json", "init", "--mission", bad, "--title", "x"])
            .assert()
            .failure()
            .get_output()
            .stdout
            .clone();
        let env = parse_json(&out);
        assert_eq!(env["ok"], false);
        assert_eq!(env["code"], "MISSION_ID_INVALID", "should reject {bad}");
    }
}

#[test]
fn init_then_validate_succeeds() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .success();
    let out = bin(&dir)
        .args(["--json", "validate", "--mission", "smoke"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], "codex1.validate.v1");
    assert_eq!(env["state_revision"], 1);
    assert_eq!(env["task_count"], 0);
}

#[test]
fn validate_rejects_missing_destination_section() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .success();
    // Corrupt OUTCOME-LOCK by removing Destination section.
    let lock_path = dir.path().join("PLANS/smoke/OUTCOME-LOCK.md");
    let content = fs::read_to_string(&lock_path).unwrap();
    let corrupted = content.replace(
        "## Destination\n<!-- What does \"done\" look like from the user's perspective? Written during $clarify. -->\n",
        "",
    );
    fs::write(&lock_path, corrupted).unwrap();

    let out = bin(&dir)
        .args(["--json", "validate", "--mission", "smoke"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["ok"], false);
    assert_eq!(env["code"], "LOCK_INVALID");
}

#[test]
fn validate_rejects_mangled_frontmatter() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .success();
    let lock_path = dir.path().join("PLANS/smoke/OUTCOME-LOCK.md");
    fs::write(&lock_path, "no frontmatter here\n").unwrap();

    let out = bin(&dir)
        .args(["--json", "validate", "--mission", "smoke"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["code"], "LOCK_INVALID");
}

#[test]
fn validate_errors_when_events_seq_greater_than_state_revision() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .success();
    // Append a synthetic event with seq=99 > state_revision=1.
    let events_path = dir.path().join("PLANS/smoke/events.jsonl");
    let line = r#"{"seq":99,"kind":"malicious","at":"2026-04-18T00:00:00Z"}"#;
    let mut contents = fs::read_to_string(&events_path).unwrap();
    contents.push_str(line);
    contents.push('\n');
    fs::write(&events_path, contents).unwrap();

    let out = bin(&dir)
        .args(["--json", "validate", "--mission", "smoke"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["code"], "STATE_CORRUPT");
}

#[test]
fn validate_fails_closed_on_corrupt_review_bundle() {
    // Round 8 follow-up: a malformed reviews/B*.json file must make
    // `codex1 validate` refuse rather than report ok. `status` and
    // `mission-close check` already fail closed; validate's contract
    // to be the structural superset would be a lie otherwise.
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .success();
    let reviews = dir.path().join("PLANS/smoke/reviews");
    fs::create_dir_all(&reviews).unwrap();
    fs::write(reviews.join("B999.json"), b"{ not valid json").unwrap();

    let out = bin(&dir)
        .args(["--json", "validate", "--mission", "smoke"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["ok"], false);
    assert_eq!(env["code"], "REVIEW_BUNDLE_CORRUPT");
}

#[test]
fn validate_warns_but_succeeds_when_events_lag_state() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "smoke", "--title", "Smoke"])
        .assert()
        .success();
    // Truncate events.jsonl so last_seq < state_revision (state_revision=1 but
    // events.jsonl is empty → last_seq = None, which the validator treats
    // as a warning).
    let events_path = dir.path().join("PLANS/smoke/events.jsonl");
    fs::write(&events_path, "").unwrap();

    let out = bin(&dir)
        .args(["--json", "validate", "--mission", "smoke"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = parse_json(&out);
    assert_eq!(env["ok"], true);
    assert!(
        env["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap().contains("events.jsonl"))
    );
}
