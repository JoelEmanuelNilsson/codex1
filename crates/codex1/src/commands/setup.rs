use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::commands::SetupArgs;
use crate::commands::resolve_repo_root;
use crate::support_surface::{
    AGENTS_BLOCK, AGENTS_BLOCK_BEGIN, AGENTS_BLOCK_END, MANAGED_STOP_HOOK_STATUS, ManagedSkillFile,
    OBSERVATIONAL_STOP_HOOK_FLAG, OBSERVATIONAL_STOP_HOOK_FLAG_CAMEL, SkillInstallMode,
    SkillSurfaceInspection, SkillSurfaceStatus, default_skill_root,
    inspect_skill_surface_with_source, is_managed_stop_handler, managed_skill_files,
    resolve_source_skills_root, summarize_stop_authority_with_observational,
    summarize_stop_handlers,
};
const CONFIG_MODEL: &str = "gpt-5.4";
const CONFIG_REVIEW_MODEL: &str = "gpt-5.4-mini";
const CONFIG_REASONING_EFFORT: &str = "high";
const CONFIG_FAST_PARALLEL_MODEL: &str = "gpt-5.3-codex-spark";
const CONFIG_FAST_PARALLEL_REASONING_EFFORT: &str = "high";
const CONFIG_HARD_CODING_MODEL: &str = "gpt-5.3-codex";
const CONFIG_HARD_CODING_REASONING_EFFORT: &str = "xhigh";

#[derive(Debug, Serialize)]
pub struct SetupReport {
    pub repo_root: String,
    pub trusted_repo: bool,
    pub backup_id: Option<String>,
    pub skill_surface_status: &'static str,
    pub skill_install_mode: Option<String>,
    pub skill_surface_root: Option<String>,
    pub changed_paths: Vec<ChangedPathReport>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChangedPathReport {
    pub path: String,
    pub change_kind: &'static str,
    pub component: &'static str,
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
struct PlannedChange {
    path: PathBuf,
    component: &'static str,
    ownership_mode: &'static str,
    managed_selector: String,
    change_kind: &'static str,
    restore_action: &'static str,
    previous_contents: Option<String>,
    next_contents: String,
}

#[derive(Debug, Clone, Copy)]
enum ConfigValue<'a> {
    Bool(bool),
    Integer(i64),
    String(&'a str),
}

impl<'a> ConfigValue<'a> {
    fn render(self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::String(value) => format!("\"{value}\""),
        }
    }
}

pub fn run(args: SetupArgs) -> Result<()> {
    let repo_root = resolve_repo_root(args.common.repo_root.as_deref())?;
    let trusted_repo = is_repo_trusted(&repo_root)?;
    let source_skill_root = resolve_source_skills_root()?;
    let skill_inspection = inspect_skill_surface_with_source(&repo_root, &source_skill_root)?;

    if !trusted_repo {
        bail!(
            "{} is not trusted by Codex. Mark it trusted first, or add [projects.\"{}\"] trust_level = \"trusted\" to ~/.codex/config.toml.",
            repo_root.display(),
            repo_root.display()
        );
    }

    let user_hooks_path = codex_home()?.join("hooks.json");
    let user_hook_authority = user_stop_hook_authority(&user_hooks_path)?;
    if user_hook_authority.authoritative() > 0 {
        bail!(
            "{} contains {} user-level authoritative Stop handler(s); supported Codex1 environments require one authoritative Stop pipeline across config layers, so remove or mark those handlers observational before running setup",
            user_hooks_path.display(),
            user_hook_authority.authoritative()
        );
    }

    let config_path = repo_root.join(".codex/config.toml");
    let hooks_path = repo_root.join(".codex/hooks.json");
    let agents_path = repo_root.join("AGENTS.md");
    let managed_hook_command = managed_hook_command()?;

    let mut planned_changes = Vec::new();

    let existing_config = read_optional_string(&config_path)?;
    let next_config = build_project_config(existing_config.as_deref());
    if existing_config.as_deref() != Some(next_config.as_str()) {
        planned_changes.push(PlannedChange {
            path: config_path,
            component: "project_config",
            ownership_mode: "managed_entry",
            managed_selector:
                "root:model,root:review_model,root:model_reasoning_effort,features.codex_hooks,agents.max_threads,agents.max_depth,codex1_orchestration.model,codex1_orchestration.reasoning_effort,codex1_review.model,codex1_review.reasoning_effort,codex1_fast_parallel.model,codex1_fast_parallel.reasoning_effort,codex1_hard_coding.model,codex1_hard_coding.reasoning_effort"
                    .to_string(),
            change_kind: if existing_config.is_some() {
                "modified"
            } else {
                "created"
            },
            restore_action: if existing_config.is_some() {
                "restore_backup"
            } else {
                "delete_if_created"
            },
            previous_contents: existing_config,
            next_contents: next_config,
        });
    }

    let existing_hooks = read_optional_string(&hooks_path)?;
    let next_hooks =
        build_hooks_json(existing_hooks.as_deref(), args.force, &managed_hook_command)?;
    if existing_hooks.as_deref() != Some(next_hooks.as_str()) {
        planned_changes.push(PlannedChange {
            path: hooks_path,
            component: "hooks",
            ownership_mode: "managed_entry",
            managed_selector: format!("hooks.Stop.command:{managed_hook_command}"),
            change_kind: if existing_hooks.is_some() {
                "modified"
            } else {
                "created"
            },
            restore_action: if existing_hooks.is_some() {
                "restore_backup"
            } else {
                "delete_if_created"
            },
            previous_contents: existing_hooks,
            next_contents: next_hooks,
        });
    }

    let existing_agents = read_optional_string(&agents_path)?;
    let next_agents = build_agents_doc(existing_agents.as_deref(), args.force)?;
    if existing_agents.as_deref() != Some(next_agents.as_str()) {
        planned_changes.push(PlannedChange {
            path: agents_path,
            component: "agents_md",
            ownership_mode: "managed_block",
            managed_selector: "AGENTS.md:codex1:block".to_string(),
            change_kind: if existing_agents.is_some() {
                "modified"
            } else {
                "created"
            },
            restore_action: if existing_agents.is_some() {
                "restore_backup"
            } else {
                "delete_if_created"
            },
            previous_contents: existing_agents,
            next_contents: next_agents,
        });
    }

    let (skill_surface_status, skill_install_mode) = match skill_inspection.status {
        SkillSurfaceStatus::Missing => {
            planned_changes.extend(plan_copied_skill_changes(
                &default_skill_root(&repo_root),
                &managed_skill_files(&source_skill_root)?,
            )?);
            (
                "installed",
                Some(SkillInstallMode::CopiedSkills.as_str().to_string()),
            )
        }
        SkillSurfaceStatus::ValidExisting => (
            "reused_existing",
            skill_inspection
                .install_mode
                .map(|mode| mode.as_str().to_string()),
        ),
        SkillSurfaceStatus::PartialOrDrifted => {
            if !args.force {
                bail!(
                    "target skill surface at {} is partial or drifted (missing: {}; drifted: {}); rerun with --force to rewrite the managed copied skill set",
                    skill_inspection.root.display(),
                    if skill_inspection.missing_required_public_skills.is_empty() {
                        "none".to_string()
                    } else {
                        skill_inspection.missing_required_public_skills.join(", ")
                    },
                    if skill_inspection.drifted_managed_files.is_empty() {
                        "none".to_string()
                    } else {
                        skill_inspection.drifted_managed_files.join(", ")
                    }
                );
            }

            planned_changes.extend(plan_copied_skill_changes(
                &default_skill_root(&repo_root),
                &managed_skill_files(&source_skill_root)?,
            )?);
            (
                "installed",
                Some(SkillInstallMode::CopiedSkills.as_str().to_string()),
            )
        }
    };

    if planned_changes.is_empty() {
        let report = SetupReport {
            repo_root: repo_root.display().to_string(),
            trusted_repo,
            backup_id: None,
            skill_surface_status,
            skill_install_mode,
            skill_surface_root: Some(skill_inspection.discovery_root.display().to_string()),
            changed_paths: Vec::new(),
            notes: setup_notes(&skill_inspection, None, skill_surface_status),
        };
        return emit_report(args.common.json, &report, render_setup_report(&report));
    }

    let preflight = render_setup_preflight(&planned_changes);
    if args.common.json {
        eprintln!("{preflight}");
    } else {
        println!("{preflight}");
    }

    let backup_root = match args.backup_root {
        Some(path) => path,
        None => default_backup_root()?,
    };
    let backup_id = new_backup_id()?;
    let backup_dir = backup_root.join(&backup_id);
    let backup_files_dir = backup_dir.join("files");
    fs::create_dir_all(&backup_files_dir).with_context(|| {
        format!(
            "create backup directory for setup at {}",
            backup_files_dir.display()
        )
    })?;
    let backup_files_dir = fs::canonicalize(&backup_files_dir).with_context(|| {
        format!(
            "canonicalize backup directory {}",
            backup_files_dir.display()
        )
    })?;

    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format backup timestamp")?;

    let mut manifest_paths = Vec::new();
    for (index, change) in planned_changes.iter().enumerate() {
        let backup_path = if let Some(previous_contents) = change.previous_contents.as_ref() {
            let path = backup_files_dir.join(format!("{index:02}_{}.bak", change.component));
            fs::write(&path, previous_contents)
                .with_context(|| format!("write backup copy to {}", path.display()))?;
            Some(
                fs::canonicalize(&path)
                    .with_context(|| format!("canonicalize backup copy {}", path.display()))?,
            )
        } else {
            None
        };

        manifest_paths.push(ManifestPathEntry {
            path: change.path.display().to_string(),
            scope: "project".to_string(),
            change_kind: change.change_kind.to_string(),
            managed_by: "codex1".to_string(),
            component: change.component.to_string(),
            install_mode: if change.component == "skill_file" {
                SkillInstallMode::CopiedSkills.as_str().to_string()
            } else {
                "support_surface".to_string()
            },
            ownership_mode: change.ownership_mode.to_string(),
            managed_selector: change.managed_selector.clone(),
            origin: "codex1 setup".to_string(),
            backup_path: backup_path.map(|path| path.display().to_string()),
            before_hash: change
                .previous_contents
                .as_ref()
                .map(|text| content_hash(text)),
            after_hash: Some(content_hash(&change.next_contents)),
            restore_action: change.restore_action.to_string(),
            applied: false,
        });
    }

    let manifest_path = backup_dir.join("manifest.json");
    let mut manifest = BackupManifest {
        backup_id: backup_id.clone(),
        created_at,
        repo_root: repo_root.display().to_string(),
        codex1_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        skill_install_mode: skill_install_mode.clone(),
        paths: manifest_paths,
    };
    write_manifest(&manifest_path, &manifest)?;

    let mut applied_indices = Vec::new();
    for (index, change) in planned_changes.iter().enumerate() {
        if let Some(parent) = change.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent directory {}", parent.display()))?;
        }
        if let Err(error) = fs::write(&change.path, &change.next_contents)
            .with_context(|| format!("write {}", change.path.display()))
        {
            rollback_setup_changes(
                &planned_changes,
                &applied_indices,
                &mut manifest,
                &manifest_path,
            )
            .context("rollback setup changes after failed write")?;
            return Err(error);
        }
        applied_indices.push(index);
        manifest.paths[index].applied = true;
        write_manifest(&manifest_path, &manifest)?;
    }

    let report = SetupReport {
        repo_root: repo_root.display().to_string(),
        trusted_repo,
        backup_id: Some(backup_id),
        skill_surface_status,
        skill_install_mode,
        skill_surface_root: Some(skill_inspection.discovery_root.display().to_string()),
        changed_paths: planned_changes
            .iter()
            .map(|change| ChangedPathReport {
                path: change.path.display().to_string(),
                change_kind: change.change_kind,
                component: change.component,
            })
            .collect(),
        notes: setup_notes(&skill_inspection, Some(&backup_root), skill_surface_status),
    };

    emit_report(args.common.json, &report, render_setup_report(&report))
}

fn rollback_setup_changes(
    planned_changes: &[PlannedChange],
    applied_indices: &[usize],
    manifest: &mut BackupManifest,
    manifest_path: &Path,
) -> Result<()> {
    for index in applied_indices.iter().rev().copied() {
        restore_planned_change(&planned_changes[index])?;
        manifest.paths[index].applied = false;
    }
    write_manifest(manifest_path, manifest)
}

fn restore_planned_change(change: &PlannedChange) -> Result<()> {
    match &change.previous_contents {
        Some(previous_contents) => {
            if let Some(parent) = change.path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create parent directory {}", parent.display()))?;
            }
            fs::write(&change.path, previous_contents)
                .with_context(|| format!("rollback {}", change.path.display()))?;
        }
        None => {
            if change.path.exists() {
                fs::remove_file(&change.path)
                    .with_context(|| format!("remove {}", change.path.display()))?;
            }
        }
    }
    Ok(())
}

fn default_backup_root() -> Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".codex1/backups"))
}

fn codex_home() -> Result<PathBuf> {
    if let Some(explicit) = env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(explicit));
    }

    let home = env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn is_repo_trusted(repo_root: &Path) -> Result<bool> {
    let config_path = codex_home()?.join("config.toml");
    let Some(raw) = read_optional_string(&config_path)? else {
        return Ok(false);
    };

    let marker = format!("[projects.\"{}\"]", repo_root.display());
    let mut in_project = false;
    for line in raw.lines() {
        let trimmed = strip_comment(line).trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_project = trimmed == marker;
            continue;
        }
        if in_project && trimmed == "trust_level = \"trusted\"" {
            return Ok(true);
        }
    }

    Ok(false)
}

fn build_project_config(existing: Option<&str>) -> String {
    let mut text = existing.unwrap_or_default().to_string();
    text = upsert_config_value(&text, None, "model", ConfigValue::String(CONFIG_MODEL));
    text = upsert_config_value(
        &text,
        None,
        "review_model",
        ConfigValue::String(CONFIG_REVIEW_MODEL),
    );
    text = upsert_config_value(
        &text,
        None,
        "model_reasoning_effort",
        ConfigValue::String(CONFIG_REASONING_EFFORT),
    );
    text = upsert_config_value(
        &text,
        Some("features"),
        "codex_hooks",
        ConfigValue::Bool(true),
    );
    text = upsert_config_value(
        &text,
        Some("agents"),
        "max_threads",
        ConfigValue::Integer(16),
    );
    text = upsert_config_value(&text, Some("agents"), "max_depth", ConfigValue::Integer(1));
    text = upsert_config_value(
        &text,
        Some("codex1_orchestration"),
        "model",
        ConfigValue::String(CONFIG_MODEL),
    );
    text = upsert_config_value(
        &text,
        Some("codex1_orchestration"),
        "reasoning_effort",
        ConfigValue::String(CONFIG_REASONING_EFFORT),
    );
    text = upsert_config_value(
        &text,
        Some("codex1_review"),
        "model",
        ConfigValue::String(CONFIG_REVIEW_MODEL),
    );
    text = upsert_config_value(
        &text,
        Some("codex1_review"),
        "reasoning_effort",
        ConfigValue::String(CONFIG_REASONING_EFFORT),
    );
    text = upsert_config_value(
        &text,
        Some("codex1_fast_parallel"),
        "model",
        ConfigValue::String(CONFIG_FAST_PARALLEL_MODEL),
    );
    text = upsert_config_value(
        &text,
        Some("codex1_fast_parallel"),
        "reasoning_effort",
        ConfigValue::String(CONFIG_FAST_PARALLEL_REASONING_EFFORT),
    );
    text = upsert_config_value(
        &text,
        Some("codex1_hard_coding"),
        "model",
        ConfigValue::String(CONFIG_HARD_CODING_MODEL),
    );
    text = upsert_config_value(
        &text,
        Some("codex1_hard_coding"),
        "reasoning_effort",
        ConfigValue::String(CONFIG_HARD_CODING_REASONING_EFFORT),
    );
    ensure_trailing_newline(text)
}

fn build_hooks_json(
    existing: Option<&str>,
    force: bool,
    managed_hook_command: &str,
) -> Result<String> {
    let mut root = match existing {
        Some(raw) if !raw.trim().is_empty() => {
            serde_json::from_str::<Value>(raw).or_else(|error| {
                if force {
                    Ok(Value::Object(Map::new()))
                } else {
                    Err(error)
                }
            })?
        }
        _ => Value::Object(Map::new()),
    };
    let stop_counts = summarize_stop_authority_with_observational(&root);

    let root_object = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("hooks.json root must be a JSON object"))?;
    let hooks_value = root_object
        .entry("hooks".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks_object = hooks_value
        .as_object_mut()
        .ok_or_else(|| anyhow!("hooks field in hooks.json must be a JSON object"))?;
    let stop_value = hooks_object
        .entry("Stop".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let stop_groups = stop_value
        .as_array_mut()
        .ok_or_else(|| anyhow!("hooks.Stop must be a JSON array"))?;
    let (_total_stop_handlers, managed_stop_handlers) = summarize_stop_handlers(stop_groups);
    if stop_counts.authoritative() > 1 && !force {
        bail!(
            "hooks.json contains multiple authoritative Stop handlers; rerun codex1 setup --force to normalize to one authoritative Codex1 Stop pipeline"
        );
    }
    if stop_counts.authoritative() == 1 && managed_stop_handlers == 0 && !force {
        return serde_json::to_string_pretty(&root).context("serialize preserved hooks.json");
    }
    if stop_counts.authoritative() == 1 && managed_stop_handlers == 1 {
        let mut changed_existing_handler = false;
        for group in stop_groups.iter_mut() {
            if let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                for hook in hooks {
                    if is_managed_stop_handler(hook)
                        || hook
                            .as_object()
                            .and_then(|object| object.get("command"))
                            .and_then(Value::as_str)
                            == Some(managed_hook_command)
                    {
                        let hook_object = hook
                            .as_object_mut()
                            .ok_or_else(|| anyhow!("managed stop hook must be a JSON object"))?;
                        if hook_object.get("command").and_then(Value::as_str)
                            != Some(managed_hook_command)
                        {
                            hook_object.insert(
                                "command".to_string(),
                                Value::String(managed_hook_command.to_string()),
                            );
                            changed_existing_handler = true;
                        }
                        if hook_object.get("statusMessage").and_then(Value::as_str)
                            != Some(MANAGED_STOP_HOOK_STATUS)
                        {
                            hook_object.insert(
                                "statusMessage".to_string(),
                                Value::String(MANAGED_STOP_HOOK_STATUS.to_string()),
                            );
                            changed_existing_handler = true;
                        }
                        let removed_snake =
                            hook_object.remove(OBSERVATIONAL_STOP_HOOK_FLAG).is_some();
                        let removed_camel = hook_object
                            .remove(OBSERVATIONAL_STOP_HOOK_FLAG_CAMEL)
                            .is_some();
                        if removed_snake || removed_camel {
                            changed_existing_handler = true;
                        }
                    }
                }
            } else if is_managed_stop_handler(group)
                || group
                    .as_object()
                    .and_then(|object| object.get("command"))
                    .and_then(Value::as_str)
                    == Some(managed_hook_command)
            {
                let group_object = group
                    .as_object_mut()
                    .ok_or_else(|| anyhow!("managed stop hook must be a JSON object"))?;
                if group_object.get("type").and_then(Value::as_str) != Some("command") {
                    group_object.insert("type".to_string(), Value::String("command".to_string()));
                    changed_existing_handler = true;
                }
                if group_object.get("command").and_then(Value::as_str) != Some(managed_hook_command)
                {
                    group_object.insert(
                        "command".to_string(),
                        Value::String(managed_hook_command.to_string()),
                    );
                    changed_existing_handler = true;
                }
                if group_object.get("statusMessage").and_then(Value::as_str)
                    != Some(MANAGED_STOP_HOOK_STATUS)
                {
                    group_object.insert(
                        "statusMessage".to_string(),
                        Value::String(MANAGED_STOP_HOOK_STATUS.to_string()),
                    );
                    changed_existing_handler = true;
                }
                let removed_snake = group_object.remove(OBSERVATIONAL_STOP_HOOK_FLAG).is_some();
                let removed_camel = group_object
                    .remove(OBSERVATIONAL_STOP_HOOK_FLAG_CAMEL)
                    .is_some();
                if removed_snake || removed_camel {
                    changed_existing_handler = true;
                }
            }
        }
        return serde_json::to_string_pretty(&root).context(if changed_existing_handler {
            "serialize normalized hooks.json"
        } else {
            "serialize idempotent hooks.json"
        });
    }

    let observational_groups = stop_groups
        .iter()
        .filter(|group| stop_group_is_observational(group))
        .cloned()
        .collect::<Vec<_>>();
    stop_groups.clear();
    stop_groups.push(serde_json::json!({
        "hooks": [{
            "type": "command",
            "command": managed_hook_command,
            "statusMessage": MANAGED_STOP_HOOK_STATUS,
        }]
    }));
    stop_groups.extend(observational_groups);

    serde_json::to_string_pretty(&root).context("serialize hooks.json")
}

fn stop_group_is_observational(value: &Value) -> bool {
    if let Some(hooks) = value.get("hooks").and_then(Value::as_array) {
        !hooks.is_empty()
            && hooks.iter().all(|hook| {
                crate::support_surface::is_observational_stop_handler(hook)
                    || stop_group_is_observational(hook)
            })
    } else {
        crate::support_surface::is_observational_stop_handler(value)
    }
}

fn user_stop_hook_authority(path: &Path) -> Result<crate::support_surface::StopAuthorityCounts> {
    let raw = match read_optional_string(path)? {
        Some(raw) => raw,
        None => {
            return Ok(crate::support_surface::StopAuthorityCounts {
                total: 0,
                managed: 0,
                observational: 0,
            });
        }
    };
    let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse user hooks file {}", path.display()))?;
    Ok(summarize_stop_authority_with_observational(&parsed))
}

fn managed_hook_command() -> Result<String> {
    let executable = env::current_exe().context("resolve current codex1 executable")?;
    Ok(format!(
        "{} internal stop-hook",
        shell_escape_arg(&executable.display().to_string())
    ))
}

fn shell_escape_arg(value: &str) -> String {
    if !value.contains([' ', '\t', '\n', '\'', '"', '\\']) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn build_agents_doc(existing: Option<&str>, force: bool) -> Result<String> {
    match existing {
        None => Ok(AGENTS_BLOCK.to_string()),
        Some(raw) => replace_or_append_managed_block(raw, force),
    }
}

fn plan_copied_skill_changes(
    target_root: &Path,
    managed_files: &[ManagedSkillFile],
) -> Result<Vec<PlannedChange>> {
    let mut changes = Vec::new();
    for managed_file in managed_files {
        let path = target_root.join(&managed_file.relative_path);
        let previous_contents = read_optional_string(&path)?;
        let next_contents =
            String::from_utf8(managed_file.contents.clone()).with_context(|| {
                format!(
                    "managed skill file {} is not valid UTF-8",
                    managed_file.relative_path.display()
                )
            })?;
        if previous_contents.as_deref() == Some(next_contents.as_str()) {
            continue;
        }
        changes.push(PlannedChange {
            path,
            component: "skill_file",
            ownership_mode: "full_file",
            managed_selector: format!("skill:{}", managed_file.relative_path.display()),
            change_kind: if previous_contents.is_some() {
                "modified"
            } else {
                "created"
            },
            restore_action: if previous_contents.is_some() {
                "restore_backup"
            } else {
                "delete_if_created"
            },
            previous_contents,
            next_contents,
        });
    }
    Ok(changes)
}

fn replace_or_append_managed_block(existing: &str, force: bool) -> Result<String> {
    let begin_count = existing.matches(AGENTS_BLOCK_BEGIN).count();
    let end_count = existing.matches(AGENTS_BLOCK_END).count();

    if begin_count == 0 && end_count == 0 {
        let mut output = existing.trim_end().to_string();
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(AGENTS_BLOCK);
        return Ok(output);
    }

    if begin_count != 1 || end_count != 1 {
        let _ = force;
        bail!(
            "AGENTS.md contains malformed Codex1 managed markers; repair the shared file manually instead of overwriting the whole document"
        );
    }

    let Some(begin_index) = existing.find(AGENTS_BLOCK_BEGIN) else {
        bail!("failed to find Codex1 begin marker in AGENTS.md");
    };
    let Some(end_index) = existing.find(AGENTS_BLOCK_END) else {
        bail!("failed to find Codex1 end marker in AGENTS.md");
    };
    if begin_index > end_index {
        bail!("Codex1 markers are out of order in AGENTS.md");
    }

    let end_index = end_index + AGENTS_BLOCK_END.len();
    let before = existing[..begin_index].trim_end();
    let after = existing[end_index..].trim_start_matches(['\r', '\n']);

    let mut output = String::new();
    if !before.is_empty() {
        output.push_str(before);
        output.push_str("\n\n");
    }
    output.push_str(AGENTS_BLOCK.trim_end());
    if !after.is_empty() {
        output.push_str("\n\n");
        output.push_str(after.trim_start());
        output.push('\n');
    } else {
        output.push('\n');
    }
    Ok(output)
}

fn read_optional_string(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
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

fn render_setup_report(report: &SetupReport) -> String {
    let mut output = String::new();
    let _ = writeln!(&mut output, "repo root: {}", report.repo_root);
    let _ = writeln!(&mut output, "trusted repo: {}", yes_no(report.trusted_repo));
    let _ = writeln!(
        &mut output,
        "skill surface status: {}",
        report.skill_surface_status
    );
    let _ = writeln!(
        &mut output,
        "skill install state: {}",
        report
            .skill_install_mode
            .as_deref()
            .unwrap_or("reused_existing")
    );
    if let Some(root) = report.skill_surface_root.as_deref() {
        let _ = writeln!(&mut output, "skill surface root: {root}");
    }
    if let Some(backup_id) = report.backup_id.as_deref() {
        let _ = writeln!(&mut output, "backup id: {backup_id}");
    }
    if report.changed_paths.is_empty() {
        let _ = writeln!(&mut output, "changed paths: none");
    } else {
        let _ = writeln!(&mut output, "changed paths:");
        for path in &report.changed_paths {
            let _ = writeln!(
                &mut output,
                "- {} ({}, {})",
                path.path, path.change_kind, path.component
            );
        }
    }
    if !report.notes.is_empty() {
        let _ = writeln!(&mut output, "notes:");
        for note in &report.notes {
            let _ = writeln!(&mut output, "- {note}");
        }
    }
    output.trim_end().to_string()
}

fn render_setup_preflight(planned_changes: &[PlannedChange]) -> String {
    let mut output = String::new();
    let _ = writeln!(&mut output, "planned Codex surface changes before apply:");
    for change in planned_changes {
        let _ = writeln!(
            &mut output,
            "- {} ({}, {})",
            change.path.display(),
            change.change_kind,
            change.component
        );
    }
    output.trim_end().to_string()
}

fn setup_notes(
    inspection: &SkillSurfaceInspection,
    backup_root: Option<&Path>,
    skill_surface_status: &str,
) -> Vec<String> {
    let mut notes = Vec::new();
    if backup_root.is_none() {
        notes.push("setup is already in the desired state".to_string());
    } else if let Some(backup_root) = backup_root {
        notes
            .push("created a reversible backup manifest before mutating managed files".to_string());
        notes.push(format!("backup root: {}", backup_root.display()));
    }

    match skill_surface_status {
        "installed" => notes.push(format!(
            "installed the managed copied skill surface under {}",
            inspection.root.display()
        )),
        "reused_existing" => notes.push(format!(
            "reused an existing valid {} skill surface under {}",
            inspection
                .install_mode
                .map(|mode| mode.as_str())
                .unwrap_or("repo-local"),
            inspection.discovery_root.display()
        )),
        _ => notes.push(format!(
            "skill surface status for {} remains {}",
            inspection.discovery_root.display(),
            skill_surface_status
        )),
    }

    if !inspection.drifted_managed_files.is_empty() {
        notes.push(format!(
            "rewriting drifted managed skill files would affect: {}",
            inspection.drifted_managed_files.join(", ")
        ));
    }

    notes
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn new_backup_id() -> Result<String> {
    let now = OffsetDateTime::now_utc();
    Ok(format!(
        "{}-{}",
        now.unix_timestamp(),
        Uuid::new_v4().simple()
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

fn write_manifest(path: &Path, manifest: &BackupManifest) -> Result<()> {
    let manifest_json = serde_json::to_string_pretty(manifest).context("serialize manifest")?;
    fs::write(path, manifest_json).with_context(|| format!("write manifest to {}", path.display()))
}

const fn default_manifest_applied() -> bool {
    true
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn upsert_config_value(
    source: &str,
    section: Option<&str>,
    key: &str,
    value: ConfigValue<'_>,
) -> String {
    let rendered = value.render();
    let new_line = format!("{key} = {rendered}");
    let mut lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();

    match section {
        None => {
            let section_start = first_section_index(&lines).unwrap_or(lines.len());
            for line in lines.iter_mut().take(section_start) {
                if key_matches(line, key) {
                    *line = new_line.clone();
                    return ensure_trailing_newline(lines.join("\n"));
                }
            }

            let mut updated = Vec::new();
            updated.push(new_line);
            if !lines.is_empty() {
                updated.push(String::new());
            }
            updated.extend(lines);
            ensure_trailing_newline(updated.join("\n"))
        }
        Some(section_name) => {
            let dotted_key = format!("{section_name}.{key}");
            let dotted_line = format!("{dotted_key} = {rendered}");
            let mut replaced_dotted = false;
            for line in &mut lines {
                if key_matches(line, &dotted_key) {
                    *line = dotted_line.clone();
                    replaced_dotted = true;
                }
            }
            if replaced_dotted {
                return ensure_trailing_newline(lines.join("\n"));
            }
            if let Some((start, end)) = find_section_range(&lines, section_name) {
                for line in lines.iter_mut().take(end).skip(start + 1) {
                    if key_matches(line, key) {
                        *line = new_line.clone();
                        return ensure_trailing_newline(lines.join("\n"));
                    }
                }
                lines.insert(end, new_line);
                return ensure_trailing_newline(lines.join("\n"));
            }

            if !lines.is_empty() && !lines.last().is_some_and(String::is_empty) {
                lines.push(String::new());
            }
            lines.push(format!("[{section_name}]"));
            lines.push(new_line);
            ensure_trailing_newline(lines.join("\n"))
        }
    }
}

fn key_matches(line: &str, key: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.starts_with("[[") || trimmed.starts_with('[') {
        return false;
    }
    let Some((candidate, _)) = trimmed.split_once('=') else {
        return false;
    };
    candidate.trim() == key
}

fn first_section_index(lines: &[String]) -> Option<usize> {
    lines
        .iter()
        .position(|line| is_section_header(line).is_some())
}

fn find_section_range(lines: &[String], target: &str) -> Option<(usize, usize)> {
    let mut start = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some(section_name) = is_section_header(line) {
            if start.is_some() {
                return Some((start?, index));
            }
            if section_name == target {
                start = Some(index);
            }
        }
    }
    start.map(|index| (index, lines.len()))
}

fn is_section_header(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.starts_with("[[") || !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }
    Some(trimmed.trim_start_matches('[').trim_end_matches(']'))
}

fn strip_comment(line: &str) -> &str {
    match line.split_once('#') {
        Some((before, _)) => before,
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigValue, build_hooks_json, shell_escape_arg, upsert_config_value};
    use crate::support_surface::{
        MANAGED_STOP_HOOK_STATUS, OBSERVATIONAL_STOP_HOOK_FLAG, OBSERVATIONAL_STOP_HOOK_FLAG_CAMEL,
        is_managed_stop_handler, summarize_stop_authority_with_observational,
        summarize_stop_handlers,
    };
    use serde_json::{Value, json};
    use tempfile::TempDir;

    #[test]
    fn build_hooks_json_rejects_duplicate_stop_handlers_without_force() {
        let existing = json!({
            "hooks": {
                "Stop": [
                    {"hooks": [{"type": "command", "command": "codex1 internal stop-hook", "statusMessage": MANAGED_STOP_HOOK_STATUS}]},
                    {"hooks": [{"type": "command", "command": "python3 stop.py", "statusMessage": "Legacy stop"}]}
                ]
            }
        })
        .to_string();

        let error = build_hooks_json(Some(&existing), false, "codex1 internal stop-hook")
            .expect_err("duplicate stop handlers should require force");
        assert!(
            error
                .to_string()
                .contains("multiple authoritative Stop handlers")
        );
    }

    #[test]
    fn build_hooks_json_force_normalizes_to_one_managed_stop_handler() {
        let existing = json!({
            "hooks": {
                "Stop": [
                    {"hooks": [{"type": "command", "command": "python3 stop.py", "statusMessage": "Legacy stop"}]}
                ]
            }
        })
        .to_string();

        let normalized = build_hooks_json(Some(&existing), true, "codex1 internal stop-hook")
            .expect("normalize hooks");
        let parsed: Value = serde_json::from_str(&normalized).expect("parse normalized hooks");
        let stop_groups = parsed["hooks"]["Stop"]
            .as_array()
            .expect("stop groups should be present");
        let (total, managed) = summarize_stop_handlers(stop_groups);
        assert_eq!(total, 1);
        assert_eq!(managed, 1);
    }

    #[test]
    fn build_hooks_json_preserves_observational_stop_hooks() {
        let existing = json!({
            "hooks": {
                "Stop": [
                    {
                        "hooks": [{
                            "type": "command",
                            "command": "codex1 internal stop-hook",
                            "statusMessage": MANAGED_STOP_HOOK_STATUS
                        }]
                    },
                    {
                        "hooks": [{
                            "type": "command",
                            "command": "python3 observe.py",
                            "codex1_observational": true
                        }]
                    }
                ]
            }
        })
        .to_string();

        let normalized = build_hooks_json(Some(&existing), false, "codex1 internal stop-hook")
            .expect("normalize hooks");
        let parsed: Value = serde_json::from_str(&normalized).expect("parse normalized hooks");
        let counts = summarize_stop_authority_with_observational(&parsed);
        assert_eq!(counts.total, 2);
        assert_eq!(counts.authoritative(), 1);
        assert_eq!(counts.observational, 1);
    }

    #[test]
    fn build_hooks_json_preserves_existing_authoritative_aggregator() {
        let existing = json!({
            "hooks": {
                "Stop": [{
                    "type": "command",
                    "command": "./.codex/stop-hook-aggregator.sh",
                    "statusMessage": MANAGED_STOP_HOOK_STATUS
                }]
            }
        })
        .to_string();

        let normalized = build_hooks_json(Some(&existing), false, "codex1 internal stop-hook")
            .expect("preserve existing aggregator");
        let parsed: Value = serde_json::from_str(&normalized).expect("parse normalized hooks");
        assert_eq!(
            parsed["hooks"]["Stop"][0]["command"],
            "./.codex/stop-hook-aggregator.sh"
        );
    }

    #[test]
    fn upsert_config_value_rewrites_existing_dotted_keys() {
        let existing = "features.codex_hooks = false\nagents.max_threads = 2\n";
        let updated = upsert_config_value(
            existing,
            Some("features"),
            "codex_hooks",
            ConfigValue::Bool(true),
        );
        let updated = upsert_config_value(
            &updated,
            Some("agents"),
            "max_threads",
            ConfigValue::Integer(16),
        );

        assert!(updated.contains("features.codex_hooks = true"));
        assert!(updated.contains("agents.max_threads = 16"));
        assert!(!updated.contains("features.codex_hooks = false"));
        assert!(!updated.contains("agents.max_threads = 2"));
    }

    #[test]
    fn shell_escaped_managed_hook_commands_are_still_detected() {
        let temp = TempDir::new().expect("temp dir");
        let bin_dir = temp.path().join("bin with spaces");
        std::fs::create_dir_all(&bin_dir).expect("create bin dir");
        let binary = bin_dir.join("codex1");
        std::fs::write(&binary, "#!/bin/sh\n").expect("write fake binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = std::fs::metadata(&binary).expect("metadata").permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&binary, permissions).expect("mark executable");
        }

        let hook = json!({
            "type": "command",
            "command": format!("{} internal stop-hook", shell_escape_arg(&binary.display().to_string())),
            "statusMessage": MANAGED_STOP_HOOK_STATUS
        });

        assert!(is_managed_stop_handler(&hook));
    }

    #[test]
    fn build_hooks_json_strips_observational_flags_from_managed_stop_hooks() {
        let existing = json!({
            "hooks": {
                "Stop": [{
                    "type": "command",
                    "command": "codex1 internal stop-hook",
                    "statusMessage": MANAGED_STOP_HOOK_STATUS,
                    "codex1_observational": true,
                    "codex1Observational": true
                }]
            }
        })
        .to_string();

        let normalized = build_hooks_json(Some(&existing), false, "codex1 internal stop-hook")
            .expect("normalize hooks");
        let parsed: Value = serde_json::from_str(&normalized).expect("parse normalized hooks");
        let hook = &parsed["hooks"]["Stop"][0];
        assert!(hook.get(OBSERVATIONAL_STOP_HOOK_FLAG).is_none());
        assert!(hook.get(OBSERVATIONAL_STOP_HOOK_FLAG_CAMEL).is_none());
    }
}
