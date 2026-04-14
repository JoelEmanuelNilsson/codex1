use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
struct BackupManifest {
    backup_id: String,
    created_at: String,
    repo_root: String,
    codex1_version: Option<String>,
    skill_install_mode: Option<String>,
    paths: Vec<ManifestPathEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestPathEntry {
    path: String,
    scope: String,
    change_kind: String,
    managed_by: String,
    component: String,
    install_mode: String,
    ownership_mode: String,
    managed_selector: String,
    origin: String,
    backup_path: Option<String>,
    before_hash: Option<String>,
    after_hash: Option<String>,
    restore_action: String,
    #[serde(default = "default_manifest_applied")]
    applied: bool,
}

#[derive(Debug, Clone)]
struct RollbackSnapshot {
    path: PathBuf,
    previous_contents: Option<String>,
}

pub fn run(args: RestoreArgs) -> Result<()> {
    let repo_root = resolve_repo_root(args.common.repo_root.as_deref())?;
    let backup_root = match args.backup_root {
        Some(path) => path,
        None => default_backup_root()?,
    };
    let backup_root = absolute_root_path(&backup_root)?;
    let mut manifest = load_manifest(&backup_root, args.backup_id.as_deref(), &repo_root)?;
    let manifest_path = backup_root.join(&manifest.backup_id).join("manifest.json");

    let manifest_repo_root = fs::canonicalize(&manifest.repo_root)
        .with_context(|| format!("canonicalize manifest repo root {}", manifest.repo_root))?;
    if manifest_repo_root != repo_root {
        bail!(
            "backup {} belongs to {}, not {}",
            manifest.backup_id,
            manifest_repo_root.display(),
            repo_root.display()
        );
    }

    let mut restored_paths = Vec::new();
    let mut failures = 0_usize;
    let mut preflight_failed = false;
    for entry in &manifest.paths {
        if !entry.applied {
            restored_paths.push(PathOutcome {
                path: entry.path.clone(),
                action: "skipped_unapplied_entry".to_string(),
                error: None,
            });
            continue;
        }
        let target_path = resolve_manifest_repo_path(&repo_root, &entry.path)?;
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

    let mut applied_paths = Vec::new();
    let mut rollback_snapshots = Vec::new();
    let mut apply_failure = None::<String>;
    for entry in &manifest.paths {
        if !entry.applied {
            applied_paths.push(PathOutcome {
                path: entry.path.clone(),
                action: "skipped_unapplied_entry".to_string(),
                error: None,
            });
            continue;
        }
        if apply_failure.is_some() {
            applied_paths.push(PathOutcome {
                path: entry.path.clone(),
                action: "skipped_after_failure".to_string(),
                error: None,
            });
            continue;
        }
        let target_path = resolve_manifest_repo_path(&repo_root, &entry.path)?;
        let rollback_snapshot = match snapshot_current_path(&target_path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                failures += 1;
                apply_failure = Some(error.to_string());
                applied_paths.push(PathOutcome {
                    path: entry.path.clone(),
                    action: "failed_prepare_rollback".to_string(),
                    error: apply_failure.clone(),
                });
                continue;
            }
        };
        match entry.restore_action.as_str() {
            "restore_backup" => {
                let backup_path = entry.backup_path.as_deref().ok_or_else(|| {
                    anyhow!("manifest entry for {} is missing backup_path", entry.path)
                })?;
                let backup_path = resolve_manifest_backup_path(&backup_root, backup_path)?;
                let contents = fs::read_to_string(&backup_path)
                    .with_context(|| format!("read backup copy {}", backup_path.display()))?;
                if let Some(parent) = target_path.parent() {
                    if let Err(error) = fs::create_dir_all(parent)
                        .with_context(|| format!("create parent directory {}", parent.display()))
                    {
                        failures += 1;
                        apply_failure = Some(error.to_string());
                        applied_paths.push(PathOutcome {
                            path: entry.path.clone(),
                            action: "failed_create_parent".to_string(),
                            error: apply_failure.clone(),
                        });
                        continue;
                    }
                }
                if let Err(error) = fs::write(&target_path, contents)
                    .with_context(|| format!("restore {}", target_path.display()))
                {
                    failures += 1;
                    apply_failure = Some(error.to_string());
                    applied_paths.push(PathOutcome {
                        path: entry.path.clone(),
                        action: "failed_restore_backup".to_string(),
                        error: apply_failure.clone(),
                    });
                    continue;
                }
                applied_paths.push(PathOutcome {
                    path: entry.path.clone(),
                    action: "restored_backup".to_string(),
                    error: None,
                });
                rollback_snapshots.push(rollback_snapshot);
            }
            "delete_if_created" => {
                if target_path.exists() {
                    if let Err(error) = fs::remove_file(&target_path)
                        .with_context(|| format!("delete {}", target_path.display()))
                    {
                        failures += 1;
                        apply_failure = Some(error.to_string());
                        applied_paths.push(PathOutcome {
                            path: entry.path.clone(),
                            action: "failed_delete_created_file".to_string(),
                            error: apply_failure.clone(),
                        });
                        continue;
                    }
                    applied_paths.push(PathOutcome {
                        path: entry.path.clone(),
                        action: "deleted_created_file".to_string(),
                        error: None,
                    });
                    rollback_snapshots.push(rollback_snapshot);
                } else {
                    applied_paths.push(PathOutcome {
                        path: entry.path.clone(),
                        action: "already_absent".to_string(),
                        error: None,
                    });
                }
            }
            other => bail!("unsupported restore action {other} for {}", entry.path),
        }
    }
    if apply_failure.is_some() {
        rollback_applied_changes(&rollback_snapshots, &mut applied_paths, &mut failures);
    } else {
        prune_empty_skill_dirs(&repo_root, &manifest)?;
        for entry in &mut manifest.paths {
            entry.applied = false;
        }
        write_manifest(&manifest_path, &manifest)?;
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
        return Ok(());
    };

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

fn snapshot_current_path(path: &Path) -> Result<RollbackSnapshot> {
    Ok(RollbackSnapshot {
        path: path.to_path_buf(),
        previous_contents: read_optional_string(path)?,
    })
}

fn resolve_manifest_repo_path(repo_root: &Path, raw_path: &str) -> Result<PathBuf> {
    resolve_manifest_contained_path(repo_root, raw_path, "repo")
}

fn resolve_manifest_backup_path(backup_root: &Path, raw_path: &str) -> Result<PathBuf> {
    resolve_manifest_contained_path(backup_root, raw_path, "backup")
}

fn resolve_manifest_contained_path(root: &Path, raw_path: &str, scope: &str) -> Result<PathBuf> {
    let raw = Path::new(raw_path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        root.join(raw)
    };
    let normalized = normalize_absolute_path(&candidate);
    if !normalized.starts_with(root) {
        bail!(
            "manifest {} path {} escapes {} root {}",
            scope,
            raw_path,
            scope,
            root.display()
        );
    }
    let mut existing = Some(normalized.as_path());
    while let Some(path) = existing {
        if path.exists() {
            let canonical = fs::canonicalize(path)
                .with_context(|| format!("canonicalize {}", path.display()))?;
            if !canonical.starts_with(root) {
                bail!(
                    "manifest {} path {} resolves outside {} root {}",
                    scope,
                    raw_path,
                    scope,
                    root.display()
                );
            }
            break;
        }
        existing = path.parent();
    }
    Ok(normalized)
}

fn absolute_root_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path).with_context(|| format!("canonicalize {}", path.display()));
    }
    let base = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().context("resolve current working directory")?
    };
    Ok(normalize_absolute_path(&base.join(path)))
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

fn restore_snapshot(snapshot: &RollbackSnapshot) -> Result<()> {
    match &snapshot.previous_contents {
        Some(contents) => {
            if let Some(parent) = snapshot.path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create parent directory {}", parent.display()))?;
            }
            fs::write(&snapshot.path, contents)
                .with_context(|| format!("rollback {}", snapshot.path.display()))?;
        }
        None => {
            if snapshot.path.exists() {
                fs::remove_file(&snapshot.path)
                    .with_context(|| format!("remove {}", snapshot.path.display()))?;
            }
        }
    }
    Ok(())
}

fn rollback_applied_changes(
    snapshots: &[RollbackSnapshot],
    outcomes: &mut Vec<PathOutcome>,
    failures: &mut usize,
) {
    for snapshot in snapshots.iter().rev() {
        match restore_snapshot(snapshot) {
            Ok(()) => outcomes.push(PathOutcome {
                path: snapshot.path.display().to_string(),
                action: "rolled_back_after_failure".to_string(),
                error: None,
            }),
            Err(error) => {
                *failures += 1;
                outcomes.push(PathOutcome {
                    path: snapshot.path.display().to_string(),
                    action: "failed_rollback".to_string(),
                    error: Some(error.to_string()),
                });
            }
        }
    }
}

fn default_backup_root() -> Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".codex1/backups"))
}

fn load_manifest(
    backup_root: &Path,
    backup_id: Option<&str>,
    repo_root: &Path,
) -> Result<BackupManifest> {
    let manifest_path = match backup_id {
        Some(backup_id) => backup_root.join(backup_id).join("manifest.json"),
        None => latest_manifest_path(backup_root, Some(repo_root))?,
    };
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read manifest {}", manifest_path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parse manifest {}", manifest_path.display()))
}

fn latest_manifest_path(backup_root: &Path, repo_root: Option<&Path>) -> Result<PathBuf> {
    let mut newest: Option<(String, String, PathBuf)> = None;
    for entry in fs::read_dir(backup_root)
        .with_context(|| format!("read backup root {}", backup_root.display()))?
    {
        let entry = entry?;
        let path = entry.path().join("manifest.json");
        if !path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("read manifest {}", path.display()))?;
        let manifest: BackupManifest = serde_json::from_str(&raw)
            .with_context(|| format!("parse manifest {}", path.display()))?;
        if let Some(expected_repo_root) = repo_root {
            let manifest_repo_root = fs::canonicalize(&manifest.repo_root).with_context(|| {
                format!("canonicalize manifest repo root {}", manifest.repo_root)
            })?;
            if manifest_repo_root != expected_repo_root
                || !manifest.paths.iter().any(|entry| entry.applied)
            {
                continue;
            }
        }
        match &newest {
            Some((best_created_at, best_backup_id, _))
                if manifest.created_at < *best_created_at
                    || (manifest.created_at == *best_created_at
                        && manifest.backup_id <= *best_backup_id) => {}
            _ => newest = Some((manifest.created_at, manifest.backup_id, path)),
        }
    }

    newest
        .map(|(_, _, path)| path)
        .ok_or_else(|| anyhow!("no backup manifests found under {}", backup_root.display()))
}

fn manifest_matches_repo(path: &Path, expected_repo_root: &Path) -> Result<bool> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read manifest {}", path.display()))?;
    let manifest: BackupManifest =
        serde_json::from_str(&raw).with_context(|| format!("parse manifest {}", path.display()))?;
    let manifest_repo_root = fs::canonicalize(&manifest.repo_root)
        .with_context(|| format!("canonicalize manifest repo root {}", manifest.repo_root))?;
    Ok(manifest_repo_root == expected_repo_root)
}

fn manifest_is_default_candidate(path: &Path, expected_repo_root: &Path) -> Result<bool> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read manifest {}", path.display()))?;
    let manifest: BackupManifest =
        serde_json::from_str(&raw).with_context(|| format!("parse manifest {}", path.display()))?;
    let manifest_repo_root = fs::canonicalize(&manifest.repo_root)
        .with_context(|| format!("canonicalize manifest repo root {}", manifest.repo_root))?;
    Ok(
        manifest_repo_root == expected_repo_root
            && manifest.paths.iter().any(|entry| entry.applied),
    )
}

fn write_manifest(path: &Path, manifest: &BackupManifest) -> Result<()> {
    let manifest_json = serde_json::to_string_pretty(manifest).context("serialize manifest")?;
    fs::write(path, manifest_json).with_context(|| format!("write manifest {}", path.display()))
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
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(
        fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
    ))
}

fn content_hash(text: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}

const fn default_manifest_applied() -> bool {
    true
}

fn prune_empty_skill_dirs(repo_root: &Path, manifest: &BackupManifest) -> Result<()> {
    let mut dirs = manifest
        .paths
        .iter()
        .filter(|entry| entry.component == "skill_file")
        .filter_map(|entry| PathBuf::from(&entry.path).parent().map(Path::to_path_buf))
        .collect::<Vec<_>>();
    dirs.sort();
    dirs.dedup();
    dirs.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));

    for dir in dirs {
        prune_empty_dir_chain(repo_root, &dir)?;
    }

    Ok(())
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
    use super::{absolute_root_path, latest_manifest_path, resolve_manifest_repo_path};
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
                "{{\"backup_id\":\"applied\",\"created_at\":\"now\",\"repo_root\":\"{}\",\"codex1_version\":null,\"skill_install_mode\":null,\"paths\":[{{\"path\":\"{}/AGENTS.md\",\"scope\":\"project\",\"change_kind\":\"modified\",\"managed_by\":\"codex1\",\"component\":\"agents_md\",\"install_mode\":\"support_surface\",\"ownership_mode\":\"managed_block\",\"managed_selector\":\"AGENTS.md:codex1:block\",\"origin\":\"codex1 setup\",\"backup_path\":null,\"before_hash\":null,\"after_hash\":null,\"restore_action\":\"restore_backup\",\"applied\":true}}]}}",
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
                "{{\"backup_id\":\"rolled-back\",\"created_at\":\"later\",\"repo_root\":\"{}\",\"codex1_version\":null,\"skill_install_mode\":null,\"paths\":[{{\"path\":\"{}/AGENTS.md\",\"scope\":\"project\",\"change_kind\":\"modified\",\"managed_by\":\"codex1\",\"component\":\"agents_md\",\"install_mode\":\"support_surface\",\"ownership_mode\":\"managed_block\",\"managed_selector\":\"AGENTS.md:codex1:block\",\"origin\":\"codex1 setup\",\"backup_path\":null,\"before_hash\":null,\"after_hash\":null,\"restore_action\":\"restore_backup\",\"applied\":false}}]}}",
                repo_root.display(),
                repo_root.display()
            ),
        )
        .expect("write rolled-back manifest");

        let latest =
            latest_manifest_path(backups.path(), Some(&repo_root)).expect("select latest manifest");
        assert!(latest.ends_with("applied/manifest.json"));
    }
}
