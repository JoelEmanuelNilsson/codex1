use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{Codex1Error, Result};
use crate::paths::{create_dir_all_contained, ensure_contained_for_write};

use super::{catalog, guidance};

const BACKUP_MANIFEST_VERSION: u32 = 1;
const BACKUP_MANIFEST: &str = ".codex1/setup-backups/manifest.json";
const BACKUP_DIR: &str = ".codex1/setup-backups/files";

#[derive(Clone, Debug, Serialize)]
pub(super) struct SetupPlan {
    pub dry_run: bool,
    pub writes: Vec<PathBuf>,
    pub removes: Vec<PathBuf>,
    pub backups: Vec<PathBuf>,
    pub materialized: Vec<PathBuf>,
}

impl SetupPlan {
    pub(super) fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            writes: Vec::new(),
            removes: Vec::new(),
            backups: Vec::new(),
            materialized: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct BackupManifest {
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
pub(super) struct BackupRecord {
    pub id: String,
    pub timestamp: String,
    pub target_kind: String,
    pub target_path: PathBuf,
    pub target_path_label: String,
    pub backup_path: Option<PathBuf>,
    pub existed: bool,
    pub reason: String,
}

pub(super) fn ensure_owned_file_writable(
    path: &Path,
    body: &str,
    allow_repair: bool,
) -> Result<()> {
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

pub(super) fn write_owned_file(
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
        write_text_contained(repo, path, body)?;
    }
    Ok(())
}

pub(super) fn write_guidance_file(
    repo: &Path,
    path: &Path,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    ensure_setup_target(repo, path)?;
    let block = guidance::managed_block();
    let next = match fs::read_to_string(path) {
        Ok(existing) if existing == guidance::body() || existing.contains(&block) => return Ok(()),
        Ok(existing) if guidance::has_managed_block(&existing) => {
            guidance::replace_block(&existing, &block).ok_or_else(|| {
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
        write_text_contained(repo, path, &next)?;
    }
    Ok(())
}

pub(super) fn remove_owned_file_if_managed(
    repo: &Path,
    path: &Path,
    relative: &str,
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
    if !catalog::is_managed_restore_body(relative, &text) {
        if strict {
            return Err(Codex1Error::SetupBundle(format!(
                "refusing to remove modified setup file {}",
                path.display()
            )));
        }
        return Ok(());
    }
    remove_file_with_backup(repo, path, "remove managed setup file", plan, dry_run)
}

pub(super) fn remove_guidance_if_owned(
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
    let next = if text == guidance::body() || text == guidance::managed_block() {
        None
    } else {
        let Some(edited) = guidance::remove_block(&text) else {
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
            Some(edited) if !edited.trim().is_empty() => write_text_contained(repo, path, &edited)?,
            _ => fs::remove_file(path).map_err(|source| {
                Codex1Error::SetupBundle(format!("failed to remove {}: {source}", path.display()))
            })?,
        }
    }
    Ok(())
}

pub(super) fn remove_bundle_marker_file(
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
    let marker: catalog::BundleMarker = serde_json::from_str(&text).map_err(|source| {
        Codex1Error::SetupBundle(format!("failed to parse {}: {source}", path.display()))
    })?;
    if !catalog::is_managed_bundle_marker(&marker) {
        return Err(Codex1Error::SetupBundle(format!(
            "refusing to remove non-managed marker {}",
            path.display()
        )));
    }
    remove_file_with_backup(repo, path, "remove managed setup marker", plan, dry_run)
}

pub(super) fn remove_file_with_backup(
    repo: &Path,
    path: &Path,
    reason: &str,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    backup_target(repo, path, reason, plan, dry_run)?;
    plan.removes.push(path.to_path_buf());
    if !dry_run {
        fs::remove_file(path).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to remove {}: {source}", path.display()))
        })?;
    }
    Ok(())
}

pub(super) fn backup_target(
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

pub(super) fn read_manifest(repo: &Path) -> Result<BackupManifest> {
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

pub(super) fn setup_target(repo: &Path, relative: impl AsRef<Path>) -> Result<PathBuf> {
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

pub(super) fn ensure_restore_target(repo: &Path, path: &Path) -> Result<()> {
    ensure_setup_target(repo, path).map_err(|error| {
        Codex1Error::SetupRestore(format!(
            "invalid restore target {}: {error}",
            path.display()
        ))
    })?;
    for relative in catalog::managed_restore_files() {
        if path == setup_target(repo, relative)? {
            return Ok(());
        }
    }
    Err(Codex1Error::SetupRestore(format!(
        "backup target is not a managed setup file: {}",
        path.display()
    )))
}

pub(super) fn ensure_backup_file(repo: &Path, path: &Path) -> Result<()> {
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

pub(super) fn copy_backup_to_target(repo: &Path, backup_path: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        create_dir_all_contained(repo, parent.strip_prefix(repo).unwrap())?;
    }
    fs::copy(backup_path, target).map_err(|source| {
        Codex1Error::SetupRestore(format!(
            "failed to restore {} from {}: {source}",
            target.display(),
            backup_path.display()
        ))
    })?;
    Ok(())
}

pub(super) fn restore_absence(repo: &Path, path: &Path, dry_run: bool) -> Result<()> {
    if path == setup_target(repo, catalog::BUNDLE_GUIDANCE)? {
        return restore_guidance_absence(path, dry_run);
    }
    let Some(relative) = managed_restore_relative_for_path(repo, path)? else {
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
    if !catalog::is_managed_restore_body(relative, &text) {
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

fn managed_restore_relative_for_path(repo: &Path, path: &Path) -> Result<Option<&'static str>> {
    for relative in catalog::managed_restore_files() {
        if path == setup_target(repo, relative)? {
            return Ok(Some(relative));
        }
    }
    Ok(None)
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
    if text == guidance::body() || text == guidance::managed_block() {
        if dry_run {
            return Ok(());
        }
        return fs::remove_file(path).map_err(|source| {
            Codex1Error::SetupRestore(format!("failed to remove {}: {source}", path.display()))
        });
    }
    let Some(edited) = guidance::remove_block(&text) else {
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

pub(super) fn read_bundle_marker(path: &Path) -> Result<Option<catalog::BundleMarker>> {
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

fn write_text_contained(repo: &Path, path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all_contained(repo, parent.strip_prefix(repo).unwrap())?;
    }
    fs::write(path, text).map_err(|source| {
        Codex1Error::SetupBundle(format!("failed to write {}: {source}", path.display()))
    })
}
