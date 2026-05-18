use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::cli::{
    SetupBackupRestoreArgs, SetupBackupsCommand, SetupCommand, SetupRepoArgs, SetupStatusArgs,
};
use crate::envelope;
use crate::error::{Codex1Error, Result};
use crate::paths::{create_dir_all_contained, discover_repo_root, ensure_contained_for_write};

const BACKUP_MANIFEST_VERSION: u32 = 1;
const BUNDLE_VERSION: u32 = 6;
const MANAGED_GUIDANCE_START: &str = "<!-- codex1-managed setup guidance start -->";
const MANAGED_GUIDANCE_END: &str = "<!-- codex1-managed setup guidance end -->";
const OVERVIEW_SKILL: &str = ".agents/skills/codex1/SKILL.md";
const OVERVIEW_OPENAI_YAML: &str = ".agents/skills/codex1/agents/openai.yaml";
const CLARIFY_SKILL: &str = ".agents/skills/clarify/SKILL.md";
const CLARIFY_OPENAI_YAML: &str = ".agents/skills/clarify/agents/openai.yaml";
const CLARIFY_ADR_FORMAT: &str = ".agents/skills/clarify/ADR-FORMAT.md";
const CLARIFY_CONTEXT_FORMAT: &str = ".agents/skills/clarify/CONTEXT-FORMAT.md";
const CREATE_PRD_SKILL: &str = ".agents/skills/create-prd/SKILL.md";
const CREATE_PRD_OPENAI_YAML: &str = ".agents/skills/create-prd/agents/openai.yaml";
const CREATE_PRD_FORMAT: &str = ".agents/skills/create-prd/PRD-FORMAT.md";
const PLAN_SKILL: &str = ".agents/skills/plan/SKILL.md";
const PLAN_OPENAI_YAML: &str = ".agents/skills/plan/agents/openai.yaml";
const PLAN_ADR_FORMAT: &str = ".agents/skills/plan/ADR-FORMAT.md";
const PLAN_SUBPLAN_BRIEF: &str = ".agents/skills/plan/SUBPLAN-BRIEF.md";
const PLAN_GOAL_BRIEF_FORMAT: &str = ".agents/skills/plan/GOAL-BRIEF-FORMAT.md";
const TDD_SKILL: &str = ".agents/skills/tdd/SKILL.md";
const TDD_OPENAI_YAML: &str = ".agents/skills/tdd/agents/openai.yaml";
const TDD_TESTS: &str = ".agents/skills/tdd/tests.md";
const TDD_MOCKING: &str = ".agents/skills/tdd/mocking.md";
const TDD_DEEP_MODULES: &str = ".agents/skills/tdd/deep-modules.md";
const TDD_INTERFACE_DESIGN: &str = ".agents/skills/tdd/interface-design.md";
const TDD_REFACTORING: &str = ".agents/skills/tdd/refactoring.md";
const DIAGNOSE_SKILL: &str = ".agents/skills/diagnose/SKILL.md";
const DIAGNOSE_OPENAI_YAML: &str = ".agents/skills/diagnose/agents/openai.yaml";
const DIAGNOSE_HITL_LOOP_TEMPLATE: &str = ".agents/skills/diagnose/scripts/hitl-loop.template.sh";
const ARCHITECTURE_SKILL: &str = ".agents/skills/improve-codebase-architecture/SKILL.md";
const ARCHITECTURE_OPENAI_YAML: &str =
    ".agents/skills/improve-codebase-architecture/agents/openai.yaml";
const ARCHITECTURE_LANGUAGE: &str = ".agents/skills/improve-codebase-architecture/LANGUAGE.md";
const ARCHITECTURE_INTERFACE_DESIGN: &str =
    ".agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md";
const ARCHITECTURE_DEEPENING: &str = ".agents/skills/improve-codebase-architecture/DEEPENING.md";
const PROTOTYPE_SKILL: &str = ".agents/skills/prototype/SKILL.md";
const PROTOTYPE_OPENAI_YAML: &str = ".agents/skills/prototype/agents/openai.yaml";
const PROTOTYPE_LOGIC: &str = ".agents/skills/prototype/LOGIC.md";
const PROTOTYPE_UI: &str = ".agents/skills/prototype/UI.md";
const LEGACY_PLAN_EXECUTION_PROMPT_FORMAT: &str = ".agents/skills/plan/EXECUTION-PROMPT-FORMAT.md";
const WORKFLOW_DOC: &str = "docs/agents/codex1-workflow.md";
const DOMAIN_DOC: &str = "docs/agents/codex1-domain.md";
const ARTIFACT_BRIEFS_DOC: &str = "docs/agents/codex1-artifact-briefs.md";
const BUNDLE_GUIDANCE: &str = "AGENTS.md";
const BUNDLE_MARKER: &str = ".codex1/setup-bundle.json";
const BACKUP_MANIFEST: &str = ".codex1/setup-backups/manifest.json";
const BACKUP_DIR: &str = ".codex1/setup-backups/files";
const MANAGED_SKILL_FILES: [&str; 8] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CREATE_PRD_SKILL,
    PLAN_SKILL,
    TDD_SKILL,
    DIAGNOSE_SKILL,
    ARCHITECTURE_SKILL,
    PROTOTYPE_SKILL,
];
const MANAGED_SUPPORTING_DOC_FILES: [&str; 28] = [
    OVERVIEW_OPENAI_YAML,
    CLARIFY_OPENAI_YAML,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_OPENAI_YAML,
    CREATE_PRD_FORMAT,
    PLAN_OPENAI_YAML,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    TDD_OPENAI_YAML,
    TDD_TESTS,
    TDD_MOCKING,
    TDD_DEEP_MODULES,
    TDD_INTERFACE_DESIGN,
    TDD_REFACTORING,
    DIAGNOSE_OPENAI_YAML,
    DIAGNOSE_HITL_LOOP_TEMPLATE,
    ARCHITECTURE_OPENAI_YAML,
    ARCHITECTURE_LANGUAGE,
    ARCHITECTURE_INTERFACE_DESIGN,
    ARCHITECTURE_DEEPENING,
    PROTOTYPE_OPENAI_YAML,
    PROTOTYPE_LOGIC,
    PROTOTYPE_UI,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
];
const MANAGED_BUNDLE_FILES: [&str; 37] = [
    OVERVIEW_SKILL,
    OVERVIEW_OPENAI_YAML,
    CLARIFY_SKILL,
    CLARIFY_OPENAI_YAML,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_OPENAI_YAML,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_OPENAI_YAML,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    TDD_SKILL,
    TDD_OPENAI_YAML,
    TDD_TESTS,
    TDD_MOCKING,
    TDD_DEEP_MODULES,
    TDD_INTERFACE_DESIGN,
    TDD_REFACTORING,
    DIAGNOSE_SKILL,
    DIAGNOSE_OPENAI_YAML,
    DIAGNOSE_HITL_LOOP_TEMPLATE,
    ARCHITECTURE_SKILL,
    ARCHITECTURE_OPENAI_YAML,
    ARCHITECTURE_LANGUAGE,
    ARCHITECTURE_INTERFACE_DESIGN,
    ARCHITECTURE_DEEPENING,
    PROTOTYPE_SKILL,
    PROTOTYPE_OPENAI_YAML,
    PROTOTYPE_LOGIC,
    PROTOTYPE_UI,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];
const LEGACY_BUNDLE_FILES_V5: [&str; 14] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    PLAN_GOAL_BRIEF_FORMAT,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];
const LEGACY_BUNDLE_FILES_V4: [&str; 14] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CLARIFY_ADR_FORMAT,
    CLARIFY_CONTEXT_FORMAT,
    CREATE_PRD_SKILL,
    CREATE_PRD_FORMAT,
    PLAN_SKILL,
    PLAN_ADR_FORMAT,
    PLAN_SUBPLAN_BRIEF,
    LEGACY_PLAN_EXECUTION_PROMPT_FORMAT,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];
const LEGACY_BUNDLE_FILES_V3: [&str; 8] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CREATE_PRD_SKILL,
    PLAN_SKILL,
    WORKFLOW_DOC,
    DOMAIN_DOC,
    ARTIFACT_BRIEFS_DOC,
    BUNDLE_GUIDANCE,
];
const LEGACY_BUNDLE_FILES_V2: [&str; 5] = [
    OVERVIEW_SKILL,
    CLARIFY_SKILL,
    CREATE_PRD_SKILL,
    PLAN_SKILL,
    BUNDLE_GUIDANCE,
];
const LEGACY_BUNDLE_FILES_V1: [&str; 2] = [OVERVIEW_SKILL, BUNDLE_GUIDANCE];

#[derive(Clone, Debug, Serialize)]
pub struct SetupPlan {
    pub dry_run: bool,
    pub writes: Vec<PathBuf>,
    pub removes: Vec<PathBuf>,
    pub backups: Vec<PathBuf>,
    pub materialized: Vec<PathBuf>,
}

impl SetupPlan {
    fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            writes: Vec::new(),
            removes: Vec::new(),
            backups: Vec::new(),
            materialized: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupStatus {
    pub repo: PathBuf,
    pub marker: SetupFileState,
    pub skill: SetupFileState,
    pub skills: Vec<SetupManagedSkill>,
    pub supporting_doc: SetupFileState,
    pub supporting_docs: Vec<SetupManagedDoc>,
    pub guidance: SetupFileState,
    pub repo_bundle_materialized: bool,
    pub backups_available: usize,
    pub warnings: Vec<String>,
    pub anti_oracle: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupManagedSkill {
    pub path: &'static str,
    pub state: SetupFileState,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupManagedDoc {
    pub path: &'static str,
    pub state: SetupFileState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SetupFileState {
    Current,
    Missing,
    Stale,
    Invalid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: u32,
    #[serde(default)]
    pub records: Vec<BackupRecord>,
}

impl Default for BackupManifest {
    fn default() -> Self {
        Self {
            version: BACKUP_MANIFEST_VERSION,
            records: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub timestamp: String,
    pub target_kind: String,
    pub target_path: PathBuf,
    pub target_path_label: String,
    pub backup_path: Option<PathBuf>,
    pub existed: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BundleMarker {
    managed_by: String,
    version: u32,
    files: Vec<String>,
}

pub fn run(cli_json: bool, global_repo: Option<PathBuf>, command: SetupCommand) -> Result<()> {
    match command {
        SetupCommand::Install(mut args) | SetupCommand::Enable(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, install(args)?)
        }
        SetupCommand::Disable(mut args) | SetupCommand::Uninstall(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, uninstall(args)?)
        }
        SetupCommand::Status(mut args) => {
            args.repo = args.repo.or(global_repo);
            emit(cli_json, status_value(args)?)
        }
        SetupCommand::Doctor(mut args) => {
            args.repo = args.repo.or(global_repo);
            emit(cli_json, doctor(args)?)
        }
        SetupCommand::Backups { command } => match command {
            SetupBackupsCommand::List => {
                let args = SetupStatusArgs { repo: global_repo };
                emit(cli_json, backups_list(args)?)
            }
            SetupBackupsCommand::Restore(mut args) => {
                args.repo = args.repo.or(global_repo);
                emit(cli_json, backups_restore(args)?)
            }
        },
    }
}

fn install(args: SetupRepoArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let mut plan = SetupPlan::new(args.dry_run);
    materialize_bundle(&repo, &mut plan, args.dry_run)?;
    Ok(json!({
        "summary": "setup installed repo-scoped Codex1 guidance",
        "repo": repo,
        "plan": plan,
        "anti_oracle": "setup materializes artifact workflow guidance only",
    }))
}

fn uninstall(args: SetupRepoArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let mut plan = SetupPlan::new(args.dry_run);
    remove_bundle(&repo, &mut plan, args.dry_run)?;
    Ok(json!({
        "summary": "setup removed repo-scoped Codex1 guidance",
        "repo": repo,
        "plan": plan,
        "anti_oracle": "setup removal does not delete mission artifacts",
    }))
}

fn status_value(args: SetupStatusArgs) -> Result<serde_json::Value> {
    let status = status(args.repo)?;
    Ok(json!({
        "summary": "setup status complete",
        "status": status,
    }))
}

fn doctor(args: SetupStatusArgs) -> Result<serde_json::Value> {
    let status = status(args.repo)?;
    let backup_manifest = match read_manifest(&status.repo) {
        Ok(_) => json!({"name": "backup_manifest", "ok": true}),
        Err(error) => {
            json!({"name": "backup_manifest", "ok": false, "error": error.to_string()})
        }
    };
    let mut checks = vec![
        json!({"name": "bundle_marker", "ok": status.marker == SetupFileState::Current}),
        json!({"name": "managed_skill", "ok": status.skill == SetupFileState::Current}),
        json!({"name": "managed_supporting_doc", "ok": status.supporting_doc == SetupFileState::Current}),
        json!({"name": "managed_guidance", "ok": status.guidance == SetupFileState::Current}),
        backup_manifest,
    ];
    checks.extend(status.skills.iter().map(|skill| {
        json!({
            "name": "managed_skill_file",
            "path": skill.path,
            "ok": skill.state == SetupFileState::Current,
        })
    }));
    checks.extend(status.supporting_docs.iter().map(|doc| {
        json!({
            "name": "managed_supporting_doc_file",
            "path": doc.path,
            "ok": doc.state == SetupFileState::Current,
        })
    }));
    Ok(json!({
        "summary": "setup doctor complete",
        "checks": checks,
        "status": status,
        "anti_oracle": "setup doctor diagnoses repo guidance mechanics only",
    }))
}

fn status(repo_arg: Option<PathBuf>) -> Result<SetupStatus> {
    let repo = resolve_repo(repo_arg)?;
    let marker = marker_state(&repo);
    let skills: Vec<_> = MANAGED_SKILL_FILES
        .iter()
        .map(|path| SetupManagedSkill {
            path,
            state: owned_file_state(&repo, path),
        })
        .collect();
    let skill = aggregate_states(skills.iter().map(|skill| skill.state));
    let supporting_docs: Vec<_> = MANAGED_SUPPORTING_DOC_FILES
        .iter()
        .map(|path| SetupManagedDoc {
            path,
            state: owned_file_state(&repo, path),
        })
        .collect();
    let supporting_doc = aggregate_states(supporting_docs.iter().map(|doc| doc.state));
    let guidance = guidance_state(&repo);
    let mut warnings = Vec::new();
    for (name, state) in [
        ("marker", marker),
        ("skill", skill),
        ("supporting_doc", supporting_doc),
        ("guidance", guidance),
    ] {
        match state {
            SetupFileState::Current | SetupFileState::Missing => {}
            SetupFileState::Stale => warnings.push(format!("{name} is stale")),
            SetupFileState::Invalid => warnings.push(format!("{name} is invalid")),
        }
    }
    let backups_available = read_manifest(&repo)
        .map(|manifest| manifest.records.len())
        .unwrap_or(0);
    Ok(SetupStatus {
        repo,
        marker,
        skill,
        skills,
        supporting_doc,
        supporting_docs,
        guidance,
        repo_bundle_materialized: marker == SetupFileState::Current
            && skill == SetupFileState::Current
            && supporting_doc == SetupFileState::Current
            && guidance == SetupFileState::Current,
        backups_available,
        warnings,
        anti_oracle: "setup status is mechanical and does not report mission or native goal state",
    })
}

fn backups_list(args: SetupStatusArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let manifest = read_manifest(&repo)?;
    Ok(json!({
        "summary": "setup backups listed",
        "repo": repo,
        "backups": manifest.records,
    }))
}

fn backups_restore(args: SetupBackupRestoreArgs) -> Result<serde_json::Value> {
    if !args.force && !args.dry_run {
        return Err(Codex1Error::SetupRestore(
            "setup backups restore requires --force".into(),
        ));
    }
    let repo = resolve_repo(args.repo)?;
    let manifest = read_manifest(&repo)?;
    let record = manifest
        .records
        .iter()
        .find(|record| record.id == args.id)
        .ok_or_else(|| Codex1Error::SetupRestore(format!("backup not found: {}", args.id)))?;
    ensure_restore_target(&repo, &record.target_path)?;
    let mut plan = SetupPlan::new(args.dry_run);
    if record.existed {
        let backup_path = record.backup_path.as_ref().ok_or_else(|| {
            Codex1Error::SetupRestore(format!("backup record {} has no file", record.id))
        })?;
        ensure_backup_file(&repo, backup_path)?;
        plan.writes.push(record.target_path.clone());
        if !args.dry_run {
            if let Some(parent) = record.target_path.parent() {
                create_dir_all_contained(&repo, parent.strip_prefix(&repo).unwrap())?;
            }
            fs::copy(backup_path, &record.target_path).map_err(|source| {
                Codex1Error::SetupRestore(format!(
                    "failed to restore {} from {}: {source}",
                    record.target_path.display(),
                    backup_path.display()
                ))
            })?;
        }
    } else {
        plan.removes.push(record.target_path.clone());
        restore_absence(&repo, &record.target_path, args.dry_run)?;
    }
    Ok(json!({
        "summary": "setup backup restored",
        "repo": repo,
        "record": record,
        "plan": plan,
    }))
}

fn emit(cli_json: bool, data: serde_json::Value) -> Result<()> {
    if cli_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope::success(data)).unwrap()
        );
    } else if let Some(summary) = data.get("summary").and_then(|value| value.as_str()) {
        println!("{summary}");
    } else {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    }
    Ok(())
}

fn resolve_repo(repo_arg: Option<PathBuf>) -> Result<PathBuf> {
    discover_repo_root(repo_arg)
}

fn materialize_bundle(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    let guidance = setup_target(repo, BUNDLE_GUIDANCE)?;
    let marker = setup_target(repo, BUNDLE_MARKER)?;
    let marker_data = read_bundle_marker(&marker)?;
    if marker_data
        .as_ref()
        .is_some_and(|marker| !is_known_managed_marker(marker))
    {
        return Err(Codex1Error::SetupBundle(
            "invalid Codex1 setup bundle marker".into(),
        ));
    }

    for relative in MANAGED_SKILL_FILES {
        let skill = setup_target(repo, relative)?;
        ensure_owned_file_writable(
            &skill,
            &expected_body(relative),
            marker_allows_file_repair(marker_data.as_ref(), relative),
        )?;
    }
    for relative in MANAGED_SUPPORTING_DOC_FILES {
        let doc = setup_target(repo, relative)?;
        ensure_owned_file_writable(
            &doc,
            &expected_body(relative),
            marker_allows_file_repair(marker_data.as_ref(), relative),
        )?;
    }

    for relative in MANAGED_SKILL_FILES {
        let skill = setup_target(repo, relative)?;
        write_owned_file(
            repo,
            &skill,
            &expected_body(relative),
            marker_allows_file_repair(marker_data.as_ref(), relative),
            "managed skill",
            plan,
            dry_run,
        )?;
    }
    for relative in MANAGED_SUPPORTING_DOC_FILES {
        let doc = setup_target(repo, relative)?;
        write_owned_file(
            repo,
            &doc,
            &expected_body(relative),
            marker_allows_file_repair(marker_data.as_ref(), relative),
            "managed supporting doc",
            plan,
            dry_run,
        )?;
    }
    remove_legacy_bundle_files_not_current(repo, marker_data.as_ref(), plan, dry_run)?;
    write_guidance_file(repo, &guidance, plan, dry_run)?;
    write_owned_file(
        repo,
        &marker,
        &bundle_marker_body(),
        marker_data.as_ref().is_some_and(is_known_managed_marker),
        "bundle marker",
        plan,
        dry_run,
    )?;
    Ok(())
}

fn remove_legacy_bundle_files_not_current(
    repo: &Path,
    marker: Option<&BundleMarker>,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    let Some(marker) = marker.filter(|marker| is_known_managed_marker(marker)) else {
        return Ok(());
    };
    let current = bundle_files(MANAGED_BUNDLE_FILES);
    for relative in &marker.files {
        if current.iter().any(|current| current == relative) || relative == BUNDLE_GUIDANCE {
            continue;
        }
        let expected = expected_body(relative);
        if expected.is_empty() {
            continue;
        }
        let path = setup_target(repo, relative)?;
        match fs::read_to_string(&path) {
            Ok(text) if text == expected => {
                backup_target(
                    repo,
                    &path,
                    "remove legacy managed setup file",
                    plan,
                    dry_run,
                )?;
                plan.removes.push(path.clone());
                if !dry_run {
                    fs::remove_file(&path).map_err(|source| {
                        Codex1Error::SetupBundle(format!(
                            "failed to remove {}: {source}",
                            path.display()
                        ))
                    })?;
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Codex1Error::SetupBundle(format!(
                    "failed to read {}: {error}",
                    path.display()
                )))
            }
        }
    }
    Ok(())
}

fn ensure_owned_file_writable(path: &Path, body: &str, allow_repair: bool) -> Result<()> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == body => Ok(()),
        Ok(_) if !allow_repair => Err(Codex1Error::SetupBundle(format!(
            "refusing to overwrite non-managed file {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Codex1Error::SetupBundle(format!(
            "failed to read {}: {error}",
            path.display()
        ))),
    }
}

fn remove_bundle(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    let marker = setup_target(repo, BUNDLE_MARKER)?;
    let (files, strict) = match read_bundle_marker(&marker)? {
        Some(marker_data) => {
            if !is_known_managed_marker(&marker_data) {
                return Err(Codex1Error::SetupBundle(
                    "invalid Codex1 setup bundle marker".into(),
                ));
            }
            (marker_data.files, true)
        }
        None => (MANAGED_BUNDLE_FILES.map(String::from).to_vec(), false),
    };
    for relative in files {
        let path = setup_target(repo, &relative)?;
        if relative == BUNDLE_GUIDANCE {
            remove_guidance_if_owned(repo, &path, strict, plan, dry_run)?;
        } else {
            remove_owned_file(
                repo,
                &path,
                &expected_body(&relative),
                strict,
                plan,
                dry_run,
            )?;
        }
    }
    remove_bundle_marker_file(repo, &marker, plan, dry_run)?;
    Ok(())
}

fn write_owned_file(
    repo: &Path,
    path: &Path,
    body: &str,
    allow_repair: bool,
    reason: &str,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    ensure_setup_target(repo, path)?;
    match fs::read_to_string(path) {
        Ok(existing) if existing == body => return Ok(()),
        Ok(_) if !allow_repair => {
            return Err(Codex1Error::SetupBundle(format!(
                "refusing to overwrite non-managed file {}",
                path.display()
            )))
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    }
    backup_target(repo, path, reason, plan, dry_run)?;
    plan.writes.push(path.to_path_buf());
    plan.materialized.push(path.to_path_buf());
    if !dry_run {
        if let Some(parent) = path.parent() {
            create_dir_all_contained(repo, parent.strip_prefix(repo).unwrap())?;
        }
        fs::write(path, body).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to write {}: {source}", path.display()))
        })?;
    }
    Ok(())
}

fn write_guidance_file(
    repo: &Path,
    path: &Path,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    ensure_setup_target(repo, path)?;
    let block = managed_guidance_block();
    let next = match fs::read_to_string(path) {
        Ok(existing) if existing == guidance_body() || existing.contains(&block) => return Ok(()),
        Ok(existing) if guidance_has_managed_block(&existing) => {
            replace_guidance_block(&existing, &block).ok_or_else(|| {
                Codex1Error::SetupBundle(format!(
                    "failed to replace managed guidance block in {}",
                    path.display()
                ))
            })?
        }
        Ok(mut existing) => {
            if !existing.ends_with('\n') {
                existing.push('\n');
            }
            if !existing.ends_with("\n\n") {
                existing.push('\n');
            }
            existing.push_str(&block);
            existing
        }
        Err(error) if error.kind() == ErrorKind::NotFound => block,
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    backup_target(repo, path, "managed guidance", plan, dry_run)?;
    plan.writes.push(path.to_path_buf());
    plan.materialized.push(path.to_path_buf());
    if !dry_run {
        if let Some(parent) = path.parent() {
            create_dir_all_contained(repo, parent.strip_prefix(repo).unwrap())?;
        }
        fs::write(path, next).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to write {}: {source}", path.display()))
        })?;
    }
    Ok(())
}

fn remove_owned_file(
    repo: &Path,
    path: &Path,
    expected: &str,
    strict: bool,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    ensure_setup_target(repo, path)?;
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    if text != expected && strict {
        return Err(Codex1Error::SetupBundle(format!(
            "refusing to remove non-managed file {}",
            path.display()
        )));
    }
    if text != expected {
        return Ok(());
    }
    backup_target(repo, path, "remove managed setup file", plan, dry_run)?;
    plan.removes.push(path.to_path_buf());
    if !dry_run {
        fs::remove_file(path).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to remove {}: {source}", path.display()))
        })?;
    }
    Ok(())
}

fn remove_guidance_if_owned(
    repo: &Path,
    path: &Path,
    strict: bool,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    ensure_setup_target(repo, path)?;
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    let next = if text == guidance_body() || text == managed_guidance_block() {
        None
    } else {
        let Some(edited) = remove_guidance_block(&text) else {
            if strict {
                return Err(Codex1Error::SetupBundle(format!(
                    "refusing to remove unmanaged guidance file {}",
                    path.display()
                )));
            }
            return Ok(());
        };
        Some(edited)
    };
    backup_target(repo, path, "remove managed guidance", plan, dry_run)?;
    plan.removes.push(path.to_path_buf());
    if !dry_run {
        match next {
            Some(edited) if !edited.trim().is_empty() => {
                fs::write(path, edited).map_err(|source| {
                    Codex1Error::SetupBundle(format!(
                        "failed to write {}: {source}",
                        path.display()
                    ))
                })?
            }
            _ => fs::remove_file(path).map_err(|source| {
                Codex1Error::SetupBundle(format!("failed to remove {}: {source}", path.display()))
            })?,
        }
    }
    Ok(())
}

fn remove_bundle_marker_file(
    repo: &Path,
    path: &Path,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    ensure_setup_target(repo, path)?;
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    let marker: BundleMarker = serde_json::from_str(&text).map_err(|source| {
        Codex1Error::SetupBundle(format!("failed to parse {}: {source}", path.display()))
    })?;
    if !is_known_managed_marker(&marker) {
        return Err(Codex1Error::SetupBundle(format!(
            "refusing to remove non-managed marker {}",
            path.display()
        )));
    }
    backup_target(repo, path, "remove managed setup marker", plan, dry_run)?;
    plan.removes.push(path.to_path_buf());
    if !dry_run {
        fs::remove_file(path).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to remove {}: {source}", path.display()))
        })?;
    }
    Ok(())
}

fn backup_target(
    repo: &Path,
    target: &Path,
    reason: &str,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    ensure_setup_target(repo, target)?;
    let existed = target.exists();
    let mut manifest = read_manifest(repo)?;
    let id = format!(
        "{}-{}",
        Utc::now().format("%Y%m%dT%H%M%S%3fZ"),
        manifest.records.len() + 1
    );
    let backup_path = if existed {
        let relative = target.strip_prefix(repo).map_err(|_| {
            Codex1Error::SetupBackup(format!("target escapes repo: {}", target.display()))
        })?;
        let backup = setup_target(repo, Path::new(BACKUP_DIR).join(&id).join(relative))?;
        plan.backups.push(backup.clone());
        if !dry_run {
            if let Some(parent) = backup.parent() {
                create_dir_all_contained(repo, parent.strip_prefix(repo).unwrap())?;
            }
            fs::copy(target, &backup).map_err(|source| {
                Codex1Error::SetupBackup(format!(
                    "failed to back up {} to {}: {source}",
                    target.display(),
                    backup.display()
                ))
            })?;
        }
        Some(backup)
    } else {
        plan.backups.push(target.to_path_buf());
        None
    };
    if !dry_run {
        manifest.records.push(BackupRecord {
            id,
            timestamp: Utc::now().to_rfc3339(),
            target_kind: "repo-setup".into(),
            target_path: target.to_path_buf(),
            target_path_label: target.display().to_string(),
            backup_path,
            existed,
            reason: reason.to_string(),
        });
        write_manifest(repo, &manifest)?;
    }
    Ok(())
}

fn read_manifest(repo: &Path) -> Result<BackupManifest> {
    let path = setup_target(repo, BACKUP_MANIFEST)?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(BackupManifest::default()),
        Err(error) => {
            return Err(Codex1Error::SetupBackup(format!(
                "failed to read backup manifest {}: {error}",
                path.display()
            )))
        }
    };
    serde_json::from_str(&text).map_err(|source| {
        Codex1Error::SetupBackup(format!(
            "failed to parse backup manifest {}: {source}",
            path.display()
        ))
    })
}

fn write_manifest(repo: &Path, manifest: &BackupManifest) -> Result<()> {
    let path = setup_target(repo, BACKUP_MANIFEST)?;
    if let Some(parent) = path.parent() {
        create_dir_all_contained(repo, parent.strip_prefix(repo).unwrap())?;
    }
    let text = serde_json::to_string_pretty(manifest).unwrap();
    fs::write(&path, text + "\n").map_err(|source| {
        Codex1Error::SetupBackup(format!(
            "failed to write backup manifest {}: {source}",
            path.display()
        ))
    })
}

fn setup_target(repo: &Path, relative: impl AsRef<Path>) -> Result<PathBuf> {
    let relative = relative.as_ref();
    if relative.is_absolute() {
        return Err(Codex1Error::SetupBundle(format!(
            "setup path must be relative: {}",
            relative.display()
        )));
    }
    let path = repo.join(relative);
    ensure_setup_target(repo, &path)?;
    Ok(path)
}

fn ensure_setup_target(repo: &Path, path: &Path) -> Result<()> {
    ensure_contained_for_write(repo, path).map_err(|error| {
        Codex1Error::SetupBundle(format!(
            "setup path escapes repo or crosses a symlink: {}: {error}",
            path.display()
        ))
    })
}

fn ensure_restore_target(repo: &Path, path: &Path) -> Result<()> {
    ensure_setup_target(repo, path).map_err(|error| {
        Codex1Error::SetupRestore(format!(
            "invalid restore target {}: {error}",
            path.display()
        ))
    })?;
    for relative in managed_restore_files() {
        if path == setup_target(repo, relative)? {
            return Ok(());
        }
    }
    Err(Codex1Error::SetupRestore(format!(
        "backup target is not a managed setup file: {}",
        path.display()
    )))
}

fn ensure_backup_file(repo: &Path, path: &Path) -> Result<()> {
    ensure_setup_target(repo, path).map_err(|error| {
        Codex1Error::SetupRestore(format!("invalid backup file {}: {error}", path.display()))
    })?;
    let backup_root = setup_target(repo, BACKUP_DIR)?;
    let backup_root = fs::canonicalize(&backup_root).map_err(|source| {
        Codex1Error::SetupRestore(format!(
            "failed to canonicalize backup root {}: {source}",
            backup_root.display()
        ))
    })?;
    let path = fs::canonicalize(path).map_err(|source| {
        Codex1Error::SetupRestore(format!(
            "failed to canonicalize backup file {}: {source}",
            path.display()
        ))
    })?;
    if path.starts_with(&backup_root) {
        Ok(())
    } else {
        Err(Codex1Error::SetupRestore(format!(
            "backup file is outside setup backups: {}",
            path.display()
        )))
    }
}

fn restore_absence(repo: &Path, path: &Path, dry_run: bool) -> Result<()> {
    if path == setup_target(repo, BUNDLE_GUIDANCE)? {
        return restore_guidance_absence(path, dry_run);
    }
    let expected = if let Some(relative) = managed_owned_relative_for_path(repo, path)? {
        expected_body(relative)
    } else if path == setup_target(repo, BUNDLE_MARKER)? {
        expected_body(BUNDLE_MARKER)
    } else {
        return Err(Codex1Error::SetupRestore(format!(
            "backup target is not a managed setup file: {}",
            path.display()
        )));
    };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Codex1Error::SetupRestore(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    if text != expected {
        return Err(Codex1Error::SetupRestore(format!(
            "refusing to remove non-managed setup file {}",
            path.display()
        )));
    }
    if dry_run {
        return Ok(());
    }
    fs::remove_file(path).map_err(|source| {
        Codex1Error::SetupRestore(format!("failed to remove {}: {source}", path.display()))
    })
}

fn restore_guidance_absence(path: &Path, dry_run: bool) -> Result<()> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Codex1Error::SetupRestore(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    if text == guidance_body() || text == managed_guidance_block() {
        if dry_run {
            return Ok(());
        }
        return fs::remove_file(path).map_err(|source| {
            Codex1Error::SetupRestore(format!("failed to remove {}: {source}", path.display()))
        });
    }
    let Some(edited) = remove_guidance_block(&text) else {
        return Err(Codex1Error::SetupRestore(format!(
            "refusing to remove non-managed guidance file {}",
            path.display()
        )));
    };
    if dry_run {
        return Ok(());
    }
    if edited.trim().is_empty() {
        fs::remove_file(path).map_err(|source| {
            Codex1Error::SetupRestore(format!("failed to remove {}: {source}", path.display()))
        })
    } else {
        fs::write(path, edited).map_err(|source| {
            Codex1Error::SetupRestore(format!("failed to write {}: {source}", path.display()))
        })
    }
}

fn marker_state(repo: &Path) -> SetupFileState {
    let Ok(path) = setup_target(repo, BUNDLE_MARKER) else {
        return SetupFileState::Invalid;
    };
    match read_bundle_marker(&path) {
        Ok(Some(marker)) => match validate_marker(&marker) {
            Ok(()) => SetupFileState::Current,
            Err(_) => SetupFileState::Invalid,
        },
        Ok(None) => SetupFileState::Missing,
        Err(_) => SetupFileState::Invalid,
    }
}

fn owned_file_state(repo: &Path, relative: &str) -> SetupFileState {
    let Ok(path) = setup_target(repo, relative) else {
        return SetupFileState::Invalid;
    };
    match fs::read_to_string(&path) {
        Ok(text) if text == expected_body(relative) => SetupFileState::Current,
        Ok(_) => SetupFileState::Stale,
        Err(error) if error.kind() == ErrorKind::NotFound => SetupFileState::Missing,
        Err(_) => SetupFileState::Invalid,
    }
}

fn guidance_state(repo: &Path) -> SetupFileState {
    let Ok(path) = setup_target(repo, BUNDLE_GUIDANCE) else {
        return SetupFileState::Invalid;
    };
    match fs::read_to_string(&path) {
        Ok(text) if text == guidance_body() || text.contains(&managed_guidance_block()) => {
            SetupFileState::Current
        }
        Ok(text) if guidance_has_managed_block(&text) => SetupFileState::Stale,
        Ok(_) => SetupFileState::Missing,
        Err(error) if error.kind() == ErrorKind::NotFound => SetupFileState::Missing,
        Err(_) => SetupFileState::Invalid,
    }
}

fn read_bundle_marker(path: &Path) -> Result<Option<BundleMarker>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    serde_json::from_str(&text).map(Some).map_err(|source| {
        Codex1Error::SetupBundle(format!("failed to parse {}: {source}", path.display()))
    })
}

fn validate_marker(marker: &BundleMarker) -> Result<()> {
    if !is_current_marker(marker) {
        return Err(Codex1Error::SetupBundle(
            "invalid Codex1 setup bundle marker".into(),
        ));
    }
    Ok(())
}

fn is_current_marker(marker: &BundleMarker) -> bool {
    marker.managed_by == "codex1-managed"
        && marker.version == BUNDLE_VERSION
        && marker.files == bundle_files(MANAGED_BUNDLE_FILES)
}

fn is_known_managed_marker(marker: &BundleMarker) -> bool {
    marker.managed_by == "codex1-managed"
        && (marker.files == bundle_files(MANAGED_BUNDLE_FILES)
            || marker.files == bundle_files(LEGACY_BUNDLE_FILES_V5)
            || marker.files == bundle_files(LEGACY_BUNDLE_FILES_V4)
            || marker.files == bundle_files(LEGACY_BUNDLE_FILES_V3)
            || marker.files == bundle_files(LEGACY_BUNDLE_FILES_V2)
            || marker.files == bundle_files(LEGACY_BUNDLE_FILES_V1))
}

fn marker_allows_file_repair(marker: Option<&BundleMarker>, relative: &str) -> bool {
    marker.is_some_and(|marker| {
        is_known_managed_marker(marker) && marker.files.iter().any(|file| file == relative)
    })
}

fn aggregate_states(states: impl IntoIterator<Item = SetupFileState>) -> SetupFileState {
    let states: Vec<_> = states.into_iter().collect();
    if states.iter().all(|state| *state == SetupFileState::Current) {
        SetupFileState::Current
    } else if states.contains(&SetupFileState::Invalid) {
        SetupFileState::Invalid
    } else if states.iter().all(|state| *state == SetupFileState::Missing) {
        SetupFileState::Missing
    } else {
        SetupFileState::Stale
    }
}

fn expected_body(relative: &str) -> String {
    match relative {
        OVERVIEW_SKILL => overview_skill_body().to_string(),
        OVERVIEW_OPENAI_YAML => {
            include_str!("../.agents/skills/codex1/agents/openai.yaml").to_string()
        }
        CLARIFY_SKILL => clarify_skill_body().to_string(),
        CLARIFY_OPENAI_YAML => {
            include_str!("../.agents/skills/clarify/agents/openai.yaml").to_string()
        }
        CLARIFY_ADR_FORMAT => adr_format_body().to_string(),
        CLARIFY_CONTEXT_FORMAT => context_format_body().to_string(),
        CREATE_PRD_SKILL => create_prd_skill_body().to_string(),
        CREATE_PRD_OPENAI_YAML => {
            include_str!("../.agents/skills/create-prd/agents/openai.yaml").to_string()
        }
        CREATE_PRD_FORMAT => prd_format_body().to_string(),
        PLAN_SKILL => plan_skill_body().to_string(),
        PLAN_OPENAI_YAML => include_str!("../.agents/skills/plan/agents/openai.yaml").to_string(),
        PLAN_ADR_FORMAT => adr_format_body().to_string(),
        PLAN_SUBPLAN_BRIEF => subplan_brief_body().to_string(),
        PLAN_GOAL_BRIEF_FORMAT => goal_brief_format_body().to_string(),
        TDD_SKILL => include_str!("../.agents/skills/tdd/SKILL.md").to_string(),
        TDD_OPENAI_YAML => include_str!("../.agents/skills/tdd/agents/openai.yaml").to_string(),
        TDD_TESTS => include_str!("../.agents/skills/tdd/tests.md").to_string(),
        TDD_MOCKING => include_str!("../.agents/skills/tdd/mocking.md").to_string(),
        TDD_DEEP_MODULES => include_str!("../.agents/skills/tdd/deep-modules.md").to_string(),
        TDD_INTERFACE_DESIGN => {
            include_str!("../.agents/skills/tdd/interface-design.md").to_string()
        }
        TDD_REFACTORING => include_str!("../.agents/skills/tdd/refactoring.md").to_string(),
        DIAGNOSE_SKILL => include_str!("../.agents/skills/diagnose/SKILL.md").to_string(),
        DIAGNOSE_OPENAI_YAML => {
            include_str!("../.agents/skills/diagnose/agents/openai.yaml").to_string()
        }
        DIAGNOSE_HITL_LOOP_TEMPLATE => {
            include_str!("../.agents/skills/diagnose/scripts/hitl-loop.template.sh").to_string()
        }
        ARCHITECTURE_SKILL => {
            include_str!("../.agents/skills/improve-codebase-architecture/SKILL.md").to_string()
        }
        ARCHITECTURE_OPENAI_YAML => {
            include_str!("../.agents/skills/improve-codebase-architecture/agents/openai.yaml")
                .to_string()
        }
        ARCHITECTURE_LANGUAGE => {
            include_str!("../.agents/skills/improve-codebase-architecture/LANGUAGE.md").to_string()
        }
        ARCHITECTURE_INTERFACE_DESIGN => {
            include_str!("../.agents/skills/improve-codebase-architecture/INTERFACE-DESIGN.md")
                .to_string()
        }
        ARCHITECTURE_DEEPENING => {
            include_str!("../.agents/skills/improve-codebase-architecture/DEEPENING.md").to_string()
        }
        PROTOTYPE_SKILL => include_str!("../.agents/skills/prototype/SKILL.md").to_string(),
        PROTOTYPE_OPENAI_YAML => {
            include_str!("../.agents/skills/prototype/agents/openai.yaml").to_string()
        }
        PROTOTYPE_LOGIC => include_str!("../.agents/skills/prototype/LOGIC.md").to_string(),
        PROTOTYPE_UI => include_str!("../.agents/skills/prototype/UI.md").to_string(),
        LEGACY_PLAN_EXECUTION_PROMPT_FORMAT => legacy_execution_prompt_format_body().to_string(),
        WORKFLOW_DOC => workflow_doc_body().to_string(),
        DOMAIN_DOC => domain_doc_body().to_string(),
        ARTIFACT_BRIEFS_DOC => artifact_briefs_doc_body().to_string(),
        BUNDLE_GUIDANCE => guidance_body().to_string(),
        BUNDLE_MARKER => bundle_marker_body(),
        _ => String::new(),
    }
}

fn managed_restore_files() -> Vec<&'static str> {
    let mut files = MANAGED_BUNDLE_FILES.to_vec();
    files.push(BUNDLE_MARKER);
    files
}

fn managed_owned_relative_for_path(repo: &Path, path: &Path) -> Result<Option<&'static str>> {
    for &relative in &MANAGED_BUNDLE_FILES {
        if relative == BUNDLE_GUIDANCE {
            continue;
        }
        if path == setup_target(repo, relative)? {
            return Ok(Some(relative));
        }
    }
    Ok(None)
}

fn bundle_files<const N: usize>(files: [&str; N]) -> Vec<String> {
    files.map(String::from).to_vec()
}

fn bundle_marker_body() -> String {
    serde_json::to_string_pretty(&BundleMarker {
        managed_by: "codex1-managed".into(),
        version: BUNDLE_VERSION,
        files: bundle_files(MANAGED_BUNDLE_FILES),
    })
    .unwrap()
        + "\n"
}

fn overview_skill_body() -> &'static str {
    r#"---
name: codex1
description: Repo-scoped Codex1 artifact workflow overview. Use as a router to the clarify, create-prd, and plan skills, and as a reminder of the native /goal boundary.
---

# Codex1

Codex1 is a deterministic artifact helper for clarification context, PRD, PLAN, GOAL_BRIEF, SPEC, SUBPLAN, REVIEW, TRIAGE, PROOF, CLOSEOUT, receipts, and inventory inspection.

Repo-local consumer docs installed by setup:

- `docs/agents/codex1-workflow.md`: the user-facing flow and native `/goal` boundary.
- `docs/agents/codex1-domain.md`: domain glossary and ADR consumption/production rules.
- `docs/agents/codex1-artifact-briefs.md`: PRD, subplan, goal brief, proof, review, and closeout quality bars.

Skill-local references installed by setup:

- `$clarify`: `ADR-FORMAT.md` and `CONTEXT-FORMAT.md`.
- `$create-prd`: `PRD-FORMAT.md`.
- `$plan`: `ADR-FORMAT.md`, `SUBPLAN-BRIEF.md`, and `GOAL-BRIEF-FORMAT.md`.
- `$tdd`: red-green-refactor guidance plus testing, mocking, interface, deep-module, and refactoring references.
- `$diagnose`: reproduce-first debugging guidance plus the HITL loop template.
- `$improve-codebase-architecture`: deep-module architecture guidance and interface references.
- `$prototype`: throwaway logic and UI prototype guidance.

Preferred UX:

- Use `$clarify` to gather and preserve user intent while questions are still allowed.
- Use `$create-prd` to synthesize known context into `PRD.md`.
- Use `$plan` to design the mission and write `GOAL_BRIEF.md`.
- The user asks Codex to create or refine a native goal from the generated goal brief.

During execution, ready subplans may name an `Execution Lane`: `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`. `$plan` assigns lanes; native `/goal` executes them.

Native Codex `/goal` owns persistent objectives, continuation, pause/resume, accounting, budgets, and completion. Codex1 must not create, mirror, inspect, or complete native goals.

Codex1 setup is mechanical repo guidance. It is not mission truth, task readiness, review pass/fail, proof sufficiency, close safety, or native goal state.
"#
}

fn clarify_skill_body() -> &'static str {
    r#"---
name: clarify
description: Relentlessly clarify a future Codex1 mission before PRD synthesis. Use when the user wants write-me-docs/grill-me style discovery, to stress-test an idea, resolve ambiguity, or prepare context for create-prd.
---

<codex1-local>

Before asking questions, read repo-local Codex1 workflow docs and existing mission artifacts if present:

- `docs/agents/codex1-workflow.md`
- `docs/agents/codex1-domain.md`
- `docs/agents/codex1-artifact-briefs.md`
- `.codex1/missions/<id>/`

Clarify prepares context for `$create-prd`. Do not write `PRD.md` or `PLAN.md`, create issue-tracker tickets, or create/complete native `/goal` state unless the user explicitly switches workflows.

</codex1-local>

<what-to-do>

Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.

Ask the questions one at a time, waiting for feedback on each question before continuing.

If a question can be answered by exploring the codebase, explore the codebase instead.

</what-to-do>

<supporting-info>

## Domain awareness

During codebase exploration, also look for existing documentation:

### File structure

Most repos have a single context:

```
/
├── CONTEXT.md
├── docs/
│   └── adr/
│       ├── 0001-event-sourced-orders.md
│       └── 0002-postgres-for-write-model.md
└── src/
```

If a `CONTEXT-MAP.md` exists at the root, the repo has multiple contexts. The map points to where each one lives:

```
/
├── CONTEXT-MAP.md
├── docs/
│   └── adr/                          ← system-wide decisions
├── src/
│   ├── ordering/
│   │   ├── CONTEXT.md
│   │   └── docs/adr/                 ← context-specific decisions
│   └── billing/
│       ├── CONTEXT.md
│       └── docs/adr/
```

Create files lazily — only when you have something to write. If no `CONTEXT.md` exists, create one when the first term is resolved. If no `docs/adr/` exists, create it when the first ADR is needed.

## During the session

### Challenge against the glossary

When the user uses a term that conflicts with the existing language in `CONTEXT.md`, call it out immediately. "Your glossary defines 'cancellation' as X, but you seem to mean Y — which is it?"

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term. "You're saying 'account' — do you mean the Customer or the User? Those are different things."

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific scenarios. Invent scenarios that probe edge cases and force the user to be precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you find a contradiction, surface it: "Your code cancels entire Orders, but you just said partial cancellation is possible — which is right?"

### Update CONTEXT.md inline

When a term is resolved, update `CONTEXT.md` right there. Don't batch these up — capture them as they happen. Use the format in [CONTEXT-FORMAT.md](./CONTEXT-FORMAT.md).

Don't couple `CONTEXT.md` to implementation details. Only include terms that are meaningful to domain experts.

### Offer ADRs sparingly

Only offer to create an ADR when all three are true:

1. **Hard to reverse** — the cost of changing your mind later is meaningful
2. **Surprising without context** — a future reader will wonder "why did they do it this way?"
3. **The result of a real trade-off** — there were genuine alternatives and you picked one for specific reasons

If any of the three is missing, skip the ADR. Use the format in [ADR-FORMAT.md](./ADR-FORMAT.md).

</supporting-info>
"#
}

fn create_prd_skill_body() -> &'static str {
    r#"---
name: create-prd
description: Synthesize known context into a local Codex1 PRD artifact. Use when the user wants a PRD from the current conversation, clarification output, repo context, and references; do not publish to an issue tracker.
---

This skill takes the current conversation context and codebase understanding and produces a PRD. Do NOT interview the user — just synthesize what you already know.

Codex1-local change: write `PRD.md` locally through the Codex1 artifact workflow. Do not publish to GitHub Issues, Linear, Jira, GitLab, or any issue tracker. Do not apply triage labels.

## Process

1. Explore the repo to understand the current state of the codebase, if you haven't already. Use the project's domain glossary vocabulary throughout the PRD, and respect any ADRs in the area you're touching. If Codex1 workflow docs or mission artifacts exist, use them as local context.

2. Sketch out the major modules you will need to build or modify to complete the implementation. Actively look for opportunities to extract deep modules that can be tested in isolation.

A deep module (as opposed to a shallow module) is one which encapsulates a lot of functionality in a simple, testable interface which rarely changes.

Check with the user that these modules match their expectations. Check with the user which modules they want tests written for.

3. Write the PRD using the template below, then write it locally as `PRD.md` through the Codex1 artifact workflow. Do not publish it anywhere.

<prd-template>

## Problem Statement

The problem that the user is facing, from the user's perspective.

## Solution

The solution to the problem, from the user's perspective.

## User Stories

A LONG, numbered list of user stories. Each user story should be in the format of:

1. As an <actor>, I want a <feature>, so that <benefit>

<user-story-example>
1. As a mobile bank customer, I want to see balance on my accounts, so that I can make better informed decisions about my spending
</user-story-example>

This list of user stories should be extremely extensive and cover all aspects of the feature.

## Implementation Decisions

A list of implementation decisions that were made. This can include:

- The modules that will be built/modified
- The interfaces of those modules that will be modified
- Technical clarifications from the developer
- Architectural decisions
- Schema changes
- API contracts
- Specific interactions

Do NOT include specific file paths or code snippets. They may end up being outdated very quickly.

Exception: if a prototype produced a snippet that encodes a decision more precisely than prose can (state machine, reducer, schema, type shape), inline it within the relevant decision and note briefly that it came from a prototype. Trim to the decision-rich parts — not a working demo, just the important bits.

## Testing Decisions

A list of testing decisions that were made. Include:

- A description of what makes a good test (only test external behavior, not implementation details)
- Which modules will be tested
- Prior art for the tests (i.e. similar types of tests in the codebase)

## Out of Scope

A description of the things that are out of scope for this PRD.

## Further Notes

Any further notes about the feature.

</prd-template>
"#
}

fn plan_skill_body() -> &'static str {
    r#"---
name: plan
description: Design an executable Codex1 mission from PRD.md, including research, specs, vertical subplans, and a native goal brief. Use after create-prd; do not create issue-tracker tickets.
---

# Plan

Use this after `PRD.md` exists. Planning turns the PRD into an executable route. It is not execution and not a project-management exercise.

Ask the user only when a product, scope, UX, credential, or human-judgment decision is missing. Do not ask the user to decide technical dependency ordering, slice granularity, parallelization, test placement, or other planning mechanics that Codex can infer from the repo.

Do not stop at phases, waves, or workstreams. `PLAN.md` must preserve the execution spine: outcome contract, implementation shape, execution order, ready subplans, proof strategy, risks/non-goals, and unresolved human decisions if any.

Read `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, and `docs/agents/codex1-artifact-briefs.md` if present. Read [ADR-FORMAT.md](ADR-FORMAT.md) before writing ADRs, [SUBPLAN-BRIEF.md](SUBPLAN-BRIEF.md) before writing ready subplans, and [GOAL-BRIEF-FORMAT.md](GOAL-BRIEF-FORMAT.md) before writing `GOAL_BRIEF.md`.

## Process

1. Read `PRD.md` first. Treat it as the outcome contract.
2. Inspect repo context before planning: tests, docs, domain glossary, ADRs, prior mission artifacts, and relevant code.
3. Restate the outcome contract: what must be true, what is out of scope, and what proof will matter.
4. Decide whether research is needed. If uncertainty affects architecture, product behavior, verification, or external APIs, create `RESEARCH_PLAN.md` and record research before finalizing the plan.
5. Identify the implementation shape: existing patterns, likely deep modules, needed contracts, risk areas, and whether architecture thinking is only a planning lens or a dedicated refactor mission.
6. Create ADRs in `ADRS/` when planning makes or preserves a durable architecture decision, chooses between plausible alternatives, rejects a tempting approach for a load-bearing reason, or changes a previous architectural direction. Use [ADR-FORMAT.md](ADR-FORMAT.md) and keep ADRs lightweight unless the decision needs structure.
7. Create specs for bounded contracts where implementation needs more precision than the PRD.
8. Break work into tracer-bullet vertical slices. Each slice cuts end-to-end through the smallest behavior path that can be reviewed, tested, and proven independently.
9. Assign an `Execution Lane` to every ready subplan: `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`. Use `standard` for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial.
10. Write the execution order. Use simple serial order by default. Add parallel-safe groups only when they are obvious and useful. This is guidance, not a dependency graph engine.
11. Mark each slice as `AFK` or `HITL`. `AFK` means an agent can execute from artifacts without more human decisions. `HITL` means a human decision, design review, credential, or manual judgment is still required.
12. Put only fully specified AFK slices in `SUBPLANS/ready/`. Keep HITL work out of ready execution; use `SUBPLANS/paused/` only when a durable placeholder is useful.
13. Define proof for every executable slice: tests, commands, screenshots, logs, manual checks, review evidence, or accepted-risk records.
14. Write `GOAL_BRIEF.md` as a native goal brief that helps Codex create or refine the actual `/goal` objective.

## Artifacts

- `PLAN.md`: outcome contract, implementation shape, execution order, parallelization notes when useful, ready subplans, proof strategy, risks, and human decisions if any.
- `RESEARCH_PLAN.md`: research questions, sources, experiments, expected outputs, stopping criteria, and how findings affect the plan.
- `RESEARCH/`: durable research records with sources, facts, experiments, uncertainty, options, and recommendations.
- `ADRS/`: durable architecture decisions with context, decision, options considered, tradeoffs, consequences, and links to PRD/plan/specs.
- `SPECS/`: implementation contracts for bounded areas.
- `SUBPLANS/ready/`: executable vertical slices that require no further user decisions.
- `GOAL_BRIEF.md`: a native goal brief the user or Codex may use to create or refine the real `/goal` objective.

## Subplan Quality Bar

Every ready subplan is an agent brief. Use [SUBPLAN-BRIEF.md](SUBPLAN-BRIEF.md). It must be durable even if files move, and must include:

- slice type: AFK unless already resolved HITL work has become executable
- execution lane: one of `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`
- current behavior or current repo state
- desired behavior after the slice
- key interfaces, stable types, commands, artifacts, or contracts
- exact in-scope and out-of-scope work
- dependencies and blocked-by relationships
- worker/subagent ownership rules when useful
- concrete acceptance criteria
- required proof and where to record it
- exit criteria that leave the repo working

Do not reference line numbers. Avoid file paths unless they name stable artifacts such as `PRD.md`, `PLAN.md`, or `SUBPLANS/ready/`. Prefer behavior and interfaces over procedural instructions.

## Goal Brief Requirements

Use [GOAL-BRIEF-FORMAT.md](GOAL-BRIEF-FORMAT.md). The goal brief is not native goal state, not a file-loading instruction, and not a sacred final prompt. It must not say to read `GOAL_BRIEF.md` as the first execution step. It should give Codex enough context to create or refine a strong whole-mission native goal.

- mission path
- primary artifacts to read
- execution order
- subplan selection rules
- worker/subagent rules when useful
- editable scope
- proof recording rules
- review and triage rules
- explicit completion criteria
- non-completion behavior
- closeout rules
- prohibited actions

Completion criteria are only completion criteria. Do not put pause, escalation, or "ask the user" criteria under completion. The `/goal` execution phase may not ask questions. If completion cannot be reached from the artifacts, the objective should instruct Codex to record non-completion evidence, accepted risks, or deferred work instead of inventing scope or asking the user.

Do not create issue-tracker tickets. Do not create, inspect, or complete native goal state. Do not treat Codex1 inspect/status/events/receipts as proof of readiness or completion. The user keeps the go moment by asking Codex to create a native goal from `GOAL_BRIEF.md` or by editing the brief before `/goal`.
"#
}

fn adr_format_body() -> &'static str {
    r#"# ADR Format

Use this when `$clarify` or `$plan` records an architecture decision.

## Location

- Repo-wide or long-lived decisions: `docs/adr/`
- Mission-specific execution decisions: `.codex1/missions/<id>/ADRS/`

Create the directory lazily only when the first ADR is needed.

## Template

```md
# {Short title of the decision}

{1-3 sentences: what context led to the decision, what was decided, and why.}
```

That can be the whole ADR. The value is recording that a decision was made and why, not filling out ceremony.

## Optional Sections

Only include these when they add genuine value:

- Status: `proposed`, `accepted`, `deprecated`, or `superseded by ADR-NNNN`
- Considered Options: rejected alternatives worth remembering
- Tradeoffs: real costs of the chosen direction
- Consequences: non-obvious effects future agents must know
- Links: related PRD, PLAN, SPECS, or subplans

## Numbering

For `docs/adr/`, use sequential names like `0001-slug.md`. Scan for the highest existing number and increment by one.

For mission `ADRS/`, use the mission's normal Codex1 artifact creation flow unless the repo has a stronger local convention.

## When To Offer Or Write An ADR

All three must be true:

1. Hard to reverse: changing later would be meaningfully costly.
2. Surprising without context: a future reader would wonder why.
3. Real trade-off: plausible alternatives existed and one was chosen for a reason.

Skip ADRs for easy-to-reverse choices, obvious implementation details, and decisions with no real alternative.

## What Qualifies

- Architectural shape
- Integration patterns between contexts
- Technology choices with lock-in
- Ownership and state boundaries
- Deliberate deviations from the obvious path
- Durable constraints not visible in code
- Non-obvious rejected alternatives
"#
}

fn context_format_body() -> &'static str {
    r#"# CONTEXT.md Format

Use this when `$clarify` resolves project language.

## Structure

```md
# {Context Name}

{One or two sentence description of what this context is and why it exists.}

## Language

**Order**:
{A concise description of the term}
_Avoid_: Purchase, transaction

**Invoice**:
A request for payment sent to a customer after delivery.
_Avoid_: Bill, payment request

## Relationships

- An **Order** produces one or more **Invoices**
- An **Invoice** belongs to exactly one **Customer**

## Example dialogue

> **Dev:** "When a **Customer** places an **Order**, do we create the **Invoice** immediately?"
> **Domain expert:** "No. An **Invoice** is generated once **Fulfillment** is confirmed."

## Flagged ambiguities

- "account" was used to mean both **Customer** and **User**. Resolved: these are distinct concepts.
```

## Rules

- Be opinionated. Pick one canonical term and list aliases to avoid.
- Flag conflicts explicitly.
- Keep definitions tight: one sentence, defining what the term is.
- Show relationships and cardinality where obvious.
- Include only domain terms, not generic programming concepts.
- Group terms under headings when natural clusters emerge.
- Write example dialogue when it clarifies boundaries.

## Single vs Multi-context Repos

Single context: one root `CONTEXT.md`.

Multi-context: root `CONTEXT-MAP.md` points to per-context `CONTEXT.md` files. If `CONTEXT-MAP.md` exists, read it and update the relevant context. If unclear, ask only when the context changes the mission.

Create context files lazily. If no `CONTEXT.md` exists, create one only when the first term is resolved.
"#
}

fn prd_format_body() -> &'static str {
    r#"# PRD Format

Use this when `$create-prd` writes `PRD.md`. This is adapted from the reference `to-prd` format, but Codex1 keeps the PRD local and does not publish to an issue tracker.

## Required Quality Bar

The PRD must be sufficient for `$plan` to design execution without reconstructing product intent. Write from the user's perspective first, then capture implementation and testing decisions.

## Template

```md
## Problem Statement

The problem the user is facing, from the user's perspective.

## Solution

The solution, from the user's perspective.

## User Stories

A long numbered list of user stories:

1. As an <actor>, I want <feature>, so that <benefit>.

Cover all major behavior, actors, edge cases, and artifact interactions.

## Success Criteria

Observable facts that make the PRD satisfied.

## Module Sketch

Likely modules, interfaces, contracts, and deep-module opportunities. Use stable names and concepts, not brittle paths.

## Implementation Decisions

- Modules that will be built or modified
- Interfaces or contracts that change
- Technical clarifications
- Architectural decisions
- Schema changes
- API contracts
- State ownership
- Specific interactions

Do not include brittle file paths or code snippets.

## Testing Decisions

- What makes a good test for this change
- External behavior to test
- Modules worth testing directly
- Prior art in the existing test suite
- Testing non-goals

Tests should verify behavior through public interfaces, not implementation details.

## Out Of Scope

What this PRD intentionally does not include.

## Proof Expectations

Commands, tests, screenshots, manual checks, review evidence, or other proof expected later.

## Review Expectations

Reviewer posture, review artifacts, triage expectations, or explicit "no special review" statement.

## Further Notes

Any useful context that does not fit above.
```

## Local-only Rule

Do not publish this PRD to GitHub Issues, Linear, Jira, GitLab, or another tracker. Write it into the Codex1 mission artifact tree.
"#
}

fn subplan_brief_body() -> &'static str {
    r#"# Subplan Brief Format

Ready subplans are agent briefs for future Codex work. They should stay useful even if code moves.

## Principles

- Durable over precise: describe behavior, interfaces, contracts, artifacts, and acceptance criteria.
- Behavioral over procedural: say what must be true, not which line to edit.
- Complete enough for AFK execution: a ready subplan should need no further user decision.
- Explicit scope boundaries: prevent adjacent gold-plating.

Avoid line numbers. Avoid file paths unless they name stable artifacts such as `PRD.md`, `PLAN.md`, `SPECS/`, `SUBPLANS/ready/`, or a durable command.

## Slice Types

- `AFK`: an agent can execute from artifacts without more human decisions.
- `HITL`: a human decision, design review, credential, visual judgment, or manual access is still required.

Only fully specified AFK work belongs in `SUBPLANS/ready/`.

## Execution Lanes

Every ready subplan must include `Execution Lane` with one allowed value:

- `tdd`: behavior-changing code that should use red-green-refactor through public interfaces
- `diagnose`: hard bug or regression work that needs a reproduce-first loop
- `improve-codebase-architecture`: architecture deepening work using modules, interfaces, seams, adapters, depth, leverage, and locality
- `prototype`: throwaway work that answers a named design, state, or UI question
- `proof-qa`: mission-scoped acceptance proof, Browser checks, screenshots, logs, manual checks, review evidence, closeout, or accepted-risk records
- `standard`: docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial

`$plan` assigns the lane. Native `/goal` executes from the subplans.

## Template

```md
## Slice Type

AFK or HITL, with one sentence explaining why.

## Execution Lane

One of `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`.

## Current Behavior

What happens now, or what repo/artifact state currently exists.

## Desired Behavior

What should be true after this slice.

## Key Interfaces

- Stable type, command, artifact, contract, or workflow the agent should understand

## Scope

- In-scope behavior or artifact work

## Out Of Scope

- Adjacent work that should not be changed

## Dependencies

- Prior slice, spec, ADR, research, credential, or human decision required

## Blocked By

- "None" or concrete blockers

## Acceptance Criteria

- [ ] Specific, testable criterion
- [ ] Specific, testable criterion

## Expected Proof

- Command, test, screenshot, manual check, log, review, or accepted-risk record

## Exit Criteria

- What lets Codex stop this slice with the repo working
```

## Tracer Bullet Rule

Each subplan should deliver the thinnest complete vertical path through the system that can be reviewed, tested, and proven independently.
"#
}

fn goal_brief_format_body() -> &'static str {
    r#"# Goal Brief Format

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective for the whole mission.

It is a brief, not native goal state and not the final authority. The brief must not tell Codex to read `GOAL_BRIEF.md` as the first execution step.

## Goal Brief Must Include

- Purpose
- Suggested goal request
- Mission path
- Primary artifacts to read
- Execution order
- Subplan selection rules
- Worker/subagent rules when useful
- Editable scope
- Proof recording rules
- Review and triage rules
- Explicit completion criteria
- If completion cannot be reached
- Closeout rules
- Prohibited actions

## Completion Criteria

Completion criteria are only completion criteria. Do not include pause, escalation, "ask the user", or "wait for clarification" criteria.

Good completion criteria are observable:

- Required ready subplans are complete or explicitly triaged not applicable.
- Required proofs exist and were audited.
- PRD success criteria are satisfied or recorded as deferred with reason.
- Closeout summarizes completed, superseded, paused, deferred, and risky work.

## No-question Execution

The `/goal` execution phase may not ask questions. If artifacts are insufficient, Codex should record non-completion evidence, blockers, accepted risks, or deferred work rather than inventing scope or asking the user.

## Worker Rules

When using workers, give each worker explicit ownership, relevant artifacts, editable scope, proof expectations, and non-goals. Workers should not edit mission-level artifacts unless assigned.

## Prohibited Actions

Always prohibit:

- Creating, inspecting, or completing native goal state from Codex1.
- Treating `codex1 inspect`, setup status, events, or receipts as completion proof.
- Creating issue-tracker tickets.
- Reading `GOAL_BRIEF.md` as the first step of the native goal.
"#
}

fn legacy_execution_prompt_format_body() -> &'static str {
    r#"# Execution Prompt Format

`EXECUTION_PROMPT.md` contains the objective text the user copies after typing native `/goal`.

It is a copy source. The prompt must not tell Codex to read `EXECUTION_PROMPT.md`; the user has already copied from it.

## Native Goal Objective Must Include

- Mission path
- Primary artifacts to read
- Execution order
- Subplan selection rules
- Worker/subagent rules when useful
- Editable scope
- Proof recording rules
- Review and triage rules
- Explicit completion criteria
- If completion cannot be reached
- Closeout rules
- Prohibited actions

## Completion Criteria

Completion criteria are only completion criteria. Do not include pause, escalation, "ask the user", or "wait for clarification" criteria.

Good completion criteria are observable:

- Required ready subplans are complete or explicitly triaged not applicable.
- Required proofs exist and were audited.
- PRD success criteria are satisfied or recorded as deferred with reason.
- Closeout summarizes completed, superseded, paused, deferred, and risky work.

## No-question Execution

The `/goal` execution phase may not ask questions. If artifacts are insufficient, Codex should record non-completion evidence, blockers, accepted risks, or deferred work rather than inventing scope or asking the user.

## Worker Rules

When using workers, give each worker explicit ownership, relevant artifacts, editable scope, proof expectations, and non-goals. Workers should not edit mission-level artifacts unless assigned.

## Prohibited Actions

Always prohibit:

- Creating, inspecting, or completing native goal state from Codex1.
- Treating `codex1 inspect`, setup status, events, or receipts as completion proof.
- Creating issue-tracker tickets.
- Reading `EXECUTION_PROMPT.md` as the first step of the pasted objective.
"#
}

fn workflow_doc_body() -> &'static str {
    r#"# Codex1 Workflow

Codex1 is a local artifact workflow, not an issue tracker and not native goal state.

## Flow

1. `$clarify` sharpens intent while questions are allowed.
2. `$create-prd` synthesizes known context into `PRD.md`.
3. `$plan` designs research, specs, ADRs, vertical subplans, and `GOAL_BRIEF.md`.
4. The user asks Codex to create or refine a native `/goal` from `GOAL_BRIEF.md`.

`GOAL_BRIEF.md` is a native goal brief. It should not instruct Codex to read itself as the first execution step.

## Core Skills And Lane Skills

Core skills shape the mission: `$codex1`, `$clarify`, `$create-prd`, and `$plan`.

Lane skills guide execution inside ready subplans: `$tdd`, `$diagnose`, `$improve-codebase-architecture`, and `$prototype`. `$plan` assigns the lane; native `/goal` executes. Use `standard` for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be fake ceremony.

## No Issue Tracker

Codex1 does not publish PRDs, issues, or plans to GitHub Issues, Linear, Jira, GitLab, or any other tracker. Durable work lives in `.codex1/missions/<id>/`.

## Native Goal Boundary

Native `/goal` owns persistent objectives, continuation, pause/resume, usage accounting, and completion. Codex1 artifacts provide context and evidence. They do not create, mirror, inspect, or complete native goals.

## Mechanical Commands

`codex1 setup`, `codex1 inspect`, events, and receipts are mechanical helpers. They are not proof of readiness, review cleanliness, PRD satisfaction, closeout, or native goal state.

## Proof/QA

Proof/QA is mission-scoped. It proves the PRD and ready subplans through tests, commands, Browser checks, screenshots, logs, manual checks, review evidence, or accepted-risk records. It is not a broad default dogfood audit of the whole app.
"#
}

fn domain_doc_body() -> &'static str {
    r#"# Codex1 Domain Docs

Use the repo's domain language before inventing vocabulary.

## Before Exploring

Read these when present:

- `CONTEXT.md` at repo root, or `CONTEXT-MAP.md` for multi-context repos.
- Repo ADRs in `docs/adr/`.
- Mission ADRs in `.codex1/missions/<id>/ADRS/`.
- Relevant specs and existing mission artifacts.

If the files do not exist, proceed silently. Do not suggest creating them upfront. Producer workflows create them lazily when terms or decisions actually crystallize.

## Glossary Rules

When `$clarify` resolves a domain term, update the relevant `CONTEXT.md` inline:

- Pick a canonical term and list aliases to avoid.
- Keep definitions tight.
- Show relationships between terms.
- Flag ambiguities explicitly.
- Include an example dialogue when it clarifies boundaries.
- Exclude generic programming terms.

If `CONTEXT-MAP.md` exists, infer the relevant context. Ask only if the context is unclear and the answer affects the mission.

## ADR Rules

Offer or write an ADR only when all three are true:

1. Hard to reverse: changing later would be meaningfully costly.
2. Surprising without context: a future reader would wonder why.
3. Real trade-off: plausible alternatives existed and one was chosen for a reason.

Repo-wide or long-lived architecture decisions belong in `docs/adr/`. Mission-specific execution decisions belong in `.codex1/missions/<id>/ADRS/`.

Keep ADRs lightweight by default: title plus one paragraph explaining context, decision, and why. Add status, options, tradeoffs, consequences, and artifact links only when they add real value.
"#
}

fn artifact_briefs_doc_body() -> &'static str {
    r#"# Codex1 Artifact Briefs

Codex1 artifacts should stay durable as code changes. Prefer behavior, interfaces, stable artifact names, and acceptance criteria over brittle paths or line numbers.

## PRD Quality

`PRD.md` should include problem statement, solution, extensive user stories, success criteria, module sketch, implementation decisions, testing decisions, out-of-scope work, proof expectations, review expectations, and PR intent.

User stories should be numbered and broad enough that `$plan` can map slices back to them.

## Subplans As Agent Briefs

Ready subplans are contracts for future Codex work. Each ready subplan should include:

- slice type: `AFK` or already-resolved `HITL`
- execution lane: `tdd`, `diagnose`, `improve-codebase-architecture`, `prototype`, `proof-qa`, or `standard`
- current behavior or repo state
- desired behavior
- key interfaces or stable contracts
- in-scope and out-of-scope work
- dependencies and blocked-by relationships
- acceptance criteria
- expected proof
- exit criteria

Write subplans as tracer bullets: thin vertical slices that deliver a complete, independently verifiable path through the system.

`standard` is the escape hatch for docs, simple config, mechanical updates, low-risk chores, and work where a specialist lane would be artificial. `$plan` assigns lanes; native `/goal` executes from them.

## Goal Brief

`GOAL_BRIEF.md` helps Codex create or refine the native `/goal` objective. The brief must include purpose, suggested goal request, mission path, primary artifacts to read, execution order, subplan selection, worker rules, editable scope, proof rules, review/triage rules, completion criteria, non-completion behavior, closeout, and prohibited actions.

Execution may not ask the user questions. If completion cannot be reached from artifacts, record non-completion evidence, accepted risks, or deferred work.

## Proof And Closeout

Proofs record commands, tests, Browser checks, screenshots, manual checks, failures, and accepted risks. Closeout is written only after auditing PRD satisfaction against proofs and reviews. Closeout does not complete native `/goal` by itself. Proof/QA proves the mission; it is not a broad default dogfood audit.
"#
}

fn guidance_body() -> &'static str {
    r#"# Codex1 Setup Guidance

codex1-managed

Codex1 is enabled in this repository as a local artifact workflow convention. Use `$clarify`, `$create-prd`, and `$plan` for the mission workflow. Read `docs/agents/codex1-workflow.md`, `docs/agents/codex1-domain.md`, and `docs/agents/codex1-artifact-briefs.md` for the repo-local workflow, domain, ADR, and artifact rules. Use `codex1` for durable mission artifacts and mechanical evidence. Use native `/goal` for persistent objectives and continuation. The preferred flow is clarify, create PRD, plan, then create or refine the native goal from `GOAL_BRIEF.md`.

Codex remains the semantic judge. Codex1 inspect, setup status, events, and receipts are not readiness, completion, review, proof, closeout, or native goal state.
"#
}

fn managed_guidance_block() -> String {
    format!(
        "{MANAGED_GUIDANCE_START}\n{}{MANAGED_GUIDANCE_END}\n",
        guidance_body()
    )
}

fn guidance_has_managed_block(text: &str) -> bool {
    text.contains(MANAGED_GUIDANCE_START) && text.contains(MANAGED_GUIDANCE_END)
}

fn replace_guidance_block(text: &str, replacement: &str) -> Option<String> {
    let start = text.find(MANAGED_GUIDANCE_START)?;
    let after_start = start + MANAGED_GUIDANCE_START.len();
    let relative_end = text[after_start..].find(MANAGED_GUIDANCE_END)?;
    let mut end = after_start + relative_end + MANAGED_GUIDANCE_END.len();
    if text[end..].starts_with("\r\n") {
        end += 2;
    } else if text[end..].starts_with('\n') {
        end += 1;
    }
    let mut edited = String::new();
    edited.push_str(&text[..start]);
    edited.push_str(replacement);
    edited.push_str(&text[end..]);
    Some(edited)
}

fn remove_guidance_block(text: &str) -> Option<String> {
    let start = text.find(MANAGED_GUIDANCE_START)?;
    let after_start = start + MANAGED_GUIDANCE_START.len();
    let relative_end = text[after_start..].find(MANAGED_GUIDANCE_END)?;
    let mut end = after_start + relative_end + MANAGED_GUIDANCE_END.len();
    if text[end..].starts_with("\r\n") {
        end += 2;
    } else if text[end..].starts_with('\n') {
        end += 1;
    }
    let mut edited = String::new();
    let mut prefix = text[..start].to_string();
    while prefix.ends_with('\n') {
        prefix.pop();
        if prefix.ends_with('\r') {
            prefix.pop();
        }
    }
    edited.push_str(&prefix);
    if !prefix.is_empty() && !text[end..].is_empty() {
        edited.push('\n');
    }
    edited.push_str(&text[end..]);
    while edited.contains("\n\n\n") {
        edited = edited.replace("\n\n\n", "\n\n");
    }
    Some(edited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guidance_block_round_trips_with_neighboring_text() {
        let text = "before\n\n<!-- codex1-managed setup guidance start -->\nold\n<!-- codex1-managed setup guidance end -->\nafter\n";
        let replacement = managed_guidance_block();
        let replaced = replace_guidance_block(text, &replacement).unwrap();
        assert!(replaced.contains("before"));
        assert!(replaced.contains("after"));
        assert!(replaced.contains("native `/goal`"));
        let removed = remove_guidance_block(&replaced).unwrap();
        assert!(removed.contains("before"));
        assert!(removed.contains("after"));
        assert!(!removed.contains("codex1-managed setup guidance start"));
    }

    #[test]
    fn marker_body_matches_expected_files() {
        let marker: BundleMarker = serde_json::from_str(&bundle_marker_body()).unwrap();
        validate_marker(&marker).unwrap();
        assert_eq!(marker.version, BUNDLE_VERSION);
        assert_eq!(marker.files, bundle_files(MANAGED_BUNDLE_FILES));
    }
}
