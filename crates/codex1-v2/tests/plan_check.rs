//! Wave 1 acceptance: `codex1-v2 plan check` catches DAG problems.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn bin(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("codex1").expect("binary built");
    cmd.arg("--repo-root").arg(dir.path());
    cmd
}

fn init(dir: &TempDir) {
    bin(dir)
        .args(["--json", "init", "--mission", "m1", "--title", "Test"])
        .assert()
        .success();
    // Round 13 P2: plan check now refuses a draft lock. Tests that
    // exercise DAG-level plan acceptance must first ratify the lock,
    // the same hand-edit $clarify does in the skill.
    ratify_lock(dir);
}

fn ratify_lock(dir: &TempDir) {
    let path = dir.path().join("PLANS/m1/OUTCOME-LOCK.md");
    let content = fs::read_to_string(&path).unwrap();
    let ratified = content.replace("lock_status: draft", "lock_status: ratified");
    assert_ne!(
        content, ratified,
        "fixture expected draft lock to flip to ratified"
    );
    fs::write(&path, ratified).unwrap();
}

fn write_blueprint(dir: &Path, yaml_body: &str) {
    let path = dir.join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    let content = format!(
        "# Program Blueprint\n\n<!-- codex1:plan-dag:start -->\n{yaml_body}\n<!-- codex1:plan-dag:end -->\n"
    );
    fs::write(&path, content).unwrap();
}

fn last_json(out: &[u8]) -> Value {
    let s = std::str::from_utf8(out).unwrap();
    serde_json::from_str(s.lines().last().unwrap()).unwrap()
}

#[test]
fn empty_tasks_rejected_by_plan_check() {
    // V2 PRD: a plan without a DAG is not executable. `plan check` must
    // fail so orchestrators never see a green light on an unshippable plan.
    // (`status` still routes empty-DAG missions to `plan_dag_empty` for UX.)
    let dir = TempDir::new().unwrap();
    init(&dir);
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["ok"], false);
    assert_eq!(env["code"], "DAG_EMPTY");
    assert_eq!(env["details"]["mission"], "m1");
}

#[test]
fn single_task_dag_passes() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    // Round 6 Fix #2: every task must declare spec_ref, write_paths,
    // proof, and review_profiles — the four fields V2 treats as the
    // executability/proof/review contract.
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Scaffold\n    kind: code\n    depends_on: []\n\
         \x20   spec_ref: specs/T1/SPEC.md\n\
         \x20   write_paths: [src/**]\n\
         \x20   proof: [\"cargo build\"]\n\
         \x20   review_profiles: [code_bug_correctness, local_spec_intent]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["task_count"], 1);
    assert_eq!(env["task_ids"], serde_json::json!(["T1"]));
}

#[test]
fn underspecified_task_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    // Include depends_on: [] so the four content-field check (spec_ref,
    // write_paths, proof, review_profiles) is what fires. The
    // depends_on case has its own test below.
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Thin\n    kind: code\n    depends_on: []\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_TASK_UNDERSPECIFIED");
    assert_eq!(env["details"]["task_id"], "T1");
    let missing = env["details"]["missing"].as_array().unwrap();
    for field in ["spec_ref", "write_paths", "proof", "review_profiles"] {
        assert!(
            missing.iter().any(|v| v == field),
            "missing should include {field}: {missing:?}"
        );
    }
}

#[test]
fn missing_depends_on_rejected() {
    // Round 10 P2: every task must declare `depends_on` explicitly
    // so the dependency graph is deliberate and wave execution is
    // auditable. Omitting the key (rather than setting it to `[]`)
    // used to pass plan check silently.
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n\
         \x20   spec_ref: specs/T1/SPEC.md\n\
         \x20   write_paths: [src/**]\n\
         \x20   proof: [\"cargo build\"]\n\
         \x20   review_profiles: [code_bug_correctness, local_spec_intent]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_TASK_UNDERSPECIFIED");
    assert_eq!(env["details"]["task_id"], "T1");
    assert_eq!(env["details"]["missing"], serde_json::json!(["depends_on"]));
}

#[test]
fn code_task_without_bug_correctness_rejected() {
    // Round 8 Fix #3: `kind: code` tasks must include `code_bug_correctness`
    // in review_profiles so a code slice cannot become review-clean on
    // spec-intent alone. `review open`'s superset rule then keeps the
    // minimum from being dropped at open-time.
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n    depends_on: []\n\
         \x20   spec_ref: specs/T1/SPEC.md\n\
         \x20   write_paths: [src/**]\n\
         \x20   proof: [\"cargo build\"]\n\
         \x20   review_profiles: [local_spec_intent]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_KIND_REVIEW_PROFILE_MISSING");
    assert_eq!(env["details"]["task_id"], "T1");
    assert_eq!(env["details"]["kind"], "code");
    assert_eq!(
        env["details"]["missing"],
        serde_json::json!(["code_bug_correctness"])
    );
}

#[test]
fn code_task_without_local_spec_intent_rejected() {
    // Round 12 P1: `kind: code` needs *both* `code_bug_correctness`
    // and `local_spec_intent`. Bug-correctness alone lets correct-in-
    // isolation code drift from the slice's declared intent.
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n    depends_on: []\n\
         \x20   spec_ref: specs/T1/SPEC.md\n\
         \x20   write_paths: [src/**]\n\
         \x20   proof: [\"cargo build\"]\n\
         \x20   review_profiles: [code_bug_correctness]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_KIND_REVIEW_PROFILE_MISSING");
    assert_eq!(env["details"]["task_id"], "T1");
    assert_eq!(env["details"]["kind"], "code");
    assert_eq!(
        env["details"]["missing"],
        serde_json::json!(["local_spec_intent"])
    );
}

#[test]
fn code_task_with_bug_correctness_plus_extras_passes() {
    // Adding profiles beyond the required minimum is fine — the check
    // is a subset, not an equality.
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n    depends_on: []\n\
         \x20   spec_ref: specs/T1/SPEC.md\n\
         \x20   write_paths: [src/**]\n\
         \x20   proof: [\"cargo build\"]\n\
         \x20   review_profiles: [code_bug_correctness, local_spec_intent]\n",
    );
    bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .success();
}

#[test]
fn draft_outcome_lock_refuses_plan_check() {
    // Round 13 P2: `plan check` is the user-facing acceptance gate
    // for route truth. It must not certify a plan while
    // OUTCOME-LOCK.md is still `lock_status: draft`; $clarify has to
    // ratify destination truth first.
    let dir = TempDir::new().unwrap();
    // Skip the init() helper — init() ratifies the lock, but we want
    // a draft here.
    bin(&dir)
        .args(["--json", "init", "--mission", "m1", "--title", "Test"])
        .assert()
        .success();
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n    depends_on: []\n\
         \x20   spec_ref: specs/T1/SPEC.md\n\
         \x20   write_paths: [src/**]\n\
         \x20   proof: [\"cargo build\"]\n\
         \x20   review_profiles: [code_bug_correctness, local_spec_intent]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "PLAN_CHECK_LOCK_DRAFT");
    assert_eq!(env["details"]["mission"], "m1");

    // Once ratified, plan check accepts.
    ratify_lock(&dir);
    bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .success();
}

#[test]
fn review_boundaries_rejected_until_implemented() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: Impl\n    kind: code\n    depends_on: []\n\
         \x20   spec_ref: specs/T1/SPEC.md\n\
         \x20   write_paths: [src/**]\n\
         \x20   proof: [\"cargo build\"]\n\
         \x20   review_profiles: [code_bug_correctness]\n\
         review_boundaries:\n  - id: B1\n    kind: phase\n    tasks: [T1]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_BOUNDARIES_NOT_SUPPORTED");
}

#[test]
fn duplicate_id_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n\
         \x20 - id: T1\n    title: B\n    kind: code\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_DUPLICATE_ID");
}

#[test]
fn cycle_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    depends_on: [T2]\n\
         \x20 - id: T2\n    title: B\n    kind: code\n    depends_on: [T1]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_CYCLE");
}

#[test]
fn invalid_id_format_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    for bad in ["t1", "TASK1", "T-1"] {
        write_blueprint(
            dir.path(),
            &format!(
                "planning:\n  requested_level: light\n  graph_revision: 1\n\
                 tasks:\n  - id: {bad}\n    title: X\n    kind: code\n"
            ),
        );
        let out = bin(&dir)
            .args(["--json", "plan", "check", "--mission", "m1"])
            .assert()
            .failure()
            .get_output()
            .stdout
            .clone();
        let env = last_json(&out);
        assert_eq!(env["code"], "DAG_BAD_ID", "{bad} should be rejected");
    }
}

#[test]
fn missing_dep_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    depends_on: [T99]\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_MISSING_DEP");
    assert_eq!(env["details"]["missing"], "T99");
}

#[test]
fn missing_markers_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    let path = dir.path().join("PLANS/m1/PROGRAM-BLUEPRINT.md");
    fs::write(&path, "# just prose\n").unwrap();
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_NO_BLOCK");
}

#[test]
fn unknown_task_field_rejected() {
    let dir = TempDir::new().unwrap();
    init(&dir);
    write_blueprint(
        dir.path(),
        "planning:\n  requested_level: light\n  graph_revision: 1\n\
         tasks:\n  - id: T1\n    title: A\n    kind: code\n    wat: unexpected\n",
    );
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "DAG_BAD_SCHEMA");
}

#[test]
fn mission_not_found_when_no_init() {
    let dir = TempDir::new().unwrap();
    let out = bin(&dir)
        .args(["--json", "plan", "check", "--mission", "m1"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let env = last_json(&out);
    assert_eq!(env["code"], "MISSION_NOT_FOUND");
}
