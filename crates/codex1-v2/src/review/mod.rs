//! Review bundles, reviewer outputs, and cleanliness computation.
//!
//! The review contract is the retrospective's most important remedy: no
//! parent self-review, every reviewer output binds to the truth it was
//! produced against, and any P0/P1/P2 finding keeps the bundle dirty.

// Non-test callers land in T21/T22 (review CLI commands).
#![allow(dead_code)]

pub(crate) mod bundle;
pub(crate) mod clean;
pub(crate) mod output;

use std::path::Path;

use walkdir::WalkDir;

use crate::error::CliError;

// Re-exports consumed by T21/T22 review CLI commands and by test helpers.
#[allow(unused_imports)]
pub use bundle::{
    ReviewBundle, ReviewRequirement, ReviewStatus, ReviewTarget, mission_close_evidence_hash,
};
#[allow(unused_imports)]
pub use clean::{CleanlinessVerdict, CurrentTruth, compute_cleanliness};
#[allow(unused_imports)]
pub use output::{Finding, FindingSeverity, ReviewerOutput, ReviewerResultKind};

/// Conventional directory for bundles inside a mission.
pub const BUNDLES_DIRNAME: &str = "reviews";
/// Conventional directory for reviewer outputs inside a mission.
pub const OUTPUTS_DIRNAME: &str = "reviews/outputs";

/// Load every `B<digits>.json` bundle in `bundles_dir`. Any bundle-shaped
/// file that fails to deserialize raises `REVIEW_BUNDLE_CORRUPT` rather
/// than being silently dropped: review artefacts are mission truth, so a
/// malformed bundle is a fail-closed condition.
///
/// Files with unrelated names (READMEs, scratch JSON, `.DS_Store`) are
/// ignored so mission authors can keep notes alongside bundles.
pub fn load_all_bundles(bundles_dir: &Path) -> Result<Vec<ReviewBundle>, CliError> {
    if !bundles_dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(bundles_dir).min_depth(1).max_depth(1) {
        let entry = entry.map_err(|e| CliError::Io {
            path: bundles_dir.display().to_string(),
            source: e
                .into_io_error()
                .unwrap_or_else(|| std::io::Error::other("walkdir error")),
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if !is_bundle_filename(&name) {
            continue;
        }
        let bytes = std::fs::read(entry.path()).map_err(|e| CliError::Io {
            path: entry.path().display().to_string(),
            source: e,
        })?;
        match serde_json::from_slice::<ReviewBundle>(&bytes) {
            Ok(b) => out.push(b),
            Err(e) => {
                return Err(CliError::ReviewBundleCorrupt {
                    path: entry.path().display().to_string(),
                    reason: format!("JSON parse: {e}"),
                });
            }
        }
    }
    Ok(out)
}

fn is_bundle_filename(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };
    let Some(rest) = stem.strip_prefix('B') else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{is_bundle_filename, load_all_bundles};
    use tempfile::tempdir;

    #[test]
    fn bundle_filename_shape() {
        assert!(is_bundle_filename("B1.json"));
        assert!(is_bundle_filename("B999.json"));
        assert!(!is_bundle_filename("B.json"));
        assert!(!is_bundle_filename("Babc.json"));
        assert!(!is_bundle_filename("b1.json"));
        assert!(!is_bundle_filename("B1.txt"));
        assert!(!is_bundle_filename("README.md"));
    }

    #[test]
    fn corrupt_bundle_is_fail_closed() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("B1.json"), b"{ not json").unwrap();
        let err = load_all_bundles(dir.path()).unwrap_err();
        assert_eq!(err.code(), "REVIEW_BUNDLE_CORRUPT");
    }

    #[test]
    fn unrelated_files_are_ignored() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), b"notes").unwrap();
        std::fs::write(dir.path().join("scratch.json"), b"{ not json").unwrap();
        // Stray JSON that isn't bundle-shaped stays tolerated.
        let bundles = load_all_bundles(dir.path()).unwrap();
        assert!(bundles.is_empty());
    }

    #[test]
    fn missing_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let bundles = load_all_bundles(&dir.path().join("nope")).unwrap();
        assert!(bundles.is_empty());
    }
}
