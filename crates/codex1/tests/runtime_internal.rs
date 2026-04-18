use std::path::Path;

use assert_cmd::Command;
use codex1_core::runtime::{
    AdvisorCheckpointState, AutopilotPlanSealCaller, AutopilotPlanSealInput, NextRequiredBranch,
    evaluate_autopilot_plan_seal,
};
use serde_json::{Value, json};
use tempfile::TempDir;

fn test_parent_authority_token_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".ralph/test-parent-authority-token")
}

fn test_parent_authority_token(repo_root: &Path) -> Option<String> {
    std::fs::read_to_string(test_parent_authority_token_path(repo_root))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn run_json(repo_root: &Path, args: &[&str], input: Value) -> Value {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let mut command = Command::new(binary);
    command
        .args(args)
        .arg("--repo-root")
        .arg(repo_root)
        .arg("--json");
    if args.contains(&"begin-loop-lease") {
        command.env("CODEX1_PARENT_LOOP_BEGIN", "1");
    }
    if let Some(token) = test_parent_authority_token(repo_root) {
        command.env("CODEX1_PARENT_LOOP_AUTHORITY_TOKEN", token);
    }
    let output = command
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
    let mut command = Command::new(binary);
    command
        .args(args)
        .arg("--repo-root")
        .arg(repo_root)
        .arg("--json");
    if args.contains(&"begin-loop-lease") {
        command.env("CODEX1_PARENT_LOOP_BEGIN", "1");
    }
    if let Some(token) = test_parent_authority_token(repo_root) {
        command.env("CODEX1_PARENT_LOOP_AUTHORITY_TOKEN", token);
    }
    let output = command
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

fn run_json_failure_without_parent_begin(repo_root: &Path, args: &[&str], input: Value) -> String {
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

fn run_json_with_parent_authority(
    repo_root: &Path,
    args: &[&str],
    input: Value,
    authority_token: &str,
) -> Value {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let output = Command::new(binary)
        .args(args)
        .arg("--repo-root")
        .arg(repo_root)
        .arg("--json")
        .env("CODEX1_PARENT_LOOP_AUTHORITY_TOKEN", authority_token)
        .env("CODEX1_PARENT_LOOP_BEGIN", "1")
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

fn run_stop_hook_with_payload(repo_root: &Path, payload: Value) -> Value {
    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let output = Command::new(binary)
        .args(["internal", "stop-hook"])
        .current_dir(repo_root)
        .write_stdin(serde_json::to_vec(&payload).expect("encode stop-hook input"))
        .output()
        .expect("run stop-hook");

    assert!(
        output.status.success(),
        "stop-hook failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stop-hook stdout should contain JSON")
}

fn setup_review_wave(repo: &TempDir, mission_id: &str) -> (String, String) {
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Reviewer Lane Guard",
            "objective": "Prove reviewer lane mutation guard.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use one guarded review slice."),
            "plan_level": 5,
            "problem_size": "M",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Create a guarded review gate.",
                "body_markdown": canonical_spec_body("Create a guarded review gate."),
                "artifact_status": "active",
                "packetization_status": "runnable",
                "execution_status": "packaged"
            }]
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
    let package_id = package["package_id"]
        .as_str()
        .expect("package id")
        .to_string();
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
    let bundle_id = bundle["bundle_id"].as_str().expect("bundle id").to_string();
    (package_id, bundle_id)
}

fn capture_review_truth_snapshot(repo_root: &Path, mission_id: &str, bundle_id: &str) -> Value {
    if test_parent_authority_token(repo_root).is_none() {
        let lease = run_json(
            repo_root,
            &["internal", "begin-loop-lease"],
            json!({
                "mission_id": mission_id,
                "mode": "review_loop",
                "owner": "parent-review-loop-test",
                "reason": "Test helper parent review authority."
            }),
        );
        let token = lease["parent_authority_token"]
            .as_str()
            .expect("parent authority token");
        std::fs::create_dir_all(repo_root.join(".ralph")).expect("create ralph root");
        std::fs::write(test_parent_authority_token_path(repo_root), token)
            .expect("write parent authority token");
    }
    run_json(
        repo_root,
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_id,
        ],
        json!({}),
    )
}

fn capture_review_evidence_snapshot(repo_root: &Path, mission_id: &str, bundle_id: &str) -> Value {
    run_json(
        repo_root,
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            bundle_id,
        ],
        json!({}),
    )
}

fn record_none_reviewer_output(
    repo_root: &Path,
    mission_id: &str,
    bundle_id: &str,
    reviewer_id: &str,
) -> Value {
    run_json(
        repo_root,
        &["internal", "record-reviewer-output"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer_id": reviewer_id,
            "output_kind": "none",
            "findings": []
        }),
    )
}

fn record_blocking_reviewer_output(
    repo_root: &Path,
    mission_id: &str,
    bundle_id: &str,
    reviewer_id: &str,
) -> Value {
    run_json(
        repo_root,
        &["internal", "record-reviewer-output"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer_id": reviewer_id,
            "output_kind": "findings",
            "findings": [{
                "severity": "P1",
                "title": "Blocking reviewer finding",
                "evidence_refs": ["crates/codex1-core/src/runtime.rs:1"],
                "rationale": "This reviewer output intentionally blocks the reviewed target.",
                "suggested_next_action": "Repair the reviewed target."
            }]
        }),
    )
}

#[test]
fn public_execute_and_autopilot_skills_require_mission_close_review() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical repo root");

    let execute_skill = std::fs::read_to_string(repo_root.join(".codex/skills/execute/SKILL.md"))
        .expect("read execute skill");
    let autopilot_skill =
        std::fs::read_to_string(repo_root.join(".codex/skills/autopilot/SKILL.md"))
            .expect("read autopilot skill");
    let runtime_backend = std::fs::read_to_string(repo_root.join("docs/runtime-backend.md"))
        .expect("read runtime backend doc");
    let qualification_readme =
        std::fs::read_to_string(repo_root.join("docs/qualification/README.md"))
            .expect("read qualification readme");
    let qualification_gates =
        std::fs::read_to_string(repo_root.join("docs/qualification/gates.md"))
            .expect("read qualification gates doc");

    assert!(
        execute_skill.contains(
            "If the current frontier is clean and the remaining owed gate is\n   mission-close review, route into `$review-loop` for the mission-close bundle"
        ),
        "execute skill should explicitly route the clean final frontier into mission-close review"
    );
    assert!(
        autopilot_skill.contains(
            "If the frontier is clean and the remaining owed gate is mission-close review,\n  the next branch is `review-loop` for the mission-close bundle"
        ),
        "autopilot skill should explicitly route the clean final frontier into mission-close review"
    );
    assert!(
        runtime_backend.contains(
            "$execute` owns bounded target advancement under a passed package, including\n  honest routing into review, mission-close review, repair, replan, and durable\n  waiting branches."
        ),
        "runtime backend doc should reflect mission-close review as an execute/autopilot responsibility"
    );
    assert!(
        qualification_readme.contains(
            "The fully autonomous execute or autopilot promise is not proven by one gate."
        ),
        "qualification readme should state that public execute/autopilot proof is broader than one parity gate"
    );
    assert!(
        qualification_gates.contains(
            "Together they form the autonomy-governance proof surface for execute and autopilot."
        ),
        "qualification gates doc should frame parity as one component of the broader execute/autopilot proof surface"
    );
}

#[test]
fn autopilot_plan_seal_requires_autonomy_and_advisor_checkpoint() {
    let manual = evaluate_autopilot_plan_seal(&AutopilotPlanSealInput {
        caller: AutopilotPlanSealCaller::ManualPlan,
        autonomy_granted: true,
        effective_plan_level: 5,
        planning_rigor_satisfied: true,
        human_only_decisions_open: false,
        advisor_checkpoint: AdvisorCheckpointState::Satisfied,
        blueprint_fresh: true,
        package_fresh: true,
        post_seal_artifact_changes: Vec::new(),
    });
    assert!(!manual.self_seal_allowed);
    assert_eq!(manual.next_required_branch, NextRequiredBranch::NeedsUser);
    assert!(
        manual
            .blockers
            .contains(&"manual_plan_requires_user_seal".to_string())
    );

    let missing_autonomy = evaluate_autopilot_plan_seal(&AutopilotPlanSealInput {
        caller: AutopilotPlanSealCaller::Autopilot,
        autonomy_granted: false,
        effective_plan_level: 5,
        planning_rigor_satisfied: true,
        human_only_decisions_open: false,
        advisor_checkpoint: AdvisorCheckpointState::Satisfied,
        blueprint_fresh: true,
        package_fresh: true,
        post_seal_artifact_changes: Vec::new(),
    });
    assert!(!missing_autonomy.self_seal_allowed);
    assert_eq!(
        missing_autonomy.next_required_branch,
        NextRequiredBranch::NeedsUser
    );
    assert!(
        missing_autonomy
            .blockers
            .contains(&"autonomy_grant_missing".to_string())
    );

    let missing_advisor = evaluate_autopilot_plan_seal(&AutopilotPlanSealInput {
        caller: AutopilotPlanSealCaller::Autopilot,
        autonomy_granted: true,
        effective_plan_level: 5,
        planning_rigor_satisfied: true,
        human_only_decisions_open: false,
        advisor_checkpoint: AdvisorCheckpointState::Missing,
        blueprint_fresh: true,
        package_fresh: true,
        post_seal_artifact_changes: Vec::new(),
    });
    assert!(!missing_advisor.self_seal_allowed);
    assert_eq!(
        missing_advisor.next_required_branch,
        NextRequiredBranch::Replan
    );
    assert!(
        missing_advisor
            .blockers
            .contains(&"advisor_checkpoint_missing_for_level_5".to_string())
    );
}

#[test]
fn autopilot_plan_seal_blocks_stale_package_truth_and_allows_fresh_autopilot_path() {
    let stale = evaluate_autopilot_plan_seal(&AutopilotPlanSealInput {
        caller: AutopilotPlanSealCaller::Autopilot,
        autonomy_granted: true,
        effective_plan_level: 5,
        planning_rigor_satisfied: true,
        human_only_decisions_open: false,
        advisor_checkpoint: AdvisorCheckpointState::SkippedWithDisposition,
        blueprint_fresh: true,
        package_fresh: false,
        post_seal_artifact_changes: vec!["PLANS/demo/PROGRAM-BLUEPRINT.md".to_string()],
    });
    assert!(!stale.self_seal_allowed);
    assert_eq!(stale.next_required_branch, NextRequiredBranch::Replan);
    assert!(stale.blockers.contains(&"package_truth_stale".to_string()));
    assert!(
        stale
            .blockers
            .contains(&"post_seal_artifact_change_detected".to_string())
    );

    let fresh = evaluate_autopilot_plan_seal(&AutopilotPlanSealInput {
        caller: AutopilotPlanSealCaller::Autopilot,
        autonomy_granted: true,
        effective_plan_level: 5,
        planning_rigor_satisfied: true,
        human_only_decisions_open: false,
        advisor_checkpoint: AdvisorCheckpointState::Satisfied,
        blueprint_fresh: true,
        package_fresh: true,
        post_seal_artifact_changes: Vec::new(),
    });
    assert!(fresh.self_seal_allowed);
    assert_eq!(fresh.next_required_branch, NextRequiredBranch::Execution);
    assert!(fresh.blockers.is_empty());
}

#[test]
fn autopilot_plan_seal_cli_pairs_with_package_freshness_validation() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "autopilot-seal-freshness";

    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Autopilot Seal Freshness",
            "objective": "Ensure autopilot seal decisions respect package freshness.",
            "clarify_status": "ratified",
            "lock_status": "locked",
            "outcome_lock_body": "# Outcome Lock\n\n## Objective\n\nEnsure autopilot seal decisions respect package freshness.\n\n## Done-When Criteria\n\n- Autopilot seal checks are bound to durable mission truth.\n\n## Protected Surfaces\n\n- crates/codex1/src/internal/mod.rs\n\n## Unacceptable Tradeoffs\n\n- Do not trust caller-supplied freshness.\n\n## Autonomy Boundary\n\nCodex may update repo docs, runtime code, tests, and mission artifacts when authorized by an execution package.\n\nCodex must ask before destructive or irreversible actions.\n"
        }),
    );
    run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": autopilot_seal_blueprint_body("Use internal commands."),
            "plan_level": 5,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:seal_flow",
            "specs": [{
                "spec_id": "seal_flow",
                "purpose": "Exercise autopilot seal freshness.",
                "body_markdown": canonical_spec_body_with_scope_and_note(
                    "Exercise autopilot seal freshness.",
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

    let package = run_json(
        repo.path(),
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "seal_flow",
            "included_spec_ids": ["seal_flow"],
            "dependency_satisfaction_state": [{"name": "lock_current", "satisfied": true}],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["spec review"]
        }),
    );
    let package_id = package["package_id"].as_str().expect("package id");

    let fresh_package = run_json(
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
    assert_eq!(fresh_package["valid"], true);

    let fresh_decision = run_json(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "autopilot"
        }),
    );
    assert_eq!(fresh_decision["self_seal_allowed"], true);
    assert_eq!(fresh_decision["next_required_branch"], "execution");

    let manual_decision = run_json(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "manual_plan"
        }),
    );
    assert_eq!(manual_decision["self_seal_allowed"], false);
    assert_eq!(manual_decision["next_required_branch"], "needs_user");
    assert!(
        manual_decision["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker == "manual_plan_requires_user_seal")
    );

    let forged_caller_truth_error = run_json_failure(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "autopilot",
            "autonomy_granted": true
        }),
    );
    assert!(
        forged_caller_truth_error.contains("unknown field `autonomy_granted`"),
        "expected strict autopilot seal input contract; stderr: {forged_caller_truth_error}"
    );

    let missing_bindings_error = run_json_failure(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "caller": "autopilot"
        }),
    );
    assert!(
        missing_bindings_error.contains("missing field `mission_id`"),
        "expected mission/package bindings to be required; stderr: {missing_bindings_error}"
    );

    let spec_path = repo
        .path()
        .join("PLANS")
        .join(mission_id)
        .join("specs/seal_flow/SPEC.md");
    let mut changed_spec = std::fs::read_to_string(&spec_path).expect("read spec");
    changed_spec.push_str("\n## Post-Seal Edit\n\n- This edit should stale the package.\n");
    std::fs::write(&spec_path, changed_spec).expect("mutate spec after package");

    let stale_package = run_json(
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
    assert_eq!(stale_package["valid"], false);

    let stale_decision = run_json(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "autopilot"
        }),
    );
    assert_eq!(stale_decision["self_seal_allowed"], false);
    assert_eq!(stale_decision["next_required_branch"], "replan");
    assert!(
        stale_decision["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker == "package_truth_stale")
    );
    assert!(
        stale_decision["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker == "post_seal_artifact_change_detected")
    );
}

#[test]
fn autopilot_plan_seal_cli_rejects_marker_only_level_five_rigor() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "autopilot-marker-rigor";
    let package_id = setup_autopilot_plan_seal_cli_repo(
        repo.path(),
        mission_id,
        autopilot_marker_only_blueprint_body("Use marker-only planning evidence."),
    );

    let decision = run_json(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "autopilot"
        }),
    );
    assert_eq!(decision["self_seal_allowed"], false);
    assert_eq!(decision["next_required_branch"], "replan");
    assert!(
        decision["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker == "planning_rigor_evidence_missing")
    );
}

#[test]
fn autopilot_plan_seal_cli_rejects_required_methods_without_methods_run() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "autopilot-required-only-rigor";
    let package_id = setup_autopilot_plan_seal_cli_repo(
        repo.path(),
        mission_id,
        autopilot_required_only_blueprint_body("Use required-method markers only."),
    );

    let decision = run_json(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "autopilot"
        }),
    );
    assert_eq!(decision["self_seal_allowed"], false);
    assert_eq!(decision["next_required_branch"], "replan");
    assert!(
        decision["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker == "planning_rigor_evidence_missing")
    );
}

#[test]
fn autopilot_plan_seal_cli_rejects_advisor_without_plan_seal_checkpoint() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "autopilot-advisor-marker";
    let package_id = setup_autopilot_plan_seal_cli_repo(
        repo.path(),
        mission_id,
        autopilot_seal_blueprint_body_without_checkpoint("Use advisor marker without checkpoint."),
    );

    let decision = run_json(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "autopilot"
        }),
    );
    assert_eq!(decision["self_seal_allowed"], false);
    assert_eq!(decision["next_required_branch"], "replan");
    assert!(
        decision["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker == "advisor_checkpoint_missing_for_level_5")
    );
}

#[test]
fn autopilot_plan_seal_cli_rejects_generic_advisor_skip_without_checkpoint() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "autopilot-generic-advisor-skip";
    let package_id = setup_autopilot_plan_seal_cli_repo(
        repo.path(),
        mission_id,
        autopilot_generic_advisor_skip_blueprint_body("Use generic advisor skip marker."),
    );

    let decision = run_json(
        repo.path(),
        &["internal", "evaluate-autopilot-plan-seal"],
        json!({
            "mission_id": mission_id,
            "package_id": package_id,
            "caller": "autopilot"
        }),
    );
    assert_eq!(decision["self_seal_allowed"], false);
    assert_eq!(decision["next_required_branch"], "replan");
    assert!(
        decision["blockers"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|blocker| blocker == "advisor_checkpoint_missing_for_level_5")
    );
}

#[test]
fn loop_skill_surface_documents_lease_and_close_contract() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical repo root");
    let plan = std::fs::read_to_string(repo_root.join(".codex/skills/plan/SKILL.md"))
        .expect("read plan skill");
    let execute = std::fs::read_to_string(repo_root.join(".codex/skills/execute/SKILL.md"))
        .expect("read execute skill");
    let review_loop = std::fs::read_to_string(repo_root.join(".codex/skills/review-loop/SKILL.md"))
        .expect("read review-loop skill");
    assert!(
        !repo_root.join(".codex/skills/review/SKILL.md").exists(),
        "$review removal is intentional; $review-loop is the public parent review workflow"
    );
    let autopilot = std::fs::read_to_string(repo_root.join(".codex/skills/autopilot/SKILL.md"))
        .expect("read autopilot skill");
    let clarify = std::fs::read_to_string(repo_root.join(".codex/skills/clarify/SKILL.md"))
        .expect("read clarify skill");
    let orchestration =
        std::fs::read_to_string(repo_root.join(".codex/skills/internal-orchestration/SKILL.md"))
            .expect("read internal-orchestration skill");
    let close = std::fs::read_to_string(repo_root.join(".codex/skills/close/SKILL.md"))
        .expect("read close skill");
    let runtime_backend = std::fs::read_to_string(repo_root.join("docs/runtime-backend.md"))
        .expect("read runtime backend doc");

    assert!(plan.contains("mode = \"planning_loop\""));
    assert!(execute.contains("mode = \"execution_loop\""));
    assert!(review_loop.contains("mode = \"review_loop\""));
    assert!(autopilot.contains("mode = \"autopilot_loop\""));
    assert!(clarify.contains("Do not acquire a parent loop lease"));
    assert!(orchestration.contains("Child agents never acquire Ralph loop leases"));
    assert!(close.contains("codex1 internal pause-loop-lease"));
    assert!(close.contains("codex1 internal clear-loop-lease"));
    let public_skill_tokens: Vec<&str> = runtime_backend
        .lines()
        .take_while(|line| *line != "## Public Skill Responsibilities")
        .filter(|line| line.starts_with("- `$"))
        .filter_map(|line| line.split('`').nth(1))
        .map(|token| token.trim_start_matches('$'))
        .collect();
    assert!(public_skill_tokens.contains(&"close"));
    assert!(public_skill_tokens.contains(&"review-loop"));
    assert!(
        !public_skill_tokens.contains(&"review"),
        "legacy $review must not reappear as a public parent workflow"
    );
    assert!(
        runtime_backend.contains("`$close` owns the user escape hatch"),
        "runtime backend responsibilities should explain close/pause ownership"
    );
    assert!(runtime_backend.contains("`$close` pauses or clears the active lease"));
}

#[test]
fn findings_only_reviewer_stop_hook_bypasses_parent_review_gate_block() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-lane-stop-hook";
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Review Lane Stop Hook",
            "objective": "Prove child review lanes can return findings while parent review gate is open.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    run_json(
        repo.path(),
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": canonical_blueprint_body("Use one bounded runtime slice."),
            "plan_level": 5,
            "problem_size": "M",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:runtime_core",
            "specs": [{
                "spec_id": "runtime_core",
                "purpose": "Create a review gate for the parent.",
                "body_markdown": canonical_spec_body("Create a review gate for the parent."),
                "artifact_status": "active",
                "packetization_status": "runnable",
                "execution_status": "packaged"
            }]
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
    run_json(
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

    let parent_stop_without_lease = run_stop_hook_with_payload(
        repo.path(),
        json!({ "cwd": repo.path().display().to_string() }),
    );
    assert_eq!(parent_stop_without_lease["decision"], Value::Null);
    assert!(
        parent_stop_without_lease["systemMessage"]
            .as_str()
            .expect("parent no-lease message")
            .contains("Ralph loop is not active")
    );

    run_json(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "parent-review-loop",
            "reason": "Prove active parent leases still enforce review gates."
        }),
    );
    let parent_stop = run_stop_hook_with_payload(
        repo.path(),
        json!({ "cwd": repo.path().display().to_string() }),
    );
    assert_eq!(parent_stop["decision"], "block");
    assert!(
        parent_stop["reason"]
            .as_str()
            .expect("parent reason")
            .contains("review gate")
    );

    let reviewer_stop = run_stop_hook_with_payload(
        repo.path(),
        json!({
            "cwd": repo.path().display().to_string(),
            "laneRole": "findings_only_reviewer",
            "childLaneKind": "local_spec_intent"
        }),
    );
    assert_eq!(reviewer_stop["decision"], Value::Null);
    assert!(
        reviewer_stop["systemMessage"]
            .as_str()
            .expect("reviewer system message")
            .contains("Subagent lane may stop")
    );
}

#[test]
fn ralph_control_loop_boundary_scopes_stop_hook_to_active_parent_lease() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "ralph-control-loop-boundary-runtime";
    let (_package_id, _bundle_id) = setup_review_wave(&repo, mission_id);

    let no_lease_stop = run_stop_hook(repo.path());
    assert_eq!(no_lease_stop["decision"], Value::Null);
    assert!(
        no_lease_stop["systemMessage"]
            .as_str()
            .expect("no lease message")
            .contains("Ralph loop is not active")
    );

    let child_stop = run_stop_hook_with_payload(
        repo.path(),
        json!({
            "cwd": repo.path().display().to_string(),
            "laneRole": "child_helper",
            "agentName": "/root/explorer"
        }),
    );
    assert_eq!(child_stop["decision"], Value::Null);
    assert!(
        child_stop["systemMessage"]
            .as_str()
            .expect("child stop message")
            .contains("Subagent lane may stop")
    );

    let lease = run_json(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "parent-review-loop",
            "reason": "Review loop is intentionally active."
        }),
    );
    assert_eq!(lease["status"], "active");
    assert_eq!(lease["mode"], "review_loop");
    let loop_authority_token = lease["parent_authority_token"]
        .as_str()
        .expect("loop authority token");

    let active_parent_stop = run_stop_hook(repo.path());
    assert_eq!(active_parent_stop["decision"], "block");
    assert!(
        active_parent_stop["reason"]
            .as_str()
            .expect("active lease reason")
            .contains("review gate")
    );
    let validation_after_block = run_json(
        repo.path(),
        &[
            "internal",
            "validate-mission-artifacts",
            "--mission-id",
            mission_id,
        ],
        json!({}),
    );
    assert_eq!(
        validation_after_block["success"], true,
        "open-gate Stop-hook reporting must not poison cached mission state"
    );

    let pause_without_authority = run_json_failure(
        repo.path(),
        &["internal", "pause-loop-lease"],
        json!({
            "mission_id": mission_id,
            "paused_by": "user",
            "reason": "Missing parent authority should not pause an active verifier-backed lease."
        }),
    );
    assert!(
        pause_without_authority.contains("requires parent loop authority"),
        "unexpected error: {pause_without_authority}"
    );

    let paused = run_json_with_parent_authority(
        repo.path(),
        &["internal", "pause-loop-lease"],
        json!({
            "mission_id": mission_id,
            "paused_by": "user",
            "reason": "User paused Ralph to discuss."
        }),
        loop_authority_token,
    );
    assert_eq!(paused["lease"]["status"], "paused");
    let paused_stop = run_stop_hook(repo.path());
    assert_eq!(paused_stop["decision"], Value::Null);
    assert!(
        paused_stop["systemMessage"]
            .as_str()
            .expect("paused message")
            .contains("Ralph loop is not active")
    );

    let replace_paused_without_authority = run_json_failure(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "reviewer-child",
            "reason": "A child must not replace a paused verifier-backed parent lease."
        }),
    );
    assert!(
        replace_paused_without_authority.contains("already owns parent authority"),
        "unexpected error: {replace_paused_without_authority}"
    );

    let clear_without_authority =
        run_json_failure(repo.path(), &["internal", "clear-loop-lease"], json!({}));
    assert!(
        clear_without_authority.contains("requires parent loop authority"),
        "unexpected error: {clear_without_authority}"
    );

    let cleared = run_json_with_parent_authority(
        repo.path(),
        &["internal", "clear-loop-lease"],
        json!({}),
        loop_authority_token,
    );
    assert_eq!(cleared["lease"]["status"], "paused");
    let inspected = run_json(repo.path(), &["internal", "inspect-loop-lease"], json!({}));
    assert_eq!(inspected["lease"], Value::Null);
}

#[test]
fn manual_ratified_clarify_yields_for_explicit_plan_instead_of_blocking() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "manual-clarify-handoff";
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Manual Clarify Handoff",
            "objective": "Prove manual clarify waits for explicit plan invocation.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );

    let stop = run_stop_hook(repo.path());
    assert_eq!(stop["decision"], Value::Null);
    assert_eq!(stop["reason"], Value::Null);
    assert!(
        stop["systemMessage"]
            .as_str()
            .expect("manual clarify handoff message")
            .contains("Invoke $plan manually")
    );

    let state: Value = serde_json::from_slice(
        &std::fs::read(
            repo.path()
                .join(".ralph/missions/manual-clarify-handoff/state.json"),
        )
        .expect("read mission state"),
    )
    .expect("parse mission state");
    assert_eq!(state["verdict"], "needs_user");
    assert_eq!(state["resume_mode"], "yield_to_user");
    assert_eq!(state["waiting_for"], "manual_plan_invocation");
    assert_eq!(state["next_phase"], "planning");
}

#[test]
fn reviewer_lane_mutation_guard_rejects_contaminated_review_wave() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-lane-mutation-rejects";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_mutation_rejects",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_mutation_rejects",
    );

    std::fs::write(
        repo.path()
            .join("PLANS/reviewer-lane-mutation-rejects/REVIEW-LEDGER.md"),
        "# contaminated by child reviewer\n",
    )
    .expect("mutate review ledger");

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Child reviewer returned NONE."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("reviewer_lane_truth_mutation_detected"),
        "unexpected error: {error}"
    );

    let gates: Value = serde_json::from_slice(
        &std::fs::read(
            repo.path()
                .join(".ralph/missions/reviewer-lane-mutation-rejects/gates.json"),
        )
        .expect("read gates"),
    )
    .expect("parse gates");
    let review_gate = gates["gates"]
        .as_array()
        .expect("gates array")
        .iter()
        .find(|gate| gate["evaluated_against_ref"].as_str() == Some(bundle_id.as_str()))
        .expect("review gate");
    assert_eq!(review_gate["status"], "open");
}

#[test]
fn reviewer_lane_mutation_guard_accepts_clean_parent_writeback() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-lane-mutation-clean";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_mutation_clean",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_mutation_clean",
    );

    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Child reviewer returned NONE and mission truth stayed unchanged."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert_eq!(review["blocking_findings"], 0);

    let gates = run_json(
        repo.path(),
        &["internal", "validate-gates", "--mission-id", mission_id],
        json!({}),
    );
    assert_eq!(gates["valid"], true);
    let closeouts = run_json(
        repo.path(),
        &["internal", "validate-closeouts", "--mission-id", mission_id],
        json!({}),
    );
    assert_eq!(closeouts["valid"], true);
}

#[test]
fn reviewer_parent_writeback_guard_rejects_reviewer_lane_self_clear() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-parent-writeback-clean-reject";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "review_qual_gate_fast",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                format!("reviewer-output:review_qual_gate_fast:{bundle_id}"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Reviewer lane attempted to self-clear the gate."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("writeback is parent-owned"),
        "unexpected error: {error}"
    );

    let gates: Value = serde_json::from_slice(
        &std::fs::read(
            repo.path()
                .join(".ralph/missions/reviewer-parent-writeback-clean-reject/gates.json"),
        )
        .expect("read gates"),
    )
    .expect("parse gates");
    let review_gate = gates["gates"]
        .as_array()
        .expect("gates array")
        .iter()
        .find(|gate| gate["evaluated_against_ref"].as_str() == Some(bundle_id.as_str()))
        .expect("review gate");
    assert_eq!(review_gate["status"], "open");
}

#[test]
fn reviewer_parent_writeback_guard_rejects_self_referential_reviewer_output() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-parent-writeback-evidence-reject";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                format!("reviewer-output:review_qual_gate_fast:{bundle_id}"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Reviewer output evidence is not writeback authority."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("reviewer-output evidence")
            && error.contains("child reviewer self-writeback"),
        "unexpected error: {error}"
    );
}

#[test]
fn reviewer_parent_writeback_guard_rejects_reviewer_lane_blocking_writeback() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-parent-writeback-blocked-reject";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "/root/review_bug_correctness_1",
            "verdict": "blocked",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                format!("reviewer-output:review_bug_correctness_1:{bundle_id}"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [{
                "class": "B-Proof",
                "summary": "Reviewer lane attempted parent-owned writeback.",
                "blocking": true,
                "evidence_refs": [format!("reviewer-output:review_bug_correctness_1:{bundle_id}")],
                "disposition": "return bounded output instead"
            }],
            "disposition_notes": ["Reviewer lane attempted to write a blocking disposition."],
            "next_required_branch": "repair",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("writeback is parent-owned"),
        "unexpected error: {error}"
    );
}

#[test]
fn review_writeback_authority_token_is_required_for_parent_writeback() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-writeback-token-required";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_token_required",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_token_required",
    );

    assert!(
        snapshot["writeback_authority_token"].is_string(),
        "parent command output should include plaintext token: {snapshot}"
    );
    assert!(
        snapshot["writeback_authority_verifier"].is_string(),
        "parent command output should include persisted verifier: {snapshot}"
    );

    let persisted_path = repo.path().join(format!(
        ".ralph/missions/{mission_id}/review-truth-snapshots/{bundle_id}.json"
    ));
    let persisted: Value =
        serde_json::from_slice(&std::fs::read(&persisted_path).expect("read persisted snapshot"))
            .expect("parse persisted snapshot");
    assert!(
        persisted.get("writeback_authority_token").is_none(),
        "persisted repo-visible snapshot must not expose the plaintext token: {persisted}"
    );
    assert_eq!(
        persisted["writeback_authority_verifier"],
        snapshot["writeback_authority_verifier"]
    );

    let missing_token_error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Repo-visible snapshot should not be enough."],
            "next_required_branch": "execution",
            "review_truth_snapshot": persisted
        }),
    );
    assert!(
        missing_token_error.contains("writeback_authority_token is required"),
        "unexpected error: {missing_token_error}"
    );

    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Parent-held token authorizes writeback."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn review_writeback_authority_token_rejects_mismatch() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-writeback-token-mismatch";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let mut snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_token_mismatch",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_token_mismatch",
    );
    snapshot["writeback_authority_token"] = json!("wrong-token");

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Wrong token should not authorize writeback."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("writeback_authority_token does not match verifier"),
        "unexpected error: {error}"
    );
}

#[test]
fn review_writeback_authority_token_survives_child_evidence_snapshot_capture() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-writeback-token-evidence-lifecycle";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let parent_truth = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let child_evidence = run_json(
        repo.path(),
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );

    assert!(
        child_evidence.get("review_truth_snapshot").is_none(),
        "child evidence must not expose the parent writeback snapshot: {child_evidence}"
    );
    assert!(
        child_evidence
            .to_string()
            .find(
                parent_truth["writeback_authority_token"]
                    .as_str()
                    .expect("token")
            )
            .is_none(),
        "child evidence leaked the parent writeback token: {child_evidence}"
    );

    let persisted_path = repo.path().join(format!(
        ".ralph/missions/{mission_id}/review-truth-snapshots/{bundle_id}.json"
    ));
    let persisted: Value =
        serde_json::from_slice(&std::fs::read(&persisted_path).expect("read persisted snapshot"))
            .expect("parse persisted snapshot");
    assert_eq!(
        persisted["writeback_authority_verifier"],
        parent_truth["writeback_authority_verifier"]
    );
    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_token_lifecycle",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_token_lifecycle",
    );

    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Original parent-held token still authorizes writeback after child evidence capture."],
            "next_required_branch": "execution",
            "review_truth_snapshot": parent_truth
        }),
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn review_writeback_authority_cannot_be_reminted_for_same_bundle() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-writeback-single-capture";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_single_capture");

    let error = run_json_failure(
        repo.path(),
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    assert!(
        error.contains("refusing to remint parent writeback authority"),
        "unexpected error: {error}"
    );
}

#[test]
fn review_writeback_requires_parent_loop_authority_even_without_existing_lease() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-writeback-no-lease-authority";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    std::fs::remove_file(test_parent_authority_token_path(repo.path())).ok();
    std::fs::remove_file(repo.path().join(".ralph/loop-lease.json")).ok();

    let error = run_json_failure(
        repo.path(),
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    assert!(
        error.contains("requires parent loop authority"),
        "unexpected error: {error}"
    );
}

#[test]
fn begin_loop_lease_requires_parent_begin_authority_without_existing_lease() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-begin-no-parent-authority";
    setup_review_wave(&repo, mission_id);
    std::fs::remove_file(test_parent_authority_token_path(repo.path())).ok();
    std::fs::remove_file(repo.path().join(".ralph/loop-lease.json")).ok();

    let error = run_json_failure_without_parent_begin(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "reviewer-child",
            "reason": "Child should not be able to mint parent authority from no-lease state."
        }),
    );
    assert!(
        error.contains("requires parent begin authority"),
        "unexpected error: {error}"
    );
}

#[test]
fn review_writeback_rejects_truth_snapshot_captured_after_reviewer_output() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-writeback-ordering";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let mut snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_ordering_check");

    snapshot["captured_at"] = json!("2099-01-01T00:00:00Z");
    let mut persisted = snapshot.clone();
    persisted
        .as_object_mut()
        .expect("snapshot object")
        .remove("writeback_authority_token");
    let persisted_path = repo.path().join(format!(
        ".ralph/missions/{mission_id}/review-truth-snapshots/{bundle_id}.json"
    ));
    std::fs::write(
        persisted_path,
        serde_json::to_vec_pretty(&persisted).expect("encode persisted snapshot"),
    )
    .expect("write persisted snapshot");

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [output["evidence_ref"].as_str().expect("evidence ref"), "RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["Reviewer output must be downstream of parent truth capture."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("was recorded before parent review_truth_snapshot capture"),
        "unexpected error: {error}"
    );
}

#[test]
fn paused_parent_loop_blocks_review_outcome_writeback_until_resume() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "paused-review-writeback-blocked";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_paused",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_paused",
    );

    run_json(
        repo.path(),
        &["internal", "pause-loop-lease"],
        json!({
            "mission_id": mission_id,
            "paused_by": "user",
            "reason": "User invoked $close while reviewer outputs may still finish."
        }),
    );

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Paused parent loop must not integrate reviewer outputs."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("requires an active parent loop lease"),
        "unexpected error: {error}"
    );
}

#[test]
fn reviewer_outputs_can_land_while_close_paused_but_integrate_only_after_resume() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "close-paused-reviewer-output-drain";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let parent_token = test_parent_authority_token(repo.path()).expect("parent token");

    run_json(
        repo.path(),
        &["internal", "pause-loop-lease"],
        json!({
            "mission_id": mission_id,
            "paused_by": "user",
            "reason": "User invoked $close while bounded reviewer lanes may still drain."
        }),
    );

    let gates_before_outputs = std::fs::read(
        repo.path()
            .join(format!(".ralph/missions/{mission_id}/gates.json")),
    )
    .expect("read gates before paused outputs");
    let closeouts_before_outputs = std::fs::read(
        repo.path()
            .join(format!(".ralph/missions/{mission_id}/closeouts.ndjson")),
    )
    .expect("read closeouts before paused outputs");

    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_close_paused",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_close_paused",
    );

    assert_eq!(
        std::fs::read(
            repo.path()
                .join(format!(".ralph/missions/{mission_id}/gates.json"))
        )
        .expect("read gates after paused outputs"),
        gates_before_outputs,
        "bounded reviewer-output drain while paused must not mutate gate truth"
    );
    assert_eq!(
        std::fs::read(
            repo.path()
                .join(format!(".ralph/missions/{mission_id}/closeouts.ndjson"))
        )
        .expect("read closeouts after paused outputs"),
        closeouts_before_outputs,
        "bounded reviewer-output drain while paused must not append closeouts"
    );

    let paused_writeback = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref")
            ],
            "findings": [],
            "disposition_notes": ["Paused parent must not integrate drained reviewer outputs."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot.clone()
        }),
    );
    assert!(
        paused_writeback.contains("requires an active parent loop lease"),
        "unexpected error: {paused_writeback}"
    );

    let resumed = run_json_with_parent_authority(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "parent-review-loop",
            "reason": "User resumed after $close; integrate only after freshness validation."
        }),
        &parent_token,
    );
    let resumed_token = resumed["parent_authority_token"]
        .as_str()
        .expect("resumed parent token");

    let review = run_json_with_parent_authority(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref")
            ],
            "findings": [],
            "disposition_notes": ["After resume, freshness-validated reviewer outputs may be integrated."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
        resumed_token,
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn parent_loop_authority_blocks_child_mission_truth_mutations() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-parent-loop-authority";
    let (package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let lease = run_json(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "parent-review-loop",
            "reason": "Protect parent-owned review writeback."
        }),
    );
    let authority_token = lease["parent_authority_token"]
        .as_str()
        .expect("parent authority token");
    assert!(
        lease["parent_authority_verifier"].is_string(),
        "lease should include a persisted verifier: {lease}"
    );

    let inspect = run_json(repo.path(), &["internal", "inspect-loop-lease"], json!({}));
    assert!(
        inspect["lease"].get("parent_authority_token").is_none(),
        "persisted lease must not expose the parent authority token: {inspect}"
    );

    let clear_without_authority =
        run_json_failure(repo.path(), &["internal", "clear-loop-lease"], json!({}));
    assert!(
        clear_without_authority.contains("requires parent loop authority"),
        "unexpected error: {clear_without_authority}"
    );
    let pause_without_authority = run_json_failure(
        repo.path(),
        &["internal", "pause-loop-lease"],
        json!({
            "mission_id": mission_id,
            "paused_by": "reviewer-child",
            "reason": "Child should not be able to pause the parent lease."
        }),
    );
    assert!(
        pause_without_authority.contains("requires parent loop authority"),
        "unexpected error: {pause_without_authority}"
    );
    let begin_without_authority = run_json_failure(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "reviewer-child",
            "reason": "Child should not be able to replace the parent lease."
        }),
    );
    assert!(
        begin_without_authority.contains("already owns parent authority"),
        "unexpected error: {begin_without_authority}"
    );

    run_json_with_parent_authority(
        repo.path(),
        &["internal", "pause-loop-lease"],
        json!({
            "mission_id": mission_id,
            "paused_by": "parent-review-loop",
            "reason": "User invoked $close; bounded child outputs may finish but parent integration is paused."
        }),
        authority_token,
    );
    let begin_while_paused_without_authority = run_json_failure(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "reviewer-child",
            "reason": "Child should not be able to replace a paused parent lease."
        }),
    );
    assert!(
        begin_while_paused_without_authority.contains("already owns parent authority"),
        "unexpected error: {begin_while_paused_without_authority}"
    );
    let compile_while_paused = run_json_failure(
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
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["runtime contract"]
        }),
    );
    assert!(
        compile_while_paused.contains("requires an active parent loop lease"),
        "unexpected error: {compile_while_paused}"
    );

    let resumed_lease = run_json_with_parent_authority(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "owner": "parent-review-loop",
            "reason": "Resume parent review loop after close/pause."
        }),
        authority_token,
    );
    let authority_token = resumed_lease["parent_authority_token"]
        .as_str()
        .expect("resumed parent authority token");

    let capture_without_authority = run_json_failure(
        repo.path(),
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    assert!(
        capture_without_authority.contains("requires parent loop authority"),
        "unexpected error: {capture_without_authority}"
    );

    let bundle_compile_without_authority = run_json_failure(
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
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["runtime contract"]
        }),
    );
    assert!(
        bundle_compile_without_authority.contains("requires parent loop authority"),
        "unexpected error: {bundle_compile_without_authority}"
    );

    let snapshot = run_json_with_parent_authority(
        repo.path(),
        &[
            "internal",
            "capture-review-truth-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
        authority_token,
    );
    run_json_with_parent_authority(
        repo.path(),
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
        authority_token,
    );
    let code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_authority_guard",
    );
    let spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_authority_guard",
    );

    let record_without_authority = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Missing parent loop authority should fail."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot.clone()
        }),
    );
    assert!(
        record_without_authority.contains("requires parent loop authority"),
        "unexpected error: {record_without_authority}"
    );

    let review = run_json_with_parent_authority(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Parent loop authority plus parent truth snapshot authorizes writeback."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
        authority_token,
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn single_generic_reviewer_output_cannot_satisfy_all_required_lanes() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-lane-generic-single";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let generic_output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_runtime_flow");

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [generic_output["evidence_ref"].as_str().expect("generic evidence ref")],
            "findings": [],
            "disposition_notes": ["A single generic legacy reviewer output must not satisfy every required lane."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("missing required reviewer-output lane coverage"),
        "unexpected error: {error}"
    );
}

#[test]
fn multiple_generic_reviewer_outputs_cannot_satisfy_distinct_required_lanes() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-lane-generic-multiple";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let generic_a =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_runtime_flow");
    let generic_b = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_runtime_flow_followup",
    );

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                generic_a["evidence_ref"].as_str().expect("generic evidence ref"),
                generic_b["evidence_ref"].as_str().expect("generic followup evidence ref")
            ],
            "findings": [],
            "disposition_notes": ["Generic reviewer outputs must not impersonate profile coverage."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("missing required reviewer-output lane coverage"),
        "unexpected error: {error}"
    );
}

#[test]
fn compile_review_bundle_rejects_duplicate_open_wave_for_same_boundary() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-wave-duplicate-open";
    let (package_id, first_bundle_id) = setup_review_wave(&repo, mission_id);

    let error = run_json_failure(
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
            "changed_files_or_diff": ["src/lib.rs"],
            "touched_interface_contracts": ["runtime contract"]
        }),
    );

    assert!(
        error.contains("active review wave already open"),
        "unexpected error: {error}"
    );
    assert!(
        error.contains(&first_bundle_id),
        "duplicate-wave error should point at the active bundle/gate: {error}"
    );
}

#[test]
fn reviewer_output_profile_metadata_satisfies_required_lane_coverage() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-output-profile-metadata";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);

    let code_output = run_json(
        repo.path(),
        &["internal", "record-reviewer-output"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer_id": "lane_a",
            "reviewer_profile": "code_bug_correctness",
            "output_kind": "none",
            "findings": []
        }),
    );
    let spec_output = run_json(
        repo.path(),
        &["internal", "record-reviewer-output"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer_id": "lane_b",
            "reviewer_profile": "local_spec_intent",
            "output_kind": "none",
            "findings": []
        }),
    );

    for (output, expected_profile) in [
        (&code_output, "code_bug_correctness"),
        (&spec_output, "local_spec_intent"),
    ] {
        let artifact: Value = serde_json::from_slice(
            &std::fs::read(output["path"].as_str().expect("output path"))
                .expect("read reviewer output artifact"),
        )
        .expect("parse reviewer output artifact");
        assert_eq!(artifact["reviewer_profile"], expected_profile);
    }

    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref")
            ],
            "findings": [],
            "disposition_notes": ["Explicit reviewer profile metadata satisfies required lane coverage."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn malformed_reviewer_output_profile_cannot_clear_clean_review() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-output-profile-malformed";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let code_output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_code_bug_1");
    let spec_output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_spec_intent_1");

    let code_path = code_output["path"].as_str().expect("code output path");
    let mut artifact: Value =
        serde_json::from_slice(&std::fs::read(code_path).expect("read code output"))
            .expect("parse code output");
    artifact["reviewer_profile"] = json!("not_a_real_profile");
    std::fs::write(
        code_path,
        serde_json::to_vec_pretty(&artifact).expect("encode tampered output"),
    )
    .expect("write tampered output");

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                code_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_output["evidence_ref"].as_str().expect("spec evidence ref")
            ],
            "findings": [],
            "disposition_notes": ["Malformed reviewer-output metadata must not clear review."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("reviewer output artifact profile is not a known review lane profile"),
        "unexpected error: {error}"
    );
}

#[test]
fn begin_loop_lease_rejects_non_verifier_upgrade_without_parent_begin_authority() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "legacy-lease-upgrade";
    run_json(
        repo.path(),
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Legacy Lease Upgrade",
            "objective": "Prove non-verifier leases cannot mint parent authority.",
            "clarify_status": "ratified",
            "lock_status": "locked"
        }),
    );
    std::fs::write(
        repo.path().join(".ralph/loop-lease.json"),
        serde_json::to_vec_pretty(&json!({
            "mission_id": mission_id,
            "mode": "review_loop",
            "status": "active",
            "owner": "legacy-parent",
            "reason": "Legacy lease without verifier.",
            "acquired_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }))
        .expect("encode legacy lease"),
    )
    .expect("write legacy lease");

    let binary = assert_cmd::cargo::cargo_bin("codex1");
    let output = Command::new(binary)
        .args(["internal", "begin-loop-lease"])
        .arg("--repo-root")
        .arg(repo.path())
        .arg("--json")
        .write_stdin(
            serde_json::to_vec(&json!({
                "mission_id": mission_id,
                "mode": "review_loop",
                "owner": "reviewer-child",
                "reason": "Child tries to upgrade legacy lease."
            }))
            .expect("encode input"),
        )
        .output()
        .expect("run begin-loop-lease");
    assert!(
        !output.status.success(),
        "legacy lease upgrade unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot upgrade non-verifier lease"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn reviewer_output_inbox_contract_records_bounded_output_without_advancing_truth() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-output-inbox-bounded";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let gates_before = std::fs::read(
        repo.path()
            .join(format!(".ralph/missions/{mission_id}/gates.json")),
    )
    .expect("read gates before");
    let closeouts_before = std::fs::read(
        repo.path()
            .join(format!(".ralph/missions/{mission_id}/closeouts.ndjson")),
    )
    .expect("read closeouts before");

    let output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_spec_intent_1");

    assert_eq!(output["mission_id"], mission_id);
    assert_eq!(output["bundle_id"], bundle_id);
    assert!(
        output["evidence_ref"]
            .as_str()
            .expect("evidence ref")
            .starts_with(&format!("reviewer-output:{bundle_id}:"))
    );
    let output_path = output["path"].as_str().expect("output path");
    let artifact: Value =
        serde_json::from_slice(&std::fs::read(output_path).expect("read reviewer output artifact"))
            .expect("parse reviewer output artifact");
    assert_eq!(artifact["output_kind"], "none");
    assert_eq!(artifact["findings"].as_array().expect("findings").len(), 0);
    assert_eq!(
        std::fs::read(
            repo.path()
                .join(format!(".ralph/missions/{mission_id}/gates.json"))
        )
        .expect("read gates after"),
        gates_before,
        "reviewer-output inbox write must not mutate gates"
    );
    assert_eq!(
        std::fs::read(
            repo.path()
                .join(format!(".ralph/missions/{mission_id}/closeouts.ndjson"))
        )
        .expect("read closeouts after"),
        closeouts_before,
        "reviewer-output inbox write must not append closeouts"
    );
}

#[test]
fn reviewer_output_inbox_contract_requires_real_inbox_artifact_for_parent_writeback() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-output-inbox-writeback";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);

    let fake_ref_error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [format!("reviewer-output:{bundle_id}:not-a-real-output")],
            "findings": [],
            "disposition_notes": ["Fake reviewer-output refs must not clear gates."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        fake_ref_error.contains("invalid reviewer-output evidence ref"),
        "unexpected error: {fake_ref_error}"
    );

    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let spec_output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_spec_intent_1");
    let code_output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_code_bug_1");
    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                code_output["evidence_ref"].as_str().expect("code evidence ref")
            ],
            "findings": [],
            "disposition_notes": ["Real reviewer-output inbox artifact authorizes delegated evidence."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn clean_code_review_writeback_requires_code_and_spec_reviewer_outputs() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "review-lane-completion-guard";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let spec_output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_spec_intent_1");

    let missing_code_error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [spec_output["evidence_ref"].as_str().expect("spec evidence ref")],
            "findings": [],
            "disposition_notes": ["Spec-only reviewer output must not clear code correctness review."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        missing_code_error
            .contains("missing required reviewer-output lane coverage: code_bug_correctness"),
        "unexpected error: {missing_code_error}"
    );

    let code_output =
        record_none_reviewer_output(repo.path(), mission_id, &bundle_id, "review_code_bug_1");
    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                code_output["evidence_ref"].as_str().expect("code evidence ref")
            ],
            "findings": [],
            "disposition_notes": ["Both required reviewer lanes are present and clean."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn reviewer_output_inbox_contract_rejects_clean_writeback_with_blocking_reviewer_finding() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-output-inbox-blocking";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let output =
        record_blocking_reviewer_output(repo.path(), mission_id, &bundle_id, "review_code_bug_1");

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [output["evidence_ref"].as_str().expect("evidence ref")],
            "findings": [],
            "disposition_notes": ["A clean parent outcome cannot override blocking reviewer output."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains(
            "cannot be clean while cited reviewer-output artifact contains P0/P1/P2 findings"
        ),
        "unexpected error: {error}"
    );
}

#[test]
fn delegated_review_authority_rejects_parent_only_clean_outcome() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "delegated-review-authority-clean";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["Parent-local review should not be accepted as review evidence."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("requires reviewer-agent output evidence"),
        "unexpected error: {error}"
    );
}

#[test]
fn delegated_review_authority_rejects_parent_only_blocking_finding() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "delegated-review-authority-blocking";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "blocked",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["RECEIPTS/test-output.txt"],
            "findings": [{
                "class": "B-Spec",
                "summary": "Parent-local blocking review judgment is not enough.",
                "blocking": true,
                "evidence_refs": ["RECEIPTS/test-output.txt"],
                "disposition": "repair"
            }],
            "disposition_notes": ["A blocking finding must cite reviewer-agent output evidence."],
            "next_required_branch": "repair",
            "review_truth_snapshot": snapshot
        }),
    );
    assert!(
        error.contains("requires reviewer-agent output evidence"),
        "unexpected error: {error}"
    );
}

#[test]
fn delegated_review_authority_allows_contaminated_replan_route_without_clearing_clean() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "delegated-review-authority-contaminated";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "blocked",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["review-wave-contaminated:missing-reviewer-output"],
            "findings": [{
                "class": "B-Proof",
                "summary": "Review wave cannot prove delegated reviewer judgment.",
                "blocking": true,
                "evidence_refs": ["review-wave-contaminated:missing-reviewer-output"],
                "disposition": "replan"
            }],
            "disposition_notes": ["The wave is invalid and must route away instead of clearing review."],
            "next_required_branch": "replan",
            "review_truth_snapshot": snapshot
        }),
    );
    assert_eq!(review["blocking_findings"], 1);
}

#[test]
fn delegated_review_authority_contract_is_documented_on_public_surfaces() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical repo root");
    let review_loop = std::fs::read_to_string(repo_root.join(".codex/skills/review-loop/SKILL.md"))
        .expect("read review-loop skill");
    let internal_orchestration =
        std::fs::read_to_string(repo_root.join(".codex/skills/internal-orchestration/SKILL.md"))
            .expect("read internal-orchestration skill");
    let runtime_backend = std::fs::read_to_string(repo_root.join("docs/runtime-backend.md"))
        .expect("read runtime backend doc");
    let multi_agent_guide = std::fs::read_to_string(repo_root.join("docs/MULTI-AGENT-V2-GUIDE.md"))
        .expect("read multi-agent guide");

    for (name, contents) in [
        ("review-loop", review_loop.as_str()),
        ("internal-orchestration", internal_orchestration.as_str()),
        ("runtime-backend", runtime_backend.as_str()),
        ("multi-agent-guide", multi_agent_guide.as_str()),
    ] {
        assert!(
            contents.contains("reviewer-agent output"),
            "{name} should require reviewer-agent output evidence"
        );
        assert!(
            contents.contains("parent") && contents.contains("judgment"),
            "{name} should distinguish parent orchestration from review judgment"
        );
    }

    assert!(
        review_loop.contains("must not substitute its own code, spec, intent,\nintegration, or mission-close judgment"),
        "review-loop must explicitly forbid parent self-review without a small-slice loophole"
    );
}

#[test]
fn reviewer_lane_mutation_guard_requires_parent_truth_snapshot() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-lane-mutation-requires-snapshot";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "would-be-child-reviewer",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": ["reviewer-output:no-snapshot-lane", "RECEIPTS/test-output.txt"],
            "findings": [],
            "disposition_notes": ["No snapshot should be rejected."],
            "next_required_branch": "execution"
        }),
    );
    assert!(
        error.contains("requires review_truth_snapshot"),
        "unexpected error: {error}"
    );
}

#[test]
fn reviewer_evidence_snapshot_contract_captures_and_validates_bounded_brief() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-evidence-snapshot";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let snapshot = run_json(
        repo.path(),
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    assert_eq!(snapshot["bundle_id"], bundle_id);
    assert_eq!(snapshot["source_package_id"].is_string(), true);
    assert!(
        snapshot["reviewer_instructions"]
            .as_array()
            .expect("instructions")
            .iter()
            .any(|instruction| instruction.as_str().expect("instruction").contains("NONE"))
    );
    assert!(
        snapshot["evidence_refs"]
            .as_array()
            .expect("evidence refs")
            .iter()
            .any(|reference| reference
                .as_str()
                .expect("reference")
                .contains("RECEIPTS/test-output.txt"))
    );
    assert!(
        snapshot.get("review_truth_snapshot").is_none(),
        "child-visible review evidence snapshot must not include the parent writeback snapshot"
    );
    assert_eq!(snapshot["review_truth_guard"]["bundle_id"], bundle_id);
    assert!(
        snapshot["review_truth_guard"]["guard_fingerprint"]
            .as_str()
            .expect("guard fingerprint")
            .starts_with("sha256:")
    );

    let validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    assert_eq!(validation["valid"], true);

    let snapshot_path = repo.path().join(format!(
        ".ralph/missions/{mission_id}/review-evidence-snapshots/{bundle_id}.json"
    ));
    let mut tampered: Value =
        serde_json::from_slice(&std::fs::read(&snapshot_path).expect("read snapshot"))
            .expect("parse snapshot");
    tampered["proof_rows_under_review"] = json!([]);
    std::fs::write(
        &snapshot_path,
        serde_json::to_vec_pretty(&tampered).expect("encode tampered snapshot"),
    )
    .expect("write tampered snapshot");

    let tampered_validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    assert_eq!(tampered_validation["valid"], false);
    assert!(
        tampered_validation["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| finding == "proof_rows_under_review_missing")
    );
    assert!(
        tampered_validation["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| finding == "proof_rows_under_review_mismatch")
    );
}

#[test]
fn reviewer_lane_canonical_write_isolation_redacts_parent_truth_snapshot_from_child_evidence() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-lane-canonical-write-isolation";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    let snapshot = run_json(
        repo.path(),
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );

    assert!(
        snapshot.get("review_truth_snapshot").is_none(),
        "review evidence snapshot leaked the parent writeback capability: {snapshot}"
    );
    assert_eq!(snapshot["review_truth_guard"]["bundle_id"], bundle_id);
    assert!(
        snapshot["review_truth_guard"]["artifact_fingerprint_count"]
            .as_u64()
            .expect("artifact count")
            > 0
    );

    let canonical_truth_path = repo.path().join(format!(
        ".ralph/missions/{mission_id}/review-truth-snapshots/{bundle_id}.json"
    ));
    assert!(
        canonical_truth_path.is_file(),
        "parent-held canonical truth snapshot should still exist"
    );
}

#[test]
fn reviewer_evidence_snapshot_rejects_tampered_review_truth_guard_binding() {
    let repo = TempDir::new().expect("temp repo");
    let mission_id = "reviewer-evidence-truth-binding";
    let (_package_id, bundle_id) = setup_review_wave(&repo, mission_id);
    let parent_truth = capture_review_truth_snapshot(repo.path(), mission_id, &bundle_id);

    run_json(
        repo.path(),
        &[
            "internal",
            "capture-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    let snapshot_path = repo.path().join(format!(
        ".ralph/missions/{mission_id}/review-evidence-snapshots/{bundle_id}.json"
    ));
    let mut tampered: Value =
        serde_json::from_slice(&std::fs::read(&snapshot_path).expect("read snapshot"))
            .expect("parse snapshot");
    tampered["review_truth_guard"]["artifact_fingerprint_count"] = json!(0);
    std::fs::write(
        &snapshot_path,
        serde_json::to_vec_pretty(&tampered).expect("encode tampered snapshot"),
    )
    .expect("write tampered snapshot");

    let validation = run_json(
        repo.path(),
        &[
            "internal",
            "validate-review-evidence-snapshot",
            "--mission-id",
            mission_id,
            "--bundle-id",
            &bundle_id,
        ],
        json!({}),
    );
    assert_eq!(validation["valid"], false);
    assert!(
        validation["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .filter_map(|finding| finding.as_str())
            .any(|finding| finding.starts_with("review_truth_guard_binding_invalid:")),
        "unexpected findings: {validation}"
    );

    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let persisted_truth_code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_persisted_parent_truth",
    );
    let persisted_truth_spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_persisted_parent_truth",
    );

    let parent_truth_path = repo.path().join(format!(
        ".ralph/missions/{mission_id}/review-truth-snapshots/{bundle_id}.json"
    ));
    let persisted_parent_truth: Value =
        serde_json::from_slice(&std::fs::read(&parent_truth_path).expect("read parent truth"))
            .expect("parse parent truth");
    let missing_token_error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                persisted_truth_code_output["evidence_ref"].as_str().expect("code evidence ref"),
                persisted_truth_spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Persisted parent truth is a verifier, not writeback authority."],
            "next_required_branch": "execution",
            "review_truth_snapshot": persisted_parent_truth
        }),
    );
    assert!(
        missing_token_error.contains("writeback_authority_token is required"),
        "unexpected error: {missing_token_error}"
    );

    let error = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                persisted_truth_code_output["evidence_ref"].as_str().expect("code evidence ref"),
                persisted_truth_spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Tampered snapshot should not be accepted."],
            "next_required_branch": "execution",
            "review_truth_snapshot": tampered["review_truth_guard"].clone()
        }),
    );
    assert!(
        error.contains("missing field") || error.contains("artifact_fingerprints"),
        "unexpected error: {error}"
    );

    capture_review_evidence_snapshot(repo.path(), mission_id, &bundle_id);
    let parent_truth_code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_code_bug_parent_held_truth",
    );
    let parent_truth_spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        &bundle_id,
        "review_spec_intent_parent_held_truth",
    );
    let review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": bundle_id,
            "reviewer": "parent-review-loop",
            "verdict": "clean",
            "target_spec_id": "runtime_core",
            "governing_refs": ["bundle"],
            "evidence_refs": [
                parent_truth_code_output["evidence_ref"].as_str().expect("code evidence ref"),
                parent_truth_spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Parent-held truth snapshot remains the writeback capability."],
            "next_required_branch": "execution",
            "review_truth_snapshot": parent_truth
        }),
    );
    assert_eq!(review["blocking_findings"], 0);
}

#[test]
fn reviewer_lane_canonical_write_isolation_stop_hook_detects_review_task_path() {
    let repo = TempDir::new().expect("temp repo");
    let stop = run_stop_hook_with_payload(
        repo.path(),
        json!({
            "cwd": repo.path().display().to_string(),
            "taskPath": "/root/review_authority_code_1"
        }),
    );
    assert_eq!(stop["decision"], Value::Null);
    assert!(
        stop["systemMessage"]
            .as_str()
            .expect("reviewer system message")
            .contains("Subagent lane may stop")
    );
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
    assert!(stdout.contains("capture-review-evidence-snapshot"));
    assert!(stdout.contains("validate-review-evidence-snapshot"));
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

fn autopilot_seal_blueprint_body(route: &str) -> String {
    format!(
        "{}\n## Planning Rigor Record\n\n```yaml\nplanning_rigor:\n  user_requested_rigor: max\n  mission_risk_floor: max\n  effective_rigor: max\n  required_methods:\n    - truth_register\n    - system_map\n    - boundary_coupling_map\n    - invariant_register\n    - proof_matrix\n    - decision_obligations\n    - adversarial_critique\n    - advisor_checkpoint_design\n    - review_design\n    - execution_graph\n    - package_next_target\n  methods_run:\n    - truth_register\n    - system_map\n    - boundary_coupling_map\n    - invariant_register\n    - proof_matrix\n    - decision_obligations\n    - adversarial_critique\n    - advisor_checkpoint_design\n    - review_design\n    - execution_graph\n    - package_next_target\n  advisors_used:\n    - checkpoint: high_risk_plan_seal\n      role: advisor_v1_critic\n      disposition: followed\n```\n",
        canonical_blueprint_body(route)
    )
}

fn autopilot_seal_blueprint_body_without_checkpoint(route: &str) -> String {
    autopilot_seal_blueprint_body(route).replace("checkpoint: high_risk_plan_seal\n      ", "")
}

fn autopilot_marker_only_blueprint_body(route: &str) -> String {
    format!(
        "{}\n## Planning Rigor Record\n\n```yaml\nplanning_rigor:\n  user_requested_rigor: max\n  mission_risk_floor: max\n  effective_rigor: max\n  methods_run:\n    - proof_matrix\n    - review_design\n  advisors_used:\n    - checkpoint: high_risk_plan_seal\n      role: advisor_v1_critic\n      disposition: followed\n```\n",
        canonical_blueprint_body(route)
    )
}

fn autopilot_required_only_blueprint_body(route: &str) -> String {
    format!(
        "{}\n## Planning Rigor Record\n\n```yaml\nplanning_rigor:\n  user_requested_rigor: max\n  mission_risk_floor: max\n  effective_rigor: max\n  required_methods:\n    - truth_register\n    - system_map\n    - boundary_coupling_map\n    - invariant_register\n    - proof_matrix\n    - decision_obligations\n    - adversarial_critique\n    - advisor_checkpoint_design\n    - review_design\n    - execution_graph\n    - package_next_target\n  methods_run:\n    - proof_matrix\n    - review_design\n  advisors_used:\n    - checkpoint: high_risk_plan_seal\n      role: advisor_v1_critic\n      disposition: followed\n```\n",
        canonical_blueprint_body(route)
    )
}

fn autopilot_generic_advisor_skip_blueprint_body(route: &str) -> String {
    format!(
        "{}\n## Planning Rigor Record\n\n```yaml\nplanning_rigor:\n  user_requested_rigor: max\n  mission_risk_floor: max\n  effective_rigor: max\n  required_methods:\n    - truth_register\n    - system_map\n    - boundary_coupling_map\n    - invariant_register\n    - proof_matrix\n    - decision_obligations\n    - adversarial_critique\n    - advisor_checkpoint_design\n    - review_design\n    - execution_graph\n    - package_next_target\n  methods_run:\n    - truth_register\n    - system_map\n    - boundary_coupling_map\n    - invariant_register\n    - proof_matrix\n    - decision_obligations\n    - adversarial_critique\n    - advisor_checkpoint_design\n    - review_design\n    - execution_graph\n    - package_next_target\n  advisor_checkpoint_skip:\n    checkpoint: unrelated_checkpoint\n    disposition: skipped_with_disposition\n```\n",
        canonical_blueprint_body(route)
    )
}

fn setup_autopilot_plan_seal_cli_repo(
    repo_root: &Path,
    mission_id: &str,
    blueprint_body: String,
) -> String {
    run_json(
        repo_root,
        &["internal", "init-mission"],
        json!({
            "mission_id": mission_id,
            "title": "Autopilot Seal Freshness",
            "objective": "Ensure autopilot seal decisions respect package freshness.",
            "clarify_status": "ratified",
            "lock_status": "locked",
            "outcome_lock_body": "# Outcome Lock\n\n## Objective\n\nEnsure autopilot seal decisions respect package freshness.\n\n## Done-When Criteria\n\n- Autopilot seal checks are bound to durable mission truth.\n\n## Protected Surfaces\n\n- crates/codex1/src/internal/mod.rs\n\n## Unacceptable Tradeoffs\n\n- Do not trust caller-supplied freshness.\n\n## Autonomy Boundary\n\nCodex may update repo docs, runtime code, tests, and mission artifacts when authorized by an execution package.\n\nCodex must ask before destructive or irreversible actions.\n"
        }),
    );
    run_json(
        repo_root,
        &["internal", "materialize-plan"],
        json!({
            "mission_id": mission_id,
            "body_markdown": blueprint_body,
            "plan_level": 5,
            "problem_size": "S",
            "status": "approved",
            "proof_matrix": [{"claim_ref": "claim:default-proof", "statement": "The selected route has explicit proof coverage.", "required_evidence": ["RECEIPTS/test-output.txt"], "review_lenses": ["correctness"], "governing_contract_refs": ["blueprint"]}],
            "decision_obligations": [],
            "selected_target_ref": "spec:seal_flow",
            "specs": [{
                "spec_id": "seal_flow",
                "purpose": "Exercise autopilot seal freshness.",
                "body_markdown": canonical_spec_body_with_scope_and_note(
                    "Exercise autopilot seal freshness.",
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

    let package = run_json(
        repo_root,
        &["internal", "compile-execution-package"],
        json!({
            "mission_id": mission_id,
            "target_type": "spec",
            "target_id": "seal_flow",
            "included_spec_ids": ["seal_flow"],
            "dependency_satisfaction_state": [{"name": "lock_current", "satisfied": true}],
            "read_scope": ["src"],
            "write_scope": ["src"],
            "proof_obligations": ["cargo test"],
            "review_obligations": ["spec review"]
        }),
    );
    package["package_id"]
        .as_str()
        .expect("package id")
        .to_string()
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

    let review_snapshot = capture_review_truth_snapshot(repo.path(), mission_id, bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, bundle_id);
    let code_reviewer_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        bundle_id,
        "review_code_bug_runtime_flow",
    );
    let spec_reviewer_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        bundle_id,
        "review_spec_intent_runtime_flow",
    );
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
            "evidence_refs": [
                code_reviewer_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_reviewer_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Review bundle is fresh and clean."],
            "next_required_branch": "execution",
            "review_truth_snapshot": review_snapshot
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
    let post_completion_snapshot =
        capture_review_truth_snapshot(repo.path(), mission_id, post_completion_bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, post_completion_bundle_id);
    let post_completion_code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        post_completion_bundle_id,
        "review_code_bug_clean_before_ledger_removal",
    );
    let post_completion_spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        post_completion_bundle_id,
        "review_spec_intent_clean_before_ledger_removal",
    );
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
            "evidence_refs": [
                post_completion_code_output["evidence_ref"].as_str().expect("code evidence ref"),
                post_completion_spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Post-completion review bundle is fresh and clean."],
            "next_required_branch": "mission_close",
            "review_truth_snapshot": post_completion_snapshot
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
    let mission_close_snapshot =
        capture_review_truth_snapshot(repo.path(), mission_id, mission_close_bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, mission_close_bundle_id);
    let mission_close_generic_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        mission_close_bundle_id,
        "review_mission_close_generic",
    );
    let missing_mission_close_lanes = run_json_failure(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": mission_close_bundle_id,
            "reviewer": "integration-test",
            "verdict": "complete",
            "governing_refs": ["mission-close-bundle"],
            "evidence_refs": [
                mission_close_generic_output["evidence_ref"].as_str().expect("generic evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Mission-close review cannot close with only a generic reviewer output."],
            "review_truth_snapshot": mission_close_snapshot.clone()
        }),
    );
    assert!(
        missing_mission_close_lanes.contains("missing required reviewer-output lane coverage"),
        "unexpected error: {missing_mission_close_lanes}"
    );
    let mission_close_code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        mission_close_bundle_id,
        "review_code_bug_mission_close",
    );
    let mission_close_spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        mission_close_bundle_id,
        "review_spec_intent_mission_close",
    );

    let mission_close_review = run_json(
        repo.path(),
        &["internal", "record-review-outcome"],
        json!({
            "mission_id": mission_id,
            "bundle_id": mission_close_bundle_id,
            "reviewer": "integration-test",
            "verdict": "complete",
            "governing_refs": ["mission-close-bundle"],
            "evidence_refs": [
                mission_close_code_output["evidence_ref"].as_str().expect("code evidence ref"),
                mission_close_spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Mission-close review is clean."],
            "review_truth_snapshot": mission_close_snapshot
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
    let snapshot = capture_review_truth_snapshot(repo.path(), mission_id, bundle_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, bundle_id);
    let code_reviewer_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        bundle_id,
        "review_code_bug_ledger_removal",
    );
    let spec_reviewer_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        bundle_id,
        "review_spec_intent_ledger_removal",
    );
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
            "evidence_refs": [
                code_reviewer_output["evidence_ref"].as_str().expect("code evidence ref"),
                spec_reviewer_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Review bundle is fresh and clean."],
            "next_required_branch": "execution",
            "review_truth_snapshot": snapshot
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
    assert_eq!(stop["decision"], Value::Null);
    assert!(
        stop["systemMessage"]
            .as_str()
            .is_some_and(|message| message.contains("Ralph loop is not active"))
    );

    run_json(
        repo.path(),
        &["internal", "begin-loop-lease"],
        json!({
            "mission_id": "blocked-mission",
            "mode": "execution_loop",
            "owner": "parent-execute",
            "reason": "Explicit execution loop should still block on hard-block review requirements."
        }),
    );
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
    let stale_snapshot = capture_review_truth_snapshot(repo.path(), mission_id, bundle_one_id);
    capture_review_evidence_snapshot(repo.path(), mission_id, bundle_one_id);
    let stale_code_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        bundle_one_id,
        "review_code_bug_stale_bundle",
    );
    let stale_spec_output = record_none_reviewer_output(
        repo.path(),
        mission_id,
        bundle_one_id,
        "review_spec_intent_stale_bundle",
    );

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
            "evidence_refs": [
                stale_code_output["evidence_ref"].as_str().expect("code evidence ref"),
                stale_spec_output["evidence_ref"].as_str().expect("spec evidence ref"),
                "RECEIPTS/test-output.txt"
            ],
            "findings": [],
            "disposition_notes": ["Should fail because the bundle is stale."],
            "next_required_branch": "execution",
            "review_truth_snapshot": stale_snapshot
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
