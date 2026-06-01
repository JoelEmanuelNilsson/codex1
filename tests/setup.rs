mod common;

use std::fs;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;

use common::*;

#[test]
fn setup_install_materializes_repo_scoped_guidance() {
    let repo = repo();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    assert_eq!(value["ok"], true);
    for skill in MANAGED_SKILLS {
        assert!(repo.path().join(skill).is_file(), "{skill}");
    }
    for doc in MANAGED_SUPPORTING_DOCS {
        assert!(repo.path().join(doc).is_file(), "{doc}");
    }
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".codex1/setup-bundle.json").is_file());

    let guidance = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert!(guidance.contains("codex1-managed setup guidance start"));
    let clarify = fs::read_to_string(repo.path().join(".agents/skills/clarify/SKILL.md")).unwrap();
    assert!(clarify.contains("clarify observable success outcomes and boundaries"));
    assert!(clarify.contains("Before considering clarification complete"));
    assert!(clarify.contains("assume the final finished product"));
    assert!(clarify.contains("Always Preserve"));
    let prd_skill =
        fs::read_to_string(repo.path().join(".agents/skills/create-prd/SKILL.md")).unwrap();
    assert!(prd_skill.contains("per-story acceptance-criteria engine"));
    assert!(prd_skill.contains("final finished-product contract"));
    assert!(prd_skill.contains("## Boundaries"));
    assert!(prd_skill.contains("do not introduce fallback paths, legacy compatibility"));
    let prd_format =
        fs::read_to_string(repo.path().join(".agents/skills/create-prd/PRD-FORMAT.md")).unwrap();
    assert!(prd_format.contains("A long numbered list of behavior-focused user stories"));
    assert!(prd_format.contains("final finished-product contract"));
    assert!(prd_format.contains("do not introduce fallback paths, legacy compatibility"));
    assert!(prd_format.contains("### Always Preserve"));
    assert!(prd_format.contains("### Ask Before Changing"));
    let plan = fs::read_to_string(repo.path().join(".agents/skills/plan/SKILL.md")).unwrap();
    assert!(plan.contains("subplans are implementation slices, not product stages"));
    assert!(plan.contains("## Suitability Gate"));
    assert!(plan.contains("## Artifact Minimalism"));
    assert!(plan.contains("Do not use `$plan` when the user asks to diagnose"));
    assert!(plan.contains("Run a lightweight execution-readiness audit"));
    assert!(plan.contains("next-action policy between continuations"));
    let goal_brief_format =
        fs::read_to_string(repo.path().join(".agents/skills/plan/GOAL-BRIEF-FORMAT.md")).unwrap();
    assert!(goal_brief_format.contains("GOAL_PROMPT.md"));
    assert!(goal_brief_format.contains("Do not force the whole brief under a character limit"));
    assert!(goal_brief_format.contains("## Goal Contract Pattern"));
    assert!(goal_brief_format.contains("Stop and ask rules"));
    assert!(goal_brief_format.contains("notes.md"));
    assert!(goal_brief_format.contains("proven"));
    assert!(goal_brief_format.contains("safe during goal"));
    let artifact_briefs =
        fs::read_to_string(repo.path().join("docs/agents/codex1-artifact-briefs.md")).unwrap();
    assert!(artifact_briefs.contains("assume the final finished product"));
    assert!(artifact_briefs.contains("Artifact Minimalism"));
    assert!(artifact_briefs.contains("desired end state, verified by specific evidence"));
    assert!(!repo.path().join(".codex/config.toml").exists());
}

#[test]
fn setup_without_subcommand_materializes_repo_scoped_guidance() {
    let repo = repo();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .arg("setup"),
    );

    assert_eq!(value["ok"], true);
    assert!(repo.path().join(".agents/skills/codex1/SKILL.md").is_file());
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".codex1/setup-bundle.json").is_file());
}

#[test]
fn setup_status_reports_bundle_state_only() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "status"]),
    );

    assert_eq!(value["ok"], true);
    let status = &value["data"]["status"];
    assert_eq!(status["repo_bundle_materialized"], true);
    assert_eq!(status["marker"], "current");
    assert_eq!(status["skill"], "current");
    assert_eq!(status["supporting_doc"], "current");
    assert_eq!(status["guidance"], "current");
    assert!(!value.to_string().contains("native_goal_state"));
}

#[test]
fn setup_install_dry_run_does_not_materialize_files() {
    let repo = repo();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install", "--dry-run"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["plan"]["dry_run"], true);
    for skill in MANAGED_SKILLS {
        assert!(!repo.path().join(skill).exists(), "{skill}");
    }
    for doc in MANAGED_SUPPORTING_DOCS {
        assert!(!repo.path().join(doc).exists(), "{doc}");
    }
    assert!(!repo.path().join("AGENTS.md").exists());
    assert!(!repo.path().join(".codex1/setup-bundle.json").exists());
    assert!(!repo
        .path()
        .join(".codex1/setup-backups/manifest.json")
        .exists());
}

#[test]
fn setup_disable_and_enable_preserve_user_guidance_and_missions() {
    let repo = repo();
    fs::write(
        repo.path().join("AGENTS.md"),
        "# Local Rules\n\nKeep this.\n",
    )
    .unwrap();
    init(&repo, "alpha");
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "disable"]),
    );

    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("Keep this."));
    assert!(!agents.contains("codex1-managed setup guidance start"));
    for skill in MANAGED_SKILLS {
        assert!(!repo.path().join(skill).exists(), "{skill}");
    }
    assert!(repo.path().join(".codex1/missions/alpha").is_dir());

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "enable"]),
    );
    let restored = fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert!(restored.contains("Keep this."));
    assert!(restored.contains("codex1-managed setup guidance start"));
}

#[test]
fn setup_uninstall_without_marker_preserves_unmanaged_repo_files() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# User skill\n",
    )
    .unwrap();
    fs::write(repo.path().join("AGENTS.md"), "# Local Rules\n").unwrap();
    init(&repo, "alpha");

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "uninstall"]),
    );

    assert_eq!(value["ok"], true);
    assert_eq!(
        fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap(),
        "# User skill\n"
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("AGENTS.md")).unwrap(),
        "# Local Rules\n"
    );
    assert!(repo.path().join(".codex1/missions/alpha").is_dir());
}

#[test]
fn setup_enable_repairs_stale_managed_skill_and_marker() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let marker_path = repo.path().join(".codex1/setup-bundle.json");
    let marker = fs::read_to_string(&marker_path).unwrap();
    fs::write(
        &marker_path,
        marker.replace(r#""version": 13"#, r#""version": 12"#),
    )
    .unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# Stale managed skill\n",
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "enable"]),
    );

    let skill = fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap();
    let marker = fs::read_to_string(repo.path().join(".codex1/setup-bundle.json")).unwrap();
    assert!(skill.contains("$clarify"));
    assert!(repo.path().join(".agents/skills/plan/SKILL.md").is_file());
    assert!(marker.contains(r#""version": 13"#));
}

#[test]
fn setup_enable_upgrades_pre_handoff_bundle() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    fs::remove_dir_all(repo.path().join(".agents/skills/handoff")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# Old managed overview\n",
    )
    .unwrap();

    let marker_path = repo.path().join(".codex1/setup-bundle.json");
    let mut marker: Value =
        serde_json::from_str(&fs::read_to_string(&marker_path).unwrap()).unwrap();
    marker["version"] = serde_json::json!(11);
    let files = marker["files"].as_array_mut().unwrap();
    files.retain(|file| {
        !matches!(
            file.as_str(),
            Some(".agents/skills/handoff/SKILL.md")
                | Some(".agents/skills/handoff/agents/openai.yaml")
        )
    });
    fs::write(
        &marker_path,
        serde_json::to_string_pretty(&marker).unwrap() + "\n",
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "enable"]),
    );

    let status = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "status"]),
    );
    assert_eq!(status["data"]["status"]["repo_bundle_materialized"], true);
    assert!(repo
        .path()
        .join(".agents/skills/handoff/SKILL.md")
        .is_file());
    assert!(repo
        .path()
        .join(".agents/skills/handoff/agents/openai.yaml")
        .is_file());
    let overview = fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap();
    assert!(overview.contains("$handoff"));
}

#[test]
fn setup_uninstall_accepts_v1_managed_marker() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": [".agents/skills/codex1/SKILL.md", "AGENTS.md"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "uninstall"]),
    );

    assert!(!repo.path().join(".agents/skills/codex1/SKILL.md").exists());
    assert!(!repo.path().join(".codex1/setup-bundle.json").exists());
}

#[test]
fn setup_uninstall_refuses_modified_marker_owned_skill() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let skill = repo.path().join(".agents/skills/codex1/SKILL.md");
    fs::write(&skill, "# User edited skill\n").unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "uninstall"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

    assert_eq!(fs::read_to_string(skill).unwrap(), "# User edited skill\n");
}

#[test]
fn setup_install_removes_retired_managed_bundle_file_before_rewriting_marker() {
    let repo = repo();
    let retired = repo
        .path()
        .join(".agents/skills/plan/EXECUTION-PROMPT-FORMAT.md");
    fs::create_dir_all(retired.parent().unwrap()).unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(&retired, LEGACY_EXECUTION_PROMPT_FORMAT_BODY).unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 4,
            "files": [
                ".agents/skills/codex1/SKILL.md",
                ".agents/skills/clarify/SKILL.md",
                ".agents/skills/clarify/ADR-FORMAT.md",
                ".agents/skills/clarify/CONTEXT-FORMAT.md",
                ".agents/skills/create-prd/SKILL.md",
                ".agents/skills/create-prd/PRD-FORMAT.md",
                ".agents/skills/plan/SKILL.md",
                ".agents/skills/plan/ADR-FORMAT.md",
                ".agents/skills/plan/SUBPLAN-BRIEF.md",
                ".agents/skills/plan/EXECUTION-PROMPT-FORMAT.md",
                "docs/agents/codex1-workflow.md",
                "docs/agents/codex1-domain.md",
                "docs/agents/codex1-artifact-briefs.md",
                "AGENTS.md"
            ]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );

    assert!(!retired.exists());
}

#[test]
fn setup_uninstall_refuses_marker_with_unmanaged_docs_path() {
    let repo = repo();
    let private_doc = repo.path().join("docs/agents/private.md");
    fs::create_dir_all(private_doc.parent().unwrap()).unwrap();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(&private_doc, "# User doc\n").unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": ["docs/agents/private.md"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "uninstall"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

    assert_eq!(fs::read_to_string(private_doc).unwrap(), "# User doc\n");
}

#[test]
fn setup_install_refuses_marker_with_unmanaged_paths() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": ["src/lib.rs"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "install"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));
}

#[test]
fn setup_install_refuses_partial_managed_marker() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex1")).unwrap();
    fs::create_dir_all(repo.path().join(".agents/skills/codex1")).unwrap();
    fs::write(
        repo.path().join(".agents/skills/codex1/SKILL.md"),
        "# User skill\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".codex1/setup-bundle.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 1,
            "files": [".agents/skills/codex1/SKILL.md"]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "install"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

    assert_eq!(
        fs::read_to_string(repo.path().join(".agents/skills/codex1/SKILL.md")).unwrap(),
        "# User skill\n"
    );
}

#[test]
fn setup_install_refuses_unmanaged_managed_files_without_marker() {
    for relative in [MANAGED_SKILLS[0], MANAGED_SUPPORTING_DOCS[0]] {
        let repo = repo();
        let path = repo.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# User file\n").unwrap();

        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"])
            .assert()
            .failure()
            .stdout(predicate::str::contains("SETUP_BUNDLE_ERROR"));

        assert_eq!(fs::read_to_string(path).unwrap(), "# User file\n");
    }
}

#[test]
fn setup_backups_restore_previous_absence() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    let backups = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "list"]),
    );
    let id = backups["data"]["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| {
            record["target_path_label"]
                .as_str()
                .unwrap()
                .ends_with("AGENTS.md")
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "backups", "restore", &id, "--force"]),
    );

    assert!(!repo.path().join("AGENTS.md").exists());
}

#[test]
fn setup_backups_restore_previous_marker_absence_from_prior_bundle() {
    let repo = repo();
    let repo_root = fs::canonicalize(repo.path()).unwrap();
    fs::create_dir_all(repo_root.join(".codex1/setup-backups")).unwrap();
    let marker = repo_root.join(".codex1/setup-bundle.json");
    fs::write(
        &marker,
        serde_json::to_string_pretty(&serde_json::json!({
            "managed_by": "codex1-managed",
            "version": 7,
            "files": [
                ".agents/skills/codex1/SKILL.md",
                ".agents/skills/codex1/agents/openai.yaml",
                ".agents/skills/clarify/SKILL.md",
                ".agents/skills/clarify/agents/openai.yaml",
                ".agents/skills/clarify/ADR-FORMAT.md",
                ".agents/skills/clarify/CONTEXT-FORMAT.md",
                ".agents/skills/create-prd/SKILL.md",
                ".agents/skills/create-prd/agents/openai.yaml",
                ".agents/skills/create-prd/PRD-FORMAT.md",
                ".agents/skills/plan/SKILL.md",
                ".agents/skills/plan/agents/openai.yaml",
                ".agents/skills/plan/ADR-FORMAT.md",
                ".agents/skills/plan/SUBPLAN-BRIEF.md",
                ".agents/skills/plan/GOAL-BRIEF-FORMAT.md",
                ".agents/skills/tdd/SKILL.md",
                ".agents/skills/tdd/agents/openai.yaml",
                ".agents/skills/tdd/tests.md",
                ".agents/skills/tdd/mocking.md",
                ".agents/skills/tdd/deep-modules.md",
                ".agents/skills/tdd/interface-design.md",
                ".agents/skills/tdd/refactoring.md",
                ".agents/skills/diagnose/SKILL.md",
                ".agents/skills/diagnose/agents/openai.yaml",
                ".agents/skills/diagnose/scripts/hitl-loop.template.sh",
                ".agents/skills/improve-codebase-architecture/SKILL.md",
                ".agents/skills/improve-codebase-architecture/agents/openai.yaml",
                ".agents/skills/improve-codebase-architecture/LANGUAGE.md",
                ".agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md",
                ".agents/skills/improve-codebase-architecture/DEEPENING.md",
                ".agents/skills/prototype/SKILL.md",
                ".agents/skills/prototype/agents/openai.yaml",
                ".agents/skills/prototype/LOGIC.md",
                ".agents/skills/prototype/UI.md",
                ".agents/skills/codex-review/SKILL.md",
                ".agents/skills/codex-review/agents/openai.yaml",
                ".agents/skills/codex-review/scripts/codex-review",
                ".agents/skills/brutal-review/SKILL.md",
                ".agents/skills/brutal-review/agents/openai.yaml",
                ".agents/skills/handoff/SKILL.md",
                ".agents/skills/handoff/agents/openai.yaml",
                "docs/agents/codex1-workflow.md",
                "docs/agents/codex1-domain.md",
                "docs/agents/codex1-artifact-briefs.md",
                "AGENTS.md"
            ]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    fs::write(
        repo_root.join(".codex1/setup-backups/manifest.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "records": [{
                "id": "old-marker-absence",
                "timestamp": "2026-05-02T00:00:00Z",
                "target_kind": "repo-setup",
                "target_path": marker,
                "target_path_label": ".codex1/setup-bundle.json",
                "backup_path": null,
                "existed": false,
                "reason": "bundle marker"
            }]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    json_output(bin().args(["--json", "--repo-root"]).arg(&repo_root).args([
        "setup",
        "backups",
        "restore",
        "old-marker-absence",
        "--force",
    ]));

    assert!(!marker.exists());
}

#[test]
fn setup_backups_restore_rejects_non_setup_targets() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".codex1/setup-backups/files/tampered")).unwrap();
    fs::write(
        repo.path()
            .join(".codex1/setup-backups/files/tampered/PRD.md"),
        "# Backup\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".codex1/setup-backups/manifest.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "version": 1,
            "records": [{
                "id": "tampered",
                "timestamp": "2026-05-02T00:00:00Z",
                "target_kind": "repo-setup",
                "target_path": repo.path().join(".codex1/missions/alpha/PRD.md"),
                "target_path_label": "PRD.md",
                "backup_path": repo.path().join(".codex1/setup-backups/files/tampered/PRD.md"),
                "existed": true,
                "reason": "tampered"
            }]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    bin()
        .args(["--json", "--repo-root"])
        .arg(repo.path())
        .args(["setup", "backups", "restore", "tampered", "--force"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("SETUP_RESTORE_ERROR"));

    assert!(!repo.path().join(".codex1/missions/alpha/PRD.md").exists());
}

#[test]
fn setup_doctor_reports_malformed_backup_manifest() {
    let repo = repo();
    json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "install"]),
    );
    fs::write(
        repo.path().join(".codex1/setup-backups/manifest.json"),
        "not json\n",
    )
    .unwrap();

    let value = json_output(
        bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(["setup", "doctor"]),
    );

    assert_eq!(value["ok"], true);
    let backup_manifest = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "backup_manifest")
        .unwrap();
    assert_eq!(backup_manifest["ok"], false);
}

#[test]
fn unknown_setup_options_fail_through_argument_parser() {
    let repo = repo();

    for args in [
        vec!["setup", "nope"],
        vec!["setup", "install", "--bad-flag"],
    ] {
        let output = bin()
            .args(["--json", "--repo-root"])
            .arg(repo.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "ARGUMENT_ERROR");
    }
}
