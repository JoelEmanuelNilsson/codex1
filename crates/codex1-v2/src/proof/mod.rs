//! Proof receipts — hash-bound references to per-task PROOF.md files.
//!
//! Workers finish a task by producing a proof artefact at
//! `specs/T<N>/PROOF.md`. `task finish` reads that file, hashes it, and
//! records the hash on the task state. Reviewers later bind their outputs
//! to the same hash via `evidence_snapshot_hash`; if the proof file
//! changes, all prior reviewer outputs bind to a stale hash and are
//! quarantined as `STALE_OUTPUT`.

// Non-test callers arrive in T15/T16 (`task start`, `task finish`).
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::CliError;

/// Conventional filename inside `specs/T<N>/`.
pub const PROOF_FILENAME: &str = "PROOF.md";

/// Hash-bound proof artefact receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofReceipt {
    /// Path relative to the mission directory (e.g. `specs/T1/PROOF.md`).
    pub proof_ref: String,
    /// Content hash: `sha256:<hex>`.
    pub proof_hash: String,
    /// RFC-3339 timestamp when the receipt was captured.
    pub captured_at: String,
}

/// Default proof-file path for a task: `specs/T<N>/PROOF.md`.
#[must_use]
pub fn default_proof_ref(task_id: &str) -> PathBuf {
    PathBuf::from("specs").join(task_id).join(PROOF_FILENAME)
}

/// Read the proof file and produce a [`ProofReceipt`].
///
/// `proof_rel` is interpreted relative to `mission_dir`. Missing file
/// becomes [`CliError::ProofInvalid`] (not `Io`) so the envelope is
/// actionable.
pub fn read_and_hash(
    mission_dir: &Path,
    proof_rel: &Path,
    now_iso: &str,
) -> Result<ProofReceipt, CliError> {
    // Guard against `..` segments that would escape the mission dir.
    if proof_rel.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::RootDir
        )
    }) {
        return Err(CliError::ProofInvalid {
            path: proof_rel.display().to_string(),
            reason: "proof path must not contain '..' or '/'".into(),
        });
    }
    let full = mission_dir.join(proof_rel);
    let bytes = std::fs::read(&full).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::ProofInvalid {
                path: full.display().to_string(),
                reason: "file not found".into(),
            }
        } else {
            CliError::Io {
                path: full.display().to_string(),
                source: e,
            }
        }
    })?;
    if bytes.is_empty() {
        return Err(CliError::ProofInvalid {
            path: full.display().to_string(),
            reason: "proof file is empty".into(),
        });
    }
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let proof_hash = format!("sha256:{:x}", hasher.finalize());
    Ok(ProofReceipt {
        proof_ref: proof_rel.to_string_lossy().into_owned(),
        proof_hash,
        captured_at: now_iso.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{default_proof_ref, read_and_hash, ProofReceipt};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn default_proof_ref_builds_specs_path() {
        assert_eq!(
            default_proof_ref("T1").to_string_lossy(),
            "specs/T1/PROOF.md"
        );
    }

    #[test]
    fn read_and_hash_returns_stable_receipt() {
        let dir = tempdir().unwrap();
        let rel = std::path::Path::new("specs/T1/PROOF.md");
        let full = dir.path().join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, b"some proof content").unwrap();
        let receipt = read_and_hash(dir.path(), rel, "2026-04-18T10:00:00Z").unwrap();
        assert_eq!(receipt.proof_ref, "specs/T1/PROOF.md");
        assert!(receipt.proof_hash.starts_with("sha256:"));
        assert_eq!(receipt.captured_at, "2026-04-18T10:00:00Z");
        let r2 = read_and_hash(dir.path(), rel, "2026-04-18T10:00:01Z").unwrap();
        // Same bytes → same hash.
        assert_eq!(receipt.proof_hash, r2.proof_hash);
    }

    #[test]
    fn different_bytes_produce_different_hashes() {
        let dir = tempdir().unwrap();
        let rel = std::path::Path::new("specs/T1/PROOF.md");
        let full = dir.path().join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();

        fs::write(&full, b"v1").unwrap();
        let a = read_and_hash(dir.path(), rel, "x").unwrap();
        fs::write(&full, b"v2").unwrap();
        let b = read_and_hash(dir.path(), rel, "x").unwrap();
        assert_ne!(a.proof_hash, b.proof_hash);
    }

    #[test]
    fn missing_proof_returns_proof_invalid() {
        let dir = tempdir().unwrap();
        let rel = std::path::Path::new("specs/T1/PROOF.md");
        let err = read_and_hash(dir.path(), rel, "x").unwrap_err();
        assert_eq!(err.code(), "PROOF_INVALID");
    }

    #[test]
    fn empty_proof_returns_proof_invalid() {
        let dir = tempdir().unwrap();
        let rel = std::path::Path::new("specs/T1/PROOF.md");
        let full = dir.path().join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, b"").unwrap();
        let err = read_and_hash(dir.path(), rel, "x").unwrap_err();
        assert_eq!(err.code(), "PROOF_INVALID");
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn parent_dir_segments_rejected() {
        let dir = tempdir().unwrap();
        let rel = std::path::Path::new("../escape/PROOF.md");
        let err = read_and_hash(dir.path(), rel, "x").unwrap_err();
        assert_eq!(err.code(), "PROOF_INVALID");
        assert!(err.to_string().contains(".."));
    }

    #[test]
    fn receipt_serde_roundtrip() {
        let r = ProofReceipt {
            proof_ref: "specs/T1/PROOF.md".into(),
            proof_hash: "sha256:abc".into(),
            captured_at: "2026-04-18T10:00:00Z".into(),
        };
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(j["proof_ref"], "specs/T1/PROOF.md");
        assert_eq!(j["proof_hash"], "sha256:abc");
        let parsed: ProofReceipt = serde_json::from_value(j).unwrap();
        assert_eq!(parsed, r);
    }
}
