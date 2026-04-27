use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use toml_edit::DocumentMut;

use crate::cli::{
    SetupBackupRestoreArgs, SetupBackupsCommand, SetupCommand, SetupInstallArgs, SetupMigrateArgs,
    SetupModeArg, SetupRepoArgs, SetupScopeArg, SetupUninstallArgs,
};
use crate::envelope;
use crate::error::{Codex1Error, IoContext, Result};
use crate::paths::{discover_repo_root, ensure_contained_for_write};

const CONFIG_FILE: &str = "config.toml";
const BACKUP_MANIFEST_VERSION: u32 = 1;
const BUNDLE_VERSION: u32 = 1;
const MANAGED_HOOK_START: &str = "# codex1-managed-ralph-start";
const MANAGED_HOOK_END: &str = "# codex1-managed-ralph-end";
const MANAGED_HOOK_STATUS: &str = "Codex1 Ralph";
const MANAGED_GUIDANCE_START: &str = "<!-- codex1-managed setup guidance start -->";
const MANAGED_GUIDANCE_END: &str = "<!-- codex1-managed setup guidance end -->";
const BUNDLE_SKILL: &str = ".agents/skills/codex1/SKILL.md";
const BUNDLE_GUIDANCE: &str = "AGENTS.md";
const BUNDLE_MARKER: &str = ".codex1/setup-bundle.json";
const MANAGED_BUNDLE_FILES: [&str; 2] = [BUNDLE_SKILL, BUNDLE_GUIDANCE];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SetupScope {
    Global,
    Project,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivationMode {
    Off,
    Allowlist,
    Denylist,
    All,
}

impl ActivationMode {
    fn from_arg(value: Option<SetupModeArg>) -> Self {
        match value {
            Some(SetupModeArg::Off) => Self::Off,
            Some(SetupModeArg::Allowlist) | None => Self::Allowlist,
            Some(SetupModeArg::Denylist) => Self::Denylist,
            Some(SetupModeArg::All) => Self::All,
        }
    }
}

impl SetupScope {
    fn from_arg(value: Option<SetupScopeArg>) -> Self {
        match value {
            Some(SetupScopeArg::Project) => Self::Project,
            Some(SetupScopeArg::Global) | None => Self::Global,
        }
    }

    fn hook_arg(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoPolicyEntry {
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationPolicy {
    pub mode: ActivationMode,
    #[serde(default)]
    pub repos: Vec<RepoPolicyEntry>,
}

impl Default for ActivationPolicy {
    fn default() -> Self {
        Self {
            mode: ActivationMode::Allowlist,
            repos: Vec::new(),
        }
    }
}

impl ActivationPolicy {
    pub fn effective_for(&self, repo: &Path) -> bool {
        let explicit = self
            .repos
            .iter()
            .rev()
            .find(|entry| entry.path == repo)
            .map(|entry| entry.enabled);
        match self.mode {
            ActivationMode::Off => false,
            ActivationMode::Allowlist => explicit.unwrap_or(false),
            ActivationMode::Denylist => explicit.unwrap_or(true),
            ActivationMode::All => true,
        }
    }

    fn set_repo(&mut self, repo: PathBuf, enabled: bool) {
        self.repos.retain(|entry| entry.path != repo);
        self.repos.push(RepoPolicyEntry {
            path: repo,
            enabled,
        });
        self.repos.sort_by(|left, right| left.path.cmp(&right.path));
    }

    fn disable_repo(&mut self, repo: PathBuf) {
        if matches!(self.mode, ActivationMode::All) {
            self.mode = ActivationMode::Denylist;
        }
        self.set_repo(repo, false);
    }

    fn has_any_global_activation(&self) -> bool {
        match self.mode {
            ActivationMode::Off => false,
            ActivationMode::Allowlist => self.repos.iter().any(|entry| entry.enabled),
            ActivationMode::Denylist | ActivationMode::All => true,
        }
    }

    fn to_toml(&self) -> String {
        let mut text = format!("mode = \"{}\"\n", mode_name(self.mode));
        for entry in &self.repos {
            text.push_str("\n[[repos]]\n");
            text.push_str(&format!("path = \"{}\"\n", toml_escape_path(&entry.path)));
            text.push_str(&format!("enabled = {}\n", entry.enabled));
        }
        text
    }
}

#[derive(Clone, Debug)]
pub struct SetupPaths {
    pub codex_home: PathBuf,
    pub codex_config: PathBuf,
    pub codex1_home: PathBuf,
    pub codex1_config: PathBuf,
    pub backups_dir: PathBuf,
    pub backup_manifest: PathBuf,
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
    pub codex_home: PathBuf,
    pub codex_config: PathBuf,
    pub codex1_home: PathBuf,
    pub global_config_found: bool,
    pub global_config_parseable: bool,
    pub activation_mode: ActivationMode,
    pub repo_policy_enabled: bool,
    pub effective_active: bool,
    pub global_hook_installed: bool,
    pub project_hook_installed: bool,
    pub repo_bundle_materialized: bool,
    pub duplicate_hook_risk: bool,
    pub backups_available: usize,
    pub project_trust_caveat: bool,
    pub warnings: Vec<String>,
    pub anti_oracle: &'static str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BundleMarker {
    managed_by: String,
    version: u32,
    files: Vec<String>,
}

pub fn run(cli_json: bool, global_repo: Option<PathBuf>, command: SetupCommand) -> Result<()> {
    match command {
        SetupCommand::Install(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, install(args)?)
        }
        SetupCommand::Enable(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, enable(args)?)
        }
        SetupCommand::Disable(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, disable(args)?)
        }
        SetupCommand::Uninstall(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, uninstall(args)?)
        }
        SetupCommand::Migrate(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, migrate(args)?)
        }
        SetupCommand::Status(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, status_value(args.repo)?)
        }
        SetupCommand::Doctor(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, doctor(args.repo)?)
        }
        SetupCommand::Backups { command } => match command {
            SetupBackupsCommand::List => emit(cli_json, backups_list()?),
            SetupBackupsCommand::Restore(args) => {
                emit(cli_json, backups_restore(global_repo, args)?)
            }
        },
    }
}

pub fn ralph_should_scan_repo(repo: &Path, project_hook: bool) -> bool {
    if env::var_os("CODEX1_DOCTOR_SMOKE").is_some() {
        return true;
    }
    if project_hook {
        return repo_bundle_materialized(repo);
    }
    let Ok(paths) = resolve_paths() else {
        return false;
    };
    if !paths.codex1_config.exists() {
        return managed_hook_count(&paths.codex_config) == 0;
    }
    let Ok(policy) = read_policy(&paths) else {
        return false;
    };
    global_policy_should_scan_repo(&policy, repo)
}

fn global_policy_should_scan_repo(policy: &ActivationPolicy, repo: &Path) -> bool {
    if !policy.effective_for(repo) {
        return false;
    }
    match policy.mode {
        ActivationMode::Off => false,
        ActivationMode::Allowlist => repo_bundle_materialized(repo),
        ActivationMode::Denylist | ActivationMode::All => true,
    }
}

fn known_policy_repos(
    paths: &SetupPaths,
    policy: &ActivationPolicy,
    fallback: &Path,
) -> Vec<PathBuf> {
    let mut repos = vec![fallback.to_path_buf()];
    for entry in &policy.repos {
        if !repos.contains(&entry.path) {
            repos.push(entry.path.clone());
        }
    }
    if let Ok(manifest) = read_manifest(paths) {
        for record in manifest.records {
            if let Some(repo) = project_config_repo(&record.target_path) {
                if !repos.contains(&repo) {
                    repos.push(repo);
                }
            }
        }
    }
    repos
}

fn remove_repo_scoped_setup(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    if !repo.exists() {
        return Ok(());
    }
    remove_bundle(repo, plan, dry_run)?;
    remove_project_hook(repo, plan, dry_run)
}

fn project_setup_recorded(paths: &SetupPaths, repo: &Path) -> bool {
    read_manifest(paths)
        .map(|manifest| {
            manifest
                .records
                .iter()
                .filter_map(|record| project_config_repo(&record.target_path))
                .any(|record_repo| record_repo == repo)
        })
        .unwrap_or(false)
}

fn dedup_paths(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

fn global_setup_remains_for_repo(paths: &SetupPaths, repo: &Path) -> Result<bool> {
    if managed_hook_count_parseable(&paths.codex_config) == 0 {
        return Ok(false);
    }
    if !managed_hook_executable_ok(&paths.codex_config, SetupScope::Global) {
        return Ok(false);
    }
    let policy = read_policy_or_default(paths)?;
    Ok(global_policy_should_scan_repo(&policy, repo))
}

fn global_policy_enables_repo(paths: &SetupPaths, repo: &Path) -> Result<bool> {
    if !paths.codex1_config.exists() {
        return Ok(false);
    }
    let policy = read_policy_or_default(paths)?;
    Ok(global_policy_should_scan_repo(&policy, repo))
}

fn remove_project_hook(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    let path = project_config_path(repo);
    if !path.exists() {
        return Ok(());
    }
    remove_managed_hook(
        &project_config_path_checked(repo)?,
        plan,
        dry_run,
        "project hook",
    )
}

fn preflight_remove_project_hook(repo: &Path) -> Result<()> {
    let path = project_config_path(repo);
    if !path.exists() {
        return Ok(());
    }
    preflight_remove_managed_hook(&project_config_path_checked(repo)?)
}

fn preflight_remove_bundle(repo: &Path) -> Result<()> {
    validate_bundle_removal(repo).map(|_| ())
}

fn validate_bundle_removal(repo: &Path) -> Result<Option<Vec<String>>> {
    let marker = repo_bundle_target(repo, BUNDLE_MARKER)?;
    let marker_text = match fs::read_to_string(&marker) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                marker.display()
            )))
        }
    };
    let marker_data = parse_bundle_marker(&marker, &marker_text)?;
    if marker_data.managed_by != "codex1-managed" || marker_data.version != BUNDLE_VERSION {
        return Err(Codex1Error::SetupBundle(format!(
            "invalid Codex1 bundle marker {}",
            marker.display()
        )));
    }
    if marker_text != bundle_marker_body() {
        return Err(Codex1Error::SetupBundle(format!(
            "refusing to remove non-managed file {}",
            marker.display()
        )));
    }
    for relative in &marker_data.files {
        if !MANAGED_BUNDLE_FILES.contains(&relative.as_str()) {
            return Err(Codex1Error::SetupBundle(format!(
                "bundle marker contains unmanaged file {}",
                relative
            )));
        }
        let path = repo_bundle_target(repo, relative)?;
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(Codex1Error::SetupBundle(format!(
                    "failed to read {}: {error}",
                    path.display()
                )))
            }
        };
        let owned = if relative == BUNDLE_GUIDANCE {
            guidance_has_managed_block(&text) || text == guidance_body()
        } else {
            text == expected_bundle_body(relative)
        };
        if !owned {
            return Err(Codex1Error::SetupBundle(format!(
                "refusing to remove non-managed file {}",
                path.display()
            )));
        }
    }
    Ok(Some(marker_data.files))
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

fn install(args: SetupInstallArgs) -> Result<serde_json::Value> {
    let scope = SetupScope::from_arg(args.scope);
    let mode = ActivationMode::from_arg(args.mode);
    if scope == SetupScope::Project && mode == ActivationMode::Off {
        return Err(Codex1Error::SetupArgument(
            "setup install --scope project cannot use --mode off".into(),
        ));
    }
    let repo = resolve_repo(args.repo)?;
    let mut plan = SetupPlan::new(args.dry_run);
    match scope {
        SetupScope::Global => {
            let paths = resolve_paths()?;
            let mut policy = read_policy_or_default(&paths)?;
            let cleanup_repos = if matches!(mode, ActivationMode::Off) {
                known_policy_repos(&paths, &policy, &repo)
            } else {
                Vec::new()
            };
            policy.mode = mode;
            if matches!(
                policy.mode,
                ActivationMode::Allowlist | ActivationMode::Denylist | ActivationMode::All
            ) {
                policy.set_repo(repo.clone(), true);
            }
            if policy.effective_for(&repo) {
                let had_bundle = repo_bundle_materialized(&repo);
                let had_global_hook = managed_hook_count_parseable(&paths.codex_config) > 0;
                let original_policy = read_optional_text(&paths.codex1_config)?;
                preflight_materialize_bundle(&repo)?;
                preflight_write_policy(&paths)?;
                preflight_install_managed_hook(&paths.codex_config, SetupScope::Global)?;
                preflight_remove_project_hook(&repo)?;
                if let Err(error) = materialize_bundle(&repo, &mut plan, args.dry_run)
                    .and_then(|()| write_policy(&paths, &policy, &mut plan, args.dry_run))
                    .and_then(|()| {
                        install_managed_hook(
                            &paths.codex_config,
                            SetupScope::Global,
                            &mut plan,
                            args.dry_run,
                            "global hook",
                        )
                    })
                    .and_then(|()| remove_project_hook(&repo, &mut plan, args.dry_run))
                {
                    if !args.dry_run {
                        let _ = restore_text(&paths.codex1_config, original_policy.as_deref());
                        if !had_global_hook {
                            let _ = remove_managed_hook(
                                &paths.codex_config,
                                &mut SetupPlan::new(false),
                                false,
                                "global hook rollback",
                            );
                        }
                    }
                    if !args.dry_run && !had_bundle {
                        let _ = remove_bundle(&repo, &mut SetupPlan::new(false), false);
                    }
                    return Err(error);
                }
            } else {
                for repo in &cleanup_repos {
                    preflight_remove_bundle(repo)?;
                    preflight_remove_project_hook(repo)?;
                }
                if !matches!(mode, ActivationMode::Off) {
                    preflight_remove_bundle(&repo)?;
                }
                write_policy(&paths, &policy, &mut plan, args.dry_run)?;
                for repo in cleanup_repos {
                    remove_repo_scoped_setup(&repo, &mut plan, args.dry_run)?;
                }
                if !matches!(mode, ActivationMode::Off) {
                    remove_bundle(&repo, &mut plan, args.dry_run)?;
                }
            }
        }
        SetupScope::Project => {
            let paths = resolve_paths()?;
            let project_config = project_config_path_checked(&repo)?;
            preflight_materialize_bundle(&repo)?;
            let suppress_global = global_policy_enables_repo(&paths, &repo)?;
            let original_policy = if suppress_global {
                Some(read_optional_text(&paths.codex1_config)?)
            } else {
                None
            };
            let mut suppressed_policy = if suppress_global {
                let mut policy = read_policy_or_default(&paths)?;
                policy.disable_repo(repo.clone());
                preflight_write_policy(&paths)?;
                Some(policy)
            } else {
                None
            };
            let had_bundle = repo_bundle_materialized(&repo);
            let had_project_hook = managed_hook_count_parseable(&project_config) > 0;
            let install_result = install_managed_hook(
                &project_config,
                SetupScope::Project,
                &mut plan,
                args.dry_run,
                "project hook",
            );
            if let Err(error) = install_result
                .and_then(|()| materialize_bundle(&repo, &mut plan, args.dry_run))
                .and_then(|()| {
                    if let Some(policy) = suppressed_policy.take() {
                        write_policy(&paths, &policy, &mut plan, args.dry_run)
                    } else {
                        Ok(())
                    }
                })
            {
                if !args.dry_run {
                    if let Some(original_policy) = original_policy.as_ref() {
                        let _ = restore_text(&paths.codex1_config, original_policy.as_deref());
                    }
                    if !had_project_hook {
                        let _ = remove_managed_hook(
                            &project_config,
                            &mut SetupPlan::new(false),
                            false,
                            "project hook",
                        );
                    }
                    if !had_bundle {
                        let _ = remove_bundle(&repo, &mut SetupPlan::new(false), false);
                    }
                }
                return Err(error);
            }
        }
    }
    Ok(json!({
        "summary": format!("setup install planned/applied for {}", repo.display()),
        "command": "setup install",
        "scope": scope,
        "activation_mode": mode,
        "repo": repo,
        "plan": plan,
    }))
}

fn enable(args: SetupRepoArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let paths = resolve_paths()?;
    if !paths.codex1_config.exists() {
        let project_config = project_config_path(&repo);
        if managed_hook_count_parseable(&project_config) > 0
            || project_setup_recorded(&paths, &repo)
        {
            return install(SetupInstallArgs {
                mode: Some(SetupModeArg::Allowlist),
                scope: Some(SetupScopeArg::Project),
                repo: Some(repo),
                dry_run: args.dry_run,
            });
        }
        return install(SetupInstallArgs {
            mode: Some(SetupModeArg::Allowlist),
            scope: Some(SetupScopeArg::Global),
            repo: Some(repo),
            dry_run: args.dry_run,
        });
    }
    let mut plan = SetupPlan::new(args.dry_run);
    let mut policy = read_policy_or_default(&paths)?;
    if matches!(policy.mode, ActivationMode::Off) {
        policy.mode = ActivationMode::Allowlist;
    }
    policy.set_repo(repo.clone(), true);
    let had_bundle = repo_bundle_materialized(&repo);
    let had_global_hook = managed_hook_count_parseable(&paths.codex_config) > 0;
    let original_policy = read_optional_text(&paths.codex1_config)?;
    preflight_materialize_bundle(&repo)?;
    preflight_write_policy(&paths)?;
    preflight_install_managed_hook(&paths.codex_config, SetupScope::Global)?;
    preflight_remove_project_hook(&repo)?;
    if let Err(error) = materialize_bundle(&repo, &mut plan, args.dry_run)
        .and_then(|()| write_policy(&paths, &policy, &mut plan, args.dry_run))
        .and_then(|()| {
            install_managed_hook(
                &paths.codex_config,
                SetupScope::Global,
                &mut plan,
                args.dry_run,
                "global hook",
            )
        })
        .and_then(|()| remove_project_hook(&repo, &mut plan, args.dry_run))
    {
        if !args.dry_run {
            let _ = restore_text(&paths.codex1_config, original_policy.as_deref());
            if !had_global_hook {
                let _ = remove_managed_hook(
                    &paths.codex_config,
                    &mut SetupPlan::new(false),
                    false,
                    "global hook rollback",
                );
            }
        }
        if !args.dry_run && !had_bundle {
            let _ = remove_bundle(&repo, &mut SetupPlan::new(false), false);
        }
        return Err(error);
    }
    Ok(json!({
        "summary": format!("setup enabled for {}", repo.display()),
        "command": "setup enable",
        "repo": repo,
        "plan": plan,
    }))
}

fn disable(args: SetupRepoArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let paths = resolve_paths()?;
    let mut plan = SetupPlan::new(args.dry_run);
    if !paths.codex1_config.exists()
        && managed_hook_count_parseable(&project_config_path(&repo)) > 0
    {
        preflight_remove_bundle(&repo)?;
        preflight_remove_project_hook(&repo)?;
        remove_bundle(&repo, &mut plan, args.dry_run)?;
        remove_project_hook(&repo, &mut plan, args.dry_run)?;
        return Ok(json!({
            "summary": format!("setup disabled for {}", repo.display()),
            "command": "setup disable",
            "repo": repo,
            "plan": plan,
        }));
    }
    let mut policy = read_policy_or_default(&paths)?;
    policy.disable_repo(repo.clone());
    preflight_remove_bundle(&repo)?;
    preflight_remove_project_hook(&repo)?;
    write_policy(&paths, &policy, &mut plan, args.dry_run)?;
    remove_bundle(&repo, &mut plan, args.dry_run)?;
    remove_project_hook(&repo, &mut plan, args.dry_run)?;
    Ok(json!({
        "summary": format!("setup disabled for {}", repo.display()),
        "command": "setup disable",
        "repo": repo,
        "plan": plan,
    }))
}

fn uninstall(args: SetupUninstallArgs) -> Result<serde_json::Value> {
    let scope = SetupScope::from_arg(args.scope);
    let repo = resolve_repo(args.repo)?;
    let paths = resolve_paths()?;
    let mut plan = SetupPlan::new(args.dry_run);
    match scope {
        SetupScope::Global => {
            let original_policy = read_policy_or_default(&paths)?;
            let known_repos = known_policy_repos(&paths, &original_policy, &repo);
            let mut policy = original_policy.clone();
            policy.disable_repo(repo.clone());
            let remove_shared_hook = !policy.has_any_global_activation();
            let selected_has_project_hook =
                managed_hook_count_parseable(&project_config_path(&repo)) > 0;
            let bundle_repos = if remove_shared_hook {
                dedup_paths(known_repos.into_iter().filter(|known_repo| {
                    managed_hook_count_parseable(&project_config_path(known_repo)) == 0
                }))
            } else if selected_has_project_hook {
                Vec::new()
            } else {
                vec![repo.clone()]
            };
            for bundle_repo in &bundle_repos {
                preflight_remove_bundle(bundle_repo)?;
            }
            if remove_shared_hook {
                preflight_remove_managed_hook(&paths.codex_config)?;
            }
            if paths.codex1_config.exists() {
                write_policy(&paths, &policy, &mut plan, args.dry_run)?;
            }
            if remove_shared_hook {
                remove_managed_hook(&paths.codex_config, &mut plan, args.dry_run, "global hook")?;
            }
            for bundle_repo in bundle_repos {
                remove_bundle(&bundle_repo, &mut plan, args.dry_run)?;
            }
        }
        SetupScope::Project => {
            let keep_bundle = global_setup_remains_for_repo(&paths, &repo)?;
            if !keep_bundle {
                preflight_remove_bundle(&repo)?;
            }
            remove_managed_hook(
                &project_config_path_checked(&repo)?,
                &mut plan,
                args.dry_run,
                "project hook",
            )?;
            if !keep_bundle {
                remove_bundle(&repo, &mut plan, args.dry_run)?;
            }
        }
    }
    Ok(json!({
        "summary": format!("setup uninstall planned/applied for {}", repo.display()),
        "command": "setup uninstall",
        "scope": scope,
        "repo": repo,
        "plan": plan,
    }))
}

fn migrate(args: SetupMigrateArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let paths = resolve_paths()?;
    let mut plan = SetupPlan::new(args.dry_run);
    let target = SetupScope::from_arg(Some(args.to));
    match target {
        SetupScope::Project => {
            preflight_materialize_bundle(&repo)?;
            let project_config = project_config_path_checked(&repo)?;
            let mut policy = read_policy_or_default(&paths)?;
            let original_policy = read_optional_text(&paths.codex1_config)?;
            policy.disable_repo(repo.clone());
            preflight_write_policy(&paths)?;
            let had_bundle = repo_bundle_materialized(&repo);
            let had_project_hook = managed_hook_count_parseable(&project_config) > 0;
            if let Err(error) = install_managed_hook(
                &project_config,
                SetupScope::Project,
                &mut plan,
                args.dry_run,
                "project hook",
            )
            .and_then(|()| materialize_bundle(&repo, &mut plan, args.dry_run))
            .and_then(|()| write_policy(&paths, &policy, &mut plan, args.dry_run))
            {
                if !args.dry_run {
                    let _ = restore_text(&paths.codex1_config, original_policy.as_deref());
                    if !had_project_hook {
                        let _ = remove_managed_hook(
                            &project_config,
                            &mut SetupPlan::new(false),
                            false,
                            "project hook rollback",
                        );
                    }
                    if !had_bundle {
                        let _ = remove_bundle(&repo, &mut SetupPlan::new(false), false);
                    }
                }
                return Err(error);
            }
        }
        SetupScope::Global => {
            preflight_materialize_bundle(&repo)?;
            let project_config = project_config_path_checked(&repo)?;
            parse_toml_file(&project_config)?;
            preflight_remove_managed_hook(&project_config)?;
            preflight_install_managed_hook(&paths.codex_config, SetupScope::Global)?;
            preflight_write_policy(&paths)?;
            let mut policy = read_policy_or_default(&paths)?;
            if matches!(policy.mode, ActivationMode::Off) {
                policy.mode = ActivationMode::Allowlist;
            }
            policy.set_repo(repo.clone(), true);
            let had_bundle = repo_bundle_materialized(&repo);
            let had_global_hook = managed_hook_count_parseable(&paths.codex_config) > 0;
            let original_policy = read_optional_text(&paths.codex1_config)?;
            if let Err(error) = install_managed_hook(
                &paths.codex_config,
                SetupScope::Global,
                &mut plan,
                args.dry_run,
                "global hook",
            )
            .and_then(|()| materialize_bundle(&repo, &mut plan, args.dry_run))
            .and_then(|()| write_policy(&paths, &policy, &mut plan, args.dry_run))
            .and_then(|()| {
                remove_managed_hook(&project_config, &mut plan, args.dry_run, "project hook")
            }) {
                if !args.dry_run {
                    let _ = restore_text(&paths.codex1_config, original_policy.as_deref());
                    if !had_global_hook {
                        let _ = remove_managed_hook(
                            &paths.codex_config,
                            &mut SetupPlan::new(false),
                            false,
                            "global hook rollback",
                        );
                    }
                    if !had_bundle {
                        let _ = remove_bundle(&repo, &mut SetupPlan::new(false), false);
                    }
                }
                return Err(error);
            }
        }
    }
    Ok(json!({
        "summary": format!("setup migrated for {}", repo.display()),
        "command": "setup migrate",
        "to": target,
        "repo": repo,
        "plan": plan,
    }))
}

fn status_value(repo_arg: Option<PathBuf>) -> Result<serde_json::Value> {
    let status = status(repo_arg)?;
    Ok(json!({
        "summary": if status.effective_active { "codex1 setup active" } else { "codex1 setup inactive" },
        "status": status,
    }))
}

fn doctor(repo_arg: Option<PathBuf>) -> Result<serde_json::Value> {
    let status = status(repo_arg)?;
    let paths = resolve_paths()?;
    let project_config = project_config_path(&status.repo);
    let checks = vec![
        json!({"name": "codex_config_parseable", "ok": parse_toml_file(&status.codex_config).is_ok()}),
        json!({"name": "codex1_policy_parseable", "ok": status.global_config_parseable}),
        json!({"name": "backup_manifest_parseable", "ok": read_manifest(&paths).is_ok()}),
        json!({"name": "global_hook_installed", "ok": status.global_hook_installed || status.project_hook_installed}),
        json!({"name": "managed_hook_executable", "ok": managed_hook_executable_ok(&status.codex_config, SetupScope::Global) && managed_hook_executable_ok(&project_config, SetupScope::Project)}),
        json!({"name": "repo_bundle_materialized", "ok": status.repo_bundle_materialized}),
        json!({"name": "duplicate_hook_risk", "ok": !status.duplicate_hook_risk}),
    ];
    Ok(json!({
        "summary": "setup doctor complete",
        "checks": checks,
        "status": status,
        "anti_oracle": "setup doctor diagnoses activation/config only",
    }))
}

fn status(repo_arg: Option<PathBuf>) -> Result<SetupStatus> {
    let repo = resolve_repo(repo_arg)?;
    let paths = resolve_paths()?;
    let global_config_found = paths.codex1_config.exists();
    let (policy, global_config_parseable, mut warnings) = match read_policy(&paths) {
        Ok(policy) => (policy, true, Vec::new()),
        Err(error) if global_config_found => (
            ActivationPolicy::default(),
            false,
            vec![format!("failed to parse Codex1 config: {error}")],
        ),
        Err(_) => (ActivationPolicy::default(), true, Vec::new()),
    };
    let global_hook_count = managed_hook_count_parseable(&paths.codex_config);
    let project_config = project_config_path(&repo);
    let project_hook_count = managed_hook_count_parseable(&project_config);
    let global_hook_executable_ok =
        managed_hook_executable_ok(&paths.codex_config, SetupScope::Global);
    let project_hook_executable_ok =
        managed_hook_executable_ok(&project_config, SetupScope::Project);
    let repo_bundle_materialized = repo_bundle_materialized(&repo);
    if project_hook_count > 0 {
        warnings.push("project-local hooks depend on official Codex project trust".to_string());
    }
    let repo_policy_enabled = policy.effective_for(&repo);
    let global_hook_active = global_hook_count > 0
        && global_hook_executable_ok
        && global_policy_should_scan_repo(&policy, &repo);
    let project_hook_active =
        project_hook_count > 0 && project_hook_executable_ok && repo_bundle_materialized;
    let effective_active = global_hook_active || project_hook_active;
    let backups_available = read_manifest(&paths)
        .map(|manifest| manifest.records.len())
        .unwrap_or(0);
    Ok(SetupStatus {
        repo,
        codex_home: paths.codex_home,
        codex_config: paths.codex_config,
        codex1_home: paths.codex1_home,
        global_config_found,
        global_config_parseable,
        activation_mode: policy.mode,
        repo_policy_enabled,
        effective_active,
        global_hook_installed: global_hook_count > 0,
        project_hook_installed: project_hook_count > 0,
        repo_bundle_materialized,
        duplicate_hook_risk: global_hook_active && project_hook_active,
        backups_available,
        project_trust_caveat: project_hook_count > 0,
        warnings,
        anti_oracle: "setup status reports activation/config only",
    })
}

fn backups_list() -> Result<serde_json::Value> {
    let paths = resolve_paths()?;
    let manifest = read_manifest(&paths)?;
    Ok(json!({
        "summary": format!("{} setup backups", manifest.records.len()),
        "backups": manifest.records,
    }))
}

fn backups_restore(
    repo_arg: Option<PathBuf>,
    args: SetupBackupRestoreArgs,
) -> Result<serde_json::Value> {
    if !args.force && !args.dry_run {
        return Err(Codex1Error::SetupRestore(
            "setup backups restore requires --force unless --dry-run is used".into(),
        ));
    }
    let paths = resolve_paths()?;
    let manifest = read_manifest(&paths)?;
    let record = manifest
        .records
        .iter()
        .find(|record| record.id == args.id)
        .ok_or_else(|| Codex1Error::SetupRestore(format!("unknown backup id {}", args.id)))?;
    validate_backup_record_for_restore(&paths, repo_arg.as_deref(), record)?;
    if !args.dry_run {
        if record.existed {
            let backup_path = record.backup_path.as_ref().ok_or_else(|| {
                Codex1Error::SetupRestore(format!("backup {} has no file path", record.id))
            })?;
            if let Some(parent) = record.target_path.parent() {
                fs::create_dir_all(parent).map_err(|source| {
                    Codex1Error::SetupRestore(format!(
                        "failed to create restore parent {}: {source}",
                        parent.display()
                    ))
                })?;
            }
            fs::copy(backup_path, &record.target_path).map_err(|source| {
                Codex1Error::SetupRestore(format!(
                    "failed to restore {} from {}: {source}",
                    record.target_path.display(),
                    backup_path.display()
                ))
            })?;
        } else {
            match fs::remove_file(&record.target_path) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(Codex1Error::SetupRestore(format!(
                        "failed to restore missing state for {}: {error}",
                        record.target_path.display()
                    )))
                }
            }
        }
    }
    Ok(json!({
        "summary": format!("restored setup backup {}", record.id),
        "restored": record,
        "dry_run": args.dry_run,
    }))
}

fn validate_backup_record_for_restore(
    paths: &SetupPaths,
    repo_arg: Option<&Path>,
    record: &BackupRecord,
) -> Result<()> {
    let valid_target = record.target_path == paths.codex_config
        || record.target_path == paths.codex1_config
        || validate_project_config_restore_target(repo_arg, &record.target_path).is_ok();
    if !valid_target {
        return Err(Codex1Error::SetupRestore(format!(
            "backup {} target is outside managed setup paths: {}",
            record.id,
            record.target_path.display()
        )));
    }
    if record.target_path == paths.codex_config || record.target_path == paths.codex1_config {
        reject_symlinked_config_target(&record.target_path).map_err(|error| {
            Codex1Error::SetupRestore(format!("backup {} target is unsafe: {error}", record.id))
        })?;
    }
    if let Some(backup_path) = &record.backup_path {
        let backups_dir = fs::canonicalize(&paths.backups_dir).map_err(|source| {
            Codex1Error::SetupRestore(format!(
                "failed to canonicalize backups dir {}: {source}",
                paths.backups_dir.display()
            ))
        })?;
        let metadata = fs::symlink_metadata(backup_path).map_err(|source| {
            Codex1Error::SetupRestore(format!(
                "failed to inspect backup file {}: {source}",
                backup_path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(Codex1Error::SetupRestore(format!(
                "backup {} file must not be a symlink: {}",
                record.id,
                backup_path.display()
            )));
        }
        let backup_real = fs::canonicalize(backup_path).map_err(|source| {
            Codex1Error::SetupRestore(format!(
                "failed to canonicalize backup file {}: {source}",
                backup_path.display()
            ))
        })?;
        if !backup_real.starts_with(backups_dir) {
            return Err(Codex1Error::SetupRestore(format!(
                "backup {} file is outside managed backups: {}",
                record.id,
                backup_path.display()
            )));
        }
    } else if record.existed {
        return Err(Codex1Error::SetupRestore(format!(
            "backup {} has no file path",
            record.id
        )));
    }
    Ok(())
}

fn validate_project_config_restore_target(repo_arg: Option<&Path>, target: &Path) -> Result<()> {
    if target.file_name().and_then(OsStr::to_str) != Some(CONFIG_FILE) {
        return Err(Codex1Error::SetupRestore(format!(
            "project config backup target must be config.toml: {}",
            target.display()
        )));
    }
    let codex_dir = target.parent().ok_or_else(|| {
        Codex1Error::SetupRestore(format!(
            "project config backup target has no parent: {}",
            target.display()
        ))
    })?;
    if codex_dir.file_name().and_then(OsStr::to_str) != Some(".codex") {
        return Err(Codex1Error::SetupRestore(format!(
            "project config backup target must be under .codex: {}",
            target.display()
        )));
    }
    let repo = project_config_repo(target).ok_or_else(|| {
        Codex1Error::SetupRestore(format!(
            "project config backup target has no repo parent: {}",
            target.display()
        ))
    })?;
    let selected_repo = resolve_repo(repo_arg.map(Path::to_path_buf))?;
    if repo != selected_repo {
        return Err(Codex1Error::SetupRestore(format!(
            "project config backup target is outside selected repo: {}",
            target.display()
        )));
    }
    if !selected_repo.join(".git").exists() {
        return Err(Codex1Error::SetupRestore(format!(
            "project config backup target is not under a repo: {}",
            target.display()
        )));
    }
    ensure_contained_for_write(&selected_repo, target).map_err(|source| {
        Codex1Error::SetupRestore(format!(
            "project config backup target escapes repo: {}: {source}",
            target.display()
        ))
    })
}

fn project_config_repo(target: &Path) -> Option<PathBuf> {
    if target.file_name().and_then(OsStr::to_str) != Some(CONFIG_FILE) {
        return None;
    }
    let codex_dir = target.parent()?;
    if codex_dir.file_name().and_then(OsStr::to_str) != Some(".codex") {
        return None;
    }
    let repo = codex_dir.parent()?;
    fs::canonicalize(repo).ok()
}

fn resolve_repo(repo: Option<PathBuf>) -> Result<PathBuf> {
    discover_repo_root(repo)
        .map_err(|error| Codex1Error::SetupArgument(format!("failed to resolve repo: {error}")))
}

fn resolve_paths() -> Result<SetupPaths> {
    let codex_home_env = env::var_os("CODEX_HOME");
    let codex1_home_env = env::var_os("CODEX1_HOME");
    let home = if codex_home_env.is_none() || codex1_home_env.is_none() {
        Some(home_dir()?)
    } else {
        None
    };
    let codex_home = codex_home_env
        .map(PathBuf::from)
        .unwrap_or_else(|| home.as_ref().unwrap().join(".codex"));
    let codex1_home = codex1_home_env
        .map(PathBuf::from)
        .unwrap_or_else(|| home.as_ref().unwrap().join(".codex1"));
    Ok(SetupPaths {
        codex_config: codex_home.join(CONFIG_FILE),
        codex_home,
        codex1_config: codex1_home.join(CONFIG_FILE),
        backups_dir: codex1_home.join("backups"),
        backup_manifest: codex1_home.join("backups").join("manifest.json"),
        codex1_home,
    })
}

fn home_dir() -> Result<PathBuf> {
    if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home));
    }
    if let Some(home) = env::var_os("USERPROFILE").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home));
    }
    match (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH")) {
        (Some(drive), Some(path)) if !drive.is_empty() && !path.is_empty() => {
            let mut home = PathBuf::from(drive);
            home.push(path);
            Ok(home)
        }
        _ => Err(Codex1Error::SetupArgument(
            "HOME, USERPROFILE, or CODEX_HOME/CODEX1_HOME are required for setup".into(),
        )),
    }
}

fn read_policy_or_default(paths: &SetupPaths) -> Result<ActivationPolicy> {
    match read_policy(paths) {
        Ok(policy) => Ok(policy),
        Err(_) if !paths.codex1_config.exists() => Ok(ActivationPolicy::default()),
        Err(error) => Err(error),
    }
}

fn read_policy(paths: &SetupPaths) -> Result<ActivationPolicy> {
    let text = match fs::read_to_string(&paths.codex1_config) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(ActivationPolicy::default()),
        Err(error) => {
            return Err(Codex1Error::SetupConfigParse(format!(
                "failed to read {}: {error}",
                paths.codex1_config.display()
            )))
        }
    };
    parse_policy(&text)
}

fn parse_policy(text: &str) -> Result<ActivationPolicy> {
    let doc = text
        .parse::<DocumentMut>()
        .map_err(|error| Codex1Error::SetupConfigParse(format!("invalid Codex1 TOML: {error}")))?;
    let mode = match doc.get("mode").and_then(|value| value.as_str()) {
        Some("off") => ActivationMode::Off,
        Some("allowlist") | None => ActivationMode::Allowlist,
        Some("denylist") => ActivationMode::Denylist,
        Some("all") => ActivationMode::All,
        Some(other) => {
            return Err(Codex1Error::SetupConfigParse(format!(
                "invalid activation mode {other}"
            )))
        }
    };
    let mut repos = Vec::new();
    if let Some(array) = doc
        .get("repos")
        .and_then(|value| value.as_array_of_tables())
    {
        for table in array {
            let path = table
                .get("path")
                .and_then(|value| value.as_str())
                .ok_or_else(|| Codex1Error::SetupConfigParse("repo entry missing path".into()))?;
            let enabled = table
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(true);
            repos.push(RepoPolicyEntry {
                path: PathBuf::from(path),
                enabled,
            });
        }
    }
    Ok(ActivationPolicy { mode, repos })
}

fn write_policy(
    paths: &SetupPaths,
    policy: &ActivationPolicy,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    plan.writes.push(paths.codex1_config.clone());
    if dry_run {
        return Ok(());
    }
    preflight_write_policy(paths)?;
    backup_target(
        paths,
        &paths.codex1_config,
        "codex1-config",
        "write policy",
        plan,
    )?;
    if let Some(parent) = paths.codex1_config.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            Codex1Error::SetupConfigWrite(format!(
                "failed to create {}: {source}",
                parent.display()
            ))
        })?;
    }
    fs::write(&paths.codex1_config, policy.to_toml()).map_err(|source| {
        Codex1Error::SetupConfigWrite(format!(
            "failed to write {}: {source}",
            paths.codex1_config.display()
        ))
    })
}

fn preflight_write_policy(paths: &SetupPaths) -> Result<()> {
    reject_symlinked_config_target(&paths.codex1_config)?;
    read_manifest(paths).map(|_| ())
}

fn read_optional_text(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Codex1Error::SetupConfigParse(format!(
            "failed to read {}: {error}",
            path.display()
        ))),
    }
}

fn restore_text(path: &Path, text: Option<&str>) -> Result<()> {
    match text {
        Some(text) => write_text(path, text),
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Codex1Error::SetupConfigWrite(format!(
                "failed to remove {}: {error}",
                path.display()
            ))),
        },
    }
}

fn preflight_install_managed_hook(config_path: &Path, hook_scope: SetupScope) -> Result<()> {
    reject_symlinked_config_target(config_path)?;
    let mut text = match fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(Codex1Error::SetupConfigParse(format!(
                "failed to read {}: {error}",
                config_path.display()
            )))
        }
    };
    parse_toml_text(&text)?;
    text = remove_managed_hook_block(&text)?;
    if !text.trim().is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&managed_hook_block(hook_scope)?);
    parse_toml_text(&text)
}

fn preflight_remove_managed_hook(config_path: &Path) -> Result<()> {
    if !config_path.exists() {
        return Ok(());
    }
    reject_symlinked_config_target(config_path)?;
    let original = fs::read_to_string(config_path).map_err(|source| {
        Codex1Error::SetupConfigParse(format!(
            "failed to read {}: {source}",
            config_path.display()
        ))
    })?;
    let edited = remove_managed_hook_block(&original)?;
    if edited != original {
        parse_toml_text(&edited)?;
    }
    Ok(())
}

fn install_managed_hook(
    config_path: &Path,
    hook_scope: SetupScope,
    plan: &mut SetupPlan,
    dry_run: bool,
    reason: &str,
) -> Result<()> {
    install_managed_hook_inner(config_path, hook_scope, plan, dry_run, Some(reason))
}

fn install_managed_hook_inner(
    config_path: &Path,
    hook_scope: SetupScope,
    plan: &mut SetupPlan,
    dry_run: bool,
    backup_reason: Option<&str>,
) -> Result<()> {
    plan.writes.push(config_path.to_path_buf());
    if dry_run {
        return Ok(());
    }
    reject_symlinked_config_target(config_path)?;
    if let Some(reason) = backup_reason {
        let paths = resolve_paths()?;
        backup_target(&paths, config_path, "codex-config", reason, plan)?;
    }
    let mut text = match fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(Codex1Error::SetupConfigParse(format!(
                "failed to read {}: {error}",
                config_path.display()
            )))
        }
    };
    parse_toml_text(&text)?;
    text = remove_managed_hook_block(&text)?;
    if !text.trim().is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&managed_hook_block(hook_scope)?);
    parse_toml_text(&text)?;
    write_text(config_path, &text)
}

fn remove_managed_hook(
    config_path: &Path,
    plan: &mut SetupPlan,
    dry_run: bool,
    reason: &str,
) -> Result<()> {
    if !config_path.exists() {
        return Ok(());
    }
    reject_symlinked_config_target(config_path)?;
    let original = fs::read_to_string(config_path).map_err(|source| {
        Codex1Error::SetupConfigParse(format!(
            "failed to read {}: {source}",
            config_path.display()
        ))
    })?;
    let edited = remove_managed_hook_block(&original)?;
    if edited == original {
        return Ok(());
    }
    plan.writes.push(config_path.to_path_buf());
    if dry_run {
        return Ok(());
    }
    let paths = resolve_paths()?;
    backup_target(&paths, config_path, "codex-config", reason, plan)?;
    parse_toml_text(&edited)?;
    write_text(config_path, &edited)
}

fn managed_hook_count(config_path: &Path) -> usize {
    let Ok(text) = fs::read_to_string(config_path) else {
        return 0;
    };
    text.matches(MANAGED_HOOK_START).count()
}

fn managed_hook_count_parseable(config_path: &Path) -> usize {
    if parse_toml_file(config_path).is_err() {
        return 0;
    }
    managed_hook_count(config_path)
}

fn managed_hook_executable_ok(config_path: &Path, hook_scope: SetupScope) -> bool {
    let Ok(commands) = managed_hook_commands(config_path) else {
        return false;
    };
    if managed_hook_count(config_path) == 0 {
        return true;
    }
    if commands.is_empty() {
        return false;
    }
    commands
        .iter()
        .all(|command| managed_hook_command_ok(command, hook_scope))
}

fn managed_hook_command_ok(command: &str, hook_scope: SetupScope) -> bool {
    let Some((path, rest)) = hook_command_invocation(command) else {
        return false;
    };
    if !hook_executable_ok(&path) {
        return false;
    }
    let mut args = rest.split_whitespace();
    matches!(
        (
            args.next(),
            args.next(),
            args.next(),
            args.next(),
            args.next()
        ),
        (Some("ralph"), Some("stop-hook"), Some("--scope"), Some(scope), None)
            if scope == hook_scope.hook_arg()
    )
}

fn hook_executable_ok(path: &Path) -> bool {
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

fn managed_hook_commands(config_path: &Path) -> Result<Vec<String>> {
    let text = match fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(Codex1Error::SetupConfigParse(format!(
                "failed to read {}: {error}",
                config_path.display()
            )))
        }
    };
    let mut commands = Vec::new();
    let mut scanning = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == MANAGED_HOOK_START {
            scanning = true;
            continue;
        }
        if trimmed == MANAGED_HOOK_END {
            scanning = false;
            continue;
        }
        if !scanning {
            continue;
        }
        let Some(rhs) = trimmed
            .strip_prefix("command")
            .and_then(|rest| rest.trim_start().strip_prefix('=').map(str::trim_start))
        else {
            continue;
        };
        let snippet = format!("value = {rhs}\n");
        let doc = snippet.parse::<DocumentMut>().map_err(|error| {
            Codex1Error::SetupConfigParse(format!("invalid managed hook command: {error}"))
        })?;
        let command = doc
            .get("value")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                Codex1Error::SetupConfigParse("managed hook command must be a string".into())
            })?;
        commands.push(command.to_string());
    }
    Ok(commands)
}

#[cfg(test)]
fn hook_command_executable(command: &str) -> Option<PathBuf> {
    hook_command_invocation(command).map(|(path, _)| path)
}

fn hook_command_invocation(command: &str) -> Option<(PathBuf, String)> {
    let command = command.trim_start();
    if let Some(rest) = command.strip_prefix('\'') {
        return posix_single_quoted_word(rest)
            .map(|(word, rest)| (PathBuf::from(word), rest.trim_start().to_string()));
    }
    if let Some(rest) = command.strip_prefix('"') {
        let mut path = String::new();
        let mut chars = rest.char_indices().peekable();
        while let Some(ch) = chars.next() {
            match ch.1 {
                '"' => {
                    let rest = &rest[ch.0 + ch.1.len_utf8()..];
                    return Some((PathBuf::from(path), rest.trim_start().to_string()));
                }
                '\\' if chars.peek().is_some_and(|next| next.1 == '"') => {
                    chars.next();
                    path.push('"');
                }
                ch => path.push(ch),
            }
        }
        return None;
    }
    let mut parts = command.splitn(2, char::is_whitespace);
    let word = parts.next()?;
    let rest = parts.next().unwrap_or("").trim_start();
    Some((PathBuf::from(word), rest.to_string()))
}

fn posix_single_quoted_word(mut rest: &str) -> Option<(String, &str)> {
    let mut word = String::new();
    loop {
        let end = rest.find('\'')?;
        word.push_str(&rest[..end]);
        rest = &rest[end + 1..];
        if let Some(next) = rest.strip_prefix("\\''") {
            word.push('\'');
            rest = next;
            continue;
        }
        return Some((word, rest));
    }
}

fn managed_hook_block(scope: SetupScope) -> Result<String> {
    let exe = env::current_exe().io_context("failed to resolve current executable")?;
    Ok(managed_hook_block_for_command(&hook_command_for_exe(
        &exe, scope,
    )))
}

fn managed_hook_block_for_command(command: &str) -> String {
    format!(
        r#"{MANAGED_HOOK_START}
[[hooks.Stop]]

[[hooks.Stop.hooks]]
type = "command"
command = {}
timeout = 10
statusMessage = "{MANAGED_HOOK_STATUS}"
{MANAGED_HOOK_END}
"#,
        toml_string(command)
    )
}

fn hook_command_for_exe(exe: &Path, scope: SetupScope) -> String {
    #[cfg(windows)]
    let shell = CommandShell::WindowsCmd;
    #[cfg(not(windows))]
    let shell = CommandShell::PosixSh;
    hook_command_for_exe_with_shell(exe, scope, shell)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum CommandShell {
    PosixSh,
    WindowsCmd,
}

fn hook_command_for_exe_with_shell(exe: &Path, scope: SetupScope, shell: CommandShell) -> String {
    format!(
        "{} ralph stop-hook --scope {}",
        shell_escape_exe(exe, shell),
        scope.hook_arg()
    )
}

fn shell_escape_exe(exe: &Path, shell: CommandShell) -> String {
    let text = exe.display().to_string();
    match shell {
        CommandShell::PosixSh => format!("'{}'", text.replace('\'', r#"'\''"#)),
        CommandShell::WindowsCmd => {
            let mut out = String::from("\"");
            for ch in text.chars() {
                match ch {
                    '^' | '%' | '!' => {
                        out.push('^');
                        out.push(ch);
                    }
                    '"' => out.push_str("\\\""),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
    }
}

fn remove_managed_hook_block(text: &str) -> Result<String> {
    let mut out = String::new();
    let mut skipping = false;
    for line in text.lines() {
        if line.trim() == MANAGED_HOOK_START {
            if skipping {
                return Err(Codex1Error::SetupConfigParse(
                    "nested Codex1 managed hook marker".into(),
                ));
            }
            skipping = true;
            continue;
        }
        if skipping && line.trim() == MANAGED_HOOK_END {
            skipping = false;
            continue;
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    if skipping {
        return Err(Codex1Error::SetupConfigParse(
            "unterminated Codex1 managed hook block".into(),
        ));
    }
    Ok(out)
}

fn parse_toml_file(path: &Path) -> Result<()> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Codex1Error::SetupConfigParse(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    parse_toml_text(&text)
}

fn parse_toml_text(text: &str) -> Result<()> {
    text.parse::<DocumentMut>()
        .map(|_| ())
        .map_err(|error| Codex1Error::SetupConfigParse(format!("invalid TOML: {error}")))
}

fn repo_bundle_materialized(repo: &Path) -> bool {
    validate_bundle(repo).is_ok()
}

fn validate_bundle(repo: &Path) -> Result<()> {
    let marker = repo_bundle_target(repo, BUNDLE_MARKER)?;
    let marker_text = fs::read_to_string(&marker).map_err(|source| {
        Codex1Error::SetupBundle(format!(
            "failed to read bundle marker {}: {source}",
            marker.display()
        ))
    })?;
    let marker_data = parse_bundle_marker(&marker, &marker_text)?;
    if marker_data.managed_by != "codex1-managed" || marker_data.version != BUNDLE_VERSION {
        return Err(Codex1Error::SetupBundle(format!(
            "invalid Codex1 bundle marker {}",
            marker.display()
        )));
    }
    if marker_data.files != MANAGED_BUNDLE_FILES.map(String::from) {
        return Err(Codex1Error::SetupBundle(format!(
            "bundle marker has unexpected files {}",
            marker.display()
        )));
    }
    for relative in MANAGED_BUNDLE_FILES {
        let path = repo_bundle_target(repo, relative)?;
        let text = fs::read_to_string(&path).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to read {}: {source}", path.display()))
        })?;
        let valid = if relative == BUNDLE_GUIDANCE {
            guidance_has_current_managed_block(&text)
        } else {
            text == expected_bundle_body(relative)
        };
        if !valid {
            return Err(Codex1Error::SetupBundle(format!(
                "invalid managed bundle file {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn preflight_materialize_bundle(repo: &Path) -> Result<()> {
    for relative in [BUNDLE_SKILL, BUNDLE_GUIDANCE, BUNDLE_MARKER] {
        let path = repo_bundle_target(repo, relative)?;
        if path.exists() {
            let existing = fs::read_to_string(&path).map_err(|source| {
                Codex1Error::SetupBundle(format!("failed to read {}: {source}", path.display()))
            })?;
            let valid = if relative == BUNDLE_GUIDANCE {
                true
            } else {
                existing == expected_bundle_body(relative)
            };
            if !valid {
                return Err(Codex1Error::SetupBundle(format!(
                    "refusing to overwrite non-managed file {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn materialize_bundle(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    let skill = repo_bundle_target(repo, BUNDLE_SKILL)?;
    let guidance = repo_bundle_target(repo, BUNDLE_GUIDANCE)?;
    let marker = repo_bundle_target(repo, BUNDLE_MARKER)?;
    plan.materialized.push(skill.clone());
    plan.materialized.push(guidance.clone());
    plan.writes.push(marker.clone());
    if dry_run {
        return Ok(());
    }
    write_owned_file(repo, &skill, skill_body())?;
    write_guidance_file(repo, &guidance)?;
    write_owned_file(repo, &marker, &bundle_marker_body())?;
    Ok(())
}

fn remove_bundle(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    let marker = repo_bundle_target(repo, BUNDLE_MARKER)?;
    let Some(files) = validate_bundle_removal(repo)? else {
        return remove_known_bundle_files(repo, plan, dry_run);
    };

    for relative in files {
        let path = repo_bundle_target(repo, &relative)?;
        plan.removes.push(path.clone());
        if !dry_run {
            if relative == BUNDLE_GUIDANCE {
                remove_guidance_if_owned(repo, &path)?;
            } else {
                remove_file_if_owned(repo, &path, &expected_bundle_body(&relative))?;
            }
        }
    }
    plan.removes.push(marker.clone());
    if !dry_run {
        remove_file_if_owned(repo, &marker, &bundle_marker_body())?;
    }
    Ok(())
}

fn parse_bundle_marker(marker: &Path, marker_text: &str) -> Result<BundleMarker> {
    serde_json::from_str(marker_text).map_err(|source| {
        Codex1Error::SetupBundle(format!(
            "failed to parse bundle marker {}: {source}",
            marker.display()
        ))
    })
}

fn remove_known_bundle_files(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    for relative in MANAGED_BUNDLE_FILES {
        let path = repo_bundle_target(repo, relative)?;
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(Codex1Error::SetupBundle(format!(
                    "failed to read {}: {error}",
                    path.display()
                )))
            }
        };
        let owned = if relative == BUNDLE_GUIDANCE {
            guidance_has_managed_block(&text) || text == guidance_body()
        } else {
            text == expected_bundle_body(relative)
        };
        if !owned {
            continue;
        }
        plan.removes.push(path.clone());
        if !dry_run {
            if relative == BUNDLE_GUIDANCE {
                remove_guidance_if_owned(repo, &path)?;
            } else {
                remove_file_if_owned(repo, &path, &expected_bundle_body(relative))?;
            }
        }
    }
    Ok(())
}

fn repo_bundle_target(repo: &Path, relative: impl AsRef<Path>) -> Result<PathBuf> {
    let relative = relative.as_ref();
    if relative.is_absolute() {
        return Err(Codex1Error::SetupBundle(format!(
            "bundle path must be relative: {}",
            relative.display()
        )));
    }
    if relative.components().any(
        |component| !matches!(component, std::path::Component::Normal(part) if !part.is_empty()),
    ) {
        return Err(Codex1Error::SetupBundle(format!(
            "unsafe bundle path: {}",
            relative.display()
        )));
    }
    let path = repo.join(relative);
    ensure_contained_for_write(repo, &path).map_err(|error| {
        Codex1Error::SetupBundle(format!(
            "bundle path escapes repo or crosses a symlink: {}: {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

fn write_owned_file(repo: &Path, path: &Path, body: &str) -> Result<()> {
    ensure_contained_for_write(repo, path).map_err(|error| {
        Codex1Error::SetupBundle(format!(
            "bundle path escapes repo or crosses a symlink: {}: {error}",
            path.display()
        ))
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to create {}: {source}", parent.display()))
        })?;
    }
    if path.exists() {
        let existing = fs::read_to_string(path).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to read {}: {source}", path.display()))
        })?;
        if existing != body {
            return Err(Codex1Error::SetupBundle(format!(
                "refusing to overwrite non-managed file {}",
                path.display()
            )));
        }
    }
    fs::write(path, body).map_err(|source| {
        Codex1Error::SetupBundle(format!("failed to write {}: {source}", path.display()))
    })
}

fn write_guidance_file(repo: &Path, path: &Path) -> Result<()> {
    ensure_contained_for_write(repo, path).map_err(|error| {
        Codex1Error::SetupBundle(format!(
            "bundle path escapes repo or crosses a symlink: {}: {error}",
            path.display()
        ))
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to create {}: {source}", parent.display()))
        })?;
    }
    let block = managed_guidance_block();
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            fs::write(path, &block).map_err(|source| {
                Codex1Error::SetupBundle(format!("failed to write {}: {source}", path.display()))
            })?;
            return Ok(());
        }
        Err(error) => {
            return Err(Codex1Error::SetupBundle(format!(
                "failed to read {}: {error}",
                path.display()
            )))
        }
    };
    if guidance_has_current_managed_block(&text) {
        return Ok(());
    }
    let mut edited = if guidance_has_managed_block(&text) {
        replace_guidance_block(&text, &block).ok_or_else(|| {
            Codex1Error::SetupBundle(format!(
                "failed to replace managed guidance block in {}",
                path.display()
            ))
        })?
    } else {
        text
    };
    if !edited.ends_with('\n') {
        edited.push('\n');
    }
    if !guidance_has_managed_block(&edited) {
        if !edited.ends_with("\n\n") {
            edited.push('\n');
        }
        edited.push_str(&block);
    }
    fs::write(path, edited).map_err(|source| {
        Codex1Error::SetupBundle(format!("failed to write {}: {source}", path.display()))
    })
}

fn remove_file_if_owned(repo: &Path, path: &Path, expected: &str) -> Result<()> {
    ensure_contained_for_write(repo, path).map_err(|error| {
        Codex1Error::SetupBundle(format!(
            "bundle path escapes repo or crosses a symlink: {}: {error}",
            path.display()
        ))
    })?;
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
    if text != expected {
        return Err(Codex1Error::SetupBundle(format!(
            "refusing to remove non-managed file {}",
            path.display()
        )));
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Codex1Error::SetupBundle(format!(
            "failed to remove {}: {error}",
            path.display()
        ))),
    }
}

fn remove_guidance_if_owned(repo: &Path, path: &Path) -> Result<()> {
    ensure_contained_for_write(repo, path).map_err(|error| {
        Codex1Error::SetupBundle(format!(
            "bundle path escapes repo or crosses a symlink: {}: {error}",
            path.display()
        ))
    })?;
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
    if text == guidance_body() || text == managed_guidance_block() {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Codex1Error::SetupBundle(format!(
                "failed to remove {}: {error}",
                path.display()
            ))),
        };
    }
    let Some(edited) = remove_guidance_block(&text) else {
        return Ok(());
    };
    if edited.trim().is_empty() {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Codex1Error::SetupBundle(format!(
                "failed to remove {}: {error}",
                path.display()
            ))),
        }
    } else {
        fs::write(path, edited).map_err(|source| {
            Codex1Error::SetupBundle(format!("failed to write {}: {source}", path.display()))
        })
    }
}

fn expected_bundle_body(relative: &str) -> String {
    match relative {
        BUNDLE_SKILL => skill_body().to_string(),
        BUNDLE_GUIDANCE => guidance_body().to_string(),
        BUNDLE_MARKER => bundle_marker_body(),
        _ => String::new(),
    }
}

fn bundle_marker_body() -> String {
    let marker_body = serde_json::to_string_pretty(&BundleMarker {
        managed_by: "codex1-managed".to_string(),
        version: BUNDLE_VERSION,
        files: vec![BUNDLE_SKILL.into(), BUNDLE_GUIDANCE.into()],
    })
    .unwrap();
    marker_body + "\n"
}

fn skill_body() -> &'static str {
    r#"---
name: codex1
description: codex1-managed setup bundle skill for Codex1 artifact workflows in enabled repositories.
---

# Codex1

This is a codex1-managed repo-scoped skill. Use Codex1 as a deterministic artifact helper through the `codex1` CLI, Ralph loop state, and the mission artifact conventions documented in this repository.

Setup activation is mechanical. Do not treat setup status as mission truth, task readiness, review pass/fail, proof sufficiency, close readiness, or PRD satisfaction.
"#
}

fn guidance_body() -> &'static str {
    r#"# Codex1 Setup Guidance

codex1-managed

Codex1 is active for this repository when setup status says the bundle is materialized and hook policy allows it. Mission truth remains in human-facing artifacts; setup only controls activation.
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

fn guidance_has_current_managed_block(text: &str) -> bool {
    text == guidance_body() || text.contains(&managed_guidance_block())
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

fn project_config_path(repo: &Path) -> PathBuf {
    repo.join(".codex").join(CONFIG_FILE)
}

fn project_config_path_checked(repo: &Path) -> Result<PathBuf> {
    let path = project_config_path(repo);
    ensure_contained_for_write(repo, &path).map_err(|error| {
        Codex1Error::SetupConfigWrite(format!(
            "project config path escapes repo or crosses a symlink: {}: {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

fn backup_target(
    paths: &SetupPaths,
    target: &Path,
    target_kind: &str,
    reason: &str,
    plan: &mut SetupPlan,
) -> Result<()> {
    fs::create_dir_all(&paths.backups_dir).map_err(|source| {
        Codex1Error::SetupBackup(format!(
            "failed to create backups dir {}: {source}",
            paths.backups_dir.display()
        ))
    })?;
    let mut manifest = read_manifest(paths)?;
    let id = format!(
        "{}-{}",
        Utc::now().format("%Y%m%dT%H%M%S%3fZ"),
        manifest.records.len() + 1
    );
    let existed = target.exists();
    let backup_path = if existed {
        let dir = paths.backups_dir.join(&id);
        fs::create_dir_all(&dir).map_err(|source| {
            Codex1Error::SetupBackup(format!("failed to create {}: {source}", dir.display()))
        })?;
        let backup = dir.join(safe_backup_name(target));
        fs::copy(target, &backup).map_err(|source| {
            Codex1Error::SetupBackup(format!(
                "failed to back up {} to {}: {source}",
                target.display(),
                backup.display()
            ))
        })?;
        Some(backup)
    } else {
        None
    };
    let record = BackupRecord {
        id,
        timestamp: Utc::now().to_rfc3339(),
        target_kind: target_kind.to_string(),
        target_path: target.to_path_buf(),
        target_path_label: target.display().to_string(),
        backup_path: backup_path.clone(),
        existed,
        reason: reason.to_string(),
    };
    manifest.records.push(record);
    write_manifest(paths, &manifest)?;
    if let Some(path) = backup_path {
        plan.backups.push(path);
    } else {
        plan.backups.push(target.to_path_buf());
    }
    Ok(())
}

fn read_manifest(paths: &SetupPaths) -> Result<BackupManifest> {
    let text = match fs::read_to_string(&paths.backup_manifest) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(BackupManifest::default()),
        Err(error) => {
            return Err(Codex1Error::SetupBackup(format!(
                "failed to read backup manifest {}: {error}",
                paths.backup_manifest.display()
            )))
        }
    };
    serde_json::from_str(&text).map_err(|source| {
        Codex1Error::SetupBackup(format!(
            "failed to parse backup manifest {}: {source}",
            paths.backup_manifest.display()
        ))
    })
}

fn write_manifest(paths: &SetupPaths, manifest: &BackupManifest) -> Result<()> {
    if let Some(parent) = paths.backup_manifest.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            Codex1Error::SetupBackup(format!("failed to create {}: {source}", parent.display()))
        })?;
    }
    let text = serde_json::to_string_pretty(manifest).unwrap();
    fs::write(&paths.backup_manifest, text + "\n").map_err(|source| {
        Codex1Error::SetupBackup(format!(
            "failed to write backup manifest {}: {source}",
            paths.backup_manifest.display()
        ))
    })
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    reject_symlinked_config_target(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            Codex1Error::SetupConfigWrite(format!(
                "failed to create {}: {source}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, text).map_err(|source| {
        Codex1Error::SetupConfigWrite(format!("failed to write {}: {source}", path.display()))
    })
}

fn reject_symlinked_config_target(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(Codex1Error::SetupConfigWrite(
            format!("refusing to write symlinked config {}", path.display()),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Codex1Error::SetupConfigWrite(format!(
            "failed to inspect {}: {error}",
            path.display()
        ))),
    }
}

fn safe_backup_name(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("config")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn toml_escape_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn toml_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn mode_name(mode: ActivationMode) -> &'static str {
    match mode {
        ActivationMode::Off => "off",
        ActivationMode::Allowlist => "allowlist",
        ActivationMode::Denylist => "denylist",
        ActivationMode::All => "all",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_policy_modes_are_mechanical() {
        let repo = PathBuf::from("/repo");
        let mut policy = ActivationPolicy::default();
        assert!(!policy.effective_for(&repo));

        policy.set_repo(repo.clone(), true);
        assert!(policy.effective_for(&repo));

        policy.mode = ActivationMode::Off;
        assert!(!policy.effective_for(&repo));

        policy.mode = ActivationMode::All;
        assert!(policy.effective_for(&PathBuf::from("/other")));

        policy.mode = ActivationMode::Denylist;
        policy.set_repo(repo.clone(), false);
        assert!(!policy.effective_for(&repo));
        assert!(policy.effective_for(&PathBuf::from("/other")));
    }

    #[test]
    fn policy_toml_round_trips_stably() {
        let mut policy = ActivationPolicy::default();
        policy.set_repo(PathBuf::from("/tmp/repo"), true);
        let text = policy.to_toml();
        assert_eq!(
            text,
            "mode = \"allowlist\"\n\n[[repos]]\npath = \"/tmp/repo\"\nenabled = true\n"
        );
        assert_eq!(parse_policy(&text).unwrap(), policy);
    }

    #[test]
    fn managed_hook_block_removal_preserves_neighbors() {
        let text = format!(
            "model = \"x\"\n{MANAGED_HOOK_START}\n[[hooks.Stop]]\n{MANAGED_HOOK_END}\n[other]\na = 1\n"
        );
        assert_eq!(
            remove_managed_hook_block(&text).unwrap(),
            "model = \"x\"\n[other]\na = 1\n"
        );
    }

    #[test]
    fn managed_hook_block_removal_rejects_unterminated_marker() {
        let text = format!("model = \"x\"\n{MANAGED_HOOK_START}\n[other]\na = 1\n");
        assert!(remove_managed_hook_block(&text).is_err());
    }

    #[test]
    fn hook_command_is_toml_escaped_for_windows_paths() {
        let command = hook_command_for_exe_with_shell(
            Path::new(r"C:\Users\me\codex1.exe"),
            SetupScope::Global,
            CommandShell::WindowsCmd,
        );
        assert_eq!(
            toml_string(&command),
            r#""\"C:\\Users\\me\\codex1.exe\" ralph stop-hook --scope global""#
        );
        let block = managed_hook_block_for_command(&command);
        parse_toml_text(&block).unwrap();
    }

    #[test]
    fn hook_command_shell_quotes_posix_metacharacters() {
        let command = hook_command_for_exe_with_shell(
            Path::new("/tmp/codex $`thing'/codex1"),
            SetupScope::Project,
            CommandShell::PosixSh,
        );
        assert_eq!(
            command,
            r#"'/tmp/codex $`thing'\''/codex1' ralph stop-hook --scope project"#
        );
        let block = managed_hook_block_for_command(&command);
        parse_toml_text(&block).unwrap();
    }

    #[test]
    fn hook_command_executable_parses_posix_quoted_path() {
        let command = hook_command_for_exe_with_shell(
            Path::new("/tmp/codex $`thing'/codex1"),
            SetupScope::Project,
            CommandShell::PosixSh,
        );
        assert_eq!(
            hook_command_executable(&command).unwrap(),
            PathBuf::from("/tmp/codex $`thing'/codex1")
        );
    }

    #[test]
    fn hook_command_executable_preserves_windows_backslashes() {
        let command = hook_command_for_exe_with_shell(
            Path::new(r"C:\Users\me\codex1.exe"),
            SetupScope::Global,
            CommandShell::WindowsCmd,
        );
        assert_eq!(
            hook_command_executable(&command).unwrap(),
            PathBuf::from(r"C:\Users\me\codex1.exe")
        );
    }
}
