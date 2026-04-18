//! Wave 5 acceptance: mission-close lifecycle — check, open mission-close
//! bundle, submit clean output, close, complete.

use assert_cmd::Command;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn bin(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codex1-v2").expect("binary built");
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
    fs::write(
        dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md"),
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         <!-- codex1:plan-dag:end -->\n",
    )
    .unwrap();
    let sp = dir.path().join("PLANS/m1/STATE.json");
    let current: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    let new = serde_json::json!({
        "mission_id": current["mission_id"],
        "state_revision": current["state_revision"],
        "phase": "executing",
        "parent_loop": { "mode": "none", "paused": false },
        "tasks": { "T1": { "status": "review_clean" } }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();
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
    let req_id = bundle["requirements"][0]["id"].as_str().unwrap().to_string();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let evidence = bundle["evidence_snapshot_hash"].as_str().unwrap().to_string();

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
        .args([
            "--json", "mission-close", "check", "--mission", "m1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    assert!(env["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["code"] == "MISSION_CLOSE_BUNDLE_MISSING"));
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
        .args([
            "--json", "mission-close", "check", "--mission", "m1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["can_close"], false);
    assert!(env["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["code"] == "TASK_NOT_CLEAN"));
}

#[test]
fn complete_refuses_when_check_fails() {
    let dir = TempDir::new().unwrap();
    boot_with_clean_task(&dir); // no mission-close bundle
    let out = bin(&dir)
        .args([
            "--json", "mission-close", "complete", "--mission", "m1",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], false);
    assert!(env["message"]
        .as_str()
        .unwrap()
        .contains("refuse to complete"));
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
        .args([
            "--json", "mission-close", "check", "--mission", "m1",
        ])
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
        .args([
            "--json", "mission-close", "complete", "--mission", "m1",
        ])
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
        .args([
            "--json", "mission-close", "complete", "--mission", "m1",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert!(env["message"]
        .as_str()
        .unwrap()
        .contains("already complete"));
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
