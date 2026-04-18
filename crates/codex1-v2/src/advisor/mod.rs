//! Advisor checkpoints — parent-invoked, non-formal critique.
//!
//! The V2 retrospective called out that advisor / `CritiqueScout` output is
//! *useful* but **not formal review evidence**. This module stores
//! advisor notes in a dedicated `advisor-notes.jsonl` file so they can be
//! inspected later without being counted toward bundle cleanliness.

#![allow(dead_code)]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CliError;

pub const ADVISOR_NOTES_FILENAME: &str = "advisor-notes.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdvisorNote {
    pub at: String,
    pub checkpoint: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    /// Explicit marker that this note is **not** review evidence.
    #[serde(default = "default_non_formal")]
    pub non_formal: bool,
}

fn default_non_formal() -> bool {
    true
}

/// Append an advisor note to `advisor-notes.jsonl`. Notes are audit-only
/// (like `events.jsonl`): never authoritative, never counted.
pub fn append_note(mission_dir: &Path, note: &AdvisorNote) -> Result<(), CliError> {
    let path = mission_dir.join(ADVISOR_NOTES_FILENAME);
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|e| CliError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
    let line = serde_json::to_string(note).map_err(|e| CliError::Internal {
        message: format!("serialize advisor note: {e}"),
    })?;
    file.write_all(line.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
        .map_err(|e| CliError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ADVISOR_NOTES_FILENAME, AdvisorNote, append_note};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn append_then_read_lines() {
        let dir = tempdir().unwrap();
        let note = AdvisorNote {
            at: "2026-04-18T10:00:00Z".into(),
            checkpoint: "pre_mission_close".into(),
            summary: "check coupling to cart".into(),
            task_id: Some("T3".into()),
            bundle_id: None,
            non_formal: true,
        };
        append_note(dir.path(), &note).unwrap();
        let raw = fs::read_to_string(dir.path().join(ADVISOR_NOTES_FILENAME)).unwrap();
        assert_eq!(raw.lines().count(), 1);
        let parsed: AdvisorNote = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
        assert_eq!(parsed, note);
    }

    #[test]
    fn default_non_formal_is_true() {
        let json = serde_json::json!({
            "at": "2026-04-18T10:00:00Z",
            "checkpoint": "pre_plan",
            "summary": "test"
        });
        let parsed: AdvisorNote = serde_json::from_value(json).unwrap();
        assert!(parsed.non_formal);
    }

    #[test]
    fn deny_unknown_fields() {
        let json = serde_json::json!({
            "at": "t",
            "checkpoint": "c",
            "summary": "s",
            "not_known": 1
        });
        assert!(serde_json::from_value::<AdvisorNote>(json).is_err());
    }
}
