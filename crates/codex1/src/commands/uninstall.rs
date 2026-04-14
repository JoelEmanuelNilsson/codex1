use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::{UninstallArgs, resolve_repo_root};
use crate::support_surface::{
    AGENTS_BLOCK_BEGIN, AGENTS_BLOCK_END, AgentsScaffoldStatus, inspect_agents_scaffold_details,
};
const CONFIG_MODEL: &str = "gpt-5.4";
const CONFIG_REVIEW_MODEL: &str = "gpt-5.4-mini";
const CONFIG_REASONING_EFFORT: &str = "high";
const CONFIG_FAST_PARALLEL_MODEL: &str = "gpt-5.3-codex-spark";
const CONFIG_FAST_PARALLEL_REASONING_EFFORT: &str = "high";
const CONFIG_HARD_CODING_MODEL: &str = "gpt-5.3-codex";
const CONFIG_HARD_CODING_REASONING_EFFORT: &str = "xhigh";

#[derive(Debug, Serialize)]
pub struct UninstallReport {
    pub repo_root: String,
    pub backup_id: String,
    pub removed_paths: Vec<PathOutcome>,
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

#[derive(Debug)]
enum PlannedUninstallAction {
    WriteFile {
        path: PathBuf,
        contents: String,
        label: &'static str,
    },
    DeleteFile {
        path: PathBuf,
        label: &'static str,
    },
    Noop {
        path: PathBuf,
        label: &'static str,
    },
}

#[derive(Debug, Clone)]
struct RollbackSnapshot {
    path: PathBuf,
    previous_contents: Option<String>,
}

impl PlannedUninstallAction {
    fn path(&self) -> &Path {
        match self {
            PlannedUninstallAction::WriteFile { path, .. }
            | PlannedUninstallAction::DeleteFile { path, .. }
            | PlannedUninstallAction::Noop { path, .. } => path,
        }
    }
}

pub fn run(args: UninstallArgs) -> Result<()> {
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

    let mut planned = Vec::new();
    for entry in &manifest.paths {
        if !entry.applied {
            planned.push(PlannedUninstallAction::Noop {
                path: resolve_manifest_repo_path(&repo_root, &entry.path)?,
                label: "skipped_unapplied_entry",
            });
            continue;
        }
        planned.push(plan_uninstall_action(&repo_root, &backup_root, entry)?);
    }

    let mut removed_paths = Vec::new();
    let mut rollback_snapshots = Vec::new();
    let mut failures = 0_usize;
    let mut apply_failure = None::<String>;
    for action in planned {
        if apply_failure.is_some() {
            removed_paths.push(PathOutcome {
                path: action.path().display().to_string(),
                action: "skipped_after_failure".to_string(),
                error: None,
            });
            continue;
        }
        let rollback_snapshot = match snapshot_current_path(action.path()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                failures += 1;
                apply_failure = Some(error.to_string());
                removed_paths.push(PathOutcome {
                    path: action.path().display().to_string(),
                    action: "failed_prepare_rollback".to_string(),
                    error: apply_failure.clone(),
                });
                continue;
            }
        };
        match action {
            PlannedUninstallAction::WriteFile {
                path,
                contents,
                label,
            } => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("create parent directory {}", parent.display()))?;
                }
                if let Err(error) =
                    fs::write(&path, contents).with_context(|| format!("write {}", path.display()))
                {
                    failures += 1;
                    apply_failure = Some(error.to_string());
                    removed_paths.push(PathOutcome {
                        path: path.display().to_string(),
                        action: "failed_write_file".to_string(),
                        error: apply_failure.clone(),
                    });
                    continue;
                }
                removed_paths.push(PathOutcome {
                    path: path.display().to_string(),
                    action: label.to_string(),
                    error: None,
                });
                rollback_snapshots.push(rollback_snapshot);
            }
            PlannedUninstallAction::DeleteFile { path, label } => {
                if path.exists() {
                    if let Err(error) =
                        fs::remove_file(&path).with_context(|| format!("delete {}", path.display()))
                    {
                        failures += 1;
                        apply_failure = Some(error.to_string());
                        removed_paths.push(PathOutcome {
                            path: path.display().to_string(),
                            action: "failed_delete_file".to_string(),
                            error: apply_failure.clone(),
                        });
                        continue;
                    }
                    rollback_snapshots.push(rollback_snapshot);
                }
                removed_paths.push(PathOutcome {
                    path: path.display().to_string(),
                    action: label.to_string(),
                    error: None,
                });
            }
            PlannedUninstallAction::Noop { path, label } => removed_paths.push(PathOutcome {
                path: path.display().to_string(),
                action: label.to_string(),
                error: None,
            }),
        }
    }
    if apply_failure.is_some() {
        rollback_applied_changes(&rollback_snapshots, &mut removed_paths, &mut failures);
    } else {
        prune_empty_skill_dirs(&repo_root, &manifest)?;
        for entry in &mut manifest.paths {
            entry.applied = false;
        }
        write_manifest(&manifest_path, &manifest)?;
    }

    let report = UninstallReport {
        repo_root: repo_root.display().to_string(),
        backup_id: manifest.backup_id,
        removed_paths,
    };

    emit_report(args.common.json, &report, render_uninstall_report(&report))?;
    if failures > 0 {
        bail!("uninstall could not remove {failures} path(s) exactly");
    }
    Ok(())
}

fn plan_uninstall_action(
    repo_root: &Path,
    backup_root: &Path,
    entry: &ManifestPathEntry,
) -> Result<PlannedUninstallAction> {
    let path = resolve_manifest_repo_path(repo_root, &entry.path)?;
    let current_contents = read_optional_string(&path)?;

    match current_contents {
        None => {
            if entry.change_kind == "created" {
                return Ok(PlannedUninstallAction::Noop {
                    path,
                    label: "already_absent",
                });
            }
            return plan_restore_driven_uninstall(backup_root, entry, path);
        }
        Some(current_contents) => {
            let current_hash = content_hash(&current_contents);
            if entry.after_hash.as_deref() == Some(current_hash.as_str()) {
                return plan_restore_driven_uninstall(backup_root, entry, path);
            }

            match entry.component.as_str() {
                "project_config" => {
                    assert_managed_config_stable(&current_contents)?;
                    let next =
                        remove_managed_config_entries(&current_contents, &entry.managed_selector)?;
                    if next.trim().is_empty() && entry.change_kind == "created" {
                        Ok(PlannedUninstallAction::DeleteFile {
                            path,
                            label: "deleted_managed_project_config",
                        })
                    } else {
                        Ok(PlannedUninstallAction::WriteFile {
                            path,
                            contents: ensure_trailing_newline(next),
                            label: "removed_managed_config_entries",
                        })
                    }
                }
                "hooks" => {
                    let next = remove_managed_hook(&current_contents, &entry.managed_selector)?;
                    if next.trim().is_empty() && entry.change_kind == "created" {
                        Ok(PlannedUninstallAction::DeleteFile {
                            path,
                            label: "deleted_managed_hooks_file",
                        })
                    } else {
                        Ok(PlannedUninstallAction::WriteFile {
                            path,
                            contents: next,
                            label: "removed_managed_stop_hook",
                        })
                    }
                }
                "agents_md" => {
                    assert_managed_agents_block_stable(&current_contents)?;
                    let next = remove_managed_agents_block(&current_contents)?;
                    if next.trim().is_empty() && entry.change_kind == "created" {
                        Ok(PlannedUninstallAction::DeleteFile {
                            path,
                            label: "deleted_managed_agents_md",
                        })
                    } else {
                        Ok(PlannedUninstallAction::WriteFile {
                            path,
                            contents: ensure_trailing_newline(next),
                            label: "removed_managed_agents_block",
                        })
                    }
                }
                "skill_file" => bail!(
                    "{} drifted after setup; uninstall refuses to remove a managed skill file that no longer matches the installed Codex1 state",
                    entry.path
                ),
                other => bail!("unsupported manifest component {other}"),
            }
        }
    }
}

fn plan_restore_driven_uninstall(
    backup_root: &Path,
    entry: &ManifestPathEntry,
    path: PathBuf,
) -> Result<PlannedUninstallAction> {
    match entry.restore_action.as_str() {
        "restore_backup" => {
            let backup_path = entry.backup_path.as_deref().ok_or_else(|| {
                anyhow!("manifest entry for {} is missing backup_path", entry.path)
            })?;
            let backup_path = resolve_manifest_backup_path(backup_root, backup_path)?;
            let contents = fs::read_to_string(&backup_path)
                .with_context(|| format!("read backup copy {}", backup_path.display()))?;
            Ok(PlannedUninstallAction::WriteFile {
                path,
                contents,
                label: "restored_backup",
            })
        }
        "delete_if_created" => Ok(PlannedUninstallAction::DeleteFile {
            path,
            label: "deleted_created_file",
        }),
        other => bail!("unsupported restore action {other} for {}", entry.path),
    }
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

fn assert_managed_config_stable(current_contents: &str) -> Result<()> {
    let expectations = [
        (None, "model", CONFIG_MODEL),
        (None, "review_model", CONFIG_REVIEW_MODEL),
        (None, "model_reasoning_effort", CONFIG_REASONING_EFFORT),
        (Some("features"), "codex_hooks", "true"),
        (Some("agents"), "max_threads", "16"),
        (Some("agents"), "max_depth", "1"),
        (Some("codex1_orchestration"), "model", CONFIG_MODEL),
        (
            Some("codex1_orchestration"),
            "reasoning_effort",
            CONFIG_REASONING_EFFORT,
        ),
        (Some("codex1_review"), "model", CONFIG_REVIEW_MODEL),
        (
            Some("codex1_review"),
            "reasoning_effort",
            CONFIG_REASONING_EFFORT,
        ),
        (
            Some("codex1_fast_parallel"),
            "model",
            CONFIG_FAST_PARALLEL_MODEL,
        ),
        (
            Some("codex1_fast_parallel"),
            "reasoning_effort",
            CONFIG_FAST_PARALLEL_REASONING_EFFORT,
        ),
        (
            Some("codex1_hard_coding"),
            "model",
            CONFIG_HARD_CODING_MODEL,
        ),
        (
            Some("codex1_hard_coding"),
            "reasoning_effort",
            CONFIG_HARD_CODING_REASONING_EFFORT,
        ),
    ];

    for (section, key, expected) in expectations {
        let current = lookup_config_value(current_contents, section, key);
        if current.as_deref() != Some(expected) {
            bail!(
                "{}{} drifted after setup; uninstall refuses to delete a value that no longer matches the installed Codex1 setting",
                section
                    .map(|section| format!("{section}."))
                    .unwrap_or_default(),
                key
            );
        }
    }
    Ok(())
}

fn remove_managed_config_entries(source: &str, selector: &str) -> Result<String> {
    let mut text = source.to_string();
    for part in selector
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some(key) = part.strip_prefix("root:") {
            text = remove_config_key(&text, None, key);
        } else if let Some((section, key)) = part.split_once('.') {
            text = remove_config_key(&text, Some(section), key);
        } else {
            bail!("unsupported managed config selector {part}");
        }
    }
    Ok(cleanup_empty_sections(text))
}

fn remove_managed_config(source: &str) -> String {
    let mut text = source.to_string();
    text = remove_config_key(&text, None, "model");
    text = remove_config_key(&text, None, "review_model");
    text = remove_config_key(&text, None, "model_reasoning_effort");
    text = remove_config_key(&text, Some("features"), "codex_hooks");
    text = remove_config_key(&text, Some("agents"), "max_threads");
    text = remove_config_key(&text, Some("agents"), "max_depth");
    text = remove_config_key(&text, Some("codex1_orchestration"), "model");
    text = remove_config_key(&text, Some("codex1_orchestration"), "reasoning_effort");
    text = remove_config_key(&text, Some("codex1_review"), "model");
    text = remove_config_key(&text, Some("codex1_review"), "reasoning_effort");
    text = remove_config_key(&text, Some("codex1_fast_parallel"), "model");
    text = remove_config_key(&text, Some("codex1_fast_parallel"), "reasoning_effort");
    text = remove_config_key(&text, Some("codex1_hard_coding"), "model");
    text = remove_config_key(&text, Some("codex1_hard_coding"), "reasoning_effort");
    cleanup_empty_sections(text)
}

fn remove_config_key(source: &str, section: Option<&str>, key: &str) -> String {
    let mut lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();
    match section {
        None => {
            let stop = lines
                .iter()
                .position(|line| is_section_header(line).is_some())
                .unwrap_or(lines.len());
            lines = lines
                .into_iter()
                .enumerate()
                .filter_map(|(index, line)| {
                    if index < stop && key_matches(&line, key) {
                        None
                    } else {
                        Some(line)
                    }
                })
                .collect();
        }
        Some(target_section) => {
            let mut in_section = false;
            let mut filtered = Vec::with_capacity(lines.len());
            for line in lines {
                if let Some(section_name) = is_section_header(&line) {
                    in_section = section_name == target_section;
                    filtered.push(line);
                    continue;
                }
                let dotted_key = format!("{target_section}.{key}");
                if key_matches(&line, &dotted_key) {
                    continue;
                }
                if in_section && key_matches(&line, key) {
                    continue;
                }
                filtered.push(line);
            }
            lines = filtered;
        }
    }
    lines.join("\n")
}

fn cleanup_empty_sections(source: String) -> String {
    let lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();
    let mut output = Vec::new();
    let mut index = 0_usize;
    while index < lines.len() {
        if let Some(_) = is_section_header(&lines[index]) {
            let start = index;
            index += 1;
            while index < lines.len() && is_section_header(&lines[index]).is_none() {
                index += 1;
            }
            let body = &lines[start + 1..index];
            let has_content = body.iter().any(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            });
            if has_content {
                output.extend_from_slice(&lines[start..index]);
            }
        } else {
            output.push(lines[index].clone());
            index += 1;
        }
    }
    output.join("\n")
}

fn lookup_config_value(source: &str, section: Option<&str>, key: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    match section {
        None => {
            let stop = lines
                .iter()
                .position(|line| is_section_header(line).is_some())
                .unwrap_or(lines.len());
            for line in &lines[..stop] {
                if let Some(value) = parse_key_value(line, key) {
                    return Some(value);
                }
            }
            None
        }
        Some(target) => {
            let mut in_section = false;
            for line in lines {
                if let Some(section_name) = is_section_header(line) {
                    in_section = section_name == target;
                    continue;
                }
                let dotted_key = format!("{target}.{key}");
                if let Some(value) = parse_key_value(line, &dotted_key) {
                    return Some(value);
                }
                if in_section {
                    if let Some(value) = parse_key_value(line, key) {
                        return Some(value);
                    }
                }
            }
            None
        }
    }
}

fn parse_key_value(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let (candidate, value) = trimmed.split_once('=')?;
    if candidate.trim() != key {
        return None;
    }
    Some(value.trim().trim_matches('"').to_string())
}

fn key_matches(line: &str, key: &str) -> bool {
    parse_key_value(line, key).is_some()
}

fn is_section_header(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.starts_with("[[") || !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    Some(trimmed.trim_start_matches('[').trim_end_matches(']'))
}

fn remove_managed_hook(source: &str, selector: &str) -> Result<String> {
    let mut root = serde_json::from_str::<Value>(source).context("parse hooks.json")?;
    let root_object = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("hooks.json root must be a JSON object"))?;
    let hooks_object = root_object
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow!("hooks.json is missing the hooks object"))?;
    let stop_groups = hooks_object
        .get_mut("Stop")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow!("hooks.json is missing hooks.Stop"))?;
    let managed_command = selector
        .strip_prefix("hooks.Stop.command:")
        .ok_or_else(|| anyhow!("unsupported managed hook selector {selector}"))?;

    let mut managed_found = false;
    stop_groups.retain_mut(|group| {
        if hook_command(group).as_deref() == Some(managed_command) {
            managed_found = true;
            return false;
        }
        let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
            return true;
        };
        let before = hooks.len();
        hooks.retain(|hook| hook_command(hook).as_deref() != Some(managed_command));
        if hooks.len() != before {
            managed_found = true;
        }
        !hooks.is_empty()
    });

    if !managed_found {
        bail!("the managed Stop hook no longer matches the installed Codex1 handler");
    }

    if stop_groups.is_empty() {
        hooks_object.remove("Stop");
    }
    if hooks_object.is_empty() {
        root_object.remove("hooks");
    }

    if root_object.is_empty() {
        return Ok(String::new());
    }

    serde_json::to_string_pretty(&root).context("serialize hooks.json after uninstall")
}

fn hook_command(value: &Value) -> Option<String> {
    value
        .get("command")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn assert_managed_agents_block_stable(source: &str) -> Result<()> {
    let inspection = inspect_agents_scaffold_details(Some(source));
    if inspection.status != AgentsScaffoldStatus::Present {
        bail!(
            "the managed AGENTS.md block drifted after setup; uninstall will not guess how to rewrite it"
        );
    }
    Ok(())
}

fn remove_managed_agents_block(source: &str) -> Result<String> {
    let begin_index = source
        .find(AGENTS_BLOCK_BEGIN)
        .ok_or_else(|| anyhow!("the Codex1 AGENTS.md block is missing"))?;
    let end_index = source
        .find(AGENTS_BLOCK_END)
        .ok_or_else(|| anyhow!("the Codex1 AGENTS.md block is missing its end marker"))?;
    if begin_index > end_index {
        bail!("the Codex1 AGENTS.md markers are out of order");
    }

    let end_index = end_index + AGENTS_BLOCK_END.len();
    let before = source[..begin_index].trim_end();
    let after = source[end_index..].trim_start_matches(['\r', '\n']);
    if before.is_empty() && after.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();
    if !before.is_empty() {
        output.push_str(before);
    }
    if !before.is_empty() && !after.is_empty() {
        output.push_str("\n\n");
    }
    if !after.is_empty() {
        output.push_str(after);
    }
    Ok(output)
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

fn read_optional_string(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn content_hash(text: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
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

fn render_uninstall_report(report: &UninstallReport) -> String {
    let mut output = String::new();
    let _ = writeln!(&mut output, "repo root: {}", report.repo_root);
    let _ = writeln!(&mut output, "backup id: {}", report.backup_id);
    let _ = writeln!(&mut output, "removed paths:");
    for path in &report.removed_paths {
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
    use super::{
        absolute_root_path, latest_manifest_path, remove_managed_agents_block,
        resolve_manifest_repo_path,
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

    #[test]
    fn removing_managed_agents_block_preserves_indented_following_content() {
        let source = concat!(
            "# Repo Instructions\n\n",
            "<!-- codex1:begin -->\nmanaged\n<!-- codex1:end -->\n\n",
            "    - keep this indentation\n"
        );

        let updated =
            remove_managed_agents_block(source).expect("managed block should be removed cleanly");
        assert!(updated.contains("\n\n    - keep this indentation\n"));
    }
}
