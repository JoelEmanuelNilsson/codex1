//! `OUTCOME-LOCK.md` structural validator.
//!
//! Wave 1 contract (see plan "File contracts Wave 1 writes"):
//!
//! ```text
//! ---
//! mission_id: <id>
//! title: <title>
//! lock_status: draft|ratified
//! created_at: <RFC-3339>
//! updated_at: <RFC-3339>
//! ---
//!
//! # Outcome Lock: <title>
//!
//! ## Destination
//! ## Constraints
//! ## Success Criteria
//! ```
//!
//! Content under each heading is **not** inspected — only structural
//! presence is verified. `$clarify` (Wave 4) flips `lock_status` from
//! `draft` to `ratified`.

// T11 (`init`, `validate`) will be the non-test caller.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CliError;

/// Parsed + validated OUTCOME-LOCK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeLock {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
}

/// YAML frontmatter. `deny_unknown_fields` so typos fail loud.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Frontmatter {
    pub mission_id: String,
    pub title: String,
    pub lock_status: LockStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    Draft,
    Ratified,
}

const REQUIRED_HEADINGS: [&str; 3] = ["## Destination", "## Constraints", "## Success Criteria"];

/// Read and validate OUTCOME-LOCK.md at `path`.
pub fn parse_and_validate(path: &Path) -> Result<OutcomeLock, CliError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::LockInvalid {
                path: path.display().to_string(),
                reason: "file not found".into(),
                source: None,
            }
        } else {
            CliError::Io {
                path: path.display().to_string(),
                source: e,
            }
        }
    })?;
    validate_content(path, &content)
}

/// Validate already-loaded content. Exposed for tests.
pub fn validate_content(path: &Path, content: &str) -> Result<OutcomeLock, CliError> {
    let (frontmatter_block, body) = split_frontmatter(path, content)?;
    let frontmatter: Frontmatter =
        serde_yaml::from_str(frontmatter_block).map_err(|e| CliError::LockInvalid {
            path: path.display().to_string(),
            reason: format!("frontmatter YAML: {e}"),
            source: None,
        })?;
    validate_frontmatter(path, &frontmatter)?;
    validate_sections(path, body)?;
    Ok(OutcomeLock {
        path: path.to_path_buf(),
        frontmatter,
    })
}

fn split_frontmatter<'a>(path: &Path, content: &'a str) -> Result<(&'a str, &'a str), CliError> {
    // The file must start with `---\n` (tolerate leading BOM? Not in Wave 1).
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| CliError::LockInvalid {
            path: path.display().to_string(),
            reason: "file must begin with '---' frontmatter fence".into(),
            source: None,
        })?;
    // Find the closing `---` on its own line.
    let closing = find_closing_fence(rest).ok_or_else(|| CliError::LockInvalid {
        path: path.display().to_string(),
        reason: "frontmatter missing closing '---' fence".into(),
        source: None,
    })?;
    let (yaml, after) = rest.split_at(closing.start);
    // Skip the fence line itself.
    let body = &after[closing.fence_len..];
    Ok((yaml, body))
}

struct ClosingFence {
    start: usize,
    fence_len: usize,
}

fn find_closing_fence(s: &str) -> Option<ClosingFence> {
    // Search for `\n---\n` or `\n---\r\n` (a line that is just `---`).
    let patterns: [(&str, usize); 2] = [("\n---\n", 5), ("\n---\r\n", 6)];
    for (pat, len) in patterns {
        if let Some(idx) = s.find(pat) {
            return Some(ClosingFence {
                start: idx + 1,     // start of `---`
                fence_len: len - 1, // skip `---\n` (we already consumed the leading \n)
            });
        }
    }
    // Also accept a fence at end-of-file without trailing newline.
    if let Some(stripped) = s.strip_suffix("\n---") {
        return Some(ClosingFence {
            start: stripped.len() + 1,
            fence_len: 3,
        });
    }
    None
}

fn validate_frontmatter(path: &Path, fm: &Frontmatter) -> Result<(), CliError> {
    if fm.mission_id.trim().is_empty() {
        return Err(CliError::LockInvalid {
            path: path.display().to_string(),
            reason: "frontmatter.mission_id must be non-empty".into(),
            source: None,
        });
    }
    if fm.title.trim().is_empty() {
        return Err(CliError::LockInvalid {
            path: path.display().to_string(),
            reason: "frontmatter.title must be non-empty".into(),
            source: None,
        });
    }
    if !is_rfc3339_ish(&fm.created_at) {
        return Err(CliError::LockInvalid {
            path: path.display().to_string(),
            reason: format!(
                "frontmatter.created_at is not RFC-3339: {:?}",
                fm.created_at
            ),
            source: None,
        });
    }
    if !is_rfc3339_ish(&fm.updated_at) {
        return Err(CliError::LockInvalid {
            path: path.display().to_string(),
            reason: format!(
                "frontmatter.updated_at is not RFC-3339: {:?}",
                fm.updated_at
            ),
            source: None,
        });
    }
    Ok(())
}

fn validate_sections(path: &Path, body: &str) -> Result<(), CliError> {
    for heading in REQUIRED_HEADINGS {
        if !contains_heading_line(body, heading) {
            return Err(CliError::LockInvalid {
                path: path.display().to_string(),
                reason: format!("missing required section {heading:?}"),
                source: None,
            });
        }
    }
    Ok(())
}

fn contains_heading_line(body: &str, heading: &str) -> bool {
    body.lines().any(|line| line.trim_end() == heading)
}

fn is_rfc3339_ish(s: &str) -> bool {
    // Strict RFC-3339 parse via the `time` crate.
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::parse(s, &Rfc3339).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{LockStatus, parse_and_validate, validate_content};
    use std::path::Path;
    use tempfile::tempdir;

    const VALID: &str = "---\n\
mission_id: example\n\
title: Smoke\n\
lock_status: draft\n\
created_at: 2026-04-18T10:00:00Z\n\
updated_at: 2026-04-18T10:00:00Z\n\
---\n\
\n\
# Outcome Lock: Smoke\n\
\n\
## Destination\nTBD\n\
\n\
## Constraints\nTBD\n\
\n\
## Success Criteria\nTBD\n";

    #[test]
    fn valid_lock_parses() {
        let parsed = validate_content(Path::new("/x/OUTCOME-LOCK.md"), VALID).unwrap();
        assert_eq!(parsed.frontmatter.mission_id, "example");
        assert_eq!(parsed.frontmatter.title, "Smoke");
        assert_eq!(parsed.frontmatter.lock_status, LockStatus::Draft);
    }

    #[test]
    fn ratified_status_accepted() {
        let src = VALID.replace("lock_status: draft", "lock_status: ratified");
        let parsed = validate_content(Path::new("/x/L.md"), &src).unwrap();
        assert_eq!(parsed.frontmatter.lock_status, LockStatus::Ratified);
    }

    #[test]
    fn missing_frontmatter_rejected() {
        let err = validate_content(Path::new("/x/L.md"), "# just body\n").unwrap_err();
        assert_eq!(err.code(), "LOCK_INVALID");
        assert!(err.to_string().contains("frontmatter"));
    }

    #[test]
    fn malformed_yaml_rejected() {
        let src = "---\nmission_id: :bad yaml\n---\n\n## Destination\n## Constraints\n## Success Criteria\n";
        let err = validate_content(Path::new("/x/L.md"), src).unwrap_err();
        assert_eq!(err.code(), "LOCK_INVALID");
        assert!(err.to_string().contains("frontmatter YAML"));
    }

    #[test]
    fn unknown_frontmatter_field_rejected() {
        let src = VALID.replace(
            "updated_at: 2026-04-18T10:00:00Z\n",
            "updated_at: 2026-04-18T10:00:00Z\nextra: uh oh\n",
        );
        let err = validate_content(Path::new("/x/L.md"), &src).unwrap_err();
        assert_eq!(err.code(), "LOCK_INVALID");
    }

    #[test]
    fn missing_destination_section_rejected() {
        let src = VALID.replace("## Destination\nTBD\n\n", "");
        let err = validate_content(Path::new("/x/L.md"), &src).unwrap_err();
        assert!(err.to_string().contains("Destination"));
    }

    #[test]
    fn missing_constraints_section_rejected() {
        let src = VALID.replace("## Constraints\nTBD\n\n", "");
        let err = validate_content(Path::new("/x/L.md"), &src).unwrap_err();
        assert!(err.to_string().contains("Constraints"));
    }

    #[test]
    fn missing_success_criteria_section_rejected() {
        let src = VALID.replace("## Success Criteria\nTBD\n", "");
        let err = validate_content(Path::new("/x/L.md"), &src).unwrap_err();
        assert!(err.to_string().contains("Success Criteria"));
    }

    #[test]
    fn non_rfc3339_timestamps_rejected() {
        let src = VALID.replace("created_at: 2026-04-18T10:00:00Z", "created_at: not-a-date");
        let err = validate_content(Path::new("/x/L.md"), &src).unwrap_err();
        assert!(err.to_string().contains("created_at"));
    }

    #[test]
    fn invalid_lock_status_rejected() {
        let src = VALID.replace("lock_status: draft", "lock_status: maybe");
        let err = validate_content(Path::new("/x/L.md"), &src).unwrap_err();
        assert_eq!(err.code(), "LOCK_INVALID");
    }

    #[test]
    fn parse_and_validate_reads_from_disk() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("OUTCOME-LOCK.md");
        std::fs::write(&p, VALID).unwrap();
        let parsed = parse_and_validate(&p).unwrap();
        assert_eq!(parsed.frontmatter.mission_id, "example");
    }

    #[test]
    fn missing_file_reports_lock_invalid() {
        let err = parse_and_validate(Path::new("/nope/OUTCOME-LOCK.md")).unwrap_err();
        assert_eq!(err.code(), "LOCK_INVALID");
        assert!(err.to_string().contains("file not found"));
    }
}
