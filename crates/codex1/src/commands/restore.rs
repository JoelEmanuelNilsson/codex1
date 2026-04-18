use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use codex1_core::{
    ManagedBackupManifest as BackupManifest, ManagedManifestPathEntry as ManifestPathEntry,
    SupportSurfaceManifestMode, SupportSurfaceMutation, SupportSurfaceMutationKind,
    absolute_root_path as core_absolute_root_path,
    default_support_surface_backup_root as core_default_backup_root,
    execute_support_surface_transaction, load_managed_backup_manifest as core_load_manifest,
    read_optional_string as core_read_optional,
    resolve_support_surface_contained_path as core_resolve_contained_path,
    support_surface_content_hash as core_content_hash,
};
#[cfg(test)]
use codex1_core::{
    latest_support_surface_manifest_path as core_latest_manifest_path,
    validate_managed_backup_manifest as core_validate_manifest,
};
use serde::Serialize;

use crate::commands::{RestoreArgs, resolve_repo_root};

#[derive(Debug, Serialize)]
pub struct RestoreReport {
    pub repo_root: String,
    pub backup_id: String,
    pub restored_paths: Vec<PathOutcome>,
}

#[derive(Debug, Serialize)]
pub struct PathOutcome {
    pub path: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn run(args: RestoreArgs) -> Result<()> {
    let repo_root = resolve_repo_root(args.common.repo_root.as_deref())?;
    let user_root = codex_home_root()?;
    let backup_root = match args.backup_root {
        Some(path) => path,
        None => default_backup_root()?,
    };
    let backup_root = absolute_root_path(&backup_root)?;
    let mut manifest = load_manifest(
        &backup_root,
        args.backup_id.as_deref(),
        &repo_root,
        &user_root,
    )?;
    let manifest_path = backup_root.join(&manifest.backup_id).join("manifest.json");

    let manifest_repo_root = fs::canonicalize(&manifest.repo_root)
        .with_context(|| format!("canonicalize manifest repo root {}", manifest.repo_root))?;
    if manifest_repo_root != repo_root && manifest_repo_root != user_root {
        bail!(
            "backup {} belongs to {}, not project {} or Codex home {}",
            manifest.backup_id,
            manifest_repo_root.display(),
            repo_root.display(),
            user_root.display()
        );
    }

    let mut restored_paths = Vec::new();
    let mut transaction_mutations = Vec::new();
    let mut failures = 0_usize;
    let mut preflight_failed = false;
    for (index, entry) in manifest.paths.iter().enumerate() {
        if !entry.applied {
            restored_paths.push(PathOutcome {
                path: entry.path.clone(),
                action: "skipped_unapplied_entry".to_string(),
                error: None,
            });
            continue;
        }
        let target_path = resolve_manifest_target_path(&repo_root, &user_root, entry)?;
        if let Err(error) = assert_restore_safe(entry, &target_path) {
            failures += 1;
            preflight_failed = true;
            restored_paths.push(PathOutcome {
                path: entry.path.clone(),
                action: "failed_safety_check".to_string(),
                error: Some(error.to_string()),
            });
            continue;
        }
        if entry.restore_action == "restore_backup" {
            let backup_path = entry.backup_path.as_deref().ok_or_else(|| {
                anyhow!("manifest entry for {} is missing backup_path", entry.path)
            })?;
            let backup_path = resolve_manifest_backup_path(&backup_root, backup_path)?;
            if let Err(error) = fs::read_to_string(&backup_path)
                .with_context(|| format!("read backup copy {}", backup_path.display()))
            {
                failures += 1;
                preflight_failed = true;
                restored_paths.push(PathOutcome {
                    path: entry.path.clone(),
                    action: "failed_read_backup".to_string(),
                    error: Some(error.to_string()),
                });
                continue;
            }
            transaction_mutations.push(SupportSurfaceMutation {
                path: target_path.clone(),
                manifest_index: Some(index),
                kind: SupportSurfaceMutationKind::WriteFile {
                    contents: fs::read_to_string(&backup_path)
                        .with_context(|| format!("read backup copy {}", backup_path.display()))?,
                },
                success_label: "restored_backup".to_string(),
                failure_label: "failed_restore_backup".to_string(),
                missing_label: None,
            });
        } else if entry.restore_action == "delete_if_created" {
            let target_path = resolve_manifest_target_path(&repo_root, &user_root, entry)?;
            transaction_mutations.push(SupportSurfaceMutation {
                path: target_path,
                manifest_index: Some(index),
                kind: SupportSurfaceMutationKind::DeleteFile,
                success_label: "deleted_created_file".to_string(),
                failure_label: "failed_delete_created_file".to_string(),
                missing_label: Some("already_absent".to_string()),
            });
        } else if entry.restore_action == "noop" {
            let target_path = resolve_manifest_target_path(&repo_root, &user_root, entry)?;
            transaction_mutations.push(SupportSurfaceMutation {
                path: target_path,
                manifest_index: Some(index),
                kind: SupportSurfaceMutationKind::Noop,
                success_label: "noop".to_string(),
                failure_label: "noop".to_string(),
                missing_label: None,
            });
        }
        restored_paths.push(PathOutcome {
            path: entry.path.clone(),
            action: "ready_to_restore".to_string(),
            error: None,
        });
    }

    if preflight_failed {
        let report = RestoreReport {
            repo_root: repo_root.display().to_string(),
            backup_id: manifest.backup_id,
            restored_paths,
        };
        emit_report(args.common.json, &report, render_restore_report(&report))?;
        bail!("restore could not restore {failures} path(s) exactly");
    }

    let transaction = execute_support_surface_transaction(
        &mut manifest,
        &manifest_path,
        &transaction_mutations,
        SupportSurfaceManifestMode::MarkAllEntriesUnappliedOnSuccess,
    )
    .context("execute support-surface restore transaction")?;
    let applied_paths = transaction
        .outcomes
        .into_iter()
        .map(|outcome| PathOutcome {
            path: outcome.path,
            action: outcome.action,
            error: outcome.error,
        })
        .collect::<Vec<_>>();
    failures += transaction.failures;
    if transaction.first_error.is_none() {
        prune_empty_skill_dirs(&repo_root, &user_root, &manifest)?;
    }

    let report = RestoreReport {
        repo_root: repo_root.display().to_string(),
        backup_id: manifest.backup_id,
        restored_paths: applied_paths,
    };

    emit_report(args.common.json, &report, render_restore_report(&report))?;
    if failures > 0 {
        bail!("restore could not restore {failures} path(s) exactly");
    }
    Ok(())
}

fn assert_restore_safe(entry: &ManifestPathEntry, target_path: &Path) -> Result<()> {
    let Some(expected_after_hash) = entry.after_hash.as_deref() else {
        bail!(
            "{} is missing after_hash; restore refuses to proceed without an installed-state hash",
            entry.path
        );
    };
    if entry.restore_action == "restore_backup"
        && fs::symlink_metadata(target_path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
    {
        bail!(
            "{} is currently a symlink; restore refuses to replace linked managed paths in-place",
            entry.path
        );
    }

    let current_contents = read_optional_string(target_path)?;
    let Some(current_contents) = current_contents else {
        if matches!(
            entry.restore_action.as_str(),
            "delete_if_created" | "restore_backup"
        ) {
            return Ok(());
        }
        bail!(
            "{} drifted after setup; restore will not overwrite a missing managed path without manual confirmation",
            entry.path
        );
    };

    let current_hash = content_hash(&current_contents);
    if current_hash != expected_after_hash {
        bail!(
            "{} drifted after setup; restore refuses to overwrite content that no longer matches the installed Codex1 state",
            entry.path
        );
    }

    Ok(())
}

fn resolve_manifest_repo_path(repo_root: &Path, raw_path: &str) -> Result<PathBuf> {
    core_resolve_contained_path(repo_root, raw_path, "repo").map_err(anyhow::Error::new)
}

fn resolve_manifest_target_path(
    repo_root: &Path,
    user_root: &Path,
    entry: &ManifestPathEntry,
) -> Result<PathBuf> {
    match entry.scope.as_str() {
        "project" => resolve_manifest_repo_path(repo_root, &entry.path),
        "user" => resolve_manifest_contained_path(user_root, &entry.path, "user"),
        other => bail!("unsupported manifest scope {other} for {}", entry.path),
    }
}

fn resolve_manifest_backup_path(backup_root: &Path, raw_path: &str) -> Result<PathBuf> {
    core_resolve_contained_path(backup_root, raw_path, "backup").map_err(anyhow::Error::new)
}

fn resolve_manifest_contained_path(root: &Path, raw_path: &str, scope: &str) -> Result<PathBuf> {
    core_resolve_contained_path(root, raw_path, scope).map_err(anyhow::Error::new)
}

fn absolute_root_path(path: &Path) -> Result<PathBuf> {
    core_absolute_root_path(path).map_err(anyhow::Error::new)
}

fn default_backup_root() -> Result<PathBuf> {
    core_default_backup_root().map_err(anyhow::Error::new)
}

fn codex_home_root() -> Result<PathBuf> {
    if let Some(explicit) = env::var_os("CODEX_HOME") {
        return absolute_root_path(Path::new(&explicit));
    }
    let home = env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    absolute_root_path(&PathBuf::from(home).join(".codex"))
}

fn load_manifest(
    backup_root: &Path,
    backup_id: Option<&str>,
    repo_root: &Path,
    user_root: &Path,
) -> Result<BackupManifest> {
    if backup_id.is_some() {
        return core_load_manifest(backup_root, backup_id, None).map_err(anyhow::Error::new);
    }
    core_load_manifest(backup_root, None, Some(repo_root))
        .or_else(|_| core_load_manifest(backup_root, None, Some(user_root)))
        .map_err(anyhow::Error::new)
}

#[cfg(test)]
fn latest_manifest_path(backup_root: &Path, repo_root: Option<&Path>) -> Result<PathBuf> {
    core_latest_manifest_path(backup_root, repo_root).map_err(anyhow::Error::new)
}

#[cfg(test)]
fn validate_manifest(manifest: &BackupManifest) -> Result<()> {
    core_validate_manifest(manifest).map_err(anyhow::Error::new)
}

fn emit_report<T>(json: bool, report: &T, human: String) -> Result<()>
where
    T: Serialize,
{
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).context("serialize report as JSON")?
        );
    } else {
        println!("{human}");
    }
    Ok(())
}

fn render_restore_report(report: &RestoreReport) -> String {
    let mut output = String::new();
    let _ = writeln!(&mut output, "repo root: {}", report.repo_root);
    let _ = writeln!(&mut output, "backup id: {}", report.backup_id);
    let _ = writeln!(&mut output, "restored paths:");
    for path in &report.restored_paths {
        if let Some(error) = path.error.as_deref() {
            let _ = writeln!(
                &mut output,
                "- {} ({}, error: {})",
                path.path, path.action, error
            );
        } else {
            let _ = writeln!(&mut output, "- {} ({})", path.path, path.action);
        }
    }
    output.trim_end().to_string()
}

fn read_optional_string(path: &Path) -> Result<Option<String>> {
    core_read_optional(path).map_err(anyhow::Error::new)
}

fn content_hash(text: &str) -> String {
    core_content_hash(text)
}

fn prune_empty_skill_dirs(
    repo_root: &Path,
    user_root: &Path,
    manifest: &BackupManifest,
) -> Result<()> {
    let mut dirs = manifest
        .paths
        .iter()
        .filter(|entry| entry.component == "skill_file")
        .map(|entry| {
            let stop_root = manifest_scope_root(repo_root, user_root, entry)?;
            let target_path = resolve_manifest_target_path(repo_root, user_root, entry)?;
            let dir = target_path
                .parent()
                .map(Path::to_path_buf)
                .with_context(|| format!("skill file {} has no parent directory", entry.path))?;
            Ok((stop_root, dir))
        })
        .collect::<Result<Vec<_>>>()?;
    dirs.sort();
    dirs.dedup();
    dirs.sort_by_key(|(_, dir)| std::cmp::Reverse(dir.components().count()));

    for (stop_root, dir) in dirs {
        prune_empty_dir_chain(&stop_root, &dir)?;
    }

    Ok(())
}

fn manifest_scope_root(
    repo_root: &Path,
    user_root: &Path,
    entry: &ManifestPathEntry,
) -> Result<PathBuf> {
    match entry.scope.as_str() {
        "project" => Ok(repo_root.to_path_buf()),
        "user" => Ok(user_root.to_path_buf()),
        other => bail!("unsupported manifest scope {other} for {}", entry.path),
    }
}

fn prune_empty_dir_chain(repo_root: &Path, start: &Path) -> Result<()> {
    let mut current = Some(start.to_path_buf());
    while let Some(dir) = current {
        if dir == repo_root {
            break;
        }
        if !dir.is_dir() {
            break;
        }
        if fs::read_dir(&dir)
            .with_context(|| format!("read directory {}", dir.display()))?
            .next()
            .is_some()
        {
            break;
        }
        fs::remove_dir(&dir)
            .with_context(|| format!("remove empty directory {}", dir.display()))?;
        current = dir.parent().map(Path::to_path_buf);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        absolute_root_path, latest_manifest_path, resolve_manifest_repo_path, validate_manifest,
    };
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn resolve_manifest_repo_path_rejects_escape() {
        let repo = TempDir::new().expect("temp repo");
        let repo_root = absolute_root_path(repo.path()).expect("canonical repo root");
        let error = resolve_manifest_repo_path(&repo_root, "../outside.txt")
            .expect_err("path escape should be rejected");
        assert!(error.to_string().contains("escapes repo root"));
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

        let latest =
            latest_manifest_path(backups.path(), Some(&repo_root)).expect("select latest manifest");
        assert!(latest.ends_with("applied/manifest.json"));
    }

    #[test]
    fn latest_manifest_path_skips_broken_repo_root_entries() {
        let repo = TempDir::new().expect("temp repo");
        let repo_root = absolute_root_path(repo.path()).expect("canonical repo root");
        let backups = TempDir::new().expect("backup root");

        let broken_dir = backups.path().join("broken");
        fs::create_dir_all(&broken_dir).expect("create broken dir");
        fs::write(
            broken_dir.join("manifest.json"),
            r#"{"backup_id":"broken","created_at":"2026-04-15T00:00:00Z","repo_root":"/definitely/missing/repo","codex1_version":null,"skill_install_mode":null,"paths":[{"path":"/definitely/missing/repo/AGENTS.md","scope":"project","change_kind":"modified","managed_by":"codex1","component":"agents_md","install_mode":"support_surface","ownership_mode":"managed_block","managed_selector":"AGENTS.md:codex1:block","origin":"codex1 setup","backup_path":null,"before_hash":null,"after_hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","restore_action":"restore_backup","applied":true}]}"#,
        )
        .expect("write broken manifest");

        let valid_dir = backups.path().join("valid");
        fs::create_dir_all(&valid_dir).expect("create valid dir");
        fs::write(
            valid_dir.join("manifest.json"),
            format!(
                "{{\"backup_id\":\"valid\",\"created_at\":\"2026-04-15T00:00:01Z\",\"repo_root\":\"{}\",\"codex1_version\":null,\"skill_install_mode\":null,\"paths\":[{{\"path\":\"{}/AGENTS.md\",\"scope\":\"project\",\"change_kind\":\"modified\",\"managed_by\":\"codex1\",\"component\":\"agents_md\",\"install_mode\":\"support_surface\",\"ownership_mode\":\"managed_block\",\"managed_selector\":\"AGENTS.md:codex1:block\",\"origin\":\"codex1 setup\",\"backup_path\":null,\"before_hash\":null,\"after_hash\":\"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\",\"restore_action\":\"restore_backup\",\"applied\":true}}]}}",
                repo_root.display(),
                repo_root.display()
            ),
        )
        .expect("write valid manifest");

        let latest =
            latest_manifest_path(backups.path(), Some(&repo_root)).expect("select latest manifest");
        assert!(latest.ends_with("valid/manifest.json"));
    }

    #[test]
    fn validate_manifest_rejects_unsupported_recreate_symlink_entries() {
        let manifest = serde_json::from_str(
            r#"{
                "backup_id":"backup-1",
                "created_at":"2026-04-15T00:00:00Z",
                "repo_root":"/repo",
                "codex1_version":null,
                "skill_install_mode":null,
                "paths":[{
                    "path":"/repo/.codex/link",
                    "scope":"project",
                    "change_kind":"linked",
                    "managed_by":"codex1",
                    "component":"hook_link",
                    "install_mode":"support_surface",
                    "ownership_mode":"symlink",
                    "managed_selector":"",
                    "origin":"codex1 setup",
                    "backup_path":null,
                    "before_hash":null,
                    "after_hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "restore_action":"recreate_symlink",
                    "applied":true
                }]
            }"#,
        )
        .expect("parse manifest");

        let error = validate_manifest(&manifest)
            .expect_err("recreate_symlink should fail during manifest validation");
        assert!(error.to_string().contains("recreate_symlink"));
    }
}
