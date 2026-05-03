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
const BUNDLE_VERSION: u32 = 2;
const MANAGED_GUIDANCE_START: &str = "<!-- codex1-managed setup guidance start -->";
const MANAGED_GUIDANCE_END: &str = "<!-- codex1-managed setup guidance end -->";
const OVERVIEW_SKILL: &str = ".agents/skills/codex1/SKILL.md";
const CLARIFY_SKILL: &str = ".agents/skills/clarify/SKILL.md";
const CREATE_PRD_SKILL: &str = ".agents/skills/create-prd/SKILL.md";
const PLAN_SKILL: &str = ".agents/skills/plan/SKILL.md";
const BUNDLE_GUIDANCE: &str = "AGENTS.md";
const BUNDLE_MARKER: &str = ".codex1/setup-bundle.json";
const BACKUP_MANIFEST: &str = ".codex1/setup-backups/manifest.json";
const BACKUP_DIR: &str = ".codex1/setup-backups/files";
const MANAGED_SKILL_FILES: [&str; 4] =
    [OVERVIEW_SKILL, CLARIFY_SKILL, CREATE_PRD_SKILL, PLAN_SKILL];
const MANAGED_BUNDLE_FILES: [&str; 5] = [
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
    let guidance = guidance_state(&repo);
    let mut warnings = Vec::new();
    for (name, state) in [("marker", marker), ("skill", skill), ("guidance", guidance)] {
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
        guidance,
        repo_bundle_materialized: marker == SetupFileState::Current
            && skill == SetupFileState::Current
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
    let expected = if let Some(relative) = managed_skill_relative_for_path(repo, path)? {
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
    } else if states.iter().any(|state| *state == SetupFileState::Invalid) {
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
        CLARIFY_SKILL => clarify_skill_body().to_string(),
        CREATE_PRD_SKILL => create_prd_skill_body().to_string(),
        PLAN_SKILL => plan_skill_body().to_string(),
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

fn managed_skill_relative_for_path(repo: &Path, path: &Path) -> Result<Option<&'static str>> {
    for &relative in &MANAGED_SKILL_FILES {
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

Codex1 is a deterministic artifact helper for clarification context, PRD, PLAN, EXECUTION_PROMPT, SPEC, SUBPLAN, REVIEW, TRIAGE, PROOF, CLOSEOUT, receipts, and inventory inspection.

Preferred UX:

- Use `$clarify` to gather and preserve user intent while questions are still allowed.
- Use `$create-prd` to synthesize known context into `PRD.md`.
- Use `$plan` to design the mission and write `EXECUTION_PROMPT.md`.
- The user manually starts a new Codex CLI session, types `/goal`, and pastes the generated objective.

Native Codex `/goal` owns persistent objectives, continuation, pause/resume, accounting, budgets, and completion. Codex1 must not create, mirror, inspect, or complete native goals.

Codex1 setup is mechanical repo guidance. It is not mission truth, task readiness, review pass/fail, proof sufficiency, close safety, or native goal state.
"#
}

fn clarify_skill_body() -> &'static str {
    r#"---
name: clarify
description: Gather and preserve the user's intent for a Codex1 mission before PRD synthesis. Use when the user wants to explore, clarify, or write-me-docs for a future mission.
---

# Clarify

Use this skill before PRD creation. Your job is to turn rough intent into clear context, not to execute.

Ask questions when ambiguity matters. Challenge assumptions when the user's idea is underspecified. Preserve the resolved understanding, constraints, open questions, examples, references, desired outcomes, non-goals, and proof expectations.

Do not start implementation. Do not create or complete native `/goal` state. Do not treat clarification notes as mission truth; they are inputs for `$create-prd`.

When enough context is available, summarize it in a durable shape that `$create-prd` can synthesize into `PRD.md`.
"#
}

fn create_prd_skill_body() -> &'static str {
    r#"---
name: create-prd
description: Synthesize the current conversation, clarification output, repo context, and user references into a Codex1 PRD artifact without re-interviewing by default.
---

# Create PRD

Use this skill after clarification or whenever the user asks Codex to create a PRD from known context.

Do not interview the user by default. Read the available conversation context, clarification notes, repo context, and user-provided references. Inspect the repository when it helps ground the PRD.

Write `PRD.md` through the Codex1 PRD artifact workflow. The PRD should capture the original request, interpreted destination, success criteria, non-goals, constraints, verified context, assumptions, resolved questions, proof expectations, review expectations, and PR intent.

Do not start implementation. Do not create or complete native `/goal` state. If important information is missing, record the assumption or open question in the PRD instead of blocking.
"#
}

fn plan_skill_body() -> &'static str {
    r#"---
name: plan
description: Design a Codex1 mission from PRD.md, create planning artifacts, and write the pasteable native /goal objective in EXECUTION_PROMPT.md.
---

# Plan

Use this skill after `PRD.md` exists. Read the PRD first, then inspect the repository and existing mission artifacts as needed.

Create or update planning artifacts through Codex1's deterministic artifact workflow:

- `PLAN.md` for strategy, workstreams, phases, risks, and recommended slices.
- `RESEARCH_PLAN.md` and `RESEARCH/` records when uncertainty needs durable research.
- `SPECS/` for bounded implementation contracts.
- `SUBPLANS/ready/` for executable slices.
- `EXECUTION_PROMPT.md` for the native `/goal` objective the user may review, edit, and paste.

The execution prompt is the objective text, not a file-loading instruction. It must tell Codex the mission path, primary artifacts to read, execution order, subplan selection rules, worker/subagent rules when useful, editable scope, proof rules, review/triage rules, explicit completion criteria, non-completion behavior, closeout rules, and prohibited actions.

The `/goal` execution phase may not ask questions. Clarification belongs before PRD creation and planning. If completion cannot be reached from the artifacts, the objective should instruct Codex to record non-completion rather than invent scope or ask the user.

Do not create, inspect, or complete native goal state. The user keeps the go moment by manually starting a new Codex CLI session, typing `/goal`, and pasting the generated objective.
"#
}

fn guidance_body() -> &'static str {
    r#"# Codex1 Setup Guidance

codex1-managed

Codex1 is enabled in this repository as a local artifact workflow convention. Use `$clarify`, `$create-prd`, and `$plan` for the mission workflow. Use `codex1` for durable mission artifacts and mechanical evidence. Use native `/goal` for persistent objectives and continuation. The preferred flow is clarify, create PRD, plan, then manually paste the generated execution objective after `/goal`.

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
