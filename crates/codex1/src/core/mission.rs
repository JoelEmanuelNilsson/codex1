//! Mission discovery precedence.
//!
//! 1. `--mission <id>` + `--repo-root <path>` → `<path>/PLANS/<id>/`.
//! 2. `--mission <id>` alone → `<CWD>/PLANS/<id>/`.
//! 3. Neither → walk up from CWD to find a `PLANS/` dir containing exactly
//!    one mission (error if 0 or >1 missions).
//!
//! All paths are returned absolute; symlinks are followed by the filesystem.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::error::CliError;
use crate::core::paths::{validate_mission_id, MissionPaths};

/// CLI arguments that influence mission discovery. Every command wires
/// these through `clap` and hands the struct to `resolve_mission`.
#[derive(Debug, Clone, Default)]
pub struct MissionSelector {
    pub mission: Option<String>,
    pub repo_root: Option<PathBuf>,
}

/// Resolve the mission paths under the precedence defined above.
///
/// When `require_exists` is true (the default for every command except
/// `init`), the mission directory and its `STATE.json` must already exist.
pub fn resolve_mission(
    selector: &MissionSelector,
    require_exists: bool,
) -> Result<MissionPaths, CliError> {
    let cwd = std::env::current_dir().map_err(CliError::Io)?;
    if let Some(mission) = &selector.mission {
        validate_mission_id(mission)?;
    }
    let paths = match (&selector.mission, &selector.repo_root) {
        (Some(mission), Some(root)) => MissionPaths::new(absolutize(root, &cwd), mission.clone()),
        (Some(mission), None) => MissionPaths::new(cwd.clone(), mission.clone()),
        (None, Some(root)) => discover_single_mission(&absolutize(root, &cwd))?,
        (None, None) => walk_up_for_mission(&cwd)?,
    };

    if require_exists && !paths.state().is_file() {
        return Err(CliError::MissionNotFound {
            message: format!("Expected STATE.json at {}", paths.state().display()),
            hint: Some(
                "Run `codex1 init --mission <id>` first, or pass --mission/--repo-root."
                    .to_string(),
            ),
            ambiguous: false,
        });
    }

    Ok(paths)
}

/// Resolve a mission for `init` (the parent directory must exist, but the
/// mission directory itself may or may not).
pub fn resolve_mission_for_init(selector: &MissionSelector) -> Result<MissionPaths, CliError> {
    let cwd = std::env::current_dir().map_err(CliError::Io)?;
    let (root, mission) = match (&selector.mission, &selector.repo_root) {
        (Some(m), Some(r)) => {
            validate_mission_id(m)?;
            (absolutize(r, &cwd), m.clone())
        }
        (Some(m), None) => {
            validate_mission_id(m)?;
            (cwd, m.clone())
        }
        (None, _) => {
            return Err(CliError::MissionNotFound {
                message: "`--mission <id>` is required for `init`".to_string(),
                hint: Some("Example: `codex1 init --mission demo`.".to_string()),
                ambiguous: false,
            });
        }
    };
    if !root.is_dir() {
        return Err(CliError::MissionNotFound {
            message: format!("Repo root does not exist: {}", root.display()),
            hint: None,
            ambiguous: false,
        });
    }
    Ok(MissionPaths::new(root, mission))
}

fn absolutize(p: &Path, cwd: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

fn discover_single_mission(root: &Path) -> Result<MissionPaths, CliError> {
    let plans = root.join("PLANS");
    if !plans.is_dir() {
        return Err(CliError::MissionNotFound {
            message: format!("No PLANS/ directory under {}", root.display()),
            hint: Some(
                "Run `codex1 init --mission <id>` to create one, or point --repo-root elsewhere."
                    .to_string(),
            ),
            ambiguous: false,
        });
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&plans).map_err(CliError::Io)?.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("STATE.json").is_file() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                candidates.push(name.to_string());
            }
        }
    }
    match candidates.len() {
        0 => Err(CliError::MissionNotFound {
            message: format!("No missions with STATE.json under {}", plans.display()),
            hint: None,
            ambiguous: false,
        }),
        1 => Ok(MissionPaths::new(root.to_path_buf(), candidates.remove(0))),
        n => Err(CliError::MissionNotFound {
            message: format!(
                "{n} candidate missions under {}; pass --mission <id> to disambiguate",
                plans.display()
            ),
            hint: None,
            ambiguous: true,
        }),
    }
}

fn walk_up_for_mission(cwd: &Path) -> Result<MissionPaths, CliError> {
    let mut current = Some(cwd);
    while let Some(dir) = current {
        let plans = dir.join("PLANS");
        if plans.is_dir() {
            return discover_single_mission(dir);
        }
        current = dir.parent();
    }
    Err(CliError::MissionNotFound {
        message: "No PLANS/ directory found walking up from the current directory".to_string(),
        hint: Some(
            "Pass --mission <id> --repo-root <path>, or run `codex1 init` first.".to_string(),
        ),
        ambiguous: false,
    })
}
