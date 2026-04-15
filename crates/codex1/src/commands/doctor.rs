use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use serde_json::Value;

use crate::commands::{DoctorArgs, resolve_repo_root};
use crate::support_surface::{
    AgentsCommandStatus, AgentsScaffoldStatus, SkillSurfaceInspection, SkillSurfaceStatus,
    compute_support_surface_signature, extract_managed_agents_block,
    inspect_agents_scaffold_details, inspect_skill_surface, lookup_toml_value,
    summarize_stop_authority_with_observational, toml_repo_is_trusted,
};

const CONFIG_MODEL: &str = "gpt-5.4";
const CONFIG_REVIEW_MODEL: &str = "gpt-5.4-mini";
const CONFIG_REASONING_EFFORT: &str = "high";
const CONFIG_FAST_PARALLEL_MODEL: &str = "gpt-5.3-codex-spark";
const CONFIG_FAST_PARALLEL_REASONING_EFFORT: &str = "high";
const CONFIG_HARD_CODING_MODEL: &str = "gpt-5.3-codex";
const CONFIG_HARD_CODING_REASONING_EFFORT: &str = "xhigh";
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub repo_root: String,
    pub supported: bool,
    pub findings: Vec<DoctorFinding>,
    pub effective_config: Vec<EffectiveConfigEntry>,
    pub hook_summary: HookSummary,
    pub skill_surface: SkillSurfaceSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualification: Option<QualificationSummary>,
}

#[derive(Debug, Serialize)]
pub struct DoctorFinding {
    pub status: FindingStatus,
    pub check: String,
    pub message: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Serialize)]
pub struct EffectiveConfigEntry {
    pub key: String,
    pub required_value: String,
    pub effective_value: Option<String>,
    pub source_layer: String,
    pub status: FindingStatus,
}

#[derive(Debug, Serialize)]
pub struct HookSummary {
    pub file_path: String,
    pub file_present: bool,
    pub valid_json: bool,
    pub total_stop_handlers: usize,
    pub managed_stop_handlers: usize,
    pub observational_stop_handlers: usize,
}

#[derive(Debug, Serialize)]
pub struct QualificationSummary {
    pub latest_report_path: String,
    pub status: String,
    pub failed_gates: usize,
    pub skipped_gates: usize,
    pub qualification_scope: Option<String>,
    pub supported_build_qualified: bool,
    pub support_surface_signature: Option<String>,
    pub stale_relative_to_current_support_surface: bool,
    pub stale_for_build: bool,
    pub stale_for_support_surface: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_codex_build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_codex_build: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillSurfaceSummary {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_mode: Option<String>,
    pub status: FindingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_error: Option<String>,
    pub missing_required_public_skills: Vec<String>,
    pub drifted_managed_files: Vec<String>,
    pub matched_managed_files: usize,
}

pub fn run(args: DoctorArgs) -> Result<()> {
    let repo_root = resolve_repo_root(args.common.repo_root.as_deref())?;
    let runtime_overrides = parse_runtime_overrides(&args.runtime_overrides)?;
    let user_config_path = codex_home()?.join("config.toml");
    let user_hooks_path = codex_home()?.join("hooks.json");
    let project_config_path = repo_root.join(".codex/config.toml");
    let hooks_path = repo_root.join(".codex/hooks.json");
    let agents_path = repo_root.join("AGENTS.md");

    let user_config = read_optional_string(&user_config_path)?;
    let user_hooks = read_optional_string(&user_hooks_path)?;
    let project_config = read_optional_string(&project_config_path)?;
    let hooks_config = read_optional_string(&hooks_path)?;
    let agents_doc = read_optional_string(&agents_path)?;
    let trusted_repo = is_repo_trusted(&repo_root, user_config.as_deref());

    let mut findings = Vec::new();
    findings.push(if trusted_repo {
        DoctorFinding {
            status: FindingStatus::Pass,
            check: "trusted_repo".to_string(),
            message: "the target repo is trusted by Codex".to_string(),
            remediation: None,
        }
    } else {
        DoctorFinding {
            status: FindingStatus::Fail,
            check: "trusted_repo".to_string(),
            message: "Codex will ignore project-scoped .codex configuration until the repo is trusted".to_string(),
            remediation: Some(format!(
                "mark {} as trusted in Codex or add [projects.\"{}\"] trust_level = \"trusted\" to ~/.codex/config.toml",
                repo_root.display(),
                repo_root.display()
            )),
        }
    });

    let effective_config = vec![
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            None,
            "model",
            CONFIG_MODEL,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            None,
            "review_model",
            CONFIG_REVIEW_MODEL,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            None,
            "model_reasoning_effort",
            CONFIG_REASONING_EFFORT,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("features"),
            "codex_hooks",
            "true",
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("agents"),
            "max_threads",
            "16",
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("agents"),
            "max_depth",
            "1",
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_orchestration"),
            "model",
            CONFIG_MODEL,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_orchestration"),
            "reasoning_effort",
            CONFIG_REASONING_EFFORT,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_review"),
            "model",
            CONFIG_REVIEW_MODEL,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_review"),
            "reasoning_effort",
            CONFIG_REASONING_EFFORT,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_fast_parallel"),
            "model",
            CONFIG_FAST_PARALLEL_MODEL,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_fast_parallel"),
            "reasoning_effort",
            CONFIG_FAST_PARALLEL_REASONING_EFFORT,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_hard_coding"),
            "model",
            CONFIG_HARD_CODING_MODEL,
            true,
        ),
        inspect_config_key(
            &runtime_overrides,
            trusted_repo,
            user_config.as_deref(),
            project_config.as_deref(),
            Some("codex1_hard_coding"),
            "reasoning_effort",
            CONFIG_HARD_CODING_REASONING_EFFORT,
            true,
        ),
    ];

    for entry in &effective_config {
        match entry.status {
            FindingStatus::Pass => {}
            FindingStatus::Warn => findings.push(DoctorFinding {
                status: FindingStatus::Warn,
                check: format!("config:{}", entry.key),
                message: format!(
                    "{} resolves to the required value, but the source layer is {} rather than the trusted project config",
                    entry.key, entry.source_layer
                ),
                remediation: Some(format!(
                    "run codex1 setup to pin {} in .codex/config.toml",
                    entry.key
                )),
            }),
            FindingStatus::Fail => findings.push(DoctorFinding {
                status: FindingStatus::Fail,
                check: format!("config:{}", entry.key),
                message: format!(
                    "{} resolves to {:?} from {} instead of {}",
                    entry.key, entry.effective_value, entry.source_layer, entry.required_value
                ),
                remediation: Some(format!(
                    "set {} = {} in the trusted project config",
                    entry.key, entry.required_value
                )),
            }),
        }
    }

    let hook_summary = inspect_hooks(&hooks_path, hooks_config.as_deref())?;
    let user_hook_summary = inspect_hooks(&user_hooks_path, user_hooks.as_deref())?;
    findings.push(match (
        hook_summary.file_present,
        hook_summary.valid_json,
        hook_summary
            .total_stop_handlers
            .saturating_sub(hook_summary.observational_stop_handlers),
        hook_summary.total_stop_handlers,
        hook_summary.observational_stop_handlers,
    ) {
        (false, _, _, _, _) => DoctorFinding {
            status: FindingStatus::Fail,
            check: "hooks_json".to_string(),
            message: format!("{} is missing", hook_summary.file_path),
            remediation: Some("run codex1 setup to install the managed Stop hook".to_string()),
        },
        (true, false, _, _, _) => DoctorFinding {
            status: FindingStatus::Fail,
            check: "hooks_json".to_string(),
            message: format!("{} is not valid JSON", hook_summary.file_path),
            remediation: Some("repair .codex/hooks.json or rerun codex1 setup --force".to_string()),
        },
        (true, true, 0, _, _) => DoctorFinding {
            status: FindingStatus::Fail,
            check: "hooks_json".to_string(),
            message: "the Codex1 Stop hook is not registered".to_string(),
            remediation: Some("run codex1 setup to install the managed Stop hook".to_string()),
        },
        (true, true, authoritative, total, observational) if authoritative == 1 => DoctorFinding {
            status: if hook_summary.managed_stop_handlers == 1 {
                FindingStatus::Pass
            } else {
                FindingStatus::Warn
            },
            check: "hooks_json".to_string(),
            message: if observational > 0 {
                format!(
                    "exactly one authoritative Stop pipeline is installed, with {observational} observational Stop hook(s) preserved"
                )
            } else if hook_summary.managed_stop_handlers == 1 {
                "exactly one authoritative Stop handler is installed".to_string()
            } else {
                format!(
                    "exactly one authoritative Stop pipeline is installed via a non-Codex1 aggregator (total Stop hooks: {total}); verify that this is the supported Ralph pipeline"
                )
            },
            remediation: if hook_summary.managed_stop_handlers == 1 {
                None
            } else {
                Some(
                    "verify the repo Stop handler is the intended Ralph aggregator or rerun codex1 setup --force to normalize it".to_string(),
                )
            },
        },
        (true, true, _, total, observational) => DoctorFinding {
            status: FindingStatus::Fail,
            check: "hooks_json".to_string(),
            message: format!(
                "found {} authoritative Stop handlers across {} total Stop hooks ({} observational); supported Codex1 repos need exactly one authoritative Stop pipeline",
                total.saturating_sub(observational),
                total,
                observational
            ),
            remediation: Some("remove extra Stop handlers or merge them behind one aggregator".to_string()),
        },
    });
    findings.push(if !user_hook_summary.valid_json && user_hook_summary.file_present {
        DoctorFinding {
            status: FindingStatus::Fail,
            check: "user_stop_hook_conflict".to_string(),
            message: format!("{} is not valid JSON", user_hook_summary.file_path),
            remediation: Some(
                "repair ~/.codex/hooks.json so Codex1 can verify there is no user-level Stop hook conflict"
                    .to_string(),
            ),
        }
    } else if user_hook_summary.total_stop_handlers == 0 {
        DoctorFinding {
            status: FindingStatus::Pass,
            check: "user_stop_hook_conflict".to_string(),
            message: "no user-level Stop hook conflicts with the repo-local Codex1 Stop pipeline".to_string(),
            remediation: None,
        }
    } else if user_hook_summary.total_stop_handlers
        == user_hook_summary.observational_stop_handlers
    {
        DoctorFinding {
            status: FindingStatus::Pass,
            check: "user_stop_hook_conflict".to_string(),
            message: format!(
                "{} user-level Stop handler(s) are marked observational, so they do not conflict with the repo-local Codex1 Stop pipeline",
                user_hook_summary.total_stop_handlers
            ),
            remediation: None,
        }
    } else {
        DoctorFinding {
            status: FindingStatus::Fail,
            check: "user_stop_hook_conflict".to_string(),
            message: format!(
                "found {} user-level authoritative Stop handler(s) in {}; supported Codex1 environments need one authoritative Stop pipeline across config layers",
                user_hook_summary
                    .total_stop_handlers
                    .saturating_sub(user_hook_summary.observational_stop_handlers),
                user_hooks_path.display()
            ),
            remediation: Some(
                "remove, disable, or mark non-authoritative user-level Stop hooks observational so the repo-local Codex1 Stop handler remains the only active authority"
                    .to_string(),
            ),
        }
    });

    let agents_inspection = inspect_agents_scaffold_details(agents_doc.as_deref());
    findings.push(match agents_inspection.status {
        AgentsScaffoldStatus::Present => DoctorFinding {
            status: if agents_inspection.command_status == AgentsCommandStatus::Concrete {
                FindingStatus::Pass
            } else {
                FindingStatus::Warn
            },
            check: "agents_md".to_string(),
            message: if agents_inspection.command_status == AgentsCommandStatus::Concrete {
                "AGENTS.md contains the Codex1 managed scaffold block with concrete repo commands"
                    .to_string()
            } else {
                "AGENTS.md contains the Codex1 managed scaffold block, but the repo command slots still need concrete values".to_string()
            },
            remediation: if agents_inspection.command_status == AgentsCommandStatus::Concrete {
                None
            } else {
                Some(
                    "fill in the Build/Test/Lint or format command lines inside the managed Codex1 block"
                        .to_string(),
                )
            },
        },
        AgentsScaffoldStatus::MissingFile => DoctorFinding {
            status: FindingStatus::Warn,
            check: "agents_md".to_string(),
            message: "AGENTS.md is missing".to_string(),
            remediation: Some("run codex1 setup to install the thin Codex1 scaffold".to_string()),
        },
        AgentsScaffoldStatus::MissingBlock => DoctorFinding {
            status: FindingStatus::Warn,
            check: "agents_md".to_string(),
            message: "AGENTS.md exists but the Codex1 managed block is missing".to_string(),
            remediation: Some(
                "run codex1 setup to reapply the managed AGENTS.md block".to_string(),
            ),
        },
        AgentsScaffoldStatus::DriftedBlock => DoctorFinding {
            status: FindingStatus::Fail,
            check: "agents_md".to_string(),
            message: "AGENTS.md contains Codex1 markers, but the managed block has drifted"
                .to_string(),
            remediation: Some(
                "repair the managed AGENTS.md block manually or rerun codex1 setup --force"
                    .to_string(),
            ),
        },
        AgentsScaffoldStatus::MalformedMarkers => DoctorFinding {
            status: FindingStatus::Fail,
            check: "agents_md".to_string(),
            message: "AGENTS.md has malformed Codex1 markers".to_string(),
            remediation: Some(
                "repair the markers manually or rerun codex1 setup --force".to_string(),
            ),
        },
    });

    let skill_inspection = inspect_skill_surface(&repo_root)?;
    let skill_surface = summarize_skill_surface(&skill_inspection);
    findings.push(render_skill_surface_finding(&skill_inspection));

    let managed_agents_block = agents_doc.as_deref().and_then(extract_managed_agents_block);
    let support_surface_signature = compute_support_surface_signature(
        project_config.as_deref(),
        hooks_config.as_deref(),
        user_hooks.as_deref(),
        managed_agents_block.as_deref(),
        &skill_inspection.discovery_root,
    )?;
    let mut qualification_supports_repo = false;
    let qualification = match inspect_qualification(&repo_root, &support_surface_signature) {
        Ok(qualification) => {
            findings.push(match qualification.as_ref() {
            Some(summary)
                if summary.status == "pass"
                    && !summary.stale_relative_to_current_support_surface =>
            {
                qualification_supports_repo = true;
                DoctorFinding {
                    status: FindingStatus::Pass,
                    check: "qualification".to_string(),
                    message: format!(
                        "latest qualification passed with {} failed gate(s)",
                        summary.failed_gates
                    ),
                    remediation: None,
                }
            }
            Some(summary) if summary.status == "pass" => DoctorFinding {
                status: FindingStatus::Warn,
                check: "qualification".to_string(),
                message: if summary.stale_for_build && summary.stale_for_support_surface {
                    "latest qualification passed, but both the tested Codex build and the repo support surface have drifted".to_string()
                } else if summary.stale_for_build {
                    format!(
                        "latest qualification passed, but the current Codex build ({}) no longer matches the tested build ({})",
                        summary
                            .current_codex_build
                            .as_deref()
                            .unwrap_or("unknown"),
                        summary
                            .recorded_codex_build
                            .as_deref()
                            .unwrap_or("unknown")
                    )
                } else {
                    "latest qualification passed, but its support-surface signature no longer matches the current repo state".to_string()
                },
                remediation: Some(
                    "rerun codex1 qualify-codex to refresh qualification evidence for the current support surface"
                        .to_string(),
                ),
            },
            Some(summary) if summary.status == "fail" => DoctorFinding {
                status: FindingStatus::Fail,
                check: "qualification".to_string(),
                message: format!(
                    "latest qualification failed with {} failed gate(s)",
                    summary.failed_gates
                ),
                remediation: Some(
                    "rerun codex1 qualify-codex after fixing the failing gates".to_string(),
                ),
            },
            Some(summary) => DoctorFinding {
                status: FindingStatus::Warn,
                check: "qualification".to_string(),
                message: format!(
                    "latest qualification is {} with {} skipped gate(s)",
                    summary.status, summary.skipped_gates
                ),
                remediation: Some(
                    "review the latest qualification report and decide whether to rerun qualify-codex"
                        .to_string(),
                ),
            },
            None => DoctorFinding {
                status: FindingStatus::Warn,
                check: "qualification".to_string(),
                message: "no qualification evidence found under .codex1/qualification/latest.json"
                    .to_string(),
                remediation: Some(
                    "run codex1 qualify-codex to generate qualification evidence".to_string(),
                ),
            },
            });
            qualification
        }
        Err(error) => {
            findings.push(DoctorFinding {
                status: FindingStatus::Warn,
                check: "qualification".to_string(),
                message: format!(
                    "latest qualification evidence is unreadable or malformed: {}",
                    error
                ),
                remediation: Some(
                    "repair or remove .codex1/qualification/latest.json, then rerun codex1 qualify-codex"
                        .to_string(),
                ),
            });
            None
        }
    };

    let supported = qualification_supports_repo
        && findings
            .iter()
            .all(|finding| !matches!(finding.status, FindingStatus::Fail));
    let report = DoctorReport {
        repo_root: repo_root.display().to_string(),
        supported,
        findings,
        effective_config,
        hook_summary,
        skill_surface,
        qualification,
    };

    emit_report(args.common.json, &report, render_doctor_report(&report))
}

fn codex_home() -> Result<PathBuf> {
    if let Some(explicit) = env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(explicit));
    }

    let home = env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn read_optional_string(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn is_repo_trusted(repo_root: &Path, user_config: Option<&str>) -> bool {
    user_config.is_some_and(|raw| toml_repo_is_trusted(raw, repo_root))
}

fn inspect_config_key(
    runtime_overrides: &BTreeMap<String, String>,
    trusted_repo: bool,
    user_config: Option<&str>,
    project_config: Option<&str>,
    section: Option<&str>,
    key: &str,
    required_value: &str,
    prefer_project: bool,
) -> EffectiveConfigEntry {
    let full_key = section
        .map(|section| format!("{section}.{key}"))
        .unwrap_or_else(|| key.to_string());
    let user_value = user_config.and_then(|raw| lookup_toml_value(raw, section, key));
    let project_value = if trusted_repo {
        project_config.and_then(|raw| lookup_toml_value(raw, section, key))
    } else {
        None
    };

    let runtime_value = runtime_overrides.get(&full_key).cloned();

    let (effective_value, source_layer) = if let Some(runtime_value) = runtime_value {
        (Some(runtime_value), "runtime_flag")
    } else if let Some(project_value) = project_value {
        (Some(project_value), "project")
    } else if let Some(user_value) = user_value {
        (Some(user_value), "user")
    } else {
        (None, "default")
    };

    let status = match effective_value.as_deref() {
        Some(value) if value == required_value => {
            if source_layer == "runtime_flag" {
                FindingStatus::Pass
            } else if prefer_project && source_layer != "project" {
                FindingStatus::Warn
            } else {
                FindingStatus::Pass
            }
        }
        _ => FindingStatus::Fail,
    };

    EffectiveConfigEntry {
        key: full_key,
        required_value: required_value.to_string(),
        effective_value,
        source_layer: source_layer.to_string(),
        status,
    }
}

fn parse_runtime_overrides(raw: &[String]) -> Result<BTreeMap<String, String>> {
    let mut overrides = BTreeMap::new();
    for entry in raw {
        let (key, value) = entry
            .split_once('=')
            .ok_or_else(|| anyhow!("runtime override `{entry}` must use KEY=VALUE"))?;
        let key = key.trim();
        if key.is_empty() {
            bail!("runtime override `{entry}` must have a non-empty key");
        }
        overrides.insert(key.to_string(), value.trim().to_string());
    }
    Ok(overrides)
}

fn inspect_hooks(path: &Path, raw: Option<&str>) -> Result<HookSummary> {
    let Some(raw) = raw else {
        return Ok(HookSummary {
            file_path: path.display().to_string(),
            file_present: false,
            valid_json: false,
            total_stop_handlers: 0,
            managed_stop_handlers: 0,
            observational_stop_handlers: 0,
        });
    };

    let parsed = serde_json::from_str::<Value>(&raw);
    let Ok(value) = parsed else {
        return Ok(HookSummary {
            file_path: path.display().to_string(),
            file_present: true,
            valid_json: false,
            total_stop_handlers: 0,
            managed_stop_handlers: 0,
            observational_stop_handlers: 0,
        });
    };

    let counts = summarize_stop_authority_with_observational(&value);

    Ok(HookSummary {
        file_path: path.display().to_string(),
        file_present: true,
        valid_json: true,
        total_stop_handlers: counts.total,
        managed_stop_handlers: counts.managed,
        observational_stop_handlers: counts.observational,
    })
}

fn summarize_skill_surface(inspection: &SkillSurfaceInspection) -> SkillSurfaceSummary {
    SkillSurfaceSummary {
        file_path: inspection.discovery_root.display().to_string(),
        install_mode: inspection
            .install_mode
            .map(|mode| mode.as_str().to_string()),
        status: match inspection.status {
            SkillSurfaceStatus::ValidExisting => FindingStatus::Pass,
            SkillSurfaceStatus::Missing
            | SkillSurfaceStatus::PartialOrDrifted
            | SkillSurfaceStatus::InvalidBridge => FindingStatus::Fail,
        },
        bridge_error: inspection.bridge_error.clone(),
        missing_required_public_skills: inspection.missing_required_public_skills.clone(),
        drifted_managed_files: inspection.drifted_managed_files.clone(),
        matched_managed_files: inspection.matched_managed_files,
    }
}

fn render_skill_surface_finding(inspection: &SkillSurfaceInspection) -> DoctorFinding {
    match inspection.status {
        SkillSurfaceStatus::ValidExisting => DoctorFinding {
            status: FindingStatus::Pass,
            check: "skill_surface".to_string(),
            message: format!(
                "required public skills are discoverable via {} at {}",
                inspection
                    .install_mode
                    .map(|mode| mode.as_str())
                    .unwrap_or("the repo-local skill surface"),
                inspection.discovery_root.display()
            ),
            remediation: None,
        },
        SkillSurfaceStatus::Missing => DoctorFinding {
            status: FindingStatus::Fail,
            check: "skill_surface".to_string(),
            message: format!(
                "the discoverable skill surface at {} is missing the required public workflows: {}",
                inspection.discovery_root.display(),
                inspection.missing_required_public_skills.join(", ")
            ),
            remediation: Some(
                "run codex1 setup to install the managed copied skill surface, or configure a valid `[[skills.config]]` bridge"
                    .to_string(),
            ),
        },
        SkillSurfaceStatus::PartialOrDrifted => DoctorFinding {
            status: FindingStatus::Fail,
            check: "skill_surface".to_string(),
            message: format!(
                "the discoverable skill surface at {} is partial or drifted (missing: {}; drifted: {})",
                inspection.discovery_root.display(),
                if inspection.missing_required_public_skills.is_empty() {
                    "none".to_string()
                } else {
                    inspection.missing_required_public_skills.join(", ")
                },
                if inspection.drifted_managed_files.is_empty() {
                    "none".to_string()
                } else {
                    inspection.drifted_managed_files.join(", ")
                }
            ),
            remediation: Some(
                "rerun codex1 setup --force to rewrite the managed copied skill surface, or repair the bridged or linked skill root"
                    .to_string(),
            ),
        },
        SkillSurfaceStatus::InvalidBridge => DoctorFinding {
            status: FindingStatus::Fail,
            check: "skill_surface".to_string(),
            message: format!(
                "the configured `[[skills.config]]` bridge is not support-ready at {} ({})",
                inspection.discovery_root.display(),
                inspection
                    .bridge_error
                    .as_deref()
                    .unwrap_or("invalid bridge target")
            ),
            remediation: Some(
                "repair or remove the stale `[[skills.config]]` bridge, or rerun codex1 setup --force to reinstall the managed copied skill surface".to_string(),
            ),
        },
    }
}

fn inspect_qualification(
    repo_root: &Path,
    current_support_surface_signature: &str,
) -> Result<Option<QualificationSummary>> {
    let latest_path = repo_root.join(".codex1/qualification/latest.json");
    let Some(raw) = read_optional_string(&latest_path)? else {
        return Ok(None);
    };

    let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse qualification report {}", latest_path.display()))?;
    let failed_gates = parsed
        .get("summary")
        .and_then(|value| value.get("failed"))
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let skipped_gates = parsed
        .get("summary")
        .and_then(|value| value.get("skipped"))
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let passed_all = parsed
        .get("summary")
        .and_then(|value| value.get("passed_all_required_gates"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let supported_build_qualified = parsed
        .get("summary")
        .and_then(|value| value.get("supported_build_qualified"))
        .and_then(Value::as_bool)
        .unwrap_or(passed_all);
    let qualification_scope = parsed
        .get("summary")
        .and_then(|value| value.get("qualification_scope"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    let status = if supported_build_qualified {
        "pass"
    } else if failed_gates > 0 {
        "fail"
    } else {
        "warn"
    };
    let support_surface_signature = parsed
        .get("support_surface_signature")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let recorded_codex_build = parsed
        .get("codex_build")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let current_codex_build = current_codex_build()?;
    let stale_for_build = is_stale_for_current_build(
        recorded_codex_build.as_deref(),
        current_codex_build.as_deref(),
    );
    let stale_for_support_surface =
        support_surface_signature.as_deref() != Some(current_support_surface_signature);

    Ok(Some(QualificationSummary {
        latest_report_path: latest_path.display().to_string(),
        status: status.to_string(),
        failed_gates,
        skipped_gates,
        qualification_scope,
        supported_build_qualified,
        stale_relative_to_current_support_surface: stale_for_support_surface || stale_for_build,
        stale_for_build,
        stale_for_support_surface,
        support_surface_signature,
        recorded_codex_build,
        current_codex_build,
    }))
}

fn current_codex_build() -> Result<Option<String>> {
    let output = match Command::new("codex").arg("--version").output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("failed to execute `codex --version`"),
    };

    if !output.status.success() {
        return Ok(None);
    }

    let raw_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw_version.is_empty() {
        return Ok(None);
    }

    Ok(Some(raw_version))
}

fn is_stale_for_current_build(
    recorded_codex_build: Option<&str>,
    current_codex_build: Option<&str>,
) -> bool {
    match (recorded_codex_build, current_codex_build) {
        (Some(recorded), Some(current)) => recorded != current,
        _ => true,
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

fn render_doctor_report(report: &DoctorReport) -> String {
    let mut output = String::new();
    let _ = writeln!(&mut output, "repo root: {}", report.repo_root);
    let _ = writeln!(&mut output, "supported: {}", yes_no(report.supported));
    let _ = writeln!(&mut output, "findings:");
    for finding in &report.findings {
        let _ = writeln!(
            &mut output,
            "- [{}] {}: {}",
            render_status(&finding.status),
            finding.check,
            finding.message
        );
        if let Some(remediation) = finding.remediation.as_deref() {
            let _ = writeln!(&mut output, "  remediation: {remediation}");
        }
    }
    let _ = writeln!(&mut output, "effective config:");
    for entry in &report.effective_config {
        let _ = writeln!(
            &mut output,
            "- [{}] {} -> {:?} via {} (required {})",
            render_status(&entry.status),
            entry.key,
            entry.effective_value,
            entry.source_layer,
            entry.required_value
        );
    }
    let _ = writeln!(
        &mut output,
        "hooks: valid_json={}, total_stop_handlers={}, managed_stop_handlers={}, observational_stop_handlers={}",
        yes_no(report.hook_summary.valid_json),
        report.hook_summary.total_stop_handlers,
        report.hook_summary.managed_stop_handlers,
        report.hook_summary.observational_stop_handlers,
    );
    let _ = writeln!(
        &mut output,
        "skill surface: [{}] {} via {} (matched managed files: {}, missing: {}, drifted: {})",
        render_status(&report.skill_surface.status),
        report.skill_surface.file_path,
        report
            .skill_surface
            .install_mode
            .as_deref()
            .unwrap_or("unknown"),
        report.skill_surface.matched_managed_files,
        if report
            .skill_surface
            .missing_required_public_skills
            .is_empty()
        {
            "none".to_string()
        } else {
            report
                .skill_surface
                .missing_required_public_skills
                .join(", ")
        },
        if report.skill_surface.drifted_managed_files.is_empty() {
            "none".to_string()
        } else {
            report.skill_surface.drifted_managed_files.join(", ")
        },
    );
    if let Some(qualification) = &report.qualification {
        let _ = writeln!(
            &mut output,
            "qualification: status={}, report={}, failed_gates={}, skipped_gates={}, stale_for_build={}, stale_for_support_surface={}",
            qualification.status,
            qualification.latest_report_path,
            qualification.failed_gates,
            qualification.skipped_gates,
            yes_no(qualification.stale_for_build),
            yes_no(qualification.stale_for_support_surface),
        );
    }
    output.trim_end().to_string()
}

fn render_status(status: &FindingStatus) -> &'static str {
    match status {
        FindingStatus::Pass => "pass",
        FindingStatus::Warn => "warn",
        FindingStatus::Fail => "fail",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::support_surface::{SkillSurfaceStatus, inspect_skill_surface_with_source};

    fn source_skill_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.codex/skills")
            .canonicalize()
            .expect("canonical source skill root")
    }

    fn copy_source_skills(target: &TempDir) {
        let source = source_skill_root();
        for entry in walkdir::WalkDir::new(&source) {
            let entry = entry.expect("walk source skills");
            let relative = entry
                .path()
                .strip_prefix(&source)
                .expect("relative source path");
            let destination = target.path().join(".codex/skills").join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&destination).expect("create destination dir");
            } else {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).expect("create parent dir");
                }
                fs::copy(entry.path(), &destination).expect("copy skill file");
            }
        }
    }

    #[test]
    fn repo_local_public_skills_pass() {
        let temp = TempDir::new().expect("temp dir");
        copy_source_skills(&temp);

        let inspection =
            inspect_skill_surface_with_source(temp.path(), &source_skill_root()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::ValidExisting);
    }

    #[test]
    fn missing_skill_surface_fails() {
        let temp = TempDir::new().expect("temp dir");
        let inspection =
            inspect_skill_surface_with_source(temp.path(), &source_skill_root()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::Missing);
    }

    #[test]
    fn drifted_skill_surface_fails() {
        let temp = TempDir::new().expect("temp dir");
        copy_source_skills(&temp);
        fs::write(
            temp.path().join(".codex/skills/clarify/SKILL.md"),
            "# drifted\n",
        )
        .expect("drift skill");

        let inspection =
            inspect_skill_surface_with_source(temp.path(), &source_skill_root()).expect("inspect");
        assert_eq!(inspection.status, SkillSurfaceStatus::PartialOrDrifted);
        assert!(!inspection.drifted_managed_files.is_empty());
    }

    #[test]
    fn qualification_build_mismatch_marks_report_stale() {
        assert!(super::is_stale_for_current_build(
            Some("codex 1.2.3"),
            Some("codex 1.2.4")
        ));
        assert!(!super::is_stale_for_current_build(
            Some("codex 1.2.3"),
            Some("codex 1.2.3")
        ));
        assert!(super::is_stale_for_current_build(Some("codex 1.2.3"), None));
    }
}
