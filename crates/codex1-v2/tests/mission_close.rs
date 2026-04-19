//! Wave 5 acceptance: mission-close lifecycle — check, open mission-close
//! bundle, submit clean output, close, complete.

use assert_cmd::Command;
use serde_json::{Value, json};
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

fn boot_with_clean_task(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    ratify_lock(dir);
    fs::write(
        dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md"),
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         <!-- codex1:plan-dag:end -->\n",
    )
    .unwrap();
    // Round 9 lineage check: every terminal task needs a recorded
    // task_run_id + proof_hash, and a Clean task-scoped review bundle
    // bound to both. Fixture fakes all three so fast-forward tests
    // stay honest about what the real execution path would produce.
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": "executing",
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": {
            "T1": {
                "status": "review_clean",
                "task_run_id": "run-T1",
                "proof_hash": "sha256:proof-T1"
            }
        }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();

    // Write a Clean task-scoped review bundle bound to (T1, run-T1,
    // sha256:proof-T1) so the lineage check in check_readiness is
    // satisfied. Filename must match the B<digits>.json convention
    // that review::load_all_bundles enforces; B100 keeps it out of
    // the way of sequential bundle ids minted by later CLI calls.
    let reviews = dir.path().join("PLANS/m1/reviews");
    fs::create_dir_all(&reviews).unwrap();
    let task_bundle = serde_json::json!({
        "bundle_id": "B100",
        "mission_id": "m1",
        "graph_revision": 1,
        "state_revision": current["state_revision"],
        "target": {
            "kind": "task",
            "task_id": "T1",
            "task_run_id": "run-T1"
        },
        "requirements": [{
            "id": "req-b100-code",
            "profile": "code_bug_correctness",
            "min_outputs": 1,
            "allowed_roles": ["reviewer"]
        }],
        "evidence_refs": ["specs/T1/PROOF.md"],
        "evidence_snapshot_hash": "sha256:proof-T1",
        "status": "clean",
        "opened_at": "2026-04-19T10:00:00Z",
        "closed_at": "2026-04-19T10:05:00Z",
        "opener_role": "parent"
    });
    fs::write(
        reviews.join("B100.json"),
        serde_json::to_vec_pretty(&task_bundle).unwrap(),
    )
    .unwrap();
}

/// Flip OUTCOME-LOCK.md to `lock_status: ratified`. Round 8 Fix #1
/// requires the lock to be ratified before mission-close can complete,
/// so tests that fast-forward past $clarify need to flip the status
/// themselves (the same hand-edit $clarify does in the skill).
fn ratify_lock(dir: &TempDir) {
    let lock_path = dir.path().join("PLANS/m1/OUTCOME-LOCK.md");
    let content = fs::read_to_string(&lock_path).unwrap();
    let ratified = content.replace("lock_status: draft", "lock_status: ratified");
    assert_ne!(
        content, ratified,
        "test fixture expected a draft lock to flip to ratified"
    );
    fs::write(&lock_path, ratified).unwrap();
}

fn submit_clean_mc_output(dir: &TempDir, bundle_id: &str) {
    let bundle: Value = serde_json::from_slice(
        &fs::read(
            dir.path()
                .join(format!("PLANS/m1/reviews/{bundle_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    let req_id = bundle["requirements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let evidence = bundle["evidence_snapshot_hash"]
        .as_str()
        .unwrap()
        .to_string();

    let path = dir.path().join(format!("mc-output-{bundle_id}.json"));
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "bundle_id": bundle_id,
            "reviewer_id": "R9",
            "reviewer_role": "reviewer",
            "requirement_id": req_id,
            "profile": "mission_close",
            "graph_revision": graph_rev,
            "state_revision": state_rev,
            "evidence_snapshot_hash": evidence,
            "packet_id": format!("pkt-{bundle_id}"),
            "result": "none",
            "findings": [],
            "produced_at": "2026-04-18T10:00:00Z"
        }))
        .unwrap(),
    )
    .unwrap();
    let rel = path.strip_prefix(dir.path()).unwrap();
    bin(dir)
        .args([
            "--json",
            "review",
            "submit",
            "--mission",
            "m1",
            "--bundle",
            bundle_id,
            "--input",
            rel.to_str().unwrap(),
        ])
        .assert()
        .success();
}

fn write_proof(dir: &Path, task_id: &str) {
    let proof = dir.join(format!("PLANS/m1/specs/{task_id}/PROOF.md"));
    fs::create_dir_all(proof.parent().unwrap()).unwrap();
    fs::write(&proof, b"proof").unwrap();
}

#[test]
fn check_refuses_when_no_mission_close_bundle() {
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);
    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    assert!(
        env["blocking_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["code"] == "MISSION_CLOSE_BUNDLE_MISSING")
    );
}

#[test]
fn check_refuses_when_task_not_clean() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    fs::write(
        dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md"),
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         <!-- codex1:plan-dag:end -->\n",
    )
    .unwrap();
    // T1 not clean
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": "executing",
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": { "T1": { "status": "ready" } }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();

    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    assert!(
        env["blocking_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["code"] == "TASK_NOT_CLEAN")
    );
}

#[test]
fn complete_refuses_when_check_fails() {
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir); // no mission-close bundle
    let out = bin(&dir)
        .args(["--json", "mission-close", "complete", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], false);
    assert!(
        env["message"]
            .as_str()
            .unwrap()
            .contains("refuse to complete")
    );
}

#[test]
fn full_mission_close_lifecycle_reaches_terminal() {
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);

    // Open mission-close review bundle.
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let bundle_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();

    // Submit a clean reviewer output.
    submit_clean_mc_output(&dir, &bundle_id);

    // Close the bundle.
    bin(&dir)
        .args([
            "--json",
            "review",
            "close",
            "--mission",
            "m1",
            "--bundle",
            &bundle_id,
        ])
        .assert()
        .success();

    // Check should now pass.
    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], true);
    assert_eq!(env["mission_close_clean"], true);
    assert!(env["blocking_reasons"].as_array().unwrap().is_empty());

    // Complete should transition to terminal.
    let out = bin(&dir)
        .args(["--json", "mission-close", "complete", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["phase"], "complete");

    // Status envelope should now be complete/terminal.
    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["verdict"], "complete");
    assert_eq!(env["terminality"], "terminal");
    assert_eq!(env["next_action"]["kind"], "complete");

    // Second complete should refuse (already complete).
    let out = bin(&dir)
        .args(["--json", "mission-close", "complete", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert!(
        env["message"]
            .as_str()
            .unwrap()
            .contains("already complete")
    );
}

#[test]
fn status_envelope_emits_mission_close_check_when_all_clean_no_bundle() {
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);
    // No mission-close bundle yet.
    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["verdict"], "continue_required");
    assert_eq!(env["next_action"]["kind"], "mission_close_check");
}

#[test]
fn status_envelope_emits_mission_close_complete_when_bundle_clean() {
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let bundle_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    submit_clean_mc_output(&dir, &bundle_id);
    bin(&dir)
        .args([
            "--json",
            "review",
            "close",
            "--mission",
            "m1",
            "--bundle",
            &bundle_id,
        ])
        .assert()
        .success();

    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["next_action"]["kind"], "mission_close_complete");
    let _ = write_proof; // silence unused warning in this test
}

#[test]
fn hand_authored_review_clean_without_proof_or_task_bundle_is_blocked() {
    // Round 9 P1: a hand-authored STATE.json that jumps T1 to
    // review_clean (no proof_hash, no task_run_id) plus only a clean
    // mission-close bundle must NOT satisfy mission-close readiness.
    // The lineage check surfaces TASK_PROOF_MISSING (or
    // TASK_REVIEW_MISSING) so terminal close can't happen without
    // actual proof-of-work and proof-of-review.
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    ratify_lock(&dir);
    fs::write(
        dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md"),
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         <!-- codex1:plan-dag:end -->\n",
    )
    .unwrap();
    // Hand-author STATE: T1 claims review_clean with no lineage.
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    fs::write(
        &sp,
        serde_json::to_vec_pretty(&serde_json::json!({
            "mission_id": current["mission_id"],
            "state_revision": current["state_revision"],
            "phase": "executing",
            "parent_loop": { "mode": "none", "paused": false },
            "tasks": { "T1": { "status": "review_clean" } }
        }))
        .unwrap(),
    )
    .unwrap();

    // Open + close a clean mission-close bundle (no task-review bundle).
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let bundle_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    submit_clean_mc_output(&dir, &bundle_id);
    bin(&dir)
        .args([
            "--json",
            "review",
            "close",
            "--mission",
            "m1",
            "--bundle",
            &bundle_id,
        ])
        .assert()
        .success();

    // mission-close check must refuse with TASK_PROOF_MISSING.
    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    let codes: Vec<&str> = env["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["code"].as_str().unwrap())
        .collect();
    assert!(
        codes.contains(&"TASK_PROOF_MISSING"),
        "expected TASK_PROOF_MISSING in {codes:?}"
    );

    // mission-close complete must refuse.
    bin(&dir)
        .args(["--json", "mission-close", "complete", "--mission", "m1"])
        .assert()
        .failure();
}

#[test]
fn open_mission_close_refuses_when_tasks_not_terminal() {
    // Round 8 Fix #2a: mission-close review cannot open while any
    // non-superseded task is non-terminal. The bundle must bind to the
    // terminal truth it certifies, and there is no such truth yet.
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    ratify_lock(&dir);
    fs::write(
        dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md"),
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         <!-- codex1:plan-dag:end -->\n",
    )
    .unwrap();
    // T1 is still Ready, not ReviewClean.
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": "executing",
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": { "T1": { "status": "ready" } }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();

    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "MISSION_CLOSE_NOT_READY");
    assert_eq!(env["details"]["non_terminal_count"], 1);
    assert_eq!(env["details"]["task_ids"], serde_json::json!(["T1"]));
}

#[test]
fn clean_mission_close_bundle_becomes_stale_when_truth_drifts() {
    // Round 8 Fix #2c: open + close a Clean mission-close bundle, then
    // mutate task terminal truth out-of-band. `mission-close check`
    // must report MISSION_CLOSE_STALE and refuse to terminalize; status
    // must stay on mission_close_check, not mission_close_complete.
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let bundle_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    submit_clean_mc_output(&dir, &bundle_id);
    bin(&dir)
        .args([
            "--json",
            "review",
            "close",
            "--mission",
            "m1",
            "--bundle",
            &bundle_id,
        ])
        .assert()
        .success();

    // Drift: flip T1 from review_clean back to ready, simulating state
    // mutating after the mission-close reviewer certified the mission.
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"].as_u64().unwrap() + 1,
        "phase": current["phase"],
        "parent_loop": current["parent_loop"],
        "tasks": { "T1": { "status": "ready" } }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();

    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    let codes: Vec<&str> = env["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["code"].as_str().unwrap())
        .collect();
    assert!(
        codes.contains(&"MISSION_CLOSE_STALE"),
        "expected MISSION_CLOSE_STALE in {codes:?}"
    );
}

#[test]
fn status_stays_on_check_when_a_second_mission_close_bundle_is_open() {
    // Round 7 P1: opening a second mission-close bundle after the first
    // closed clean must force status back to mission_close_check so it
    // cannot silently route to mission_close_complete while a reviewer
    // is still finishing a re-review.
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);

    // First mission-close bundle: opened + clean-submitted + closed.
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let first_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    submit_clean_mc_output(&dir, &first_id);
    bin(&dir)
        .args([
            "--json",
            "review",
            "close",
            "--mission",
            "m1",
            "--bundle",
            &first_id,
        ])
        .assert()
        .success();

    // Second mission-close bundle: opened, still Open.
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let second_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    assert_ne!(first_id, second_id);

    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["next_action"]["kind"], "mission_close_check");

    // `mission-close check` surfaces the Open bundle as the blocker.
    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    assert_eq!(env["mission_close_bundle"], second_id);
    let blocking_codes: Vec<&str> = env["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["code"].as_str().unwrap())
        .collect();
    assert!(blocking_codes.contains(&"MISSION_CLOSE_OPEN"));
}

#[test]
fn draft_outcome_lock_blocks_mission_close() {
    // Round 8 Fix #1: mission-close must refuse to terminalize a mission
    // whose outcome lock is still draft. $clarify must ratify first.
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);
    // Undo the ratify_lock() done inside boot_with_clean_task so the
    // lock is back to draft.
    let lock_path = dir.path().join("PLANS/m1/OUTCOME-LOCK.md");
    let content = fs::read_to_string(&lock_path).unwrap();
    fs::write(
        &lock_path,
        content.replace("lock_status: ratified", "lock_status: draft"),
    )
    .unwrap();

    // Open, submit clean, close the MC bundle — everything except the
    // lock is ready to complete.
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let bundle_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    submit_clean_mc_output(&dir, &bundle_id);
    bin(&dir)
        .args([
            "--json",
            "review",
            "close",
            "--mission",
            "m1",
            "--bundle",
            &bundle_id,
        ])
        .assert()
        .success();

    // `mission-close check` refuses with LOCK_NOT_RATIFIED.
    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    let blocking_codes: Vec<&str> = env["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["code"].as_str().unwrap())
        .collect();
    assert!(
        blocking_codes.contains(&"LOCK_NOT_RATIFIED"),
        "expected LOCK_NOT_RATIFIED in {blocking_codes:?}"
    );

    // `mission-close complete` also refuses.
    bin(&dir)
        .args(["--json", "mission-close", "complete", "--mission", "m1"])
        .assert()
        .failure();

    // status() routes to user_decision / clarify, not mission_close_check.
    // Round 10 P1 strengthened this: a draft lock shunts status to
    // $clarify regardless of the rest of the mission state. Without
    // that, a draft mission that happened to have terminal tasks and
    // a clean mission-close bundle would be told to run mission-close
    // — exactly the class of false completion V2 is preventing.
    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["next_action"]["kind"], "user_decision");
    assert_eq!(env["required_user_decision"], "lock_not_ratified");
}

#[test]
fn status_fails_closed_on_corrupt_bundle_file() {
    // Round 7 P1: a corrupt reviews/B*.json must be a loud error, not a
    // silently-dropped bundle. status used to swallow the file and let
    // the remaining bundles decide the verdict, hiding ground truth.
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);
    let reviews_dir = dir.path().join("PLANS/m1/reviews");
    fs::create_dir_all(&reviews_dir).unwrap();
    fs::write(reviews_dir.join("B999.json"), b"{ not real json").unwrap();

    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], false);
    assert_eq!(env["code"], "REVIEW_BUNDLE_CORRUPT");

    // mission-close check also refuses.
    let out = bin(&dir)
        .args(["--json", "mission-close", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "REVIEW_BUNDLE_CORRUPT");
}

/// Round 14 P1: `review open-mission-close` must refuse when the
/// mission is already terminal (`phase: complete`). Without this
/// refusal, the CLI itself creates the dirty post-terminal bundle
/// that makes `mission-close check` block while status claims clean
/// completion.
#[test]
fn review_open_mission_close_refuses_terminal_mission() {
    let dir = TempDir::new().unwrap();
    bin(&dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    ratify_lock(&dir);
    fs::write(
        dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md"),
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         <!-- codex1:plan-dag:end -->\n",
    )
    .unwrap();
    // Hand-write a terminal STATE (phase=complete, T1=complete) with
    // the lineage fields so the fixture is internally consistent.
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": "complete",
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": {
            "T1": {
                "status": "complete",
                "task_run_id": "run-T1",
                "proof_hash": "sha256:proof-T1",
            }
        }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();

    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open-mission-close",
            "--mission",
            "m1",
            "--profiles",
            "mission_close",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "MISSION_ALREADY_TERMINAL");
    assert_eq!(env["details"]["mission_id"], "m1");
}

/// Round 14 P1: when a mission is terminal but a NEW mission-close
/// review bundle is Open (hand-written, or from a legacy build that
/// didn't refuse the open), `status` must agree with
/// `mission-close check` and route back to `mission_close_check`
/// instead of reporting `complete`. Previously status short-circuited
/// on `phase == Complete` and diverged from the check command.
#[test]
fn status_routes_to_mission_close_check_on_post_terminal_open_bundle() {
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir);
    // Move the fixture forward to phase=complete. (boot_with_clean_task
    // leaves T1 at review_clean + phase=executing. Flip both to their
    // terminal values to simulate `mission-close complete` having run.)
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": "complete",
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": {
            "T1": {
                "status": "complete",
                "task_run_id": "run-T1",
                "proof_hash": "sha256:proof-T1",
            }
        }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();

    // Drop a hand-written Open mission-close bundle into reviews/
    // (simulates the prior-build bug where `review open-mission-close`
    // didn't refuse a terminal mission, or any future path that
    // introduces post-terminal drift).
    let open_mc = serde_json::json!({
        "bundle_id": "B999",
        "mission_id": "m1",
        "graph_revision": 1,
        "state_revision": current["state_revision"],
        "target": { "kind": "mission_close" },
        "requirements": [{
            "id": "B999-mc",
            "profile": "mission_close",
            "min_outputs": 1,
            "allowed_roles": ["reviewer"],
        }],
        "evidence_refs": ["PROGRAM-BLUEPRINT.md", "STATE.json"],
        "evidence_snapshot_hash": "sha256:ignored",
        "status": "open",
        "opened_at": "2026-04-19T11:00:00Z",
        "closed_at": null,
        "opener_role": "parent",
    });
    fs::write(
        dir.path().join("PLANS/m1/reviews/B999.json"),
        serde_json::to_vec_pretty(&open_mc).unwrap(),
    )
    .unwrap();

    // Status must route to mission_close_check, not complete —
    // otherwise Ralph/status diverge from `mission-close check`.
    let out = bin(&dir)
        .args(["--json", "status", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["verdict"], "continue_required");
    assert_eq!(env["next_action"]["kind"], "mission_close_check");
}
