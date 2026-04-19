//! Wave 3 acceptance: `review open → submit → close` end-to-end, plus
//! parent-self-review refusal and stale binding quarantine.

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

fn boot(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    let bp = dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    fs::write(
        &bp,
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

fn do_start_and_finish(dir: &TempDir, task_id: &str) -> Value {
    bin(dir)
        .args(["--json", "task", "start", "--mission", "m1", task_id])
        .assert()
        .success();
    write_proof(dir.path(), task_id, b"proof v1");
    let out = bin(dir)
        .args(["--json", "task", "finish", "--mission", "m1", task_id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    last_json(&out)
}

fn open_review(dir: &TempDir, task_id: &str, profiles: &str) -> Value {
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
            profiles,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    last_json(&out)
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn write_reviewer_output(
    dir: &TempDir,
    bundle_id: &str,
    requirement_id: &str,
    profile: &str,
    task_id: &str,
    task_run_id: &str,
    evidence_hash: &str,
    state_revision: u64,
    graph_revision: u64,
    reviewer_role: &str,
    packet_id: &str,
    result_json: Value,
) -> std::path::PathBuf {
    let path = dir.path().join(format!("reviewer-output-{packet_id}.json"));
    let mut body = serde_json::Map::new();
    body.insert("bundle_id".into(), json!(bundle_id));
    body.insert("reviewer_id".into(), json!("R1"));
    body.insert("reviewer_role".into(), json!(reviewer_role));
    body.insert("requirement_id".into(), json!(requirement_id));
    body.insert("profile".into(), json!(profile));
    body.insert("task_id".into(), json!(task_id));
    body.insert("task_run_id".into(), json!(task_run_id));
    body.insert("graph_revision".into(), json!(graph_revision));
    body.insert("state_revision".into(), json!(state_revision));
    body.insert("evidence_snapshot_hash".into(), json!(evidence_hash));
    body.insert("packet_id".into(), json!(packet_id));
    body.insert("produced_at".into(), json!("2026-04-18T10:00:00Z"));
    for (k, v) in result_json.as_object().unwrap() {
        body.insert(k.clone(), v.clone());
    }
    fs::write(
        &path,
        serde_json::to_vec_pretty(&Value::Object(body)).unwrap(),
    )
    .unwrap();
    path
}

#[test]
fn clean_review_transitions_task_to_review_clean() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let finish = do_start_and_finish(&dir, "T1");
    let task_run_id = finish["task_run_id"].as_str().unwrap().to_string();
    let proof_hash = finish["proof_hash"].as_str().unwrap().to_string();

    let open = open_review(&dir, "T1", "code_bug_correctness");
    let bundle_id = open["bundle_id"].as_str().unwrap().to_string();

    // Read bundle to get requirement id + current revisions.
    let bundle_path = dir
        .path()
        .join(format!("PLANS/m1/reviews/{bundle_id}.json"));
    let bundle: Value = serde_json::from_slice(&fs::read(&bundle_path).unwrap()).unwrap();
    let req_id = bundle["requirements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();

    let input = write_reviewer_output(
        &dir,
        &bundle_id,
        &req_id,
        "code_bug_correctness",
        "T1",
        &task_run_id,
        &proof_hash,
        state_rev,
        graph_rev,
        "reviewer",
        "pkt-1",
        json!({ "result": "none", "findings": [] }),
    );
    let rel = input.strip_prefix(dir.path()).unwrap();
    bin(&dir)
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

    let out = bin(&dir)
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
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["clean"], true);

    // Verify task state.
    let state: Value =
        serde_json::from_slice(&fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap()).unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "review_clean");
}

#[test]
fn p1_finding_routes_task_to_needs_repair() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let finish = do_start_and_finish(&dir, "T1");
    let task_run_id = finish["task_run_id"].as_str().unwrap().to_string();
    let proof_hash = finish["proof_hash"].as_str().unwrap().to_string();

    let open = open_review(&dir, "T1", "code_bug_correctness");
    let bundle_id = open["bundle_id"].as_str().unwrap().to_string();
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
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();

    let input = write_reviewer_output(
        &dir,
        &bundle_id,
        &req_id,
        "code_bug_correctness",
        "T1",
        &task_run_id,
        &proof_hash,
        state_rev,
        graph_rev,
        "reviewer",
        "pkt-1",
        json!({
            "result": "findings",
            "findings": [{
                "severity": "P1",
                "title": "Bug found",
                "rationale": "test"
            }]
        }),
    );
    let rel = input.strip_prefix(dir.path()).unwrap();
    bin(&dir)
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

    let out = bin(&dir)
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
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["clean"], false);
    assert_eq!(env["blocking_findings"], 1);

    let state: Value =
        serde_json::from_slice(&fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap()).unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "needs_repair");
}

#[test]
fn self_review_is_refused() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let finish = do_start_and_finish(&dir, "T1");
    let task_run_id = finish["task_run_id"].as_str().unwrap().to_string();
    let proof_hash = finish["proof_hash"].as_str().unwrap().to_string();
    let open = open_review(&dir, "T1", "code_bug_correctness");
    let bundle_id = open["bundle_id"].as_str().unwrap().to_string();
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
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();

    // reviewer_role == "parent" should be refused at submit time.
    let input = write_reviewer_output(
        &dir,
        &bundle_id,
        &req_id,
        "code_bug_correctness",
        "T1",
        &task_run_id,
        &proof_hash,
        state_rev,
        graph_rev,
        "parent", // same as opener_role
        "pkt-1",
        json!({ "result": "none", "findings": [] }),
    );
    let rel = input.strip_prefix(dir.path()).unwrap();
    let out = bin(&dir)
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
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "STALE_OUTPUT");
    assert!(
        env["details"]["reason"]
            .as_str()
            .unwrap()
            .contains("parent")
    );
}

#[test]
fn stale_task_run_id_rejected_at_submit() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    let finish = do_start_and_finish(&dir, "T1");
    let proof_hash = finish["proof_hash"].as_str().unwrap().to_string();

    let open = open_review(&dir, "T1", "code_bug_correctness");
    let bundle_id = open["bundle_id"].as_str().unwrap().to_string();
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
    let state_rev = bundle["state_revision"].as_u64().unwrap();
    let graph_rev = bundle["graph_revision"].as_u64().unwrap();

    let input = write_reviewer_output(
        &dir,
        &bundle_id,
        &req_id,
        "code_bug_correctness",
        "T1",
        "run-STALE-id", // wrong run id
        &proof_hash,
        state_rev,
        graph_rev,
        "reviewer",
        "pkt-1",
        json!({ "result": "none", "findings": [] }),
    );
    let rel = input.strip_prefix(dir.path()).unwrap();
    let out = bin(&dir)
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
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "STALE_OUTPUT");
}

#[test]
fn review_open_transitions_task_to_review_owed() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    do_start_and_finish(&dir, "T1");
    open_review(&dir, "T1", "code_bug_correctness");
    let state: Value =
        serde_json::from_slice(&fs::read(dir.path().join("PLANS/m1/STATE.json")).unwrap()).unwrap();
    assert_eq!(state["tasks"]["T1"]["status"], "review_owed");
}

#[test]
fn review_open_requires_proof_submitted_status() {
    let dir = TempDir::new().unwrap();
    boot(&dir);
    // Start the task (in_progress) but don't finish.
    bin(&dir)
        .args(["--json", "task", "start", "--mission", "m1", "T1"])
        .assert()
        .success();
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open",
            "--mission",
            "m1",
            "--task",
            "T1",
            "--profiles",
            "code_bug_correctness",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "TASK_STATE_INVALID");
}

/// Round 6 Fix #3: `review open` must reject --profiles that drops any
/// profile declared in the blueprint. Otherwise the parent could open a
/// bundle that skips a mandatory review.
#[test]
fn review_open_rejects_profile_omission_from_blueprint() {
    let dir = TempDir::new().unwrap();
    // Custom boot with a blueprint that declares review_profiles explicitly.
    bin(&dir)
        .args(["--json", "init", "--mission", "m1", "--title", "t"])
        .assert()
        .success();
    let bp = dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    fs::write(
        &bp,
        "# BP\n\n<!-- codex1:plan-dag:start -->\n\
         planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n\
         \x20   review_profiles: [code_bug_correctness, local_spec_intent]\n\
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
    do_start_and_finish(&dir, "T1");

    // Try to open with only one of the two required profiles.
    let out = bin(&dir)
        .args([
            "--json",
            "review",
            "open",
            "--mission",
            "m1",
            "--task",
            "T1",
            "--profiles",
            "local_spec_intent",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "REVIEW_PROFILE_MISSING");
    assert_eq!(env["details"]["missing"], json!(["code_bug_correctness"]));

    // Opening with the full superset succeeds (regression check).
    bin(&dir)
        .args([
            "--json",
            "review",
            "open",
            "--mission",
            "m1",
            "--task",
            "T1",
            "--profiles",
            "code_bug_correctness,local_spec_intent,integration_intent",
        ])
        .assert()
        .success();
}
