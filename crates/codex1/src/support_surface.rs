use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use toml::Value as TomlValue;
use walkdir::WalkDir;

pub const REQUIRED_PUBLIC_SKILLS: &[&str] = &["clarify", "plan", "execute", "review", "autopilot"];
pub const MANAGED_SKILLS: &[&str] = &[
    "clarify",
    "plan",
    "execute",
    "review",
    "autopilot",
    "internal-replan",
    "internal-orchestration",
];
pub const MANAGED_STOP_HOOK_STATUS: &str = "Codex1 Ralph stop hook";
pub const OBSERVATIONAL_STOP_HOOK_FLAG: &str = "codex1_observational";
pub const OBSERVATIONAL_STOP_HOOK_FLAG_CAMEL: &str = "codex1Observational";
pub const AGENTS_BLOCK_BEGIN: &str = "<!-- codex1:begin -->";
pub const AGENTS_BLOCK_END: &str = "<!-- codex1:end -->";
pub const LEGACY_AGENTS_BLOCK_BEGIN: &str = "<!-- CODEX1:BEGIN MANAGED BLOCK -->";
pub const LEGACY_AGENTS_BLOCK_END: &str = "<!-- CODEX1:END MANAGED BLOCK -->";
pub const AGENTS_BUILD_COMMAND_PLACEHOLDER: &str = "{{BUILD_COMMAND}}";
pub const AGENTS_TEST_COMMAND_PLACEHOLDER: &str = "{{TEST_COMMAND}}";
pub const AGENTS_LINT_COMMAND_PLACEHOLDER: &str = "{{LINT_OR_FORMAT_COMMAND}}";
pub const AGENTS_BLOCK: &str = "<!-- codex1:begin -->\n## Codex1\n### Workflow Stance\n- Use the native Codex skills surface for `clarify`, `plan`, `execute`, `review`, and `autopilot`.\n- Keep mission truth in visible repo artifacts instead of hidden chat state.\n- Replan stays internal unless the repo truth explicitly says otherwise.\n\n### Quality Bar\n- Work is complete only when the locked outcome, proof, review, and closeout contracts are all satisfied.\n- Review is mandatory before mission completion.\n- Hold the repo to production-grade changes with explicit validation and review-clean closeout.\n\n### Repo Commands\n- Build: {{BUILD_COMMAND}}\n- Test: {{TEST_COMMAND}}\n- Lint or format: {{LINT_OR_FORMAT_COMMAND}}\n\n### Artifact Conventions\n- Mission packages live under `PLANS/<mission-id>/`.\n- `OUTCOME-LOCK.md` is canonical for destination truth.\n- `PROGRAM-BLUEPRINT.md` is canonical for route truth.\n- `specs/*/SPEC.md` is canonical for one bounded execution slice.\n<!-- codex1:end -->\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedAgentsBlockSpan {
    pub begin_marker: &'static str,
    pub end_marker: &'static str,
    pub begin_index: usize,
    pub end_index: usize,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentsScaffoldStatus {
    Present,
    MissingFile,
    MissingBlock,
    DriftedBlock,
    MalformedMarkers,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillInstallMode {
    SkillsConfigBridge,
    LinkedSkills,
    CopiedSkills,
}

impl SkillInstallMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SkillsConfigBridge => "skills_config_bridge",
            Self::LinkedSkills => "linked_skills",
            Self::CopiedSkills => "copied_skills",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillSurfaceStatus {
    Missing,
    ValidExisting,
    PartialOrDrifted,
    InvalidBridge,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillSurfaceInspection {
    pub status: SkillSurfaceStatus,
    pub root: PathBuf,
    pub discovery_root: PathBuf,
    pub install_mode: Option<SkillInstallMode>,
    pub source_root: PathBuf,
    pub missing_required_public_skills: Vec<String>,
    pub drifted_managed_files: Vec<String>,
    pub matched_managed_files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentsCommandStatus {
    Missing,
    Placeholder,
    Concrete,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentsScaffoldInspection {
    pub status: AgentsScaffoldStatus,
    pub build_command: Option<String>,
    pub test_command: Option<String>,
    pub lint_or_format_command: Option<String>,
    pub command_status: AgentsCommandStatus,
}

pub fn render_managed_agents_block(
    build_command: &str,
    test_command: &str,
    lint_or_format_command: &str,
) -> String {
    render_managed_agents_block_with_markers(
        AGENTS_BLOCK_BEGIN,
        AGENTS_BLOCK_END,
        build_command,
        test_command,
        lint_or_format_command,
    )
}

pub fn render_managed_agents_block_with_markers(
    begin_marker: &str,
    end_marker: &str,
    build_command: &str,
    test_command: &str,
    lint_or_format_command: &str,
) -> String {
    AGENTS_BLOCK
        .replace(AGENTS_BLOCK_BEGIN, begin_marker)
        .replace(AGENTS_BLOCK_END, end_marker)
        .replace(AGENTS_BUILD_COMMAND_PLACEHOLDER, build_command)
        .replace(AGENTS_TEST_COMMAND_PLACEHOLDER, test_command)
        .replace(AGENTS_LINT_COMMAND_PLACEHOLDER, lint_or_format_command)
}

#[derive(Debug, Clone)]
pub struct ManagedSkillFile {
    pub relative_path: PathBuf,
    pub contents: Vec<u8>,
}

pub fn default_skill_root(repo_root: &Path) -> PathBuf {
    repo_root.join(".codex/skills")
}

pub fn lookup_toml_value(source: &str, section: Option<&str>, key: &str) -> Option<String> {
    let parsed = source.parse::<TomlValue>().ok()?;
    let value = lookup_toml_value_ref(&parsed, section, key)?;
    render_toml_scalar(value)
}

pub fn detect_toml_bool(source: &str, section: Option<&str>, key: &str) -> Option<bool> {
    let parsed = source.parse::<TomlValue>().ok()?;
    lookup_toml_value_ref(&parsed, section, key).and_then(TomlValue::as_bool)
}

pub fn toml_repo_is_trusted(source: &str, repo_root: &Path) -> bool {
    let Ok(parsed) = source.parse::<TomlValue>() else {
        return false;
    };
    parsed
        .as_table()
        .and_then(|table| table.get("projects"))
        .and_then(TomlValue::as_table)
        .is_some_and(|projects| {
            projects.iter().any(|(project_path, project)| {
                project
                    .as_table()
                    .and_then(|table| table.get("trust_level"))
                    .and_then(TomlValue::as_str)
                    == Some("trusted")
                    && trusted_project_path_matches(project_path, repo_root)
            })
        })
}

fn trusted_project_path_matches(project_path: &str, repo_root: &Path) -> bool {
    if project_path == repo_root.display().to_string() {
        return true;
    }

    let candidate = Path::new(project_path);
    if !candidate.is_absolute() {
        return false;
    }

    match fs::canonicalize(candidate) {
        Ok(canonical) => canonical == repo_root,
        Err(_) => false,
    }
}

fn lookup_toml_value_ref<'a>(
    parsed: &'a TomlValue,
    section: Option<&str>,
    key: &str,
) -> Option<&'a TomlValue> {
    let mut current = parsed.as_table()?;
    if let Some(section) = section {
        for segment in section.split('.') {
            current = current.get(segment)?.as_table()?;
        }
    }
    current.get(key)
}

fn render_toml_scalar(value: &TomlValue) -> Option<String> {
    match value {
        TomlValue::String(value) => Some(value.clone()),
        TomlValue::Integer(value) => Some(value.to_string()),
        TomlValue::Float(value) => Some(value.to_string()),
        TomlValue::Boolean(value) => Some(value.to_string()),
        TomlValue::Datetime(value) => Some(value.to_string()),
        _ => None,
    }
}

pub fn resolve_source_skills_root() -> Result<PathBuf> {
    if let Some(explicit) = env::var_os("CODEX1_SKILLS_ROOT") {
        let candidate = fs::canonicalize(PathBuf::from(explicit))
            .context("failed to canonicalize CODEX1_SKILLS_ROOT while resolving skill assets")?;
        return validate_source_skills_root(&candidate);
    }

    let exe = env::current_exe().context("failed to resolve current executable")?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| anyhow!("current executable has no parent directory"))?;

    for candidate in [
        exe_dir.join(".codex/skills"),
        exe_dir.join("skills"),
        exe_dir.join("../share/codex1/skills"),
    ] {
        if candidate.is_dir() {
            let canonical = fs::canonicalize(&candidate).with_context(|| {
                format!(
                    "failed to canonicalize candidate skill root {}",
                    candidate.display()
                )
            })?;
            if let Ok(validated) = validate_source_skills_root(&canonical) {
                return Ok(validated);
            }
        }
    }

    for ancestor in exe.ancestors() {
        let repo_root = ancestor;
        if repo_root.join("docs/codex1-prd.md").is_file() {
            let candidate = repo_root.join(".codex/skills");
            if candidate.is_dir() {
                let canonical = fs::canonicalize(&candidate).with_context(|| {
                    format!(
                        "failed to canonicalize source skill root {}",
                        candidate.display()
                    )
                })?;
                if let Ok(validated) = validate_source_skills_root(&canonical) {
                    return Ok(validated);
                }
            }
        }
    }

    bail!(
        "unable to resolve Codex1 skill assets; set CODEX1_SKILLS_ROOT or run from a repo/build that includes .codex/skills"
    )
}

pub fn inspect_skill_surface(repo_root: &Path) -> Result<SkillSurfaceInspection> {
    let source_root = resolve_source_skills_root()?;
    inspect_skill_surface_with_source(repo_root, &source_root)
}

pub fn inspect_agents_scaffold_details(raw: Option<&str>) -> AgentsScaffoldInspection {
    let Some(raw) = raw else {
        return AgentsScaffoldInspection {
            status: AgentsScaffoldStatus::MissingFile,
            build_command: None,
            test_command: None,
            lint_or_format_command: None,
            command_status: AgentsCommandStatus::Missing,
        };
    };

    match locate_managed_agents_block_span(raw) {
        Some(_) => match extract_managed_agents_block(raw) {
            Some(block) => inspect_managed_agents_block(&block),
            None => AgentsScaffoldInspection {
                status: AgentsScaffoldStatus::MalformedMarkers,
                build_command: None,
                test_command: None,
                lint_or_format_command: None,
                command_status: AgentsCommandStatus::Missing,
            },
        },
        None if managed_agents_block_is_malformed(raw) => AgentsScaffoldInspection {
            status: AgentsScaffoldStatus::MalformedMarkers,
            build_command: None,
            test_command: None,
            lint_or_format_command: None,
            command_status: AgentsCommandStatus::Missing,
        },
        None => AgentsScaffoldInspection {
            status: AgentsScaffoldStatus::MissingBlock,
            build_command: None,
            test_command: None,
            lint_or_format_command: None,
            command_status: AgentsCommandStatus::Missing,
        },
    }
}

pub fn inspect_skill_surface_with_source(
    repo_root: &Path,
    source_root: &Path,
) -> Result<SkillSurfaceInspection> {
    let root = default_skill_root(repo_root);
    let (discovery_root, install_mode, bridge_error) =
        resolve_skill_surface_root(repo_root, &root)?;
    let managed_files = managed_skill_files(source_root)?;
    let source_relatives: Vec<PathBuf> = managed_files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect();

    if let Some(bridge_error) = bridge_error {
        return Ok(SkillSurfaceInspection {
            status: SkillSurfaceStatus::InvalidBridge,
            root,
            discovery_root,
            install_mode,
            source_root: source_root.to_path_buf(),
            missing_required_public_skills: REQUIRED_PUBLIC_SKILLS
                .iter()
                .map(|skill| (*skill).to_string())
                .collect(),
            drifted_managed_files: Vec::new(),
            matched_managed_files: 0,
            bridge_error: Some(bridge_error),
        });
    }

    if !discovery_root.exists() {
        return Ok(SkillSurfaceInspection {
            status: SkillSurfaceStatus::Missing,
            root,
            discovery_root,
            install_mode,
            source_root: source_root.to_path_buf(),
            missing_required_public_skills: REQUIRED_PUBLIC_SKILLS
                .iter()
                .map(|skill| (*skill).to_string())
                .collect(),
            drifted_managed_files: Vec::new(),
            matched_managed_files: 0,
            bridge_error: None,
        });
    }

    let mut matched_managed_files = 0_usize;
    let mut drifted_managed_files = Vec::new();
    for managed_file in &managed_files {
        let target_path = discovery_root.join(&managed_file.relative_path);
        match fs::read(&target_path) {
            Ok(current) if current == managed_file.contents => matched_managed_files += 1,
            Ok(_) => drifted_managed_files.push(path_string(&managed_file.relative_path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if managed_prefix_exists(&discovery_root, &managed_file.relative_path) {
                    drifted_managed_files.push(path_string(&managed_file.relative_path));
                }
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read {}", target_path.display()));
            }
        }
    }

    let missing_required_public_skills = REQUIRED_PUBLIC_SKILLS
        .iter()
        .filter(|skill| !discovery_root.join(skill).join("SKILL.md").is_file())
        .map(|skill| (*skill).to_string())
        .collect::<Vec<_>>();

    let status =
        if matched_managed_files == source_relatives.len() && drifted_managed_files.is_empty() {
            SkillSurfaceStatus::ValidExisting
        } else if target_has_any_managed_path(&discovery_root, &source_relatives)? {
            SkillSurfaceStatus::PartialOrDrifted
        } else {
            SkillSurfaceStatus::Missing
        };

    Ok(SkillSurfaceInspection {
        status,
        root,
        discovery_root,
        install_mode,
        source_root: source_root.to_path_buf(),
        missing_required_public_skills,
        drifted_managed_files,
        matched_managed_files,
        bridge_error: None,
    })
}

fn resolve_skill_surface_root(
    repo_root: &Path,
    default_root: &Path,
) -> Result<(PathBuf, Option<SkillInstallMode>, Option<String>)> {
    match resolve_skills_config_bridge_root(repo_root)? {
        SkillsConfigBridgeResolution::Valid(bridge_root) => {
            return Ok((
                bridge_root,
                Some(SkillInstallMode::SkillsConfigBridge),
                None,
            ));
        }
        SkillsConfigBridgeResolution::Invalid {
            discovery_root,
            reason,
        } => {
            return Ok((
                discovery_root,
                Some(SkillInstallMode::SkillsConfigBridge),
                Some(reason),
            ));
        }
        SkillsConfigBridgeResolution::NotConfigured => {}
    }

    if default_root.exists() {
        let install_mode = match fs::symlink_metadata(default_root) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Some(SkillInstallMode::LinkedSkills)
            }
            Ok(_) => Some(SkillInstallMode::CopiedSkills),
            Err(_) => None,
        };
        return Ok((default_root.to_path_buf(), install_mode, None));
    }

    Ok((default_root.to_path_buf(), None, None))
}

enum SkillsConfigBridgeResolution {
    NotConfigured,
    Valid(PathBuf),
    Invalid {
        discovery_root: PathBuf,
        reason: String,
    },
}

fn resolve_skills_config_bridge_root(repo_root: &Path) -> Result<SkillsConfigBridgeResolution> {
    let config_path = repo_root.join(".codex/config.toml");
    let Ok(raw) = fs::read_to_string(&config_path) else {
        return Ok(SkillsConfigBridgeResolution::NotConfigured);
    };
    let Ok(parsed) = raw.parse::<TomlValue>() else {
        if contains_uncommented_skills_config_header(&raw) {
            return Ok(SkillsConfigBridgeResolution::Invalid {
                discovery_root: config_path.clone(),
                reason: "failed to parse .codex/config.toml while reading [[skills.config]]"
                    .to_string(),
            });
        }
        return Ok(SkillsConfigBridgeResolution::NotConfigured);
    };
    let Some(entries) = parsed
        .get("skills")
        .and_then(|value| value.get("config"))
        .and_then(TomlValue::as_array)
    else {
        return Ok(SkillsConfigBridgeResolution::NotConfigured);
    };

    let mut first_invalid = None;
    for entry in entries {
        let Some(table) = entry.as_table() else {
            first_invalid.get_or_insert_with(|| SkillsConfigBridgeResolution::Invalid {
                discovery_root: config_path.clone(),
                reason: "each [[skills.config]] entry must be a TOML table".to_string(),
            });
            continue;
        };
        let path_value = table.get("path");
        let enabled_value = table.get("enabled");
        let path = path_value.and_then(TomlValue::as_str);
        let enabled = enabled_value.and_then(TomlValue::as_bool);
        match (path_value, enabled_value, path, enabled) {
            (Some(_), Some(_), Some(path), Some(true)) => {
                return resolve_bridge_candidate(&config_path, repo_root, path);
            }
            (Some(_), Some(_), Some(_), Some(false)) | (None, None, _, _) => {}
            (Some(_), Some(_), None, Some(false)) | (None, Some(_), _, Some(false)) => {}
            (Some(_), Some(_), None, _) => {
                first_invalid.get_or_insert_with(|| SkillsConfigBridgeResolution::Invalid {
                    discovery_root: config_path.clone(),
                    reason: "skills.config path must be a string".to_string(),
                });
            }
            (Some(_), Some(_), _, None) => {
                first_invalid.get_or_insert_with(|| SkillsConfigBridgeResolution::Invalid {
                    discovery_root: config_path.clone(),
                    reason: "skills.config enabled must be a boolean".to_string(),
                });
            }
            (Some(_), None, _, _) => {
                first_invalid.get_or_insert_with(|| SkillsConfigBridgeResolution::Invalid {
                    discovery_root: config_path.clone(),
                    reason: "skills.config entry is missing enabled".to_string(),
                });
            }
            (None, Some(_), _, Some(true)) => {
                first_invalid.get_or_insert_with(|| SkillsConfigBridgeResolution::Invalid {
                    discovery_root: config_path.clone(),
                    reason: "enabled skills.config entry is missing path".to_string(),
                });
            }
            (None, Some(_), _, None) => {
                first_invalid.get_or_insert_with(|| SkillsConfigBridgeResolution::Invalid {
                    discovery_root: config_path.clone(),
                    reason: "skills.config enabled must be a boolean".to_string(),
                });
            }
        }
    }

    Ok(first_invalid.unwrap_or(SkillsConfigBridgeResolution::NotConfigured))
}

fn resolve_bridge_candidate(
    config_path: &Path,
    repo_root: &Path,
    raw_path: &str,
) -> Result<SkillsConfigBridgeResolution> {
    let candidate = PathBuf::from(raw_path);
    let candidates = if candidate.is_absolute() {
        vec![candidate.clone()]
    } else {
        vec![
            config_path.parent().unwrap_or(repo_root).join(&candidate),
            repo_root.join(&candidate),
        ]
    };

    for candidate in candidates {
        if candidate.exists() {
            let canonical = fs::canonicalize(&candidate).with_context(|| {
                format!(
                    "failed to canonicalize bridged skill root {}",
                    candidate.display()
                )
            })?;
            return Ok(match validate_source_skills_root(&canonical) {
                Ok(validated) => SkillsConfigBridgeResolution::Valid(validated),
                Err(error) => SkillsConfigBridgeResolution::Invalid {
                    discovery_root: canonical,
                    reason: format!("{error:#}"),
                },
            });
        }
    }

    let hint = if candidate.is_absolute() {
        candidate
    } else {
        config_path.parent().unwrap_or(repo_root).join(&candidate)
    };
    Ok(SkillsConfigBridgeResolution::Invalid {
        discovery_root: hint,
        reason: format!(
            "skills.config bridge points to {}, but no valid Codex1 skill root was found",
            raw_path
        ),
    })
}

fn contains_uncommented_skills_config_header(raw: &str) -> bool {
    raw.lines()
        .filter_map(parse_skills_config_header_line)
        .any(|header| header == "[[skills.config]]")
}

pub fn managed_skill_files(source_root: &Path) -> Result<Vec<ManagedSkillFile>> {
    validate_source_skills_root(source_root)?;
    let mut files = Vec::new();
    for skill in MANAGED_SKILLS {
        let skill_root = source_root.join(skill);
        for entry in WalkDir::new(&skill_root).sort_by_file_name() {
            let entry =
                entry.with_context(|| format!("failed to walk {}", skill_root.display()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let relative_path = path
                .strip_prefix(source_root)
                .expect("managed skill files should stay under the source skill root")
                .to_path_buf();
            files.push(ManagedSkillFile {
                relative_path,
                contents: fs::read(path)
                    .with_context(|| format!("failed to read {}", path.display()))?,
            });
        }
    }
    Ok(files)
}

pub fn compute_support_surface_signature(
    project_config: Option<&str>,
    hooks_json: Option<&str>,
    user_hooks_json: Option<&str>,
    agents_block: Option<&str>,
    skill_root: &Path,
) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"codex1-support-surface-v1\n");
    hash_named_bytes(
        &mut hasher,
        "project_config",
        project_config.map(str::as_bytes),
    );
    hash_named_bytes(&mut hasher, "hooks_json", hooks_json.map(str::as_bytes));
    hash_named_bytes(
        &mut hasher,
        "user_hooks_json",
        user_hooks_json.map(str::as_bytes),
    );
    hash_named_bytes(&mut hasher, "agents_block", agents_block.map(str::as_bytes));

    hasher.update(b"skill_root\n");
    if skill_root.is_dir() {
        for skill in MANAGED_SKILLS {
            let skill_dir = skill_root.join(skill);
            hasher.update(skill.as_bytes());
            hasher.update(b"\n");
            if !skill_dir.is_dir() {
                hasher.update(b"absent\n");
                continue;
            }
            for entry in WalkDir::new(&skill_dir).sort_by_file_name() {
                let entry =
                    entry.with_context(|| format!("failed to walk {}", skill_dir.display()))?;
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                let relative = path
                    .strip_prefix(skill_root)
                    .expect("managed skill walk should remain under the skill root");
                hasher.update(relative.to_string_lossy().as_bytes());
                hasher.update(b"\n");
                let contents =
                    fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
                hasher.update(contents.as_slice());
                hasher.update(b"\n");
            }
        }
    } else {
        hasher.update(b"absent\n");
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopAuthorityCounts {
    pub total: usize,
    pub managed: usize,
    pub observational: usize,
}

impl StopAuthorityCounts {
    #[must_use]
    pub const fn authoritative(self) -> usize {
        self.total.saturating_sub(self.observational)
    }
}

pub fn summarize_stop_handlers(stop_groups: &[Value]) -> (usize, usize) {
    let counts = summarize_stop_handlers_with_observational(stop_groups);
    (counts.total, counts.managed)
}

pub fn summarize_stop_handlers_with_observational(stop_groups: &[Value]) -> StopAuthorityCounts {
    let mut total = 0_usize;
    let mut managed = 0_usize;
    let mut observational = 0_usize;
    for group in stop_groups {
        if let Some(hooks) = group.get("hooks").and_then(Value::as_array) {
            let nested = summarize_stop_handlers_with_observational(hooks);
            total += nested.total;
            managed += nested.managed;
            observational += nested.observational;
        } else if group
            .as_object()
            .is_some_and(|object| object.contains_key("command") || object.contains_key("type"))
        {
            total += 1;
            managed += usize::from(is_managed_stop_handler(group));
            observational += usize::from(is_observational_stop_handler(group));
        } else if let Some(items) = group.as_array() {
            let nested = summarize_stop_handlers_with_observational(items);
            total += nested.total;
            managed += nested.managed;
            observational += nested.observational;
        }
    }
    StopAuthorityCounts {
        total,
        managed,
        observational,
    }
}

pub fn summarize_stop_authority(value: &Value) -> (usize, usize) {
    let counts = summarize_stop_authority_with_observational(value);
    (counts.total, counts.managed)
}

pub fn summarize_stop_authority_with_observational(value: &Value) -> StopAuthorityCounts {
    match value {
        Value::Array(items) => items.iter().fold(
            StopAuthorityCounts {
                total: 0,
                managed: 0,
                observational: 0,
            },
            |acc, item| {
                let nested = summarize_stop_authority_with_observational(item);
                StopAuthorityCounts {
                    total: acc.total + nested.total,
                    managed: acc.managed + nested.managed,
                    observational: acc.observational + nested.observational,
                }
            },
        ),
        Value::Object(map) => {
            if matches!(map.get("event").and_then(Value::as_str), Some("Stop"))
                || map
                    .get("events")
                    .and_then(Value::as_array)
                    .is_some_and(|events| events.iter().any(|event| event.as_str() == Some("Stop")))
            {
                return StopAuthorityCounts {
                    total: 1,
                    managed: usize::from(
                        value.as_object().is_some_and(|object| {
                            object.contains_key("command") || object.contains_key("type")
                        }) && is_managed_stop_handler(value),
                    ),
                    observational: usize::from(is_observational_stop_handler(value)),
                };
            }

            map.iter().fold(
                StopAuthorityCounts {
                    total: 0,
                    managed: 0,
                    observational: 0,
                },
                |acc, (key, nested)| {
                    let nested_counts = if key == "Stop" {
                        match nested {
                            Value::Array(items) => {
                                summarize_stop_handlers_with_observational(items)
                            }
                            other => summarize_stop_handlers_with_observational(
                                std::slice::from_ref(other),
                            ),
                        }
                    } else {
                        summarize_stop_authority_with_observational(nested)
                    };
                    StopAuthorityCounts {
                        total: acc.total + nested_counts.total,
                        managed: acc.managed + nested_counts.managed,
                        observational: acc.observational + nested_counts.observational,
                    }
                },
            )
        }
        _ => StopAuthorityCounts {
            total: 0,
            managed: 0,
            observational: 0,
        },
    }
}

pub fn is_managed_stop_handler(value: &Value) -> bool {
    let command = value
        .as_object()
        .and_then(|object| object.get("command"))
        .and_then(Value::as_str);
    let status = value
        .as_object()
        .and_then(|object| object.get("statusMessage"))
        .and_then(Value::as_str);

    (status == Some(MANAGED_STOP_HOOK_STATUS) && command.is_some_and(is_managed_stop_hook_command))
        || command.is_some_and(is_managed_stop_hook_command)
}

fn is_managed_stop_hook_command(command: &str) -> bool {
    let Some(parts) = split_shell_words(command) else {
        return false;
    };
    if parts.as_slice() == ["codex1", "internal", "stop-hook"] {
        return true;
    }
    if parts.len() != 3 || parts[1] != "internal" || parts[2] != "stop-hook" {
        return false;
    }
    let binary = Path::new(&parts[0]);
    binary.file_name().and_then(|name| name.to_str()) == Some("codex1")
        && is_executable_file(binary)
}

pub fn is_observational_stop_handler(value: &Value) -> bool {
    if is_managed_stop_handler(value) {
        return false;
    }

    value
        .as_object()
        .and_then(|object| {
            object
                .get(OBSERVATIONAL_STOP_HOOK_FLAG)
                .or_else(|| object.get(OBSERVATIONAL_STOP_HOOK_FLAG_CAMEL))
        })
        .and_then(Value::as_bool)
        == Some(true)
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn split_shell_words(input: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut single_quoted = false;
    let mut double_quoted = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            '\\' if !single_quoted => {
                let escaped = chars.next()?;
                current.push(escaped);
            }
            ch if ch.is_whitespace() && !single_quoted && !double_quoted => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if single_quoted || double_quoted {
        return None;
    }
    if !current.is_empty() {
        words.push(current);
    }
    Some(words)
}

pub fn locate_managed_agents_block_span(contents: &str) -> Option<ManagedAgentsBlockSpan> {
    match detect_managed_agents_block(contents) {
        ManagedAgentsBlockDetection::Located(span) => Some(span),
        ManagedAgentsBlockDetection::Missing | ManagedAgentsBlockDetection::Malformed => None,
    }
}

pub fn managed_agents_block_is_malformed(contents: &str) -> bool {
    matches!(
        detect_managed_agents_block(contents),
        ManagedAgentsBlockDetection::Malformed
    )
}

pub fn extract_managed_agents_block(contents: &str) -> Option<String> {
    let span = locate_managed_agents_block_span(contents)?;
    let block = contents[span.begin_index..span.end_index + span.end_marker.len()].to_string();
    Some(ensure_trailing_newline(
        &block
            .replace("\r\n", "\n")
            .replace(span.begin_marker, AGENTS_BLOCK_BEGIN)
            .replace(span.end_marker, AGENTS_BLOCK_END),
    ))
}

enum ManagedAgentsBlockDetection {
    Missing,
    Located(ManagedAgentsBlockSpan),
    Malformed,
}

fn detect_managed_agents_block(contents: &str) -> ManagedAgentsBlockDetection {
    let current =
        locate_agents_block_span_with_markers(contents, AGENTS_BLOCK_BEGIN, AGENTS_BLOCK_END);
    let legacy = locate_agents_block_span_with_markers(
        contents,
        LEGACY_AGENTS_BLOCK_BEGIN,
        LEGACY_AGENTS_BLOCK_END,
    );
    match (current, legacy) {
        (MarkerFamilyDetection::Missing, MarkerFamilyDetection::Missing) => {
            ManagedAgentsBlockDetection::Missing
        }
        (MarkerFamilyDetection::Located(span), MarkerFamilyDetection::Missing)
        | (MarkerFamilyDetection::Missing, MarkerFamilyDetection::Located(span)) => {
            ManagedAgentsBlockDetection::Located(span)
        }
        _ => ManagedAgentsBlockDetection::Malformed,
    }
}

fn locate_agents_block_span_with_markers(
    contents: &str,
    begin_marker: &'static str,
    end_marker: &'static str,
) -> MarkerFamilyDetection {
    let mut begin_positions = Vec::new();
    let mut end_positions = Vec::new();
    let mut offset = 0;
    let mut in_fence = false;
    for line in contents.split_inclusive('\n') {
        let line = line.trim_end_matches(['\r', '\n']);
        let trimmed = line.trim();
        if !in_fence {
            if trimmed == begin_marker
                && let Some(relative) = line.find(begin_marker)
            {
                begin_positions.push(offset + relative);
            }
            if trimmed == end_marker
                && let Some(relative) = line.find(end_marker)
            {
                end_positions.push(offset + relative);
            }
        }
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        }
        offset += line.len();
        if contents.as_bytes().get(offset) == Some(&b'\r') {
            offset += 1;
        }
        if contents.as_bytes().get(offset) == Some(&b'\n') {
            offset += 1;
        }
    }

    match (begin_positions.as_slice(), end_positions.as_slice()) {
        ([], []) => MarkerFamilyDetection::Missing,
        ([begin_index], [end_index]) if begin_index < end_index => {
            MarkerFamilyDetection::Located(ManagedAgentsBlockSpan {
                begin_marker,
                end_marker,
                begin_index: *begin_index,
                end_index: *end_index,
            })
        }
        _ => MarkerFamilyDetection::Malformed,
    }
}

enum MarkerFamilyDetection {
    Missing,
    Located(ManagedAgentsBlockSpan),
    Malformed,
}

fn parse_skills_config_header_line(line: &str) -> Option<&str> {
    let code = strip_toml_comment(line).trim();
    (!code.is_empty()).then_some(code)
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '#' if !in_single && !in_double => return &line[..index],
            '"' if !in_single && !escaped => in_double = !in_double,
            '\'' if !in_double => in_single = !in_single,
            '\\' if in_double => {
                escaped = !escaped;
                continue;
            }
            _ => {}
        }
        escaped = false;
    }
    line
}

fn is_unresolved_agents_command(value: &str) -> bool {
    matches!(
        value.trim(),
        "true # no dedicated build command detected"
            | "true # no dedicated test command detected"
            | "true # no dedicated lint-or-format command detected"
            | "true # no build script detected"
            | "true # no test script detected"
            | "true # no lint-or-format script detected"
            | "true # codex1 fallback: build command not auto-detected"
            | "true # codex1 fallback: test command not auto-detected"
            | "true # codex1 fallback: lint-or-format command not auto-detected"
    )
}

fn ensure_trailing_newline(value: &str) -> String {
    let mut output = value.to_string();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn inspect_managed_agents_block(block: &str) -> AgentsScaffoldInspection {
    let normalized = ensure_trailing_newline(block);
    let lines: Vec<&str> = normalized.lines().collect();
    if lines.len() != 23 {
        return AgentsScaffoldInspection {
            status: AgentsScaffoldStatus::DriftedBlock,
            build_command: None,
            test_command: None,
            lint_or_format_command: None,
            command_status: AgentsCommandStatus::Missing,
        };
    }

    let expected_static = [
        (0, AGENTS_BLOCK_BEGIN),
        (1, "## Codex1"),
        (2, "### Workflow Stance"),
        (
            3,
            "- Use the native Codex skills surface for `clarify`, `plan`, `execute`, `review`, and `autopilot`.",
        ),
        (
            4,
            "- Keep mission truth in visible repo artifacts instead of hidden chat state.",
        ),
        (
            5,
            "- Replan stays internal unless the repo truth explicitly says otherwise.",
        ),
        (6, ""),
        (7, "### Quality Bar"),
        (
            8,
            "- Work is complete only when the locked outcome, proof, review, and closeout contracts are all satisfied.",
        ),
        (9, "- Review is mandatory before mission completion."),
        (
            10,
            "- Hold the repo to production-grade changes with explicit validation and review-clean closeout.",
        ),
        (11, ""),
        (12, "### Repo Commands"),
        (16, ""),
        (17, "### Artifact Conventions"),
        (18, "- Mission packages live under `PLANS/<mission-id>/`."),
        (
            19,
            "- `OUTCOME-LOCK.md` is canonical for destination truth.",
        ),
        (20, "- `PROGRAM-BLUEPRINT.md` is canonical for route truth."),
        (
            21,
            "- `specs/*/SPEC.md` is canonical for one bounded execution slice.",
        ),
        (22, AGENTS_BLOCK_END),
    ];
    for (index, expected) in expected_static {
        if lines.get(index).copied() != Some(expected) {
            return AgentsScaffoldInspection {
                status: AgentsScaffoldStatus::DriftedBlock,
                build_command: None,
                test_command: None,
                lint_or_format_command: None,
                command_status: AgentsCommandStatus::Missing,
            };
        }
    }

    let build_command = lines[13].strip_prefix("- Build: ").map(ToOwned::to_owned);
    let test_command = lines[14].strip_prefix("- Test: ").map(ToOwned::to_owned);
    let lint_or_format_command = lines[15]
        .strip_prefix("- Lint or format: ")
        .map(ToOwned::to_owned);

    let command_values = [
        build_command.as_deref(),
        test_command.as_deref(),
        lint_or_format_command.as_deref(),
    ];
    let command_status = if command_values
        .iter()
        .any(|value| value.is_none() || value.is_some_and(|value| value.trim().is_empty()))
    {
        AgentsCommandStatus::Missing
    } else if build_command.as_deref() == Some(AGENTS_BUILD_COMMAND_PLACEHOLDER)
        || test_command.as_deref() == Some(AGENTS_TEST_COMMAND_PLACEHOLDER)
        || lint_or_format_command.as_deref() == Some(AGENTS_LINT_COMMAND_PLACEHOLDER)
        || command_values
            .iter()
            .flatten()
            .any(|value| is_unresolved_agents_command(value))
    {
        AgentsCommandStatus::Placeholder
    } else {
        AgentsCommandStatus::Concrete
    };

    AgentsScaffoldInspection {
        status: AgentsScaffoldStatus::Present,
        build_command,
        test_command,
        lint_or_format_command,
        command_status,
    }
}

fn validate_source_skills_root(candidate: &Path) -> Result<PathBuf> {
    if !candidate.is_dir() {
        bail!("{} is not a directory", candidate.display());
    }
    let missing = MANAGED_SKILLS
        .iter()
        .filter(|skill| !candidate.join(skill).join("SKILL.md").is_file())
        .map(|skill| (*skill).to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "{} is not a valid Codex1 skill root; missing managed skills: {}",
            candidate.display(),
            missing.join(", ")
        );
    }
    Ok(candidate.to_path_buf())
}

fn managed_prefix_exists(root: &Path, relative_path: &Path) -> bool {
    relative_path
        .components()
        .next()
        .is_some_and(|component| root.join(component.as_os_str()).exists())
}

fn target_has_any_managed_path(root: &Path, source_relatives: &[PathBuf]) -> Result<bool> {
    if !root.is_dir() {
        return Ok(false);
    }
    for relative in source_relatives {
        if root.join(relative).is_file() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn hash_named_bytes(hasher: &mut Sha256, label: &str, bytes: Option<&[u8]>) {
    hasher.update(label.as_bytes());
    hasher.update(b"\n");
    match bytes {
        Some(bytes) => hasher.update(bytes),
        None => hasher.update(b"absent"),
    }
    hasher.update(b"\n");
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;
    use walkdir::WalkDir;

    use super::{
        AGENTS_BLOCK, AGENTS_BUILD_COMMAND_PLACEHOLDER, AGENTS_LINT_COMMAND_PLACEHOLDER,
        AGENTS_TEST_COMMAND_PLACEHOLDER, AgentsCommandStatus, AgentsScaffoldStatus,
        LEGACY_AGENTS_BLOCK_BEGIN, LEGACY_AGENTS_BLOCK_END, MANAGED_SKILLS,
        MANAGED_STOP_HOOK_STATUS, SkillInstallMode, SkillSurfaceStatus,
        compute_support_surface_signature, extract_managed_agents_block,
        inspect_agents_scaffold_details, inspect_skill_surface_with_source,
        is_managed_stop_handler, is_observational_stop_handler,
        summarize_stop_authority_with_observational,
    };
    use serde_json::json;

    fn seed_source_skills(root: &Path) {
        for skill in MANAGED_SKILLS {
            let dir = root.join(skill);
            fs::create_dir_all(&dir).expect("create skill dir");
            fs::write(dir.join("SKILL.md"), format!("# {skill}\n")).expect("write skill file");
        }
    }

    #[test]
    fn detects_missing_surface() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::Missing);
    }

    #[test]
    fn detects_valid_existing_surface() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());
        for entry in WalkDir::new(source.path()) {
            let entry = entry.expect("walk source skills");
            let relative = entry
                .path()
                .strip_prefix(source.path())
                .expect("relative path");
            let destination = target.path().join(".codex/skills").join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&destination).expect("create destination dir");
            } else {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).expect("create destination parent");
                }
                fs::copy(entry.path(), &destination).expect("copy skill file");
            }
        }

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::ValidExisting);
        assert_eq!(
            inspection.install_mode,
            Some(SkillInstallMode::CopiedSkills)
        );
    }

    #[test]
    fn detects_linked_skill_surface() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        #[cfg(unix)]
        std::os::unix::fs::symlink(source.path(), codex_dir.join("skills")).expect("link skills");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(source.path(), codex_dir.join("skills"))
            .expect("link skills");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::ValidExisting);
        assert_eq!(
            inspection.install_mode,
            Some(SkillInstallMode::LinkedSkills)
        );
    }

    #[test]
    fn detects_skills_config_bridge_surface() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        fs::write(
            codex_dir.join("config.toml"),
            format!(
                "[[skills.config]]\npath = \"{}\"\nenabled = true\n",
                source.path().display()
            ),
        )
        .expect("write config");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::ValidExisting);
        assert_eq!(
            inspection.install_mode,
            Some(SkillInstallMode::SkillsConfigBridge)
        );
        assert_eq!(
            inspection.discovery_root,
            source.path().canonicalize().expect("canonical source")
        );
    }

    #[test]
    fn detects_invalid_skills_config_bridge_without_aborting() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        fs::write(
            codex_dir.join("config.toml"),
            "[[skills.config]]\npath = \"./missing-skills\"\nenabled = true\n",
        )
        .expect("write config");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::InvalidBridge);
        assert_eq!(
            inspection.install_mode,
            Some(SkillInstallMode::SkillsConfigBridge)
        );
        assert!(inspection.bridge_error.is_some());
    }

    #[test]
    fn detects_incomplete_skills_config_bridge_as_invalid() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());
        seed_source_skills(&target.path().join(".codex/skills"));

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        fs::write(
            codex_dir.join("config.toml"),
            "[[skills.config]]\npath = \"./missing-skills\"\n",
        )
        .expect("write config");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::InvalidBridge);
    }

    #[test]
    fn parse_errors_do_not_hide_uncommented_skills_config_bridge_headers() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());
        seed_source_skills(&target.path().join(".codex/skills"));

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        fs::write(
            codex_dir.join("config.toml"),
            "[[skills.config]]\npath = \"./missing-skills\nenabled = true\n",
        )
        .expect("write config");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::InvalidBridge);
    }

    #[test]
    fn parse_errors_do_not_hide_commented_skills_config_bridge_headers() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        seed_source_skills(source.path());
        seed_source_skills(&target.path().join(".codex/skills"));

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        fs::write(
            codex_dir.join("config.toml"),
            "[[skills.config]] # keep\npath = \"./missing-skills\nenabled = true\n",
        )
        .expect("write config");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::InvalidBridge);
    }

    #[test]
    fn valid_bridge_wins_after_earlier_invalid_entry() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        let bridge_root = target.path().join("shared-skills");
        seed_source_skills(source.path());
        seed_source_skills(&bridge_root);

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        fs::write(
            codex_dir.join("config.toml"),
            format!(
                "[[skills.config]]\npath = 42\nenabled = true\n\n[[skills.config]]\npath = \"{}\"\nenabled = true\n",
                bridge_root.display()
            ),
        )
        .expect("write config");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::ValidExisting);
        assert_eq!(
            inspection.discovery_root,
            bridge_root.canonicalize().expect("canonical bridge root")
        );
    }

    #[test]
    fn skills_config_bridge_preserves_hash_in_quoted_path() {
        let source = TempDir::new().expect("source temp dir");
        let target = TempDir::new().expect("target temp dir");
        let bridge_root = target.path().join("skills#v2");
        seed_source_skills(source.path());
        seed_source_skills(&bridge_root);

        let codex_dir = target.path().join(".codex");
        fs::create_dir_all(&codex_dir).expect("create .codex");
        fs::write(
            codex_dir.join("config.toml"),
            format!(
                "[[skills.config]]\npath = \"{}\"\nenabled = true\n",
                bridge_root.display()
            ),
        )
        .expect("write config");

        let inspection =
            inspect_skill_surface_with_source(target.path(), source.path()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::ValidExisting);
        assert_eq!(
            inspection.discovery_root,
            bridge_root.canonicalize().expect("canonical bridge root")
        );
        assert_eq!(
            inspection.install_mode,
            Some(SkillInstallMode::SkillsConfigBridge)
        );
    }

    #[test]
    fn agents_scaffold_details_distinguish_placeholders_from_concrete_commands() {
        let placeholder = inspect_agents_scaffold_details(Some(AGENTS_BLOCK));
        assert_eq!(placeholder.status, AgentsScaffoldStatus::Present);
        assert_eq!(placeholder.command_status, AgentsCommandStatus::Placeholder);
        assert_eq!(
            placeholder.build_command.as_deref(),
            Some(AGENTS_BUILD_COMMAND_PLACEHOLDER)
        );
        assert_eq!(
            placeholder.test_command.as_deref(),
            Some(AGENTS_TEST_COMMAND_PLACEHOLDER)
        );
        assert_eq!(
            placeholder.lint_or_format_command.as_deref(),
            Some(AGENTS_LINT_COMMAND_PLACEHOLDER)
        );

        let concrete = inspect_agents_scaffold_details(Some(
            &AGENTS_BLOCK
                .replace(AGENTS_BUILD_COMMAND_PLACEHOLDER, "cargo build -p codex1")
                .replace(AGENTS_TEST_COMMAND_PLACEHOLDER, "cargo test -p codex1")
                .replace(AGENTS_LINT_COMMAND_PLACEHOLDER, "cargo fmt --all --check"),
        ));
        assert_eq!(concrete.status, AgentsScaffoldStatus::Present);
        assert_eq!(concrete.command_status, AgentsCommandStatus::Concrete);
    }

    #[test]
    fn legacy_noop_agents_commands_stay_placeholder() {
        let inspection = inspect_agents_scaffold_details(Some(
            &AGENTS_BLOCK
                .replace(
                    AGENTS_BUILD_COMMAND_PLACEHOLDER,
                    "true # no dedicated build command detected",
                )
                .replace(
                    AGENTS_TEST_COMMAND_PLACEHOLDER,
                    "true # no dedicated test command detected",
                )
                .replace(
                    AGENTS_LINT_COMMAND_PLACEHOLDER,
                    "true # no dedicated lint-or-format command detected",
                ),
        ));
        assert_eq!(inspection.status, AgentsScaffoldStatus::Present);
        assert_eq!(inspection.command_status, AgentsCommandStatus::Placeholder);
    }

    #[test]
    fn codex1_fallback_agents_commands_stay_placeholder() {
        let inspection = inspect_agents_scaffold_details(Some(
            &AGENTS_BLOCK
                .replace(
                    AGENTS_BUILD_COMMAND_PLACEHOLDER,
                    "true # codex1 fallback: build command not auto-detected",
                )
                .replace(
                    AGENTS_TEST_COMMAND_PLACEHOLDER,
                    "true # codex1 fallback: test command not auto-detected",
                )
                .replace(
                    AGENTS_LINT_COMMAND_PLACEHOLDER,
                    "true # codex1 fallback: lint-or-format command not auto-detected",
                ),
        ));
        assert_eq!(inspection.status, AgentsScaffoldStatus::Present);
        assert_eq!(inspection.command_status, AgentsCommandStatus::Placeholder);
    }

    #[test]
    fn agents_scaffold_details_accept_legacy_markers() {
        let legacy_block = AGENTS_BLOCK
            .replace("<!-- codex1:begin -->", LEGACY_AGENTS_BLOCK_BEGIN)
            .replace("<!-- codex1:end -->", LEGACY_AGENTS_BLOCK_END);
        let inspection = inspect_agents_scaffold_details(Some(&legacy_block));
        assert_eq!(inspection.status, AgentsScaffoldStatus::Present);
        assert_eq!(inspection.command_status, AgentsCommandStatus::Placeholder);
    }

    #[test]
    fn legacy_marker_mentions_outside_current_block_do_not_break_detection() {
        let contents = format!(
            "{}\n\nMigration note: keep `{}` and `{}` in historical docs.\n",
            AGENTS_BLOCK, LEGACY_AGENTS_BLOCK_BEGIN, LEGACY_AGENTS_BLOCK_END
        );
        let inspection = inspect_agents_scaffold_details(Some(&contents));
        assert_eq!(inspection.status, AgentsScaffoldStatus::Present);
        assert_eq!(inspection.command_status, AgentsCommandStatus::Placeholder);
    }

    #[test]
    fn extract_managed_agents_block_handles_crlf_line_endings() {
        let crlf = AGENTS_BLOCK.replace('\n', "\r\n");
        let extracted = extract_managed_agents_block(&crlf).expect("extract managed block");
        assert_eq!(extracted, AGENTS_BLOCK);
    }

    #[test]
    fn signature_changes_when_skill_root_changes() {
        let temp = TempDir::new().expect("temp dir");
        let skill_root = temp.path().join("skills");
        seed_source_skills(&skill_root);

        let first = compute_support_surface_signature(
            Some("model = \"gpt-5.4\"\n"),
            Some("{}\n"),
            Some("{}\n"),
            Some("<!-- codex1:begin -->x<!-- codex1:end -->"),
            &skill_root,
        )
        .expect("compute signature");
        fs::write(skill_root.join("clarify/SKILL.md"), "# changed\n").expect("mutate skill");
        let second = compute_support_surface_signature(
            Some("model = \"gpt-5.4\"\n"),
            Some("{}\n"),
            Some("{}\n"),
            Some("<!-- codex1:begin -->x<!-- codex1:end -->"),
            &skill_root,
        )
        .expect("compute signature");
        assert_ne!(first, second);
    }

    #[test]
    fn signature_ignores_unmanaged_extra_skills() {
        let temp = TempDir::new().expect("temp dir");
        let skill_root = temp.path().join("skills");
        seed_source_skills(&skill_root);

        let first = compute_support_surface_signature(
            Some("model = \"gpt-5.4\"\n"),
            Some("{}\n"),
            Some("{}\n"),
            Some("<!-- codex1:begin -->x<!-- codex1:end -->"),
            &skill_root,
        )
        .expect("compute signature");

        let extra_root = skill_root.join("user-owned-skill");
        fs::create_dir_all(&extra_root).expect("create unmanaged skill dir");
        fs::write(extra_root.join("SKILL.md"), "# extra\n").expect("write unmanaged skill");

        let second = compute_support_surface_signature(
            Some("model = \"gpt-5.4\"\n"),
            Some("{}\n"),
            Some("{}\n"),
            Some("<!-- codex1:begin -->x<!-- codex1:end -->"),
            &skill_root,
        )
        .expect("compute signature");
        assert_eq!(first, second);
    }

    #[test]
    fn managed_stop_handler_requires_an_executable_binary() {
        let temp = TempDir::new().expect("temp dir");
        let binary = temp.path().join("codex1");
        fs::write(&binary, "not executable\n").expect("write fake codex1");

        let hook = json!({
            "type": "command",
            "command": format!("{} internal stop-hook", binary.display()),
            "statusMessage": MANAGED_STOP_HOOK_STATUS
        });

        assert!(!is_managed_stop_handler(&hook));
    }

    #[test]
    fn managed_stop_hooks_cannot_self_label_as_observational() {
        let hook = json!({
            "type": "command",
            "command": "codex1 internal stop-hook",
            "statusMessage": MANAGED_STOP_HOOK_STATUS,
            "codex1_observational": true
        });

        assert!(is_managed_stop_handler(&hook));
        assert!(!is_observational_stop_handler(&hook));

        let counts = summarize_stop_authority_with_observational(&json!({
            "hooks": {
                "Stop": [hook]
            }
        }));
        assert_eq!(counts.total, 1);
        assert_eq!(counts.managed, 1);
        assert_eq!(counts.observational, 0);
        assert_eq!(counts.authoritative(), 1);
    }
}
