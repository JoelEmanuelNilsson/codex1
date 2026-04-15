use std::path::Path;

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn run_json(repo_root: &Path, args: &[&str], input: Value) -> Value {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let output = Command::new(binary)
        .args(args)
        .arg("--repo-root")
        .arg(repo_root)
        .arg("--json")
        .write_stdin(serde_json::to_vec(&input).expect("encode input"))
        .output()
        .expect("run codex1 internal command");

    assert!(
        output.status.success(),
        "command {:?} failed with stderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("stdout should contain JSON")
}

fn run_json_failure(repo_root: &Path, args: &[&str], input: Value) -> String {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let output = Command::new(binary)
        .args(args)
        .arg("--repo-root")
        .arg(repo_root)
        .arg("--json")
        .write_stdin(serde_json::to_vec(&input).expect("encode input"))
        .output()
        .expect("run codex1 internal command");

    assert!(
        !output.status.success(),
        "command {:?} unexpectedly succeeded with stdout: {}",
        args,
        String::from_utf8_lossy(&output.stdout)
    );

    String::from_utf8_lossy(&output.stderr).to_string()
}

fn run_stop_hook(repo_root: &Path) -> Value {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let output = Command::new(binary)
        .args(["internal", "stop-hook"])
        .current_dir(repo_root)
        .write_stdin(
            serde_json::to_vec(&json!({ "cwd": repo_root.display().to_string() }))
                .expect("encode stop-hook input"),
        )
        .output()
        .expect("run stop-hook");

    assert!(
        output.status.success(),
        "stop-hook failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stop-hook stdout should contain JSON")
}

fn run_stop_hook_raw(repo_root: &Path) -> std::process::Output {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    Command::new(binary)
        .args(["internal", "stop-hook"])
        .current_dir(repo_root)
        .write_stdin(
            serde_json::to_vec(&json!({ "cwd": repo_root.display().to_string() }))
                .expect("encode stop-hook input"),
        )
        .output()
        .expect("run stop-hook")
}

#[test]
fn internal_help_prefers_canonical_command_names() {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let output = Command::new(binary)
        .args(["internal", "--help"])
        .output()
        .expect("run codex1 internal --help");

    assert!(
        output.status.success(),
        "internal help failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("materialize-plan"));
    assert!(stdout.contains("record-review-outcome"));
    assert!(stdout.contains("append-closeout"));
    assert!(stdout.contains("repair-state"));
    assert!(stdout.contains("validate-mission-artifacts"));
    assert!(stdout.contains("validate-visible-artifacts"));
    assert!(stdout.contains("validate-machine-state"));
    assert!(stdout.contains("validate-gates"));
    assert!(stdout.contains("validate-closeouts"));
    assert!(stdout.contains("latest-valid-closeout"));
    assert!(stdout.contains("inspect-effective-config"));
    assert!(stdout.contains("clear-selection-wait"));
}

fn canonical_blueprint_body(route: &str) -> String {
    format!(
        "# Program Blueprint\n\n## Locked Mission Reference\n\n- Integration test mission truth is locked.\n\n## Truth Register Summary\n\n- Internal runtime commands persist the mission state.\n\n## System Model\n\n- Touched surfaces: visible planning artifacts and hidden runtime state.\n\n## Invariants And Protected Behaviors\n\n- Keep proof and review contracts explicit.\n\n## Proof Matrix\n\n- claim:default-proof\n\n## Decision Obligations\n\n- none\n\n## In-Scope Work Inventory\n\n- runtime_core\n\n## Selected Architecture\n\n{route}\n\n## Execution Graph and Safe-Wave Rules\n\n- Single-node routes may execute directly; multi-node routes must follow the declared graph frontier.\n\n## Decision Log\n\n- Chose the runtime-backed route because it keeps the mission contract explicit.\n\n## Review Bundle Design\n\n- Mandatory review lenses: correctness, evidence_adequacy\n\n## Workstream Overview\n\n- runtime_core\n\n## Risks And Unknowns\n\n- Integration coverage should stay honest.\n\n## Replan Policy\n\n- Reopen planning if route truth or proof changes.\n"
    )
}

fn canonical_spec_body(purpose: &str) -> String {
    canonical_spec_body_with_scope_and_note(
        purpose,
        &["crates/codex1", "crates/codex1-core"],
        &["crates/codex1", "crates/codex1-core"],
        "Keep the workstream bounded and reviewable.",
    )
}

fn canonical_spec_body_with_note(purpose: &str, implementation_note: &str) -> String {
    canonical_spec_body_with_scope_and_note(
        purpose,
        &["crates/codex1", "crates/codex1-core"],
        &["crates/codex1", "crates/codex1-core"],
        implementation_note,
    )
}

fn canonical_spec_body_with_scope_and_note(
    purpose: &str,
    read_scope: &[&str],
    write_scope: &[&str],
    implementation_note: &str,
) -> String {
    format!(
        "# Workstream Spec\n\n## Purpose\n\n{purpose}\n\n## In Scope\n\n- Execute the bounded integration slice.\n\n## Out Of Scope\n\n- Unrelated repo changes.\n\n## Dependencies\n\n- Outcome Lock and Program Blueprint stay current.\n\n## Touched Surfaces\n\n- Runtime backend.\n\n## Read Scope\n\n{}\n\n## Write Scope\n\n{}\n\n## Interfaces And Contracts Touched\n\n- internal command JSON contract\n\n## Implementation Shape\n\n{implementation_note}\n\n## Proof-Of-Completion Expectations\n\n- cargo test\n\n## Non-Breakage Expectations\n\n- Existing mission contracts still validate.\n\n## Review Lenses\n\n- correctness\n\n## Replan Boundary\n\n- Reopen planning on scope expansion.\n\n## Truth Basis Refs\n\n- PROGRAM-BLUEPRINT.md\n\n## Freshness Notes\n\n- Current for the integration test.\n\n## Support Files\n\n- `REVIEW.md`\n",
        read_scope
            .iter()
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n"),
        write_scope
            .iter()
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

#[test]
fn internal_runtime_flow_creates_mission_package_and_review_contracts() {
    let repo = TempDir::new().expect("temp repo");

    let mission_id = "runtime-flow";
    let init = run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Runtime Flow",
            "objective": "Wire the backend to create real mission contracts.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    assert_eq!(init["mission_id"], mission_id);

    let blueprint = run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use internal commands plus visible artifacts."),
            "plan_level": 5,
            "problem_size": "M",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [
                {
                    "spec_id": "runtime_core",
                    "purpose": "Create the first execution-safe workstream.",
                    "body_markdown": canonical_spec_body("Create the first execution-safe workstream."),
                    "artifact_status": "active",
                    "packetization_status": "runnable",
                    "execution_status": "packaged"
                }
            ]
        }),
    );
    assert_eq!(blueprint["mission_id"], mission_id);

    let package = run_json(
        repo.path(),
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [
                {"name": "lock_current", "satisfied": true, "detail": "Outcome Lock revision is current."}
            ],
            "read_scope": ["crates/codex1", "crates/codex1-core"],
            "write_scope": ["crates/codex1", "crates/codex1-core"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["spec review"]
        }),
    );
    assert_eq!(package["status"], "passed");
    let package_id = package["package_id"].as_str().expect("package id");

    let package_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-execution-package",
            "--mission-id",
            mission_id,
            "--package-id",
            package_id,
        ],
        json!({}),
    );
    assert_eq!(package_validation["valid"], true);

    let packet = run_json(
        repo.path(),
        &["internal", "derive-writer-packet"],
        json!({
            "mission_id": mission_id,
            "source_package_id": package_id,
            "target_spec_id": "runtime_core",
            "required_checks": ["cargo test"],
            "review_lenses": ["correctness", "evidence_adequacy"],
            "explicitly_disallowed_decisions": ["do not expand write scope", "do not choose new architecture"]
        }),
    );
    assert_eq!(packet["source_package_id"], package_id);

    let bundle = run_json(
        repo.path(),
        &["internal", "compile-review-bundle"],
        json!({
            "mission_id": mission_id,
            "source_package_id": package_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness", "evidence_adequacy"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test"],
            "receipts": ["RECEIPTS/test-output.txt"],
            "changed_files_or_diff": ["crates/codex1/src/internal/mod.rs"],
            "touched_interface_contracts": ["internal command JSON contract"]
        }),
    );
    let bundle_id = bundle["bundle_id"].as_str().expect("bundle id");

    let bundle_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-review-bundle",
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_id,
        ],
        json!({}),
    );
    assert_eq!(bundle_validation["valid"], true);

    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "integration-test",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["Review bundle is fresh and clean."],
            "next_required_branch": "execution"
        }),
    );
    assert_eq!(review["blocking_findings"], 0);

    let mission_close_package = run_json(
        repo.path(),
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [
                {"name": "lock_current", "satisfied": true, "detail": "Outcome Lock revision is current."}
            ],
            "read_scope": ["crates/codex1", "crates/codex1-core"],
            "write_scope": ["crates/codex1", "crates/codex1-core"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["mission close"]
        }),
    );
    let mission_close_package_id = mission_close_package["package_id"]
        .as_str()
        .expect("mission-close package id");
    let post_completion_bundle = run_json(
        repo.path(),
        &["internal", "compile-review-bundle"],
        json!({
            "mission_id": mission_id,
            "source_package_id": mission_close_package_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness", "evidence_adequacy"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test", "review clean"],
            "receipts": ["RECEIPTS/test-output.txt"],
            "changed_files_or_diff": ["crates/codex1/src/internal/mod.rs"],
            "touched_interface_contracts": ["internal command JSON contract"]
        }),
    );
    let post_completion_bundle_id = post_completion_bundle["bundle_id"]
        .as_str()
        .expect("post-completion bundle id");
    let post_completion_review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": post_completion_bundle_id,
            "reviewer": "integration-test",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["Post-completion review bundle is fresh and clean."],
            "next_required_branch": "mission_close"
        }),
    );
    assert_eq!(post_completion_review["blocking_findings"], 0);
    let outcome_lock_ref = repo
        .path()
        .join("PLANS/runtime-flow/OUTCOME-LOCK.md")
        .canonicalize()
        .expect("canonicalize outcome lock");
    let blueprint_ref = repo
        .path()
        .join("PLANS/runtime-flow/PROGRAM-BLUEPRINT.md")
        .canonicalize()
        .expect("canonicalize blueprint");
    let review_ledger_ref = repo
        .path()
        .join("PLANS/runtime-flow/REVIEW-LEDGER.md")
        .canonicalize()
        .expect("canonicalize review ledger");

    let mission_close_bundle = run_json(
        repo.path(),
        &["internal", "compile-review-bundle"],
        json!({
            "mission_id": mission_id,
            "source_package_id": mission_close_package_id,
            "bundle_kind": "mission_close",
            "mandatory_review_lenses": ["spec_conformance", "correctness", "evidence_adequacy"],
            "mission_level_proof_rows": ["cargo test", "review clean"],
            "cross_spec_claim_refs": ["runtime_core complete"],
            "visible_artifact_refs": [
                outcome_lock_ref.display().to_string(),
                blueprint_ref.display().to_string(),
                review_ledger_ref.display().to_string()
            ],
            "deferred_descoped_follow_on_refs": [],
            "open_finding_summary": []
        }),
    );
    let mission_close_bundle_id = mission_close_bundle["bundle_id"]
        .as_str()
        .expect("mission-close bundle id");

    let mission_close_review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": mission_close_bundle_id,
            "reviewer": "integration-test",
            "verdict": "complete",
            "governing_refs": ["mission-close-bundle"],
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["Mission-close review is clean."]
        }),
    );
    assert_eq!(mission_close_review["blocking_findings"], 0);

    let gate_index: Value = serde_json::from_slice(
        &std::fs::read(repo.path().join(".ralph/missions/runtime-flow/gates.json"))
            .expect("read gates index"),
    )
    .expect("parse gates index");
    let gates = gate_index["gates"].as_array().expect("gates array");
    assert!(
        gates
            .iter()
            .any(|gate| gate["gate_kind"] == "execution_package")
    );
    assert!(
        gates
            .iter()
            .any(|gate| gate["gate_kind"] == "blocking_review")
    );
    assert!(
        gates
            .iter()
            .any(|gate| gate["gate_kind"] == "mission_close_review")
    );
    assert!(
        repo.path()
            .join("PLANS/runtime-flow/PROGRAM-BLUEPRINT.md")
            .exists()
    );
    assert!(
        repo.path()
            .join("PLANS/runtime-flow/specs/runtime_core/SPEC.md")
            .exists()
    );

    let state: Value = serde_json::from_slice(
        &std::fs::read(repo.path().join(".ralph/missions/runtime-flow/state.json"))
            .expect("read Ralph state"),
    )
    .expect("parse Ralph state");
    assert_eq!(state["verdict"], "complete");
}

#[test]
fn canonical_materialize_plan_command_works() {
    let repo = TempDir::new().expect("temp repo");

    let mission_id = "canonical-plan";
    let init = run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Canonical Plan Command",
            "objective": "Exercise the canonical internal command names.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    assert_eq!(init["mission_id"], mission_id);

    let blueprint = run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use the canonical plan materialization command."),
            "plan_level": 4,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The canonical plan command writes planning truth.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [
                {
                    "spec_id": "runtime_core",
                    "purpose": "Create the first execution-safe workstream.",
                    "body_markdown": canonical_spec_body("Create the first execution-safe workstream."),
                    "artifact_status": "active",
                    "packetization_status": "runnable",
                    "execution_status": "packaged"
                }
            ]
        }),
    );

    assert_eq!(blueprint["mission_id"], mission_id);
}

#[test]
fn gate_and_closeout_inspection_commands_report_current_mission_state() {
    let repo = TempDir::new().expect("temp repo");

    let mission_id = "inspection-flow";
    let init = run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Inspection Flow",
            "objective": "Exercise narrow gate and closeout inspection commands.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    assert_eq!(init["mission_id"], mission_id);

    let gates = run_json(
        repo.path(),
        &["internal", "validate-gates", "--mission-id", mission_id],
        json!({}),
    );
    assert_eq!(gates["valid"], true);
    assert_eq!(gates["mission_id"], mission_id);
    assert!(gates["gate_count"].as_u64().unwrap_or(0) >= 1);

    let closeouts = run_json(
        repo.path(),
        &["internal", "validate-closeouts", "--mission-id", mission_id],
        json!({}),
    );
    assert_eq!(closeouts["valid"], true);
    assert_eq!(closeouts["mission_id"], mission_id);
    assert!(closeouts["closeout_count"].as_u64().unwrap_or(0) >= 1);

    let latest = run_json(
        repo.path(),
        &[
            "internal",
            "latest-valid-closeout",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(latest["mission_id"], mission_id);
    assert_eq!(latest["closeout_count"], closeouts["closeout_count"]);
    assert_eq!(
        latest["latest_closeout"]["mission_id"],
        Value::String(mission_id.to_string())
    );
}

#[test]
fn artifact_validation_split_reports_visible_and_machine_truth() {
    let repo = TempDir::new().expect("temp repo");

    let mission_id = "artifact-validation-flow";
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Artifact Validation Flow",
            "objective": "Exercise visible and machine validation split.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );

    run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use visible and machine validators against the same mission."),
            "plan_level": 5,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The mission has a bounded route.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [
                {
                    "spec_id": "runtime_core",
                    "purpose": "Create a minimal visible and machine mission surface.",
                    "body_markdown": canonical_spec_body(
                        "Create a minimal visible and machine mission surface.",
                    ),
                    "artifact_status": "active",
                    "packetization_status": "runnable",
                    "execution_status": "not_started"
                }
            ]
        }),
    );

    let visible = run_json(
        repo.path(),
        &[
            "internal",
            "validate-visible-artifacts",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(visible["success"], true);
    assert_eq!(visible["mission_id"], mission_id);

    let machine = run_json(
        repo.path(),
        &[
            "internal",
            "validate-machine-state",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(machine["success"], true);
    assert_eq!(machine["mission_id"], mission_id);

    let combined = run_json(
        repo.path(),
        &[
            "internal",
            "validate-mission-artifacts",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(combined["success"], true);
    assert_eq!(combined["mission_id"], mission_id);
    assert_eq!(combined["visible_artifacts"]["success"], true);
    assert_eq!(combined["machine_state"]["success"], true);
}

#[test]
fn visible_artifact_validation_fails_when_canonical_readme_is_missing() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "artifact-validation-missing-readme";
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Artifact Validation Missing README",
            "objective": "Prove missing visible artifacts fail validation.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );

    std::fs::remove_file(repo.path().join(format!("PLANS/{mission_id}/README.md")))
        .expect("remove README");

    let visible = run_json(
        repo.path(),
        &[
            "internal",
            "validate-visible-artifacts",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(visible["success"], false);
    assert!(
        visible["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .any(|finding| {
                finding["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("/README.md"))
                    && finding["level"] == "error"
            })
    );
}

#[test]
fn waiting_acknowledgements_preserve_canonical_stop_messages_until_resolution() {
    let repo = TempDir::new().expect("temp repo");

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "waiting-flow",
            "title": "Waiting Flow",
            "objective": "Exercise the two-phase waiting handshake.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Please choose the rollout posture.",
                "resume_condition": "The user chooses one rollout posture."
            }
        }),
    );

    let first_stop = run_stop_hook(repo.path());
    assert_eq!(
        first_stop["systemMessage"],
        "Please choose the rollout posture."
    );

    let waiting_state: Value = serde_json::from_slice(
        &std::fs::read(repo.path().join(".ralph/missions/waiting-flow/state.json"))
            .expect("read waiting Ralph state"),
    )
    .expect("parse waiting Ralph state");
    let waiting_request_id = waiting_state["waiting_request_id"]
        .as_str()
        .expect("waiting request id");

    run_json(
        repo.path(),
        &[
            "internal",
            "acknowledge-waiting-request",
            "--mission-id",
            "waiting-flow",
        ],
        json!({
            "waiting_request_id": waiting_request_id
        }),
    );

    let second_stop = run_stop_hook(repo.path());
    assert_eq!(
        second_stop["systemMessage"],
        "Please choose the rollout posture."
    );

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "other-mission",
            "title": "Other Mission",
            "objective": "Create resume-selection ambiguity.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Pick the other mission posture.",
                "resume_condition": "The user chooses the other mission posture."
            }
        }),
    );

    let selection = run_json(repo.path(), &["internal", "resolve-resume"], json!({}));
    assert_eq!(selection["resume_status"], "waiting_selection");
    assert_eq!(selection["selection_outcome"], "opened_selection_wait");
    let selection_state: Value = serde_json::from_slice(
        &std::fs::read(repo.path().join(".ralph/selection-state.json"))
            .expect("read selection state"),
    )
    .expect("parse selection state");
    let selection_request_id = selection_state["selection_request_id"]
        .as_str()
        .expect("selection request id");

    let repeated_selection = run_json(repo.path(), &["internal", "resolve-resume"], json!({}));
    assert_eq!(repeated_selection["resume_status"], "waiting_selection");
    assert_eq!(
        repeated_selection["selection_outcome"],
        "preserved_selection_wait"
    );

    let selection_stop = run_stop_hook(repo.path());
    assert_eq!(
        selection_stop["systemMessage"],
        "Select the mission to resume."
    );

    run_json(
        repo.path(),
        &["internal", "acknowledge-selection-request"],
        json!({
            "selection_request_id": selection_request_id
        }),
    );

    let acknowledged_selection_stop = run_stop_hook(repo.path());
    assert_eq!(
        acknowledged_selection_stop["systemMessage"],
        "Select the mission to resume."
    );

    run_json(
        repo.path(),
        &["internal", "resolve-selection-wait"],
        json!({
            "selected_mission_id": "other-mission"
        }),
    );

    let resolved = run_json(
        repo.path(),
        &["internal", "resolve-resume"],
        json!({
            "mission_id": "other-mission"
        }),
    );
    assert_eq!(resolved["resume_status"], "waiting_needs_user");
    assert_eq!(resolved["selection_outcome"], "explicit_mission_override");
    assert_eq!(resolved["selected_mission_id"], "other-mission");
    assert_eq!(resolved["selection_state_action"], "superseded");
    let explicit_selection_state: Value = serde_json::from_slice(
        &std::fs::read(repo.path().join(".ralph/selection-state.json"))
            .expect("read superseded selection state"),
    )
    .expect("parse superseded selection state");
    assert_eq!(
        explicit_selection_state["selection_request_id"],
        selection_request_id
    );
    assert!(explicit_selection_state["cleared_at"].is_string());
}

#[test]
fn stop_hook_blocks_on_malformed_selection_state_instead_of_replacing_it() {
    let repo = TempDir::new().expect("temp repo");

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "alpha",
            "title": "Alpha",
            "objective": "Create ambiguous selection state.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose alpha.",
                "resume_condition": "The user chooses alpha."
            }
        }),
    );
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "beta",
            "title": "Beta",
            "objective": "Create ambiguous selection state.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose beta.",
                "resume_condition": "The user chooses beta."
            }
        }),
    );
    let opened = run_json(repo.path(), &["internal", "resolve-resume"], json!({}));
    assert_eq!(opened["resume_status"], "waiting_selection");

    let selection_path = repo.path().join(".ralph/selection-state.json");
    std::fs::write(&selection_path, "{ not valid json").expect("corrupt selection state");

    let stop = run_stop_hook_raw(repo.path());
    assert!(
        stop.status.success(),
        "stop-hook should surface repair JSON instead of crashing: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    let parsed: Value = serde_json::from_slice(&stop.stdout).expect("parse stop-hook JSON");
    assert_eq!(parsed["decision"], "block");
    assert!(
        parsed["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("Repair malformed selection state"))
    );
    let current = std::fs::read_to_string(&selection_path).expect("read selection state");
    assert_eq!(current, "{ not valid json");
}

#[test]
fn resolve_resume_handles_no_single_and_ambiguous_candidates() {
    let repo = TempDir::new().expect("temp repo");

    let no_mission = run_json(repo.path(), &["internal", "resolve-resume"], json!({}));
    assert_eq!(no_mission["resume_status"], "no_active_mission");
    assert_eq!(no_mission["selection_outcome"], "no_active_mission");

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "solo-mission",
            "title": "Solo Mission",
            "objective": "Exercise single-candidate resume binding.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose the solo mission posture.",
                "resume_condition": "The user chooses the solo mission posture."
            }
        }),
    );

    let single = run_json(repo.path(), &["internal", "resolve-resume"], json!({}));
    assert_eq!(single["resume_status"], "waiting_needs_user");
    assert_eq!(single["selection_outcome"], "auto_bound_single_candidate");
    assert_eq!(single["selected_mission_id"], "solo-mission");

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "second-mission",
            "title": "Second Mission",
            "objective": "Exercise ambiguous resume binding.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose the second mission posture.",
                "resume_condition": "The user chooses the second mission posture."
            }
        }),
    );

    let ambiguous = run_json(repo.path(), &["internal", "resolve-resume"], json!({}));
    assert_eq!(ambiguous["resume_status"], "waiting_selection");
    assert_eq!(ambiguous["selection_outcome"], "opened_selection_wait");
    let ambiguous_selection_state: Value = serde_json::from_slice(
        &std::fs::read(repo.path().join(".ralph/selection-state.json"))
            .expect("read ambiguous selection state"),
    )
    .expect("parse ambiguous selection state");
    assert_eq!(
        ambiguous_selection_state["canonical_selection_request"],
        "Select the mission to resume."
    );

    run_json(
        repo.path(),
        &["internal", "resolve-selection-wait"],
        json!({
            "selected_mission_id": "second-mission"
        }),
    );
    let resolved_selection_resume =
        run_json(repo.path(), &["internal", "resolve-resume"], json!({}));
    assert_eq!(
        resolved_selection_resume["selection_outcome"],
        "consumed_resolved_selection"
    );
    assert_eq!(
        resolved_selection_resume["selection_state_action"],
        "consumed"
    );
    let consumed_selection_state: Value = serde_json::from_slice(
        &std::fs::read(repo.path().join(".ralph/selection-state.json"))
            .expect("read consumed selection state"),
    )
    .expect("parse consumed selection state");
    assert!(consumed_selection_state["cleared_at"].is_string());
}

#[test]
fn contradiction_driven_needs_user_keeps_wait_identity_and_yields_through_stop_hook() {
    let repo = TempDir::new().expect("temp repo");

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "contradiction-mission",
            "title": "Contradiction Mission",
            "objective": "Exercise contradiction-driven waiting.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );

    let contradiction = run_json(
        repo.path(),
        &["internal", "record-contradiction"],
        json!({
            "mission_id": "contradiction-mission",
            "discovered_in_phase": "execution",
            "discovered_by": "codex",
            "target_type": "spec",
            "target_id": "spec_api",
            "evidence_refs": ["RECEIPTS/test.txt"],
            "violated_assumption_or_contract": "Need the user to choose the rollout posture.",
            "suggested_reopen_layer": "execution_package",
            "reason_code": "needs_user",
            "governing_revision": "spec:spec_api:1",
            "status": "accepted_for_replan",
            "triage_decision": "reopen_execution_package",
            "triaged_by": "codex",
            "machine_action": "yield_needs_user",
            "next_required_branch": "needs_user"
        }),
    );
    let contradiction_id = contradiction["contradiction_id"]
        .as_str()
        .expect("contradiction id");

    let first = run_json(
        repo.path(),
        &["internal", "resolve-resume"],
        json!({
            "mission_id": "contradiction-mission"
        }),
    );
    assert_eq!(first["resume_status"], "waiting_needs_user");
    let first_state: Value = serde_json::from_slice(
        &std::fs::read(
            repo.path()
                .join(".ralph/missions/contradiction-mission/state.json"),
        )
        .expect("read first contradiction state"),
    )
    .expect("parse first contradiction state");
    let first_wait_id = first_state["waiting_request_id"]
        .as_str()
        .expect("first waiting id")
        .to_string();

    let second = run_json(
        repo.path(),
        &["internal", "resolve-resume"],
        json!({
            "mission_id": "contradiction-mission"
        }),
    );
    assert_eq!(second["resume_status"], "waiting_needs_user");
    let second_state: Value = serde_json::from_slice(
        &std::fs::read(
            repo.path()
                .join(".ralph/missions/contradiction-mission/state.json"),
        )
        .expect("read second contradiction state"),
    )
    .expect("parse second contradiction state");
    assert_eq!(
        second_state["waiting_request_id"].as_str(),
        Some(first_wait_id.as_str())
    );

    let stop = run_stop_hook(repo.path());
    let expected = format!(
        "Resolve contradiction {} with user input: Need the user to choose the rollout posture..",
        contradiction_id
    );
    assert_eq!(stop["systemMessage"].as_str(), Some(expected.as_str()));
}

#[test]
fn visible_artifact_validation_requires_replan_log_after_non_local_replan() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "artifact-validation-replan-log";
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Artifact Validation Replan Log",
            "objective": "Require visible replan history after non-local reopen.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );

    run_json(
        repo.path(),
        &["internal", "record-contradiction"],
        json!({
            "mission_id": mission_id,
            "discovered_in_phase": "execution",
            "discovered_by": "codex",
            "target_type": "spec",
            "target_id": "spec_api",
            "evidence_refs": ["RECEIPTS/test.txt"],
            "violated_assumption_or_contract": "The rollout needs a non-local reopen.",
            "suggested_reopen_layer": "blueprint",
            "reason_code": "non_local_replan",
            "governing_revision": "spec:spec_api:1",
            "status": "accepted_for_replan",
            "triage_decision": "reopen_blueprint",
            "triaged_by": "codex",
            "machine_action": "force_replan",
            "next_required_branch": "replan"
        }),
    );

    let visible = run_json(
        repo.path(),
        &[
            "internal",
            "validate-visible-artifacts",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(visible["success"], false);
    assert!(
        visible["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .any(|finding| {
                finding["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("/REPLAN-LOG.md"))
                    && finding["level"] == "error"
            })
    );
}

#[test]
fn visible_artifact_validation_requires_review_ledger_after_review_history() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "artifact-validation-review-ledger";
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Artifact Validation Review Ledger",
            "objective": "Require readable review history after review disposition.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Review history should require REVIEW-LEDGER."),
            "plan_level": 5,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The mission has a bounded route.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [
                {
                    "spec_id": "runtime_core",
                    "purpose": "Create review history.",
                    "body_markdown": canonical_spec_body("Create review history."),
                    "artifact_status": "active",
                    "packetization_status": "runnable",
                    "execution_status": "not_started"
                }
            ]
        }),
    );
    let package = run_json(
        repo.path(),
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [
                {"name": "lock_current", "satisfied": true, "detail": "Outcome Lock revision is current."}
            ],
            "read_scope": ["crates/codex1", "crates/codex1-core"],
            "write_scope": ["crates/codex1", "crates/codex1-core"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["correctness"]
        }),
    );
    let package_id = package["package_id"].as_str().expect("package id");
    let bundle = run_json(
        repo.path(),
        &["internal", "compile-review-bundle"],
        json!({
            "mission_id": mission_id,
            "source_package_id": package_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test"],
            "receipts": ["RECEIPTS/test-output.txt"],
            "changed_files_or_diff": ["crates/codex1/src/internal/mod.rs"],
            "touched_interface_contracts": ["internal command JSON contract"]
        }),
    );
    let bundle_id = bundle["bundle_id"].as_str().expect("bundle id");
    run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "integration-test",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["Review bundle is fresh and clean."],
            "next_required_branch": "execution"
        }),
    );

    std::fs::remove_file(
        repo.path()
            .join(format!("PLANS/{mission_id}/REVIEW-LEDGER.md")),
    )
    .expect("remove review ledger");

    let visible = run_json(
        repo.path(),
        &[
            "internal",
            "validate-visible-artifacts",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(visible["success"], false);
    assert!(
        visible["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .any(|finding| {
                finding["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("/REVIEW-LEDGER.md"))
                    && finding["level"] == "error"
            })
    );
}

#[test]
fn halt_hard_blocked_contradictions_stay_non_terminal_until_reviewed_closeout() {
    let repo = TempDir::new().expect("temp repo");

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "blocked-mission",
            "title": "Blocked Mission",
            "objective": "Exercise terminal contradiction handling.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );

    run_json(
        repo.path(),
        &["internal", "record-contradiction"],
        json!({
            "mission_id": "blocked-mission",
            "discovered_in_phase": "execution",
            "discovered_by": "codex",
            "target_type": "spec",
            "target_id": "spec_api",
            "evidence_refs": ["RECEIPTS/test.txt"],
            "violated_assumption_or_contract": "The rollout is blocked by a non-local contradiction.",
            "suggested_reopen_layer": "mission_lock",
            "reason_code": "hard_blocked",
            "governing_revision": "spec:spec_api:1",
            "status": "accepted_for_replan",
            "triage_decision": "reopen_mission_lock",
            "triaged_by": "codex",
            "machine_action": "halt_hard_blocked",
            "next_required_branch": "needs_user"
        }),
    );

    let resume = run_json(
        repo.path(),
        &["internal", "resolve-resume"],
        json!({
            "mission_id": "blocked-mission"
        }),
    );
    assert_eq!(resume["resume_status"], "contradictory_state");
    assert_eq!(resume["next_phase"], "replan");

    let state: Value = serde_json::from_slice(
        &std::fs::read(
            repo.path()
                .join(".ralph/missions/blocked-mission/state.json"),
        )
        .expect("read blocked mission state"),
    )
    .expect("parse blocked mission state");
    assert_eq!(state["verdict"], "replan_required");
    assert_eq!(state["terminality"], "actionable_non_terminal");

    let stop = run_stop_hook(repo.path());
    assert_eq!(stop["decision"], "block");
    assert!(
        stop["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("reviewed hard-block closeout"))
    );
}

#[test]
fn stop_hook_blocks_selection_resume_failures_instead_of_crashing() {
    let repo = TempDir::new().expect("temp repo");
    std::fs::create_dir_all(repo.path().join(".ralph")).expect("create .ralph");
    std::fs::write(
        repo.path().join(".ralph/selection-state.json"),
        serde_json::to_vec_pretty(&json!({
            "selection_request_id": "selection-1",
            "candidate_mission_ids": ["missing-mission"],
            "canonical_selection_request": "Select the mission to resume.",
            "selected_mission_id": "missing-mission",
            "request_emitted_at": null,
            "created_at": "2026-04-14T00:00:00Z",
            "resolved_at": "2026-04-14T00:01:00Z",
            "cleared_at": null
        }))
        .expect("encode selection state"),
    )
    .expect("write selection state");

    let stop = run_stop_hook_raw(repo.path());
    assert!(
        stop.status.success(),
        "stop-hook should emit repair JSON instead of crashing: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    let parsed: Value = serde_json::from_slice(&stop.stdout).expect("parse stop-hook JSON");
    assert_eq!(parsed["decision"], "block");
    assert!(
        parsed["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("Repair resume state before continuing"))
    );
}

#[test]
fn interrupted_waiting_cycle_resumes_as_interrupted_not_waiting() {
    let repo = TempDir::new().expect("temp repo");

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": "waiting-flow",
            "title": "Waiting Flow",
            "objective": "Exercise interrupted waiting resume truth.",
            "waiting_request": {
                "waiting_for": "human_decision",
                "canonical_request": "Choose the rollout posture.",
                "resume_condition": "The user chooses one rollout posture."
            }
        }),
    );

    let active_cycle_path = repo
        .path()
        .join(".ralph/missions/waiting-flow/active-cycle.json");
    std::fs::write(
        &active_cycle_path,
        serde_json::to_vec_pretty(&json!({
            "cycle_id": "cycle-interrupted",
            "mission_id": "waiting-flow",
            "phase": "clarify",
            "current_target": "mission:waiting-flow",
            "expected_child_lanes": [],
        }))
        .expect("serialize active cycle"),
    )
    .expect("write interrupted active cycle");

    let report = run_json(
        repo.path(),
        &["internal", "resolve-resume"],
        json!({
            "mission_id": "waiting-flow"
        }),
    );
    assert_eq!(report["active_cycle_status"], "interrupted");
    assert_eq!(report["resume_status"], "interrupted_cycle");
}

#[test]
fn newer_packages_and_bundles_stale_older_artifacts() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "freshness-flow";

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Freshness Flow",
            "objective": "Ensure newer runtime artifacts stale older ones.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use internal commands."),
            "plan_level": 4,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Exercise freshness.",
                "body_markdown": canonical_spec_body_with_scope_and_note(
                    "Exercise freshness.",
                    &["src"],
                    &["src"],
                    "Keep the workstream bounded and reviewable."
                ),
                "artifact_status": "active",
                "packetization_status": "runnable",
                "execution_status": "packaged"
            }]
        }),
    );

    let package_one = run_json(
        repo.path(),
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [{"name": "lock_current", "satisfied": true}],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["spec review"]
        }),
    );
    let package_one_id = package_one["package_id"].as_str().expect("package one id");

    let packet_one = run_json(
        repo.path(),
        &["internal", "derive-writer-packet"],
        json!({
            "mission_id": mission_id,
            "source_package_id": package_one_id,
            "target_spec_id": "runtime_core",
            "required_checks": ["cargo test"],
            "review_lenses": ["correctness"]
        }),
    );
    let packet_one_id = packet_one["packet_id"].as_str().expect("packet one id");

    let bundle_one = run_json(
        repo.path(),
        &["internal", "compile-review-bundle"],
        json!({
            "mission_id": mission_id,
            "source_package_id": package_one_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test"],
            "receipts": ["RECEIPTS/test-output.txt"],
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["runtime contract"]
        }),
    );
    let bundle_one_id = bundle_one["bundle_id"].as_str().expect("bundle one id");

    let package_two = run_json(
        repo.path(),
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [{"name": "lock_current", "satisfied": true}],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["spec review"]
        }),
    );
    let package_two_id = package_two["package_id"].as_str().expect("package two id");

    let package_one_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-execution-package",
            "--mission-id",
            mission_id,
            "--package-id",
            package_one_id,
        ],
        json!({}),
    );
    assert_eq!(package_one_validation["valid"], false);

    let packet_one_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-writer-packet",
            "--mission-id",
            mission_id,
            "--packet-id",
            packet_one_id,
        ],
        json!({}),
    );
    assert_eq!(packet_one_validation["valid"], false);

    let package_two_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-execution-package",
            "--mission-id",
            mission_id,
            "--package-id",
            package_two_id,
        ],
        json!({}),
    );
    assert_eq!(package_two_validation["valid"], true);

    let bundle_two = run_json(
        repo.path(),
        &["internal", "compile-review-bundle"],
        json!({
            "mission_id": mission_id,
            "source_package_id": package_two_id,
            "bundle_kind": "spec_review",
            "mandatory_review_lenses": ["correctness"],
            "target_spec_id": "runtime_core",
            "proof_rows_under_review": ["cargo test"],
            "receipts": ["RECEIPTS/test-output.txt"],
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["runtime contract"]
        }),
    );
    let bundle_two_id = bundle_two["bundle_id"].as_str().expect("bundle two id");

    let bundle_one_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-review-bundle",
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_one_id,
        ],
        json!({}),
    );
    assert_eq!(bundle_one_validation["valid"], false);

    let bundle_two_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-review-bundle",
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_two_id,
        ],
        json!({}),
    );
    assert_eq!(bundle_two_validation["valid"], true);

    let stale_review_error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_one_id,
            "reviewer": "integration-test",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["Should fail because the bundle is stale."],
            "next_required_branch": "execution"
        }),
    );
    assert!(stale_review_error.contains("review bundle"));
}

#[test]
fn planning_reuses_or_advances_spec_revision_honestly() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "revision-flow";

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Revision Flow",
            "objective": "Ensure spec revisions are meaningful.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );

    let first = run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use runtime commands."),
            "plan_level": 4,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Initial purpose.",
                "body_markdown": canonical_spec_body("Initial purpose.")
            }]
        }),
    );
    assert_eq!(first["written_specs"][0]["spec_revision"], 1);

    let unchanged = run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use runtime commands."),
            "plan_level": 4,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Initial purpose.",
                "body_markdown": canonical_spec_body("Initial purpose.")
            }]
        }),
    );
    assert_eq!(unchanged["written_specs"][0]["spec_revision"], 1);

    let changed = run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use runtime commands. Changed details."),
            "plan_level": 4,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Initial purpose.",
                "body_markdown": canonical_spec_body_with_note("Initial purpose.", "Changed body.")
            }]
        }),
    );
    assert_eq!(changed["written_specs"][0]["spec_revision"], 2);
}

#[test]
fn mission_close_bundle_requires_integrated_visible_truth() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "mission-close-validation";

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Mission Close Validation",
            "objective": "Ensure mission-close bundles are complete.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    let blueprint = run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use runtime commands."),
            "plan_level": 4,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Initial purpose.",
                "body_markdown": canonical_spec_body_with_scope_and_note(
                    "Initial purpose.",
                    &["src"],
                    &["src"],
                    "Keep the workstream bounded and reviewable."
                ),
                "artifact_status": "active",
                "packetization_status": "runnable",
                "execution_status": "packaged"
            }]
        }),
    );
    assert_eq!(blueprint["written_specs"][0]["spec_revision"], 1);
    let package = run_json(
        repo.path(),
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "runtime_core",
            "included_spec_ids": ["runtime_core"],
            "dependency_satisfaction_state": [{"name": "lock_current", "satisfied": true}],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["mission close"]
        }),
    );
    let package_id = package["package_id"].as_str().expect("package id");
    let outcome_lock_ref = repo
        .path()
        .join("PLANS/mission-close-validation/OUTCOME-LOCK.md")
        .canonicalize()
        .expect("canonicalize outcome lock");
    let blueprint_ref = repo
        .path()
        .join("PLANS/mission-close-validation/PROGRAM-BLUEPRINT.md")
        .canonicalize()
        .expect("canonicalize blueprint");

    let incomplete_bundle = run_json(
        repo.path(),
        &["internal", "compile-review-bundle"],
        json!({
            "mission_id": mission_id,
            "source_package_id": package_id,
            "bundle_kind": "mission_close",
            "mandatory_review_lenses": ["correctness"],
            "mission_level_proof_rows": ["cargo test"],
            "cross_spec_claim_refs": [],
            "visible_artifact_refs": [
                outcome_lock_ref.display().to_string(),
                blueprint_ref.display().to_string()
            ],
            "deferred_descoped_follow_on_refs": [],
            "open_finding_summary": []
        }),
    );
    let bundle_id = incomplete_bundle["bundle_id"].as_str().expect("bundle id");
    let validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-review-bundle",
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_id,
        ],
        json!({}),
    );
    assert_eq!(validation["valid"], false);
    let findings = validation["findings"].as_array().expect("findings");
    assert!(
        findings
            .iter()
            .any(|value| value == "visible_artifact_ref_missing:REVIEW-LEDGER.md")
    );
}

#[test]
fn internal_write_closeout_rejects_terminal_verdicts() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "terminal-closeout";

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Terminal Closeout",
            "objective": "Reject terminal low-level closeouts."
        }),
    );

    let error = run_json_failure(
        repo.path(),
        &["internal", "append-closeout", "--mission-id", mission_id],
        json!({
            "closeout_seq": 0,
            "mission_id": mission_id,
            "phase": "review",
            "activity": "forced_terminal_closeout",
            "verdict": "complete",
            "terminality": "terminal",
            "resume_mode": "allow_stop",
            "next_phase": "complete",
            "next_action": "Should not be accepted.",
            "cycle_kind": "mission_close",
            "reason_code": "forced_terminal_closeout",
            "summary": "This should fail."
        }),
    );
    assert!(error.contains("terminal closeouts must come from workflow-specific reviewed paths"));
}
