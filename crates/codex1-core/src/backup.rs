use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use time::OffsetDateTime;

    use super::{
        BackupChangeKind, BackupEntry, BackupManifest, BackupScope, OwnershipMode, RestoreAction,
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
}
