use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use time::OffsetDateTime;

use crate::error::{CoreError, Result};
use crate::fingerprint::Fingerprint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupScope {
    User,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupChangeKind {
    Created,
    Modified,
    Removed,
    Linked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipMode {
    FullFile,
    ManagedBlock,
    ManagedEntry,
    Symlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreAction {
    RestoreFromBackup,
    RemovePath,
    RecreateSymlink,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub scope: BackupScope,
    pub change_kind: BackupChangeKind,
    pub managed_by: String,
    pub component: String,
    pub install_mode: String,
    pub ownership_mode: OwnershipMode,
    pub managed_selector: Option<String>,
    pub origin: Option<String>,
    pub backup_path: Option<PathBuf>,
    pub before_hash: Option<Fingerprint>,
    pub after_hash: Option<Fingerprint>,
    pub restore_action: RestoreAction,
}

impl BackupEntry {
    pub fn validate(&self) -> Result<()> {
        if self.managed_by.trim().is_empty() {
            return Err(CoreError::Validation(format!(
                "backup entry `{}` is missing managed_by",
                self.path.display()
            )));
        }

        if self.component.trim().is_empty() {
            return Err(CoreError::Validation(format!(
                "backup entry `{}` is missing component",
                self.path.display()
            )));
        }

        match self.ownership_mode {
            OwnershipMode::ManagedBlock | OwnershipMode::ManagedEntry => {
                if self
                    .managed_selector
                    .as_deref()
                    .is_none_or(|selector| selector.trim().is_empty())
                {
                    return Err(CoreError::Validation(format!(
                        "backup entry `{}` needs managed_selector for shared ownership",
                        self.path.display()
                    )));
                }
            }
            OwnershipMode::FullFile | OwnershipMode::Symlink => {}
        }

        Ok(())
    }

    #[must_use]
    pub fn matches_path(&self, path: &Path) -> bool {
        self.path == path
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub backup_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub repo_root: PathBuf,
    pub codex1_version: Option<String>,
    pub paths: Vec<BackupEntry>,
}

impl BackupManifest {
    pub fn validate(&self) -> Result<()> {
        if self.backup_id.trim().is_empty() {
            return Err(CoreError::Validation(
                "backup_id must not be empty".to_owned(),
            ));
        }

        if self.paths.is_empty() {
            return Err(CoreError::Validation(
                "backup manifest must contain at least one path entry".to_owned(),
            ));
        }

        for entry in &self.paths {
            entry.validate()?;
        }

        Ok(())
    }

    #[must_use]
    pub fn entry_for_path(&self, path: &Path) -> Option<&BackupEntry> {
        self.paths.iter().find(|entry| entry.matches_path(path))
    }

    #[must_use]
    pub fn managed_paths(&self) -> Vec<&PathBuf> {
        self.paths.iter().map(|entry| &entry.path).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedManifestPathEntry {
    pub path: String,
    pub scope: String,
    pub change_kind: String,
    pub managed_by: String,
    pub component: String,
    pub install_mode: String,
    pub ownership_mode: String,
    pub managed_selector: String,
    pub origin: String,
    pub backup_path: Option<String>,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub restore_action: String,
    #[serde(default = "default_manifest_applied")]
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedBackupManifest {
    pub backup_id: String,
    pub created_at: String,
    pub repo_root: String,
    pub codex1_version: Option<String>,
    pub skill_install_mode: Option<String>,
    pub paths: Vec<ManagedManifestPathEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSurfaceRollbackSnapshot {
    pub path: PathBuf,
    pub previous_contents: Option<String>,
    pub previous_symlink_target: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportSurfaceMutationKind {
    WriteFile { contents: String },
    DeleteFile,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSurfaceMutation {
    pub path: PathBuf,
    pub manifest_index: Option<usize>,
    pub kind: SupportSurfaceMutationKind,
    pub success_label: String,
    pub failure_label: String,
    pub missing_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportSurfaceManifestMode {
    Preserve,
    MarkAppliedPerStep,
    MarkAllEntriesUnappliedOnSuccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSurfacePathOutcome {
    pub path: String,
    pub action: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSurfaceTransactionReport {
    pub outcomes: Vec<SupportSurfacePathOutcome>,
    pub failures: usize,
    pub first_error: Option<String>,
}

pub const fn default_manifest_applied() -> bool {
    true
}

pub fn support_surface_content_hash(text: &str) -> String {
    Fingerprint::from_bytes(text.as_bytes()).to_string()
}

pub fn write_managed_backup_manifest(path: &Path, manifest: &ManagedBackupManifest) -> Result<()> {
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    atomic_write_string(path, &manifest_json)
}

pub fn load_managed_backup_manifest(
    backup_root: &Path,
    backup_id: Option<&str>,
    repo_root: Option<&Path>,
) -> Result<ManagedBackupManifest> {
    let manifest_path = match backup_id {
        Some(backup_id) => backup_root
            .join(validate_backup_id_component(backup_id)?)
            .join("manifest.json"),
        None => latest_support_surface_manifest_path(backup_root, repo_root)?,
    };
    let raw = fs::read_to_string(&manifest_path)?;
    let manifest: ManagedBackupManifest = serde_json::from_str(&raw)?;
    validate_managed_backup_manifest(&manifest)?;
    Ok(manifest)
}

pub fn latest_support_surface_manifest_path(
    backup_root: &Path,
    repo_root: Option<&Path>,
) -> Result<PathBuf> {
    let mut newest: Option<(OffsetDateTime, String, PathBuf)> = None;
    for entry in fs::read_dir(backup_root)? {
        let entry = entry?;
        let path = entry.path().join("manifest.json");
        if !path.exists() {
            continue;
        }
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let manifest: ManagedBackupManifest = match serde_json::from_str(&raw) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        if validate_managed_backup_manifest(&manifest).is_err() {
            continue;
        }
        let created_at = match OffsetDateTime::parse(
            &manifest.created_at,
            &time::format_description::well_known::Rfc3339,
        ) {
            Ok(created_at) => created_at,
            Err(_) => continue,
        };
        if let Some(expected_repo_root) = repo_root {
            let manifest_repo_root = match fs::canonicalize(&manifest.repo_root) {
                Ok(path) => path,
                Err(_) => continue,
            };
            if manifest_repo_root != expected_repo_root
                || !manifest.paths.iter().any(|entry| entry.applied)
            {
                continue;
            }
        }
        match &newest {
            Some((best_created_at, best_backup_id, _))
                if created_at < *best_created_at
                    || (created_at == *best_created_at
                        && manifest.backup_id <= *best_backup_id) => {}
            _ => newest = Some((created_at, manifest.backup_id.clone(), path)),
        }
    }

    newest.map(|(_, _, path)| path).ok_or_else(|| {
        CoreError::Validation(format!(
            "no backup manifests found under {}",
            backup_root.display()
        ))
    })
}

pub fn validate_managed_backup_manifest(manifest: &ManagedBackupManifest) -> Result<()> {
    validate_backup_id_component(&manifest.backup_id)?;
    let created_at = OffsetDateTime::parse(
        &manifest.created_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|error| {
        CoreError::Validation(format!("manifest created_at must be RFC3339: {error}"))
    })?;
    let validated = BackupManifest {
        backup_id: manifest.backup_id.clone(),
        created_at,
        repo_root: PathBuf::from(&manifest.repo_root),
        codex1_version: manifest.codex1_version.clone(),
        paths: manifest
            .paths
            .iter()
            .map(validate_managed_manifest_entry)
            .collect::<Result<Vec<_>>>()?,
    };
    validated.validate()?;
    if validated
        .paths
        .iter()
        .any(|entry| matches!(entry.restore_action, RestoreAction::RecreateSymlink))
    {
        return Err(CoreError::Validation(format!(
            "backup {} uses recreate_symlink, which support-surface restore does not support yet",
            manifest.backup_id
        )));
    }
    Ok(())
}

pub fn validate_managed_manifest_entry(entry: &ManagedManifestPathEntry) -> Result<BackupEntry> {
    let validated = BackupEntry {
        path: PathBuf::from(&entry.path),
        scope: match entry.scope.as_str() {
            "project" => BackupScope::Project,
            "user" => BackupScope::User,
            other => {
                return Err(CoreError::Validation(format!(
                    "unsupported backup scope {other} for {}",
                    entry.path
                )));
            }
        },
        change_kind: match entry.change_kind.as_str() {
            "created" => BackupChangeKind::Created,
            "modified" => BackupChangeKind::Modified,
            "removed" => BackupChangeKind::Removed,
            "linked" => BackupChangeKind::Linked,
            other => {
                return Err(CoreError::Validation(format!(
                    "unsupported change_kind {other} for {}",
                    entry.path
                )));
            }
        },
        managed_by: entry.managed_by.clone(),
        component: entry.component.clone(),
        install_mode: entry.install_mode.clone(),
        ownership_mode: match entry.ownership_mode.as_str() {
            "full_file" => OwnershipMode::FullFile,
            "managed_block" => OwnershipMode::ManagedBlock,
            "managed_entry" => OwnershipMode::ManagedEntry,
            "symlink" => OwnershipMode::Symlink,
            other => {
                return Err(CoreError::Validation(format!(
                    "unsupported ownership_mode {other} for {}",
                    entry.path
                )));
            }
        },
        managed_selector: (!entry.managed_selector.trim().is_empty())
            .then_some(entry.managed_selector.clone()),
        origin: (!entry.origin.trim().is_empty()).then_some(entry.origin.clone()),
        backup_path: entry.backup_path.as_ref().map(PathBuf::from),
        before_hash: entry
            .before_hash
            .as_deref()
            .map(Fingerprint::parse)
            .transpose()?,
        after_hash: entry
            .after_hash
            .as_deref()
            .map(Fingerprint::parse)
            .transpose()?,
        restore_action: match entry.restore_action.as_str() {
            "restore_backup" => RestoreAction::RestoreFromBackup,
            "delete_if_created" => RestoreAction::RemovePath,
            "recreate_symlink" => RestoreAction::RecreateSymlink,
            "noop" => RestoreAction::Noop,
            other => {
                return Err(CoreError::Validation(format!(
                    "unsupported restore_action {other} for {}",
                    entry.path
                )));
            }
        },
    };
    if entry.applied && validated.after_hash.is_none() {
        return Err(CoreError::Validation(format!(
            "applied manifest entry {} is missing after_hash",
            entry.path
        )));
    }
    validated.validate()?;
    Ok(validated)
}

pub fn default_support_surface_backup_root() -> Result<PathBuf> {
    let home =
        env::var_os("HOME").ok_or_else(|| CoreError::Validation("HOME is not set".to_owned()))?;
    Ok(PathBuf::from(home).join(".codex1/backups"))
}

pub fn absolute_root_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(fs::canonicalize(path)?);
    }
    let base = if path.is_absolute() {
        PathBuf::new()
    } else {
        env::current_dir().map_err(CoreError::from)?
    };
    Ok(normalize_absolute_path(&base.join(path)))
}

pub fn resolve_support_surface_contained_path(
    root: &Path,
    raw_path: &str,
    scope: &str,
) -> Result<PathBuf> {
    let raw = Path::new(raw_path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        root.join(raw)
    };
    let normalized = normalize_absolute_path(&candidate);
    if !normalized.starts_with(root) {
        return Err(CoreError::Validation(format!(
            "manifest {} path {} escapes {} root {}",
            scope,
            raw_path,
            scope,
            root.display()
        )));
    }
    let mut existing = Some(normalized.as_path());
    while let Some(path) = existing {
        if path.exists() {
            let canonical = fs::canonicalize(path)?;
            if !canonical.starts_with(root) {
                return Err(CoreError::Validation(format!(
                    "manifest {} path {} resolves outside {} root {}",
                    scope,
                    raw_path,
                    scope,
                    root.display()
                )));
            }
            break;
        }
        existing = path.parent();
    }
    Ok(normalized)
}

pub fn atomic_write_string(path: &Path, contents: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        CoreError::Validation(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    let mut temp = NamedTempFile::new_in(parent)?;
    use std::io::Write as _;
    temp.write_all(contents.as_bytes())?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|error| CoreError::Io(error.error))?;
    Ok(())
}

pub fn snapshot_current_path(path: &Path) -> Result<SupportSurfaceRollbackSnapshot> {
    let previous_symlink_target =
        if path.exists() && fs::symlink_metadata(path)?.file_type().is_symlink() {
            Some(fs::read_link(path)?)
        } else {
            None
        };
    Ok(SupportSurfaceRollbackSnapshot {
        path: path.to_path_buf(),
        previous_contents: if previous_symlink_target.is_some() {
            None
        } else {
            read_optional_string(path)?
        },
        previous_symlink_target,
    })
}

pub fn restore_rollback_snapshot(snapshot: &SupportSurfaceRollbackSnapshot) -> Result<()> {
    if let Some(target) = &snapshot.previous_symlink_target {
        create_or_replace_symlink(target, &snapshot.path)?;
        return Ok(());
    }
    match &snapshot.previous_contents {
        Some(contents) => {
            if let Some(parent) = snapshot.path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&snapshot.path, contents)?;
        }
        None => {
            if snapshot.path.exists() {
                fs::remove_file(&snapshot.path)?;
            }
        }
    }
    Ok(())
}

pub fn execute_support_surface_transaction(
    manifest: &mut ManagedBackupManifest,
    manifest_path: &Path,
    mutations: &[SupportSurfaceMutation],
    manifest_mode: SupportSurfaceManifestMode,
) -> Result<SupportSurfaceTransactionReport> {
    let original_manifest = manifest.clone();
    let mut outcomes = Vec::new();
    let mut snapshots = Vec::new();
    let mut failures = 0_usize;
    let mut first_error = None::<String>;

    for mutation in mutations {
        if first_error.is_some() {
            outcomes.push(SupportSurfacePathOutcome {
                path: mutation.path.display().to_string(),
                action: "skipped_after_failure".to_string(),
                error: None,
            });
            continue;
        }

        match &mutation.kind {
            SupportSurfaceMutationKind::Noop => {
                outcomes.push(SupportSurfacePathOutcome {
                    path: mutation.path.display().to_string(),
                    action: mutation.success_label.clone(),
                    error: None,
                });
                continue;
            }
            SupportSurfaceMutationKind::WriteFile { .. }
            | SupportSurfaceMutationKind::DeleteFile => {}
        }

        let existed_before = mutation.path.exists();
        let snapshot = match snapshot_current_path(&mutation.path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                failures += 1;
                first_error = Some(error.to_string());
                outcomes.push(SupportSurfacePathOutcome {
                    path: mutation.path.display().to_string(),
                    action: "failed_prepare_rollback".to_string(),
                    error: first_error.clone(),
                });
                continue;
            }
        };

        let apply_result: Result<()> = match &mutation.kind {
            SupportSurfaceMutationKind::WriteFile { contents } => {
                atomic_write_string(&mutation.path, contents)
            }
            SupportSurfaceMutationKind::DeleteFile => {
                if mutation.path.exists() {
                    fs::remove_file(&mutation.path).map_err(CoreError::from)
                } else {
                    Ok(())
                }
            }
            SupportSurfaceMutationKind::Noop => Ok(()),
        };

        if let Err(error) = apply_result {
            failures += 1;
            first_error = Some(error.to_string());
            outcomes.push(SupportSurfacePathOutcome {
                path: mutation.path.display().to_string(),
                action: mutation.failure_label.clone(),
                error: first_error.clone(),
            });
            continue;
        }

        let action = match &mutation.kind {
            SupportSurfaceMutationKind::DeleteFile if !existed_before => mutation
                .missing_label
                .clone()
                .unwrap_or_else(|| mutation.success_label.clone()),
            _ => mutation.success_label.clone(),
        };
        outcomes.push(SupportSurfacePathOutcome {
            path: mutation.path.display().to_string(),
            action,
            error: None,
        });
        snapshots.push(snapshot);

        if manifest_mode == SupportSurfaceManifestMode::MarkAppliedPerStep {
            if let Some(index) = mutation.manifest_index {
                manifest.paths[index].applied = true;
            }
            if let Err(error) = write_managed_backup_manifest(manifest_path, manifest) {
                failures += 1;
                first_error = Some(error.to_string());
                outcomes.push(SupportSurfacePathOutcome {
                    path: manifest_path.display().to_string(),
                    action: "failed_write_manifest".to_string(),
                    error: first_error.clone(),
                });
            }
        }
    }

    if first_error.is_none()
        && manifest_mode == SupportSurfaceManifestMode::MarkAllEntriesUnappliedOnSuccess
    {
        for entry in &mut manifest.paths {
            entry.applied = false;
        }
        if let Err(error) = write_managed_backup_manifest(manifest_path, manifest) {
            failures += 1;
            first_error = Some(error.to_string());
            outcomes.push(SupportSurfacePathOutcome {
                path: manifest_path.display().to_string(),
                action: "failed_write_manifest".to_string(),
                error: first_error.clone(),
            });
        }
    }

    if first_error.is_some() {
        *manifest = original_manifest;
        let _ = write_managed_backup_manifest(manifest_path, manifest);
        rollback_support_surface_snapshots(&snapshots, &mut outcomes, &mut failures);
    }

    Ok(SupportSurfaceTransactionReport {
        outcomes,
        failures,
        first_error,
    })
}

pub fn read_optional_string(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read_to_string(path)?))
}

fn validate_backup_id_component(backup_id: &str) -> Result<&str> {
    let path = Path::new(backup_id);
    if backup_id.trim().is_empty() || path.is_absolute() || path.components().count() != 1 {
        return Err(CoreError::Validation(
            "backup_id must be a single safe backup directory name".to_owned(),
        ));
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(CoreError::Validation(
                "backup_id must be a single safe backup directory name".to_owned(),
            ));
        }
    }
    Ok(backup_id)
}

fn normalize_absolute_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn rollback_support_surface_snapshots(
    snapshots: &[SupportSurfaceRollbackSnapshot],
    outcomes: &mut Vec<SupportSurfacePathOutcome>,
    failures: &mut usize,
) {
    for snapshot in snapshots.iter().rev() {
        match restore_rollback_snapshot(snapshot) {
            Ok(()) => outcomes.push(SupportSurfacePathOutcome {
                path: snapshot.path.display().to_string(),
                action: "rolled_back_after_failure".to_string(),
                error: None,
            }),
            Err(error) => {
                *failures += 1;
                outcomes.push(SupportSurfacePathOutcome {
                    path: snapshot.path.display().to_string(),
                    action: "failed_rollback".to_string(),
                    error: Some(error.to_string()),
                });
            }
        }
    }
}

#[cfg(unix)]
fn create_or_replace_symlink(target: &Path, link: &Path) -> Result<()> {
    if link.exists()
        || fs::symlink_metadata(link)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
    {
        fs::remove_file(link)?;
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_or_replace_symlink(_target: &Path, _link: &Path) -> Result<()> {
    Err(CoreError::Validation(
        "symlink restoration is not supported on this platform".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use tempfile::TempDir;

    use time::OffsetDateTime;

    use super::{
        BackupChangeKind, BackupEntry, BackupManifest, BackupScope, ManagedBackupManifest,
        ManagedManifestPathEntry, OwnershipMode, RestoreAction, absolute_root_path,
        latest_support_surface_manifest_path, validate_managed_backup_manifest,
    };

    #[test]
    fn shared_entries_require_selector() {
        let entry = BackupEntry {
            path: PathBuf::from("/repo/.codex/hooks.json"),
            scope: BackupScope::Project,
            change_kind: BackupChangeKind::Modified,
            managed_by: "codex1".to_owned(),
            component: "hooks".to_owned(),
            install_mode: "managed_update".to_owned(),
            ownership_mode: OwnershipMode::ManagedEntry,
            managed_selector: None,
            origin: None,
            backup_path: Some(PathBuf::from("/backup/hooks.json")),
            before_hash: None,
            after_hash: None,
            restore_action: RestoreAction::RestoreFromBackup,
        };

        assert!(entry.validate().is_err());
    }

    #[test]
    fn manifest_exposes_entries_by_path() {
        let manifest = BackupManifest {
            backup_id: "2026-04-12T00-00-00Z".to_owned(),
            created_at: OffsetDateTime::now_utc(),
            repo_root: PathBuf::from("/repo"),
            codex1_version: Some("0.1.0".to_owned()),
            paths: vec![BackupEntry {
                path: PathBuf::from("/repo/AGENTS.md"),
                scope: BackupScope::Project,
                change_kind: BackupChangeKind::Modified,
                managed_by: "codex1".to_owned(),
                component: "agents".to_owned(),
                install_mode: "managed_update".to_owned(),
                ownership_mode: OwnershipMode::ManagedBlock,
                managed_selector: Some("codex1-block".to_owned()),
                origin: None,
                backup_path: Some(PathBuf::from("/backup/AGENTS.md")),
                before_hash: None,
                after_hash: None,
                restore_action: RestoreAction::RestoreFromBackup,
            }],
        };

        manifest.validate().expect("manifest should validate");
        assert!(
            manifest
                .entry_for_path(Path::new("/repo/AGENTS.md"))
                .is_some()
        );
    }

    #[test]
    fn latest_manifest_path_skips_rolled_back_manifests() {
        let repo = TempDir::new().expect("temp repo");
        let repo_root = absolute_root_path(repo.path()).expect("canonical repo root");
        let backups = TempDir::new().expect("backup root");

        let applied_dir = backups.path().join("applied");
        fs::create_dir_all(&applied_dir).expect("create applied dir");
        fs::write(
            applied_dir.join("manifest.json"),
            format!(
                "{{\"backup_id\":\"applied\",\"created_at\":\"2026-04-15T00:00:00Z\",\"repo_root\":\"{}\",\"codex1_version\":null,\"skill_install_mode\":null,\"paths\":[{{\"path\":\"{}/AGENTS.md\",\"scope\":\"project\",\"change_kind\":\"modified\",\"managed_by\":\"codex1\",\"component\":\"agents_md\",\"install_mode\":\"support_surface\",\"ownership_mode\":\"managed_block\",\"managed_selector\":\"AGENTS.md:codex1:block\",\"origin\":\"codex1 setup\",\"backup_path\":null,\"before_hash\":null,\"after_hash\":\"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\",\"restore_action\":\"restore_backup\",\"applied\":true}}]}}",
                repo_root.display(),
                repo_root.display()
            ),
        )
        .expect("write applied manifest");

        std::thread::sleep(Duration::from_millis(20));

        let rolled_back_dir = backups.path().join("rolled-back");
        fs::create_dir_all(&rolled_back_dir).expect("create rolled-back dir");
        fs::write(
            rolled_back_dir.join("manifest.json"),
            format!(
                "{{\"backup_id\":\"rolled-back\",\"created_at\":\"2026-04-15T00:00:01Z\",\"repo_root\":\"{}\",\"codex1_version\":null,\"skill_install_mode\":null,\"paths\":[{{\"path\":\"{}/AGENTS.md\",\"scope\":\"project\",\"change_kind\":\"modified\",\"managed_by\":\"codex1\",\"component\":\"agents_md\",\"install_mode\":\"support_surface\",\"ownership_mode\":\"managed_block\",\"managed_selector\":\"AGENTS.md:codex1:block\",\"origin\":\"codex1 setup\",\"backup_path\":null,\"before_hash\":null,\"after_hash\":\"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\",\"restore_action\":\"restore_backup\",\"applied\":false}}]}}",
                repo_root.display(),
                repo_root.display()
            ),
        )
        .expect("write rolled-back manifest");

        let latest = latest_support_surface_manifest_path(backups.path(), Some(&repo_root))
            .expect("select latest manifest");
        assert!(latest.ends_with("applied/manifest.json"));
    }

    #[test]
    fn validate_managed_manifest_rejects_unsupported_recreate_symlink_entries() {
        let manifest = ManagedBackupManifest {
            backup_id: "backup-1".to_owned(),
            created_at: "2026-04-15T00:00:00Z".to_owned(),
            repo_root: "/repo".to_owned(),
            codex1_version: None,
            skill_install_mode: None,
            paths: vec![ManagedManifestPathEntry {
                path: "/repo/.codex/link".to_owned(),
                scope: "project".to_owned(),
                change_kind: "linked".to_owned(),
                managed_by: "codex1".to_owned(),
                component: "hook_link".to_owned(),
                install_mode: "support_surface".to_owned(),
                ownership_mode: "symlink".to_owned(),
                managed_selector: String::new(),
                origin: "codex1 setup".to_owned(),
                backup_path: None,
                before_hash: None,
                after_hash: Some(
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_owned(),
                ),
                restore_action: "recreate_symlink".to_owned(),
                applied: true,
            }],
        };

        let error = validate_managed_backup_manifest(&manifest)
            .expect_err("recreate_symlink should fail during manifest validation");
        assert!(error.to_string().contains("recreate_symlink"));
    }
}
