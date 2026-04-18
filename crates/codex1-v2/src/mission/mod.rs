//! Mission directory resolution and safe-slug validation.
//!
//! Mission-id contract: `^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$` (1–64 chars,
//! lowercase ASCII + digits + internal dashes only). Rejecting unsafe slugs
//! closes off directory traversal (`..`, absolute paths) and anchors the
//! layout at `<repo_root>/PLANS/<mission-id>/`.

// T11 (`init`, `validate`) will consume these helpers; until then the only
// call sites are in tests.
#![allow(dead_code)]

pub(crate) mod lock;

use std::path::{Path, PathBuf};

use crate::error::CliError;

/// Directory (under the repo root) that holds every mission.
pub const MISSIONS_ROOT: &str = "PLANS";

/// Canonical filenames inside a mission directory.
pub const OUTCOME_LOCK_FILENAME: &str = "OUTCOME-LOCK.md";
pub const PROGRAM_BLUEPRINT_FILENAME: &str = "PROGRAM-BLUEPRINT.md";

/// Maximum length of a mission id (bytes = chars because ASCII only).
const MISSION_ID_MAX_LEN: usize = 64;

/// Paths derived from a `(repo_root, mission_id)` pair. All paths are
/// absolute if `repo_root` is absolute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionPaths {
    pub repo_root: PathBuf,
    pub mission_id: String,
    pub mission_dir: PathBuf,
}

impl MissionPaths {
    /// Path to `OUTCOME-LOCK.md`.
    #[must_use]
    pub fn outcome_lock(&self) -> PathBuf {
        self.mission_dir.join(OUTCOME_LOCK_FILENAME)
    }

    /// Path to `PROGRAM-BLUEPRINT.md`.
    #[must_use]
    pub fn program_blueprint(&self) -> PathBuf {
        self.mission_dir.join(PROGRAM_BLUEPRINT_FILENAME)
    }

    /// Path to `specs/`.
    #[must_use]
    pub fn specs_dir(&self) -> PathBuf {
        self.mission_dir.join("specs")
    }

    /// Path to `reviews/`.
    #[must_use]
    pub fn reviews_dir(&self) -> PathBuf {
        self.mission_dir.join("reviews")
    }
}

/// Resolve `--repo-root` against the process cwd. Explicit errors for
/// missing/non-directory paths. **No git-root walk** — policy is to fail
/// loud if the caller supplies a bad value.
pub fn resolve_repo_root(provided: Option<&Path>) -> Result<PathBuf, CliError> {
    let raw = match provided {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().map_err(|e| CliError::RepoRootInvalid {
            reason: format!("cwd not available: {e}"),
            path: String::new(),
        })?,
    };
    let canonical = raw.canonicalize().map_err(|e| CliError::RepoRootInvalid {
        reason: format!("canonicalize failed: {e}"),
        path: raw.display().to_string(),
    })?;
    if !canonical.is_dir() {
        return Err(CliError::RepoRootInvalid {
            reason: "not a directory".into(),
            path: canonical.display().to_string(),
        });
    }
    Ok(canonical)
}

/// Validate a mission id against the safe-slug contract.
pub fn validate_mission_id(id: &str) -> Result<(), CliError> {
    let len = id.len();
    if !(1..=MISSION_ID_MAX_LEN).contains(&len) {
        return Err(CliError::MissionIdInvalid {
            got: id.to_string(),
        });
    }
    let bytes = id.as_bytes();
    if !is_lower_alnum(bytes[0]) {
        return Err(CliError::MissionIdInvalid {
            got: id.to_string(),
        });
    }
    if len > 1 && !is_lower_alnum(bytes[len - 1]) {
        return Err(CliError::MissionIdInvalid {
            got: id.to_string(),
        });
    }
    if len >= 3 {
        for &b in &bytes[1..len - 1] {
            if !(is_lower_alnum(b) || b == b'-') {
                return Err(CliError::MissionIdInvalid {
                    got: id.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// Validate the mission id and construct the derived paths. Does **not**
/// require the mission directory to exist — that's the caller's concern
/// (`init` creates it; `status`/`validate` error out via STATE.json load).
pub fn resolve_mission(repo_root: &Path, mission_id: &str) -> Result<MissionPaths, CliError> {
    validate_mission_id(mission_id)?;
    let mission_dir = repo_root.join(MISSIONS_ROOT).join(mission_id);
    Ok(MissionPaths {
        repo_root: repo_root.to_path_buf(),
        mission_id: mission_id.to_string(),
        mission_dir,
    })
}

fn is_lower_alnum(b: u8) -> bool {
    b.is_ascii_digit() || b.is_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{MissionPaths, resolve_mission, resolve_repo_root, validate_mission_id};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn valid_mission_ids_accepted() {
        for id in [
            "a",
            "a1",
            "example",
            "codex1-v2",
            "m-1",
            "12345",
            "kernel-and-dag",
            &"a".repeat(64),
        ] {
            assert!(validate_mission_id(id).is_ok(), "{id:?} should be accepted");
        }
    }

    #[test]
    fn invalid_mission_ids_rejected() {
        for id in [
            "",
            " ",
            "-leading",
            "trailing-",
            "Uppercase",
            "has space",
            "dot.segment",
            "slash/segment",
            "..",
            "../escape",
            "/abs",
            &"a".repeat(65),
            "a_underscore",
        ] {
            let err = validate_mission_id(id).unwrap_err();
            assert_eq!(err.code(), "MISSION_ID_INVALID", "{id:?}");
        }
    }

    #[test]
    fn resolve_mission_builds_expected_paths() {
        let repo = PathBuf::from("/tmp/repo");
        let paths = resolve_mission(&repo, "example").unwrap();
        assert_eq!(paths.mission_dir, PathBuf::from("/tmp/repo/PLANS/example"));
        assert_eq!(
            paths.outcome_lock(),
            PathBuf::from("/tmp/repo/PLANS/example/OUTCOME-LOCK.md")
        );
        assert_eq!(
            paths.program_blueprint(),
            PathBuf::from("/tmp/repo/PLANS/example/PROGRAM-BLUEPRINT.md")
        );
        assert_eq!(
            paths.specs_dir(),
            PathBuf::from("/tmp/repo/PLANS/example/specs")
        );
    }

    #[test]
    fn resolve_mission_rejects_bad_id() {
        let repo = PathBuf::from("/tmp/repo");
        assert!(resolve_mission(&repo, "../escape").is_err());
        assert!(resolve_mission(&repo, "Upper").is_err());
    }

    #[test]
    fn resolve_repo_root_with_provided_existing_dir() {
        let dir = tempdir().unwrap();
        let resolved = resolve_repo_root(Some(dir.path())).unwrap();
        // canonicalize may resolve /private/var/folders on macOS
        assert!(resolved.is_dir());
    }

    #[test]
    fn resolve_repo_root_rejects_missing() {
        let err = resolve_repo_root(Some(&PathBuf::from("/nope/does-not-exist-42"))).unwrap_err();
        assert_eq!(err.code(), "REPO_ROOT_INVALID");
    }

    #[test]
    fn resolve_repo_root_rejects_non_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("a-file");
        std::fs::write(&file_path, "hi").unwrap();
        let err = resolve_repo_root(Some(&file_path)).unwrap_err();
        assert_eq!(err.code(), "REPO_ROOT_INVALID");
    }

    #[test]
    fn mission_paths_equality() {
        let repo = PathBuf::from("/tmp/repo");
        let a = resolve_mission(&repo, "example").unwrap();
        let b = MissionPaths {
            repo_root: repo.clone(),
            mission_id: "example".into(),
            mission_dir: repo.join("PLANS").join("example"),
        };
        assert_eq!(a, b);
    }
}
