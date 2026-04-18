//! Wave 3 acceptance: failed review → `needs_repair` → retry → clean.

use assert_cmd::Command;
use serde_json::{json, Value};
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

fn boot(dir: &TempDir) {
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
        "tasks": { "T1": { "status": "ready" } }
    });
    fs::write(&sp, serde_json::to_vec_pretty(&new).unwrap()).unwrap();
}

fn write_proof(dir: &Path, task_id: &str, content: &[u8]) {
    let proof = dir.join(format!("PLANS/m1/specs/{task_id}/PROOF.md"));
    fs::create_dir_all(proof.parent().unwrap()).unwrap();
    fs::write(&proof, content).unwrap();
}

fn start_finish(dir: &TempDir, task_id: &str, proof_body: &[u8]) -> Value {
    bin(dir)
        .args(["--json", "task", "start", "--mission", "m1", task_id])
        .assert()
        .success();
    write_proof(dir.path(), task_id, proof_body);
    let out = bin(dir)
        .args(["--json", "task", "finish", "--mission", "m1", task_id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    last_json(&out)
}

fn open_and_close_with_finding(
    dir: &TempDir,
    task_id: &str,
    task_run_id: &str,
    proof_hash: &str,
) {
    let out = bin(dir)
        .args([
            "--json",
            "review",
            "open",
            "--mission",
            "m1",
            "--task",
            task_id,
            "--profiles",
            "code_bug_correctness",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let bundle_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    let bundle: Value = serde_json::from_slice(
        &fs::read(
            dir.path()
                .join(format!("PLANS/m1/reviews/{bundle_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    let req_id = bundle["requirements"][0]["id"].as_str().unwrap().to_string();
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();

    let output_path = dir.path().join(format!("reviewer-{bundle_id}.json"));
    fs::write(
        &output_path,
        serde_json::to_vec_pretty(&json!({
            "bundle_id": bundle_id,
            "reviewer_id": "R1",
            "reviewer_role": "reviewer",
            "requirement_id": req_id,
            "profile": "code_bug_correctness",
            "task_id": task_id,
            "task_run_id": task_run_id,
            "graph_revision": graph_rev,
            "state_revision": state_rev,
            "evidence_snapshot_hash": proof_hash,
            "packet_id": format!("pkt-{bundle_id}"),
            "result": "findings",
            "findings": [{
                "severity": "P1",
                "title": "fix this",
                "rationale": "r"
            }],
            "produced_at": "2026-04-18T10:00:00Z"
        }))
        .unwrap(),
    )
    .unwrap();
    let rel = output_path.strip_prefix(dir.path()).unwrap();
    bin(dir)
        .args([
            "--json",
            "review",
            "submit",
            "--mission",
            "m1",
            "--bundle",
            &bundle_id,
            "--input",
            rel.to_str().unwrap(),
        ])
        .assert()
        .success();
    bin(dir)
        .args([
            "--json", "review", "close", "--mission", "m1", "--bundle", &bundle_id,
        ])
        .assert()
        .success();
}

fn open_and_close_clean(
    dir: &TempDir,
    task_id: &str,
    task_run_id: &str,
    proof_hash: &str,
) {
    let out = bin(dir)
        .args([
            "--json",
            "review",
            "open",
            "--mission",
            "m1",
            "--task",
            task_id,
            "--profiles",
            "code_bug_correctness",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let bundle_id = last_json(&out)["bundle_id"].as_str().unwrap().to_string();
    let bundle: Value = serde_json::from_slice(
        &fs::read(
            dir.path()
                .join(format!("PLANS/m1/reviews/{bundle_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    let req_id = bundle["requirements"][0]["id"].as_str().unwrap().to_string();
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();

    let output_path = dir.path().join(format!("reviewer-{bundle_id}.json"));
    fs::write(
        &output_path,
        serde_json::to_vec_pretty(&json!({
            "bundle_id": bundle_id,
            "reviewer_id": "R1",
            "reviewer_role": "reviewer",
            "requirement_id": req_id,
            "profile": "code_bug_correctness",
            "task_id": task_id,
            "task_run_id": task_run_id,
            "graph_revision": graph_rev,
            "state_revision": state_rev,
            "evidence_snapshot_hash": proof_hash,
            "packet_id": format!("pkt-{bundle_id}"),
            "result": "none",
            "findings": [],
            "produced_at": "2026-04-18T10:00:00Z"
        }))
        .unwrap(),
    )
    .unwrap();
    let rel = output_path.strip_prefix(dir.path()).unwrap();
    bin(dir)
        .args([
            "--json",
            "review",
            "submit",
            "--mission",
            "m1",
            "--bundle",
            &bundle_id,
            "--input",
            rel.to_str().unwrap(),
        ])
        .assert()
        .success();
    bin(dir)
        .args([
            "--json", "review", "close", "--mission", "m1", "--bundle", &bundle_id,
        ])
        .assert()
        .success();
}

#[test]
fn fail_repair_pass_cycle_reaches_review_clean() {
    let dir = TempDir::new().unwrap();
    boot(&dir);

    // First run: fail
    let finish = start_finish(&dir, "T1", b"proof v1");
    let run_id = finish["task_run_id"].as_str().unwrap().to_string();
    let hash = finish["proof_hash"].as_str().unwrap().to_string();
    open_and_close_with_finding(&dir, "T1", &run_id, &hash);

    // Task should now be needs_repair
    let state: Value = serde_json::from_slice(
        &fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "needs_repair");

    // Second run: pass
    let finish2 = start_finish(&dir, "T1", b"proof v2 better");
    let run_id2 = finish2["task_run_id"].as_str().unwrap().to_string();
    assert_ne!(run_id, run_id2, "repair should mint fresh run id");
    let hash2 = finish2["proof_hash"].as_str().unwrap().to_string();
    open_and_close_clean(&dir, "T1", &run_id2, &hash2);

    let state: Value = serde_json::from_slice(
        &fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "review_clean");
}

#[test]
fn needs_repair_is_eligible_to_restart() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let finish = start_finish(&dir, "T1", b"p1");
    let run_id = finish["task_run_id"].as_str().unwrap().to_string();
    let hash = finish["proof_hash"].as_str().unwrap().to_string();
    open_and_close_with_finding(&dir, "T1", &run_id, &hash);

    // Wave derivation should place T1 back in a serial wave.
    let out = bin(&dir)
        .args(["--json", "plan", "waves", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["waves"][0]["tasks"], json!(["T1"]));
}
