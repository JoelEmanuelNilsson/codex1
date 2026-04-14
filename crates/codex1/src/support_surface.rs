use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
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
pub const AGENTS_BLOCK: &str = "<!-- codex1:begin -->\n## Codex1\n### Workflow Stance\n- Use the native Codex skills surface for `clarify`, `plan`, `execute`, `review`, and `autopilot`.\n- Keep mission truth in visible repo artifacts instead of hidden chat state.\n- Replan stays internal unless the repo truth explicitly says otherwise.\n\n### Quality Bar\n- Work is complete only when the locked outcome, proof, review, and closeout contracts are all satisfied.\n- Review is mandatory before mission completion.\n- Hold the repo to production-grade changes with explicit validation and review-clean closeout.\n\n### Repo Commands\n- Build: document the repo-specific build command before relying on autopilot.\n- Test: document the repo-specific test command before relying on autopilot.\n- Lint or format: document the repo-specific lint or format command before relying on autopilot.\n\n### Artifact Conventions\n- Mission packages live under `PLANS/<mission-id>/`.\n- `OUTCOME-LOCK.md` is canonical for destination truth.\n- `PROGRAM-BLUEPRINT.md` is canonical for route truth.\n- `specs/*/SPEC.md` is canonical for one bounded execution slice.\n<!-- codex1:end -->\n";

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
}

#[derive(Debug, Clone)]
pub struct ManagedSkillFile {
    pub relative_path: PathBuf,
    pub contents: Vec<u8>,
}

pub fn default_skill_root(repo_root: &Path) -> PathBuf {
    repo_root.join(".codex/skills")
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

pub fn inspect_skill_surface_with_source(
    repo_root: &Path,
    source_root: &Path,
) -> Result<SkillSurfaceInspection> {
    let root = default_skill_root(repo_root);
    let (discovery_root, install_mode) = resolve_skill_surface_root(repo_root, &root)?;
    let managed_files = managed_skill_files(source_root)?;
    let source_relatives: Vec<PathBuf> = managed_files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect();

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
    })
}

fn resolve_skill_surface_root(
    repo_root: &Path,
    default_root: &Path,
) -> Result<(PathBuf, Option<SkillInstallMode>)> {
    if let Some(bridge_root) = resolve_skills_config_bridge_root(repo_root)? {
        return Ok((bridge_root, Some(SkillInstallMode::SkillsConfigBridge)));
    }

    if default_root.exists() {
        let install_mode = match fs::symlink_metadata(default_root) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Some(SkillInstallMode::LinkedSkills)
            }
            Ok(_) => Some(SkillInstallMode::CopiedSkills),
            Err(_) => None,
        };
        return Ok((default_root.to_path_buf(), install_mode));
    }

    Ok((default_root.to_path_buf(), None))
}

fn resolve_skills_config_bridge_root(repo_root: &Path) -> Result<Option<PathBuf>> {
    let config_path = repo_root.join(".codex/config.toml");
    let Ok(raw) = fs::read_to_string(&config_path) else {
        return Ok(None);
    };

    let mut in_bridge = false;
    let mut enabled = false;
    let mut path = None::<String>;
    for line in raw.lines() {
        let trimmed = strip_comment(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
            if in_bridge
                && enabled
                && let Some(path) = path.take()
            {
                return resolve_bridge_candidate(&config_path, repo_root, &path).map(Some);
            }
            in_bridge = trimmed == "[[skills.config]]";
            enabled = false;
            path = None;
            continue;
        }
        if !in_bridge {
            continue;
        }
        if let Some(value) = parse_key_value(trimmed, "path") {
            path = Some(value);
        } else if let Some(value) = parse_key_value(trimmed, "enabled") {
            enabled = value.eq_ignore_ascii_case("true");
        }
    }

    if in_bridge
        && enabled
        && let Some(path) = path
    {
        return resolve_bridge_candidate(&config_path, repo_root, &path).map(Some);
    }

    Ok(None)
}

fn resolve_bridge_candidate(
    config_path: &Path,
    repo_root: &Path,
    raw_path: &str,
) -> Result<PathBuf> {
    let candidate = PathBuf::from(raw_path);
    let candidates = if candidate.is_absolute() {
        vec![candidate]
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
            validate_source_skills_root(&canonical)?;
            return Ok(canonical);
        }
    }

    bail!(
        "skills.config bridge points to {}, but no valid Codex1 skill root was found",
        raw_path
    )
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

pub fn extract_managed_block(contents: &str, begin: &str, end: &str) -> Option<String> {
    let begin_index = contents.find(begin)?;
    let end_index = contents.find(end)?;
    if end_index < begin_index {
        return None;
    }
    let end_index = end_index + end.len();
    Some(contents[begin_index..end_index].to_string())
}

fn parse_key_value(line: &str, key: &str) -> Option<String> {
    let trimmed = strip_comment(line).trim();
    if trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let (candidate, value) = trimmed.split_once('=')?;
    if candidate.trim() != key {
        return None;
    }
    Some(value.trim().trim_matches('"').to_string())
}

fn strip_comment(line: &str) -> &str {
    match line.split_once('#') {
        Some((before, _)) => before,
        None => line,
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
        MANAGED_SKILLS, MANAGED_STOP_HOOK_STATUS, SkillInstallMode, SkillSurfaceStatus,
        compute_support_surface_signature, inspect_skill_surface_with_source,
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
