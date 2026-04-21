//! Mission path resolution helpers.
//!
//! Foundation pins the layout under `PLANS/<mission-id>/`:
//! ```text
//! PLANS/<id>/
//!   OUTCOME.md
//!   PLAN.yaml
//!   STATE.json
//!   STATE.json.lock    (fs2 exclusive lock file)
//!   EVENTS.jsonl
//!   CLOSEOUT.md        (written by `close complete`)
//!   specs/
//!   reviews/
//! ```

use std::path::{Component, Path, PathBuf};

use crate::core::error::CliError;

/// Mission directory paths computed from a `<repo-root>/PLANS/<mission-id>/`
/// base. Every helper returns an absolute path; no IO is performed.
#[derive(Debug, Clone)]
pub struct MissionPaths {
    pub repo_root: PathBuf,
    pub mission_id: String,
    pub mission_dir: PathBuf,
}

impl MissionPaths {
    #[must_use]
    pub fn new(repo_root: PathBuf, mission_id: String) -> Self {
        let mission_dir = repo_root.join("PLANS").join(&mission_id);
        Self {
            repo_root,
            mission_id,
            mission_dir,
        }
    }

    #[must_use]
    pub fn outcome(&self) -> PathBuf {
        self.mission_dir.join("OUTCOME.md")
    }

    #[must_use]
    pub fn plan(&self) -> PathBuf {
        self.mission_dir.join("PLAN.yaml")
    }

    #[must_use]
    pub fn state(&self) -> PathBuf {
        self.mission_dir.join("STATE.json")
    }

    #[must_use]
    pub fn state_lock(&self) -> PathBuf {
        self.mission_dir.join("STATE.json.lock")
    }

    #[must_use]
    pub fn events(&self) -> PathBuf {
        self.mission_dir.join("EVENTS.jsonl")
    }

    #[must_use]
    pub fn closeout(&self) -> PathBuf {
        self.mission_dir.join("CLOSEOUT.md")
    }

    #[must_use]
    pub fn specs_dir(&self) -> PathBuf {
        self.mission_dir.join("specs")
    }

    #[must_use]
    pub fn reviews_dir(&self) -> PathBuf {
        self.mission_dir.join("reviews")
    }

    #[must_use]
    pub fn spec_dir_for(&self, task_id: &str) -> PathBuf {
        self.specs_dir().join(task_id)
    }

    #[must_use]
    pub fn spec_file_for(&self, task_id: &str) -> PathBuf {
        self.spec_dir_for(task_id).join("SPEC.md")
    }

    #[must_use]
    pub fn proof_file_for(&self, task_id: &str) -> PathBuf {
        self.spec_dir_for(task_id).join("PROOF.md")
    }

    #[must_use]
    pub fn review_file_for(&self, task_id: &str) -> PathBuf {
        self.reviews_dir().join(format!("{task_id}.md"))
    }
}

pub fn validate_mission_id(id: &str) -> Result<(), CliError> {
    if id.trim().is_empty() {
        return Err(invalid_mission_id(id, "mission id cannot be empty"));
    }
    let path = Path::new(id);
    if path.is_absolute() {
        return Err(invalid_mission_id(
            id,
            "mission id must not be an absolute path",
        ));
    }
    if path.components().count() != 1 {
        return Err(invalid_mission_id(
            id,
            "mission id must be a single path component",
        ));
    }
    match path.components().next() {
        Some(Component::Normal(_)) if id != "." && id != ".." => Ok(()),
        _ => Err(invalid_mission_id(id, "mission id must not be `.` or `..`")),
    }
}

fn invalid_mission_id(id: &str, detail: &str) -> CliError {
    CliError::MissionNotFound {
        message: format!("Invalid mission id `{id}`: {detail}"),
        hint: Some("Use a simple directory name such as `demo` or `codex1-rebuild`.".to_string()),
        ambiguous: false,
    }
}

pub fn resolve_existing_mission_file(
    paths: &MissionPaths,
    rel: &str,
    field: &str,
) -> Result<PathBuf, CliError> {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return Err(CliError::PlanInvalid {
            message: format!("{field} must be relative to the mission directory, got `{rel}`"),
            hint: Some("Use a mission-local path such as `specs/T1/SPEC.md`.".to_string()),
        });
    }
    if candidate.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(CliError::PlanInvalid {
            message: format!("{field} must not escape the mission directory: `{rel}`"),
            hint: Some("Use a mission-local path such as `specs/T1/SPEC.md`.".to_string()),
        });
    }
    let abs = paths.mission_dir.join(candidate);
    if !abs.is_file() {
        return Err(CliError::PlanInvalid {
            message: format!("{field} file not found at {}", abs.display()),
            hint: Some(format!("Create `{rel}` under the mission directory.")),
        });
    }
    let mission_dir = paths.mission_dir.canonicalize()?;
    let file = abs.canonicalize()?;
    if !file.starts_with(&mission_dir) {
        return Err(CliError::PlanInvalid {
            message: format!("{field} escapes the mission directory: `{rel}`"),
            hint: Some("Use a mission-local path such as `specs/T1/SPEC.md`.".to_string()),
        });
    }
    Ok(file)
}

/// True if the given path has a `PLANS` directory with at least one
/// mission subdirectory containing a `STATE.json`.
#[must_use]
pub fn looks_like_repo_root(path: &Path) -> bool {
    let plans = path.join("PLANS");
    if !plans.is_dir() {
        return false;
    }
    match std::fs::read_dir(&plans) {
        Ok(iter) => iter
            .flatten()
            .any(|entry| entry.path().join("STATE.json").is_file()),
        Err(_) => false,
    }
}
